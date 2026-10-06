//! Fresh Level-1 Type-47 Guard graphs must hand off to Pursuing and walk
//! Chase once the player is in the authored 1792-unit cube.
//!
//! Live player ids must be nonzero: Chase treats handle 0 as no target.
//! First query is `V200003.run` `full_reset` for seeds `0x2B/0x2C/0x2D`.

use v2k_formats::models::ModelEdgeStyle;
use v2k_game::entity::EntityManager;
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackResult;
use v2k_game::ordinary_type47_death_live::{
    TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
    TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR,
    TYPE47_COMMON_DYING_SUB_H_RECORDS,
};
use v2k_game::ordinary_type47_live::{
    drain_live_ordinary_type47_shot_queues, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
    FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES,
};
use v2k_game::session::GameSession;
use v2k_game::type47_chase_mover::Type47ChaseMoverBlock;
use v2k_game::type47_scheduler_production::{
    tick_ordinary_type47_scheduler_owner, OrdinaryType47SchedulerOwner,
    OrdinaryType47SchedulerProductionOutcome, Type47ChaseVisitResult, Type47WanderVisitResult,
};
use v2k_game::world_fx::{ParticleEnvironment, WorldFx};
use v2k_game::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};

fn type47_model_records(session: &GameSession) -> Option<&[[i16; 4]]> {
    session
        .cache
        .global_model(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
        .map(|model| model.records.as_slice())
}

fn newant_leg_gate_stats(
    session: &GameSession,
    entity: &v2k_game::entity::Entity,
    model: &v2k_formats::models::ModelEntry,
    retail_tick: u32,
) -> v2k_render::edge_quads::EdgeGateStats {
    let descriptor = session
        .cache
        .global_entity_type(47)
        .and_then(|record| record.sub_h_external_frame_descriptor())
        .expect("type 47 Sub-H descriptor");
    let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
        panic!("spawn 11 must own live Sub-H");
    };
    let axes = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
        RetailRuntimeValue::Unresolved => None,
    };
    let origin_raw = entity.position_raw();
    let origin_world = entity.position;
    // Live draw uses the camera-relative image, not the wrapped [0,256)
    // store. A hive at Z 127 with the craft near 0 is the torus case.
    let cam_store = [0.5_f32, 20.0, 0.5];
    let draw_pos = [
        cam_store[0] + v2k_core::world::delta(origin_world[0], cam_store[0]),
        origin_world[1],
        cam_store[2] + v2k_core::world::delta(origin_world[2], cam_store[2]),
    ];
    let terrain = session.cache.terrain();
    let points = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        sub_h,
        &descriptor,
        &model.records,
        origin_raw,
        draw_pos,
        axes,
        |x, z| terrain.map_or(0, |grid| grid.bilinear_height_raw(x, z)),
    )
    .expect("Sub-H geometry");
    let vars = entity.presentation_anim_vars(retail_tick);
    let matured = model.materialize(&vars);
    let extra_scale = 100.0 / 256.0;
    let scale = extra_scale / 100.0;
    let orientation = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => panic!("missing FUN_00413F70 basis"),
    };
    let external = v2k_render::ExternalFrameMode::SelectorWorldPoints(points);
    let surface =
        terrain.map(|grid| v2k_render::WorldSurfaceProjection::new(grid, retail_tick as i32));
    let resolved_local: Vec<Option<[f32; 3]>> = matured
        .vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            let raw = [vertex[0] as f32, vertex[1] as f32, vertex[2] as f32];
            if matured.vertex_type_flags.get(index) == Some(&13) {
                return surface.and_then(|projection| {
                    let resolved = v2k_render::gl_backend::retail_world_surface_vertex(
                        raw,
                        orientation,
                        draw_pos,
                        extra_scale,
                        projection,
                    );
                    (resolved.clip == v2k_formats::models::ModelSlotClip::Clear)
                        .then(|| resolved.position_raw.map(|value| value as f32))
                });
            }
            Some(raw)
        })
        .collect();
    let resolved: Vec<Option<[f32; 3]>> = (0..matured.vertices.len())
        .map(|index| {
            v2k_render::gl_backend::edge_endpoint_world(
                index,
                &matured.vertices,
                &matured.vertex_type_flags,
                &resolved_local,
                external,
                orientation,
                draw_pos,
                scale,
            )
        })
        .collect();
    let widths: Vec<(u16, u16)> = matured
        .edges
        .iter()
        .map(|edge| match edge.style {
            ModelEdgeStyle::Sprite { sprite_id, .. } => session
                .cache
                .global_sprite(sprite_id)
                .map(|(_, entry)| (entry.flags as u16, (entry.flags >> 16) as u16))
                .unwrap_or((0, 0)),
            ModelEdgeStyle::Palette { .. } => (0, 0),
        })
        .collect();
    let materials = vec![
        v2k_render::FaceMaterial {
            palette_rgb555: None,
            color: [1.0; 3],
            emissive: [0.0; 3],
            texture: None,
            blend: v2k_render::WorldSpriteBlend::Masked,
            flat_shade_row: 31,
        };
        matured.edges.len()
    ];
    let mut camera = v2k_render::Camera::new(1920.0 / 1080.0);
    camera.position = [draw_pos[0] + 11.5, draw_pos[1] + 8.0, draw_pos[2] - 24.0];
    camera.yaw = std::f32::consts::PI;
    camera.pitch = -0.7;
    camera.left_handed = true;
    camera.fov = 60.0_f32.to_radians();
    let view = camera.view_matrix();
    let proj = camera.projection_matrix();
    let basis = [
        [view[0], view[4], view[8]],
        [view[1], view[5], view[9]],
        [view[2], view[6], view[10]],
    ];
    let intr = v2k_render::edge_quads::ScreenIntrinsics {
        width_px: 1920,
        height_px: 1080,
        scale_x: proj[0] * 960.0,
        scale_y: proj[5] * 540.0,
        center_x: 960.0,
        center_y: 540.0,
        clip_near: camera.near,
        clip_far: camera.far,
        projection_effect: v2k_render::projection::ProjectionEffect::None,
    };
    v2k_render::edge_quads::select_edge_quads_with_stats(
        &matured.edges,
        &widths,
        &materials,
        &resolved,
        &intr,
        camera.position,
        basis,
    )
    .1
}

