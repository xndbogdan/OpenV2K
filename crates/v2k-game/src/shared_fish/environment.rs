//! `44EC60` mode-zero drag and steady wind/current, after DCA0's basis write.
//!
//! Mode2's oscillating vector needs the separate global44EBD0 clock owner.
//! The caller admits the authored static modes before invoking this leaf.

use crate::{
    common_mover::type9_tail::{apply_type9_environment_drag_raw, Type9ModeZeroDrag},
    hover::q31_mul,
};
use v2k_formats::terrain::TerrainGrid;

#[derive(Debug, Clone, Copy)]
pub(super) struct SteadyWind {
    pub vector_raw: [i16; 3],
    pub height_limit_raw: i32,
    /// `42ED00`: Section13+80, interpreted as a boolean at44ECB5.
    pub above_sea: bool,
}

pub(super) struct EnvironmentFrame<'a> {
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    pub drag: Type9ModeZeroDrag,
    pub wind: Option<SteadyWind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EnvironmentBody {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub lateral_q31: [i32; 3],
    pub forward_q31: [i32; 3],
}

pub(super) fn apply(body: &mut EnvironmentBody, frame: EnvironmentFrame<'_>) {
    let wind = frame
        .wind
        .filter(|wind| (frame.terrain.sea_level_raw() < body.position_raw[1]) == wind.above_sea);
    let Some(wind) = wind else {
        apply_type9_environment_drag_raw(&mut body.velocity_raw, frame.elapsed_micros, frame.drag);
        return;
    };
    //44ECD4/44ECE0 use signed WORD shifts. Both bilinear probes wrap X/Z.
    let [x, y, z] = body.position_raw;
    let half = frame.terrain.bilinear_height_raw(
        x.wrapping_sub(wind.vector_raw[0] >> 1),
        z.wrapping_sub(wind.vector_raw[2] >> 1),
    );
    let full = frame.terrain.bilinear_height_raw(
        x.wrapping_sub(wind.vector_raw[0]),
        z.wrapping_sub(wind.vector_raw[2]),
    );
    let clearance = (i32::from(y) - i32::from(half.max(full))).min(wind.height_limit_raw);
    if clearance < 0 {
        //44EEAB returns directly: sheltered actors do not receive fallback drag.
        return;
    }
    let scale = clearance.wrapping_shl(5);
    let difference: [i16; 3] = std::array::from_fn(|axis| {
        ((i32::from(wind.vector_raw[axis]).wrapping_mul(scale) >> 15) as i16)
            .wrapping_sub(body.velocity_raw[axis])
    });
    let project = |basis: [i32; 3]| {
        difference.into_iter().zip(basis).fold(0i32, |sum, (v, b)| {
            sum.wrapping_add(q31_mul(i32::from(v), b))
        })
    };
    body.roll_raw = body.roll_raw.wrapping_add(q31_mul(
        frame.elapsed_micros as i32,
        project(body.lateral_q31).wrapping_neg().wrapping_shl(14),
    ) as i16);
    body.pitch_raw = body.pitch_raw.wrapping_add(q31_mul(
        frame.elapsed_micros as i32,
        project(body.forward_q31).wrapping_shl(12),
    ) as i16);
    //44EFF6 divides the LOW dword of dt*strength by unsigned (B0<<3).
    let factor = (frame.elapsed_micros.wrapping_mul(frame.drag.strength)
        / (u32::from(frame.drag.callback_mass_raw.get()) << 3)) as i32;
    for (velocity, difference) in body.velocity_raw.iter_mut().zip(difference) {
        *velocity =
            velocity.wrapping_add((factor.wrapping_mul(i32::from(difference)) >> 15) as i16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU16;
    use v2k_formats::terrain::TerrainCell;

    fn drag() -> Type9ModeZeroDrag {
        Type9ModeZeroDrag {
            callback_mass_raw: NonZeroU16::new(100).unwrap(),
            strength: 3,
        }
    }

    #[test]
    fn medium_mismatch_uses_drag_but_negative_clearance_returns_without_drag() {
        let terrain = TerrainGrid {
            header: [2000 * 256, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                65536
            ],
        };
        let mut body = EnvironmentBody {
            position_raw: [0, -1, 0],
            velocity_raw: [1000, -1000, 100],
            pitch_raw: 77,
            roll_raw: -99,
            lateral_q31: [i32::MAX, 0, 0],
            forward_q31: [0, 0, i32::MAX],
        };
        let wind = SteadyWind {
            vector_raw: [1000, 0, 0],
            height_limit_raw: 2000,
            above_sea: false,
        };
        let initial = body;
        apply(
            &mut body,
            EnvironmentFrame {
                terrain: &terrain,
                elapsed_micros: 20000,
                drag: drag(),
                wind: Some(wind),
            },
        );
        assert_eq!(
            body, initial,
            "the matched medium is sheltered below both upwind terrain samples"
        );
        body.position_raw[1] = 2001;
        apply(
            &mut body,
            EnvironmentFrame {
                terrain: &terrain,
                elapsed_micros: 20000,
                drag: drag(),
                wind: Some(wind),
            },
        );
        assert_eq!(body.velocity_raw, [998, -997, 100]);
        assert_eq!([body.pitch_raw, body.roll_raw], [77, -99]);
        // At equality, source's strict sea<Y test is false: the water branch runs.
        body = initial;
        body.position_raw[1] = 2000;
        apply(
            &mut body,
            EnvironmentFrame {
                terrain: &terrain,
                elapsed_micros: 20000,
                drag: drag(),
                wind: Some(wind),
            },
        );
        assert_ne!(body.roll_raw, initial.roll_raw);
        assert_ne!(body.velocity_raw, [998, -997, 100]);
    }

    #[test]
    fn steady_current_matches_original_executable_vectors() {
        // Actual PE44EC60; oracle seed4460, cases1/2/78. Only lookup and
        //42ED00 were stubbed. Terrain is signed and varies in both directions.
        // Layout: mode,medium,dt,strength,mass,limit,sea,wind3,pos3,vel3,
        // pitch,roll,lateral3,forward3, expected velocity3/pitch/roll.
        let cases: [[i64; 29]; 3] = [
            [
                1,
                1,
                125000,
                0,
                100,
                32767,
                4000,
                28534,
                10476,
                -11521,
                5309,
                24344,
                -22794,
                11235,
                20568,
                -13134,
                -12352,
                32050,
                -494066088,
                685797621,
                60524746,
                258927932,
                1349396566,
                -1855290609,
                11235,
                20568,
                -13134,
                -11236,
                -29400,
            ],
            [
                1,
                1,
                20000,
                4294967263,
                65535,
                64,
                -1000,
                4772,
                -7101,
                8046,
                -8206,
                25977,
                1759,
                27105,
                -5398,
                24249,
                3409,
                -17795,
                596294233,
                60961696,
                1812270127,
                -1491292511,
                1898376597,
                1665367246,
                20404,
                -4160,
                18313,
                3583,
                -13623,
            ],
            [
                1,
                0,
                125000,
                3,
                100,
                64,
                4000,
                -27598,
                -14516,
                29969,
                -10829,
                868,
                -32071,
                11278,
                -16393,
                -23142,
                30280,
                -21859,
                -1808415043,
                -751448960,
                -1560501269,
                1735084981,
                1424721260,
                -927953388,
                11092,
                -16172,
                -22785,
                27647,
                -9797,
            ],
        ];
        let mut terrain = TerrainGrid {
            header: [0; 5],
            cells: (0..65536)
                .map(|i| TerrainCell {
                    height: ((((i / 256) * 3 + (i % 256) * 5) % 256) as i32 - 128) as u8,
                    attribute: 0,
                    terrain_type: 0,
                })
                .collect(),
        };
        for row in cases {
            terrain.header[0] = row[6] as i32 * 256;
            let mut body = EnvironmentBody {
                position_raw: std::array::from_fn(|i| row[10 + i] as i16),
                velocity_raw: std::array::from_fn(|i| row[13 + i] as i16),
                pitch_raw: row[16] as i16,
                roll_raw: row[17] as i16,
                lateral_q31: std::array::from_fn(|i| row[18 + i] as i32),
                forward_q31: std::array::from_fn(|i| row[21 + i] as i32),
            };
            let before = body;
            apply(
                &mut body,
                EnvironmentFrame {
                    terrain: &terrain,
                    elapsed_micros: row[2] as u32,
                    drag: Type9ModeZeroDrag {
                        callback_mass_raw: NonZeroU16::new(row[4] as u16).unwrap(),
                        strength: row[3] as u32,
                    },
                    wind: Some(SteadyWind {
                        vector_raw: std::array::from_fn(|i| row[7 + i] as i16),
                        height_limit_raw: row[5] as i32,
                        above_sea: row[1] != 0,
                    }),
                },
            );
            assert_eq!(
                body.velocity_raw,
                std::array::from_fn(|i| row[24 + i] as i16)
            );
            assert_eq!(
                [body.pitch_raw, body.roll_raw],
                [row[27] as i16, row[28] as i16]
            );
            assert_eq!(body.position_raw, before.position_raw);
            assert_eq!(body.lateral_q31, before.lateral_q31);
            assert_eq!(body.forward_q31, before.forward_q31);
        }
    }
}
