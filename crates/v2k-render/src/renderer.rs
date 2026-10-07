use v2k_formats::models::{
    Billboard, ModelEntry, ModelFaceCull, ModelFaceShading, ModelFaceVertices,
    ModelVertexProjection,
};
use v2k_formats::system::{FogGradientEntry, PaletteEntry};
use v2k_formats::terrain::TerrainGrid;

use crate::camera::Camera;
use crate::config::ScalingMode;
use crate::projection::ProjectionEffect;
use crate::ui_mapping::UiSubmissionPolicy;

/// Build a row-major 3×3 orientation matrix from yaw/pitch/roll (radians),
/// composed as `R = Ry(yaw) · Rx(pitch) · Rz(roll)`. Applied by the renderer
/// as `world = R · v_local`. Yaw about +Y preserves the legacy
/// `glRotatef(yaw, 0,1,0)` transform exactly.
pub fn orientation_from_ypr(yaw: f32, pitch: f32, roll: f32) -> [[f32; 3]; 3] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let (sr, cr) = roll.sin_cos();
    // Rotation about +Y (yaw), +X (pitch), +Z (roll).
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rx = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
    let rz = [[cr, -sr, 0.0], [sr, cr, 0.0], [0.0, 0.0, 1.0]];
    mat3_mul(mat3_mul(ry, rx), rz)
}

/// Row-major 3×3 matrix product.
pub fn mat3_mul(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    out
}

/// Convert a resolved model-space `[[f64;3];3]` orientation (e.g. an
/// op-0x0E [`ModelInstance::orientation`](v2k_formats::models::ModelInstance))
/// to the `f32` form the renderer consumes.
pub fn orientation_f32(m: [[f64; 3]; 3]) -> [[f32; 3]; 3] {
    [
        [m[0][0] as f32, m[0][1] as f32, m[0][2] as f32],
        [m[1][0] as f32, m[1][1] as f32, m[1][2] as f32],
        [m[2][0] as f32, m[2][1] as f32, m[2][2] as f32],
    ]
}

/// Available renderer backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    OpenGL,
    Software,
}

impl RenderBackend {
    pub fn name(&self) -> &str {
        match self {
            RenderBackend::OpenGL => "opengl",
            RenderBackend::Software => "software",
        }
    }
}

/// Opaque handle to a persistent GPU texture created with
/// [`Renderer::create_texture`]. Valid until [`Renderer::destroy_texture`]
/// or renderer teardown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureId(pub u32);

/// One Section-3 record in its retail memory form.
#[derive(Debug, Clone)]
pub struct NativeSprite<'a> {
    /// Render flags (record `+0x04` low byte).
    pub flags: u16,
    pub shade_count: u16,
    pub width: u16,
    pub height: u16,
    pub texels: NativeTexels<'a>,
}

/// A record's texels: indices with display-format palette words (row-major
/// shade rows of sixteen, or a flat table), or raw display-format words.
#[derive(Debug, Clone)]
pub enum NativeTexels<'a> {
    Indexed {
        indices: &'a [u8],
        palette: NativePalette,
    },
    Raw {
        words: &'a [u16],
    },
}

/// A Section-3 palette block shared by its records, and where one record's
/// palette starts in it.
#[derive(Debug, Clone)]
pub struct NativePalette {
    pub block: std::sync::Arc<[u16]>,
    pub start: usize,
}

/// Indexed Section-3 image plus its complete 32×16 shade ramp.
///
/// `fallback_rgba` keeps the texture handle usable by renderer paths that do
/// not understand indexed materials (notably software fallbacks). A capable
/// model or sprite backend samples `indices` and `palette_rgba`
/// separately so Gouraud shade rows are selected after interpolation, as in
/// the retail software filler.
#[derive(Debug, Clone, Copy)]
pub struct IndexedModelTexture<'a> {
    pub fallback_rgba: &'a [u8],
    pub indices: &'a [u8],
    pub palette_rgba: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub transparent_zero: bool,
}

/// Resolved material for one materialized model triangle.
///
/// Palette-backed faces retain their authored word for the solid filler.
/// Sprite-backed faces use a persistent Section-3 `texture`; `color` is a
/// presentation tint (normally white) for either source, while `emissive`
/// adds coloured light after ordinary directional shading. Texture coordinates
/// and Gouraud corner normals stay in the model
/// data because they are geometry, not resource-cache state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceMaterial {
    /// Presentation tint, or a direct RGB colour when neither palette nor
    /// texture is present (for example a diagnostic fallback).
    pub color: [f32; 3],
    /// Authored Section-7 word. Uniform solid lighting adds a packed shade
    /// contribution to this word; multiplying its decoded RGB is different.
    pub palette_rgb555: Option<u16>,
    pub emissive: [f32; 3],
    pub texture: Option<TextureId>,
    /// Framebuffer compositing selected by a sprite-backed face's Section-3
    /// flags. Solid Section-7 faces use [`WorldSpriteBlend::Masked`], whose
    /// model-body interpretation is ordinary opaque drawing.
    pub blend: WorldSpriteBlend,
    /// Palette shade row used by an unlit textured primitive. Sprite flag
    /// `0x04` selects row 28; otherwise the palette starts at row 0.
    /// Uniformly lit and Gouraud primitives override this with their
    /// face-normal or interpolated corner-normal Section-6 lookup.
    pub flat_shade_row: u8,
}

impl FaceMaterial {
    /// Resolve the unlit colour, retaining the five authored channel bits.
    pub fn unlit_color(&self) -> [f32; 3] {
        let Some(palette) = self.palette_rgb555 else {
            return self.color;
        };
        let [r, g, b, _] = v2k_formats::palette::decode_rgb555(palette);
        std::array::from_fn(|axis| [r, g, b][axis] as f32 / 255.0 * self.color[axis])
    }
}

/// Raw model-light vector installed by the ordinary world and local-menu
/// render contexts.
///
/// Retail transforms these signed integer components into model orientation
/// before resolving face/corner normal references through the Section-6 table.
/// Keeping the authored scale matters because the final bin is selected after
/// a fixed `>> 19`, not from a normalized direction alone.
pub const RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW: [i32; 3] = [73, 73, -73];

/// Endpoint producer mode for the manual 0x02/0x22 screen constructor.
#[derive(Debug, Clone, Copy)]
pub enum ModelEdgeProjection<'a> {
    Compatibility,
    CommandSnapshots(&'a [v2k_formats::models::ModelEdgeEndpointSnapshot]),
}

