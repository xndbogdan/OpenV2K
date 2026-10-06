//! Intro2 Type66 spawn51 / model 210 `factory6` world submission extents.
//!
//! The sky-spanning white verticals were authored `0x22` sprite-608 roof
//! ribbons. Sub-M's zero-threshold factory publishes `u16::MAX` into dyn[2],
//! so the `0x2C` production skip is not taken. `FUN_00459000` feeds the raw
//! size short to `FUN_004594C0`; packed `FUN_00470840` decode turned 45 into
//! 2048 and extruded those horizontal roof segments into screen-vertical
//! beams.

use v2k_formats::models::ModelEdgeStyle;
use v2k_formats::system::PaletteEntry;
use v2k_formats::terrain::TerrainGrid;
use v2k_game::{
    entity::{
        world_position_raw, EntityConstructionResources, EntityManager, Intro2BirthSelection,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    factory_production_live::published_status,
    factory_status_runtime::project_factory_status,
    gameplay_notifications::GameplayNotifications,
    intro2_type17::{tick_intro2_type17, Intro2Type17Frame, Intro2Type17Owner},
    model_color::ModelMaterialCache,
    model_tree::{model_is_camera_facing_actor, ModelTreeRenderer, ModelTreeView},
    opening::{
        intro2_uses_live_actor_pose, intro_actor_heading, intro_actor_pose, intro_actor_visible,
    },
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::WorldFx,
};
use v2k_render::{
    edge_quads::{
        build_edge_line, build_edge_quad, select_edge_quads_with_stats, ScreenIntrinsics,
    },
    gl_backend::edge_endpoint_world,
    mat3_mul, orientation_from_ypr,
    projection::SceneProjectionAuthority,
    Camera, ExternalFrameMode, FaceMaterial, ModelBillboardDraw, ModelDraw, Renderer,
    TerrainFrames, TerrainLightWindow, WorldSpriteBlend,
};

const GAMEPLAY_MODEL_SCALE: f32 = 100.0 / 256.0;

fn fixture() -> (GameSession, EntityManager) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *slots,
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();
    (session, manager)
}

fn world_point(
    raw: [f64; 3],
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    extra_scale: f32,
) -> [f32; 3] {
    let scale = extra_scale / 100.0;
    let r = [raw[0] as f32, raw[1] as f32, raw[2] as f32];
    [
        position[0]
            + (orientation[0][0] * r[0] + orientation[0][1] * r[1] + orientation[0][2] * r[2])
                * scale,
        position[1]
            + (orientation[1][0] * r[0] + orientation[1][1] * r[1] + orientation[1][2] * r[2])
                * scale,
        position[2]
            + (orientation[2][0] * r[0] + orientation[2][1] * r[1] + orientation[2][2] * r[2])
                * scale,
    ]
}

#[derive(Default)]
struct RecordingRenderer {
    bodies: Vec<RecordedBody>,
}

struct RecordedBody {
    vertices: Vec<[f64; 3]>,
    vertex_type_flags: Vec<i16>,
    edges: Vec<v2k_formats::models::ModelEdge>,
    edge_widths: Vec<(u16, u16)>,
    edge_materials: Vec<FaceMaterial>,
    transform: v2k_render::ModelTransform,
    external_frame: ExternalFrameMode,
}

impl Renderer for RecordingRenderer {
    fn backend_name(&self) -> &str {
        "factory-extents"
    }
    fn clear(&mut self, _r: f32, _g: f32, _b: f32) {}
    fn present(&mut self) {}
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn set_camera(&mut self, _camera: &Camera) {}
    fn set_fog(&mut self, _enabled: bool, _near: f32, _far: f32, _color: [f32; 3]) {}
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
        self.bodies.push(RecordedBody {
            vertices: draw.mesh.vertices.to_vec(),
            vertex_type_flags: draw.mesh.vertex_type_flags.to_vec(),
            edges: draw.mesh.edges.to_vec(),
            edge_widths: draw.mesh.edge_widths.to_vec(),
            edge_materials: draw.mesh.edge_materials.to_vec(),
            transform: draw.transform,
            external_frame: draw.external_frame,
        });
    }
    fn draw_model_billboards(&mut self, _draw: ModelBillboardDraw<'_>) {}
    fn draw_sprite(&mut self, _rgba: &[u8], _width: u32, _height: u32, _x: i32, _y: i32) {}
    fn draw_fullscreen(&mut self, _rgba: &[u8], _width: u32, _height: u32) {}
    fn draw_color_overlay(&mut self, _r: f32, _g: f32, _b: f32, _a: f32) {}
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
    fn viewport_size(&self) -> (u32, u32) {
        (640, 480)
    }
}

