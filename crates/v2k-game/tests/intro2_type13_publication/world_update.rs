//! Complete Type-13 world visits around the already tested task traversal.

use super::*;
use v2k_game::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::Entity,
    entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    intro2_type13_live::{
        tick_intro2_type13_world_owner_with_random, Intro2Type13WorldBlock, Intro2Type13WorldDrop,
        Intro2Type13WorldOutcome, Intro2Type13WorldOwner,
    },
};

fn session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session.load_level_by_id(50, 1).expect("Intro2");
    session
}

fn frame(session: &GameSession, elapsed_micros: u32) -> Intro2Type13PrimaryFrame<'_> {
    Intro2Type13PrimaryFrame {
        dispatch_mode: CommonMoverDispatchMode::Normal,
        ..primary_frame(session, elapsed_micros)
    }
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .expect("Type-13 owner")
}

fn tasks(manager: &EntityManager, id: u32) -> [Option<ActorTaskRuntime>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| entity(manager, id).actor_task_state(slot).copied())
}

fn scheduler_words(entity: &Entity) -> [RetailRuntimeValue<u32>; 3] {
    [
        entity.collision.recent_relation_elapsed_us_at_0x68,
        entity.collision.callback_scheduler_accumulator_us_at_0x6c,
        entity.collision.subject_scan_gate_at_0x70,
    ]
}

fn continuing_fixture(session: &GameSession) -> (EntityManager, Intro2Type13WorldOwner) {
    let (_, mut manager, task_owner) = captured_intro2_type13(session);
    let entity = manager.entity_mut(task_owner.entity_id()).unwrap();
    // Synthetic post-activation fixture. The separate dormant test proves
    // the real scheduler clear; this is not a constructor-residue claim.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.collision.state_flags_at_0x08.overwrite(
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | 0x0006_8000,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | 0x0006_8000,
    );
    let owner = Intro2Type13WorldOwner::adopt(&manager).expect("captured world owner");
    (manager, owner)
}

fn environment_and_motion_oracle(
    position: [i16; 3],
    mut velocity: [i16; 3],
    elapsed_micros: u32,
    mass: u16,
) -> ([i16; 3], [i16; 3]) {
    // Independent fixed-point E100 -> EC60 -> 12DA0 expressions. Intro2
    // authors effective flags 8, no underwater response, zero wind, drag 3.
    velocity[1] =
        velocity[1].wrapping_sub(((i64::from(elapsed_micros) * 0x0030_0000) >> 31) as i16);
    let factor = ((u64::from(elapsed_micros) * 3) as u32 / (u32::from(mass) << 3)) as i32;
    for component in &mut velocity {
        *component =
            component.wrapping_sub((factor.wrapping_mul(i32::from(*component)) >> 15) as i16);
    }
    let position = std::array::from_fn(|axis| {
        let product = (elapsed_micros >> 5).wrapping_mul(i32::from(velocity[axis]) as u32) as i32;
        position[axis].wrapping_add((product >> 15) as i16)
    });
    (position, velocity)
}

