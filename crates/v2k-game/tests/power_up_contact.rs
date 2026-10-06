//! Real-data coverage for the bounded player/type-61 contact transaction.

use v2k_game::entity::EntityManager;
use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::player::PlayerCraft;
use v2k_game::player_hull::PlayerHull;
use v2k_game::power_up_contact::{
    resolve_player_power_up_contacts, AcceptedPowerUpContact, PlayerCampaignProgress,
    PlayerPowerUpRecipient, PowerUpContactRejection, TROPHY_RESTORED_BUFFER_RAW,
    TROPHY_RESTORED_HULL_RAW,
};
use v2k_game::session::GameSession;
use v2k_game::weapon_inventory::{
    PlayerCapabilities, PowerUpPayload, WeaponAcquisition, WeaponInventory,
    LEVEL_ONE_WEAPON_SELECTOR, TARGETTER_SELECTOR,
};

fn level_one() -> Option<(GameSession, EntityManager)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    // Variant 1 is retail's normal high-resolution tier. Section-12 lives in
    // system level 3 and must be retained before loading the world overlay.
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
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
        .collect::<Vec<_>>();
    session
        .load_level_by_id(13, 1)
        .expect("retail fixture must load");
    let manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc()?,
        &type_metadata,
        Some(session.cache.terrain()?),
    );
    Some((session, manager))
}

fn place_player_at_payload(manager: &mut EntityManager, packed: u32) -> u32 {
    let (id, position) = manager
        .iter_all()
        .find(|entity| entity.power_up_payload_packed == Some(packed))
        .map(|entity| (entity.id, entity.position))
        .expect("authored Level-1 Power Up payload");
    manager.player_mut().expect("persistent player").position = position;
    id
}

fn resolve_contacts(
    session: &GameSession,
    manager: &mut EntityManager,
    craft: &mut PlayerCraft,
    inventory: &mut WeaponInventory,
    capabilities: &mut PlayerCapabilities,
    hull: &mut PlayerHull,
    campaign_progress: &mut PlayerCampaignProgress,
) -> v2k_game::power_up_contact::PlayerPowerUpContactPass {
    resolve_player_power_up_contacts(
        manager,
        &session.cache,
        0,
        PlayerPowerUpRecipient {
            craft,
            weapon_inventory: inventory,
            capabilities,
            hull,
            campaign_progress,
        },
    )
    .expect("authored player/powerup collision program")
}

fn accepted_contacts(
    pass: &v2k_game::power_up_contact::PlayerPowerUpContactPass,
) -> Vec<AcceptedPowerUpContact> {
    pass.accepted().copied().collect()
}

fn rejected_contacts(
    pass: &v2k_game::power_up_contact::PlayerPowerUpContactPass,
) -> Vec<v2k_game::power_up_contact::RejectedPowerUpContact> {
    pass.rejected().copied().collect()
}

#[v2k_test_support::retail_test]
fn selector_two_exact_contact_mutates_then_defers_ordered_destroy() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let pickup_id = place_player_at_payload(&mut manager, 0x0000_c802);
    let order_before = manager.retail_live_order_ids().collect::<Vec<_>>();
    let mut inventory = WeaponInventory::new();
    let mut craft = PlayerCraft::new();
    let mut capabilities = PlayerCapabilities::default();
    let mut hull = PlayerHull::default();
    let mut campaign_progress = PlayerCampaignProgress::default();

    let pass = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );

    let accepted = accepted_contacts(&pass);
    assert_eq!(accepted.len(), 1);
    assert!(matches!(
        accepted[0],
        AcceptedPowerUpContact::Weapon {
            entity_id,
            payload: PowerUpPayload {
                selector: LEVEL_ONE_WEAPON_SELECTOR,
                amount: 200,
            },
            ..
        } if entity_id == pickup_id
    ));
    assert_eq!(inventory.selected_descriptor().selector(), 2);
    assert_eq!(inventory.selected_descriptor().stored_resource_count(), 200);
    assert_eq!(manager.pending_power_up_destroy_ids(), &[pickup_id]);
    assert!(manager.iter_all().any(|entity| entity.id == pickup_id));

    // A second scan in the same frame cannot acquire a pending allocation.
    let repeat = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );
    assert!(repeat.contacts.is_empty());

    assert_eq!(manager.cleanup_pending_power_up_destroys(), vec![pickup_id]);
    assert!(!manager.iter_all().any(|entity| entity.id == pickup_id));
    assert_eq!(
        manager.retail_live_order_ids().collect::<Vec<_>>(),
        order_before
            .into_iter()
            .filter(|id| *id != pickup_id)
            .collect::<Vec<_>>(),
        "deferred splice must preserve every survivor's live-list order"
    );
    assert!(manager.cleanup_pending_power_up_destroys().is_empty());
}

