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

/// 170A0's 10C10 reaches world 20's flyer Type14 and world 31's Type39 in
/// their alternate class1 (BAC0): the shared blast/radial terminal with no
/// Type60 ring, then deferred removal.
#[v2k_test_support::retail_test]
fn exploding_flyer_abort_explodes_through_its_class1_terminal() {
    for (level, entity_type) in [(20, 14), (31, 39)] {
        abort_explodes(level, entity_type);
    }
}

fn abort_explodes(level: u32, entity_type: u32) {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(level);
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == entity_type)
        .unwrap()
        .id;
    // Lift the source clear of every other actor; the radial stays real.
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([100, 10000, -300]);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_intro2_flyers(&manager) > 0);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut player_hull = PlayerHull::default();
    let geometry = snapshot_main_base_abort_terrain_geometry(&session.cache).unwrap();
    let mut effects =
        MainBaseAbortWorldEffects::new(&geometry, &mut session.cache, &mut static_damage).unwrap();
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
    .expect("an owned exploding flyer is a native class1 terminal");
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

/// 170A0's 10C10 reaches world 25's Type71 gunner in its alternate class63
/// (BC90): BAF0's blast, a Type61 appended behind the live-list cursor from its
/// `+88` payload, then deferred removal.
#[v2k_test_support::retail_test]
fn type71_abort_takes_class63_and_appends_its_power_up() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(25);
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 71)
        .unwrap()
        .id;
    manager
        .entity_mut(id)
        .unwrap()
        .set_position_raw([100, 10000, -300]);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_intro2_type47_guards(&manager) > 0);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut player_hull = PlayerHull::default();
    let geometry = snapshot_main_base_abort_terrain_geometry(&session.cache).unwrap();
    let mut effects =
        MainBaseAbortWorldEffects::new(&geometry, &mut session.cache, &mut static_damage).unwrap();
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
    .expect("an owned Type71 is a native class63 terminal");
    let result = match result {
        Ok(result) => result,
        Err(error) => panic!("{:?}", error.callback_error),
    };
    assert_eq!(
        result.disposition,
        MainBaseAbortActorDisposition::Class49Death
    );
    assert_eq!(
        publications.appended_type60_actors, 0,
        "class63 has no ring"
    );
    assert_eq!(publications.appended_type61_actors, 1);
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
}