#[v2k_test_support::retail_test]
fn intro2_factory6_roof_ribbons_stay_factory_scale() {
    let (session, manager) = fixture();
    let factory = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(51))
        .unwrap();
    assert_eq!(factory.entity_type, 66);
    assert_eq!(
        factory.model_slots,
        [Some(210), Some(225), Some(210), Some(225)]
    );
    assert!(intro2_uses_live_actor_pose(factory));
    let model_id = factory.model_index.unwrap();
    let model = session.cache.global_model(model_id).unwrap();
    assert_eq!(model.name.as_deref(), Some("factory6"));

    let mut authored_ribbon_sizes = Vec::new();
    let mut pc = 0usize;
    let words = &model.cmd_words;
    while pc < words.len() {
        let op = words[pc];
        if op == 0 {
            break;
        }
        if op == 0x22 && pc + 4 < words.len() {
            authored_ribbon_sizes.push(words[pc + 2]);
        }
        pc += match op {
            0x02 => 4,
            0x22 => 5,
            0x03 | 0x43 | 0x83 | 0xC3 | 0x07 | 0x47 | 0x87 | 0xC7 => 6,
            0x04 | 0x44 | 0x84 | 0xC4 | 0x08 | 0x48 | 0x88 | 0xC8 => 7,
            0x23 | 0xA3 | 0x27 | 0xA7 => 9,
            0x24 | 0xA4 | 0x28 | 0xA8 => 11,
            0x2B | 0x2C => 4,
            0x0B | 0x0C => 3,
            0x13 | 0x14 => 4,
            0x0D | 0x1D | 0x2D | 0x3D | 0x4D | 0x5D | 0x6D | 0x7D | 0x8D | 0x9D | 0xAD | 0xBD
            | 0xCD | 0xDD | 0xED | 0xFD => 4,
            0x68 | 0xE8 | 0x78 | 0xB8 | 0xF8 => 5,
            0x0F | 0x66 | 0x86 | 0xE6 | 0x10 => 1,
            0x1C => 3,
            0x3C | 0x5C => 4,
            0x0E => {
                if pc + 3 >= words.len() {
                    break;
                }
                1 + (words[pc + 3] as usize / 2)
            }
            _ => 1,
        };
    }
    assert!(
        !authored_ribbon_sizes.is_empty(),
        "factory6 authors 0x22 roof ribbons"
    );

    let RetailRuntimeValue::Known(Some(factory_state)) = factory.base_factory_runtime else {
        panic!("Type66 spawn51 missing Sub-M");
    };
    let production = factory_state
        .production
        .expect("Working Factory production");
    let status = published_status(production);
    assert_eq!(
        status.production_or_cooldown_ratio_raw,
        u16::MAX,
        "zero-threshold Intro2 factory publishes MAX, so 0x2C does not skip the ribbons"
    );
    let vars = project_factory_status(factory_state, status)
        .after
        .anim_vars(200);

    let orientation = match factory.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    ModelTreeRenderer::new_world(
        &mut renderer,
        &session.cache,
        &colors,
        GAMEPLAY_MODEL_SCALE,
        None,
        200,
    )
    .with_view(ModelTreeView {
        position: [
            factory.position[0],
            factory.position[1] + 8.0,
            factory.position[2] + 12.0,
        ],
        forward: [0.0, -0.4, -1.0],
    })
    .draw_linked(model_id, orientation, factory.position, 8, None, &vars);

    assert!(
        !renderer.bodies.is_empty(),
        "world model-tree must submit factory6"
    );

    let factory_radius_world = f32::from(model.radius) / 256.0;
    let mut ribbon_count = 0usize;
    let mut all_edges = Vec::new();
    let mut all_widths = Vec::new();
    let mut all_materials = Vec::new();
    let mut all_world = Vec::new();
    for body in &renderer.bodies {
        for v in &body.vertices {
            let w = world_point(
                *v,
                body.transform.orientation,
                body.transform.position,
                body.transform.scale,
            );
            let dx = w[0] - factory.position[0];
            let dy = w[1] - factory.position[1];
            let dz = w[2] - factory.position[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            assert!(
                dist < factory_radius_world * 2.5,
                "factory primitive left the factory6 hierarchy: world={w:?} dist={dist} radius={factory_radius_world}"
            );
        }
        let edge_base = all_world.len();
        all_world.extend(body.vertices.iter().map(|v| {
            Some(world_point(
                *v,
                body.transform.orientation,
                body.transform.position,
                body.transform.scale,
            ))
        }));
        for edge in &body.edges {
            let mut shifted = *edge;
            shifted.vertices = [
                edge.vertices[0] + edge_base as u16,
                edge.vertices[1] + edge_base as u16,
            ];
            all_edges.push(shifted);
            if let ModelEdgeStyle::Sprite { sprite_id, size } = edge.style {
                ribbon_count += 1;
                assert_eq!(sprite_id, 608);
                assert!(
                    authored_ribbon_sizes.contains(&size),
                    "0x22 size {size} must stay the raw factory6 stream word, not packed decode"
                );
                let [a, b] = edge.vertices;
                let wa = world_point(
                    body.vertices[usize::from(a)],
                    body.transform.orientation,
                    body.transform.position,
                    body.transform.scale,
                );
                let wb = world_point(
                    body.vertices[usize::from(b)],
                    body.transform.orientation,
                    body.transform.position,
                    body.transform.scale,
                );
                assert!(
                    (wa[1] - wb[1]).abs() < 0.05,
                    "factory6 0x22 ribbons are horizontal roof segments, not vertical world beams: {wa:?} -> {wb:?}"
                );
            }
        }
        all_widths.extend_from_slice(&body.edge_widths);
        all_materials.extend_from_slice(&body.edge_materials);
    }
    assert_eq!(
        ribbon_count, 5,
        "published Sub-M progress admits factory6's five sprite-608 roof ribbons"
    );

    let mut cam = Camera::new(640.0 / 480.0);
    cam.left_handed = true;
    cam.position = [
        factory.position[0],
        factory.position[1] + 8.0,
        factory.position[2] + 18.0,
    ];
    cam.yaw = 0.0;
    cam.pitch = -0.4;
    let fwd = cam.forward();
    let basis = [cam.right(), [0.0, 1.0, 0.0], [-fwd[0], -fwd[1], -fwd[2]]];
    let proj = cam.projection_matrix();
    let intr = ScreenIntrinsics::from_projection_terms(
        [640, 480],
        [
            proj[0],
            proj[5],
            cam.projection_offset[0],
            cam.projection_offset[1],
        ],
        [cam.near, cam.far],
    )
    .expect("intrinsics");
    let (selection, stats) = select_edge_quads_with_stats(
        &all_edges,
        &all_widths,
        &all_materials,
        &all_world,
        &intr,
        cam.position,
        basis,
    );
    assert_eq!(stats.submitted, 5);
    assert_eq!(selection.quads.len(), 5);
    assert!(selection.lines.is_empty());
    for (i, quad) in selection.quads.iter().enumerate() {
        let ys = quad.screen_corners.map(|c| c[1]);
        let y_span = ys.iter().copied().fold(f32::MIN, f32::max)
            - ys.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            y_span < 48.0,
            "roof ribbon {i} must stay a factory-scale screen quad, not a sky beam (y_span={y_span}, corners={:?})",
            quad.screen_corners
        );
    }
}

