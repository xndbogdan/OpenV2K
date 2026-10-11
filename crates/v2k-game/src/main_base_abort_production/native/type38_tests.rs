use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    main_base_abort_world_effects::{
        snapshot_main_base_abort_terrain_geometry, MainBaseAbortClass49Frame,
    },
    native_type38::Type38Row,
    player_hull::PlayerHull,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

/// 170A0's 10C10 reaches each row's alternate: Type38's class1 (BAF0, then
/// BAC0 stages removal with nothing appended) and Type129's class63 (BAF0,
/// then the authored Type61 is tail-appended behind the cursor).
#[v2k_test_support::retail_test]
fn type38_family_abort_runs_class1_and_class63() {
    for (level, row) in [
        (41, Type38Row::Type129),
        (42, Type38Row::Type38),
        (43, Type38Row::Type38),
    ] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let entity = manager
            .iter_all()
            .find(|entity| entity.entity_type == row.entity_type())
            .unwrap();
        let (id, payload) = (entity.id, entity.auto_pilot_payload_packed);
        let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_type38_family(&manager) > 0);
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
        .expect("a Type38-family actor is a native BAF0 terminal");
        let result = match result {
            Ok(result) => result,
            Err(error) => panic!("world{level}: {:?}", error.callback_error),
        };
        assert_eq!(
            result.disposition,
            MainBaseAbortActorDisposition::Class49Death
        );
        assert_eq!(publications.appended_type60_actors, 0, "world{level}");
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        match row {
            Type38Row::Type38 => {
                assert_eq!(publications.appended_type61_actors, 0, "world{level}");
                assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
            }
            Type38Row::Type129 => {
                assert_eq!(publications.appended_type61_actors, 1, "world{level}");
                let drop = manager
                    .iter_all()
                    .find(|entity| entity.id > newest && entity.entity_type == 61)
                    .unwrap();
                assert_eq!(drop.power_up_payload_packed, payload);
            }
        }
    }
}