#[v2k_test_support::retail_test]
fn newant_stream_is_mostly_sub_h_edges_not_solid_faces() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("aux");
    session.load_level_by_id(13, 1).expect("level 1");
    let newant = session
        .cache
        .global_model(v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
        .expect("newant");
    let matured = newant.materialize(&v2k_formats::models::AnimVars::default());
    let mut type13_ribbons = 0usize;
    let mut type14_legs = 0usize;
    let mut palette = 0usize;
    for edge in &matured.edges {
        let t0 = matured.vertex_type_flags[edge.vertices[0] as usize];
        let t1 = matured.vertex_type_flags[edge.vertices[1] as usize];
        match edge.style {
            ModelEdgeStyle::Sprite { .. } => match (t0, t1) {
                (13, 13) => type13_ribbons += 1,
                (0, 14) | (14, 0) | (14, 14) => type14_legs += 1,
                _ => {}
            },
            ModelEdgeStyle::Palette { .. } => palette += 1,
        }
    }
    assert_eq!(
        type13_ribbons, 6,
        "six type-13 0x22 ribbons plant on terrain as the ground blob"
    );
    assert_eq!(
        type14_legs, 12,
        "twelve type-14 0x22 segments are the visible newant legs"
    );
    assert_eq!(palette, 14, "fourteen 0x02 hairlines complete the legs");
    let all_type13_faces = matured
        .triangles
        .iter()
        .filter(|tri| {
            tri.iter()
                .all(|&index| matured.vertex_type_flags[index as usize] == 13)
        })
        .count();
    let mixed_or_body = matured.triangles.len() - all_type13_faces;
    assert!(
        mixed_or_body > 0,
        "newant must keep a non-footprint body mesh, faces={} type13_only={all_type13_faces}",
        matured.triangles.len()
    );
    assert!(
        matured.instances.is_empty(),
        "newant legs are on the root stream, not a child instance"
    );
    let nrecords = newant.records.len();
    for (index, record) in v2k_game::ordinary_type47_death_live::TYPE47_COMMON_DYING_SUB_H_RECORDS
        .iter()
        .enumerate()
    {
        for (role, slot) in record.vertex_refs.iter().copied().enumerate() {
            let rec_index = usize::from(slot) >> 1;
            assert!(
                rec_index < nrecords,
                "Sub-H record {index} ref {role} slot {slot} is past model.records ({nrecords})"
            );
            assert_eq!(
                newant.records[rec_index][0], 0,
                "Sub-H record {index} ref {role} slot {slot} must be type-0, flag={}",
                newant.records[rec_index][0]
            );
        }
    }
}

#[test]
fn type14_draw_delta_is_a_wrapping_short_at_the_level1_z_seam() {
    let origin_world = [127.0_f32, 4.0, 127.0];
    let origin_raw = [
        ((127.0_f32 * 256.0).round() as i32) as i16,
        ((4.0_f32 * 256.0).round() as i32) as i16,
        ((127.0_f32 * 256.0).round() as i32) as i16,
    ];
    assert_eq!(origin_raw[2], 0x7F00_u16 as i16, "TTD spawn 11 +0x9A");
    let local_z = 1024_i16;
    let endpoint = [
        i32::from(origin_raw[0]),
        i32::from(origin_raw[1]),
        i32::from(origin_raw[2].wrapping_add(local_z)),
    ];
    let draw =
        v2k_game::sub_h_external_frame::endpoint_as_draw_world(origin_world, origin_raw, endpoint);
    assert!(
        (draw[2] - (127.0 + f32::from(local_z) / 256.0)).abs() < 1.0e-4,
        "seam foot must stay +4 world, draw={draw:?}"
    );
    let naive = origin_world[2] + (endpoint[2] - i32::from(origin_raw[2])) as f32 / 256.0;
    assert!(
        (naive - draw[2]).abs() > 200.0,
        "sign-extended i32 subtract is the sky-cable path ({naive} vs {})",
        draw[2]
    );
}

