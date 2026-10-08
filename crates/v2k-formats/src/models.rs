//! Section 8 3D model decoder — canonical command stream interpreter.
//!
//! The retired Python extractor was an early discovery aid, not a reference
//! implementation; later Rust fixes intentionally diverge from its static
//! vertex output. Ground truth comes from the three 256-entry dispatch tables
//! in V2000.EXE (0x4D3CE0 render near,
//! 0x4D40E0 far LOD, 0x4D44E0 transform-only; render entry FUN_00464e60) and
//! the per-vertex type_flag transform table at 0x4D4A78. See
//! FORMAT_DOCUMENTATION.md §8 for the full opcode and type_flag tables.
//!
//! Model entry layout (file == runtime):
//!   `+0x00` u16 cmd_words     command stream length in words
//!   `+0x02` u8  extra         extra dword count
//!   `+0x03` u8  flags         bit 6: no embedded name; bit 5: force near LOD
//!   `+0x04` u16 slot_count    runtime vertex SLOT count (2 per model vertex)
//!   `+0x06` u16 face_val      normal pool size = (face_val - 2) >> 1
//!   `+0x08` u16 radius        LOD switch distance
//!   `+0x0A` u16 collision radius (raw 8.8 world units)
//!   `+0x0C` extra*4           extra data
//!   then  (slot_count>>1)*8   vertex records: int16 (type_flag, X, Y, Z)
//!   then  normals*8           normal pool: int16 (anchor_slot, nX, nY, nZ)
//!   then  cmd_words*2         command stream
//!   then  name (if !(flags & 0x40)), 4-byte aligned
//!
//! Runtime vertex slots are "doubled": slot 2i = vertex i, slot 2i+1 = its
//! X-negated mirror copy. All vertex/normal refs in commands are direct slot
//! indices. Mirrored faces use SEPARATE opcodes (the 0x_7/0x_8/0x27/0x28
//! families draw twice, second time with every ref XOR 1). The per-vertex
//! type_flag selects a transform: ~20% of vertices are *generated* (midpoints,
//! lerp/Bézier morphs, parallelogram completion, aliases, sky pins, ...)
//! rather than read literally. Ordinary affine generators commute with the
//! camera transform, but their inputs can depend on a presentation callback:
//! world type-12 aliases and type-13 endpoints must resolve before their
//! dependents consume their coordinates or linked imports copy their clip state.

use std::collections::HashMap;
use std::ops::Range;
use v2k_core::{Result, V2kError};

use crate::terrain::TerrainGrid;

mod terrain_collision;
use terrain_collision::terrain_sphere_hit;

mod native_spatial;
mod vertex;
pub use native_spatial::ModelNativeSpatialGenerator;
use vertex::SlotResolver;
pub use vertex::{
    LinkedModelSlots, ModelEdgeConstructorClip, ModelEdgeEndpointSnapshot, ModelNativeEdgeBoundary,
    ModelNativeEdgeEndpoint, ModelNativeEdgeFailure, ModelNativeVertexBoundary,
    ModelNativeVertexFailure, ModelSlotClip, ModelSurfaceOrigin, ModelVertexKind,
    ModelVertexResolver, ResolvedModelSlot,
};

mod painter;
pub use painter::{ModelPainterDepthKey, ModelPainterOp, ModelPainterSorting};
mod view_selection;
pub use view_selection::ModelViewSelection;

/// Lighting data carried by one Section-8 face command.
///
/// The 03/04 families submit no Section-6 shade values. The 43/44 families
/// submit the face normal's one Section-6 dword for the entire polygon
/// (`FUN_0045A9C0` -> `FUN_0047CBE0` for textured C3), while Gouraud 23/24
/// families resolve one normal-pool shade reference per corner. Texture and
/// mirror bits preserve these three distinct lighting policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFaceShading {
    Flat,
    FlatLit,
    Gouraud,
}

/// Complete authored polygon retained alongside each materialized triangle.
/// Both halves of a quad keep the same four vertex indices, including when
/// one half is degenerate. This preserves primitive boundaries for render
/// consumers without imposing any camera or clipping policy on decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFaceVertices {
    Triangle([u16; 3]),
    Quad([u16; 4]),
}

impl ModelFaceVertices {
    pub fn indices(&self) -> &[u16] {
        match self {
            Self::Triangle(indices) => indices,
            Self::Quad(indices) => indices,
        }
    }
}

/// Intrinsic source dependency used by a materialized vertex's projector.
/// Type 1 averages two projected source points; type 5 and other generated
/// model-space positions use the ordinary position projector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelVertexProjection {
    Position,
    /// Callback-owned true world point; bypass the body matrix when projecting.
    WorldPoint([f64; 3]),
    ScreenMidpoint([u16; 2]),
}

/// Authored face plane consumed by retail's model backface test.
///
/// This is not derived from triangle winding. `FUN_0046D3F0` pairs every face
/// normal with the model vertex named by normal-pool word 0, transforms that
/// plane with the model, and accepts only a strictly negative camera-relative
/// dot product.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelFaceCullPlane {
    /// Materialized model-space anchor in raw model units.
    pub anchor_raw: [f64; 3],
    /// Raw source record words selected by the normal's mirrored anchor ref.
    /// Native 46D3F0 reads these without invoking any generator callback.
    pub native_anchor_raw: Option<[i32; 3]>,
    /// Signed authored normal components before normalization.
    pub normal_raw: [i32; 3],
}

/// Retail backface policy attached to one materialized body triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelFaceCull {
    /// Normal refs 0/1 are implicit shade-zero entries initialized without a
    /// plane test by `FUN_0046D5A0`.
    AlwaysVisible,
    /// An authored plane accepted only from its camera-facing side.
    Plane(ModelFaceCullPlane),
    /// The installed live owner already accepted this raw source plane before
    /// any corner callback. Later renderers preserve that admission; projected
    /// or generated anchor coordinates cannot trigger a second plane test.
    LiveAdmitted(ModelFaceCullPlane),
    /// Malformed source reference. The renderer draws conservatively rather
    /// than silently deleting geometry.
    Unresolved,
}

/// Model entry header size in bytes.
const HEADER_SIZE: usize = 12;

/// 3×3 identity matrix (row-major). Root parent orientation for op-0x0E
/// child-instance frames.
const IDENTITY3: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

// ── Public data structures ──────────────────────────────────────────────────

/// Live per-entity animation state consumed by the command-stream interpreter.
///
/// Two operand sources are read at runtime (see `FUN_00470700` /
/// `FUN_00470840`): operand `& 0xC0 == 0xC0` reads the 64-entry **register
/// file** at `ctx+0x94` (`u16`), and operand `& 0xC0 == 0x80` invokes the
/// per-entity **callback** `(*ctx+0x58)(ctx+0x60, ctx, idx)` — modelled here
/// as a 64-entry table of the values that callback would return. Both are
/// indexed by `operand & 0x3F`.
///
/// `AnimVars::default()` supplies animation-frame-0 callback inputs. Both live
/// and cached default materialization execute register commands embedded in
/// the stream, matching the original interpreter rather than retired tooling.
#[derive(Debug, Clone)]
pub struct AnimVars {
    /// Register file `ctx+0x94` — operands with `& 0xC0 == 0xC0`.
    pub registers: [u16; 64],
    /// Dynamic callback values `(*ctx+0x58)(…, idx)` — operands with
    /// `& 0xC0 == 0x80` (e.g. the hull morph `t` on callback idx 5).
    pub dynamic: [i32; 64],
}

impl Default for AnimVars {
    fn default() -> Self {
        Self {
            registers: [0u16; 64],
            dynamic: [0i32; 64],
        }
    }
}

/// Per-frame materialized geometry produced by [`ModelEntry::materialize`].
/// Same layout as the resolved fields of [`ModelEntry`], but recomputed for a
/// live [`AnimVars`] (morphs, conditional-jump sub-assembly selection, mount
/// frames and child orientations all evaluated for this frame).
#[derive(Debug, Clone, Default)]
pub struct MaterializedModel {
    /// This evaluation reached a view-dependent 0x0B/0x0C command.
    pub has_view_commands: bool,
    /// Model-space positions in raw units (divide by 100 for world units).
    pub vertices: Vec<[f64; 3]>,
    /// Generator type for each materialized vertex (parallel to `vertices`).
    pub vertex_type_flags: Vec<i16>,
    /// Projector dependencies indexed into `vertices`, parallel to `vertices`.
    pub vertex_projection: Vec<ModelVertexProjection>,
    /// Semantic callback rejection, parallel to vertices; coordinates remain available.
    pub vertex_clip: Vec<ModelSlotClip>,
    /// World-surface endpoint provenance, parallel to vertices.
    pub vertex_surface_origin: Vec<ModelSurfaceOrigin>,
    /// Each vertex's current-node VIEW point (`FUN_0046D610`'s cache) when a
    /// vertex resolver owns that frame, parallel to `vertices`; `None` where
    /// the frame does not own the slot. Empty for intrinsic geometry.
    pub vertex_view_raw: Vec<Option<[i32; 3]>>,
    /// Triangle indices into `vertices`.
    pub triangles: Vec<[u16; 3]>,
    /// Complete source polygon for each triangle, indexed into `vertices`.
    pub face_vertices: Vec<ModelFaceVertices>,
    /// Per-triangle unit normals (parallel to `triangles`).
    pub normals: Vec<[f32; 3]>,
    /// Per-triangle authored backface policies (parallel to `triangles`). A
    /// malformed source reference remains explicitly unresolved and is drawn
    /// conservatively.
    pub face_cull: Vec<ModelFaceCull>,
    /// Per-triangle packed material id (`mat` operand; parallel to
    /// `triangles`). Bit 15 = sprite-pool flag (set when the face opcode had
    /// bit 0x80): clear → `mat` is a Section-7 palette colour index; set →
    /// `mat` is a global sprite id whose Section-3 image is texture-mapped
    /// over the face using `face_uvs`. Decode with [`face_material`].
    pub face_materials: Vec<u16>,
    /// Implicit texture coordinates generated by the original face handlers
    /// (parallel to `triangles`). Sprite-backed faces map their complete
    /// Section-3 image over the source triangle/quad; palette faces carry the
    /// same coordinates but ignore them.
    pub face_uvs: Vec<[[f32; 2]; 3]>,
    /// Per-corner lighting normals (parallel to `triangles`). Gouraud face
    /// commands resolve their trailing `s0..s3` normal-pool references here;
    /// uniformly lit faces repeat their one face normal at all three corners.
    /// Unlit faces also retain that normal as diagnostic geometry.
    pub face_corner_normals: Vec<[[f32; 3]; 3]>,
    /// The pool vectors behind `normals` and `face_corner_normals` as
    /// `FUN_0046D3F0` reads them (odd references X-negated, references 0/1
    /// zero): the face normal, then its three corners. Parallel to
    /// `triangles`.
    pub face_normals_raw: Vec<[[i32; 3]; 4]>,
    /// Retail unlit/uniformly lit/Gouraud handler family for each triangle.
    pub face_shading: Vec<ModelFaceShading>,
    /// Legacy diagnostic layer. Canonical materialization keeps authored
    /// type-13 faces in `triangles` with their materials and face metadata;
    /// this collection is intentionally empty.
    pub shadow_triangles: Vec<[u16; 3]>,
    /// Authored `0x02`/`0x22` segments.
    pub edges: Vec<ModelEdge>,
    /// Exact endpoint receipts captured at each reached edge command, parallel
    /// to `edges`. These must never be regenerated from final animation vars.
    pub edge_endpoint_snapshots: Vec<ModelEdgeEndpointSnapshot>,
    /// Failed claimed-native commands, including those with no surviving
    /// geometry. Consumers report these before buffer or GPU submission.
    pub native_edge_failures: Vec<ModelNativeEdgeFailure>,
    /// Billboard primitives.
    pub billboards: Vec<Billboard>,
    /// Inline instances of other models (op 0x0E), with resolved orientation.
    pub instances: Vec<ModelInstance>,
    /// Authored primitive, child-instance, and nested painter-group order.
    /// Group scopes include child instances; a renderer must traverse an
    /// instance at its command position before processing the following op.
    pub painter_program: Vec<ModelPainterOp>,
}

/// One command-stream line primitive (`0x02` palette or `0x22` sprite ribbon).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelEdge {
    pub vertices: [u16; 2],
    pub style: ModelEdgeStyle,
}

/// Retail presentation for one [`ModelEdge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelEdgeStyle {
    /// Opcode `0x02`: Section-7 palette hairline `[mat][v0][v1]`.
    Palette { mat: u16 },
    /// Opcode `0x22`: sprite-textured screen-space ribbon `[sprite][size][v0][v1]`.
    ///
    /// Near-pass `FUN_00459000` looks up the sprite, passes this **raw stream
    /// short** (sign-extended) to `FUN_004594C0`, and submits a `FUN_0047AA20`
    /// quad. Unlike billboard size/angle, this operand is not `FUN_00470840`
    /// packed-decoded: factory6's authored 45 would become 2048 and paint
    /// sky-high roof ribbons.
    Sprite { sprite_id: u16, size: u16 },
}

impl ModelEdge {
    /// Packed material id consumed by [`face_material`].
    pub const fn packed_material(self) -> u16 {
        match self.style {
            ModelEdgeStyle::Palette { mat } => mat & 0x7FFF,
            ModelEdgeStyle::Sprite { sprite_id, .. } => sprite_id | 0x8000,
        }
    }
}

/// Inputs shared by command interpretation, generated vertices, mount frames,
/// face anchors and linked exports. Context never changes the authored bytes.
#[derive(Clone, Copy)]
pub struct ModelMaterializationContext<'a> {
    pub vars: &'a AnimVars,
    pub linked: Option<&'a LinkedModelSlots>,
    pub vertex_resolver: Option<&'a dyn ModelVertexResolver>,
    pub view_selection: ModelViewSelection,
}

impl<'a> ModelMaterializationContext<'a> {
    pub fn intrinsic(vars: &'a AnimVars, linked: Option<&'a LinkedModelSlots>) -> Self {
        Self {
            vars,
            linked,
            vertex_resolver: None,
            view_selection: ModelViewSelection::IntrinsicAllBranches,
        }
    }
}

/// An inline instance of another model (op 0x0E, FUN_00467410): the stream
/// embeds a whole other model by GLOBAL Section 8 pool id (cumulative over
/// the preloaded system levels — the runtime array at DAT_004FE640),
/// anchored at a vertex slot. Used e.g. by the menu wrapper props
/// (`screenop` instances `screeno2` = id 0x13D).
///
/// Namespace note (verified 2026-07-03): the id IS the global pool id, not a
/// local within-OVL entry index. The resolver callback (`ctx+0x70`, invoked
/// by `FUN_004673e0`) dereferences `*(DAT_004FE640 + id*4)` — a flat table
/// indexed by global id. The tie-breaker is L5 `screenop`, whose op-0x0E
/// operand is `317`: L5's Section 8 has only ~9 local entries, so `317` is
/// only meaningful as a global id (= `screeno2`). player4's operands (42..80)
/// happen to be valid as either local or global indices, which is why an
/// earlier pass mistook them for local sibling references.
/// Native mount commands retained at the executed command occurrence. They
/// describe VIEW-frame construction; the float geometry matrix remains separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelNativeMountCommand {
    Identity,
    Parent(u16),
    Vertices {
        slots: [u16; 3],
        registers: [u16; 64],
    },
    Rotate {
        axis: u16,
        angle: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelNativeInstanceFrame {
    Parent(u16),
    Mount(Vec<ModelNativeMountCommand>),
}

#[derive(Debug, Clone)]
pub struct ModelInstance {
    /// Global Section 8 model pool id.
    pub model_id: u16,
    /// Raw attach slot ref.
    pub attach_slot: u16,
    /// Model-space attach position in raw units (divide by 100), if the
    /// slot resolves statically.
    pub attach_pos: Option<[f64; 3]>,
    /// Resolved 3×3 orientation for the child (row-major, model space).
    /// Built per `FUN_004676c0`: orientation code `& 7 == 6` copies the
    /// current mount basis established by op-0x3C or op-0x5C; otherwise it is
    /// an octahedral
    /// axis-permutation + per-axis negation of the parent basis
    /// (`FUN_00467730`). Parent basis is identity at the root, so a static
    /// materialization yields a signed permutation matrix (or the mount basis
    /// resolved from this entry's vertices).
    pub orientation: [[f64; 3]; 3],
    /// Source frame provenance, independent of float geometry composition.
    pub native_frame: Option<ModelNativeInstanceFrame>,
    /// Raw op-0x0E remap table used by child type_flag 11 records. Runtime
    /// stores a pointer to the table at child ctx+0x90, beginning at the
    /// `attach_v` word; tf-11 indexes it by record field `a`, then XORs the
    /// requested child slot's mirror bit before resolving the parent slot.
    pub linked_slots: Vec<u16>,
    /// Register-file snapshot at the point where the parent stream emitted
    /// this instance. Parent linked slots and the attachment point must be
    /// resolved against this state, not against the registers left at the end
    /// of the stream.
    pub registers: [u16; 64],
}

/// A billboard primitive (screen-space rotated quad anchored at a vertex).
///
/// Opcodes 0x68/0xE8 (flat color fill) and 0x78/0xB8/0xF8 (textured, sized to
/// the sprite's aspect ratio). Used for engine glows, lights, and particles.
#[derive(Debug, Clone, Copy)]
pub struct Billboard {
    /// Index into the entry's materialized `vertices` (anchor point).
    pub vertex: u16,
    /// Raw slot ref of the anchor.
    pub slot: u16,
    /// Color id (flat) or sprite id (textured).
    pub id: u16,
    /// Size operand (packed-operand decoded, static).
    pub size: u16,
    /// Rotation angle (0x4000 = 90°), packed-operand decoded.
    pub angle: u16,
    /// True for the textured family (0x78/0xB8/0xF8).
    pub textured: bool,
}

/// A sub-block containing multiple model entries.
#[derive(Debug, Clone)]
pub struct ModelSubBlock {
    /// Sub-block index.
    pub index: usize,
    /// Alloc size (bytes of entry data in this sub-block).
    pub alloc_size: u16,
    /// Declared entry count.
    pub entry_count: u16,
    /// Range of this sub-block's entries in `ModelCollection::all_entries`.
    pub entry_range: Range<usize>,
}

/// A single 3D model entry from a sub-block.
#[derive(Debug, Clone, Default)]
pub struct ModelEntry {
    /// Global entry index within the section (0-based).
    pub index: usize,
    /// Command stream length in words.
    pub cmd_word_count: u16,
    /// Extra data dword count.
    pub extra_count: u8,
    /// Flags byte. Bit 6: if 0, has embedded name. Bit 5: force near LOD.
    pub flags: u8,
    /// Runtime vertex slot count (2 slots per stored vertex record).
    pub slot_count: u16,
    /// Raw face_val; normal pool entries = `(face_val - 2) >> 1`.
    pub face_val: u16,
    /// LOD switch distance.
    pub radius: u16,
    /// Authored collision-sphere radius in raw 8.8 world units.
    ///
    /// The retail entity-collision path (`FUN_00412530`) selects each
    /// entity's active model, zero-extends this word from model header
    /// `+0x0A`, and compares the summed radii directly against raw entity
    /// position deltas.
    pub collision_radius_raw: u16,
    /// Raw collision-program bytes stored immediately after the 12-byte
    /// entry header (`extra_count * 4` bytes).
    ///
    /// `FUN_0046AF20` interprets this stream independently of the later
    /// render command words. It describes the model's authored collision
    /// primitives and must not be reconstructed from visible triangles.
    pub collision_program: Vec<u8>,
    /// Raw 8-byte vertex records `(type_flag, X, Y, Z)`.
    pub records: Vec<[i16; 4]>,
    /// Normal pool records `(anchor_slot, nX, nY, nZ)`. Retained so the model
    /// can be re-materialized per frame with live [`AnimVars`]. Word 0 names
    /// the authored point used by the face-plane rejection test.
    pub normal_pool: Vec<[i16; 4]>,
    /// Raw command stream words. Retained for per-frame re-interpretation
    /// ([`ModelEntry::materialize`]).
    pub cmd_words: Vec<u16>,
    /// The cached intrinsic/default evaluation reached a 0x0B/0x0C command.
    /// Static presentation must re-materialize this node with a live view.
    /// This describes the cached path, not untaken animation branches.
    pub has_view_commands: bool,
    /// Materialized model-space positions in raw units (divide by 100 for
    /// world units). Holds command-referenced slots and the projected source
    /// dependencies of type-1 midpoints, in first-reference order — or, for
    /// entries whose stream emits no faces/edges (sprite anchors, attachment
    /// points), the plain (type_flag 0) records as a point cloud.
    pub vertices: Vec<[f64; 3]>,
    /// Generator type for each materialized vertex (parallel to `vertices`).
    pub vertex_type_flags: Vec<i16>,
    /// Projector dependencies indexed into `vertices`, parallel to `vertices`.
    pub vertex_projection: Vec<ModelVertexProjection>,
    /// Semantic callback rejection, parallel to vertices; coordinates remain available.
    pub vertex_clip: Vec<ModelSlotClip>,
    /// World-surface endpoint provenance, parallel to vertices.
    pub vertex_surface_origin: Vec<ModelSurfaceOrigin>,
    /// Triangle indices into `vertices` (quads triangulated, mirror
    /// instances materialized).
    pub triangles: Vec<[u16; 3]>,
    /// Complete source polygon for each triangle, indexed into `vertices`.
    pub face_vertices: Vec<ModelFaceVertices>,
    /// Per-triangle unit normals (parallel to `triangles`), resolved from
    /// the normal pool; odd refs are X-negated mirror normals.
    pub normals: Vec<[f32; 3]>,
    /// Per-triangle authored backface planes (parallel to `triangles`).
    pub face_cull: Vec<ModelFaceCull>,
    /// Per-triangle material id (`mat` operand, parallel to `triangles`): a
    /// Section-7 colour or global Section-3 sprite texture.
    pub face_materials: Vec<u16>,
    /// Per-triangle implicit texture coordinates. Quads retain one continuous
    /// rectangular mapping across their two materialized triangles.
    pub face_uvs: Vec<[[f32; 2]; 3]>,
    /// Per-triangle, per-corner lighting normals. Gouraud commands preserve
    /// their `s0..s3` normal-pool references instead of collapsing to a flat
    /// face normal.
    pub face_corner_normals: Vec<[[f32; 3]; 3]>,
    /// Retail unlit/uniformly lit/Gouraud handler family for each triangle.
    pub face_shading: Vec<ModelFaceShading>,
    /// Legacy diagnostic layer. Canonical materialization keeps authored
    /// type-13 faces in `triangles` with their materials and face metadata;
    /// this collection is intentionally empty.
    pub shadow_triangles: Vec<[u16; 3]>,
    /// Authored `0x02`/`0x22` segments.
    pub edges: Vec<ModelEdge>,
    /// Billboard primitives.
    pub billboards: Vec<Billboard>,
    /// Inline instances of other models (op 0x0E, global pool ids).
    pub instances: Vec<ModelInstance>,
    /// Authored primitive, child-instance and painter-group order of the
    /// intrinsic materialization (see [`MaterializedModel::painter_program`]).
    pub painter_program: Vec<ModelPainterOp>,
    /// Embedded name string (e.g. "hovercraft"), if present and non-empty.
    pub name: Option<String>,
}

/// Lookup used by Section-8 collision opcode `0x05`.
///
/// Collision children use global model-pool ids, just like render opcode
/// `0x0E`, but they are authored in a separate byte stream and must be walked
/// independently.
pub trait CollisionModelPool {
    fn collision_model(&self, global_id: usize) -> Option<&ModelEntry>;
}

impl CollisionModelPool for () {
    fn collision_model(&self, _global_id: usize) -> Option<&ModelEntry> {
        None
    }
}

/// Authored-model result returned by the Section-8 sphere-query callback table.
/// Retail stores the normal as signed Q12; the port exposes its normalized
/// equivalent because projectile dispatch uses only the hit flag.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelCollisionHit {
    pub normal: [f64; 3],
    pub penetration_raw: f64,
}

/// A collision program could not be evaluated faithfully.
///
/// Callers may retain a conservative broad-sphere fallback for these cases,
/// but must not confuse that fallback with an authored miss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCollisionError {
    Truncated { pc: usize, opcode: u8 },
    UnsupportedOpcode { pc: usize, opcode: u8 },
    InvalidBranch { pc: usize },
    MissingChild { global_id: usize },
    RecursionLimit,
}

/// Root transform used while collecting the authored model-effect points
/// consumed by the Main Base / Working Factory staged-destruction callback.
///
/// Both fields use raw model/gameplay coordinates. Child-model transforms from
/// collision opcode `0x05` are composed into this transform before their
/// points are returned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StagedEffectRequest {
    /// Row-major model-to-output basis.
    pub model_to_output_basis: [[f64; 3]; 3],
    /// Root model origin in the requested output frame.
    pub model_origin_raw: [f64; 3],
}

impl Default for StagedEffectRequest {
    fn default() -> Self {
        Self {
            model_to_output_basis: IDENTITY3,
            model_origin_raw: [0.0; 3],
        }
    }
}

/// One opcode-`0x8E` candidate visited by retail `FUN_00419D90`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StagedEffectPoint {
    /// Resolved and transformed center in the request's raw output frame.
    pub center_raw: [f64; 3],
    /// Low word of the opcode's radius dword, used by retail as the random
    /// effect-scatter radius.
    pub scatter_radius_raw: u16,
    /// Raw slot operand, retained for provenance and diagnostics.
    pub source_slot: u16,
}

