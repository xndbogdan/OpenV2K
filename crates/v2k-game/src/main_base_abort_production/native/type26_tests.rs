use super::*;
use crate::{
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    gameplay_notifications::GameplayNotifications,
    native_type122::construction_tests::native_fixture_with_player,
};

#[v2k_test_support::retail_test]
fn native_type26_abort_replaces_only_completed_live_graphs_and_preserves_corpse() {
    for inside_callback in [false, true] {
        let (_, mut manager, mut fx) = native_fixture_with_player(15);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 26)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_type26(&manager) > 0);
        let entity = manager.entity_mut(id).unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        if inside_callback {
            entity
                .actor_tasks
                .begin_exact_visit_with(
                    ActorTaskVisit {
                        slot: ActorTaskSlot::Primary,
                        task_id: primary,
                    },
                    |_| (),
                )
                .unwrap();
        }
        let before = entity.collision.clone();
        let observation = manager.main_base_abort_actor_observation(id).unwrap();
        let mut publications = MainBaseAbortPublicationCounts::default();
        let mut notifications = GameplayNotifications::new();
        let result = dispatch_native_actor(
            observation,
            &mut manager,
            &mut fx,
            &mut tasks,
            &mut publications,
            &mut MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut notifications,
                retail_tick: 0x1dcb,
            },
        )
        .unwrap();
        if inside_callback {
            assert!(result.is_err());
            assert_eq!(manager.entity_mut(id).unwrap().collision, before);
            assert_eq!(publications.type26_common_dying, 0);
            continue;
        }
        let result = result.unwrap_or_else(|failure| panic!("{:?}", failure.callback_error));
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type26Death
        );
        assert_eq!(publications.type26_common_dying, 1);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(primary)
        );
        let dead_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(crate::actor_task_dispatcher::ActorTaskRuntime::CommonDying(
                _
            ))
        ));
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let observation = manager.main_base_abort_actor_observation(id).unwrap();
        assert!(dispatch_native_actor(
            observation,
            &mut manager,
            &mut fx,
            &mut tasks,
            &mut publications,
            &mut MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut notifications,
                retail_tick: 0x1dcc
            },
        )
        .unwrap()
        .is_ok());
        assert_eq!(publications.type26_common_dying, 1);
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary),
            dead_primary
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}
