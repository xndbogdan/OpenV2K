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

/// 170A0's 10C10 reaches Type128's alternate class63, never Type16's class12:
/// BAF0, then the authored Type61 is tail-appended behind the cursor.
#[v2k_test_support::retail_test]
fn type128_abort_drops_its_power_up_through_class63() {
    for level in [25, 28] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| {
                entity.entity_type == 128
                    && matches!(entity.current_behavior_context,
                        RetailRuntimeValue::Known(Some(context))
                            if !matches!(context.active_style().style_address(),
                                0x004c_8080 | 0x004c_80c8 | 0x004c_8110 | 0x004c_8158))
            })
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
        assert!(scheduler.adopt_intro2_type16(&manager) > 0);
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
        assert_eq!(publications.appended_type61_actors, 1, "world{level}");
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        let drop = manager
            .iter_all()
            .find(|entity| entity.id > newest && entity.entity_type == 61)
            .unwrap();
        assert_eq!(drop.power_up_payload_packed, Some(payload));
    }
}