/// A staged-effect collision-program walk could not be reproduced faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagedEffectError {
    Truncated { pc: usize, opcode: u8 },
    UnsupportedOpcode { pc: usize, opcode: u8 },
    InvalidChildSpan { pc: usize, span: i16 },
    MissingChild { global_id: usize },
    UnresolvedSlot { pc: usize, slot: u16 },
    RecursionLimit,
}

/// Anomaly counters accumulated while interpreting command streams.
/// All-zero/empty on the shipped game data.
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    /// Command words > 0xFF (invalid; dispatch is 8-bit) → count by word.
    pub bad_words: HashMap<u16, u32>,
    /// Opcodes with no handler → count by opcode.
    pub unknown_ops: HashMap<u16, u32>,
    /// Unknown vertex type_flags → count by flag.
    pub unknown_type_flags: HashMap<i16, u32>,
    /// Streams aborted by the interpreter step guard.
    pub guard_aborts: u32,
    /// View branches stopped by truncated operands, invalid normal references,
    /// or a jump target outside the command stream.
    pub invalid_view_branches: u32,
    /// Conditional jumps taken.
    pub jumps_taken: u32,
    /// Inline model instances (op 0x0E) skipped.
    pub instances: u32,
    /// Generated-vertex resolutions by type_flag kind.
    pub generated: HashMap<&'static str, u32>,
}

impl StreamStats {
    /// True if every stream decoded cleanly (no bad words, unknown opcodes,
    /// unknown type_flags, or guard aborts).
    pub fn is_clean(&self) -> bool {
        self.bad_words.is_empty()
            && self.unknown_ops.is_empty()
            && self.unknown_type_flags.is_empty()
            && self.guard_aborts == 0
            && self.invalid_view_branches == 0
    }
}

/// All models parsed from Section 8.
#[derive(Debug)]
pub struct ModelCollection {
    /// Sub-blocks as read from the section data.
    pub sub_blocks: Vec<ModelSubBlock>,
    /// Flat list of all entries across all sub-blocks.
    pub all_entries: Vec<ModelEntry>,
    /// Aggregated interpreter anomaly counters.
    pub stats: StreamStats,
}

impl ModelCollection {
    /// Count of entries with non-empty names.
    pub fn named_count(&self) -> usize {
        self.all_entries.iter().filter(|e| e.name.is_some()).count()
    }

    /// Total materialized vertex count across all entries.
    pub fn total_vertices(&self) -> usize {
        self.all_entries.iter().map(|e| e.vertices.len()).sum()
    }

    /// Total triangle count across all entries (body + ground-shadow tris).
    pub fn total_triangles(&self) -> usize {
        self.all_entries
            .iter()
            .map(|e| e.triangles.len() + e.shadow_triangles.len())
            .sum()
    }

    /// Total ground-shadow triangle count across all entries.
    pub fn total_shadow_triangles(&self) -> usize {
        self.all_entries
            .iter()
            .map(|e| e.shadow_triangles.len())
            .sum()
    }

    /// Total edge count across all entries.
    pub fn total_edges(&self) -> usize {
        self.all_entries.iter().map(|e| e.edges.len()).sum()
    }

    /// Total billboard count across all entries.
    pub fn total_billboards(&self) -> usize {
        self.all_entries.iter().map(|e| e.billboards.len()).sum()
    }
}

// ── Operand decoding ────────────────────────────────────────────────────────

/// Packed operand decode (FUN_00470840): bit 7+6 = register file, bit 7 only
/// = dynamic callback, else rotate-left of `val & 0xFF03` by `(val >> 2) & 0xF`
/// and complement the result when bit 6 is set. With `AnimVars::default()` the
/// two dynamic branches both yield 0, matching the previous parse-time
/// behavior.
fn decode_value(f: u16, vars: &AnimVars) -> u16 {
    if f & 0x80 != 0 {
        if f & 0x40 != 0 {
            return vars.registers[(f & 0x3F) as usize];
        }
        return (vars.dynamic[(f & 0x3F) as usize] & 0xFFFF) as u16; // callback
    }
    let mut value = (f & 0xFF03).rotate_left(u32::from((f >> 2) & 0xF));
    if f & 0x40 != 0 {
        value = !value;
    }
    value
}

/// Animation-variable eval (FUN_00470700). `& 0xC0`: 0x00 = 6-bit immediate,
/// 0x40 = immediate << 10, 0x80 = dynamic callback value (`& 0xFFFF`), 0xC0 =
/// register file. With `AnimVars::default()` both dynamic sources yield 0
/// (parse-time / frame-0 behavior).
fn eval_var(v: i32, vars: &AnimVars) -> i32 {
    match v & 0xC0 {
        0x00 => v & 0x3F,
        0x40 => (v & 0x3F) << 10,
        0x80 => vars.dynamic[(v & 0x3F) as usize] & 0xFFFF,
        _ => vars.registers[(v & 0x3F) as usize] as i32, // 0xC0 register file
    }
}

/// Retail tf-8 axis interpolation (`FUN_0046F7D0` / `FUN_0046F8F0` /
/// `FUN_0046FA20`). The original deliberately drops the low interpolation
/// bit: `a + 2 * (((b - a) * eval) >> 17)`. This preserves its signed
/// arithmetic-shift bias and even-coordinate output instead of rounding a
/// floating-point lerp.
fn retail_tf8_axis(a: f64, b: f64, eval: i32) -> f64 {
    f64::from(native_spatial::lerp_axis(a as i32, b as i32, eval))
}

/// Retail tf-9 cubic Bézier evaluation (`FUN_0046FB80`) for one transformed
/// coordinate. The executable evaluates the power-basis polynomial in three
/// separately truncated fixed-point stages: the linear, quadratic and cubic
/// contributions are quantized to multiples of 2, 4 and 8 respectively.
/// Keeping those signed arithmetic shifts matters for negative control-point
/// deltas; a conventional floating-point Bézier rounds to visibly different
/// raw coordinates on small animated parts.
fn retail_tf9_axis(p0: f64, p1: f64, p2: f64, p3: f64, eval: i32) -> f64 {
    f64::from(native_spatial::bezier_axis(
        p0 as i32, p1 as i32, p2 as i32, p3 as i32, eval,
    ))
}

/// Execute one member of the `0x_D` register-op family exactly as the three
/// original command tables do (FUN_00466410..FUN_004669A0). Operands are
/// packed values decoded against the register file *before* the destination
/// is overwritten.
fn apply_register_op(op: u16, args: &[u16], vars: &mut AnimVars) {
    if args.len() < 3 {
        return;
    }
    let dst = args[0] as usize;
    if dst >= vars.registers.len() {
        return;
    }
    let a = decode_value(args[1], vars);
    let b = decode_value(args[2], vars);
    let shift = (b & 0x1f) as u32;
    let value = match op {
        0x0D => a.wrapping_add(b),
        0x1D => a.wrapping_sub(b),
        0x2D => (a as i16).wrapping_mul(b as i16) as u16,
        0x3D => {
            if b == 0 {
                a
            } else {
                a / b
            }
        }
        0x4D => a >> shift,
        0x5D => a.wrapping_shl(shift),
        0x6D => a.max(b),
        0x7D => {
            if a < b {
                a
            } else {
                b.wrapping_sub(1)
            }
        }
        0x8D => {
            if b == 0 {
                a
            } else {
                (((a as u32) << 16) / b as u32) as u16
            }
        }
        0x9D => ((a as i16) >> shift) as u16,
        0xAD => {
            let index = a.wrapping_add(b) as usize;
            vars.dynamic.get(index).copied().unwrap_or(0) as u16
        }
        0xBD => {
            if b <= a {
                a
            } else {
                0
            }
        }
        0xCD => {
            if a <= b {
                a
            } else {
                0
            }
        }
        0xDD | 0xED => {
            let angle = if op == 0xED {
                b.wrapping_add(0x4000)
            } else {
                b
            };
            crate::fixed_math::retail_model_sine_product(a, angle)
        }
        0xFD => a & b,
        _ => return,
    };
    vars.registers[dst] = value;
}

// ── Command stream interpreter ──────────────────────────────────────────────

/// Decode a packed [`ModelEntry::face_materials`] value into `(mat_id,
/// is_sprite)`. `is_sprite` = bit 15: true → `mat_id` is a global Section-3
/// sprite texture id; false → `mat_id` is a Section-7 palette colour index.
pub fn face_material(packed: u16) -> (u16, bool) {
    (packed & 0x7FFF, packed & 0x8000 != 0)
}

/// Pack a face's mat operand with the sprite-pool flag from its opcode
/// (bit 0x80 of the command word → sprite pool).
#[inline]
fn pack_mat(mat: u16, op: u16) -> u16 {
    (mat & 0x7FFF) | ((op & 0x80) << 8)
}

/// A triangle emitted by the interpreter before slot/normal resolution.
struct RawFace {
    refs: [u16; 3],
    /// Original triangle/quad corners before triangulation, in raw slot refs.
    source_vertices: ModelFaceVertices,
    normal: u16,
    material: u16,
    uvs: [[f32; 2]; 3],
    corner_normals: [u16; 3],
    shading: ModelFaceShading,
}

/// Emit a face record's triangles (quads split (0,1,2)+(0,2,3)), skipping
/// degenerates. Mirror-family opcodes draw a second instance with every
/// vertex AND normal ref XOR 1 in the same authored corner order. `mat` is
/// carried onto every emitted triangle. Texture coordinates are implicit in the primitive:
/// triangles cover one half of the sprite rectangle and quads cover the full
/// rectangle continuously. Gouraud `shade_refs` parallel `refs`; flat faces
/// pass `None` and repeat the face normal.
fn emit(
    tris: &mut Vec<RawFace>,
    refs: &[u16],
    normal: u16,
    mat: u16,
    mirror_too: bool,
    shading: ModelFaceShading,
    shade_refs: Option<&[u16]>,
) {
    fn add(
        tris: &mut Vec<RawFace>,
        refs: &[u16],
        normal: u16,
        mat: u16,
        shade_refs: &[u16],
        shading: ModelFaceShading,
        uvs: &[[f32; 2]],
    ) {
        let source_vertices = match refs {
            &[a, b, c] => ModelFaceVertices::Triangle([a, b, c]),
            &[a, b, c, d] => ModelFaceVertices::Quad([a, b, c, d]),
            _ => unreachable!("face handlers emit only triangles and quads"),
        };
        let mut push = |a: usize, b: usize, c: usize| {
            if refs[a] != refs[b] && refs[b] != refs[c] && refs[a] != refs[c] {
                tris.push(RawFace {
                    refs: [refs[a], refs[b], refs[c]],
                    source_vertices,
                    normal,
                    material: mat,
                    uvs: [uvs[a], uvs[b], uvs[c]],
                    corner_normals: [shade_refs[a], shade_refs[b], shade_refs[c]],
                    shading,
                });
            }
        };
        push(0, 1, 2);
        if refs.len() == 4 {
            push(0, 2, 3);
        }
    }

    // Section-8 triangles and quads use the retail rasterizers' implicit
    // texture corners. FUN_0047C600's triangle samples the sprite's
    // upper/right domain: (0,0), (Umax,0), (Umax,Vmax). That exact half is
    // observable on pesnthut's sprite 1365; the other half is opaque magenta
    // padding. The three quad rasterizers at FUN_0047EF10, FUN_0047F320, and
    // FUN_0047F750 all synthesize the ordinary full rectangle:
    // (0,0), (Umax,0), (Umax,Vmax), (0,Vmax).
    //
    // Presentation handedness belongs in the scene's model basis. Reversing U
    // here made directional menu props appear correct by accident, but moved
    // Klaus's asymmetric keyed wing pixels away from their authored joints.
    let base_uvs: &[[f32; 2]] = if refs.len() == 3 {
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
    } else {
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    };
    let flat_normals;
    let shades = if let Some(shades) = shade_refs {
        shades
    } else {
        flat_normals = vec![normal; refs.len()];
        flat_normals.as_slice()
    };
    add(tris, refs, normal, mat, shades, shading, base_uvs);
    if mirror_too {
        // Retail's mirror-family handlers queue A^1/B^1/C^1/D^1 in the same
        // authored order and restart the primitive's ordinary implicit UVs.
        // X-negating the referenced slots already mirrors the geometry; a
        // second corner reversal changes which keyed sprite edge meets the
        // authored symmetry plane.
        let mirrored_refs: Vec<u16> = refs.iter().map(|&r| r ^ 1).collect();
        let mirrored_shades: Vec<u16> = shades.iter().map(|&r| r ^ 1).collect();
        add(
            tris,
            &mirrored_refs,
            normal ^ 1,
            mat,
            &mirrored_shades,
            shading,
            base_uvs,
        );
    }
}

/// Raw interpreter output: faces, edges, billboards as raw slot refs.
struct RawStream {
    has_view_commands: bool,
    tris: Vec<RawFace>,
    edges: Vec<RawEdge>,
    billboards: Vec<(u16, u16, u16, u16, bool)>, // slot, id, size, angle, textured
    instances: Vec<RawInstance>,
    painter_program: Vec<ModelPainterOp>,
}

struct RawEdge {
    r1: u16,
    r2: u16,
    style: ModelEdgeStyle,
    endpoint_snapshot: ModelEdgeEndpointSnapshot,
    requires_native_endpoints: bool,
}

/// Raw op-0x0E inline instance decoded from the command stream.
struct RawInstance {
    /// Global Section 8 model pool id.
    model_id: u16,
    /// Raw attach slot ref.
    attach_slot: u16,
    /// Resolved child orientation (row-major).
    orientation: [[f64; 3]; 3],
    native_frame: Option<ModelNativeInstanceFrame>,
    /// Raw remap table at child ctx+0x90, beginning with `attach_slot`.
    linked_slots: Vec<u16>,
    /// Parent register state at this exact point in the command stream.
    registers: [u16; 64],
    /// Attachment point resolved against `registers` while walking the
    /// stream. Deferring this until the end would use the wrong register
    /// values when a later `0x_D` command overwrites them.
    attach_pos: Option<[f64; 3]>,
}

