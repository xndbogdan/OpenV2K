//! Shared submission of Section-8 model hierarchies to the runtime renderer.
//!
//! Models, inline instances, linked parent slots, billboards, global material
//! references, and context-specific type-13 callback faces must travel through one
//! implementation. The game and asset workbench both use this module so a
//! diagnostic preview cannot quietly become a second renderer.

use std::cell::RefCell;
use std::collections::HashMap;

use v2k_formats::models::{
    AnimVars, Billboard, LinkedModelSlots, MaterializedModel, ModelEntry, ModelFaceCull,
    ModelFaceShading, ModelFaceVertices, ModelInstance, ModelMaterializationContext,
    ModelNativeEdgeFailure, ModelNativeSpatialGenerator, ModelNativeVertexFailure, ModelVertexKind,
    ModelVertexProjection, ModelVertexResolver, ModelViewSelection, ResolvedModelSlot,
};
use v2k_formats::system::PaletteEntry;
use v2k_render::renderer::ModelSurfaceResolution;
use v2k_render::renderer::{
    retail_world_model_light_direction_raw, RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
};
use v2k_render::{
    mat3_mul, orientation_f32, BillboardMaterial, ExternalFrameMode, FaceMaterial,
    ModelBillboardDraw, ModelDepthFade, ModelDraw, ModelMesh, ModelNearClip, ModelOverlayKind,
    ModelPainterNode, ModelTransform, Renderer, ViewPinMode, WorldSurfaceProjection,
};

use crate::model_color::ModelMaterialCache;
use crate::resource_cache::ResourceCache;

/// World tf12/tf13 callbacks resolve inside the slot dependency graph.
/// Resolve in raw-local space before generators and linked imports consume it;
/// collision and intrinsic asset decoding deliberately retain the pure alias.
struct WorldModelVertexResolver<'a> {
    surface: Option<WorldSurfaceProjection<'a>>,
    transform: ModelTransform,
    view: Option<ModelTreeView>,
    native_view: Option<ModelViewSelection>,
    native_context: Option<(
        crate::native_model_frame::NativeModelFrame,
        crate::native_model_frame::NativeWorldViewport,
    )>,
    native_edges_required: bool,
    model: &'a ModelEntry,
    external_owner: Option<&'a dyn Fn([i16; 3]) -> Option<external_frame::ActorExternalSlotPoint>>,
    external_points: RefCell<HashMap<u16, Option<external_frame::ActorExternalSlotPoint>>>,
    native_vertex_failures: RefCell<Vec<ModelNativeVertexFailure>>,
}

impl ModelVertexResolver for WorldModelVertexResolver<'_> {
    fn resolve_vertex_raw(
        &self,
        kind: ModelVertexKind,
        source: ResolvedModelSlot,
    ) -> Option<ResolvedModelSlot> {
        let surface = self.surface?;
        let source_raw = source.position_raw;
        if let Some((_, viewport)) = self.native_context.as_ref() {
            let view = source.native_view_point?;
            let native = match kind {
                ModelVertexKind::ViewPin => {
                    crate::native_world_surface::project_native_world_surface(
                        view, *viewport, surface,
                    )
                }
                ModelVertexKind::Alias => crate::native_world_surface::project_native_world_alias(
                    view, *viewport, surface,
                ),
            };
            // The native callback's semantic world endpoint is transported
            // into the same selected draw image. It is never inverted through
            // the body merely to recover VIEW cache custody.
            let draw_image = self
                .view?
                .position
                .map(|value| (value * 256.0).round() as i32);
            let draw_world = std::array::from_fn(|axis| {
                f64::from(
                    native.world_raw[axis]
                        .wrapping_add(draw_image[axis].wrapping_sub(viewport.origin_raw[axis])),
                ) / 256.0
            });
            let mut result = ResolvedModelSlot::clear(
                v2k_render::gl_backend::world_point_as_raw_local(
                    draw_world.map(|v| v as f32),
                    self.transform.orientation,
                    self.transform.position,
                    self.transform.scale,
                )
                .map(f64::from),
            );
            result.world_point = Some(draw_world);
            result.native_view_point = Some(native.view_raw);
            result.clip = native.clip;
            result.surface_origin = if kind == ModelVertexKind::ViewPin {
                v2k_formats::models::ModelSurfaceOrigin::ViewPin
            } else {
                v2k_formats::models::ModelSurfaceOrigin::None
            };
            return Some(result);
        }
        if let Some(world) = source.world_point {
            let world = world.map(|component| component as f32);
            let mut resolved = match kind {
                ModelVertexKind::Alias => {
                    let mut point = world;
                    point[1] =
                        f32::from(surface.terrain().bilinear_height_raw(
                            (point[0] * 256.0) as i16,
                            (point[2] * 256.0) as i16,
                        )) / 256.0;
                    let mut slot = ResolvedModelSlot::clear([0.0; 3]);
                    slot.world_point = Some(point.map(f64::from));
                    slot
                }
                ModelVertexKind::ViewPin => {
                    v2k_render::gl_backend::retail_world_surface_point(world, surface)
                }
            };
            resolved.position_raw = v2k_render::gl_backend::world_point_as_raw_local(
                resolved.world_point?.map(|component| component as f32),
                self.transform.orientation,
                self.transform.position,
                self.transform.scale,
            )
            .map(f64::from);
            return Some(resolved);
        }
        Some(match kind {
            ModelVertexKind::Alias => ResolvedModelSlot::clear(
                v2k_render::gl_backend::retail_world_surface_alias_vertex(
                    source_raw.map(|v| v as f32),
                    self.transform.orientation,
                    self.transform.position,
                    self.transform.scale,
                    surface.terrain(),
                )
                .map(f64::from),
            ),
            ModelVertexKind::ViewPin => v2k_render::gl_backend::retail_world_surface_vertex(
                source_raw.map(|v| v as f32),
                self.transform.orientation,
                self.transform.position,
                self.transform.scale,
                surface,
            ),
        })
    }

    fn resolve_native_view_point(&self, slot: u16, _vars: &AnimVars) -> Option<[i32; 3]> {
        let (frame, _) = self.native_context.as_ref()?;
        let &[kind, a, b, c] = self.model.records.get(usize::from(slot >> 1))?;
        matches!(kind, 0 | 4).then(|| frame.plain_slot_view_raw(slot, kind, [a, b, c]))
    }

    fn resolve_native_spatial_point(
        &self,
        generator: ModelNativeSpatialGenerator,
    ) -> Option<[i32; 3]> {
        let (frame, _) = self.native_context.as_ref()?;
        Some(generator.evaluate(frame.origin_view_raw))
    }

    fn report_native_vertex_failure(&self, failure: ModelNativeVertexFailure) {
        let mut failures = self.native_vertex_failures.borrow_mut();
        if !failures.contains(&failure) {
            failures.push(failure);
        }
    }

    fn owns_external_frame_slots(&self) -> bool {
        self.external_owner.is_some()
    }
    fn owns_native_view(&self) -> bool {
        self.native_context.is_some()
    }
    fn native_node_origin_view(&self) -> Option<[i32; 3]> {
        self.native_context
            .as_ref()
            .map(|(frame, _)| frame.origin_view_raw)
    }
    fn requires_native_edge_endpoints(&self) -> bool {
        self.native_edges_required
    }

    fn admit_face_raw(&self, anchor: [i32; 3], normal: [i32; 3]) -> bool {
        if self.external_owner.is_none() {
            return true;
        }
        if let Some(ModelViewSelection::Retail {
            local_origin_from_camera_raw: origin,
        }) = self.native_view
        {
            return (0..3).fold(0_i32, |sum, axis| {
                sum.wrapping_add(
                    origin[axis]
                        .wrapping_add(anchor[axis])
                        .wrapping_mul(normal[axis]),
                )
            }) < 0;
        }
        self.view.is_some_and(|view| {
            v2k_render::authored_face_plane_visible(
                v2k_formats::models::ModelFaceCullPlane {
                    anchor_raw: anchor.map(f64::from),
                    native_anchor_raw: Some(anchor),
                    normal_raw: normal,
                },
                self.transform.orientation,
                self.transform.position,
                self.transform.scale,
                view.position,
            )
        })
    }

    fn resolve_external_frame_raw(
        &self,
        slot: u16,
        parameters: [i16; 3],
    ) -> Option<ResolvedModelSlot> {
        let owner = self.external_owner?;
        let cached = self.external_points.borrow().get(&slot).copied();
        let world = if let Some(point) = cached {
            point
        } else {
            let point = owner(parameters);
            self.external_points.borrow_mut().insert(slot, point);
            point
        }?;
        let mut resolved = ResolvedModelSlot::clear(
            v2k_render::gl_backend::world_point_as_raw_local(
                world.draw_world,
                self.transform.orientation,
                self.transform.position,
                self.transform.scale,
            )
            .map(f64::from),
        );
        resolved.world_point = Some(world.draw_world.map(f64::from));
        resolved.native_view_point = self.native_context.as_ref().map(|(_, viewport)| {
            // 40D350 rebases each callback WORD around viewport+78 before
            // 6ECF0 subtracts the full origin and projects it into VIEW.
            let image =
                viewport.actor_world_image(world.native_world_raw.map(|value| value as i16));
            viewport.world_vector_to_view(std::array::from_fn(|axis| {
                image[axis].wrapping_sub(viewport.origin_raw[axis])
            }))
        });
        Some(resolved)
    }
}

mod external_frame;
mod submission_buffer;
pub use submission_buffer::ModelTreeSubmissionBuffer;
mod painter;
mod view;
use external_frame::ModelTreeExternalFrame;
pub use painter::ModelTreePainterComposition;
pub use view::ModelTreeView;
use view::{model_far_plane, ModelTreeViewPolicy};

use v2k_render::renderer::ModelDepthPolicy;

/// Optional tint applied after resolving authored face and billboard assets.
#[derive(Clone, Copy, Debug)]
pub struct ModelSceneLight {
    pub tint: [f32; 3],
    pub emissive: [f32; 3],
}

impl ModelSceneLight {
    pub const NEUTRAL: Self = Self {
        tint: [1.0; 3],
        emissive: [0.0; 3],
    };
}

/// Whether a Section-8 model is one of the engine's camera-facing flat actors.
///
/// These actors are authored as one textured XY quad plus type-13 footprint
/// supports rather than as Section-8 billboard commands. Keep the policy
/// structural so world, cinematic, and HUD cargo presentations agree without
/// naming `man2`, `lev1sci2`, or any other particular resource.
pub fn model_is_camera_facing_actor(model: &ModelEntry) -> bool {
    let is_footprint = |triangle: &&[u16; 3]| {
        triangle
            .iter()
            .all(|&index| model.vertex_type_flags[index as usize] == 13)
    };
    let body_face_count = model
        .triangles
        .iter()
        .filter(|face| !is_footprint(face))
        .count();
    let body_materials_are_textured = model
        .triangles
        .iter()
        .enumerate()
        .filter(|(_, face)| !is_footprint(face))
        .all(|(index, _)| {
            model
                .face_materials
                .get(index)
                .is_some_and(|material| *material & 0x8000 != 0)
        });
    let vertex_indices = ModelNodeGeometry::from_static(model).submitted_vertex_indices();
    if body_face_count != 2
        || !model.billboards.is_empty()
        || !model.instances.is_empty()
        || model.face_materials.len() != model.triangles.len()
        || !body_materials_are_textured
        || vertex_indices
            .iter()
            .filter(|&&index| model.vertex_type_flags.get(index) == Some(&13))
            .count()
            < 2
    {
        return false;
    }
    let extent = |axis: usize| {
        let (min, max) = vertex_indices
            .iter()
            .map(|&index| &model.vertices[index])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), vertex| {
                (min.min(vertex[axis]), max.max(vertex[axis]))
            });
        max - min
    };
    extent(0) > 1.0 && extent(1) > 1.0 && extent(2).abs() < 1.0e-6
}

/// Per-child orientation hook for special live animation layered over an
/// authored instance transform (for example the player fan-blade spin).
pub trait ModelTreeChildTransform {
    fn transform(
        &self,
        child_id: usize,
        child_name: Option<&str>,
        authored_local: [[f32; 3]; 3],
    ) -> [[f32; 3]; 3];

    /// A presentation hook must explicitly retain the authored native mount
    /// for this child. Other policies cannot provide integer slot custody.
    fn native_frame_policy(
        &self,
        _child_id: usize,
        _child_name: Option<&str>,
    ) -> ModelTreeNativeChildPolicy {
        ModelTreeNativeChildPolicy::Unowned
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTreeNativeChildPolicy {
    Authored,
    Unowned,
}

/// Presentation of the points exported by the root's op-0x0E links.
/// Child attachments, model geometry, and descendant link policies remain
/// authored; adjusted root points travel through the normal linked traversal.
#[derive(Clone, Copy, Debug, Default)]
pub enum ModelTreeRootLinkPolicy {
    #[default]
    Authored,
    /// Klaus root slots 2/4 (and their mirrored partners) are the outer
    /// supports of the head/jaw's palette-32 matte. Extend only their X
    /// coordinates for a wider logical viewport, preserving the mouth and
    /// ordinary menu projection. 4:3 and Stretched have 4:3 logical surfaces.
    KlausMatte { viewport: (u32, u32) },
}

impl ModelTreeRootLinkPolicy {
    fn point(self, slot: u16, mut position: [f64; 3]) -> [f64; 3] {
        if let Self::KlausMatte { viewport } = self {
            if matches!(slot & !1, 2 | 4) {
                let horizontal_extent = (f64::from(viewport.0.max(1)) * 3.0
                    / (f64::from(viewport.1.max(1)) * 4.0))
                    .max(1.0);
                position[0] *= horizontal_extent;
            }
        }
        position
    }
}

/// One authored extreme point resolved through a live Section-8 hierarchy.
///
/// This is used for attachment-like points which the format does not expose as
/// a separate socket. `origin_world` is the centroid of the named model's
/// furthest vertices along the requested model-local axis; `direction_unit`
/// is that same axis after every authored and live child transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelTreeNamedTip {
    pub model_id: usize,
    pub origin_world: [f32; 3],
    pub direction_unit: [f32; 3],
}

/// World-space origin of one named node in a live Section-8 hierarchy.
///
/// Unlike [`ModelTreeNamedTip`], this is the instance attach point the draw
/// path already uses for the child — not a vertex-extreme on that child's body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelTreeNamedOrigin {
    pub model_id: usize,
    pub origin_world: [f32; 3],
}

/// Complete request for resolving named model tips through the same linked
/// materialization and child-transform policy as the draw path.
pub struct ModelTreeNamedTipRequest<'a> {
    pub model_id: usize,
    pub orientation: [[f32; 3]; 3],
    pub position: [f32; 3],
    pub scale: f32,
    pub depth: u8,
    pub linked: Option<&'a LinkedModelSlots>,
    pub vars: &'a AnimVars,
    pub child_transform: Option<&'a dyn ModelTreeChildTransform>,
    pub model_name: &'a str,
    pub local_tip_axis: [f32; 3],
}

/// Optional diagnostic presentation controls. Runtime callers retain the
/// default authored-material body path; the workbench can isolate textures or
/// preview canonical all-type-13 faces as an unprojected shadow layer without
/// implementing another renderer. Canonical Section-8 type-13 faces remain in
/// the authored body regardless of the `shadows` toggle.
#[derive(Clone, Copy, Debug)]
pub struct ModelTreeStyle {
    pub textures: bool,
    pub shadows: bool,
}

/// Borrowed geometry for one static or live-materialized hierarchy node.
///
/// Traversal remains deliberately separate because static and linked children
/// have different resolution rules. Once a node has been resolved, however,
/// its authored body, footprint, billboard, UV, normal, and material streams
/// all belong to the same renderer submission policy.
struct ModelNodeGeometry<'a> {
    model_flags: u8,
    radius_raw: u16,
    vertices: &'a [[f64; 3]],
    vertex_type_flags: &'a [i16],
    vertex_projection: &'a [ModelVertexProjection],
    vertex_clip: &'a [v2k_formats::models::ModelSlotClip],
    vertex_surface_origin: &'a [v2k_formats::models::ModelSurfaceOrigin],
    vertex_view_raw: &'a [Option<[i32; 3]>],
    triangles: &'a [[u16; 3]],
    face_vertices: &'a [ModelFaceVertices],
    normals: &'a [[f32; 3]],
    face_cull: &'a [ModelFaceCull],
    face_materials: &'a [u16],
    face_uvs: &'a [[[f32; 2]; 3]],
    face_corner_normals: &'a [[[f32; 3]; 3]],
    face_normals_raw: &'a [[[i32; 3]; 4]],
    face_shading: &'a [ModelFaceShading],
    edges: &'a [v2k_formats::models::ModelEdge],
    edge_projection: v2k_render::renderer::ModelEdgeProjection<'a>,
    billboards: &'a [Billboard],
    /// Retail group, primitive and instance order; empty for subsets.
    painter_program: &'a [v2k_formats::models::ModelPainterOp],
}

/// How a submitted node takes part in a backend's retail painter queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodePainter {
    /// A tree node expanding its parent's `Instance` op (`None`: a root).
    /// [`ModelTreeRenderer::end_model_node`] follows its children.
    Tree { parent_instance: Option<usize> },
    /// Already ordered by the caller (the isolated painter path).
    Ordered,
}

impl<'a> ModelNodeGeometry<'a> {
    /// Visible geometry consumers must not measure support-only vertices
    /// retained for screen-midpoint admission. Preserve point-cloud models
    /// when the command stream has no drawable primitives.
    fn submitted_vertex_indices(&self) -> Vec<usize> {
        if self.triangles.is_empty() && self.edges.is_empty() && self.billboards.is_empty() {
            return (0..self.vertices.len()).collect();
        }
        let mut referenced = vec![false; self.vertices.len()];
        for index in self
            .triangles
            .iter()
            .flatten()
            .copied()
            .chain(self.edges.iter().flat_map(|edge| edge.vertices))
            .chain(self.billboards.iter().map(|billboard| billboard.vertex))
        {
            if let Some(value) = referenced.get_mut(usize::from(index)) {
                *value = true;
            }
        }
        referenced
            .iter()
            .enumerate()
            .filter_map(|(index, &used)| used.then_some(index))
            .collect()
    }

    fn from_static(model: &'a ModelEntry) -> Self {
        Self {
            model_flags: model.flags,
            radius_raw: model.radius,
            vertices: &model.vertices,
            vertex_type_flags: &model.vertex_type_flags,
            vertex_projection: &model.vertex_projection,
            vertex_clip: &model.vertex_clip,
            vertex_surface_origin: &model.vertex_surface_origin,
            vertex_view_raw: &[],
            triangles: &model.triangles,
            face_vertices: &model.face_vertices,
            normals: &model.normals,
            face_cull: &model.face_cull,
            face_materials: &model.face_materials,
            face_uvs: &model.face_uvs,
            face_corner_normals: &model.face_corner_normals,
            face_normals_raw: &[],
            face_shading: &model.face_shading,
            edges: &model.edges,
            edge_projection: v2k_render::renderer::ModelEdgeProjection::Compatibility,
            billboards: &model.billboards,
            painter_program: &model.painter_program,
        }
    }

    fn from_materialized(model: &'a MaterializedModel, model_flags: u8, radius_raw: u16) -> Self {
        Self {
            model_flags,
            radius_raw,
            vertices: &model.vertices,
            vertex_type_flags: &model.vertex_type_flags,
            vertex_projection: &model.vertex_projection,
            vertex_clip: &model.vertex_clip,
            vertex_surface_origin: &model.vertex_surface_origin,
            vertex_view_raw: &model.vertex_view_raw,
            triangles: &model.triangles,
            face_vertices: &model.face_vertices,
            normals: &model.normals,
            face_cull: &model.face_cull,
            face_materials: &model.face_materials,
            face_uvs: &model.face_uvs,
            face_corner_normals: &model.face_corner_normals,
            face_normals_raw: &model.face_normals_raw,
            face_shading: &model.face_shading,
            edges: &model.edges,
            edge_projection: v2k_render::renderer::ModelEdgeProjection::CommandSnapshots(
                &model.edge_endpoint_snapshots,
            ),
            billboards: &model.billboards,
            painter_program: &model.painter_program,
        }
    }
}

/// FUN_00464E60 forces the near constructor table for header bit 0x20.
/// Resolve this at each node; the flag belongs to that model, not its children.
fn model_node_depth_fade(model_flags: u8, requested: ModelDepthFade) -> ModelDepthFade {
    if model_flags & 0x20 != 0 {
        ModelDepthFade::Disabled
    } else {
        requested
    }
}

impl Default for ModelTreeStyle {
    fn default() -> Self {
        Self {
            textures: true,
            shadows: false,
        }
    }
}

/// Apply a scene tint to resolved face materials.
pub fn apply_model_scene_light(materials: &mut [FaceMaterial], light: ModelSceneLight) {
    for material in materials {
        material.color[0] *= light.tint[0];
        material.color[1] *= light.tint[1];
        material.color[2] *= light.tint[2];
        material.emissive = light.emissive;
    }
}

/// A failed reached native edge retained independently of renderable geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelTreeNativeEdgeFailure {
    pub model_id: usize,
    pub command: ModelNativeEdgeFailure,
}

/// A skipped world callback with the reached dependency that lost native VIEW custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelTreeNativeVertexFailure {
    pub model_id: usize,
    pub dependency: ModelNativeVertexFailure,
}

fn report_native_vertex_failure(failure: ModelTreeNativeVertexFailure) {
    static REPORTED: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashSet<(usize, ModelNativeVertexFailure)>>,
    > = std::sync::OnceLock::new();
    if let Ok(mut reported) = REPORTED.get_or_init(Default::default).lock() {
        if reported.insert((failure.model_id, failure.dependency)) {
            eprintln!("native model vertex presentation blocked: {:?} (model{} consumer slot{} source slot{}; callback geometry skipped)",
                failure.dependency.boundary, failure.model_id,
                failure.dependency.consumer_slot, failure.dependency.source_slot);
        }
    }
}

fn native_edge_endpoints_required(
    native_view_owned: bool,
    authority: v2k_render::projection::SceneProjectionAuthority,
) -> bool {
    native_view_owned
        && !matches!(
            authority,
            v2k_render::projection::SceneProjectionAuthority::Compatibility(_)
        )
}

fn report_native_edge_failure(failure: ModelTreeNativeEdgeFailure) {
    // Keep the per-draw data receipt above even when a repeated unsupported
    // family has already been reported. A failing feature must remain visible
    // without flooding every frame or borrowing a GPU primitive for logging.
    static REPORTED: std::sync::OnceLock<
        std::sync::Mutex<
            std::collections::HashSet<(usize, v2k_formats::models::ModelNativeEdgeBoundary)>,
        >,
    > = std::sync::OnceLock::new();
    let reported = REPORTED.get_or_init(Default::default);
    if let Ok(mut reported) = reported.lock() {
        if reported.insert((failure.model_id, failure.command.boundary)) {
            eprintln!("native model edge presentation blocked: {:?} (model{} source edge{} slots{:?}; no geometry remap)", failure.command.boundary, failure.model_id, failure.command.source_edge_index, failure.command.source_slots);
        }
    }
}