fn pursuing_world_fixture(session: &GameSession) -> (EntityManager, Intro2Type13WorldOwner, u32) {
    let (mut manager, task_owner) = captured_intro2_type13_pursuing(session);
    let id = task_owner.entity_id();
    manager.set_authored_behavior_components_enabled(INTRO2_TYPE13_SPAWN_INDEX, true);
    let subject = manager.entity_mut(id).unwrap();
    subject.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    subject.collision.state_flags_at_0x08.overwrite(
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    let Some(ActorTaskRuntime::ChaseTarget(chase)) =
        subject.actor_task_state(ActorTaskSlot::Primary)
    else {
        panic!("acquisition publishes a Chase Primary");
    };
    let target_id = chase.target_id();
    let owner = Intro2Type13WorldOwner::adopt(&manager).unwrap();
    (manager, owner, target_id)
}

#[v2k_test_support::retail_test]
fn dormant_intro_visit_clears_residue_before_operation_two_enables_motion() {
    let session = session();
    let (_, mut manager, task_owner) = captured_intro2_type13(&session);
    let id = task_owner.entity_id();
    assert_eq!(
        entity(&manager, id).collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    // A later writer's contribution is cleared by this dormant scheduler
    // visit, independently of the one-time native birth policy.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
    manager.disable_authored_behavior_components();
    let before_tasks = tasks(&manager, id);
    let before = entity(&manager, id);
    let before_motion = (
        before.position_raw(),
        before.velocity_raw(),
        before.physical_body_basis_q31(),
    );
    assert_eq!(
        before.collision.state_flags_at_0x08.masked(0x0006_8000),
        RetailRuntimeValue::Known(0)
    );
    let owner = Intro2Type13WorldOwner::adopt(&manager).expect("dormant captured owner");
    let mut world_fx = WorldFx::new();
    let dormant = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| panic!("detailed dormant visit has no scheduler or callback RNG"),
    );
    assert!(matches!(
        dormant.outcome,
        Intro2Type13WorldOutcome::CallbackDisabled {
            callback_elapsed_micros: 20_000,
            ..
        }
    ));
    assert_eq!(tasks(&manager, id), before_tasks);
    let after = entity(&manager, id);
    assert_eq!(
        (
            after.position_raw(),
            after.velocity_raw(),
            after.physical_body_basis_q31()
        ),
        before_motion
    );
    assert_eq!(after.mass_raw, 100);
    assert_eq!(
        after.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        scheduler_words(after),
        [
            RetailRuntimeValue::Known(20_000),
            RetailRuntimeValue::Known(0),
            RetailRuntimeValue::Known(0)
        ]
    );

    manager.set_authored_behavior_components_enabled(INTRO2_TYPE13_SPAWN_INDEX, true);
    let active = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        dormant.retained_owner.unwrap(),
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(
        active.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: true,
            ..
        }
    ));
    assert_ne!(entity(&manager, id).position_raw(), before_motion.0);
    assert!(active.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn coarse_wait_retains_callback_state_then_consumes_the_accumulated_delta() {
    let session = session();
    let (mut manager, owner) = continuing_fixture(&session);
    let id = owner.entity_id();
    let subject = manager.entity_mut(id).unwrap();
    subject
        .collision
        .state_flags_at_0x08
        .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
    subject.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
    subject.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
    let before = (
        subject.position_raw(),
        subject.velocity_raw(),
        subject.physical_body_basis_q31(),
        subject.mass_raw,
    );
    let before_tasks = tasks(&manager, id);
    let mut words = [0xffff_u32, 0xffff].into_iter();
    let mut world_fx = WorldFx::new();
    let waiting = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| words.next().expect("subject gate then callback gate only"),
    );
    assert!(words.next().is_none());
    assert!(matches!(
        waiting.outcome,
        Intro2Type13WorldOutcome::SchedulerWaiting { .. }
    ));
    let after = entity(&manager, id);
    assert_eq!(
        (
            after.position_raw(),
            after.velocity_raw(),
            after.physical_body_basis_q31(),
            after.mass_raw
        ),
        before
    );
    assert_eq!(tasks(&manager, id), before_tasks);
    assert_eq!(
        scheduler_words(after),
        [
            RetailRuntimeValue::Known(7),
            RetailRuntimeValue::Known(20_000),
            RetailRuntimeValue::Known(20_000)
        ]
    );
    assert_eq!(
        after.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );

    let mut words = [0_u32, 0, 1, 1].into_iter();
    let continued = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        waiting.retained_owner.unwrap(),
        Some(frame(&session, 20_000)),
        &mut |_| words.next().expect("two gates then the retarget X/Z draws"),
    );
    assert!(words.next().is_none());
    assert!(matches!(
        continued.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: true,
            ..
        }
    ));
    assert!(
        matches!(entity(&manager, id).actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms() == 40)
    );
    assert_eq!(
        scheduler_words(entity(&manager, id)),
        [
            RetailRuntimeValue::Known(40_007),
            RetailRuntimeValue::Known(0),
            RetailRuntimeValue::Known(0)
        ]
    );
    assert_eq!(
        entity(&manager, id).mass_raw,
        100,
        "the wait cleared B2 before callback mass"
    );
}