/// Build the op-0x3C mount basis from three resolved parent points
/// (`FUN_00466cd0`): with `e = normalize(b − a)`, `u = normalize((c − a) × e)`
/// and `w = e × u`, the basis is stored row-major as `[u, w, e]` (the runtime
/// writes them to `ctx+0xC4 / +0xD0 / +0xDC`, the slot code-6 instances read).
/// Returns `None` for a degenerate triangle (matching a zero-length normalize).
fn mount_basis(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[[f64; 3]; 3]> {
    fn sub(p: [f64; 3], q: [f64; 3]) -> [f64; 3] {
        [p[0] - q[0], p[1] - q[1], p[2] - q[2]]
    }
    fn cross(p: [f64; 3], q: [f64; 3]) -> [f64; 3] {
        [
            p[1] * q[2] - p[2] * q[1],
            p[2] * q[0] - p[0] * q[2],
            p[0] * q[1] - p[1] * q[0],
        ]
    }
    fn norm(v: [f64; 3]) -> Option<[f64; 3]> {
        let m = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if m > 0.0 {
            Some([v[0] / m, v[1] / m, v[2] / m])
        } else {
            None
        }
    }
    let e = norm(sub(b, a))?;
    let ca = sub(c, a);
    let u = norm(cross(ca, e))?;
    let w = cross(e, u);
    Some([u, w, e])
}

/// Octahedral reorientation of a parent basis (`FUN_00467730`): axis
/// permutation by `code & 7` plus per-source-row negation by bits
/// `0x20`/`0x10`/`0x08`. `map[dst]` gives which parent row lands in each
/// destination row; the negation bit is keyed on the *source* row.
fn permute_negate(parent: &[[f64; 3]; 3], code: u16) -> [[f64; 3]; 3] {
    let map: [usize; 3] = match code & 7 {
        1 => [2, 0, 1],
        2 => [1, 2, 0],
        3 => [2, 1, 0],
        4 => [1, 0, 2],
        5 => [0, 2, 1],
        _ => [0, 1, 2], // 0, 6, 7
    };
    let neg = [
        code & 0x20 != 0, // source row 0
        code & 0x10 != 0, // source row 1
        code & 0x08 != 0, // source row 2
    ];
    let mut out = [[0.0; 3]; 3];
    for dst in 0..3 {
        let src = map[dst];
        let sign = if neg[src] { -1.0 } else { 1.0 };
        for k in 0..3 {
            out[dst][k] = sign * parent[src][k];
        }
    }
    out
}

/// Convert the engine's three consecutively stored basis vectors (columns of
/// the child-to-parent transform) into the row-major matrix exposed by this
/// crate and consumed by the Rust renderer.
fn transpose_basis(basis: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [
        [basis[0][0], basis[1][0], basis[2][0]],
        [basis[0][1], basis[1][1], basis[2][1]],
        [basis[0][2], basis[1][2], basis[2][2]],
    ]
}

/// Apply an op-0x5C or op-0x1C axis rotation (`FUN_004671f0`) to a mount
/// basis. The original uses its 16-bit sine table; evaluating the same packed
/// angle in floating point avoids carrying fixed-point rounding into the
/// renderer while preserving the matrix and rotation direction.
fn rotate_mount_basis(basis: &mut [[f64; 3]; 3], axis: u16, angle: u16) {
    let radians = angle as f64 / 65536.0 * std::f64::consts::TAU;
    let (sin, cos) = radians.sin_cos();
    let (a, b) = match axis {
        1 => (2, 0),
        2 => (0, 1),
        _ => (1, 2),
    };
    let old_a = basis[a];
    let old_b = basis[b];
    for component in 0..3 {
        basis[b][component] = old_b[component] * cos - old_a[component] * sin;
        basis[a][component] = old_b[component] * sin + old_a[component] * cos;
    }
}

/// Walk a V2000 command stream. Operand-dependent branches (0x2B/0x2C jumps,
/// tf-8/9 morphs, billboard operands) evaluate against the context variables.
/// View branches use the explicit per-node selection policy. The dormant
/// 0x13/0x14 distance branches retain their existing static interpretation.
///
/// The resolver is used to evaluate op-0x3C mount frames during the walk (the
/// "current mount basis" that code-6 op-0x0E instances copy). Resolving mount
/// slots only memoizes into the resolver cache; it does not change the vertex
/// ordering produced later by [`resolve_stream`].
fn interpret(
    words: &[u16],
    records: &[[i16; 4]],
    normal_pool: &[[i16; 4]],
    context: ModelMaterializationContext<'_>,
    stats: &mut StreamStats,
) -> (RawStream, AnimVars) {
    let mut out = RawStream {
        has_view_commands: false,
        tris: Vec::new(),
        edges: Vec::new(),
        billboards: Vec::new(),
        instances: Vec::new(),
        painter_program: Vec::new(),
    };
    let mut variables = context.vars.clone();
    let linked = context.linked;
    let vertex_resolver = context.vertex_resolver;
    let mut painter_cache = painter::PainterSlotCache::default();
    // Current op-0x3C/0x5C mount basis, with any subsequent op-0x1C rotations.
    let mut mount: Option<[[f64; 3]; 3]> = None;
    let mut native_mount: Option<Vec<ModelNativeMountCommand>> = None;
    let n = words.len();
    let mut pc = 0usize;
    let mut guard = 0u32;

    while pc < n {
        guard += 1;
        if guard > 400_000 {
            stats.guard_aborts += 1;
            break;
        }
        let op = words[pc];
        if op > 0xFF {
            *stats.bad_words.entry(op).or_insert(0) += 1;
            break;
        }
        let primitive_start = (
            out.tris.len(),
            out.edges.len(),
            out.billboards.len(),
            out.instances.len(),
        );

        match op {
            // End of stream.
            0x00 => break,

            // Triangle [mat][norm][v0][v1][v2]; 0x_7 family adds the mirror
            // instance.
            0x03 | 0x43 | 0x83 | 0xC3 | 0x07 | 0x47 | 0x87 | 0xC7 => {
                if pc + 6 > n {
                    break;
                }
                let mirror = matches!(op, 0x07 | 0x47 | 0x87 | 0xC7);
                emit(
                    &mut out.tris,
                    &[words[pc + 3], words[pc + 4], words[pc + 5]],
                    words[pc + 2],
                    pack_mat(words[pc + 1], op),
                    mirror,
                    if op & 0x40 != 0 {
                        ModelFaceShading::FlatLit
                    } else {
                        ModelFaceShading::Flat
                    },
                    None,
                );
                pc += 6;
            }

            // Quad [mat][norm][v0..v3]; 0x_8 family adds the mirror instance.
            0x04 | 0x44 | 0x84 | 0xC4 | 0x08 | 0x48 | 0x88 | 0xC8 => {
                if pc + 7 > n {
                    break;
                }
                let mirror = matches!(op, 0x08 | 0x48 | 0x88 | 0xC8);
                emit(
                    &mut out.tris,
                    &[words[pc + 3], words[pc + 4], words[pc + 5], words[pc + 6]],
                    words[pc + 2],
                    pack_mat(words[pc + 1], op),
                    mirror,
                    if op & 0x40 != 0 {
                        ModelFaceShading::FlatLit
                    } else {
                        ModelFaceShading::Flat
                    },
                    None,
                );
                pc += 7;
            }

            // Gouraud triangle [mat][norm][v0][v1][v2][s0][s1][s2].
            0x23 | 0xA3 | 0x27 | 0xA7 => {
                if pc + 9 > n {
                    break;
                }
                let mirror = matches!(op, 0x27 | 0xA7);
                emit(
                    &mut out.tris,
                    &[words[pc + 3], words[pc + 4], words[pc + 5]],
                    words[pc + 2],
                    pack_mat(words[pc + 1], op),
                    mirror,
                    ModelFaceShading::Gouraud,
                    Some(&words[pc + 6..pc + 9]),
                );
                pc += 9;
            }

            // Gouraud quad [mat][norm][v0..v3][s0..s3].
            0x24 | 0xA4 | 0x28 | 0xA8 => {
                if pc + 11 > n {
                    break;
                }
                let mirror = matches!(op, 0x28 | 0xA8);
                emit(
                    &mut out.tris,
                    &[words[pc + 3], words[pc + 4], words[pc + 5], words[pc + 6]],
                    words[pc + 2],
                    pack_mat(words[pc + 1], op),
                    mirror,
                    ModelFaceShading::Gouraud,
                    Some(&words[pc + 7..pc + 11]),
                );
                pc += 11;
            }

            // Edge [mat][v0][v1].
            0x02 => {
                if pc + 4 > n {
                    break;
                }
                let (r1, r2) = (words[pc + 2], words[pc + 3]);
                if r1 != r2 {
                    out.edges.push(RawEdge {
                        r1,
                        r2,
                        style: ModelEdgeStyle::Palette { mat: words[pc + 1] },
                        endpoint_snapshot: ModelEdgeEndpointSnapshot::Compatibility,
                        requires_native_endpoints: false,
                    });
                }
                pc += 4;
            }

            // Sprite ribbon [sprite][size][v0][v1]. FUN_00459000 receives the
            // raw size short at param_2[1] and sign-extends it into
            // FUN_004594C0; do not run FUN_00470840 packed decode here.
            0x22 => {
                if pc + 5 > n {
                    break;
                }
                let (r1, r2) = (words[pc + 3], words[pc + 4]);
                if r1 != r2 {
                    out.edges.push(RawEdge {
                        r1,
                        r2,
                        style: ModelEdgeStyle::Sprite {
                            sprite_id: words[pc + 1],
                            size: words[pc + 2],
                        },
                        endpoint_snapshot: ModelEdgeEndpointSnapshot::Compatibility,
                        requires_native_endpoints: false,
                    });
                }
                pc += 5;
            }

            // Billboards: [slot][id][size packed][angle packed]
            // (flat FUN_00467840, textured FUN_00467c60).
            0x68 | 0xE8 | 0x78 | 0xB8 | 0xF8 => {
                if pc + 5 > n {
                    break;
                }
                let textured = matches!(op, 0x78 | 0xB8 | 0xF8);
                out.billboards.push((
                    words[pc + 1],
                    words[pc + 2],
                    decode_value(words[pc + 3], &variables),
                    decode_value(words[pc + 4], &variables),
                    textured,
                ));
                pc += 5;
            }

            // FUN_00467050/00467090 share the normal-cache flag computed by
            // FUN_0046D3F0. Select before executing any commands in the block:
            // it may contain children, ribbons, registers or painter groups.
            0x0B | 0x0C => {
                out.has_view_commands = true;
                if pc + 3 > n {
                    stats.invalid_view_branches += 1;
                    break;
                }
                let ModelViewSelection::Retail {
                    local_origin_from_camera_raw,
                } = context.view_selection
                else {
                    pc += 3;
                    continue;
                };
                let Some(backfacing) = view_selection::retail_normal_backfacing(
                    records,
                    normal_pool,
                    words[pc + 2],
                    local_origin_from_camera_raw,
                ) else {
                    stats.invalid_view_branches += 1;
                    break;
                };
                if if op == 0x0B { backfacing } else { !backfacing } {
                    // The handler receives args=pc+1 and sign-extends *args.
                    let target = (pc + 1).checked_add_signed(words[pc + 1] as i16 as isize);
                    let Some(target) = target.filter(|&target| target < n) else {
                        stats.invalid_view_branches += 1;
                        break;
                    };
                    pc = target;
                    stats.jumps_taken += 1;
                } else {
                    pc += 3;
                }
            }

            // Jump if imm32 <= view_dist; static dist = 0, imm > 0 → fall
            // through.
            0x13 => pc += 4,

            // Jump if view_dist < imm32; static dist = 0 → take the jump.
            0x14 => {
                if pc + 4 > n {
                    break;
                }
                pc = pc + 1 + words[pc + 1] as usize;
                stats.jumps_taken += 1;
            }

            // Conditional jumps on packed operands (animation frame select).
            0x2B | 0x2C => {
                if pc + 4 > n {
                    break;
                }
                let a = decode_value(words[pc + 2], &variables);
                let b = decode_value(words[pc + 3], &variables);
                let cond = if op == 0x2B { a != b } else { a == b };
                if cond {
                    pc = pc + 1 + words[pc + 1] as usize;
                    stats.jumps_taken += 1;
                } else {
                    pc += 4;
                }
            }

            // Variable ops (0x0D family, step 0x10) on the 64-entry register
            // file at ctx+0x94. Every materialization executes the original
            // handlers so following branches and generated vertices see the
            // correct values.
            0x0D | 0x1D | 0x2D | 0x3D | 0x4D | 0x5D | 0x6D | 0x7D | 0x8D | 0x9D | 0xAD | 0xBD
            | 0xCD | 0xDD | 0xED | 0xFD => {
                if pc + 4 > n {
                    break;
                }
                apply_register_op(op, &words[pc + 1..pc + 4], &mut variables);
                painter_cache.clear();
                pc += 4;
            }

            // FUN_004669E0/00466A70 open an unsorted child queue at the
            // maximum/minimum referenced camera Z. 0x38 is the optional
            // 4689D0 vertex-number diagnostic: with4FEEF4 clear it skips
            // resolution entirely. Normal materialization ignores it.
            0x06 | 0x26 | 0x38 => {
                let mut q = pc + 1;
                while q < n && words[q] != 0xFFFF {
                    q += 1;
                }
                if op != 0x38 {
                    let depth_key = painter_cache.with_resolver(
                        records,
                        &variables,
                        linked,
                        vertex_resolver,
                        |resolver| {
                            painter::extrema_key(resolver, &words[pc + 1..q], op == 0x06, stats)
                        },
                    );
                    out.painter_program.push(ModelPainterOp::BeginGroup {
                        depth_key,
                        sorting: ModelPainterSorting::Unsorted,
                        referenced_slots: words[pc + 1..q].to_vec(),
                    });
                }
                pc = q + 1;
            }

            // 0x15 calls 494AB0 (sorted children); 0x46 and 0xC6 call
            // 494B60 (unsorted children). Their outer records still carry
            // the reference depth when the enclosing queue is sorted.
            0x46 | 0x15 | 0xC6 => {
                let length = if op == 0xC6 { 4 } else { 2 };
                if pc + length > n {
                    break;
                }
                let offset_raw = if op == 0xC6 {
                    painter::signed_offset(words[pc + 2], words[pc + 3])
                } else {
                    0
                };
                let depth_key = painter_cache.with_resolver(
                    records,
                    &variables,
                    linked,
                    vertex_resolver,
                    |resolver| painter::vertex_key(resolver, words[pc + 1], offset_raw, stats),
                );
                out.painter_program.push(ModelPainterOp::BeginGroup {
                    depth_key,
                    referenced_slots: vec![words[pc + 1]],
                    sorting: if op == 0x15 {
                        ModelPainterSorting::Sorted
                    } else {
                        ModelPainterSorting::Unsorted
                    },
                });
                pc += length;
            }
            0xA6 => {
                if pc + 3 > n {
                    break;
                }
                out.painter_program.push(ModelPainterOp::BeginGroup {
                    depth_key: ModelPainterDepthKey::Fixed(painter::signed_offset(
                        words[pc + 1],
                        words[pc + 2],
                    )),
                    sorting: ModelPainterSorting::Unsorted,
                    referenced_slots: Vec::new(),
                });
                pc += 3;
            }
            0x66 | 0x86 => {
                out.painter_program.push(ModelPainterOp::BeginGroup {
                    depth_key: if op == 0x66 {
                        if let Some(owner) =
                            vertex_resolver.filter(|owner| owner.owns_native_view())
                        {
                            owner
                                .native_node_origin_view()
                                .map(|point| ModelPainterDepthKey::Native(point[2]))
                                .unwrap_or(ModelPainterDepthKey::Unresolved)
                        } else {
                            ModelPainterDepthKey::Vertex {
                                position_raw: [0.0; 3],
                                offset_raw: 0,
                            }
                        }
                    } else {
                        ModelPainterDepthKey::Fixed(-1)
                    },
                    sorting: ModelPainterSorting::Unsorted,
                    referenced_slots: Vec::new(),
                });
                pc += 1;
            }
            0xE6 => {
                out.painter_program.push(ModelPainterOp::EndGroup);
                pc += 1;
            }
            0x10 => pc += 1,
            // Mount reset (FUN_00466fc0): restore the current mount frame to
            // Q1.31 identity. The Level-1 `lifter` uses this between its
            // animated arm/weight children and the world-upright `factpole`.
            0x0F => {
                mount = Some(IDENTITY3);
                native_mount = Some(vec![ModelNativeMountCommand::Identity]);
                pc += 1;
            }
            // Additional mount rotation (FUN_004671d0): unlike op-0x5C,
            // this preserves the existing mount and rotates it in place.
            // All three retail dispatch tables use the same handler.
            0x1C => {
                if pc + 3 > n {
                    break;
                }
                let angle = decode_value(words[pc + 2], &variables);
                if let Some(basis) = mount.as_mut() {
                    rotate_mount_basis(basis, words[pc + 1], angle);
                }
                if let Some(commands) = native_mount.as_mut() {
                    commands.push(ModelNativeMountCommand::Rotate {
                        axis: words[pc + 1],
                        angle,
                    });
                }
                pc += 3;
            }
            // Mount frame (FUN_00466cd0): 3 parent slot refs → orthonormal
            // basis stored as the current mount basis for code-6 instances.
            // Operands are runtime slot refs (÷2 → record, low bit → mirror),
            // so values above the vertex count are the mirror slots; the
            // resolver decodes them the same way it decodes face refs.
            0x3C => {
                if pc + 4 > n {
                    break;
                }
                native_mount = Some(vec![ModelNativeMountCommand::Vertices {
                    slots: [words[pc + 1], words[pc + 2], words[pc + 3]],
                    registers: variables.registers,
                }]);
                let mut resolver = SlotResolver::with_linked(records, &variables, linked)
                    .with_vertex_resolver(vertex_resolver);
                let a = resolver.resolve(words[pc + 1], stats);
                let b = resolver.resolve(words[pc + 2], stats);
                let c = resolver.resolve(words[pc + 3], stats);
                if let (Some(a), Some(b), Some(c)) = (a, b, c) {
                    if let Some(basis) = mount_basis(a.position_raw, b.position_raw, c.position_raw)
                    {
                        mount = Some(basis);
                    }
                }
                pc += 4;
            }

            // Mount orientation (FUN_00466f80): copy/permute/negate the
            // model's parent basis, then optionally rotate two basis rows.
            // Materialized child matrices are local to the model, so that
            // parent basis is identity here; the renderer composes it with
            // the actual parent world transform when walking the hierarchy.
            0x5C => {
                if pc + 4 > n {
                    break;
                }
                let mut basis = permute_negate(&IDENTITY3, words[pc + 1]);
                let mut commands = vec![ModelNativeMountCommand::Parent(words[pc + 1])];
                let angle_operand = words[pc + 3];
                if angle_operand != 0 {
                    let angle = decode_value(angle_operand, &variables);
                    rotate_mount_basis(&mut basis, words[pc + 2], angle);
                    commands.push(ModelNativeMountCommand::Rotate {
                        axis: words[pc + 2],
                        angle,
                    });
                }
                mount = Some(basis);
                native_mount = Some(commands);
                pc += 4;
            }

            // Inline instance of another model (FUN_00467410):
            // [orient_code][model_id][skip_bytes][attach_v]; next cmd at
            // args + bytes. `model_id` is a GLOBAL Section 8 pool id.
            0x0E => {
                stats.instances += 1;
                if pc + 4 > n {
                    break;
                }
                let code = words[pc + 1];
                let model_id = words[pc + 2];
                let skip_words = (words[pc + 3] / 2) as usize;
                let next_pc = pc.saturating_add(1).saturating_add(skip_words).min(n);
                let attach = if pc + 5 <= n { words[pc + 4] } else { 0 };
                // Child orientation (FUN_004676c0): code&7==6 copies the mount
                // basis; otherwise permute/negate the parent basis (identity at
                // the root of a static materialization).
                let engine_basis = if code & 7 == 6 {
                    mount.unwrap_or(IDENTITY3)
                } else {
                    permute_negate(&IDENTITY3, code)
                };
                let orient = transpose_basis(engine_basis);
                let linked_slots = if pc + 4 < next_pc {
                    words[pc + 4..next_pc].to_vec()
                } else {
                    Vec::new()
                };
                let attach_pos = {
                    let mut resolver = SlotResolver::with_linked(records, &variables, linked)
                        .with_vertex_resolver(vertex_resolver);
                    resolver
                        .resolve(attach, stats)
                        .map(|slot| slot.position_raw)
                };
                out.instances.push(RawInstance {
                    model_id,
                    attach_slot: attach,
                    orientation: orient,
                    native_frame: if code & 7 == 6 {
                        native_mount.clone().map(ModelNativeInstanceFrame::Mount)
                    } else {
                        Some(ModelNativeInstanceFrame::Parent(code))
                    },
                    linked_slots,
                    registers: variables.registers,
                    attach_pos,
                });
                pc = next_pc;
            }

            _ => {
                *stats.unknown_ops.entry(op).or_insert(0) += 1;
                break;
            }
        }
        if primitive_start
            != (
                out.tris.len(),
                out.edges.len(),
                out.billboards.len(),
                out.instances.len(),
            )
        {
            painter_cache.with_resolver(records, &variables, linked, vertex_resolver, |resolver| {
                painter::record_primitives(
                    &mut out,
                    primitive_start,
                    op,
                    resolver,
                    normal_pool,
                    stats,
                );
            });
        }
    }

    (out, variables)
}

// ── Materialization ─────────────────────────────────────────────────────────

/// The pool vector `FUN_0046D3F0` dots with the model-space light: odd refs
/// negate X, and the reserved refs 0/1 (or a missing record) are zero.
fn pool_normal_raw(pool: &[[i16; 4]], nref: u16) -> [i32; 3] {
    let Some(rec) = nref
        .checked_sub(2)
        .and_then(|index| pool.get(usize::from(index >> 1)))
    else {
        return [0; 3];
    };
    let x = i32::from(rec[1]);
    [
        if nref & 1 != 0 { -x } else { x },
        i32::from(rec[2]),
        i32::from(rec[3]),
    ]
}

/// Resolve a normal pool ref to a unit normal. Odd refs are the X-negated
/// mirror pair of the even entry.
fn pool_normal(pool: &[[i16; 4]], nref: u16) -> [f32; 3] {
    // Runtime reserves refs 0/1 as an implicit shade-table-zero pair. Stored
    // pool record zero begins at refs 2/3 (`ctx+0x8C - 8 + (ref>>1)*8`).
    if nref < 2 {
        return [0.0; 3];
    }
    let idx = ((nref >> 1) - 1) as usize;
    if idx >= pool.len() {
        return [0.0; 3];
    }
    // Pool record: (anchor_slot, nX, nY, nZ).
    let rec = pool[idx];
    let mut nx = rec[1] as f32;
    let (ny, nz) = (rec[2] as f32, rec[3] as f32);
    if nref & 1 != 0 {
        nx = -nx;
    }
    let mag = (nx * nx + ny * ny + nz * nz).sqrt();
    if mag > 0.0 {
        [nx / mag, ny / mag, nz / mag]
    } else {
        // A stored zero normal also selects the current shade table's slot
        // zero in FUN_0046D3F0; do not invent a lighting direction for it.
        [0.0; 3]
    }
}

/// Resolve the exact point/normal pair used by `FUN_0046D3F0`.
///
/// Normal refs are doubled mirror slots. Their low bit negates normal X and
/// is XORed into the anchor-slot ref stored in word 0, so mirrored faces test
/// against the mirrored authored plane rather than triangle winding.
fn pool_face_cull(
    pool: &[[i16; 4]],
    nref: u16,
    resolver: &mut SlotResolver<'_>,
    stats: &mut StreamStats,
) -> ModelFaceCull {
    // Refs 0/1 are the two implicit always-visible/default-shade entries
    // initialized by FUN_0046D5A0. They have no authored source plane.
    let Some(pool_index) = (nref >> 1).checked_sub(1).map(usize::from) else {
        return ModelFaceCull::AlwaysVisible;
    };
    let Some(rec) = pool.get(pool_index).copied() else {
        return ModelFaceCull::Unresolved;
    };
    let anchor_slot = rec[0] as u16 ^ (nref & 1);
    let native_anchor_raw = resolver
        .records
        .get(usize::from(anchor_slot >> 1))
        .map(|record| {
            [
                i32::from(record[1]) * if anchor_slot & 1 == 0 { 1 } else { -1 },
                i32::from(record[2]),
                i32::from(record[3]),
            ]
        });
    let live = resolver
        .vertex_resolver
        .is_some_and(|owner| owner.owns_external_frame_slots());
    let anchor_raw = if live {
        let Some(anchor) = native_anchor_raw else {
            return ModelFaceCull::Unresolved;
        };
        anchor.map(f64::from)
    } else {
        let Some(anchor) = resolver.resolve(anchor_slot, stats) else {
            return ModelFaceCull::Unresolved;
        };
        anchor.position_raw
    };
    let mirror = if nref & 1 == 0 { 1 } else { -1 };
    let plane = ModelFaceCullPlane {
        anchor_raw,
        native_anchor_raw,
        normal_raw: [
            i32::from(rec[1]) * mirror,
            i32::from(rec[2]),
            i32::from(rec[3]),
        ],
    };
    if live {
        ModelFaceCull::LiveAdmitted(plane)
    } else {
        ModelFaceCull::Plane(plane)
    }
}

/// Resolve raw slot refs to model-space positions, building the vertex list
/// in first-reference order.
struct ResolvedGeometry {
    vertices: Vec<[f64; 3]>,
    vertex_type_flags: Vec<i16>,
    vertex_projection: Vec<ModelVertexProjection>,
    vertex_clip: Vec<ModelSlotClip>,
    vertex_surface_origin: Vec<ModelSurfaceOrigin>,
    vertex_view_raw: Vec<Option<[i32; 3]>>,
    triangles: Vec<[u16; 3]>,
    face_vertices: Vec<ModelFaceVertices>,
    normals: Vec<[f32; 3]>,
    face_cull: Vec<ModelFaceCull>,
    face_materials: Vec<u16>,
    face_uvs: Vec<[[f32; 2]; 3]>,
    face_corner_normals: Vec<[[f32; 3]; 3]>,
    face_normals_raw: Vec<[[i32; 3]; 4]>,
    face_shading: Vec<ModelFaceShading>,
    shadow_triangles: Vec<[u16; 3]>,
    edges: Vec<ModelEdge>,
    edge_endpoint_snapshots: Vec<ModelEdgeEndpointSnapshot>,
    native_edge_failures: Vec<ModelNativeEdgeFailure>,
    billboards: Vec<Billboard>,
    instances: Vec<ModelInstance>,
    painter_program: Vec<ModelPainterOp>,
}

/// Materialize referenced slots and the additional source vertices required
/// by type-1 screen midpoint projection. Keep this remap independent from
/// SlotResolver's coordinate cache: ordinary generators need only their final
/// position, whereas a projected midpoint retains both source projectors.
#[derive(Default)]
struct ResolvedVertexPool {
    remap: HashMap<u16, u16>,
    positions: Vec<[f64; 3]>,
    type_flags: Vec<i16>,
    projection: Vec<ModelVertexProjection>,
    clip: Vec<ModelSlotClip>,
    surface_origin: Vec<ModelSurfaceOrigin>,
    view_raw: Vec<Option<[i32; 3]>>,
}

impl ResolvedVertexPool {
    fn get(
        &mut self,
        slot: u16,
        resolver: &mut SlotResolver,
        stats: &mut StreamStats,
    ) -> Option<u16> {
        if let Some(&index) = self.remap.get(&slot) {
            return Some(index);
        }
        // Resolve the complete generator chain first. Missing sources and
        // cycles fail here before publishing a source-dependent vertex.
        let position = resolver.resolve(slot, stats)?;
        let record = resolver.records[(slot >> 1) as usize];
        let index = self.positions.len() as u16;
        self.positions.push(position.position_raw);
        self.clip.push(position.clip);
        self.surface_origin.push(position.surface_origin);
        self.view_raw.push(position.native_view_point);
        self.type_flags.push(record[0]);
        self.projection.push(position.world_point.map_or(
            ModelVertexProjection::Position,
            ModelVertexProjection::WorldPoint,
        ));
        if record[0] == 1 {
            let mirror = slot & 1;
            let sources = [
                self.get(record[2] as u16 ^ mirror, resolver, stats)?,
                self.get(record[3] as u16 ^ mirror, resolver, stats)?,
            ];
            self.projection[usize::from(index)] = ModelVertexProjection::ScreenMidpoint(sources);
        }
        self.remap.insert(slot, index);
        Some(index)
    }
}

fn resolve_stream(
    resolver: &mut SlotResolver,
    raw: &RawStream,
    normal_pool: &[[i16; 4]],
    stats: &mut StreamStats,
) -> ResolvedGeometry {
    let mut vertices = ResolvedVertexPool::default();

    let mut tris = Vec::new();
    let mut face_vertices = Vec::new();
    let mut normals = Vec::new();
    let mut face_cull = Vec::new();
    let mut face_materials: Vec<u16> = Vec::new();
    let mut face_uvs = Vec::new();
    let mut face_corner_normals = Vec::new();
    let mut face_normals_raw = Vec::new();
    let mut face_shading = Vec::new();
    let shadow_tris: Vec<[u16; 3]> = Vec::new();
    let mut triangle_prefix = vec![0];
    for face in &raw.tris {
        if !resolver.live_face_admitted(face.normal, normal_pool) {
            triangle_prefix.push(tris.len());
            continue;
        }
        // Resolve the entire authored primitive before accepting either quad
        // half. An unresolved fourth corner cannot leave its first triangle
        // behind with a narrower set of clipping dependencies.
        let source_vertices = match face.source_vertices {
            ModelFaceVertices::Triangle(refs) => {
                match refs.map(|r| vertices.get(r, resolver, stats)) {
                    [Some(a), Some(b), Some(c)] => Some(ModelFaceVertices::Triangle([a, b, c])),
                    _ => None,
                }
            }
            ModelFaceVertices::Quad(refs) => match refs.map(|r| vertices.get(r, resolver, stats)) {
                [Some(a), Some(b), Some(c), Some(d)] => Some(ModelFaceVertices::Quad([a, b, c, d])),
                _ => None,
            },
        };
        let Some(source_vertices) = source_vertices else {
            triangle_prefix.push(tris.len());
            continue;
        };
        let refs = face.refs;
        let m = [
            vertices.get(refs[0], resolver, stats),
            vertices.get(refs[1], resolver, stats),
            vertices.get(refs[2], resolver, stats),
        ];
        if let [Some(a), Some(b), Some(c)] = m {
            // `FUN_0045A4E0` sends every authored face through the active
            // per-corner callback and then submits the original opcode,
            // material, UVs and cull plane. Type 13 is a vertex policy, not a
            // second geometry stream; splitting all-type-13 faces discarded
            // their parallel face metadata and caused a non-retail redraw.
            tris.push([a, b, c]);
            face_vertices.push(source_vertices);
            normals.push(pool_normal(normal_pool, face.normal));
            face_cull.push(pool_face_cull(normal_pool, face.normal, resolver, stats));
            face_materials.push(face.material);
            face_uvs.push(face.uvs);
            face_corner_normals.push([
                pool_normal(normal_pool, face.corner_normals[0]),
                pool_normal(normal_pool, face.corner_normals[1]),
                pool_normal(normal_pool, face.corner_normals[2]),
            ]);
            face_normals_raw.push([
                pool_normal_raw(normal_pool, face.normal),
                pool_normal_raw(normal_pool, face.corner_normals[0]),
                pool_normal_raw(normal_pool, face.corner_normals[1]),
                pool_normal_raw(normal_pool, face.corner_normals[2]),
            ]);
            face_shading.push(face.shading);
        }
        triangle_prefix.push(tris.len());
    }

    let mut edges = Vec::new();
    let mut edge_endpoint_snapshots = Vec::new();
    let mut native_edge_failures = Vec::new();
    let mut edge_remap = Vec::new();
    for (source_edge_index, raw_edge) in raw.edges.iter().enumerate() {
        if raw_edge.requires_native_endpoints {
            if let ModelEdgeEndpointSnapshot::MissingNative { boundary } =
                raw_edge.endpoint_snapshot
            {
                // Eligibility is unowned, not an authenticated source miss.
                // Retain the failure separately and never borrow final-vars
                // geometry to visit an endpoint the command did not reach.
                native_edge_failures.push(ModelNativeEdgeFailure {
                    source_edge_index,
                    source_slots: [raw_edge.r1, raw_edge.r2],
                    boundary,
                });
                edge_remap.push(None);
                continue;
            }
        }
        if matches!(
            raw_edge.endpoint_snapshot,
            ModelEdgeEndpointSnapshot::SourceClipped { .. }
        ) {
            // The native constructor returned before queue allocation. Do not
            // resolve final-vars geometry: endpoint2 may never have been read,
            // and its H/E/provider callback cannot be replayed by remapping.
            edge_remap.push(None);
            continue;
        }
        let m = (
            vertices.get(raw_edge.r1, resolver, stats),
            vertices.get(raw_edge.r2, resolver, stats),
        );
        if let (Some(a), Some(b)) = m {
            edge_remap.push(Some(edges.len()));
            edge_endpoint_snapshots.push(raw_edge.endpoint_snapshot);
            edges.push(ModelEdge {
                vertices: [a, b],
                style: raw_edge.style,
            });
        } else {
            edge_remap.push(None);
        }
    }

    let mut billboards = Vec::new();
    let mut billboard_remap = Vec::new();
    for &(slot, id, size, angle, textured) in &raw.billboards {
        if let Some(i) = vertices.get(slot, resolver, stats) {
            billboard_remap.push(Some(billboards.len()));
            billboards.push(Billboard {
                vertex: i,
                slot,
                id,
                size,
                angle,
                textured,
            });
        } else {
            billboard_remap.push(None);
        }
    }

    // Instances resolve their attach position directly (NOT via `get`), so
    // attachment-only slots never pollute the materialized render vertex list.
    let mut instances = Vec::new();
    for inst in &raw.instances {
        instances.push(ModelInstance {
            model_id: inst.model_id,
            attach_slot: inst.attach_slot,
            attach_pos: inst.attach_pos,
            orientation: inst.orientation,
            native_frame: inst.native_frame.clone(),
            linked_slots: inst.linked_slots.clone(),
            registers: inst.registers,
        });
    }

    ResolvedGeometry {
        vertices: vertices.positions,
        vertex_type_flags: vertices.type_flags,
        vertex_projection: vertices.projection,
        vertex_clip: vertices.clip,
        vertex_surface_origin: vertices.surface_origin,
        vertex_view_raw: vertices.view_raw,
        triangles: tris,
        face_vertices,
        normals,
        face_cull,
        face_materials,
        face_uvs,
        face_corner_normals,
        face_normals_raw,
        face_shading,
        shadow_triangles: shadow_tris,
        edges,
        edge_endpoint_snapshots,
        native_edge_failures,
        billboards,
        instances,
        painter_program: painter::remap_program(
            &raw.painter_program,
            &triangle_prefix,
            &edge_remap,
            &billboard_remap,
        ),
    }
}

// ── Binary reading helpers ──────────────────────────────────────────────────

fn read_u16(data: &[u8], off: usize) -> Option<u16> {
    data.get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn read_i16(data: &[u8], off: usize) -> Option<i16> {
    data.get(off..off + 2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
}

// ── Entry parsing ───────────────────────────────────────────────────────────

struct EntryHeader {
    cmd_words: u16,
    extra: u8,
    flags: u8,
    slots: u16,
    face_val: u16,
    radius: u16,
    collision_radius_raw: u16,
    /// Geometry byte size from the loader stride formula (FUN_004ab5b0).
    geom: usize,
}

fn read_header(data: &[u8], off: usize) -> Option<EntryHeader> {
    if off + HEADER_SIZE > data.len() {
        return None;
    }
    let cmd_words = read_u16(data, off)?;
    let extra = data[off + 2];
    let flags = data[off + 3];
    let slots = read_u16(data, off + 4)?;
    let face_val = read_u16(data, off + 6)?;
    let radius = read_u16(data, off + 8)?;
    let collision_radius_raw = read_u16(data, off + 10)?;

    let vh = (slots >> 1) as i64;
    let fh = ((face_val as i64 - 2) >> 1).max(0);
    let geom = (cmd_words as i64 + 6 + (extra as i64 + (vh + fh) * 2) * 2) * 2;

    Some(EntryHeader {
        cmd_words,
        extra,
        flags,
        slots,
        face_val,
        radius,
        collision_radius_raw,
        geom: geom as usize,
    })
}

/// Read vertex records, normal pool, and command words for one entry.
type RawGeometry = (Vec<u8>, Vec<[i16; 4]>, Vec<[i16; 4]>, Vec<u16>);

fn read_geometry(data: &[u8], off: usize, hdr: &EntryHeader) -> RawGeometry {
    let vh = (hdr.slots >> 1) as usize;
    // Signed on purpose: face_val < 2 shifts cmd_off backwards exactly like
    // the runtime's pointer math.
    let fh = (hdr.face_val as i64 - 2) >> 1;
    let vert_off = off as i64 + HEADER_SIZE as i64 + hdr.extra as i64 * 4;
    let norm_off = vert_off + vh as i64 * 8;
    let cmd_off = norm_off + fh * 8;

    let collision_start = off + HEADER_SIZE;
    let collision_end = collision_start + usize::from(hdr.extra) * 4;
    let collision_program = data
        .get(collision_start..collision_end)
        .unwrap_or_default()
        .to_vec();

    let mut records = Vec::with_capacity(vh);
    for i in 0..vh {
        let p = (vert_off + i as i64 * 8) as usize;
        match (
            read_i16(data, p),
            read_i16(data, p + 2),
            read_i16(data, p + 4),
            read_i16(data, p + 6),
        ) {
            (Some(tf), Some(x), Some(y), Some(z)) => records.push([tf, x, y, z]),
            _ => break,
        }
    }

    let mut pool = Vec::new();
    for i in 0..fh.max(0) {
        let p = (norm_off + i * 8) as usize;
        match (
            read_i16(data, p),
            read_i16(data, p + 2),
            read_i16(data, p + 4),
            read_i16(data, p + 6),
        ) {
            (Some(f), Some(nx), Some(ny), Some(nz)) => pool.push([f, nx, ny, nz]),
            _ => break,
        }
    }

    let mut words = Vec::with_capacity(hdr.cmd_words as usize);
    for i in 0..hdr.cmd_words as usize {
        match read_u16(data, (cmd_off + i as i64 * 2) as usize) {
            Some(w) => words.push(w),
            None => break,
        }
    }

    (collision_program, records, pool, words)
}

// ── Section parsing ─────────────────────────────────────────────────────────

/// Parse V2000 Section 8 model sub-blocks.
///
/// The section is a chain of sub-blocks `[u16 alloc][u16 count][alloc bytes
/// of entries]`. The FIRST sub-block's header is the section's
/// `header_value` dword (low 16 = alloc, high 16 = count); `data` starts at
/// its entry data.
pub fn parse_model_subblocks(data: &[u8], header_value: u32) -> Result<ModelCollection> {
    let mut alloc = (header_value & 0xFFFF) as usize;
    let mut count = (header_value >> 16) as usize;

    if alloc == 0 || alloc > data.len() {
        return Err(V2kError::section(8, "invalid header_value for section 8"));
    }

    let mut sub_blocks = Vec::new();
    let mut all_entries: Vec<ModelEntry> = Vec::new();
    let mut stats = StreamStats::default();
    let mut base = 0usize;
    let mut sb_index = 0usize;

    loop {
        let range_start = all_entries.len();
        let end = (base + alloc).min(data.len());
        let mut p = base;

        for _ in 0..count {
            if p + HEADER_SIZE > end {
                break;
            }
            let hdr = match read_header(data, p) {
                Some(h) => h,
                None => break,
            };
            if p + hdr.geom > end + 3 {
                break;
            }

            // Embedded name (flags bit 6 clear), then 4-byte alignment.
            let mut name = None;
            let mut q = p + hdr.geom;
            if hdr.flags & 0x40 == 0 {
                let scan_end = (q + 64).min(end);
                let z = match data
                    .get(q..scan_end)
                    .and_then(|s| s.iter().position(|&b| b == 0))
                {
                    Some(z) => q + z,
                    None => break,
                };
                let bytes = &data[q..z];
                if !bytes.is_empty() && bytes.is_ascii() {
                    name = Some(String::from_utf8_lossy(bytes).into_owned());
                }
                q = z + 1;
            }
            q = (q + 3) & !3;

            let (collision_program, records, pool, words) = read_geometry(data, p, &hdr);

            // Build the entry with the raw geometry, then materialize its
            // animation-frame-0 geometry through the same command interpreter
            // used at runtime. Register opcodes execute here as they do in the
            // original; retired extractor output is not a compatibility target.
            let mut entry = ModelEntry {
                index: all_entries.len(),
                cmd_word_count: hdr.cmd_words,
                extra_count: hdr.extra,
                flags: hdr.flags,
                slot_count: hdr.slots,
                face_val: hdr.face_val,
                radius: hdr.radius,
                collision_radius_raw: hdr.collision_radius_raw,
                collision_program,
                records,
                normal_pool: pool,
                cmd_words: words,
                has_view_commands: false,
                vertices: Vec::new(),
                vertex_type_flags: Vec::new(),
                vertex_projection: Vec::new(),
                vertex_clip: Vec::new(),
                vertex_surface_origin: Vec::new(),
                triangles: Vec::new(),
                face_vertices: Vec::new(),
                normals: Vec::new(),
                face_cull: Vec::new(),
                face_materials: Vec::new(),
                face_uvs: Vec::new(),
                face_corner_normals: Vec::new(),
                face_shading: Vec::new(),
                shadow_triangles: Vec::new(),
                edges: Vec::new(),
                billboards: Vec::new(),
                instances: Vec::new(),
                painter_program: Vec::new(),
                name,
            };
            let m = entry.materialize_into(&AnimVars::default(), &mut stats);
            entry.has_view_commands = m.has_view_commands;
            entry.vertices = m.vertices;
            entry.vertex_type_flags = m.vertex_type_flags;
            entry.vertex_projection = m.vertex_projection;
            entry.vertex_clip = m.vertex_clip;
            entry.vertex_surface_origin = m.vertex_surface_origin;
            entry.triangles = m.triangles;
            entry.face_vertices = m.face_vertices;
            entry.normals = m.normals;
            entry.face_cull = m.face_cull;
            entry.face_materials = m.face_materials;
            entry.face_uvs = m.face_uvs;
            entry.face_corner_normals = m.face_corner_normals;
            entry.face_shading = m.face_shading;
            entry.shadow_triangles = m.shadow_triangles;
            entry.edges = m.edges;
            entry.billboards = m.billboards;
            entry.instances = m.instances;
            entry.painter_program = m.painter_program;
            all_entries.push(entry);

            if q <= p {
                break;
            }
            p = q;
        }

        sub_blocks.push(ModelSubBlock {
            index: sb_index,
            alloc_size: alloc as u16,
            entry_count: count as u16,
            entry_range: range_start..all_entries.len(),
        });
        sb_index += 1;

        // Next sub-block header.
        base += alloc;
        if base + 4 > data.len() {
            break;
        }
        alloc = read_u16(data, base).unwrap_or(0) as usize;
        count = read_u16(data, base + 2).unwrap_or(0) as usize;
        base += 4;
        if alloc == 0 || count == 0 {
            break;
        }
    }

    if all_entries.is_empty() {
        return Err(V2kError::section(8, "no valid model entries found"));
    }

    Ok(ModelCollection {
        sub_blocks,
        all_entries,
        stats,
    })
}

/// Parse Section 8 models from an OVL section.
/// `header_value` is the uint32 value field from the section header.
pub fn parse_models_section(section_data: &[u8], header_value: u32) -> Result<ModelCollection> {
    parse_model_subblocks(section_data, header_value)
}

// ── Authored collision-program interpreter ────────────────────────────────

const COLLISION_RECURSION_LIMIT: u8 = 8;

struct StagedEffectInterpreter<'a, P: CollisionModelPool + ?Sized> {
    pool: &'a P,
}

impl<P: CollisionModelPool + ?Sized> StagedEffectInterpreter<'_, P> {
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        model: &ModelEntry,
        model_to_output_basis: [[f64; 3]; 3],
        model_origin: [f64; 3],
        vars: &AnimVars,
        linked: Option<&LinkedModelSlots>,
        depth: u8,
        output: &mut Vec<StagedEffectPoint>,
    ) -> std::result::Result<(), StagedEffectError> {
        if depth > COLLISION_RECURSION_LIMIT {
            return Err(StagedEffectError::RecursionLimit);
        }
        if model.collision_program.is_empty() {
            return Ok(());
        }

        let program = &model.collision_program;
        let mut pc = 0usize;
        while pc < program.len() {
            let opcode = program[pc];
            match opcode {
                0x88 => return Ok(()),

                // Effect point: align4(pc+4), [u32 scatter radius][i16 slot].
                // FUN_00419D90 consumes only the radius dword's low word.
                0x8E => {
                    let payload = collision_align4(pc);
                    let radius = collision_u32(program, payload)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    let slot = collision_u16(program, payload + 4)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    let next = payload + 6;
                    if next > program.len() {
                        return Err(StagedEffectError::Truncated { pc, opcode });
                    }
                    let center = resolve_collision_slot(model, slot, vars, linked)
                        .ok_or(StagedEffectError::UnresolvedSlot { pc, slot })?;
                    output.push(StagedEffectPoint {
                        center_raw: transform_collision_point(
                            model_to_output_basis,
                            model_origin,
                            center,
                        ),
                        scatter_radius_raw: radius as u16,
                        source_slot: slot,
                    });
                    pc = next;
                }

                // FUN_00419D90 advances over boxes but does not emit from
                // them. Validate the complete payload so malformed streams do
                // not accidentally desynchronize the following opcode.
                0x8F => {
                    let payload = collision_align4(pc);
                    let Some(next) = payload.checked_add(14) else {
                        return Err(StagedEffectError::Truncated { pc, opcode });
                    };
                    if next > program.len() {
                        return Err(StagedEffectError::Truncated { pc, opcode });
                    }
                    pc = next;
                }

                // Unlike collision queries, staged destruction walks both a
                // gate sphere and its detail spheres. The signed branch word
                // is therefore skipped linearly and never interpreted.
                0x8B | 0x8C | 0x95 => {
                    let payload = collision_align2(pc);
                    collision_i16(program, payload)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    pc = payload + 2;
                }

                //19D90 is a linear effect walk, not the sphere-query
                // interpreter. Convex faces and control records only advance
                // its cursor; their slot references are never materialized.
                0x36 | 0x89 | 0x8A | 0x90 => {
                    let (payload, size) = match opcode {
                        0x36 | 0x89 => (collision_align2(pc), 8),
                        0x8A => (collision_align2(pc), 0),
                        0x90 => (collision_align4(pc), 10),
                        _ => unreachable!(),
                    };
                    let next = payload + size;
                    if next > program.len() {
                        return Err(StagedEffectError::Truncated { pc, opcode });
                    }
                    pc = next;
                }

                // Collision-only child. Its payload order and byte-sized span
                // differ from render opcode 0x0E.
                0x05 => {
                    let payload = collision_align2(pc);
                    let child_id = collision_u16(program, payload)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?
                        as usize;
                    let orientation_code = collision_u16(program, payload + 2)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    let span = collision_i16(program, payload + 4)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    let attach_slot = collision_u16(program, payload + 6)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    if span < 8 {
                        return Err(StagedEffectError::InvalidChildSpan { pc, span });
                    }
                    let next = payload
                        .checked_add(span as usize)
                        .ok_or(StagedEffectError::Truncated { pc, opcode })?;
                    if next > program.len() {
                        return Err(StagedEffectError::Truncated { pc, opcode });
                    }
                    // Code 6 copies mount-basis state established by an
                    // earlier transform command. That state is outside this
                    // focused collision-program subset.
                    if orientation_code & 7 == 6 {
                        return Err(StagedEffectError::UnsupportedOpcode { pc, opcode });
                    }
                    let child = self.pool.collision_model(child_id).ok_or(
                        StagedEffectError::MissingChild {
                            global_id: child_id,
                        },
                    )?;
                    let attach_pos = resolve_collision_slot(model, attach_slot, vars, linked)
                        .ok_or(StagedEffectError::UnresolvedSlot {
                            pc,
                            slot: attach_slot,
                        })?;
                    let child_local_basis =
                        transpose_basis(permute_negate(&IDENTITY3, orientation_code));
                    let child_origin =
                        transform_collision_point(model_to_output_basis, model_origin, attach_pos);
                    let child_basis = mat3_mul_f64(model_to_output_basis, child_local_basis);
                    let remap_slots = (payload + 6..next)
                        .step_by(2)
                        .filter_map(|offset| collision_u16(program, offset))
                        .collect::<Vec<_>>();
                    let child_linked = build_collision_child_links(
                        model,
                        linked,
                        &remap_slots,
                        vars,
                        attach_pos,
                        child_local_basis,
                    );
                    self.run(
                        child,
                        child_basis,
                        child_origin,
                        vars,
                        Some(&child_linked),
                        depth + 1,
                        output,
                    )?;
                    pc = next;
                }

                // These source cases mutate the current mount frame. Keep
                // them explicit until the staged interpreter owns that frame.
                0x30 | 0x31 => return Err(StagedEffectError::UnsupportedOpcode { pc, opcode }),
                // Source initializes next=opcode+1 before its switch. This
                // includes the one-byte8D convex terminator and inert bytes;
                // importing collision-query branch semantics here is wrong.
                _ => pc += 1,
            }
        }

        Err(StagedEffectError::Truncated {
            pc,
            opcode: program.get(pc).copied().unwrap_or(0),
        })
    }
}