/// Geometry and render data for one model-body submission.
///
/// Static models and per-frame materialized models use the same renderer path;
/// only the borrowed slices differ. Keeping those slices together prevents the
/// backend API from growing another near-identical draw method whenever one
/// more piece of model data is recovered.
#[derive(Debug, Clone, Copy)]
pub struct ModelMesh<'a> {
    /// Authored model header +0x08 unsigned extent, in raw model units.
    /// The node origin plus/minus this radius selects the near/far constructor
    /// table; it is not a bound recomputed from materialized vertices.
    pub radius_raw: u16,
    pub vertices: &'a [[f64; 3]],
    pub vertex_type_flags: &'a [i16],
    /// Original screen-midpoint dependencies, parallel to `vertices`.
    /// Empty diagnostic metadata treats every vertex as an ordinary position.
    pub vertex_projection: &'a [ModelVertexProjection],
    /// Callback clipping, parallel to `vertices`. Intrinsic/diagnostic meshes
    /// without this metadata have no surface-band rejection.
    pub vertex_clip: &'a [v2k_formats::models::ModelSlotClip],
    /// World tf13 provenance after dependency resolution, including linked
    /// imports. Used only with [`ModelSurfaceResolution::ContextResolved`].
    pub vertex_surface_origin: &'a [v2k_formats::models::ModelSurfaceOrigin],
    pub triangles: &'a [[u16; 3]],
    /// Original triangle or quad for each triangulated face. A quad's two
    /// triangles retain all four vertices for atomic retail near rejection.
    /// Empty diagnostic provenance falls back to each submitted triangle.
    pub face_vertices: &'a [ModelFaceVertices],
    pub normals: &'a [[f32; 3]],
    pub face_cull: &'a [ModelFaceCull],
    pub face_uvs: &'a [[[f32; 2]; 3]],
    pub face_corner_normals: &'a [[[f32; 3]; 3]],
    pub face_shading: &'a [ModelFaceShading],
    /// Authored `0x02`/`0x22` segments into `vertices`.
    pub edges: &'a [v2k_formats::models::ModelEdge],
    pub edge_projection: ModelEdgeProjection<'a>,
    /// Parallel materials for [`Self::edges`] (palette colour or sprite).
    pub edge_materials: &'a [FaceMaterial],
    /// Parallel resolved sprite width pair `(start, end)` for each
    /// [`Self::edges`] entry, taken from the sprite record's packed
    /// `+0x10/+0x12` word. A `(0, 0)` entry marks an unresolved sprite edge.
    /// Palette `0x02` hairlines also store `(0, 0)` here; the selector
    /// submits them as lines rather than tapered quads.
    pub edge_widths: &'a [(u16, u16)],
    /// Active Section-6 render-shade table. The recovered retail model light
    /// table addresses entries 0..7 and maps its remaining slots to entry 0.
    pub shade_table: Option<&'a [FogGradientEntry]>,
    /// Signed raw light vector active for this render context. Ordinary world
    /// and local menu props use `(73,73,-73)`; the frontend Klaus context uses
    /// its separately authored `(-100,50,-50)` vector.
    pub light_direction_raw: [i32; 3],
    /// Signed contextual shift applied to the 16-slot model-light table.
    /// World entity draws use terrain light minus the 0..8 underwater
    /// darkness step; menu and diagnostic draws leave this at zero.
    pub shade_shift: i32,
    pub materials: &'a [FaceMaterial],
}

impl<'a> ModelMesh<'a> {
    pub fn from_model(model: &'a ModelEntry, materials: &'a [FaceMaterial]) -> Self {
        Self {
            radius_raw: model.radius,
            vertices: &model.vertices,
            vertex_type_flags: &model.vertex_type_flags,
            vertex_projection: &model.vertex_projection,
            vertex_clip: &model.vertex_clip,
            vertex_surface_origin: &model.vertex_surface_origin,
            triangles: &model.triangles,
            face_vertices: &model.face_vertices,
            normals: &model.normals,
            face_cull: &model.face_cull,
            face_uvs: &model.face_uvs,
            face_corner_normals: &model.face_corner_normals,
            face_shading: &model.face_shading,
            edges: &model.edges,
            edge_projection: ModelEdgeProjection::Compatibility,
            edge_materials: &[],
            edge_widths: &[],
            shade_table: None,
            light_direction_raw: RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
            shade_shift: 0,
            materials,
        }
    }
}

/// Placement of raw model coordinates in the reconstructed world.
#[derive(Debug, Clone, Copy)]
pub struct ModelTransform {
    pub orientation: [[f32; 3]; 3],
    pub position: [f32; 3],
    /// Existing model scale contract: the backend converts this to raw/100.
    pub scale: f32,
}

/// The reconstructed frontend model path maps one hundred authored coordinate
/// units to one renderer view unit.
pub(crate) const FRONTEND_MODEL_UNITS_PER_VIEW: f32 = 100.0;

/// Raw parent-viewport planes published by the world loader or Main Base.
/// This owns `464E60` node-table selection, independently of the active
/// `470A10` projector globals and the software cache's per-vertex fog byte.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldModelFog {
    pub planes: v2k_formats::levels::LevelFogPlanes,
    pub color: [f32; 3],
}

/// Constructor table selected for a queued native world model node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeModelFogPass {
    Near,
    Fog,
}

impl WorldModelFog {
    /// `464E60`: header bit 0x20 bypasses both plane comparisons. Radius is
    /// unsigned, and origin +/- radius wraps in the original signed dword.
    /// None rejects before cache preparation, H/E callbacks or model commands.
    /// The viewport's absent-queue pre-resolve table is a separate context;
    /// this method belongs to the active queued world submission consumer.
    pub fn select_node(
        self,
        origin_view_z_raw: i32,
        radius_raw: u16,
        flags: u8,
    ) -> Option<NativeModelFogPass> {
        if flags & 0x20 != 0 {
            return Some(NativeModelFogPass::Near);
        }
        if origin_view_z_raw.wrapping_sub(i32::from(radius_raw)) >= self.planes.far_raw {
            return None;
        }
        Some(
            if origin_view_z_raw.wrapping_add(i32::from(radius_raw)) < self.planes.near_raw {
                NativeModelFogPass::Near
            } else {
                NativeModelFogPass::Fog
            },
        )
    }
}

/// Depth-fade policy owned by one model-node submission.
///
/// Retail installs several independent `FUN_00470A10` plane pairs while
/// composing the frontend: Klaus uses the live menu-world planes, every ring
/// prop uses a root-relative band, and `optionsh` uses an equal-plane step.
/// Keeping that choice on [`ModelDraw`] prevents those transient planes from
/// leaking into terrain, later models, billboards, or 2-D overlays through
/// the renderer's persistent gameplay-fog state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelDepthFade {
    /// Use the scene-wide fog configured through [`Renderer::set_fog`].
    InheritWorldFog,
    /// Draw this model without depth fading even if world fog is active.
    Disabled,
    /// Linear fade-coordinate planes in renderer view-depth units.
    /// Indexed models attenuate palette shade before lookup and apply the
    /// blend family's fog-colour contribution; this is not a universal RGB
    /// interpolation. Direct-RGB fallbacks retain their existing colour lerp.
    ///
    /// Equal or reversed planes are an authored hard step at `near`: depths
    /// before it have zero fade and depths at/after it have terminal fade.
    Linear {
        near: f32,
        far: f32,
        color: [f32; 3],
    },
    /// Raw queued-world node selection retained before live callbacks. The
    /// GL fade adapter uses /256 view planes, while pass selection is already
    /// source-owned. This does not publish the active `470A10` reciprocal or
    /// claim exact software-cache fog bytes.
    WorldRaw {
        fog: WorldModelFog,
        pass: NativeModelFogPass,
    },
    /// Retail fixed-point fade when the source integer plane domain is known.
    ///
    /// Faces use the corresponding view-space planes. Screen-space B-path
    /// primitives additionally retain these integers so their endpoint bytes
    /// can reproduce `FUN_00470A10`'s truncated reciprocal before
    /// interpolation. The variant names the frontend's `/100` coordinate
    /// convention explicitly rather than accepting an unchecked scale.
    FrontendFixedPointLinear {
        near_raw: i32,
        far_raw: i32,
        color: [f32; 3],
    },
}

