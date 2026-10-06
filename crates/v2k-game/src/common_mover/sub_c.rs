//! Exact detached Section-12 Sub-C lift phase.
//!
//! Retail `FUN_0041F1C0` samples either static terrain or the animated wave
//! surface, corrects only the penetrations owned by that selected path, adds a
//! capped impulse along the entity's previous-frame body-up axis, and removes
//! only velocity directed into the sampled surface. The terrain and wave
//! lookup routines remain with the caller so this module does not fabricate a
//! live world/actor adapter.

use crate::hover::{dot_q31, q31_mul, HoverLiftConfig};

/// Distance applied along body-up before Sub-C samples terrain or waves.
pub const SUB_C_BODY_SAMPLE_OFFSET_RAW: i32 = 200;
/// Sub-C selects waves only while attached cargo mass is at most this value.
pub const SUB_C_WAVE_ATTACHED_MASS_MAX: u32 = 99;
/// Hard-coded upper bound applied after the authored lift/near-boost formula.
pub const SUB_C_MAX_LIFT_IMPULSE_RAW: i32 = 200;

/// Surface already sampled for one `FUN_0041F1C0` call.
///
/// Wave response retains the underlying static terrain sample because retail
/// may hard-correct a hull that penetrates terrain while leaving penetration
/// of the moving wave force-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubCSurfaceSample {
    Terrain { terrain_y_raw: i16 },
    Wave { wave_y_raw: i16, terrain_y_raw: i16 },
}

/// Live clock/state consumed by 1F470's water sampler.
#[derive(Debug, Clone, Copy)]
pub struct SubCWaveClock {
    pub wave_tick_50hz: i32,
    pub waves_enabled: bool,
}

/// 45860 terrain or 1F470 water sampling. Disabled waves retain the static sea
/// plane; both water paths clamp to the signed bilinear terrain height.
pub fn sample_sub_c_world_surface(
    terrain: &v2k_formats::terrain::TerrainGrid,
    point_raw: [i16; 2],
    wave: Option<SubCWaveClock>,
) -> SubCSurfaceSample {
    let [x, z] = point_raw;
    let terrain_y_raw = terrain.bilinear_height_raw(x, z);
    match wave {
        None => SubCSurfaceSample::Terrain { terrain_y_raw },
        Some(clock) => SubCSurfaceSample::Wave {
            terrain_y_raw,
            wave_y_raw: if clock.waves_enabled {
                v2k_formats::terrain::wave_surface_raw(
                    x,
                    z,
                    clock.wave_tick_50hz,
                    terrain.sea_level_raw(),
                    terrain_y_raw,
                )
            } else {
                terrain_y_raw.max(terrain.sea_level_raw())
            },
        },
    }
}

impl SubCSurfaceSample {
    const fn selected_y_raw(self) -> i16 {
        match self {
            Self::Terrain { terrain_y_raw } => terrain_y_raw,
            Self::Wave { wave_y_raw, .. } => wave_y_raw,
        }
    }
}

/// Observable deterministic result of one detached Sub-C phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubCLiftOutcome {
    /// Final signed gap after the path-specific hard correction.
    pub final_gap_raw: i32,
    /// Signed Y correction calculated before narrowing to the entity word.
    pub position_correction_raw: i32,
    /// Final capped impulse applied along body-up, or zero outside lift range.
    pub lift_impulse_raw: i32,
    /// Negative projection removed inside the damping range, otherwise zero.
    pub removed_surface_velocity_raw: i32,
}

/// Return whether retail selects its wave sampler for this descriptor/mass.
///
/// The caller owns world availability. Disabled wave animation still uses the
/// static sea plane on this path; it does not change the selected response mode.
pub const fn sub_c_uses_wave_surface(
    descriptor: HoverLiftConfig,
    attached_cargo_mass: u32,
) -> bool {
    descriptor.use_wave_surface
        && (attached_cargo_mass as i32) <= SUB_C_WAVE_ATTACHED_MASS_MAX as i32
}

