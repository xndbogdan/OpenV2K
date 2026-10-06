//! Moving class2 fish retain synchronous abort custody after leaving their anchor.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot, gameplay_notifications::GameplayNotifications,
    native_type122::construction_tests::native_fixture_with_player,
};

#[derive(Clone, Copy)]
enum InvalidCustody {
    ForeignAllocation,
    PendingImpact,
    MissingScheduler,
}

#[v2k_test_support::retail_test]
fn type62_abort_uses_moving_fish_custody_and_retires_only_completed_owners() {
    assert_fish_abort_contract(13, 62);
}

#[v2k_test_support::retail_test]
fn shared_fish_abort_admits_types22_23_24_with_the_same_complete_quiet_terminal() {
    for kind in [22, 23, 24] {
        assert_fish_abort_contract(30, kind);
    }
}

fn assert_fish_abort_contract(world: u32, kind: u32) {
    for invalid in [
        None,
        Some(InvalidCustody::ForeignAllocation),
        Some(InvalidCustody::PendingImpact),
        Some(InvalidCustody::MissingScheduler),
    ] {
        let (_, mut manager, mut fx) = native_fixture_with_player(world);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        let [x, y, z] = entity.position_raw();
        entity.set_motion_raw([x.wrapping_add(7), y, z.wrapping_sub(5)], [12, -2, 6]);
        let moved_position = entity.position_raw();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_shared_fish(&manager) > 0);
        match invalid {
            Some(InvalidCustody::ForeignAllocation) => {
                let (_, foreign, _) = native_fixture_with_player(world);
                manager.entity_mut(id).unwrap().shared_fish_runtime = foreign
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .unwrap()
                    .shared_fish_runtime
                    .clone();
            }
            Some(InvalidCustody::PendingImpact) => {
                manager
                    .entity_mut(id)
                    .unwrap()
                    .shared_fish_runtime
                    .as_mut()
                    .unwrap()
                    .impact_prefix_pending = true;
            }
            Some(InvalidCustody::MissingScheduler) => scheduler.retire_shared_fish(id),
            None => {}
        }
        let entity = manager.entity_mut(id).unwrap();
        let before = (
            entity.collision.clone(),
            entity.current_behavior_context,
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        );
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let mut publications = MainBaseAbortPublicationCounts::default();
        let mut notifications = GameplayNotifications::new();
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
        .expect("retained class2 fish selects the native route");
        if invalid.is_some() {
            assert!(result.is_err());
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(
                (
                    entity.collision.clone(),
                    entity.current_behavior_context,
                    entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                ),
                before
            );
            assert!(!manager.pending_actor_deferred_destroy_ids().contains(&id));
        } else {
            let success = result.unwrap_or_else(|failure| panic!("{:?}", failure.callback_error));
            assert_eq!(
                success.disposition,
                MainBaseAbortActorDisposition::QuietDeath
            );
            assert!(crate::shared_fish::death::completed_shared_fish_death(
                &manager, id
            ));
            assert_eq!(scheduler.family_for(id), None);
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.position_raw(), moved_position);
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
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
                    retail_tick: 5001,
                },
            )
            .unwrap()
            .is_ok());
            assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), vec![id]);
            assert!(manager.iter_all().all(|entity| entity.id != id));
        }
        assert_eq!(publications, MainBaseAbortPublicationCounts::default());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}
