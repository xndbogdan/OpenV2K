use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    native_flying_surface_contact::native_flyer_quiet_death_authenticates,
    native_type122::construction_tests::native_fixture_with_player,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

/// 170A0's 10C10 reaches the native flyers' alternate class2: sound 11 stops
/// and C470's quiet terminal retires the living owner, as a lethal hit does.
#[v2k_test_support::retail_test]
fn native_flyer_abort_dies_quietly_through_alternate_class2() {
    for (level, entity_type) in [(20, 15), (42, 108)] {
        let (_, mut manager, mut fx) = native_fixture_with_player(level);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_flyers(&manager) > 0);
        assert!(tasks.intro2_flyer_completed_owner(&manager, id));
        let mut publications = MainBaseAbortPublicationCounts::default();
        let observation = manager.main_base_abort_actor_observation(id).unwrap();
        let result = dispatch_native_actor(
            observation,
            &mut manager,
            &mut fx,
            &mut tasks,
            &mut publications,
            &mut MainBaseAbortGameplayContext {
                extra_lives: RetailRuntimeValue::Unresolved,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 600,
            },
            &crate::resource_cache::ResourceCache::new(Vec::new()),
        )
        .expect("a native flyer has a native abort death")
        .unwrap_or_else(|failure| panic!("world{level}: {:?}", failure.callback_error));
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::QuietDeath
        );
        assert!(native_flyer_quiet_death_authenticates(&manager, id));
        assert!(!tasks.intro2_flyer_completed_owner(&manager, id));
    }
}
