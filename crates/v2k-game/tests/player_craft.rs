//! Integration tests for the player craft draw path against the real game data
//! (ignored when no retail install is configured).
//!
//! Exercises the pieces the renderer feeds on each frame — player4 resolution,
//! authored Sub-O mode-joint materialization, and sub-model instance resolution
//! — so reversed gear/fan endpoints or a loadout swap are caught headlessly.

use v2k_game::damage::{
    DamageDeliveryRecord, DamageProfile, EntityHitEntry, PRIMARY_PROJECTILE_DAMAGE_PACKET,
    TYPE_46_DAMAGE_PROFILE,
};
use v2k_game::entity::{CheckedProjectileDamageOutcome, CheckedProjectileDamageRequest};
use v2k_game::entity_behavior::{BehaviorSelection, BehaviorStyle, ReleaseCallbackPolicy};
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::factory_production::FactoryProductionPhase;
use v2k_game::model_tree::{linked_model_named_tips, ModelTreeNamedTipRequest};
use v2k_game::player::{MotionChannels, PlayerCraft, VehicleMode, VehicleModeToggleOutcome};
use v2k_game::session::GameSession;

#[v2k_test_support::retail_test]
fn player4_materializes_retail_hover_and_vtol_mode_joint_endpoints() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    // System level 3 (the global model pool that carries player4).
    session.load_auxiliary_ovl(3, 0).expect("load L3 aux");

    let (pid, entry) = session
        .cache
        .global_model_by_name("player4")
        .expect("player4 in global pool");
    assert_eq!(entry.name.as_deref(), Some("player4"));
    assert_eq!(pid, 41, "player spawn's Section-13 model override");
    // Re-resolvable by id (the render path uses global_model(id)).
    assert!(session.cache.global_model(pid).is_some());

    let model = session.cache.global_model(pid).unwrap();

    // A freshly allocated retail joint array is zeroed. Let style-0 Hover drive
    // both Sub-O mode words to its retained 50-Hz equilibrium before comparing
    // the authored model endpoint.
    let mut craft = PlayerCraft::new();
    craft.fuel_raw = v2k_game::player::FUEL_FULL_RAW;
    let mut player = make_player();
    let ch = MotionChannels::default();
    for _ in 0..500 {
        craft.integrate_configured_micros(20_000, &ch, &mut player, 1);
    }
    assert_eq!(craft.mode_joint_words(), [65_509, 65_509]);
    let hover = model.materialize(&craft.anim_vars());
    assert_eq!(hover.vertices.len(), 80, "Hover deploys PLAYER4 panels");
    assert_eq!(hover.triangles.len(), 128);

    // player4 directly instances the engine surround (the spinning fan shroud),
    // so the fan-spin target set (`PlayerCraft::fan_spin_names`) matches real data.
    assert!(
        hover.instances.iter().any(|inst| {
            session
                .cache
                .global_model(inst.model_id as usize)
                .and_then(|e| e.name.as_deref())
                == Some("pl4enginesurround")
        }),
        "player4 instances pl4enginesurround"
    );
    let hover_sidepods = hover
        .instances
        .iter()
        .filter(|inst| {
            session
                .cache
                .global_model(inst.model_id as usize)
                .and_then(|entry| entry.name.as_deref())
                == Some("pl4sidepods")
        })
        .map(|inst| inst.attach_pos.expect("sidepod attach position"))
        .collect::<Vec<_>>();
    assert_eq!(hover_sidepods.len(), 2);
    assert!((hover_sidepods[0][0] - 138.0).abs() < 0.1);
    assert!((hover_sidepods[0][1] + 48.0).abs() < 0.1);
    assert!((hover_sidepods[1][0] + 140.0).abs() < 0.1);
    assert!((hover_sidepods[1][1] + 48.0).abs() < 0.1);

    assert_eq!(
        craft.toggle_mode(),
        VehicleModeToggleOutcome::Changed(VehicleMode::Vtol)
    );
    for _ in 0..300 {
        craft.integrate_configured_micros(20_000, &ch, &mut player, 1);
    }
    assert_eq!(craft.mode_joint_words(), [0, 0]);
    let vtol = model.materialize(&craft.anim_vars());
    assert_eq!(vtol.vertices.len(), 68, "VTOL retracts PLAYER4 panels");
    assert_eq!(vtol.triangles.len(), 112);
    let vtol_sidepods = vtol
        .instances
        .iter()
        .filter(|inst| {
            session
                .cache
                .global_model(inst.model_id as usize)
                .and_then(|entry| entry.name.as_deref())
                == Some("pl4sidepods")
        })
        .map(|inst| inst.attach_pos.expect("sidepod attach position"))
        .collect::<Vec<_>>();
    assert_eq!(
        vtol_sidepods,
        vec![[100.0, -20.0, -100.0], [-100.0, -20.0, -100.0]]
    );

    // Callback word 4 is an independent weapon/loadout selector; TAB must not
    // turn the two default gatling guns into the authored cannon alternative.
    let vtol_guns = vtol
        .instances
        .iter()
        .filter_map(|inst| {
            session
                .cache
                .global_model(inst.model_id as usize)
                .and_then(|entry| entry.name.as_deref())
        })
        .filter(|name| *name == "pl4gatgun")
        .count();
    assert_eq!(vtol_guns, 2);

    for inst in hover.instances.iter().chain(&vtol.instances) {
        assert!(
            session.cache.global_model(inst.model_id as usize).is_some(),
            "instance model_id {} unresolved",
            inst.model_id
        );
    }
}

