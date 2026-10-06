//! Real-data checks for the retail terrain-radar and fullscreen-map resources.

use std::collections::HashSet;
use std::path::PathBuf;

use v2k_game::fullscreen_map_status::FullscreenMapStatusResources;
use v2k_game::gameplay_radar::{GameplayRadar, RadarRect};
use v2k_game::session::GameSession;

fn data_dir() -> PathBuf {
    let dir = v2k_test_support::retail_dir();
    assert!(
        dir.join("PRELOAD.DAT").is_file(),
        "canonical retail corpus required"
    );
    dir
}

fn variant_one_level_session(level_id: u32) -> GameSession {
    let dir = data_dir();
    let mut session = GameSession::init(&dir).expect("PRELOAD");
    session.load_auxiliary_ovl(3, 1).expect("system level 3");
    session.load_level_by_id(level_id, 1).expect("world");
    session
        .cache
        .initialize_level_terrain_radar(&mut || 0)
        .expect("shared terrain radar");
    session
}

fn variant_one_level_one_session() -> GameSession {
    variant_one_level_session(13)
}

fn fnv1a64(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn retail_rgb565_from_disk_rgb555(color: u16) -> u16 {
    ((color & 0x7c00) << 1) | ((color & 0x03e0) << 1) | (color & 0x001f)
}

fn rgba_from_rgb565(color: u16) -> [u8; 4] {
    let red = ((color >> 11) & 0x1f) as u8;
    let green = ((color >> 5) & 0x3f) as u8;
    let blue = (color & 0x1f) as u8;
    [
        (red << 3) | (red >> 2),
        (green << 2) | (green >> 4),
        (blue << 3) | (blue >> 2),
        255,
    ]
}

#[v2k_test_support::retail_test]
fn variant_one_level_one_radar_matches_the_retail_resource_capture() {
    let session = variant_one_level_one_session();

    let cache = &session.cache;
    let authored_scalars = (0..10)
        .map(|index| cache.system_data_value(3, index).expect("L3 radar scalar"))
        .collect::<Vec<_>>();
    assert_eq!(authored_scalars, [96, 96, 4, 5, 3, 50, 2, 7, 464, 464]);
    assert_eq!(cache.system_layout_point(3, 10), Some((516, 350)));
    assert_eq!(
        cache.level_desc().and_then(|level| level.raw_u32(0x50)),
        Some(87),
        "Level 1 radar palette base"
    );
    assert_eq!(
        cache.level_desc().and_then(|level| level.raw_u32(0x80)),
        Some(1),
        "Level 1 underwater terrain-colour override"
    );

    let projection = cache
        .system_radar_projection_tables(3)
        .expect("variant-1 radar projection tables");
    assert_eq!((projection.width, projection.height), (96, 96));
    assert_eq!(
        (projection.half_width(), projection.half_height()),
        (48, 48)
    );
    assert_eq!(projection.shade_grid.len(), 9_216);
    assert_eq!(projection.coordinate_tuples.len(), 2_304);
    assert_eq!(projection.mask_grid.len(), 2_304);
    assert_eq!(
        fnv1a64(projection.shade_grid.iter().copied()),
        0x050e_7dc1_1506_ece9
    );
    assert_eq!(
        fnv1a64(projection.coordinate_tuples.iter().flat_map(|tuple| {
            tuple.samples.iter().flat_map(|sample| {
                sample
                    .x
                    .to_le_bytes()
                    .into_iter()
                    .chain(sample.z.to_le_bytes())
            })
        })),
        0x29cc_6164_7ba2_f525
    );
    assert_eq!(
        fnv1a64(projection.mask_grid.iter().copied()),
        0x0559_f455_9eb2_5648
    );

    let palette_base = 87usize;
    let terrain_colors = (0..32)
        .map(|index| {
            retail_rgb565_from_disk_rgb555(
                cache
                    .global_palette_entry(palette_base + index)
                    .expect("terrain radar palette word")
                    .rgb555,
            )
        })
        .chain([31usize, 34, 33].into_iter().map(|index| {
            retail_rgb565_from_disk_rgb555(
                cache
                    .global_palette_entry(index)
                    .expect("special radar palette word")
                    .rgb555,
            )
        }))
        .collect::<Vec<_>>();
    assert_eq!(
        terrain_colors,
        [
            4492, 15108, 25353, 17350, 31687, 30720, 2377, 10819, 19015, 13060, 25349, 24576, 2246,
            6530, 12676, 8707, 16899, 16384, 67, 2241, 6338, 4353, 8449, 8192, 65503, 63488, 34374,
            1631, 9796, 49672, 9284, 50688, 1, 8452, 33808,
        ]
    );
    assert_eq!(
        (32..48)
            .map(|index| {
                retail_rgb565_from_disk_rgb555(
                    cache
                        .global_palette_entry(palette_base + index)
                        .expect("radar shade word")
                        .rgb555,
                )
            })
            .collect::<Vec<_>>(),
        [
            0, 2113, 4226, 6339, 8452, 10565, 12678, 14791, 16904, 19017, 21130, 23243, 25356,
            27469, 29582, 31695,
        ]
    );
    assert_eq!(
        [24usize, 25, 26, 28, 29, 30, 31].map(|offset| {
            retail_rgb565_from_disk_rgb555(
                cache
                    .global_palette_entry(palette_base + offset)
                    .expect("radar marker word")
                    .rgb555,
            )
        }),
        [65503, 63488, 34374, 9796, 49672, 9284, 50688]
    );

    let mut radar = GameplayRadar::from_cache(cache, 1).expect("Level 1 gameplay radar");
    assert_eq!(radar.virtual_size(), [640, 480]);
    assert_eq!(
        radar.map_rect(),
        RadarRect {
            x: 2,
            y: 7,
            width: 464,
            height: 464,
        }
    );

    for (entity_type, selector, dimensions) in [
        (2usize, 484, (8, 18)),
        (46, 441, (16, 24)),
        (47, 476, (12, 14)),
        (67, 488, (16, 16)),
    ] {
        assert_eq!(
            cache
                .global_entity_type(entity_type)
                .and_then(|record| record.fullscreen_map_icon_sprite_id()),
            Some(selector),
            "entity type {entity_type} icon selector"
        );
        let icon = radar
            .fullscreen_icon(entity_type as u32)
            .expect("decoded fullscreen-map icon");
        assert_eq!((icon.width, icon.height), dimensions);
    }
    for entity_type in [54usize, 61, 62] {
        assert_eq!(
            cache
                .global_entity_type(entity_type)
                .and_then(|record| record.fullscreen_map_icon_sprite_id()),
            None
        );
        assert!(radar.fullscreen_icon(entity_type as u32).is_none());
    }

    radar.enter_fullscreen();
    assert!(radar
        .fullscreen_image(cache.level_terrain_radar().unwrap())
        .is_none());
    for _ in 0..3 {
        radar.advance_fullscreen(0);
    }
    let map = radar
        .fullscreen_image(cache.level_terrain_radar().unwrap())
        .expect("third retail map pass materializes the raster");
    assert_eq!((map.width, map.height), (464, 464));
    assert_eq!(map.rgba.len(), 464 * 464 * 4);
    let captured_terrain_rgba = terrain_colors
        .iter()
        .copied()
        .map(rgba_from_rgb565)
        .collect::<HashSet<_>>();
    assert!(map
        .rgba
        .chunks_exact(4)
        .all(|pixel| captured_terrain_rgba
            .contains(&<[u8; 4]>::try_from(pixel).expect("RGBA pixel"))));
}

#[v2k_test_support::retail_test]
fn variant_one_cistern_radar_resolves_the_canonical_world_six_palette() {
    let session = variant_one_level_session(30);

    let cache = &session.cache;
    assert_eq!(
        cache.level_desc().and_then(|level| level.raw_u32(0x50)),
        Some(327),
        "Cistern radar palette base"
    );
    for local_index in 0..48 {
        assert_eq!(
            cache
                .global_palette_entry_with_source(327 + local_index)
                .map(|(level, local, _)| (level, local)),
            Some((11, local_index)),
            "Cistern palette local {local_index}"
        );
    }

    let mut radar = GameplayRadar::from_cache(cache, 1).expect("Cistern gameplay radar");
    assert_eq!(radar.virtual_size(), [640, 480]);
    assert_eq!(
        radar.map_rect(),
        RadarRect {
            x: 2,
            y: 7,
            width: 464,
            height: 464,
        }
    );
    radar.enter_fullscreen();
    for _ in 0..3 {
        radar.advance_fullscreen(0);
    }
    let map = radar
        .fullscreen_image(cache.level_terrain_radar().unwrap())
        .expect("Cistern fullscreen map raster");
    assert_eq!((map.width, map.height), (464, 464));
    assert_eq!(map.rgba.len(), 464 * 464 * 4);
}

#[v2k_test_support::retail_test]
fn fullscreen_map_status_uses_each_tiers_authored_layout_and_strings() {
    let dir = data_dir();

    for (variant, origin, value_advance, group_advance, expected_baselines) in [
        (
            0,
            (236, 10),
            10,
            15,
            [[236, 10], [236, 20], [236, 35], [236, 45]],
        ),
        (
            1,
            (469, 20),
            20,
            30,
            [[469, 20], [469, 40], [469, 70], [469, 90]],
        ),
    ] {
        let mut session = GameSession::init(&dir).expect("game session");
        session
            .load_auxiliary_ovl(3, variant)
            .expect("system level 3");

        assert_eq!(session.cache.system_layout_point(3, 29), Some(origin));
        assert_eq!(session.cache.system_data_value(3, 12), Some(value_advance));
        assert_eq!(session.cache.system_data_value(3, 11), Some(group_advance));
        assert_eq!(session.cache.global_string(149), Some("Craft"));
        assert_eq!(session.cache.global_string(150), Some("Trophies"));
        assert_eq!(session.cache.global_string(151), Some("Building"));
        assert_eq!(session.cache.global_string(152), Some("new ship"));

        let lines = FullscreenMapStatusResources::from_cache(&session.cache)
            .expect("authored map status resources")
            .lines(0, 1);
        assert_eq!(
            lines.each_ref().map(|line| line.baseline),
            expected_baselines
        );
        assert_eq!(
            lines.map(|line| line.text),
            [
                "Craft".to_owned(),
                "1".to_owned(),
                "Trophies".to_owned(),
                "1".to_owned(),
            ]
        );
    }
}
