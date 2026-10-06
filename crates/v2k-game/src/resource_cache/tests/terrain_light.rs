use super::*;
use crate::gameplay_radar::TerrainRadarError;
use crate::static_damage::{StaticDamageAction, StaticDamageScheduler, StaticGroundProgramOutcome};

#[test]
fn static_terrain_light_clamps_all_authored_values_and_preserves_every_low_bit() {
    let cell = [255, 2];
    let original = TerrainCell {
        height: 0x92,
        attribute: 0xb3,
        terrain_type: 0,
    };
    let mut cache = ResourceCache::new(Vec::new());
    cache.load_level(level_state_with_cell(cell, original));
    let index = usize::from(cell[0]) * 256 + usize::from(cell[1]);
    for raw in 0..=u8::MAX {
        for amount in [i32::MIN, -1, 0, 1, 3, 8, i32::MAX] {
            cache.level_terrain_mut().unwrap().cells[index].terrain_type = raw;
            let changed = amount > 0 && raw >> 5 != 0;
            let result = cache.lower_level_terrain_light(cell, amount, &mut || {
                panic!("uninitialized radar consumed RNG")
            });
            assert_eq!(
                result,
                if changed {
                    Err(TerrainRadarError::Uninitialized)
                } else {
                    Ok(false)
                }
            );
            let actual = cache.level_terrain().unwrap().cells[index];
            let expected_light = if amount > 0 {
                i32::from(raw >> 5).saturating_sub(amount).max(0) as u8
            } else {
                raw >> 5
            };
            assert_eq!(actual.terrain_type, (expected_light << 5) | (raw & 31));
            assert_eq!(actual.height, original.height);
            assert_eq!(actual.attribute, original.attribute);
            assert_eq!(
                cache.take_level_terrain_presentation_dirty(),
                changed,
                "raw={raw}, amount={amount}"
            );
            assert!(!cache.take_level_terrain_presentation_dirty());
        }
    }
}

#[test]
fn static_terrain_light_never_falls_back_to_preload_or_missing_cells() {
    let cell = [31, 47];
    let original = TerrainCell {
        height: 7,
        attribute: 4,
        terrain_type: 0xff,
    };
    let mut cache = ResourceCache::new(vec![level_state_with_cell(cell, original)]);
    let mut no_rng = || panic!("missing live terrain consumed RNG");
    assert!(cache
        .lower_level_terrain_light(cell, 1, &mut no_rng)
        .is_err());
    let mut missing = level_state_with_cell(cell, original);
    missing.terrain = None;
    cache.load_level(missing);
    assert!(cache
        .lower_level_terrain_light(cell, 1, &mut no_rng)
        .is_err());
    let mut short = level_state_with_cell(cell, original);
    short.terrain.as_mut().unwrap().cells.clear();
    cache.load_level(short);
    assert!(cache
        .lower_level_terrain_light(cell, 1, &mut no_rng)
        .is_err());
    assert_eq!(
        cache.preload()[0]
            .terrain
            .as_ref()
            .unwrap()
            .cell(31, 47)
            .unwrap()
            .terrain_type,
        0xff
    );
    assert!(!cache.take_level_terrain_presentation_dirty());
}

#[test]
fn static_terrain_light_dirty_request_follows_abort_commit_and_level_lifetime() {
    let cell = [31, 47];
    let original = TerrainCell {
        height: 7,
        attribute: 0,
        terrain_type: 0xff,
    };
    let mut cache = ResourceCache::new(Vec::new());
    cache.load_level(level_state_with_cell(cell, original));
    {
        let mut transaction = cache.begin_main_base_abort_resource_transaction();
        assert_eq!(
            transaction.lower_level_terrain_light(cell, 1, &mut || 0),
            Err(TerrainRadarError::Uninitialized)
        );
    }
    assert!(!cache.take_level_terrain_presentation_dirty());
    assert_eq!(
        cache
            .level_terrain()
            .unwrap()
            .cell(31, 47)
            .unwrap()
            .terrain_type,
        0xff
    );
    {
        let mut transaction = cache.begin_main_base_abort_resource_transaction();
        assert_eq!(
            transaction.lower_level_terrain_light(cell, 1, &mut || 0),
            Err(TerrainRadarError::Uninitialized)
        );
        transaction.commit();
    }
    assert_eq!(
        cache
            .level_terrain()
            .unwrap()
            .cell(31, 47)
            .unwrap()
            .terrain_type,
        0xdf
    );
    {
        let mut transaction = cache.begin_main_base_abort_resource_transaction();
        assert!(transaction.take_level_terrain_presentation_dirty());
    }
    assert!(
        cache.take_level_terrain_presentation_dirty(),
        "rollback restores a previously pending request"
    );
    assert_eq!(
        cache.lower_level_terrain_light(cell, 1, &mut || 0),
        Err(TerrainRadarError::Uninitialized)
    );
    cache.unload_level();
    assert!(!cache.take_level_terrain_presentation_dirty());
    cache.load_level(level_state_with_cell(cell, original));
    assert!(!cache.take_level_terrain_presentation_dirty());
}

#[v2k_test_support::retail_test]
fn fireball_ground_program_mutates_next_pass_and_refreshes_covered_radar_once() {
    let dir = v2k_test_support::retail_dir();
    let mut session = crate::session::GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    session
        .cache
        .initialize_level_terrain_radar(&mut || 0)
        .unwrap();
    // Canonical Intro2 kind8 footprint at (64,235), retained before mutation.
    let cell = [64, 235];
    let index = usize::from(cell[0]) * 256 + usize::from(cell[1]);
    session.cache.level_terrain_mut().unwrap().cells[index].terrain_type = 0x3f;
    let mut scheduler = StaticDamageScheduler::new();
    assert!(scheduler.advance(40_000, |_| None).is_empty());
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Started
    );
    assert_eq!(
        scheduler.submit_fireball_ground_program(cell),
        StaticGroundProgramOutcome::Duplicate
    );
    assert_eq!(
        session.cache.level_terrain().unwrap().cells[index].terrain_type,
        0x3f
    );
    assert!(!session.cache.take_level_terrain_presentation_dirty());
    let mut draws = 0;
    for action in scheduler.advance(0, |_| None) {
        let StaticDamageAction::LowerTerrainLight { cell, amount } = action else {
            panic!("unexpected fireball action")
        };
        assert_eq!(
            session
                .cache
                .lower_level_terrain_light(cell, amount, &mut || {
                    draws += 1;
                    0
                }),
            Ok(true)
        );
    }
    assert_eq!(draws, 1);
    assert_eq!(
        session.cache.level_terrain().unwrap().cells[index].terrain_type,
        0x1f
    );
    assert!(session.cache.take_level_terrain_presentation_dirty());
    assert_eq!(
        session
            .cache
            .lower_level_terrain_light(cell, 1, &mut || panic!("zero light refreshed radar")),
        Ok(false)
    );
    assert!(!session.cache.take_level_terrain_presentation_dirty());
}