fn camera_relative(camera: &Camera, pos: [f32; 3]) -> [f32; 3] {
    [
        camera.position[0] + v2k_core::world::delta(pos[0], camera.position[0]),
        pos[1],
        camera.position[2] + v2k_core::world::delta(pos[2], camera.position[2]),
    ]
}

fn facing_orientation(
    camera: &Camera,
    draw_position: [f32; 3],
    body: [[f32; 3]; 3],
    model_id: usize,
    session: &GameSession,
) -> [[f32; 3]; 3] {
    let Some(model) = session.cache.global_model(model_id) else {
        return body;
    };
    if !model_is_camera_facing_actor(model) {
        return body;
    }
    let to_camera_x = camera.position[0] - draw_position[0];
    let to_camera_z = camera.position[2] - draw_position[2];
    let yaw = if to_camera_x.abs() + to_camera_z.abs() > 1.0e-6 {
        to_camera_x.atan2(to_camera_z)
    } else {
        0.0
    };
    let facing = orientation_from_ypr(yaw, 0.0, 0.0);
    if camera.left_handed {
        mat3_mul(facing, [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    } else {
        facing
    }
}

fn screen_intrinsics(cam: &Camera) -> ScreenIntrinsics {
    let proj = cam.projection_matrix();
    ScreenIntrinsics::from_projection_terms(
        [640, 480],
        [
            proj[0],
            proj[5],
            cam.projection_offset[0],
            cam.projection_offset[1],
        ],
        [cam.near, cam.far],
    )
    .expect("intrinsics")
}

fn camera_basis(cam: &Camera) -> [[f32; 3]; 3] {
    let fwd = cam.forward();
    [cam.right(), [0.0, 1.0, 0.0], [-fwd[0], -fwd[1], -fwd[2]]]
}

#[derive(Debug)]
#[allow(dead_code)]
struct SkyBeam {
    spawn: Option<usize>,
    entity_type: u32,
    model: String,
    style: &'static str,
    size: Option<u16>,
    vertex_types: [i16; 2],
    world_span: f32,
    y_span: f32,
    signature: &'static str,
}

fn classify_sky_beam(
    size: Option<u16>,
    types: [i16; 2],
    world_span: f32,
    y_span: f32,
) -> Option<&'static str> {
    if size == Some(2048) {
        return Some("packed-size-2048");
    }
    if types.contains(&14) && world_span >= 8.0 {
        return Some("torus-wrap-or-period-span");
    }
    if y_span < 200.0 && world_span < 32.0 {
        return None;
    }
    if world_span >= 32.0 {
        return Some("torus-wrap-or-period-span");
    }
    if types.contains(&13) {
        return Some("type13-view-y-pin");
    }
    Some("sky-span-unclassified")
}

fn collect_sky_beams(
    bodies: &[RecordedBody],
    cam: &Camera,
    spawn: Option<usize>,
    entity_type: u32,
    model_name: &str,
) -> Vec<SkyBeam> {
    let scale = GAMEPLAY_MODEL_SCALE / 100.0;
    let intr = screen_intrinsics(cam);
    let basis = camera_basis(cam);
    let mut beams = Vec::new();
    for body in bodies {
        let locals: Vec<Option<[f32; 3]>> = body
            .vertices
            .iter()
            .map(|v| Some([v[0] as f32, v[1] as f32, v[2] as f32]))
            .collect();
        let resolved: Vec<Option<[f32; 3]>> = (0..body.vertices.len())
            .map(|index| {
                edge_endpoint_world(
                    index,
                    &body.vertices,
                    &body.vertex_type_flags,
                    &locals,
                    body.external_frame,
                    body.transform.orientation,
                    body.transform.position,
                    scale,
                )
            })
            .collect();
        for (edge_i, edge) in body.edges.iter().enumerate() {
            let (Some(a), Some(b)) = (
                resolved
                    .get(usize::from(edge.vertices[0]))
                    .copied()
                    .flatten(),
                resolved
                    .get(usize::from(edge.vertices[1]))
                    .copied()
                    .flatten(),
            ) else {
                continue;
            };
            let world_span = {
                let dx = a[0] - b[0];
                let dy = a[1] - b[1];
                let dz = a[2] - b[2];
                (dx * dx + dy * dy + dz * dz).sqrt()
            };
            let t0 = body
                .vertex_type_flags
                .get(usize::from(edge.vertices[0]))
                .copied()
                .unwrap_or(0);
            let t1 = body
                .vertex_type_flags
                .get(usize::from(edge.vertices[1]))
                .copied()
                .unwrap_or(0);
            let (Some(pa), Some(pb)) = (
                intr.project(a, cam.position, basis, SceneProjectionAuthority::default()),
                intr.project(b, cam.position, basis, SceneProjectionAuthority::default()),
            ) else {
                continue;
            };
            let (style, size, y_span) = match edge.style {
                ModelEdgeStyle::Sprite { size, .. } => {
                    let widths = body.edge_widths.get(edge_i).copied().unwrap_or((0, 0));
                    if widths == (0, 0) || widths.0 == widths.1 {
                        continue;
                    }
                    let Some((corners, _)) = build_edge_quad(
                        (pa.0, pa.1),
                        (pb.0, pb.1),
                        pa.2,
                        pb.2,
                        &intr,
                        widths.0,
                        widths.1,
                        i32::from(size as i16),
                    ) else {
                        continue;
                    };
                    let ys = corners.map(|c| c[1]);
                    let y_span = ys.iter().copied().fold(f32::MIN, f32::max)
                        - ys.iter().copied().fold(f32::MAX, f32::min);
                    ("0x22", Some(size), y_span)
                }
                ModelEdgeStyle::Palette { .. } => {
                    let Some((start, end)) =
                        build_edge_line((pa.0, pa.1), (pb.0, pb.1), intr.width_px, intr.height_px)
                    else {
                        continue;
                    };
                    ("0x02", None, (start[1] - end[1]).abs())
                }
            };
            let Some(signature) = classify_sky_beam(size, [t0, t1], world_span, y_span) else {
                continue;
            };
            beams.push(SkyBeam {
                spawn,
                entity_type,
                model: model_name.to_string(),
                style,
                size,
                vertex_types: [t0, t1],
                world_span,
                y_span,
                signature,
            });
        }
    }
    beams
}

fn draw_intro2_actor(
    session: &GameSession,
    manager: &mut EntityManager,
    entity_id: u32,
    cam: &Camera,
    elapsed_secs: f32,
    retail_tick: u32,
) -> (RecordingRenderer, Option<usize>, u32, String) {
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    let entity = manager.entity_mut(entity_id).unwrap();
    let spawn = entity.authored_spawn_index;
    let entity_type = entity.entity_type;
    let live = intro2_uses_live_actor_pose(entity);
    let (model_id, position, body_orientation) = if live {
        let model_id = entity.model_index.expect("live model");
        (
            model_id,
            entity.position,
            match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
                RetailRuntimeValue::Unresolved => orientation_from_ypr(entity.heading, 0.0, 0.0),
            },
        )
    } else {
        let Some(spawn) = spawn else {
            return (renderer, spawn, entity_type, String::new());
        };
        let pose = intro_actor_pose(spawn, entity.position, elapsed_secs);
        let Some(model_id) = entity.model_in_slot(pose.model_slot) else {
            return (renderer, Some(spawn), entity_type, String::new());
        };
        (
            model_id,
            pose.position,
            orientation_from_ypr(entity.heading + intro_actor_heading(spawn), 0.0, 0.0),
        )
    };
    let model_name = session
        .cache
        .global_model(model_id)
        .and_then(|m| m.name.clone())
        .unwrap_or_else(|| format!("model{model_id}"));
    let draw_position = camera_relative(cam, position);
    let orientation = facing_orientation(cam, draw_position, body_orientation, model_id, session);
    let vars = entity.presentation_anim_vars(retail_tick);
    let descriptor = session
        .cache
        .global_entity_type(entity.entity_type as usize)
        .and_then(|record| record.sub_h_external_frame_descriptor());
    let body_axes_q31 = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
        RetailRuntimeValue::Unresolved => None,
    };
    let origin_raw = world_position_raw(position);
    let mut tree = ModelTreeRenderer::new_world(
        &mut renderer,
        &session.cache,
        &colors,
        GAMEPLAY_MODEL_SCALE,
        None,
        retail_tick as i32,
    )
    .with_view(cam.into());
    if let (true, Some(descriptor), Some(model), RetailRuntimeValue::Known(Some(runtime))) = (
        live,
        descriptor.as_ref(),
        session.cache.global_model(model_id),
        &mut entity.sub_h_external_frame_runtime,
    ) {
        tree = tree.with_sub_h_presentation(v2k_game::sub_h_external_frame::SubHPresentation {
            runtime,
            retail_tick,
            descriptor,
            model_records: &model.records,
            origin_raw,
            origin_world: draw_position,
            body_axes_q31,
            native_context: None,
            fallback: None,
            emitter: None,
        });
    }
    tree.draw_linked(model_id, orientation, draw_position, 8, None, &vars);
    drop(tree);
    (renderer, spawn, entity_type, model_name)
}

