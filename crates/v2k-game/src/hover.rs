//! Retail type-46 skimmer physics data and fixed-point helpers.
//!
//! The executable drives the player through Section-12 blocks A/B/C/D. The
//! behavior callback changes the 16-bit angle words, applies lift, removes
//! lateral velocity, and applies signed drive. The entity basis is rebuilt
//! only afterwards, so all force projections deliberately use the previous
//! frame's Q31 basis. Common environment gravity/drag follows before position
//! integration uses signed 8.8 words.

use v2k_formats::collision::{CollisionEntry, SubCLiftDescriptor};
use v2k_formats::fixed_math::retail_sine_q15;

const KEYBOARD_FULL_SCALE_RAW: i32 = 0x0d80;
const KEYBOARD_SENSITIVITY_MAX: i32 = 15;
const STEERING_NUMERATOR: i32 = 0x34;
pub const RETAIL_DEFAULT_SENSITIVITY: u8 = 10;
/// `FUN_0044FE20` clamps the global gameplay delta before behavior dispatch.
pub use crate::retail_clock::RETAIL_FRAME_DELTA_MAX_US;

/// Type-46 Section-12 Sub-C (`FUN_0041F1C0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverLiftConfig {
    pub base_clearance_raw: i16,
    pub lift_range_raw: i16,
    pub strength_raw: i32,
    pub near_boost_range_raw: i16,
    pub damping_range_raw: i16,
    pub use_wave_surface: bool,
    pub offset_sample: bool,
}

impl From<SubCLiftDescriptor> for HoverLiftConfig {
    fn from(descriptor: SubCLiftDescriptor) -> Self {
        Self {
            base_clearance_raw: descriptor.base_clearance_raw,
            lift_range_raw: descriptor.lift_range_raw,
            strength_raw: descriptor.strength_raw,
            near_boost_range_raw: descriptor.near_boost_range_raw,
            damping_range_raw: descriptor.damping_range_raw,
            use_wave_surface: descriptor.surface_mode_raw != 0,
            offset_sample: descriptor.offset_sample_raw != 0,
        }
    }
}

/// Authored type-46 A/B/C/D data consumed by the retail Hover callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverPhysicsConfig {
    /// Sub-A word 0: powered acceleration coefficient.
    pub drive_acceleration_raw: i16,
    /// Sub-A word 1: correction used after passing the signed target speed.
    pub overspeed_correction_raw: i16,
    /// Sub-A word 2: base projected-speed target before constructor jitter.
    pub target_speed_base_raw: i16,
    /// Sub-B dword 0: lateral-velocity dead zone.
    pub lateral_dead_zone_raw: i32,
    /// Sub-B dword 1: lateral correction rate.
    pub lateral_correction_raw: i32,
    pub lift: HoverLiftConfig,
    /// Sub-D dword 0, used by `turn * 0x34 / divisor`.
    pub steering_divisor_raw: i32,
    /// Sub-D byte 4 seeds the generic steering component. Hover explicitly
    /// clears the corresponding runtime field before updating heading.
    pub steering_constructor_axis_flag: bool,
    /// Remaining Sub-D bytes are retained for the still-unported secondary
    /// steering state rather than silently discarded.
    pub steering_tail_raw: [u8; 7],
    /// Section-12 type header `+0xC0`, copied to runtime entity `+0xC8`.
    /// Hover behavior ORs `0x10010` and clears no bits, so bit 3 remains the
    /// exact gate for common `FUN_0044EC60` wind/no-wind drag.
    pub environment_flags_at_type_record_0xc0: u32,
}

impl Default for HoverPhysicsConfig {
    fn default() -> Self {
        Self {
            drive_acceleration_raw: 1400,
            overspeed_correction_raw: -300,
            target_speed_base_raw: 3000,
            lateral_dead_zone_raw: 0,
            lateral_correction_raw: 500,
            lift: HoverLiftConfig {
                base_clearance_raw: 150,
                lift_range_raw: 125,
                strength_raw: 0x0090_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                use_wave_surface: true,
                offset_sample: true,
            },
            steering_divisor_raw: 28,
            steering_constructor_axis_flag: true,
            steering_tail_raw: [0; 7],
            environment_flags_at_type_record_0xc0: 8,
        }
    }
}

