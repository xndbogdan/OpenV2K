//! Real authored Hive cohorts exercise the shared 1BEB0 component pass.

use v2k_game::{
    entity::{AuthoredHiveInfectionBlock, EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_emitters::ComponentUpdateMode,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    hive_controller::{
        AuthoredHiveComponentFrame, HIVE_ENTITY_TYPE, HIVE_LOCKED_HEALTH_RAW, OBJECTIVE_STATE_BIT,
    },
    session::GameSession,
    world_complete_results::WorldCompleteTally,
    world_fx::WorldFx,
};

fn fixture(level: u32, health_override: Option<i32>) -> (GameSession, EntityManager) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("canonical retail data required");
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level, 1).unwrap();
    let mut metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    if let Some(health) = health_override {
        metadata[67].initial_health_raw = Some(health);
    }
    let manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    (session, manager)
}

fn clear_objectives(manager: &mut EntityManager) {
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for id in ids {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(OBJECTIVE_STATE_BIT, 0);
    }
}

fn tick(
    session: &GameSession,
    manager: &mut EntityManager,
    elapsed: u32,
    notifications: &mut GameplayNotifications,
    tally: &mut WorldCompleteTally,
    fx: &mut WorldFx,
) -> Result<(), AuthoredHiveInfectionBlock> {
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: elapsed,
                terrain: session.cache.terrain(),
                retail_tick: 100,
                notification_phase: GameplayNotificationPhase::Playing,
                notifications,
                world_complete_tally: tally,
            },
            |_| ComponentUpdateMode::Detailed,
            fx,
        )
        .map(|_| ())
}

#[v2k_test_support::retail_test]
fn native_hive_controller_runs_on_peasant_cistern_and_overridden_later_models() {
    for (level, live_model) in [(13, 341), (16, 583), (30, 1186)] {
        let (session, mut manager) = fixture(level, None);
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == HIVE_ENTITY_TYPE)
            .unwrap()
            .id;
        assert_eq!(
            manager.entity_mut(id).unwrap().model_index,
            Some(live_model)
        );
        let mut notifications = GameplayNotifications::new();
        let mut tally = WorldCompleteTally::default();
        let mut fx = WorldFx::new();
        tick(
            &session,
            &mut manager,
            20_000,
            &mut notifications,
            &mut tally,
            &mut fx,
        )
        .unwrap();
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(HIVE_LOCKED_HEALTH_RAW)
        );
        assert_eq!(notifications.save_tail_seen_mask() & (1 << 10), 1 << 10);
        clear_objectives(&mut manager);
        // The first callback already retained20ms in the component's timer.
        tick(
            &session,
            &mut manager,
            1_979_999,
            &mut notifications,
            &mut tally,
            &mut fx,
        )
        .unwrap();
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .authored_radial_emitter
                .as_ref()
                .unwrap()
                .controller_state(),
            1
        );
        tick(
            &session,
            &mut manager,
            1,
            &mut notifications,
            &mut tally,
            &mut fx,
        )
        .unwrap();
        let hive = manager.entity_mut(id).unwrap();
        assert_eq!(hive.collision.health_raw, RetailRuntimeValue::Known(2_000));
        assert_eq!(
            hive.authored_radial_emitter
                .as_ref()
                .unwrap()
                .controller_state(),
            2
        );
        assert_eq!(
            hive.authored_radial_emitter
                .as_ref()
                .unwrap()
                .accumulator_us(),
            0
        );
        assert_eq!(tally.ticks_0x2bc, -1);
        assert_ne!(notifications.save_tail_seen_mask() & (1 << 14), 0);
    }
}

#[v2k_test_support::retail_test]
fn multiple_native_hives_keep_independent_timers_and_use_metadata_health() {
    let (session, mut manager) = fixture(35, Some(7_777));
    clear_objectives(&mut manager);
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| {
            entity.entity_type == HIVE_ENTITY_TYPE && entity.authored_radial_emitter.is_some()
        })
        .map(|entity| entity.id)
        .collect();
    assert!(ids.len() > 1);
    let first_spawn = manager
        .entity_mut(ids[0])
        .unwrap()
        .authored_spawn_index
        .unwrap();
    manager.set_authored_behavior_components_enabled(first_spawn, false);
    let mut notifications = GameplayNotifications::new();
    let mut tally = WorldCompleteTally::default();
    let mut fx = WorldFx::new();
    tick(
        &session,
        &mut manager,
        2_000_000,
        &mut notifications,
        &mut tally,
        &mut fx,
    )
    .unwrap();
    for (index, id) in ids.iter().enumerate() {
        let hive = manager.entity_mut(*id).unwrap();
        assert_eq!(
            hive.authored_radial_emitter
                .as_ref()
                .unwrap()
                .controller_state(),
            if index == 0 { 1 } else { 2 }
        );
        assert_eq!(hive.collision.health_raw, RetailRuntimeValue::Known(7_777));
    }
    manager.set_authored_behavior_components_enabled(first_spawn, true);
    tick(
        &session,
        &mut manager,
        1_999_999,
        &mut notifications,
        &mut tally,
        &mut fx,
    )
    .unwrap();
    assert_eq!(
        manager
            .entity_mut(ids[0])
            .unwrap()
            .authored_radial_emitter
            .as_ref()
            .unwrap()
            .controller_state(),
        1
    );
    tick(
        &session,
        &mut manager,
        1,
        &mut notifications,
        &mut tally,
        &mut fx,
    )
    .unwrap();
    assert!(ids.iter().all(|id| manager
        .entity_mut(*id)
        .unwrap()
        .authored_radial_emitter
        .as_ref()
        .unwrap()
        .controller_state()
        == 2));
}

#[v2k_test_support::retail_test]
fn absent_sub_n_cannot_unlock_or_replay_an_unowned_live_component() {
    let (session, mut manager) = fixture(30, None);
    clear_objectives(&mut manager);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == HIVE_ENTITY_TYPE)
        .unwrap()
        .id;
    manager.entity_mut(id).unwrap().sub_n_runtime = RetailRuntimeValue::Known(None);
    let before = manager
        .entity_mut(id)
        .unwrap()
        .authored_radial_emitter
        .clone();
    let mut notifications = GameplayNotifications::new();
    let mut tally = WorldCompleteTally::default();
    let mut fx = WorldFx::new();
    assert_eq!(
        tick(
            &session,
            &mut manager,
            2_000_000,
            &mut notifications,
            &mut tally,
            &mut fx
        ),
        Err(AuthoredHiveInfectionBlock::UnauthenticatedHiveComponent { entity_id: id })
    );
    assert_eq!(
        manager.entity_mut(id).unwrap().authored_radial_emitter,
        before
    );
    assert_eq!(
        manager.entity_mut(id).unwrap().collision.health_raw,
        RetailRuntimeValue::Known(2_000)
    );
    assert_eq!(notifications.save_tail_seen_mask(), 0);
    assert_eq!(tally.ticks_0x2bc, 0);
}
