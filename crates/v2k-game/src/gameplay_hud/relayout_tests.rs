use super::*;
use crate::power_up_contact::PlayerCampaignProgress;
use crate::session::GameSession;
use crate::weapon_inventory::PowerUpPayload;

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    GameSession::init(&data).unwrap()
}

#[v2k_test_support::retail_test]
fn authored_high_tier_relayout_preserves_evaluated_trail_reveal_carousel_cargo_and_clock() {
    let mut session = session();
    let layouts = [1, 2, 3].map(|variant| {
        session.load_auxiliary_ovl(3, variant).unwrap();
        GameplayHudLayout::from_cache(&session.cache, variant).unwrap()
    });
    let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
    let mut hud = GameplayHud::default();
    hud.advance_spins(12_345);
    let mut inventory = WeaponInventory::new();
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(0));
    let trophy = TimeTrophyRuntime::from_loaded_world(270, false, &mut progress)
        .unwrap()
        .0;
    for _ in 0..3 {
        hud.frame(
            &resources,
            layouts[0],
            FUEL_FULL_RAW,
            HULL_FULL_RAW,
            12_345,
            20_000,
            &GameplayHudWeaponRoster::from_inventory(&inventory),
            &[],
            trophy,
        );
    }
    inventory.acquire_weapon(PowerUpPayload::from_runtime_word(0x0000_c802));
    let roster = GameplayHudWeaponRoster::from_inventory(&inventory);
    let cargo = [Some(9), None, Some(67)];
    let before = hud;
    let mut frame = hud.frame(
        &resources,
        layouts[0],
        FUEL_FULL_RAW / 3,
        10_000,
        12_345,
        20_000,
        &roster,
        &cargo,
        trophy,
    );
    let original = frame.clone();
    assert!(frame
        .layers
        .iter()
        .any(|layer| layer.role == GameplayHudLayerRole::HullTrail));
    assert!(frame
        .layers
        .iter()
        .any(|layer| layer.role == GameplayHudLayerRole::StatusLeft));
    assert_eq!(frame.weapons.len(), 2);
    assert_eq!(frame.cargo.len(), 3);
    assert!(frame.time_trophy.is_some());
    let state_after = hud;
    let mut previous = layouts[0];
    for next in [layouts[1], layouts[2], layouts[0]] {
        let mut expected_state = before;
        let expected = expected_state.frame(
            &resources,
            next,
            FUEL_FULL_RAW / 3,
            10_000,
            12_345,
            20_000,
            &roster,
            &cargo,
            trophy,
        );
        frame
            .relayout(GameplayHudRelayout { previous, next })
            .unwrap();
        assert_eq!(
            frame, expected,
            "all commands and semantic clips match the new authored anchors"
        );
        assert_eq!(expected_state, state_after);
        assert_eq!(
            hud, state_after,
            "relayout never advances the live HUD state"
        );
        previous = next;
    }
    assert_eq!(frame, original, "high-tier roundtrip is lossless");
}

#[v2k_test_support::retail_test]
fn rejected_relayout_preserves_the_complete_old_frame() {
    let mut session = session();
    session.load_auxiliary_ovl(3, 1).unwrap();
    let layout = GameplayHudLayout::from_cache(&session.cache, 1).unwrap();
    let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
    let mut frame = GameplayHud::default().frame(
        &resources,
        layout,
        FUEL_FULL_RAW,
        HULL_FULL_RAW,
        0,
        0,
        &GameplayHudWeaponRoster::default(),
        &[],
        TimeTrophyRuntime::default(),
    );
    let original = frame.clone();
    let mut incompatible = layout;
    incompatible.weapon_spacing.y += 1;
    assert_eq!(
        frame.relayout(GameplayHudRelayout {
            previous: layout,
            next: incompatible
        }),
        Err(GameplayHudRelayoutError::IntrinsicGeometryChanged)
    );
    assert_eq!(frame, original);
    frame
        .layers
        .iter_mut()
        .find(|layer| layer.role == GameplayHudLayerRole::HullFill)
        .unwrap()
        .clip
        .as_mut()
        .unwrap()
        .width -= 1;
    let malformed = frame.clone();
    assert_eq!(
        frame.relayout(GameplayHudRelayout {
            previous: layout,
            next: layout
        }),
        Err(GameplayHudRelayoutError::ClipShape(
            GameplayHudLayerRole::HullFill
        ))
    );
    assert_eq!(frame, malformed, "validation does not publish a prefix");
    let mut wrong_previous = layout;
    wrong_previous.virtual_width += 1;
    assert_eq!(
        frame.relayout(GameplayHudRelayout {
            previous: wrong_previous,
            next: layout
        }),
        Err(GameplayHudRelayoutError::FrameLayoutMismatch)
    );
    assert_eq!(frame, malformed);
}
