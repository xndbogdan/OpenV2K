use super::*;
use crate::{
    common_mover::SubAPropulsionRuntime,
    entity::EntityKind,
    entity_collision_state::{CommonMoverComponentTopology, RetailStateWord},
};
use v2k_formats::collision::{CommonAxisDescriptor, SubAPropulsionDescriptor};

const OWNER: u32 = 1;
const TARGET: u32 = 2;
const OWNER_TYPE: u32 = 53;

fn fixture(target_id: u32) -> (EntityManager, crate::actor_task_owner::ActorTaskId) {
    let metadata = EntityTypeRuntimeMetadata {
        common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            ..Default::default()
        }),
        sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
            acceleration_raw: 1500,
            overspeed_correction_raw: -3000,
            target_speed_base_raw: 400,
        })),
        ..Default::default()
    };
    let mut owner = Entity::unresolved_port_entity(OWNER, EntityKind::from_type(53), OWNER_TYPE);
    owner.set_position_raw([100, -30, 200]);
    owner.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: 256,
        raw_word_at_0x04: 5,
    });
    let mut sub_a = SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(400), 1, 71);
    let task = ChaseTargetTaskState::prepare_after_allocation(
        OWNER,
        owner.position_raw(),
        target_id,
        &metadata,
    )
    .unwrap()
    .map_task(ActorTaskRuntime::ChaseTarget)
    .apply_suffix(|_, suffix| {
        sub_a.apply_shared_initializer_target_speed_write(suffix.sub_a_target_speed_raw().unwrap());
    });
    owner.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
    owner
        .actor_tasks
        .replace_prepared(ActorTaskSlot::Primary, task);
    let primary = owner
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let mut target = Entity::unresolved_port_entity(TARGET, EntityKind::from_type(9), 9);
    target.set_motion_raw([120, 0, 230], [11, -22, 33]);
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(0x8000);
    let mut records: Vec<EntityTypeRuntimeMetadata> =
        std::iter::repeat_with(Default::default).take(54).collect();
    records[OWNER_TYPE as usize] = metadata;
    (
        EntityManager::from_entities_with_type_metadata_for_test(
            vec![owner, target],
            records,
            false,
        ),
        primary,
    )
}

fn frame(
    elapsed_micros: u32,
    dispatch_mode: CommonMoverDispatchMode,
) -> ChaseTargetLivePrimaryFrame {
    ChaseTargetLivePrimaryFrame {
        entity_id: OWNER,
        elapsed_micros,
        dispatch_mode,
    }
}

fn task(manager: &mut EntityManager) -> ChaseTargetTaskState {
    let Some(ActorTaskRuntime::ChaseTarget(task)) = manager
        .entity_mut(OWNER)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Chase remains the current Primary")
    };
    *task
}

fn assert_unwound(manager: &mut EntityManager, primary: crate::actor_task_owner::ActorTaskId) {
    let flags = manager
        .entity_mut(OWNER)
        .unwrap()
        .actor_tasks
        .wrapper_flags(primary)
        .unwrap();
    assert!(flags.alive);
    assert!(!flags.in_callback);
}

#[test]
fn shared_chase_ages_before_target_or_axis_block_and_unwinds() {
    for axis_block in [false, true] {
        let (mut manager, primary) = fixture(TARGET);
        if axis_block {
            manager
                .entity_mut(OWNER)
                .unwrap()
                .actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        } else {
            manager
                .entity_mut(TARGET)
                .unwrap()
                .collision
                .state_flags_at_0x08 = RetailStateWord::from_known_bits(0, 0);
        }
        let before = task(&mut manager).private_state();
        let result = tick_chase_target_live_primary(
            &mut manager,
            frame(20_999, CommonMoverDispatchMode::Normal),
            |_| -> Result<bool, ()> { panic!("blocked before mover") },
        );
        assert_eq!(
            result,
            Err(ChaseTargetLivePrimaryError::Runtime(if axis_block {
                "Chase actor axis"
            } else {
                "Chase target dying bit"
            }))
        );
        assert_eq!(task(&mut manager).elapsed_ms(), 20);
        assert_eq!(task(&mut manager).private_state(), before);
        assert_unwound(&mut manager, primary);
    }
}

