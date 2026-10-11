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

/// 170A0's 10C10 reaches the Type80/126 carriers' alternate class63 (BC90),
/// never their Type10 owner's class11 Tumble: BAF0, then the authored Type61
/// is tail-appended and the carrier keeps its Search tasks until the sweep.
#[v2k_test_support::retail_test]
fn type80_126_abort_drops_their_power_ups_through_class63() {
    for (level, entity_type) in [(39, 80), (25, 126)] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == entity_type)
            .unwrap()
            .id;
        let (payload, primary) = {
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            (
                entity.auto_pilot_payload_packed.unwrap(),
                entity
                    .actor_tasks
                    .task_in_slot(crate::actor_task_owner::ActorTaskSlot::Primary),
            )
        };
        let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type10(&manager) > 0);
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
        .expect("a class63 carrier is a native BAF0 terminal");
        let result = match result {
            Ok(result) => result,
            Err(error) => panic!("world{level}: {:?}", error.callback_error),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Class49Death
        );
        assert_eq!(publications.type10_tumble, 0, "no class11 Tumble");
        assert_eq!(publications.appended_type61_actors, 1, "world{level}");
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        assert_eq!(
            entity
                .actor_tasks
                .task_in_slot(crate::actor_task_owner::ActorTaskSlot::Primary),
            primary
        );
        let drop = manager
            .iter_all()
            .find(|entity| entity.id > newest && entity.entity_type == 61)
            .unwrap();
        assert_eq!(drop.power_up_payload_packed, Some(payload));
    }
}
