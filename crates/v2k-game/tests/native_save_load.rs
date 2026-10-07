//! Corpus-backed acceptance for native player-first save reconstruction.

use v2k_formats::saves::{GameState as NativeGameState, NativePlayerSnapshot};
use v2k_game::entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::player_hull::{HullDamageProfile, PlayerHull};
use v2k_game::save::{NativeCompatibilityPreview, NativeSaveRestore, SavedPlayerState};
use v2k_game::session::GameSession;
use v2k_game::world_fx::WorldFx;

const PLAYER_ENTITY_TYPE: usize = 46;

#[v2k_test_support::retail_test]
fn campaign_arrival_and_checkpoint_share_hull_shield_and_pose_before_authored_births() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(15, 1).unwrap();
    let profile =
        HullDamageProfile::from_type_record(session.cache.global_entity_type(46).unwrap());
    let mut hull = PlayerHull::new(profile);
    hull.health_raw = 12345;
    hull.pre_health_damage_buffer_raw = 80833;
    let mut craft = v2k_game::player::PlayerCraft::new();
    craft.mode = v2k_game::player::VehicleMode::Vtol;
    craft.fuel_raw = 99119;
    craft.restore_body_angle_words([-1234, 2345]);
    let inventory = v2k_game::weapon_inventory::WeaponInventory::new();
    let capabilities = v2k_game::weapon_inventory::PlayerCapabilities::default();
    let cargo = v2k_game::entity::CampaignCargoControllerState::from_packed(1, []);
    let campaign = v2k_game::power_up_contact::PlayerCampaignProgress::new();
    let arrival = v2k_game::campaign_transition::WarpArrival {
        position_raw: [0x2b00, 0x0a00, 0x6000],
        heading_raw: 0x4000,
    };
    let checkpoint = v2k_game::save::NativeSaveSnapshot {
        logical_level_id: 3,
        player: SavedPlayerState {
            position_raw: [1, 2, 3],
            velocity_raw: [7, 8, 9],
            heading_raw: 123,
            pitch_raw: 0,
            roll_raw: 0,
            health_raw: hull.health_raw,
        },
        craft: &craft,
        hull: &hull,
        inventory: &inventory,
        capabilities: &capabilities,
        cargo: &cargo,
        campaign: &campaign,
    }
    .encode_campaign_arrival(&[0; 0x248], arrival, "Castle")
    .unwrap();
    let restore = NativeSaveRestore::decode(&checkpoint).unwrap();
    let metadata = global_type_metadata(&session);
    let mut terrain = session.cache.level_terrain().unwrap().clone();
    let extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let mut fx = WorldFx::new();
    let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
    let entities = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 3,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&extent),
            },
            player_arrival: None,
            retail_tick: 148,
        }
        .with_native_save(v2k_game::entity::NativeSaveWorldState {
            restore: &restore,
            terrain: &mut terrain,
            resources: &session.cache,
            static_damage: &mut static_damage,
        }),
        &mut fx,
    )
    .unwrap();
    let player = entities.player().unwrap();
    assert_eq!(player.position_raw(), arrival.position_raw);
    assert_eq!(player.velocity_raw(), [0; 3]);
    assert_eq!(player.rotation_heading_pitch_roll_raw(), [0x4000, 0, 0]);
    let direct = hull.campaign_world_replacement(profile);
    assert_eq!(direct.health_raw, 40000);
    assert_eq!(direct.pre_health_damage_buffer_raw, 80833);
    assert_eq!(
        player.collision.health_raw,
        RetailRuntimeValue::Known(direct.health_raw)
    );
    assert_eq!(
        player.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(direct.pre_health_damage_buffer_raw)
    );
    let mut destination_craft = craft.campaign_world_replacement(Default::default(), 0);
    restore.restore_craft(&mut destination_craft);
    assert_eq!(destination_craft.mode, v2k_game::player::VehicleMode::Vtol);
    assert_eq!(destination_craft.fuel_raw, 99119);
    assert_eq!(destination_craft.body_angle_words(), [0, 0]);
    assert_eq!(entities.retail_live_order_ids().next(), Some(player.id));
}

