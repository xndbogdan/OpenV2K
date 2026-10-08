//! Owned model commands separate draw-time actor callbacks from GL raster order.
//!
//! Enqueue only after materialization: flush cannot revisit actor components,
//! RNG, emitters, or limb caches. The caller installs the same frame's current
//! surface after particle/terrain updates, before draining these commands.
use super::*;
use v2k_formats::models::{ModelEdge, ModelSlotClip, ModelSurfaceOrigin};
use v2k_formats::system::FogGradientEntry;

#[derive(Default)]
pub struct ModelTreeSubmissionBuffer {
    commands: Vec<OwnedSubmission>,
}

impl ModelTreeSubmissionBuffer {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Drain in the original body/billboard/child submission order. A command
    /// that originally lacked a surface remains unowned at flush. The caller
    /// supplies the same draw tick and current terrain allocation after the
    /// source-ordered particle writes; no borrowed cache survives enqueue.
    pub fn flush(
        &mut self,
        renderer: &mut dyn Renderer,
        surface: Option<WorldSurfaceProjection<'_>>,
    ) {
        for command in self.commands.drain(..) {
            match command {
                OwnedSubmission::Body(body) => renderer.draw_model_body(body.borrow(surface)),
                OwnedSubmission::Billboards(billboards) => {
                    renderer.draw_model_billboards(billboards.borrow())
                }
                OwnedSubmission::EndNode => renderer.end_model_node(),
            }
        }
    }

    pub(super) fn push_body(&mut self, draw: ModelDraw<'_>) {
        self.commands
            .push(OwnedSubmission::Body(OwnedBody::new(draw)));
    }
    pub(super) fn push_billboards(&mut self, draw: ModelBillboardDraw<'_>) {
        self.commands
            .push(OwnedSubmission::Billboards(OwnedBillboards::new(draw)));
    }
    pub(super) fn push_end_node(&mut self) {
        self.commands.push(OwnedSubmission::EndNode);
    }
}

enum OwnedSubmission {
    Body(OwnedBody),
    Billboards(OwnedBillboards),
    EndNode,
}

struct OwnedBody {
    radius_raw: u16,
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
    face_uvs: Vec<[[f32; 2]; 3]>,
    face_corner_normals: Vec<[[f32; 3]; 3]>,
    face_normals_raw: Vec<[[i32; 3]; 4]>,
    face_shading: Vec<ModelFaceShading>,
    edges: Vec<ModelEdge>,
    edge_materials: Vec<FaceMaterial>,
    edge_snapshots: Option<Vec<v2k_formats::models::ModelEdgeEndpointSnapshot>>,
    edge_widths: Vec<(u16, u16)>,
    shade_table: Option<Vec<FogGradientEntry>>,
    light_direction_raw: [i32; 3],
    native_light_raw: Option<[i32; 3]>,
    shade_shift: i32,
    materials: Vec<FaceMaterial>,
    transform: ModelTransform,
    near_clip: ModelNearClip,
    projection_authority: v2k_render::projection::SceneProjectionAuthority,
    depth_fade: ModelDepthFade,
    depth_policy: ModelDepthPolicy,
    view_pin: ViewPinMode,
    surface_resolution: ModelSurfaceResolution,
    surface_waves_enabled: Option<bool>,
    external_frame: ExternalFrameMode,
    overlay: ModelOverlayKind,
    /// The node's painter program and parent instance, when it is a tree node.
    painter: Option<(Vec<v2k_formats::models::ModelPainterOp>, Option<usize>)>,
}