#[v2k_test_support::retail_test]
fn published_guard_graphs_fire_when_the_player_enters_range() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();

    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let player_id = manager.player().expect("persistent type-46 player").id;
    assert_ne!(
        player_id, 0,
        "live player id 0 is retail's null Chase handle"
    );
    let player_capability = manager.player().unwrap().capability_flags;
    assert_ne!(
        player_capability & 5,
        0,
        "type-46 capability {player_capability:#x} must satisfy Guard filter 5"
    );
    assert_ne!(
        manager
            .player()
            .unwrap()
            .collision
            .state_flags_at_0x08
            .known_mask(),
        u32::MAX,
        "constructor SURFACE_STATE_MASK bits stay unknown; FUN_00403490 must not require a full +0x08 mask"
    );

    let publications = manager.fresh_level1_type47_initial_productions().to_vec();
    assert_eq!(
        publications
            .iter()
            .map(|publication| publication.authored_spawn_index)
            .collect::<Vec<_>>(),
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
    );

    let mut fired = 0usize;
    for publication in publications {
        let spawn = publication.authored_spawn_index;
        let ant_id = publication.entity_id;
        let ant_position = manager
            .entity_mut(ant_id)
            .expect("published Type-47")
            .position_raw();
        // Heading 0 faces +X (`HoverBasis::from_angle_words`); place the
        // player one raw unit ahead so Aim's forward half-space accepts.
        manager.entity_mut(ant_id).unwrap().heading = 0.0;
        manager.player_mut().unwrap().set_motion_raw(
            [
                ant_position[0].wrapping_add(0x100),
                ant_position[1],
                ant_position[2],
            ],
            [0; 3],
        );

        let metadata = &type_metadata[47];
        assert_eq!(
            metadata.sub_a_propulsion_descriptor,
            v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR
            )),
            "live type-47 Sub-A"
        );
        assert_eq!(
            metadata.sub_b_lateral_descriptor,
            v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR
            )),
            "live type-47 Sub-B"
        );
        assert_eq!(
            metadata.sub_c_lift_descriptor,
            v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(
                TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR
            )),
            "live type-47 Sub-C"
        );
        let v2k_game::entity_collision_state::RetailRuntimeValue::Known(Some(sub_h)) =
            &metadata.sub_h_external_frame_descriptor
        else {
            panic!("live type-47 Sub-H missing")
        };
        assert_eq!(sub_h.records.as_slice(), TYPE47_COMMON_DYING_SUB_H_RECORDS);

        let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication)
            .unwrap_or_else(|error| panic!("spawn {spawn} adopt: {error:?}"));
        let tick = tick_ordinary_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            session.cache.terrain(),
            type47_model_records(&session),
            19_500,
            &mut |_: &mut WorldFx| 0,
        );
        let owner = match tick.outcome {
            OrdinaryType47SchedulerProductionOutcome::Acquisition {
                result:
                    GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { candidate, .. },
                wander,
                same_pass_aim: Some(Ok(_)),
                ..
            } if candidate.id == player_id => {
                assert!(
                    !matches!(
                        wander.result,
                        Type47WanderVisitResult::CommonMoverBlocked(
                            Type47ChaseMoverBlock::SubDFirstQueryUnavailable
                                | Type47ChaseMoverBlock::UnexpectedSubA
                                | Type47ChaseMoverBlock::UnexpectedSubB
                                | Type47ChaseMoverBlock::UnexpectedSubC
                                | Type47ChaseMoverBlock::UnexpectedSubH
                                | Type47ChaseMoverBlock::BodyBasisUnavailable
                                | Type47ChaseMoverBlock::SubDSteeringUnavailable
                        )
                    ),
                    "spawn {spawn} Guard wander must apply FUN_00401430, got {:?}",
                    wander.result
                );
                let owner = tick
                    .retained_owner
                    .expect("ADE0 keeps the pursuing owner");
                let entity = manager
                    .iter_all()
                    .find(|entity| entity.id == ant_id)
                    .expect("spawn remains");
                let Some(v2k_game::actor_task_dispatcher::ActorTaskRuntime::ChaseTarget(chase)) =
                    entity.actor_task_state(v2k_game::actor_task_owner::ActorTaskSlot::Primary)
                else {
                    panic!("spawn {spawn} Primary should be Chase after ADE0")
                };
                assert_eq!(
                    chase.target_id(),
                    player_id,
                    "spawn {spawn} Chase must keep the player handle after ADE0"
                );
                owner
            }
            other => panic!(
                "spawn {spawn} expected in-range ADE0/Aim, got {other:?}; player cap={player_capability:#x} ant_pos={ant_position:?}"
            ),
        };
        // Wander/Chase may have walked the ant. Aim's forward half-space is
        // heading 0 / +X; park the player one cell ahead of the live pose.
        manager.entity_mut(ant_id).unwrap().heading = 0.0;
        let ant_position = manager
            .entity_mut(ant_id)
            .expect("published Type-47")
            .position_raw();
        manager.player_mut().unwrap().set_motion_raw(
            [
                ant_position[0].wrapping_add(0x100),
                ant_position[1],
                ant_position[2],
            ],
            [0; 3],
        );
        // Construction already consumed process RNG, so the same-pass 19.5 ms
        // Aim visit usually rejects. A later 800 ms pursuing visit is the
        // same FUN_00402300 catch-up the unit oracle uses from cadence 0.
        let tick = tick_ordinary_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            session.cache.terrain(),
            type47_model_records(&session),
            800_000,
            &mut |_: &mut WorldFx| 0,
        );
        match tick.outcome {
            OrdinaryType47SchedulerProductionOutcome::Pursuing {
                chase,
                aim: Some(Ok(aim)),
                ..
            } => {
                assert!(
                    matches!(chase.result, Type47ChaseVisitResult::Continue),
                    "spawn {spawn} Chase must walk after V200003 first query, got {:?}",
                    chase.result
                );
                // Walking can leave Aim's forward half-space. Cadence still
                // runs; a zero append here is not a locomotion failure.
                if aim.queued_shots_added > 0 {
                    fired += 1;
                }
            }
            other => panic!("spawn {spawn} expected pursuing Aim, got {other:?}"),
        }
        assert!(tick.retained_owner.is_some());
    }

    let drained = drain_live_ordinary_type47_shot_queues(
        &mut manager,
        &mut world_fx,
        ParticleEnvironment::Dry,
        0,
    );
    assert_eq!(drained.len(), fired, "each queued shot drains once");
    assert!(drained.iter().all(|(_, outcome)| {
        outcome.as_ref().is_ok_and(|drain| {
            drain.consumed_requests >= 1
                && drain.materialized_particle_slots.len() == drain.consumed_requests
        })
    }));
}

