use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    chase_target::ChaseTargetCommonMoverReturn,
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot, type9_attitude::Type9BodyBasis,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::{CommonMoverGklPayloads, EntityTypeRuntimeMetadata, RetailStateWord},
    entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    gkl_common_mover::{
        evaluate_gkl_common_mover, GklCommonMoverProfile, GklCommonMoverRequest,
        GklCommonMoverRuntime,
    },
    intro2_type47_live::world::native_intro2_fixture,
    sub_g_runtime::SubG06070RuntimeState,
    wander_near_location::WanderNearPrivateState,
    world_fx::WorldFx,
};
use v2k_formats::terrain::{TerrainCell, TerrainGrid};

fn profile() -> GklCommonMoverProfile {
    GklCommonMoverProfile {
        topology: TOPOLOGY,
        sub_d: SUB_D,
        gkl: CommonMoverGklPayloads {
            sub_g: Some(SUB_G),
            sub_k: Some(SUB_K),
            sub_l: Some(SUB_L),
        },
    }
}

fn detached_runtime() -> GklCommonMoverRuntime {
    GklCommonMoverRuntime {
        sub_d_runtime: Type9SubDRuntime::from_constructor(),
        // This test deliberately retains an unknown origin. Type10 flags0
        // must not query it, nor borrow Type13's accepted constructor image.
        sub_d_frame_owner: Type9SubDFrameOwner::pending_constructor_origin(0x22),
        sub_k_smoothed_raw: 0,
        sub_k_output_raw: [30_000, -30_000],
        sub_l_target_raw: [11, 12, 13],
        sub_l_exact_raw: 0,
        sub_l_output_raw: [30_000, -30_000],
    }
}

fn detached_request<'a>(
    metadata: &'a EntityTypeRuntimeMetadata,
    terrain: &'a TerrainGrid,
    dispatch_mode: CommonMoverDispatchMode,
) -> GklCommonMoverRequest<'a> {
    let mut sub_g = SubG06070RuntimeState::from_1b8c0_constructor(&SUB_G, 0);
    sub_g.apply_shared_06070_sub_g_branch(700, 0);
    GklCommonMoverRequest {
        entity_id: 56,
        entity_type: 10,
        metadata: Some(metadata),
        dispatch_mode,
        position_raw: [0, 500, 0],
        velocity_raw: [10, 20, 30],
        heading_raw: 0,
        pitch_raw: 0,
        roll_raw: 0,
        body_basis: RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0)),
        runtime: detached_runtime(),
        sub_g_runtime: sub_g,
        target_private: WanderNearPrivateState::tracked_entity([0; 3], 1),
        tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: RetailStateWord::exact(4),
            position_raw: [1000, 600, 1500],
            velocity_raw: [0; 3],
        })),
        terrain,
        active_model_extent_raw: 660,
        self_mass_raw: 100,
        attached_cargo_mass: 0,
        capability_flags: 8,
        retail_tick: 0,
        elapsed_micros: 19_500,
        global_elapsed_micros: 19_500,
    }
}

fn detached_metadata() -> EntityTypeRuntimeMetadata {
    EntityTypeRuntimeMetadata {
        common_mover_topology: RetailRuntimeValue::Known(TOPOLOGY),
        common_mover_gkl_payloads: RetailRuntimeValue::Known(profile().gkl),
        sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(SUB_D)),
        ..Default::default()
    }
}

fn flat_terrain() -> TerrainGrid {
    TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            256 * 256
        ],
    }
}

#[test]
fn native_type10_gkl_target_zero_precedes_unknown_basis_and_lazy_component_payloads() {
    let terrain = flat_terrain();
    let mut metadata = detached_metadata();
    metadata.common_mover_gkl_payloads = RetailRuntimeValue::Unresolved;
    for mode in [
        CommonMoverDispatchMode::Normal,
        CommonMoverDispatchMode::Restricted,
    ] {
        for target_state in [0, 0x4000] {
            let mut request = detached_request(&metadata, &terrain, mode);
            request.body_basis = RetailRuntimeValue::Unresolved;
            request.sub_g_runtime = SubG06070RuntimeState::pending();
            request.tracked_target =
                RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: RetailStateWord::exact(target_state),
                    position_raw: [1000, 600, 1500],
                    velocity_raw: [0; 3],
                }));
            let before_runtime = request.runtime;
            let before_target = request.target_private;
            let result = evaluate_gkl_common_mover(request, profile(), || {
                panic!("dead/inactive target returns before any random or component work")
            })
            .unwrap();
            assert_eq!(result.result, ChaseTargetCommonMoverReturn::Zero);
            assert_eq!(result.runtime, before_runtime);
            assert_eq!(result.target_private, before_target);
            assert_eq!(result.sub_g_runtime, SubG06070RuntimeState::pending());
            assert_eq!(result.position_raw, [0, 500, 0]);
            assert_eq!(result.velocity_raw, [10, 20, 30]);
        }
    }
}