#[v2k_test_support::retail_test]
fn completed_hive_constructor_damage_survives_final_hull_readback() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(14, 1).unwrap();
    let metadata = global_type_metadata(&session);
    let profile =
        HullDamageProfile::from_type_record(session.cache.global_entity_type(46).unwrap());
    let damage = v2k_game::hive_death::HIVE_DEATH_RADIAL_TEMPLATE_BASE
        .packet
        .filtered_raw(Some(&profile.damage));
    assert_eq!(damage, 7_900);
    // World 2's native Hive stands at [-15872, -512, 27904]. Put the
    // restored player 256 raw units away, inside the full-damage radius.
    let position_raw = [-15_616_i16, -512, 27_904];
    let velocity_raw = [7_i16, -8, 9];
    for initial_health in [40_000_i32, 1] {
        let mut payload = [0; 0x248];
        payload[0x20..0x24].copy_from_slice(&2_u32.to_le_bytes());
        for (offset, words) in [(0x24, position_raw), (0x2a, velocity_raw)] {
            for (axis, word) in words.into_iter().enumerate() {
                payload[offset + axis * 2..offset + axis * 2 + 2]
                    .copy_from_slice(&word.to_le_bytes());
            }
        }
        payload[0x3c..0x40].copy_from_slice(&initial_health.to_le_bytes());
        payload[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
        payload[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
        payload[0x16c..0x170].copy_from_slice(&100_i32.to_le_bytes());
        payload[0x1bc..0x1c0].copy_from_slice(&0x42f_u32.to_le_bytes());
        let restore = NativeSaveRestore::decode(&NativeCompatibilityPreview {
            logical_level_id: 2,
            state_payload: payload,
            saved_hint_mask: None,
        })
        .unwrap();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut terrain = session.cache.level_terrain().unwrap().clone();
        let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
        let mut fx = WorldFx::new();
        let entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 2,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: None,
                retail_tick: 148,
            }
            .with_native_save(v2k_game::entity::NativeSaveWorldState {
                restore: &restore,
                terrain: &mut terrain,
                resources: &session.cache,
                static_damage: &mut static_damage,
            }),
            &mut fx,
        )
        .unwrap();
        let player = entities.player().unwrap();
        assert_eq!(player.position_raw(), position_raw);
        assert_ne!(player.velocity_raw(), velocity_raw);
        assert_ne!(player.velocity_raw(), [0; 3]);
        let mut hull = PlayerHull::new(profile);
        restore.restore_hull(&mut hull);
        hull.readback_entity_damage_state(&player.collision);
        assert_eq!(hull.health_raw, (initial_health - (damage - 100)).max(0));
        assert_eq!(hull.pre_health_damage_buffer_raw, 0);
        assert_eq!(hull.dying, initial_health == 1);
        assert_eq!(
            player
                .collision
                .state_flags_at_0x08
                .masked(v2k_game::entity_collision_state::DYING_STATE_BIT),
            RetailRuntimeValue::Known(if hull.dying {
                v2k_game::entity_collision_state::DYING_STATE_BIT
            } else {
                0
            })
        );
    }
}

fn native_state(logical_level_id: u32) -> NativeGameState {
    NativeGameState {
        display_name: "Synthetic mapping probe".into(),
        logical_level_id,
        player: NativePlayerSnapshot {
            position_raw: [0; 3],
            velocity_raw: [0; 3],
            heading_raw: 0,
            pitch_raw: 0,
            roll_raw: 0,
            health_raw: 40_000,
        },
        field_38: 0,
        field_40: 0,
        field_44: 0,
        field_48: 0,
        field_4c: 0,
        field_50: 0,
        field_54: 0,
        field_60: 0,
        field_64: 0,
        field_68: 0,
    }
}

