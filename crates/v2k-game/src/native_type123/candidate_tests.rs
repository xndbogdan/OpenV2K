//! AF50 retires the executing candidate without discarding the accepted graph.
use super::*;
use crate::native_type122::construction_tests::native_fixture_with_player;

#[v2k_test_support::retail_test]
fn type123_candidate_self_retirement_readopts_target_graph_without_ticking_new_primary() {
    let (session, mut manager, mut fx) = native_fixture_with_player(49);
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager
        .iter_all()
        .find(|entity| {
            entity.entity_type == 123
                && matches!(
                    entity.actor_task_state(ActorTaskSlot::Secondary),
                    Some(ActorTaskRuntime::AttractAttentionCandidate(_))
                )
        })
        .expect("actual world49 odd-parity BA40 constructor")
        .id;
    let player = manager.player().unwrap().id;
    let entity = manager.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let receipt = entity.native_type123_runtime;
    let old_primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let old_secondary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .unwrap();
    let mut owner = Type123Owner::take_birth(entity).unwrap();
    let metadata = manager.type_runtime_metadata(123).unwrap().clone();
    manager.entity_mut(player).unwrap().set_position_raw([
        position[0].wrapping_add(800),
        position[1],
        position[2],
    ]);
    assert_eq!(owner.kind, TaskKind::AttractAcquiring);
    for _ in 0..64 {
        assert!(!task::run_task(
            &mut manager,
            &mut owner,
            &task::TaskFrame {
                metadata: &metadata,
                terrain: session.cache.terrain().unwrap(),
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                scheduler_mode: 0,
            },
            &mut fx
        )
        .unwrap());
        let actor = manager.entity_mut(id).unwrap();
        if matches!(
            actor.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(_))
        ) {
            // This is the regression boundary: AF50 published a new task but
            // the old implementation returned Continue and kept the old owner.
            assert_eq!(owner.kind, TaskKind::AttractTarget);
            let primary = actor
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            assert_ne!(primary, old_primary);
            assert_eq!(owner.task_id, primary);
            assert!(actor.actor_tasks.wrapper_flags(old_secondary).is_none());
            assert!(actor.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(actor.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                actor.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!()
            };
            assert_eq!(route.target_id(), Some(player));
            assert_eq!(
                route.elapsed_ms(),
                0,
                "new Primary must wait for its next actor visit"
            );
            assert_eq!(
                actor.native_type123_runtime,
                receipt.map(|mut runtime| {
                    runtime.birth_pending = false;
                    runtime
                })
            );
            assert!(owner.completed_hit_boundary(&manager));
            let next = tick_type123(
                &mut manager,
                owner,
                Type123Frame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: 1,
                },
            );
            assert!(
                matches!(
                    next.outcome,
                    Type123Outcome::Advanced { .. } | Type123Outcome::Waiting { .. }
                ),
                "{:?}",
                next.outcome
            );
            assert!(next
                .retained_owner
                .unwrap()
                .completed_hit_boundary(&manager));
            return;
        }
    }
    panic!("bounded real RNG visits never accepted the nearby player");
}