#[v2k_test_support::retail_test]
fn first_targetter_contact_sets_capability_and_queues_destroy() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let targetter_id = place_player_at_payload(&mut manager, 0x0000_003c);
    let targetter_position_raw = manager
        .iter_all()
        .find(|entity| entity.id == targetter_id)
        .expect("authored Targetter")
        .position_raw();
    let mut inventory = WeaponInventory::new();
    let mut craft = PlayerCraft::new();
    let mut capabilities = PlayerCapabilities::default();
    let mut hull = PlayerHull::default();
    let mut campaign_progress = PlayerCampaignProgress::default();

    let pass = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );

    assert_eq!(
        accepted_contacts(&pass),
        vec![AcceptedPowerUpContact::Targetter {
            entity_id: targetter_id,
            position_raw: targetter_position_raw,
            payload: PowerUpPayload {
                selector: TARGETTER_SELECTOR,
                amount: 0,
            },
        }]
    );
    assert!(capabilities.has_targetter());
    assert_eq!(manager.pending_power_up_destroy_ids(), &[targetter_id]);
    assert!(manager.iter_all().any(|entity| entity.id == targetter_id));
}

#[v2k_test_support::retail_test]
fn selector_two_duplicate_is_still_accepted_and_consumed() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let pickup_id = place_player_at_payload(&mut manager, 0x0000_c802);
    let payload = PowerUpPayload {
        selector: LEVEL_ONE_WEAPON_SELECTOR,
        amount: 200,
    };
    let mut inventory = WeaponInventory::new();
    let mut craft = PlayerCraft::new();
    assert!(matches!(
        inventory.acquire_weapon(payload),
        WeaponAcquisition::Accepted { .. }
    ));
    let mut capabilities = PlayerCapabilities::default();
    let mut hull = PlayerHull::default();
    let mut campaign_progress = PlayerCampaignProgress::default();

    let pass = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );

    assert_eq!(accepted_contacts(&pass).len(), 1);
    assert!(rejected_contacts(&pass).is_empty());
    assert_eq!(inventory.selected_descriptor().stored_resource_count(), 200);
    assert_eq!(manager.pending_power_up_destroy_ids(), &[pickup_id]);
}

#[v2k_test_support::retail_test]
fn duplicate_targetter_and_unresolved_slot_trophy_contacts_stay_live() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let targetter_id = place_player_at_payload(&mut manager, 0x0000_003c);
    let mut inventory = WeaponInventory::new();
    let mut craft = PlayerCraft::new();
    let mut capabilities = PlayerCapabilities::default();
    let mut hull = PlayerHull::default();
    let mut campaign_progress = PlayerCampaignProgress::default();
    assert_eq!(
        capabilities.acquire_targetter(PowerUpPayload {
            selector: TARGETTER_SELECTOR,
            amount: 0,
        }),
        v2k_game::weapon_inventory::TargetterAcquisition::Acquired
    );

    let duplicate = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );
    assert!(accepted_contacts(&duplicate).is_empty());
    let rejected = rejected_contacts(&duplicate);
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].entity_id, targetter_id);
    assert_eq!(
        rejected[0].reason,
        PowerUpContactRejection::TargetterAlreadyAcquired
    );
    assert!(manager.pending_power_up_destroy_ids().is_empty());
    assert!(manager.iter_all().any(|entity| entity.id == targetter_id));

    let trophy_id = place_player_at_payload(&mut manager, 0x0000_003f);
    let trophy = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );
    assert!(accepted_contacts(&trophy).is_empty());
    let rejected = rejected_contacts(&trophy);
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].entity_id, trophy_id);
    assert_eq!(
        rejected[0].reason,
        PowerUpContactRejection::Trophy(
            v2k_game::power_up_contact::TrophyAcquisitionRejection::ControlSlotUnresolved
        )
    );
    assert!(manager.pending_power_up_destroy_ids().is_empty());
    assert!(manager.iter_all().any(|entity| entity.id == trophy_id));
}

#[v2k_test_support::retail_test]
fn level_one_trophy_restores_hull_claims_slot_counts_and_defers_destroy() {
    let (session, mut manager) = level_one().expect("retail fixture");
    let trophy_id = place_player_at_payload(&mut manager, 0x0000_003f);
    let mut inventory = WeaponInventory::new();
    let mut craft = PlayerCraft::new();
    let mut capabilities = PlayerCapabilities::default();
    let mut hull = PlayerHull::default();
    hull.health_raw = 1;
    hull.pre_health_damage_buffer_raw = 2;
    let mut campaign_progress = PlayerCampaignProgress::default();
    campaign_progress.set_current_control_slot(Some(1));

    let pass = resolve_contacts(
        &session,
        &mut manager,
        &mut craft,
        &mut inventory,
        &mut capabilities,
        &mut hull,
        &mut campaign_progress,
    );

    let accepted = accepted_contacts(&pass);
    assert_eq!(accepted.len(), 1);
    assert!(matches!(
        accepted[0],
        AcceptedPowerUpContact::Trophy {
            entity_id,
            acquisition: v2k_game::power_up_contact::TrophyAcquisition {
                restored_hull: true,
                first_claim: true,
                counted_trophy: true,
                awarded_extra_life: false,
                trophy_count: 1,
                extra_lives: 0,
            },
            ..
        } if entity_id == trophy_id
    ));
    assert_eq!(hull.health_raw, TROPHY_RESTORED_HULL_RAW);
    assert_eq!(
        hull.pre_health_damage_buffer_raw,
        TROPHY_RESTORED_BUFFER_RAW
    );
    assert_eq!(campaign_progress.control_slot_claimed(1), Some(true));
    assert_eq!(manager.pending_power_up_destroy_ids(), &[trophy_id]);
}
