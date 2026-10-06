//! Real-data checks for the retail gameplay status-orb resources.

use v2k_formats::models::face_material;
use v2k_game::gameplay_hud::{
    decode_gameplay_hud_sprite, GameplayHudCargoAnimation, GameplayHudCargoFrame,
    GameplayHudLayout, GameplayHudPoint, GameplayHudResources, GameplayHudSprite,
    GameplayHudTimeTrophyLayout, GameplayHudWeaponState, DEFAULT_WEAPON_MODEL_ID,
};
use v2k_game::model_tree::model_is_camera_facing_actor;
use v2k_game::session::GameSession;
use v2k_game::weapon_inventory::{WeaponDescriptor, WEAPON_MASTER_TABLE_RAW};
use v2k_render::WorldSpriteBlend;

fn session_with_common_pool() -> GameSession {
    session_with_common_pool_variant(0)
}

fn session_with_common_pool_variant(variant: u32) -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, variant)
        .expect("retail fixture must load");
    session
}

#[v2k_test_support::retail_test]
fn variant_zero_status_orb_layout_comes_from_level_three_section_one() {
    let session = session_with_common_pool();
    let layout = GameplayHudLayout::from_cache(&session.cache, 0).expect("L3 HUD layout points");

    assert_eq!(
        layout,
        GameplayHudLayout {
            virtual_width: 320,
            virtual_height: 240,
            base: GameplayHudPoint { x: 7, y: 165 },
            outer_frame: GameplayHudPoint { x: 0, y: 0 },
            final_frame: GameplayHudPoint { x: 0, y: 0 },
            inner_frame: GameplayHudPoint { x: 0, y: 0 },
            fuel: GameplayHudPoint { x: 0, y: 1 },
            hull: GameplayHudPoint { x: 40, y: 2 },
            digits: GameplayHudPoint { x: 22, y: 8 },
            status_halves: GameplayHudPoint { x: 20, y: 8 },
            center_shade: GameplayHudPoint { x: 0, y: 0 },
            weapon_fallback: GameplayHudPoint { x: 22, y: 22 },
            weapon_model: GameplayHudPoint { x: 37, y: 33 },
            weapon_spacing: GameplayHudPoint { x: 0, y: 20 },
            cargo_origin: GameplayHudPoint { x: 37, y: 72 },
            cargo_spacing: GameplayHudPoint { x: 10, y: 0 },
            time_trophy: GameplayHudTimeTrophyLayout {
                model: GameplayHudPoint { x: 30, y: 15 },
                text: GameplayHudPoint { x: 45, y: 12 },
            },
        }
    );
}

#[v2k_test_support::retail_test]
fn variant_one_uses_the_authored_high_resolution_orb_and_composition() {
    let session = session_with_common_pool_variant(1);
    let layout = GameplayHudLayout::from_cache(&session.cache, 1).expect("1X3XX HUD layout");
    assert_eq!((layout.virtual_width, layout.virtual_height), (640, 480));
    assert_eq!(layout.base, GameplayHudPoint { x: 20, y: 350 });
    assert_eq!(layout.fuel, GameplayHudPoint { x: 0, y: 2 });
    assert_eq!(layout.hull, GameplayHudPoint { x: 57, y: 3 });
    assert_eq!(layout.weapon_model, GameplayHudPoint { x: 55, y: 50 });

    let resources = GameplayHudResources::from_cache(&session.cache).expect("1X3XX HUD sprites");
    for (sprite, size) in [
        (GameplayHudSprite::OuterFrame, (92, 91)),
        (GameplayHudSprite::InnerFrame, (92, 91)),
        (GameplayHudSprite::Fuel, (34, 86)),
        (GameplayHudSprite::Hull, (34, 86)),
        (GameplayHudSprite::StatusLeft, (22, 22)),
        (GameplayHudSprite::StatusRight, (22, 22)),
        (GameplayHudSprite::CenterShade, (96, 96)),
    ] {
        let decoded = resources.sprite(sprite);
        assert_eq!((decoded.width, decoded.height), size, "sprite {sprite:?}");
    }
}