impl HoverPhysicsConfig {
    /// Decode the A/B/C/D blocks used by type 46. Missing, short, or invalid
    /// blocks return `None`; callers may then retain the known retail default.
    pub fn from_record(record: &CollisionEntry) -> Option<Self> {
        let mut config = Self::from_blocks(
            record.subsection("A")?,
            record.subsection("B")?,
            record.subsection("C")?,
            record.subsection("D")?,
        )?;
        config.environment_flags_at_type_record_0xc0 =
            u32::from_le_bytes(record.raw_header.get(0xc0..0xc4)?.try_into().ok()?);
        Some(config)
    }

    fn from_blocks(a: &[u8], b: &[u8], c: &[u8], d: &[u8]) -> Option<Self> {
        if a.len() < 6 || b.len() < 8 || c.len() < 16 || d.len() < 12 {
            return None;
        }
        let steering_divisor_raw = read_i32(d, 0)?;
        if steering_divisor_raw == 0 {
            return None;
        }
        let mut steering_tail_raw = [0; 7];
        steering_tail_raw.copy_from_slice(&d[5..12]);
        Some(Self {
            drive_acceleration_raw: read_i16(a, 0)?,
            overspeed_correction_raw: read_i16(a, 2)?,
            target_speed_base_raw: read_i16(a, 4)?,
            lateral_dead_zone_raw: read_i32(b, 0)?,
            lateral_correction_raw: read_i32(b, 4)?,
            lift: HoverLiftConfig {
                base_clearance_raw: read_i16(c, 0)?,
                lift_range_raw: read_i16(c, 2)?,
                strength_raw: read_i32(c, 4)?,
                near_boost_range_raw: read_i16(c, 8)?,
                damping_range_raw: read_i16(c, 10)?,
                use_wave_surface: c[12] != 0,
                offset_sample: c[13] != 0,
            },
            steering_divisor_raw,
            steering_constructor_axis_flag: d[4] != 0,
            steering_tail_raw,
            environment_flags_at_type_record_0xc0: 8,
        })
    }

    /// `FUN_00420450`: add 0..approximately 10% constructor jitter to the
    /// authored Sub-A target using the low word of one retail RNG result.
    pub fn randomized_target_speed(self, random_sample: u16) -> i32 {
        let base = i32::from(self.target_speed_base_raw);
        base + (i32::from(random_sample >> 8) * base) / 0x0a00
    }
}

fn read_i16(bytes: &[u8], offset: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// Previous-frame body basis in the executable's signed Q31 domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HoverBasis {
    pub lateral: [i32; 3],
    pub up: [i32; 3],
    pub forward: [i32; 3],
}

impl HoverBasis {
    /// Exact `FUN_00413F70` Euler-to-body-basis conversion.
    ///
    /// Retail reads the three signed entity angle words, folds each lookup
    /// through the 4,096-entry quarter-sine table, narrows after every Q15
    /// product, and finally stores each signed result in the high word of a
    /// Q31 component. The three contiguous vectors are the lateral, up, and
    /// forward axes used directly by hover physics. Keeping those narrowing
    /// points is significant: composing renderer floats and quantizing at the
    /// end differs by several raw bits away from cardinal headings.
    pub fn from_angle_words(heading: i16, pitch: i16, roll: i16) -> Self {
        let sin_roll = retail_sine_q15(roll as u32);
        let cos_roll = retail_sine_q15((i32::from(roll).wrapping_add(0x4000)) as u32);
        let sin_heading = retail_sine_q15(heading as u32);
        let cos_heading = retail_sine_q15((i32::from(heading).wrapping_add(0x4000)) as u32);
        let sin_pitch = retail_sine_q15(pitch as u32);
        let cos_pitch = retail_sine_q15((i32::from(pitch).wrapping_add(0x4000)) as u32);

        let heading_sin_roll_cos = q15_word_mul(sin_heading, cos_roll);
        let heading_cos_roll_cos = q15_word_mul(cos_heading, cos_roll);
        let heading_sin_roll_sin = q15_word_mul(sin_heading, sin_roll);
        let heading_cos_roll_sin = q15_word_mul(cos_heading, sin_roll);

        Self {
            lateral: [
                word_to_q31(
                    q15_word_mul(sin_pitch, heading_cos_roll_sin)
                        .wrapping_add(heading_sin_roll_cos),
                ),
                word_to_q31(q15_word_mul(cos_pitch, sin_roll)),
                word_to_q31(
                    q15_word_mul(sin_pitch, heading_sin_roll_sin)
                        .wrapping_sub(heading_cos_roll_cos),
                ),
            ],
            up: [
                word_to_q31(
                    q15_word_mul(sin_pitch, heading_cos_roll_cos)
                        .wrapping_sub(heading_sin_roll_sin),
                ),
                word_to_q31(q15_word_mul(cos_pitch, cos_roll)),
                word_to_q31(
                    q15_word_mul(sin_pitch, heading_sin_roll_cos)
                        .wrapping_add(heading_cos_roll_sin),
                ),
            ],
            forward: [
                word_to_q31(q15_word_mul(cos_pitch, cos_heading)),
                word_to_q31(-sin_pitch),
                word_to_q31(q15_word_mul(cos_pitch, sin_heading)),
            ],
        }
    }
}

