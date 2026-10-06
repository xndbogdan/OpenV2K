//! Post-entry callback failures preserve the retail prefix and stop the owner.

use super::*;

fn session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail-data session");
    session.load_auxiliary_ovl(3, 1).expect("high-tier system");
    session.load_level_by_id(50, 1).expect("Intro2");
    session
}

fn entity(manager: &EntityManager, id: u32) -> &v2k_game::entity::Entity {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .expect("owner")
}

fn tasks(manager: &EntityManager, id: u32) -> [Option<ActorTaskRuntime>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| entity(manager, id).actor_task_state(slot).copied())
}

fn assert_parked_without_replay(
    session: &GameSession,
    manager: &mut EntityManager,
    mut owner: Intro2Type13SchedulerOwner,
    world_fx: &mut WorldFx,
) -> Intro2Type13SchedulerOwner {
    let id = owner.entity_id();
    // Repairing the rejected capability must not restart an entered callback.
    manager.entity_mut(id).unwrap().capability_flags &= !1;
    let before = entity(manager, id);
    let tasks_before = tasks(manager, id);
    let context_before = before.current_behavior_context;
    let basis_before = before.physical_body_basis_q31();
    let motion_before = (
        before.position_raw(),
        before.velocity_raw(),
        before.rotation_heading_pitch_roll_raw(),
    );
    let common_before = before.intro2_type13_common_mover_runtime;
    let sub_g_before = before.sub_g_06070_runtime;
    let queued_before = before
        .intro2_type13_aim_runtime
        .as_ref()
        .map(|runtime| runtime.queued_shot_count());
    let mut source_id = None;
    for frame in [Some(primary_frame(session, 6_000_000)), None] {
        let tick = tick_intro2_type13_scheduler_owner_with_random(
            manager,
            owner,
            world_fx,
            frame,
            &mut |_| panic!("parked callback must not redraw RNG"),
        );
        match tick.outcome {
            Intro2Type13SchedulerProductionOutcome::CallbackFailurePending {
                source,
                reason,
                ..
            } => {
                assert_eq!(source.slot, ActorTaskSlot::Primary);
                if let Some(previous) = source_id {
                    assert_eq!(source.task_id, previous);
                }
                source_id = Some(source.task_id);
                assert_eq!(
                    reason,
                    GklCommonMoverBlock::SubG(Type13SubGFrameBlock::PlayerProjectionUnsupported)
                );
            }
            other => panic!("expected durable callback failure, got {other:?}"),
        }
        owner = tick.retained_owner.expect("retain failed owner custody");
        let after = entity(manager, id);
        assert_eq!(tasks(manager, id), tasks_before);
        assert_eq!(after.current_behavior_context, context_before);
        assert_eq!(
            after.physical_body_basis_q31(),
            basis_before,
            "a parked callback must not reach the post-task basis rebuild"
        );
        assert_eq!(
            (
                after.position_raw(),
                after.velocity_raw(),
                after.rotation_heading_pitch_roll_raw()
            ),
            motion_before
        );
        assert_eq!(after.intro2_type13_common_mover_runtime, common_before);
        assert_eq!(after.sub_g_06070_runtime, sub_g_before);
        assert_eq!(
            after
                .intro2_type13_aim_runtime
                .as_ref()
                .map(|runtime| runtime.queued_shot_count()),
            queued_before
        );
        assert!(world_fx.take_positional_sounds().is_empty());
    }
    owner
}

