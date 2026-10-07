//! Sub-H and Sub-M callback admission belongs to model presentation, after command and
//! normal selection but before the corner projector's near rejection.

use super::{ModelNodeGeometry, ModelTreeView};
use crate::sub_h_external_frame::{SubHPresentation, SubHSurfaceSampler};
use crate::sub_m_external_frame::{NativeMarkerBoundary, SubMPresentation};
use v2k_formats::models::{ModelFaceCull, ModelFaceVertices, ModelVertexProjection};
use v2k_formats::terrain::TerrainGrid;
use v2k_render::{
    authored_face_plane_visible, ExternalFrameMode, ModelTransform, WorldSurfaceProjection,
};

#[derive(Clone, Copy)]
pub(super) struct ActorExternalSlotPoint {
    pub draw_world: [f32; 3],
    pub native_world_raw: [i32; 3],
}

pub(super) enum ModelTreeExternalFrame<'a> {
    Fixed(ExternalFrameMode),
    /// A fixed callback frame whose nodes keep their source-owned VIEW
    /// frames, as `FUN_00427090` submits static terrain objects.
    NativeFixed {
        mode: ExternalFrameMode,
        native_context: Option<(
            crate::native_model_frame::NativeModelFrame,
            crate::native_model_frame::NativeWorldViewport,
        )>,
    },
    SubH(SubHPresentation<'a>),
    SubM(SubMPresentation<'a>),
    Emitter(crate::actor_emitter_external_frame::ActorEmitterModelPresentation<'a>),
}

pub(super) enum ModelTreeNativeParentFrame {
    None,
    NativeFixed(
        Option<(
            crate::native_model_frame::NativeModelFrame,
            crate::native_model_frame::NativeWorldViewport,
        )>,
    ),
    SubM(Option<crate::native_model_frame::NativeModelFrame>),
    SubH(
        Option<(
            crate::native_model_frame::NativeModelFrame,
            crate::native_model_frame::NativeWorldViewport,
        )>,
    ),
    Emitter(
        crate::native_model_frame::NativeModelFrame,
        bool,
        Option<[Option<[i32; 3]>; 2]>,
    ),
}

impl ModelTreeExternalFrame<'_> {
    pub(super) fn needs_live_materialization(&self) -> bool {
        matches!(
            self,
            Self::SubH(_)
                | Self::SubM(_)
                | Self::Emitter(_)
                | Self::NativeFixed {
                    native_context: Some(_),
                    ..
                }
        )
    }

    pub(super) fn owns_external_vertices(&self) -> bool {
        matches!(self, Self::SubH(_) | Self::Emitter(_))
    }

    /// Current node is installed at root construction/begin_child, before
    /// prepare_node_surface can execute any model-slot H/E dependencies.
    pub(super) fn native_origin_view_raw(&self) -> Option<[i32; 3]> {
        match self {
            Self::SubH(presentation) => presentation
                .native_context
                .as_ref()
                .map(|(frame, _)| frame.origin_view_raw),
            Self::SubM(presentation) => presentation
                .native_frame
                .as_ref()
                .map(|frame| frame.origin_view_raw),
            Self::Emitter(presentation) => presentation
                .emitter
                .current_node_owned
                .then_some(presentation.emitter.frame.origin_view_raw),
            Self::NativeFixed { native_context, .. } => native_context
                .as_ref()
                .map(|(frame, _)| frame.origin_view_raw),
            Self::Fixed(_) => None,
        }
    }

    pub(super) fn native_context(
        &self,
    ) -> Option<(
        crate::native_model_frame::NativeModelFrame,
        crate::native_model_frame::NativeWorldViewport,
    )> {
        match self {
            Self::SubH(presentation) => presentation.native_context.clone(),
            Self::SubM(presentation) => presentation.native_context(),
            Self::NativeFixed { native_context, .. } => native_context.clone(),
            Self::Emitter(presentation) => presentation.emitter.current_node_owned.then(|| {
                (
                    presentation.emitter.frame.clone(),
                    presentation.emitter.viewport,
                )
            }),
            _ => None,
        }
    }

    pub(super) fn resolve_parameter_world(
        &mut self,
        parameters: [i16; 3],
        surface: Option<WorldSurfaceProjection<'_>>,
    ) -> Option<ActorExternalSlotPoint> {
        if let Self::Emitter(presentation) = self {
            let point = presentation.emitter.resolve(0, parameters).ok()??;
            return Some(ActorExternalSlotPoint {
                draw_world: presentation.draw_world(point.position_raw),
                native_world_raw: point.position_raw,
            });
        }
        let Self::SubH(presentation) = self else {
            return None;
        };
        let selector = i32::from(parameters[0]);
        let Some(binding) = crate::sub_h_external_frame::selector_binding(
            selector,
            presentation.runtime.records().len(),
        ) else {
            let (draw_world, native_world_raw) = presentation.resolve_post_h_point(parameters)?;
            return Some(ActorExternalSlotPoint {
                draw_world,
                native_world_raw,
            });
        };
        let sampler = surface.map(|surface| SubHSurfaceSampler {
            terrain: surface.terrain(),
            retail_tick: presentation.retail_tick,
            waves_enabled: surface.waves_enabled(),
            policy: presentation.runtime.surface_policy(),
        });
        let draw_world = presentation
            .resolve([selector], |x, z| {
                sampler
                    .as_ref()
                    .map_or(0, |surface| surface.sample_raw(x, z))
            })?
            .get(selector as usize)
            .copied()
            .flatten()?;
        let record = presentation.runtime.records()[binding.record_index];
        let raw = match binding.endpoint {
            crate::sub_h_external_frame::SubHEndpoint::Primary => record.primary_raw,
            crate::sub_h_external_frame::SubHEndpoint::Secondary => record.secondary_raw,
        };
        Some(ActorExternalSlotPoint {
            draw_world,
            native_world_raw: raw.map(i32::from),
        })
    }

    pub(super) fn prepare_node_surface(
        &mut self,
        model: &v2k_formats::models::ModelEntry,
        vars: &v2k_formats::models::AnimVars,
        linked: Option<&v2k_formats::models::LinkedModelSlots>,
        world: bool,
        terrain: Option<&TerrainGrid>,
    ) {
        if let Self::SubM(presentation) = self {
            let surface = presentation.native_slot_surface(world, terrain);
            presentation.prepare_node_with_surface(model, vars, linked, surface);
        } else if let Self::SubH(presentation) = self {
            presentation.prepare_post_h_node(model, vars);
        } else if let Self::Emitter(presentation) = self {
            let frame = presentation
                .emitter
                .current_node_owned
                .then(|| presentation.emitter.frame.clone());
            presentation
                .emitter
                .prepare_native_node(model, frame.as_ref());
        }
    }

    pub(super) fn native_sub_m_boundaries(&self) -> &[NativeMarkerBoundary] {
        if let Self::SubM(presentation) = self {
            presentation.native_boundaries()
        } else {
            &[]
        }
    }

    pub(super) fn native_view_selection(&self) -> Option<v2k_formats::models::ModelViewSelection> {
        match self {
            Self::SubM(presentation) => presentation.native_view_selection(),
            Self::SubH(presentation) => presentation
                .native_context
                .as_ref()
                .map(|(frame, _)| frame.view_selection()),
            Self::Emitter(presentation) => presentation
                .emitter
                .current_node_owned
                .then(|| presentation.emitter.frame.view_selection()),
            Self::NativeFixed { native_context, .. } => native_context
                .as_ref()
                .map(|(frame, _)| frame.view_selection()),
            Self::Fixed(_) => None,
        }
    }

    pub(super) fn begin_child(
        &mut self,
        model: &v2k_formats::models::ModelEntry,
        instance: &v2k_formats::models::ModelInstance,
        vars: &v2k_formats::models::AnimVars,
        authored: bool,
        world: bool,
        terrain: Option<&TerrainGrid>,
    ) -> ModelTreeNativeParentFrame {
        if let Self::SubM(presentation) = self {
            let surface = presentation.native_slot_surface(world, terrain);
            ModelTreeNativeParentFrame::SubM(
                presentation.begin_child(model, instance, vars, authored, surface),
            )
        } else if let Self::SubH(presentation) = self {
            let previous = presentation.native_context.take();
            presentation.native_context = previous.as_ref().and_then(|(frame, viewport)| {
                let surface = if world {
                    crate::native_model_frame::NativeSlotSurface::World {
                        viewport: *viewport,
                        terrain,
                    }
                } else {
                    crate::native_model_frame::NativeSlotSurface::Intrinsic
                };
                authored
                    .then(|| frame.child_with_surface(model, instance, vars, surface))
                    .flatten()
                    .map(|child| (child, *viewport))
            });
            ModelTreeNativeParentFrame::SubH(previous)
        } else if let Self::NativeFixed { native_context, .. } = self {
            // 67410/676C0 derive each reached child's frame from its
            // parent's; a child without one keeps the float path.
            let previous = native_context.take();
            *native_context = previous.as_ref().and_then(|(frame, viewport)| {
                let surface = if world {
                    crate::native_model_frame::NativeSlotSurface::World {
                        viewport: *viewport,
                        terrain,
                    }
                } else {
                    crate::native_model_frame::NativeSlotSurface::Intrinsic
                };
                authored
                    .then(|| frame.child_with_surface(model, instance, vars, surface))
                    .flatten()
                    .map(|child| (child, *viewport))
            });
            ModelTreeNativeParentFrame::NativeFixed(previous)
        } else if let Self::Emitter(presentation) = self {
            let emitter = &mut presentation.emitter;
            let previous = emitter.frame.clone();
            let previous_owned = emitter.current_node_owned;
            let previous_sources = emitter.current_source_points_view;
            let surface = if world {
                crate::native_model_frame::NativeSlotSurface::World {
                    viewport: emitter.viewport,
                    terrain,
                }
            } else {
                crate::native_model_frame::NativeSlotSurface::Intrinsic
            };
            // Transport the current authored child frame before resolving its
            // plain emitter source slots. Missing native custody suppresses
            // a reached selector rather than borrowing the parent's cache.
            if let Some(child) = (authored && previous_owned)
                .then(|| previous.child_with_surface(model, instance, vars, surface))
                .flatten()
            {
                emitter.frame = child;
                emitter.current_node_owned = true;
            } else {
                emitter.current_node_owned = false;
            }
            ModelTreeNativeParentFrame::Emitter(previous, previous_owned, previous_sources)
        } else {
            ModelTreeNativeParentFrame::None
        }
    }

    pub(super) fn native_child_is_owned(&self, previous: &ModelTreeNativeParentFrame) -> bool {
        if let (Self::Emitter(presentation), ModelTreeNativeParentFrame::Emitter(_, _, _)) =
            (self, previous)
        {
            // Ordinary geometry can retain its explicit float presentation
            // policy. An executed E selector still fails closed without a
            // native node frame; it cannot borrow its parent's slot cache.
            let _ = presentation;
            return true;
        }
        !matches!((self, previous), (Self::SubH(presentation), ModelTreeNativeParentFrame::SubH(Some(_))) if presentation.native_context.is_none())
    }

    pub(super) fn end_child(&mut self, previous: ModelTreeNativeParentFrame) {
        match (self, previous) {
            (Self::SubM(presentation), ModelTreeNativeParentFrame::SubM(previous)) => {
                presentation.end_child(previous)
            }
            (Self::SubH(presentation), ModelTreeNativeParentFrame::SubH(previous)) => {
                presentation.native_context = previous
            }
            (
                Self::NativeFixed { native_context, .. },
                ModelTreeNativeParentFrame::NativeFixed(previous),
            ) => *native_context = previous,
            (
                Self::Emitter(presentation),
                ModelTreeNativeParentFrame::Emitter(previous, owned, sources),
            ) => {
                presentation.emitter.frame = previous;
                presentation.emitter.current_node_owned = owned;
                presentation.emitter.current_source_points_view = sources;
            }
            _ => {}
        }
    }

    pub(super) fn resolve(
        &mut self,
        geometry: &ModelNodeGeometry<'_>,
        transform: ModelTransform,
        view: Option<ModelTreeView>,
        surface: Option<WorldSurfaceProjection<'_>>,
    ) -> ExternalFrameMode {
        match self {
            Self::Fixed(frame) => *frame,
            Self::NativeFixed { mode, .. } => *mode,
            // Live E selectors were resolved and stamped by the command-time
            // vertex owner. Deferred backend submission cannot replay them.
            Self::Emitter(_) => ExternalFrameMode::Raw,
            Self::SubM(presentation) => {
                let Some(view) = view else {
                    return ExternalFrameMode::Raw;
                };
                let selectors = if presentation.native_requested {
                    let Some(frame) = presentation.native_frame.as_ref() else {
                        // Diagnose only an executed selector request; unrelated
                        // model children do not create a marker obligation.
                        if requested_selectors(geometry, transform, view).contains(&0) {
                            presentation.record_native_boundary(None);
                        }
                        return ExternalFrameMode::Raw;
                    };
                    match requested_selectors_native(geometry, transform, view, Some(frame)) {
                        Some(selectors) => selectors,
                        None => {
                            presentation.record_native_boundary(Some(
                                NativeMarkerBoundary::UnownedFacePlane,
                            ));
                            return ExternalFrameMode::Raw;
                        }
                    }
                } else {
                    requested_selectors(geometry, transform, view)
                };
                if !selectors.contains(&0) {
                    return ExternalFrameMode::Raw;
                }
                let world = if presentation.native_requested {
                    let Some(native) = presentation.marker_native_world else {
                        presentation.record_native_boundary(None);
                        return ExternalFrameMode::Raw;
                    };
                    presentation.publish_native(native)
                } else {
                    let Some(marker_local) = presentation.marker_local else {
                        return ExternalFrameMode::Raw;
                    };
                    let world = super::instance_world_position(
                        transform.orientation,
                        transform.position,
                        marker_local,
                        transform.scale,
                    );
                    presentation.publish(world);
                    world
                };
                let mut points = [None; 16];
                points[0] = Some(world);
                ExternalFrameMode::SelectorWorldPoints(points)
            }
            Self::SubH(presentation) => {
                // A live draw needs its installed camera to reproduce the
                // normal test. An intrinsic inspection is not a live visit.
                let Some(view) = view else {
                    return ExternalFrameMode::Raw;
                };
                let sampler = surface.map(|surface| SubHSurfaceSampler {
                    terrain: surface.terrain(),
                    retail_tick: presentation.retail_tick,
                    waves_enabled: surface.waves_enabled(),
                    policy: presentation.runtime.surface_policy(),
                });
                presentation
                    .resolve(requested_selectors(geometry, transform, view), |x, z| {
                        sampler
                            .as_ref()
                            .map_or(0, |surface| surface.sample_raw(x, z))
                    })
                    .map(ExternalFrameMode::SelectorWorldPoints)
                    .unwrap_or(ExternalFrameMode::Raw)
            }
        }
    }
}

fn requested_selectors(
    geometry: &ModelNodeGeometry<'_>,
    transform: ModelTransform,
    view: ModelTreeView,
) -> Vec<i32> {
    requested_selectors_native(geometry, transform, view, None).unwrap_or_default()
}

fn requested_selectors_native(
    geometry: &ModelNodeGeometry<'_>,
    transform: ModelTransform,
    view: ModelTreeView,
    native: Option<&crate::native_model_frame::NativeModelFrame>,
) -> Option<Vec<i32>> {
    let mut requested = vec![false; geometry.vertices.len()];
    let mut selectors = Vec::new();
    for (index, triangle) in geometry.triangles.iter().enumerate() {
        // 45A4E0 checks 46D3F0 before invoking any corner callback. Near
        // admission cannot precede D360: its result is the point projected.
        if let Some(ModelFaceCull::Plane(plane)) = geometry.face_cull.get(index) {
            let visible = if let Some(frame) = native {
                frame.face_plane_visible(*plane)?
            } else {
                authored_face_plane_visible(
                    *plane,
                    transform.orientation,
                    transform.position,
                    transform.scale,
                    view.position,
                )
            };
            if !visible {
                continue;
            }
        }
        let corners: &[u16] = match geometry.face_vertices.get(index) {
            Some(ModelFaceVertices::Triangle(corners)) => corners,
            Some(ModelFaceVertices::Quad(corners)) => corners,
            None => triangle,
        };
        for &vertex in corners {
            request_vertex(geometry, vertex, &mut requested, &mut selectors);
        }
    }
    for vertex in geometry
        .edges
        .iter()
        .flat_map(|edge| edge.vertices)
        .chain(geometry.billboards.iter().map(|billboard| billboard.vertex))
    {
        request_vertex(geometry, vertex, &mut requested, &mut selectors);
    }
    Some(selectors)
}

fn request_vertex(
    geometry: &ModelNodeGeometry<'_>,
    vertex: u16,
    requested: &mut [bool],
    selectors: &mut Vec<i32>,
) {
    let index = usize::from(vertex);
    let Some(seen) = requested.get_mut(index) else {
        return;
    };
    if *seen {
        return;
    }
    *seen = true;
    if geometry.vertex_type_flags.get(index) == Some(&14) {
        selectors.push(geometry.vertices[index][0] as i32);
    }
    if let Some(ModelVertexProjection::ScreenMidpoint(sources)) =
        geometry.vertex_projection.get(index)
    {
        for &source in sources {
            request_vertex(geometry, source, requested, selectors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sub_h_external_frame::{SubHPhaseRecord, SubHRuntimeState, SubHSurfacePolicy};
    use v2k_formats::collision::{SubHExternalFrameDescriptor, SubHExternalFrameRecord};
    use v2k_formats::models::{
        AnimVars, ModelEntry, ModelFaceCullPlane, ModelMaterializationContext,
    };

    fn transform() -> ModelTransform {
        ModelTransform {
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 100.0 / 256.0,
        }
    }

    #[test]
    fn sub_h_presentation_applies_retained_surface_policy_and_draw_tick_only_to_requested_feet() {
        // Two valid limbs, only selector0 submitted. The long two-edge chain
        // keeps both the controlled sea and terrain within reach, isolating
        // the production surface adapter from D360's separate reach clamp.
        let model = ModelEntry {
            vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vertex_type_flags: vec![14, 14, 0, 0],
            triangles: vec![[0, 2, 3]],
            face_cull: vec![ModelFaceCull::AlwaysVisible],
            records: vec![[0, 0, 0, 0], [0, 0, 5000, 0], [0, 0, 100, 0]],
            ..ModelEntry::default()
        };
        let geometry = ModelNodeGeometry::from_static(&model);
        let descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: vec![
                SubHExternalFrameRecord {
                    resolver_flags_raw: 0,
                    phase_rate_raw: 1,
                    vertex_refs: [0, 2, 4],
                    axis_mode_raw: 0,
                    dependencies: [0; 4],
                };
                2
            ],
        };
        let terrain = TerrainGrid {
            header: [4096 << 8, 0, 0, 0, 0],
            cells: vec![
                v2k_formats::terrain::TerrainCell {
                    height: (-128_i8) as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                256 * 256
            ],
        };
        let view = ModelTreeView {
            position: [0.0, 0.0, -1.0],
            forward: [0.0, 0.0, 1.0],
        };
        let draw = |runtime: &mut SubHRuntimeState, retail_tick| {
            let mut external_frame = ModelTreeExternalFrame::SubH(SubHPresentation {
                runtime,
                descriptor: &descriptor,
                model_records: &model.records,
                retail_tick,
                origin_raw: [0; 3],
                origin_world: [0.0; 3],
                body_axes_q31: Some([
                    [0x7FFF_0000, 0, 0],
                    [0, 0x7FFF_0000, 0],
                    [0, 0, 0x7FFF_0000],
                ]),
                native_context: None,
                fallback: None,
                emitter: None,
            });
            let ExternalFrameMode::SelectorWorldPoints(points) = external_frame.resolve(
                &geometry,
                transform(),
                Some(view),
                Some(WorldSurfaceProjection::new(&terrain, 0)),
            ) else {
                panic!("the valid submitted Sub-H selector must resolve")
            };
            assert!(points[1..].iter().all(Option::is_none));
            points[0].unwrap()
        };
        for (policy, heights) in [
            (SubHSurfacePolicy::Terrain, [-4096, -4096]),
            (SubHSurfacePolicy::TerrainAndWater, [4096, 4114]),
        ] {
            let mut runtime = SubHRuntimeState::new(2).unwrap();
            runtime.set_surface_policy(policy);
            assert_eq!(draw(&mut runtime, 1), [0.0, 99.0 / 256.0, 0.0]);
            assert_eq!(runtime.records()[1], SubHPhaseRecord::default());
            runtime.update(&descriptor, 1000).unwrap();
            for (tick, expected_y) in heights.into_iter().enumerate() {
                let mut frame_runtime = runtime.clone();
                assert_eq!(
                    draw(&mut frame_runtime, tick as u32),
                    [0.0, expected_y as f32 / 256.0, 0.0],
                    "retained policy {policy:?}, presentation tick {tick}"
                );
                assert_eq!(frame_runtime.surface_policy(), policy);
                assert_eq!(frame_runtime.records()[1], SubHPhaseRecord::default());
                let cached = frame_runtime.clone();
                assert_eq!(
                    draw(&mut frame_runtime, tick as u32 + 100),
                    [0.0, expected_y as f32 / 256.0, 0.0],
                    "a later draw clock must not invalidate a warm primary cache"
                );
                assert_eq!(frame_runtime, cached);
            }
        }
    }

    #[test]
    fn fixed_external_frames_keep_legacy_submission_without_a_live_surface_context() {
        let model = ModelEntry::default();
        let geometry = ModelNodeGeometry::from_static(&model);
        let mut points = [None; 16];
        points[4] = Some([1.0, 2.0, 3.0]);
        for frame in [
            ExternalFrameMode::Raw,
            ExternalFrameMode::WorldPoint([5.0, 6.0, 7.0]),
            ExternalFrameMode::SelectorWorldPoints(points),
        ] {
            assert_eq!(
                ModelTreeExternalFrame::Fixed(frame).resolve(&geometry, transform(), None, None),
                frame
            );
        }
    }

    #[test]
    fn sub_h_requests_keep_original_quad_corners_and_omit_backfaces_and_unused_vertices() {
        let model = ModelEntry {
            vertices: (0..5).map(|index| [f64::from(index), 0.0, 0.0]).collect(),
            vertex_type_flags: vec![14; 5],
            triangles: vec![[0, 1, 0], [2, 2, 2]],
            face_vertices: vec![
                ModelFaceVertices::Quad([0, 1, 0, 3]),
                ModelFaceVertices::Triangle([2, 2, 2]),
            ],
            face_cull: vec![
                ModelFaceCull::AlwaysVisible,
                ModelFaceCull::Plane(ModelFaceCullPlane {
                    anchor_raw: [0.0; 3],
                    native_anchor_raw: None,
                    normal_raw: [0, 0, 1],
                }),
            ],
            ..ModelEntry::default()
        };
        let view = ModelTreeView {
            position: [0.0, 0.0, -1.0],
            forward: [0.0, 0.0, 1.0],
        };
        assert_eq!(
            requested_selectors(&ModelNodeGeometry::from_static(&model), transform(), view),
            [0, 1, 3]
        );
        let front_view = ModelTreeView {
            position: [0.0, 0.0, 1.0],
            ..view
        };
        assert_eq!(
            requested_selectors(
                &ModelNodeGeometry::from_static(&model),
                transform(),
                front_view
            ),
            [0, 1, 3, 2]
        );
    }

    #[test]
    fn sub_h_requests_include_screen_midpoint_projectors_without_selecting_unrelated_supports() {
        let model = ModelEntry {
            vertices: vec![[0.0; 3], [4.0, 0.0, 0.0], [7.0, 0.0, 0.0], [9.0, 0.0, 0.0]],
            vertex_type_flags: vec![1, 14, 14, 14],
            vertex_projection: vec![ModelVertexProjection::ScreenMidpoint([1, 2])],
            vertex_clip: Vec::new(),
            vertex_surface_origin: Vec::new(),
            triangles: vec![[0; 3]],
            ..ModelEntry::default()
        };
        let view = ModelTreeView {
            position: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
        };
        assert_eq!(
            requested_selectors(&ModelNodeGeometry::from_static(&model), transform(), view),
            [4, 7]
        );
    }

    #[test]
    fn sub_h_requests_follow_executed_view_commands() {
        let model = ModelEntry {
            records: vec![
                [0, 0, 0, 0],
                [14, 2, 0, 0],
                [14, 5, 0, 0],
                [0, 0, 10, 0],
                [0, 0, 0, 10],
            ],
            normal_pool: vec![[0, 0, 0, 1]],
            // Each guarded block contains one normal-0 triangle, so only
            // the command's view predicate decides whether its selector runs.
            cmd_words: vec![
                0x0B, 8, 2, 0x03, 0, 0, 2, 6, 8, 0x0C, 8, 2, 0x03, 0, 0, 4, 6, 8, 0,
            ],
            ..ModelEntry::default()
        };
        let vars = AnimVars::default();
        for (camera_z, expected) in [(1.0, 2), (-1.0, 5)] {
            let view = ModelTreeView {
                position: [0.0, 0.0, camera_z],
                forward: [0.0, 0.0, -camera_z],
            };
            let transform = transform();
            let materialized = model.materialize_with_context(ModelMaterializationContext {
                view_selection: view.selection(
                    transform.orientation,
                    transform.position,
                    transform.scale,
                ),
                ..ModelMaterializationContext::intrinsic(&vars, None)
            });
            assert_eq!(
                requested_selectors(
                    &ModelNodeGeometry::from_materialized(&materialized, 0, 0),
                    transform,
                    view,
                ),
                [expected]
            );
        }
    }
    #[test]
    fn sub_m_marker_returns_geometry_and_only_writes_after_face_admission() {
        use crate::sub_m_external_frame::{SubMPresentation, SubMProductMarkerWrite};
        use v2k_formats::collision::StatusComponentDescriptor;
        use v2k_formats::models::ModelFaceCullPlane;
        let model = ModelEntry {
            vertices: vec![[0.0; 3], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]],
            vertex_type_flags: vec![14, 0, 0],
            triangles: vec![[0, 1, 2]],
            face_cull: vec![ModelFaceCull::Plane(ModelFaceCullPlane {
                anchor_raw: [0.0; 3],
                native_anchor_raw: None,
                normal_raw: [0, 0, 1],
            })],
            records: vec![[14, 0, 0, 0], [0, 0, 0, 0], [0, 0, 128, 0]],
            ..ModelEntry::default()
        };
        let mut descriptor = StatusComponentDescriptor {
            raw_word_at_0x00: 4,
            variable_bindings: [0; 6],
            raw_tail: [0; 10],
        };
        for (axis, value) in [31_i16, -17, 23].into_iter().enumerate() {
            descriptor.raw_tail[4 + axis * 2..6 + axis * 2].copy_from_slice(&value.to_le_bytes());
        }
        let geometry = ModelNodeGeometry::from_static(&model);
        let mut write = None;
        let mut frame = ModelTreeExternalFrame::SubM(SubMPresentation::new(
            descriptor,
            42,
            [265.0, 2.0, 4.0],
            [9.0, 2.0, 4.0],
            &mut write,
        ));
        frame.prepare_node_surface(&model, &AnimVars::default(), None, false, None);
        let transform = ModelTransform {
            position: [9.0, 2.0, 4.0],
            ..transform()
        };
        assert_eq!(
            frame.resolve(
                &geometry,
                transform,
                Some(ModelTreeView {
                    position: [9.0, 2.0, 3.0],
                    forward: [0.0, 0.0, 1.0],
                }),
                None
            ),
            ExternalFrameMode::Raw
        );
        let result = frame.resolve(
            &geometry,
            transform,
            Some(ModelTreeView {
                position: [9.0, 2.0, 5.0],
                forward: [0.0, 0.0, -1.0],
            }),
            None,
        );
        drop(frame);
        let ExternalFrameMode::SelectorWorldPoints(points) = result else {
            panic!("Sub-M callback");
        };
        assert_eq!(points[0], Some([9.0, 2.5, 4.0]));
        assert!(points[1..].iter().all(Option::is_none));
        assert_eq!(
            write,
            Some(SubMProductMarkerWrite {
                product_id: 42,
                position_raw: [(265_i32 * 256 + 31) as i16, 640 - 17, 1024 + 23],
            })
        );
    }

    #[test]
    fn native_sub_m_rejects_unowned_body_and_generated_slot_without_float_write() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        use crate::sub_m_external_frame::{NativeMarkerBoundary, SubMPresentation};
        use v2k_formats::collision::StatusComponentDescriptor;
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let model = ModelEntry {
            vertices: vec![[0.0; 3]],
            vertex_type_flags: vec![14],
            triangles: vec![[0; 3]],
            records: vec![[14, 0, 0, 0], [0, 0, 128, 0]],
            ..ModelEntry::default()
        };
        let descriptor = StatusComponentDescriptor {
            raw_word_at_0x00: 2,
            variable_bindings: [0; 6],
            raw_tail: [0; 10],
        };
        let mut write = None;
        let mut presentation =
            SubMPresentation::new(descriptor, 42, [0.0; 3], [0.0; 3], &mut write)
                .with_native_viewport(viewport);
        presentation.prepare_node_with_surface(
            &model,
            &AnimVars::default(),
            None,
            crate::native_model_frame::NativeSlotSurface::Intrinsic,
        );
        assert!(
            presentation.marker_local.is_some(),
            "the float inspector can resolve this marker"
        );
        let mut frame = ModelTreeExternalFrame::SubM(presentation);
        let view = Some(ModelTreeView {
            position: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
        });
        assert_eq!(
            frame.resolve(
                &ModelNodeGeometry::from_static(&model),
                transform(),
                view,
                None
            ),
            ExternalFrameMode::Raw
        );
        assert_eq!(
            frame.native_sub_m_boundaries(),
            &[NativeMarkerBoundary::UnownedActorBody]
        );
        drop(frame);
        assert!(write.is_none());

        let descriptor = StatusComponentDescriptor {
            raw_word_at_0x00: 0,
            ..descriptor
        };
        let mut presentation =
            SubMPresentation::new(descriptor, 42, [0.0; 3], [0.0; 3], &mut write)
                .with_native_viewport(viewport);
        presentation.native_frame = Some(NativeModelFrame::from_actor(
            viewport,
            [0; 3],
            viewport.axes_q31,
        ));
        presentation.prepare_node_with_surface(
            &model,
            &AnimVars::default(),
            None,
            crate::native_model_frame::NativeSlotSurface::Intrinsic,
        );
        let mut frame = ModelTreeExternalFrame::SubM(presentation);
        assert_eq!(
            frame.resolve(
                &ModelNodeGeometry::from_static(&model),
                transform(),
                view,
                None
            ),
            ExternalFrameMode::Raw
        );
        assert_eq!(
            frame.native_sub_m_boundaries(),
            &[NativeMarkerBoundary::UnsupportedMarkerSlot {
                model_id: model.index,
                slot: 0,
                kind: Some(14)
            }]
        );
        drop(frame);
        assert!(write.is_none());
    }

    #[test]
    fn native_sub_m_normal_gate_consumes_raw_anchor_and_strict_wrapping_dot() {
        use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let mut native = NativeModelFrame::from_actor(viewport, [0; 3], viewport.axes_q31);
        native.identity = true;
        native.origin_view_raw = [1, 0, 0];
        let plane = ModelFaceCullPlane {
            anchor_raw: [-999.0, 0.0, 0.0],
            native_anchor_raw: Some([0; 3]),
            normal_raw: [1, 0, 0],
        };
        assert_eq!(
            native.face_plane_visible(plane),
            Some(false),
            "generated float anchor cannot admit a native face"
        );
        native.origin_view_raw = [0; 3];
        assert_eq!(
            native.face_plane_visible(plane),
            Some(false),
            "native equality is rejected"
        );
        native.origin_view_raw = [-1, 0, 0];
        assert_eq!(native.face_plane_visible(plane), Some(true));
        native.origin_view_raw = [i32::MAX, 0, 0];
        assert_eq!(
            native.face_plane_visible(ModelFaceCullPlane {
                normal_raw: [2, 0, 0],
                ..plane
            }),
            Some(true),
            "native IMUL narrows before the sign test"
        );
        assert_eq!(
            native.face_plane_visible(ModelFaceCullPlane {
                native_anchor_raw: None,
                ..plane
            }),
            None
        );
    }
}
