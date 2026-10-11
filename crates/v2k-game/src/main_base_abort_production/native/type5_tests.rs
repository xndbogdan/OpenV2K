use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    gameplay_notifications::GameplayNotifications,
    native_type122::construction_tests::native_fixture_with_player,
};

/// 170A0's 10C10 reaches the ordinary Type5 receipt: DB80 selects alternate
/// class11 and C660 publishes the falling Tumble, replacing the Search owner.
/// A repeat visit sees the dying body and publishes nothing further.
#[v2k_test_support::retail_test]
fn ordinary_type5_abort_publishes_its_class11_tumble_once() {
    for level in [42, 46, 47] {
        let (_, mut manager, mut fx) = native_fixture_with_player(level);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 5)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_type10(&manager) > 0);
        let mut publications = MainBaseAbortPublicationCounts::default();
        let mut notifications = GameplayNotifications::new();
        let mut dispatch =
            |manager: &mut EntityManager,
             tasks: &mut SpecializedActorTaskScheduler,
             publications: &mut MainBaseAbortPublicationCounts| {
                let observation = manager.main_base_abort_actor_observation(id).unwrap();
                dispatch_native_actor(
                    observation,
                    manager,
                    &mut fx,
                    tasks,
                    publications,
                    &mut MainBaseAbortGameplayContext {
                        extra_lives: RetailRuntimeValue::Unresolved,
                        notifications: &mut notifications,
                        retail_tick: 0x1dcb,
                    },
                )
                .expect("ordinary Type5 is a native abort actor")
                .unwrap_or_else(|failure| panic!("world{level}: {:?}", failure.callback_error))
            };
        let result = dispatch(&mut manager, &mut tasks, &mut publications);
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type10Death
        );
        assert_eq!(publications.type10_tumble, 1);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::TumbleOutOfSky(_))
        ));
        assert!(matches!(
            tasks.family_for(id),
            Some(crate::specialized_actor_task_production::SpecializedActorTaskFamily::Intro2Type10Tumble)
        ));
        dispatch(&mut manager, &mut tasks, &mut publications);
        assert_eq!(publications.type10_tumble, 1, "world{level}");
    }
}
