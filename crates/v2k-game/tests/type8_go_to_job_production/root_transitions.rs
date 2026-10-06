use super::*;
use v2k_game::entity_behavior::{
    behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource, BehaviorContextRuntime,
};
use v2k_game::entity_collision_state::{
    RetailStateWord, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, DYING_STATE_BIT,
};
use v2k_game::go_to_job_owner::{GoToJobTaggedSingleton, GoToJobTransitionReason};
use v2k_game::type8_go_to_job_production::{Type8RootTransitionBlock, Type8RootTransitionReason};

struct Fixture {
    session: GameSession,
    entities: EntityManager,
    owner: Type8GoToJobSchedulerOwner,
    scientist_id: u32,
    factory_id: u32,
    birth_position: [i16; 3],
}

fn fixture() -> Fixture {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .unwrap()
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .unwrap()
        .id;
    let scientist_id = spawn_factory_bound_scientist_at(
        &session,
        &mut entities,
        &mut WorldFx::new(),
        factory_id,
        main_base_id,
        FACTORY_POSITION_RAW,
    );
    let scientist = entities.entity_mut(scientist_id).unwrap();
    let program = behavior_program(54).unwrap();
    // Context words are independent of the task's target. Reselection must
    // preserve them and the original weighted birth receipt.
    scientist.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(0x1234)),
            RetailRuntimeValue::Known(0x9876),
            program.initial_style,
        )
        .unwrap(),
    ));
    let birth_position = scientist.position_raw();
    let owner = Type8GoToJobSchedulerOwner::adopt_published(scientist, &type_metadata(&session)[8])
        .unwrap();
    Fixture {
        session,
        entities,
        owner,
        scientist_id,
        factory_id,
        birth_position,
    }
}

fn close_every_job(entities: &mut EntityManager) {
    let ids = entities
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    for id in ids {
        let entity = entities.entity_mut(id).unwrap();
        if let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime {
            factory.current_scientists = factory.required_scientists;
        }
    }
}

