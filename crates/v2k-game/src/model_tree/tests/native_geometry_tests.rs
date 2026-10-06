//! Complete retail hierarchies retain body, surface, edge and sprite geometry.
use super::*;
use crate::actor_emitter_external_frame::{
    ActorEmitterExternalFramePresentation, ActorEmitterModelPresentation,
};
use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
use crate::player::PlayerCraft;
use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
use v2k_formats::collision::SubHExternalFrameDescriptor;
use v2k_formats::models::{ModelSlotClip, ModelSurfaceOrigin};
use v2k_render::projection::{NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority};

const AXES: [[i32; 3]; 3] = [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]];
const ORIGIN: [i16; 3] = [1_000, 2_048, 6_000];
const ORIENTATION: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

fn corpus() -> crate::session::GameSession {
    let mut session = native_painter_session();
    // A controlled dry surface separates missing vertices from sea-band
    // rejection. This does not assert an all-terrain or all-animation result.
    let terrain = session.cache.level_terrain_mut().unwrap();
    terrain.header = [-0x1800 << 8, 0, 73, 0, 0];
    for cell in &mut terrain.cells {
        cell.height = 0;
    }
    session
}

fn viewport(case: usize) -> NativeWorldViewport {
    NativeWorldViewport {
        origin_raw: if case == 0 { [0; 3] } else { [2_000, 3_000, 0] },
        axes_q31: AXES,
        identity: true,
    }
}

struct CorpusDraw<'a> {
    model_id: usize,
    vars: &'a AnimVars,
    viewport: NativeWorldViewport,
    native: bool,
    sub_h_type: Option<usize>,
}

fn draw_corpus(cache: &ResourceCache, request: CorpusDraw<'_>) -> RecordingRenderer {
    let root = cache.global_model(request.model_id).unwrap();
    let frame = NativeModelFrame::from_actor(request.viewport, ORIGIN, AXES);
    let position = ORIGIN.map(|value| f32::from(value) / 256.0);
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    let empty = SubHExternalFrameDescriptor {
        completion_sound_id: None,
        records: Vec::new(),
    };
    let descriptor = request
        .sub_h_type
        .map(|type_id| {
            cache
                .global_entity_type(type_id)
                .unwrap()
                .sub_h_external_frame_descriptor()
                .unwrap()
        })
        .unwrap_or(empty);
    let mut runtime = SubHRuntimeState::new(descriptor.records.len()).unwrap();
    // Actual authored Sub-H descriptors consume explicit settled caches. This
    // fixture verifies complete presentation, independently of endpoint motion.
    for (index, record) in runtime.records_mut().iter_mut().enumerate() {
        record.flags_raw = 7;
        record.primary_raw = [
            ORIGIN[0] + 128 + index as i16 * 128,
            384,
            ORIGIN[2] - 128 - index as i16 * 128,
        ];
        record.secondary_raw = [
            ORIGIN[0] + 256 + index as i16 * 128,
            640,
            ORIGIN[2] - 256 - index as i16 * 128,
        ];
    }
    let mut stamp = |_| {};
    let mut boundary = None;
    let mut tree =
        ModelTreeRenderer::new_world(&mut renderer, cache, &colors, 100.0 / 256.0, None, 952)
            .with_scene_projection_authority(if request.native {
                SceneProjectionAuthority::Native(
                    NativeScreenProjection::new(
                        [512, 512],
                        [320, 240],
                        [640, 480],
                        ProjectionEffect::None,
                    )
                    .unwrap(),
                )
            } else {
                SceneProjectionAuthority::default()
            })
            .with_view(ModelTreeView {
                position: request
                    .viewport
                    .origin_raw
                    .map(|value| value as f32 / 256.0),
                forward: [0.0, 0.0, 1.0],
            });
    if request.model_id == 41 {
        assert!(
            request.native,
            "PLAYER4 comparison keeps the same native callback and culling owner"
        );
        tree = tree.with_actor_emitter_presentation(ActorEmitterModelPresentation {
            emitter: ActorEmitterExternalFramePresentation {
                descriptor: cache
                    .global_entity_type(46)
                    .unwrap()
                    .projectile_emitter_descriptor()
                    .unwrap(),
                root_model: root,
                root_vars: request.vars,
                frame,
                viewport: request.viewport,
                stamp_origin: &mut stamp,
                last_boundary: &mut boundary,
                current_node_owned: true,
                current_source_points_view: None,
            },
            actor_origin_raw: ORIGIN,
            actor_draw_origin: position,
        });
    } else {
        // Non-callback roots use an empty owner to share authored admission
        // between the native and compatibility lanes. Real limb actors above
        // supply their own descriptor rather than borrowing model parameters.
        tree = tree.with_sub_h_presentation(SubHPresentation {
            runtime: &mut runtime,
            descriptor: &descriptor,
            model_records: &root.records,
            retail_tick: 952,
            origin_raw: ORIGIN,
            origin_world: position,
            body_axes_q31: Some(AXES),
            native_context: request.native.then_some((frame, request.viewport)),
            fallback: None,
            emitter: None,
        });
    }
    tree.draw_linked(
        request.model_id,
        ORIENTATION,
        position,
        8,
        None,
        request.vars,
    );
    assert!(
        tree.native_vertex_failures().is_empty(),
        "model{} native dependency receipts: {:?}",
        request.model_id,
        tree.native_vertex_failures()
    );
    assert!(
        tree.native_edge_failures().is_empty(),
        "model{} native edge receipts: {:?}",
        request.model_id,
        tree.native_edge_failures()
    );
    drop(tree);
    assert!(boundary.is_none(), "PLAYER4 emitter boundary: {boundary:?}");
    renderer
}

