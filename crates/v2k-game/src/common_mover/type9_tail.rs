//! Bounded normal-callback phases for ordinary first-world type 9 and the
//! shared master-motion suffix used by common actors.
//!
//! This module keeps two retail owners separate:
//!
//! - `FUN_0040E870` applies `FUN_0040E100`, conditionally applies
//!   `FUN_0040DF70`, and later calls `FUN_0040E370`, whose ordinary live
//!   branches are retained separately in [`super::type9_surface`].
//! - `FUN_00412DA0` subsequently owns the state gates, optional velocity
//!   approach, and signed-word position integration.
//!
//! In particular, type 9 is snapped against terrain at its *old* X/Z before
//! the master function integrates horizontal position. Combining those phases
//! would sample a different terrain cell at boundaries. The drag and snap
//! helpers deliberately remain separate from the complete callback:
//! first-world type 9 has Section-12 byte `+0x72 = 1`, so `FUN_0040E370`
//! subsequently mutates a timer, can consume RNG, and can emit underwater
//! bubbles before the callback returns zero.

use v2k_formats::terrain::TerrainGrid;

/// Type 9's effective environment flags after the normal callback applies its
/// behavior mask: `(0x2f | 0) & !0`.
pub const ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS: u32 = 0x2f;

/// Type 9's authored self-mass word consumed by `FUN_0044EC60`.
pub const ORDINARY_TYPE9_SELF_MASS_RAW: u16 = 10;

/// The captured/static first-world environment selects the mode-zero fallback
/// in `FUN_0044EC60`.
pub const ORDINARY_TYPE9_RUNTIME_WIND_MODE: i32 = 0;

/// `DAT_004F71A4` in the first-world runtime.
pub const ORDINARY_TYPE9_DRAG_STRENGTH: i32 = 3;

const ENVIRONMENT_WIND_DRAG_FLAG: u32 = 0x0000_0008;
const MASTER_MOTION_ENABLE_STATE_BIT: u32 = 0x0004_0000;
const MASTER_ZERO_APPROACH_STATE_BITS: u32 = 0x0088_0000;
pub(crate) const MASTER_GROUNDED_STATE_BIT: u32 = 0x0080_0000;
const MASTER_SUPPRESS_MOTION_STATE_BIT: u32 = 0x0800_0000;
const POSITION_CHANGED_STATE_BIT: u32 = 0x0000_0020;

/// State domains which must be known before common master motion can be
/// planned without guessing at any retail control-flow gate.
pub const COMMON_MASTER_MOTION_REQUIRED_STATE_MASK: u32 = MASTER_MOTION_ENABLE_STATE_BIT
    | MASTER_ZERO_APPROACH_STATE_BITS
    | MASTER_SUPPRESS_MOTION_STATE_BIT;

/// Planned retained-state publication for the common post-callback master
/// motion suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedCommonMasterMotion {
    pub(crate) active: bool,
    pub(crate) position_after_raw: [i16; 3],
    pub(crate) velocity_after_raw: [i16; 3],
    pub(crate) state_write_mask: u32,
    pub(crate) state_write_bits: u32,
}

/// Live inputs to the mode-zero branch of `FUN_0044EC60`. The world adapter
/// authenticates wind mode zero; B0 is the scheduler's nonzero callback mass,
/// including that visit's B2 contribution, and strength comes from the level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type9ModeZeroDrag {
    pub callback_mass_raw: std::num::NonZeroU16,
    pub strength: u32,
}

/// Type9's effective flags contain drag bit3. Apply E100's mode-zero drag
/// using the carry-adjusted/capped callback delta. DF70's ground snap and
/// E370's surface lifecycle remain separate phases.
pub fn apply_type9_environment_drag_raw(
    velocity_raw: &mut [i16; 3],
    effective_elapsed_micros: u32,
    drag: Type9ModeZeroDrag,
) {
    let drag_factor = ordinary_type9_no_wind_drag_factor(effective_elapsed_micros, drag);
    apply_mode_zero_drag_raw(velocity_raw, drag_factor);
}