fn invalidate_target(entities: &mut EntityManager, factory_id: u32) {
    entities
        .entity_mut(factory_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
}

fn next_after_draws(draws: usize) -> u16 {
    let mut expected = WorldFx::new();
    for _ in 0..draws {
        expected.next_shared_retail_random_u16();
    }
    expected.next_shared_retail_random_u16()
}

#[v2k_test_support::retail_test]
fn invalid_job_reselects_wander_after_unwind_with_two_draws_and_no_same_frame_visit() {
    let mut fixture = fixture();
    close_every_job(&mut fixture.entities);
    invalidate_target(&mut fixture.entities, fixture.factory_id);
    let birth_selection = fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .initial_behavior;
    let mut fx = WorldFx::new();
    let tick = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        fixture.owner,
        &mut fx,
        fixture.session.cache.terrain(),
        5_001_000,
    );
    match tick.outcome {
        Type8GoToJobSchedulerProductionOutcome::Transition {
            reason,
            selection,
            predecessor_task_id,
            published_task_id,
            ..
        } => {
            assert_eq!(
                reason,
                Type8RootTransitionReason::GoToJob(GoToJobTransitionReason::TaggedCallbackResult(
                    GoToJobTaggedSingleton::InvalidTarget
                )),
                "tagged return precedes simultaneous strict expiry"
            );
            assert_eq!(selection.program.class_id, 6);
            assert_ne!(predecessor_task_id, published_task_id);
        }
        other => panic!("expected actual root publication: {other:?}"),
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        next_after_draws(2),
        "root selector then class6 constructor, with no Wander callback draw"
    );
    let scientist = fixture.entities.entity_mut(fixture.scientist_id).unwrap();
    assert_eq!(scientist.initial_behavior, birth_selection);
    let RetailRuntimeValue::Known(Some(context)) = scientist.current_behavior_context else {
        panic!("context")
    };
    assert_eq!(
        context.target_handle_at_0x08(),
        RetailRuntimeValue::Known(Some(0x1234))
    );
    assert_eq!(
        context.auxiliary_word_at_0x0c(),
        RetailRuntimeValue::Known(0x9876)
    );
    assert_eq!(
        context.active_style(),
        ActiveBehaviorStyle::Audited(behavior_program(6).unwrap().initial_style)
    );
    let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
        scientist.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("published Wander")
    };
    assert_eq!(state.elapsed_ms(), 0);
    assert_eq!(
        state.private_state().target_position_raw,
        fixture.birth_position
    );
    assert!(tick.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn pending_invalid_job_rechecks_live_capacity_and_publishes_fresh_go_to_job() {
    let mut fixture = fixture();
    invalidate_target(&mut fixture.entities, fixture.factory_id);
    let scientist = fixture.entities.entity_mut(fixture.scientist_id).unwrap();
    let saved_sub_a = scientist.sub_a_propulsion_runtime;
    scientist.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
    let mut fx = WorldFx::new();
    let tick = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        fixture.owner,
        &mut fx,
        fixture.session.cache.terrain(),
        20_000,
    );
    assert!(
        matches!(
            tick.outcome,
            Type8GoToJobSchedulerProductionOutcome::TransitionBlocked {
                block: Type8RootTransitionBlock::ComponentRuntimeUnavailable,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    // Repair the dependency and make the authored factory live again before
    // root evaluation. The consumed invalid-target callback is not repeated.
    fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .sub_a_propulsion_runtime = saved_sub_a;
    fixture
        .entities
        .entity_mut(fixture.factory_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, 0);
    let tick = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        tick.retained_owner.unwrap(),
        &mut fx,
        fixture.session.cache.terrain(),
        900_000,
    );
    let Type8GoToJobSchedulerProductionOutcome::Transition {
        selection, reason, ..
    } = tick.outcome
    else {
        panic!("root publication: {:?}", tick.outcome)
    };
    assert_eq!(
        reason,
        Type8RootTransitionReason::GoToJob(GoToJobTransitionReason::TaggedCallbackResult(
            GoToJobTaggedSingleton::InvalidTarget
        ))
    );
    assert_eq!(
        selection.program.class_id, 54,
        "cold retail RNG selects positive JobNearby weight"
    );
    assert_eq!(
        tick.retained_owner.unwrap().target_id(),
        Some(fixture.factory_id)
    );
    assert_eq!(fx.next_shared_retail_random_u16(), next_after_draws(2));
    let Some(ActorTaskRuntime::GoToJob(state)) = fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("GoToJob")
    };
    assert_eq!(state.target_id(), Some(fixture.factory_id));
    assert_eq!(state.elapsed_ms(), 0);
}

#[v2k_test_support::retail_test]
fn unknown_post_unwind_gate_parks_and_retry_never_reenters_consumed_callback() {
    let mut fixture = fixture();
    close_every_job(&mut fixture.entities);
    invalidate_target(&mut fixture.entities, fixture.factory_id);
    let scientist = fixture.entities.entity_mut(fixture.scientist_id).unwrap();
    let flags = scientist.collision.state_flags_at_0x08;
    scientist.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
        flags.known_value_bits(),
        flags.known_mask() & !ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    );
    let mut fx = WorldFx::new();
    let tick = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        fixture.owner,
        &mut fx,
        fixture.session.cache.terrain(),
        20_000,
    );
    assert!(
        matches!(
            tick.outcome,
            Type8GoToJobSchedulerProductionOutcome::TransitionBlocked {
                block: Type8RootTransitionBlock::StateUnavailable,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    let tick = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        owner,
        &mut fx,
        fixture.session.cache.terrain(),
        900_000,
    );
    assert!(matches!(
        tick.outcome,
        Type8GoToJobSchedulerProductionOutcome::TransitionBlocked { .. }
    ));
    let scientist = fixture.entities.entity_mut(fixture.scientist_id).unwrap();
    let Some(ActorTaskRuntime::GoToJob(state)) = scientist.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("old task")
    };
    assert_eq!(state.elapsed_ms(), 20);
    assert_eq!(scientist.position_raw(), fixture.birth_position);
    scientist
        .collision
        .state_flags_at_0x08
        .overwrite(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, 0);
    let resumed = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        tick.retained_owner.unwrap(),
        &mut fx,
        fixture.session.cache.terrain(),
        900_000,
    );
    assert!(
        matches!(
            resumed.outcome,
            Type8GoToJobSchedulerProductionOutcome::Transition { .. }
        ),
        "{:?}",
        resumed.outcome
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        next_after_draws(2),
        "blocked visits did not redraw selection or repeat the old callback"
    );
}

