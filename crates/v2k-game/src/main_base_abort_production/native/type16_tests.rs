use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    native_type122::construction_tests::native_fixture_with_player,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
};

/// 170A0's 10C10 reaches the native Type16 row's alternate class12, Flip Over
/// And Die: the common class12 owner publishes, as for a lethal hit.
#[v2k_test_support::retail_test]
fn type16_abort_flips_over_through_class12() {
    for level in [20, 24, 37] {
        let (_, mut manager, mut fx) = native_fixture_with_player(level);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 16)
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_type16(&manager) > 0);
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
        .expect("a native Type16 has a native abort death")
        .unwrap_or_else(|failure| panic!("world{level}: {:?}", failure.callback_error));
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Type16Death
        );
        assert_eq!(publications.type16_common_dying, 1, "world{level}");
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
    }
}