/// One retail signed Q15 multiply, including the immediate signed-word
/// narrowing performed by `FUN_00413F70` after every intermediate product.
fn q15_word_mul(lhs: i32, rhs: i32) -> i32 {
    ((lhs * rhs) >> 15) as i16 as i32
}

fn word_to_q31(value: i32) -> i32 {
    (value as i16 as i32) << 16
}

/// Inputs retained between the player callback's heading stage and the entity
/// callback's C/B/A force stages in the same retail update.
#[derive(Debug, Clone, Copy)]
pub struct HoverFrameForces {
    pub(crate) basis: HoverBasis,
    pub(crate) throttle: f32,
    pub(crate) target_speed_raw: i32,
    pub(crate) drive_scale_percent: i32,
}

impl HoverFrameForces {
    pub(crate) fn new(basis: HoverBasis, throttle: f32, target_speed_raw: i32) -> Self {
        Self {
            basis,
            throttle,
            target_speed_raw,
            drive_scale_percent: 100,
        }
    }
}

impl Default for HoverFrameForces {
    fn default() -> Self {
        Self {
            basis: HoverBasis {
                lateral: [0x7ffe_0000, 0, 0],
                up: [0, 0x7ffe_0000, 0],
                forward: [0, 0, 0x7ffe_0000],
            },
            throttle: 0.0,
            target_speed_raw: HoverPhysicsConfig::default().target_speed_base_raw.into(),
            drive_scale_percent: 100,
        }
    }
}

/// Exact default Relative-mode keyboard channel for a fully held left/right
/// binding during this input interval (`FUN_004445E0`).
fn keyboard_relative_axis_raw(axis: f32, dt_us: u32, sensitivity: u8) -> i16 {
    let dt_us = dt_us.min(RETAIL_FRAME_DELTA_MAX_US);
    if dt_us == 0 || axis == 0.0 {
        return 0;
    }
    let held_us = i64::from(dt_us);
    let sensitivity = i64::from(sensitivity.min(KEYBOARD_SENSITIVITY_MAX as u8));
    let quantized = sensitivity * held_us / i64::from(KEYBOARD_SENSITIVITY_MAX);
    let amplitude = quantized * i64::from(KEYBOARD_FULL_SCALE_RAW) / held_us;
    if axis.is_sign_negative() {
        -(amplitude as i16)
    } else {
        amplitude as i16
    }
}

/// Default Relative-mode keyboard channel used by LEFT/RIGHT.
pub(crate) fn keyboard_turn_raw(turn: f32, dt_us: u32, sensitivity: u8) -> i16 {
    keyboard_relative_axis_raw(turn, dt_us, sensitivity)
}

/// Default Relative-mode keyboard channel used by UP/DOWN.
///
/// The 2026-07-16 retail trace measured stable values of +2303/-2303 at the
/// default sensitivity 10 and a 20 ms tick. The pitch and turn bindings pass
/// through the same duty-cycle quantization before their mode-specific
/// integrators consume them.
pub(crate) fn keyboard_pitch_raw(pitch: f32, dt_us: u32, sensitivity: u8) -> i16 {
    keyboard_relative_axis_raw(pitch, dt_us, sensitivity)
}

/// Type-46 Sub-D plus `FUN_00420360`: derive the wrapping yaw/roll step.
pub(crate) fn steering_step_raw(turn_raw: i16, dt_us: u32, steering_divisor_raw: i32) -> i16 {
    let dt_us = dt_us.min(RETAIL_FRAME_DELTA_MAX_US);
    if steering_divisor_raw == 0 {
        return 0;
    }
    let steer_rate = i32::from(turn_raw) * STEERING_NUMERATOR / steering_divisor_raw;
    ((i64::from(steer_rate) * i64::from(dt_us >> 2)) >> 15) as i16
}

