use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    main_base_abort_world_effects::{
        snapshot_main_base_abort_terrain_geometry, MainBaseAbortClass49Frame,
    },
    player_hull::PlayerHull,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

/// 170A0's 10C10 reaches Type43's alternate class1: BAF0, then BAC0 clears
/// every task and stages removal. No ring or power-up follows.
#[v2k_test_support::retail_test]
fn type43_abort_explodes_through_class1() {
    for level in [42, 46, 47] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == crate::native_type43::ENTITY_TYPE)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_type43(&manager) > 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut player_hull = PlayerHull::default();
        let geometry = snapshot_main_base_abort_terrain_geometry(&session.cache).unwrap();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut session.cache, &mut static_damage)
                .unwrap();
        let mut publications = MainBaseAbortPublicationCounts::default();
        let actor = manager.main_base_abort_actor_observation(id).unwrap();
        let result = dispatch_native_class49(
            actor,
            &mut effects,
            MainBaseAbortClass49Frame {
                extra_lives: RetailRuntimeValue::Unresolved,
                entities: &mut manager,
                world_fx: &mut fx,
                notifications: &mut notifications,
                retail_tick: 600,
                scheduler: &mut scheduler,
                player_hull: &mut player_hull,
            },
            &mut publications,
        )
        .expect("an emitter-only Type43 is a native BAF0 terminal");
        let result = match result {
            Ok(result) => result,
            Err(error) => panic!("world{level}: {:?}", error.callback_error),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Class49Death
        );
        assert_eq!(publications.appended_type60_actors, 0, "world{level}");
        assert_eq!(publications.appended_type61_actors, 0, "world{level}");
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    }
}
