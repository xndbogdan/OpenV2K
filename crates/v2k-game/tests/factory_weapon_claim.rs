use std::time::Duration;

use v2k_game::entity::EntityManager;
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::factory_activation_live::tick_level_one_factory_owner;
use v2k_game::factory_production::FactoryProductionPhase;
use v2k_game::factory_production_live::published_status;
use v2k_game::factory_status_runtime::project_factory_status;
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::player::PlayerCraft;
use v2k_game::player_hull::PlayerHull;
use v2k_game::power_up_contact::{
    resolve_player_power_up_contacts, AcceptedPowerUpContact, PlayerCampaignProgress,
    PlayerPowerUpRecipient,
};
use v2k_game::primary_weapon::{
    fire_profile_from_descriptor, PrimaryFireGeometry, PrimaryGunChannel, PrimaryGunMounts,
    PrimaryLaunchBasis, PrimaryShotBudget, PrimaryTriggerInput, PrimaryWeapon,
};
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler;
use v2k_game::weapon_inventory::{
    AmmoCommit, Ammunition, PlayerCapabilities, PowerUpPayload, WeaponAcquisition, WeaponInventory,
    DEFAULT_WEAPON_MASTER_DESCRIPTOR_RAW, FACTORY_LEVEL_ONE_WEAPON_SELECTOR,
    LEVEL_ONE_WEAPON_DESCRIPTOR_RAW, WEAPON_MASTER_TABLE_RAW,
};
use v2k_game::world_fx::{WorldFx, RAPID_PRIMARY_PARTICLE_CLASS};

#[test]
fn master_table_matches_the_known_selector_one_and_two_records() {
    assert_eq!(
        WEAPON_MASTER_TABLE_RAW[0],
        DEFAULT_WEAPON_MASTER_DESCRIPTOR_RAW
    );
    assert_eq!(WEAPON_MASTER_TABLE_RAW[1], LEVEL_ONE_WEAPON_DESCRIPTOR_RAW);
    assert!(WEAPON_MASTER_TABLE_RAW.iter().any(|raw| raw[0] == 0x12));
}

#[test]
fn factory_level_one_payload_installs_selector_eighteen() {
    let mut inventory = WeaponInventory::new();
    let payload = v2k_game::weapon_inventory::PowerUpPayload::from_runtime_word(0x0001_f412);
    assert_eq!(payload.selector, FACTORY_LEVEL_ONE_WEAPON_SELECTOR);
    assert_eq!(payload.amount, 500);
    assert_eq!(
        inventory.acquire_weapon(payload),
        WeaponAcquisition::Accepted {
            slot: 1,
            previous_resource_count: 0,
            resource_count: 500,
            auto_selected: true,
        }
    );
    let descriptor = inventory.selected_descriptor();
    assert_eq!(descriptor.selector(), 0x12);
    assert_eq!(descriptor.stored_resource_count(), 500);
    assert_eq!(descriptor.callback_selector(), 0x0f);
    assert_eq!(descriptor.cadence_micros(), 40_000);
    assert_eq!(descriptor.firing_sound_id(), 88);
    assert_eq!(descriptor.ammunition(), Ammunition::Infinite);
}