#[v2k_test_support::retail_test]
fn intro2_live_pose_edge_fills_do_not_span_the_sky() {
    let (mut session, mut manager) = fixture();
    let factory_pos = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(51))
        .map(|e| e.position)
        .unwrap_or([0.0; 3]);
    let insect_pos = manager
        .iter_all()
        .find(|e| e.entity_type == 17)
        .map(|e| e.position)
        .unwrap_or(factory_pos);

    let mut cameras = Vec::new();
    for (origin, yaw, pitch) in [
        (factory_pos, 0.0_f32, -0.4_f32),
        (factory_pos, std::f32::consts::FRAC_PI_2, -0.2),
        (factory_pos, std::f32::consts::PI, -0.35),
        (insect_pos, -0.7, -0.15),
        (insect_pos, 1.2, 0.1),
    ] {
        let mut cam = Camera::new(640.0 / 480.0);
        cam.left_handed = true;
        cam.position = [origin[0], origin[1] + 10.0, origin[2] + 18.0];
        cam.yaw = yaw;
        cam.pitch = pitch;
        cameras.push(cam);
    }

    let ids: Vec<u32> = manager
        .iter_all()
        .filter(|entity| entity.active)
        .filter(|entity| entity.authored_spawn_index.is_some_and(intro_actor_visible))
        .map(|entity| entity.id)
        .collect();

    let mut fx = WorldFx::new();
    let mut tick = 200u32;
    for id in manager
        .iter_all()
        .filter(|entity| entity.entity_type == 17)
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x68000, 0x68000);
        if let Ok(mut owner) = Intro2Type17Owner::adopt(&manager, id) {
            for _ in 0..8 {
                tick += 1;
                manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x68000, 0x68000);
                let pass = tick_intro2_type17(
                    &mut manager,
                    owner,
                    Intro2Type17Frame {
                        capture_tasks: &mut SpecializedActorTaskScheduler::default(),
                        notifications: &mut GameplayNotifications::new(),
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 20_000,
                        retail_tick: tick,
                    },
                );
                owner = match pass.retained_owner {
                    Some(next) => next,
                    None => break,
                };
            }
        }
    }

    let mut beams = Vec::new();
    for elapsed in [0.0_f32, 6.5, 20.0, 40.0] {
        for cam in &cameras {
            for &id in &ids {
                let (renderer, spawn, entity_type, model_name) =
                    draw_intro2_actor(&session, &mut manager, id, cam, elapsed, tick);
                beams.extend(collect_sky_beams(
                    &renderer.bodies,
                    cam,
                    spawn,
                    entity_type,
                    &model_name,
                ));
            }
        }
    }
    assert!(
        beams.is_empty(),
        "Intro2 0x02/0x22 sky-span fills: {beams:#?}"
    );
}

