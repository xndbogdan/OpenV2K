//! The loaded world owns the radar raster independently of HUD/map lifetime.

use v2k_game::{
    gameplay_radar::{GameplayRadar, TerrainRadarView},
    resource_cache::ResourceCache,
    session::GameSession,
};

fn session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    assert!(
        dir.join("PRELOAD.DAT").is_file(),
        "shared radar tests require the retail corpus; run workspace-doctor -RequireRetailData"
    );
    let mut session = GameSession::init(&dir).expect("retail PRELOAD.DAT");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal-resolution presentation resources");
    session
}

fn covered_cells(radar: TerrainRadarView<'_>) -> usize {
    radar
        .packed_coverage()
        .iter()
        .map(|byte| usize::from(byte & 15 != 0) + usize::from(byte >> 4 != 0))
        .sum()
}

fn open_nearest_map(cache: &ResourceCache) -> GameplayRadar {
    let mut radar = GameplayRadar::from_cache(cache, 1).expect("shared-raster map resources");
    radar.enter_fullscreen();
    for _ in 0..3 {
        radar.advance_fullscreen(0);
    }
    radar
}

/// Choose an authored covered cell whose light contributes to its radar color.
/// Keep the adjacent material equal so 37100's half-cell-X refresh differs only
/// in light from the integer-cell sample used by the initial 4AB20 raster.
fn light_cell(cache: &ResourceCache) -> [u8; 2] {
    let radar = cache.level_terrain_radar().unwrap();
    let terrain = cache.level_terrain().unwrap();
    let objects = cache.terrain_objects().unwrap();
    for x in 1..255 {
        for z in 1..256 {
            if x % 32 == 0 || z % 32 == 0 {
                continue;
            }
            let linear = z * 256 + x;
            let coverage = (radar.packed_coverage()[linear >> 1] >> ((linear & 1) * 4)) & 15;
            let cell = terrain.cell(x, z).unwrap();
            let adjacent = terrain.cell(x + 1, z).unwrap();
            let static_radar = cell.attribute != 0
                && cell.terrain_type & 0x18 == 0
                && objects.records[usize::from(cell.attribute)].kind_index == 8;
            if coverage != 0
                && cell.terrain_type >> 5 >= 2
                && cell.terrain_type & 7 == adjacent.terrain_type & 7
                && !static_radar
            {
                return [x as u8, z as u8];
            }
        }
    }
    panic!("loaded corpus world has no covered authored light sample");
}

#[v2k_test_support::retail_test]
fn ordinary_and_intro2_light_changes_reach_existing_and_recreated_maps() {
    let mut session = session();
    for level_id in [13, 22, 50] {
        session.load_level_by_id(level_id, 1).unwrap();
        assert!(session.cache.level_terrain_radar().is_none());
        assert!(GameplayRadar::from_cache(&session.cache, 1).is_none());
        let mut draws = 0;
        let initialized = session
            .cache
            .initialize_level_terrain_radar(&mut || {
                draws += 1;
                0
            })
            .unwrap();
        let initial = session.cache.level_terrain_radar().unwrap();
        assert_eq!(initialized, covered_cells(initial), "level {level_id}");
        assert_eq!(initialized, draws);
        assert!(
            draws > 0,
            "level {level_id} must exercise covered refreshes"
        );
        let coverage = initial.packed_coverage().to_vec();
        let indices = initial.indices().to_vec();
        let revision = initial.revision();
        let mut presentation = open_nearest_map(&session.cache);
        let before = presentation.fullscreen_image(initial).unwrap().clone();

        let cell = light_cell(&session.cache);
        let old_cell = *session
            .cache
            .level_terrain()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(
            session.cache.lower_level_terrain_light(cell, 7, &mut || {
                draws += 1;
                0
            }),
            Ok(true),
            "ordinary and Intro2 worlds must both own the refresh allocation"
        );
        assert_eq!(draws, initialized + 1);
        let changed = session.cache.level_terrain_radar().unwrap();
        assert_eq!(changed.packed_coverage(), coverage);
        assert_eq!(changed.revision(), revision + 1);
        let changed_cells = indices
            .iter()
            .zip(changed.indices())
            .enumerate()
            .filter_map(|(index, (before, after))| (before != after).then_some(index))
            .collect::<Vec<_>>();
        assert_eq!(
            changed_cells,
            [usize::from(cell[1]) * 256 + usize::from(cell[0])],
            "level {level_id}: refresh the same Z-major raster"
        );
        let after = presentation.fullscreen_image(changed).unwrap().clone();
        assert_ne!(before, after, "level {level_id}: cached map must refresh");
        let mut recreated = open_nearest_map(&session.cache);
        assert_eq!(recreated.fullscreen_image(changed), Some(&after));
        assert_eq!(
            draws,
            initialized + 1,
            "map creation never rasterizes terrain"
        );

        assert_eq!(
            session.cache.initialize_level_terrain_radar(&mut || {
                panic!("resuming level {level_id} repeated raster RNG")
            }),
            Ok(0)
        );
        let resumed = session.cache.level_terrain_radar().unwrap();
        assert_eq!(resumed.revision(), revision + 1);
        assert_eq!(presentation.fullscreen_image(resumed), Some(&after));
        let new_cell = session
            .cache
            .level_terrain()
            .unwrap()
            .cell(usize::from(cell[0]), usize::from(cell[1]))
            .unwrap();
        assert_eq!(new_cell.terrain_type, old_cell.terrain_type & 31);
        assert_eq!(new_cell.height, old_cell.height);
        assert_eq!(new_cell.attribute, old_cell.attribute);
        assert!(session.cache.take_level_terrain_presentation_dirty());
    }
}

#[v2k_test_support::retail_test]
fn replacing_and_reloading_worlds_discards_the_previous_raster() {
    let mut session = session();
    let mut first_world = None;
    for level_id in [13, 50, 22, 13] {
        session.load_level_by_id(level_id, 1).unwrap();
        assert!(session.cache.level_terrain_radar().is_none());
        let draws = session
            .cache
            .initialize_level_terrain_radar(&mut || 0)
            .unwrap();
        let radar = session.cache.level_terrain_radar().unwrap();
        assert_eq!(radar.revision(), 1);
        assert_eq!(draws, covered_cells(radar));
        let snapshot = (radar.packed_coverage().to_vec(), radar.indices().to_vec());
        if level_id == 13 {
            if let Some(original) = &first_world {
                assert_eq!(
                    &snapshot, original,
                    "reload restores authored terrain/radar"
                );
            } else {
                first_world = Some(snapshot);
            }
        }
        let cell = light_cell(&session.cache);
        session
            .cache
            .lower_level_terrain_light(cell, 7, &mut || 0)
            .unwrap();
        assert_eq!(session.cache.level_terrain_radar().unwrap().revision(), 2);
    }
    session.cache.unload_level();
    assert!(session.cache.level_terrain_radar().is_none());
    assert!(!session.cache.take_level_terrain_presentation_dirty());
    assert!(session
        .cache
        .initialize_level_terrain_radar(&mut || panic!("unloaded world consumed radar RNG"))
        .is_err());
    assert!(GameplayRadar::from_cache(&session.cache, 1).is_none());
}
