//! Portable Radar's recovered task kernel against authored ordinary worlds.
//! These tests exercise shared coverage/resources, not native actor custody.

use v2k_game::gameplay_radar::{GameplayRadar, TerrainRadarError};
use v2k_game::portable_radar::{
    destroy_portable_radar, tick_portable_radar, PortableRadarFrame, PortableRadarTickOutcome,
};
use v2k_game::resource_cache::ResourceCache;
use v2k_game::retail_rng::retail_random_u16;
use v2k_game::session::GameSession;

const PORTABLE_RADAR_WORLDS: [u32; 6] = [30, 39, 40, 42, 46, 47];

fn world(level_id: u32) -> GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "canonical retail corpus required"
    );
    let mut session = GameSession::init(&data).expect("PRELOAD");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system radar");
    session
        .load_level_by_id(level_id, 1)
        .expect("authored world");
    session
        .cache
        .initialize_level_terrain_radar(&mut || 0)
        .expect("shared radar initialization");
    session
}

fn authored_positions(session: &GameSession) -> Vec<[i16; 3]> {
    session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .filter(|spawn| spawn.entity_type == 83)
        .map(|spawn| spawn.position_raw())
        .collect()
}

fn terminal_frame(position_raw: [i16; 3]) -> PortableRadarFrame {
    PortableRadarFrame {
        state_flags_raw: 0x80,
        elapsed_micros: 16,
        position_raw,
    }
}

fn deploy(
    sub_k1: &mut u16,
    position_raw: [i16; 3],
    resources: &mut ResourceCache,
    next_random: &mut impl FnMut() -> u16,
) -> usize {
    match tick_portable_radar(sub_k1, terminal_frame(position_raw), resources, next_random)
        .expect("terminal deployment")
    {
        PortableRadarTickOutcome::Deployed { random_draws } => random_draws,
        other => panic!("expected first deployment, got {other:?}"),
    }
}

#[v2k_test_support::retail_test]
fn all_fourteen_authored_portable_radars_restore_packed_coverage_after_destruction() {
    let mut births = 0;
    for level_id in PORTABLE_RADAR_WORLDS {
        let mut session = world(level_id);
        let positions = authored_positions(&session);
        assert!(!positions.is_empty(), "world {level_id} has Type83 births");
        births += positions.len();
        for position in positions {
            let before = session.cache.level_terrain_radar().unwrap();
            let original_coverage = before.packed_coverage().to_vec();
            let original_revision = before.revision();
            let mut sub_k1 = u16::MAX - 1;
            let mut random_state = 0x1357_2468;
            let mut calls = 0;
            let deployed_draws = deploy(&mut sub_k1, position, &mut session.cache, &mut || {
                calls += 1;
                retail_random_u16(&mut random_state)
            });
            assert_eq!(sub_k1, u16::MAX);
            assert_eq!(deployed_draws, calls);
            assert!(deployed_draws > 0, "world {level_id} at {position:?}");
            let deployed = session.cache.level_terrain_radar().unwrap();
            assert_ne!(deployed.packed_coverage(), original_coverage);
            assert_eq!(deployed.revision(), original_revision + 1);
            let mut removal_calls = 0;
            let removed_draws =
                destroy_portable_radar(&mut sub_k1, position, &mut session.cache, &mut || {
                    removal_calls += 1;
                    retail_random_u16(&mut random_state)
                })
                .expect("deployed destructor");
            assert_eq!(removed_draws, removal_calls);
            assert_eq!(sub_k1, 0);
            let restored = session.cache.level_terrain_radar().unwrap();
            assert_eq!(
                restored.packed_coverage(),
                original_coverage,
                "world {level_id} at {position:?}"
            );
            assert_eq!(restored.revision(), original_revision + 2);
            // Raster shades are refreshed with new process words; destruction
            // restores coverage, not an old cached image or RNG snapshot.
            assert_ne!(random_state, 0x1357_2468);
        }
    }
    assert_eq!(births, 14, "canonical Type83 ordinary-world census");
}