#[v2k_test_support::retail_test]
fn intro2_type17_type14_edges_stay_insect_scale_at_signed_88_seam() {
    let (session, mut manager) = fixture();
    let Some(id) = manager
        .iter_all()
        .find(|entity| entity.entity_type == 17)
        .map(|entity| entity.id)
    else {
        panic!("Intro2 authors Type17");
    };
    {
        let entity = manager.entity_mut(id).unwrap();
        entity.position = [127.0, entity.position[1], 127.0];
        entity.heading = 1.2;
    }
    let mut cam = Camera::new(640.0 / 480.0);
    cam.left_handed = true;
    cam.position = [127.0, 10.0, 145.0];
    cam.yaw = 0.0;
    cam.pitch = -0.35;
    let mut beams = Vec::new();
    let mut type14_edges = 0usize;
    for _ in 0..3 {
        let (renderer, spawn, entity_type, model_name) =
            draw_intro2_actor(&session, &mut manager, id, &cam, 0.0, 220);
        type14_edges += renderer
            .bodies
            .iter()
            .map(|body| {
                body.edges
                    .iter()
                    .filter(|edge| {
                        let t0 = body
                            .vertex_type_flags
                            .get(usize::from(edge.vertices[0]))
                            .copied()
                            .unwrap_or(0);
                        let t1 = body
                            .vertex_type_flags
                            .get(usize::from(edge.vertices[1]))
                            .copied()
                            .unwrap_or(0);
                        t0 == 14 || t1 == 14
                    })
                    .count()
            })
            .sum::<usize>();
        beams.extend(
            collect_sky_beams(&renderer.bodies, &cam, spawn, entity_type, &model_name)
                .into_iter()
                .filter(|beam| beam.vertex_types.contains(&14)),
        );
        cam.yaw += 0.9;
    }
    assert!(
        type14_edges > 0,
        "Intro2 Type17 must submit type-14 0x02/0x22 at the seam"
    );
    assert!(
        beams.is_empty(),
        "type-14 0x02/0x22 at the signed-8.8 seam must keep wrapping-short draw deltas: {beams:#?}"
    );
}