#[v2k_test_support::retail_test]
fn factory_product_contact_auto_selects_and_fires_the_rapid_machine_gun() {
    let dir = v2k_test_support::retail_dir();
    assert!(dir.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&dir).expect("initialize retail corpus");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal-tier system types");
    let metadata = session
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
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    session
        .load_level_by_id(13, 1)
        .expect("first-world overlay");
    let terrain = session.cache.terrain().expect("first-world terrain");
    let mut world_fx = WorldFx::new();
    let mut entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("first-world descriptor"),
        &metadata,
        Some(terrain),
        0,
        &mut world_fx,
    )
    .expect("authenticated fresh first-world entities");
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(23))
        .expect("authored Working Factory")
        .id;

    // Begin at the fully staffed precondition. Scientist delivery has separate
    // coverage; all production, allocation, lift and pickup actions below use
    // the live owners, including the real Section-13 output payload.
    let factory = entities.entity_mut(factory_id).unwrap();
    let RetailRuntimeValue::Known(Some(base)) = &mut factory.base_factory_runtime else {
        panic!("fresh factory runtime must be resolved");
    };
    let production = base.production.as_mut().expect("Section-13 production");
    assert_eq!(production.output_payload_packed, 0x0001_f412);
    assert_eq!(production.scientist_capacity_raw, 2);
    production.current_scientists_raw = 2;
    *base = project_factory_status(*base, published_status(base.production.unwrap())).after;

    let mut notifications = GameplayNotifications::new();
    let mut product = None;
    for frame in 1..=550 {
        let outcome = tick_level_one_factory_owner(
            &mut entities,
            terrain,
            &mut world_fx,
            &mut notifications,
            frame,
            factory_id,
            20_000,
        )
        .expect("live staffed production and delivery");
        if frame == 300 {
            product = outcome.product;
            assert!(
                product.is_some(),
                "six-second production creates the pickup"
            );
            assert_eq!(outcome.production.phase, FactoryProductionPhase::Delivering);
        }
        if frame == 550 {
            assert_eq!(
                outcome.production.phase,
                FactoryProductionPhase::WaitingForPickup
            );
        }
        entities.follow_base_factory_products_to_marker(&session.cache, frame);
    }
    let product_id = product.expect("published product").entity_id.get();
    let product_entity = entities
        .iter_all()
        .find(|entity| entity.id == product_id)
        .unwrap();
    assert!(product_entity.factory_type61_birth_provenance().is_some());
    assert_eq!(product_entity.power_up_payload_packed, Some(0x0001_f412));
    let pickup_position = product_entity.position;
    entities.player_mut().expect("persistent player").position = pickup_position;

    let mut inventory = WeaponInventory::new();
    let mut craft = PlayerCraft::new();
    let mut capabilities = PlayerCapabilities::default();
    let mut hull = PlayerHull::default();
    let mut campaign_progress = PlayerCampaignProgress::default();
    // Main reaches the inventory suffix only after this complete active-solid
    // census succeeds. A unit-only payload claim must not hide a live gate
    // which would prevent the player from collecting the delivered product.
    entities
        .resolve_player_active_contacts(
            &session.cache,
            v2k_game::player_active_contact::PlayerActivePairFrame {
                resources: &session.cache,
                retail_tick: 550,
                player_craft: &craft,
                player_hull: &mut hull,
                scheduler: &mut SpecializedActorTaskScheduler::new(),
                world_fx: &mut WorldFx::new(),
                notifications: &mut GameplayNotifications::new(),
                extra_lives: 0,
            },
        )
        .expect("main's active-solid gate at the delivered factory product");
    let contacts = resolve_player_power_up_contacts(
        &mut entities,
        &session.cache,
        550,
        PlayerPowerUpRecipient {
            craft: &mut craft,
            weapon_inventory: &mut inventory,
            capabilities: &mut capabilities,
            hull: &mut hull,
            campaign_progress: &mut campaign_progress,
        },
    )
    .expect("real player and product collision programs");
    assert!(contacts.accepted().any(|contact| matches!(contact,
        AcceptedPowerUpContact::Weapon {
            entity_id,
            payload: PowerUpPayload { selector: FACTORY_LEVEL_ONE_WEAPON_SELECTOR, amount: 500 },
            acquisition: WeaponAcquisition::Accepted { auto_selected: true, .. }, ..
        } if *entity_id == product_id
    )), "factory product must reach the inventory through contact: {contacts:?}");
    assert!(entities.is_power_up_destroy_pending(product_id));
    assert_eq!(inventory.selected_descriptor().selector(), 0x12);
    assert_eq!(inventory.selected_ammunition(), Ammunition::Infinite);
    assert_eq!(
        entities.cleanup_pending_power_up_destroys(),
        vec![product_id]
    );

    let player = entities.player().unwrap();
    let geometry = PrimaryFireGeometry {
        launch_basis: PrimaryLaunchBasis {
            origin_world: player.position,
            direction_unit: [0.0, 0.0, 1.0],
        },
        // Muzzle presentation is independently tested against the normal-tier
        // gun hierarchy; the recovered physics birth is the player center.
        gun_mounts: PrimaryGunMounts::Unresolved,
        shooter_origin_world: player.position,
        shooter_id: player.id,
        shooter_entity_type_at_birth: Some(
            u8::try_from(player.entity_type).expect("player type byte"),
        ),
        shooter_velocity_world: player.velocity,
        sound_origin_world: player.position,
    };
    let profile = fire_profile_from_descriptor(inventory.selected_descriptor())
        .expect("factory descriptor must be fireable after auto-selection");
    let mut weapon = PrimaryWeapon::new();
    let held = PrimaryTriggerInput {
        source_a: true,
        source_b: false,
    };
    let first = weapon.update_with_profile(
        Duration::ZERO,
        held,
        geometry,
        profile,
        PrimaryShotBudget::Unlimited,
    );
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].gun_channel, PrimaryGunChannel::A);
    let catch_up = weapon.update_with_profile(
        Duration::from_millis(100),
        held,
        geometry,
        profile,
        PrimaryShotBudget::Unlimited,
    );
    assert_eq!(catch_up.len(), 2);
    assert_eq!(catch_up[0].gun_channel, PrimaryGunChannel::B);
    assert_eq!(catch_up[1].gun_channel, PrimaryGunChannel::A);
    for event in first.iter().chain(&catch_up) {
        assert_eq!(
            event.projectile.spec.projectile_class,
            RAPID_PRIMARY_PARTICLE_CLASS
        );
        assert_eq!(event.projectile.origin_world, player.position);
        assert!(matches!(
            inventory.commit_selected_round(),
            AmmoCommit::Fired {
                selector: 0x12,
                remaining: Ammunition::Infinite,
                ..
            }
        ));
    }

    // One callback can discharge two rounds, but FUN_00424650's sound is
    // submitted once after that catch-up loop. Isolate the firing effects from
    // the preceding factory presentation transactions.
    let mut firing_fx = WorldFx::new();
    firing_fx.queue_primary_fire_batch(&first, None);
    firing_fx.queue_primary_fire_batch(&catch_up, None);
    firing_fx.process_pending();
    assert_eq!(
        firing_fx.particle_count(),
        6,
        "three bullets and three companion flashes"
    );
    let sounds = firing_fx.take_positional_sounds();
    assert_eq!(sounds.len(), 2, "one sound per firing callback");
    assert!(sounds.iter().all(|sound| sound.sound_id == 88));
    assert_eq!(inventory.selected_descriptor().stored_resource_count(), 500);
    assert_eq!(inventory.selected_ammunition(), Ammunition::Infinite);
}