struct CollisionInterpreter<'a, P: CollisionModelPool + ?Sized> {
    pool: &'a P,
}

impl<P: CollisionModelPool + ?Sized> CollisionInterpreter<'_, P> {
    #[allow(clippy::too_many_arguments)]
    fn run(
        &self,
        model: &ModelEntry,
        query_center: [f64; 3],
        query_radius: f64,
        model_to_query_basis: [[f64; 3]; 3],
        model_origin: [f64; 3],
        vars: &AnimVars,
        linked: Option<&LinkedModelSlots>,
        depth: u8,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        if depth > COLLISION_RECURSION_LIMIT {
            return Err(ModelCollisionError::RecursionLimit);
        }
        if model.collision_program.is_empty() {
            return Ok(None);
        }

        let program = &model.collision_program;
        let mut pc = 0usize;
        let mut guard = 0usize;
        let mut scratch: Option<ModelCollisionHit> = None;
        let mut best: Option<ModelCollisionHit> = None;
        // FUN_0046AF20 keeps the callback's acceptance bit separate from its
        // depth/normal: 0x8C clears only the latter, and an empty 0x8D group
        // inherits the last successful primitive, even across a later miss.
        let mut scratch_hit = false;
        let mut previous_hit = None;
        let mut group_end = None;
        let mut group_active = false;

        while pc < program.len() {
            guard += 1;
            if guard > program.len().saturating_mul(4).max(16) {
                return Err(ModelCollisionError::InvalidBranch { pc });
            }
            let opcode = program[pc];
            match opcode {
                // End of collision program. Retail requires at least one raw
                // unit of final penetration. A tangent primitive remains in
                // `scratch` so it can pass a following 0x95 gate, but is not a
                // standalone final hit.
                0x88 => {
                    return Ok(best.filter(|hit| hit.penetration_raw >= 1.0));
                }

                // Convex group: the signed span is relative to the aligned
                // payload. Any rejected plane jumps directly to that end.
                0x8C => {
                    let payload = collision_align2(pc);
                    let span = collision_i16(program, payload)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    let target = payload as isize + span as isize;
                    if target < 0 || target as usize >= program.len() {
                        return Err(ModelCollisionError::InvalidBranch { pc });
                    }
                    group_end = Some(target as usize);
                    group_active = true;
                    scratch = None;
                    pc = payload + 2;
                }

                // The sphere-query table at 004D4920 selects 0046ABF0 for
                // 0x8A. Its 0x89 wrapper (0046A8B0) uses the same first three
                // slots and merely consumes one additional, unused word.
                0x89 | 0x8A => {
                    let payload = collision_align2(pc);
                    let next = payload + if opcode == 0x89 { 8 } else { 6 };
                    if next > program.len() {
                        return Err(ModelCollisionError::Truncated { pc, opcode });
                    }
                    let end = group_end.ok_or(ModelCollisionError::InvalidBranch { pc })?;
                    let points = std::array::from_fn::<_, 3, _>(|index| {
                        let slot = collision_u16(program, payload + index * 2)?;
                        resolve_collision_slot(model, slot, vars, linked).map(|point| {
                            transform_collision_point(model_to_query_basis, model_origin, point)
                        })
                    });
                    let hit = match points {
                        [Some(a), Some(b), Some(c)] => {
                            collision_plane_hit(query_center, query_radius, [a, b, c])
                        }
                        _ => None,
                    };
                    scratch_hit = hit.is_some();
                    if let Some(hit) = hit {
                        if scratch
                            .is_none_or(|current| hit.penetration_raw < current.penetration_raw)
                        {
                            scratch = Some(hit);
                        }
                        pc = next;
                    } else {
                        group_active = false;
                        pc = end;
                    }
                }

                // This group guard consumes a word but invokes no callback.
                0x8B => {
                    let payload = collision_align2(pc);
                    collision_u16(program, payload)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    if scratch_hit {
                        pc = payload + 2;
                    } else {
                        group_active = false;
                        pc = group_end.ok_or(ModelCollisionError::InvalidBranch { pc })?;
                    }
                }

                // Plane callbacks minimize penetration within a group; the
                // completed group competes with other primitives by maximum.
                0x8D => {
                    let next = pc + 1;
                    if group_active {
                        scratch_hit = true;
                        scratch = scratch.or(previous_hit);
                        previous_hit = scratch;
                        promote_collision_hit(program, next, scratch, &mut best);
                    }
                    pc = next;
                }

                // Sphere: align4(pc+4), [u32 radius][i16 center slot].
                0x8E => {
                    let payload = collision_align4(pc);
                    let radius = collision_u32(program, payload)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    let slot = collision_u16(program, payload + 4)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    let next = payload + 6;
                    if next > program.len() {
                        return Err(ModelCollisionError::Truncated { pc, opcode });
                    }
                    scratch =
                        resolve_collision_slot(model, slot, vars, linked).and_then(|center| {
                            collision_sphere_hit(
                                query_center,
                                query_radius,
                                transform_collision_point(
                                    model_to_query_basis,
                                    model_origin,
                                    center,
                                ),
                                radius as f64,
                            )
                        });
                    scratch_hit = scratch.is_some();
                    if scratch_hit {
                        previous_hit = scratch;
                    }
                    promote_collision_hit(program, next, scratch, &mut best);
                    pc = next;
                }

                // Axis-aligned box: align4(pc+4), three u32 half extents and
                // one i16 center slot.
                0x8F => {
                    let payload = collision_align4(pc);
                    let half = [
                        collision_u32(program, payload),
                        collision_u32(program, payload + 4),
                        collision_u32(program, payload + 8),
                    ];
                    let slot = collision_u16(program, payload + 12);
                    let (Some(hx), Some(hy), Some(hz), Some(slot)) =
                        (half[0], half[1], half[2], slot)
                    else {
                        return Err(ModelCollisionError::Truncated { pc, opcode });
                    };
                    let next = payload + 14;
                    if next > program.len() {
                        return Err(ModelCollisionError::Truncated { pc, opcode });
                    }
                    scratch =
                        resolve_collision_slot(model, slot, vars, linked).and_then(|center| {
                            collision_aabb_hit(
                                query_center,
                                query_radius,
                                transform_collision_point(
                                    model_to_query_basis,
                                    model_origin,
                                    center,
                                ),
                                [hx as f64, hy as f64, hz as f64],
                            )
                        });
                    scratch_hit = scratch.is_some();
                    if scratch_hit {
                        previous_hit = scratch;
                    }
                    promote_collision_hit(program, next, scratch, &mut best);
                    pc = next;
                }

                // Conditional gate: continue after the signed offset word if
                // the previous primitive hit; otherwise branch from the
                // aligned payload base. Broad gate primitives are deliberately
                // not promoted when immediately followed by 0x95.
                0x95 => {
                    let payload = collision_align2(pc);
                    let offset = collision_i16(program, payload)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    if scratch_hit {
                        pc = payload + 2;
                    } else {
                        let Some(target) = (payload as isize).checked_add(offset as isize) else {
                            return Err(ModelCollisionError::InvalidBranch { pc });
                        };
                        if target < 0 || target as usize >= program.len() {
                            return Err(ModelCollisionError::InvalidBranch { pc });
                        }
                        pc = target as usize;
                    }
                }

                // Collision-only child model. Payload order differs from the
                // render stream's 0x0E instance record.
                0x05 => {
                    let payload = collision_align2(pc);
                    let child_id = collision_u16(program, payload)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?
                        as usize;
                    let orientation_code = collision_u16(program, payload + 2)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    let span = collision_i16(program, payload + 4)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    let attach_slot = collision_u16(program, payload + 6)
                        .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                    if span < 8 {
                        return Err(ModelCollisionError::InvalidBranch { pc });
                    }
                    let next = payload.saturating_add(span as usize);
                    if next > program.len() {
                        return Err(ModelCollisionError::Truncated { pc, opcode });
                    }
                    // Code 6 depends on an earlier 0x31 mount-basis command;
                    // that rarer transform state is intentionally explicit
                    // rather than silently treated as identity.
                    if orientation_code & 7 == 6 {
                        return Err(ModelCollisionError::UnsupportedOpcode { pc, opcode });
                    }
                    let child = self.pool.collision_model(child_id).ok_or(
                        ModelCollisionError::MissingChild {
                            global_id: child_id,
                        },
                    )?;
                    let Some(attach_pos) = resolve_collision_slot(model, attach_slot, vars, linked)
                    else {
                        scratch = None;
                        scratch_hit = false;
                        pc = next;
                        continue;
                    };
                    let child_local_basis =
                        transpose_basis(permute_negate(&IDENTITY3, orientation_code));
                    let child_origin =
                        transform_collision_point(model_to_query_basis, model_origin, attach_pos);
                    let child_basis = mat3_mul_f64(model_to_query_basis, child_local_basis);
                    let remap_slots = (payload + 6..next)
                        .step_by(2)
                        .filter_map(|offset| collision_u16(program, offset))
                        .collect::<Vec<_>>();
                    let child_linked = build_collision_child_links(
                        model,
                        linked,
                        &remap_slots,
                        vars,
                        attach_pos,
                        child_local_basis,
                    );
                    scratch = self.run(
                        child,
                        query_center,
                        query_radius,
                        child_basis,
                        child_origin,
                        vars,
                        Some(&child_linked),
                        depth + 1,
                    )?;
                    scratch_hit = scratch.is_some();
                    if scratch_hit {
                        previous_hit = scratch;
                    }
                    promote_collision_hit(program, next, scratch, &mut best);
                    pc = next;
                }

                _ => return Err(ModelCollisionError::UnsupportedOpcode { pc, opcode }),
            }
        }

        Err(ModelCollisionError::Truncated {
            pc,
            opcode: program.get(pc).copied().unwrap_or(0),
        })
    }
}

/// Retail's Section-10 contact path runs the moving model's collision program
/// first. Each authored sphere then becomes a query against the static model's
/// complete collision program (`FUN_0046AE40` -> the `0x00469B70` callback ->
/// nested `FUN_0046AF20`). Type 46's active `player4` program uses only this
/// sphere/gate subset, while the static side may contain spheres, boxes, and
/// collision-only children. Query-side collision children preserve that same
/// selected sphere callback through their attachment/orientation/link context.
struct ModelPairCollisionInterpreter<'a, P: CollisionModelPool + ?Sized> {
    target: &'a ModelEntry,
    target_vars: &'a AnimVars,
    pool: &'a P,
}

impl<P: CollisionModelPool + ?Sized> ModelPairCollisionInterpreter<'_, P> {
    fn run(
        &self,
        query: &ModelEntry,
        query_origin_in_target: [f64; 3],
        query_to_target_basis: [[f64; 3]; 3],
        query_vars: &AnimVars,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        if self.target.collision_radius_raw == 0 {
            return Ok(None);
        }
        if self.target.collision_program.is_empty() {
            return Ok(None);
        }

        run_query_sphere_program(
            query,
            query_origin_in_target,
            query_to_target_basis,
            query_vars,
            self.pool,
            |sphere_center, sphere_radius| {
                CollisionInterpreter { pool: self.pool }.run(
                    self.target,
                    sphere_center,
                    sphere_radius,
                    IDENTITY3,
                    [0.0; 3],
                    self.target_vars,
                    None,
                    0,
                )
            },
        )
    }
}