#[v2k_test_support::retail_test]
fn player4_side_gun_tips_resolve_to_two_distinct_authored_origins() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 0).expect("load L3 aux");
    let (player_id, _) = session
        .cache
        .global_model_by_name("player4")
        .expect("player4 in global pool");
    let craft = PlayerCraft::new();
    let vars = craft.anim_vars();
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    let tips = linked_model_named_tips(
        &session.cache,
        ModelTreeNamedTipRequest {
            model_id: player_id,
            orientation: identity,
            position: [0.0; 3],
            scale: 100.0 / 256.0,
            depth: 3,
            linked: None,
            vars: &vars,
            child_transform: None,
            model_name: "pl4gatgun",
            local_tip_axis: [0.0, 0.0, 1.0],
        },
    );

    assert_eq!(tips.len(), 2, "player4 authored two pl4gatgun instances");
    let distance_squared = tips[0]
        .origin_world
        .iter()
        .zip(tips[1].origin_world)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>();
    assert!(
        distance_squared > 1.0,
        "gun tips must not collapse to center"
    );
    for tip in &tips {
        assert_eq!(tip.direction_unit, [0.0, 0.0, 1.0]);
        assert!((tip.origin_world[1] - 0.085_937_5).abs() < 1.0e-6);
        assert!((tip.origin_world[2] - 0.546_875).abs() < 1.0e-6);
    }
    assert!((tips[0].origin_world[0] - 0.531_25).abs() < 1.0e-6);
    assert!((tips[1].origin_world[0] + 0.531_25).abs() < 1.0e-6);
}