#[v2k_test_support::retail_test]
fn shared_retarget_failure_commits_each_rng_prefix_once_for_both_roots() {
    let session = session();
    for class5 in [false, true] {
        // Near: X/Z only. Far miss: gate only. Far hit: gate then X/Z.
        for (near, words) in [
            (true, vec![0x1000, 0x2000]),
            (false, vec![1]),
            (false, vec![0, 0x1000, 0x2000]),
        ] {
            let (_, mut manager, mut owner) = captured_intro2_type13(&session);
            if class5 {
                owner = reselect_class5_from_b6c0(&session, &mut manager, owner);
            }
            if !near {
                owner = prime_distant_shared_retarget(&session, &mut manager, owner);
            }
            let id = owner.entity_id();
            manager.entity_mut(id).unwrap().capability_flags |= 1;
            let before = entity(&manager, id);
            let tasks_before = tasks(&manager, id);
            let common_before = before.intro2_type13_common_mover_runtime;
            let sub_g_before = before.sub_g_06070_runtime;
            let rotation_before = before.rotation_heading_pitch_roll_raw();
            let velocity_before = before.velocity_raw();
            let Some(ActorTaskRuntime::SharedRetarget(task_before)) = tasks_before[0] else {
                panic!("SharedRetarget")
            };
            let mut expected_task = task_before;
            expected_task.before_callback(20_000);
            let mut expected_words = words.iter().copied();
            expected_task.stage_callback(before.position_raw(), || {
                expected_words.next().expect("retarget oracle word") as u16
            });
            assert!(expected_words.next().is_none());
            let mut actual_words = words.into_iter();
            let mut world_fx = WorldFx::new();
            let tick = tick_intro2_type13_scheduler_owner_with_random(
                &mut manager,
                owner,
                &mut world_fx,
                Some(primary_frame(&session, 20_000)),
                &mut |_| actual_words.next().expect("no draw after retarget prefix"),
            );
            assert!(actual_words.next().is_none());
            let Intro2Type13SchedulerProductionOutcome::PrimaryBlocked { primary, .. } =
                tick.outcome
            else {
                panic!("expected blocked Primary")
            };
            assert_eq!(
                primary.post_unwind,
                SharedRetargetPostUnwind::UnresolvedCommonMover
            );
            assert_eq!(primary.sound, None);
            let after = entity(&manager, id);
            assert_eq!(
                after.actor_task_state(ActorTaskSlot::Primary),
                Some(&ActorTaskRuntime::SharedRetarget(expected_task))
            );
            assert_eq!(&tasks(&manager, id)[1..], &tasks_before[1..]);
            assert_eq!(after.intro2_type13_common_mover_runtime, common_before);
            assert_eq!(after.sub_g_06070_runtime, sub_g_before);
            assert_eq!(after.rotation_heading_pitch_roll_raw(), rotation_before);
            assert_eq!(after.velocity_raw(), velocity_before);
            assert!(world_fx.take_positional_sounds().is_empty());
            assert_parked_without_replay(
                &session,
                &mut manager,
                tick.retained_owner.unwrap(),
                &mut world_fx,
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn chase_failure_ages_once_and_does_not_revisit_aim() {
    let session = session();
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let id = owner.entity_id();
    manager.entity_mut(id).unwrap().capability_flags |= 1;
    let before = tasks(&manager, id);
    let Some(ActorTaskRuntime::ChaseTarget(chase_before)) = before[0] else {
        panic!("Chase")
    };
    let mut expected_chase = chase_before;
    expected_chase.before_callback(20_000);
    let mut world_fx = WorldFx::new();
    let tick = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut world_fx,
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("authored Chase mover consumes no RNG"),
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            chase: Some(v2k_game::intro2_type13_live::Intro2Type13ChaseVisit {
                result: Intro2Type13ChaseVisitResult::CommonMoverBlocked(_),
                ..
            }),
            aim: None,
            acquisition: None,
            post_chase_plus00_c690: None,
            ..
        }
    ));
    assert_eq!(
        tasks(&manager, id)[0],
        Some(ActorTaskRuntime::ChaseTarget(expected_chase))
    );
    assert_eq!(&tasks(&manager, id)[1..], &before[1..]);
    assert!(world_fx.take_positional_sounds().is_empty());
    assert_parked_without_replay(
        &session,
        &mut manager,
        tick.retained_owner.unwrap(),
        &mut world_fx,
    );
}

#[v2k_test_support::retail_test]
fn parked_failure_rejects_stale_primary_context_or_identity() {
    let session = session();
    for stale in 0..4 {
        let (metadata, mut manager, owner) = captured_intro2_type13(&session);
        let id = owner.entity_id();
        manager.entity_mut(id).unwrap().capability_flags |= 1;
        let mut world_fx = WorldFx::new();
        let tick = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut world_fx,
            Some(primary_frame(&session, 20_000)),
            &mut |_| 0,
        );
        let parked = tick.retained_owner.expect("parked callback");
        let entity = manager.entity_mut(id).unwrap();
        match stale {
            0 => {
                entity.current_behavior_context = RetailRuntimeValue::Unresolved;
                v2k_game::intro2_type13_live::publish_intro2_type13(
                    entity,
                    &metadata[TYPE13_ENTITY_TYPE as usize],
                    v2k_game::intro2_type13_live::Intro2Type13BirthSelection::CapturedClass7,
                    &mut || 0,
                )
                .expect("replacement task graph");
            }
            1 => {
                entity.current_behavior_context = RetailRuntimeValue::Unresolved;
            }
            2 => {
                tick_intro2_type13_primary(
                    entity,
                    metadata.get(TYPE13_ENTITY_TYPE as usize),
                    primary_frame(&session, 1_000),
                    &mut || 0,
                )
                .expect("out-of-band prefix changes committed Primary");
            }
            3 => {
                entity.authored_spawn_index = None;
            }
            _ => unreachable!(),
        }
        let before = tasks(&manager, id);
        let tick = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            parked,
            &mut world_fx,
            None,
            &mut |_| panic!("stale receipt cannot consume RNG"),
        );
        assert!(matches!(
            tick.outcome,
            Intro2Type13SchedulerProductionOutcome::Dropped {
                reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                ..
            }
        ));
        assert!(tick.retained_owner.is_none());
        assert_eq!(tasks(&manager, id), before);
        assert!(world_fx.take_positional_sounds().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn unresolved_target_before_chase_entry_can_be_retried() {
    let session = session();
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let id = owner.entity_id();
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("Chase")
    };
    let target_id = chase.target_id();
    let target_state = entity(&manager, target_id).collision.state_flags_at_0x08;
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::from_known_bits(0, DYING_STATE_BIT);
    let before = tasks(&manager, id);
    let tick = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| panic!("pre-entry failure consumes no RNG"),
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingChaseBlocked { .. }
    ));
    assert_eq!(tasks(&manager, id), before);
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = target_state;
    let resumed = tick_intro2_type13_scheduler_owner(
        &mut manager,
        tick.retained_owner.unwrap(),
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
    );
    assert!(matches!(
        resumed.outcome,
        Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            chase: Some(_),
            aim: Some(_),
            ..
        }
    ));
    assert!(resumed.retained_owner.is_some());
}