#[test]
fn native_type10_gkl_modes_share_physics_but_keep_own_l_limits_and_output_dispatch() {
    let metadata = detached_metadata();
    let terrain = flat_terrain();
    let evaluate = |mode| {
        evaluate_gkl_common_mover(
            detached_request(&metadata, &terrain, mode),
            profile(),
            || panic!("Type10 tracked flags0 steering consumes no classifier RNG"),
        )
        .unwrap()
    };
    let restricted = evaluate(CommonMoverDispatchMode::Restricted);
    let detailed = evaluate(CommonMoverDispatchMode::Normal);
    assert_eq!(detailed.result, ChaseTargetCommonMoverReturn::NonZero);
    assert_eq!(restricted.result, detailed.result);
    assert_eq!(restricted.position_raw, detailed.position_raw);
    assert_eq!(restricted.velocity_raw, detailed.velocity_raw);
    assert_eq!(restricted.heading_raw, detailed.heading_raw);
    assert_eq!(restricted.pitch_raw, detailed.pitch_raw);
    assert_eq!(restricted.roll_raw, detailed.roll_raw);
    assert_eq!(restricted.sub_g_runtime, detailed.sub_g_runtime);
    assert_eq!(
        restricted.runtime.sub_d_runtime,
        detailed.runtime.sub_d_runtime
    );
    assert_eq!(restricted.runtime.sub_l_target_raw, [1000, 600, 1500]);
    assert_eq!(
        restricted.runtime.sub_l_exact_raw,
        detailed.runtime.sub_l_exact_raw
    );
    assert_eq!(restricted.runtime.sub_k_output_raw, [30_000, -30_000]);
    assert_eq!(restricted.runtime.sub_l_output_raw, [30_000, -30_000]);
    assert_eq!(detailed.runtime.sub_k_output_raw, [0x1800, -0x1200]);
    assert_eq!(detailed.runtime.sub_l_output_raw, [8000, -8000]);
    assert_eq!(
        detailed.runtime.sub_d_frame_owner.classifier_cache().rows(),
        [0; 8]
    );
    assert_eq!(
        detailed
            .runtime
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        0x23
    );
    assert_eq!(detailed.sound, None);
}

fn admit_callbacks(entity: &mut Entity, detailed: bool) {
    entity.collision.state_flags_at_0x08.overwrite(
        0x0002_0000 | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        0x0002_0000
            | if detailed {
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            } else {
                0
            },
    );
    entity.collision.callback_scheduler_accumulator_us_at_0x6c =
        RetailRuntimeValue::Known(if detailed { 0 } else { 125_000 });
}

fn isolate_targets(manager: &mut EntityManager, owner_id: u32) {
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        if id == owner_id {
            continue;
        }
        let candidate = manager.entity_mut(id).unwrap();
        // Type10's authored search mask includes 0x80 and 0x04 as well as
        // 0x0c01. Clearing the older peasant fixture mask left real targets.
        candidate.capability_flags &= !0x0c85;
        candidate.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    }
}