impl OwnedBody {
    fn new(draw: ModelDraw<'_>) -> Self {
        let mesh = draw.mesh;
        Self {
            radius_raw: mesh.radius_raw,
            vertices: mesh.vertices.to_vec(),
            vertex_type_flags: mesh.vertex_type_flags.to_vec(),
            vertex_projection: mesh.vertex_projection.to_vec(),
            vertex_clip: mesh.vertex_clip.to_vec(),
            vertex_surface_origin: mesh.vertex_surface_origin.to_vec(),
            vertex_view_raw: mesh.vertex_view_raw.to_vec(),
            triangles: mesh.triangles.to_vec(),
            face_vertices: mesh.face_vertices.to_vec(),
            normals: mesh.normals.to_vec(),
            face_cull: mesh.face_cull.to_vec(),
            face_uvs: mesh.face_uvs.to_vec(),
            face_corner_normals: mesh.face_corner_normals.to_vec(),
            face_normals_raw: mesh.face_normals_raw.to_vec(),
            face_shading: mesh.face_shading.to_vec(),
            edges: mesh.edges.to_vec(),
            edge_materials: mesh.edge_materials.to_vec(),
            edge_snapshots: match mesh.edge_projection {
                v2k_render::renderer::ModelEdgeProjection::Compatibility => None,
                v2k_render::renderer::ModelEdgeProjection::CommandSnapshots(points) => {
                    Some(points.to_vec())
                }
            },
            edge_widths: mesh.edge_widths.to_vec(),
            shade_table: mesh.shade_table.map(<[_]>::to_vec),
            light_direction_raw: mesh.light_direction_raw,
            native_light_raw: mesh.native_light_raw,
            shade_shift: mesh.shade_shift,
            materials: mesh.materials.to_vec(),
            transform: draw.transform,
            near_clip: draw.near_clip,
            projection_authority: draw.projection_authority,
            depth_fade: draw.depth_fade,
            depth_policy: draw.depth_policy,
            view_pin: draw.view_pin,
            surface_resolution: draw.surface_resolution,
            surface_waves_enabled: draw.world_surface.map(|surface| surface.waves_enabled()),
            external_frame: draw.external_frame,
            overlay: draw.overlay,
            painter: draw
                .painter
                .map(|node| (node.program.to_vec(), node.parent_instance)),
        }
    }

    fn borrow<'a>(&'a self, surface: Option<WorldSurfaceProjection<'a>>) -> ModelDraw<'a> {
        ModelDraw {
            mesh: ModelMesh {
                radius_raw: self.radius_raw,
                vertices: &self.vertices,
                vertex_type_flags: &self.vertex_type_flags,
                vertex_projection: &self.vertex_projection,
                vertex_clip: &self.vertex_clip,
                vertex_surface_origin: &self.vertex_surface_origin,
                vertex_view_raw: &self.vertex_view_raw,
                triangles: &self.triangles,
                face_vertices: &self.face_vertices,
                normals: &self.normals,
                face_cull: &self.face_cull,
                face_uvs: &self.face_uvs,
                face_corner_normals: &self.face_corner_normals,
                face_normals_raw: &self.face_normals_raw,
                face_shading: &self.face_shading,
                edge_projection: self.edge_snapshots.as_deref().map_or(
                    v2k_render::renderer::ModelEdgeProjection::Compatibility,
                    v2k_render::renderer::ModelEdgeProjection::CommandSnapshots,
                ),
                edges: &self.edges,
                edge_materials: &self.edge_materials,
                edge_widths: &self.edge_widths,
                shade_table: self.shade_table.as_deref(),
                light_direction_raw: self.light_direction_raw,
                native_light_raw: self.native_light_raw,
                shade_shift: self.shade_shift,
                materials: &self.materials,
            },
            transform: self.transform,
            near_clip: self.near_clip,
            projection_authority: self.projection_authority,
            depth_fade: self.depth_fade,
            depth_policy: self.depth_policy,
            view_pin: self.view_pin,
            surface_resolution: self.surface_resolution,
            world_surface: self
                .surface_waves_enabled
                .and_then(|enabled| surface.map(|surface| surface.with_waves_enabled(enabled))),
            external_frame: self.external_frame,
            overlay: self.overlay,
            painter: self
                .painter
                .as_ref()
                .map(|(program, parent_instance)| ModelPainterNode {
                    program,
                    parent_instance: *parent_instance,
                }),
        }
    }
}