#[v2k_test_support::retail_test]
fn spawn_11_guard_wander_translates_without_the_player() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();

    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let publication = manager
        .fresh_level1_type47_initial_productions()
        .iter()
        .find(|publication| publication.authored_spawn_index == 11)
        .copied()
        .expect("spawn 11");
    let ant_id = publication.entity_id;
    let start = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("spawn 11 entity")
        .position_raw();
    let mut owner =
        OrdinaryType47SchedulerOwner::adopt(&manager, publication).expect("spawn 11 Guard owner");
    for frame in 0..90 {
        let tick = tick_ordinary_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            session.cache.terrain(),
            type47_model_records(&session),
            19_500,
            &mut |_: &mut WorldFx| u32::from(frame == 0),
        );
        match tick.outcome {
            OrdinaryType47SchedulerProductionOutcome::Acquisition { wander, .. } => {
                assert!(
                    !matches!(
                        wander.result,
                        Type47WanderVisitResult::CommonMoverBlocked(_)
                    ),
                    "spawn 11 Guard wander blocked on frame {frame}: {:?}",
                    wander.result
                );
            }
            other => panic!("spawn 11 expected Guard acquisition, got {other:?}"),
        }
        owner = tick.retained_owner.expect("Guard owner stays live");
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("spawn 11 entity");
    let end = entity.position_raw();
    let vel = entity.velocity_raw();
    assert_ne!(
        [start[0], start[2]],
        [end[0], end[2]],
        "spawn 11 must translate on Guard wander, start={start:?} end={end:?} vel={vel:?} flags={:?}",
        entity.collision.state_flags_at_0x08
    );
}

