use super::*;
use crate::main_base_type54_abort::{MainBaseType54NetworkSession, TYPE85_ENTITY_TYPE};

/// Retail's world-43 row X 0x7F, Z 0x6D..=0x97, in height units, after a
/// direct 2000-damage hit killed Type85 at (0x7F00, 0x840, 0x8200): before
/// the hit, then after the 10th, 20th, 30th, 37th, 45th, 49th and 50th
/// 20-ms frame. The 50th frame ends the second, so the ground is back and the
/// crater is cut.
const RETAIL_ROWS: [(u32, [i8; 43]); 8] = [
    (
        0,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 49, 48, 51, 56, 64, 69, 69, 67, 66, 66,
            66, 66, 66, 66, 73, 91, 111, 109, 105, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        10,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 49, 48, 51, 56, 64, 69, 76, 75, 74, 74,
            74, 74, 73, 66, 73, 91, 111, 109, 105, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        20,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 49, 48, 51, 56, 69, 79, 77, 70, 66, 66,
            66, 69, 74, 76, 78, 91, 111, 109, 105, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        30,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 49, 48, 52, 66, 68, 69, 69, 67, 66, 66,
            66, 66, 66, 66, 77, 101, 112, 109, 105, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        37,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 49, 53, 60, 56, 64, 69, 69, 67, 66, 66,
            66, 66, 66, 66, 73, 91, 120, 114, 105, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        45,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 55, 48, 51, 56, 64, 69, 69, 67, 66, 66,
            66, 66, 66, 66, 73, 91, 111, 109, 111, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        49,
        [
            54, 60, 65, 70, 74, 78, 79, 78, 72, 63, 57, 51, 49, 48, 51, 56, 64, 69, 69, 67, 66, 66,
            66, 66, 66, 66, 73, 91, 111, 109, 105, 98, 90, 82, 75, 68, 62, 57, 50, 44, 37, 29, 19,
        ],
    ),
    (
        50,
        [
            56, 62, 66, 69, 72, 76, 76, 74, 67, 60, 57, 51, 49, 48, 51, 56, 55, 58, 58, 56, 55, 54,
            55, 55, 55, 55, 62, 81, 101, 100, 96, 90, 83, 76, 70, 64, 62, 57, 50, 44, 38, 31, 21,
        ],
    ),
];

/// Retail's 25 Type34 births from the same death, from RNG state
/// `0x0CE6D853` at the hit: X, Y (after `44E770`'s +300) and Z.
const RETAIL_METEORS: [[u16; 3]; 25] = [
    [0x7742, 0x07dd, 0x7a8d],
    [0x7ba8, 0x090f, 0x7ac8],
    [0x7f6a, 0x08ce, 0x7afa],
    [0x8343, 0x0959, 0x7ac2],
    [0x8768, 0x059b, 0x7a09],
    [0x7746, 0x08fa, 0x7e66],
    [0x7b34, 0x0802, 0x7e28],
    [0x7f6d, 0x08dd, 0x7ed1],
    [0x836a, 0x0831, 0x7ed5],
    [0x872b, 0x06c5, 0x7e1b],
    [0x772b, 0x0592, 0x8292],
    [0x7b9e, 0x06bf, 0x82f6],
    [0x7f66, 0x05b9, 0x8296],
    [0x832f, 0x05b4, 0x829e],
    [0x871d, 0x070b, 0x829b],
    [0x77ae, 0x080d, 0x862d],
    [0x7b40, 0x07af, 0x86b9],
    [0x7f51, 0x068f, 0x8634],
    [0x830f, 0x0854, 0x8626],
    [0x8772, 0x066d, 0x8666],
    [0x77a6, 0x0959, 0x8ae7],
    [0x7b0e, 0x08b3, 0x8ac5],
    [0x7f8a, 0x07f6, 0x8a9b],
    [0x83de, 0x0808, 0x8ab3],
    [0x87c1, 0x07e3, 0x8a9d],
];

fn row(resources: &ResourceCache) -> [i8; 43] {
    let terrain = resources.level_terrain().unwrap();
    std::array::from_fn(|z| terrain.cell(0x7f, 0x6d + z).unwrap().height as i8)
}