struct OwnedBillboards {
    vertices: Vec<[f64; 3]>,
    vertex_projection: Vec<ModelVertexProjection>,
    vertex_clip: Vec<ModelSlotClip>,
    vertex_view_raw: Vec<Option<[i32; 3]>>,
    billboards: Vec<Billboard>,
    materials: Vec<BillboardMaterial>,
    transform: ModelTransform,
    radius_raw: u16,
    depth_fade: ModelDepthFade,
    near_clip: ModelNearClip,
    depth_policy: ModelDepthPolicy,
}
impl OwnedBillboards {
    fn new(draw: ModelBillboardDraw<'_>) -> Self {
        Self {
            vertices: draw.vertices.to_vec(),
            vertex_projection: draw.vertex_projection.to_vec(),
            vertex_clip: draw.vertex_clip.to_vec(),
            vertex_view_raw: draw.vertex_view_raw.to_vec(),
            billboards: draw.billboards.to_vec(),
            materials: draw.materials.to_vec(),
            transform: draw.transform,
            radius_raw: draw.radius_raw,
            depth_fade: draw.depth_fade,
            near_clip: draw.near_clip,
            depth_policy: draw.depth_policy,
        }
    }
    fn borrow(&self) -> ModelBillboardDraw<'_> {
        ModelBillboardDraw {
            vertices: &self.vertices,
            vertex_projection: &self.vertex_projection,
            vertex_clip: &self.vertex_clip,
            vertex_view_raw: &self.vertex_view_raw,
            billboards: &self.billboards,
            materials: &self.materials,
            transform: self.transform,
            radius_raw: self.radius_raw,
            depth_fade: self.depth_fade,
            near_clip: self.near_clip,
            depth_policy: self.depth_policy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::TerrainGrid;
    #[test]
    fn buffer_owns_callback_metadata_and_source_order_without_retaining_draw_borrows() {
        let mut buffer = ModelTreeSubmissionBuffer::default();
        {
            let mut slot = ResolvedModelSlot::clear([900.0, 800.0, 700.0]);
            slot.world_point = Some([1.125, -2.5, 3.75]);
            let model = ModelEntry {
                vertices: vec![slot.position_raw],
                vertex_type_flags: vec![14],
                vertex_projection: vec![ModelVertexProjection::WorldPoint(
                    slot.world_point.unwrap(),
                )],
                vertex_clip: vec![ModelSlotClip::SurfaceBand],
                vertex_surface_origin: vec![ModelSurfaceOrigin::ViewPin],
                ..ModelEntry::default()
            };
            let transform = ModelTransform {
                orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                position: [3.0, 4.0, 5.0],
                scale: 0.5,
            };
            let materials = vec![FaceMaterial {
                color: [0.25, 0.5, 0.75],
                palette_rgb555: Some(0x1234),
                emissive: [1.0, 2.0, 3.0],
                texture: None,
                blend: v2k_render::WorldSpriteBlend::HalfAdditive,
                flat_shade_row: 7,
            }];
            let terrain = TerrainGrid {
                header: [0; 5],
                cells: Vec::new(),
            };
            let draw = ModelDraw {
                projection_authority: v2k_render::projection::SceneProjectionAuthority::default(),
                mesh: ModelMesh::from_model(&model, &materials),
                transform,
                near_clip: ModelNearClip::RetailWorld,
                depth_fade: ModelDepthFade::Disabled,
                depth_policy: ModelDepthPolicy::Painter,
                view_pin: ViewPinMode::WorldSurface,
                surface_resolution: ModelSurfaceResolution::ContextResolved,
                world_surface: Some(
                    WorldSurfaceProjection::new(&terrain, 42).with_waves_enabled(false),
                ),
                external_frame: ExternalFrameMode::Raw,
                overlay: ModelOverlayKind::TerrainSurface,
                painter: None,
            };
            let mut later = OwnedBody::new(ModelDraw {
                projection_authority: draw.projection_authority,
                mesh: draw.mesh,
                transform: draw.transform,
                near_clip: draw.near_clip,
                depth_fade: draw.depth_fade,
                depth_policy: draw.depth_policy,
                view_pin: draw.view_pin,
                surface_resolution: draw.surface_resolution,
                world_surface: None,
                external_frame: draw.external_frame,
                overlay: draw.overlay,
                painter: None,
            });
            later.shade_shift = 19;
            buffer.push_body(draw);
            buffer.push_billboards(ModelBillboardDraw {
                vertices: &model.vertices,
                vertex_projection: &model.vertex_projection,
                vertex_clip: &model.vertex_clip,
                vertex_view_raw: &[],
                billboards: &[],
                materials: &[],
                transform,
                radius_raw: 77,
                depth_fade: ModelDepthFade::Disabled,
                near_clip: ModelNearClip::RetailWorld,
                depth_policy: ModelDepthPolicy::Geometry,
            });
            buffer.commands.push(OwnedSubmission::Body(later));
        }
        assert_eq!(buffer.len(), 3);
        let [OwnedSubmission::Body(first), OwnedSubmission::Billboards(mid), OwnedSubmission::Body(last)] =
            buffer.commands.as_slice()
        else {
            panic!("source ordering");
        };
        assert_eq!(
            first.vertex_projection,
            [ModelVertexProjection::WorldPoint([1.125, -2.5, 3.75])]
        );
        assert_eq!(first.vertex_clip, [ModelSlotClip::SurfaceBand]);
        assert_eq!(first.vertex_surface_origin, [ModelSurfaceOrigin::ViewPin]);
        assert_eq!(first.materials[0].palette_rgb555, Some(0x1234));
        assert_eq!(first.near_clip, ModelNearClip::RetailWorld);
        assert_eq!(mid.radius_raw, 77);
        assert_eq!(last.shade_shift, 19);
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: Vec::new(),
        };
        let surface = Some(WorldSurfaceProjection::new(&terrain, 42));
        assert!(!first.borrow(surface).world_surface.unwrap().waves_enabled(), "flush keeps the producer's authored wave policy, despite the current surface's enabled default");
        assert_eq!(
            first.borrow(surface).world_surface.unwrap().retail_tick(),
            42
        );
        assert!(
            last.borrow(surface).world_surface.is_none(),
            "flush cannot invent a surface owner"
        );
        buffer.clear();
        assert!(buffer.is_empty());
    }
}