/// Compute the signed-word X/Z point sampled by `FUN_0041F1C0`.
///
/// Sub-C byte 13 moves the point 200 raw units opposite body-up. Each Q31
/// product narrows to an `i16` before wrapping subtraction, preserving retail's
/// toroidal world-seam behavior.
pub fn sub_c_surface_sample_point_raw(
    position_raw: [i16; 3],
    body_up_q31: [i32; 3],
    offset_sample: bool,
) -> [i16; 2] {
    if !offset_sample {
        return [position_raw[0], position_raw[2]];
    }

    [
        position_raw[0].wrapping_sub(q31_mul(body_up_q31[0], SUB_C_BODY_SAMPLE_OFFSET_RAW) as i16),
        position_raw[2].wrapping_sub(q31_mul(body_up_q31[2], SUB_C_BODY_SAMPLE_OFFSET_RAW) as i16),
    ]
}

/// Execute the deterministic portion of retail `FUN_0041F1C0`.
///
/// `elapsed_micros` is deliberately not clamped here: retail's outer mover
/// owns the 50,000-us clamp before component dispatch. Signed multiplications
/// use x86-style wrapping before each signed division, while Q31 products keep
/// the executable's doubled-high-dword narrowing.
///
/// Authored Sub-C divisors are expected to be nonzero whenever their branches
/// are entered. Like retail `idiv`, Rust will fail rather than inventing a
/// fallback if malformed data reaches an active zero-divisor branch.
pub fn apply_sub_c_lift_raw(
    descriptor: HoverLiftConfig,
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    elapsed_micros: u32,
    surface: SubCSurfaceSample,
    body_up_q31: [i32; 3],
) -> SubCLiftOutcome {
    let base_clearance_raw = i32::from(descriptor.base_clearance_raw);
    let mut gap_raw = i32::from(position_raw[1])
        .wrapping_sub(i32::from(surface.selected_y_raw()))
        .wrapping_sub(base_clearance_raw);
    let mut position_correction_raw = 0;

    if gap_raw < 0 {
        match surface {
            SubCSurfaceSample::Terrain { .. } => {
                position_correction_raw = gap_raw.wrapping_neg();
                position_raw[1] = position_raw[1].wrapping_sub(gap_raw as i16);
                gap_raw = 0;
            }
            SubCSurfaceSample::Wave { terrain_y_raw, .. } => {
                let terrain_penetration_raw = i32::from(terrain_y_raw)
                    .wrapping_add(base_clearance_raw)
                    .wrapping_sub(i32::from(position_raw[1]));
                if terrain_penetration_raw > 0 {
                    position_correction_raw = terrain_penetration_raw;
                    position_raw[1] = position_raw[1].wrapping_add(terrain_penetration_raw as i16);
                    gap_raw = gap_raw.wrapping_add(terrain_penetration_raw);
                }
            }
        }
    }

    let lift_range_raw = i32::from(descriptor.lift_range_raw);
    let mut lift_impulse_raw = 0;
    if gap_raw < lift_range_raw {
        let strength_this_frame = q31_mul(descriptor.strength_raw, elapsed_micros as i32);
        lift_impulse_raw = lift_range_raw
            .wrapping_sub(gap_raw)
            .wrapping_mul(strength_this_frame)
            / lift_range_raw;

        let near_boost_range_raw = i32::from(descriptor.near_boost_range_raw);
        if gap_raw < near_boost_range_raw {
            lift_impulse_raw = lift_impulse_raw.wrapping_add(
                near_boost_range_raw
                    .wrapping_sub(gap_raw)
                    .wrapping_mul(lift_impulse_raw)
                    / near_boost_range_raw,
            );
        }
        if lift_impulse_raw > SUB_C_MAX_LIFT_IMPULSE_RAW {
            lift_impulse_raw = SUB_C_MAX_LIFT_IMPULSE_RAW;
        }

        for (word, component) in velocity_raw.iter_mut().zip(body_up_q31) {
            *word = word.wrapping_add(q31_mul(component, lift_impulse_raw) as i16);
        }
    }

    let mut removed_surface_velocity_raw = 0;
    if gap_raw < i32::from(descriptor.damping_range_raw) {
        let into_surface_raw = dot_q31(body_up_q31, *velocity_raw);
        if into_surface_raw < 0 {
            removed_surface_velocity_raw = into_surface_raw;
            for (word, component) in velocity_raw.iter_mut().zip(body_up_q31) {
                *word = word.wrapping_sub(q31_mul(component, into_surface_raw) as i16);
            }
        }
    }

    SubCLiftOutcome {
        final_gap_raw: gap_raw,
        position_correction_raw,
        lift_impulse_raw,
        removed_surface_velocity_raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_wave_sampler_retains_static_sea_and_signed_terrain_when_disabled() {
        use v2k_formats::terrain::{TerrainCell, TerrainGrid};
        let mut terrain = TerrainGrid {
            header: [512 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 128,
                    attribute: 0,
                    terrain_type: 0
                };
                256 * 256
            ],
        };
        let disabled = Some(SubCWaveClock {
            wave_tick_50hz: 99,
            waves_enabled: false,
        });
        assert_eq!(
            sample_sub_c_world_surface(&terrain, [-1, i16::MIN], disabled),
            SubCSurfaceSample::Wave {
                terrain_y_raw: -4096,
                wave_y_raw: 512
            }
        );
        assert_eq!(
            sample_sub_c_world_surface(&terrain, [-1, i16::MIN], None),
            SubCSurfaceSample::Terrain {
                terrain_y_raw: -4096
            }
        );
        for cell in &mut terrain.cells {
            cell.height = 32;
        }
        assert_eq!(
            sample_sub_c_world_surface(&terrain, [i16::MAX, -1], disabled),
            SubCSurfaceSample::Wave {
                terrain_y_raw: 1024,
                wave_y_raw: 1024
            }
        );
    }

    #[test]
    fn world_wave_sampler_uses_draw_clock_and_wraps_coordinates() {
        use v2k_formats::terrain::{wave_surface_raw, TerrainCell, TerrainGrid};
        let terrain = TerrainGrid {
            header: [4096 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 128,
                    attribute: 0,
                    terrain_type: 0
                };
                256 * 256
            ],
        };
        let point = [i16::MAX.wrapping_add(1), (-1_i16).wrapping_add(2)];
        let mut samples = Vec::new();
        for tick in [0, 1, 49, 100, i32::MAX] {
            let expected = wave_surface_raw(point[0], point[1], tick, 4096, -4096);
            assert_eq!(
                sample_sub_c_world_surface(
                    &terrain,
                    point,
                    Some(SubCWaveClock {
                        wave_tick_50hz: tick,
                        waves_enabled: true
                    })
                ),
                SubCSurfaceSample::Wave {
                    terrain_y_raw: -4096,
                    wave_y_raw: expected
                }
            );
            samples.push(expected);
        }
        assert!(samples.windows(2).any(|pair| pair[0] != pair[1]));
    }

    const DESCRIPTOR: HoverLiftConfig = HoverLiftConfig {
        base_clearance_raw: 150,
        lift_range_raw: 125,
        strength_raw: 0x0090_0000,
        near_boost_range_raw: 100,
        damping_range_raw: 200,
        use_wave_surface: true,
        offset_sample: true,
    };
    const UP_Q31: [i32; 3] = [0, i32::MAX, 0];

    #[test]
    fn wave_selection_uses_strict_attached_mass_boundary() {
        assert!(sub_c_uses_wave_surface(DESCRIPTOR, 99));
        assert!(!sub_c_uses_wave_surface(DESCRIPTOR, 100));
        // FUN_004185C0's accumulated dword reaches FUN_0041F1C0 as a signed
        // `int`; preserve that signed comparison after wrapping.
        assert!(sub_c_uses_wave_surface(DESCRIPTOR, u32::MAX));
        assert!(!sub_c_uses_wave_surface(
            HoverLiftConfig {
                use_wave_surface: false,
                ..DESCRIPTOR
            },
            0,
        ));
    }

    #[test]
    fn sample_offset_narrows_and_wraps_signed_words() {
        let position = [i16::MIN + 50, 123, i16::MAX - 50];
        let diagonal_up = [0x4000_0000, 0, -0x4000_0000];
        assert_eq!(
            sub_c_surface_sample_point_raw(position, diagonal_up, true),
            [position[0].wrapping_sub(100), position[2].wrapping_add(100)]
        );
        assert_eq!(
            sub_c_surface_sample_point_raw(position, diagonal_up, false),
            [position[0], position[2]]
        );
    }

    #[test]
    fn terrain_penetration_snaps_to_base_gap_before_force_and_damping() {
        let mut position = [0, -64, 0];
        let mut velocity = [0, -256, 0];
        let outcome = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut position,
            &mut velocity,
            20_000,
            SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
            UP_Q31,
        );

        assert_eq!(position, [0, 150, 0]);
        assert_eq!(outcome.position_correction_raw, 214);
        assert_eq!(outcome.final_gap_raw, 0);
        assert_eq!(outcome.lift_impulse_raw, 174);
        assert_eq!(outcome.removed_surface_velocity_raw, -83);
        assert_eq!(velocity, [0, 0, 0]);
    }

    #[test]
    fn wave_penetration_remains_force_based_above_static_terrain() {
        let mut position = [0, 0, 0];
        let mut velocity = [0, 0, 0];
        let outcome = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut position,
            &mut velocity,
            20_000,
            SubCSurfaceSample::Wave {
                wave_y_raw: 256,
                terrain_y_raw: -2_560,
            },
            UP_Q31,
        );

        assert_eq!(position, [0, 0, 0]);
        assert_eq!(outcome.position_correction_raw, 0);
        assert_eq!(outcome.final_gap_raw, -406);
        assert_eq!(outcome.lift_impulse_raw, 200);
        assert_eq!(velocity, [0, 199, 0]);
    }

    #[test]
    fn wave_path_corrects_only_underlying_terrain_penetration() {
        let mut position = [0, 0, 0];
        let mut velocity = [0, 0, 0];
        let outcome = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut position,
            &mut velocity,
            20_000,
            SubCSurfaceSample::Wave {
                wave_y_raw: 512,
                terrain_y_raw: 32,
            },
            UP_Q31,
        );

        assert_eq!(position[1], 182);
        assert_eq!(outcome.position_correction_raw, 182);
        assert_eq!(outcome.final_gap_raw, -480);
        assert_eq!(outcome.lift_impulse_raw, 200);
    }

    #[test]
    fn lift_and_damping_ranges_are_strict() {
        let mut at_lift_range_position = [0, 275, 0];
        let mut at_lift_range_velocity = [0, 0, 0];
        let lift_boundary = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut at_lift_range_position,
            &mut at_lift_range_velocity,
            20_000,
            SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
            UP_Q31,
        );
        assert_eq!(lift_boundary.final_gap_raw, 125);
        assert_eq!(lift_boundary.lift_impulse_raw, 0);
        assert_eq!(at_lift_range_velocity, [0, 0, 0]);

        let mut at_damping_range_position = [0, 350, 0];
        let mut at_damping_range_velocity = [0, -100, 0];
        let damping_boundary = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut at_damping_range_position,
            &mut at_damping_range_velocity,
            20_000,
            SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
            UP_Q31,
        );
        assert_eq!(damping_boundary.final_gap_raw, 200);
        assert_eq!(damping_boundary.removed_surface_velocity_raw, 0);
        assert_eq!(at_damping_range_velocity, [0, -100, 0]);
    }

    #[test]
    fn damping_preserves_velocity_away_from_surface() {
        let mut position = [0, 250, 0];
        let mut velocity = [0, 100, 0];
        let outcome = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut position,
            &mut velocity,
            0,
            SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
            UP_Q31,
        );
        assert_eq!(outcome.final_gap_raw, 100);
        assert_eq!(outcome.lift_impulse_raw, 0);
        assert_eq!(outcome.removed_surface_velocity_raw, 0);
        assert_eq!(velocity, [0, 100, 0]);
    }

    #[test]
    fn lift_uses_previous_body_up_on_all_axes() {
        let mut position = [0, 0, 0];
        let mut velocity = [0, 0, 0];
        let outcome = apply_sub_c_lift_raw(
            DESCRIPTOR,
            &mut position,
            &mut velocity,
            20_000,
            SubCSurfaceSample::Wave {
                wave_y_raw: 256,
                terrain_y_raw: -2_560,
            },
            [0x4000_0000, 0x4000_0000, 0],
        );
        assert_eq!(outcome.lift_impulse_raw, 200);
        assert_eq!(velocity, [100, 100, 0]);
    }
}
