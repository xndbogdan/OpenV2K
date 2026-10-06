use super::*;
use crate::{
    common_mover::SubAPropulsionRuntime, entity::EntityKind,
    entity_behavior::BehaviorChoiceListSource,
    entity_collision_state::CommonMoverComponentTopology,
    shared_retarget_mover::SharedRetargetTaskState, sub_h_external_frame::SubHRuntimeState,
};
use v2k_formats::collision::SubAPropulsionDescriptor;

fn fixture(scores: &[i32]) -> (EntityManager, EntityTypeRuntimeMetadata) {
    let mut owner = Entity::unresolved_port_entity(1, EntityKind::from_type(58), 58);
    owner.collision =
        EntityCollisionRuntimeState::from_constructor(None, 0, RetailStateWord::exact(0x68000));
    owner.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(33).unwrap(),
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(999)),
            RetailRuntimeValue::Known(0x12345678),
            *audited_behavior_style(33, 0).unwrap(),
        )
        .unwrap(),
    ));
    owner.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(19), -1, 71),
    ));
    owner.sub_h_external_frame_runtime =
        RetailRuntimeValue::Known(Some(SubHRuntimeState::new(6).unwrap()));
    owner.actor_common_axis_descriptor = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: 1,
        raw_word_at_0x04: 5,
    });
    owner.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
            SharedRetargetTaskState::new([0; 3], 500),
        )),
    );
    owner.actor_tasks.replace_prepared(
        ActorTaskSlot::Secondary,
        PreparedActorTask::new(ActorTaskRuntime::FollowBeaconAcquisition(
            FollowBeaconAcquisitionTaskState::new(WrappedAxisRange::from_raw(0x300), 5),
        )),
    );
    let mut entities = vec![owner];
    for (index, score) in scores.iter().copied().enumerate() {
        let mut target =
            Entity::unresolved_port_entity(index as u32 + 2, EntityKind::from_type(16), 16);
        target.collision =
            EntityCollisionRuntimeState::from_constructor(None, 0, RetailStateWord::exact(0x68000));
        target.capability_flags = 0x100;
        target.authored_follow_beacon_priority_raw = Some(score);
        target.set_position_raw([100, 0, 100]);
        entities.push(target);
    }
    let metadata = EntityTypeRuntimeMetadata {
        common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
            sub_a: true,
            sub_h: true,
            sub_e: true,
            ..Default::default()
        }),
        sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
            acceleration_raw: 1500,
            overspeed_correction_raw: -3000,
            target_speed_base_raw: 300,
        })),
        ..Default::default()
    };
    (EntityManager::from_entities_for_test(entities), metadata)
}

#[test]
fn shared_follow_acquisition_tie_then_suffix_rng_publishes_before_old_wrapper_unwind() {
    let (mut manager, metadata) = fixture(&[9, 9]);
    let entity = manager.entity_mut(1).unwrap();
    let old = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
    let before_state = entity.collision.state_flags_at_0x08;
    let mut words = [1, 0xffff].into_iter();
    assert_eq!(
        tick_follow_beacons_live_acquisition(
            &mut manager,
            FollowBeaconsLiveAcquisitionRequest {
                entity_id: 1,
                metadata: &metadata
            },
            &mut || words.next().unwrap()
        )
        .unwrap(),
        FollowBeaconsLiveAcquisitionOutcome::Following { target_id: 3 }
    );
    assert_eq!(words.next(), None);
    let entity = manager.entity_mut(1).unwrap();
    for id in old.into_iter().flatten() {
        assert!(entity.actor_tasks.wrapper_flags(id).is_none());
    }
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) if task.elapsed_ms()==0)
    );
    assert_eq!(entity.collision.state_flags_at_0x08, before_state);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("context")
    };
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(3))
    );
    assert_eq!(
        context.auxiliary_word_at_0x0c(),
        RetailRuntimeValue::Known(0x12345678)
    );
    assert_eq!(
        context.choice_list_source(),
        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
    );
    assert_eq!(
        context.active_style().style_address(),
        FOLLOW_BEACONS_FOLLOWING_STYLE_ADDRESS
    );
    let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
        panic!("A")
    };
    assert_eq!(
        a.target_speed_raw(),
        RetailRuntimeValue::Known(400),
        "03B70 fixed base*4/3 follows its random write"
    );
    assert_eq!(a.direction_multiplier(), 1);
    assert_eq!(a.drive_scale_percent(), 71);
}

#[test]
fn shared_follow_no_positive_target_keeps_graph_after_zero_score_tie_draws() {
    let (mut manager, metadata) = fixture(&[0, 0]);
    let entity = manager.entity_mut(1).unwrap();
    let context = entity.current_behavior_context;
    let primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let a = entity.sub_a_propulsion_runtime;
    let mut draws = 0;
    assert_eq!(
        tick_follow_beacons_live_acquisition(
            &mut manager,
            FollowBeaconsLiveAcquisitionRequest {
                entity_id: 1,
                metadata: &metadata
            },
            &mut || {
                draws += 1;
                1
            }
        )
        .unwrap(),
        FollowBeaconsLiveAcquisitionOutcome::NoTarget
    );
    assert_eq!(
        draws, 2,
        "zero-score ties draw, but no following constructor runs"
    );
    let entity = manager.entity_mut(1).unwrap();
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        primary
    );
    assert_eq!(entity.current_behavior_context, context);
    assert_eq!(entity.sub_a_propulsion_runtime, a);
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(secondary)
            .unwrap()
            .in_callback
    );
    assert_eq!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: 0x300,
            raw_word_at_0x04: 0x100
        })
    );
}