#[v2k_test_support::retail_test]
fn player4_selector_two_uses_two_distinct_authored_tube_gun_tips() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 0).expect("load L3 aux");
    let (player_id, player) = session
        .cache
        .global_model_by_name("player4")
        .expect("player4 in global pool");
    let mut craft = PlayerCraft::new();
    craft.select_weapon_callback(4);
    let vars = craft.anim_vars();
    let materialized = player.materialize(&vars);
    let tube_guns = materialized
        .instances
        .iter()
        .filter(|instance| {
            session
                .cache
                .global_model(instance.model_id as usize)
                .and_then(|entry| entry.name.as_deref())
                == Some("pl4tubegun")
        })
        .count();
    assert_eq!(tube_guns, 2);

    let tips = linked_model_named_tips(
        &session.cache,
        ModelTreeNamedTipRequest {
            model_id: player_id,
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 100.0 / 256.0,
            depth: 3,
            linked: None,
            vars: &vars,
            child_transform: None,
            model_name: "pl4tubegun",
            local_tip_axis: [0.0, 0.0, 1.0],
        },
    );
    assert_eq!(tips.len(), 2);
    for tip in &tips {
        assert_eq!(tip.direction_unit, [0.0, 0.0, 1.0]);
        assert!((tip.origin_world[1] - 0.093_75).abs() < 1.0e-6);
        assert!((tip.origin_world[2] - 0.703_125).abs() < 1.0e-6);
    }
    assert!((tips[0].origin_world[0] - 0.578_125).abs() < 1.0e-6);
    assert!((tips[1].origin_world[0] + 0.578_125).abs() < 1.0e-6);
}

#[v2k_test_support::retail_test]
fn factory_machine_gun_callback_selects_the_larger_authored_pair() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("normal-tier L3");
    let (player_id, _) = session.cache.global_model_by_name("player4").unwrap();
    let mut craft = PlayerCraft::new();
    craft.select_weapon_callback(15);
    let vars = craft.anim_vars();
    let tips = linked_model_named_tips(
        &session.cache,
        ModelTreeNamedTipRequest {
            model_id: player_id,
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: [0.0; 3],
            scale: 100.0 / 256.0,
            depth: 3,
            linked: None,
            vars: &vars,
            child_transform: None,
            model_name: "pl4biggatgun",
            local_tip_axis: [0.0, 0.0, 1.0],
        },
    );
    assert_eq!(tips.len(), 2);
    for (tip, raw) in tips
        .iter()
        .zip([[136.0, 20.0, 140.0], [-136.0, 20.0, 140.0]])
    {
        assert_eq!(tip.direction_unit, [0.0, 0.0, 1.0]);
        for axis in 0..3 {
            assert!((tip.origin_world[axis] - raw[axis] / 256.0).abs() < 1.0e-6);
        }
    }
}