#[test]
fn shared_chase_invalid_target_precedes_axis_mover_and_expired_timeout() {
    // Null handle, absent allocation, removed allocation, zero state and dying
    // state take the same tag before the otherwise unresolved live axis.
    for case in 0..5 {
        let target_id = match case {
            0 => 0,
            1 => 99,
            _ => TARGET,
        };
        let (mut manager, primary) = fixture(target_id);
        manager
            .entity_mut(OWNER)
            .unwrap()
            .actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        match case {
            2 => manager.remove_entity_for_test(TARGET),
            3 => {
                manager
                    .entity_mut(TARGET)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08 = RetailStateWord::exact(0)
            }
            4 => {
                manager
                    .entity_mut(TARGET)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08 = RetailStateWord::exact(DYING_STATE_BIT)
            }
            _ => {}
        }
        let transition = tick_chase_target_live_primary(
            &mut manager,
            frame(5_001_000, CommonMoverDispatchMode::Restricted),
            |_| -> Result<bool, ()> { panic!("invalid target must skip mover") },
        )
        .unwrap()
        .unwrap();
        assert_eq!(transition.task_id, primary);
        assert_eq!(
            transition.reason,
            ChaseTargetTransitionReason::TaggedCallbackResult(
                ChaseTargetTaggedSingleton::InvalidTarget
            )
        );
        assert_eq!(
            transition.committed_prefix.lifetime_status,
            ChaseTargetLifetimeStatus::OwnerTransitionDue
        );
        assert_eq!(task(&mut manager).elapsed_ms(), 5001);
        assert_unwound(&mut manager, primary);
    }
}

#[test]
fn shared_chase_strict_wrapped_route_runs_mover_and_forwards_both_modes() {
    for mode in [
        CommonMoverDispatchMode::Normal,
        CommonMoverDispatchMode::Restricted,
    ] {
        for (position, limit, expected) in [
            ([355, -30, 200], 256, None),
            (
                [356, -30, 200],
                256,
                Some(ChaseTargetTaggedSingleton::OutOfRange),
            ),
            (
                [100, 226, 200],
                256,
                Some(ChaseTargetTaggedSingleton::OutOfRange),
            ),
            ([i16::MIN, -30, 200], 0, None),
        ] {
            let (mut manager, primary) = fixture(TARGET);
            manager
                .entity_mut(TARGET)
                .unwrap()
                .set_position_raw(position);
            manager
                .entity_mut(OWNER)
                .unwrap()
                .actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
                strict_axis_limit_raw: limit,
                raw_word_at_0x04: 5,
            });
            let mut calls = 0;
            let result =
                tick_chase_target_live_primary(&mut manager, frame(20_999, mode), |request| {
                    calls += 1;
                    assert_eq!(request.elapsed_micros, 20_999);
                    assert_eq!(request.dispatch_mode, mode);
                    let RetailRuntimeValue::Known(Some(target)) = request.tracked_target else {
                        panic!("validated live target snapshot")
                    };
                    assert_eq!(target.position_raw, position);
                    assert_eq!(target.velocity_raw, [11, -22, 33]);
                    assert!(
                        request
                            .entity
                            .actor_tasks
                            .wrapper_flags(primary)
                            .unwrap()
                            .in_callback
                    );
                    request.target.reversal_timer_ms = 123;
                    Ok::<_, ()>(true)
                })
                .unwrap();
            assert_eq!(calls, 1, "out-of-range still calls01430 in either mode");
            assert_eq!(
                result.map(|transition| transition.reason),
                expected.map(ChaseTargetTransitionReason::TaggedCallbackResult)
            );
            assert_eq!(task(&mut manager).private_state().reversal_timer_ms, 123);
            assert_unwound(&mut manager, primary);
        }
    }
    let (mut manager, _) = fixture(TARGET);
    manager
        .entity_mut(OWNER)
        .unwrap()
        .set_position_raw([i16::MAX, 0, 0]);
    manager
        .entity_mut(TARGET)
        .unwrap()
        .set_position_raw([i16::MIN, 0, 0]);
    assert_eq!(
        tick_chase_target_live_primary(
            &mut manager,
            frame(1000, CommonMoverDispatchMode::Normal),
            |_| Ok::<_, ()>(true)
        )
        .unwrap(),
        None
    );
}

