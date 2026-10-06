use super::*;
use crate::{
    base_factory_progression::ProgressiveDeathState,
    entity::BaseFactoryRuntimeState,
    ordinary_type9_initial_selection::{
        LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK, LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
    },
    ordinary_type9_wander_production::{
        tick_ordinary_type9_wander_owner_with_random, OrdinaryType9WanderProductionBlock,
        OrdinaryType9WanderProductionFrame, OrdinaryType9WanderProductionState,
    },
};
use v2k_formats::collision::StatusComponentDescriptor;

const JOB_ID: u32 = 0x04AF_0100;
const PLAYER_ID: u32 = 0x04AF_0101;

fn append_root_candidates(manager: &mut EntityManager, owner_id: u32) {
    // This compact Run Away fixture supplies generic nearby rows rather than
    // factory constructors. Make their absent job component explicit before
    // testing the first complete JobNearby scan.
    let ids = manager
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    for id in ids {
        let entity = manager.entity_mut_for_test(id).unwrap();
        if entity.base_factory_runtime == RetailRuntimeValue::Unresolved {
            entity.base_factory_runtime = RetailRuntimeValue::Known(None);
        }
    }
    let position = manager
        .iter_all()
        .find(|entity| entity.id == owner_id)
        .unwrap()
        .position_raw();
    let mut job = Entity::unresolved_port_entity(JOB_ID, EntityKind::Unknown(66), 66);
    job.capability_flags = LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK;
    job.collision = collision(1);
    job.base_factory_runtime = RetailRuntimeValue::Known(Some(BaseFactoryRuntimeState {
        status_descriptor: StatusComponentDescriptor {
            raw_word_at_0x00: 0,
            variable_bindings: [0; 6],
            raw_tail: [0; 10],
        },
        control_value_raw: 0,
        required_scientists: 1,
        current_scientists: 0,
        lifter_progress_raw: 0,
        production_progress_raw: 0,
        recovery_progress_raw: 0,
        production: None,
        live_owner: None,
        progressive_death: ProgressiveDeathState::idle(0),
    }));
    job.set_motion_raw(position, [0; 3]);
    manager.append_entity_for_test(job);
    let mut player = Entity::unresolved_port_entity(PLAYER_ID, EntityKind::Unknown(99), 99);
    player.capability_flags = LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK;
    player.collision = collision(1);
    player.base_factory_runtime = RetailRuntimeValue::Known(None);
    player.set_motion_raw(
        [position[0].wrapping_add(20), position[1], position[2]],
        [0; 3],
    );
    manager.append_entity_for_test(player);
}

fn selector_word(manager: &EntityManager, entity_id: u32, class_id: u8) -> u32 {
    (0..=u32::from(u16::MAX))
        .step_by(128)
        .find(
            |word| match plan_root_with_word(manager, entity_id, *word).selection() {
                OrdinaryType9RootSelection::Alternate { program } => program.class_id == class_id,
                OrdinaryType9RootSelection::Weighted { selection, .. } => {
                    selection.program.class_id == class_id
                }
            },
        )
        .expect("the authored nearby conditions give the requested class positive weight")
}

fn assert_new_graph(manager: &EntityManager, entity_id: u32, class_id: u8, callback_elapsed: u32) {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .unwrap();
    if class_id == 54 {
        assert!(matches!(entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(task)) if task.elapsed_ms() == 0 && task.target_id() == Some(JOB_ID)));
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    } else {
        assert!(matches!(entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms() == 0));
        assert!(matches!(entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AttractAttentionCue(task)) if task.elapsed_ms() == callback_elapsed / 1_000));
    }
    assert!(
        entity.actor_task_state(ActorTaskSlot::Secondary).is_none(),
        "the newborn Secondary runs once in the original traversal"
    );
}