#[v2k_test_support::retail_test]
fn invalid_system_variant_cannot_mix_cached_points_with_guessed_dimensions() {
    let session = session_with_common_pool();

    assert!(GameplayHudLayout::from_cache(&session.cache, 4).is_none());
    assert!(GameplayHudLayout::from_cache(&session.cache, u32::MAX).is_none());
}

#[v2k_test_support::retail_test]
fn default_selected_weapon_uses_the_captured_model_and_fallback_resources() {
    let session = session_with_common_pool();
    let model = session
        .cache
        .global_model(DEFAULT_WEAPON_MODEL_ID)
        .expect("captured HUD weapon model 115");
    assert_eq!(model.name.as_deref(), Some("hudweap"));
    assert_eq!(model.vertices.len(), 16);
    assert_eq!(model.triangles.len(), 28);
    assert!(model.instances.is_empty());
    assert!(model.billboards.is_empty());
    assert!(model.shadow_triangles.is_empty());

    let resources = GameplayHudResources::from_cache(&session.cache).expect("L3 HUD resources");
    let fallback = resources.sprite(GameplayHudSprite::WeaponFallback);
    assert_eq!(GameplayHudSprite::WeaponFallback.global_id(), 518);
    assert!(!fallback.rgba.is_empty());
    assert!(fallback.width > 0 && fallback.height > 0);
}

#[v2k_test_support::retail_test]
fn recovered_weapon_hud_sprites_use_the_descriptor_high_word() {
    let selector5 = GameplayHudWeaponState::from_descriptor(&WeaponDescriptor::from_raw(
        WEAPON_MASTER_TABLE_RAW[6],
    ));
    assert_eq!(selector5.selector, 5);
    assert_eq!(selector5.fallback_sprite, Some(520));

    let mut session = session_with_common_pool_variant(1);
    session
        .load_auxiliary_ovl(3, 1)
        .expect("high-resolution HUD pool");
    let decoded = decode_gameplay_hud_sprite(&session.cache, 520).expect("sprite 520 in overlay 3");
    assert!(decoded.width > 0 && decoded.height > 0);
    assert!(!decoded.rgba.is_empty());
}

#[v2k_test_support::retail_test]
fn carried_peasant_materializes_the_retail_four_phase_walk_cycle() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("game session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("high-resolution HUD pool");
    session
        .load_level_by_id(13, 1)
        .expect("high-resolution level-one resources");

    let peasant = session
        .cache
        .global_model(558)
        .expect("global model 558 man2");
    assert_eq!(peasant.name.as_deref(), Some("man2"));
    let weight = session
        .cache
        .global_model(81)
        .expect("global model 81 weight");
    assert_eq!(weight.name.as_deref(), Some("weight"));
    assert!(model_is_camera_facing_actor(peasant));
    assert!(!model_is_camera_facing_actor(weight));

    let cargo_at_first_angle = GameplayHudCargoFrame {
        global_model_id: Some(558),
        marker: GameplayHudSprite::CargoEmpty,
        marker_position: GameplayHudPoint { x: 0, y: 0 },
        model_position: GameplayHudPoint { x: 0, y: 0 },
        spin_raw: 0x1000,
        animation: GameplayHudCargoAnimation::from_retail_tick(0),
    };
    let cargo_at_second_angle = GameplayHudCargoFrame {
        spin_raw: 0x5000,
        ..cargo_at_first_angle
    };
    assert_eq!(
        cargo_at_first_angle.orientation_for_model(peasant),
        cargo_at_second_angle.orientation_for_model(peasant),
        "flat cargo remains face-on while its animation advances"
    );
    assert_eq!(
        cargo_at_first_angle.orientation_for_model(peasant),
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        "the peasant's local +Z face points into the fixed HUD camera"
    );
    assert_ne!(
        cargo_at_first_angle.orientation_for_model(weight),
        cargo_at_second_angle.orientation_for_model(weight),
        "rigid cargo retains the shared 3D spin"
    );

    for (retail_tick, expected_sprite) in [
        (0, 1_341),
        (6, 1_342),
        (12, 1_341),
        (18, 1_342),
        (24, 1_341),
    ] {
        let vars = GameplayHudCargoAnimation::from_retail_tick(retail_tick).anim_vars();
        let materialized = peasant.materialize(&vars);
        assert_eq!(materialized.triangles.len(), 4);
        assert_eq!(
            materialized
                .triangles
                .iter()
                .enumerate()
                .filter(|(_, triangle)| {
                    !triangle
                        .iter()
                        .all(|&index| materialized.vertex_type_flags[index as usize] == 13)
                })
                .map(|(index, _)| face_material(materialized.face_materials[index]))
                .collect::<Vec<_>>(),
            vec![(expected_sprite, true); 2],
            "retail cargo phase at tick {retail_tick}"
        );
    }
}