#[v2k_test_support::retail_test]
fn detailed_callback_uses_capped_carry_and_the_unit_delta_override() {
    let session = session();
    for (unit_delta, expected_elapsed) in [(0, 125_000), (1, 1)] {
        let (mut manager, owner) = continuing_fixture(&session);
        let id = owner.entity_id();
        let subject = manager.entity_mut(id).unwrap();
        subject.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(7);
        subject.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(unit_delta);
        let mut words = [1_u32, 1].into_iter();
        let tick = tick_intro2_type13_world_owner_with_random(
            &mut manager,
            &mut WorldFx::new(),
            owner,
            Some(frame(&session, 200_001)),
            &mut |_| {
                words
                    .next()
                    .expect("detailed mode skips both scheduler RNG gates")
            },
        );
        assert!(words.next().is_none());
        assert!(matches!(
            tick.outcome,
            Intro2Type13WorldOutcome::Task {
                completed: true,
                ..
            }
        ));
        let after = entity(&manager, id);
        assert_eq!(
            scheduler_words(after),
            [
                RetailRuntimeValue::Known(expected_elapsed),
                RetailRuntimeValue::Known(75_008),
                RetailRuntimeValue::Known(0)
            ]
        );
        assert!(
            matches!(after.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms() == expected_elapsed / 1_000)
        );
    }
}

#[v2k_test_support::retail_test]
fn complete_world_visit_orders_task_gravity_drag_motion_and_retains_moved_identity() {
    let session = session();
    let (mut manager, owner) = continuing_fixture(&session);
    let (mut callback_only, _) = continuing_fixture(&session);
    let id = owner.entity_id();
    for manager in [&mut manager, &mut callback_only] {
        manager
            .entity_mut(id)
            .unwrap()
            .set_motion_raw(INTRO2_TYPE13_POSITION_RAW, [1200, -500, -900]);
    }
    let callback_owner =
        Intro2Type13SchedulerOwner::adopt_published(entity(&callback_only, id)).unwrap();
    let callback = tick_intro2_type13_scheduler_owner_with_random(
        &mut callback_only,
        callback_owner,
        &mut WorldFx::new(),
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(
        callback.outcome,
        Intro2Type13SchedulerProductionOutcome::B6c0Visit { .. }
    ));
    let control = entity(&callback_only, id);
    let expected =
        environment_and_motion_oracle(control.position_raw(), control.velocity_raw(), 20_000, 100);
    let mut world_fx = WorldFx::new();
    let tick = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: true,
            ..
        }
    ));
    let after = entity(&manager, id);
    assert_eq!((after.position_raw(), after.velocity_raw()), expected);
    assert_eq!(
        after.physical_body_basis_q31(),
        control.physical_body_basis_q31()
    );
    assert_ne!(after.position_raw(), INTRO2_TYPE13_POSITION_RAW);
    assert!(v2k_game::opening::intro2_uses_live_actor_pose(after));
    assert_eq!(
        after.position,
        [
            f32::from(expected.0[0] as u16) / 256.0,
            f32::from(expected.0[1]) / 256.0,
            f32::from(expected.0[2] as u16) / 256.0,
        ],
        "the renderer and camera consume the integrated position",
    );
    let mut unpublished = Entity::unresolved_port_entity(after.id, after.kind, after.entity_type);
    unpublished.active = after.active;
    unpublished.authored_spawn_index = after.authored_spawn_index;
    unpublished.model_slots = after.model_slots;
    unpublished.model_index = after.model_index;
    unpublished.position = after.position;
    assert!(
        !v2k_game::opening::intro2_uses_live_actor_pose(&unpublished),
        "a matching model and spawn alone do not prove captured publication",
    );
    let next = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        tick.retained_owner.unwrap(),
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(
        matches!(
            next.outcome,
            Intro2Type13WorldOutcome::Task {
                completed: true,
                ..
            }
        ),
        "moved owner must retain its allocation identity: {:?}",
        next.outcome
    );
    assert!(next.retained_owner.is_some());
}

