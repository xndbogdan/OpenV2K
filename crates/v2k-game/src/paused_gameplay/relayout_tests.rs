use super::*;
use v2k_game::gameplay_hud::{GameplayHud, GameplayHudLayout, GameplayHudWeaponRoster};
use v2k_game::gameplay_radar::RadarImage;
use v2k_game::system_layout::HighSystemLayoutTier;

#[v2k_test_support::retail_test]
fn frozen_overlay_relayout_preflights_without_advancing_or_replacing_other_commands() {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = v2k_game::session::GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(2, 1).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    let previous = GameplayHudLayout::from_cache(&session.cache, 1).unwrap();
    let radar_previous = GameplayRadarLayout::from_cache(&session.cache, 1).unwrap();
    let stage = session
        .prepare_high_system_layout_refresh(HighSystemLayoutTier::High1024)
        .unwrap();
    let next = GameplayHudLayout::from_cache(&stage, 3).unwrap();
    let radar_next = GameplayRadarLayout::from_cache(&stage, 3).unwrap();
    let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
    let hud = GameplayHud::default().frame(
        &resources,
        previous,
        v2k_game::player::FUEL_FULL_RAW / 2,
        v2k_game::gameplay_hud::HULL_FULL_RAW,
        123,
        20_000,
        &GameplayHudWeaponRoster::default(),
        &[Some(9)],
        v2k_game::time_trophy::TimeTrophyRuntime::default(),
    );
    let radar = RadarHudFrame {
        image: RadarImage {
            width: 96,
            height: 96,
            rgba: vec![73; 96 * 96 * 4],
        },
        origin: radar_previous.hud_origin,
        virtual_size: radar_previous.virtual_size,
    };
    let notification = GameplayNotificationLine {
        string_id: 0xe6,
        text: "Factory".into(),
        x_percent: 20,
        baseline_percent: 70,
        width_percent: 60,
        center_x: false,
    };
    let lights = vec![TerrainExplosionLight {
        x_raw: 17,
        z_raw: 19,
        radius_raw: 400,
    }];
    let full_frame_sprite = Some(FullFrameSpriteFrame {
        table_index: 1,
        global_sprite_id: 0x227,
    });
    let shield = v2k_game::player_shield::PlayerShieldFrame {
        spin_raw: 0x1234,
        retail_tick: 123,
        damage_flash_level: 3,
    };
    let mut fx = v2k_game::world_fx::WorldFx::new();
    fx.emit_player_wreck_burst_raw([0; 3], 128, None, 46);
    let particles = fx.prepare_presentation([640, 480], 0x1800, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [50, 50],
            depth_raw: 0x100,
            clip: 0,
        }
    });
    assert!(particles.particles().len() > 0);
    let retained_particles = particles.clone();
    let mut overlay = GameplayOverlayFrame {
        hud: Some(hud.clone()),
        radar: Some(radar.clone()),
        notifications: vec![notification.clone()],
        explosion_lights: lights.clone(),
        static_explosion_lights: lights.clone(),
        full_frame_sprite,
        presentation: Some(GameplayWorldPresentation {
            shield: Some(shield),
            particles,
        }),
    };
    let request = GameplayOverlayRelayout {
        hud: GameplayHudRelayout { previous, next },
        radar_previous,
        radar_next,
    };
    let prepared = overlay.prepare_relayout(request).unwrap();
    assert_eq!(overlay.hud, Some(hud.clone()));
    assert_eq!(
        overlay.radar,
        Some(radar.clone()),
        "preflight publishes no prefix"
    );
    let mut wrong_radar = radar_previous;
    wrong_radar.hud_origin[0] += 1;
    assert!(matches!(
        overlay.prepare_relayout(GameplayOverlayRelayout {
            radar_previous: wrong_radar,
            ..request
        }),
        Err(GameplayOverlayRelayoutError::Radar(
            GameplayRadarRelayoutError::FrameLayoutMismatch
        ))
    ));
    assert_eq!(overlay.hud, Some(hud));
    assert_eq!(
        overlay.radar,
        Some(radar.clone()),
        "a later radar rejection keeps the old HUD too"
    );
    overlay.publish_relayout(prepared);
    assert_eq!(overlay.hud.as_ref().unwrap().virtual_width, 1024);
    assert_eq!(
        overlay.radar.as_ref().unwrap().origin,
        radar_next.hud_origin
    );
    assert_eq!(overlay.radar.as_ref().unwrap().image, radar.image);
    assert_eq!(
        overlay.notifications,
        [notification],
        "typed prefix and notification cadence remain cached"
    );
    assert_eq!(overlay.explosion_lights, lights);
    assert_eq!(overlay.static_explosion_lights, lights);
    assert_eq!(overlay.full_frame_sprite, full_frame_sprite);
    let retained = overlay.presentation.as_ref().unwrap();
    assert_eq!(retained.shield, Some(shield));
    assert_eq!(
        retained.particles, retained_particles,
        "relayout preserves the already-admitted world effects"
    );
}