fn totals(renderer: &RecordingRenderer) -> (usize, usize, usize) {
    (
        renderer
            .bodies
            .iter()
            .map(|body| body.triangles.len())
            .sum(),
        renderer.bodies.iter().map(|body| body.edges.len()).sum(),
        renderer
            .billboards
            .iter()
            .map(|draw| draw.billboards.len())
            .sum(),
    )
}

fn verify_geometry(renderer: &RecordingRenderer) {
    for body in &renderer.bodies {
        for &slot in body
            .triangles
            .iter()
            .flatten()
            .chain(body.edges.iter().flat_map(|edge| edge.vertices.iter()))
        {
            let index = usize::from(slot);
            assert!(
                body.vertices[index].iter().all(|value| value.is_finite()),
                "type{} submitted a nonfinite endpoint",
                body.vertex_type_flags[index]
            );
            assert_eq!(
                body.vertex_clip[index],
                ModelSlotClip::Clear,
                "controlled dry geometry cannot be sea-band clipped"
            );
            if body.vertex_type_flags[index] == 13 {
                let ModelVertexProjection::WorldPoint(point) = body.vertex_projection[index] else {
                    panic!("tf13 must retain world endpoint custody");
                };
                assert!(point.iter().all(|value| value.is_finite()));
                assert_eq!(
                    point[1], 0.0,
                    "shadow endpoint follows the controlled terrain"
                );
                assert_eq!(
                    body.vertex_surface_origin[index],
                    ModelSurfaceOrigin::ViewPin
                );
            }
        }
    }
    for draw in &renderer.billboards {
        for sprite in &draw.billboards {
            let index = usize::from(sprite.vertex);
            assert!(draw.vertices[index].iter().all(|value| value.is_finite()));
            assert_eq!(draw.vertex_clip[index], ModelSlotClip::Clear);
        }
    }
}

fn player_vars(callback: u8, barrel: u16) -> AnimVars {
    let mut craft = PlayerCraft::new();
    craft.select_weapon_callback(callback);
    craft.gun_barrel = f32::from(barrel);
    craft.set_primary_joint_pulses([u16::MAX; 2]);
    craft.anim_vars()
}

