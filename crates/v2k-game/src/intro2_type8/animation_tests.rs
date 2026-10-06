//! Sub-I custody across the actual Intro2 and Playing compatibility-pass order.

use super::*;
use crate::{
    actor_animation::ActorAnimationController,
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
};

const ELAPSED_MICROS: u32 = 20_000;

#[derive(Clone, Copy, Debug)]
enum CompatibilityPassOrder {
    Intro2BeforeTasks,
    PlayingAfterTasks,
}

fn animation(manager: &EntityManager, id: u32) -> ActorAnimationController {
    let RetailRuntimeValue::Known(Some(animation)) = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .actor_animation_runtime
    else {
        panic!("native worker Sub-I");
    };
    animation
}

fn prepare_workers(manager: &mut EntityManager, detailed: bool) -> Vec<u32> {
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 8)
        .map(|entity| entity.id)
        .collect();
    for &id in &ids {
        let entity = manager.entity_mut(id).unwrap();
        // These are live view/scheduler inputs, not substituted birth data.
        // Coarse callbacks start beyond the wait threshold so this test checks
        // mode1's omitted Sub-I call independently of random waiting.
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                | if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
        );
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(if detailed { 0 } else { 125_001 });
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
    }
    ids
}

fn tick(
    scheduler: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut GameSession,
    fx: &mut WorldFx,
    notification_phase: GameplayNotificationPhase,
) -> Vec<SpecializedActorTaskProductionOutcome> {
    let pass = scheduler.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::default(),
            elapsed_micros: ELAPSED_MICROS,
            global_elapsed_micros: ELAPSED_MICROS,
            retail_tick: 1,
            main_base_abort_active: false,
        },
        &mut GameplayNotifications::new(),
    );
    assert!(pass.block.is_none(), "{:?}", pass.block);
    pass.outcomes
}