#[v2k_test_support::retail_test]
fn player4_plasma_guns_have_two_distinct_authored_tips() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("normal-tier L3");
    let (player_id, player) = session
        .cache
        .global_model_by_name("player4")
        .expect("player4");

    for (callback, name, model_id) in [
        (6, "pl4plasmared", 46usize),
        (7, "pl4plasmagreen", 47usize),
        (8, "pl4plasmablue", 48usize),
    ] {
        let mut craft = PlayerCraft::new();
        craft.select_weapon_callback(callback);
        let vars = craft.anim_vars();
        let materialized = player.materialize(&vars);
        let guns = materialized
            .instances
            .iter()
            .filter(|instance| {
                session
                    .cache
                    .global_model(instance.model_id as usize)
                    .and_then(|entry| entry.name.as_deref())
                    == Some(name)
            })
            .count();
        assert_eq!(guns, 2, "{name}: two plasma mounts");

        let gun_model = session.cache.global_model(model_id).expect(name);
        assert_eq!(
            gun_model.records.get(21..23),
            Some(&[[14, 0, 0, 0], [14, 1, 0, 0]][..]),
            "{name}: A/B type-14 muzzle callbacks"
        );

        let tips = linked_model_named_tips(
            &session.cache,
            ModelTreeNamedTipRequest {
                model_id: player_id,
                orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                position: [0.0; 3],
                scale: 100.0 / 256.0,
                depth: 3,
                linked: None,
                vars: &vars,
                child_transform: None,
                model_name: name,
                local_tip_axis: [0.0, 0.0, 1.0],
            },
        );
        assert_eq!(tips.len(), 2, "{name}: two +Z tips");
        for (tip, raw) in tips
            .iter()
            .zip([[140.0, 24.0, 120.0], [-140.0, 24.0, 120.0]])
        {
            assert_eq!(tip.direction_unit, [0.0, 0.0, 1.0], "{name}");
            for axis in 0..3 {
                assert!(
                    (tip.origin_world[axis] - raw[axis] / 256.0).abs() < 1.0e-6,
                    "{name} axis {axis}"
                );
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn medaeval_preserves_spawn_behavior_payloads() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session.load_level_by_id(14, 0).expect("load Medaeval");
    let level = session.cache.level_desc().expect("Section 13");

    assert_eq!(level.entities.len(), 45);
    assert_eq!(
        level
            .entities
            .iter()
            .filter(|spawn| spawn.has_animation)
            .count(),
        1
    );
    assert_eq!(
        level
            .entities
            .iter()
            .filter(|spawn| spawn.has_config)
            .count(),
        2
    );
    assert!(level
        .entities
        .iter()
        .filter(|spawn| spawn.has_animation)
        .all(|spawn| spawn.animation.is_some()));
    assert!(level
        .entities
        .iter()
        .filter(|spawn| spawn.has_config)
        .all(|spawn| spawn.config.is_some()));
    assert!(level
        .entities
        .iter()
        .any(|spawn| spawn.extra.iter().any(|&byte| byte != 0)));
    let main_base = level
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 6)
        .expect("type-6 Main Base spawn");
    assert_eq!(main_base.rotation, [0x3555, 0, 0]);
}

#[v2k_test_support::retail_test]
fn section12_type_models_and_section13_spawn_position_match_runtime_layout() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 0).expect("load L3 aux");

    let table = session.cache.global_entity_model_table();
    assert_eq!(table.len(), 130, "L2 types 0-1 + L3 types 2-129");
    assert_eq!(table[0], [1; 4], "menu backdrop type -> klaus");
    assert_eq!(table[1], [145; 4], "menu vehicle type -> flag");
    assert_eq!(table[6], [286, 225, 286, 225], "Main Base type slots");
    assert_eq!(table[46], [41, 67, 41, 67], "player4 craft type slots");

    let player_type = session
        .cache
        .global_entity_type(46)
        .expect("type-46 Section-12 record");
    assert_eq!(player_type.scale as u16, 100, "player self mass");
    assert_eq!(player_type.id_field, 5, "player runtime capabilities");
    assert_eq!(
        DamageProfile::from_section12_header(&player_type.raw_header),
        Some(TYPE_46_DAMAGE_PROFILE),
        "all seven retail player damage channels"
    );
    assert_eq!(
        &player_type.raw_header[0x88..0x90],
        &[0; 8],
        "type 46 authors no static or active solid-contact sounds"
    );
    assert_eq!(
        player_type
            .subsections
            .iter()
            .find(|subsection| subsection.name == "J")
            .and_then(|subsection| subsection.data.first())
            .copied(),
        Some(5),
        "player Sub-J attachment maximum"
    );
    let weight_type = session
        .cache
        .global_entity_type(68)
        .expect("type-68 Section-12 record");
    assert_eq!(weight_type.scale as u16, 200, "weight cargo mass");
    assert_eq!(weight_type.id_field, 0x1040, "weight runtime capabilities");
    let solid_first_world_profile = DamageProfile {
        thresholds_raw: [0, 2_000, 200, 0, 200, 0, 0],
        multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
    };
    for (entity_type, expected_health, expected_mass) in
        [(6usize, 99_999u32, 1_000u16), (66, 99_999, 1_000)]
    {
        let record = session
            .cache
            .global_entity_type(entity_type)
            .expect("first-world solid entity type");
        assert_eq!(record.dims[0], expected_health);
        assert_eq!(record.scale as u16, expected_mass);
        assert_eq!(
            record.accepted_hit_presentation_sound_id(),
            Some(7),
            "Main Base and Working Factory share the authored damage cue"
        );
        assert_eq!(
            DamageProfile::from_section12_header(&record.raw_header),
            Some(solid_first_world_profile)
        );
    }
    assert_eq!(weight_type.dims[0], 50_000);
    assert_eq!(
        DamageProfile::from_section12_header(&weight_type.raw_header),
        Some(DamageProfile {
            thresholds_raw: [0, 2_000, 200, 0, 200, 0, 0],
            multipliers_q8: [0; 7],
        }),
        "the loose weight type is collision-damage immune even though it has health"
    );

    let type_metadata: Vec<_> = table
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();
    session.load_level_by_id(13, 0).expect("load level 13");
    let level = session.cache.level_desc().expect("Section 13");
    let main_base_spawn = level
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 6)
        .expect("type-6 Main Base spawn");
    assert_eq!(main_base_spawn.model_overrides[0], 0);
    let terrain = session.cache.terrain().expect("first-world terrain");
    let mut manager = v2k_game::entity::EntityManager::from_level_with_type_metadata(
        level,
        &type_metadata,
        Some(terrain),
    );
    assert_eq!(
        manager.iter().count(),
        36,
        "35 world records + persistent player"
    );
    // FUN_004104B0 tail-appends after successful initialization. The retail
    // capture has pair-ineligible persistent types 1/0 before the player and
    // type 111 before these Section-13 allocations; removing those unported
    // nodes leaves this exact player-plus-authored order.
    assert_eq!(
        manager
            .retail_live_order_ids()
            .zip(manager.iter_all())
            .map(|(id, entity)| (id, entity.entity_type))
            .collect::<Vec<_>>(),
        [
            46, 52, 52, 52, 52, 62, 68, 6, 52, 54, 9, 9, 47, 47, 47, 9, 9, 9, 17, 17, 17, 17, 52,
            9, 66, 67, 62, 62, 52, 52, 52, 52, 52, 61, 61, 61,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, entity_type)| (index as u32 + 1, entity_type))
        .collect::<Vec<_>>()
    );
    assert_eq!(
        manager
            .iter()
            .filter(|entity| entity.entity_type == 61)
            .map(|entity| (entity.authored_spawn_index, entity.power_up_payload_packed,))
            .collect::<Vec<_>>(),
        vec![
            (Some(32), Some(0x0000_003c)),
            (Some(33), Some(0x0000_003f)),
            (Some(34), Some(0x0000_c802)),
        ],
        "Power Up copies spawn +0x1C to runtime +0x88"
    );
    let player = manager.player().expect("persistent type-46 player");
    assert_eq!(player.entity_type, 46);
    assert_eq!(player.position, [77.0, -2.109375, 58.0]);
    assert_eq!(player.model_index, Some(41));
    assert_eq!(player.mass_raw, 100);
    assert_eq!(player.capability_flags, 5);
    assert_eq!(
        player.collision.active_model_slot(),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        player.collision.health_raw,
        RetailRuntimeValue::Known(40_000)
    );
    assert_eq!(
        player.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        player.collision.damage_profile,
        RetailRuntimeValue::Known(TYPE_46_DAMAGE_PROFILE)
    );
    assert_eq!(
        player.collision.state_flags_at_0x08.known_mask(),
        !0x0060_0000,
        "only creation-time surface classification remains unknown"
    );
    assert_eq!(
        player.collision.state_flags_at_0x08.known_value_bits(),
        0x0607_8805,
        "the later level-handoff clear of 0x40000 is not constructor policy"
    );
    assert_eq!(
        player.collision.subject_scan_gate_at_0x70,
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        player.initial_behavior.as_known_behavior_class_for_test(),
        Some(24)
    );
    assert_eq!(
        player
            .initial_behavior
            .known_style_for_test()
            .release_callback_policy(),
        ReleaseCallbackPolicy::None
    );
    assert!((player.heading - std::f32::consts::FRAC_PI_2).abs() < 0.000_001);
    let main_base = manager
        .iter()
        .find(|entity| entity.entity_type == 6)
        .expect("Main Base entity");
    assert_eq!(main_base.position, [80.0, -3.0, 60.0]);
    assert_eq!(main_base.model_index, Some(286));
    assert_eq!(
        main_base.collision.health_raw,
        RetailRuntimeValue::Known(99_999)
    );
    assert_eq!(
        main_base.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(main_base_spawn.initial_damage_buffer_raw)
    );
    assert_eq!(
        main_base.model_slots,
        [Some(286), Some(225), Some(286), Some(225)]
    );
    assert_eq!(
        main_base
            .initial_behavior
            .as_known_behavior_class_for_test(),
        Some(41)
    );
    assert_eq!(
        (
            main_base.collision.state_flags_at_0x08.known_mask(),
            main_base.collision.state_flags_at_0x08.known_value_bits(),
        ),
        (!0x0060_0000, 0x0e02_8805)
    );
    let RetailRuntimeValue::Known(Some(main_base_runtime)) = main_base.base_factory_runtime else {
        panic!("Main Base Sub-M runtime");
    };
    assert_eq!(
        main_base_runtime.production, None,
        "type 6 shares Sub-M status but not Working Factory production"
    );
    let factory = manager
        .iter()
        .find(|entity| entity.entity_type == 66)
        .expect("factory entity");
    assert_eq!(
        factory.initial_behavior.as_known_behavior_class_for_test(),
        Some(39)
    );
    assert_eq!(
        (
            factory.collision.state_flags_at_0x08.known_mask(),
            factory.collision.state_flags_at_0x08.known_value_bits(),
        ),
        (!0x0060_0000, 0x0e02_8805)
    );
    let weight = manager
        .iter()
        .find(|entity| entity.entity_type == 68)
        .expect("weight entity");
    assert_eq!(
        weight.initial_behavior.as_known_behavior_class_for_test(),
        Some(0),
        "class zero is the authored None behavior, not an absent behavior"
    );
    assert_eq!(
        weight
            .initial_behavior
            .known_style_for_test()
            .release_callback_policy(),
        ReleaseCallbackPolicy::TerrainAlignAndReselect
    );
    let job_marker = manager
        .iter()
        .find(|entity| entity.entity_type == 54)
        .expect("type-54 entity");
    assert_eq!(
        job_marker
            .initial_behavior
            .as_known_behavior_class_for_test(),
        Some(0)
    );
    assert_eq!(
        job_marker
            .initial_behavior
            .known_style_for_test()
            .release_callback_policy(),
        ReleaseCallbackPolicy::TerrainAlignAndReselect
    );
    assert_eq!(
        (
            weight.collision.state_flags_at_0x08.known_mask(),
            weight.collision.state_flags_at_0x08.known_value_bits(),
        ),
        (!0x0060_0000, 0x0e00_8805)
    );
    for weighted_type in [9, 17, 47] {
        assert!(
            manager
                .iter()
                .filter(|entity| entity.entity_type == weighted_type)
                .all(|entity| entity.initial_behavior == RetailRuntimeValue::Unresolved),
            "type {weighted_type} still depends on evaluators or the retail RNG draw"
        );
    }
    assert!((main_base.heading - 11468.0 * std::f32::consts::TAU / 65536.0).abs() < 0.000_001);
    assert_eq!(
        session
            .cache
            .global_model(286)
            .and_then(|model| model.name.as_deref()),
        Some("college")
    );
    assert_eq!(
        session
            .cache
            .global_model_by_name("player4")
            .map(|(id, _)| id),
        Some(41)
    );
    assert_eq!(
        session
            .cache
            .global_model(41)
            .and_then(|model| model.name.as_deref()),
        Some("player4")
    );

    // Live retail snapshot immediately after Intro2: the first playable world
    // is 0X13XX, not Medaeval/0X14XX. These three records account for the
    // starting-island weight and working factory; type 104 is absent.
    for (entity_type, position, model_id, model_name) in [
        // Authored Y is -3.0; the audited birth policy places this exact
        // allocation at retail's stable terrain height before its first frame.
        (68, [75.0, -3.25, 60.0], 81, "weight"),
        (66, [87.0, -3.0, 58.0], 227, "lifter"),
        (67, [-70.0, -1.0, -128.0], 341, "hive1xa"),
    ] {
        let entity = manager
            .iter()
            .find(|entity| entity.entity_type == entity_type)
            .expect("retail first-world entity");
        assert_eq!(entity.position, position);
        assert_eq!(entity.model_index, Some(model_id));
        if entity_type == 68 {
            assert_eq!(entity.mass_raw, 200);
            assert_eq!(entity.capability_flags, 0x1040);
        }
        assert_eq!(
            session
                .cache
                .global_model(model_id)
                .and_then(|model| model.name.as_deref()),
            Some(model_name)
        );
    }
    let factory = manager
        .iter()
        .find(|entity| entity.entity_type == 66)
        .expect("Working Factory entity");
    assert_eq!(factory.heading, 0.0);
    assert_eq!(
        factory.model_slots,
        [Some(227), Some(225), Some(227), Some(225)]
    );
    let RetailRuntimeValue::Known(Some(factory_presentation)) = factory.base_factory_runtime else {
        panic!("type 66 Sub-M runtime state");
    };
    assert_eq!(factory_presentation.required_scientists, 2);
    assert_eq!(factory_presentation.current_scientists, 0);
    assert_eq!(factory_presentation.lifter_progress_raw, 0);
    assert_eq!(factory_presentation.production_progress_raw, 0);
    let production = factory_presentation
        .production
        .expect("Level-1 Working Factory production constructor state");
    assert_eq!(production.output_payload_packed, 0x0001_f412);
    assert_eq!(production.scientist_capacity_raw, 2);
    assert_eq!(production.production_threshold_micros_raw, 6_000_000);
    assert_eq!(production.delivery_duration_micros_raw, 5_000_000);
    assert_eq!(production.cooldown_duration_micros_raw, -1_000);
    assert_eq!(production.remaining_stock_raw, 1);
    assert_eq!(production.cached_health_raw, 99_999);
    assert_eq!(production.health_per_scientist_raw, 49_999);
    assert_eq!(production.repair_rate_raw, 833);
    assert_eq!(production.phase, FactoryProductionPhase::Producing);

    let vars = factory.presentation_anim_vars(0);
    assert_eq!(vars.dynamic[..5], [0, 0, 0, 2, 0]);
    let lifter = session
        .cache
        .global_model(227)
        .expect("working-factory lifter model");
    let lifter_materialized = lifter.materialize(&vars);
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let lifter_arm = lifter_materialized
        .instances
        .iter()
        .find(|instance| instance.model_id == 228)
        .expect("animated lifterarm child");
    assert_ne!(
        lifter_arm.orientation, identity,
        "the live channel-1 mount still animates the lifter arm"
    );
    let factpole_child = lifter_materialized
        .instances
        .iter()
        .find(|instance| instance.model_id == 179)
        .expect("factpole child after the authored mount reset");
    assert_eq!(
        factpole_child.orientation, identity,
        "opcode 0x0F keeps the empty factory pole world-upright"
    );
    let (_, factpole) = session
        .cache
        .global_model_by_name("factpole")
        .expect("factory scientist indicator model");
    let green_light_count = |vars: &v2k_formats::models::AnimVars| {
        factpole
            .materialize(vars)
            .billboards
            .iter()
            .filter(|billboard| billboard.textured && billboard.id == 606)
            .count()
    };
    assert_eq!(
        green_light_count(&factory_presentation.anim_vars(0)),
        2,
        "all unfilled positions share the retail blink-on phase"
    );
    assert_eq!(
        green_light_count(&factory_presentation.anim_vars(1)),
        0,
        "an empty factory has no solid lights during blink-off"
    );
    let one_scientist = v2k_game::entity::BaseFactoryRuntimeState {
        current_scientists: 1,
        ..factory_presentation
    };
    assert_eq!(
        green_light_count(&one_scientist.anim_vars(1)),
        1,
        "the occupied bottom position remains solid during blink-off"
    );

    for entity_type in [6, 66] {
        let target_id = manager
            .iter()
            .find(|entity| entity.entity_type == entity_type)
            .expect("audited first-world damage target")
            .id;
        let CheckedProjectileDamageOutcome::Applied(applied) = manager
            .apply_audited_base_factory_projectile_damage(
                target_id,
                CheckedProjectileDamageRequest {
                    delivery: DamageDeliveryRecord {
                        packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                        source_entity_type_raw: 46,
                        owner_handle: manager.player().expect("player projectile owner").id,
                    },
                    entry: EntityHitEntry::PrimaryProjectile,
                    retail_tick: 11,
                },
            )
        else {
            panic!("real first-world type {entity_type} must accept a surviving primary hit");
        };
        assert_eq!(applied.transition.accepted_damage_raw, 1_800);
        assert_eq!(applied.transition.generic_hit_sound_id, None);
        assert_eq!(applied.accepted_hit_presentation.unwrap().sound_id, Some(7));
        assert_eq!(
            manager
                .iter()
                .find(|entity| entity.id == target_id)
                .expect("surviving target remains live")
                .collision
                .health_raw,
            RetailRuntimeValue::Known(98_199)
        );
    }
    assert!(manager.iter().all(|entity| entity.entity_type != 104));
}