#[v2k_test_support::retail_test]
fn complete_player4_keeps_the_common_hull_and_footprint_across_native_loadouts() {
    let session = corpus();
    let root = session.cache.global_model(41).unwrap();
    for view_case in 0..2 {
        for barrel in [0, 0x1000] {
            let draws = [0, 4, 5].map(|callback| {
                let vars = player_vars(callback, barrel);
                let authored = root.materialize(&vars);
                assert_eq!(
                    authored
                        .instances
                        .iter()
                        .map(|child| child.model_id)
                        .collect::<Vec<_>>(),
                    [
                        42,
                        44,
                        44,
                        44,
                        45,
                        45,
                        match callback {
                            0 => 51,
                            4 => 56,
                            _ => 55,
                        },
                        match callback {
                            0 => 51,
                            4 => 56,
                            _ => 55,
                        }
                    ]
                );
                let frame = NativeModelFrame::from_actor(viewport(view_case), ORIGIN, AXES);
                let expected_root = authored
                    .triangles
                    .iter()
                    .zip(&authored.face_cull)
                    .enumerate()
                    .filter_map(|(face_index, (triangle, policy))| {
                        let admitted = match *policy {
                            ModelFaceCull::AlwaysVisible | ModelFaceCull::Unresolved => true,
                            ModelFaceCull::Plane(plane) | ModelFaceCull::LiveAdmitted(plane) => {
                                frame.face_plane_visible(plane).unwrap()
                            }
                        };
                        admitted.then_some((face_index, *triangle))
                    })
                    .collect::<Vec<_>>();
                let draw = draw_corpus(
                    &session.cache,
                    CorpusDraw {
                        model_id: 41,
                        vars: &vars,
                        viewport: viewport(view_case),
                        native: true,
                        sub_h_type: None,
                    },
                );
                verify_geometry(&draw);
                assert_eq!(
                    draw.bodies[0].triangles.len(),
                    expected_root.len(),
                    "all authored visible hull and footprint faces survive callback{callback}"
                );
                // Live admission resolves only referenced corners, so its
                // compact vertex indices differ from intrinsic all-face data.
                // Compare actual source geometry and authored face metadata.
                let body = &draw.bodies[0];
                for (index, (triangle, (source_index, source_triangle))) in
                    body.triangles.iter().zip(&expected_root).enumerate()
                {
                    assert_eq!(body.face_uvs[index], authored.face_uvs[*source_index]);
                    assert_eq!(
                        body.face_shading[index],
                        authored.face_shading[*source_index]
                    );
                    for (&corner, &source_corner) in triangle.iter().zip(source_triangle) {
                        let corner = usize::from(corner);
                        let source_corner = usize::from(source_corner);
                        let kind = authored.vertex_type_flags[source_corner];
                        assert_eq!(body.vertex_type_flags[corner], kind);
                        if !matches!(kind, 12 | 13) {
                            assert_eq!(
                                body.vertices[corner], authored.vertices[source_corner],
                                "callback{callback} retains each visible authored hull corner"
                            );
                        }
                    }
                }
                let shadows = draw.bodies[0]
                    .triangles
                    .iter()
                    .filter(|triangle| {
                        triangle
                            .iter()
                            .all(|slot| draw.bodies[0].vertex_type_flags[usize::from(*slot)] == 13)
                    })
                    .count();
                assert_eq!(shadows, 8);
                assert!(
                    expected_root.len() - shadows > 40,
                    "a small surviving shadow cannot stand in for the main hull"
                );
                assert_eq!(
                    draw.bodies.len(),
                    if callback == 4 { 12 } else { 10 },
                    "the complete selected weapon hierarchy is retained"
                );
                assert_eq!(totals(&draw).1, 0);
                assert_eq!(totals(&draw).2, 0);
                draw
            });
            for draw in &draws[1..] {
                for (body, common) in draw.bodies[..8].iter().zip(&draws[0].bodies[..8]) {
                    assert_eq!(body.triangles, common.triangles);
                    assert_eq!(body.face_vertices, common.face_vertices);
                    assert_eq!(body.materials, common.materials);
                    assert_eq!(body.transform.orientation, common.transform.orientation);
                    assert_eq!(body.transform.position, common.transform.position);
                    assert_eq!(body.transform.scale, common.transform.scale);
                    for &slot in body.triangles.iter().flatten() {
                        let index = usize::from(slot);
                        assert_eq!(
                            body.vertices[index], common.vertices[index],
                            "changing the callback cannot change common hull/footprint geometry"
                        );
                        assert_eq!(
                            body.vertex_projection[index],
                            common.vertex_projection[index]
                        );
                    }
                }
            }
            assert!(
                draws[1].bodies[8..]
                    .iter()
                    .map(|body| body.triangles.len())
                    .sum::<usize>()
                    > draws[2].bodies[8..]
                        .iter()
                        .map(|body| body.triangles.len())
                        .sum::<usize>(),
                "tube guns and their barrels retain geometry distinct from the missile loadout"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn complete_retail_native_models_preserve_body_edges_and_billboards_with_shared_admission() {
    let session = corpus();
    // Family representatives include generated shadow sources, animated factory
    // children, linked wing chains and authored live limb endpoint consumers.
    for (model_id, sub_h_type) in [
        (83, None),
        (195, None),
        (201, None),
        (218, None),
        (221, None),
        (278, None),
        (351, None),
        (436, None),
        (440, None),
        (444, None),
        (448, None),
        (452, None),
        (456, None),
        (492, None),
        (256, Some(17)),
        (267, Some(26)),
        (302, Some(47)),
    ] {
        for phase in [0, 0x8000, 0xffff] {
            let mut vars = AnimVars::default();
            vars.dynamic[1..8].fill(phase);
            vars.registers[1..8].fill(phase as u16);
            for view_case in 0..2 {
                let compatibility = draw_corpus(
                    &session.cache,
                    CorpusDraw {
                        model_id,
                        vars: &vars,
                        viewport: viewport(view_case),
                        native: false,
                        sub_h_type,
                    },
                );
                let native = draw_corpus(
                    &session.cache,
                    CorpusDraw {
                        model_id,
                        vars: &vars,
                        viewport: viewport(view_case),
                        native: true,
                        sub_h_type,
                    },
                );
                verify_geometry(&native);
                assert_eq!(totals(&native), totals(&compatibility), "model{model_id}, phase{phase}, view{view_case}: differing raw-only diagnostic culling is deliberately excluded");
                assert_eq!(native.bodies.len(), compatibility.bodies.len());
                for (body, reference) in native.bodies.iter().zip(&compatibility.bodies) {
                    assert_eq!(
                        body.triangles, reference.triangles,
                        "model{model_id} complete body faces"
                    );
                    assert_eq!(
                        body.edges, reference.edges,
                        "model{model_id} complete authored edges"
                    );
                    assert_eq!(body.face_vertices, reference.face_vertices);
                    assert_eq!(body.materials, reference.materials);
                }
                if model_id == 278 {
                    assert_eq!(totals(&native), (2, 0, 1));
                }
                if model_id == 492 {
                    assert_eq!(totals(&native), (4, 0, 0));
                }
                if model_id == 83 {
                    assert_eq!(totals(&native), (0, 0, 3));
                }
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn retail_spatial_generators_receive_native_view_for_both_parities_and_current_phases() {
    let session = corpus();
    for model_id in [83, 278, 492] {
        let model = session.cache.global_model(model_id).unwrap();
        for view_case in 0..2 {
            let viewport = viewport(view_case);
            let frame = NativeModelFrame::from_actor(viewport, ORIGIN, AXES);
            let resolver = WorldModelVertexResolver {
                surface: session
                    .cache
                    .terrain()
                    .map(|terrain| WorldSurfaceProjection::new(terrain, 952)),
                transform: ModelTransform {
                    orientation: ORIENTATION,
                    position: ORIGIN.map(|value| f32::from(value) / 256.0),
                    scale: 100.0 / 256.0,
                },
                view: Some(ModelTreeView {
                    position: viewport.origin_raw.map(|value| value as f32 / 256.0),
                    forward: [0.0, 0.0, 1.0],
                }),
                native_view: Some(frame.view_selection()),
                native_context: Some((frame, viewport)),
                native_edges_required: true,
                model,
                external_owner: None,
                external_points: RefCell::new(HashMap::new()),
                native_vertex_failures: RefCell::new(Vec::new()),
            };
            for phase in [0, 0x8000, 0xffff] {
                let mut vars = AnimVars::default();
                vars.dynamic[1..8].fill(phase);
                vars.registers[1..8].fill(phase as u16);
                for (index, record) in model
                    .records
                    .iter()
                    .enumerate()
                    .filter(|(_, record)| matches!(record[0], 2 | 5 | 6 | 9))
                {
                    for parity in [0, 1] {
                        let slot = index as u16 * 2 + parity;
                        let point = model
                            .resolve_slot_with_context(
                                slot,
                                ModelMaterializationContext {
                                    vars: &vars,
                                    linked: None,
                                    vertex_resolver: Some(&resolver),
                                    view_selection: resolver.native_view.unwrap(),
                                },
                            )
                            .unwrap_or_else(|| {
                                panic!(
                                    "model{model_id} tf{} source slot{slot} must resolve",
                                    record[0]
                                )
                            });
                        assert!(point.position_raw.iter().all(|value| value.is_finite()));
                        assert!(point.native_view_point.is_some(), "model{model_id} tf{} slot{slot} phase{phase} lost native VIEW ownership", record[0]);
                    }
                }
            }
            assert!(resolver.native_vertex_failures.borrow().is_empty());
        }
    }
}