#[v2k_test_support::retail_test]
fn detailed_and_coarse_workers_keep_one_sub_i_owner_in_both_main_orders() {
    for order in [
        CompatibilityPassOrder::Intro2BeforeTasks,
        CompatibilityPassOrder::PlayingAfterTasks,
    ] {
        for detailed in [true, false] {
            let mut fx = WorldFx::new();
            let Some((mut session, mut manager)) = authored_tests::load(25, &mut fx) else {
                return;
            };
            let ids = prepare_workers(&mut manager, detailed);
            assert_eq!(ids.len(), 4);
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(scheduler.adopt_intro2_type8(&mut manager), ids.len());
            let claims: Vec<_> = scheduler.actor_animation_claims().collect();
            assert_eq!(claims.len(), ids.len());
            for &id in &ids {
                assert!(
                    claims.contains(&manager.main_base_abort_actor_observation(id).unwrap().lease)
                );
            }
            let before: Vec<_> = ids.iter().map(|&id| animation(&manager, id)).collect();
            assert!(before
                .iter()
                .all(|animation| animation.is_neutral_runtime()));
            if matches!(order, CompatibilityPassOrder::Intro2BeforeTasks) {
                manager.advance_unclaimed_actor_animations(ELAPSED_MICROS, &claims);
                for (&id, &expected) in ids.iter().zip(&before) {
                    assert_eq!(animation(&manager, id), expected, "{order:?} worker{id}");
                }
            }
            let notification_phase = match order {
                CompatibilityPassOrder::Intro2BeforeTasks => GameplayNotificationPhase::NonGameplay,
                CompatibilityPassOrder::PlayingAfterTasks => GameplayNotificationPhase::Playing,
            };
            let outcomes = tick(
                &mut scheduler,
                &mut manager,
                &mut session,
                &mut fx,
                notification_phase,
            );
            for &id in &ids {
                assert!(outcomes.iter().any(|outcome| matches!(outcome,
                    SpecializedActorTaskProductionOutcome::Intro2Type8(Intro2Type8Outcome::Advanced {
                        entity_id, detailed: actual_detail, terminal: false, ..
                    }) if *entity_id == id && *actual_detail == detailed)), "{order:?}: {outcomes:?}");
            }
            let after_tasks: Vec<_> = ids.iter().map(|&id| animation(&manager, id)).collect();
            if matches!(order, CompatibilityPassOrder::PlayingAfterTasks) {
                manager.advance_unclaimed_actor_animations(ELAPSED_MICROS, &claims);
                for (&id, &expected) in ids.iter().zip(&after_tasks) {
                    assert_eq!(animation(&manager, id), expected, "{order:?} worker{id}");
                }
            }
            if detailed {
                // 420520: the first 20-ms callback crosses the initial zero
                // timer once, reloads 80 ms, and discards the overshoot. A
                // second host callback would already leave only 60 ms.
                for (&id, state) in ids.iter().zip(&after_tasks) {
                    assert_eq!(
                        (state.phase(), state.countdown_millis()),
                        (1, 80),
                        "worker{id}"
                    );
                }
            } else {
                assert_eq!(after_tasks, before, "mode1 never dispatches Sub-I");
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn first_worker_adoption_after_the_entry_snapshot_claims_its_whole_frame() {
    for detailed in [true, false] {
        let mut fx = WorldFx::new();
        let Some((mut session, mut manager)) = authored_tests::load(25, &mut fx) else {
            return;
        };
        let ids = prepare_workers(&mut manager, detailed);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        // Playing snapshots before cargo and adopts newly authored/tail-born
        // workers later, immediately before the actor task pass.
        let mut claims: Vec<_> = scheduler.actor_animation_claims().collect();
        assert!(claims.is_empty());
        let before: Vec<_> = ids.iter().map(|&id| animation(&manager, id)).collect();
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), ids.len());
        for claim in scheduler.actor_animation_claims() {
            if !claims.contains(&claim) {
                claims.push(claim);
            }
        }
        let outcomes = tick(
            &mut scheduler,
            &mut manager,
            &mut session,
            &mut fx,
            GameplayNotificationPhase::Playing,
        );
        for &id in &ids {
            assert!(
                outcomes.iter().any(|outcome| matches!(outcome,
                SpecializedActorTaskProductionOutcome::Intro2Type8(Intro2Type8Outcome::Advanced {
                    entity_id, detailed: actual_detail, terminal: false, ..
                }) if *entity_id == id && *actual_detail == detailed)),
                "{outcomes:?}"
            );
        }
        let after_tasks: Vec<_> = ids.iter().map(|&id| animation(&manager, id)).collect();
        manager.advance_unclaimed_actor_animations(ELAPSED_MICROS, &claims);
        for (&id, &expected) in ids.iter().zip(&after_tasks) {
            assert_eq!(animation(&manager, id), expected, "new worker{id}");
        }
        if detailed {
            for state in after_tasks {
                assert_eq!((state.phase(), state.countdown_millis()), (1, 80));
            }
        } else {
            assert_eq!(after_tasks, before, "the first coarse visit skips Sub-I");
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum InterruptedVisit {
    BlockedMover,
    DroppedGraph,
}

#[v2k_test_support::retail_test]
fn entry_worker_claim_survives_a_blocked_or_dropped_task_visit() {
    for interruption in [
        InterruptedVisit::BlockedMover,
        InterruptedVisit::DroppedGraph,
    ] {
        let mut fx = WorldFx::new();
        let Some((mut session, mut manager)) = authored_tests::load(25, &mut fx) else {
            return;
        };
        let ids = prepare_workers(&mut manager, true);
        let id = ids[0];
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type8(&mut manager), ids.len());
        let claims: Vec<_> = scheduler.actor_animation_claims().collect();
        let lease = manager.main_base_abort_actor_observation(id).unwrap().lease;
        assert!(claims.contains(&lease));
        let before = animation(&manager, id);
        let entity = manager.entity_mut(id).unwrap();
        match interruption {
            InterruptedVisit::BlockedMover => {
                entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved
            }
            InterruptedVisit::DroppedGraph => {
                entity.current_behavior_context = RetailRuntimeValue::Known(None)
            }
        }
        let outcomes = tick(
            &mut scheduler,
            &mut manager,
            &mut session,
            &mut fx,
            GameplayNotificationPhase::Playing,
        );
        let outcome = outcomes
            .iter()
            .find(|outcome| outcome.entity_id() == id)
            .unwrap();
        match interruption {
            InterruptedVisit::BlockedMover => {
                assert!(
                    matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::Intro2Type8(
                            Intro2Type8Outcome::Blocked {
                                reason: Intro2Type8Block::Runtime("Sub-A"),
                                prefix_committed: true,
                                ..
                            }
                        )
                    ),
                    "{outcome:?}"
                );
                assert!(scheduler
                    .actor_animation_claims()
                    .any(|claim| claim == lease));
            }
            InterruptedVisit::DroppedGraph => {
                assert!(
                    matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::Intro2Type8(
                            Intro2Type8Outcome::Dropped { .. }
                        )
                    ),
                    "{outcome:?}"
                );
                assert!(!scheduler
                    .actor_animation_claims()
                    .any(|claim| claim == lease));
            }
        }
        manager.advance_unclaimed_actor_animations(ELAPSED_MICROS, &claims);
        assert_eq!(
            animation(&manager, id),
            before,
            "{interruption:?} retains the frame-entry claim"
        );
    }
}