#[v2k_test_support::retail_test]
fn both_native_type10_births_move_with_varied_frames_in_distinct_scheduler_modes() {
    for spawn in INTRO2_TYPE10_SPAWN_INDICES {
        for detailed in [false, true] {
            let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
                return;
            };
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn))
                .unwrap()
                .id;
            isolate_targets(&mut manager, id);
            let entity = manager.entity_mut(id).unwrap();
            assert!(intro2_type10_allocation_authenticates(entity));
            admit_callbacks(entity, detailed);
            let original = entity.position_raw();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(19);
            let emitter = entity.intro2_type10_runtime.unwrap().sub_e_runtime;
            let mut owner = Intro2Type10Owner::adopt(&manager, id).unwrap();
            let mut fx = WorldFx::new();
            let mut elapsed = 0;
            let mut callbacks = 0;
            for index in 0..128 {
                let dt = [1_000, 16_667, 40_000, 125_000][index % 4];
                elapsed += dt;
                let tick = tick_intro2_type10(
                    &mut manager,
                    owner,
                    Intro2Type10Frame {
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: dt,
                        retail_tick: elapsed / 20_000,
                    },
                );
                match tick.outcome {
                    Intro2Type10Outcome::Waiting { .. } => {}
                    Intro2Type10Outcome::Advanced {
                        callback_enabled: true,
                        ..
                    } => {
                        let entity = manager.entity_mut(id).unwrap();
                        assert_eq!(entity.mass_raw, if callbacks == 0 { 119 } else { 100 });
                        assert_eq!(
                            entity.collision.animation_offset_at_0xb2,
                            RetailRuntimeValue::Known(0)
                        );
                        callbacks += 1;
                    }
                    other => panic!("spawn={spawn} detailed={detailed} visit={index}: {other:?}"),
                }
                owner = tick
                    .retained_owner
                    .expect("live allocation retains its exact owner");
                assert!(!owner.has_pending_prefix());
            }
            assert!(callbacks > 10);
            let entity = manager.entity_mut(id).unwrap();
            assert_ne!(entity.position_raw(), original);
            assert_eq!(entity.model_slots, [Some(MODEL); 4]);
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(32_000)
            );
            let runtime = entity.intro2_type10_runtime.unwrap();
            assert_eq!(
                runtime.sub_e_runtime, emitter,
                "no target means no Aim cadence writes"
            );
            assert_eq!(runtime.sub_d_frame_owner.classifier_cache().rows(), [0; 8]);
            if detailed {
                assert_ne!(runtime.sub_k_output_raw, [0; 2]);
            } else {
                assert_eq!(runtime.sub_k_output_raw, [0; 2]);
                assert_eq!(runtime.sub_l_output_raw, [0; 2]);
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type10_late_basis_block_retains_unwound_task_prefix_without_replay() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(55))
        .unwrap()
        .id;
    isolate_targets(&mut manager, id);
    let entity = manager.entity_mut(id).unwrap();
    admit_callbacks(entity, true);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let owner = Intro2Type10Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let tick = tick_intro2_type10(
        &mut manager,
        owner,
        Intro2Type10Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type10Outcome::Blocked {
                reason: Intro2Type10Block::Mover(
                    crate::gkl_common_mover::GklCommonMoverBlock::BodyBasisUnavailable
                ),
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    assert!(owner.has_pending_prefix());
    let entity = manager.entity_mut(id).unwrap();
    let task_before = entity.actor_task_state(ActorTaskSlot::Primary).copied();
    let Some(ActorTaskRuntime::SharedRetarget(task)) = task_before else {
        panic!("retained wander")
    };
    assert_eq!(task.elapsed_ms(), 20);
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(task_id)
            .unwrap()
            .in_callback
    );
    let runtime_before = entity.intro2_type10_runtime;
    let position_before = entity.position_raw();
    let mut frozen_fx = fx.fork_for_main_base_abort_transaction();
    let repeat = tick_intro2_type10(
        &mut manager,
        owner,
        Intro2Type10Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 125_000,
            retail_tick: 8,
        },
    );
    assert!(matches!(
        repeat.outcome,
        Intro2Type10Outcome::Pending { .. }
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary).copied(),
        task_before
    );
    assert_eq!(entity.intro2_type10_runtime, runtime_before);
    assert_eq!(entity.position_raw(), position_before);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        frozen_fx.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_acquisition_visits_new_aim_once_and_new_primary_next_pass() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(55))
        .unwrap()
        .id;
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    isolate_targets(&mut manager, id);
    let entity = manager.entity_mut(id).unwrap();
    admit_callbacks(entity, true);
    let position = entity.position_raw();
    let old_primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let old_secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags |= 1;
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    target.set_position_raw([
        position[0].wrapping_add(700),
        position[1],
        position[2].wrapping_add(1100),
    ]);
    let mut owner = Intro2Type10Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let mut retained_primary = None;
    for index in 0..3 {
        let tick = tick_intro2_type10(
            &mut manager,
            owner,
            Intro2Type10Frame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 3000 + index,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2Type10Outcome::Advanced {
                    callback_enabled: true,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
        let entity = manager.entity_mut(id).unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(primary, old_primary);
        assert!(entity.actor_tasks.wrapper_flags(old_primary).is_none());
        assert!(entity.actor_tasks.wrapper_flags(old_secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        if let Some(previous) = retained_primary {
            assert_eq!(primary, previous);
        }
        retained_primary = Some(primary);
        let Some(ActorTaskRuntime::ChaseTarget(chase)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("native Chase")
        };
        assert_eq!(chase.elapsed_ms(), index * 20);
        let Some(ActorTaskRuntime::AimAndFire(aim)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("native Aim")
        };
        assert_eq!(aim.elapsed_ms(), (index + 1) * 20);
        assert!(super::search::pursuing_graph_authenticates(entity));
    }
}
