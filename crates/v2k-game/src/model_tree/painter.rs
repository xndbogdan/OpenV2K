//! Authored model painter queues. Geometry remains in raw model-local space;
//! only queue keys pass through the active camera before atomic submission.

use std::ops::Range;

use v2k_formats::models::{ModelPainterDepthKey, ModelPainterOp, ModelPainterSorting};

use super::*;

/// How an ordered hierarchy interacts with the enclosing scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelTreePainterComposition {
    /// An isolated overlay, such as Klaus's mouth handoff, has no scene depth.
    Isolated,
    /// One opaque/keyed outer group is drawn before other scene submissions.
    /// Its covered pixels retain the authored group key, not descendant Z.
    /// This is not a general compositor for translucent or multiple groups.
    OpaqueSceneGroup,
}

struct PainterNode {
    model_flags: u8,
    radius_raw: u16,
    geometry: MaterializedModel,
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    surface_resolution: ModelSurfaceResolution,
    depth_fade: Option<ModelDepthFade>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PainterFace {
    node: usize,
    triangles: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PainterPrimitive {
    Face(PainterFace),
    Edge { node: usize, edge: usize },
    Billboard { node: usize, billboard: usize },
}

enum PainterItem {
    Primitive(PainterPrimitive),
    Group(PainterGroup),
}

struct PainterGroup {
    key: i32,
    sorting: ModelPainterSorting,
    items: Vec<(i32, PainterItem)>,
}

impl PainterGroup {
    fn drain(mut self, output: &mut Vec<PainterPrimitive>) {
        if self.sorting == ModelPainterSorting::Sorted {
            // 494930 keeps the earlier allocated record on equal keys.
            self.items.sort_by(|left, right| right.0.cmp(&left.0));
        }
        for (_, item) in self.items {
            match item {
                PainterItem::Primitive(primitive) => output.push(primitive),
                PainterItem::Group(group) => group.drain(output),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PainterError {
    MissingModel,
    InvalidInstance,
    InvalidFaceRange,
    InvalidEdge,
    InvalidBillboard,
    NativeProgram(v2k_formats::models::NativePainterProgramBoundary),
    MissingNativeProjection(v2k_render::projection::ProjectionAuthorityMissing),
    UnresolvedDepth,
    UnbalancedGroup,
    UnsupportedPrimitive,
    ExpectedSingleOuterGroup,
    TranslucentSceneGroup,
    MissingMaterial,
}

struct PainterTraversal<'a> {
    cache: &'a ResourceCache,
    scale: f32,
    view: ModelTreeView,
    far_plane: Option<f32>,
    nodes: Vec<PainterNode>,
    groups: Vec<PainterGroup>,
    prepared_root: Option<(MaterializedModel, ModelSurfaceResolution, ModelDepthFade)>,
}

impl ModelTreeView {
    fn point_depth(
        self,
        point: [f64; 3],
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        scale: f32,
    ) -> Option<i32> {
        // ModelTree's raw-to-world conversion is /100. Keep the existing
        // floating pose convention, truncate once to signed raw view depth,
        // then use wrapping integer arithmetic for source queue offsets.
        // This does not claim retail's separate Q31 transform is bit exact.
        let mut depth = 0.0;
        for axis in 0..3 {
            let rotated: f64 = (0..3)
                .map(|component| f64::from(orientation[axis][component]) * point[component])
                .sum();
            let relative = rotated * f64::from(scale)
                + (f64::from(position[axis]) - f64::from(self.position[axis])) * 100.0;
            depth += relative * f64::from(self.forward[axis]);
        }
        depth.is_finite().then(|| depth.trunc() as i64 as i32)
    }

    fn depth_key(
        self,
        key: &ModelPainterDepthKey,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        scale: f32,
    ) -> Result<i32, PainterError> {
        let point = |point| {
            self.point_depth(point, orientation, position, scale)
                .ok_or(PainterError::UnresolvedDepth)
        };
        match key {
            ModelPainterDepthKey::Native(depth) => Ok(*depth),
            ModelPainterDepthKey::Vertex {
                position_raw,
                offset_raw,
            } => Ok(point(*position_raw)?.wrapping_add(*offset_raw)),
            ModelPainterDepthKey::Fixed(depth) => Ok(*depth),
            ModelPainterDepthKey::Minimum(points) => points
                .iter()
                .try_fold(i32::MAX, |depth, p| Ok(depth.min(point(*p)?))),
            ModelPainterDepthKey::Maximum(points) => points
                .iter()
                .try_fold(-i32::MAX, |depth, p| Ok(depth.max(point(*p)?))),
            ModelPainterDepthKey::Unresolved => Err(PainterError::UnresolvedDepth),
        }
    }
}

impl<'a> PainterTraversal<'a> {
    fn new(
        cache: &'a ResourceCache,
        scale: f32,
        view: ModelTreeView,
        far_plane: Option<f32>,
    ) -> Self {
        Self {
            cache,
            scale,
            view,
            far_plane,
            nodes: Vec::new(),
            prepared_root: None,
            groups: vec![PainterGroup {
                key: 0,
                sorting: ModelPainterSorting::Sorted,
                items: Vec::new(),
            }],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn visit(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
        child_transform: Option<&dyn ModelTreeChildTransform>,
        root_link_policy: ModelTreeRootLinkPolicy,
    ) -> Result<(), PainterError> {
        let model = self
            .cache
            .global_model(model_id)
            .ok_or(PainterError::MissingModel)?;
        if self.prepared_root.is_none()
            && !self.view.admits_model(
                model.flags,
                model.radius,
                position,
                self.scale,
                self.far_plane,
            )
        {
            return Ok(());
        }
        let prepared = self.prepared_root.take();
        let depth_fade = prepared.as_ref().map(|(_, _, fade)| *fade);
        let (geometry, surface_resolution) = prepared
            .map(|(geometry, resolution, _)| (geometry, resolution))
            .unwrap_or_else(|| {
                (
                    model.materialize_with_context(ModelMaterializationContext {
                        view_selection: self.view.selection(orientation, position, self.scale),
                        ..ModelMaterializationContext::intrinsic(vars, linked)
                    }),
                    ModelSurfaceResolution::Intrinsic,
                )
            });
        let program = geometry.painter_program.clone();
        let node = self.nodes.len();
        self.nodes.push(PainterNode {
            model_flags: model.flags,
            radius_raw: model.radius,
            geometry,
            orientation,
            position,
            surface_resolution,
            depth_fade,
        });
        for operation in program {
            match operation {
                ModelPainterOp::Face {
                    triangle_range,
                    depth_key,
                } => {
                    if triangle_range.start > triangle_range.end
                        || triangle_range.end > self.nodes[node].geometry.triangles.len()
                    {
                        return Err(PainterError::InvalidFaceRange);
                    }
                    let key = self
                        .view
                        .depth_key(&depth_key, orientation, position, self.scale)?;
                    self.groups
                        .last_mut()
                        .ok_or(PainterError::UnbalancedGroup)?
                        .items
                        .push((
                            key,
                            PainterItem::Primitive(PainterPrimitive::Face(PainterFace {
                                node,
                                triangles: triangle_range,
                            })),
                        ));
                }
                ModelPainterOp::Instance { instance_index } => {
                    if depth == 0 {
                        continue;
                    }
                    let instance = self.nodes[node]
                        .geometry
                        .instances
                        .get(instance_index)
                        .ok_or(PainterError::InvalidInstance)?
                        .clone();
                    let child_id = usize::from(instance.model_id);
                    if child_id == model_id {
                        continue;
                    }
                    let authored = orientation_f32(instance.orientation);
                    let child_name = self
                        .cache
                        .global_model(child_id)
                        .and_then(|child| child.name.as_deref());
                    let child_local = child_transform
                        .map(|hook| hook.transform(child_id, child_name, authored))
                        .unwrap_or(authored);
                    let child_position = instance_world_position(
                        orientation,
                        position,
                        instance.attach_pos.unwrap_or([0.0; 3]),
                        self.scale,
                    );
                    let child_linked = build_child_linked_slots(
                        model,
                        linked,
                        &instance,
                        vars,
                        root_link_policy,
                        None,
                    );
                    let child_vars = inherited_child_vars(vars, &instance);
                    // 67410 shares the current queue context. There is no
                    // implicit scope at a model entry or return boundary.
                    self.visit(
                        child_id,
                        mat3_mul(orientation, child_local),
                        child_position,
                        depth - 1,
                        Some(&child_linked),
                        &child_vars,
                        child_transform,
                        ModelTreeRootLinkPolicy::Authored,
                    )?;
                }
                ModelPainterOp::BeginGroup {
                    depth_key, sorting, ..
                } => {
                    let key = self
                        .view
                        .depth_key(&depth_key, orientation, position, self.scale)?;
                    self.groups.push(PainterGroup {
                        key,
                        sorting,
                        items: Vec::new(),
                    });
                }
                ModelPainterOp::EndGroup => {
                    if self.groups.len() <= 1 {
                        return Err(PainterError::UnbalancedGroup);
                    }
                    let group = self.groups.pop().ok_or(PainterError::UnbalancedGroup)?;
                    self.groups
                        .last_mut()
                        .ok_or(PainterError::UnbalancedGroup)?
                        .items
                        .push((group.key, PainterItem::Group(group)));
                }
                ModelPainterOp::Edge {
                    edge_index,
                    depth_key,
                } => {
                    if edge_index >= self.nodes[node].geometry.edges.len() {
                        return Err(PainterError::InvalidEdge);
                    }
                    let key = self
                        .view
                        .depth_key(&depth_key, orientation, position, self.scale)?;
                    self.groups
                        .last_mut()
                        .ok_or(PainterError::UnbalancedGroup)?
                        .items
                        .push((
                            key,
                            PainterItem::Primitive(PainterPrimitive::Edge {
                                node,
                                edge: edge_index,
                            }),
                        ));
                }
                ModelPainterOp::Billboard {
                    billboard_index,
                    depth_key,
                } => {
                    if billboard_index >= self.nodes[node].geometry.billboards.len() {
                        return Err(PainterError::InvalidBillboard);
                    }
                    let key = self
                        .view
                        .depth_key(&depth_key, orientation, position, self.scale)?;
                    self.groups
                        .last_mut()
                        .ok_or(PainterError::UnbalancedGroup)?
                        .items
                        .push((
                            key,
                            PainterItem::Primitive(PainterPrimitive::Billboard {
                                node,
                                billboard: billboard_index,
                            }),
                        ));
                }
            }
        }
        Ok(())
    }

    fn finish(
        mut self,
        composition: ModelTreePainterComposition,
    ) -> Result<(Vec<PainterNode>, Vec<PainterPrimitive>, ModelDepthPolicy), PainterError> {
        if self.groups.len() != 1 {
            return Err(PainterError::UnbalancedGroup);
        }
        let root = self.groups.pop().ok_or(PainterError::UnbalancedGroup)?;
        let depth_policy = scene_depth_policy(&root, composition)?;
        let mut faces = Vec::new();
        root.drain(&mut faces);
        Ok((self.nodes, faces, depth_policy))
    }
}

fn scene_depth_policy(
    root: &PainterGroup,
    composition: ModelTreePainterComposition,
) -> Result<ModelDepthPolicy, PainterError> {
    match composition {
        ModelTreePainterComposition::Isolated => Ok(ModelDepthPolicy::Painter),
        ModelTreePainterComposition::OpaqueSceneGroup => match root.items.as_slice() {
            [(key, PainterItem::Group(_))] if *key > 0 => Ok(ModelDepthPolicy::PainterGroup {
                view_depth_raw: *key,
            }),
            _ => Err(PainterError::ExpectedSingleOuterGroup),
        },
    }
}

fn face_slice<T>(values: &[T], range: Range<usize>) -> &[T] {
    if values.is_empty() {
        values
    } else {
        &values[range]
    }
}

impl ModelNodeGeometry<'_> {
    fn face_range(&self, range: Range<usize>) -> ModelNodeGeometry<'_> {
        ModelNodeGeometry {
            model_flags: self.model_flags,
            radius_raw: self.radius_raw,
            vertices: self.vertices,
            vertex_type_flags: self.vertex_type_flags,
            vertex_projection: self.vertex_projection,
            vertex_clip: self.vertex_clip,
            vertex_surface_origin: self.vertex_surface_origin,
            triangles: face_slice(self.triangles, range.clone()),
            face_vertices: face_slice(self.face_vertices, range.clone()),
            normals: face_slice(self.normals, range.clone()),
            face_cull: face_slice(self.face_cull, range.clone()),
            face_materials: face_slice(self.face_materials, range.clone()),
            face_uvs: face_slice(self.face_uvs, range.clone()),
            face_corner_normals: face_slice(self.face_corner_normals, range.clone()),
            face_shading: face_slice(self.face_shading, range),
            edges: &[],
            edge_projection: v2k_render::renderer::ModelEdgeProjection::Compatibility,
            billboards: &[],
        }
    }

    fn edge_index(&self, index: usize) -> ModelNodeGeometry<'_> {
        let mut geometry = self.face_range(0..0);
        geometry.edges = &self.edges[index..index + 1];
        geometry.edge_projection = match self.edge_projection {
            v2k_render::renderer::ModelEdgeProjection::Compatibility => {
                v2k_render::renderer::ModelEdgeProjection::Compatibility
            }
            v2k_render::renderer::ModelEdgeProjection::CommandSnapshots(snapshots) => {
                v2k_render::renderer::ModelEdgeProjection::CommandSnapshots(
                    &snapshots[index..index + 1],
                )
            }
        };
        geometry
    }

    fn billboard_index(&self, index: usize) -> ModelNodeGeometry<'_> {
        let mut geometry = self.face_range(0..0);
        geometry.billboards = &self.billboards[index..index + 1];
        geometry
    }
}

impl ModelTreeRenderer<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_painter_hierarchy(
        &mut self,
        model_id: usize,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
        child_transform: Option<&dyn ModelTreeChildTransform>,
        view: ModelTreeView,
        composition: ModelTreePainterComposition,
    ) {
        let Some(model) = self.cache.global_model(model_id) else {
            return;
        };
        let Some(depth_fade) = self.node_depth_fade(model, position) else {
            return;
        };
        let native_painter = self.view_pin == ViewPinMode::WorldSurface
            && self.external_frame.native_context().is_some()
            && !matches!(
                self.projection_authority,
                v2k_render::projection::SceneProjectionAuthority::Compatibility(_)
            );
        let prepared_root = if native_painter {
            if let v2k_render::projection::SceneProjectionAuthority::Missing(reason) =
                self.projection_authority
            {
                report_painter_error(model_id, &PainterError::MissingNativeProjection(reason));
                return;
            }
            let Some(model) = self.cache.global_model(model_id) else {
                return;
            };
            if let Err(boundary) = model.native_painter_root_preflight() {
                // The childless adapter cannot reorder parent callbacks around
                // a live child. Reject every possible child path before H/E.
                report_painter_error(model_id, &PainterError::NativeProgram(boundary));
                return;
            }
            let (geometry, _, resolution) = self.materialize_live_node(
                model_id,
                model,
                orientation,
                position,
                depth,
                linked,
                vars,
                self.root_link_policy,
            );
            Some((geometry, resolution, depth_fade))
        } else {
            None
        };
        let mut traversal = PainterTraversal::new(
            self.cache,
            self.scale,
            view,
            model_far_plane(self.depth_fade, self.world_fog_planes),
        );
        traversal.prepared_root = prepared_root;
        let frame = traversal
            .visit(
                model_id,
                orientation,
                position,
                depth,
                linked,
                vars,
                child_transform,
                self.root_link_policy,
            )
            .and_then(|()| traversal.finish(composition))
            .and_then(|frame| {
                if composition == ModelTreePainterComposition::OpaqueSceneGroup {
                    // A translucent descendant would need the farther scene
                    // colour before this group drains. Validate the complete
                    // hierarchy before making any visible submission.
                    for node in &frame.0 {
                        if !node.geometry.edges.is_empty() || !node.geometry.billboards.is_empty() {
                            return Err(PainterError::UnsupportedPrimitive);
                        }
                        for &packed in &node.geometry.face_materials {
                            let (id, is_sprite) = v2k_formats::models::face_material(packed);
                            if is_sprite {
                                let flags = self
                                    .colors
                                    .sprite_render_flags(self.cache, id)
                                    .ok_or(PainterError::MissingMaterial)?;
                                if crate::model_color::sprite_blend(flags)
                                    != v2k_render::WorldSpriteBlend::Masked
                                {
                                    return Err(PainterError::TranslucentSceneGroup);
                                }
                            }
                        }
                    }
                }
                Ok(frame)
            });
        let (nodes, faces, depth_policy) = match frame {
            Ok(frame) => frame,
            Err(error) => {
                // An unsupported program must never fall back to geometric
                // depth and cover the body with its deliberately distant mask.
                report_painter_error(model_id, &error);
                return;
            }
        };
        for primitive in faces {
            let node_index = match &primitive {
                PainterPrimitive::Face(face) => face.node,
                PainterPrimitive::Edge { node, .. } | PainterPrimitive::Billboard { node, .. } => {
                    *node
                }
            };
            let node = &nodes[node_index];
            let geometry = ModelNodeGeometry::from_materialized(
                &node.geometry,
                node.model_flags,
                node.radius_raw,
            );
            match primitive {
                PainterPrimitive::Face(face) => self.submit_node(
                    geometry.face_range(face.triangles),
                    node.orientation,
                    node.position,
                    self.view_pin,
                    depth_policy,
                    node.surface_resolution,
                    node.depth_fade.unwrap_or_else(|| {
                        model_node_depth_fade(node.model_flags, self.depth_fade)
                    }),
                ),
                PainterPrimitive::Edge { edge, .. } => self.submit_node(
                    geometry.edge_index(edge),
                    node.orientation,
                    node.position,
                    self.view_pin,
                    depth_policy,
                    node.surface_resolution,
                    node.depth_fade.unwrap_or_else(|| {
                        model_node_depth_fade(node.model_flags, self.depth_fade)
                    }),
                ),
                PainterPrimitive::Billboard { billboard, .. } => self.submit_node_billboards(
                    &geometry.billboard_index(billboard),
                    node.orientation,
                    node.position,
                    depth_policy,
                    node.depth_fade.unwrap_or_else(|| {
                        model_node_depth_fade(node.model_flags, self.depth_fade)
                    }),
                ),
            }
        }
    }
}

impl std::hash::Hash for PainterError {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&std::mem::discriminant(self), state);
        match self {
            Self::NativeProgram(boundary) => std::hash::Hash::hash(boundary, state),
            Self::MissingNativeProjection(reason) => {
                std::hash::Hash::hash(&std::mem::discriminant(reason), state)
            }
            Self::MissingModel
            | Self::InvalidInstance
            | Self::InvalidFaceRange
            | Self::InvalidEdge
            | Self::InvalidBillboard
            | Self::UnresolvedDepth
            | Self::UnbalancedGroup
            | Self::UnsupportedPrimitive
            | Self::ExpectedSingleOuterGroup
            | Self::TranslucentSceneGroup
            | Self::MissingMaterial => {}
        }
    }
}