#[test]
fn expired_run_away_hands_off_45_and_54_without_replaying_primary_or_prefix() {
    for class_id in [45, 54] {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        prepare_expired_acquiring_scheduler(&mut manager, &mut owner);
        append_root_candidates(&mut manager, entity_id);
        let selector = selector_word(&manager, entity_id, class_id);
        let expected = if class_id == 45 {
            vec![0x1111, 0x2222, selector, 0x1000, 0xD2F6, 0x1234]
        } else {
            vec![0x1111, 0x2222, selector, 0xD2F6]
        };
        let mut words = VecDeque::from(expected.clone());
        let mut consumed = Vec::new();
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let text_receipts_before = manager
            .pending_fresh_level1_type9_resource_text_receipts()
            .len();
        let live_text = std::cell::RefCell::new(Vec::new());
        let tick = tick_ordinary_type9_run_away_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9RunAwayProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                resources: &resources,
                elapsed_micros: 501_000,
                global_elapsed_micros: 501_000,
                retail_tick: 17,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("no repeated prefix, selector or Primary");
                consumed.push(word);
                word
            },
        );
        let OrdinaryType9RunAwayProductionOutcome::RootContinuation { outcome, .. } = tick.outcome
        else {
            panic!("expected a root-family handoff: {:?}", tick.outcome);
        };
        assert!(
            matches!(
                (class_id, outcome.as_ref()),
                (
                    45,
                    OrdinaryType9WanderProductionOutcome::RootAttractAttentionPublished { .. }
                ) | (
                    54,
                    OrdinaryType9WanderProductionOutcome::RootGoToJobPublished { .. }
                )
            ),
            "class {class_id}: {outcome:?}"
        );
        assert!(tick.retained_owner.is_none());
        let replacement = tick
            .replacement_owner
            .expect("the replacement retains sole actor custody");
        let OrdinaryType9WanderProductionState::PostBasisTailPending { post_task_frame } =
            replacement.state()
        else {
            panic!("successful root handoff finishes its original outer tail");
        };
        assert_eq!(consumed, expected);
        assert!(words.is_empty());
        assert_new_graph(
            &manager,
            entity_id,
            class_id,
            post_task_frame.callback_elapsed_micros(),
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(7 + post_task_frame.callback_elapsed_micros())
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(501_000 - post_task_frame.callback_elapsed_micros())
        );
        assert_eq!(
            manager
                .pending_fresh_level1_type9_resource_text_receipts()
                .len(),
            text_receipts_before
        );
        let live_text = live_text.borrow();
        assert_eq!(live_text.len(), usize::from(class_id == 45));
        if class_id == 45 {
            assert_eq!(
                (
                    live_text[0].0.event,
                    live_text[0].0.global_resource_id,
                    live_text[0].1
                ),
                (0x10, 0xf0, 17)
            );
        }
    }
}

