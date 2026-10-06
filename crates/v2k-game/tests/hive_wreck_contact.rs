use v2k_game::{
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    hive_controller::AuthoredHiveComponentFrame,
};

use v2k_game::{
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_emitters::ComponentUpdateMode,
    hive_wreck_presentation::HiveWreckPresentationRequest,
    session::GameSession,
    world_fx::WorldFx,
};

fn wreck_fixture() -> (GameSession, EntityManager, u32, [i16; 3]) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, models)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *models,
                    ..Default::default()
                })
        })
        .collect();
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let hive = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(runtime)) = hive.sub_n_runtime else {
        panic!("native Sub-N")
    };
    let RetailRuntimeValue::Known(origin) = runtime.anchor_raw() else {
        panic!("native anchor")
    };
    // Controlled wreck state isolates contact admission from the separately
    // tested lethal transaction, retaining the authored Sub-N and controller.
    hive.collision.state_flags_at_0x08.overwrite(0x4000, 0x4000);
    hive.model_index = Some(343);
    let emitter = hive.authored_radial_emitter.as_mut().unwrap();
    emitter.enter_dying_slot0();
    emitter.arm_wreck_suction();
    (session, manager, id, origin)
}

#[v2k_test_support::retail_test]
fn wreck_ring_callback_requires_the_native_render_flag() {
    let (_, mut manager, id, origin) = wreck_fixture();
    let hive = manager.entity_mut(id).unwrap();
    let request = HiveWreckPresentationRequest::from_entity(hive).unwrap();
    assert_eq!(request.anchor_raw, origin);
    assert!(request.marker_present);
    hive.collision.state_flags_at_0x08.overwrite(0x800, 0);
    assert!(HiveWreckPresentationRequest::from_entity(hive).is_none());
}

#[v2k_test_support::retail_test]
fn absent_player_does_not_pause_the_wreck_clock() {
    let (session, mut manager, id, _) = wreck_fixture();
    manager.remove_player_and_attached_entities();
    let mut effects = WorldFx::new();
    for _ in 0..301 {
        manager
            .advance_authored_hive_components(
                AuthoredHiveComponentFrame {
                    elapsed_us: 20_000,
                    terrain: session.cache.terrain(),
                    retail_tick: 0,
                    notification_phase: GameplayNotificationPhase::NonGameplay,
                    notifications: &mut GameplayNotifications::new(),
                    world_complete_tally:
                        &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
                },
                |_| ComponentUpdateMode::Detailed,
                &mut effects,
            )
            .unwrap();
    }
    let hive = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_eq!(
        hive.authored_radial_emitter
            .as_ref()
            .unwrap()
            .suction_timer_us(),
        20_000
    );
    assert!(manager.player().is_none());
    assert!(
        hive.authored_radial_emitter
            .as_ref()
            .unwrap()
            .wreck_suction_ready(),
        "the source component clock advances even without a player"
    );
}

#[v2k_test_support::retail_test]
fn contact_eligibility_gates_force_but_not_the_clock() {
    for admitted in [false, true] {
        let (session, mut manager, id, origin) = wreck_fixture();
        let player = manager.player_mut().unwrap();
        player.set_motion_raw([origin[0].wrapping_add(100), origin[1], origin[2]], [0; 3]);
        player
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, if admitted { 0x8000 } else { 0 });
        let mut effects = WorldFx::new();
        for _ in 0..300 {
            manager
                .advance_authored_hive_components(
                    AuthoredHiveComponentFrame {
                        elapsed_us: 20_000,
                        terrain: session.cache.terrain(),
                        retail_tick: 0,
                        notification_phase: GameplayNotificationPhase::NonGameplay,
                        notifications: &mut GameplayNotifications::new(),
                        world_complete_tally:
                            &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
                    },
                    |_| ComponentUpdateMode::Detailed,
                    &mut effects,
                )
                .unwrap();
        }
        assert_eq!(
            manager.player().unwrap().velocity_raw(),
            [0; 3],
            "strict six-second boundary"
        );
        assert!(!manager
            .entity_mut(id)
            .unwrap()
            .authored_radial_emitter
            .as_ref()
            .unwrap()
            .wreck_suction_ready());
        manager
            .advance_authored_hive_components(
                AuthoredHiveComponentFrame {
                    elapsed_us: 20_000,
                    terrain: session.cache.terrain(),
                    retail_tick: 0,
                    notification_phase: GameplayNotificationPhase::NonGameplay,
                    notifications: &mut GameplayNotifications::new(),
                    world_complete_tally:
                        &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
                },
                |_| ComponentUpdateMode::Detailed,
                &mut effects,
            )
            .unwrap();
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .authored_radial_emitter
                .as_ref()
                .unwrap()
                .suction_timer_us(),
            20_000
        );
        assert_eq!(
            manager.player().unwrap().velocity_raw(),
            if admitted { [-3, -28, 0] } else { [0; 3] }
        );
        assert!(
            manager
                .entity_mut(id)
                .unwrap()
                .authored_radial_emitter
                .as_ref()
                .unwrap()
                .wreck_suction_ready(),
            "readiness is component time; player admission gates the actual pull above"
        );
        assert_eq!(
            HiveWreckPresentationRequest::from_entity(manager.entity_mut(id).unwrap())
                .unwrap()
                .anchor_raw,
            origin
        );
    }
}