#[cfg(test)]
mod native_edge_buffer_tests {
    use super::*;
    use v2k_formats::models::{ModelEdgeEndpointSnapshot, ModelNativeEdgeEndpoint, ModelSlotClip};
    use v2k_render::projection::{
        NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority,
    };
    #[test]
    fn buffered_draw_freezes_native_edge_receipt_and_scene_authority() {
        let native =
            NativeScreenProjection::new([512; 2], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap();
        let mut snapshots = vec![ModelEdgeEndpointSnapshot::Native {
            endpoints: [
                ModelNativeEdgeEndpoint {
                    slot: 128,
                    view_raw: [-120, 20, 512],
                    clip: ModelSlotClip::Clear,
                },
                ModelNativeEdgeEndpoint {
                    slot: 130,
                    view_raw: [140, 80, 768],
                    clip: ModelSlotClip::SurfaceBand,
                },
            ],
        }];
        let model = ModelEntry::default();
        let mut mesh = ModelMesh::from_model(&model, &[]);
        mesh.edge_projection =
            v2k_render::renderer::ModelEdgeProjection::CommandSnapshots(&snapshots);
        let owned = OwnedBody::new(ModelDraw {
            mesh,
            projection_authority: SceneProjectionAuthority::Native(native),
            transform: ModelTransform {
                orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                position: [0.; 3],
                scale: 1.,
            },
            near_clip: ModelNearClip::RetailWorld,
            depth_fade: ModelDepthFade::Disabled,
            depth_policy: ModelDepthPolicy::Geometry,
            view_pin: ViewPinMode::Raw,
            surface_resolution: ModelSurfaceResolution::ContextResolved,
            world_surface: None,
            external_frame: ExternalFrameMode::Raw,
            overlay: ModelOverlayKind::None,
            painter: None,
        });
        snapshots[0] = ModelEdgeEndpointSnapshot::Compatibility;
        let draw = owned.borrow(None);
        assert_eq!(
            draw.projection_authority,
            SceneProjectionAuthority::Native(native)
        );
        let v2k_render::renderer::ModelEdgeProjection::CommandSnapshots(points) =
            draw.mesh.edge_projection
        else {
            panic!("receipt mode");
        };
        let ModelEdgeEndpointSnapshot::Native { endpoints } = points[0] else {
            panic!("owned snapshot");
        };
        assert_eq!(endpoints[0].slot, 128);
        assert_eq!(endpoints[0].view_raw, [-120, 20, 512]);
        assert_eq!(endpoints[1].clip, ModelSlotClip::SurfaceBand);
    }
}