/// Apply ordinary type 9's exact `FUN_0040DF70` ground-snap phase.
///
/// The authored flags are `0x2f`, so bit `0x40` is absent: the active model's
/// origin-Y word is deliberately *not* added to the terrain height. This phase
/// clears vertical velocity, samples terrain at the old X/Z, and sets state
/// bit `0x00800000`.
pub fn apply_type9_ground_snap_raw(
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    state_flags: &mut u32,
    terrain: &TerrainGrid,
) -> i16 {
    velocity_raw[1] = 0;
    let terrain_height_raw = bilinear_terrain_height_raw(terrain, position_raw[0], position_raw[2]);
    position_raw[1] = terrain_height_raw;
    *state_flags |= MASTER_GROUNDED_STATE_BIT;
    terrain_height_raw
}

/// Apply the exact motion-owned suffix of `FUN_00412DA0`.
///
/// The return value is retail's callback result propagation. Motion only runs
/// when state bit `0x00040000` is set, the entity callback returned zero, and
/// state bit `0x08000000` is clear. The optional velocity approach is narrower:
/// it requires both bits in `0x00880000`. Nonzero velocity then integrates
/// X/Z/Y, in that order in retail memory, with `effective_elapsed >> 5` and
/// wrapping signed-word storage, and sets state bit `0x20`.
///
/// `effective_elapsed_micros` must be the carry-adjusted and capped owner step
/// computed by the scheduler portion of `FUN_00412DA0`; passing a raw frame
/// delta is not equivalent when the owner defers or carries time.
pub fn apply_common_master_motion_raw(
    position_raw: &mut [i16; 3],
    velocity_raw: &mut [i16; 3],
    state_flags: &mut u32,
    effective_elapsed_micros: u32,
    callback_result: u32,
) -> u32 {
    let state_at_gate = *state_flags;
    if state_at_gate & MASTER_MOTION_ENABLE_STATE_BIT == 0 {
        return callback_result;
    }
    if callback_result != 0 {
        return callback_result;
    }
    if state_at_gate & MASTER_SUPPRESS_MOTION_STATE_BIT != 0 {
        return 0;
    }

    if state_at_gate & MASTER_ZERO_APPROACH_STATE_BITS == MASTER_ZERO_APPROACH_STATE_BITS {
        let approach_raw = (effective_elapsed_micros >> 8) as i16;
        for component in velocity_raw.iter_mut() {
            *component = approach_zero_raw(*component, approach_raw);
        }
        *state_flags = state_at_gate & !MASTER_GROUNDED_STATE_BIT;
    }

    if velocity_raw.iter().any(|&component| component != 0) {
        // Retail stores X/Y/Z consecutively but writes X, Z, then Y. The
        // updates are independent, so the array-order loop is equivalent while
        // retaining the exact per-axis integer expression.
        for axis in [0, 2, 1] {
            position_raw[axis] = integrate_position_word_raw(
                position_raw[axis],
                velocity_raw[axis],
                effective_elapsed_micros,
            );
        }
        *state_flags |= POSITION_CHANGED_STATE_BIT;
    }

    0
}

/// Plan `FUN_00412DA0`'s final master-motion reread and retained writes.
///
/// Callers preflight only the exact control bits consumed by the helper, then
/// publish this data-bearing plan after their callback-specific suffix. A
/// terminal actor passes zero because the variant-one mark has already cleared
/// motion-enable bit `0x00040000`; representing that no-op as a plan keeps the
/// final reread in the executable transaction rather than as an assertion.
pub(crate) fn plan_common_master_motion(
    mut position_raw: [i16; 3],
    mut velocity_raw: [i16; 3],
    mut state_flags: u32,
    elapsed_micros: u32,
) -> PlannedCommonMasterMotion {
    debug_assert_eq!(state_flags & !COMMON_MASTER_MOTION_REQUIRED_STATE_MASK, 0);
    let active = state_flags & MASTER_MOTION_ENABLE_STATE_BIT != 0
        && state_flags & MASTER_SUPPRESS_MOTION_STATE_BIT == 0;
    let state_before = state_flags;
    let callback_result = apply_common_master_motion_raw(
        &mut position_raw,
        &mut velocity_raw,
        &mut state_flags,
        elapsed_micros,
        0,
    );
    debug_assert_eq!(callback_result, 0);

    let mut state_write_mask =
        (state_before ^ state_flags) & COMMON_MASTER_MOTION_REQUIRED_STATE_MASK;
    if state_flags & POSITION_CHANGED_STATE_BIT != 0 {
        state_write_mask |= POSITION_CHANGED_STATE_BIT;
    }
    PlannedCommonMasterMotion {
        active,
        position_after_raw: position_raw,
        velocity_after_raw: velocity_raw,
        state_write_mask,
        state_write_bits: state_flags & state_write_mask,
    }
}