fn global_type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
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
        .collect()
}

#[v2k_test_support::retail_test]
fn native_save_worlds_publish_restored_player_before_authored_allocations() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load high-resolution common type metadata");
    let hull_profile = session
        .cache
        .global_entity_type(PLAYER_ENTITY_TYPE)
        .map(HullDamageProfile::from_type_record)
        .expect("retail type-46 hull profile");

    let cases = [
        (
            1,
            13,
            SavedPlayerState {
                position_raw: [19_713, -501, 14_849],
                velocity_raw: [7, -8, 9],
                heading_raw: 0x4123,
                pitch_raw: 0x0123,
                roll_raw: 0xffed,
                health_raw: 35_111,
            },
        ),
        (
            2,
            14,
            SavedPlayerState {
                position_raw: [10_752, -512, -13_568],
                velocity_raw: [-17, 23, 31],
                heading_raw: 0x4000,
                pitch_raw: 0xffa1,
                roll_raw: 0x0112,
                health_raw: 39_001,
            },
        ),
        (
            3,
            15,
            SavedPlayerState {
                position_raw: [11_008, 2_560, 24_576],
                velocity_raw: [101, -202, 303],
                heading_raw: 0x1234,
                pitch_raw: 0,
                roll_raw: 0,
                health_raw: 38_002,
            },
        ),
        (
            5,
            17,
            SavedPlayerState {
                position_raw: [-13_312, 5_120, -15_616],
                velocity_raw: [i16::MIN, 0, i16::MAX],
                heading_raw: 0xC321,
                pitch_raw: 0,
                roll_raw: 0,
                health_raw: 37_003,
            },
        ),
        (
            18,
            30,
            SavedPlayerState {
                position_raw: [7_424, 5_120, 9_728],
                velocity_raw: [-1, 2, -3],
                heading_raw: 0xFFFF,
                pitch_raw: 0,
                roll_raw: 0,
                health_raw: 36_004,
            },
        ),
    ];

    for (logical_level_id, expected_global_level_id, saved) in cases {
        assert_eq!(
            native_state(logical_level_id).saveable_global_level_id(),
            Some(expected_global_level_id),
            "native logical level {logical_level_id}"
        );

        session
            .load_level_by_id(expected_global_level_id, 1)
            .unwrap_or_else(|error| {
                panic!("load global level {expected_global_level_id}: {error}")
            });
        let type_metadata = global_type_metadata(&session);
        let level = session
            .cache
            .level_desc()
            .unwrap_or_else(|| panic!("global level {expected_global_level_id} lacks Section 13"));
        let terrain = session.cache.terrain();
        let mut payload = [0; 0x248];
        payload[0x20..0x24].copy_from_slice(&logical_level_id.to_le_bytes());
        for (offset, words) in [(0x24, saved.position_raw), (0x2a, saved.velocity_raw)] {
            for (axis, word) in words.into_iter().enumerate() {
                payload[offset + axis * 2..offset + axis * 2 + 2]
                    .copy_from_slice(&word.to_le_bytes());
            }
        }
        for (offset, word) in [
            (0x30, saved.heading_raw),
            (0x32, saved.pitch_raw),
            (0x34, saved.roll_raw),
        ] {
            payload[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
        }
        payload[0x3c..0x40].copy_from_slice(&saved.health_raw.to_le_bytes());
        payload[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
        payload[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
        payload[0x149] = 3;
        payload[0x16c..0x170].copy_from_slice(&7_531_i32.to_le_bytes());
        if logical_level_id == 2 {
            // NoCD Slot00 oracle: current world complete, hidden trophy claimed.
            payload[0x1bc..0x1c0].copy_from_slice(&0x42f_u32.to_le_bytes());
        }
        let restore = NativeSaveRestore::decode(&NativeCompatibilityPreview {
            logical_level_id,
            state_payload: payload,
            saved_hint_mask: None,
        })
        .expect("native controller profile");
        let mut fx = WorldFx::new();
        let mut native_terrain = session.cache.level_terrain().unwrap().clone();
        let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level,
                logical_world_index: logical_level_id as i32,
                type_metadata: &type_metadata,
                resources: EntityConstructionResources {
                    terrain,
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: None,
                retail_tick: 4793,
            }
            .with_native_save(v2k_game::entity::NativeSaveWorldState {
                restore: &restore,
                terrain: &mut native_terrain,
                resources: &session.cache,
                static_damage: &mut static_damage,
            }),
            &mut fx,
        )
        .unwrap_or_else(|error| panic!("world{expected_global_level_id}: {error:?}"));

        let player = entities.player().expect("restored persistent player");
        assert_eq!(player.position_raw(), saved.position_raw);
        assert_eq!(player.velocity_raw(), saved.velocity_raw);
        assert_eq!(player.heading_raw(), saved.heading_raw);
        assert_eq!(
            [
                player.rotation_heading_pitch_roll_raw()[1],
                player.rotation_heading_pitch_roll_raw()[2]
            ],
            [saved.pitch_raw as i16, saved.roll_raw as i16]
        );
        assert_eq!(
            player.collision.health_raw,
            RetailRuntimeValue::Known(saved.health_raw)
        );
        assert_eq!(
            player.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(7_531)
        );
        assert_eq!(player.authored_spawn_index, None);
        assert_eq!(
            entities.retail_live_order_ids().next(),
            Some(player.id),
            "451AF0 restores the player before42E570 publishes authored entries"
        );
        assert_eq!(
            player.construction_stamp_at_0xb4,
            RetailRuntimeValue::Known((logical_level_id << 10) as u16)
        );
        assert_eq!(entities.player_cargo_unlock_raw(), 3);
        if logical_level_id == 2 {
            let pending = entities.pending_actor_deferred_destroy_ids();
            let removed_births: Vec<_> = entities
                .iter_all()
                .filter(|entity| pending.contains(&entity.id))
                .filter_map(|entity| entity.authored_spawn_index)
                .collect();
            assert_eq!(removed_births, [28, 29, 30, 31, 32, 33, 34, 35, 36, 42]);
            let hive = entities
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(38))
                .unwrap();
            assert_eq!(hive.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                hive.collision.active_model_slot(),
                RetailRuntimeValue::Known(3)
            );
            assert!(hive.authored_radial_emitter.as_ref().unwrap().dying_slot0());
            assert!(
                matches!(hive.sub_n_runtime, RetailRuntimeValue::Known(Some(sub_n))
                    if sub_n.terrain_patch_present() == RetailRuntimeValue::Known(true)),
                "loaded completed Hive retains its authored marker"
            );
            let factory = entities
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(39))
                .unwrap();
            let RetailRuntimeValue::Known(Some(base)) = factory.base_factory_runtime else {
                panic!("native factory Sub-M");
            };
            assert_eq!(base.current_scientists, 3);
            assert_eq!(base.production.unwrap().current_scientists_raw, 3);
            assert_ne!(native_terrain.cell(195, 110).unwrap().terrain_type & 8, 0);
            assert_eq!(
                session
                    .cache
                    .level_terrain()
                    .unwrap()
                    .cell(195, 110)
                    .unwrap()
                    .terrain_type
                    & 8,
                0,
                "working terrain commits only after load succeeds"
            );
        }

        let mut hull = PlayerHull::new(hull_profile);
        restore.restore_hull(&mut hull);
        assert_eq!(hull.pre_health_damage_buffer_raw, 7_531);
        assert!(saved.health_raw > 0);
        assert!(entities.sync_player_hull_collision_state(&hull));
        assert_eq!(
            entities
                .player()
                .expect("health-synchronized player")
                .collision
                .health_raw,
            RetailRuntimeValue::Known(saved.health_raw)
        );
    }
}