#[v2k_test_support::retail_test]
fn pending_c690_finishes_environment_and_motion_once_without_replaying_the_prefix() {
    let session = session();
    let (metadata, mut manager, task_owner) = captured_intro2_type13(&session);
    let task_owner = prime_distant_shared_retarget(&session, &mut manager, task_owner);
    let id = task_owner.entity_id();
    tick_intro2_type13_primary(
        manager.entity_mut(id).unwrap(),
        metadata.get(TYPE13_ENTITY_TYPE as usize),
        frame(&session, 500_000),
        &mut || 1,
    )
    .expect("age the existing Primary to the strict timeout boundary");
    let subject = manager.entity_mut(id).unwrap();
    subject.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    subject.collision.state_flags_at_0x08.overwrite(
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | 0x0006_8000,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | 0x0006_8000,
    );
    let axis = subject.actor_common_axis_descriptor;
    subject.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
    let before_position = subject.position_raw();
    let owner = Intro2Type13WorldOwner::adopt(&manager).unwrap();
    let mut words = [1_u32, 0x4000].into_iter();
    let mut world_fx = WorldFx::new();
    let blocked = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| {
            words
                .next()
                .expect("old Primary gate then one frozen root selector")
        },
    );
    assert!(words.next().is_none());
    assert!(matches!(
        blocked.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: false,
            outcome: Intro2Type13SchedulerProductionOutcome::C690Blocked {
                reason: Intro2Type13C690Block::Publication(_),
                ..
            },
            ..
        }
    ));
    let after = entity(&manager, id);
    let retained_words = scheduler_words(after);
    assert_eq!(
        after.position_raw(),
        before_position,
        "incomplete traversal cannot integrate"
    );
    let expected =
        environment_and_motion_oracle(after.position_raw(), after.velocity_raw(), 20_000, 100);
    manager.entity_mut(id).unwrap().actor_common_axis_descriptor = axis;
    let mut words = [0x1234_u32, 0xabcd].into_iter();
    let resumed = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        blocked.retained_owner.unwrap(),
        None,
        &mut |_| {
            words
                .next()
                .expect("only the retained class-7 constructor suffix")
        },
    );
    assert!(words.next().is_none());
    assert!(matches!(
        resumed.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: true,
            outcome: Intro2Type13SchedulerProductionOutcome::C690Transition { primary: None, .. },
            ..
        }
    ));
    let after = entity(&manager, id);
    assert_eq!(scheduler_words(after), retained_words);
    assert_eq!((after.position_raw(), after.velocity_raw()), expected);
    let finished_tasks = tasks(&manager, id);
    let duplicate = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        resumed.retained_owner.unwrap(),
        None,
        &mut |_| panic!("a completed frame cannot be replayed"),
    );
    assert!(matches!(
        duplicate.outcome,
        Intro2Type13WorldOutcome::Blocked {
            reason: Intro2Type13WorldBlock::FrameUnavailable,
            ..
        }
    ));
    assert!(duplicate.retained_owner.is_some());
    assert_eq!(tasks(&manager, id), finished_tasks);
    assert_eq!(
        (
            entity(&manager, id).position_raw(),
            entity(&manager, id).velocity_raw()
        ),
        expected
    );
}

#[v2k_test_support::retail_test]
fn entered_callback_failure_keeps_world_time_and_motion_parked() {
    let session = session();
    let (mut manager, owner) = continuing_fixture(&session);
    let id = owner.entity_id();
    manager.entity_mut(id).unwrap().capability_flags |= 1;
    let mut world_fx = WorldFx::new();
    let failed = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(
        failed.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: false,
            outcome: Intro2Type13SchedulerProductionOutcome::PrimaryBlocked { .. },
            ..
        }
    ));
    let before = entity(&manager, id);
    let before_words = scheduler_words(before);
    let before_motion = (
        before.position_raw(),
        before.velocity_raw(),
        before.physical_body_basis_q31(),
    );
    let before_tasks = tasks(&manager, id);
    let mut owner = failed.retained_owner.unwrap();
    for retry_frame in [Some(frame(&session, 6_000_000)), None] {
        let retry = tick_intro2_type13_world_owner_with_random(
            &mut manager,
            &mut world_fx,
            owner,
            retry_frame,
            &mut |_| panic!("parked callback must not repeat scheduler or task RNG"),
        );
        assert!(matches!(
            retry.outcome,
            Intro2Type13WorldOutcome::IncompletePending { .. }
                | Intro2Type13WorldOutcome::Task {
                    completed: false,
                    outcome: Intro2Type13SchedulerProductionOutcome::CallbackFailurePending { .. },
                    ..
                }
        ));
        let after = entity(&manager, id);
        assert_eq!(scheduler_words(after), before_words);
        assert_eq!(
            (
                after.position_raw(),
                after.velocity_raw(),
                after.physical_body_basis_q31()
            ),
            before_motion
        );
        assert_eq!(tasks(&manager, id), before_tasks);
        owner = retry.retained_owner.unwrap();
    }
    manager.entity_mut(id).unwrap().capability_flags &= !1;
    let changed = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| panic!("changed callback state cannot re-enter the suspended traversal"),
    );
    assert!(matches!(
        changed.outcome,
        Intro2Type13WorldOutcome::Dropped {
            reason: Intro2Type13WorldDrop::PendingObservationChanged,
            ..
        }
    ));
    assert!(changed.retained_owner.is_none());
}