#[v2k_test_support::retail_test]
fn status_orb_and_cargo_decode_the_nine_exact_global_sprites() {
    let session = session_with_common_pool();
    let resources = GameplayHudResources::from_cache(&session.cache).expect("L3 HUD sprites");

    for (sprite, size, flags, blend) in [
        (
            GameplayHudSprite::FinalFrame,
            (62, 61),
            0x0d,
            WorldSpriteBlend::HalfAdditive,
        ),
        (
            GameplayHudSprite::OuterFrame,
            (62, 61),
            0x05,
            WorldSpriteBlend::Masked,
        ),
        (
            GameplayHudSprite::InnerFrame,
            (62, 61),
            0x05,
            WorldSpriteBlend::Masked,
        ),
        (
            GameplayHudSprite::Fuel,
            (23, 58),
            0x15,
            WorldSpriteBlend::Additive,
        ),
        (
            GameplayHudSprite::Hull,
            (23, 58),
            0x15,
            WorldSpriteBlend::Additive,
        ),
        (
            GameplayHudSprite::StatusLeft,
            (11, 11),
            0x05,
            WorldSpriteBlend::Masked,
        ),
        (
            GameplayHudSprite::StatusRight,
            (11, 11),
            0x05,
            WorldSpriteBlend::Masked,
        ),
        (
            GameplayHudSprite::CenterShade,
            (64, 64),
            0x15,
            WorldSpriteBlend::Additive,
        ),
        (
            GameplayHudSprite::CargoEmpty,
            (6, 6),
            0x05,
            WorldSpriteBlend::Masked,
        ),
    ] {
        let (atlas, entry) = session
            .cache
            .global_sprite(sprite.global_id())
            .expect("L3 HUD sprite metadata");
        assert_eq!(entry.pal_size as u8, flags, "sprite {sprite:?} flags");
        let decoded = resources.sprite(sprite);
        assert_eq!((decoded.width, decoded.height), size, "sprite {sprite:?}");
        assert_eq!(decoded.blend, blend, "sprite {sprite:?} blend");
        assert_eq!(
            decoded.rgba.len(),
            (decoded.width * decoded.height * 4) as usize,
            "sprite {sprite:?} RGBA length"
        );

        // All nine records have flag 0x04, so the near textured-billboard
        // filler advances its palette pointer by 0x380 bytes: shade row 28.
        let indices = atlas.decode_indices(entry).expect("HUD indices");
        let palette = atlas.palette_row(entry, 28).expect("HUD row 28");
        let pixel = indices
            .indices
            .iter()
            .position(|&index| index != 0)
            .expect("HUD sprite foreground");
        let expected = palette[usize::from(indices.indices[pixel])];
        assert_eq!(
            &decoded.rgba[pixel * 4..pixel * 4 + 4],
            &expected,
            "sprite {sprite:?} fixed shade"
        );
    }
}

