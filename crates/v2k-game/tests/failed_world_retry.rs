//! Corpus-backed failed-Hive retry reconstruction (`456D10 -> 42DE20`).
//!
//! These controls start at the admitted retry request. Abort/suction dispatch
//! is a separate production control; no authored exit marker is fabricated.

use v2k_game::{
    campaign_transition::{CampaignTransition, FailedWorldRetry},
    entity::{
        AuthoredWorldConstruction, CampaignCargoControllerState, CampaignCargoRestoreContext,
        EntityConstructionResources, EntityManager, NativeSaveWorldState,
        PlayerCargoAttachmentFrame,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    player::{PlayerCraft, VehicleMode},
    player_hull::{HullDamageProfile, PlayerHull},
    power_up_contact::{PlayerCampaignProgress, RETAIL_CONTROL_SLOT_COUNT},
    save::{NativeCompatibilityPreview, NativeSaveRestore, NativeSaveSnapshot, SavedPlayerState},
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    weapon_inventory::{PlayerCapabilities, PowerUpPayload, WeaponAcquisition, WeaponInventory},
    world_complete_results::WorldCompleteResultsRuntime,
    world_fx::WorldFx,
};

struct RetryFixture {
    session: GameSession,
    entities: EntityManager,
    fx: WorldFx,
    craft: PlayerCraft,
    hull: PlayerHull,
    inventory: WeaponInventory,
    capabilities: PlayerCapabilities,
    campaign: PlayerCampaignProgress,
    cargo: CampaignCargoControllerState,
    retained_entry_raw: [i16; 3],
    map: WorldCompleteResultsRuntime,
    baseline: [u8; 0x248],
}

fn metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect()
}

impl RetryFixture {
    fn new() -> Self {
        let data = v2k_test_support::retail_dir();
        assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let types = metadata(&session);
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let mut fx = WorldFx::new();
        let mut entities = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: 1,
                type_metadata: &types,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&extent),
                },
                player_arrival: None,
                retail_tick: 0,
            },
            &mut fx,
        )
        .unwrap();
        let retained_entry_raw = entities.player().unwrap().position_raw();

        // Use an actual native authored Type68 and the same attachment owner
        // as collection, rather than inventing a saved type/stamp identity.
        entities.raise_player_cargo_capacity_to(3);
        let weight = entities
            .iter_all()
            .find(|entity| entity.entity_type == 68)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_class0_actors(&entities);
        entities
            .attach_player_cargo(
                weight,
                PlayerCargoAttachmentFrame {
                    scheduler: &mut scheduler,
                    world_fx: &mut fx,
                    notifications: &mut GameplayNotifications::new(),
                    retail_tick: 0,
                },
            )
            .unwrap();
        let RetailRuntimeValue::Known(cargo) = entities.campaign_cargo_controller_state() else {
            panic!("native cargo snapshot");
        };
        assert_eq!(cargo.occupied_slots(), 1);

        let wreck_pose = entities
            .iter_all()
            .find(|entity| entity.entity_type == 67)
            .unwrap()
            .position_raw();
        assert_ne!(wreck_pose, retained_entry_raw);
        let player = entities.player_mut().unwrap();
        player.set_motion_raw(wreck_pose, [731, -419, 1_234]);
        player.set_rotation_heading_pitch_roll_raw([-23_456, 789, -456]);

        let mut craft = PlayerCraft::new();
        craft.mode = VehicleMode::Vtol;
        craft.fuel_raw = 99_119;
        craft.restore_body_angle_words([-1_234, 2_345]);
        let mut hull = PlayerHull::new(HullDamageProfile::from_type_record(
            session.cache.global_entity_type(46).unwrap(),
        ));
        hull.health_raw = 12_345;
        hull.pre_health_damage_buffer_raw = 80_833;
        let mut inventory = WeaponInventory::new();
        for (selector, amount) in [(2, 87), (7, 23)] {
            assert!(matches!(
                inventory.acquire_weapon(PowerUpPayload { selector, amount }),
                WeaponAcquisition::Accepted { .. }
            ));
        }
        let capabilities = PlayerCapabilities::from_native_flags(3);
        // Current world remains unsaved with no route-pair/exit bits. Other
        // slots retain nonzero progress, including unknown native high bits.
        let bits = std::array::from_fn(|slot| {
            if slot == 1 {
                0x8000_0000
            } else {
                0x8000_0029 | ((slot as u32 & 7) << 9)
            }
        });
        let campaign = PlayerCampaignProgress::from_native_snapshot(1, bits, 4, 9);
        let mut baseline = [0xa5; 0x248];
        baseline[0x145] = 0x5a;
        baseline[0x147] = 0xf0;
        baseline[0x1b0..0x1b4].copy_from_slice(&0x29_u32.to_le_bytes());
        let mut map = WorldCompleteResultsRuntime::default();
        map.open_progress_map(CampaignTransition::FailedWorldRetry(
            FailedWorldRetry::new(13, retained_entry_raw).unwrap(),
        ));
        Self {
            session,
            entities,
            fx,
            craft,
            hull,
            inventory,
            capabilities,
            campaign,
            cargo,
            retained_entry_raw,
            map,
            baseline,
        }
    }

    fn map_checkpoint(&self) -> NativeCompatibilityPreview {
        let transition = self.map.progress_map_route().unwrap();
        let player = self.entities.player().unwrap();
        NativeSaveSnapshot {
            logical_level_id: transition.destination_logical_level(),
            player: SavedPlayerState {
                position_raw: player.position_raw(),
                velocity_raw: player.velocity_raw(),
                heading_raw: player.heading_raw(),
                pitch_raw: 789,
                roll_raw: (-456_i16) as u16,
                health_raw: self.hull.health_raw,
            },
            craft: &self.craft,
            hull: &self.hull,
            inventory: &self.inventory,
            capabilities: &self.capabilities,
            cargo: &self.cargo,
            campaign: &self.campaign,
        }
        .encode_campaign_arrival(&self.baseline, transition.arrival(), "Level1 retry")
        .unwrap()
    }
}