impl Default for ModelDepthFade {
    fn default() -> Self {
        Self::InheritWorldFog
    }
}

impl ModelDepthFade {
    /// Evaluate an explicit per-model policy. `None` means that the caller
    /// must inherit the renderer's scene-wide world-fog calculation.
    pub fn explicit_amount_at(self, view_depth: f32) -> Option<f32> {
        match self {
            Self::InheritWorldFog => None,
            Self::Disabled => Some(0.0),
            Self::Linear { near, far, .. } => {
                Some(linear_depth_fade_amount(view_depth.abs(), near, far))
            }
            Self::WorldRaw { fog, pass } => Some(if pass == NativeModelFogPass::Near {
                0.0
            } else {
                linear_depth_fade_amount(
                    view_depth.abs(),
                    fog.planes.near_raw as f32 / 256.0,
                    fog.planes.far_raw as f32 / 256.0,
                )
            }),
            Self::FrontendFixedPointLinear {
                near_raw, far_raw, ..
            } => Some(linear_depth_fade_amount(
                view_depth.abs(),
                near_raw as f32 / FRONTEND_MODEL_UNITS_PER_VIEW,
                far_raw as f32 / FRONTEND_MODEL_UNITS_PER_VIEW,
            )),
        }
    }
}

/// Retail's clamped linear depth byte. `FUN_00470A10` installs the unit scale
/// whenever `near >= far`, making that case a hard step at `near`.
fn linear_depth_fade_amount(view_depth: f32, near: f32, far: f32) -> f32 {
    if near >= far {
        return if view_depth >= near { 1.0 } else { 0.0 };
    }
    ((view_depth - near) / (far - near)).clamp(0.0, 1.0)
}

/// Retail interpretation of type-14 model vertices for one draw context.
///
/// Unlike ordinary model-local records, type 14 delegates its three authored
/// operands to the callback installed at render context `+0x5C`. The callback
/// returns an already world-space point which the vertex handler transforms
/// directly into view space. Callback ownership differs between entity, menu,
/// and static-terrain contexts, so callers must opt into a recovered policy.
// The 16-slot retail selector table is the authored shape; Copy submission
// beats an indirection per draw.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExternalFrameMode {
    /// Preserve the record's authored operands when the live callback is not
    /// available. This is the existing diagnostic/entity fallback.
    Raw,
    /// Replace every type-14 record with this callback-produced world point.
    WorldPoint([f32; 3]),
    /// `FUN_0041D360` per-selector world point. The type-14 X operand is the
    /// authored selector; a missing slot keeps the raw operands.
    SelectorWorldPoints([Option<[f32; 3]>; 16]),
}

/// Retail interpretation of type-13 model vertices for this draw context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPinMode {
    /// Exclude type-13 support/shadow vertices from this draw context.
    Disabled,
    /// Treat type-13 records as ordinary authored vertices. This is a
    /// diagnostic/default fallback for contexts whose callback policy has not
    /// yet been recovered; canonical models retain these faces in one stream.
    Raw,
    /// Apply retail's type-13 transform: preserve source view X/Z and replace
    /// view Y with signed -32767 in the original 8.8 world domain.
    CameraFacing,
    /// Apply the world callbacks installed by `FUN_00433FA0`: type-13 corners
    /// extend along the Section-10 projection direction until they reach
    /// terrain or the animated water surface, and type-13-adjacent tf-12 alias
    /// corners have their Y replaced by the plain terrain sample at their own
    /// world X/Z (the swapped `FUN_004340B0` family).
    WorldSurface,
}

/// Whether one model-body submission is a terrain-conformant overlay.
///
/// World-surface faces detect this from resolved tf13 provenance, including
/// linked imports (`player4` shadow uses direct tf13 corners). Intrinsic
/// diagnostics use authored flags. The Targetter crosshair is a flat ordinary
/// mesh placed on the recovered terrain hit (`FUN_0044DF00`), so that draw
/// must opt in explicitly rather than inventing type-13 vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelOverlayKind {
    #[default]
    None,
    /// Terrain-conformant overlay: same GL depth treatment as an all-type-13
    /// world-surface decal under [`OverlayDepthPolicy`].
    TerrainSurface,
}

/// Controlled depth policy for ground-conformant world overlays while the
/// open surface-overlay depth gap is being diagnosed
/// ([RENDER_PIPELINE.md](../../../docs/re/RENDER_PIPELINE.md) "Open
/// ground-surface overlay depth gap").
///
/// Live default is [`Self::TerrainRecede`]: retail has no z-buffer, plants
/// type-13 / Targetter overlays on the sampled surface, and the stable painter
/// drains terrain before same-depth model primitives so those overlays always
/// paint. A GPU depth buffer is a strict improvement for occlusion, but it
/// must not reintroduce a test the software filler never ran. [`Self::DecalOffset`]
/// is the previous port workaround; [`Self::OverlayAlways`] is diagnosis only.
/// Each policy applies to all-type-13 world-surface decals **and** draws
/// tagged [`ModelOverlayKind::TerrainSurface`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayDepthPolicy {
    /// Previous port workaround: terrain-conformant overlay faces use `LEQUAL`
    /// plus a small eye-side polygon offset; terrain writes ordinary depth.
    /// Ties improve; genuine geometric crossing with the hardware terrain
    /// mesh still drops fragments.
    DecalOffset,
    /// Recede the opaque terrain fill's depth with a slope-scaled offset so
    /// later-submitted surface overlays win ties — retail's stable painter
    /// sorts terrain before same-depth model primitives. Decal-side offsets
    /// are skipped so this isolates the terrain-side variable; foreground
    /// geometry stays unoffset and still occludes.
    #[default]
    TerrainRecede,
    /// Diagnosis only: submit terrain-conformant overlay faces without a
    /// depth test. Foreground terrain can no longer occlude them, so missing
    /// fragments under this policy rule out depth as the cause.
    OverlayAlways,
}