fn report_painter_error(model_id: usize, error: &PainterError) {
    static REPORTED: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashSet<(usize, PainterError)>>,
    > = std::sync::OnceLock::new();
    let reported = REPORTED.get_or_init(Default::default);
    if let Ok(mut reported) = reported.lock() {
        if reported.insert((model_id, *error)) {
            eprintln!("Model {model_id} painter program not submitted: {error:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    const VIEW: ModelTreeView = ModelTreeView {
        position: [0.0; 3],
        forward: [0.0, 0.0, -1.0],
    };

    fn face(index: usize) -> PainterItem {
        PainterItem::Primitive(PainterPrimitive::Face(PainterFace {
            node: 0,
            triangles: index..index + 1,
        }))
    }

    #[test]
    fn scene_group_keeps_the_outer_key_and_rejects_loose_or_multiple_groups() {
        let group = |key| {
            (
                key,
                PainterItem::Group(PainterGroup {
                    key,
                    sorting: ModelPainterSorting::Sorted,
                    items: vec![(1, face(0)), (9000, face(1))],
                }),
            )
        };
        let mut root = PainterGroup {
            key: 0,
            sorting: ModelPainterSorting::Sorted,
            items: vec![group(5120)],
        };
        let policy = ModelTreePainterComposition::OpaqueSceneGroup;
        assert_eq!(
            scene_depth_policy(&root, policy),
            Ok(ModelDepthPolicy::PainterGroup {
                view_depth_raw: 5120
            })
        );
        root.items.push(group(7000));
        assert_eq!(
            scene_depth_policy(&root, policy),
            Err(PainterError::ExpectedSingleOuterGroup)
        );
        root.items = vec![(5120, face(0))];
        assert_eq!(
            scene_depth_policy(&root, policy),
            Err(PainterError::ExpectedSingleOuterGroup)
        );
        root.items = vec![group(-2560)];
        assert_eq!(
            scene_depth_policy(&root, policy),
            Err(PainterError::ExpectedSingleOuterGroup)
        );
        assert_eq!(
            scene_depth_policy(&root, ModelTreePainterComposition::Isolated),
            Ok(ModelDepthPolicy::Painter)
        );
    }

    #[test]
    fn sorted_queues_keep_equal_keys_stable_and_drain_nested_groups_atomically() {
        let mut output = Vec::new();
        PainterGroup {
            key: 0,
            sorting: ModelPainterSorting::Sorted,
            items: vec![
                (20, face(0)),
                (-30, face(4)),
                (20, face(1)),
                (
                    10,
                    PainterItem::Group(PainterGroup {
                        key: 10,
                        sorting: ModelPainterSorting::Unsorted,
                        items: vec![(-200, face(2)), (200, face(3))],
                    }),
                ),
            ],
        }
        .drain(&mut output);
        assert_eq!(
            output
                .iter()
                .map(|primitive| match primitive {
                    PainterPrimitive::Face(face) => face.triangles.start,
                    _ => panic!("expected face"),
                })
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn raw_depth_keys_use_the_camera_pose_signed_offsets_and_all_extrema_points() {
        let view = ModelTreeView {
            position: [3.0, 0.0, 2.0],
            ..VIEW
        };
        let quarter_y = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert_eq!(
            view.depth_key(
                &ModelPainterDepthKey::Vertex {
                    position_raw: [50.9, 0.0, 0.0],
                    offset_raw: 1500,
                },
                quarter_y,
                [3.0, 0.0, 1.0],
                2.0
            ),
            Ok(1701)
        );
        let points = vec![[0.0, 0.0, -200.0], [0.0, 0.0, 300.0]];
        assert_eq!(
            VIEW.depth_key(
                &ModelPainterDepthKey::Minimum(points.clone()),
                IDENTITY,
                [0.0; 3],
                1.0
            ),
            Ok(-300)
        );
        assert_eq!(
            VIEW.depth_key(
                &ModelPainterDepthKey::Maximum(points),
                IDENTITY,
                [0.0; 3],
                1.0
            ),
            Ok(200)
        );
        assert_eq!(
            VIEW.depth_key(
                &ModelPainterDepthKey::Minimum(vec![]),
                IDENTITY,
                [0.0; 3],
                1.0
            ),
            Ok(i32::MAX)
        );
        assert_eq!(
            VIEW.depth_key(
                &ModelPainterDepthKey::Maximum(vec![]),
                IDENTITY,
                [0.0; 3],
                1.0
            ),
            Ok(-i32::MAX)
        );
        assert_eq!(
            VIEW.depth_key(
                &ModelPainterDepthKey::Vertex {
                    position_raw: [0.0, 0.0, -1.0],
                    offset_raw: i32::MAX
                },
                IDENTITY,
                [0.0; 3],
                1.0
            ),
            Ok(i32::MIN)
        );
        assert_eq!(
            VIEW.depth_key(&ModelPainterDepthKey::Unresolved, IDENTITY, [0.0; 3], 1.0),
            Err(PainterError::UnresolvedDepth)
        );
    }
}
