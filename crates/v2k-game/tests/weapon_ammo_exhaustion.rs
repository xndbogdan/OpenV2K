//! Source-backed final-round selection through the production HUD compositor.

use std::time::Duration;

use v2k_game::gameplay_hud::{
    GameplayHud, GameplayHudLayerRole, GameplayHudLayout, GameplayHudResources,
    GameplayHudWeaponRoster,
};
use v2k_game::primary_weapon::{
    fire_profile_from_descriptor, PrimaryFireGeometry, PrimaryGunMounts, PrimaryLaunchBasis,
    PrimaryShotBudget, PrimaryTriggerInput, PrimaryWeapon,
};
use v2k_game::session::GameSession;
use v2k_game::time_trophy::TimeTrophyRuntime;
use v2k_game::weapon_inventory::{
    AmmoCommit, Ammunition, PowerUpPayload, WeaponAcquisition, WeaponCycleDirection,
    WeaponInventory,
};

#[v2k_test_support::retail_test]
fn final_round_switches_hud_then_manual_cycles_skip_empty_until_same_slot_reacquisition() {
    let data_dir = v2k_test_support::retail_dir();
    assert!(
        data_dir.join("PRELOAD.DAT").is_file(),
        "retail corpus required"
    );
    let mut session = GameSession::init(&data_dir).expect("initialize retail corpus");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal high system art");
    let resources =
        GameplayHudResources::from_cache(&session.cache).expect("normal high HUD sprites");
    let layout = GameplayHudLayout::from_cache(&session.cache, 1).expect("normal high HUD layout");
    let mut inventory = WeaponInventory::new();
    let mut hud = GameplayHud::default();
    let make_frame = |hud: &mut GameplayHud, inventory: &WeaponInventory| {
        hud.frame(
            &resources,
            layout,
            200_000,
            40_000,
            100,
            20_000,
            &GameplayHudWeaponRoster::from_inventory(inventory),
            &[],
            TimeTrophyRuntime::default(),
        )
    };
    assert_eq!(make_frame(&mut hud, &inventory).weapons[0].selector, 1);
    inventory.acquire_weapon(PowerUpPayload {
        selector: 2,
        amount: 1,
    });
    assert_eq!(
        make_frame(&mut hud, &inventory)
            .weapons
            .last()
            .unwrap()
            .selector,
        2
    );
    let descriptor = *inventory.selected_descriptor();
    let profile = fire_profile_from_descriptor(&descriptor).expect("source class-3 flare profile");
    let basis = PrimaryLaunchBasis {
        origin_world: [0.0; 3],
        direction_unit: [0.0, 0.0, 1.0],
    };
    let geometry = PrimaryFireGeometry {
        launch_basis: basis,
        gun_mounts: PrimaryGunMounts::Unresolved,
        shooter_origin_world: [0.0; 3],
        shooter_id: 46,
        shooter_entity_type_at_birth: Some(46),
        shooter_velocity_world: [0.0; 3],
        sound_origin_world: [0.0; 3],
    };
    let events = PrimaryWeapon::new().update_with_profile(
        Duration::from_millis(20),
        PrimaryTriggerInput {
            source_a: true,
            source_b: false,
        },
        geometry,
        profile,
        PrimaryShotBudget::Limited(1),
    );
    assert_eq!(
        events.len(),
        1,
        "the final projectile is emitted before selection changes"
    );
    assert_eq!(events[0].projectile.spec.projectile_class, 3);
    assert_eq!(inventory.selected_descriptor(), &descriptor);
    assert_eq!(
        inventory.commit_selected_round(),
        AmmoCommit::Fired {
            slot: 1,
            selector: 2,
            remaining: Ammunition::Finite(0),
            automatic_successor_slot: Some(0),
        }
    );

    // 443260 retains the exhausted selector/count pair; 4292B0 can complete
    // its outgoing neighbor while the selected weapon is already the default.
    assert_eq!(GameplayHudWeaponRoster::from_inventory(&inventory).len(), 2);
    let transition = make_frame(&mut hud, &inventory);
    assert_eq!(
        transition
            .weapons
            .iter()
            .map(|weapon| weapon.selector)
            .collect::<Vec<_>>(),
        [2, 1],
        "the outgoing zero-count neighbor precedes the new selected weapon"
    );
    for _ in 0..64 {
        make_frame(&mut hud, &inventory);
    }
    let settled = make_frame(&mut hud, &inventory);
    assert_eq!(settled.weapons.len(), 1);
    assert_eq!(settled.weapons[0].selector, 1);
    assert!(!settled.layers.iter().any(|layer| matches!(
        layer.role,
        GameplayHudLayerRole::AmmoHundreds
            | GameplayHudLayerRole::AmmoTens
            | GameplayHudLayerRole::AmmoOnes
    )));
    for direction in [
        WeaponCycleDirection::Forward,
        WeaponCycleDirection::Backward,
    ] {
        assert_eq!(inventory.cycle(direction), 0);
        assert!(make_frame(&mut hud, &inventory)
            .weapons
            .iter()
            .all(|weapon| weapon.selector == 1));
    }

    let exhausted = *inventory.slot(1).unwrap();
    assert_eq!(exhausted.selector(), 2);
    assert_eq!(exhausted.ammunition(), Ammunition::Finite(0));
    let mut saved = [0; 0x248];
    inventory.encode_into_native_payload(&mut saved);
    let restored =
        WeaponInventory::restore_from_native_payload(&saved).expect("native retained zero count");
    assert_eq!(restored.slot(1), Some(&exhausted));
    assert_eq!(restored.selected_slot(), 0);
    assert_eq!(
        inventory.acquire_weapon(PowerUpPayload {
            selector: 2,
            amount: 3
        }),
        WeaponAcquisition::Accepted {
            slot: 1,
            previous_resource_count: 0,
            resource_count: 3,
            auto_selected: true
        }
    );
    let reacquired = make_frame(&mut hud, &inventory);
    assert_eq!(reacquired.weapons.last().unwrap().selector, 2);
    assert_eq!(inventory.occupied_slot_count(), 2);
    assert_eq!(inventory.selected_ammunition(), Ammunition::Finite(3));
}