/// Walk the authored query spheres/gates and collision-only children, retaining
/// the selected primitive callback through each child. `46AF20 ->46B6D0` copies
/// the callback context before recursing; model and heightfield contacts share
/// this traversal, independently of the visible render hierarchy.
fn run_query_sphere_program<P, F>(
    query: &ModelEntry,
    query_origin: [f64; 3],
    query_to_target_basis: [[f64; 3]; 3],
    query_vars: &AnimVars,
    pool: &P,
    mut collide_sphere: F,
) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError>
where
    P: CollisionModelPool + ?Sized,
    F: FnMut([f64; 3], f64) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError>,
{
    run_query_sphere_program_linked(
        query,
        query_origin,
        query_to_target_basis,
        query_vars,
        None,
        pool,
        &mut collide_sphere,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_query_sphere_program_linked<P, F>(
    query: &ModelEntry,
    query_origin: [f64; 3],
    query_to_target_basis: [[f64; 3]; 3],
    query_vars: &AnimVars,
    linked: Option<&LinkedModelSlots>,
    pool: &P,
    collide_sphere: &mut F,
    depth: u8,
) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError>
where
    P: CollisionModelPool + ?Sized,
    F: FnMut([f64; 3], f64) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError>,
{
    if depth > COLLISION_RECURSION_LIMIT {
        return Err(ModelCollisionError::RecursionLimit);
    }
    if query.collision_radius_raw == 0 || query.collision_program.is_empty() {
        return Ok(None);
    }

    let program = &query.collision_program;
    let mut pc = 0usize;
    let mut guard = 0usize;
    let mut scratch: Option<ModelCollisionHit> = None;
    let mut best: Option<ModelCollisionHit> = None;

    while pc < program.len() {
        guard += 1;
        if guard > program.len().saturating_mul(4).max(16) {
            return Err(ModelCollisionError::InvalidBranch { pc });
        }

        let opcode = program[pc];
        match opcode {
            0x88 => return Ok(best.filter(|hit| hit.penetration_raw >= 1.0)),
            0x8E => {
                let payload = collision_align4(pc);
                let radius = collision_u32(program, payload)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                let slot = collision_u16(program, payload + 4)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                let next = payload + 6;
                if next > program.len() {
                    return Err(ModelCollisionError::Truncated { pc, opcode });
                }

                scratch = match resolve_collision_slot(query, slot, query_vars, linked) {
                    Some(center) => {
                        let sphere_center =
                            transform_collision_point(query_to_target_basis, query_origin, center);
                        collide_sphere(sphere_center, f64::from(radius))?
                    }
                    None => None,
                };
                promote_collision_hit(program, next, scratch, &mut best);
                pc = next;
            }
            0x95 => {
                let payload = collision_align2(pc);
                let offset = collision_i16(program, payload)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                if scratch.is_some() {
                    pc = payload + 2;
                } else {
                    let Some(target) = (payload as isize).checked_add(offset as isize) else {
                        return Err(ModelCollisionError::InvalidBranch { pc });
                    };
                    if target < 0 || target as usize >= program.len() {
                        return Err(ModelCollisionError::InvalidBranch { pc });
                    }
                    pc = target as usize;
                }
            }
            0x05 => {
                let payload = collision_align2(pc);
                let child_id = collision_u16(program, payload)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?
                    as usize;
                let orientation_code = collision_u16(program, payload + 2)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                let span = collision_i16(program, payload + 4)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                let attach_slot = collision_u16(program, payload + 6)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                if span < 8 {
                    return Err(ModelCollisionError::InvalidBranch { pc });
                }
                let next = payload
                    .checked_add(span as usize)
                    .ok_or(ModelCollisionError::Truncated { pc, opcode })?;
                if next > program.len() {
                    return Err(ModelCollisionError::Truncated { pc, opcode });
                }
                // Native code6 copies the mount basis from prior transform31.
                // Preserve the same explicit boundary as the target walker.
                if orientation_code & 7 == 6 {
                    return Err(ModelCollisionError::UnsupportedOpcode { pc, opcode });
                }
                let child =
                    pool.collision_model(child_id)
                        .ok_or(ModelCollisionError::MissingChild {
                            global_id: child_id,
                        })?;
                let Some(attach_pos) =
                    resolve_collision_slot(query, attach_slot, query_vars, linked)
                else {
                    scratch = None;
                    pc = next;
                    continue;
                };
                let child_local_basis =
                    transpose_basis(permute_negate(&IDENTITY3, orientation_code));
                let child_origin =
                    transform_collision_point(query_to_target_basis, query_origin, attach_pos);
                let child_basis = mat3_mul_f64(query_to_target_basis, child_local_basis);
                let remap_slots = (payload + 6..next)
                    .step_by(2)
                    .filter_map(|offset| collision_u16(program, offset))
                    .collect::<Vec<_>>();
                let child_linked = build_collision_child_links(
                    query,
                    linked,
                    &remap_slots,
                    query_vars,
                    attach_pos,
                    child_local_basis,
                );
                // A child replaces the last callback result, including a miss.
                // Existing best penetration survives; a following95 gates on
                // this child result rather than a successful older sibling.
                scratch = run_query_sphere_program_linked(
                    child,
                    child_origin,
                    child_basis,
                    query_vars,
                    Some(&child_linked),
                    pool,
                    collide_sphere,
                    depth + 1,
                )?;
                promote_collision_hit(program, next, scratch, &mut best);
                pc = next;
            }
            _ => return Err(ModelCollisionError::UnsupportedOpcode { pc, opcode }),
        }
    }

    Err(ModelCollisionError::Truncated {
        pc,
        opcode: program.get(pc).copied().unwrap_or(0),
    })
}

/// The retail interpreter aligns by adding the largest possible record-header
/// pad and then rounding down, not by rounding the opcode itself up.
fn collision_align2(pc: usize) -> usize {
    (pc + 2) & !1
}

fn collision_align4(pc: usize) -> usize {
    (pc + 4) & !3
}

fn collision_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let b = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

fn collision_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    collision_u16(bytes, offset).map(|value| value as i16)
}

fn collision_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let b = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn promote_collision_hit(
    program: &[u8],
    next: usize,
    scratch: Option<ModelCollisionHit>,
    best: &mut Option<ModelCollisionHit>,
) {
    // A primitive immediately followed by 0x95 is only a broad gate for the
    // more detailed records after the branch and is never itself returned.
    if program.get(next) == Some(&0x95) {
        return;
    }
    if let Some(hit) = scratch {
        if best.is_none_or(|current| hit.penetration_raw > current.penetration_raw) {
            *best = Some(hit);
        }
    }
}

fn collision_sphere_hit(
    query_center: [f64; 3],
    query_radius: f64,
    primitive_center: [f64; 3],
    primitive_radius: f64,
) -> Option<ModelCollisionHit> {
    // FUN_00469770 receives materialized i32 model slots and a signed-word
    // relative query; the primitive callback itself is entirely integer. The
    // port's orientation is still floating point, so materialize its center
    // once here. Flooring a transformed coordinate preserves the arithmetic
    // right-shift direction of retail's independently shifted Q31 products.
    let query_center = query_center.map(|component| component.trunc() as i32);
    let primitive_center = primitive_center.map(|component| component.floor() as i32);
    let delta = std::array::from_fn::<_, 3, _>(|axis| {
        query_center[axis].wrapping_sub(primitive_center[axis])
    });
    let radius = primitive_radius as i64 + query_radius as i64;
    if delta
        .iter()
        .any(|component| i64::from(*component).abs() > radius)
    {
        return None;
    }
    let distance_sq = delta
        .iter()
        .map(|component| i64::from(*component) * i64::from(*component))
        .sum::<i64>();
    if distance_sq > radius * radius {
        return None;
    }
    let distance = collision_integer_sqrt(distance_sq as u32) as i32;
    let normal = if distance > 0 {
        delta.map(|component| {
            // FUN_00469770 stores a signed Q12 normal, with C integer division
            // truncating toward zero.
            let q12 = component.wrapping_mul(0x1000) / distance;
            f64::from(q12 as i16) / 4096.0
        })
    } else {
        [1.0, 0.0, 0.0]
    };
    Some(ModelCollisionHit {
        normal,
        penetration_raw: (radius - i64::from(distance)) as f64,
    })
}

/// Sphere/plane callback `FUN_0046ABF0`, including its signed-word edge
/// differences and `FUN_0046ABA0` Q12 normalization. These are oriented
/// half-spaces, not finite triangle-distance tests; 0x8C/0x8D intersects them.
fn collision_plane_hit(
    query_center: [f64; 3],
    query_radius: f64,
    points: [[f64; 3]; 3],
) -> Option<ModelCollisionHit> {
    let query = query_center.map(|component| component.trunc() as i32);
    let [a, b, c] = points.map(|point| point.map(|component| component.floor() as i32));
    let u = std::array::from_fn::<_, 3, _>(|axis| i32::from(b[axis].wrapping_sub(a[axis]) as i16));
    let v = std::array::from_fn::<_, 3, _>(|axis| i32::from(c[axis].wrapping_sub(a[axis]) as i16));
    let cross = [
        u[1].wrapping_mul(v[2])
            .wrapping_sub(v[1].wrapping_mul(u[2])),
        v[0].wrapping_mul(u[2])
            .wrapping_sub(v[2].wrapping_mul(u[0])),
        v[1].wrapping_mul(u[0])
            .wrapping_sub(v[0].wrapping_mul(u[1])),
    ];
    // 004576E0 returns the highest set bit (zero for zero). Retail selects
    // shifts in five-bit steps before squaring, preserving signed rounding.
    let magnitude = cross
        .iter()
        .fold(0u32, |bits, value| bits | value.unsigned_abs());
    let highest_bit = 31 - magnitude.max(1).leading_zeros();
    let shift = match highest_bit {
        0..=13 => 0,
        14..=18 => 5,
        19..=23 => 10,
        24..=28 => 15,
        _ => 20,
    };
    let cross = cross.map(|component| component >> shift);
    let squared = cross.iter().fold(0u32, |sum, component| {
        sum.wrapping_add(component.wrapping_mul(*component) as u32)
    });
    let length = i32::from(collision_integer_sqrt(squared) as i16);
    let normal_q12 = if length == 0 {
        [0x1000i16, 0, 0]
    } else {
        cross.map(|component| (component.wrapping_shl(12) / length) as i16)
    };
    let dot = (0..3).fold(0i32, |sum, axis| {
        sum.wrapping_add(
            query[axis]
                .wrapping_sub(a[axis])
                .wrapping_mul(i32::from(normal_q12[axis])),
        )
    });
    let penetration = (query_radius as i32).wrapping_sub(dot >> 12);
    (penetration >= 1).then(|| ModelCollisionHit {
        normal: normal_q12.map(|component| f64::from(component) / 4096.0),
        penetration_raw: f64::from(penetration),
    })
}

/// Unsigned floor square root used by `FUN_00457730` for collision distances.
fn collision_integer_sqrt(mut value: u32) -> u32 {
    let mut result = 0u32;
    let mut bit = 1u32 << 30;
    while bit > value {
        bit >>= 2;
    }
    while bit != 0 {
        if value >= result + bit {
            value -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result
}

fn collision_aabb_hit(
    query_center: [f64; 3],
    query_radius: f64,
    primitive_center: [f64; 3],
    half_extents: [f64; 3],
) -> Option<ModelCollisionHit> {
    // The static-contact callback receives materialized integer centers. As
    // with the sphere callback above, preserve the signed shift direction at
    // the floating-point orientation boundary rather than carrying fractions
    // into an otherwise-integer interval test.
    let query_center = query_center.map(|component| component.trunc() as i32);
    let primitive_center = primitive_center.map(|component| component.floor() as i32);
    let delta = std::array::from_fn::<_, 3, _>(|axis| {
        f64::from(query_center[axis].wrapping_sub(primitive_center[axis]))
    });
    let overlap = std::array::from_fn::<_, 3, _>(|axis| {
        half_extents[axis] + query_radius - delta[axis].abs()
    });
    // FUN_0046A1C0 uses strict interval comparisons on every axis.
    if overlap.iter().any(|amount| *amount <= 0.0) {
        return None;
    }
    let axis = (0..3)
        .min_by(|&left, &right| overlap[left].total_cmp(&overlap[right]))
        .unwrap_or(0);
    let mut normal = [0.0; 3];
    // FUN_0046A1C0 selects the face on the query sphere's side of the box:
    // the normal points from the authored primitive toward the query, exactly
    // like FUN_00469770's sphere/sphere result.
    normal[axis] = if delta[axis] < 0.0 { -1.0 } else { 1.0 };
    Some(ModelCollisionHit {
        normal,
        penetration_raw: overlap[axis],
    })
}

fn mat3_apply_f64(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn mat3_mul_f64(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            a[row][0] * b[0][column] + a[row][1] * b[1][column] + a[row][2] * b[2][column]
        })
    })
}

fn transform_collision_point(basis: [[f64; 3]; 3], origin: [f64; 3], local: [f64; 3]) -> [f64; 3] {
    let rotated = mat3_apply_f64(basis, local);
    std::array::from_fn(|axis| origin[axis] + rotated[axis])
}

fn child_local_from_parent(
    parent_position: [f64; 3],
    child_origin: [f64; 3],
    child_basis: [[f64; 3]; 3],
) -> [f64; 3] {
    let relative = [
        parent_position[0] - child_origin[0],
        parent_position[1] - child_origin[1],
        parent_position[2] - child_origin[2],
    ];
    // Inverse of an orthonormal child-to-parent basis is its transpose.
    [
        child_basis[0][0] * relative[0]
            + child_basis[1][0] * relative[1]
            + child_basis[2][0] * relative[2],
        child_basis[0][1] * relative[0]
            + child_basis[1][1] * relative[1]
            + child_basis[2][1] * relative[2],
        child_basis[0][2] * relative[0]
            + child_basis[1][2] * relative[1]
            + child_basis[2][2] * relative[2],
    ]
}

fn build_collision_child_links(
    parent: &ModelEntry,
    parent_linked: Option<&LinkedModelSlots>,
    remap_slots: &[u16],
    vars: &AnimVars,
    child_origin: [f64; 3],
    child_basis: [[f64; 3]; 3],
) -> LinkedModelSlots {
    remap_slots
        .iter()
        .map(|&slot| {
            let even = resolve_collision_slot(parent, slot, vars, parent_linked).map(|position| {
                ResolvedModelSlot::clear(child_local_from_parent(
                    position,
                    child_origin,
                    child_basis,
                ))
            });
            let odd =
                resolve_collision_slot(parent, slot ^ 1, vars, parent_linked).map(|position| {
                    ResolvedModelSlot::clear(child_local_from_parent(
                        position,
                        child_origin,
                        child_basis,
                    ))
                });
            [even, odd]
        })
        .collect()
}

fn resolve_collision_slot(
    model: &ModelEntry,
    slot: u16,
    vars: &AnimVars,
    linked: Option<&LinkedModelSlots>,
) -> Option<[f64; 3]> {
    // Runtime generators write signed integer model coordinates. The shared
    // materializer retains fractional f64 interpolation for rendering, so
    // narrow it here with C's signed truncation semantics. This is especially
    // relevant to lifter's type-5 midpoint collision slots.
    model
        .resolve_slot_with_context(slot, ModelMaterializationContext::intrinsic(vars, linked))
        .map(|slot| slot.position_raw.map(f64::trunc))
}

// ── OBJ export ──────────────────────────────────────────────────────────────

impl ModelEntry {
    /// Re-materialize this model's geometry for a live [`AnimVars`] frame.
    ///
    /// Re-runs the command-stream interpreter and slot resolver with the given
    /// animation variables so morphs (tf-8/9), conditional-jump sub-assembly
    /// selection (op-0x2B/0x2C), billboard operands, mount frames and op-0x0E
    /// child orientations are all evaluated for this frame. Call it per frame
    /// for animated models. `AnimVars::default()` supplies zero callback
    /// values but still executes register commands embedded in the stream.
    pub fn materialize(&self, vars: &AnimVars) -> MaterializedModel {
        let mut stats = StreamStats::default();
        self.materialize_into(vars, &mut stats)
    }

    /// Re-materialize with explicit linked imports, presentation callbacks and
    /// view-dependent command selection.
    /// An intrinsic context with no linked table is equivalent to
    /// [`Self::materialize`]; unresolved imports drop their dependent faces.
    pub fn materialize_with_context(
        &self,
        context: ModelMaterializationContext<'_>,
    ) -> MaterializedModel {
        let mut stats = StreamStats::default();
        self.materialize_into_with_context(context, &mut stats)
    }

    /// Resolve one raw slot ref with the same optional linked-parent context
    /// used by [`Self::materialize_with_context`]. This is useful to build a
    /// child's linked table from the current parent's slot space. Rejected
    /// surface vertices retain their coordinates; `None` is an unavailable
    /// source and must not stand in for semantic callback clipping.
    pub fn resolve_slot_with_context(
        &self,
        slot: u16,
        context: ModelMaterializationContext<'_>,
    ) -> Option<ResolvedModelSlot> {
        let mut stats = StreamStats::default();
        let mut resolver = SlotResolver::with_linked(&self.records, context.vars, context.linked)
            .with_vertex_resolver(context.vertex_resolver);
        resolver.resolve(slot, &mut stats)
    }

    /// Collect the authored staged-destruction effect points from this
    /// model's Section-8 collision program.
    ///
    /// This reproduces the collision-program subset walked by
    /// `FUN_00419D90`: every opcode-`0x8E` sphere is returned in traversal
    /// order, opcode-`0x95` branch operands are skipped linearly, opcode
    /// `0x8F` boxes are ignored, and opcode-`0x05` child models are traversed
    /// with their authored attachment, orientation, and linked-slot remaps.
    /// Unsupported opcodes and malformed child recursion remain explicit
    /// errors rather than silently changing the effect layout.
    pub fn staged_effect_points_raw<P: CollisionModelPool + ?Sized>(
        &self,
        request: StagedEffectRequest,
        vars: &AnimVars,
        pool: &P,
    ) -> std::result::Result<Vec<StagedEffectPoint>, StagedEffectError> {
        let mut output = Vec::new();
        StagedEffectInterpreter { pool }.run(
            self,
            request.model_to_output_basis,
            request.model_origin_raw,
            vars,
            None,
            0,
            &mut output,
        )?;
        Ok(output)
    }

    /// Evaluate the authored Section-8 collision program against a sphere in
    /// this model's local raw-coordinate frame.
    ///
    /// This is the `FUN_0046AF20` path used after an entity's broad collision
    /// radius succeeds. Raw model coordinates and signed 8.8 entity/world
    /// coordinates share the same `/256` gameplay scale. The currently
    /// supported opcode set covers the Peasant/first-world player,
    /// Main Base, weight, lifter, and convex fence programs, including lifter's
    /// collision child. Programs using the remaining transform opcodes return an
    /// explicit error so callers can distinguish them from an authored miss.
    pub fn collide_sphere_raw<P: CollisionModelPool + ?Sized>(
        &self,
        query_center_raw: [f64; 3],
        query_radius_raw: u16,
        vars: &AnimVars,
        pool: &P,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        self.collide_sphere_raw_oriented(query_center_raw, query_radius_raw, IDENTITY3, vars, pool)
    }

    /// Evaluate the collision program after forward-transforming authored
    /// primitive centers through a model-to-query basis.
    ///
    /// `query_center_raw` is normally the projectile probe relative to the
    /// entity origin in world-axis raw units. Retail supplies the entity's
    /// signed-Q31 local-to-world matrix here. A caller whose live orientation
    /// is still stored as floating point should pass that orthonormal basis
    /// directly rather than inventing a lower-precision fixed-point format.
    /// Forward-transforming centers is significant for opcode `0x8F`: its
    /// extents stay axis-aligned in the query frame.
    pub fn collide_sphere_raw_oriented<P: CollisionModelPool + ?Sized>(
        &self,
        query_center_raw: [f64; 3],
        query_radius_raw: u16,
        model_to_query_basis: [[f64; 3]; 3],
        vars: &AnimVars,
        pool: &P,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        CollisionInterpreter { pool }.run(
            self,
            query_center_raw,
            f64::from(query_radius_raw),
            model_to_query_basis,
            [0.0; 3],
            vars,
            None,
            0,
        )
    }

    /// Evaluate this moving model against one identity-oriented static model.
    ///
    /// This is the model/model ordering used by Section-10 terrain-object
    /// contact: `self` is the active entity model, `target` is the selected
    /// static model, and `query_origin_in_target_raw` is the moving origin
    /// relative to the static object's origin in raw 8.8 units. The moving
    /// model's authored sphere centers are transformed through
    /// `query_to_target_basis`; the returned normal is expressed in the static
    /// target/world axes and points from the target toward the moving model.
    ///
    /// Retail type 46's `player4` program is a broad gated sphere followed by
    /// eight detailed spheres, so this implements its exact `FUN_0046AE40` /
    /// nested `FUN_0046AF20` path. A moving-side opcode outside
    /// `0x88/0x8E/0x95` remains an explicit error. The target side retains the
    /// complete subset supported by [`Self::collide_sphere_raw`], including
    /// boxes, convex plane groups, and collision-only children.
    pub fn collide_model_raw_oriented<P: CollisionModelPool + ?Sized>(
        &self,
        target: &ModelEntry,
        query_origin_in_target_raw: [f64; 3],
        query_to_target_basis: [[f64; 3]; 3],
        query_vars: &AnimVars,
        target_vars: &AnimVars,
        pool: &P,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        ModelPairCollisionInterpreter {
            target,
            target_vars,
            pool,
        }
        .run(
            self,
            query_origin_in_target_raw,
            query_to_target_basis,
            query_vars,
        )
    }

    /// Evaluate this moving model's authored collision spheres against the
    /// Section-10 terrain heightfield.
    ///
    /// `query_origin_world_raw` is the live entity origin in the signed 8.8
    /// world words used by retail. X/Z wrap toroidally through the 256×256
    /// terrain grid; Y remains signed. `query_to_world_basis` rotates each
    /// detailed sphere center into world axes before the heightfield callback.
    /// The returned normal is expressed in world axes and points out of the
    /// terrain's authored upper surface.
    ///
    /// This compatibility entry has no child-model pool. The active `player4`
    /// sphere/gate program needs none; models with collision-only children use
    /// [`Self::collide_terrain_raw_oriented_with_pool`]. Unavailable children and
    /// unowned transform commands remain explicit errors.
    pub fn collide_terrain_raw_oriented(
        &self,
        terrain: &TerrainGrid,
        query_origin_world_raw: [i16; 3],
        query_to_world_basis: [[f64; 3]; 3],
        query_vars: &AnimVars,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        self.collide_terrain_raw_oriented_with_pool(
            terrain,
            query_origin_world_raw,
            query_to_world_basis,
            query_vars,
            &(),
        )
    }

    /// Walk the same authored query-side child hierarchy as model contacts,
    /// retaining the heightfield primitive callback through each nested node.
    pub fn collide_terrain_raw_oriented_with_pool<P: CollisionModelPool + ?Sized>(
        &self,
        terrain: &TerrainGrid,
        query_origin_world_raw: [i16; 3],
        query_to_world_basis: [[f64; 3]; 3],
        query_vars: &AnimVars,
        pool: &P,
    ) -> std::result::Result<Option<ModelCollisionHit>, ModelCollisionError> {
        run_query_sphere_program(
            self,
            query_origin_world_raw.map(f64::from),
            query_to_world_basis,
            query_vars,
            pool,
            |sphere_center, sphere_radius| {
                Ok(terrain_sphere_hit(terrain, sphere_center, sphere_radius))
            },
        )
    }

    /// [`Self::materialize`] accumulating interpreter anomalies into `stats`
    /// (used by the parser to build the collection-wide `StreamStats`).
    fn materialize_into(&self, vars: &AnimVars, stats: &mut StreamStats) -> MaterializedModel {
        self.materialize_into_with_context(
            ModelMaterializationContext::intrinsic(vars, None),
            stats,
        )
    }

    fn materialize_into_with_context(
        &self,
        context: ModelMaterializationContext<'_>,
        stats: &mut StreamStats,
    ) -> MaterializedModel {
        let (raw, evaluated_vars) = interpret(
            &self.cmd_words,
            &self.records,
            &self.normal_pool,
            context,
            stats,
        );
        let mut resolver =
            SlotResolver::with_linked(&self.records, &evaluated_vars, context.linked)
                .with_vertex_resolver(context.vertex_resolver);
        let ResolvedGeometry {
            mut vertices,
            mut vertex_type_flags,
            mut vertex_projection,
            mut vertex_clip,
            mut vertex_surface_origin,
            mut vertex_view_raw,
            triangles,
            face_vertices,
            normals,
            face_cull,
            face_materials,
            face_uvs,
            face_corner_normals,
            face_normals_raw,
            face_shading,
            shadow_triangles,
            edges,
            edge_endpoint_snapshots,
            native_edge_failures,
            billboards,
            instances,
            painter_program,
        } = resolve_stream(&mut resolver, &raw, &self.normal_pool, stats);

        // Nodes without drawn primitives retain their plain attachment points.
        // Billboard-only nodes already have resolved anchors and projector
        // dependencies; replacing them here invalidates the billboard indices.
        if triangles.is_empty()
            && edges.is_empty()
            && shadow_triangles.is_empty()
            && billboards.is_empty()
        {
            vertices = self
                .records
                .iter()
                .filter(|r| r[0] == 0)
                .map(|r| [r[1] as f64, r[2] as f64, r[3] as f64])
                .collect();
            vertex_type_flags = vec![0; vertices.len()];
            vertex_projection = vec![ModelVertexProjection::Position; vertices.len()];
            vertex_clip = vec![ModelSlotClip::Clear; vertices.len()];
            vertex_surface_origin = vec![ModelSurfaceOrigin::None; vertices.len()];
            vertex_view_raw = Vec::new();
        }

        MaterializedModel {
            has_view_commands: raw.has_view_commands,
            vertices,
            vertex_type_flags,
            vertex_projection,
            vertex_clip,
            vertex_surface_origin,
            vertex_view_raw,
            triangles,
            face_vertices,
            normals,
            face_cull,
            face_materials,
            face_uvs,
            face_corner_normals,
            face_normals_raw,
            face_shading,
            shadow_triangles,
            edges,
            edge_endpoint_snapshots,
            native_edge_failures,
            billboards,
            instances,
            painter_program,
        }
    }

    /// Normal pool entry count derived from face_val.
    pub fn normal_pool_len(&self) -> usize {
        if self.face_val >= 2 {
            ((self.face_val - 2) >> 1) as usize
        } else {
            0
        }
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::{TerrainCell, GRID_SIZE};

    #[test]
    fn empty_data_returns_error() {
        assert!(parse_model_subblocks(&[], 0).is_err());
    }

    #[test]
    fn zero_header_returns_error() {
        assert!(parse_model_subblocks(&[0u8; 100], 0).is_err());
    }

    #[test]
    fn face_stream_keeps_flat_and_gouraud_handler_modes_distinct() {
        let mut flat = Vec::new();
        emit(
            &mut flat,
            &[0, 2, 4],
            6,
            7,
            false,
            ModelFaceShading::Flat,
            None,
        );
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].shading, ModelFaceShading::Flat);
        assert_eq!(flat[0].corner_normals, [6, 6, 6]);