#[v2k_test_support::retail_test]
fn progress_map_save_retains_retry_entry_and_exact_controller_state_until_continue() {
    let mut fixture = RetryFixture::new();
    let transition = fixture.map.progress_map_route().unwrap();
    assert!(matches!(
        transition,
        CampaignTransition::FailedWorldRetry(_)
    ));
    assert_eq!(transition.source_level_id(), 13);
    assert_eq!(transition.destination_level_id(), 13);
    assert_eq!(transition.destination_logical_level(), 1);
    let wreck_pose = fixture.entities.player().unwrap().position_raw();
    let checkpoint = fixture.map_checkpoint();
    let restore = NativeSaveRestore::decode(&checkpoint).unwrap();
    assert_eq!(restore.logical_level_id, 1);
    assert_eq!(restore.player.position_raw, fixture.retained_entry_raw);
    assert_ne!(restore.player.position_raw, wreck_pose);
    assert_eq!(restore.player.velocity_raw, [0; 3]);
    assert_eq!(
        (
            restore.player.heading_raw,
            restore.player.pitch_raw,
            restore.player.roll_raw
        ),
        (0x4000, 0, 0)
    );
    assert_eq!(restore.player.health_raw, 40_000);
    assert_eq!(restore.pre_health_damage_buffer_raw, 80_833);
    assert_eq!(restore.inventory.slots(), fixture.inventory.slots());
    assert_eq!(
        restore.inventory.selected_slot(),
        fixture.inventory.selected_slot()
    );
    assert_eq!(restore.capabilities, fixture.capabilities);
    assert_eq!(restore.fuel_raw, fixture.craft.fuel_raw);
    assert_eq!(restore.mode, fixture.craft.mode);
    assert_eq!(restore.cargo.unlock_raw, fixture.cargo.unlock_raw);
    assert_eq!(
        &restore.cargo.carried_identities()[..fixture.cargo.carried_identities().len()],
        fixture.cargo.carried_identities()
    );
    assert_eq!(restore.controller_195_raw, 0x5a);
    assert_eq!(restore.session_flags_raw, 0x29);
    assert_eq!(checkpoint.state_payload[0x147], 0xf3);
    assert_eq!(restore.campaign, fixture.campaign);
    for slot in 0..RETAIL_CONTROL_SLOT_COUNT {
        assert_eq!(
            restore.campaign.control_slot_bits(slot),
            fixture.campaign.control_slot_bits(slot),
            "retry must not write saved, route-pair, exit or trophy bits in slot {slot}"
        );
    }
    assert_eq!(restore.campaign.control_slot_world_saved(1), Some(false));
    assert_eq!(restore.campaign.current_control_slot(), Some(1));
    assert!(fixture.map.is_progress_map_active());
    assert_eq!(fixture.map.progress_map_route(), Some(transition));
    assert_eq!(
        fixture.entities.player().unwrap().position_raw(),
        wreck_pose
    );
    assert_eq!(fixture.craft.body_angle_words(), [-1_234, 2_345]);
    assert_eq!(fixture.hull.health_raw, 12_345);
    assert_eq!(fixture.map.continue_progress_map(), Some(transition));
    assert_eq!(fixture.map.continue_progress_map(), None);
}

#[derive(Debug, PartialEq, Eq)]
struct AuthoredAftermathActor {
    spawn_index: usize,
    entity_type: u32,
    position_raw: [i16; 3],
    model_index: Option<usize>,
    health_raw: RetailRuntimeValue<i32>,
    power_up_payload: Option<u32>,
}

fn authored_aftermath(entities: &EntityManager) -> Vec<AuthoredAftermathActor> {
    entities
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 6 | 61 | 66 | 67))
        .map(|entity| AuthoredAftermathActor {
            spawn_index: entity.authored_spawn_index.unwrap(),
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            model_index: entity.model_index,
            health_raw: entity.collision.health_raw,
            power_up_payload: entity.power_up_payload_packed,
        })
        .collect()
}