#[test]
fn shared_chase_out_of_range_ignores_zero_mover_and_in_range_tags_it() {
    for (position, singleton) in [
        ([120, 0, 230], ChaseTargetTaggedSingleton::InRangeZeroMover),
        ([356, -30, 200], ChaseTargetTaggedSingleton::OutOfRange),
    ] {
        let (mut manager, _) = fixture(TARGET);
        manager
            .entity_mut(TARGET)
            .unwrap()
            .set_position_raw(position);
        let result = tick_chase_target_live_primary(
            &mut manager,
            frame(5_001_000, CommonMoverDispatchMode::Normal),
            |_| Ok::<_, ()>(false),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            result.reason,
            ChaseTargetTransitionReason::TaggedCallbackResult(singleton)
        );
    }
}

#[test]
fn shared_chase_keeps_mover_and_private_prefix_on_error_and_panic() {
    for panic_in_mover in [false, true] {
        let (mut manager, primary) = fixture(TARGET);
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tick_chase_target_live_primary(
                &mut manager,
                frame(20_000, CommonMoverDispatchMode::Normal),
                |request| {
                    request.target.target_position_raw = [300, -40, 500];
                    request.target.direction = -1;
                    request.target.reversal_timer_ms = 700;
                    request
                        .entity
                        .set_motion_raw([101, -29, 202], [31, -41, 59]);
                    if panic_in_mover {
                        panic!("late mover panic")
                    }
                    Err::<bool, _>("late Sub-H block")
                },
            )
        }));
        if panic_in_mover {
            assert!(run.is_err());
        } else {
            assert_eq!(
                run.unwrap(),
                Err(ChaseTargetLivePrimaryError::Mover("late Sub-H block"))
            );
        }
        let state = task(&mut manager);
        assert_eq!(state.elapsed_ms(), 20);
        assert_eq!(
            state.private_state(),
            WanderNearPrivateState {
                target_position_raw: [300, -40, 500],
                tracked_entity_handle: TARGET,
                direction: -1,
                reversal_timer_ms: 700
            }
        );
        let owner = manager.entity_mut(OWNER).unwrap();
        assert_eq!(owner.position_raw(), [101, -29, 202]);
        assert_eq!(owner.velocity_raw(), [31, -41, 59]);
        assert_unwound(&mut manager, primary);
    }
}

#[test]
fn shared_chase_retirement_discards_tag_timeout_and_preserves_new_primary() {
    for moved in [false, true] {
        let (mut manager, primary) = fixture(TARGET);
        let result = tick_chase_target_live_primary(
            &mut manager,
            frame(5_001_000, CommonMoverDispatchMode::Normal),
            |request| {
                request.target.direction = -1;
                request.entity.actor_tasks.replace_prepared(
                    ActorTaskSlot::Primary,
                    crate::actor_task_owner::PreparedActorTask::new(
                        ActorTaskRuntime::SharedRetarget(
                            crate::shared_retarget_mover::SharedRetargetTaskState::new(
                                [9, 8, 7],
                                500,
                            ),
                        ),
                    ),
                );
                let old = request.entity.actor_tasks.wrapper_flags(primary).unwrap();
                assert!(!old.alive && old.in_callback);
                Ok::<_, ()>(moved)
            },
        )
        .unwrap();
        assert_eq!(result, None);
        let owner = manager.entity_mut(OWNER).unwrap();
        assert!(owner.actor_tasks.wrapper_flags(primary).is_none());
        assert!(matches!(owner.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms()==0));
    }
}

