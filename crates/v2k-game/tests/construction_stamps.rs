//! Source world identity and player reservation across actual load boundaries.

use v2k_game::{
    campaign_transition::collect_authored_warp_markers,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    session::GameSession,
    world_fx::WorldFx,
};

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session
}

fn construct(
    session: &GameSession,
    logical_world_index: i32,
    player: bool,
    fx: &mut WorldFx,
) -> EntityManager {
    let rows: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index,
            type_metadata: &rows,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: player.then_some(AuthoredPlayerArrival {
                position_raw: [19_712, -500, 14_848],
                heading_raw: 0x4000,
            }),
            retail_tick: 0,
        },
        fx,
    )
    .unwrap()
}

fn authored_stamps(manager: &EntityManager) -> Vec<RetailRuntimeValue<u16>> {
    manager
        .iter_all()
        .filter(|entity| entity.authored_spawn_index.is_some())
        .map(|entity| entity.construction_stamp_at_0xb4)
        .collect()
}

fn marker_construction_attempts(session: &GameSession) -> u16 {
    // 33BD0 allocates every marker before 42EFB0 queues duplicate/Hive
    // helpers for removal. Pending cleanup does not refund their stamps.
    collect_authored_warp_markers(
        session.cache.terrain().unwrap(),
        session.cache.terrain_objects().unwrap(),
    )
    .unwrap()
    .len() as u16
}

#[v2k_test_support::retail_test]
fn creating_or_reserving_the_player_keeps_identical_authored_ordinals() {
    let mut session = session();
    session.load_level_by_id(14, 1).unwrap();
    let mut fx = WorldFx::new();
    let created = construct(&session, 2, true, &mut fx);
    let reserved = construct(&session, 2, false, &mut fx);
    assert_eq!(
        created.player().unwrap().construction_stamp_at_0xb4,
        RetailRuntimeValue::Known(0x0800)
    );
    assert!(reserved.player().is_none());
    assert_eq!(authored_stamps(&created), authored_stamps(&reserved));
    assert_eq!(
        created.next_common_body_ordinal(),
        reserved.next_common_body_ordinal()
    );
    assert_eq!(
        authored_stamps(&created)[0],
        RetailRuntimeValue::Known(0x0801 + marker_construction_attempts(&session))
    );
    let first_authored_id = |manager: &EntityManager| {
        manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index.is_some())
            .unwrap()
            .id
    };
    assert_ne!(
        first_authored_id(&created),
        first_authored_id(&reserved),
        "the same authored stamp survives different live handles"
    );
}

#[v2k_test_support::retail_test]
fn native_world_entry_resets_stamps_while_cache_load_and_effect_clear_do_not() {
    let mut session = session();
    let mut fx = WorldFx::new();
    let mut first = None;
    for (overlay, logical) in [(14, 2), (39, 27), (14, 2), (49, 37)] {
        session.load_level_by_id(overlay, 1).unwrap();
        let manager = construct(&session, logical, true, &mut fx);
        let stamps = authored_stamps(&manager);
        assert_eq!(
            stamps[0],
            RetailRuntimeValue::Known(
                ((logical << 10) as u16).wrapping_add(1 + marker_construction_attempts(&session))
            )
        );
        if overlay == 14 {
            if let Some(first) = &first {
                assert_eq!(&stamps, first);
            } else {
                first = Some(stamps);
            }
        }
        let ordinal = manager.next_common_body_ordinal();
        let player_stamp = manager.player().unwrap().construction_stamp_at_0xb4;
        // These operations manage resources/effects. They do not represent
        // the source451710 load entry and cannot reset a still-live manager.
        session.load_level_by_id(13, 1).unwrap();
        fx.clear();
        assert_eq!(manager.next_common_body_ordinal(), ordinal);
        assert_eq!(
            manager.player().unwrap().construction_stamp_at_0xb4,
            player_stamp
        );
    }
}

#[v2k_test_support::retail_test]
fn closing_scene_bodies_carry_the_stamps_its_commands_name() {
    // A retail lab run of the closing scene (overlay 49 in phase 3, no
    // player) dumps eleven Type123s at 0x9401..0x940B, the Type122 ant at
    // 0x940C and the 34 Type52 beacons at 0x940D..0x942E.
    let mut session = session();
    session.load_level_by_id(49, 1).unwrap();
    let mut fx = WorldFx::new();
    let mut manager = construct(&session, 0x25, false, &mut fx);
    assert!(manager.player().is_none());
    let bodies: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.authored_spawn_index.is_some())
        .map(|entity| (entity.entity_type, entity.construction_stamp_at_0xb4))
        .collect();
    assert_eq!(bodies.len(), 46);
    for (index, (entity_type, stamp)) in bodies.iter().enumerate() {
        let expected_type = match index {
            0..=10 => 123,
            11 => 122,
            _ => 52,
        };
        assert_eq!(*entity_type, expected_type, "body {index}");
        assert_eq!(*stamp, RetailRuntimeValue::Known(0x9401 + index as u16));
    }
    // 451710 clears the ant's behavior bits; its record at 23 s restores them.
    manager.disable_authored_behavior_components();
    let strings = session.cache.level().unwrap().strings.clone();
    let mut scene = v2k_game::closing_scene::ClosingScene::new(
        &strings,
        v2k_game::opening::IntroCameraController::default(),
    );
    let ant = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let ant_state = |manager: &EntityManager| {
        manager
            .iter_all()
            .find(|entity| entity.id == ant)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x0006_8000)
    };
    assert_eq!(ant_state(&manager), RetailRuntimeValue::Known(0));
    // The first record (camera 0x9401 to 3 s) is never visited.
    assert!(!scene.present(&mut manager, 100).leave);
    assert!(scene.camera_subject(&manager).is_none());
    assert!(!scene.present(&mut manager, 1150).leave);
    assert_eq!(ant_state(&manager), RetailRuntimeValue::Known(0x0006_8000));
    // The camera moves to the ant at 24 s.
    assert!(!scene.present(&mut manager, 1200).leave);
    let ant_position = manager
        .iter_all()
        .find(|entity| entity.id == ant)
        .unwrap()
        .position_raw();
    assert_eq!(
        scene.camera_subject(&manager).unwrap().position_raw,
        ant_position
    );
    assert!(scene.present(&mut manager, 14_500).leave);
}