#[v2k_test_support::retail_test]
fn pre_entry_chase_retry_keeps_the_original_world_delta() {
    let session = session();
    let (mut manager, owner, target_id) = pursuing_world_fixture(&session);
    let id = owner.entity_id();
    let target = manager.entity_mut(target_id).unwrap();
    let target_state = target.collision.state_flags_at_0x08;
    target.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(0, DYING_STATE_BIT);
    let before_tasks = tasks(&manager, id);
    let mut world_fx = WorldFx::new();
    let blocked = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| panic!("target-state preflight precedes Chase RNG"),
    );
    assert!(matches!(
        blocked.outcome,
        Intro2Type13WorldOutcome::Task {
            completed: false,
            outcome: Intro2Type13SchedulerProductionOutcome::PursuingChaseBlocked {
                reason: v2k_game::intro2_type13_live::Intro2Type13ChaseBlock::TargetStateUnresolved,
                ..
            },
            ..
        }
    ));
    assert_eq!(tasks(&manager, id), before_tasks);
    let retained_words = scheduler_words(entity(&manager, id));
    assert_eq!(retained_words[0], RetailRuntimeValue::Known(20_000));
    let no_frame = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        blocked.retained_owner.unwrap(),
        None,
        &mut |_| panic!("a missing retry frame cannot repeat the scheduler prefix"),
    );
    assert!(matches!(
        no_frame.outcome,
        Intro2Type13WorldOutcome::Blocked {
            reason: Intro2Type13WorldBlock::FrameUnavailable,
            ..
        }
    ));
    assert_eq!(tasks(&manager, id), before_tasks);
    assert_eq!(scheduler_words(entity(&manager, id)), retained_words);
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = target_state;
    let resumed = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        no_frame.retained_owner.unwrap(),
        Some(frame(&session, 600_000)),
        &mut |_| 1,
    );
    assert!(
        matches!(
            resumed.outcome,
            Intro2Type13WorldOutcome::Task {
                completed: true,
                outcome: Intro2Type13SchedulerProductionOutcome::PursuingVisit { .. },
                ..
            }
        ),
        "repaired target must finish the suspended visit: {:?}",
        resumed.outcome
    );
    let Some(ActorTaskRuntime::ChaseTarget(before_chase)) = before_tasks[0] else {
        panic!("pursuing fixture has a Chase Primary");
    };
    assert!(matches!(
        entity(&manager, id).actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(chase))
            if chase.elapsed_ms() == before_chase.elapsed_ms() + 20
    ));
    assert_eq!(scheduler_words(entity(&manager, id)), retained_words);
}

#[v2k_test_support::retail_test]
fn nonpeasant_target_finishes_chase_aim_and_world_tail() {
    let session = session();
    let (mut manager, owner, target_id) = pursuing_world_fixture(&session);
    let id = owner.entity_id();
    // Isolate the former Type-9-only check without changing the target's
    // already accepted pose/state. The canonical gun acquisition is covered
    // separately in aim_targets; neither path needs the gun's task owner.
    manager.entity_mut(target_id).unwrap().entity_type = 102;
    let before_tasks = tasks(&manager, id);
    let mut world_fx = WorldFx::new();
    let completed = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(
        matches!(
            completed.outcome,
            Intro2Type13WorldOutcome::Task {
                completed: true,
                outcome: Intro2Type13SchedulerProductionOutcome::PursuingVisit {
                    chase: Some(_),
                    aim: Some(Ok(_)),
                    ..
                },
                ..
            }
        ),
        "the shared target state is valid for both Chase and Aim: {:?}",
        completed.outcome
    );
    let Some(ActorTaskRuntime::ChaseTarget(before_chase)) = before_tasks[0] else {
        panic!("fixture has a Chase Primary");
    };
    let Some(ActorTaskRuntime::AimAndFire(before_aim)) = before_tasks[2] else {
        panic!("fixture has an Aim Tertiary");
    };
    assert!(
        matches!(tasks(&manager, id)[0], Some(ActorTaskRuntime::ChaseTarget(task))
        if task.elapsed_ms() == before_chase.elapsed_ms() + 20)
    );
    assert!(
        matches!(tasks(&manager, id)[2], Some(ActorTaskRuntime::AimAndFire(task))
        if task.elapsed_ms() == before_aim.elapsed_ms() + 20)
    );
    let next = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        completed.retained_owner.unwrap(),
        Some(frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(next.outcome, Intro2Type13WorldOutcome::Task { completed: true, .. }),
        "the next native visit must advance rather than retain an artificial incomplete prefix: {:?}", next.outcome);
}