#[v2k_test_support::retail_test]
fn spawn_11_walk_keeps_insect_scale_sub_h_spans() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();

    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let publication = manager
        .fresh_level1_type47_initial_productions()
        .iter()
        .find(|publication| publication.authored_spawn_index == 11)
        .copied()
        .expect("spawn 11");
    let ant_id = publication.entity_id;
    let start_world = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("spawn 11 entity")
        .position;
    let model = session
        .cache
        .global_model(v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
        .expect("newant model 302");
    {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == ant_id)
            .expect("spawn 11 entity");
        let birth = newant_leg_gate_stats(&session, entity, model, 0);
        assert_eq!(
            birth.submitted,
            20,
            "birth at the signed-8.8 Z seam must keep 20 0x22 ribbons, origin_raw={:?} {birth:?}",
            entity.position_raw()
        );
        assert_eq!(
            birth.lines_submitted,
            14,
            "birth at the signed-8.8 Z seam must keep 14 0x02 hairlines, origin_raw={:?} {birth:?}",
            entity.position_raw()
        );
    }
    let mut owner =
        OrdinaryType47SchedulerOwner::adopt(&manager, publication).expect("spawn 11 Guard owner");
    let mut saw_stride = false;
    for frame in 0..2_000 {
        let tick = tick_ordinary_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            session.cache.terrain(),
            type47_model_records(&session),
            19_500,
            &mut |_: &mut WorldFx| u32::from(frame == 0),
        );
        match tick.outcome {
            OrdinaryType47SchedulerProductionOutcome::Acquisition { wander, .. } => {
                assert!(
                    !matches!(
                        wander.result,
                        Type47WanderVisitResult::CommonMoverBlocked(_)
                    ),
                    "spawn 11 Guard wander blocked on frame {frame}: {:?}",
                    wander.result
                );
            }
            other => panic!("spawn 11 expected Guard acquisition, got {other:?}"),
        }
        owner = tick.retained_owner.expect("Guard owner stays live");
        if manager
            .iter_all()
            .find(|entity| entity.id == ant_id)
            .and_then(|entity| match &entity.sub_h_external_frame_runtime {
                RetailRuntimeValue::Known(Some(sub_h)) => Some(sub_h),
                _ => None,
            })
            .is_some_and(|sub_h| sub_h.records().iter().any(|record| record.phase_raw != 0))
        {
            saw_stride = true;
        }
    }

    let entity = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("spawn 11 entity");
    let model = session
        .cache
        .global_model(v2k_game::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
        .expect("newant model 302");
    assert!(
        !v2k_game::model_tree::model_is_camera_facing_actor(model),
        "newant body is a 3D mesh with Sub-H ribbons, not a cylindrical person"
    );
    let descriptor = session
        .cache
        .global_entity_type(47)
        .and_then(|record| record.sub_h_external_frame_descriptor())
        .expect("type 47 Sub-H descriptor");
    let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
        panic!("spawn 11 must own live Sub-H");
    };
    let axes = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
        RetailRuntimeValue::Unresolved => None,
    };
    let origin_raw = entity.position_raw();
    let origin_world = entity.position;
    let terrain = session.cache.terrain();
    let points = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        sub_h,
        &descriptor,
        &model.records,
        origin_raw,
        origin_world,
        axes,
        |x, z| terrain.map_or(0, |grid| grid.bilinear_height_raw(x, z)),
    )
    .expect("type-0 Sub-H geometry after walk");
    let mut max_span = 0.0_f32;
    for point in points.iter().flatten() {
        let dx = point[0] - origin_world[0];
        let dy = point[1] - origin_world[1];
        let dz = point[2] - origin_world[2];
        max_span = max_span.max((dx * dx + dy * dy + dz * dz).sqrt());
    }
    assert!(
        max_span < 4.0,
        "Sub-H feet must stay insect-scale after FUN_00412DA0 walk, span={max_span} origin={origin_world:?} raw={origin_raw:?} end={:?}",
        entity.position_raw()
    );

    let camera_at_player = [75.0_f32, 20.0, 60.0];
    let draw_pos = [
        camera_at_player[0] + v2k_core::world::delta(origin_world[0], camera_at_player[0]),
        origin_world[1],
        camera_at_player[2] + v2k_core::world::delta(origin_world[2], camera_at_player[2]),
    ];
    let drawn = v2k_game::sub_h_external_frame::resolve_selector_world_points(
        sub_h,
        &descriptor,
        &model.records,
        origin_raw,
        draw_pos,
        axes,
        |x, z| terrain.map_or(0, |grid| grid.bilinear_height_raw(x, z)),
    )
    .expect("camera-relative Sub-H geometry");
    let mut draw_span = 0.0_f32;
    for point in drawn.iter().flatten() {
        let dx = point[0] - draw_pos[0];
        let dy = point[1] - draw_pos[1];
        let dz = point[2] - draw_pos[2];
        draw_span = draw_span.max((dx * dx + dy * dy + dz * dz).sqrt());
    }
    assert!(
        draw_span < 4.0,
        "camera-relative Sub-H feet must not span a world period, span={draw_span} draw={draw_pos:?} world={origin_world:?}"
    );

    let walked = {
        let dx = v2k_core::world::delta(origin_world[0], start_world[0]);
        let dz = v2k_core::world::delta(origin_world[2], start_world[2]);
        (dx * dx + dz * dz).sqrt()
    };
    assert!(
        walked < 8.0,
        "Guard wander must stay on the hive post, walked={walked} start={start_world:?} end={origin_world:?}"
    );
    assert!(
        sub_h.records().iter().any(|record| record.flags_raw != 0),
        "D360 writeback must leave constructor-zero Sub-H flags"
    );
    assert!(
        saw_stride,
        "FUN_0041D0A0 must start a stride once D360 publishes COPY_TARGET/WAIT_FOR_DEPS"
    );
    if let Some(grid) = terrain {
        let terrain_y = grid.bilinear_height_raw(origin_raw[0], origin_raw[2]);
        let height_above_terrain = i32::from(origin_raw[1]) - i32::from(terrain_y);
        assert!(
            height_above_terrain.abs() < 1_024,
            "E100 must keep spawn 11 on terrain after walk, dY={height_above_terrain} origin_raw={origin_raw:?} terrain_y={terrain_y}"
        );
    }

    let extra_scale = 100.0 / 256.0;
    let scale = extra_scale / 100.0;
    let orientation = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => {
            panic!("spawn 11 constructor FUN_00413F70 must have published a body basis")
        }
    };
    let as_raw_local = |world: [f32; 3]| -> [f32; 3] {
        let delta = [
            world[0] - origin_world[0],
            world[1] - origin_world[1],
            world[2] - origin_world[2],
        ];
        [
            delta[0] * orientation[0][0]
                + delta[1] * orientation[1][0]
                + delta[2] * orientation[2][0],
            delta[0] * orientation[0][1]
                + delta[1] * orientation[1][1]
                + delta[2] * orientation[2][1],
            delta[0] * orientation[0][2]
                + delta[1] * orientation[1][2]
                + delta[2] * orientation[2][2],
        ]
        .map(|component| component / scale)
    };
    let to_world = |raw: [f32; 3]| -> [f32; 3] {
        [
            origin_world[0]
                + (orientation[0][0] * raw[0]
                    + orientation[0][1] * raw[1]
                    + orientation[0][2] * raw[2])
                    * scale,
            origin_world[1]
                + (orientation[1][0] * raw[0]
                    + orientation[1][1] * raw[1]
                    + orientation[1][2] * raw[2])
                    * scale,
            origin_world[2]
                + (orientation[2][0] * raw[0]
                    + orientation[2][1] * raw[1]
                    + orientation[2][2] * raw[2])
                    * scale,
        ]
    };
    let resolved: Vec<Option<[f32; 3]>> = model
        .vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            let mut raw = [vertex[0] as f32, vertex[1] as f32, vertex[2] as f32];
            if model.vertex_type_flags.get(index) == Some(&14) {
                let selector = raw[0] as i32;
                let Some(world) = (0..16)
                    .contains(&selector)
                    .then(|| points[selector as usize])
                    .flatten()
                else {
                    return Some(to_world(raw));
                };
                raw = as_raw_local(world);
            }
            Some(to_world(raw))
        })
        .collect();
    let widths: Vec<(u16, u16)> = model
        .edges
        .iter()
        .map(|edge| match edge.style {
            v2k_formats::models::ModelEdgeStyle::Sprite { sprite_id, .. } => session
                .cache
                .global_sprite(sprite_id)
                .map(|(_, entry)| (entry.flags as u16, (entry.flags >> 16) as u16))
                .unwrap_or((0, 0)),
            v2k_formats::models::ModelEdgeStyle::Palette { .. } => (0, 0),
        })
        .collect();
    let materials = vec![
        v2k_render::FaceMaterial {
            palette_rgb555: None,
            color: [1.0; 3],
            emissive: [0.0; 3],
            texture: None,
            blend: v2k_render::WorldSpriteBlend::Masked,
            flat_shade_row: 31,
        };
        model.edges.len()
    ];
    let mut camera = v2k_render::Camera::new(4.0 / 3.0);
    camera.position = [
        origin_world[0],
        origin_world[1] + 8.0,
        origin_world[2] - 15.0,
    ];
    camera.yaw = std::f32::consts::PI;
    camera.pitch = 0.0;
    camera.left_handed = true;
    camera.fov = 60.0_f32.to_radians();
    let view = camera.view_matrix();
    let proj = camera.projection_matrix();
    let basis = [
        [view[0], view[4], view[8]],
        [view[1], view[5], view[9]],
        [view[2], view[6], view[10]],
    ];
    let intr = v2k_render::edge_quads::ScreenIntrinsics {
        width_px: 1024,
        height_px: 768,
        scale_x: proj[0] * 512.0,
        scale_y: proj[5] * 384.0,
        center_x: 512.0,
        center_y: 384.0,
        clip_near: camera.near,
        clip_far: camera.far,
        projection_effect: v2k_render::projection::ProjectionEffect::None,
    };
    let (selection, stats) = v2k_render::edge_quads::select_edge_quads_with_stats(
        &model.edges,
        &widths,
        &materials,
        &resolved,
        &intr,
        camera.position,
        basis,
    );
    assert!(
        stats.submitted > 0,
        "newant walk must still submit insect-scale 0x22 ribbons, {stats:?}"
    );
    let mut max_screen = 0.0_f32;
    for quad in &selection.quads {
        for a in &quad.screen_corners {
            for b in &quad.screen_corners {
                let dx = a[0] - b[0];
                let dy = a[1] - b[1];
                max_screen = max_screen.max((dx * dx + dy * dy).sqrt());
            }
        }
    }
    assert!(
        max_screen < 400.0,
        "0x22 fill must not smear across the viewport after walk, max_screen={max_screen} walked={walked} origin_raw={origin_raw:?} {stats:?}"
    );
    assert!(
        stats.lines_submitted >= 6,
        "0x02 hip-foot hairlines must still submit after walk, origin_raw={origin_raw:?} {stats:?}"
    );
    let mut max_line = 0.0_f32;
    for line in &selection.lines {
        let dx = line.end[0] - line.start[0];
        let dy = line.end[1] - line.start[1];
        max_line = max_line.max((dx * dx + dy * dy).sqrt());
    }
    assert!(
        max_line < 400.0,
        "0x02 fill must not smear across the viewport after walk, max_line={max_line} origin_raw={origin_raw:?} {stats:?}"
    );

    // Live `--level 13` dumps are 1920x1080 with the craft ~26 world from
    // the hive. Headless 1024x768 at 15 world can hide a gate that live hits.
    let mut live_cam = v2k_render::Camera::new(1920.0 / 1080.0);
    live_cam.position = [
        origin_world[0] + 11.5,
        origin_world[1] + 8.0,
        origin_world[2] - 24.0,
    ];
    live_cam.yaw = std::f32::consts::PI;
    live_cam.pitch = 0.0;
    live_cam.left_handed = true;
    live_cam.fov = 60.0_f32.to_radians();
    let live_view = live_cam.view_matrix();
    let live_proj = live_cam.projection_matrix();
    let live_basis = [
        [live_view[0], live_view[4], live_view[8]],
        [live_view[1], live_view[5], live_view[9]],
        [live_view[2], live_view[6], live_view[10]],
    ];
    let live_intr = v2k_render::edge_quads::ScreenIntrinsics {
        width_px: 1920,
        height_px: 1080,
        scale_x: live_proj[0] * 960.0,
        scale_y: live_proj[5] * 540.0,
        center_x: 960.0,
        center_y: 540.0,
        clip_near: live_cam.near,
        clip_far: live_cam.far,
        projection_effect: v2k_render::projection::ProjectionEffect::None,
    };
    let (live_selection, live_stats) = v2k_render::edge_quads::select_edge_quads_with_stats(
        &model.edges,
        &widths,
        &materials,
        &resolved,
        &live_intr,
        live_cam.position,
        live_basis,
    );
    assert_eq!(
        live_stats.submitted, 20,
        "live 1920x1080 camera must keep the parked 20 0x22 ribbons, origin_raw={origin_raw:?} {live_stats:?}"
    );
    assert_eq!(
        live_stats.lines_submitted, 14,
        "live 1920x1080 camera must keep the parked 14 0x02 hairlines, origin_raw={origin_raw:?} {live_stats:?}"
    );
    assert_eq!(live_selection.quads.len(), 20);
    assert_eq!(live_selection.lines.len(), 14);
}