/// Runtime model hierarchy submitter shared by game scenes and diagnostics.
pub struct ModelTreeRenderer<'a> {
    renderer: &'a mut dyn Renderer,
    submission_buffer: Option<&'a mut ModelTreeSubmissionBuffer>,
    cache: &'a ResourceCache,
    colors: &'a ModelMaterialCache,
    scale: f32,
    light: Option<ModelSceneLight>,
    view_pin: ViewPinMode,
    world_surface: Option<WorldSurfaceProjection<'a>>,
    external_frame: ModelTreeExternalFrame<'a>,
    overlay: ModelOverlayKind,
    style: ModelTreeStyle,
    palette_override: Option<&'a [PaletteEntry]>,
    light_direction_raw: [i32; 3],
    shade_shift: i32,
    depth_fade: ModelDepthFade,
    near_clip: ModelNearClip,
    root_link_policy: ModelTreeRootLinkPolicy,
    view_policy: ModelTreeViewPolicy,
    world_fog_planes: Option<[f32; 2]>,
    world_model_fog: Option<v2k_render::WorldModelFog>,
    projection_authority: v2k_render::projection::SceneProjectionAuthority,
    native_edge_failures: Vec<ModelTreeNativeEdgeFailure>,
    native_vertex_failures: Vec<ModelTreeNativeVertexFailure>,
}

impl<'a> ModelTreeRenderer<'a> {
    pub fn new(
        renderer: &'a mut dyn Renderer,
        cache: &'a ResourceCache,
        colors: &'a ModelMaterialCache,
        scale: f32,
        light: Option<ModelSceneLight>,
        view_pin: ViewPinMode,
    ) -> Self {
        let world_fog_planes = renderer.world_fog_planes();
        let world_model_fog = renderer.world_model_fog();
        let projection_authority = renderer.scene_projection_authority();
        Self {
            renderer,
            submission_buffer: None,
            cache,
            colors,
            scale,
            light,
            view_pin,
            world_surface: None,
            external_frame: ModelTreeExternalFrame::Fixed(ExternalFrameMode::Raw),
            overlay: ModelOverlayKind::None,
            style: ModelTreeStyle::default(),
            palette_override: None,
            light_direction_raw: RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
            shade_shift: 0,
            depth_fade: ModelDepthFade::InheritWorldFog,
            near_clip: ModelNearClip::Camera,
            root_link_policy: ModelTreeRootLinkPolicy::Authored,
            view_policy: ModelTreeViewPolicy::Intrinsic,
            world_fog_planes,
            world_model_fog,
            projection_authority,
            native_edge_failures: Vec::new(),
            native_vertex_failures: Vec::new(),
        }
    }

    /// Construct the model context installed by retail world setup.
    ///
    /// Unlike menu/default type-13 callbacks, this policy requires the live
    /// Section-10 terrain and wrapping 50 Hz clock. Missing terrain remains an
    /// explicit fail-closed context in the backend.
    pub fn new_world(
        renderer: &'a mut dyn Renderer,
        cache: &'a ResourceCache,
        colors: &'a ModelMaterialCache,
        scale: f32,
        light: Option<ModelSceneLight>,
        retail_tick: i32,
    ) -> Self {
        let mut tree = Self::new(
            renderer,
            cache,
            colors,
            scale,
            light,
            ViewPinMode::WorldSurface,
        );
        tree.world_surface = cache.terrain().map(|terrain| {
            WorldSurfaceProjection::new(terrain, retail_tick).with_waves_enabled(
                cache
                    .level_desc()
                    .and_then(|level| level.raw_u32(0x84))
                    .unwrap_or(0)
                    != 0,
            )
        });
        if let Some(terrain) = cache.terrain() {
            tree.light_direction_raw = retail_world_model_light_direction_raw(terrain);
        }
        tree.near_clip = ModelNearClip::RetailWorld;
        tree
    }

    /// Capture owned commands after callbacks without rasterizing yet. The
    /// caller drains them after the source-ordered particle/terrain pass.
    pub fn with_submission_buffer(mut self, buffer: &'a mut ModelTreeSubmissionBuffer) -> Self {
        self.submission_buffer = Some(buffer);
        self
    }

    /// Override the captured scene authority for an explicitly controlled draw.
    /// Runtime callers normally use the renderer's source-published authority.
    pub fn with_scene_projection_authority(
        mut self,
        authority: v2k_render::projection::SceneProjectionAuthority,
    ) -> Self {
        self.projection_authority = authority;
        self
    }

    pub fn world_surface_projection(&self) -> Option<WorldSurfaceProjection<'a>> {
        self.world_surface
    }

    fn emit_model_body(&mut self, draw: ModelDraw<'_>) {
        if let Some(buffer) = self.submission_buffer.as_mut() {
            buffer.push_body(draw);
        } else {
            self.renderer.draw_model_body(draw);
        }
    }

    fn emit_model_billboards(&mut self, draw: ModelBillboardDraw<'_>) {
        if let Some(buffer) = self.submission_buffer.as_mut() {
            buffer.push_billboards(draw);
        } else {
            self.renderer.draw_model_billboards(draw);
        }
    }

    /// Close the latest tree node opened by [`NodePainter::Tree`].
    fn end_model_node(&mut self) {
        if let Some(buffer) = self.submission_buffer.as_mut() {
            buffer.push_end_node();
        } else {
            self.renderer.end_model_node();
        }
    }

    pub fn with_style(mut self, style: ModelTreeStyle) -> Self {
        self.style = style;
        self
    }

    /// Install the type-14 callback result shared by this hierarchy.
    ///
    /// Retail keeps the callback/data pair in the render context while op-0x0E
    /// traverses children, so one recovered external frame applies to every
    /// node rather than being recomputed from a child attachment.
    pub fn with_external_frame(mut self, external_frame: ExternalFrameMode) -> Self {
        self.external_frame = ModelTreeExternalFrame::Fixed(external_frame);
        self
    }

    /// [`Self::with_external_frame`] for a hierarchy whose root node frame is
    /// source-owned, as static terrain objects' is: every reached node then
    /// carries its integer VIEW frame, which its vertices' VIEW points,
    /// view-dependent commands and node fog selection use. Children derive
    /// their frames from their parent's as they are reached.
    pub fn with_native_external_frame(
        mut self,
        external_frame: ExternalFrameMode,
        root: Option<(
            crate::native_model_frame::NativeModelFrame,
            crate::native_model_frame::NativeWorldViewport,
        )>,
    ) -> Self {
        self.external_frame = ModelTreeExternalFrame::NativeFixed {
            mode: external_frame,
            native_context: root,
        };
        self
    }

    /// Commit draw-owned Sub-H caches only for selectors the submitted nodes
    /// actually request. The root actor context persists through its children.
    pub fn with_sub_h_presentation(
        mut self,
        presentation: crate::sub_h_external_frame::SubHPresentation<'a>,
    ) -> Self {
        self.external_frame = ModelTreeExternalFrame::SubH(presentation);
        self
    }

    /// Actors with Sub-E and no Sub-H still own the A9F0 selector domain.
    /// Keep emitter callbacks in command materialization, with the current
    /// native node frame, rather than resolving a gun point before drawing.
    pub fn with_actor_emitter_presentation(
        mut self,
        presentation: crate::actor_emitter_external_frame::ActorEmitterModelPresentation<'a>,
    ) -> Self {
        self.external_frame = ModelTreeExternalFrame::Emitter(presentation);
        self
    }

    /// Opt this hierarchy into the terrain-overlay depth policy.
    ///
    /// Use for draws that sit on the recovered terrain hit without authored
    /// type-13 vertices (the Targetter terrain crosshair). All-type-13 world
    /// surfaces already detect themselves from vertex flags.
    pub fn with_overlay(mut self, overlay: ModelOverlayKind) -> Self {
        self.overlay = overlay;
        self
    }

    /// Override only flat-colour Section-7 material lookup.
    ///
    /// Runtime callers should retain the default active-cache policy. This is
    /// useful to diagnostics which intentionally keep multiple mutually
    /// exclusive biome packs resident in one cache.
    pub fn with_palette_override(mut self, palette: &'a [PaletteEntry]) -> Self {
        self.palette_override = Some(palette);
        self
    }

    /// Install the signed raw VIEW-space model-light vector for this
    /// hierarchy.
    ///
    /// Local contexts retain the default `(73,73,-73)` and world trees start
    /// from the level's reduced Section-10 direction. Frontend Klaus uses the
    /// distinct `(-100,50,-50)` vector recovered from its persistent world
    /// context.
    pub fn with_light_direction_raw(mut self, light_direction_raw: [i32; 3]) -> Self {
        self.light_direction_raw = light_direction_raw;
        self
    }

    /// Apply retail's contextual signed shift to the model-light table.
    /// Gameplay supplies terrain light minus underwater darkness; menu and
    /// standalone viewer draws retain the default unshifted table.
    pub fn with_shade_shift(mut self, shade_shift: i32) -> Self {
        self.shade_shift = shade_shift;
        self
    }

    /// Install one submission-local `FUN_00470A10` depth-fade policy for the
    /// complete hierarchy. Frontend callers use this for Klaus's live planes
    /// and for each prop's root-relative band without changing persistent
    /// gameplay fog or the separately sorted menu billboard.
    pub fn with_depth_fade(mut self, depth_fade: ModelDepthFade) -> Self {
        self.depth_fade = depth_fade;
        self
    }

    /// Select the scene's near-rejection coordinate domain. Frontend geometry
    /// retains its domain when composed above a world scene, and diagnostic
    /// scaling does not implicitly select a retail policy.
    pub fn with_near_clip(mut self, near_clip: ModelNearClip) -> Self {
        self.near_clip = near_clip;
        self
    }

    /// Select a presentation adapter for this live hierarchy's root links.
    /// The adapter runs before conversion into the first child's local basis.
    pub fn with_root_link_policy(mut self, policy: ModelTreeRootLinkPolicy) -> Self {
        self.root_link_policy = policy;
        self
    }

    /// Use the authored CPU painter queue for this linked hierarchy. The
    /// default world path retains geometry depth testing. This adapter is
    /// currently admitted for face/instance/group programs such as Klaus.
    pub fn with_painter_view(
        mut self,
        view: ModelTreeView,
        composition: ModelTreePainterComposition,
    ) -> Self {
        self.view_policy = ModelTreeViewPolicy::Painter { view, composition };
        self
    }

    /// Select authored viewing-angle blocks using the active scene camera.
    /// Without a view, diagnostic draws retain all intrinsic view branches.
    /// This selects ordinary geometry depth; painter callers use
    /// `with_painter_view` to install the same view with authored queue ordering.
    pub fn with_view(mut self, view: ModelTreeView) -> Self {
        self.view_policy = ModelTreeViewPolicy::Geometry(view);
        self
    }

    /// Explicit native-marker boundaries encountered by executed draw requests.
    pub fn native_sub_m_boundaries(&self) -> &[crate::sub_m_external_frame::NativeMarkerBoundary] {
        self.external_frame.native_sub_m_boundaries()
    }

    /// Failed reached native edges, including nodes with no surviving body.
    /// These receipts are consumed when the command materializes, before any
    /// deferred body buffer; geometry must not be fabricated to preserve them.
    pub fn native_edge_failures(&self) -> &[ModelTreeNativeEdgeFailure] {
        &self.native_edge_failures
    }

    /// Failed reached world callback dependencies, including nodes whose
    /// affected geometry was entirely skipped. Repeated frame logs are bounded.
    pub fn native_vertex_failures(&self) -> &[ModelTreeNativeVertexFailure] {
        &self.native_vertex_failures
    }

    fn consume_native_vertex_failures(
        &mut self,
        model_id: usize,
        failures: &[ModelNativeVertexFailure],
    ) {
        for &dependency in failures {
            let failure = ModelTreeNativeVertexFailure {
                model_id,
                dependency,
            };
            self.native_vertex_failures.push(failure);
            report_native_vertex_failure(failure);
        }
    }

    fn consume_native_edge_failures(
        &mut self,
        model_id: usize,
        failures: &[ModelNativeEdgeFailure],
    ) {
        for &command in failures {
            let failure = ModelTreeNativeEdgeFailure { model_id, command };
            self.native_edge_failures.push(failure);
            report_native_edge_failure(failure);
        }
    }

    /// Install draw-owned Sub-M selector zero for this complete hierarchy.
    /// Each nested node resolves the descriptor marker in its own context.
    pub fn with_sub_m_presentation(
        mut self,
        presentation: crate::sub_m_external_frame::SubMPresentation<'a>,
    ) -> Self {
        self.external_frame = ModelTreeExternalFrame::SubM(presentation);
        self
    }

    fn face_materials(&mut self, materials: &[u16]) -> Vec<FaceMaterial> {
        if let Some(palette) = self.palette_override {
            self.colors
                .materials_for_with_palette(self.cache, self.renderer, materials, palette)
        } else {
            self.colors
                .materials_for(self.cache, self.renderer, materials)
        }
    }

    fn billboard_materials(&mut self, billboards: &[Billboard]) -> Vec<BillboardMaterial> {
        if let Some(palette) = self.palette_override {
            self.colors.billboard_materials_with_palette(
                self.cache,
                self.renderer,
                billboards,
                palette,
            )
        } else {
            self.colors
                .billboard_materials(self.cache, self.renderer, billboards)
        }
    }

    fn apply_style(&self, materials: &mut [FaceMaterial]) {
        if !self.style.textures {
            for material in materials {
                if material.texture.take().is_some() {
                    material.color = [0.65, 0.68, 0.72];
                    material.palette_rgb555 = None;
                    material.emissive = [0.0; 3];
                }
            }
        }
    }

    fn apply_billboard_style(&self, billboards: &[Billboard], materials: &mut [BillboardMaterial]) {
        if self.style.textures {
            return;
        }
        for (billboard, material) in billboards.iter().zip(materials) {
            if billboard.textured && material.face.texture.take().is_some() {
                material.face.color = [0.65, 0.68, 0.72];
                material.face.palette_rgb555 = None;
                material.face.emissive = [0.0; 3];
            }
        }
    }

    fn draw_shadow_layer(
        &mut self,
        radius_raw: u16,
        vertices: &[[f64; 3]],
        vertex_type_flags: &[i16],
        authored_triangles: &[[u16; 3]],
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        requested_view_pin: ViewPinMode,
    ) {
        // The checkbox is a workbench diagnostic, not another runtime render
        // stream. CameraFacing hides canonical all-type-13 faces from its body
        // pass, so preview those exact faces unprojected when requested. Never
        // add this layer to WorldSurface, where the canonical faces are already
        // projected and submitted once through the retail callback.
        if !self.style.shadows || requested_view_pin != ViewPinMode::CameraFacing {
            return;
        }
        let triangles: Vec<[u16; 3]> = authored_triangles
            .iter()
            .copied()
            .filter(|triangle| {
                triangle
                    .iter()
                    .all(|&index| vertex_type_flags.get(index as usize).copied() == Some(13))
            })
            .collect();
        if triangles.is_empty() {
            return;
        }
        let materials = vec![
            FaceMaterial {
                color: [0.12, 0.12, 0.15],
                palette_rgb555: None,
                emissive: [0.0; 3],
                texture: None,
                blend: v2k_render::WorldSpriteBlend::Masked,
                flat_shade_row: v2k_formats::palette::BRIGHTEST_SHADE as u8,
            };
            triangles.len()
        ];
        self.emit_model_body(ModelDraw {
            projection_authority: self.projection_authority,
            mesh: ModelMesh {
                radius_raw,
                vertices,
                vertex_type_flags,
                vertex_projection: &[],
                vertex_clip: &[],
                vertex_surface_origin: &[],
                vertex_view_raw: &[],
                triangles: &triangles,
                // Filtered workbench shadow previews use triangle fallback.
                face_vertices: &[],
                normals: &[],
                face_cull: &[],
                face_uvs: &[],
                face_corner_normals: &[],
                face_normals_raw: &[],
                face_shading: &[],
                edges: &[],
                edge_projection: v2k_render::renderer::ModelEdgeProjection::Compatibility,
                edge_materials: &[],
                edge_widths: &[],
                shade_table: None,
                light_direction_raw: RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
                native_light_raw: None,
                shade_shift: 0,
                materials: &materials,
            },
            transform: ModelTransform {
                orientation,
                position,
                scale: self.scale,
            },
            depth_fade: self.depth_fade,
            near_clip: self.near_clip,
            depth_policy: ModelDepthPolicy::Geometry,
            view_pin: ViewPinMode::Raw,
            surface_resolution: ModelSurfaceResolution::Intrinsic,
            world_surface: None,
            external_frame: ExternalFrameMode::Raw,
            overlay: ModelOverlayKind::None,
            painter: None,
        });
    }

    fn light_billboards(&self, materials: &mut [BillboardMaterial]) {
        let Some(light) = self.light else {
            return;
        };
        for material in materials {
            material.face.color[0] *= light.tint[0];
            material.face.color[1] *= light.tint[1];
            material.face.color[2] *= light.tint[2];
            material.face.emissive = light.emissive;
        }
    }

    fn submit_node(
        &mut self,
        geometry: ModelNodeGeometry<'_>,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        requested_view_pin: ViewPinMode,
        depth_policy: ModelDepthPolicy,
        surface_resolution: ModelSurfaceResolution,
        depth_fade: ModelDepthFade,
        painter: NodePainter,
    ) {
        let external_frame = if surface_resolution == ModelSurfaceResolution::ContextResolved
            && self.external_frame.owns_external_vertices()
        {
            ExternalFrameMode::Raw
        } else {
            self.external_frame.resolve(
                &geometry,
                ModelTransform {
                    orientation,
                    position,
                    scale: self.scale,
                },
                self.view_policy.view(),
                self.world_surface,
            )
        };
        let mut face_materials = self.face_materials(geometry.face_materials);
        if let Some(light) = self.light {
            apply_model_scene_light(&mut face_materials, light);
        }
        self.apply_style(&mut face_materials);
        let packed_edge_materials: Vec<u16> = geometry
            .edges
            .iter()
            .map(|edge| edge.packed_material())
            .collect();
        let mut edge_materials = self.face_materials(&packed_edge_materials);
        self.apply_style(&mut edge_materials);
        // Resolve each sprite-style edge's authored width pair from the
        // sprite record's packed `+0x10/+0x12` word. Unresolved sprite
        // entries stay `(0, 0)` and drop. Palette `0x02` hairlines also
        // store `(0, 0)`; the selector submits those as lines.
        let edge_widths: Vec<(u16, u16)> = geometry
            .edges
            .iter()
            .map(|edge| match edge.style {
                v2k_formats::models::ModelEdgeStyle::Sprite { sprite_id, .. } => self
                    .cache
                    .global_sprite(sprite_id)
                    .map(|(_, entry)| (entry.flags as u16, (entry.flags >> 16) as u16))
                    .unwrap_or((0, 0)),
                v2k_formats::models::ModelEdgeStyle::Palette { .. } => (0, 0),
            })
            .collect();
        self.draw_shadow_layer(
            geometry.radius_raw,
            geometry.vertices,
            geometry.vertex_type_flags,
            geometry.triangles,
            orientation,
            position,
            requested_view_pin,
        );
        let mesh = ModelMesh {
            radius_raw: geometry.radius_raw,
            vertices: geometry.vertices,
            vertex_type_flags: geometry.vertex_type_flags,
            vertex_projection: geometry.vertex_projection,
            vertex_clip: geometry.vertex_clip,
            vertex_surface_origin: geometry.vertex_surface_origin,
            vertex_view_raw: geometry.vertex_view_raw,
            triangles: geometry.triangles,
            face_vertices: geometry.face_vertices,
            normals: geometry.normals,
            face_cull: geometry.face_cull,
            face_uvs: geometry.face_uvs,
            face_corner_normals: geometry.face_corner_normals,
            face_normals_raw: geometry.face_normals_raw,
            face_shading: geometry.face_shading,
            edges: geometry.edges,
            edge_projection: geometry.edge_projection,
            edge_materials: &edge_materials,
            edge_widths: &edge_widths,
            shade_table: self.cache.fog_gradient().map(Vec::as_slice),
            light_direction_raw: self.light_direction_raw,
            native_light_raw: self
                .external_frame
                .native_context()
                .map(|(frame, _)| frame.model_light_raw(self.light_direction_raw)),
            shade_shift: self.shade_shift,
            materials: &face_materials,
        };
        self.emit_model_body(ModelDraw {
            projection_authority: self.projection_authority,
            mesh,
            transform: ModelTransform {
                orientation,
                position,
                scale: self.scale,
            },
            depth_fade,
            near_clip: self.near_clip,
            depth_policy,
            view_pin: requested_view_pin,
            surface_resolution,
            world_surface: self.world_surface,
            external_frame,
            overlay: self.overlay,
            painter: match painter {
                NodePainter::Tree { parent_instance } => Some(ModelPainterNode {
                    program: geometry.painter_program,
                    parent_instance,
                }),
                NodePainter::Ordered => None,
            },
        });
        self.submit_node_billboards(&geometry, orientation, position, depth_policy, depth_fade);
    }

    fn submit_node_billboards(
        &mut self,
        geometry: &ModelNodeGeometry<'_>,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth_policy: ModelDepthPolicy,
        depth_fade: ModelDepthFade,
    ) {
        let mut billboard_materials = self.billboard_materials(geometry.billboards);
        self.light_billboards(&mut billboard_materials);
        self.apply_billboard_style(geometry.billboards, &mut billboard_materials);
        self.emit_model_billboards(ModelBillboardDraw {
            vertices: geometry.vertices,
            vertex_projection: geometry.vertex_projection,
            vertex_clip: geometry.vertex_clip,
            vertex_view_raw: geometry.vertex_view_raw,
            billboards: geometry.billboards,
            materials: &billboard_materials,
            transform: ModelTransform {
                orientation,
                position,
                scale: self.scale,
            },
            radius_raw: geometry.radius_raw,
            depth_fade,
            near_clip: self.near_clip,
            depth_policy,
        });
    }

    /// Draw cached default geometry and recurse through its authored children.
    pub fn draw_static(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        child_transform: Option<&dyn ModelTreeChildTransform>,
    ) {
        let vars = AnimVars::default();
        self.draw_static_node(
            model_id,
            orientation,
            position,
            depth,
            child_transform,
            &vars,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_static_node(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        child_transform: Option<&dyn ModelTreeChildTransform>,
        vars: &AnimVars,
        parent_instance: Option<usize>,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        let Some(depth_fade) = self.node_depth_fade(model, position) else {
            return;
        };
        // Keep cached geometry (notably the sky) only when neither callbacks
        // nor executed view commands need this node's presentation context.
        // Children still enter their own linked path with their own origin.
        if (self.view_pin == ViewPinMode::WorldSurface
            && model
                .records
                .iter()
                .any(|record| matches!(record[0], 12 | 13)))
            || (self.view_policy.view().is_some() && model.has_view_commands)
            || self.external_frame.needs_live_materialization()
        {
            self.draw_linked_node(
                model_id,
                orientation,
                position,
                depth,
                None,
                vars,
                child_transform,
                ModelTreeRootLinkPolicy::Authored,
                parent_instance,
            );
            return;
        }
        self.submit_node(
            ModelNodeGeometry::from_static(model),
            orientation,
            position,
            self.view_pin,
            ModelDepthPolicy::Geometry,
            ModelSurfaceResolution::Intrinsic,
            depth_fade,
            NodePainter::Tree { parent_instance },
        );
        if depth == 0 {
            self.end_model_node();
            return;
        }
        for (index, instance) in model.instances.iter().enumerate() {
            if instance.model_id as usize == model_id {
                continue;
            }
            let child_id = instance.model_id as usize;
            let child_name = self
                .cache
                .global_model(child_id)
                .and_then(|entry| entry.name.as_deref());
            let authored = orientation_f32(instance.orientation);
            let child_local = child_transform
                .map(|hook| hook.transform(child_id, child_name, authored))
                .unwrap_or(authored);
            let child_orientation = mat3_mul(orientation, child_local);
            let child_position = instance_world_position(
                orientation,
                position,
                instance.attach_pos.unwrap_or([0.0; 3]),
                self.scale,
            );
            let child_linked = build_child_linked_slots(
                model,
                None,
                instance,
                vars,
                ModelTreeRootLinkPolicy::Authored,
                None,
            );
            let child_vars = inherited_child_vars(vars, instance);
            self.draw_linked_node(
                child_id,
                child_orientation,
                child_position,
                depth - 1,
                Some(&child_linked),
                &child_vars,
                child_transform,
                ModelTreeRootLinkPolicy::Authored,
                Some(index),
            );
        }
        self.end_model_node();
    }

    /// Materialize a model with live animation variables and linked-parent
    /// slots, then recurse through the resulting instance hierarchy.
    pub fn draw_linked(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) {
        self.draw_linked_with_transform(model_id, orientation, position, depth, linked, vars, None);
    }

    /// Draw a live linked hierarchy while layering an optional transform over
    /// every authored child orientation. The hook propagates unchanged through
    /// nested instances, so animation policies do not need to bypass the
    /// linked-slot/materialization path to reach grandchildren.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_linked_with_transform(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
        child_transform: Option<&dyn ModelTreeChildTransform>,
    ) {
        if let ModelTreeViewPolicy::Painter { view, composition } = self.view_policy {
            self.draw_painter_hierarchy(
                model_id,
                orientation,
                position,
                depth,
                linked,
                vars,
                child_transform,
                view,
                composition,
            );
            return;
        }
        self.draw_linked_node(
            model_id,
            orientation,
            position,
            depth,
            linked,
            vars,
            child_transform,
            self.root_link_policy,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_linked_node(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
        child_transform: Option<&dyn ModelTreeChildTransform>,
        root_link_policy: ModelTreeRootLinkPolicy,
        parent_instance: Option<usize>,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        let Some(depth_fade) = self.node_depth_fade(model, position) else {
            return;
        };
        let (materialized, child_links, surface_resolution) = self.materialize_live_node(
            model_id,
            model,
            orientation,
            position,
            depth,
            linked,
            vars,
            root_link_policy,
        );
        self.submit_node(
            ModelNodeGeometry::from_materialized(&materialized, model.flags, model.radius),
            orientation,
            position,
            self.view_pin,
            ModelDepthPolicy::Geometry,
            surface_resolution,
            depth_fade,
            NodePainter::Tree { parent_instance },
        );
        if depth == 0 {
            self.end_model_node();
            return;
        }
        for (index, (instance, child_linked)) in materialized
            .instances
            .iter()
            .zip(child_links.iter())
            .enumerate()
        {
            if instance.model_id as usize == model_id {
                continue;
            }
            let child_id = instance.model_id as usize;
            let child_name = self
                .cache
                .global_model(child_id)
                .and_then(|entry| entry.name.as_deref());
            let authored = orientation_f32(instance.orientation);
            let child_local = child_transform
                .map(|hook| hook.transform(child_id, child_name, authored))
                .unwrap_or(authored);
            let child_orientation = mat3_mul(orientation, child_local);
            let child_position = instance_world_position(
                orientation,
                position,
                instance.attach_pos.unwrap_or([0.0; 3]),
                self.scale,
            );
            let child_vars = inherited_child_vars(vars, instance);
            let native_parent = self.external_frame.begin_child(
                model,
                instance,
                vars,
                child_transform.is_none_or(|hook| {
                    hook.native_frame_policy(child_id, child_name)
                        == ModelTreeNativeChildPolicy::Authored
                }),
                self.view_pin == ViewPinMode::WorldSurface,
                self.world_surface.map(|surface| surface.terrain()),
            );
            if !self.external_frame.native_child_is_owned(&native_parent) {
                self.external_frame.end_child(native_parent);
                continue;
            }
            self.draw_linked_node(
                child_id,
                child_orientation,
                child_position,
                depth - 1,
                child_linked.as_ref(),
                &child_vars,
                child_transform,
                ModelTreeRootLinkPolicy::Authored,
                Some(index),
            );
            self.external_frame.end_child(native_parent);
        }
        self.end_model_node();
    }

    /// One reached node is materialized once with the live H/E and native
    /// current-frame owners. Both geometry and the isolated painter consumer
    /// use this path; draining its prepared items cannot replay callbacks.
    #[allow(clippy::too_many_arguments)]
    fn materialize_live_node(
        &mut self,
        model_id: usize,
        model: &ModelEntry,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
        root_link_policy: ModelTreeRootLinkPolicy,
    ) -> (
        MaterializedModel,
        Vec<Option<LinkedModelSlots>>,
        ModelSurfaceResolution,
    ) {
        self.external_frame.prepare_node_surface(
            model,
            vars,
            linked,
            self.view_pin == ViewPinMode::WorldSurface,
            self.world_surface.map(|surface| surface.terrain()),
        );
        let live_external =
            self.view_policy.view().is_some() && self.external_frame.owns_external_vertices();
        let native_view = self.external_frame.native_view_selection();
        let native_context = self.external_frame.native_context();
        let owner = RefCell::new(&mut self.external_frame);
        let resolve_external = |parameters| {
            owner
                .borrow_mut()
                .resolve_parameter_world(parameters, self.world_surface)
        };
        let world_vertex =
            (self.view_pin == ViewPinMode::WorldSurface).then_some(WorldModelVertexResolver {
                surface: self.world_surface,
                transform: ModelTransform {
                    orientation,
                    position,
                    scale: self.scale,
                },
                view: self.view_policy.view(),
                native_view,
                native_edges_required: native_edge_endpoints_required(
                    native_context.is_some(),
                    self.projection_authority,
                ),
                native_context,
                model,
                external_owner: live_external.then_some(&resolve_external),
                external_points: RefCell::new(HashMap::new()),
                native_vertex_failures: RefCell::new(Vec::new()),
            });
        let context = ModelMaterializationContext {
            vars,
            linked,
            view_selection: native_view.unwrap_or_else(|| {
                self.view_policy
                    .selection(orientation, position, self.scale)
            }),
            vertex_resolver: world_vertex
                .as_ref()
                .map(|resolver| resolver as &dyn ModelVertexResolver),
        };
        let materialized = model.materialize_with_context(context);
        let child_links: Vec<_> = materialized
            .instances
            .iter()
            .map(|instance| {
                if depth == 0 || instance.model_id as usize == model_id {
                    None
                } else {
                    Some(build_child_linked_slots(
                        model,
                        linked,
                        instance,
                        vars,
                        root_link_policy,
                        context.vertex_resolver,
                    ))
                }
            })
            .collect();
        let context_resolved = world_vertex.is_some();
        let native_vertex_failures = world_vertex
            .as_ref()
            .map(|resolver| resolver.native_vertex_failures.borrow().clone())
            .unwrap_or_default();
        drop(world_vertex);
        drop(owner);
        self.consume_native_vertex_failures(model_id, &native_vertex_failures);
        self.consume_native_edge_failures(model_id, &materialized.native_edge_failures);
        let resolution = if context_resolved {
            ModelSurfaceResolution::ContextResolved
        } else {
            ModelSurfaceResolution::Intrinsic
        };
        (materialized, child_links, resolution)
    }

    fn node_depth_fade(&self, model: &ModelEntry, position: [f32; 3]) -> Option<ModelDepthFade> {
        // Only an owned queued world camera uses integer node admission.
        // Free/resize and explicit frontend fade policies keep the existing
        // float lane, even if a diagnostic happened to retain a native frame.
        if self.view_pin == ViewPinMode::WorldSurface
            && self.view_policy.view().is_some()
            && matches!(self.depth_fade, ModelDepthFade::InheritWorldFog)
            && matches!(
                self.projection_authority,
                v2k_render::projection::SceneProjectionAuthority::Native(_)
            )
        {
            if let (Some(fog), Some(origin)) = (
                self.world_model_fog,
                self.external_frame.native_origin_view_raw(),
            ) {
                return fog
                    .select_node(origin[2], model.radius, model.flags)
                    .map(|pass| ModelDepthFade::WorldRaw { fog, pass });
            }
        }
        let admitted = self.view_policy.view().is_none_or(|view| {
            view.admits_model(
                model.flags,
                model.radius,
                position,
                self.scale,
                model_far_plane(self.depth_fade, self.world_fog_planes),
            )
        });
        admitted.then(|| model_node_depth_fade(model.flags, self.depth_fade))
    }
}

/// Resolve every occurrence of one named child model to its authored extreme
/// point. Traversal order is authored instance order and is retained in the
/// returned vector; callers can therefore attach neutral A/B channel labels
/// without inventing a screen-relative side convention.
pub fn linked_model_named_tips(
    cache: &ResourceCache,
    request: ModelTreeNamedTipRequest<'_>,
) -> Vec<ModelTreeNamedTip> {
    let Some(local_tip_axis) = normalized(request.local_tip_axis) else {
        return Vec::new();
    };
    let mut traversal = NamedTipTraversal {
        cache,
        scale: request.scale,
        child_transform: request.child_transform,
        model_name: request.model_name,
        local_tip_axis,
        tips: Vec::new(),
    };
    traversal.visit(
        request.model_id,
        request.orientation,
        request.position,
        request.depth,
        request.linked,
        request.vars,
    );
    traversal.tips
}

/// Resolve every occurrence of one named child to the world-space attach
/// point the draw path already uses for that node.
pub fn linked_model_named_origins(
    cache: &ResourceCache,
    request: ModelTreeNamedTipRequest<'_>,
) -> Vec<ModelTreeNamedOrigin> {
    let mut traversal = NamedOriginTraversal {
        cache,
        scale: request.scale,
        child_transform: request.child_transform,
        model_name: request.model_name,
        origins: Vec::new(),
    };
    traversal.visit(
        request.model_id,
        request.orientation,
        request.position,
        request.depth,
        request.linked,
        request.vars,
    );
    traversal.origins
}

struct NamedOriginTraversal<'a> {
    cache: &'a ResourceCache,
    scale: f32,
    child_transform: Option<&'a dyn ModelTreeChildTransform>,
    model_name: &'a str,
    origins: Vec<ModelTreeNamedOrigin>,
}

impl NamedOriginTraversal<'_> {
    fn visit(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        if model.name.as_deref() == Some(self.model_name) {
            self.origins.push(ModelTreeNamedOrigin {
                model_id,
                origin_world: position,
            });
        }
        if depth == 0 {
            return;
        }
        let materialized = model.materialize_with_context(
            v2k_formats::models::ModelMaterializationContext::intrinsic(vars, linked),
        );
        for instance in &materialized.instances {
            if instance.model_id as usize == model_id {
                continue;
            }
            let child_id = instance.model_id as usize;
            let child_name = self
                .cache
                .global_model(child_id)
                .and_then(|entry| entry.name.as_deref());
            let authored = orientation_f32(instance.orientation);
            let child_local = self
                .child_transform
                .map(|hook| hook.transform(child_id, child_name, authored))
                .unwrap_or(authored);
            let child_orientation = mat3_mul(orientation, child_local);
            let child_position = instance_world_position(
                orientation,
                position,
                instance.attach_pos.unwrap_or([0.0; 3]),
                self.scale,
            );
            let child_linked = build_child_linked_slots(
                model,
                linked,
                instance,
                vars,
                ModelTreeRootLinkPolicy::Authored,
                None,
            );
            let child_vars = inherited_child_vars(vars, instance);
            self.visit(
                child_id,
                child_orientation,
                child_position,
                depth - 1,
                Some(&child_linked),
                &child_vars,
            );
        }
    }
}