#[test]
fn root_handoff_metadata_retry_keeps_selector_capacity_and_first_resolved_animation() {
    for class_id in [45, 54] {
        let (mut manager, mut owner, _) = root_pending_owner();
        let entity_id = owner.entity_id();
        append_root_candidates(&mut manager, entity_id);
        let selector = selector_word(&manager, entity_id, class_id);
        owner.pending_root_plan = Some(plan_root_with_word(&manager, entity_id, selector));
        owner.pending_root_sub_a = Some(live_sub_a(&manager, entity_id));
        let continuation = owner.pending_root_dispatcher_continuation.as_mut().unwrap();
        for snapshot in &mut continuation.snapshots {
            snapshot.job_capacity = job_capacity_from_entity(
                manager
                    .iter_all()
                    .find(|entity| entity.id == snapshot.id)
                    .unwrap(),
            );
        }
        for entity in manager
            .iter_all()
            .filter(|entity| matches!(entity.id, JOB_ID | PLAYER_ID))
        {
            continuation.snapshots.push(EntitySnapshot {
                id: entity.id,
                entity_type: entity.entity_type,
                position_raw: entity.position_raw(),
                velocity_raw: entity.velocity_raw(),
                capability_flags: entity.capability_flags,
                active: entity.active,
                collision: entity.collision.clone(),
                job_capacity: job_capacity_from_entity(entity),
            });
        }
        let original_animation = continuation.actor_animation_at_transition.unwrap();
        let original_sound = manager
            .type_runtime_metadata(9)
            .unwrap()
            .accepted_hit_presentation_sound_id;
        manager
            .type_runtime_metadata_mut_for_test(9)
            .unwrap()
            .accepted_hit_presentation_sound_id = RetailRuntimeValue::Known(Some(1));
        let resources = main_base_abort_world_cache(flat_terrain());
        let mut world_fx = WorldFx::new();
        let first = continue_ordinary_type9_run_away_root_transition(
            &mut manager,
            owner,
            &resources,
            OrdinaryType9LiveNotificationContext {
                dispatch_resource_text: &|_, _| {},
                retail_tick: 0,
            },
            &mut world_fx,
            &mut |_| panic!("frozen selector and rejected metadata consume no RNG"),
            |_| panic!("45/54 cannot allocate root Wander"),
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9RunAwayProductionOutcome::RootContinuation { .. }
        ));
        let mut replacement = first
            .replacement_owner
            .expect("metadata repair retains the handoff");
        assert!(matches!(
            replacement.state(),
            OrdinaryType9WanderProductionState::RootTransitionPending { .. }
        ));
        let collision_before_retry = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
            .collision
            .clone();
        manager
            .type_runtime_metadata_mut_for_test(9)
            .unwrap()
            .accepted_hit_presentation_sound_id = original_sound;
        if class_id == 45 {
            let mut replacement_animation = original_animation;
            replacement_animation.advance_neutral(20_000, 0);
            assert_ne!(replacement_animation, original_animation);
            manager
                .entity_mut_for_test(entity_id)
                .unwrap()
                .actor_animation_runtime = RetailRuntimeValue::Known(Some(replacement_animation));
            let blocked = tick_ordinary_type9_wander_owner_with_random(
                &mut manager,
                replacement,
                OrdinaryType9WanderProductionFrame {
                    dispatch_resource_text: &|_, _| {},
                    resources: &resources,
                    elapsed_micros: 999_000,
                    global_elapsed_micros: 9_999_000,
                    retail_tick: 999,
                },
                &mut world_fx,
                |_| panic!("a substituted animator cannot replay the selector or prefix"),
            );
            assert!(matches!(
                blocked.outcome,
                OrdinaryType9WanderProductionOutcome::Blocked {
                    reason:
                        OrdinaryType9WanderProductionBlock::RootPredecessorActorAnimationChanged { .. },
                    ..
                }
            ));
            replacement = blocked.retained_owner.unwrap();
            assert_eq!(
                manager
                    .iter_all()
                    .find(|entity| entity.id == entity_id)
                    .unwrap()
                    .collision,
                collision_before_retry
            );
            manager
                .entity_mut_for_test(entity_id)
                .unwrap()
                .actor_animation_runtime = RetailRuntimeValue::Known(Some(original_animation));
        } else {
            let RetailRuntimeValue::Known(Some(job)) = &mut manager
                .entity_mut_for_test(JOB_ID)
                .unwrap()
                .base_factory_runtime
            else {
                panic!("live job remains resolved");
            };
            job.current_scientists = job.required_scientists;
        }
        let expected = if class_id == 45 {
            vec![0x1000, 0xD2F6, 0x1234]
        } else {
            vec![0xD2F6]
        };
        let mut words = VecDeque::from(expected.clone());
        let mut consumed = Vec::new();
        let resumed = tick_ordinary_type9_wander_owner_with_random(
            &mut manager,
            replacement,
            OrdinaryType9WanderProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 999_000,
                global_elapsed_micros: 9_999_000,
                retail_tick: 999,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("only unexecuted constructor suffix draws remain");
                consumed.push(word);
                word
            },
        );
        assert!(
            matches!(
                resumed.outcome,
                OrdinaryType9WanderProductionOutcome::RootAttractAttentionPublished { .. }
                    | OrdinaryType9WanderProductionOutcome::RootGoToJobPublished { .. }
            ),
            "{:?}",
            resumed.outcome
        );
        assert_eq!(consumed, expected);
        assert_new_graph(&manager, entity_id, class_id, 20_000);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            collision_before_retry.recent_relation_elapsed_us_at_0x68
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            collision_before_retry.callback_scheduler_accumulator_us_at_0x6c
        );
    }
}