#[v2k_test_support::retail_test]
fn retry_reconstructs_authored_level_one_and_reattaches_native_cargo() {
    let mut fixture = RetryFixture::new();
    let authored = authored_aftermath(&fixture.entities);
    for entity_type in [6, 66, 67] {
        assert!(authored
            .iter()
            .any(|entity| entity.entity_type == entity_type));
    }
    for payload in [0x3c, 0xc802] {
        assert!(authored
            .iter()
            .any(|entity| entity.power_up_payload == Some(payload)));
    }
    // Deliberately alter only outgoing world-local state. This tests native
    // reconstruction, not execution of the systemic abort/death callbacks.
    for actor in &authored {
        let entity = fixture
            .entities
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(actor.spawn_index))
            .unwrap()
            .id;
        let entity = fixture.entities.entity_mut(entity).unwrap();
        entity.active = false;
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        if let Some(emitter) = &mut entity.authored_radial_emitter {
            emitter.enter_dying_slot0();
            emitter.arm_wreck_suction();
        }
    }
    let checkpoint = fixture.map_checkpoint();
    let restore = NativeSaveRestore::decode(&checkpoint).unwrap();
    let transition = fixture.map.continue_progress_map().unwrap();
    fixture
        .session
        .load_level_by_id(transition.destination_level_id(), 1)
        .unwrap();
    let types = metadata(&fixture.session);
    let level = fixture.session.cache.level_desc().unwrap();
    let mut terrain = fixture.session.cache.level_terrain().unwrap().clone();
    let extent = |id| {
        fixture
            .session
            .cache
            .global_model(id)
            .map(|model| model.radius)
    };
    let resources = EntityConstructionResources {
        terrain: fixture.session.cache.terrain(),
        terrain_objects: fixture.session.cache.terrain_objects(),
        model_extent_raw: Some(&extent),
    };
    let mut rebuilt = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level,
            logical_world_index: transition.destination_logical_level() as i32,
            type_metadata: &types,
            resources,
            player_arrival: None,
            retail_tick: 148,
        }
        .with_native_save(NativeSaveWorldState {
            restore: &restore,
            terrain: &mut terrain,
            resources: &fixture.session.cache,
            static_damage: &mut StaticDamageScheduler::new(),
        }),
        &mut fixture.fx,
    )
    .unwrap();
    assert_eq!(authored_aftermath(&rebuilt), authored);
    assert!(rebuilt
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 6 | 61 | 66 | 67))
        .all(|entity| entity.active));
    let hive = rebuilt
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap();
    let emitter = hive.authored_radial_emitter.as_ref().unwrap();
    assert!(!emitter.dying_slot0());
    assert_eq!(emitter.controller_state(), 1);
    let births = emitter.birth_runtime().unwrap();
    assert_eq!(births.rows().len(), 1);
    let row = &births.rows()[0];
    assert_eq!(row.record().entity_type, 15);
    assert_eq!(row.record().total_production_cap, 3);
    assert_eq!(row.produced_count(), 0);
    assert_eq!(row.timer_us(), row.record().initial_timer_us);
    assert!(row.children().is_empty());
    assert!(!row.constructor_parked());
    assert!(births
        .ejection_slots()
        .iter()
        .all(|slot| slot.child_handle.is_none() && slot.timer_us == 0));
    let player = rebuilt.player().unwrap();
    assert_eq!(player.position_raw(), fixture.retained_entry_raw);
    assert_eq!(player.velocity_raw(), [0; 3]);
    assert_eq!(player.rotation_heading_pitch_roll_raw(), [0x4000, 0, 0]);
    assert_eq!(
        player.collision.health_raw,
        RetailRuntimeValue::Known(40_000)
    );
    assert_eq!(
        player.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(80_833)
    );
    assert_eq!(rebuilt.retail_live_order_ids().next(), Some(player.id));
    assert_eq!(
        rebuilt
            .iter_all()
            .filter(|entity| entity.entity_type == 46)
            .count(),
        1
    );
    let mut scheduler = SpecializedActorTaskScheduler::new();
    rebuilt
        .restore_campaign_cargo_controller_state(
            restore.cargo.clone(),
            CampaignCargoRestoreContext {
                level,
                resources,
                retail_tick: 148,
                scheduler: &mut scheduler,
                world_fx: &mut fixture.fx,
                notifications: &mut GameplayNotifications::new(),
            },
        )
        .unwrap();
    assert_eq!(
        rebuilt.campaign_cargo_controller_state(),
        RetailRuntimeValue::Known(fixture.cargo)
    );
    let mut craft = fixture
        .craft
        .campaign_world_replacement(Default::default(), 148);
    restore.restore_craft(&mut craft);
    assert_eq!(craft.mode, VehicleMode::Vtol);
    assert_eq!(craft.fuel_raw, 99_119);
    assert_eq!(craft.body_angle_words(), [0, 0]);
    assert_eq!(
        craft.weapon_selector(),
        i32::from(restore.inventory.selected_descriptor().callback_selector())
    );
    assert_eq!(restore.campaign, fixture.campaign);
}
