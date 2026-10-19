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

/// 170A0's 10C10 reaches world 20's flyer Type14 in its alternate class1
/// (BAC0): the shared blast/radial terminal with no Type60 ring, then
/// deferred removal.
#[v2k_test_support::retail_test]
fn type14_abort_explodes_through_its_class1_terminal() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(20);
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == crate::intro2_flyers_live::TYPE14_ENTITY_TYPE)
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
    .expect("an owned Type14 is a native class1 terminal");
    let result = match result {
        Ok(result) => result,
        Err(error) => panic!("{:?}", error.callback_error),
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