#[v2k_test_support::retail_test]
fn type85s_death_ripples_launches_and_craters_as_retail() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(43);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == TYPE85_ENTITY_TYPE)
        .unwrap()
        .id;
    let lease = manager.main_base_abort_actor_observation(id).unwrap().lease;
    let extent = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(|entity| entity.model_index)
        .and_then(|model| session.cache.global_model(model))
        .unwrap()
        .radius;
    // The crater redraws the radar under it.
    session
        .cache
        .initialize_level_terrain_radar(&mut || 0)
        .unwrap();
    let before = manager.iter_all().map(|entity| entity.id).max().unwrap();
    fx.set_shared_retail_random_state(0x0ce6_d853);
    manager
        .apply_class38_death(
            lease,
            session.cache.terrain().unwrap(),
            extent,
            MainBaseType54NetworkSession::SoloNetworkingDisabled,
            &mut fx,
            crate::entity::Class38DeathWorld {
                resources: &session.cache,
                retail_tick: 871,
            },
        )
        .unwrap();
    let meteors: Vec<[u16; 3]> = manager
        .iter_all()
        .filter(|entity| entity.id > before)
        .map(|entity| entity.position_raw().map(|word| word as u16))
        .collect();
    assert_eq!(meteors, RETAIL_METEORS);
    // Each birth draws one more word; retail's stream reads this after them.
    assert_eq!(fx.shared_retail_random_state(), 0xe305_e009);
    assert!(fx.take_full_frame_flash_request());
    assert_eq!(fx.terrain_ripples().len(), 1);

    assert_eq!(row(&session.cache), RETAIL_ROWS[0].1);
    let mut static_damage = StaticDamageScheduler::new();
    let mut flash = FullFrameSpriteSequence::default();
    for frame in 1..=50 {
        let stops = advance_terrain_ripples(
            20_000,
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut static_damage,
            &mut flash,
            &mut |_, _, _, _, _, _| Err("no radial template"),
        );
        assert!(stops.is_empty(), "{stops:?}");
        if let Some((_, expected)) = RETAIL_ROWS.iter().find(|(at, _)| *at == frame) {
            assert_eq!(row(&session.cache), *expected, "frame {frame}");
        }
        assert_eq!(flash.is_active(), frame == 50, "frame {frame}");
    }
    assert!(fx.terrain_ripples().is_empty());
}

#[test]
fn a_zero_frame_ends_a_fresh_ripple_and_the_period_wraps_the_clock() {
    let mut terrain = TerrainGrid {
        header: [0; 5],
        cells: vec![
            v2k_formats::terrain::TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    let mut ripple = TerrainRipple::new([0x7f00, 0, -0x7e00], TYPE85_DEATH_RIPPLE);
    assert!(ripple.advance(&mut terrain, 0).is_empty());
    assert!(ripple.finished());
    let mut ripple = TerrainRipple::new([0x7f00, 0, -0x7e00], TYPE85_DEATH_RIPPLE);
    let mut fallen = 0;
    for _ in 0..49 {
        fallen += ripple.advance(&mut terrain, 20_000).len();
        assert!(!ripple.finished());
    }
    ripple.advance(&mut terrain, 20_000);
    assert!(ripple.finished());
    assert!(fallen > 0);
    // Each frame writes only its change, so the ground ends where it began.
    assert!(terrain.cells.iter().all(|cell| cell.height == 0));
}

/// `4374B0` runs a finished record's radial (bit 0x04) after its flash and
/// before its crater, with `37390`'s words 4..11 as `4566E0`'s template.
#[v2k_test_support::retail_test]
fn a_finished_ripple_runs_its_radial_template() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(43);
    let template = TerrainRippleTemplate {
        flags: 0x06,
        radial_words: [0x0400_0200, 200, 1, 4, 10_000, 8_000, 125, 9],
        period_us: 40_000,
        ..TYPE85_DEATH_RIPPLE
    };
    fx.queue_terrain_ripple(TerrainRipple::new([0x7f00, 0x840, 0x7e00], template));
    let mut static_damage = StaticDamageScheduler::new();
    let mut flash = FullFrameSpriteSequence::default();
    let mut calls = Vec::new();
    for _ in 0..2 {
        let stops = advance_terrain_ripples(
            20_000,
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut static_damage,
            &mut flash,
            &mut |_, _, _, _, origin, radial| {
                calls.push((origin, radial));
                Ok(())
            },
        );
        assert!(stops.is_empty(), "{stops:?}");
    }
    assert!(flash.is_active());
    assert_eq!(
        calls,
        [(
            [0x7f00, 0x840, 0x7e00],
            crate::radial_damage::RadialDamageTemplate {
                inner_radius_raw: 0x200,
                outer_radius_raw: 0x400,
                impulse_raw: 200,
                packet: crate::damage::DamagePacket {
                    channels: [1, 4],
                    amounts_raw: [10_000, 8_000],
                },
                trailing_raw: [125, 9],
            }
        )]
    );
    assert!(fx.terrain_ripples().is_empty());
}