fn cell_height_raw(terrain: &v2k_formats::terrain::TerrainGrid, x: usize, z: usize) -> i16 {
    i16::from(terrain.cell(x, z).expect("cell").height as i8) << 5
}

fn sub_d_water_class(terrain: &v2k_formats::terrain::TerrainGrid, x_raw: i16, z_raw: i16) -> bool {
    let sea = terrain.sea_level_raw();
    let x = x_raw as u16;
    let z = z_raw as u16;
    let corners = [
        (x.wrapping_sub(0x80), z.wrapping_sub(0x80)),
        (x.wrapping_add(0x80), z.wrapping_sub(0x80)),
        (x.wrapping_sub(0x80), z.wrapping_add(0x80)),
        (x.wrapping_add(0x80), z.wrapping_add(0x80)),
    ];
    corners.into_iter().any(|(cx, cz)| {
        cell_height_raw(
            terrain,
            usize::from((cx >> 8) as u8),
            usize::from((cz >> 8) as u8),
        ) < sea
    })
}

#[v2k_test_support::retail_test]
fn spawn_11_real_rng_wander_stays_on_land_without_the_player() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();

    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let publication = manager
        .fresh_level1_type47_initial_productions()
        .iter()
        .find(|publication| publication.authored_spawn_index == 11)
        .copied()
        .expect("spawn 11");
    let ant_id = publication.entity_id;
    let player_id = manager.player().expect("persistent type-46 player").id;
    let ant = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("spawn 11 entity");
    let start = ant.position;
    let start_raw = ant.position_raw();
    let player = manager.player().unwrap();
    let cube = WrappedAxisRange::from_raw(
        TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR.strict_axis_limit_raw,
    );
    assert!(
        !within_wrapped_axis_range(cube, start_raw, player.position_raw()),
        "authored first-world player {:?} must stay outside the 1792 Guard cube around spawn 11 {:?}",
        player.position_raw(),
        start_raw
    );

    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let sea = terrain.sea_level_raw();
    let cx = ((start_raw[0] as u16) >> 8) as usize;
    let cz = ((start_raw[2] as u16) >> 8) as usize;
    let mut map = String::new();
    for dz in -6i32..=6 {
        for dx in -6i32..=6 {
            let x = (cx as i32 + dx).rem_euclid(256) as usize;
            let z = (cz as i32 + dz).rem_euclid(256) as usize;
            let water = cell_height_raw(terrain, x, z) < sea;
            map.push(if water { '~' } else { '#' });
        }
        map.push('\n');
    }

    let nearby: Vec<_> = manager
        .iter_all()
        .filter(|entity| {
            entity.id != ant_id
                && within_wrapped_axis_range(cube, start_raw, entity.position_raw())
                && entity.capability_flags & 5 != 0
        })
        .map(|entity| {
            (
                entity.id,
                entity.entity_type,
                entity.authored_spawn_index,
                entity.position_raw(),
                entity.capability_flags,
            )
        })
        .collect();
    assert!(
        nearby.iter().all(|row| row.0 != player_id),
        "player must not be a birth-time Guard candidate: {nearby:?}"
    );

    let mut owner =
        OrdinaryType47SchedulerOwner::adopt(&manager, publication).expect("spawn 11 Guard owner");
    let mut pursuing_frames = 0usize;
    for _ in 0..2_000 {
        let tick = tick_ordinary_type47_scheduler_owner(
            &mut manager,
            owner,
            &mut world_fx,
            session.cache.terrain(),
            type47_model_records(&session),
            19_500,
            &mut |world_fx: &mut WorldFx| u32::from(world_fx.next_shared_retail_random_u16()),
        );
        match tick.outcome {
            OrdinaryType47SchedulerProductionOutcome::Acquisition { wander, result, .. } => {
                assert!(
                    !matches!(
                        wander.result,
                        Type47WanderVisitResult::CommonMoverBlocked(_)
                    ),
                    "spawn 11 Guard wander blocked: {:?}",
                    wander.result
                );
                assert!(
                    !matches!(
                        result,
                        GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. }
                    ),
                    "spawn 11 must not ADE0-handoff without the player in the cube: {result:?}"
                );
            }
            OrdinaryType47SchedulerProductionOutcome::Pursuing { .. } => {
                pursuing_frames += 1;
            }
            other => panic!("spawn 11 unexpected scheduler outcome {other:?}"),
        }
        owner = tick.retained_owner.expect("Guard owner stays live");
    }

    let entity = manager
        .iter_all()
        .find(|entity| entity.id == ant_id)
        .expect("spawn 11 entity");
    let end = entity.position;
    let end_raw = entity.position_raw();
    let walked = {
        let dx = v2k_core::world::delta(end[0], start[0]);
        let dz = v2k_core::world::delta(end[2], start[2]);
        (dx * dx + dz * dz).sqrt()
    };
    let end_water = sub_d_water_class(terrain, end_raw[0], end_raw[2]);
    let terrain_y = terrain.bilinear_height_raw(end_raw[0], end_raw[2]);
    let height_above_terrain = i32::from(end_raw[1]) - i32::from(terrain_y);
    assert_eq!(
        pursuing_frames, 0,
        "authored-spawn player is outside the cube; Guard must not become Chase"
    );
    assert!(
        walked < 8.0,
        "Guard wander must stay on the hive post, walked={walked} start={start:?} end={end:?}"
    );
    assert!(
        !end_water,
        "Guard wander must not walk onto a Sub-D water cell, end_raw={end_raw:?} sea={sea} map=\n{map}"
    );
    assert!(
        height_above_terrain.abs() < 1_024,
        "E100 gravity/drag must keep Type-47 Y on the spit, not integrate Sub-C lift into the sky (type-13 then plants the moving stain on water). dY={height_above_terrain} end_raw={end_raw:?} terrain_y={terrain_y}"
    );
}