impl OverlayDepthPolicy {
    /// Window-Z `units` covering about one Section-10 height byte (0.125 world)
    /// at a typical gameplay depth against the 0.1/500 projection.
    ///
    /// `PolygonOffset(1, 1)` is one depth-buffer ULP. Bilinear type-13 / cell-hit
    /// Y versus two-triangle terrain is a cell-scale mismatch (hundreds of ULPs
    /// at 20 world), and the Targetter mesh is a rigid plane on `FUN_0044DF00`'s
    /// three-sample basis, so it extends below the GL triangles. Retail never
    /// depth-tested those fragments. This is a window-Z bias, not a world-Y lift.
    pub(crate) const DEPTH_BIAS_UNITS: f32 = 512.0;

    /// `(factor, units)` polygon offset applied to the opaque terrain fill.
    pub(crate) const fn terrain_fill_offset(self) -> Option<(f32, f32)> {
        match self {
            Self::TerrainRecede => Some((1.0, Self::DEPTH_BIAS_UNITS)),
            Self::DecalOffset | Self::OverlayAlways => None,
        }
    }

    /// `(factor, units)` eye-side offset for terrain-conformant overlay faces.
    pub(crate) const fn overlay_fill_offset(self) -> Option<(f32, f32)> {
        match self {
            Self::OverlayAlways => None,
            Self::DecalOffset | Self::TerrainRecede => Some((-1.0, -Self::DEPTH_BIAS_UNITS)),
        }
    }

    /// Whether terrain-conformant overlay faces skip the depth test entirely.
    pub(crate) const fn disables_decal_depth_test(self) -> bool {
        matches!(self, Self::OverlayAlways)
    }
}

/// Per-frame state consumed by retail's world type-13 callback family.
///
/// The two Q1.31 slopes are derived once from the authored Section-10
/// direction. They remain private to the renderer so callers cannot silently
/// substitute an approximate direction or camera-facing vector.
#[derive(Debug, Clone, Copy)]
pub struct WorldSurfaceProjection<'a> {
    pub(crate) terrain: &'a TerrainGrid,
    pub(crate) retail_tick: i32,
    pub(crate) slope_x_q31: i32,
    pub(crate) slope_z_q31: i32,
    waves_enabled: bool,
}

impl<'a> WorldSurfaceProjection<'a> {
    /// The same terrain supplies tf12's plain ground sample and tf13's
    /// surface projection within the model-slot dependency graph.
    pub fn terrain(self) -> &'a TerrainGrid {
        self.terrain
    }

    /// DAT4FECE4, copied from the active world descriptor +84 by 433BD0.
    pub fn waves_enabled(self) -> bool {
        self.waves_enabled
    }

    pub fn with_waves_enabled(mut self, enabled: bool) -> Self {
        self.waves_enabled = enabled;
        self
    }

    /// 445920 returns sea when waves are disabled; 4349C0 then takes the
    /// maximum of that signed-short result and its bilinear terrain sample.
    pub fn wave_surface_raw(self, x: i16, z: i16, floor: i16) -> i16 {
        if self.waves_enabled {
            v2k_formats::terrain::wave_surface_raw(
                x,
                z,
                self.retail_tick,
                self.terrain.sea_level_raw(),
                floor,
            )
        } else {
            floor.max(self.terrain.sea_level_raw())
        }
    }

    pub fn retail_tick(self) -> i32 {
        self.retail_tick
    }
    pub fn slopes_q31(self) -> [i32; 2] {
        [self.slope_x_q31, self.slope_z_q31]
    }

    pub fn new(terrain: &'a TerrainGrid, retail_tick: i32) -> Self {
        let direction_x = terrain.header[1] / 4;
        let direction_y = terrain.header[2];
        let direction_z = terrain.header[3] / 4;
        Self {
            terrain,
            retail_tick,
            waves_enabled: true,
            slope_x_q31: retail_world_surface_slope(direction_x, direction_y),
            slope_z_q31: retail_world_surface_slope(direction_z, direction_y),
        }
    }
}

/// `FUN_00433BD0`'s exact signed/saturated Q1.31 direction setup.
fn retail_world_surface_slope(horizontal: i32, vertical: i32) -> i32 {
    let numerator = horizontal.wrapping_abs() as u32;
    let denominator = vertical.wrapping_abs() as u32;
    let mut magnitude = if numerator < denominator {
        retail_q31_divide_positive(numerator, denominator)
    } else {
        i32::MAX
    };
    if (horizontal ^ vertical) >= 0 {
        magnitude = magnitude.wrapping_neg();
    }
    magnitude
}

/// Positive-input core of `FUN_00457680`.
fn retail_q31_divide_positive(mut numerator: u32, mut denominator: u32) -> i32 {
    debug_assert!(numerator < denominator);
    let mut bit = 0x8000_0000_u32;
    let mut quotient = 0_u32;
    while numerator != 0 && denominator != 0 {
        denominator >>= 1;
        bit >>= 1;
        if denominator <= numerator {
            numerator = numerator.wrapping_sub(denominator);
            quotient |= bit;
        }
    }
    quotient as i32
}

/// How a model submission establishes visibility against earlier draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelDepthPolicy {
    /// Ordinary hardware depth testing and material-dependent depth writes.
    Geometry,
    /// The caller has resolved the retail nested painter queue. Paint in
    /// submission order without testing or modifying geometric depth.
    Painter,
    /// An opaque/keyed enclosing retail group, already ordered internally.
    /// Every covered pixel records the group's signed camera-depth key in
    /// raw model units (100 per renderer view unit), rather than face depth.
    /// Equal keys preserve submission order; nearer existing geometry wins.
    /// Keys outside the camera depth interval clamp to its near/far boundary.
    /// The caller must exclude translucent faces and separate sibling groups.
    PainterGroup { view_depth_raw: i32 },
}

/// Whether world tf12 aliases and tf13 surface callbacks have already run
/// inside the generator graph. Intrinsic and default/menu policies stay separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSurfaceResolution {
    /// Intrinsic materialization; apply active surface callbacks at submission.
    Intrinsic,
    /// World tf12/tf13 already ran inside the recursive model slot resolver.
    ContextResolved,
}

/// Coordinate domain for a model's near-plane rejection.
///
/// Retail rejects original faces before rasterization. The active render
/// context supplies the coordinate conversion independently of model scale,
/// fog planes, constructor table, and header bit `0x20`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelNearClip {
    /// Diagnostic geometry follows the renderer camera's near plane.
    Camera,
    /// World scene coordinates have 256 retail raw units per view unit.
    RetailWorld,
    /// Frontend and foreground HUD coordinates have 100 raw units per view unit.
    RetailFrontend,
}

