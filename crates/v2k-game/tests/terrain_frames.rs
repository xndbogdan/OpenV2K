//! Real-data checks for the 120 canonical opaque terrain transition sprites.

use v2k_formats::palette::{BRIGHTEST_SHADE, SHADE_LEVELS};
use v2k_game::session::GameSession;
use v2k_render::terrain_tiles::{
    build_lookup, CANONICAL_FRAME_COUNT, COMBINATION_COUNT, INFECTION_BASE_OFFSET,
    INFECTION_FRAME_COUNT,
};

#[v2k_test_support::retail_test]
fn canonical_frames_resolve_for_representative_biomes() {
    let dir = v2k_test_support::retail_dir();
    for level in [14u32, 21, 22] {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_level_by_id(level, 0).unwrap();
        let base = session.cache.level_desc().unwrap().terrain_sprite_base;
        let mut full_tiles = 0;
        for offset in 0..CANONICAL_FRAME_COUNT as u32 {
            let gid = (base + offset) as u16;
            let (atlas, entry) = session
                .cache
                .global_sprite(gid)
                .unwrap_or_else(|| panic!("level {level}: global sprite {gid} unresolved"));
            let sprite = atlas.decode_sprite_opaque(entry, BRIGHTEST_SHADE).unwrap();
            let indexed = atlas.decode_indices(entry).unwrap();
            assert_eq!(
                (indexed.width, indexed.height),
                (sprite.width, sprite.height)
            );
            assert!(
                indexed.indices.iter().all(|&index| index < 16),
                "level {level}: terrain texel exceeds its 16-colour shade row"
            );
            assert_eq!(atlas.palette_row(entry, 0).unwrap().len(), 256);
            assert!(sprite.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
            if (sprite.width, sprite.height) == (40, 40) {
                full_tiles += 1;
            }
        }
        assert!(full_tiles > 0, "level {level}: no full terrain tiles");
    }
}

#[v2k_test_support::retail_test]
fn canonical_sprite_palettes_supply_monotonic_shade_ramps() {
    let dir = v2k_test_support::retail_dir();
    for level in [14u32, 21, 22] {
        let mut session = GameSession::init(&dir).unwrap();
        session.load_level_by_id(level, 0).unwrap();
        let base = session.cache.level_desc().unwrap().terrain_sprite_base;
        let (atlas, entry) = (0..CANONICAL_FRAME_COUNT as u32)
            .filter_map(|offset| session.cache.global_sprite((base + offset) as u16))
            .find(|(atlas, entry)| {
                let sprite = atlas.decode_sprite_opaque(entry, BRIGHTEST_SHADE).unwrap();
                (sprite.width, sprite.height) == (40, 40)
            })
            .unwrap_or_else(|| panic!("level {level}: no 40x40 canonical frame"));
        let luminance: Vec<f32> = (0..SHADE_LEVELS)
            .map(|shade| {
                atlas
                    .decode_sprite_opaque(entry, shade)
                    .unwrap()
                    .rgba
                    .chunks_exact(4)
                    .map(|p| p[0] as f32 * 0.2126 + p[1] as f32 * 0.7152 + p[2] as f32 * 0.0722)
                    .sum()
            })
            .collect();
        assert!(
            luminance[7] < luminance[BRIGHTEST_SHADE],
            "level {level}: shade 7 is not darker"
        );
        for pair in luminance.windows(2) {
            assert!(
                pair[0] <= pair[1] + 1.0,
                "level {level}: non-monotonic ramp"
            );
        }
    }
}

#[test]
fn every_authored_quad_maps_to_an_uploaded_frame() {
    let lookup = build_lookup();
    assert_eq!(lookup.len(), COMBINATION_COUNT);
    assert!(lookup
        .iter()
        .all(|tile| tile.frame < CANONICAL_FRAME_COUNT as u8));
}

#[v2k_test_support::retail_test]
fn infection_transition_frames_are_fixed_shade_marching_squares() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    let mut representative_level = [None; 6];
    for level in 13u32..=50 {
        session.load_level_by_id(level, 0).unwrap();
        let descriptor = session.cache.level_desc().unwrap();
        let style = descriptor.world_style;
        assert!((1..=6).contains(&style), "level {level}: style {style}");
        let style_index = (style - 1) as usize;
        if representative_level[style_index].is_some() {
            continue;
        }
        representative_level[style_index] = Some(level);
        let base = descriptor.terrain_sprite_base;
        let mut coverages = Vec::new();
        let mut render_flags = Vec::new();
        for frame in 0..INFECTION_FRAME_COUNT as u32 {
            let gid = (base + INFECTION_BASE_OFFSET + frame) as u16;
            let (atlas, entry) = session
                .cache
                .global_sprite(gid)
                .unwrap_or_else(|| panic!("level {level}: global sprite {gid} unresolved"));
            let sprite = atlas.decode_sprite(entry, BRIGHTEST_SHADE).unwrap();
            assert_eq!((sprite.width, sprite.height), (40, 40));
            let opaque = sprite
                .rgba
                .chunks_exact(4)
                .filter(|pixel| pixel[3] != 0)
                .count();
            coverages.push(opaque);
            render_flags.push(entry.pal_size & 0x1f);
        }
        // FUN_00430430 uses partial keyed shapes for masks 1..14 and an
        // opaque full tile for mask 15. All five select fixed palette row 28.
        assert_eq!(render_flags, [0x05, 0x05, 0x05, 0x05, 0x04]);
        assert_eq!(coverages, [662, 1282, 1411, 1561, 1600]);
        if representative_level.iter().all(Option::is_some) {
            break;
        }
    }
    assert!(
        representative_level.iter().all(Option::is_some),
        "missing world-style representatives: {representative_level:?}"
    );
}
