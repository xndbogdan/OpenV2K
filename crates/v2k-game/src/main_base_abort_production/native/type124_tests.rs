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

/// 170A0's 10C10 reaches Type124's alternate class63: BAF0, then BC90 appends
/// the authored Type61 behind the cursor and stages the fish's removal. The
/// cursor's later visit gives that drop its own class49 ring death.
#[v2k_test_support::retail_test]
fn type124_abort_drops_its_power_up_and_the_drop_takes_class49() {
    for level in [23, 30, 34] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 124)
            .unwrap()
            .id;
        let payload = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .auto_pilot_payload_packed
            .unwrap();
        let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_shared_fish(&manager) > 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut player_hull = PlayerHull::default();
        let geometry = snapshot_main_base_abort_terrain_geometry(&session.cache).unwrap();
        let mut effects =
            MainBaseAbortWorldEffects::new(&geometry, &mut session.cache, &mut static_damage)
                .unwrap();
        let mut publications = MainBaseAbortPublicationCounts::default();
        let mut dispatch = |manager: &mut EntityManager, id: u32, publications: &mut _| {
            let actor = manager.main_base_abort_actor_observation(id).unwrap();
            dispatch_native_class49(
                actor,
                &mut effects,
                MainBaseAbortClass49Frame {
                    extra_lives: RetailRuntimeValue::Unresolved,
                    entities: manager,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                    retail_tick: 600,
                    scheduler: &mut scheduler,
                    player_hull: &mut player_hull,
                },
                publications,
            )
            .expect("native BAF0 terminal")
        };
        let result = match dispatch(&mut manager, id, &mut publications) {
            Ok(result) => result,
            Err(error) => panic!("world{level}: {:?}", error.callback_error),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Class49Death
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        assert_eq!(publications.appended_type61_actors, 1, "world{level}");
        let drop = manager
            .iter_all()
            .find(|entity| entity.id > newest && entity.entity_type == 61)
            .unwrap();
        assert_eq!(drop.power_up_payload_packed, Some(payload));
        let drop = drop.id;
        assert_eq!(
            manager.iter_all().last().map(|entity| entity.id),
            Some(drop)
        );

        let rings = publications.appended_type60_actors;
        let result = match dispatch(&mut manager, drop, &mut publications) {
            Ok(result) => result,
            Err(error) => panic!("world{level} drop: {:?}", error.callback_error),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Class49Death
        );
        assert_eq!(publications.appended_type60_actors, rings + 1);
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&drop));
    }
}