#[derive(Debug, Clone, Copy)]
/// Complete model-body submission. This replaces the former ladder of
/// yaw-only, oriented, coloured, material, and materialized draw methods.
pub struct ModelDraw<'a> {
    pub mesh: ModelMesh<'a>,
    /// Snapshot of scene display custody at model callback/submission time.
    pub projection_authority: crate::projection::SceneProjectionAuthority,
    pub transform: ModelTransform,
    pub near_clip: ModelNearClip,
    /// Submission-local depth fade. The default gameplay path inherits the
    /// renderer's world fog; frontend callers opt into their recovered planes.
    pub depth_fade: ModelDepthFade,
    pub depth_policy: ModelDepthPolicy,
    pub view_pin: ViewPinMode,
    pub surface_resolution: ModelSurfaceResolution,
    /// Required only by [`ViewPinMode::WorldSurface`]. A missing world
    /// context fails closed for type-13 vertices rather than guessing.
    pub world_surface: Option<WorldSurfaceProjection<'a>>,
    pub external_frame: ExternalFrameMode,
    /// Explicit terrain-overlay opt-in for draws that sit on the recovered
    /// terrain hit without authored type-13 vertices.
    pub overlay: ModelOverlayKind,
}

/// Resolved material and authored sprite dimensions for one Section-8
/// billboard. Flat-colour billboards use a square 1×1 aspect.
#[derive(Debug, Clone, Copy)]
pub struct BillboardMaterial {
    pub face: FaceMaterial,
    pub width: u16,
    pub height: u16,
    /// Framebuffer compositing selected by the source sprite's retail flags.
    pub blend: WorldSpriteBlend,
}

/// Section-8 billboard submission with its owning node's constructor policy.
/// The node origin and authored radius select the near/far table before each
/// billboard's own center supplies its fade byte.
#[derive(Debug, Clone, Copy)]
pub struct ModelBillboardDraw<'a> {
    pub vertices: &'a [[f64; 3]],
    pub vertex_projection: &'a [ModelVertexProjection],
    /// The owning resolved slot's clip state, with the same contract as
    /// [`ModelMesh::vertex_clip`].
    pub vertex_clip: &'a [v2k_formats::models::ModelSlotClip],
    pub billboards: &'a [Billboard],
    pub materials: &'a [BillboardMaterial],
    pub transform: ModelTransform,
    pub radius_raw: u16,
    pub depth_fade: ModelDepthFade,
    pub near_clip: ModelNearClip,
    /// Prepared queue submission. Ordinary billboard consumers keep Geometry.
    pub depth_policy: ModelDepthPolicy,
}

/// Blend operation for a camera-facing world sprite.
///
/// V2000's flame, glow, and impact-particle path uses source-one / destination-
/// one blending. `HalfAdditive` names the distinct retail span equation
/// `source + destination / 2`; it must not be weakened into conventional
/// source-alpha compositing merely because opacity supplies the half factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSpriteBlend {
    /// Alpha-tested sprite with no framebuffer blending.
    Masked,
    Additive,
    HalfAdditive,
}

/// Source alpha used with `ONE, ONE_MINUS_SRC_ALPHA` to reproduce retail's
/// exact `source + destination / 2` span equation.
pub const HALF_ADDITIVE_ALPHA: f32 = 0.5;

/// Fixed-sprite constructor selected before palette lookup. A far sprite
/// remains in the far family even at fade byte zero, using its far row-28
/// accumulator instead of the near sprite's authored row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteFog {
    Near,
    Far { fade_byte: u8 },
}

/// One persistent-texture sprite drawn as a camera-facing world-space quad.
///
/// `size` is the full width and height in world units. `rotation` is radians
/// around the view axis. `color` tints the sampled palette colour; `fog`
/// selects the near/far fixed-sprite palette row before that lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSprite {
    pub texture: TextureId,
    pub position: [f32; 3],
    pub size: [f32; 2],
    pub rotation: f32,
    pub color: [f32; 4],
    /// Retail painter key in original projected-depth units. Higher keys are
    /// submitted first; equal keys preserve the caller's intrusive-list order.
    pub sort_key_raw: i32,
    pub blend: WorldSpriteBlend,
    /// Authored palette row used by the near fixed-sprite constructor.
    pub flat_shade_row: u8,
    /// Center-depth fade selected by this sprite's explicit draw context.
    pub fog: SpriteFog,
}

impl WorldSprite {
    /// Construct an untinted additive sprite without depth fading.
    pub fn additive(texture: TextureId, position: [f32; 3], size: [f32; 2]) -> Self {
        Self {
            texture,
            position,
            size,
            rotation: 0.0,
            color: [1.0; 4],
            sort_key_raw: 0,
            blend: WorldSpriteBlend::Additive,
            flat_shade_row: 28,
            fog: SpriteFog::Near,
        }
    }
}

/// High-level scene boundary for restoring renderer state. The original game
/// shared one software rasterizer but rebuilt distinct menu/world contexts and
/// explicit primitive state each frame; this prevents persistent GPU state in
/// the port from crossing that same boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderScene {
    Menu,
    World,
}

/// Which color representation a frame capture retains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCaptureSource {
    /// The completed display image, including presentation color conversion.
    /// Capture after present, before modifying the next scene target.
    Presented,
    /// The current scene target before presentation color conversion or swap.
    /// This may be captured between a world draw and its menu overlay.
    CurrentScene,
}

/// Owned top-to-bottom RGBA8 pixels from an explicit frame capture source.
///
/// Scene captures allow pause underlays to retain color precision while the
/// display adapter changes, without repeating gameplay rendering callbacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFrame {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Logical render surface and the physical output rectangle it occupies.
/// Logical dimensions drive projection/UI math; the physical rectangle is
/// the backend viewport (or the software blit destination).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderViewport {
    pub x: i32,
    pub y: i32,
    pub physical_width: u32,
    pub physical_height: u32,
    pub logical_width: u32,
    pub logical_height: u32,
}

impl RenderViewport {
    pub fn for_output(
        width: u32,
        height: u32,
        reference_width: u32,
        reference_height: u32,
        mode: ScalingMode,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let reference_height = reference_height.max(1);
        let reference_width = reference_width.max(1);
        match mode {
            ScalingMode::Native => Self {
                x: 0,
                y: 0,
                physical_width: width,
                physical_height: height,
                logical_width: width,
                logical_height: height,
            },
            ScalingMode::FourThree => {
                let (physical_width, physical_height) = fit_four_three(width, height);
                let logical_height = reference_height;
                let logical_width = ((reference_height as u64 * 4 + 1) / 3).max(1) as u32;
                Self {
                    x: ((width - physical_width) / 2) as i32,
                    y: ((height - physical_height) / 2) as i32,
                    physical_width,
                    physical_height,
                    logical_width,
                    logical_height,
                }
            }
            ScalingMode::Stretched => {
                let logical_height = reference_height;
                let logical_width = if reference_width as u64 * 3 == reference_height as u64 * 4 {
                    reference_width
                } else {
                    ((reference_height as u64 * 4 + 1) / 3).max(1) as u32
                };
                Self {
                    x: 0,
                    y: 0,
                    physical_width: width,
                    physical_height: height,
                    logical_width,
                    logical_height,
                }
            }
        }
    }
}