struct NamedTipTraversal<'a> {
    cache: &'a ResourceCache,
    scale: f32,
    child_transform: Option<&'a dyn ModelTreeChildTransform>,
    model_name: &'a str,
    local_tip_axis: [f32; 3],
    tips: Vec<ModelTreeNamedTip>,
}

impl NamedTipTraversal<'_> {
    fn visit(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        let materialized = model.materialize_with_context(
            v2k_formats::models::ModelMaterializationContext::intrinsic(vars, linked),
        );
        if model.name.as_deref() == Some(self.model_name) {
            if let Some(local_tip) = extreme_vertex_centroid(
                ModelNodeGeometry::from_materialized(&materialized, model.flags, model.radius),
                self.local_tip_axis,
            ) {
                // ModelTransform's backend contract converts vertices by /100;
                // use the identical conversion here before applying the node's
                // fully authored/live orientation.
                let local = local_tip.map(|component| component * self.scale / 100.0);
                let delta = mat3_apply(orientation, local);
                let direction = mat3_apply(orientation, self.local_tip_axis);
                if let Some(direction_unit) = normalized(direction) {
                    self.tips.push(ModelTreeNamedTip {
                        model_id,
                        origin_world: [
                            position[0] + delta[0],
                            position[1] + delta[1],
                            position[2] + delta[2],
                        ],
                        direction_unit,
                    });
                }
            }
        }

        if depth == 0 {
            return;
        }
        for instance in &materialized.instances {
            if instance.model_id as usize == model_id {
                continue;
            }
            let child_id = instance.model_id as usize;
            let child_name = self
                .cache
                .global_model(child_id)
                .and_then(|entry| entry.name.as_deref());
            let authored = orientation_f32(instance.orientation);
            let child_local = self
                .child_transform
                .map(|hook| hook.transform(child_id, child_name, authored))
                .unwrap_or(authored);
            let child_orientation = mat3_mul(orientation, child_local);
            let child_position = instance_world_position(
                orientation,
                position,
                instance.attach_pos.unwrap_or([0.0; 3]),
                self.scale,
            );
            let child_linked = build_child_linked_slots(
                model,
                linked,
                instance,
                vars,
                ModelTreeRootLinkPolicy::Authored,
                None,
            );
            let child_vars = inherited_child_vars(vars, instance);
            self.visit(
                child_id,
                child_orientation,
                child_position,
                depth - 1,
                Some(&child_linked),
                &child_vars,
            );
        }
    }
}

fn extreme_vertex_centroid(
    model: ModelNodeGeometry<'_>,
    local_tip_axis: [f32; 3],
) -> Option<[f32; 3]> {
    let vertex_indices = model.submitted_vertex_indices();
    let maximum = vertex_indices
        .iter()
        .map(|&index| &model.vertices[index])
        .map(|vertex| {
            vertex[0] as f32 * local_tip_axis[0]
                + vertex[1] as f32 * local_tip_axis[1]
                + vertex[2] as f32 * local_tip_axis[2]
        })
        .reduce(f32::max)?;
    let mut sum = [0.0; 3];
    let mut count = 0_u32;
    for index in vertex_indices {
        let vertex = &model.vertices[index];
        let projection = vertex[0] as f32 * local_tip_axis[0]
            + vertex[1] as f32 * local_tip_axis[1]
            + vertex[2] as f32 * local_tip_axis[2];
        if (projection - maximum).abs() <= 1.0e-5 {
            for axis in 0..3 {
                sum[axis] += vertex[axis] as f32;
            }
            count += 1;
        }
    }
    (count != 0).then(|| sum.map(|component| component / count as f32))
}

fn normalized(vector: [f32; 3]) -> Option<[f32; 3]> {
    let length_squared = vector
        .iter()
        .map(|component| component * component)
        .sum::<f32>();
    if !length_squared.is_finite() || length_squared <= 1.0e-12 {
        return None;
    }
    let inverse_length = length_squared.sqrt().recip();
    Some(vector.map(|component| component * inverse_length))
}

/// Conservative world-space bounds for one materialized model hierarchy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelTreeBounds {
    pub center: [f32; 3],
    pub radius: f32,
}

/// Materialize and measure the same linked instance tree submitted by
/// [`ModelTreeRenderer::draw_linked`].
///
/// Geometry, child attachment transforms, and conservative billboard extents
/// are included. The result is primarily intended for diagnostic camera
/// framing; gameplay culling should retain its recovered runtime policy.
pub fn linked_model_bounds(
    cache: &ResourceCache,
    model_id: usize,
    scale: f32,
    depth: u8,
    vars: &AnimVars,
) -> Option<ModelTreeBounds> {
    let mut traversal = BoundsTraversal {
        cache,
        scale,
        accumulator: BoundsAccumulator::new(),
    };
    traversal.visit(
        model_id,
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        [0.0; 3],
        depth,
        None,
        vars,
    );
    traversal.accumulator.finish()
}

struct BoundsAccumulator {
    minimum: [f32; 3],
    maximum: [f32; 3],
}

impl BoundsAccumulator {
    fn new() -> Self {
        Self {
            minimum: [f32::INFINITY; 3],
            maximum: [f32::NEG_INFINITY; 3],
        }
    }

    fn include_point(&mut self, point: [f32; 3]) {
        for (axis, value) in point.into_iter().enumerate() {
            self.minimum[axis] = self.minimum[axis].min(value);
            self.maximum[axis] = self.maximum[axis].max(value);
        }
    }

    fn include_sphere(&mut self, center: [f32; 3], radius: f32) {
        self.include_point([center[0] - radius, center[1] - radius, center[2] - radius]);
        self.include_point([center[0] + radius, center[1] + radius, center[2] + radius]);
    }

    fn finish(self) -> Option<ModelTreeBounds> {
        if !self.minimum[0].is_finite() {
            return None;
        }
        let center = [
            (self.minimum[0] + self.maximum[0]) * 0.5,
            (self.minimum[1] + self.maximum[1]) * 0.5,
            (self.minimum[2] + self.maximum[2]) * 0.5,
        ];
        let half_extent = [
            (self.maximum[0] - self.minimum[0]) * 0.5,
            (self.maximum[1] - self.minimum[1]) * 0.5,
            (self.maximum[2] - self.minimum[2]) * 0.5,
        ];
        let radius = (half_extent[0] * half_extent[0]
            + half_extent[1] * half_extent[1]
            + half_extent[2] * half_extent[2])
            .sqrt()
            .max(0.1);
        Some(ModelTreeBounds { center, radius })
    }
}

struct BoundsTraversal<'a> {
    cache: &'a ResourceCache,
    scale: f32,
    accumulator: BoundsAccumulator,
}

impl BoundsTraversal<'_> {
    fn visit(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        let materialized = model.materialize_with_context(
            v2k_formats::models::ModelMaterializationContext::intrinsic(vars, linked),
        );
        let raw_scale = self.scale / 100.0;
        let geometry =
            ModelNodeGeometry::from_materialized(&materialized, model.flags, model.radius);
        for index in geometry.submitted_vertex_indices() {
            let vertex = &geometry.vertices[index];
            let local = [
                vertex[0] as f32 * raw_scale,
                vertex[1] as f32 * raw_scale,
                vertex[2] as f32 * raw_scale,
            ];
            let delta = mat3_apply(orientation, local);
            self.accumulator.include_point([
                position[0] + delta[0],
                position[1] + delta[1],
                position[2] + delta[2],
            ]);
        }
        for billboard in &materialized.billboards {
            let Some(anchor) = materialized.vertices.get(billboard.vertex as usize) else {
                continue;
            };
            let local = [
                anchor[0] as f32 * raw_scale,
                anchor[1] as f32 * raw_scale,
                anchor[2] as f32 * raw_scale,
            ];
            let delta = mat3_apply(orientation, local);
            let center = [
                position[0] + delta[0],
                position[1] + delta[1],
                position[2] + delta[2],
            ];
            // Flat billboards use this value on both axes; sqrt(2) includes
            // the diagonal. Textured billboards never exceed that envelope.
            let radius = billboard.size as f32 * 2.0 * raw_scale.abs() * std::f32::consts::SQRT_2;
            self.accumulator.include_sphere(center, radius);
        }
        if depth == 0 {
            return;
        }
        for instance in &materialized.instances {
            if instance.model_id as usize == model_id {
                continue;
            }
            let child_id = instance.model_id as usize;
            let child_orientation = mat3_mul(orientation, orientation_f32(instance.orientation));
            let child_position = instance_world_position(
                orientation,
                position,
                instance.attach_pos.unwrap_or([0.0; 3]),
                self.scale,
            );
            let child_linked = build_child_linked_slots(
                model,
                linked,
                instance,
                vars,
                ModelTreeRootLinkPolicy::Authored,
                None,
            );
            let child_vars = inherited_child_vars(vars, instance);
            self.visit(
                child_id,
                child_orientation,
                child_position,
                depth - 1,
                Some(&child_linked),
                &child_vars,
            );
        }
    }
}