/// Sub-D plus `FUN_00420360`: update the wrapping entity heading word.
pub(crate) fn steer_heading_raw(
    heading_raw: i16,
    turn_raw: i16,
    dt_us: u32,
    steering_divisor_raw: i32,
) -> i16 {
    heading_raw.wrapping_sub(steering_step_raw(turn_raw, dt_us, steering_divisor_raw))
}

/// Sub-B lateral correction followed by Sub-A signed propulsion. Velocity is
/// kept as the original three signed words; throttle magnitude is discarded.
pub(crate) fn apply_horizontal_velocity_raw(
    config: HoverPhysicsConfig,
    target_speed_raw: i32,
    drive_scale_percent: i32,
    throttle: f32,
    basis: HoverBasis,
    velocity: &mut [i16; 3],
    dt_us: u32,
) {
    let dt_us = dt_us.min(RETAIL_FRAME_DELTA_MAX_US);
    apply_lateral_correction_raw(config, basis.lateral, velocity, dt_us);

    let direction = if throttle > 0.0 {
        1
    } else if throttle < 0.0 {
        -1
    } else {
        return;
    };
    let projected = dot_q31(basis.forward, *velocity);
    let mut error = target_speed_raw
        .wrapping_mul(direction)
        .wrapping_sub(projected);
    let coefficient = if error.wrapping_mul(direction) < 0 {
        i32::from(config.overspeed_correction_raw)
    } else {
        i32::from(config.drive_acceleration_raw)
    };
    let signed_rate = (coefficient * drive_scale_percent / 100) * direction;
    let limited = time_scaled_q31(signed_rate, dt_us);
    if limited.unsigned_abs() < error.unsigned_abs() {
        error = limited;
    }

    for (word, component) in velocity.iter_mut().zip(basis.forward) {
        *word = word.wrapping_add(q31_mul(component, error) as i16);
    }
}

fn apply_lateral_correction_raw(
    config: HoverPhysicsConfig,
    lateral: [i32; 3],
    velocity: &mut [i16; 3],
    dt_us: u32,
) {
    let projected = dot_q31(lateral, *velocity);
    let threshold = time_scaled_q31(config.lateral_dead_zone_raw, dt_us);
    let correction_rate = time_scaled_q31(config.lateral_correction_raw, dt_us);
    let correction = if projected < -threshold {
        -correction_rate
    } else if projected > threshold {
        correction_rate
    } else {
        projected
    };
    for (word, component) in velocity.iter_mut().zip(lateral) {
        *word = word.wrapping_sub(q31_mul(component, correction) as i16);
    }
}

pub(crate) fn dot_q31(basis: [i32; 3], velocity: [i16; 3]) -> i32 {
    basis
        .into_iter()
        .zip(velocity)
        .fold(0i32, |sum, (component, word)| {
            sum.wrapping_add(q31_mul(component, i32::from(word)))
        })
}

fn time_scaled_q31(value: i32, dt_us: u32) -> i32 {
    ((i64::from(value) * (i64::from(dt_us) << 11)) >> 31) as i32
}

pub(crate) fn q31_mul(component: i32, value: i32) -> i32 {
    ((i64::from(component) * i64::from(value)) >> 31) as i32
}

