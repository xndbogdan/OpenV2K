use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskScheduler,
    },
};

/// 170A0's 10C10 takes either row to class12, with no second publication
/// when the abort reaches the same corpse again.
#[v2k_test_support::retail_test]
fn type76_family_abort_takes_class12_once() {
    for level in [26, 31, 37, 39, 40] {
        let (_session, mut entities, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        let id = entities
            .iter_all()
            .find(|entity| {
                crate::native_type76::Type76Row::from_entity_type(entity.entity_type).is_some()
            })
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_type76_family(&entities) > 0);
        let mut notifications = GameplayNotifications::new();
        let mut publications = MainBaseAbortPublicationCounts::default();
        for pass in 0..2 {
            let actor = entities.main_base_abort_actor_observation(id).unwrap();
            let result = dispatch_native_actor(
                actor,
                &mut entities,
                &mut fx,
                &mut tasks,
                &mut publications,
                &mut MainBaseAbortGameplayContext {
                    extra_lives: crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                    notifications: &mut notifications,
                    retail_tick: 400 + pass,
                },
            )
            .unwrap();
            let success = match result {
                Ok(success) => success,
                Err(failure) => panic!("world{level}: {:?}", failure.callback_error),
            };
            assert_eq!(
                success.disposition,
                MainBaseAbortActorDisposition::Type76Death
            );
        }
        assert_eq!(publications.type76_common_dying, 1, "world{level}");
        assert_eq!(
            tasks.family_for(id),
            Some(SpecializedActorTaskFamily::Intro2CommonDying)
        );
    }
}