#[test]
fn shared_chase_absent_h_skips_controller_even_with_g_and_unknown_a() {
    let (mut manager, _) = fixture(TARGET);
    let metadata = manager
        .type_runtime_metadata_mut_for_test(OWNER_TYPE)
        .unwrap();
    metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
        sub_g: true,
        ..Default::default()
    });
    metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved;
    manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let before_a = manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime;
    let before = task(&mut manager).private_state();
    let result = tick_chase_target_live_primary(
        &mut manager,
        frame(20_000, CommonMoverDispatchMode::Normal),
        |_| Ok::<_, ()>(true),
    )
    .unwrap();
    assert_eq!(result, None);
    assert_eq!(task(&mut manager).private_state(), before);
    assert_eq!(
        manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime,
        before_a
    );
}

fn enable_h(manager: &mut EntityManager) {
    let metadata = manager
        .type_runtime_metadata_mut_for_test(OWNER_TYPE)
        .unwrap();
    metadata.common_mover_topology = RetailRuntimeValue::Known(CommonMoverComponentTopology {
        sub_a: true,
        sub_h: true,
        ..Default::default()
    });
    metadata.sub_a_propulsion_descriptor =
        RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
            acceleration_raw: 1500,
            overspeed_correction_raw: -3000,
            target_speed_base_raw: -401,
        }));
}

#[test]
fn shared_chase_h_controller_uses_fresh_wrapped_xz_and_signed_speed() {
    // Initial positions are close and in route range. The mover moves the owner
    // to the final test distance; controller lookup must not reuse that prefix.
    for (dx, dz, speed, direction) in [
        (0x37f, 0x37f, -401, -1),
        (0x380, 0x37f, 1, 1),
        (0x37f, 0x380, 1, 1),
        (0x3ff, 0x3ff, 1, 1),
        (0x400, 0, -401, 1),
        (0, 0x400, -401, 1),
        (-0x400, 0, -401, 1),
    ] {
        let (mut manager, primary) = fixture(TARGET);
        enable_h(&mut manager);
        let result = tick_chase_target_live_primary(
            &mut manager,
            frame(20_000, CommonMoverDispatchMode::Restricted),
            |request| {
                assert_eq!(request.dispatch_mode, CommonMoverDispatchMode::Restricted);
                request.entity.set_position_raw([
                    120i16.wrapping_add(dx),
                    12345,
                    230i16.wrapping_add(dz),
                ]);
                Ok::<_, ()>(true)
            },
        )
        .unwrap();
        assert_eq!(result, None);
        assert_eq!(task(&mut manager).private_state().direction, direction);
        let RetailRuntimeValue::Known(Some(sub_a)) =
            manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime
        else {
            panic!("retained Sub-A allocation")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(speed));
        assert_eq!(sub_a.direction_multiplier(), direction);
        assert_eq!(sub_a.drive_scale_percent(), 71);
        assert_unwound(&mut manager, primary);
    }

    let (mut manager, _) = fixture(TARGET);
    enable_h(&mut manager);
    manager
        .entity_mut(TARGET)
        .unwrap()
        .set_position_raw([i16::MIN, 0, i16::MIN]);
    manager
        .entity_mut(OWNER)
        .unwrap()
        .set_position_raw([i16::MAX, 0, i16::MAX]);
    tick_chase_target_live_primary(
        &mut manager,
        frame(1000, CommonMoverDispatchMode::Normal),
        |_| Ok::<_, ()>(true),
    )
    .unwrap();
    assert_eq!(task(&mut manager).private_state().direction, -1);
}

#[test]
fn shared_chase_h_controller_checks_only_branch_consumed_a_state() {
    for no_a in [false, true] {
        let (mut manager, primary) = fixture(TARGET);
        enable_h(&mut manager);
        let metadata = manager
            .type_runtime_metadata_mut_for_test(OWNER_TYPE)
            .unwrap();
        metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved;
        if no_a {
            metadata.common_mover_topology =
                RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_h: true,
                    ..Default::default()
                });
            manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime =
                RetailRuntimeValue::Unresolved;
        }
        tick_chase_target_live_primary(
            &mut manager,
            frame(20_000, CommonMoverDispatchMode::Normal),
            |request| {
                request.entity.set_position_raw([120 + 0x380, 0, 230]);
                Ok::<_, ()>(true)
            },
        )
        .unwrap();
        assert_eq!(task(&mut manager).private_state().direction, 1);
        let runtime = manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime;
        if no_a {
            assert_eq!(runtime, RetailRuntimeValue::Unresolved);
        } else {
            assert_eq!(
                runtime,
                RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(1),
                    1,
                    71
                )))
            );
        }
        assert_unwound(&mut manager, primary);
    }
}