#[v2k_test_support::retail_test]
fn medaeval_zero_y_buildings_settle_while_authored_airborne_y_survives() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 0).expect("load L3 aux");
    let table = session.cache.global_entity_model_table();
    session.load_level_by_id(14, 0).expect("load Medaeval");
    let level = session.cache.level_desc().expect("Section 13");
    let factory_spawn = level
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 66)
        .expect("factory spawn record");
    assert_eq!(factory_spawn.model_overrides, [218, 225, 218, 225]);
    let terrain = session.cache.terrain().expect("Section 10");
    let manager = v2k_game::entity::EntityManager::from_level(level, &table, Some(terrain));

    let factory = manager
        .iter()
        .find(|entity| entity.entity_type == 66)
        .expect("factory3 spawn");
    assert_eq!(
        factory.position[1],
        terrain.height_at(factory.position[0], factory.position[2])
    );
    assert_eq!(factory.model_index, Some(218));
    assert_eq!(
        session
            .cache
            .global_model(factory.model_index.unwrap())
            .and_then(|model| model.name.as_deref()),
        Some("factory3")
    );
    let powerup = manager
        .iter()
        .find(|entity| entity.entity_type == 61)
        .expect("powerup spawn");
    assert_eq!(powerup.position[1], -14.0);
}

