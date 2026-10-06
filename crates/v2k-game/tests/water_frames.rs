//! Integration tests: marching-squares shoreline frame resolution against the
//! real game data (ignored when no retail install is configured).
//!
//! Ground truth (workflow 2026-07-02): the 5 shoreline shape sprites live at
//! global sprite index `terrain_sprite_base + 125 ..= +129`, where the base is
//! Section 13 field +0x4C and 125 = local_288(120) + 5 from FUN_00433180.

use v2k_formats::palette::BRIGHTEST_SHADE;
use v2k_game::session::GameSession;
use v2k_render::water::SHORELINE_BASE_OFFSET;

/// Levels with known Section-13 world-resource fields (verified against the
/// raw OVLs): (level, style, terrain_sprite_base). Style chooses the complete
/// system pack, not a second water-only overlay.
const WATER_LEVELS: &[(u32, u32, u32)] = &[
    (22, 6, 3565), // Water — deep sea, Alien tile set
    (33, 6, 3565), // Tidal — Medieval biome + Alien water
    (21, 3, 2507), // Flood — Alpine biome, style-3 water (biome 8 tiles)
    (14, 1, 1616), // Medaeval — Medieval water
];

#[v2k_test_support::retail_test]
fn section13_water_fields_parse() {
    let dir = v2k_test_support::retail_dir();
    for &(level, style, base) in WATER_LEVELS {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_level_by_id(level, 0).unwrap();
        let desc = session.cache.level_desc().unwrap();
        assert_eq!(desc.world_style, style, "level {level} style");
        assert_eq!(
            desc.terrain_sprite_base, base,
            "level {level} terrain_sprite_base"
        );
    }
}

#[v2k_test_support::retail_test]
fn every_world_uses_its_section13_selected_resource_pack() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    for level in 13..=50 {
        session.load_level_by_id(level, 0).unwrap();
        let style = session.cache.level_desc().unwrap().world_style;
        assert!((1..=6).contains(&style), "level {level} style {style}");

        let expected = session.load_ovl_by_id(5 + style, 0).unwrap();
        let expected_objects = expected.anim_frames.as_ref().unwrap();
        let active_objects = session.cache.terrain_objects().unwrap();
        assert_eq!(
            active_objects.records.len(),
            expected_objects.records.len(),
            "level {level} style {style}"
        );
        for (active, expected) in active_objects.records.iter().zip(&expected_objects.records) {
            assert_eq!(
                active.model_ids, expected.model_ids,
                "level {level} style {style}"
            );
            assert_eq!(
                active.kind_index, expected.kind_index,
                "level {level} style {style}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn shoreline_sprites_resolve_and_decode() {
    let dir = v2k_test_support::retail_dir();
    for &(level, _style, base) in WATER_LEVELS {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_level_by_id(level, 0).unwrap();
        let terrain = session.cache.terrain().unwrap();
        assert!(terrain.water_enabled(), "level {level} should have water");

        // Deep-sea styles park the sea at 4096 world units — above the
        // maximum terrain (i8 127 * 32 = 4064) — so only the full-water
        // shape (code 15) can ever be fetched, and the atlas holds 2x2
        // placeholders for shapes 0-3. Shorelines exist iff sea <= 4064.
        let sea16 = terrain.header[0] >> 8;
        let has_shorelines = sea16 <= 4064;

        let mut coverages = Vec::new();
        for k in 0..5u32 {
            let gid = (base + SHORELINE_BASE_OFFSET + k) as u16;
            let (atlas, entry) = session
                .cache
                .global_sprite(gid)
                .unwrap_or_else(|| panic!("level {level}: global sprite {gid} unresolved"));
            let dec = atlas.decode_sprite(entry, BRIGHTEST_SHADE).unwrap();
            if has_shorelines || k == 4 {
                assert_eq!(
                    (dec.width, dec.height),
                    (40, 40),
                    "level {level} frame {k} dims"
                );
            }
            let opaque = dec.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
            coverages.push(opaque as f32 / (dec.width as f32 * dec.height as f32));
        }
        // Full-water shape must be (nearly) fully opaque everywhere.
        assert!(
            coverages[4] > 0.9,
            "level {level}: full-water tile barely opaque: {coverages:?}"
        );
        if has_shorelines {
            // The 5 shapes are a monotonic submersion progression:
            // corner < edge < diagonal < 3-corner < full.
            for w in coverages.windows(2) {
                assert!(
                    w[0] < w[1] + 0.02,
                    "level {level}: coverage not increasing: {coverages:?}"
                );
            }
            assert!(
                coverages[0] < 0.5,
                "level {level}: corner tile too opaque: {coverages:?}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn world_resource_auxiliaries_do_not_stack() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    // Pchannel loads only style pack 11; a following style-1 world must not
    // see level 11's palettes or static-object table (stale-layer shadowing).
    session.load_level_by_id(33, 0).unwrap();
    session.load_level_by_id(14, 0).unwrap();
    let status = session.cache.status();
    assert!(
        !status.contains("0X11XX"),
        "stale water OVL still loaded:\n{status}"
    );
}
