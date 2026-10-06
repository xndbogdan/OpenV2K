//! Type46's standalone Sub-E follows the reached PLAYER4 mounted gun frame.
use super::*;
use crate::actor_emitter_external_frame::{
    ActorEmitterExternalFrameBoundary, ActorEmitterExternalFramePoint,
    ActorEmitterExternalFramePresentation, ActorEmitterModelPresentation,
};
use crate::entity::{
    AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources, EntityManager,
};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
use crate::player::PlayerCraft;
use crate::world_fx::WorldFx;
use v2k_formats::models::{ModelNativeInstanceFrame, ModelNativeMountCommand};
use v2k_render::projection::{NativeScreenProjection, ProjectionEffect, SceneProjectionAuthority};

struct PlayerDraw {
    session: crate::session::GameSession,
    frame: NativeModelFrame,
    viewport: NativeWorldViewport,
    origin: [i16; 3],
    orientation: [[f32; 3]; 3],
}

fn player_draw() -> PlayerDraw {
    let mut session = native_painter_session();
    session.load_level_by_id(17, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect::<Vec<_>>();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 5,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.level_terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: [1_000, 1_024, 2_000],
                heading_raw: 0,
            }),
            retail_tick: 0,
        },
        &mut WorldFx::new(),
    )
    .unwrap();
    // Enter the recovered13F70 body-basis writer before presentation. Fresh
    // player allocation itself deliberately does not claim that later phase.
    manager.player_mut().unwrap().apply_d720_euler_body_basis();
    let player = manager.player().unwrap();
    assert_eq!(player.entity_type, 46);
    let RetailRuntimeValue::Known(basis) = player.physical_body_basis_q31() else {
        panic!("authored player must retain its native body basis")
    };
    let origin = player.position_raw();
    let viewport = NativeWorldViewport {
        origin_raw: [0; 3],
        axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
        identity: true,
    };
    PlayerDraw {
        frame: NativeModelFrame::from_actor(
            viewport,
            origin,
            [basis.lateral, basis.up, basis.forward],
        ),
        orientation: basis.orientation_world_from_model(),
        viewport,
        origin,
        session,
    }
}

fn player_vars(callback: u8, barrel: u16) -> AnimVars {
    let mut craft = PlayerCraft::new();
    craft.select_weapon_callback(callback);
    craft.gun_barrel = f32::from(barrel);
    craft.set_primary_joint_pulses([u16::MAX; 2]);
    let vars = craft.anim_vars();
    assert_eq!(vars.dynamic[1], i32::from(barrel));
    vars
}

fn draw_buffered(
    draw: &PlayerDraw,
    vars: &AnimVars,
) -> (
    Vec<ActorEmitterExternalFramePoint>,
    Option<ActorEmitterExternalFrameBoundary>,
) {
    let root = draw.session.cache.global_model(41).unwrap();
    let ty = draw.session.cache.global_entity_type(46).unwrap();
    assert!(ty.sub_h_external_frame_descriptor().is_none());
    let descriptor = ty.projectile_emitter_descriptor().unwrap();
    assert_eq!(
        [
            descriptor.raw_word_at_0x12,
            descriptor.alternate_emitter_raw
        ],
        [18, 20]
    );
    let colors = ModelMaterialCache::new();
    let mut renderer = RecordingRenderer::default();
    let mut buffer = ModelTreeSubmissionBuffer::default();
    let stamps = RefCell::new(Vec::new());
    let mut stamp = |point| stamps.borrow_mut().push(point);
    let mut boundary = None;
    let world = draw.origin.map(|v| f32::from(v) / 256.0);
    let mut tree = ModelTreeRenderer::new_world(
        &mut renderer,
        &draw.session.cache,
        &colors,
        100.0 / 256.0,
        None,
        0,
    )
    .with_scene_projection_authority(SceneProjectionAuthority::Native(
        NativeScreenProjection::new([512, 512], [320, 240], [640, 480], ProjectionEffect::None)
            .unwrap(),
    ))
    .with_submission_buffer(&mut buffer)
    .with_view(ModelTreeView {
        position: [0.0; 3],
        forward: [0.0, 0.0, 1.0],
    })
    .with_actor_emitter_presentation(ActorEmitterModelPresentation {
        emitter: ActorEmitterExternalFramePresentation {
            descriptor,
            root_model: root,
            root_vars: vars,
            frame: draw.frame.clone(),
            viewport: draw.viewport,
            stamp_origin: &mut stamp,
            last_boundary: &mut boundary,
            current_node_owned: true,
            current_source_points_view: None,
        },
        actor_origin_raw: draw.origin,
        actor_draw_origin: world,
    });
    tree.draw_linked(41, draw.orientation, world, 8, None, vars);
    drop(tree);
    let before_flush = stamps.borrow().clone();
    assert!(buffer.len() > 0, "real PLAYER4 geometry was buffered");
    buffer.flush(&mut renderer, None);
    assert_eq!(
        *stamps.borrow(),
        before_flush,
        "buffer drain must not execute Sub-E again"
    );
    (stamps.into_inner(), boundary)
}

