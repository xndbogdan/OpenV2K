//! Abort uses the diver's existing allocation and Class14, without a Base-input claim.
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    native_type122::construction_tests::native_fixture_with_player,
};

#[v2k_test_support::retail_test]
fn type7_abort_replaces_each_real_owner_once_and_rejects_foreign_or_parked_receipts() {
    for invalid in [None, Some(false), Some(true)] {
        let (_session, mut manager, mut fx) = native_fixture_with_player(34);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == 7)
            .map(|e| e.id)
            .collect();
        assert_eq!(ids.len(), 5);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_native_type86(&mut manager), 5);
        let mut publications = MainBaseAbortPublicationCounts::default();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        for id in ids {
            if let Some(foreign) = invalid {
                if foreign {
                    let (_, other, _) = native_fixture_with_player(34);
                    manager.entity_mut(id).unwrap().native_type86_runtime = other
                        .iter_all()
                        .find(|e| e.id == id)
                        .unwrap()
                        .native_type86_runtime;
                } else {
                    scheduler.park_native_type86_external_prefix(id);
                }
            }
            let actor = manager.entity_mut(id).unwrap();
            let before = (
                actor.collision.clone(),
                actor.current_behavior_context,
                actor.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            );
            let body = (
                actor.native_type86_runtime,
                actor.position_raw(),
                actor.physical_body_basis_q31(),
                actor.model_slots,
                actor.construction_stamp_at_0xb4,
            );
            let ordinal = manager.next_common_body_ordinal();
            let seed = fx.next_sub_d_allocation_seed();
            let mut oracle = fx.fork_for_main_base_abort_transaction();
            if invalid.is_none() {
                // C3A0's one shared06070 draw belongs to this death publication.
                oracle.next_shared_retail_random_u16();
            }
            let observation = manager.main_base_abort_actor_observation(id).unwrap();
            let result = dispatch_native_actor(
                observation,
                &mut manager,
                &mut fx,
                &mut scheduler,
                &mut publications,
                &mut MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut notifications,
                    retail_tick: 5000,
                },
            )
            .unwrap();
            if invalid.is_some() {
                assert!(result.is_err());
                let actor = manager.entity_mut(id).unwrap();
                assert_eq!(
                    (
                        actor.collision.clone(),
                        actor.current_behavior_context,
                        actor.actor_tasks.task_in_slot(ActorTaskSlot::Primary)
                    ),
                    before
                );
                assert_eq!(publications, MainBaseAbortPublicationCounts::default());
            } else {
                let success =
                    result.unwrap_or_else(|failure| panic!("{:?}", failure.callback_error));
                assert_eq!(
                    success.disposition,
                    MainBaseAbortActorDisposition::Type86Death
                );
                let actor = manager.entity_mut(id).unwrap();
                assert_eq!(actor.collision.health_raw, RetailRuntimeValue::Known(0));
                assert!(
                    matches!(actor.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 1000)
                );
                assert_eq!(
                    (
                        actor.native_type86_runtime,
                        actor.position_raw(),
                        actor.physical_body_basis_q31(),
                        actor.model_slots,
                        actor.construction_stamp_at_0xb4
                    ),
                    body
                );
                let once = publications.clone();
                let observation = manager.main_base_abort_actor_observation(id).unwrap();
                assert!(dispatch_native_actor(
                    observation,
                    &mut manager,
                    &mut fx,
                    &mut scheduler,
                    &mut publications,
                    &mut MainBaseAbortGameplayContext {
                        extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                        notifications: &mut notifications,
                        retail_tick: 5001
                    }
                )
                .unwrap()
                .is_ok());
                assert_eq!(publications, once);
            }
            assert_eq!(manager.next_common_body_ordinal(), ordinal);
            assert_eq!(fx.next_sub_d_allocation_seed(), seed);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
        }
        assert_eq!(
            publications.type86_exploding,
            if invalid.is_none() { 5 } else { 0 }
        );
    }
}