fn ordinary_type9_no_wind_drag_factor(
    effective_elapsed_micros: u32,
    drag: Type9ModeZeroDrag,
) -> i32 {
    debug_assert_ne!(
        ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS & ENVIRONMENT_WIND_DRAG_FLAG,
        0
    );
    debug_assert_eq!(ORDINARY_TYPE9_RUNTIME_WIND_MODE, 0);

    // `FUN_0044EC60` masks the signed 64-bit product to its low u32 before
    // dividing by `(mass << 3)`.
    let numerator =
        (u64::from(effective_elapsed_micros) * u64::from(drag.strength)) & u64::from(u32::MAX);
    let divisor = u64::from(drag.callback_mass_raw.get()) << 3;
    (numerator / divisor) as u32 as i32
}

fn apply_mode_zero_drag_raw(velocity_raw: &mut [i16; 3], factor: i32) {
    for component in velocity_raw {
        // MSVC uses a wrapping signed 32-bit multiply, arithmetic shift, then
        // narrows the correction before the wrapping signed-word subtraction.
        let correction = factor.wrapping_mul(i32::from(*component)) >> 15;
        *component = component.wrapping_sub(correction as i16);
    }
}

fn approach_zero_raw(value: i16, step: i16) -> i16 {
    if value < 1 {
        if i32::from(value) < -i32::from(step) {
            value.wrapping_add(step)
        } else {
            0
        }
    } else if step < value {
        value.wrapping_sub(step)
    } else {
        0
    }
}

fn integrate_position_word_raw(position: i16, velocity: i16, effective_elapsed_micros: u32) -> i16 {
    let elapsed_step = effective_elapsed_micros >> 5;
    let product = elapsed_step.wrapping_mul(i32::from(velocity) as u32) as i32;
    position.wrapping_add((product >> 15) as i16)
}