#[v2k_test_support::retail_test]
fn known_suppression_finishes_frame_and_retains_old_task_without_selector_rng() {
    let mut fixture = fixture();
    invalidate_target(&mut fixture.entities, fixture.factory_id);
    fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
    let mut fx = WorldFx::new();
    let first = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        fixture.owner,
        &mut fx,
        fixture.session.cache.terrain(),
        20_000,
    );
    assert!(matches!(
        first.outcome,
        Type8GoToJobSchedulerProductionOutcome::TransitionSuppressed { .. }
    ));
    let second = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        first.retained_owner.unwrap(),
        &mut fx,
        fixture.session.cache.terrain(),
        20_000,
    );
    assert!(matches!(
        second.outcome,
        Type8GoToJobSchedulerProductionOutcome::TransitionSuppressed { .. }
    ));
    let Some(ActorTaskRuntime::GoToJob(state)) = fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("same task")
    };
    assert_eq!(
        state.elapsed_ms(),
        40,
        "known suppression completes each frame, unlike unresolved state"
    );
    assert_eq!(fx.next_shared_retail_random_u16(), next_after_draws(0));
}

#[v2k_test_support::retail_test]
fn wander_retargets_original_birth_anchor_and_callback_failure_cannot_replay_rng() {
    let mut fixture = fixture();
    close_every_job(&mut fixture.entities);
    invalidate_target(&mut fixture.entities, fixture.factory_id);
    let tick = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        fixture.owner,
        &mut WorldFx::new(),
        fixture.session.cache.terrain(),
        20_000,
    );
    assert!(matches!(
        tick.outcome,
        Type8GoToJobSchedulerProductionOutcome::Transition { .. }
    ));
    fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .set_motion_raw([0x6500, 0x100, 0x4900], [0; 3]);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    // Advance both streams to a proven passing 402EB0 gate, without seeding
    // a private RNG or assuming which process word happens to pass.
    while expected.next_shared_retail_random_u16() & 0x3f != 0 {
        fx.next_shared_retail_random_u16();
    }
    let x = (expected.next_shared_retail_random_u16() >> 5) as i16 - 0x400;
    let z = (expected.next_shared_retail_random_u16() >> 5) as i16 - 0x400;
    let blocked = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        tick.retained_owner.unwrap(),
        &mut fx,
        None,
        20_000,
    );
    assert!(
        matches!(
            blocked.outcome,
            Type8GoToJobSchedulerProductionOutcome::WanderBlocked {
                error: Type8GoToJobMoverBlock::TerrainUnavailable,
                ..
            }
        ),
        "{:?}",
        blocked.outcome
    );
    let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) = fixture
        .entities
        .entity_mut(fixture.scientist_id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Wander")
    };
    assert_eq!(
        state.private_state().target_position_raw,
        [
            fixture.birth_position[0].wrapping_add(x),
            fixture.birth_position[1],
            fixture.birth_position[2].wrapping_add(z)
        ]
    );
    assert_eq!(state.elapsed_ms(), 20);
    let retry = tick_type8_go_to_job_scheduler_owner(
        &mut fixture.entities,
        blocked.retained_owner.unwrap(),
        &mut fx,
        fixture.session.cache.terrain(),
        400_000,
    );
    assert!(matches!(
        retry.outcome,
        Type8GoToJobSchedulerProductionOutcome::CallbackIncomplete { .. }
    ));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