#[v2k_test_support::retail_test]
fn native_type46_callback4_stamps_each_current_tubegun_side_and_raw_barrel_mount() {
    let draw = player_draw();
    let root = draw.session.cache.global_model(41).unwrap();
    let tube = draw.session.cache.global_model(56).unwrap();
    assert_eq!(tube.records[9], [0, 28, 0, 160]);
    assert_eq!(tube.records[10], [0, 28, 0, 160]);
    assert_eq!(tube.records[15][0], 14);
    assert_eq!(tube.records[16][0], 14);
    let mut origins = Vec::new();
    for barrel in [0_u16, 0x1000] {
        let vars = player_vars(4, barrel);
        let guns = root
            .materialize(&vars)
            .instances
            .into_iter()
            .filter(|i| i.model_id == 56)
            .collect::<Vec<_>>();
        assert_eq!(guns.len(), 2);
        assert_eq!(
            guns.iter()
                .map(|i| (i.attach_slot, i.registers[3]))
                .collect::<Vec<_>>(),
            [(84, 1), (85, 0)]
        );
        let mut expected = Vec::new();
        for gun in &guns {
            let Some(ModelNativeInstanceFrame::Mount(commands)) = gun.native_frame.as_ref() else {
                panic!("PLAYER4 side gun owns its authored mount")
            };
            assert!(commands
                .iter()
                .any(|c| matches!(c, ModelNativeMountCommand::Parent(16 | 48))));
            assert!(commands.iter().any(|c| matches!(c, ModelNativeMountCommand::Rotate { axis: 0, angle } if *angle == barrel)));
            let child = draw.frame.child(root, gun, &vars).unwrap();
            // Tubegun's command branch reaches tf14 slot30/selector0 for
            // inherited side1, and slot32/selector1 for inherited side0.
            let selector = 1 - gun.registers[3];
            let slot = if selector == 0 { 18 } else { 20 };
            let view = child
                .resolve_type0_slot_view_raw(&tube.records, slot)
                .unwrap();
            expected.push((selector, slot, draw.viewport.view_point_to_world(view)));
            assert_ne!(
                Some(view),
                draw.frame.resolve_type0_slot_view_raw(&root.records, slot),
                "a child cannot borrow PLAYER4's source slot cache"
            );
        }
        let (points, boundary) = draw_buffered(&draw, &vars);
        assert_eq!(boundary, None);
        assert_eq!(
            points
                .iter()
                .map(|p| (p.emitter_index, p.source_slot, p.position_raw))
                .collect::<Vec<_>>(),
            expected
        );
        origins.push(points.iter().map(|p| p.position_raw).collect::<Vec<_>>());
    }
    assert_ne!(
        origins[0], origins[1],
        "the authored raw barrel joint rotates the source point"
    );
    for (level, raised) in origins[0].iter().zip(&origins[1]) {
        let displacement = std::array::from_fn(|axis| raised[axis] - level[axis]);
        assert!(
            crate::hover::dot_q31(draw.frame.axes_view_q31[1], displacement.map(|v| v as i16)) > 0,
            "positive barrel elevation moves both muzzle points along body-up"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type46_callback5_dormant_missile_tf14_does_not_stamp_an_origin() {
    let draw = player_draw();
    let vars = player_vars(5, 0x1000);
    let root = draw.session.cache.global_model(41).unwrap();
    let missile = draw.session.cache.global_model(55).unwrap();
    assert!(missile.records.iter().any(|record| record[0] == 14));
    assert_eq!(
        root.materialize(&vars)
            .instances
            .iter()
            .filter(|i| i.model_id == 55)
            .count(),
        2
    );
    let (points, boundary) = draw_buffered(&draw, &vars);
    assert!(
        points.is_empty(),
        "an allocated tf14 record does not prove command execution"
    );
    assert_eq!(
        boundary, None,
        "no callback was requested; flag1 can retain native centre fallback"
    );
}

#[v2k_test_support::retail_test]
fn native_player4_generated_shadow_sources_retain_the_complete_ground_footprint() {
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    use crate::native_model_frame::NativeSlotSurface;
    use v2k_formats::models::{ModelSlotClip, ModelSurfaceOrigin};

    let mut draw = player_draw();
    draw.session.load_level_by_id(13, 1).unwrap();
    let root = draw.session.cache.global_model(41).unwrap();
    assert_eq!(root.name.as_deref(), Some("player4"));
    let expected_triangles = root
        .triangles
        .iter()
        .filter(|triangle| {
            triangle
                .iter()
                .all(|&index| root.vertex_type_flags[usize::from(index)] == 13)
        })
        .count();
    assert_eq!(expected_triangles, 8, "authored PLAYER4 root footprint");
    let generated_sources = root
        .records
        .iter()
        .filter(|record| record[0] == 13)
        .map(|record| record[1] as u16)
        .filter(|&slot| root.records[usize::from(slot >> 1)][0] == 5)
        .collect::<Vec<_>>();
    assert_eq!(generated_sources.len(), 4, "footprint uses tf5 sources");
    let terrain = draw.session.cache.terrain().unwrap();
    let colors = ModelMaterialCache::new();
    let basis = Type9BodyBasis::from_angle_words(0x4000, 0, 0);
    let descriptor = draw
        .session
        .cache
        .global_entity_type(46)
        .unwrap()
        .projectile_emitter_descriptor()
        .unwrap();

    // Real dry terrain at different cell fractions exercises the callback;
    // these constructed poses do not assert a retail camera/timeline match.
    for (x, z) in [(75.25_f32, 60.25_f32), (183.75, 128.5)] {
        let [x, z] = [x, z].map(|v| (v * 256.0) as i32 as i16);
        let floor = terrain.bilinear_height_raw(x, z);
        assert!(floor > terrain.sea_level_raw(), "fixture must be dry");
        let origin = [x, floor.wrapping_add(256), z];
        let eye = [
            x.wrapping_add(900),
            origin[1].wrapping_add(850),
            z.wrapping_sub(1800),
        ];
        let viewport = crate::chase_camera::native_viewport_from_spring_points(eye, origin);
        let frame = NativeModelFrame::from_actor(
            viewport,
            origin,
            [basis.lateral, basis.up, basis.forward],
        );
        let world = origin.map(|v| f32::from(v) / 256.0);
        let surface = WorldSurfaceProjection::new(terrain, 73).with_waves_enabled(false);
        for callback in [4, 5] {
            let vars = player_vars(callback, 0x1000);
            let mut mirrored_source_changes = false;
            for &source in &generated_sources {
                let points = [0, 1].map(|odd| {
                    frame
                        .resolve_model_slot_with_surface(
                            root,
                            &vars,
                            source ^ odd,
                            NativeSlotSurface::World {
                                viewport,
                                terrain: Some(terrain),
                            },
                        )
                        .expect("even and mirrored tf5 source caches must be owned")
                });
                // Centerline midpoints can coincide; lateral sources must
                // still retain independently resolved reflected caches.
                mirrored_source_changes |= points[0] != points[1];
            }
            assert!(mirrored_source_changes, "lateral tf5 sources are mirrored");
            let mut renderer = RecordingRenderer::default();
            let mut buffer = ModelTreeSubmissionBuffer::default();
            let mut boundary = None;
            let mut stamp = |_| {};
            ModelTreeRenderer::new_world(
                &mut renderer,
                &draw.session.cache,
                &colors,
                100.0 / 256.0,
                None,
                73,
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
            .with_view(ModelTreeView {
                position: viewport.origin_raw.map(|v| v as f32 / 256.0),
                forward: viewport.axes_q31[2].map(|v| v as f32 / i32::MAX as f32),
            })
            .with_actor_emitter_presentation(ActorEmitterModelPresentation {
                emitter: ActorEmitterExternalFramePresentation {
                    descriptor,
                    root_model: root,
                    root_vars: &vars,
                    frame: frame.clone(),
                    viewport,
                    stamp_origin: &mut stamp,
                    last_boundary: &mut boundary,
                    current_node_owned: true,
                    current_source_points_view: None,
                },
                actor_origin_raw: origin,
                actor_draw_origin: world,
            })
            // Child bodies cannot mask a missing root footprint.
            .draw_linked(
                41,
                basis.orientation_world_from_model(),
                world,
                0,
                None,
                &vars,
            );
            buffer.flush(&mut renderer, Some(surface));
            assert_eq!(boundary, None);
            let mut footprint_triangles = 0;
            for body in &renderer.bodies {
                for triangle in &body.triangles {
                    if !triangle
                        .iter()
                        .all(|&index| body.vertex_type_flags[usize::from(index)] == 13)
                    {
                        continue;
                    }
                    footprint_triangles += 1;
                    assert_eq!(
                        body.surface_resolution,
                        ModelSurfaceResolution::ContextResolved
                    );
                    assert!(body.has_world_surface);
                    for &index in triangle {
                        let index = usize::from(index);
                        assert_eq!(body.vertex_clip[index], ModelSlotClip::Clear);
                        assert_eq!(
                            body.vertex_surface_origin[index],
                            ModelSurfaceOrigin::ViewPin
                        );
                        assert!(body.vertices[index].iter().all(|v| v.is_finite()));
                        let ModelVertexProjection::WorldPoint(point) =
                            body.vertex_projection[index]
                        else {
                            panic!("native footprint corner must retain world ownership");
                        };
                        assert!(point.iter().all(|v| v.is_finite()));
                        let sample = terrain.bilinear_height_raw(
                            (point[0] * 256.0) as i32 as i16,
                            (point[2] * 256.0) as i32 as i16,
                        );
                        assert_eq!(point[1], f64::from(sample) / 256.0);
                    }
                }
            }
            assert_eq!(
                footprint_triangles, expected_triangles,
                "callback{callback} at {origin:?}: all root footprint faces must survive"
            );
        }
    }
}