fn fit_four_three(width: u32, height: u32) -> (u32, u32) {
    if width as u64 * 3 > height as u64 * 4 {
        (((height as u64 * 4) / 3).max(1) as u32, height)
    } else {
        (width, ((width as u64 * 3) / 4).max(1) as u32)
    }
}

/// Rendering interface implemented by each backend.
pub trait Renderer {
    /// Human-readable backend name.
    fn backend_name(&self) -> &str;

    /// Restore the canonical transient state for a new high-level scene.
    /// Backends without persistent graphics state may leave this as a no-op.
    fn begin_scene(&mut self, scene: RenderScene) {
        let _ = scene;
    }

    /// Clear the framebuffer.
    fn clear(&mut self, r: f32, g: f32, b: f32);

    /// Present the rendered frame.
    fn present(&mut self);

    /// Copy the selected logical image, if the backend supports readback.
    /// The returned pixels are top-to-bottom RGBA8, matching
    /// [`Self::draw_fullscreen`]. CurrentScene does not present or swap buffers.
    fn capture_frame(&mut self, _source: FrameCaptureSource) -> Option<CapturedFrame> {
        None
    }

    /// Clear only 3-D depth while retaining the color image. Pause overlays
    /// restore a captured world frame, then use a fresh menu depth buffer for
    /// the authored `optionsh` prop.
    fn clear_depth(&mut self) {}

    /// Handle window resize.
    fn resize(&mut self, width: u32, height: u32);

    /// Set the camera for 3D rendering.
    fn set_camera(&mut self, camera: &Camera);

    /// Select the projected-vertex policy for subsequent 3-D submissions.
    /// Scene boundaries reset this to [`ProjectionEffect::None`]; world
    /// callers opt into the retail underwater callbacks once per frame.
    fn set_projection_effect(&mut self, effect: ProjectionEffect) {
        let _ = effect;
    }

    /// Publish source display authority for this scene. Begin-scene reset is
    /// explicit; the value is captured into each model draw before buffering.
    fn set_scene_projection_authority(
        &mut self,
        _authority: crate::projection::SceneProjectionAuthority,
    ) {
    }

    fn scene_projection_authority(&self) -> crate::projection::SceneProjectionAuthority {
        crate::projection::SceneProjectionAuthority::default()
    }

    /// Configure fog parameters.
    fn set_fog(&mut self, enabled: bool, near: f32, far: f32, color: [f32; 3]);

    /// Publish the original integer world pair without recovering it from
    /// float GL state. Backends retaining native model state override the
    /// getter; ordinary float-only backends still receive the existing policy.
    fn set_world_model_fog(&mut self, fog: Option<WorldModelFog>) {
        if let Some(fog) = fog {
            self.set_fog(
                true,
                fog.planes.near_raw as f32 / 256.0,
                fog.planes.far_raw as f32 / 256.0,
                fog.color,
            );
        } else {
            self.set_fog(false, 0.0, 0.0, [0.0; 3]);
        }
    }

    fn world_model_fog(&self) -> Option<WorldModelFog> {
        None
    }

    /// World fog planes inherited by the active scene, independent of
    /// temporary GL state during model/overlay submission. Menu scenes and
    /// backends without world fog return None. Hierarchy traversal uses the far plane before
    /// executing a model's commands, callbacks, or child instances.
    fn world_fog_planes(&self) -> Option<[f32; 2]> {
        None
    }

    /// Render a terrain grid. `frames` contains the original level's 120
    /// canonical transition sprites; `colors` is retained for the diagnostic
    /// fallback used when those assets cannot be resolved. `lights` is the
    /// original world-anchored scrolling 32×32 signed shade-delta field.
    /// `elapsed_micros` advances the recovered process-global infected-vertex
    /// motion only on frames that actually execute the terrain draw callback.
    fn draw_terrain(
        &mut self,
        terrain: &TerrainGrid,
        colors: &[PaletteEntry],
        frames: Option<&crate::terrain_tiles::TerrainFrames>,
        lights: Option<&crate::terrain_light::TerrainLightWindow>,
        elapsed_micros: u32,
    );

    /// Draw the wave-displaced sea at `sea_level_y` (port units), blended
    /// translucently over the terrain. `retail_tick` is the executable's
    /// wrapping signed 50 Hz simulation tick; `terrain` provides the ground
    /// height for per-point depth scaling, shoreline clamping and the
    /// marching-squares corner codes. When `frames` is given, cells are
    /// textured with the engine's 5 shoreline shape sprites (see
    /// [`crate::water`]); otherwise a flat `color` stand-in is used. Only
    /// called when the level has water (`TerrainGrid::water_enabled`). Default
    /// no-op for backends without a 3D pipeline.
    fn draw_water(
        &mut self,
        terrain: &TerrainGrid,
        sea_level_y: f32,
        color: [f32; 3],
        retail_tick: i32,
        frames: Option<&crate::water::WaterFrames>,
    ) {
        let _ = (terrain, sea_level_y, color, retail_tick, frames);
    }

    /// Discard any cached per-level GPU data (e.g. the terrain mesh) so the
    /// next draw rebuilds it. Called on level load. Default no-op.
    fn invalidate_terrain_cache(&mut self) {}

    /// Select the controlled ground-overlay depth policy used while a
    /// diagnosis of the open surface-overlay depth gap is running. The
    /// software backend has no depth buffer and ignores it.
    fn set_overlay_depth_policy(&mut self, _policy: OverlayDepthPolicy) {}

    /// Draw one static or materialized model body. Backends without a 3-D
    /// model pipeline may retain the default no-op.
    fn draw_model_body(&mut self, draw: ModelDraw<'_>) {
        let _ = draw;
    }

    /// Draw Section-8 billboard primitives after their owning opaque model.
    /// The OpenGL backend keeps them camera-facing and depth-tested; software
    /// backends may omit them until they have a 3-D projection path.
    fn draw_model_billboards(&mut self, draw: ModelBillboardDraw<'_>) {
        let _ = draw;
    }

    /// Draw camera-facing, depth-tested sprites in world space after opaque
    /// terrain/models. The sprite textures must have been created through
    /// [`Self::create_texture`] or [`Self::create_indexed_model_texture`].
    /// Backends without a 3-D projection path may leave this as a no-op.
    ///
    /// The backend stably orders the complete list by
    /// [`WorldSprite::sort_key_raw`], reproducing retail's shared painter
    /// queue across masked and blended particle materials.
    fn draw_world_sprites(&mut self, sprites: &[WorldSprite]) {
        let _ = sprites;
    }

    /// Whether this backend can render 3D models (the software backend
    /// currently cannot; callers fall back to 2D presentation).
    fn supports_models(&self) -> bool {
        true
    }

    /// Upload an RGBA8 image as a persistent texture and return its handle,
    /// or `None` if the backend has no texture support (software fallback).
    /// Retail's software rasterizer point samples every authored texture, so
    /// persistent backend implementations must use nearest-neighbour sampling.
    fn create_texture(&mut self, rgba: &[u8], width: u32, height: u32) -> Option<TextureId> {
        let _ = (rgba, width, height);
        None
    }