#[v2k_test_support::retail_test]
fn detail_publication_uses_moved_live_position_and_controls_the_next_world_wait() {
    use v2k_game::entity_view_detail::{
        RetailViewDetail, RetailViewDetailContext, VIEW_DETAIL_STATE_MASK,
    };
    use v2k_game::intro2_type13_live::publish_intro2_type13_view_detail;

    let session = session();
    let (mut manager, owner) = continuing_fixture(&session);
    let id = owner.entity_id();
    let source = manager.entity_mut(id).unwrap();
    let birth_position = source.position_raw();
    source.velocity = [16_000i16, 0, 0].map(|word| f32::from(word) / 256.0);
    // Isolate the common world phase: callback disabled, master motion live.
    source
        .collision
        .state_flags_at_0x08
        .overwrite(0x0006_0000, 0x0004_0000);
    let mut world_fx = WorldFx::new();
    let moved = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        owner,
        Some(frame(&session, 125_000)),
        &mut |_| panic!("the initial detailed dormant pass has no RNG"),
    );
    assert!(matches!(
        moved.outcome,
        Intro2Type13WorldOutcome::CallbackDisabled { .. }
    ));
    let position = entity(&manager, id).position_raw();
    assert_ne!(position, birth_position);
    let full_context = RetailViewDetailContext::from_raw(
        [
            i32::from(position[0]) + 26 * 256,
            i32::from(position[1]),
            i32::from(position[2]),
        ],
        0,
        (52, 30),
    );
    assert_eq!(
        full_context.classify(birth_position),
        RetailViewDetail::Coarse
    );
    assert_eq!(full_context.classify(position), RetailViewDetail::Full);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(VIEW_DETAIL_STATE_MASK, 0);
    let mut expected_state = entity(&manager, id).collision.state_flags_at_0x08;
    expected_state.overwrite(VIEW_DETAIL_STATE_MASK, VIEW_DETAIL_STATE_MASK);
    assert_eq!(
        publish_intro2_type13_view_detail(&mut manager, full_context),
        Ok(RetailRuntimeValue::Known(Some(RetailViewDetail::Full))),
    );
    assert_eq!(
        entity(&manager, id).collision.state_flags_at_0x08,
        expected_state
    );
    let detailed = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        moved.retained_owner.unwrap(),
        Some(frame(&session, 20_000)),
        &mut |_| panic!("published full detail suppresses both scheduler waits"),
    );
    assert!(matches!(
        detailed.outcome,
        Intro2Type13WorldOutcome::CallbackDisabled { .. }
    ));
    let position = entity(&manager, id).position_raw();
    let coarse_context = RetailViewDetailContext::from_raw(
        [
            i32::from(position[0]) + 0x5000,
            i32::from(position[1]),
            i32::from(position[2]),
        ],
        0,
        (52, 30),
    );
    expected_state = entity(&manager, id).collision.state_flags_at_0x08;
    expected_state.overwrite(VIEW_DETAIL_STATE_MASK, 0);
    assert_eq!(
        publish_intro2_type13_view_detail(&mut manager, coarse_context),
        Ok(RetailRuntimeValue::Known(Some(RetailViewDetail::Coarse))),
    );
    assert_eq!(
        entity(&manager, id).collision.state_flags_at_0x08,
        expected_state
    );
    let mut words = [0xffff_u32, 0xffff].into_iter();
    let waiting = tick_intro2_type13_world_owner_with_random(
        &mut manager,
        &mut world_fx,
        detailed.retained_owner.unwrap(),
        Some(frame(&session, 20_000)),
        &mut |_| {
            words
                .next()
                .expect("only subject and callback scheduler gates")
        },
    );
    assert!(words.next().is_none());
    assert!(matches!(
        waiting.outcome,
        Intro2Type13WorldOutcome::SchedulerWaiting { .. }
    ));
    assert_eq!(entity(&manager, id).position_raw(), position);
}
