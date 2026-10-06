use super::*;
use crate::native_type122::contact_tests::Fixture;

#[v2k_test_support::retail_test]
fn native_abort_type122_uses_its_own_receipt_and_class12_without_reentering_dying_tasks() {
    for level in [24, 42, 46, 49, 50] {
        let mut f = Fixture::new(level);
        let actor = f.entities.main_base_abort_actor_observation(f.id).unwrap();
        let before = f.entity().native_type122_runtime;
        let mut publications = MainBaseAbortPublicationCounts::default();
        let result = dispatch_native_actor(
            actor,
            &mut f.entities,
            &mut f.fx,
            &mut f.tasks,
            &mut publications,
            &mut MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut f.notifications,
                retail_tick: 400,
            },
        )
        .unwrap();
        let success = match result {
            Ok(success) => success,
            Err(failure) => panic!("{:?}", failure.callback_error),
        };
        assert_eq!(
            success.disposition,
            MainBaseAbortActorDisposition::Type122Death
        );
        assert_eq!(publications.type122_common_dying, 1);
        assert_eq!(f.entity().native_type122_runtime, before);
        let task = f
            .entity()
            .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
            .copied();
        let mut rng = f.fx.fork_for_main_base_abort_transaction();
        let actor = f.entities.main_base_abort_actor_observation(f.id).unwrap();
        let result = dispatch_native_actor(
            actor,
            &mut f.entities,
            &mut f.fx,
            &mut f.tasks,
            &mut publications,
            &mut MainBaseAbortGameplayContext {
                extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut f.notifications,
                retail_tick: 500,
            },
        )
        .unwrap();
        assert!(result.is_ok());
        assert_eq!(publications.type122_common_dying, 1);
        assert_eq!(
            f.entity()
                .actor_task_state(crate::actor_task_owner::ActorTaskSlot::Primary)
                .copied(),
            task
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            rng.next_shared_retail_random_u16()
        );
    }
}