#[test]
fn shared_follow_selection_failure_keeps_axis_filter_prefix_and_unwinds() {
    let (mut manager, metadata) = fixture(&[9]);
    manager
        .entity_mut(2)
        .unwrap()
        .authored_follow_beacon_priority_raw = None;
    let secondary = manager
        .entity_mut(1)
        .unwrap()
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    assert_eq!(
        tick_follow_beacons_live_acquisition(
            &mut manager,
            FollowBeaconsLiveAcquisitionRequest {
                entity_id: 1,
                metadata: &metadata
            },
            &mut || panic!("no tie")
        ),
        Err(FollowBeaconsLiveAcquisitionError::Selection(
            FollowBeaconSelectionError::CandidateScoreUnresolved { id: 2 }
        ))
    );
    let entity = manager.entity_mut(1).unwrap();
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Secondary), Some(ActorTaskRuntime::FollowBeaconAcquisition(task)) if task.filter_raw()==0x100)
    );
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(secondary)
            .unwrap()
            .in_callback
    );
    assert_eq!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: 0x300,
            raw_word_at_0x04: 0x100
        })
    );
}

#[test]
fn shared_follow_failed_initializer_keeps_context_target_and_c6b0_fallback_without_suffix_rng() {
    let (mut manager, mut metadata) = fixture(&[9]);
    metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Unresolved;
    let entity = manager.entity_mut(1).unwrap();
    let old = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_tasks.task_in_slot(slot));
    let a = entity.sub_a_propulsion_runtime;
    assert!(matches!(
        tick_follow_beacons_live_acquisition(
            &mut manager,
            FollowBeaconsLiveAcquisitionRequest {
                entity_id: 1,
                metadata: &metadata
            },
            &mut || panic!("failed task has no constructor suffix")
        ),
        Err(FollowBeaconsLiveAcquisitionError::InitializerFallback(_))
    ));
    let entity = manager.entity_mut(1).unwrap();
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        assert!(entity.actor_task_state(slot).is_none());
    }
    for id in old.into_iter().flatten() {
        assert!(entity.actor_tasks.wrapper_flags(id).is_none());
    }
    assert_eq!(entity.sub_a_propulsion_runtime, a);
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("fallback context")
    };
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(2))
    );
    assert_eq!(context, context.with_initializer_failure_fallback());
}

#[test]
fn shared_follow_bad_storage_rejects_before_axis_or_filter_mutation() {
    let (mut manager, metadata) = fixture(&[9]);
    let entity = manager.entity_mut(1).unwrap();
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Unresolved;
    let axis = entity.actor_common_axis_descriptor;
    let secondary = entity.actor_task_state(ActorTaskSlot::Secondary).copied();
    assert_eq!(
        tick_follow_beacons_live_acquisition(
            &mut manager,
            FollowBeaconsLiveAcquisitionRequest {
                entity_id: 1,
                metadata: &metadata
            },
            &mut || panic!("admission")
        ),
        Err(FollowBeaconsLiveAcquisitionError::ComponentStorage)
    );
    let entity = manager.entity_mut(1).unwrap();
    assert_eq!(entity.actor_common_axis_descriptor, axis);
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Secondary).copied(),
        secondary
    );
}

#[test]
fn shared_follow_primary_discards_retired_mover_result_before_old_target_and_timeout() {
    use crate::follow_beacons::live_primary::*;
    for mover_nonzero in [false, true] {
        let (mut manager, metadata) = fixture(&[9]);
        tick_follow_beacons_live_acquisition(
            &mut manager,
            FollowBeaconsLiveAcquisitionRequest {
                entity_id: 1,
                metadata: &metadata,
            },
            &mut || 0,
        )
        .unwrap();
        // The old target is deliberately unresolved. A retired wrapper must
        // never consume this state, even with a nonzero mover return.
        manager.entity_mut(2).unwrap().collision.state_flags_at_0x08 =
            RetailStateWord::from_known_bits(0, 0);
        let old = manager
            .entity_mut(1)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_eq!(
            tick_follow_beacons_live_primary(
                &mut manager,
                FollowBeaconsLivePrimaryRequest {
                    entity_id: 1,
                    callback_elapsed_micros: 10_000_000
                },
                |request| {
                    request
                        .entity
                        .actor_tasks
                        .clear_slot(ActorTaskSlot::Primary);
                    Ok::<_, ()>(mover_nonzero)
                }
            )
            .unwrap(),
            FollowBeaconsFollowingPostUnwind::Continue
        );
        assert!(manager
            .entity_mut(1)
            .unwrap()
            .actor_tasks
            .wrapper_flags(old)
            .is_none());
    }
}