fn wrapping_xz_world(origin_raw: [i16; 3], point_raw: [i16; 3]) -> (f32, f32) {
    (
        f32::from(point_raw[0].wrapping_sub(origin_raw[0])) / 256.0,
        f32::from(point_raw[2].wrapping_sub(origin_raw[2])) / 256.0,
    )
}

fn foot_behind_world(origin_raw: [i16; 3], foot_raw: [i16; 3], forward_q31: [i32; 3]) -> f32 {
    let (dx, dz) = wrapping_xz_world(origin_raw, foot_raw);
    let fx = forward_q31[0] as f32 / 2_147_483_648.0;
    let fz = forward_q31[2] as f32 / 2_147_483_648.0;
    let length = (fx * fx + fz * fz).sqrt().max(1.0e-6);
    -(dx * fx + dz * fz) / length
}

#[v2k_test_support::retail_test]
fn all_three_level1_newants_keep_feet_under_the_thorax() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();

    let mut world_fx = WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let model = session
        .cache
        .global_model(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
        .expect("newant model 302");
    let publications = manager.fresh_level1_type47_initial_productions().to_vec();
    assert_eq!(
        publications
            .iter()
            .map(|publication| publication.authored_spawn_index)
            .collect::<Vec<_>>(),
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
    );

    struct LiveAnt {
        spawn: usize,
        id: u32,
        start_raw: [i16; 3],
        owner: OrdinaryType47SchedulerOwner,
        saw_stride: bool,
        max_foot_world: f32,
        max_behind_world: f32,
    }

    let mut ants: Vec<LiveAnt> = publications
        .into_iter()
        .map(|publication| {
            let spawn = publication.authored_spawn_index;
            let id = publication.entity_id;
            let start_raw = manager
                .iter_all()
                .find(|entity| entity.id == id)
                .expect("spawn entity")
                .position_raw();
            let owner = OrdinaryType47SchedulerOwner::adopt(&manager, publication)
                .unwrap_or_else(|error| panic!("spawn {spawn} adopt: {error:?}"));
            LiveAnt {
                spawn,
                id,
                start_raw,
                owner,
                saw_stride: false,
                max_foot_world: 0.0,
                max_behind_world: 0.0,
            }
        })
        .collect();

    for frame in 0..400 {
        for ant in &mut ants {
            let spawn = ant.spawn;
            let tick = tick_ordinary_type47_scheduler_owner(
                &mut manager,
                ant.owner,
                &mut world_fx,
                session.cache.terrain(),
                Some(model.records.as_slice()),
                19_500,
                &mut |world_fx: &mut WorldFx| u32::from(world_fx.next_shared_retail_random_u16()),
            );
            match tick.outcome {
                OrdinaryType47SchedulerProductionOutcome::Acquisition { wander, .. } => {
                    assert!(
                        !matches!(
                            wander.result,
                            Type47WanderVisitResult::CommonMoverBlocked(_)
                        ),
                        "spawn {spawn} Guard wander blocked on frame {frame}: {:?}",
                        wander.result
                    );
                }
                OrdinaryType47SchedulerProductionOutcome::Pursuing { .. } => {}
                other => panic!("spawn {spawn} unexpected {other:?}"),
            }
            ant.owner = tick.retained_owner.expect("owner stays live");
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == ant.id)
                .expect("spawn entity");
            let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime
            else {
                panic!("spawn {spawn} missing Sub-H");
            };
            if sub_h.records().iter().any(|record| record.phase_raw != 0) {
                ant.saw_stride = true;
            }
            let origin = entity.position_raw();
            let forward_q31 = match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => basis.forward,
                RetailRuntimeValue::Unresolved => [0, 0, i32::MAX],
            };
            for record in sub_h.records() {
                let (dx, dz) = wrapping_xz_world(origin, record.primary_raw);
                ant.max_foot_world = ant.max_foot_world.max((dx * dx + dz * dz).sqrt());
                ant.max_behind_world = ant.max_behind_world.max(foot_behind_world(
                    origin,
                    record.primary_raw,
                    forward_q31,
                ));
            }
        }
    }

    let rest_c: Vec<[i16; 3]> = TYPE47_COMMON_DYING_SUB_H_RECORDS
        .iter()
        .map(|authored| {
            v2k_game::sub_h_external_frame::type0_slot_model_raw(
                &model.records,
                authored.vertex_refs[2],
            )
            .expect("type-0 endpoint C")
        })
        .collect();

    struct AntWalk {
        spawn: usize,
        origin_raw: [i16; 3],
        walked: f32,
        heading: f32,
        saw_stride: bool,
        max_foot_world: f32,
        max_behind_world: f32,
        excess: Vec<i16>,
        phases: Vec<u16>,
        flags: Vec<u32>,
        primaries: Vec<[i16; 3]>,
    }

    let walks: Vec<AntWalk> = ants
        .iter()
        .map(|ant| {
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == ant.id)
                .expect("spawn entity");
            let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime
            else {
                panic!("spawn {} missing Sub-H", ant.spawn);
            };
            let origin = entity.position_raw();
            let walked = {
                let (dx, dz) = wrapping_xz_world(ant.start_raw, origin);
                (dx * dx + dz * dz).sqrt()
            };
            AntWalk {
                spawn: ant.spawn,
                origin_raw: origin,
                walked,
                heading: entity.heading,
                saw_stride: ant.saw_stride,
                max_foot_world: ant.max_foot_world,
                max_behind_world: ant.max_behind_world,
                excess: sub_h
                    .records()
                    .iter()
                    .map(|record| record.excess_length_raw)
                    .collect(),
                phases: sub_h
                    .records()
                    .iter()
                    .map(|record| record.phase_raw)
                    .collect(),
                flags: sub_h
                    .records()
                    .iter()
                    .map(|record| record.flags_raw)
                    .collect(),
                primaries: sub_h
                    .records()
                    .iter()
                    .map(|record| record.primary_raw)
                    .collect(),
            }
        })
        .collect();

    let summary = format!(
        "rest C locals={rest_c:?}\n{}",
        walks
            .iter()
            .map(|walk| {
                format!(
                    "spawn {} origin={:?} walked={:.3} heading={:.3} stride={} max_foot={:.3} max_behind={:.3} excess={:?} phases={:?} flags={:08x?} primaries={:?}",
                    walk.spawn,
                    walk.origin_raw,
                    walk.walked,
                    walk.heading,
                    walk.saw_stride,
                    walk.max_foot_world,
                    walk.max_behind_world,
                    walk.excess,
                    walk.phases,
                    walk.flags,
                    walk.primaries
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
    for walk in &walks {
        assert!(
            walk.saw_stride,
            "spawn {} never started a Sub-H stride\n{summary}",
            walk.spawn
        );
        assert!(
            walk.max_foot_world < 1.25,
            "spawn {} planted a foot {:.3} world from the thorax (trailing-leg shot)\n{summary}",
            walk.spawn,
            walk.max_foot_world
        );
        assert!(
            walk.max_behind_world < 1.0,
            "spawn {} dragged a foot {:.3} world behind heading\n{summary}",
            walk.spawn,
            walk.max_behind_world
        );
    }
}