/// `FUN_00412DA0`: integrate one signed-8.8 position word with one signed
/// velocity word after every force callback has run.
pub(crate) fn integrate_position_word(position: i16, velocity: i16, dt_us: u32) -> i16 {
    let dt_us = dt_us.min(RETAIL_FRAME_DELTA_MAX_US);
    let delta = ((i64::from(dt_us >> 5) * i64::from(velocity)) >> 15) as i16;
    position.wrapping_add(delta)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPE46_A: [u8; 6] = [0x78, 0x05, 0xd4, 0xfe, 0xb8, 0x0b];
    const TYPE46_B: [u8; 8] = [0, 0, 0, 0, 0xf4, 0x01, 0, 0];
    const TYPE46_C: [u8; 16] = [
        0x96, 0, 0x7d, 0, 0, 0, 0x90, 0, 0x64, 0, 0xc8, 0, 1, 1, 0, 0,
    ];
    const TYPE46_D: [u8; 12] = [0x1c, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];

    #[test]
    fn decodes_authored_type46_blocks() {
        let config =
            HoverPhysicsConfig::from_blocks(&TYPE46_A, &TYPE46_B, &TYPE46_C, &TYPE46_D).unwrap();
        assert_eq!(config, HoverPhysicsConfig::default());
        assert!(
            HoverPhysicsConfig::from_blocks(&TYPE46_A[..5], &TYPE46_B, &TYPE46_C, &TYPE46_D)
                .is_none()
        );
    }

    #[test]
    fn constructor_target_jitter_matches_retail_endpoints() {
        let config = HoverPhysicsConfig::default();
        assert_eq!(config.randomized_target_speed(0), 3000);
        assert_eq!(config.randomized_target_speed(u16::MAX), 3298);
    }

    #[test]
    fn default_keyboard_turn_and_yaw_keep_signed_asymmetry() {
        assert_eq!(keyboard_turn_raw(1.0, 20_000, 10), 0x08ff);
        assert_eq!(keyboard_turn_raw(-1.0, 20_000, 10), -0x08ff);
        assert_eq!(keyboard_turn_raw(1.0, 20_000, 0), 0);
        assert_eq!(keyboard_turn_raw(1.0, 20_000, 15), 0x0d80);
        assert_eq!(steer_heading_raw(0, 0x08ff, 20_000, 28), -652);
        assert_eq!(steer_heading_raw(0, -0x08ff, 20_000, 28), 653);
    }

    #[test]
    fn default_keyboard_pitch_matches_keyed_retail_capture() {
        assert_eq!(keyboard_pitch_raw(1.0, 20_000, 10), 0x08ff);
        assert_eq!(keyboard_pitch_raw(-1.0, 20_000, 10), -0x08ff);
        assert_eq!(keyboard_pitch_raw(1.0, 20_000, 15), 0x0d80);
    }

    #[test]
    fn drive_and_lateral_rates_use_retail_fixed_point() {
        assert_eq!(time_scaled_q31(1400, 20_000), 26);
        assert_eq!(time_scaled_q31(-1400, 20_000), -27);
        assert_eq!(time_scaled_q31(-300, 20_000), -6);
        assert_eq!(time_scaled_q31(300, 20_000), 5);
        assert_eq!(time_scaled_q31(500, 20_000), 9);

        let config = HoverPhysicsConfig::default();
        let basis = HoverBasis {
            lateral: [-0x7ffe_0000, 0, 0],
            up: [0, 0x7ffe_0000, 0],
            forward: [0, 0, -0x7ffe_0000],
        };
        let mut velocity = [-100, 0, 0];
        apply_horizontal_velocity_raw(config, 3000, 100, 0.0, basis, &mut velocity, 20_000);
        assert_eq!(velocity, [-91, 0, 0]);
    }

    #[test]
    fn cardinal_basis_keeps_retail_endpoint_products() {
        let basis = HoverBasis::from_angle_words(-0x4000, 0, 0);
        // Signed right shift makes the negative endpoint product -0x7fff,
        // while the corresponding positive product truncates to +0x7ffe.
        assert_eq!(basis.forward, [0, 0, -0x7fff_0000]);
        assert_eq!(basis.lateral, [-0x7fff_0000, 0, 0]);
        assert_eq!(basis.up, [0, 0x7ffe_0000, 0]);
    }

    #[test]
    fn body_basis_matches_july_16_retail_capture_bit_for_bit() {
        // 20260716-031848-skimmer-controls-keyed.jsonl sample 0. The same
        // comparison was run over all 2,250 samples in that capture plus all
        // 2,750 samples in the skimmer/VTOL transition capture; every one of
        // the nine words matched.
        let basis = HoverBasis::from_angle_words(37_125u16 as i16, 68, 702);
        assert_eq!(
            basis,
            HoverBasis {
                lateral: [-869_924_864, 143_917_056, 1_957_691_392],
                up: [45_678_592, 2_142_502_912, -137_297_920],
                forward: [-1_962_541_056, -13_959_168, -870_973_440],
            }
        );
    }

    #[test]
    fn position_integrator_wraps_signed_words() {
        assert_eq!(integrate_position_word(1000, 3000, 20_000), 1057);
        assert_eq!(integrate_position_word(i16::MAX, 3000, 20_000), -32712);
    }

    #[test]
    fn retail_delta_cap_prevents_hitch_impulses() {
        assert_eq!(
            steer_heading_raw(0, 0x08ff, 1_000_000, 28),
            steer_heading_raw(0, 0x08ff, RETAIL_FRAME_DELTA_MAX_US, 28)
        );
        assert_eq!(
            integrate_position_word(1000, 3000, 1_000_000),
            integrate_position_word(1000, 3000, RETAIL_FRAME_DELTA_MAX_US)
        );
    }
}
