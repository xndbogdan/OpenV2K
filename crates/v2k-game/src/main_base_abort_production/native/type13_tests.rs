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

/// 170A0's 10C10 reaches ordinary Type13's alternate class1 (BAC0): the shared
/// blast/radial terminal with no Type60 ring, then deferred removal.
#[v2k_test_support::retail_test]
fn ordinary_type13_abort_explodes_through_its_class1_terminal() {
    for level in [46, 47, 48] {
        let (mut session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 13)
            .unwrap()
            .id;
        // Lift the source clear of every other actor; the radial stays real.
        manager
            .entity_mut(id)
            .unwrap()
            .set_position_raw([100, 10000, -300]);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert!(scheduler.adopt_intro2_type13_search_attack(&manager) > 0);
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
        .expect("ordinary Type13 is a native class1 terminal");
        let result = match result {
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
        assert_eq!(publications.appended_type60_actors, 0, "class1 has no ring");
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    }
}

/// Intro2's spawn0 carries no receipt; the ordinary dispatcher leaves it alone.
#[v2k_test_support::retail_test]
fn the_abort_class1_route_requires_the_ordinary_receipt() {
    let (mut session, mut manager, mut fx) = crate::intro2_gun_turret::authored_tests::fixture(46);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 13)
        .unwrap()
        .id;
    manager.entity_mut(id).unwrap().native_type13_allocation = None;
    let mut static_damage = StaticDamageScheduler::new();
    let geometry = snapshot_main_base_abort_terrain_geometry(&session.cache).unwrap();
    let mut effects =
        MainBaseAbortWorldEffects::new(&geometry, &mut session.cache, &mut static_damage).unwrap();
    let actor = manager.main_base_abort_actor_observation(id).unwrap();
    assert!(dispatch_native_class49(
        actor,
        &mut effects,
        MainBaseAbortClass49Frame {
            extra_lives: RetailRuntimeValue::Unresolved,
            entities: &mut manager,
            world_fx: &mut fx,
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 600,
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            player_hull: &mut PlayerHull::default(),
        },
        &mut MainBaseAbortPublicationCounts::default(),
    )
    .is_none());
}