#[test]
fn shared_chase_controller_admission_is_late_and_retains_mover_prefix() {
    for failure in [
        "Chase component topology",
        "Chase Sub-A descriptor",
        "Chase Sub-A runtime",
    ] {
        let (mut manager, primary) = fixture(TARGET);
        enable_h(&mut manager);
        match failure {
            "Chase component topology" => {
                manager
                    .type_runtime_metadata_mut_for_test(OWNER_TYPE)
                    .unwrap()
                    .common_mover_topology = RetailRuntimeValue::Unresolved
            }
            "Chase Sub-A descriptor" => {
                manager
                    .type_runtime_metadata_mut_for_test(OWNER_TYPE)
                    .unwrap()
                    .sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved
            }
            _ => {
                manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime =
                    RetailRuntimeValue::Unresolved
            }
        }
        let before_a = manager.entity_mut(OWNER).unwrap().sub_a_propulsion_runtime;
        assert_eq!(
            tick_chase_target_live_primary(
                &mut manager,
                frame(20_999, CommonMoverDispatchMode::Normal),
                |request| {
                    request.entity.set_velocity_raw([7, 8, 9]);
                    request.target.reversal_timer_ms = 777;
                    Ok::<_, ()>(true)
                }
            ),
            Err(ChaseTargetLivePrimaryError::Runtime(failure))
        );
        let task = task(&mut manager);
        assert_eq!(task.elapsed_ms(), 20);
        assert_eq!(task.private_state().reversal_timer_ms, 777);
        assert_eq!(task.private_state().direction, 1);
        let entity = manager.entity_mut(OWNER).unwrap();
        assert_eq!(entity.velocity_raw(), [7, 8, 9]);
        assert_eq!(entity.sub_a_propulsion_runtime, before_a);
        assert_unwound(&mut manager, primary);
    }
}

#[test]
fn shared_chase_tagged_paths_skip_unresolved_controller_policy() {
    for path in 0..3 {
        let (mut manager, primary) = fixture(if path == 0 { 0 } else { TARGET });
        manager
            .type_runtime_metadata_mut_for_test(OWNER_TYPE)
            .unwrap()
            .common_mover_topology = RetailRuntimeValue::Unresolved;
        if path == 2 {
            manager
                .entity_mut(TARGET)
                .unwrap()
                .set_position_raw([356, -30, 200]);
        }
        let mut calls = 0;
        let transition = tick_chase_target_live_primary(
            &mut manager,
            frame(20_000, CommonMoverDispatchMode::Normal),
            |_| {
                calls += 1;
                Ok::<_, ()>(false)
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(calls, usize::from(path != 0));
        assert_eq!(
            transition.reason,
            ChaseTargetTransitionReason::TaggedCallbackResult(match path {
                0 => ChaseTargetTaggedSingleton::InvalidTarget,
                1 => ChaseTargetTaggedSingleton::InRangeZeroMover,
                _ => ChaseTargetTaggedSingleton::OutOfRange,
            })
        );
        assert_unwound(&mut manager, primary);
    }
}

#[test]
fn shared_chase_timeout_is_strict_with_per_visit_millisecond_truncation() {
    let (mut manager, primary) = fixture(TARGET);
    for (elapsed, expected_age, expired) in [
        (5_000_999, 5000, false),
        (999, 5000, false),
        (1000, 5001, true),
    ] {
        let result = tick_chase_target_live_primary(
            &mut manager,
            frame(elapsed, CommonMoverDispatchMode::Restricted),
            |_| Ok::<_, ()>(true),
        )
        .unwrap();
        assert_eq!(task(&mut manager).elapsed_ms(), expected_age);
        assert_eq!(
            result.map(|transition| transition.reason),
            expired.then_some(ChaseTargetTransitionReason::LifetimeExpired)
        );
        assert_unwound(&mut manager, primary);
    }
}