trait KnownBehaviorClassForTest {
    fn as_known_behavior_class_for_test(&self) -> Option<u8>;
}

impl KnownBehaviorClassForTest
    for RetailRuntimeValue<Option<v2k_game::entity_behavior::BehaviorSelection>>
{
    fn as_known_behavior_class_for_test(&self) -> Option<u8> {
        match self {
            RetailRuntimeValue::Known(Some(selection)) => Some(selection.program.class_id),
            other => panic!("expected one known initial behavior, got {other:?}"),
        }
    }
}

trait KnownBehaviorStyleForTest {
    fn known_style_for_test(&self) -> BehaviorStyle;
}

impl KnownBehaviorStyleForTest for RetailRuntimeValue<Option<BehaviorSelection>> {
    fn known_style_for_test(&self) -> BehaviorStyle {
        match self {
            RetailRuntimeValue::Known(Some(selection)) => selection.program.initial_style,
            other => panic!("expected one known initial behavior, got {other:?}"),
        }
    }
}

fn make_player() -> v2k_game::entity::Entity {
    let mut player = v2k_game::entity::Entity::unresolved_port_entity(
        0,
        v2k_game::entity::EntityKind::Player,
        6,
    );
    player.model_slots = [Some(0); 4];
    player.model_index = Some(0);
    player
}