        let mut gouraud = Vec::new();
        emit(
            &mut gouraud,
            &[0, 2, 4, 6],
            8,
            9,
            true,
            ModelFaceShading::Gouraud,
            Some(&[10, 12, 14, 16]),
        );
        assert_eq!(gouraud.len(), 4);
        assert!(gouraud
            .iter()
            .all(|face| face.shading == ModelFaceShading::Gouraud));
        assert_eq!(gouraud[0].corner_normals, [10, 12, 14]);
        assert_eq!(gouraud[1].corner_normals, [10, 14, 16]);
        assert_eq!(gouraud[2].corner_normals, [11, 13, 15]);
        assert_eq!(gouraud[3].corner_normals, [11, 15, 17]);
    }

    #[test]
    fn face_opcode_families_preserve_three_retail_lighting_policies() {
        // Texture (0x80), lighting (0x20/0x40), and mirrored-pair (_7/_8)
        // dispatch are independent. In particular C3/C4 are uniformly lit,
        // not the unlit 83/84 handlers used by fixed-palette sprites.
        for (opcodes, shading) in [
            (
                [0x03, 0x04, 0x07, 0x08, 0x83, 0x84, 0x87, 0x88],
                ModelFaceShading::Flat,
            ),
            (
                [0x43, 0x44, 0x47, 0x48, 0xC3, 0xC4, 0xC7, 0xC8],
                ModelFaceShading::FlatLit,
            ),
            (
                [0x23, 0x24, 0x27, 0x28, 0xA3, 0xA4, 0xA7, 0xA8],
                ModelFaceShading::Gouraud,
            ),
        ] {
            for op in opcodes {
                let quad = op & 1 == 0;
                let mirrored = op & 0x0F >= 7;
                let vertices = if quad {
                    &[0, 2, 4, 6][..]
                } else {
                    &[0, 2, 4][..]
                };
                let mut words = vec![op, 17, 2];
                words.extend_from_slice(vertices);
                if shading == ModelFaceShading::Gouraud {
                    words.extend_from_slice(if quad {
                        &[4, 6, 8, 10][..]
                    } else {
                        &[4, 6, 8][..]
                    });
                }
                words.push(0);
                let mut stats = StreamStats::default();
                let raw = interpret_bare(&words, &mut stats);
                let triangle_count = if quad { 2 } else { 1 };
                assert_eq!(
                    raw.tris.len(),
                    triangle_count * if mirrored { 2 } else { 1 },
                    "opcode {op:02X}"
                );
                for (index, face) in raw.tris.iter().enumerate() {
                    assert_eq!(face.shading, shading, "opcode {op:02X}");
                    assert_eq!(face.material, if op & 0x80 != 0 { 0x8011 } else { 17 });
                    let mirror = u16::from(index >= triangle_count);
                    assert_eq!(face.normal, 2 ^ mirror);
                    if shading != ModelFaceShading::Gouraud {
                        assert_eq!(face.corner_normals, [2 ^ mirror; 3]);
                    } else {
                        let expected = if index % triangle_count == 0 {
                            [4, 6, 8]
                        } else {
                            [4, 8, 10]
                        };
                        assert_eq!(
                            face.corner_normals,
                            expected.map(|reference| reference ^ mirror)
                        );
                    }
                }
                assert!(stats.is_clean(), "opcode {op:02X}: {stats:?}");
            }
        }
    }

    #[test]
    fn uniform_lighting_retains_mirrored_face_normal_and_zero_table_slot() {
        let records = [
            [0, 0, 0, 0],
            [0, 100, 0, 0],
            [0, 100, 100, 0],
            [0, 0, 100, 0],
        ];
        let pool = [[0, 16_384, 0, 0], [0, 0, 0, 0]];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&[0xC8, 17, 2, 0, 2, 4, 6, 0], &mut stats);
        let vars = AnimVars::default();
        let mut resolver = SlotResolver::new(&records, &vars);
        let geometry = resolve_stream(&mut resolver, &raw, &pool, &mut stats);
        assert_eq!(geometry.face_shading, [ModelFaceShading::FlatLit; 4]);
        assert_eq!(
            geometry.normals,
            [
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0]
            ]
        );
        for (normal, corners) in geometry.normals.iter().zip(&geometry.face_corner_normals) {
            assert_eq!(*corners, [*normal; 3]);
        }
        // FUN_0046D5A0's implicit pair and a stored zero vector all resolve
        // shade slot zero. Scene shade shifts remain renderer-owned.
        for reference in [0, 1, 4, 5] {
            assert_eq!(pool_normal(&pool, reference), [0.0; 3]);
            assert_eq!(pool_normal_raw(&pool, reference), [0; 3]);
        }
        // 46D3F0 dots the stored vector, X negated for the mirrored pair.
        assert_eq!(
            geometry.face_normals_raw,
            [
                [[16_384, 0, 0]; 4],
                [[16_384, 0, 0]; 4],
                [[-16_384, 0, 0]; 4],
                [[-16_384, 0, 0]; 4]
            ]
        );
    }

    #[test]
    fn native_view_points_follow_each_materialized_vertex() {
        struct Owner;
        impl ModelVertexResolver for Owner {
            fn resolve_vertex_raw(
                &self,
                _: ModelVertexKind,
                _: ResolvedModelSlot,
            ) -> Option<ResolvedModelSlot> {
                None
            }
            fn owns_native_view(&self) -> bool {
                true
            }
            fn resolve_native_view_point(&self, slot: u16, _: &AnimVars) -> Option<[i32; 3]> {
                Some([i32::from(slot), -i32::from(slot), 1_000 + i32::from(slot)])
            }
        }
        let model = ModelEntry {
            records: vec![[0, 0, 0, 0], [0, 100, 0, 0], [0, 100, 100, 0]],
            normal_pool: vec![[0, 0, 0, 16_384]],
            cmd_words: vec![0x43, 17, 2, 4, 0, 2, 0],
            ..ModelEntry::default()
        };
        let vars = AnimVars::default();
        let geometry = model.materialize_with_context(ModelMaterializationContext {
            vertex_resolver: Some(&Owner),
            ..ModelMaterializationContext::intrinsic(&vars, None)
        });
        // First-reference order: slots 4, 0, 2.
        assert_eq!(
            geometry.vertex_view_raw,
            [
                Some([4, -4, 1_004]),
                Some([0, 0, 1_000]),
                Some([2, -2, 1_002])
            ]
        );
        assert_eq!(geometry.face_normals_raw, [[[0, 0, 16_384]; 4]]);
        let intrinsic = model.materialize(&vars);
        assert_eq!(intrinsic.vertex_view_raw, [None; 3]);
    }

    #[test]
    fn parses_distinct_lod_and_collision_radii() {
        // Minimal unnamed entry: one collision dword, but no vertices,
        // normals, render commands, or name.
        // Keep the two radius words deliberately distinct, with the high bit
        // set on the collision radius to prove it is retained as an unsigned
        // value just like FUN_00412530's zero-extending retail load.
        let mut data = [0u8; HEADER_SIZE + 4];
        data[2] = 1;
        data[3] = 0x40;
        data[6..8].copy_from_slice(&2u16.to_le_bytes());
        data[8..10].copy_from_slice(&0x1234u16.to_le_bytes());
        data[10..12].copy_from_slice(&0xFEDCu16.to_le_bytes());
        data[12..16].copy_from_slice(&[0x8e, 0, 0x34, 0x12]);

        let header_value = (1u32 << 16) | data.len() as u32;
        let models = parse_model_subblocks(&data, header_value).expect("minimal Section-8 entry");
        let entry = &models.all_entries[0];

        assert_eq!(entry.radius, 0x1234);
        assert_eq!(entry.collision_radius_raw, 0xFEDC);
        assert_eq!(entry.collision_program, [0x8e, 0, 0x34, 0x12]);
    }

    fn collision_entry(records: Vec<[i16; 4]>, collision_program: Vec<u8>) -> ModelEntry {
        ModelEntry {
            index: 0,
            cmd_word_count: 0,
            extra_count: (collision_program.len() / 4) as u8,
            flags: 0x40,
            slot_count: (records.len() * 2) as u16,
            face_val: 2,
            radius: 0,
            collision_radius_raw: 1_000,
            collision_program,
            records,
            normal_pool: Vec::new(),
            cmd_words: Vec::new(),
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::new(),
            instances: Vec::new(),
            painter_program: Vec::new(),
            name: None,
        }
    }

    fn flat_terrain(height: i8) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    impl CollisionModelPool for Vec<ModelEntry> {
        fn collision_model(&self, global_id: usize) -> Option<&ModelEntry> {
            self.get(global_id)
        }
    }

    #[test]
    fn collision_program_tangent_can_gate_but_is_not_a_final_hit() {
        let sphere = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let vars = AnimVars::default();
        assert!(sphere
            .collide_sphere_raw([15.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_none());
        assert!(sphere
            .collide_sphere_raw([14.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_some());
        assert!(sphere
            .collide_sphere_raw([15.01, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_none());

        // Radius-100 slot 0 gates a detailed radius-10 sphere at slot 2.
        // A query at the broad center must still miss the authored detail.
        let gated = collision_entry(
            vec![[0, 0, 0, 0], [0, 50, 0, 0]],
            vec![
                0x8E, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0x95, 0, 12, 0, 0x8E, 0, 10, 0, 0, 0, 2, 0,
                0x88, 0,
            ],
        );
        assert!(gated
            .collide_sphere_raw([0.0, 0.0, 0.0], 1, &vars, &())
            .unwrap()
            .is_none());
        assert!(gated
            .collide_sphere_raw([50.0, 0.0, 0.0], 1, &vars, &())
            .unwrap()
            .is_some());

        // The inclusive sphere callback still sets scratch on a tangent gate,
        // allowing the detailed branch to run even though that gate alone
        // could not become the final result.
        let tangent_gate = collision_entry(
            vec![[0, 0, 0, 0], [0, 101, 0, 0]],
            vec![
                0x8E, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0x95, 0, 12, 0, 0x8E, 0, 10, 0, 0, 0, 2, 0,
                0x88, 0,
            ],
        );
        assert!(tangent_gate
            .collide_sphere_raw([101.0, 0.0, 0.0], 1, &vars, &())
            .unwrap()
            .is_some());
    }

    #[test]
    fn collision_sphere_uses_retail_integer_floor_distance() {
        let sphere = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0x88, 0],
        );
        // Combined radius 15 and delta (14, 1, 0): retail computes
        // 15 - floor(sqrt(197)) = 1. A floating-point distance leaves only
        // ~0.964 raw penetration and incorrectly fails the 0x88 threshold.
        let hit = sphere
            .collide_sphere_raw([14.0, 1.0, 0.0], 1, &AnimVars::default(), &())
            .unwrap()
            .expect("integer floor distance must preserve one raw unit");
        assert_eq!(hit.penetration_raw, 1.0);
        assert!(hit.normal[0] > 0.0 && hit.normal[1] > 0.0);
    }

    #[test]
    fn collision_convex_fence493_uses_planes_after_both_sphere_gates() {
        // Exact Section-8 fence2 (global493) program/records. Slots8,18,22
        // depend on parallelogram and intrinsic alias materialization.
        let fence = collision_entry(
            vec![
                [0, 0, 0, 0],
                [0, 0, 128, 0],
                [0, -256, 256, 0],
                [12, 4, 0, 0],
                [6, 6, 2, 0],
                [13, 2, 0, 0],
                [13, 8, 0, 0],
                [5, 1, 0, 8],
                [0, -128, 0, 51],
                [12, 16, 0, 0],
                [0, -128, 0, -51],
                [12, 20, 0, 0],
            ],
            vec![
                0x8E, 0, 0, 0, 0x4C, 1, 0, 0, 0, 0, 0x95, 0, 0x23, 0, 0x8E, 0, 0xB3, 0, 0, 0, 0x0E,
                0, 0x95, 0, 0x17, 0, 0x8C, 0, 0x13, 0, 0x8A, 0, 2, 0, 8, 0, 18, 0, 0x8A, 0, 8, 0,
                2, 0, 22, 0, 0x8D, 0x88,
            ],
        );
        let vars = AnimVars::default();
        // Cross=(13056,13056,65536), shifted=(408,408,2048),
        // floor length=2127, signed Q12 normal=(785,785,3943).
        let hit = fence
            .collide_sphere_raw([0.0, 128.0, 0.0], 8, &vars, &())
            .unwrap()
            .unwrap();
        assert_eq!(hit.penetration_raw, 8.0);
        assert_eq!(
            hit.normal,
            [785.0 / 4096.0, 785.0 / 4096.0, 3943.0 / 4096.0]
        );
        assert_eq!(
            fence
                .collide_sphere_raw([0.0, 128.0, 8.0], 8, &vars, &())
                .unwrap()
                .unwrap()
                .penetration_raw,
            1.0
        );
        // Both points pass both bounding spheres but fail a convex plane.
        for point in [[0.0, 128.0, 9.0], [0.0, 192.0, 0.0]] {
            assert!(fence
                .collide_sphere_raw(point, 8, &vars, &())
                .unwrap()
                .is_none());
        }
        let yaw90 = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let rotated = fence
            .collide_sphere_raw_oriented([0.0, 128.0, 0.0], 8, yaw90, &vars, &())
            .unwrap()
            .unwrap();
        assert_eq!(rotated.penetration_raw, 8.0);
        assert_eq!(
            rotated.normal,
            [3943.0 / 4096.0, 785.0 / 4096.0, -785.0 / 4096.0]
        );

        // 00469B70's actual model-pair route installs sphere callbacks for
        // the target program, including the 0046ABF0 plane callback.
        let moving = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0x88],
        );
        assert_eq!(
            moving
                .collide_model_raw_oriented(&fence, [0.0, 128.0, 0.0], IDENTITY3, &vars, &vars, &())
                .unwrap(),
            Some(hit)
        );
    }

    #[test]
    fn collision_convex_group_minimizes_planes_and_maximizes_complete_primitives() {
        let records = vec![[0, 0, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1], [0, 1, 0, 0]];
        let program = vec![
            0x8C, 0, 19, 0, 0x8A, 0, 0, 0, 2, 0, 4, 0, 0x8A, 0, 0, 0, 4, 0, 6, 0, 0x8D, 0x88,
        ];
        let group = collision_entry(records.clone(), program.clone());
        let vars = AnimVars::default();
        let hit = group
            .collide_sphere_raw([1.0, 2.0, 0.0], 5, &vars, &())
            .unwrap()
            .unwrap();
        assert_eq!(
            hit,
            ModelCollisionHit {
                normal: [0.0, 1.0, 0.0],
                penetration_raw: 3.0
            }
        );
        let tie = group
            .collide_sphere_raw([2.0, 2.0, 0.0], 5, &vars, &())
            .unwrap()
            .unwrap();
        assert_eq!(tie.normal, [1.0, 0.0, 0.0]);
        assert!(group
            .collide_sphere_raw([1.0, 5.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_none());

        // A preceding ungated sphere penetrates by5+10-floor(sqrt(5))=13.
        let mut with_sphere = vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0];
        with_sphere.extend(program);
        let model = collision_entry(records, with_sphere);
        assert_eq!(
            model
                .collide_sphere_raw([1.0, 2.0, 0.0], 5, &vars, &())
                .unwrap()
                .unwrap()
                .penetration_raw,
            13.0
        );
    }

    #[test]
    fn collision_convex_group_gate_and_unused_words_follow_retail_control_flow() {
        // 0x89 ignores its fourth word; 0x8B consumes an unused guard word.
        // The completed group is itself gated and must not hide a detail miss.
        let model = collision_entry(
            vec![[0, 0, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1], [0, 100, 0, 0]],
            vec![
                0x8C, 0, 32, 0, 0x89, 0, 0, 0, 2, 0, 4, 0, 0xFF, 0xFF, 0x8B, 0, 0xFE, 0xFF, 0x8D,
                0x95, 14, 0, 0x8E, 0, 1, 0, 0, 0, 6, 0, 0x88, 0, 0, 0, 0x88,
            ],
        );
        let vars = AnimVars::default();
        assert!(model
            .collide_sphere_raw([0.0; 3], 5, &vars, &())
            .unwrap()
            .is_none());
        // Plane rejection jumps over the guard, commit and detailed sphere.
        assert!(model
            .collide_sphere_raw([100.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_none());
    }

    #[test]
    fn collision_plane_uses_signed_word_edges_and_degenerate_fallback() {
        let hit = collision_plane_hit(
            [0.0, 0.0, 1.0],
            1.0,
            [[0.0; 3], [40000.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        )
        .unwrap();
        // 40000 narrows to -25536 before the cross product, reversing winding.
        assert_eq!(
            hit,
            ModelCollisionHit {
                normal: [0.0, 0.0, -1.0],
                penetration_raw: 2.0
            }
        );
        let flat = [[0.0; 3]; 3];
        assert_eq!(
            collision_plane_hit([4.0, 0.0, 0.0], 5.0, flat).unwrap(),
            ModelCollisionHit {
                normal: [1.0, 0.0, 0.0],
                penetration_raw: 1.0
            }
        );
        assert!(collision_plane_hit([5.0, 0.0, 0.0], 5.0, flat).is_none());
    }

    #[test]
    fn collision_convex_empty_group_preserves_acceptance_and_previous_primitive() {
        let vars = AnimVars::default();
        // The broad sphere is gated, the following sphere misses, and the
        // empty group inherits the last successful depth/normal (0046B4A8).
        let fallback = collision_entry(
            vec![[0, 0, 0, 0], [0, 100, 0, 0]],
            vec![
                0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x95, 0, 15, 0, 0x8E, 0, 1, 0, 0, 0, 2, 0, 0x8C,
                0, 3, 0, 0x8D, 0x88,
            ],
        );
        assert_eq!(
            fallback
                .collide_sphere_raw([0.0; 3], 1, &vars, &())
                .unwrap()
                .unwrap(),
            ModelCollisionHit {
                normal: [1.0, 0.0, 0.0],
                penetration_raw: 11.0
            }
        );

        // With no earlier primitive, group acceptance is still true although
        // its depth remains -1: the following gate admits the detail sphere.
        let empty_gate = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![
                0x8C, 0, 16, 0, 0x8D, 0x95, 12, 0, 0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88,
            ],
        );
        assert_eq!(
            empty_gate
                .collide_sphere_raw([0.0; 3], 1, &vars, &())
                .unwrap()
                .unwrap()
                .penetration_raw,
            11.0
        );
    }

    #[test]
    fn collision_convex_group_reports_invalid_spans_and_truncated_planes() {
        for (program, error) in [
            (
                vec![0x8C],
                ModelCollisionError::Truncated {
                    pc: 0,
                    opcode: 0x8C,
                },
            ),
            (
                vec![0x8C, 0, 0xFD, 0xFF, 0x88],
                ModelCollisionError::InvalidBranch { pc: 0 },
            ),
            (
                vec![0x8C, 0, 10, 0, 0x88],
                ModelCollisionError::InvalidBranch { pc: 0 },
            ),
            (
                vec![0x8C, 0, 4, 0, 0x8A, 0, 0x88],
                ModelCollisionError::Truncated {
                    pc: 4,
                    opcode: 0x8A,
                },
            ),
        ] {
            let model = collision_entry(Vec::new(), program);
            assert_eq!(
                model
                    .collide_sphere_raw([0.0; 3], 1, &AnimVars::default(), &())
                    .unwrap_err(),
                error
            );
        }
    }

    #[test]
    fn collision_program_aabb_uses_strict_expanded_intervals() {
        let model = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![
                0x8F, 0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0x88, 0,
            ],
        );
        let vars = AnimVars::default();
        assert!(model
            .collide_sphere_raw([14.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_some());
        assert!(model
            .collide_sphere_raw([15.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .is_none());
    }

    #[test]
    fn collision_program_aabb_normal_points_from_box_toward_query() {
        let model = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![
                0x8F, 0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0x88, 0,
            ],
        );
        let vars = AnimVars::default();

        let positive = model
            .collide_sphere_raw([14.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .expect("positive-X face");
        assert_eq!(positive.normal, [1.0, 0.0, 0.0]);
        assert_eq!(positive.penetration_raw, 1.0);

        let negative = model
            .collide_sphere_raw([-14.0, 0.0, 0.0], 5, &vars, &())
            .unwrap()
            .expect("negative-X face");
        assert_eq!(negative.normal, [-1.0, 0.0, 0.0]);
        assert_eq!(negative.penetration_raw, 1.0);
    }

    #[test]
    fn model_pair_uses_detailed_query_spheres_and_target_axis_normal() {
        let query = collision_entry(
            vec![[0, 0, 0, 0], [0, 14, 0, 0]],
            vec![
                // Broad sphere, followed by a gate that skips to 0x88 on a
                // miss. This 110-raw overlap must never become the result.
                0x8E, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0x95, 0, 10, 0,
                // Detailed sphere at slot 2: 10 + 5 - 14 = 1 raw overlap.
                0x8E, 0, 5, 0, 0, 0, 2, 0, 0x88, 0,
            ],
        );
        let target = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let vars = AnimVars::default();

        let hit = query
            .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &())
            .unwrap()
            .expect("detailed sphere should touch target");
        assert_eq!(hit.penetration_raw, 1.0);
        assert_eq!(hit.normal, [1.0, 0.0, 0.0]);

        let yaw_90 = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let rotated = query
            .collide_model_raw_oriented(&target, [0.0; 3], yaw_90, &vars, &vars, &())
            .unwrap()
            .expect("rotated detailed sphere should touch target");
        assert_eq!(rotated.penetration_raw, 1.0);
        assert_eq!(rotated.normal, [0.0, 0.0, -1.0]);

        assert!(query
            .collide_model_raw_oriented(&target, [1_000.0, 0.0, 0.0], IDENTITY3, &vars, &vars, &(),)
            .unwrap()
            .is_none());
    }

    #[test]
    fn query_child_composes_orientation_attachment_and_linked_slot_imports() {
        let parent = collision_entry(
            vec![[0, 100, 10, 20], [0, 130, 40, 50]],
            vec![0x05, 0, 1, 0, 0x10, 0, 10, 0, 0, 0, 2, 0, 0x88],
        );
        let child = collision_entry(
            vec![[11, 1, 0, 0], [0, 10, 20, 30]],
            vec![
                0x8E, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0x8E, 0, 7, 0, 0, 0, 2, 0, 0x88,
            ],
        );
        let pool = vec![parent, child];
        let yaw_90 = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let vars = AnimVars::default();
        let mut spheres = Vec::new();
        assert!(run_query_sphere_program(
            &pool[0],
            [1_000.0, 2_000.0, 3_000.0],
            yaw_90,
            &vars,
            &pool,
            |center, radius| {
                spheres.push((center, radius));
                Ok(None)
            },
        )
        .unwrap()
        .is_none());
        assert_eq!(
            spheres,
            [
                // Child slot 0 imports parent slot 2 through remap index 1.
                ([1_050.0, 2_040.0, 2_870.0], 5.0),
                // Orientation 0x10 negates child Y before the root yaw.
                ([1_050.0, 1_990.0, 2_890.0], 7.0),
            ]
        );

        let target = collision_entry(
            vec![[0, 1_064, 2_040, 2_870]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88],
        );
        let hit = pool[0]
            .collide_model_raw_oriented(
                &target,
                [1_000.0, 2_000.0, 3_000.0],
                yaw_90,
                &vars,
                &vars,
                &pool,
            )
            .unwrap()
            .expect("imported child sphere reaches target through composed frames");
        assert_eq!(hit.penetration_raw, 1.0);
        assert_eq!(hit.normal, [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn query_child_miss_resets_gate_scratch_without_losing_earlier_best() {
        let query = collision_entry(
            vec![[0, 12, 0, 0], [0, 1_000, 0, 0]],
            vec![
                0x05, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0x05, 0, 1, 0, 0, 0, 8, 0, 2, 0,
                // A miss at the second child must skip the large fallback.
                0x95, 0, 12, 0, 0x8E, 0, 0, 0, 0x88, 0x13, 0, 0, 0, 0, 0x88,
            ],
        );
        let child = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0x88],
        );
        let target = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88],
        );
        let pool = vec![query, child];
        let vars = AnimVars::default();
        let hit = pool[0]
            .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &pool)
            .unwrap()
            .expect("first sibling's promoted hit survives the second sibling's miss");
        assert_eq!(hit.penetration_raw, 3.0);
        assert_eq!(hit.normal, [1.0, 0.0, 0.0]);

        // An unresolved attachment also clears scratch before that gate.
        let mut unresolved_pool = pool;
        unresolved_pool[0].collision_program[18..20].copy_from_slice(&u16::MAX.to_le_bytes());
        let unresolved = unresolved_pool[0]
            .collide_model_raw_oriented(
                &target,
                [0.0; 3],
                IDENTITY3,
                &vars,
                &vars,
                &unresolved_pool,
            )
            .unwrap()
            .expect("unresolved child does not erase a prior promoted result");
        assert_eq!(unresolved, hit);
    }

    #[test]
    fn query_children_select_maximum_penetration_and_preserve_first_equal_hit() {
        let query = collision_entry(
            vec![[0, 12, 0, 0], [0, -11, 0, 0]],
            vec![
                0x05, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0x05, 0, 1, 0, 0, 0, 8, 0, 2, 0, 0x88,
            ],
        );
        let child = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0x88],
        );
        let target = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88],
        );
        let mut pool = vec![query, child];
        let vars = AnimVars::default();
        let deeper = pool[0]
            .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &pool)
            .unwrap()
            .unwrap();
        assert_eq!(deeper.penetration_raw, 4.0);
        assert_eq!(deeper.normal, [-1.0, 0.0, 0.0]);

        pool[0].records[1][1] = -12;
        let equal = pool[0]
            .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &pool)
            .unwrap()
            .unwrap();
        assert_eq!(equal.penetration_raw, 3.0);
        assert_eq!(equal.normal, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn query_child_keeps_missing_mount_and_recursion_boundaries_explicit() {
        let query = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x05, 0, 0, 0, 6, 0, 8, 0, 0, 0, 0x88],
        );
        let mut pool = vec![query];
        let vars = AnimVars::default();
        let target = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88],
        );
        assert_eq!(
            pool[0]
                .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &pool)
                .unwrap_err(),
            ModelCollisionError::UnsupportedOpcode {
                pc: 0,
                opcode: 0x05
            },
        );
        pool[0].collision_program[4] = 0;
        assert_eq!(
            pool[0]
                .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &())
                .unwrap_err(),
            ModelCollisionError::MissingChild { global_id: 0 },
        );
        assert_eq!(
            pool[0]
                .collide_model_raw_oriented(&target, [0.0; 3], IDENTITY3, &vars, &vars, &pool)
                .unwrap_err(),
            ModelCollisionError::RecursionLimit,
        );
    }

    #[test]
    fn terrain_query_child_retains_heightfield_callback_and_attachment() {
        let query = collision_entry(
            vec![[0, 0, 3, 0]],
            vec![0x05, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0x88],
        );
        let child = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0x88],
        );
        let pool = vec![query, child];
        let terrain = flat_terrain(0);
        let vars = AnimVars::default();
        assert_eq!(
            pool[0]
                .collide_terrain_raw_oriented(&terrain, [128, 0, 128], IDENTITY3, &vars)
                .unwrap_err(),
            ModelCollisionError::MissingChild { global_id: 1 },
        );
        let hit = pool[0]
            .collide_terrain_raw_oriented_with_pool(
                &terrain,
                [128, 0, 128],
                IDENTITY3,
                &vars,
                &pool,
            )
            .unwrap()
            .expect("child callback tests sphere center Y3 against the heightfield");
        assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
        assert_eq!(hit.penetration_raw, 2.0);
    }

    #[test]
    fn terrain_collision_uses_authored_sphere_and_one_sided_upper_surface() {
        let query = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let terrain = flat_terrain(0);
        let vars = AnimVars::default();

        let hit = query
            .collide_terrain_raw_oriented(&terrain, [128, 9, 128], IDENTITY3, &vars)
            .unwrap()
            .expect("sphere should overlap flat terrain by one raw unit");
        assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
        assert_eq!(hit.penetration_raw, 1.0);

        // A tangent can set the interpreter scratch value but is not a final
        // collision until retail's one-raw-unit threshold is reached.
        assert!(query
            .collide_terrain_raw_oriented(&terrain, [128, 10, 128], IDENTITY3, &vars)
            .unwrap()
            .is_none());
        assert!(query
            .collide_terrain_raw_oriented(&terrain, [128, 11, 128], IDENTITY3, &vars)
            .unwrap()
            .is_none());

        let below = query
            .collide_terrain_raw_oriented(&terrain, [128, -5, 128], IDENTITY3, &vars)
            .unwrap()
            .expect("crossing the heightfield underside must not escape contact");
        assert_eq!(below.normal, [0.0, 1.0, 0.0]);
        assert_eq!(below.penetration_raw, 15.0);
    }

    #[test]
    fn terrain_collision_rotates_detailed_sphere_centers() {
        let query = collision_entry(
            vec![[0, 0, 20, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let terrain = flat_terrain(0);
        let vars = AnimVars::default();

        assert!(query
            .collide_terrain_raw_oriented(&terrain, [128, 0, 128], IDENTITY3, &vars)
            .unwrap()
            .is_none());

        let roll_180 = [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
        let hit = query
            .collide_terrain_raw_oriented(&terrain, [128, 0, 128], roll_180, &vars)
            .unwrap()
            .expect("orientation must move the authored sphere below the hull origin");
        assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
        assert_eq!(hit.penetration_raw, 30.0);
    }

    #[test]
    fn terrain_collision_recovers_below_concave_vertex_with_upward_normal() {
        let mut terrain = flat_terrain(0);
        terrain.cells[1].height = 8;
        terrain.cells[GRID_SIZE].height = 8;
        let hit = terrain_sphere_hit(&terrain, [0.0, -10.0, 0.0], 20.0)
            .expect("a sphere below a concave heightfield vertex must be recovered");

        assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
        assert_eq!(hit.penetration_raw, 30.0);
    }

    #[test]
    fn terrain_collision_uses_retail_averaged_cell_plane() {
        let query = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let mut terrain = flat_terrain(0);
        terrain.cells[GRID_SIZE + 1].height = 8; // (1,1) = +256 raw
        let vars = AnimVars::default();

        // Emulated retail 415160, including 415770/415610 normalization.
        // Collision uses a common cell plane even on the flat rendered half.
        for (position, depth) in [([64, 3, 64], 41.0), ([192, 129, 192], 13.0)] {
            let hit = query
                .collide_terrain_raw_oriented(&terrain, position, IDENTITY3, &vars)
                .unwrap()
                .expect("retail averaged cell plane must touch this sphere");
            assert_eq!(
                hit.normal,
                [-1331.0 / 4096.0, 3638.0 / 4096.0, -1331.0 / 4096.0]
            );
            assert_eq!(hit.penetration_raw, depth);
        }
    }

    #[test]
    fn terrain_collision_wraps_signed_world_coordinates_toroidally() {
        let query = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let mut terrain = flat_terrain(0);
        for x in [255usize, 0] {
            for z in [0usize, 1] {
                terrain.cells[x * GRID_SIZE + z].height = 4; // +128 raw
            }
        }

        let hit = query
            .collide_terrain_raw_oriented(&terrain, [-1, 137, 128], IDENTITY3, &AnimVars::default())
            .unwrap()
            .expect("cell 255 must meet cell 0 across the signed-word seam");
        assert_eq!(hit.normal, [0.0, 1.0, 0.0]);
        assert_eq!(hit.penetration_raw, 1.0);
    }

    #[test]
    fn terrain_collision_preserves_query_gate_semantics() {
        let query = collision_entry(
            vec![[0, 0, 0, 0], [0, 0, 200, 0]],
            vec![
                0x8E, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0x95, 0, 10, 0, 0x8E, 0, 10, 0, 0, 0, 2, 0,
                0x88, 0,
            ],
        );
        assert!(query
            .collide_terrain_raw_oriented(
                &flat_terrain(0),
                [128, 0, 128],
                IDENTITY3,
                &AnimVars::default(),
            )
            .unwrap()
            .is_none());
    }

    #[test]
    fn oriented_collision_forward_transforms_aabb_center_not_extents() {
        let model = collision_entry(
            vec![[0, 100, 0, 0]],
            vec![
                0x8F, 0, 0, 0, 10, 0, 0, 0, 20, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0x88, 0,
            ],
        );
        let vars = AnimVars::default();
        let yaw_90 = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert!(model
            .collide_sphere_raw_oriented([14.0, 0.0, -100.0], 5, yaw_90, &vars, &())
            .unwrap()
            .is_some());
        // Inverse-transforming the query would incorrectly test this X
        // offset against the much larger local-Z extent and report a hit.
        assert!(model
            .collide_sphere_raw_oriented([20.0, 0.0, -100.0], 5, yaw_90, &vars, &())
            .unwrap()
            .is_none());
    }

    #[test]
    fn collision_program_child_uses_collision_hierarchy_and_attachment() {
        let parent = collision_entry(
            vec![[0, 100, 0, 0]],
            vec![0x05, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0x88, 0],
        );
        let child = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x8E, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0x88, 0],
        );
        let pool = vec![parent, child];
        let vars = AnimVars::default();
        assert!(pool[0]
            .collide_sphere_raw([100.0, 0.0, 0.0], 1, &vars, &pool)
            .unwrap()
            .is_some());
        assert!(pool[0]
            .collide_sphere_raw([0.0, 0.0, 0.0], 1, &vars, &pool)
            .unwrap()
            .is_none());
    }

    #[test]
    fn staged_effect_walk_is_linear_and_ignores_boxes() {
        let mut program = vec![0u8; 39];
        program[0] = 0x8E;
        program[4..8].copy_from_slice(&0x1234_04D2u32.to_le_bytes());
        program[8..10].copy_from_slice(&0u16.to_le_bytes());
        program[10] = 0x95;
        // Retail's staged walk ignores this otherwise-invalid branch target.
        program[12..14].copy_from_slice(&i16::MIN.to_le_bytes());
        program[14] = 0x8F;
        program[16..20].copy_from_slice(&1u32.to_le_bytes());
        program[20..24].copy_from_slice(&2u32.to_le_bytes());
        program[24..28].copy_from_slice(&3u32.to_le_bytes());
        // An unresolved box slot must remain irrelevant to the staged walk.
        program[28..30].copy_from_slice(&u16::MAX.to_le_bytes());
        program[30] = 0x8E;
        program[32..36].copy_from_slice(&77u32.to_le_bytes());
        program[36..38].copy_from_slice(&2u16.to_le_bytes());
        program[38] = 0x88;

        let model = collision_entry(vec![[0, 10, 20, 30], [0, 40, 50, 60]], program);
        let points = model
            .staged_effect_points_raw(
                StagedEffectRequest {
                    model_to_output_basis: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
                    model_origin_raw: [1_000.0, 2_000.0, 3_000.0],
                },
                &AnimVars::default(),
                &(),
            )
            .unwrap();

        assert_eq!(
            points,
            [
                StagedEffectPoint {
                    center_raw: [1_030.0, 2_020.0, 2_990.0],
                    scatter_radius_raw: 1_234,
                    source_slot: 0,
                },
                StagedEffectPoint {
                    center_raw: [1_060.0, 2_050.0, 2_960.0],
                    scatter_radius_raw: 77,
                    source_slot: 2,
                },
            ]
        );
    }

    #[test]
    fn staged_effect_child_composes_orientation_and_linked_slot_remaps() {
        let parent = collision_entry(
            vec![[0, 100, 10, 20], [0, 130, 40, 50]],
            vec![0x05, 0, 1, 0, 0x10, 0, 10, 0, 0, 0, 2, 0, 0x88],
        );
        let child = collision_entry(
            vec![[11, 1, 0, 0], [0, 10, 20, 30]],
            vec![
                0x8E, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0x8E, 0, 30, 0, 0, 0, 2, 0, 0x88,
            ],
        );
        let pool = vec![parent, child];
        let points = pool[0]
            .staged_effect_points_raw(
                StagedEffectRequest {
                    model_to_output_basis: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
                    model_origin_raw: [1_000.0, 2_000.0, 3_000.0],
                },
                &AnimVars::default(),
                &pool,
            )
            .unwrap();

        assert_eq!(
            points,
            [
                // Child slot 0 imports remap index 1, so it lands exactly on
                // parent slot 2 after the child and root transforms compose.
                StagedEffectPoint {
                    center_raw: [1_050.0, 2_040.0, 2_870.0],
                    scatter_radius_raw: 20,
                    source_slot: 0,
                },
                // The authored 0x10 child orientation negates local Y.
                StagedEffectPoint {
                    center_raw: [1_050.0, 1_990.0, 2_890.0],
                    scatter_radius_raw: 30,
                    source_slot: 2,
                },
            ]
        );
    }

    #[test]
    fn staged_effect_convex_records_skip_linearly_without_resolving_face_slots() {
        //19D90's cursor lengths differ from6AF20, especially8A and8D.
        let mut program = vec![0xFF; 55];
        program[0] = 0x8C;
        program[2..4].copy_from_slice(&i16::MIN.to_le_bytes());
        program[4] = 0x89; // align2+8 ->14
        program[14] = 0x8A; // align2 only ->16
        program[16] = 0x01; // source default advances one byte
        program[17] = 0x8D; // one-byte group terminator
        program[18] = 0x8B; // align2+2 ->22
        program[22] = 0x90; // align4+10 ->34
        program[34] = 0x36; // align2+8 ->44
        program[44] = 0x8E;
        program[48..52].copy_from_slice(&77u32.to_le_bytes());
        program[52..54].copy_from_slice(&0u16.to_le_bytes());
        program[54] = 0x88;
        let model = collision_entry(vec![[0, 10, 20, 30]], program);
        assert_eq!(
            model
                .staged_effect_points_raw(StagedEffectRequest::default(), &AnimVars::default(), &())
                .unwrap(),
            [StagedEffectPoint {
                center_raw: [10.0, 20.0, 30.0],
                scatter_radius_raw: 77,
                source_slot: 0
            }]
        );

        for (opcode, required) in [(0x8C, 4), (0x89, 10), (0x8A, 2), (0x90, 14)] {
            let mut truncated = vec![0xFF; required - 1];
            truncated[0] = opcode;
            let model = collision_entry(Vec::new(), truncated);
            assert_eq!(
                model
                    .staged_effect_points_raw(
                        StagedEffectRequest::default(),
                        &AnimVars::default(),
                        &()
                    )
                    .unwrap_err(),
                StagedEffectError::Truncated { pc: 0, opcode }
            );
        }
    }

    #[test]
    fn staged_effect_walk_reports_unsupported_truncated_and_recursive_streams() {
        let unsupported = collision_entry(Vec::new(), vec![0x30]);
        assert_eq!(
            unsupported
                .staged_effect_points_raw(
                    StagedEffectRequest::default(),
                    &AnimVars::default(),
                    &(),
                )
                .unwrap_err(),
            StagedEffectError::UnsupportedOpcode {
                pc: 0,
                opcode: 0x30,
            }
        );

        let parent = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x05, 0, 1, 0, 0, 0, 8, 0, 0, 0, 0x88],
        );
        let truncated_child = collision_entry(Vec::new(), vec![0x8E, 0, 0, 0]);
        let truncated_pool = vec![parent, truncated_child];
        assert_eq!(
            truncated_pool[0]
                .staged_effect_points_raw(
                    StagedEffectRequest::default(),
                    &AnimVars::default(),
                    &truncated_pool,
                )
                .unwrap_err(),
            StagedEffectError::Truncated {
                pc: 0,
                opcode: 0x8E,
            }
        );

        let recursive = collision_entry(
            vec![[0, 0, 0, 0]],
            vec![0x05, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0x88],
        );
        let recursive_pool = vec![recursive];
        assert_eq!(
            recursive_pool[0]
                .staged_effect_points_raw(
                    StagedEffectRequest::default(),
                    &AnimVars::default(),
                    &recursive_pool,
                )
                .unwrap_err(),
            StagedEffectError::RecursionLimit
        );
    }

    #[test]
    fn collision_program_reports_unsupported_and_truncated_streams() {
        let unsupported = collision_entry(Vec::new(), vec![0x90, 0, 0, 0]);
        assert_eq!(
            unsupported
                .collide_sphere_raw([0.0; 3], 1, &AnimVars::default(), &())
                .unwrap_err(),
            ModelCollisionError::UnsupportedOpcode {
                pc: 0,
                opcode: 0x90
            }
        );
        let truncated = collision_entry(Vec::new(), vec![0x8E, 0, 0, 0]);
        assert_eq!(
            truncated
                .collide_sphere_raw([0.0; 3], 1, &AnimVars::default(), &())
                .unwrap_err(),
            ModelCollisionError::Truncated {
                pc: 0,
                opcode: 0x8E
            }
        );
    }

    /// Interpret a bare stream with no records and default anim vars — the
    /// shape used by the opcode tests below. The empty record set is fine for
    /// streams that do not reference vertex slots.
    fn interpret_bare(words: &[u16], stats: &mut StreamStats) -> RawStream {
        let vars = AnimVars::default();
        let records: Vec<[i16; 4]> = Vec::new();
        interpret(
            words,
            &records,
            &[],
            ModelMaterializationContext::intrinsic(&vars, None),
            stats,
        )
        .0
    }

    #[test]
    fn decode_value_immediate_and_rotate() {
        let vars = AnimVars::default();
        // No rotation: value passes masked.
        assert_eq!(decode_value(0x0003, &vars), 0x0003);
        // rot = 1: (0x07 & 0xFF03 = 0x03) rotated left 1 = 0x06.
        assert_eq!(decode_value(0x0007, &vars), 0x0006);
        // rot = 8: byte swap of masked value (0x1200 -> 0x0012).
        assert_eq!(decode_value(0x1220, &vars), 0x0012);
        // Complement happens after every rotate case, including byte swap.
        assert_eq!(decode_value(0x1260, &vars), 0xFFED);
        // Bit 6 complements an immediate after rotation. `bigfuel` subtracts
        // this packed -48 constant from its animated glow-size register.
        assert_eq!(decode_value(0xF052, &vars), 0xFFD0);
        // Bit 7 only: dynamic callback -> 0 with default vars.
        assert_eq!(decode_value(0x0080, &vars), 0);
        // Bit 7 only: dynamic callback -> live value from AnimVars.dynamic.
        let mut vars_dyn = AnimVars::default();
        vars_dyn.dynamic[4] = 0x1234;
        assert_eq!(decode_value(0x0084, &vars_dyn), 0x1234);
        // Bits 7+6: register file.
        let mut vars2 = AnimVars::default();
        vars2.registers[5] = 0xBEEF;
        assert_eq!(decode_value(0x00C5, &vars2), 0xBEEF);
        // factory6 roof ribbons author 45. Packed decode would explode to 2048.
        assert_eq!(decode_value(45, &vars), 2048);
    }

    #[test]
    fn opcode_0x22_keeps_the_raw_size_short() {
        // FUN_00459000 passes param_2[1] to FUN_004594C0 without FUN_00470840.
        let words = vec![0x22, 608, 45, 62, 58, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        assert_eq!(raw.edges.len(), 1);
        assert_eq!(
            raw.edges[0].style,
            ModelEdgeStyle::Sprite {
                sprite_id: 608,
                size: 45,
            }
        );
        assert_eq!(raw.edges[0].r1, 62);
        assert_eq!(raw.edges[0].r2, 58);
        assert!(stats.is_clean());
    }

    #[test]
    fn complemented_immediate_keeps_bigfuel_billboard_sizes_positive() {
        // Exact register/billboard subsequence from global model 141
        // `bigfuel`: r1=sin(clock<<9)*18; r1-=(-48); both glow billboards
        // consume r1 as their unsigned size word.
        let words = [
            0x5D, 1, 0x0080, 0x400A, 0xDD, 1, 0x400E, 0x00C1, 0x1D, 1, 0x00C1, 0xF052, 0xB8, 42,
            617, 0x00C1, 0, 0xB8, 42, 617, 0x00C1, 0, 0,
        ];
        let records = Vec::new();
        for (tick, expected_size) in [(1_784, 41), (2_292, 38), (5_012, 62)] {
            let mut vars = AnimVars::default();
            vars.dynamic[0] = tick;
            let mut stats = StreamStats::default();
            let raw = interpret(
                &words,
                &records,
                &[],
                ModelMaterializationContext::intrinsic(&vars, None),
                &mut stats,
            )
            .0;
            assert_eq!(
                raw.billboards
                    .iter()
                    .map(|billboard| billboard.2)
                    .collect::<Vec<_>>(),
                [expected_size, expected_size]
            );
        }
    }

    #[test]
    fn register_sine_ops_keep_authored_newant_antenna_phase_words() {
        // Model302: r1=clock<<decode(11); r2=cos/sin(r1)*0x8000+0x8000.
        // Packed11 decodes12, so the authored clock repeats every16 ticks.
        // Original466870/466900 machine outputs on all32 clock phases.
        // The positive peak is65535; floating sin formerly wrapped it to0.
        let dd: [u16; 32] = [
            0, 12539, 23170, 30273, 32767, 30268, 23161, 12528, 0, 52996, 42365, 35262, 32768,
            35267, 42374, 53007, 0, 12539, 23170, 30273, 32767, 30268, 23161, 12528, 0, 52996,
            42365, 35262, 32768, 35267, 42374, 53007,
        ];
        let ed: [u16; 32] = [
            32767, 30268, 23161, 12528, 0, 52996, 42365, 35262, 32768, 35267, 42374, 53007, 0,
            12539, 23170, 30273, 32767, 30268, 23161, 12528, 0, 52996, 42365, 35262, 32768, 35267,
            42374, 53007, 0, 12539, 23170, 30273,
        ];
        for (op, expected) in [(0xDD, dd), (0xED, ed)] {
            for (tick, product) in expected.into_iter().enumerate() {
                let mut vars = AnimVars::default();
                vars.dynamic[0] = tick as i32;
                apply_register_op(0x5D, &[1, 0x0080, 0x000B], &mut vars);
                assert_eq!(vars.registers[1], (tick as u16) << 12);
                apply_register_op(op, &[2, 0x8000, 0x00C1], &mut vars);
                assert_eq!(vars.registers[2], product);
                apply_register_op(0x0D, &[2, 0x00C2, 0x8000], &mut vars);
                assert_eq!(vars.registers[2], product.wrapping_add(0x8000));
            }
        }
    }

    #[test]
    fn register_op_drives_following_mount_rotation() {
        // reg[0] = dynamic[0] + 0; op-0x5C then consumes reg[0] as a packed
        // angle and the code-6 child copies that mount basis.
        let words = [
            0x0D, 0, 0x80, 0, // r0 = callback[0]
            0x5C, 0, 2, 0xC0, // rotate rows 0/1 by r0
            0x0E, 6, 5, 8, 0, // child model 5, code-6 orientation
            0,
        ];
        let records = Vec::new();
        let mut vars = AnimVars::default();
        vars.dynamic[0] = 0x4000; // quarter turn
        let mut stats = StreamStats::default();
        let raw = interpret(
            &words,
            &records,
            &[],
            ModelMaterializationContext::intrinsic(&vars, None),
            &mut stats,
        )
        .0;
        let m = raw.instances[0].orientation;
        assert!(m[0][0].abs() < 1.0e-12);
        assert!((m[0][1] + 1.0).abs() < 1.0e-12);
        assert!((m[1][0] - 1.0).abs() < 1.0e-12);
        assert!(m[1][1].abs() < 1.0e-12);
        assert_eq!(raw.instances[0].registers[0], 0x4000);
    }

    #[test]
    fn mount_reset_is_scoped_to_following_code_six_children() {
        // The first child inherits the animated mount. FUN_00466fc0 then
        // restores identity, so the second child is not tilted with it.
        let words = [
            0x0D, 0, 0x80, 0, // r0 = callback[0]
            0x5C, 0, 2, 0xC0, // rotate rows 0/1 by r0
            0x0E, 6, 5, 8, 0,    // child model 5, code-6 orientation
            0x0F, // reset current mount basis
            0x0E, 6, 6, 8, 0, // child model 6, code-6 orientation
            0,
        ];
        let records = Vec::new();
        let mut vars = AnimVars::default();
        vars.dynamic[0] = 0x4000; // quarter turn
        let mut stats = StreamStats::default();
        let raw = interpret(
            &words,
            &records,
            &[],
            ModelMaterializationContext::intrinsic(&vars, None),
            &mut stats,
        )
        .0;

        assert_eq!(raw.instances.len(), 2);
        assert_eq!(raw.instances[0].model_id, 5);
        assert_ne!(raw.instances[0].orientation, IDENTITY3);
        assert_eq!(raw.instances[1].model_id, 6);
        assert_eq!(raw.instances[1].orientation, IDENTITY3);
    }

    fn assert_mount_matrix_close(actual: [[f64; 3]; 3], expected: [[f64; 3]; 3]) {
        for (actual, expected) in actual
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            assert!(
                (actual - expected).abs() < 1.0e-12,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn additional_mount_rotation_decodes_signed_callback_and_register_angles_on_each_axis() {
        let mut vars = AnimVars::default();
        vars.dynamic[9] = -0x4000;
        vars.registers[7] = 0xC000;
        for (axis, expected) in [
            (0, [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]]),
            (1, [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]]),
            (2, [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]),
            // Retail routes every axis other than 1/2 through its X case.
            (0xFFFF, [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]]),
        ] {
            for operand in [0x89, 0xC7, 0xC000] {
                let words = [0x5C, 0, 0, 0, 0x1C, axis, operand, 0x0E, 6, 5, 8, 0, 0];
                let raw = interpret(
                    &words,
                    &[],
                    &[],
                    ModelMaterializationContext::intrinsic(&vars, None),
                    &mut StreamStats::default(),
                )
                .0;
                assert_eq!(raw.instances.len(), 1);
                assert_mount_matrix_close(raw.instances[0].orientation, expected);
            }
        }
    }

    #[test]
    fn additional_mount_rotations_compose_after_a_mirror_and_only_affect_later_children() {
        let words = [
            0x5C, 0x20, 2, 0x4000, // mirrored X, then a Z quarter turn
            0x0E, 6, 5, 8, 0, 0x1C, 0, 0x4000, // additional X quarter turn
            0x0E, 6, 6, 8, 0, 0x1C, 0,
            0xC000, // the inverse returns to the first child's mount
            0x0E, 6, 7, 8, 0, 0,
        ];
        let raw = interpret_bare(&words, &mut StreamStats::default());
        assert_eq!(raw.instances.len(), 3);
        let initial = [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        assert_mount_matrix_close(raw.instances[0].orientation, initial);
        assert_mount_matrix_close(
            raw.instances[1].orientation,
            [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
        assert_mount_matrix_close(raw.instances[2].orientation, initial);
    }

    #[test]
    fn additional_rotation_preserves_a_vertex_derived_mount() {
        let records = [[0, 0, 0, 0], [0, 10, 0, 0], [0, 0, 10, 0]];
        let words = [0x3C, 0, 2, 4, 0x1C, 2, 0x4000, 0x0E, 6, 5, 8, 0, 0];
        let raw = interpret(
            &words,
            &records,
            &[],
            ModelMaterializationContext::intrinsic(&AnimVars::default(), None),
            &mut StreamStats::default(),
        )
        .0;
        assert_mount_matrix_close(
            raw.instances[0].orientation,
            [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
    }

    #[test]
    fn truncated_additional_mount_rotation_stops_after_the_complete_prefix() {
        let prefix = [0x5C, 0, 2, 0x4000, 0x0E, 6, 5, 8, 0];
        for suffix in [&[0x1C][..], &[0x1C, 0][..]] {
            let words = [prefix.as_slice(), suffix].concat();
            let raw = interpret_bare(&words, &mut StreamStats::default());
            assert_eq!(raw.instances.len(), 1);
            assert_mount_matrix_close(
                raw.instances[0].orientation,
                [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            );
        }
    }

    #[test]
    fn eval_var_modes() {
        let vars = AnimVars::default();
        assert_eq!(eval_var(0x3F, &vars), 0x3F);
        assert_eq!(eval_var(0x40 | 0x20, &vars), 0x20 << 10);
        assert_eq!(eval_var(0x80, &vars), 0); // dynamic, default 0
        assert_eq!(eval_var(0xC1, &vars), 0); // register file, default 0
                                              // Live sources.
        let mut v = AnimVars::default();
        v.dynamic[5] = 0x8000;
        v.registers[1] = 0x0333;
        assert_eq!(eval_var(0x85, &v), 0x8000); // 0x80 | idx 5
        assert_eq!(eval_var(0xC1, &v), 0x0333); // 0xC0 | idx 1
    }

    #[test]
    fn resolver_plain_and_mirror() {
        let records = vec![[0i16, 100, 50, 25]];
        let vars = AnimVars::default();
        let mut stats = StreamStats::default();
        let mut r = SlotResolver::new(&records, &vars);
        assert_eq!(
            r.resolve(0, &mut stats).map(|slot| slot.position_raw),
            Some([100.0, 50.0, 25.0])
        );
        assert_eq!(
            r.resolve(1, &mut stats).map(|slot| slot.position_raw),
            Some([-100.0, 50.0, 25.0])
        );
    }

    #[test]
    fn face_cull_plane_mirrors_normal_and_authored_anchor_together() {
        let records = vec![[0i16, 100, 50, 25]];
        let pool = vec![[0i16, 4_096, 512, -256]];
        let vars = AnimVars::default();
        let mut stats = StreamStats::default();
        let mut resolver = SlotResolver::new(&records, &vars);

        assert_eq!(
            pool_face_cull(&pool, 2, &mut resolver, &mut stats),
            ModelFaceCull::Plane(ModelFaceCullPlane {
                anchor_raw: [100.0, 50.0, 25.0],
                native_anchor_raw: Some([100, 50, 25]),
                normal_raw: [4_096, 512, -256],
            })
        );
        assert_eq!(
            pool_face_cull(&pool, 3, &mut resolver, &mut stats),
            ModelFaceCull::Plane(ModelFaceCullPlane {
                anchor_raw: [-100.0, 50.0, 25.0],
                native_anchor_raw: Some([-100, 50, 25]),
                normal_raw: [-4_096, 512, -256],
            })
        );
    }

    #[test]
    fn malformed_face_cull_anchor_stays_explicitly_unavailable() {
        let records = vec![[0i16, 100, 50, 25]];
        let pool = vec![[6i16, 4_096, 0, 0]];
        let vars = AnimVars::default();
        let mut stats = StreamStats::default();
        let mut resolver = SlotResolver::new(&records, &vars);
        assert_eq!(
            pool_face_cull(&pool, 2, &mut resolver, &mut stats),
            ModelFaceCull::Unresolved
        );
        assert_eq!(
            pool_face_cull(&pool, 0, &mut resolver, &mut stats),
            ModelFaceCull::AlwaysVisible
        );
    }

    #[test]
    fn resolver_linked_model_uses_supplied_slot_table() {
        let records = vec![[11i16, 1, 0, 0]];
        let vars = AnimVars::default();
        let linked = vec![
            [Some(ResolvedModelSlot::clear([0.0; 3])); 2],
            [
                Some(ResolvedModelSlot::clear([10.0, 20.0, 30.0])),
                Some(ResolvedModelSlot::clear([-10.0, 20.0, 30.0])),
            ],
        ];

        let mut stats = StreamStats::default();
        let mut no_link = SlotResolver::new(&records, &vars);
        assert_eq!(no_link.resolve(0, &mut stats), None);

        let mut linked_resolver = SlotResolver::with_linked(&records, &vars, Some(&linked));
        assert_eq!(
            linked_resolver
                .resolve(0, &mut stats)
                .map(|slot| slot.position_raw),
            Some([10.0, 20.0, 30.0])
        );
        assert_eq!(
            linked_resolver
                .resolve(1, &mut stats)
                .map(|slot| slot.position_raw),
            Some([-10.0, 20.0, 30.0])
        );
    }

    #[test]
    fn resolver_midpoint_and_parallelogram() {
        let records = vec![
            [0i16, 100, 0, 0], // slots 0/1
            [0i16, 0, 100, 0], // slots 2/3
            [5i16, 0, 0, 2],   // slot 4: midpoint(slot 0, slot 2)
            [6i16, 0, 2, 4],   // slot 6: slot 0 + slot 2 - slot 4
        ];
        let vars = AnimVars::default();
        let mut stats = StreamStats::default();
        let mut r = SlotResolver::new(&records, &vars);
        assert_eq!(
            r.resolve(4, &mut stats).map(|slot| slot.position_raw),
            Some([50.0, 50.0, 0.0])
        );
        // Mirror of the midpoint pulls mirrored sources.
        assert_eq!(
            r.resolve(5, &mut stats).map(|slot| slot.position_raw),
            Some([-50.0, 50.0, 0.0])
        );
        assert_eq!(
            r.resolve(6, &mut stats).map(|slot| slot.position_raw),
            Some([50.0, 50.0, 0.0])
        );
    }

    #[test]
    fn resolver_alias_and_reflect() {
        let records = vec![[0i16, 10, 20, 30], [12i16, 0, 0, 0], [2i16, 0, 0, 0]];
        let vars = AnimVars::default();
        let mut stats = StreamStats::default();
        let mut r = SlotResolver::new(&records, &vars);
        assert_eq!(
            r.resolve(2, &mut stats).map(|slot| slot.position_raw),
            Some([10.0, 20.0, 30.0])
        );
        assert_eq!(
            r.resolve(4, &mut stats).map(|slot| slot.position_raw),
            Some([-10.0, -20.0, -30.0])
        );
    }

    #[test]
    fn interpret_tri_and_mirror() {
        // Plain tri (slots 0,2,4), then mirrored tri, then stop.
        let words = vec![
            0x03, 0, 1, 0, 2, 4, // tri
            0x07, 0, 1, 0, 2, 4, // tri + mirror instance
            0x00,
        ];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        assert_eq!(raw.tris.len(), 3);
        assert_eq!(raw.tris[0].refs, [0, 2, 4]);
        assert_eq!(raw.tris[1].refs, [0, 2, 4]);
        // Mirror: authored corner order retained, each ref XOR 1.
        assert_eq!(raw.tris[2].refs, [1, 3, 5]);
        assert_eq!(raw.tris[2].normal, 0);
        assert_eq!(raw.tris[2].uvs, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        assert!(stats.is_clean());
    }

    #[test]
    fn interpret_mirrored_quad_restarts_retail_corner_order() {
        let words = vec![0xC8, 0, 2, 0, 2, 4, 6, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);

        assert_eq!(raw.tris.len(), 4);
        assert_eq!(raw.tris[0].refs, [0, 2, 4]);
        assert_eq!(raw.tris[1].refs, [0, 4, 6]);
        assert_eq!(raw.tris[2].refs, [1, 3, 5]);
        assert_eq!(raw.tris[3].refs, [1, 5, 7]);
        assert_eq!(raw.tris[2].uvs, raw.tris[0].uvs);
        assert_eq!(raw.tris[3].uvs, raw.tris[1].uvs);
        assert!(stats.is_clean());
    }

    #[test]
    fn quad_triangles_retain_the_same_authored_cull_plane() {
        let records = vec![
            [0i16, 0, 0, 0],
            [0i16, 100, 0, 0],
            [0i16, 100, 100, 0],
            [0i16, 0, 100, 0],
        ];
        let pool = vec![[0i16, 0, 0, 4_096]];
        let words = vec![0x04, 7, 2, 0, 2, 4, 6, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        let vars = AnimVars::default();
        let mut resolver = SlotResolver::new(&records, &vars);
        let geometry = resolve_stream(&mut resolver, &raw, &pool, &mut stats);

        assert_eq!(geometry.triangles.len(), 2);
        assert_eq!(geometry.face_cull.len(), 2);
        assert_eq!(geometry.face_cull[0], geometry.face_cull[1]);
        assert_eq!(
            geometry.face_cull[0],
            ModelFaceCull::Plane(ModelFaceCullPlane {
                anchor_raw: [0.0, 0.0, 0.0],
                native_anchor_raw: Some([0, 0, 0]),
                normal_raw: [0, 0, 4_096],
            })
        );
    }

    #[test]
    fn all_type13_faces_retain_canonical_face_metadata() {
        let records = vec![
            [0i16, 0, 0, 0],
            [0i16, 100, 0, 0],
            [0i16, 0, 0, 100],
            [13i16, 0, 0, 0],
            [13i16, 2, 0, 0],
            [13i16, 4, 0, 0],
        ];
        let words = vec![0x03, 7, 0, 6, 8, 10, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        let vars = AnimVars::default();
        let mut resolver = SlotResolver::new(&records, &vars);
        let geometry = resolve_stream(&mut resolver, &raw, &[], &mut stats);

        assert_eq!(geometry.vertex_type_flags, vec![13, 13, 13]);
        assert_eq!(geometry.triangles, vec![[0, 1, 2]]);
        assert_eq!(geometry.face_materials, vec![7]);
        assert_eq!(
            geometry.face_uvs,
            vec![[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]]
        );
        assert_eq!(geometry.face_cull.len(), 1);
        assert_eq!(geometry.face_corner_normals.len(), 1);
        assert_eq!(geometry.face_shading, vec![ModelFaceShading::Flat]);
        assert!(geometry.shadow_triangles.is_empty());
    }

    #[test]
    fn mixed_type13_faces_remain_body_geometry() {
        let records = vec![[0i16, 0, 0, 0], [0i16, 100, 0, 0], [13i16, 0, 0, 0]];
        let words = vec![0x03, 7, 0, 0, 2, 4, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        let vars = AnimVars::default();
        let mut resolver = SlotResolver::new(&records, &vars);
        let geometry = resolve_stream(&mut resolver, &raw, &[], &mut stats);

        assert_eq!(geometry.vertex_type_flags, vec![0, 0, 13]);
        assert_eq!(geometry.triangles.len(), 1);
        assert_eq!(geometry.face_materials, vec![7]);
        assert!(geometry.shadow_triangles.is_empty());
    }

    #[test]
    fn interpret_quad_splits() {
        let words = vec![0x04, 0, 2, 0, 2, 4, 6, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        assert_eq!(raw.tris.len(), 2);
        assert_eq!(raw.tris[0].refs, [0, 2, 4]);
        assert_eq!(raw.tris[1].refs, [0, 4, 6]);
        assert_eq!(raw.tris[0].uvs, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        assert_eq!(raw.tris[1].uvs, [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    }

    #[test]
    fn interpret_gouraud_quad_preserves_corner_normals() {
        let words = vec![0xA4, 409, 2, 0, 2, 4, 6, 10, 12, 14, 16, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        assert_eq!(raw.tris.len(), 2);
        assert_eq!(raw.tris[0].corner_normals, [10, 12, 14]);
        assert_eq!(raw.tris[1].corner_normals, [10, 14, 16]);
        assert_eq!(face_material(raw.tris[0].material), (409, true));
    }

    #[test]
    fn interpret_lod_jump() {
        // 0x14: static view_dist = 0 -> jump taken over the far-LOD tri.
        let words = vec![
            0x14, 9, 0, 0, // jump +9 words from pc+1 -> lands on the 2nd tri
            0x03, 0, 1, 0, 2, 4, // skipped far tri
            0x03, 0, 1, 6, 8, 10, // near tri
            0x00,
        ];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        assert_eq!(raw.tris.len(), 1);
        assert_eq!(raw.tris[0].refs, [6, 8, 10]);
        assert_eq!(stats.jumps_taken, 1);
    }

    #[test]
    fn interpret_billboard() {
        let words = vec![0x68, 4, 7, 0x0003, 0x0000, 0x00];
        let mut stats = StreamStats::default();
        let raw = interpret_bare(&words, &mut stats);
        assert_eq!(raw.billboards.len(), 1);
        let (slot, id, size, angle, textured) = raw.billboards[0];
        assert_eq!((slot, id, size, angle, textured), (4, 7, 3, 0, false));
    }

    #[test]
    fn interpret_unknown_op_recorded() {
        let words = vec![0x99, 0, 0];
        let mut stats = StreamStats::default();
        let _ = interpret_bare(&words, &mut stats);
        assert_eq!(stats.unknown_ops.get(&0x99), Some(&1));
        assert!(!stats.is_clean());
    }

    /// Build a synthetic single-entry Section 8 with a tf-8 morph vertex, so
    /// the live morph variable can be exercised through `ModelEntry`.
    fn morph_entry() -> ModelEntry {
        // Records: slot 0/1 = P0 (0,0,0); slot 2/3 = P1 (200,0,0);
        //          slot 4/5 = tf-8 lerp(P0, P1) driven by register #0 (a=0xC0).
        let records = vec![
            [0i16, 0, 0, 0],    // slots 0/1
            [0i16, 200, 0, 0],  // slots 2/3
            [8i16, 0xC0, 0, 2], // slot 4: lerp(slot0, slot2), t from reg #0
        ];
        // One triangle over slots 0, 2, 4 so the morph vertex materializes.
        let words = vec![0x03u16, 0, 0, 0, 2, 4, 0x00];
        let mut entry = ModelEntry {
            index: 0,
            cmd_word_count: words.len() as u16,
            extra_count: 0,
            flags: 0x40,
            slot_count: 6,
            face_val: 2,
            radius: 0,
            collision_radius_raw: 0,
            collision_program: Vec::new(),
            records,
            normal_pool: Vec::new(),
            cmd_words: words,
            has_view_commands: false,
            vertices: Vec::new(),
            vertex_type_flags: Vec::new(),
            vertex_projection: Vec::new(),
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: Vec::new(),
            face_vertices: Vec::new(),
            normals: Vec::new(),
            face_cull: Vec::new(),
            face_materials: Vec::new(),
            face_uvs: Vec::new(),
            face_corner_normals: Vec::new(),
            face_shading: Vec::new(),
            shadow_triangles: Vec::new(),
            edges: Vec::new(),
            billboards: Vec::new(),
            instances: Vec::new(),
            painter_program: Vec::new(),
            name: None,
        };
        let m = entry.materialize(&AnimVars::default());
        entry.vertices = m.vertices;
        entry.vertex_projection = m.vertex_projection;
        entry.vertex_clip = m.vertex_clip;
        entry.vertex_surface_origin = m.vertex_surface_origin;
        entry.triangles = m.triangles;
        entry.face_vertices = m.face_vertices;
        entry
    }

    #[test]
    fn materialize_tf8_morph_moves_vertex() {
        let entry = morph_entry();
        // The tf-8 vertex is the 3rd referenced slot (index 2 in vertices).
        // t = 0: coincides with P0 = (0,0,0).
        let at0 = entry.materialize(&AnimVars::default());
        assert_eq!(at0.vertices[2], [0.0, 0.0, 0.0]);

        // Register #0 = 0x8000 gives the retail half-morph endpoint.
        let mut vars = AnimVars::default();
        vars.registers[0] = 0x8000;
        let half = entry.materialize(&vars);
        assert_eq!(half.vertices[2], [100.0, 0.0, 0.0]);

        // A different value moves it further — confirms the var, not a constant.
        vars.registers[0] = 0xC000;
        let more = entry.materialize(&vars);
        assert_eq!(more.vertices[2], [150.0, 0.0, 0.0]);
        assert_ne!(more.vertices[2], half.vertices[2]);

        // The u16 maximum approaches, but cannot quite reach, the endpoint;
        // the retail expression deliberately returns even coordinates.
        vars.registers[0] = u16::MAX;
        let near_full = entry.materialize(&vars);
        assert_eq!(near_full.vertices[2], [198.0, 0.0, 0.0]);
    }

    #[test]
    fn tf8_retail_shift_preserves_signed_bias() {
        assert_eq!(retail_tf8_axis(0.0, 3.0, 0x8000), 0.0);
        assert_eq!(retail_tf8_axis(3.0, 0.0, 0x8000), 1.0);
    }

    #[test]
    fn tf9_bezier_uses_retail_parameter_scale() {
        // tf-9 orders its controls as b, c, c+2, b+2. Arrange a smoothstep
        // 0,0,200,200 curve and evaluate register #0 at the retail midpoint.
        // The exact staged shifts quantize the quadratic and cubic terms, so
        // this deliberately lands at 92 rather than the ideal-float 100.
        let records = vec![
            [0i16, 0, 0, 0],    // slot 0: P0
            [0i16, 200, 0, 0],  // slot 2: P3
            [0i16, 0, 0, 0],    // slot 4: P1
            [0i16, 200, 0, 0],  // slot 6: P2
            [9i16, 0xC0, 0, 4], // slot 8: curve, register #0
        ];
        let mut vars = AnimVars::default();
        vars.registers[0] = 0x8000;
        let mut resolver = SlotResolver::new(&records, &vars);
        let mut stats = StreamStats::default();
        assert_eq!(
            resolver
                .resolve(8, &mut stats)
                .map(|slot| slot.position_raw),
            Some([92.0, 0.0, 0.0])
        );
    }

    #[test]
    fn tf9_bezier_matches_retail_staged_signed_shifts() {
        // Collinear, evenly spaced controls make the polynomial exactly
        // linear. Both directions exercise the signed EDX:EAX >>31 behavior.
        assert_eq!(retail_tf9_axis(0.0, 100.0, 200.0, 300.0, 0x8000), 150.0);
        assert_eq!(retail_tf9_axis(300.0, 200.0, 100.0, 0.0, 0x8000), 150.0);

        // A negative non-integral term rounds toward -infinity, as the x86
        // arithmetic shift does; truncation toward zero would produce 100.
        assert_eq!(retail_tf9_axis(200.0, 200.0, 0.0, 0.0, 0x8000), 96.0);
    }

    #[test]
    fn permute_negate_matches_c_table() {
        // FUN_00467730 with parent = identity.
        // code 0x10: code&7=0 (identity permute) + negate source row 1
        //            -> diag(1, -1, 1).
        let m = permute_negate(&IDENTITY3, 0x10);
        assert_eq!(m, [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]]);

        // code 0x30 (0x20|0x10): identity permute + negate rows 0 and 1
        //            -> diag(-1, -1, 1).
        let m = permute_negate(&IDENTITY3, 0x30);
        assert_eq!(m, [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]]);

        // code 0x01: permutation map [2,0,1] (dst row r takes source row map[r]),
        //            no negation.
        //   R0 <- src2 = (0,0,1); R1 <- src0 = (1,0,0); R2 <- src1 = (0,1,0).
        let m = permute_negate(&IDENTITY3, 0x01);
        assert_eq!(m, [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);

        // code 0x00: pure identity.
        assert_eq!(permute_negate(&IDENTITY3, 0x00), IDENTITY3);
    }

    #[test]
    fn mount_basis_is_orthonormal() {
        // A=(0,0,0), B=(10,0,0), C=(0,10,0): e=+X, u=normalize((C-A)x e),
        // (0,10,0)x(1,0,0) = (0*0-0*0, 0*1-0*0, 0*0-10*1) = (0,0,-10) -> (0,0,-1);
        // w = e x u = (1,0,0)x(0,0,-1) = (0*-1-0*0, 0*0-1*-1, 1*0-0*0) = (0,1,0).
        let basis = mount_basis([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0])
            .expect("non-degenerate");
        assert_eq!(basis[0], [0.0, 0.0, -1.0]); // u row
        assert_eq!(basis[1], [0.0, 1.0, 0.0]); // w row
        assert_eq!(basis[2], [1.0, 0.0, 0.0]); // e row (= normalized B-A)
                                               // Degenerate (A==B) -> None.
        assert!(mount_basis([0.0; 3], [0.0; 3], [0.0, 10.0, 0.0]).is_none());
    }
    #[test]
    fn native_instance_frames_retain_mount_reset_rotation_and_register_snapshot() {
        let model = ModelEntry {
            records: vec![[0, 0, 0, 0], [0, 0, 60, 0]],
            // Reset, rotate its existing VIEW mount, emit code6 child, then
            // install a signed parent permutation and emit a second child.
            cmd_words: vec![
                0x0F, 0x1C, 2, 0x2000, 0x0E, 6, 220, 8, 0, 0x5C, 0x21, 1, 0x3000, 0x0E, 6, 222, 8,
                2, 0,
            ],
            ..ModelEntry::default()
        };
        let material = model.materialize(&AnimVars::default());
        assert_eq!(material.instances.len(), 2);
        assert_eq!(
            material.instances[0].native_frame,
            Some(ModelNativeInstanceFrame::Mount(vec![
                ModelNativeMountCommand::Identity,
                ModelNativeMountCommand::Rotate {
                    axis: 2,
                    angle: 0x2000
                }
            ]))
        );
        assert_eq!(
            material.instances[1].native_frame,
            Some(ModelNativeInstanceFrame::Mount(vec![
                ModelNativeMountCommand::Parent(0x21),
                ModelNativeMountCommand::Rotate {
                    axis: 1,
                    angle: 0x3000
                }
            ]))
        );
        assert_eq!(material.instances[0].linked_slots, [0]);
        assert_eq!(material.instances[1].linked_slots, [2]);
    }
}

/// A native model-local painter adapter must reject children before invoking
/// any live H/E callback. The shared reached-instance visitor is not owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativePainterProgramBoundary {
    ChildInstance { word_offset: usize },
    UnsupportedDistanceBranch { word_offset: usize, opcode: u16 },
    UnknownOpcode { word_offset: usize, opcode: u16 },
    MalformedInstruction { word_offset: usize },
}

impl ModelEntry {
    /// Inspect every possible command path without evaluating operands,
    /// resolving vertices or invoking presentation callbacks. This is a
    /// conservative ownership gate, not another model interpreter.
    pub fn native_painter_root_preflight(
        &self,
    ) -> std::result::Result<(), NativePainterProgramBoundary> {
        let words = &self.cmd_words;
        let mut pending = vec![0usize];
        let mut seen = std::collections::HashSet::new();
        while let Some(pc) = pending.pop() {
            if !seen.insert(pc) {
                continue;
            }
            let Some(&op) = words.get(pc) else {
                return Err(NativePainterProgramBoundary::MalformedInstruction { word_offset: pc });
            };
            if op == 0 {
                continue;
            }
            if op == 0x0E {
                return Err(NativePainterProgramBoundary::ChildInstance { word_offset: pc });
            }
            if matches!(op, 0x13 | 0x14) {
                // The compatibility interpreter retains static distance
                // selection. Native distance-branch operands are not owned.
                return Err(NativePainterProgramBoundary::UnsupportedDistanceBranch {
                    word_offset: pc,
                    opcode: op,
                });
            }
            let fixed = match op {
                0x03 | 0x43 | 0x83 | 0xC3 | 0x07 | 0x47 | 0x87 | 0xC7 => Some(6),
                0x04 | 0x44 | 0x84 | 0xC4 | 0x08 | 0x48 | 0x88 | 0xC8 => Some(7),
                0x23 | 0xA3 | 0x27 | 0xA7 => Some(9),
                0x24 | 0xA4 | 0x28 | 0xA8 => Some(11),
                0x02 => Some(4),
                0x22 | 0x68 | 0xE8 | 0x78 | 0xB8 | 0xF8 => Some(5),
                0x46 | 0x15 => Some(2),
                0xA6 | 0x1C => Some(3),
                0xC6 | 0x3C | 0x5C => Some(4),
                0x66 | 0x86 | 0xE6 | 0x0F | 0x10 => Some(1),
                0x0B | 0x0C => Some(3),
                0x2B | 0x2C => Some(4),
                op if op <= 0xFF && op & 0x0F == 0x0D => Some(4),
                0x06 | 0x26 | 0x38 => None,
                _ => {
                    return Err(NativePainterProgramBoundary::UnknownOpcode {
                        word_offset: pc,
                        opcode: op,
                    })
                }
            };
            let next = if let Some(length) = fixed {
                pc.checked_add(length).filter(|&next| next <= words.len())
            } else {
                words[pc + 1..]
                    .iter()
                    .position(|&word| word == 0xFFFF)
                    .map(|length| pc + length + 2)
            }
            .ok_or(NativePainterProgramBoundary::MalformedInstruction { word_offset: pc })?;
            if matches!(op, 0x0B | 0x0C | 0x2B | 0x2C) {
                // Visit both possible paths. No source condition or current
                // register value can authorize skipping a child boundary.
                let offset = if matches!(op, 0x0B | 0x0C) {
                    words[pc + 1] as i16 as isize
                } else {
                    words[pc + 1] as isize
                };
                let target = (pc + 1)
                    .checked_add_signed(offset)
                    .filter(|&target| target < words.len())
                    .ok_or(NativePainterProgramBoundary::MalformedInstruction {
                        word_offset: pc,
                    })?;
                pending.push(target);
            }
            pending.push(next);
        }
        Ok(())
    }
}
