use super::*;
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

fn terrain(height: i8, sea: i16) -> TerrainGrid {
    TerrainGrid {
        header: [i32::from(sea) << 8, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn run(
    wind: Wind,
    terrain: &TerrainGrid,
    position: [i16; 3],
    velocity: &mut [i16; 3],
    angles: &mut [i16; 3],
) {
    apply_common_wind_drag_raw(
        velocity,
        angles,
        CommonWindDrag { wind, strength: 3 },
        CommonWindDragFrame {
            terrain,
            position_raw: position,
            basis: Type9BodyBasis::from_angle_words(0, 0, 0),
            callback_mass_raw: NonZeroU16::new(100).unwrap(),
            elapsed_micros: 20_000,
        },
    );
}

#[test]
fn steady_wind_uses_height_cap_and_signed_angular_and_linear_words() {
    let grid = terrain(0, 0);
    let wind = Wind::Current {
        vector_raw: [1024, 0, 2048],
        max_height_raw: 1024,
        above_sea: true,
    };
    let mut velocity = [0; 3];
    let mut angles = [10, 20, 30];
    run(wind, &grid, [0, 2048, 0], &mut velocity, &mut angles);
    assert_eq!(velocity, [2, 0, 4]);
    // Retail zero heading points forward+X and lateral-Z. Signed Q31
    // projection therefore gives forward1023/lateral-2048, then EC60 adds
    // floor(20000*1023*4096/2^31)=39 to pitch and
    // floor(20000*(-2048)*(-16384)/2^31)=312 to roll.
    assert_eq!(angles, [10, 59, 342]);

    let mut velocity = [0; 3];
    run(
        Wind::Current {
            vector_raw: [1024, 0, 2048],
            max_height_raw: 512,
            above_sea: true,
        },
        &grid,
        [0, 2048, 0],
        &mut velocity,
        &mut [0; 3],
    );
    assert_eq!(velocity, [1, 0, 2]);
}

#[test]
fn wrong_sea_side_uses_subtraction_drag_and_shelter_returns_without_drag() {
    let grid = terrain(0, -4096);
    assert!(!grid.water_enabled());
    let wind = Wind::Current {
        vector_raw: [1000; 3],
        max_height_raw: 10000,
        above_sea: false,
    };
    for wind in [Wind::None, wind] {
        let mut velocity = [-32768, 32767, -1];
        let mut angles = [11, 22, 33];
        run(wind, &grid, [0, -3008, 0], &mut velocity, &mut angles);
        assert_eq!(velocity, [-32693, 32693, 0]);
        assert_eq!(angles, [11, 22, 33]);
    }
    let grid = terrain(64, 0);
    let mut velocity = [300, -400, 500];
    let mut angles = [11, 22, 33];
    run(
        Wind::Current {
            vector_raw: [1000; 3],
            max_height_raw: 10000,
            above_sea: true,
        },
        &grid,
        [0, 1024, 0],
        &mut velocity,
        &mut angles,
    );
    assert_eq!(velocity, [300, -400, 500]);
    assert_eq!(angles, [11, 22, 33]);
}

#[test]
fn terrain_shelter_uses_higher_of_half_and_full_upwind_probes() {
    let mut grid = terrain(0, 0);
    // x=1024, windX=512: the half probe is cell3, the full probe cell2.
    // Either ridge shelters the actor even though the other probe is flat.
    for ridge_x in [2, 3] {
        grid.cells.fill(TerrainCell {
            height: 0,
            attribute: 0,
            terrain_type: 0,
        });
        grid.cells[ridge_x * GRID_SIZE].height = 64;
        let mut velocity = [600, 700, 800];
        run(
            Wind::Current {
                vector_raw: [512, 0, 0],
                max_height_raw: 10000,
                above_sea: true,
            },
            &grid,
            [1024, 1024, 0],
            &mut velocity,
            &mut [0; 3],
        );
        assert_eq!(velocity, [600, 700, 800]);
    }
}

#[test]
fn gust_sine_preserves_quadrants_signed_rounding_and_process_phase_wrap() {
    let authored = [2000, -300, 32000];
    assert_eq!(gust_vector_raw(authored, 0), [0; 3]);
    assert_eq!(gust_vector_raw(authored, 0x4000 << 7), [1999, -300, 31999]);
    assert_eq!(gust_vector_raw(authored, 0x8000 << 7), [0; 3]);
    assert_eq!(gust_vector_raw(authored, 0xc000 << 7), [-2000, 299, -32000]);
    assert_eq!(gust_vector_raw(authored, 0x10000 << 7), [0; 3]);
    // Low phase bits are discarded in EBD0 and low two lookup bits at the
    // quarter-sine site. These are integer table reads, not float sin().
    assert_eq!(
        gust_vector_raw(authored, 0x4000 << 7),
        gust_vector_raw(authored, (0x4003 << 7) + 127)
    );
    let mut fx = crate::world_fx::WorldFx::new();
    assert_eq!(fx.advance_wind_phase(0), 0x1234_5678);
    assert_eq!(
        fx.advance_wind_phase(u32::MAX - INITIAL_WIND_PHASE_RAW - 100),
        u32::MAX - 100
    );
    fx.clear();
    assert_eq!(fx.advance_wind_phase(201), 100);
}