#[v2k_test_support::retail_test]
fn status_orb_cutouts_come_only_from_authored_zero_key_texels() {
    let session = session_with_common_pool();
    let resources = GameplayHudResources::from_cache(&session.cache).expect("L3 HUD sprites");

    // These counts come directly from level 3's Section-3 atlas rectangles.
    // Pinning both the raw coverage and decoded alpha prevents a future
    // palette-layout heuristic from either filling the intended aperture or
    // punching new holes through non-zero artwork.
    for (sprite, zero_texels, visible_texels) in [
        (GameplayHudSprite::OuterFrame, 2_844, 938),
        (GameplayHudSprite::InnerFrame, 2_419, 1_363),
        (GameplayHudSprite::Hull, 917, 417),
        (GameplayHudSprite::StatusLeft, 36, 85),
        (GameplayHudSprite::StatusRight, 38, 83),
        (GameplayHudSprite::CenterShade, 2_913, 1_183),
    ] {
        let (atlas, entry) = session
            .cache
            .global_sprite(sprite.global_id())
            .expect("L3 HUD sprite metadata");
        assert!(entry.is_zero_keyed(), "sprite {sprite:?} flag bit 0");

        let indices = atlas.decode_indices(entry).expect("HUD indices");
        assert_eq!(
            indices.indices.iter().filter(|&&index| index == 0).count(),
            zero_texels,
            "sprite {sprite:?} authored cutout coverage"
        );
        assert_eq!(
            indices.indices.iter().filter(|&&index| index != 0).count(),
            visible_texels,
            "sprite {sprite:?} authored visible coverage"
        );

        let decoded = resources.sprite(sprite);
        assert_eq!(decoded.rgba.len() / 4, indices.indices.len());
        for (&index, pixel) in indices.indices.iter().zip(decoded.rgba.chunks_exact(4)) {
            assert_eq!(
                pixel[3] == 0,
                index == 0,
                "sprite {sprite:?} index {index} alpha"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn time_trophy_uses_each_tiers_absolute_authored_points_and_disappears_with_its_clock() {
    use v2k_game::gameplay_hud::{GameplayHud, GameplayHudWeaponRoster};
    use v2k_game::power_up_contact::PlayerCampaignProgress;
    use v2k_game::time_trophy::TimeTrophyRuntime;

    for (variant, model, text) in [
        (0, (30, 15), (45, 12)),
        (1, (60, 30), (90, 25)),
        (2, (75, 38), (113, 32)),
        (3, (60, 30), (90, 25)),
    ] {
        let session = session_with_common_pool_variant(variant);
        let layout = GameplayHudLayout::from_cache(&session.cache, variant).unwrap();
        assert_eq!(
            (layout.time_trophy.model.x, layout.time_trophy.model.y),
            model
        );
        assert_eq!((layout.time_trophy.text.x, layout.time_trophy.text.y), text);
        assert!(session.cache.global_model(138).is_some());
        let resources = GameplayHudResources::from_cache(&session.cache).unwrap();
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(3));
        let (mut timer, _) =
            TimeTrophyRuntime::from_loaded_world(300, false, &mut progress).unwrap();
        let mut hud = GameplayHud::default();
        let roster = GameplayHudWeaponRoster::default();
        let frame = hud.frame(
            &resources,
            layout,
            200_000,
            40_000,
            222,
            0,
            &roster,
            &[],
            timer,
        );
        assert_eq!(frame.time_trophy.as_ref().unwrap().text, "5:00");
        timer.advance(300_000_000);
        let frame = hud.frame(
            &resources,
            layout,
            200_000,
            40_000,
            222,
            0,
            &roster,
            &[],
            timer,
        );
        assert_eq!(frame.time_trophy.as_ref().unwrap().text, "0:00");
        timer.advance(500_000);
        let frame = hud.frame(
            &resources,
            layout,
            200_000,
            40_000,
            222,
            0,
            &roster,
            &[],
            timer,
        );
        assert!(
            frame.time_trophy.is_none(),
            "icon and clock must disappear together"
        );
        assert!(!frame.weapons.is_empty(), "ordinary HUD remains visible");
    }
}
