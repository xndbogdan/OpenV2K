use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    main_base_abort_world_effects::{
        snapshot_main_base_abort_terrain_geometry, MainBaseAbortClass49Frame,
    },
    native_entity_weapons::{EntityWeaponConstructionRequest, EntityWeaponKind},
    native_type122::construction_tests::native_fixture_with_player,
    native_type43::{Type43Owner, TYPE35_ENTITY_TYPE},
    player_hull::PlayerHull,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

/// 170A0's 10C10 reaches Type35's DB80: Search's +2C is null and AC60 on the
/// dying body republishes the class7 alternate; the launcher stays linked.
#[v2k_test_support::retail_test]
fn type35_abort_republishes_search_on_the_dying_body() {
    for level in 41..=47 {
        let (_, mut manager, mut fx) = native_fixture_with_player(level);
        let ids: Vec<u32> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == TYPE35_ENTITY_TYPE)
            .map(|entity| entity.id)
            .collect();
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_type43(&manager) >= ids.len());
        for id in ids {
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
            .expect("a native Type35 has a native abort death")
            .unwrap_or_else(|failure| panic!("world{level}: {:?}", failure.callback_error));
            assert_eq!(
                result.disposition,
                MainBaseAbortActorDisposition::Type35Death
            );
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
                RetailRuntimeValue::Known(DYING_STATE_BIT)
            );
            assert!(!manager.pending_actor_deferred_destroy_ids().contains(&id));
            assert!(Type43Owner::adopt(&manager, id).is_ok());
            assert!(tasks.type43_completed_owner(&manager, id));
        }
    }
}

/// A meteor in flight takes its class1 alternate: BAF0's blast, then BAC0
/// stages removal.
#[v2k_test_support::retail_test]
fn a_meteor_in_flight_explodes_in_the_abort() {
    let (mut session, mut manager, mut fx) = native_fixture_with_player(43);
    let launcher = manager
        .iter_all()
        .find(|entity| entity.entity_type == TYPE35_ENTITY_TYPE)
        .unwrap();
    let (source, position) = (launcher.id, launcher.position_raw());
    let owner = manager
        .construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Meteor,
                source_actor_id: source,
                position_raw: [position[0], position[1].wrapping_add(300), position[2]],
                velocity_raw: [0, 0x960, 0],
                rotation_raw: [0; 3],
            },
            &session.cache,
            &mut fx,
            600,
        )
        .unwrap();
    let id = owner.entity_id();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    // The blast reaches the launcher 300 below; its owner lends custody.
    assert!(scheduler.adopt_type43(&manager) > 0);
    scheduler.register_native_weapon(&manager, owner).unwrap();
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
    .expect("a native meteor is a BAF0 terminal")
    .unwrap_or_else(|failure| panic!("{:?}", failure.callback_error));
    assert_eq!(
        result.disposition,
        MainBaseAbortActorDisposition::Class49Death
    );
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &manager, id
    ));
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    assert_eq!(publications.appended_type60_actors, 0);
}