    /// Upload an RGBA8 image whose point-sampled role is structural rather than
    /// merely visual. Indexed palette/source textures use this to document that
    /// interpolation cannot occur before palette lookup; on the retail-faithful
    /// backend it shares [`Self::create_texture`]'s nearest-neighbour policy.
    fn create_texture_nearest(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Option<TextureId> {
        self.create_texture(rgba, width, height)
    }

    /// Upload a shaded indexed model texture. Backends without an indexed
    /// model shader retain the brightest-row RGBA fallback.
    fn create_indexed_model_texture(
        &mut self,
        texture: IndexedModelTexture<'_>,
    ) -> Option<TextureId> {
        self.create_texture(texture.fallback_rgba, texture.width, texture.height)
    }

    /// Register a Section-3 record for backends that rasterize retail
    /// materials directly, returning its material id. Other backends keep
    /// the default `None`.
    fn create_native_sprite(&mut self, _sprite: NativeSprite<'_>) -> Option<u32> {
        None
    }

    /// Publish the retained world viewport words for producers that
    /// transform natively. Scene boundaries do not clear it; callers pass
    /// `None` when the scene has no native viewport.
    fn set_native_world_viewport(
        &mut self,
        _viewport: Option<crate::projection::NativeViewportWords>,
    ) {
    }

    /// Free a texture created with [`Renderer::create_texture`]. Default no-op.
    fn destroy_texture(&mut self, id: TextureId) {
        let _ = id;
    }

    /// Render a 2D sprite overlay (RGBA data).
    fn draw_sprite(&mut self, rgba: &[u8], width: u32, height: u32, x: i32, y: i32);

    /// Render a 2D sprite with the framebuffer operation selected by its
    /// Section-3 material flags. This is distinct from conventional UI alpha:
    /// retail HUD layers can be masked, additive, or source-plus-destination-
    /// half. Every backend must choose explicitly how to implement this
    /// operation instead of silently degrading it to conventional alpha.
    #[allow(clippy::too_many_arguments)]
    fn draw_material_sprite(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        blend: WorldSpriteBlend,
    );

    /// Screen-space `FUN_0047aa20` quad: the same material sprite, submitted
    /// at four authored corner pixels instead of an axis-aligned rectangle.
    /// Backends that cannot rasterize a rotated blit leave this as a no-op
    /// rather than substituting an AABB.
    #[allow(clippy::too_many_arguments)]
    fn draw_material_sprite_quad(
        &mut self,
        _rgba: &[u8],
        _width: u32,
        _height: u32,
        _corners: [(i32, i32); 4],
        _blend: WorldSpriteBlend,
    ) {
    }

    /// Limit subsequent 2D overlay drawing to a top-left-origin viewport
    /// rectangle. `None` restores the full viewport. Backends without a 2D
    /// clipping facility may leave this as a no-op.
    fn set_sprite_clip(&mut self, _rect: Option<(i32, i32, u32, u32)>) {}

    /// Add a screen-space sprite at the depth-buffer value corresponding to
    /// `view_depth`, using source-one/destination-one blending without writing
    /// depth. Draw this after opaque geometry: nearer geometry masks it while
    /// the sprite adds over farther geometry, matching the original sorted
    /// additive billboard path. Backends without a depth buffer fall back to
    /// an ordinary overlay.
    #[allow(clippy::too_many_arguments)]
    fn draw_additive_sprite_at_depth(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        view_depth: f32,
        near: f32,
        far: f32,
    ) {
        let _ = (view_depth, near, far);
        self.draw_sprite(rgba, width, height, x, y);
    }

    /// Render a fullscreen RGBA image (for video playback / backgrounds).
    /// The image is scaled to fill the window.
    fn draw_fullscreen(&mut self, rgba: &[u8], width: u32, height: u32);

    /// Draw a fullscreen color overlay with alpha blending.
    /// Used for fade transitions. (r, g, b, a) are 0.0–1.0.
    fn draw_color_overlay(&mut self, r: f32, g: f32, b: f32, a: f32);

    /// Current viewport dimensions (width, height).
    fn viewport_size(&self) -> (u32, u32);

    /// Explicit presentation policy for the selected authored 2-D canvas.
    fn ui_submission_policy(&self) -> UiSubmissionPolicy {
        UiSubmissionPolicy::FitAuthoredCanvas
    }

    /// The display owner sets this independently from render surface dimensions.
    fn set_ui_submission_policy(&mut self, _policy: UiSubmissionPolicy) {}

    /// Uniform scale for screen-edge sprites authored as native pixels. High
    /// Native grows above the original resolutions; Low Native and fitted
    /// presentation keep their existing adapter.
    fn authored_pixel_scale(&self) -> f32 {
        1.0
    }

    /// Toggle the window between fullscreen-desktop and windowed at runtime
    /// (Alt+Enter). Default no-op for backends without a togglable window.
    fn set_fullscreen(&mut self, _on: bool) {}

    /// Change the OS window's drawable size. Display→Resolution uses this;
    /// unlike [`Self::resize`], it requests a real window mode change.
    fn set_window_size(&mut self, width: u32, height: u32) {
        self.resize(width, height);
    }

    /// Change how the authored 4:3 frame maps onto the current drawable.
    fn set_scaling_mode(
        &mut self,
        _mode: ScalingMode,
        _reference_width: u32,
        _reference_height: u32,
    ) {
    }

    /// Render the complete scene into the selected authored-resolution
    /// framebuffer before presenting it to the physical output. This keeps
    /// texture lookup point-sampled while allowing one final filtered scale,
    /// with the OpenGL classic adapter quantizing logical pixels to RGB565
    /// before that scale. Packed per-draw software composition remains separate.
    /// matching the way a completed retail frame was enlarged for a modern
    /// display. Backends without an offscreen color/depth target return
    /// `false`; Native presentation may also reject the request.
    fn set_classic_framebuffer(&mut self, _enabled: bool) -> bool {
        false
    }

    /// Whether authored-resolution offscreen presentation is currently in
    /// use. A requested mode can become inactive after switching to Native or
    /// when the graphics driver cannot provide framebuffer objects.
    fn classic_framebuffer_active(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod viewport_tests {
    use super::{
        ModelDepthFade, RenderViewport, SpriteFog, TextureId, WorldSprite, WorldSpriteBlend,
    };
    use crate::config::ScalingMode;

    #[test]
    fn native_uses_the_complete_output() {
        let v = RenderViewport::for_output(1920, 1080, 1024, 768, ScalingMode::Native);
        assert_eq!((v.logical_width, v.logical_height), (1920, 1080));
        assert_eq!(
            (v.x, v.y, v.physical_width, v.physical_height),
            (0, 0, 1920, 1080)
        );
    }

    #[test]
    fn additive_world_sprite_uses_original_billboard_defaults() {
        let sprite = WorldSprite::additive(TextureId(7), [1.0, 2.0, 3.0], [4.0, 5.0]);
        assert_eq!(sprite.texture, TextureId(7));
        assert_eq!(sprite.position, [1.0, 2.0, 3.0]);
        assert_eq!(sprite.size, [4.0, 5.0]);
        assert_eq!(sprite.rotation, 0.0);
        assert_eq!(sprite.color, [1.0; 4]);
        assert_eq!(sprite.sort_key_raw, 0);
        assert_eq!(sprite.blend, WorldSpriteBlend::Additive);
        assert_eq!(sprite.flat_shade_row, 28);
        assert_eq!(sprite.fog, SpriteFog::Near);
    }

    #[test]
    fn model_depth_fade_keeps_world_inheritance_explicit() {
        assert_eq!(
            ModelDepthFade::InheritWorldFog.explicit_amount_at(12.0),
            None
        );
        assert_eq!(ModelDepthFade::Disabled.explicit_amount_at(12.0), Some(0.0));
    }

    #[test]
    fn model_depth_fade_clamps_and_interpolates_in_view_space() {
        let fade = ModelDepthFade::Linear {
            near: 5.0,
            far: 9.0,
            color: [0.0; 3],
        };
        assert_eq!(fade.explicit_amount_at(4.0), Some(0.0));
        assert_eq!(fade.explicit_amount_at(5.0), Some(0.0));
        assert_eq!(fade.explicit_amount_at(7.0), Some(0.5));
        assert_eq!(fade.explicit_amount_at(9.0), Some(1.0));
        assert_eq!(fade.explicit_amount_at(12.0), Some(1.0));
        assert_eq!(
            fade.explicit_amount_at(-7.0),
            Some(0.5),
            "view-space depth follows the renderer's absolute-eye-Z convention"
        );
    }

    #[test]
    fn non_increasing_model_depth_fade_planes_are_a_deterministic_step() {
        let fade = ModelDepthFade::Linear {
            near: 30.0,
            far: 30.0,
            color: [0.0; 3],
        };
        assert_eq!(fade.explicit_amount_at(29.999), Some(0.0));
        assert_eq!(fade.explicit_amount_at(30.0), Some(1.0));
        assert_eq!(fade.explicit_amount_at(30.001), Some(1.0));

        let reversed = ModelDepthFade::Linear {
            near: 30.0,
            far: 20.0,
            color: [0.0; 3],
        };
        assert_eq!(reversed.explicit_amount_at(29.999), Some(0.0));
        assert_eq!(reversed.explicit_amount_at(30.0), Some(1.0));
    }

    #[test]
    fn four_three_is_centered_without_distortion() {
        let v = RenderViewport::for_output(1920, 1080, 1024, 768, ScalingMode::FourThree);
        assert_eq!((v.logical_width, v.logical_height), (1024, 768));
        assert_eq!(
            (v.x, v.y, v.physical_width, v.physical_height),
            (240, 0, 1440, 1080)
        );
    }

    #[test]
    fn stretched_reports_four_three_but_fills_widescreen() {
        let v = RenderViewport::for_output(1920, 1080, 1024, 768, ScalingMode::Stretched);
        assert_eq!((v.logical_width, v.logical_height), (1024, 768));
        assert_eq!(
            (v.x, v.y, v.physical_width, v.physical_height),
            (0, 0, 1920, 1080)
        );
    }

    #[test]
    fn stretched_keeps_authored_framebuffer_for_native_pixel_sprites() {
        let v = RenderViewport::for_output(1920, 1080, 640, 480, ScalingMode::Stretched);
        assert_eq!((v.logical_width, v.logical_height), (640, 480));
        assert_eq!((v.physical_width, v.physical_height), (1920, 1080));
        let frontier_width = 110.0 * v.physical_width as f32 / v.logical_width as f32;
        assert!((frontier_width - 330.0).abs() < 0.1);
    }

    #[test]
    fn overlay_depth_policies_scope_their_gl_side_effects() {
        use super::{ModelOverlayKind, OverlayDepthPolicy};

        assert_eq!(ModelOverlayKind::default(), ModelOverlayKind::None);

        // Live default: recede only the opaque terrain fill so later overlays
        // win ties the way the software painter overwrites terrain pixels.
        assert_eq!(
            OverlayDepthPolicy::default(),
            OverlayDepthPolicy::TerrainRecede
        );
        assert_eq!(
            OverlayDepthPolicy::TerrainRecede.terrain_fill_offset(),
            Some((1.0, OverlayDepthPolicy::DEPTH_BIAS_UNITS))
        );
        assert_eq!(
            OverlayDepthPolicy::TerrainRecede.overlay_fill_offset(),
            Some((-1.0, -OverlayDepthPolicy::DEPTH_BIAS_UNITS))
        );
        assert!(!OverlayDepthPolicy::TerrainRecede.disables_decal_depth_test());

        assert_eq!(OverlayDepthPolicy::DecalOffset.terrain_fill_offset(), None);
        assert_eq!(
            OverlayDepthPolicy::DecalOffset.overlay_fill_offset(),
            Some((-1.0, -OverlayDepthPolicy::DEPTH_BIAS_UNITS))
        );
        assert!(!OverlayDepthPolicy::DecalOffset.disables_decal_depth_test());

        // Diagnosis aid: decals bypass the depth test entirely and the
        // terrain stays authoritative.
        assert_eq!(
            OverlayDepthPolicy::OverlayAlways.terrain_fill_offset(),
            None
        );
        assert_eq!(
            OverlayDepthPolicy::OverlayAlways.overlay_fill_offset(),
            None
        );
        assert!(OverlayDepthPolicy::OverlayAlways.disables_decal_depth_test());
    }
    #[test]
    fn native_world_node_selector_matches_original_464e60_queued_controls() {
        use super::{
            NativeModelFogPass::{Fog, Near},
            WorldModelFog,
        };
        let fog = WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 1000,
                far_raw: 2000,
            },
            color: [0.0; 3],
        };
        // Promoted original-PE controls retain queue-absent/pass3 separately;
        // these seven rows execute the active queued-world selector.
        for (z, radius, flags, expected) in [
            (899, 100, 0, Some(Near)),
            (900, 100, 0, Some(Fog)),
            (2099, 100, 0, Some(Fog)),
            (2100, 100, 0, None),
            (2100, 100, 0x20, Some(Near)),
            (-64536, 65535, 0, Some(Near)),
        ] {
            assert_eq!(fog.select_node(z, radius, flags), expected);
        }
        let wrapped = WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 1000,
                far_raw: i32::MAX,
            },
            ..fog
        };
        assert_eq!(wrapped.select_node(i32::MAX, 1, 0), Some(Near));
        // A bypassed parent does not change the independent child's header.
        assert_eq!(fog.select_node(2100, 100, 0), None);
    }
}