fn mat3_apply(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

pub(crate) fn instance_world_position(
    parent_orientation: [[f32; 3]; 3],
    parent_position: [f32; 3],
    attach_raw: [f64; 3],
    scale: f32,
) -> [f32; 3] {
    // ModelTransform's established backend contract converts raw vertices by
    // /100, so instance attachment offsets use the same scale here.
    let local = [
        attach_raw[0] as f32 / 100.0 * scale,
        attach_raw[1] as f32 / 100.0 * scale,
        attach_raw[2] as f32 / 100.0 * scale,
    ];
    let delta = mat3_apply(parent_orientation, local);
    [
        parent_position[0] + delta[0],
        parent_position[1] + delta[1],
        parent_position[2] + delta[2],
    ]
}

fn child_local_from_parent(
    parent_position: [f64; 3],
    child_origin: [f64; 3],
    child_basis: [[f64; 3]; 3],
) -> [f64; 3] {
    let vector = [
        parent_position[0] - child_origin[0],
        parent_position[1] - child_origin[1],
        parent_position[2] - child_origin[2],
    ];
    [
        child_basis[0][0] * vector[0]
            + child_basis[1][0] * vector[1]
            + child_basis[2][0] * vector[2],
        child_basis[0][1] * vector[0]
            + child_basis[1][1] * vector[1]
            + child_basis[2][1] * vector[2],
        child_basis[0][2] * vector[0]
            + child_basis[1][2] * vector[1]
            + child_basis[2][2] * vector[2],
    ]
}

/// Build the animation state with which retail enters an op-0x0E child.
///
/// `FUN_00467410` copies the callback/resource portion of the parent context,
/// then explicitly seeds child `ctx+0x94..+0x9A` from parent registers 0..3.
/// No later parent registers are inherited. The instance snapshot is the
/// register state at the exact command-stream emission point, whereas
/// `parent_vars` supplies the shared per-entity callback values.
pub(crate) fn inherited_child_vars(parent_vars: &AnimVars, instance: &ModelInstance) -> AnimVars {
    let mut child_vars = parent_vars.clone();
    child_vars.registers.fill(0);
    child_vars.registers[..4].copy_from_slice(&instance.registers[..4]);
    child_vars
}

pub(crate) fn build_child_linked_slots(
    parent_model: &ModelEntry,
    parent_linked: Option<&LinkedModelSlots>,
    instance: &ModelInstance,
    vars: &AnimVars,
    policy: ModelTreeRootLinkPolicy,
    vertex_resolver: Option<&dyn ModelVertexResolver>,
) -> LinkedModelSlots {
    let child_origin = instance.attach_pos.unwrap_or([0.0; 3]);
    let child_basis = instance.orientation;
    let mut instance_vars = vars.clone();
    instance_vars.registers = instance.registers;
    instance
        .linked_slots
        .iter()
        .map(|&slot| {
            let even = parent_model
                .resolve_slot_with_context(
                    slot,
                    ModelMaterializationContext {
                        vars: &instance_vars,
                        linked: parent_linked,
                        vertex_resolver,
                        view_selection: ModelViewSelection::IntrinsicAllBranches,
                    },
                )
                .map(|resolved| ResolvedModelSlot {
                    world_point: resolved.world_point,
                    position_raw: child_local_from_parent(
                        policy.point(slot, resolved.position_raw),
                        child_origin,
                        child_basis,
                    ),
                    ..resolved
                });
            let odd = parent_model
                .resolve_slot_with_context(
                    slot ^ 1,
                    ModelMaterializationContext {
                        vars: &instance_vars,
                        linked: parent_linked,
                        vertex_resolver,
                        view_selection: ModelViewSelection::IntrinsicAllBranches,
                    },
                )
                .map(|resolved| ResolvedModelSlot {
                    world_point: resolved.world_point,
                    position_raw: child_local_from_parent(
                        policy.point(slot ^ 1, resolved.position_raw),
                        child_origin,
                        child_basis,
                    ),
                    ..resolved
                });
            [even, odd]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    mod emitter_tests;
    mod external_surface_tests;
    mod native_geometry_tests;
    mod surface_tests;

    use crate::level::LevelState;
    use v2k_formats::models::{ModelCollection, StreamStats};
    use v2k_formats::system::FogGradientEntry;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
    use v2k_render::{Camera, TerrainFrames, TerrainLightWindow, WorldSpriteBlend};

    #[derive(Debug)]
    struct RecordedBody {
        vertices: Vec<[f64; 3]>,
        vertex_type_flags: Vec<i16>,
        vertex_projection: Vec<ModelVertexProjection>,
        vertex_clip: Vec<v2k_formats::models::ModelSlotClip>,
        vertex_surface_origin: Vec<v2k_formats::models::ModelSurfaceOrigin>,
        triangles: Vec<[u16; 3]>,
        edges: Vec<v2k_formats::models::ModelEdge>,
        face_vertices: Vec<ModelFaceVertices>,
        normals: Vec<[f32; 3]>,
        face_uvs: Vec<[[f32; 2]; 3]>,
        face_corner_normals: Vec<[[f32; 3]; 3]>,
        face_shading: Vec<ModelFaceShading>,
        vertex_view_raw: Vec<Option<[i32; 3]>>,
        face_normals_raw: Vec<[[i32; 3]; 4]>,
        native_light_raw: Option<[i32; 3]>,
        shade_table: Vec<FogGradientEntry>,
        light_direction_raw: [i32; 3],
        shade_shift: i32,
        materials: Vec<FaceMaterial>,
        transform: ModelTransform,
        depth_fade: ModelDepthFade,
        near_clip: ModelNearClip,
        depth_policy: ModelDepthPolicy,
        view_pin: ViewPinMode,
        surface_resolution: ModelSurfaceResolution,
        has_world_surface: bool,
        external_frame: ExternalFrameMode,
        overlay: ModelOverlayKind,
    }

    #[derive(Debug)]
    struct RecordedBillboards {
        vertices: Vec<[f64; 3]>,
        vertex_projection: Vec<ModelVertexProjection>,
        vertex_clip: Vec<v2k_formats::models::ModelSlotClip>,
        billboards: Vec<Billboard>,
        materials: Vec<BillboardMaterial>,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        scale: f32,
        radius_raw: u16,
        depth_fade: ModelDepthFade,
        near_clip: ModelNearClip,
        depth_policy: ModelDepthPolicy,
    }

    #[derive(Default)]
    struct RecordingRenderer {
        order: Vec<&'static str>,
        bodies: Vec<RecordedBody>,
        billboards: Vec<RecordedBillboards>,
        fog_planes: Option<[f32; 2]>,
        model_fog: Option<v2k_render::WorldModelFog>,
    }

    impl Renderer for RecordingRenderer {
        fn backend_name(&self) -> &str {
            "recording"
        }

        fn clear(&mut self, _r: f32, _g: f32, _b: f32) {}

        fn present(&mut self) {}

        fn resize(&mut self, _width: u32, _height: u32) {}

        fn set_camera(&mut self, _camera: &Camera) {}

        fn set_fog(&mut self, enabled: bool, near: f32, far: f32, _color: [f32; 3]) {
            self.model_fog = None;
            self.fog_planes = enabled.then_some([near, far]);
        }

        fn set_world_model_fog(&mut self, fog: Option<v2k_render::WorldModelFog>) {
            self.model_fog = fog;
            self.fog_planes = fog.map(|fog| {
                [
                    fog.planes.near_raw as f32 / 256.0,
                    fog.planes.far_raw as f32 / 256.0,
                ]
            });
        }
        fn world_model_fog(&self) -> Option<v2k_render::WorldModelFog> {
            self.model_fog
        }

        fn world_fog_planes(&self) -> Option<[f32; 2]> {
            self.fog_planes
        }

        fn draw_terrain(
            &mut self,
            _terrain: &TerrainGrid,
            _colors: &[PaletteEntry],
            _frames: Option<&TerrainFrames>,
            _lights: Option<&TerrainLightWindow>,
            _elapsed_micros: u32,
        ) {
        }

        fn draw_model_body(&mut self, draw: ModelDraw<'_>) {
            if !draw.mesh.triangles.is_empty() {
                self.order.push("face");
            } else if !draw.mesh.edges.is_empty() {
                self.order.push("edge");
            }
            self.bodies.push(RecordedBody {
                vertices: draw.mesh.vertices.to_vec(),
                vertex_type_flags: draw.mesh.vertex_type_flags.to_vec(),
                vertex_projection: draw.mesh.vertex_projection.to_vec(),
                vertex_clip: draw.mesh.vertex_clip.to_vec(),
                vertex_surface_origin: draw.mesh.vertex_surface_origin.to_vec(),
                triangles: draw.mesh.triangles.to_vec(),
                edges: draw.mesh.edges.to_vec(),
                face_vertices: draw.mesh.face_vertices.to_vec(),
                normals: draw.mesh.normals.to_vec(),
                face_uvs: draw.mesh.face_uvs.to_vec(),
                face_corner_normals: draw.mesh.face_corner_normals.to_vec(),
                face_shading: draw.mesh.face_shading.to_vec(),
                vertex_view_raw: draw.mesh.vertex_view_raw.to_vec(),
                face_normals_raw: draw.mesh.face_normals_raw.to_vec(),
                native_light_raw: draw.mesh.native_light_raw,
                shade_table: draw.mesh.shade_table.unwrap_or_default().to_vec(),
                light_direction_raw: draw.mesh.light_direction_raw,
                shade_shift: draw.mesh.shade_shift,
                materials: draw.mesh.materials.to_vec(),
                transform: draw.transform,
                depth_fade: draw.depth_fade,
                near_clip: draw.near_clip,
                depth_policy: draw.depth_policy,
                view_pin: draw.view_pin,
                surface_resolution: draw.surface_resolution,
                has_world_surface: draw.world_surface.is_some(),
                external_frame: draw.external_frame,
                overlay: draw.overlay,
            });
        }

        fn draw_model_billboards(&mut self, draw: ModelBillboardDraw<'_>) {
            if draw.billboards.is_empty() {
                return;
            }
            self.order.push("billboard");
            self.billboards.push(RecordedBillboards {
                vertices: draw.vertices.to_vec(),
                vertex_projection: draw.vertex_projection.to_vec(),
                vertex_clip: draw.vertex_clip.to_vec(),
                billboards: draw.billboards.to_vec(),
                materials: draw.materials.to_vec(),
                orientation: draw.transform.orientation,
                position: draw.transform.position,
                scale: draw.transform.scale,
                radius_raw: draw.radius_raw,
                depth_fade: draw.depth_fade,
                near_clip: draw.near_clip,
                depth_policy: draw.depth_policy,
            });
        }

        fn draw_sprite(&mut self, _rgba: &[u8], _width: u32, _height: u32, _x: i32, _y: i32) {}

        fn draw_material_sprite(
            &mut self,
            _rgba: &[u8],
            _width: u32,
            _height: u32,
            _x: i32,
            _y: i32,
            _blend: WorldSpriteBlend,
        ) {
        }

        fn draw_fullscreen(&mut self, _rgba: &[u8], _width: u32, _height: u32) {}

        fn draw_color_overlay(&mut self, _r: f32, _g: f32, _b: f32, _a: f32) {}

        fn viewport_size(&self) -> (u32, u32) {
            (640, 480)
        }
    }

    #[test]
    fn model_header_no_fog_is_resolved_per_node() {
        for requested in [
            ModelDepthFade::InheritWorldFog,
            ModelDepthFade::Disabled,
            ModelDepthFade::Linear {
                near: 13.0,
                far: 21.0,
                color: [0.0, 0.5, 0.625],
            },
        ] {
            assert_eq!(model_node_depth_fade(0x40, requested), requested);
            assert_eq!(
                model_node_depth_fade(0x60, requested),
                ModelDepthFade::Disabled
            );
            // A child with ordinary flags restores the scene's own policy.
            assert_eq!(model_node_depth_fade(0, requested), requested);
        }
    }

    #[test]
    fn model_far_rejection_stops_the_hierarchy_and_exempt_children_test_their_own_headers() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let view = ModelTreeView {
            position: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
        };
        for (root_flags, child_flags, child_z, expected) in [
            (0, 0x20, 1024, 0),  // Near child cannot bypass a rejected parent.
            (0x20, 0, -1024, 1), // Parent exemption does not leak to a far child.
            (0x20, 0x20, -1024, 2),
            (0x20, 0, 1024, 2),
        ] {
            let mut root = model_entry(
                0,
                "root",
                vec![
                    [0, 0, 0, 0],
                    [0, 100, 0, 0],
                    [0, 0, 100, 0],
                    [0, 0, 0, child_z],
                ],
                vec![0x03, 0, 0, 0, 2, 4, 0x0E, 0, 1, 8, 6, 0],
            );
            root.flags = root_flags;
            root.radius = 256;
            let mut child = model_entry(
                1,
                "child",
                vec![[0, 0, 0, 0], [0, 100, 0, 0], [0, 0, 100, 0]],
                vec![0x03, 0, 0, 0, 2, 4, 0],
            );
            child.flags = child_flags;
            let cache = synthetic_cache(vec![root, child]);
            let colors = ModelMaterialCache::new();
            for traversal in 0..3 {
                let mut renderer = RecordingRenderer::default();
                renderer.set_fog(true, 13.0, 21.0, [0.0; 3]);
                let mut tree = ModelTreeRenderer::new_world(
                    &mut renderer,
                    &cache,
                    &colors,
                    100.0 / 256.0,
                    None,
                    0,
                )
                .with_view(view);
                if traversal == 2 {
                    tree = tree.with_painter_view(view, ModelTreePainterComposition::Isolated);
                }
                if traversal == 0 {
                    tree.draw_static(0, identity, [0.0, 0.0, -22.0], 2, None);
                } else {
                    tree.draw_linked(
                        0,
                        identity,
                        [0.0, 0.0, -22.0],
                        2,
                        None,
                        &AnimVars::default(),
                    );
                }
                assert_eq!(
                    renderer.bodies.len(),
                    expected,
                    "mode{traversal}, flags{root_flags:02x}/{child_flags:02x}, child{child_z}"
                );
            }
        }
    }

    #[test]
    fn model_far_rejection_precedes_live_sub_h_cache_writeback() {
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
        let mut model = model_entry(
            0,
            "limb",
            vec![
                [14, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 100, 0],
                [0, 100, 100, 0],
            ],
            vec![0x03, 0, 0, 0, 2, 4, 0],
        );
        model.radius = 256;
        let cache = synthetic_cache(vec![model]);
        let colors = ModelMaterialCache::new();
        let descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [2, 4, 6],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        };
        for (depth, expected_flags) in [(22.0, 0), (22.0 - 1.0 / 256.0, 3)] {
            let mut runtime = SubHRuntimeState::new(1).unwrap();
            runtime.set_enabled(false);
            let mut renderer = RecordingRenderer::default();
            renderer.set_fog(true, 13.0, 21.0, [0.0; 3]);
            let position = [0.0, 0.0, -depth];
            ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
                .with_view(ModelTreeView {
                    position: [0.0; 3],
                    forward: [0.0, 0.0, -1.0],
                })
                .with_sub_h_presentation(SubHPresentation {
                    runtime: &mut runtime,
                    retail_tick: 0,
                    descriptor: &descriptor,
                    model_records: &cache.global_model(0).unwrap().records,
                    origin_raw: [0, 0, (-depth * 256.0) as i16],
                    origin_world: position,
                    body_axes_q31: None,
                    native_context: None,
                    fallback: None,
                    emitter: None,
                })
                .draw_linked(
                    0,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    position,
                    2,
                    None,
                    &AnimVars::default(),
                );
            assert_eq!(runtime.records()[0].flags_raw, expected_flags);
        }
    }

    #[test]
    fn billboard_submission_keeps_each_nodes_radius_and_no_fog_policy() {
        let mut root = model_entry(
            0,
            "unfogged_root",
            vec![[1, 0, 2, 4], [0, 0, 0, -200], [0, 0, 0, 200]],
            vec![0x68, 0, 1, 3, 0, 0x0e, 0, 1, 8, 0, 0],
        );
        root.flags = 0x60;
        root.radius = 0xffff;
        let mut child = model_entry(
            1,
            "fogged_child",
            vec![[0, 0, 0, 0]],
            vec![0x68, 0, 1, 3, 0, 0],
        );
        child.radius = 234;
        let root_projection = root.vertex_projection.clone();
        let child_projection = child.vertex_projection.clone();
        assert!(root_projection
            .iter()
            .any(|projection| matches!(projection, ModelVertexProjection::ScreenMidpoint(_))));
        let cache = synthetic_cache(vec![root, child]);
        let materials = ModelMaterialCache::new();
        let depth_fade = ModelDepthFade::Linear {
            near: 13.0,
            far: 21.0,
            color: [0.5; 3],
        };
        let orientation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for static_draw in [false, true] {
            let mut renderer = RecordingRenderer::default();
            let mut tree = ModelTreeRenderer::new(
                &mut renderer,
                &cache,
                &materials,
                0.5,
                None,
                ViewPinMode::Raw,
            )
            .with_depth_fade(depth_fade)
            .with_near_clip(ModelNearClip::RetailFrontend);
            if static_draw {
                tree.draw_static(0, orientation, [0.0; 3], 2, None);
            } else {
                tree.draw_linked(0, orientation, [0.0; 3], 2, None, &AnimVars::default());
            }
            assert_eq!(renderer.billboards.len(), 2);
            assert_eq!(renderer.billboards[0].radius_raw, 0xffff);
            assert_eq!(renderer.billboards[0].depth_fade, ModelDepthFade::Disabled);
            assert_eq!(
                renderer.billboards[0].near_clip,
                ModelNearClip::RetailFrontend
            );
            assert_eq!(renderer.billboards[0].vertex_projection, root_projection);
            assert!(renderer.billboards[0]
                .vertex_clip
                .iter()
                .all(|clip| *clip == v2k_formats::models::ModelSlotClip::Clear));
            assert_eq!(renderer.billboards[1].radius_raw, 234);
            assert_eq!(renderer.billboards[1].depth_fade, depth_fade);
            assert_eq!(
                renderer.billboards[1].near_clip,
                ModelNearClip::RetailFrontend
            );
            assert_eq!(renderer.billboards[1].vertex_projection, child_projection);
            assert!(renderer.bodies.iter().all(|body| {
                body.near_clip == ModelNearClip::RetailFrontend && body.transform.scale == 0.5
            }));
        }
    }

    #[test]
    fn original_face_provenance_survives_static_live_and_painter_submissions() {
        let mut model = model_entry(
            0,
            "near-quads",
            vec![
                [0, 0, 0, 0],
                [0, 100, 0, 0],
                [0, 100, 100, 0],
                [0, 0, 100, 300],
                [0, 200, 100, 0],
                [1, 0, 12, 14],
                [0, 200, 0, 0],
                [0, 200, 0, 1000],
            ],
            vec![0x04, 1, 0, 0, 2, 4, 6, 0x04, 2, 0, 0, 4, 8, 10, 0],
        );
        model.flags |= 0x20;
        let triangles = model.triangles.clone();
        let original_faces = model.face_vertices.clone();
        let vertices = model.vertices.clone();
        let vertex_projection = model.vertex_projection.clone();
        let midpoint_sources = vertex_projection
            .iter()
            .find_map(|projection| match projection {
                ModelVertexProjection::ScreenMidpoint(sources) => Some(*sources),
                ModelVertexProjection::Position | ModelVertexProjection::WorldPoint(_) => None,
            })
            .expect("the second quad retains its projected midpoint");
        assert!(midpoint_sources
            .iter()
            .all(|source| !triangles.iter().flatten().any(|index| index == source)));
        assert_eq!(original_faces.len(), 4);
        assert!(original_faces
            .iter()
            .all(|face| matches!(face, ModelFaceVertices::Quad(_))));
        assert_eq!(original_faces[0], original_faces[1]);
        assert_eq!(original_faces[2], original_faces[3]);
        assert_ne!(original_faces[0], original_faces[2]);
        let cache = synthetic_cache(vec![model]);
        let colors = ModelMaterialCache::new();
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for path in ["static", "linked", "painter"] {
            for scale in [1.0, 100.0 / 256.0, 3.25] {
                for near_clip in [
                    ModelNearClip::Camera,
                    ModelNearClip::RetailWorld,
                    ModelNearClip::RetailFrontend,
                ] {
                    let mut renderer = RecordingRenderer::default();
                    let mut tree = if near_clip == ModelNearClip::RetailWorld {
                        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, scale, None, 0)
                    } else {
                        let tree = ModelTreeRenderer::new(
                            &mut renderer,
                            &cache,
                            &colors,
                            scale,
                            None,
                            ViewPinMode::Raw,
                        );
                        if near_clip == ModelNearClip::RetailFrontend {
                            tree.with_near_clip(near_clip)
                        } else {
                            tree
                        }
                    }
                    .with_depth_fade(ModelDepthFade::Linear {
                        near: 13.0,
                        far: 21.0,
                        color: [0.5; 3],
                    });
                    match path {
                        "static" => tree.draw_static(0, identity, [0.0, 0.0, -20.0], 0, None),
                        "linked" => tree.draw_linked(
                            0,
                            identity,
                            [0.0, 0.0, -20.0],
                            0,
                            None,
                            &AnimVars::default(),
                        ),
                        "painter" => tree
                            .with_painter_view(
                                ModelTreeView {
                                    position: [0.0; 3],
                                    forward: [0.0, 0.0, -1.0],
                                },
                                ModelTreePainterComposition::Isolated,
                            )
                            .draw_linked(
                                0,
                                identity,
                                [0.0, 0.0, -20.0],
                                0,
                                None,
                                &AnimVars::default(),
                            ),
                        _ => unreachable!(),
                    }
                    assert_eq!(renderer.bodies.len(), if path == "painter" { 2 } else { 1 });
                    for body in &renderer.bodies {
                        assert_eq!(body.near_clip, near_clip);
                        assert_eq!(body.depth_fade, ModelDepthFade::Disabled);
                        assert_eq!(body.transform.scale, scale);
                        assert_eq!(body.vertices, vertices, "{path}");
                        assert_eq!(body.vertex_projection, vertex_projection, "{path}");
                        assert_eq!(body.face_vertices.len(), body.triangles.len());
                        for (triangle, original) in body.triangles.iter().zip(&body.face_vertices) {
                            let index = triangles.iter().position(|face| face == triangle).unwrap();
                            assert_eq!(*original, original_faces[index], "{path}");
                        }
                    }
                }
            }
        }
    }

    fn model_entry(
        index: usize,
        name: &str,
        records: Vec<[i16; 4]>,
        cmd_words: Vec<u16>,
    ) -> ModelEntry {
        let mut entry = ModelEntry {
            index,
            cmd_word_count: cmd_words.len() as u16,
            extra_count: 0,
            flags: 0x40,
            slot_count: (records.len() * 2) as u16,
            face_val: 2,
            radius: 0,
            collision_radius_raw: 0,
            collision_program: Vec::new(),
            has_view_commands: false,
            records,
            normal_pool: Vec::new(),
            cmd_words,
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
            name: Some(name.to_owned()),
        };
        let materialized = entry.materialize(&AnimVars::default());
        entry.has_view_commands = materialized.has_view_commands;
        entry.vertices = materialized.vertices;
        entry.vertex_type_flags = materialized.vertex_type_flags;
        entry.vertex_projection = materialized.vertex_projection;
        entry.vertex_clip = materialized.vertex_clip;
        entry.vertex_surface_origin = materialized.vertex_surface_origin;
        entry.triangles = materialized.triangles;
        entry.face_vertices = materialized.face_vertices;
        entry.normals = materialized.normals;
        entry.face_cull = materialized.face_cull;
        entry.face_materials = materialized.face_materials;
        entry.face_uvs = materialized.face_uvs;
        entry.face_corner_normals = materialized.face_corner_normals;
        entry.face_shading = materialized.face_shading;
        entry.shadow_triangles = materialized.shadow_triangles;
        entry.edges = materialized.edges;
        entry.billboards = materialized.billboards;
        entry.instances = materialized.instances;
        entry
    }

    fn synthetic_cache(models: Vec<ModelEntry>) -> ResourceCache {
        synthetic_cache_with_terrain(models, None)
    }

    fn synthetic_cache_with_terrain(
        models: Vec<ModelEntry>,
        terrain: Option<TerrainGrid>,
    ) -> ResourceCache {
        synthetic_cache_with_terrain_and_level(models, terrain, None)
    }

    fn synthetic_cache_with_terrain_and_level(
        models: Vec<ModelEntry>,
        terrain: Option<TerrainGrid>,
        level: Option<v2k_formats::levels::LevelDescriptor>,
    ) -> ResourceCache {
        ResourceCache::new(vec![LevelState {
            source_path: "synthetic-model-tree".to_owned(),
            system_level: Some(2),
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: Some(
                (0..8)
                    .map(|value| FogGradientEntry {
                        r: value * 18,
                        g: value * 18,
                        b: value * 18,
                        shade_level: value + 7,
                    })
                    .collect(),
            ),
            color_palettes: Some(
                [0x0000, 0x110c, 0x1993, 0x296c, 0x66d4]
                    .into_iter()
                    .map(|rgb555| {
                        let [r, g, b, _] = v2k_formats::palette::decode_rgb555(rgb555);
                        PaletteEntry { rgb555, r, g, b }
                    })
                    .collect(),
            ),
            models: Some(ModelCollection {
                sub_blocks: Vec::new(),
                all_entries: models,
                stats: StreamStats::default(),
            }),
            anim_frames: None,
            terrain,
            anim_sound: None,
            collision: None,
            level,
            linkage: None,
        }])
    }

    fn hierarchy_cache() -> ResourceCache {
        let root = model_entry(
            0,
            "root",
            vec![
                [0, 0, 0, 0],
                [0, 100, 0, 0],
                [0, 0, 100, 0],
                [0, 100, 100, 0],
                [13, 0, 0, 0],
                [13, 2, 0, 0],
                [13, 4, 0, 0],
            ],
            vec![
                0x04, 2, 0, 0, 2, 4, 6, // textured-layout quad, palette material 2
                0x03, 3, 0, 8, 10, 12, // all-type-13 footprint
                0x68, 0, 1, 3, 0, // flat billboard at slot 0
                0x0E, 0, 1, 8, 2, // child 1 attached/remapped from slot 2
                0x00,
            ],
        );
        let child = model_entry(
            1,
            "child",
            vec![
                [11, 0, 0, 0], // import root's remapped slot through linked traversal
                [0, 100, 0, 0],
                [0, 0, 100, 0],
            ],
            vec![
                0x03, 4, 0, 0, 2, 4, // body needs the linked type-11 vertex
                0x0E, 0, 2, 8, 2, // nested child 2 attached at local slot 2
                0x00,
            ],
        );
        let grandchild = model_entry(
            2,
            "grandchild",
            vec![[0, 0, 0, 0], [0, 100, 0, 0], [0, 0, 100, 0]],
            vec![0x03, 4, 0, 0, 2, 4, 0x00],
        );
        synthetic_cache(vec![root, child, grandchild])
    }

    fn inherited_register_cache() -> ResourceCache {
        // Match PLAYER4's relevant structure: register 3 identifies the
        // authored side immediately before each op-0x0E child emission.
        // Register 8 is deliberately populated in the parent to prove that
        // retail's inheritance boundary stops after register 3.
        let root = model_entry(
            0,
            "register_root",
            vec![[0, 100, 0, 0], [0, -100, 0, 0]],
            vec![
                0x0D, 8, 1, 0, // parent r8 = 1 (must not reach either child)
                0x0D, 3, 1, 0, // first child inherits r3 = 1
                0x0E, 0, 1, 8, 0, // child at +X
                0x0D, 3, 0, 0, // second child inherits r3 = 0
                0x0E, 0, 1, 8, 2, // child at -X
                0x00,
            ],
        );
        let side_selector = model_entry(
            1,
            "side_selector",
            vec![[0, 0, 0, 0]],
            vec![
                0x2B, 8, 0xC3, 1, // r3 != 1: skip positive leaf
                0x0E, 0, 2, 8, 0, // positive leaf
                0x2B, 8, 0xC3, 0, // r3 != 0: skip negative leaf
                0x0E, 0, 3, 8, 0, // negative leaf
                0x2B, 8, 0xC8, 1, // r8 != 1: skip non-inherited leaf
                0x0E, 0, 4, 8, 0, // would appear if registers 4+ leaked
                0x00,
            ],
        );
        let positive = model_entry(
            2,
            "positive_leaf",
            vec![[0, 0, 0, 0], [0, 100, 0, 0], [0, 0, 100, 0]],
            vec![0x03, 4, 0, 0, 2, 4, 0x00],
        );
        let negative = model_entry(
            3,
            "negative_leaf",
            vec![[0, 0, 0, 0], [0, -100, 0, 0], [0, 0, 100, 0]],
            vec![0x03, 4, 0, 0, 2, 4, 0x00],
        );
        let non_inherited = model_entry(
            4,
            "non_inherited_leaf",
            vec![[0, 0, 0, 0], [0, 1000, 0, 0], [0, 0, 100, 0]],
            vec![0x03, 4, 0, 0, 2, 4, 0x00],
        );
        synthetic_cache(vec![root, side_selector, positive, negative, non_inherited])
    }

    struct NestedTransformHook {
        calls: RefCell<Vec<usize>>,
    }

    const FLIP_X: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    const ROTATE_X_QUARTER: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]];

    impl ModelTreeChildTransform for NestedTransformHook {
        fn transform(
            &self,
            child_id: usize,
            _child_name: Option<&str>,
            authored_local: [[f32; 3]; 3],
        ) -> [[f32; 3]; 3] {
            self.calls.borrow_mut().push(child_id);
            match child_id {
                1 => mat3_mul(authored_local, FLIP_X),
                2 => mat3_mul(authored_local, ROTATE_X_QUARTER),
                _ => authored_local,
            }
        }
    }

    #[test]
    fn diagnostic_bounds_center_and_enclose_all_axes() {
        let mut bounds = BoundsAccumulator::new();
        bounds.include_point([-2.0, -1.0, 0.0]);
        bounds.include_point([4.0, 3.0, 6.0]);
        let bounds = bounds.finish().expect("two points produce bounds");
        assert_eq!(bounds.center, [1.0, 1.0, 3.0]);
        assert!((bounds.radius - (22.0f32).sqrt()).abs() < 0.0001);
    }

    #[test]
    fn hidden_midpoint_sources_do_not_move_authored_tips_or_diagnostic_bounds() {
        let model = model_entry(
            0,
            "midpoint-tip",
            vec![
                [0, -100, 0, 0],
                [0, 100, 0, 0],
                [1, 0, 6, 8],
                [0, 0, 0, 0],
                [0, 0, 0, 1000],
            ],
            vec![0x03, 1, 0, 0, 2, 4, 0],
        );
        assert!(model.vertices.contains(&[0.0, 0.0, 1000.0]));
        let cache = synthetic_cache(vec![model]);
        let vars = AnimVars::default();
        let tips = linked_model_named_tips(
            &cache,
            ModelTreeNamedTipRequest {
                model_id: 0,
                orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                position: [0.0; 3],
                scale: 1.0,
                depth: 0,
                linked: None,
                vars: &vars,
                child_transform: None,
                model_name: "midpoint-tip",
                local_tip_axis: [0.0, 0.0, 1.0],
            },
        );
        assert_eq!(tips.len(), 1);
        assert_eq!(tips[0].origin_world, [0.0, 0.0, 5.0]);
        let bounds = linked_model_bounds(&cache, 0, 1.0, 0, &vars).unwrap();
        assert_eq!(bounds.center, [0.0, 0.0, 2.5]);
        assert!((bounds.radius - 7.25_f32.sqrt()).abs() < 0.0001);
    }

    #[test]
    fn hidden_midpoint_sources_do_not_turn_flat_actors_into_volume_models() {
        let model = model_entry(
            0,
            "flat-midpoint-actor",
            vec![
                [0, -100, 0, 0],
                [0, 100, 0, 0],
                [0, 100, 200, 0],
                [1, 0, 8, 10],
                [0, -100, 200, -1000],
                [0, -100, 200, 1000],
                [13, 0, 0, 0],
                [13, 2, 0, 0],
                [13, 4, 0, 0],
            ],
            vec![0x84, 1, 0, 0, 2, 4, 6, 0x03, 1, 0, 12, 14, 16, 0],
        );
        assert!(model.vertices.iter().any(|vertex| vertex[2] != 0.0));
        assert!(model_is_camera_facing_actor(&model));
    }

    #[test]
    fn geometry_measurement_keeps_edge_billboard_and_point_cloud_vertices() {
        let records = vec![[0, -100, 0, 0], [0, 100, 0, 0], [0, 0, 100, 0]];
        for words in [vec![0x02, 1, 0, 2, 0x68, 4, 1, 3, 0, 0], vec![0]] {
            let model = model_entry(0, "measurement", records.clone(), words);
            let geometry = ModelNodeGeometry::from_static(&model);
            assert_eq!(geometry.submitted_vertex_indices(), [0, 1, 2]);
        }
    }

    #[test]
    fn world_alias_resolution_reaches_generated_faces_and_nested_links() {
        let root = model_entry(
            0,
            "generated-support",
            vec![
                [0, 0, 0, 0],
                [0, 0, 128, -128],
                [0, 256, 256, 0],
                [12, 4, 0, 0],
                [6, 6, 2, 0],
                [13, 8, 0, 0],
            ],
            vec![0x04, 1, 0, 2, 8, 6, 0, 0x0e, 0, 1, 12, 8, 6, 0, 0],
        );
        let child = model_entry(
            1,
            "linked-support",
            vec![[11, 1, 0, 0], [0, 0, 128, 0], [0, 128, 0, 0]],
            vec![0x03, 1, 0, 0, 2, 4, 0x0e, 0, 2, 10, 0, 0, 0],
        );
        let grandchild = model_entry(
            2,
            "nested-support",
            vec![[11, 0, 0, 0], [0, 0, 128, 0], [0, 128, 0, 0]],
            vec![0x03, 1, 0, 0, 2, 4, 0],
        );
        let terrain = TerrainGrid {
            header: [-0x1800 << 8, -73, 73, -73, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let cache = synthetic_cache_with_terrain(vec![root, child, grandchild], Some(terrain));
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
            .draw_linked(
                0,
                identity,
                [64.0, 0.0, 64.0],
                2,
                None,
                &AnimVars::default(),
            );
        assert_eq!(renderer.bodies.len(), 3);
        assert_eq!(renderer.bodies[0].vertices[1], [256.0, 128.0, -128.0]);
        assert_eq!(renderer.bodies[0].vertex_type_flags, vec![0, 6, 12, 0]);
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.surface_resolution == ModelSurfaceResolution::ContextResolved));
        assert_eq!(renderer.bodies[1].transform.position, [65.0, 0.5, 63.5]);
        assert_eq!(renderer.bodies[1].vertices[0], [0.0, -128.0, 128.0]);
        assert_eq!(renderer.bodies[2].transform.position, [65.0, 0.0, 64.0]);
        assert_eq!(renderer.bodies[2].vertices[0], [0.0, 0.0, 0.0]);

        let mut cached_entry = RecordingRenderer::default();
        ModelTreeRenderer::new_world(&mut cached_entry, &cache, &colors, 100.0 / 256.0, None, 0)
            .draw_static(0, identity, [64.0, 0.0, 64.0], 2, None);
        assert_eq!(cached_entry.bodies.len(), 3);
        for (cached, linked) in cached_entry.bodies.iter().zip(&renderer.bodies) {
            assert_eq!(cached.vertices, linked.vertices);
            assert_eq!(cached.transform.position, linked.transform.position);
            assert_eq!(cached.surface_resolution, linked.surface_resolution);
        }

        let mut intrinsic = RecordingRenderer::default();
        ModelTreeRenderer::new(
            &mut intrinsic,
            &cache,
            &colors,
            100.0 / 256.0,
            None,
            ViewPinMode::Raw,
        )
        .draw_linked(
            0,
            identity,
            [64.0, 0.0, 64.0],
            2,
            None,
            &AnimVars::default(),
        );
        assert_eq!(intrinsic.bodies[0].vertices[1], [256.0, 384.0, -128.0]);
        assert_eq!(intrinsic.bodies[1].transform.position, [65.0, 1.5, 63.5]);
    }

    #[test]
    fn static_linked_and_painter_select_branches_before_child_traversal() {
        let records = vec![[0, 0, 0, 0], [0, 100, 0, 0], [0, 0, 100, 0], [0, 0, 0, 200]];
        let mut root = model_entry(
            0,
            "view-root",
            records.clone(),
            vec![
                0x0b, 13, 2, 0x03, 1, 0, 0, 2, 4, 0x0e, 0, 1, 8, 6, 0x0c, 13, 2, 0x03, 2, 0, 0, 2,
                4, 0x0e, 0, 2, 8, 6, 0,
            ],
        );
        root.normal_pool = vec![[0, 0, 0, 1]];
        let child = |id| {
            let mut model = model_entry(
                id,
                "view-child",
                records.clone(),
                vec![
                    0x0b, 8, 2, 0x03, 3, 0, 0, 2, 4, 0x0c, 9, 2, 0x04, 4, 0, 0, 2, 4, 6, 0,
                ],
            );
            model.normal_pool = vec![[0, 0, 0, 1]];
            model
        };
        let cache = synthetic_cache(vec![root, child(1), child(2)]);
        let colors = ModelMaterialCache::new();
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        struct ReverseChild;
        impl ModelTreeChildTransform for ReverseChild {
            fn transform(
                &self,
                _: usize,
                _: Option<&str>,
                authored: [[f32; 3]; 3],
            ) -> [[f32; 3]; 3] {
                mat3_mul(
                    authored,
                    [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
                )
            }
        }
        let reverse = ReverseChild;
        for camera_z in [-1.0, 1.0] {
            for reversed in [false, true] {
                let hook = reversed.then_some(&reverse as &dyn ModelTreeChildTransform);
                for path in ["static", "linked", "painter"] {
                    let mut renderer = RecordingRenderer::default();
                    let view = ModelTreeView {
                        position: [0.0, 0.0, camera_z],
                        forward: [0.0, 0.0, -1.0],
                    };
                    let mut tree = ModelTreeRenderer::new(
                        &mut renderer,
                        &cache,
                        &colors,
                        1.0,
                        None,
                        ViewPinMode::Raw,
                    )
                    .with_view(view);
                    match path {
                        "static" => tree.draw_static(0, identity, [0.0; 3], 1, hook),
                        "linked" => tree.draw_linked_with_transform(
                            0,
                            identity,
                            [0.0; 3],
                            1,
                            None,
                            &AnimVars::default(),
                            hook,
                        ),
                        "painter" => tree
                            .with_painter_view(view, ModelTreePainterComposition::Isolated)
                            .draw_linked_with_transform(
                                0,
                                identity,
                                [0.0; 3],
                                1,
                                None,
                                &AnimVars::default(),
                                hook,
                            ),
                        _ => unreachable!(),
                    }
                    let root_bodies: Vec<_> = renderer
                        .bodies
                        .iter()
                        .filter(|body| body.transform.position == [0.0; 3])
                        .collect();
                    let children: Vec<_> = renderer
                        .bodies
                        .iter()
                        .filter(|body| body.transform.position != [0.0; 3])
                        .collect();
                    assert_eq!(
                        root_bodies
                            .iter()
                            .map(|body| body.triangles.len())
                            .sum::<usize>(),
                        1,
                        "{path}"
                    );
                    assert_eq!(children.len(), 1, "{path}");
                    assert_eq!(children[0].transform.position, [0.0, 0.0, 2.0]);
                    // The mounted child is beyond both cameras; its yaw, not
                    // the root's selected side, determines the child branch.
                    assert_eq!(
                        children[0].triangles.len(),
                        if reversed { 1 } else { 2 },
                        "{path} camera={camera_z}"
                    );
                    assert_eq!(
                        root_bodies[0].materials[0].unlit_color()[0],
                        if camera_z < 0.0 { 48.0 } else { 32.0 } / 255.0
                    );
                }
            }
        }
        // Static asset inspection still retains both alternative children.
        let mut intrinsic = RecordingRenderer::default();
        ModelTreeRenderer::new(&mut intrinsic, &cache, &colors, 1.0, None, ViewPinMode::Raw)
            .draw_static(0, identity, [0.0; 3], 1, None);
        assert_eq!(
            intrinsic
                .bodies
                .iter()
                .map(|body| body.triangles.len())
                .collect::<Vec<_>>(),
            [2, 3, 3]
        );
    }

    #[test]
    fn world_constructor_propagates_live_surface_context() {
        let model = model_entry(
            0,
            "world-surface",
            vec![[13, 0, 0, 0], [13, 100, 0, 0], [13, 0, 0, 100]],
            vec![0x03, 1, 0, 0, 2, 4, 0x00],
        );
        let terrain = TerrainGrid {
            header: [-0x1800 << 8, -73, 73, -73, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let cache = synthetic_cache_with_terrain(vec![model], Some(terrain));
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();

        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 1.0, None, 77)
            .with_style(ModelTreeStyle {
                textures: true,
                shadows: true,
            })
            .draw_static(
                0,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [0.0; 3],
                0,
                None,
            );

        assert_eq!(
            renderer.bodies.len(),
            1,
            "the diagnostic shadow toggle must not duplicate WorldSurface faces"
        );
        assert_eq!(renderer.bodies[0].view_pin, ViewPinMode::WorldSurface);
        assert_eq!(renderer.bodies[0].overlay, ModelOverlayKind::None);
        assert!(renderer.bodies[0].has_world_surface);
    }

    #[test]
    fn with_overlay_tags_the_hierarchy_as_a_terrain_surface_draw() {
        let model = model_entry(
            0,
            "terrain-overlay",
            vec![[0, 0, 0, 0], [0, 100, 0, 0], [0, 0, 0, 100]],
            vec![0x03, 1, 0, 0, 2, 4, 0x00],
        );
        let cache = synthetic_cache_with_terrain(vec![model], None);
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        ModelTreeRenderer::new(
            &mut renderer,
            &cache,
            &colors,
            1.0,
            None,
            ViewPinMode::WorldSurface,
        )
        .with_overlay(ModelOverlayKind::TerrainSurface)
        .draw_static(
            0,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
            0,
            None,
        );
        assert_eq!(renderer.bodies.len(), 1);
        assert_eq!(renderer.bodies[0].overlay, ModelOverlayKind::TerrainSurface);
    }

    #[test]
    fn child_attachment_uses_parent_orientation_and_model_scale() {
        let quarter_turn = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert_eq!(
            instance_world_position(quarter_turn, [1.0, 2.0, 3.0], [200.0, 0.0, 0.0], 0.5),
            [1.0, 2.0, 2.0]
        );
    }

    #[test]
    fn painter_keeps_unsorted_group_instance_order_and_quad_atomicity() {
        let root = model_entry(
            0,
            "group_root",
            vec![
                [0, 0, 0, 0],
                [0, 100, 0, 0],
                [0, 100, 100, 0],
                [0, 0, 100, 0],
            ],
            vec![
                0xc6, 0, 1500, 0, 0x04, 1, 0, 0, 2, 4, 6, 0x0e, 0, 1, 8, 0, 0x03, 2, 0, 0, 2, 4,
                0xe6, 0x03, 3, 0, 0, 2, 4, 0,
            ],
        );
        let child = model_entry(
            1,
            "group_child",
            vec![[0, 0, 0, -800], [0, 100, 0, -800], [0, 0, 100, -800]],
            vec![0x03, 4, 0, 0, 2, 4, 0],
        );
        let cache = synthetic_cache(vec![root, child]);
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        ModelTreeRenderer::new(
            &mut renderer,
            &cache,
            &colors,
            1.0,
            None,
            ViewPinMode::Disabled,
        )
        .with_painter_view(
            ModelTreeView {
                position: [0.0; 3],
                forward: [0.0, 0.0, -1.0],
            },
            ModelTreePainterComposition::Isolated,
        )
        .draw_linked(
            0,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [0.0; 3],
            1,
            None,
            &AnimVars::default(),
        );
        assert_eq!(renderer.bodies.len(), 4);
        assert_eq!(
            renderer
                .bodies
                .iter()
                .map(|body| body.triangles.len())
                .collect::<Vec<_>>(),
            [2, 1, 1, 1]
        );
        assert_eq!(
            renderer
                .bodies
                .iter()
                .map(|body| (body.materials[0].unlit_color()[0] * 255.0).round() as u32)
                .collect::<Vec<_>>(),
            [32, 200, 48, 80]
        );
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_policy == ModelDepthPolicy::Painter));
    }

    #[v2k_test_support::retail_test]
    fn klaus_painter_submits_every_face_without_edges_or_billboards() {
        let dir = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(2, 1).unwrap();
        let colors = ModelMaterialCache::new();
        for morph in [0, 0x4000, 0x8000, 0xb000, 0xc000, 0xffff] {
            for state in [0, 1] {
                let mut vars = AnimVars::default();
                vars.dynamic[1] = morph;
                vars.dynamic[2] = state;
                let draw = |painter| {
                    let mut renderer = RecordingRenderer::default();
                    let mut tree = ModelTreeRenderer::new(
                        &mut renderer,
                        &session.cache,
                        &colors,
                        1.0,
                        None,
                        ViewPinMode::Disabled,
                    )
                    .with_root_link_policy(
                        ModelTreeRootLinkPolicy::KlausMatte {
                            viewport: (2560, 1080),
                        },
                    );
                    if painter {
                        tree = tree.with_painter_view(
                            ModelTreeView {
                                position: [0.0; 3],
                                forward: [0.0, 0.0, -1.0],
                            },
                            ModelTreePainterComposition::Isolated,
                        );
                    }
                    tree.draw_linked(
                        1,
                        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                        [0.0; 3],
                        8,
                        None,
                        &vars,
                    );
                    renderer
                };
                let authored = draw(false);
                let painter = draw(true);
                assert!(
                    !painter.bodies.is_empty(),
                    "supported Klaus program must not fail closed"
                );
                assert!(painter.billboards.is_empty());
                assert!(painter
                    .bodies
                    .iter()
                    .all(|body| body.depth_policy == ModelDepthPolicy::Painter));
                assert!(authored
                    .bodies
                    .iter()
                    .all(|body| body.depth_policy == ModelDepthPolicy::Geometry));
                assert_eq!(
                    painter
                        .bodies
                        .iter()
                        .map(|body| body.triangles.len())
                        .sum::<usize>(),
                    authored
                        .bodies
                        .iter()
                        .map(|body| body.triangles.len())
                        .sum::<usize>(),
                    "morph {morph:#06x} state {state}"
                );
                assert!(painter
                    .bodies
                    .iter()
                    .all(|body| (1..=2).contains(&body.triangles.len())));
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn klaus_frontend_depth_belongs_to_one_opaque_outer_group_in_every_tier() {
        let dir = v2k_test_support::retail_dir();
        // Nonvisual corpus check: the same group program is present in each
        // system tier. Slot 12 is +0xA00, slot 6 is -0x1400; callback 1
        // selects the latter only when the mouth morph is nonzero.
        for variant in 0..=3 {
            let mut session = crate::session::GameSession::init(&dir).unwrap();
            session.load_auxiliary_ovl(2, variant).unwrap();
            let colors = ModelMaterialCache::new();
            for root_depth in [0xA00, 0xF00, 0x1400] {
                let mut renderer = RecordingRenderer::default();
                let mut vars = AnimVars::default();
                vars.dynamic[2] = 1;
                ModelTreeRenderer::new(
                    &mut renderer,
                    &session.cache,
                    &colors,
                    1.0,
                    None,
                    ViewPinMode::Disabled,
                )
                .with_painter_view(
                    ModelTreeView {
                        position: [0.0; 3],
                        forward: [0.0, 0.0, -1.0],
                    },
                    ModelTreePainterComposition::OpaqueSceneGroup,
                )
                .draw_linked(
                    1,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
                    [0.0, 0.0, -root_depth as f32 / 100.0],
                    8,
                    None,
                    &vars,
                );
                assert!(
                    !renderer.bodies.is_empty(),
                    "tier {variant}, depth {root_depth}"
                );
                assert!(renderer.bodies.iter().all(|body| body.depth_policy
                    == ModelDepthPolicy::PainterGroup {
                        view_depth_raw: root_depth + 0xA00
                    }));
                assert!(renderer
                    .bodies
                    .iter()
                    .flat_map(|body| &body.materials)
                    .all(|material| material.blend == WorldSpriteBlend::Masked));
                assert!(renderer.billboards.is_empty());
            }
        }
    }

    #[test]
    fn klaus_matte_extends_only_mirrored_outer_supports() {
        let policy = ModelTreeRootLinkPolicy::KlausMatte {
            viewport: (2560, 1080),
        };
        for (slot, x) in [(2, -768.0), (3, 768.0), (4, -768.0), (5, 768.0)] {
            let point = policy.point(slot, [x, 640.0, -1536.0]);
            assert!((point[0] - x * 16.0 / 9.0).abs() < 1.0e-9);
            assert_eq!(&point[1..], &[640.0, -1536.0]);
        }
        for slot in [0, 1, 6, 7, 8, 9, 10, 11, 12, 13] {
            let point = [-768.0, -179.0, -5120.0];
            assert_eq!(policy.point(slot, point), point);
        }
    }

    #[test]
    fn klaus_matte_keeps_the_authored_horizontal_boundary_across_viewport_modes() {
        use v2k_render::renderer::RenderViewport;
        use v2k_render::ScalingMode;

        for (width, height) in [(640, 480), (1280, 720), (2560, 1080)] {
            for mode in [
                ScalingMode::Native,
                ScalingMode::FourThree,
                ScalingMode::Stretched,
            ] {
                let viewport = RenderViewport::for_output(width, height, 640, 480, mode);
                let aspect = f64::from(viewport.logical_width) / f64::from(viewport.logical_height);
                let policy = ModelTreeRootLinkPolicy::KlausMatte {
                    viewport: (viewport.logical_width, viewport.logical_height),
                };
                let outer = policy.point(2, [-768.0, 640.0, -1536.0]);
                assert!((outer[0] / aspect - -768.0 / (4.0 / 3.0)).abs() < 1.0e-9);
                assert_eq!(outer[1], 640.0);
                assert_eq!(outer[2], -1536.0);
            }
        }
        let point = [-768.0, 640.0, -1536.0];
        assert_eq!(
            ModelTreeRootLinkPolicy::KlausMatte {
                viewport: (480, 640)
            }
            .point(2, point),
            point,
            "narrow windows must not pull the matte inward"
        );
        assert_eq!(ModelTreeRootLinkPolicy::Authored.point(2, point), point);
    }

    #[v2k_test_support::retail_test]
    fn klaus_matte_root_links_leave_the_live_body_and_aperture_unchanged() {
        let dir = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(2, 1).unwrap();
        let root = session.cache.global_model(1).unwrap();
        assert_eq!(root.name.as_deref(), Some("klaus"));
        assert_eq!(
            &root.records[1..3],
            &[[0, -768, 640, -1536], [0, -768, -1280, -1536]]
        );
        let colors = ModelMaterialCache::new();
        for morph in [0, 0x4000, 0x8000, 0xb000, 0xc000, 0xffff] {
            for state in [0, 1] {
                let mut vars = AnimVars::default();
                vars.dynamic[1] = morph;
                vars.dynamic[2] = state;
                vars.dynamic[6] = 0x800;
                vars.dynamic[7] = 0x800;
                vars.dynamic[10] = 0xf800;
                let draw = |policy| {
                    let mut renderer = RecordingRenderer::default();
                    ModelTreeRenderer::new(
                        &mut renderer,
                        &session.cache,
                        &colors,
                        1.0,
                        None,
                        ViewPinMode::Disabled,
                    )
                    .with_root_link_policy(policy)
                    .draw_linked(
                        1,
                        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                        [0.0; 3],
                        8,
                        None,
                        &vars,
                    );
                    renderer
                };
                let authored = draw(ModelTreeRootLinkPolicy::Authored);
                let wide = draw(ModelTreeRootLinkPolicy::KlausMatte {
                    viewport: (2560, 1080),
                });
                assert_eq!(authored.bodies.len(), wide.bodies.len());
                let mut changed_faces = 0;
                for (before, after) in authored.bodies.iter().zip(&wide.bodies) {
                    assert_eq!(before.transform.position, after.transform.position);
                    assert_eq!(before.transform.orientation, after.transform.orientation);
                    assert_eq!(before.transform.scale, after.transform.scale);
                    assert_eq!(before.triangles, after.triangles);
                    for (face, triangle) in before.triangles.iter().enumerate() {
                        if triangle.iter().any(|&vertex| {
                            (0..3).any(|axis| {
                                (before.vertices[vertex as usize][axis]
                                    - after.vertices[vertex as usize][axis])
                                    .abs()
                                    > 1.0e-6
                            })
                        }) {
                            changed_faces += 1;
                            assert_eq!(
                                before.materials[face].unlit_color(),
                                [0.0; 3],
                                "only the black matte may move"
                            );
                        }
                    }
                }
                assert_eq!(
                    changed_faces > 0,
                    state == 0,
                    "only iris mode exports the matte"
                );
            }
        }
    }

    #[test]
    fn op_0e_children_inherit_retail_registers_in_static_and_linked_draws() {
        for static_draw in [false, true] {
            let cache = inherited_register_cache();
            let colors = ModelMaterialCache::new();
            let mut renderer = RecordingRenderer::default();
            let mut tree =
                ModelTreeRenderer::new(&mut renderer, &cache, &colors, 1.0, None, ViewPinMode::Raw);
            if static_draw {
                tree.draw_static(
                    0,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    [0.0; 3],
                    2,
                    None,
                );
            } else {
                tree.draw_linked(
                    0,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    [0.0; 3],
                    2,
                    None,
                    &AnimVars::default(),
                );
            }

            // root, +X selector, positive leaf, -X selector, negative leaf.
            // A sixth body would be the deliberately non-inherited r8 leaf.
            assert_eq!(renderer.bodies.len(), 5, "static draw: {static_draw}");
            assert_eq!(renderer.bodies[1].transform.position, [1.0, 0.0, 0.0]);
            assert_eq!(renderer.bodies[2].transform.position, [1.0, 0.0, 0.0]);
            assert!(renderer.bodies[2].vertices.contains(&[100.0, 0.0, 0.0]));
            assert_eq!(renderer.bodies[3].transform.position, [-1.0, 0.0, 0.0]);
            assert_eq!(renderer.bodies[4].transform.position, [-1.0, 0.0, 0.0]);
            assert!(renderer.bodies[4].vertices.contains(&[-100.0, 0.0, 0.0]));
        }
    }

    #[test]
    fn register_inheritance_is_shared_by_tip_and_bounds_traversals() {
        let cache = inherited_register_cache();
        let vars = AnimVars::default();
        let request = |model_name, local_tip_axis| ModelTreeNamedTipRequest {
            model_id: 0,
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 1.0,
            depth: 2,
            linked: None,
            vars: &vars,
            child_transform: None,
            model_name,
            local_tip_axis,
        };

        let positive = linked_model_named_tips(&cache, request("positive_leaf", [1.0, 0.0, 0.0]));
        assert_eq!(positive.len(), 1);
        assert_eq!(positive[0].origin_world, [2.0, 0.0, 0.0]);
        assert_eq!(positive[0].direction_unit, [1.0, 0.0, 0.0]);

        let negative = linked_model_named_tips(&cache, request("negative_leaf", [-1.0, 0.0, 0.0]));
        assert_eq!(negative.len(), 1);
        assert_eq!(negative[0].origin_world, [-2.0, 0.0, 0.0]);
        assert_eq!(negative[0].direction_unit, [-1.0, 0.0, 0.0]);

        assert!(
            linked_model_named_tips(&cache, request("non_inherited_leaf", [1.0, 0.0, 0.0]))
                .is_empty()
        );

        let bounds = linked_model_bounds(&cache, 0, 1.0, 2, &vars).expect("register tree bounds");
        assert_eq!(bounds.center, [0.0, 0.5, 0.0]);
        assert!((bounds.radius - 4.25_f32.sqrt()).abs() < 1.0e-6);
    }

    #[test]
    fn linked_submission_preserves_render_data_and_nested_transform_policy() {
        let cache = hierarchy_cache();
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let hook = NestedTransformHook {
            calls: RefCell::new(Vec::new()),
        };
        let root_position = [10.0, 20.0, 30.0];
        let depth_fade = ModelDepthFade::Linear {
            near: 12.0,
            far: 16.0,
            color: [0.0; 3],
        };
        let light_direction_raw = [-100, 50, -50];
        ModelTreeRenderer::new(
            &mut renderer,
            &cache,
            &colors,
            0.5,
            None,
            ViewPinMode::CameraFacing,
        )
        .with_external_frame(ExternalFrameMode::WorldPoint([7.0, 0.0, 9.0]))
        .with_light_direction_raw(light_direction_raw)
        .with_shade_shift(-3)
        .with_depth_fade(depth_fade)
        .with_style(ModelTreeStyle {
            textures: true,
            shadows: true,
        })
        .draw_linked_with_transform(
            0,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            root_position,
            2,
            None,
            &AnimVars::default(),
            Some(&hook),
        );

        // The workbench shadow toggle derives one diagnostic draw from the
        // root's canonical all-type-13 face. The canonical root, linked child,
        // and nested grandchild remain their ordinary hierarchy submissions.
        assert_eq!(renderer.bodies.len(), 4);
        let shadow = &renderer.bodies[0];
        assert_eq!(shadow.triangles.len(), 1);
        assert_eq!(shadow.view_pin, ViewPinMode::Raw);
        assert_eq!(shadow.materials[0].unlit_color(), [0.12, 0.12, 0.15]);
        assert_eq!(shadow.transform.position, root_position);
        assert_eq!(shadow.depth_fade, depth_fade);

        let root = &renderer.bodies[1];
        assert_eq!(root.triangles.len(), 3);
        assert_eq!(root.face_uvs.len(), 3);
        assert_eq!(root.face_uvs[0], [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        assert_eq!(root.materials.len(), 3);
        assert_eq!(
            root.materials[0].unlit_color(),
            [48.0 / 255.0, 96.0 / 255.0, 152.0 / 255.0]
        );
        assert_eq!(root.normals.len(), 3);
        assert_eq!(root.face_corner_normals.len(), 3);
        assert_eq!(root.face_shading, vec![ModelFaceShading::Flat; 3]);
        assert_eq!(root.shade_table.len(), 8);
        assert_eq!(root.shade_table[7].shade_level, 14);
        assert_eq!(root.light_direction_raw, light_direction_raw);
        assert_eq!(root.shade_shift, -3);
        assert_eq!(root.transform.position, root_position);
        assert_eq!(root.transform.scale, 0.5);
        assert_eq!(root.depth_fade, depth_fade);
        assert_eq!(root.view_pin, ViewPinMode::CameraFacing);
        assert_eq!(
            root.external_frame,
            ExternalFrameMode::WorldPoint([7.0, 0.0, 9.0])
        );

        // The child triangle can only resolve when the op-0x0E linked-slot
        // table reaches its type-11 vertex. Its authored attachment is +0.5 X.
        let child = &renderer.bodies[2];
        assert_eq!(child.triangles.len(), 1);
        assert_eq!(child.vertex_type_flags[0], 11);
        assert_eq!(child.transform.position, [10.5, 20.0, 30.0]);
        assert_eq!(child.transform.orientation, FLIP_X);
        assert_eq!(child.light_direction_raw, light_direction_raw);
        assert_eq!(child.depth_fade, depth_fade);
        assert_eq!(child.view_pin, ViewPinMode::CameraFacing);
        assert_eq!(child.external_frame, root.external_frame);

        // The same hook reaches the grandchild. Its +X attachment is rotated
        // by the child's flip before the grandchild's own local rotation.
        let grandchild = &renderer.bodies[3];
        assert_eq!(grandchild.light_direction_raw, light_direction_raw);
        assert_eq!(grandchild.transform.position, [10.0, 20.0, 30.0]);
        assert_eq!(
            grandchild.transform.orientation,
            mat3_mul(FLIP_X, ROTATE_X_QUARTER)
        );
        assert_eq!(grandchild.depth_fade, depth_fade);
        assert_eq!(grandchild.view_pin, ViewPinMode::CameraFacing);
        assert_eq!(grandchild.external_frame, root.external_frame);
        assert_eq!(*hook.calls.borrow(), vec![1, 2]);

        assert_eq!(renderer.billboards.len(), 1);
        let billboard = &renderer.billboards[0];
        assert_eq!(billboard.billboards.len(), 1);
        assert_eq!(billboard.billboards[0].id, 1);
        assert_eq!(billboard.materials.len(), 1);
        assert_eq!(
            billboard.materials[0].face.unlit_color(),
            [32.0 / 255.0, 64.0 / 255.0, 96.0 / 255.0]
        );
        assert!(!billboard.vertices.is_empty());
        assert_eq!(
            billboard.orientation,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        );
        assert_eq!(billboard.position, root_position);
        assert_eq!(billboard.scale, 0.5);
        assert_eq!(billboard.depth_fade, depth_fade);

        // Record all geometry fields to make omissions in future shared-path
        // refactors visible rather than silently accepted by the backend.
        assert!(!root.vertices.is_empty());
        assert!(!root.vertex_type_flags.is_empty());
    }
    #[test]
    fn sub_m_nested_marker_uses_current_child_and_root_far_rejection_stops_writeback() {
        use crate::sub_m_external_frame::{SubMPresentation, SubMProductMarkerWrite};
        use v2k_formats::collision::StatusComponentDescriptor;
        let root = model_entry(
            0,
            "factory",
            vec![[0, 256, 0, 0]],
            vec![0x0E, 0, 1, 8, 0, 0],
        );
        let child = model_entry(
            1,
            "arbitrary_lift",
            vec![
                [14, 0, 0, 0],
                [0, 100, 0, 0],
                [0, 0, 100, 0],
                [0, 0, 0, 0],
                [0, 0, 128, 0],
            ],
            vec![0x03, 0, 0, 0, 2, 4, 0],
        );
        let cache = synthetic_cache(vec![root, child]);
        let colors = ModelMaterialCache::new();
        let descriptor = StatusComponentDescriptor {
            raw_word_at_0x00: 8,
            variable_bindings: [0; 6],
            raw_tail: [0; 10],
        };
        let mut renderer = RecordingRenderer::default();
        let mut write = None;
        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
            .with_view(ModelTreeView {
                position: [0.0; 3],
                forward: [0.0, 0.0, -1.0],
            })
            .with_sub_m_presentation(SubMPresentation::new(
                descriptor,
                42,
                [0.0, 0.0, -10.0],
                [0.0, 0.0, -10.0],
                &mut write,
            ))
            .draw_linked(
                0,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [0.0, 0.0, -10.0],
                8,
                None,
                &AnimVars::default(),
            );
        assert_eq!(
            write,
            Some(SubMProductMarkerWrite {
                product_id: 42,
                position_raw: [256, 128, -2560]
            })
        );
        let ExternalFrameMode::SelectorWorldPoints(points) =
            renderer.bodies.last().unwrap().external_frame
        else {
            panic!("child marker result");
        };
        assert_eq!(points[0], Some([1.0, 0.5, -10.0]));

        let mut renderer = RecordingRenderer::default();
        renderer.set_fog(true, 13.0, 21.0, [0.0; 3]);
        let mut write = None;
        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
            .with_view(ModelTreeView {
                position: [0.0; 3],
                forward: [0.0, 0.0, -1.0],
            })
            .with_sub_m_presentation(SubMPresentation::new(
                descriptor,
                42,
                [0.0, 0.0, -22.0],
                [0.0, 0.0, -22.0],
                &mut write,
            ))
            .draw_linked(
                0,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [0.0, 0.0, -22.0],
                8,
                None,
                &AnimVars::default(),
            );
        assert!(renderer.bodies.is_empty());
        assert_eq!(write, None);
    }

    #[v2k_test_support::retail_test]
    fn level_two_factory_native_birth_delivery_and_draw_marker_are_separate_owners() {
        use crate::entity::{
            AuthoredWorldConstruction, EntityConstructionResources, EntityManager,
        };
        use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
        use crate::entity_scheduler::{
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        };
        use crate::factory_production::FactoryProductionPhase;
        use crate::factory_production_live::published_status;
        use crate::factory_status_runtime::project_factory_status;
        use crate::gameplay_notifications::GameplayNotifications;
        use crate::intro2_type66::{
            tick_intro2_type66_owner, Intro2Type66Frame, Intro2Type66Outcome, Intro2Type66Owner,
        };
        use crate::session::GameSession;
        use crate::static_damage::StaticDamageScheduler;
        use crate::sub_m_external_frame::SubMPresentation;
        use crate::world_fx::WorldFx;
        let dir = v2k_test_support::retail_dir();
        assert!(dir.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&dir).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        let metadata: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(ty, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(ty).unwrap(),
                )
            })
            .collect();
        session.load_level_by_id(14, 1).unwrap();
        let mut fx = WorldFx::new();
        let model_extent_raw = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 2,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&model_extent_raw),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let id = manager
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(39))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.model_index, Some(218));
        entity.collision.state_flags_at_0x08.overwrite(
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        let RetailRuntimeValue::Known(Some(base)) = &mut entity.base_factory_runtime else {
            panic!("Sub-M");
        };
        let production = base.production.as_mut().unwrap();
        assert_eq!(production.scientist_capacity_raw, 3);
        production.current_scientists_raw = production.scientist_capacity_raw;
        *base = project_factory_status(*base, published_status(base.production.unwrap())).after;
        let mut owner = Intro2Type66Owner::adopt(&manager, id).unwrap();
        let mut notifications = GameplayNotifications::new();
        let mut damage = StaticDamageScheduler::new();
        let mut product_id = 0;
        let mut birth_position = None;
        let mut first_marker_position = None;
        let mut delivered_position = None;
        for tick in 1..=3000 {
            let world_style_raw = session.cache.level_desc().unwrap().world_style;
            let visit = tick_intro2_type66_owner(
                &mut manager,
                owner,
                Intro2Type66Frame {
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                    static_damage: &mut damage,
                    elapsed_micros: 20_000,
                    retail_tick: tick,
                    world_style_raw,
                    main_base_abort_active: false,
                },
            );
            assert!(
                matches!(visit.outcome, Intro2Type66Outcome::Advanced { .. }),
                "tick{tick}: {:?}",
                visit.outcome
            );
            owner = visit.retained_owner.unwrap();
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
                panic!("Sub-M");
            };
            let production = base.production.unwrap();
            if product_id == 0 && production.spawned_pickup_handle != 0 {
                product_id = production.spawned_pickup_handle;
                let product = manager.iter_all().find(|e| e.id == product_id).unwrap();
                assert!(product.factory_type61_birth_provenance().is_some());
                assert_eq!(
                    product.power_up_payload_packed,
                    Some(production.output_payload_packed)
                );
                assert_eq!(
                    product.collision.recent_relation_id_at_0x60,
                    RetailRuntimeValue::Known(Some(id))
                );
                assert!(product.attached_to.is_none());
                assert_eq!(
                    product.position_raw(),
                    entity.position_raw(),
                    "zero authored offset birth"
                );
                birth_position = Some(product.position_raw());
            }
            // This is the actual shared draw path, with the native retained
            // actor basis, current published variables and nested linked frame.
            let position = entity.position;
            let orientation = match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
                RetailRuntimeValue::Unresolved => v2k_render::orientation_from_ypr(
                    std::f32::consts::FRAC_PI_2 - entity.heading,
                    0.0,
                    0.0,
                ),
            };
            let vars = entity.presentation_anim_vars(tick);
            let model_id = entity.model_index.unwrap();
            let mut write = None;
            let presentation = SubMPresentation::for_entity(entity, position, &mut write).unwrap();
            let mut renderer = RecordingRenderer::default();
            let colors = ModelMaterialCache::new();
            ModelTreeRenderer::new_world(
                &mut renderer,
                &session.cache,
                &colors,
                100.0 / 256.0,
                None,
                tick as i32,
            )
            .with_view(ModelTreeView {
                position: [position[0], position[1] + 4.0, position[2] + 8.0],
                forward: [0.0, 0.0, -1.0],
            })
            .with_sub_m_presentation(presentation)
            .draw_linked(model_id, orientation, position, 8, None, &vars);
            if let Some(write) = write {
                manager.apply_sub_m_product_marker_write(write);
                if product_id != 0 && first_marker_position.is_none() {
                    first_marker_position = Some(
                        manager
                            .iter_all()
                            .find(|e| e.id == product_id)
                            .unwrap()
                            .position_raw(),
                    );
                }
            }
            if production.phase == FactoryProductionPhase::WaitingForPickup {
                delivered_position = Some(
                    manager
                        .iter_all()
                        .find(|e| e.id == product_id)
                        .unwrap()
                        .position_raw(),
                );
                break;
            }
        }
        let birth = birth_position.expect("native factory produced a real Type61");
        let rest = first_marker_position.expect("submitted factory3 child executed Sub-M marker");
        let delivered = delivered_position.expect("native delivery timer completed");
        assert_ne!(rest, birth, "marker leaves the birth origin");
        assert_ne!(
            delivered, rest,
            "native delivery animation moves the marker"
        );
        eprintln!(
            "LEVEL2_NATIVE model218 birth={birth:?} firstmarker={rest:?} delivered={delivered:?}"
        );
    }

    #[test]
    fn native_missing_edge_receipt_stops_actual_sub_h_writeback_before_buffer() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionAuthorityMissing as Missing,
            ProjectionCompatibilityReason as Compat, SceneProjectionAuthority as Authority,
        };
        let model = model_entry(
            0,
            "missing-first-edge",
            vec![
                [0, -10, 0, 0],
                [0, 10, 0, 0],
                [1, 0, 0, 2],
                [14, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 100, 0],
                [0, 100, 100, 0],
            ],
            vec![0x22, 986, 4, 4, 6, 0],
        );
        let cache = synthetic_cache(vec![model]);
        let colors = ModelMaterialCache::new();
        let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, -i32::MAX]],
            identity: false,
        };
        let frame = NativeModelFrame::from_actor(viewport, [0, 0, -2560], axes);
        let descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [8, 10, 12],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        };
        let native = Authority::Native(
            NativeScreenProjection::new(
                [512, 512],
                [320, 240],
                [640, 480],
                v2k_render::projection::ProjectionEffect::None,
            )
            .unwrap(),
        );
        for authority in [
            native,
            Authority::Missing(Missing::NativeLens),
            Authority::Missing(Missing::NativeView),
            Authority::Compatibility(Compat::Free),
            Authority::Compatibility(Compat::LogicalViewportAdapter),
            Authority::Compatibility(Compat::Intrinsic),
        ] {
            let required = !matches!(authority, Authority::Compatibility(_));
            let mut runtime = SubHRuntimeState::new(1).unwrap();
            runtime.set_enabled(false);
            let before = runtime.records()[0];
            let mut renderer = RecordingRenderer::default();
            let mut buffer = ModelTreeSubmissionBuffer::default();
            {
                let mut tree = ModelTreeRenderer::new_world(
                    &mut renderer,
                    &cache,
                    &colors,
                    100.0 / 256.0,
                    None,
                    0,
                )
                .with_scene_projection_authority(authority)
                .with_submission_buffer(&mut buffer)
                .with_view(ModelTreeView {
                    position: [0.0; 3],
                    forward: [0.0, 0.0, -1.0],
                })
                .with_sub_h_presentation(SubHPresentation {
                    runtime: &mut runtime,
                    retail_tick: 0,
                    descriptor: &descriptor,
                    model_records: &cache.global_model(0).unwrap().records,
                    origin_raw: [0, 0, -2560],
                    origin_world: [0.0, 0.0, -10.0],
                    body_axes_q31: Some(axes),
                    native_context: Some((frame.clone(), viewport)),
                    fallback: None,
                    emitter: None,
                });
                tree.draw_linked(
                    0,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    [0.0, 0.0, -10.0],
                    2,
                    None,
                    &AnimVars::default(),
                );
                assert_eq!(tree.native_edge_failures().len(), usize::from(required));
                if required {
                    assert_eq!(
                        tree.native_edge_failures()[0],
                        ModelTreeNativeEdgeFailure {
                            model_id: 0,
                            command: ModelNativeEdgeFailure {
                                source_edge_index: 0,
                                source_slots: [4, 6],
                                boundary:
                                    v2k_formats::models::ModelNativeEdgeBoundary::ScreenMidpoint {
                                        slot: 4
                                    },
                            }
                        }
                    );
                }
            }
            if required {
                assert_eq!(
                    runtime.records()[0],
                    before,
                    "failed edge cannot first invoke the second endpoint's live H owner"
                );
            } else {
                assert_ne!(
                    runtime.records()[0],
                    before,
                    "deliberate compatibility retains the reached H callback"
                );
            }
            buffer.flush(&mut renderer, None);
            if required {
                assert!(
                    renderer.bodies.iter().all(|body| body.edges.is_empty()),
                    "buffer cannot resurrect failed geometry"
                );
            } else {
                assert!(
                    renderer.bodies.iter().any(|body| !body.edges.is_empty()),
                    "adapted geometry remains explicit compatibility"
                );
            }
        }
    }

    #[test]
    fn native_painter_preflight_reads_instruction_paths_before_any_live_owner() {
        use v2k_formats::models::NativePainterProgramBoundary as Boundary;
        let mut model = model_entry(
            0,
            "native-preflight",
            vec![[0, 0, 0, 0]; 3],
            vec![0x03, 0x0E, 0, 0, 2, 4, 0],
        );
        assert_eq!(
            model.native_painter_root_preflight(),
            Ok(()),
            "an operand 0E is not an instance command"
        );
        model.cmd_words = vec![0x2B, 9, 1, 1, 0x0E, 6, 1, 0, 0, 0, 0];
        assert_eq!(
            model.native_painter_root_preflight(),
            Err(Boundary::ChildInstance { word_offset: 4 }),
            "a currently skipped child is still an unowned possible live path"
        );
        model.cmd_words = vec![0x03, 0];
        assert_eq!(
            model.native_painter_root_preflight(),
            Err(Boundary::MalformedInstruction { word_offset: 0 })
        );
        model.cmd_words = vec![0x99, 0];
        assert_eq!(
            model.native_painter_root_preflight(),
            Err(Boundary::UnknownOpcode {
                word_offset: 0,
                opcode: 0x99
            })
        );
        model.cmd_words = vec![0x13, 3, 0, 0, 0];
        assert_eq!(
            model.native_painter_root_preflight(),
            Err(Boundary::UnsupportedDistanceBranch {
                word_offset: 0,
                opcode: 0x13
            })
        );
    }

    fn native_painter_session() -> crate::session::GameSession {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "native painter retail corpus is required"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        session
    }

    #[v2k_test_support::retail_test]
    fn static_objects_publish_integer_view_points_and_model_light() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let data = v2k_test_support::retail_dir();
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(50, 1).unwrap();
        let cache = &session.cache;
        // Intro2's tick-98 camera and the village hut at cell [139, 124].
        let viewport = NativeWorldViewport {
            origin_raw: [-29184, 590, 30834],
            axes_q31: [
                [2147347834, 0, 0],
                [0, 2072352042, 560095147],
                [0, -560130571, 2072483113],
            ],
            identity: false,
        };
        let frame = NativeModelFrame::from_static_object(viewport, [35712_u16 as i16, -272, 31872]);
        assert_eq!(frame.origin_view_raw, [-640, -562, 1225]);
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let position = [139.5, -272.0 / 256.0, 124.5];
        let mut tree =
            ModelTreeRenderer::new_world(&mut renderer, cache, &colors, 100.0 / 256.0, None, 98)
                .with_scene_projection_authority(SceneProjectionAuthority::Native(
                    NativeScreenProjection::new(
                        [512, 512],
                        [320, 240],
                        [640, 480],
                        ProjectionEffect::None,
                    )
                    .unwrap(),
                ))
                .with_view(ModelTreeView {
                    position: [142.0, 590.0 / 256.0, 30834.0 / 256.0],
                    forward: [0.0, 0.0, 1.0],
                })
                .with_native_external_frame(
                    ExternalFrameMode::WorldPoint([position[0], 0.0, position[2]]),
                    Some((frame.clone(), viewport)),
                );
        tree.draw_linked(
            364,
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position,
            8,
            None,
            &AnimVars::default(),
        );
        drop(tree);
        let body = renderer.bodies.first().expect("hut body");
        assert!(!body.triangles.is_empty());
        assert_eq!(body.vertex_view_raw.len(), body.vertices.len());
        assert!(
            body.vertex_view_raw.iter().all(Option::is_some),
            "every hut vertex is a plain slot of the static node"
        );
        assert_eq!(body.face_normals_raw.len(), body.triangles.len());
        assert_eq!(
            body.native_light_raw,
            Some(frame.model_light_raw(body.light_direction_raw))
        );
    }

    #[v2k_test_support::retail_test]
    fn native_painter_actual_newant_uses_paired_view_keys_and_drains_prepared_h_edges() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::models::{ModelPainterDepthKey, ModelPainterOp};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let mut session = native_painter_session();
        // Tick952 belongs to Intro2. Its authenticated terrain50 gives the
        // captured tf13 shadow caches; fresh ordinary terrain13 differs here.
        // Unmodified terrain50 and the captured process map agree at all seven
        // reached body-shadow samples, despite unrelated distant crater cells.
        session.load_level_by_id(50, 1).unwrap();
        let model = session.cache.global_model(302).unwrap();
        assert_eq!(model.native_painter_root_preflight(), Ok(()));
        let ty = session.cache.global_entity_type(47).unwrap();
        let descriptor = ty.sub_h_external_frame_descriptor().unwrap();
        assert_eq!(descriptor.records.len(), 6);
        // Accepted V200001 tick952 current draw words. Warm H endpoints are
        // supplied from that draw's completed records; this fixture owns its
        // prepared model queue, not the enclosing scene arena or framebuffer.
        let viewport = NativeWorldViewport {
            origin_raw: [-15097, -243, 28884],
            axes_q31: [
                [2105482641, 0, 422129991],
                [-61663396, 2124313119, 307562155],
                [-417665851, -313697527, 2082951585],
            ],
            identity: false,
        };
        let axes = [
            [1621229568, -462225408, -1329135616],
            [398852096, 2095841280, -242089984],
            [1349451776, -64225280, 1668481024],
        ];
        let origin = [-17501, -551, 31307];
        let frame = NativeModelFrame::from_actor(viewport, origin, axes);
        assert_eq!(frame.origin_view_raw, [-1881, 111, 2861]);
        let world = viewport
            .actor_world_image(origin)
            .map(|value| value as f32 / 256.0);
        let orientation = std::array::from_fn(|axis| {
            std::array::from_fn(|component| axes[component][axis] as f32 / 2147483648.0)
        });
        let view = ModelTreeView {
            position: viewport.origin_raw.map(|value| value as f32 / 256.0),
            forward: viewport.axes_q31[2].map(|value| -(value as f32 / 2147483648.0)),
        };
        let mut vars = AnimVars::default();
        vars.dynamic[0] = 952;
        vars.registers = [
            48418, 65535, 2287, 0, 6573, 0, 37536, 76, 43516, 686, 43516, 686, 0, 27259, 64416, 26,
            37360, 66, 65407, 65535, 10000, 0, 0, 0, 0, 0, 57728, 687, 52680, 9002, 0, 0, 1, 1218,
            29736, 79, 29976, 79, 0, 0, 0, 0, 0, 0, 1, 0, 0, 24794, 0, 0, 0, 44125, 0, 0, 0, 32766,
            0, 0, 0, 21411, 0, 0, 0, 24794,
        ];
        let mut runtime = SubHRuntimeState::new(6).unwrap();
        runtime.set_enabled(true);
        for (record, (primary, secondary)) in runtime.records_mut().iter_mut().zip([
            ([-17334, -682, 31293], [-17381, -587, 31289]),
            ([-17529, -608, 31529], [-17521, -567, 31426]),
            ([-17308, -671, 31341], [-17398, -601, 31307]),
            ([-17627, -586, 31505], [-17571, -548, 31405]),
            ([-17420, -676, 31220], [-17425, -579, 31227]),
            ([-17639, -610, 31378], [-17598, -525, 31353]),
        ]) {
            record.flags_raw = 7;
            record.primary_raw = primary;
            record.secondary_raw = secondary;
        }
        let before = runtime.clone();
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let mut buffer = ModelTreeSubmissionBuffer::default();
        let mut stamps = Vec::new();
        let mut stamp = |point| stamps.push(point);
        let mut emitter_boundary = None;
        let authority = SceneProjectionAuthority::Native(
            NativeScreenProjection::new([512, 512], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap(),
        );
        let mut tree = ModelTreeRenderer::new_world(
            &mut renderer,
            &session.cache,
            &colors,
            100.0 / 256.0,
            None,
            952,
        )
        .with_scene_projection_authority(authority)
        .with_submission_buffer(&mut buffer)
        .with_painter_view(view, ModelTreePainterComposition::Isolated)
        .with_sub_h_presentation(SubHPresentation {
            runtime: &mut runtime,
            retail_tick: 952,
            descriptor: &descriptor,
            model_records: &model.records,
            origin_raw: origin,
            origin_world: world,
            body_axes_q31: Some(axes),
            native_context: Some((frame.clone(), viewport)),
            fallback: None,
            emitter: Some(
                crate::actor_emitter_external_frame::ActorEmitterExternalFramePresentation {
                    descriptor: ty.projectile_emitter_descriptor().unwrap(),
                    root_model: model,
                    root_vars: &vars,
                    frame,
                    viewport,
                    stamp_origin: &mut stamp,
                    last_boundary: &mut emitter_boundary,
                    current_node_owned: true,
                    current_source_points_view: None,
                },
            ),
        });
        let (materialized, links, resolution) = tree.materialize_live_node(
            302,
            model,
            orientation,
            world,
            8,
            None,
            &vars,
            ModelTreeRootLinkPolicy::Authored,
        );
        assert!(links.is_empty());
        assert_eq!(resolution, ModelSurfaceResolution::ContextResolved);
        // The recorded native command/normal walk and warmed first-slot VIEW
        // cache supply these keys independently of the renderer's float pose.
        // PE constructors 45A4E0/45DB30/463040/462070/464270 and 458C60/459000
        // prove the first endpoint contract; source keys precede stable drain.
        let expected_depths = [
            2932, 2877, 2860, 2860, 2928, 2913, 2913, 2848, 2848, 2835, 2835, 2900, 2900, 3091,
            3091, 2835, 2835, 2875, 2875, 2887, 2887, 3084, 3084, 2830, 2830, 2779, 2779, 2882,
            2882, 2966, 2966, 2931, 2894, 2833, 2884, 2882, 2882, 2882, 2925, 2893, 2923, 2905,
            2905, 2851, 2851, 2866, 2851, 2851, 2848, 2829, 2830, 2830, 2825, 2825, 2830, 2769,
            2767, 2767, 2769, 2767, 2767, 2757, 2754, 2796, 2754, 2905, 2905, 2941, 2941,
        ];
        let keys = materialized
            .painter_program
            .iter()
            .map(|op| match op {
                ModelPainterOp::Face { depth_key, .. } | ModelPainterOp::Edge { depth_key, .. } => {
                    depth_key.clone()
                }
                _ => panic!("captured newant has no reached child/group/billboard in this draw"),
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, expected_depths.map(ModelPainterDepthKey::Native));
        let mut expected = materialized
            .painter_program
            .iter()
            .zip(expected_depths)
            .enumerate()
            .collect::<Vec<_>>();
        expected.sort_by(|left, right| (right.1).1.cmp(&(left.1).1));
        let signatures = expected
            .iter()
            .map(|(_, (op, _))| match op {
                ModelPainterOp::Face { triangle_range, .. } => (
                    materialized.triangles[triangle_range.clone()].to_vec(),
                    vec![],
                ),
                ModelPainterOp::Edge { edge_index, .. } => {
                    (vec![], vec![materialized.edges[*edge_index].clone()])
                }
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        tree.draw_linked(302, orientation, world, 8, None, &vars);
        assert!(tree.native_edge_failures().is_empty());
        drop(tree);
        assert_eq!(
            runtime, before,
            "warm H reads and queue drain do not advance its state"
        );
        assert_eq!(
            buffer.len(),
            69 * 2,
            "one prepared body plus empty billboard command per face/edge"
        );
        assert!(
            renderer.bodies.is_empty(),
            "enqueue owns data before renderer drain"
        );
        buffer.flush(&mut renderer, None);
        let actual = renderer
            .bodies
            .iter()
            .map(|body| (body.triangles.clone(), body.edges.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            actual, signatures,
            "active painter consumer uses native command-time keys and stable ties"
        );
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_policy == ModelDepthPolicy::Painter
                && body.surface_resolution == ModelSurfaceResolution::ContextResolved
                && matches!(body.external_frame, ExternalFrameMode::Raw)));
        assert_eq!(
            renderer
                .bodies
                .iter()
                .map(|body| body.edges.len())
                .sum::<usize>(),
            34
        );
        assert!(emitter_boundary.is_none());
        assert!(
            !stamps.is_empty(),
            "authored selector12 executes through the real Sub-E owner"
        );
    }

    #[v2k_test_support::retail_test]
    fn native_painter_accepts_actual_spider_root_and_rejects_stag_children_before_h() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let session = native_painter_session();
        let colors = ModelMaterialCache::new();
        let viewport = NativeWorldViewport {
            origin_raw: [0, 1024, 2048],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, -i32::MAX]],
            identity: false,
        };
        let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        for (type_id, model_id, accepted) in [(17, 256, true), (26, 267, false)] {
            let model = session.cache.global_model(model_id).unwrap();
            assert_eq!(model.native_painter_root_preflight().is_ok(), accepted);
            let descriptor = session
                .cache
                .global_entity_type(type_id)
                .unwrap()
                .sub_h_external_frame_descriptor()
                .unwrap();
            let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
            runtime.set_enabled(true);
            if accepted {
                for (index, record) in runtime.records_mut().iter_mut().enumerate() {
                    record.flags_raw = 7;
                    record.primary_raw = [128 + index as i16 * 32, 0, 0];
                    record.secondary_raw = [160 + index as i16 * 32, 128, 128];
                }
            }
            let before = runtime.clone();
            let mut renderer = RecordingRenderer::default();
            let mut buffer = ModelTreeSubmissionBuffer::default();
            ModelTreeRenderer::new_world(
                &mut renderer,
                &session.cache,
                &colors,
                100.0 / 256.0,
                None,
                1000,
            )
            .with_scene_projection_authority(SceneProjectionAuthority::Native(
                NativeScreenProjection::new(
                    [512, 512],
                    [320, 240],
                    [640, 480],
                    ProjectionEffect::None,
                )
                .unwrap(),
            ))
            .with_submission_buffer(&mut buffer)
            .with_painter_view(
                ModelTreeView {
                    position: [0.0, 4.0, 8.0],
                    forward: [0.0, 0.0, -1.0],
                },
                ModelTreePainterComposition::Isolated,
            )
            .with_sub_h_presentation(SubHPresentation {
                runtime: &mut runtime,
                retail_tick: 1000,
                descriptor: &descriptor,
                model_records: &model.records,
                origin_raw: [0, 512, 0],
                origin_world: [0.0, 2.0, 0.0],
                body_axes_q31: Some(axes),
                native_context: Some((
                    NativeModelFrame::from_actor(viewport, [0, 512, 0], axes),
                    viewport,
                )),
                fallback: None,
                emitter: None,
            })
            .draw_linked(
                model_id,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [0.0, 2.0, 0.0],
                8,
                None,
                &AnimVars::default(),
            );
            assert_eq!(runtime, before, "unsupported child paths cannot invoke H, and warmed sibling reads cannot replay it");
            if accepted {
                assert!(!buffer.is_empty());
                buffer.flush(&mut renderer, None);
                assert!(renderer.bodies.iter().any(|body| !body.edges.is_empty()));
                assert!(renderer
                    .bodies
                    .iter()
                    .all(|body| body.depth_policy == ModelDepthPolicy::Painter));
            } else {
                assert!(buffer.is_empty());
            }
        }
    }

    #[test]
    fn native_painter_prepared_group_drains_mixed_billboards_and_edges_without_h_replay() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let model = model_entry(
            0,
            "mixed-native-root",
            vec![
                [0, -10, 0, 0],
                [0, 10, 0, 100],
                [0, 0, 10, 200],
                [14, 0, 0, 0],
            ],
            vec![
                0x06, 0, 2, 0xFFFF, 0x03, 10, 0, 0, 2, 4, 0x02, 20, 2, 4, 0x68, 4, 30, 3, 0, 0xE6,
                0x02, 40, 6, 4, 0,
            ],
        );
        let cache = synthetic_cache(vec![model]);
        let model = cache.global_model(0).unwrap();
        let descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [0, 2, 4],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        };
        let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: axes,
            identity: true,
        };
        let frame = NativeModelFrame::from_actor(viewport, [0, 0, 512], axes);
        let mut runtime = SubHRuntimeState::new(1).unwrap();
        runtime.set_enabled(true);
        runtime.records_mut()[0].flags_raw = 7;
        runtime.records_mut()[0].primary_raw = [0, 0, 900];
        let before = runtime.clone();
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let mut buffer = ModelTreeSubmissionBuffer::default();
        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
            .with_scene_projection_authority(SceneProjectionAuthority::Native(
                NativeScreenProjection::new(
                    [512, 512],
                    [320, 240],
                    [640, 480],
                    ProjectionEffect::None,
                )
                .unwrap(),
            ))
            .with_submission_buffer(&mut buffer)
            .with_painter_view(
                ModelTreeView {
                    position: [0.0; 3],
                    forward: [0.0, 0.0, 1.0],
                },
                ModelTreePainterComposition::Isolated,
            )
            .with_sub_h_presentation(SubHPresentation {
                runtime: &mut runtime,
                retail_tick: 0,
                descriptor: &descriptor,
                model_records: &model.records,
                origin_raw: [0, 0, 512],
                origin_world: [0.0, 0.0, 2.0],
                body_axes_q31: Some(axes),
                native_context: Some((frame, viewport)),
                fallback: None,
                emitter: None,
            })
            .draw_linked(
                0,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [0.0, 0.0, 2.0],
                8,
                None,
                &AnimVars::default(),
            );
        assert_eq!(runtime, before);
        buffer.flush(&mut renderer, None);
        assert_eq!(
            renderer.bodies.len(),
            3,
            "outside edge, then atomic group face and edge"
        );
        assert_eq!(renderer.billboards.len(), 1);
        assert_eq!(
            renderer.order,
            ["edge", "face", "edge", "billboard"],
            "FIFO group must retain mixed constructor order despite unequal native keys"
        );
        assert_eq!(
            renderer.billboards[0].depth_policy,
            ModelDepthPolicy::Painter,
            "billboard cannot re-enable geometry depth during isolated drain"
        );
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_policy == ModelDepthPolicy::Painter));
    }

    #[test]
    fn native_painter_projection_authority_keeps_free_keys_and_missing_custody_separate() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionAuthorityMissing, ProjectionCompatibilityReason,
            ProjectionEffect, SceneProjectionAuthority,
        };
        let model = model_entry(
            0,
            "projection-key-custody",
            vec![[0, 0, 0, 100], [0, 0, 0, 200], [0, 100, 0, 0]],
            vec![0x02, 10, 0, 4, 0x02, 20, 2, 4, 0],
        );
        let cache = synthetic_cache(vec![model]);
        let model = cache.global_model(0).unwrap();
        let descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [0, 2, 4],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        };
        let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: axes,
            identity: true,
        };
        let frame = NativeModelFrame::from_actor(viewport, [0, 0, 512], axes);
        let colors = ModelMaterialCache::new();
        let native = SceneProjectionAuthority::Native(
            NativeScreenProjection::new([512, 512], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap(),
        );
        for authority in [
            native,
            SceneProjectionAuthority::Compatibility(ProjectionCompatibilityReason::Free),
            SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeLens),
            SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeView),
        ] {
            let mut runtime = SubHRuntimeState::new(1).unwrap();
            runtime.set_enabled(true);
            let before = runtime.clone();
            let mut renderer = RecordingRenderer::default();
            let mut buffer = ModelTreeSubmissionBuffer::default();
            ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
                .with_scene_projection_authority(authority)
                .with_submission_buffer(&mut buffer)
                .with_painter_view(
                    ModelTreeView {
                        position: [0.0; 3],
                        forward: [0.0, 0.0, -1.0],
                    },
                    ModelTreePainterComposition::Isolated,
                )
                .with_sub_h_presentation(SubHPresentation {
                    runtime: &mut runtime,
                    retail_tick: 0,
                    descriptor: &descriptor,
                    model_records: &model.records,
                    origin_raw: [0, 0, 512],
                    origin_world: [0.0, 0.0, 2.0],
                    body_axes_q31: Some(axes),
                    native_context: Some((frame.clone(), viewport)),
                    fallback: None,
                    emitter: None,
                })
                .draw_linked(
                    0,
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    [0.0, 0.0, 2.0],
                    8,
                    None,
                    &AnimVars::default(),
                );
            assert_eq!(runtime, before);
            buffer.flush(&mut renderer, None);
            let first_materials = renderer
                .bodies
                .iter()
                .filter_map(|body| body.edges.first().map(|edge| edge.packed_material()))
                .collect::<Vec<_>>();
            match authority {
                SceneProjectionAuthority::Native(_) => assert_eq!(
                    first_materials,
                    [20, 10],
                    "source VIEW keys are independent from the compatibility pose"
                ),
                SceneProjectionAuthority::Compatibility(_) => assert_eq!(
                    first_materials,
                    [10, 20],
                    "Free explicitly keeps existing signed float keys"
                ),
                SceneProjectionAuthority::Missing(_) => assert!(
                    first_materials.is_empty(),
                    "claimed native missing authority rejects before preparation"
                ),
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn native_world_node_fog_uses_paired_frame_before_callbacks_and_survives_buffering() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::models::{ModelPainterDepthKey, ModelPainterOp};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let mut session = native_painter_session();
        // Tick952 belongs to Intro2. Its authenticated terrain50 gives the
        // captured tf13 shadow caches; fresh ordinary terrain13 differs here.
        // Unmodified terrain50 and the captured process map agree at all seven
        // reached body-shadow samples, despite unrelated distant crater cells.
        session.load_level_by_id(50, 1).unwrap();
        let model = session.cache.global_model(302).unwrap();
        assert_eq!(model.native_painter_root_preflight(), Ok(()));
        let ty = session.cache.global_entity_type(47).unwrap();
        let descriptor = ty.sub_h_external_frame_descriptor().unwrap();
        assert_eq!(descriptor.records.len(), 6);
        // Accepted V200001 tick952 current draw words. Warm H endpoints are
        // supplied from that draw's completed records; this fixture owns its
        // prepared model queue, not the enclosing scene arena or framebuffer.
        let viewport = NativeWorldViewport {
            origin_raw: [-15097, -243, 28884],
            axes_q31: [
                [2105482641, 0, 422129991],
                [-61663396, 2124313119, 307562155],
                [-417665851, -313697527, 2082951585],
            ],
            identity: false,
        };
        let axes = [
            [1621229568, -462225408, -1329135616],
            [398852096, 2095841280, -242089984],
            [1349451776, -64225280, 1668481024],
        ];
        let origin = [-17501, -551, 31307];
        let frame = NativeModelFrame::from_actor(viewport, origin, axes);
        assert_eq!(frame.origin_view_raw, [-1881, 111, 2861]);
        let world = viewport
            .actor_world_image(origin)
            .map(|value| value as f32 / 256.0);
        let orientation = std::array::from_fn(|axis| {
            std::array::from_fn(|component| axes[component][axis] as f32 / 2147483648.0)
        });
        let mut vars = AnimVars::default();
        vars.dynamic[0] = 952;
        vars.registers = [
            48418, 65535, 2287, 0, 6573, 0, 37536, 76, 43516, 686, 43516, 686, 0, 27259, 64416, 26,
            37360, 66, 65407, 65535, 10000, 0, 0, 0, 0, 0, 57728, 687, 52680, 9002, 0, 0, 1, 1218,
            29736, 79, 29976, 79, 0, 0, 0, 0, 0, 0, 1, 0, 0, 24794, 0, 0, 0, 44125, 0, 0, 0, 32766,
            0, 0, 0, 21411, 0, 0, 0, 24794,
        ];
        let mut runtime = SubHRuntimeState::new(6).unwrap();
        runtime.set_enabled(true);
        for (record, (primary, secondary)) in runtime.records_mut().iter_mut().zip([
            ([-17334, -682, 31293], [-17381, -587, 31289]),
            ([-17529, -608, 31529], [-17521, -567, 31426]),
            ([-17308, -671, 31341], [-17398, -601, 31307]),
            ([-17627, -586, 31505], [-17571, -548, 31405]),
            ([-17420, -676, 31220], [-17425, -579, 31227]),
            ([-17639, -610, 31378], [-17598, -525, 31353]),
        ]) {
            record.flags_raw = 7;
            record.primary_raw = primary;
            record.secondary_raw = secondary;
        }
        let before = runtime.clone();
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let fog = v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 3072,
                far_raw: 6144,
            },
            color: [0.2, 0.3, 0.4],
        };
        renderer.set_world_model_fog(Some(fog));
        let divergent_world = [5000.0; 3];
        let divergent_view = ModelTreeView {
            position: [-5000.0; 3],
            forward: [0.0, 0.0, 1.0],
        };
        assert!(!divergent_view.admits_model(
            model.flags,
            model.radius,
            divergent_world,
            100.0 / 256.0,
            Some(24.0)
        ));
        let mut buffer = ModelTreeSubmissionBuffer::default();
        let stamps = std::cell::Cell::new(0usize);
        let mut stamp = |_point| stamps.set(stamps.get() + 1);
        let mut emitter_boundary = None;
        let authority = SceneProjectionAuthority::Native(
            NativeScreenProjection::new([512, 512], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap(),
        );
        let mut tree = ModelTreeRenderer::new_world(
            &mut renderer,
            &session.cache,
            &colors,
            100.0 / 256.0,
            None,
            952,
        )
        .with_scene_projection_authority(authority)
        .with_submission_buffer(&mut buffer)
        .with_painter_view(divergent_view, ModelTreePainterComposition::Isolated)
        .with_sub_h_presentation(SubHPresentation {
            runtime: &mut runtime,
            retail_tick: 952,
            descriptor: &descriptor,
            model_records: &model.records,
            origin_raw: origin,
            origin_world: world,
            body_axes_q31: Some(axes),
            native_context: Some((frame.clone(), viewport)),
            fallback: None,
            emitter: Some(
                crate::actor_emitter_external_frame::ActorEmitterExternalFramePresentation {
                    descriptor: ty.projectile_emitter_descriptor().unwrap(),
                    root_model: model,
                    root_vars: &vars,
                    frame,
                    viewport,
                    stamp_origin: &mut stamp,
                    last_boundary: &mut emitter_boundary,
                    current_node_owned: true,
                    current_source_points_view: None,
                },
            ),
        });
        let selected = ModelDepthFade::WorldRaw {
            fog,
            pass: v2k_render::NativeModelFogPass::Near,
        };
        assert_eq!(model.radius, 182);
        assert_eq!(tree.node_depth_fade(model, divergent_world), Some(selected));
        // Far rejection happens before both the real H owner and selector12 E.
        tree.world_model_fog = Some(v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 1024,
                far_raw: 2048,
            },
            ..fog
        });
        // Clear the genuine H warm-cache bit: an executed H resolver could
        // now mutate this record, unlike an already-completed warm read.
        let rejected_h =
            if let ModelTreeExternalFrame::SubH(presentation) = &mut tree.external_frame {
                presentation.runtime.records_mut()[0].flags_raw = 6;
                presentation.runtime.clone()
            } else {
                unreachable!()
            };
        tree.draw_linked(302, orientation, divergent_world, 8, None, &vars);
        assert_eq!(stamps.get(), 0, "rejected node must not execute Sub-E");
        assert_eq!(tree.submission_buffer.as_ref().unwrap().len(), 0);
        if let ModelTreeExternalFrame::SubH(presentation) = &mut tree.external_frame {
            assert_eq!(
                *presentation.runtime, rejected_h,
                "rejected node must not execute the dirty Sub-H owner"
            );
            presentation.runtime.records_mut()[0].flags_raw = 7;
        }
        tree.world_model_fog = Some(fog);
        let (materialized, links, resolution) = tree.materialize_live_node(
            302,
            model,
            orientation,
            divergent_world,
            8,
            None,
            &vars,
            ModelTreeRootLinkPolicy::Authored,
        );
        assert!(links.is_empty());
        assert_eq!(resolution, ModelSurfaceResolution::ContextResolved);
        // The recorded native command/normal walk and warmed first-slot VIEW
        // cache supply these keys independently of the renderer's float pose.
        // PE constructors 45A4E0/45DB30/463040/462070/464270 and 458C60/459000
        // prove the first endpoint contract; source keys precede stable drain.
        let expected_depths = [
            2932, 2877, 2860, 2860, 2928, 2913, 2913, 2848, 2848, 2835, 2835, 2900, 2900, 3091,
            3091, 2835, 2835, 2875, 2875, 2887, 2887, 3084, 3084, 2830, 2830, 2779, 2779, 2882,
            2882, 2966, 2966, 2931, 2894, 2833, 2884, 2882, 2882, 2882, 2925, 2893, 2923, 2905,
            2905, 2851, 2851, 2866, 2851, 2851, 2848, 2829, 2830, 2830, 2825, 2825, 2830, 2769,
            2767, 2767, 2769, 2767, 2767, 2757, 2754, 2796, 2754, 2905, 2905, 2941, 2941,
        ];
        let keys = materialized
            .painter_program
            .iter()
            .map(|op| match op {
                ModelPainterOp::Face { depth_key, .. } | ModelPainterOp::Edge { depth_key, .. } => {
                    depth_key.clone()
                }
                _ => panic!("captured newant has no reached child/group/billboard in this draw"),
            })
            .collect::<Vec<_>>();
        assert_eq!(keys, expected_depths.map(ModelPainterDepthKey::Native));
        let mut expected = materialized
            .painter_program
            .iter()
            .zip(expected_depths)
            .enumerate()
            .collect::<Vec<_>>();
        expected.sort_by(|left, right| (right.1).1.cmp(&(left.1).1));
        let signatures = expected
            .iter()
            .map(|(_, (op, _))| match op {
                ModelPainterOp::Face { triangle_range, .. } => (
                    materialized.triangles[triangle_range.clone()].to_vec(),
                    vec![],
                ),
                ModelPainterOp::Edge { edge_index, .. } => {
                    (vec![], vec![materialized.edges[*edge_index].clone()])
                }
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        tree.draw_linked(302, orientation, divergent_world, 8, None, &vars);
        assert!(tree.native_edge_failures().is_empty());
        drop(tree);
        assert_eq!(
            runtime, before,
            "warm H reads and queue drain do not advance its state"
        );
        assert_eq!(
            buffer.len(),
            69 * 2,
            "one prepared body plus empty billboard command per face/edge"
        );
        assert!(
            renderer.bodies.is_empty(),
            "enqueue owns data before renderer drain"
        );
        renderer.set_world_model_fog(Some(v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: -256,
                far_raw: 0,
            },
            color: [0.9; 3],
        }));
        buffer.flush(&mut renderer, None);
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_fade == selected));
        let actual = renderer
            .bodies
            .iter()
            .map(|body| (body.triangles.clone(), body.edges.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            actual, signatures,
            "active painter consumer uses native command-time keys and stable ties"
        );
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_policy == ModelDepthPolicy::Painter
                && body.surface_resolution == ModelSurfaceResolution::ContextResolved
                && matches!(body.external_frame, ExternalFrameMode::Raw)));
        assert_eq!(
            renderer
                .bodies
                .iter()
                .map(|body| body.edges.len())
                .sum::<usize>(),
            34
        );
        assert!(emitter_boundary.is_none());
        assert!(
            stamps.get() > 0,
            "admitted selector12 executes through the real Sub-E owner"
        );
    }

    #[test]
    fn native_world_node_fog_keeps_mixed_fifo_billboard_policy_after_publication_change() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let model = model_entry(
            0,
            "mixed-native-root",
            vec![
                [0, -10, 0, 0],
                [0, 10, 0, 100],
                [0, 0, 10, 200],
                [14, 0, 0, 0],
            ],
            vec![
                0x06, 0, 2, 0xFFFF, 0x03, 10, 0, 0, 2, 4, 0x02, 20, 2, 4, 0x68, 4, 30, 3, 0, 0xE6,
                0x02, 40, 6, 4, 0,
            ],
        );
        let cache = synthetic_cache(vec![model]);
        let model = cache.global_model(0).unwrap();
        let descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: 0,
                vertex_refs: [0, 2, 4],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        };
        let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: axes,
            identity: true,
        };
        let frame = NativeModelFrame::from_actor(viewport, [0, 0, 512], axes);
        let mut runtime = SubHRuntimeState::new(1).unwrap();
        runtime.set_enabled(true);
        runtime.records_mut()[0].flags_raw = 7;
        runtime.records_mut()[0].primary_raw = [0, 0, 900];
        let before = runtime.clone();
        let colors = ModelMaterialCache::new();
        let mut renderer = RecordingRenderer::default();
        let fog = v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 1000,
                far_raw: 2000,
            },
            color: [0.2, 0.3, 0.4],
        };
        renderer.set_world_model_fog(Some(fog));
        let selected = ModelDepthFade::WorldRaw {
            fog,
            pass: v2k_render::NativeModelFogPass::Near,
        };
        let mut buffer = ModelTreeSubmissionBuffer::default();
        ModelTreeRenderer::new_world(&mut renderer, &cache, &colors, 100.0 / 256.0, None, 0)
            .with_scene_projection_authority(SceneProjectionAuthority::Native(
                NativeScreenProjection::new(
                    [512, 512],
                    [320, 240],
                    [640, 480],
                    ProjectionEffect::None,
                )
                .unwrap(),
            ))
            .with_submission_buffer(&mut buffer)
            .with_painter_view(
                ModelTreeView {
                    position: [0.0; 3],
                    forward: [0.0, 0.0, 1.0],
                },
                ModelTreePainterComposition::Isolated,
            )
            .with_sub_h_presentation(SubHPresentation {
                runtime: &mut runtime,
                retail_tick: 0,
                descriptor: &descriptor,
                model_records: &model.records,
                origin_raw: [0, 0, 512],
                origin_world: [0.0, 0.0, 2.0],
                body_axes_q31: Some(axes),
                native_context: Some((frame, viewport)),
                fallback: None,
                emitter: None,
            })
            .draw_linked(
                0,
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [0.0, 0.0, 2.0],
                8,
                None,
                &AnimVars::default(),
            );
        assert_eq!(runtime, before);
        renderer.set_world_model_fog(Some(v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: -256,
                far_raw: 0,
            },
            color: [0.9; 3],
        }));
        buffer.flush(&mut renderer, None);
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_fade == selected));
        assert_eq!(renderer.billboards[0].depth_fade, selected);
        assert_eq!(
            renderer.bodies.len(),
            3,
            "outside edge, then atomic group face and edge"
        );
        assert_eq!(renderer.billboards.len(), 1);
        assert_eq!(
            renderer.order,
            ["edge", "face", "edge", "billboard"],
            "FIFO group must retain mixed constructor order despite unequal native keys"
        );
        assert_eq!(
            renderer.billboards[0].depth_policy,
            ModelDepthPolicy::Painter,
            "billboard cannot re-enable geometry depth during isolated drain"
        );
        assert!(renderer
            .bodies
            .iter()
            .all(|body| body.depth_policy == ModelDepthPolicy::Painter));
    }

    #[v2k_test_support::retail_test]
    fn native_world_node_fog_keeps_compatibility_and_explicit_policy_gates() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionCompatibilityReason, ProjectionEffect,
            SceneProjectionAuthority,
        };
        let session = native_painter_session();
        let model = session.cache.global_model(302).unwrap();
        let descriptor = session
            .cache
            .global_entity_type(47)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        let viewport = NativeWorldViewport {
            origin_raw: [-15097, -243, 28884],
            axes_q31: [
                [2105482641, 0, 422129991],
                [-61663396, 2124313119, 307562155],
                [-417665851, -313697527, 2082951585],
            ],
            identity: false,
        };
        let axes = [
            [1621229568, -462225408, -1329135616],
            [398852096, 2095841280, -242089984],
            [1349451776, -64225280, 1668481024],
        ];
        let origin = [-17501, -551, 31307];
        let frame = NativeModelFrame::from_actor(viewport, origin, axes);
        assert_eq!(frame.origin_view_raw, [-1881, 111, 2861]);
        let native = SceneProjectionAuthority::Native(
            NativeScreenProjection::new([512, 512], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap(),
        );
        let fog = v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 3072,
                far_raw: 6144,
            },
            color: [0.2, 0.3, 0.4],
        };
        let selected = ModelDepthFade::WorldRaw {
            fog,
            pass: v2k_render::NativeModelFogPass::Near,
        };
        let frontend = ModelDepthFade::FrontendFixedPointLinear {
            near_raw: 500000,
            far_raw: 2000000,
            color: [0.7; 3],
        };
        let view = ModelTreeView {
            position: [-5000.0; 3],
            forward: [0.0, 0.0, 1.0],
        };
        let colors = ModelMaterialCache::new();
        for (name, authority, owns_raw, owns_frame, owns_view, requested, expected) in [
            (
                "native",
                native,
                true,
                true,
                true,
                ModelDepthFade::InheritWorldFog,
                Some(selected),
            ),
            (
                "Free",
                SceneProjectionAuthority::Compatibility(ProjectionCompatibilityReason::Free),
                true,
                true,
                true,
                ModelDepthFade::InheritWorldFog,
                None,
            ),
            (
                "resize",
                SceneProjectionAuthority::Compatibility(
                    ProjectionCompatibilityReason::LogicalViewportAdapter,
                ),
                true,
                true,
                true,
                ModelDepthFade::InheritWorldFog,
                None,
            ),
            (
                "intrinsic compatibility",
                SceneProjectionAuthority::default(),
                true,
                true,
                true,
                ModelDepthFade::InheritWorldFog,
                None,
            ),
            (
                "missing native frame",
                native,
                true,
                false,
                true,
                ModelDepthFade::InheritWorldFog,
                None,
            ),
            (
                "missing raw pair",
                native,
                false,
                true,
                true,
                ModelDepthFade::InheritWorldFog,
                None,
            ),
            (
                "explicit frontend",
                native,
                true,
                true,
                true,
                frontend,
                Some(frontend),
            ),
            (
                "no scene view",
                native,
                true,
                true,
                false,
                ModelDepthFade::InheritWorldFog,
                Some(ModelDepthFade::InheritWorldFog),
            ),
        ] {
            let mut renderer = RecordingRenderer::default();
            if owns_raw {
                renderer.set_world_model_fog(Some(fog));
            } else {
                renderer.set_fog(true, 12.0, 24.0, fog.color);
            }
            let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
            let mut tree = ModelTreeRenderer::new_world(
                &mut renderer,
                &session.cache,
                &colors,
                100.0 / 256.0,
                None,
                952,
            )
            .with_scene_projection_authority(authority)
            .with_depth_fade(requested)
            .with_sub_h_presentation(SubHPresentation {
                runtime: &mut runtime,
                retail_tick: 952,
                descriptor: &descriptor,
                model_records: &model.records,
                origin_raw: origin,
                origin_world: origin.map(|v| f32::from(v) / 256.0),
                body_axes_q31: Some(axes),
                native_context: owns_frame.then_some((frame.clone(), viewport)),
                fallback: None,
                emitter: None,
            });
            if owns_view {
                tree = tree.with_view(view);
            }
            assert_eq!(tree.node_depth_fade(model, [5000.0; 3]), expected, "{name}");
        }
    }

    #[v2k_test_support::retail_test]
    fn native_world_node_fog_actual_stag_child_uses_current_frame_and_local_header() {
        use crate::native_model_frame::{NativeModelFrame, NativeSlotSurface, NativeWorldViewport};
        use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
        use v2k_render::projection::{
            NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
        };
        let session = native_painter_session();
        let parent = session.cache.global_model(267).unwrap();
        let child = session.cache.global_model(268).unwrap();
        assert_eq!(
            child.flags & 0x20,
            0,
            "the actual claw does not bypass its node gate"
        );
        let descriptor = session
            .cache
            .global_entity_type(26)
            .unwrap()
            .sub_h_external_frame_descriptor()
            .unwrap();
        let vars = AnimVars::default();
        let axes = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
        // Controlled source-valid viewport exposes the lateral claw mount in
        // VIEW Z. No authored record, instance or child header is changed.
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[0, 0, i32::MAX], [0, i32::MAX, 0], [i32::MAX, 0, 0]],
            identity: false,
        };
        let frame = NativeModelFrame::from_actor(viewport, [4000, 512, 0], axes);
        let (instance, child_frame) = parent
            .instances
            .iter()
            .filter(|i| i.model_id == 268)
            .filter_map(|i| {
                frame
                    .child_with_surface(
                        parent,
                        i,
                        &vars,
                        NativeSlotSurface::World {
                            viewport,
                            terrain: session.cache.terrain(),
                        },
                    )
                    .map(|f| (i, f))
            })
            .max_by_key(|(_, f)| f.origin_view_raw[2])
            .expect("actual stag has an owned claw instance");
        assert_ne!(
            child_frame.origin_view_raw[2], frame.origin_view_raw[2],
            "authored mount changes the current child VIEW origin"
        );
        // This controlled parent header adds20 to test the local bypass. The
        // actual Stag/claw hierarchy and unsigned radii remain the source input.
        let mut bypass_parent = model_entry(
            267,
            "controlled-stag-header20",
            parent.records.clone(),
            parent.cmd_words.clone(),
        );
        bypass_parent.flags = parent.flags | 0x20;
        bypass_parent.radius = parent.radius;
        let fog = v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: child_frame.origin_view_raw[2].wrapping_add(i32::from(child.radius)),
                far_raw: child_frame.origin_view_raw[2]
                    .wrapping_add(i32::from(child.radius))
                    .wrapping_add(1024),
            },
            color: [0.2, 0.3, 0.4],
        };
        let mut renderer = RecordingRenderer::default();
        renderer.set_world_model_fog(Some(fog));
        let colors = ModelMaterialCache::new();
        let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
        let mut tree = ModelTreeRenderer::new_world(
            &mut renderer,
            &session.cache,
            &colors,
            100.0 / 256.0,
            None,
            1000,
        )
        .with_scene_projection_authority(SceneProjectionAuthority::Native(
            NativeScreenProjection::new([512, 512], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap(),
        ))
        .with_view(ModelTreeView {
            position: [-5000.0; 3],
            forward: [0.0, 0.0, 1.0],
        })
        .with_sub_h_presentation(SubHPresentation {
            runtime: &mut runtime,
            retail_tick: 1000,
            descriptor: &descriptor,
            model_records: &parent.records,
            origin_raw: [4000, 512, 0],
            origin_world: [4000.0 / 256.0, 2.0, 0.0],
            body_axes_q31: Some(axes),
            native_context: Some((frame.clone(), viewport)),
            fallback: None,
            emitter: None,
        });
        assert_eq!(
            tree.node_depth_fade(&bypass_parent, [5000.0; 3]),
            Some(ModelDepthFade::WorldRaw {
                fog,
                pass: v2k_render::NativeModelFogPass::Near
            })
        );
        // Actual linked traversal uses this begin_child/admission/end_child
        // transaction. It installs the authored child frame before admission.
        let previous = tree.external_frame.begin_child(
            parent,
            instance,
            &vars,
            true,
            true,
            session.cache.terrain(),
        );
        assert!(tree.external_frame.native_child_is_owned(&previous));
        assert_eq!(
            tree.external_frame.native_origin_view_raw(),
            Some(child_frame.origin_view_raw)
        );
        assert_eq!(
            tree.node_depth_fade(child, [5000.0; 3]),
            Some(ModelDepthFade::WorldRaw {
                fog,
                pass: v2k_render::NativeModelFogPass::Fog
            }),
            "child near-boundary equality selects fog despite parent20"
        );
        let rejected = v2k_render::WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 0,
                far_raw: child_frame.origin_view_raw[2].wrapping_sub(i32::from(child.radius)),
            },
            ..fog
        };
        tree.world_model_fog = Some(rejected);
        assert_eq!(
            tree.node_depth_fade(child, [5000.0; 3]),
            None,
            "child far-boundary equality rejects despite parent20"
        );
        tree.external_frame.end_child(previous);
        assert_eq!(
            tree.external_frame.native_origin_view_raw(),
            Some(frame.origin_view_raw)
        );
        assert_eq!(
            tree.node_depth_fade(&bypass_parent, [5000.0; 3]),
            Some(ModelDepthFade::WorldRaw {
                fog: rejected,
                pass: v2k_render::NativeModelFogPass::Near
            })
        );
    }
}