/// Exact integer `FUN_00445860` terrain lookup in signed engine-Y units.
pub(super) fn bilinear_terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    terrain.bilinear_height_raw(x_raw, z_raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    fn drag(mass: u16, strength: u32) -> Type9ModeZeroDrag {
        Type9ModeZeroDrag {
            callback_mass_raw: std::num::NonZeroU16::new(mass).unwrap(),
            strength,
        }
    }

    fn flat_terrain(height: i8) -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn set_height(terrain: &mut TerrainGrid, x: usize, z: usize, height: i8) {
        terrain.cells[x * GRID_SIZE + z].height = height as u8;
    }

    #[test]
    fn type9_drag_factor_is_750_at_twenty_milliseconds() {
        assert_eq!(ordinary_type9_no_wind_drag_factor(20_000, drag(10, 3)), 750);
        assert_eq!(ordinary_type9_no_wind_drag_factor(20_000, drag(20, 3)), 375);
        assert_eq!(ordinary_type9_no_wind_drag_factor(20_000, drag(10, 0)), 0);

        let mut velocity = [i16::MAX, i16::MIN, 1_000];
        apply_mode_zero_drag_raw(&mut velocity, 750);
        assert_eq!(velocity, [32_018, -32_018, 978]);
    }

    #[test]
    fn callback_snaps_old_x_before_master_integrates_horizontal_position() {
        let mut terrain = flat_terrain(10);
        set_height(&mut terrain, 1, 0, 20);
        let mut position = [0, -5, 0];
        let mut velocity = [8_192, 777, 0];
        let mut state = MASTER_MOTION_ENABLE_STATE_BIT;

        apply_type9_environment_drag_raw(&mut velocity, 0, drag(10, 3));
        assert_eq!(
            apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut state, &terrain),
            320
        );
        assert_eq!(position, [0, 320, 0]);
        assert_eq!(velocity, [8_192, 0, 0]);

        // Retail's later FUN_0040E370 phase remains a separate typed planner.
        // Given its statically recovered zero return on the ordinary live
        // branch, the independently exact master suffix then integrates the
        // effective owner step.
        assert_eq!(
            apply_common_master_motion_raw(&mut position, &mut velocity, &mut state, 32_768, 0,),
            0
        );
        assert_eq!(position, [0x100, 320, 0]);
        assert_eq!(
            bilinear_terrain_height_raw(&terrain, position[0], position[2]),
            640
        );
    }

    #[test]
    fn terrain_lookup_wraps_ffff_and_sign_extends_height_bytes() {
        let mut terrain = flat_terrain(0);
        set_height(&mut terrain, 255, 255, -1);
        set_height(&mut terrain, 0, 255, 1);
        set_height(&mut terrain, 255, 0, 3);
        set_height(&mut terrain, 0, 0, 5);
        assert_eq!(bilinear_terrain_height_raw(&terrain, -1, -1), 158);
    }

    #[test]
    fn type9_ground_snap_has_no_model_origin_offset() {
        let terrain = flat_terrain(10);
        let mut position = [0x1234, -900, 0x5678];
        let mut velocity = [0, 999, 0];
        let mut state = 0;
        let terrain_height =
            apply_type9_ground_snap_raw(&mut position, &mut velocity, &mut state, &terrain);

        assert_eq!(terrain_height, 320);
        assert_eq!(position[1], 320);
        assert_ne!(position[1], 320 + 165);
        assert_eq!(velocity[1], 0);
        assert_ne!(state & MASTER_GROUNDED_STATE_BIT, 0);
    }

    #[test]
    fn master_motion_preserves_all_three_retail_gates() {
        let initial_position = [100, 200, 300];
        let initial_velocity = [4_000, 0, 0];

        let mut position = initial_position;
        let mut velocity = initial_velocity;
        let mut state = 0;
        assert_eq!(
            apply_common_master_motion_raw(&mut position, &mut velocity, &mut state, 20_000, 7,),
            7
        );
        assert_eq!(position, initial_position);

        let mut position = initial_position;
        let mut velocity = initial_velocity;
        let mut state = MASTER_MOTION_ENABLE_STATE_BIT;
        assert_eq!(
            apply_common_master_motion_raw(&mut position, &mut velocity, &mut state, 20_000, 7,),
            7
        );
        assert_eq!(position, initial_position);

        let mut position = initial_position;
        let mut velocity = initial_velocity;
        let mut state = MASTER_MOTION_ENABLE_STATE_BIT | MASTER_SUPPRESS_MOTION_STATE_BIT;
        assert_eq!(
            apply_common_master_motion_raw(&mut position, &mut velocity, &mut state, 20_000, 0,),
            0
        );
        assert_eq!(position, initial_position);
    }

    #[test]
    fn captured_type9_state_does_not_take_master_zero_approach() {
        let mut position = [0, 0, 0];
        let mut velocity = [100, -100, 1];
        let mut state = 0x00c6_8805;
        assert_eq!(
            state & MASTER_ZERO_APPROACH_STATE_BITS,
            MASTER_GROUNDED_STATE_BIT
        );

        assert_eq!(
            apply_common_master_motion_raw(&mut position, &mut velocity, &mut state, 20_000, 0,),
            0
        );
        assert_eq!(velocity, [100, -100, 1]);
        assert_ne!(state & MASTER_GROUNDED_STATE_BIT, 0);
        assert_ne!(state & POSITION_CHANGED_STATE_BIT, 0);
    }

    #[test]
    fn master_zero_approach_is_bounded_to_both_required_bits() {
        let mut position = [0, 0, 0];
        let mut velocity = [100, -100, 1];
        let mut state = MASTER_MOTION_ENABLE_STATE_BIT | MASTER_ZERO_APPROACH_STATE_BITS;

        apply_common_master_motion_raw(&mut position, &mut velocity, &mut state, 20_000, 0);
        assert_eq!(velocity, [22, -22, 0]);
        assert_eq!(state & MASTER_GROUNDED_STATE_BIT, 0);
        assert_ne!(state & POSITION_CHANGED_STATE_BIT, 0);
    }
}