#[v2k_test_support::retail_test]
fn fullscreen_presenter_observes_authored_portable_coverage_addition_and_removal() {
    let mut observed_image_change = false;
    for level_id in PORTABLE_RADAR_WORLDS {
        let mut session = world(level_id);
        let mut presenter = GameplayRadar::from_cache(&session.cache, 1).expect("gameplay radar");
        presenter.enter_fullscreen();
        for _ in 0..3 {
            presenter.advance_fullscreen(0);
        }
        for position in authored_positions(&session) {
            let baseline = presenter
                .fullscreen_image(session.cache.level_terrain_radar().unwrap())
                .unwrap()
                .clone();
            let mut sub_k1 = u16::MAX - 1;
            deploy(&mut sub_k1, position, &mut session.cache, &mut || 0);
            let deployed = presenter
                .fullscreen_image(session.cache.level_terrain_radar().unwrap())
                .unwrap()
                .clone();
            destroy_portable_radar(&mut sub_k1, position, &mut session.cache, &mut || 0).unwrap();
            let removed = presenter
                .fullscreen_image(session.cache.level_terrain_radar().unwrap())
                .unwrap();
            assert_eq!(
                removed, &baseline,
                "constant material RNG makes the restored world image exact"
            );
            if deployed != baseline {
                assert_ne!(removed, &deployed);
                observed_image_change = true;
                break;
            }
        }
        if observed_image_change {
            break;
        }
    }
    assert!(
        observed_image_change,
        "at least one authored Type83 footprint must change the live fullscreen map"
    );
}

#[v2k_test_support::retail_test]
fn overlapping_portable_radars_retain_the_surviving_footprint_on_partial_removal() {
    let mut session = world(30);
    let first_position = authored_positions(&session)[0];
    let mut second_position = first_position;
    second_position[0] = second_position[0].wrapping_add(256);
    let original = session
        .cache
        .level_terrain_radar()
        .unwrap()
        .packed_coverage()
        .to_vec();
    let mut first_k1 = u16::MAX - 1;
    let mut second_k1 = u16::MAX - 1;
    deploy(&mut first_k1, first_position, &mut session.cache, &mut || 0);
    let first_only = session
        .cache
        .level_terrain_radar()
        .unwrap()
        .packed_coverage()
        .to_vec();
    deploy(
        &mut second_k1,
        second_position,
        &mut session.cache,
        &mut || 0,
    );
    assert_ne!(
        session
            .cache
            .level_terrain_radar()
            .unwrap()
            .packed_coverage(),
        first_only
    );
    let removal_draws = destroy_portable_radar(
        &mut second_k1,
        second_position,
        &mut session.cache,
        &mut || 0,
    )
    .unwrap();
    assert!(
        removal_draws > 0,
        "remaining coverage still refreshes with RNG"
    );
    assert_eq!(first_k1, u16::MAX);
    assert_eq!(second_k1, 0);
    assert_eq!(
        session
            .cache
            .level_terrain_radar()
            .unwrap()
            .packed_coverage(),
        first_only
    );
    assert_ne!(first_only, original);
    destroy_portable_radar(&mut first_k1, first_position, &mut session.cache, &mut || 0).unwrap();
    assert_eq!(
        session
            .cache
            .level_terrain_radar()
            .unwrap()
            .packed_coverage(),
        original
    );
}

#[test]
fn missing_shared_radar_retains_the_retail_task_prefix_at_deployment_and_destruction() {
    let mut resources = ResourceCache::new(vec![]);
    let position = [1, 2, 3];
    let mut sub_k1 = 0;
    assert!(matches!(
        tick_portable_radar(
            &mut sub_k1,
            terminal_frame(position),
            &mut resources,
            &mut || panic!("partial deployment draws no random words"),
        ),
        Ok(PortableRadarTickOutcome::Deploying)
    ));
    assert_eq!(sub_k1, 1);
    assert_eq!(
        destroy_portable_radar(&mut sub_k1, position, &mut resources, &mut || panic!(
            "partial destructor draws no random words"
        )),
        Ok(0)
    );
    assert_eq!(sub_k1, 0);
    sub_k1 = u16::MAX - 1;
    assert!(matches!(
        tick_portable_radar(
            &mut sub_k1,
            terminal_frame(position),
            &mut resources,
            &mut || panic!("missing raster rejects before RNG"),
        ),
        Err(TerrainRadarError::Uninitialized)
    ));
    assert_eq!(sub_k1, u16::MAX, "405610 writes K1 before calling 44A8D0");
    assert!(matches!(
        tick_portable_radar(
            &mut sub_k1,
            terminal_frame(position),
            &mut resources,
            &mut || panic!("saturated deployment does not repeat coverage"),
        ),
        Ok(PortableRadarTickOutcome::AlreadyDeployed)
    ));
    assert_eq!(
        destroy_portable_radar(&mut sub_k1, position, &mut resources, &mut || panic!(
            "missing raster rejects before RNG"
        )),
        Err(TerrainRadarError::Uninitialized)
    );
    assert_eq!(sub_k1, u16::MAX, "4055C0 clears K1 only after removal");
}
