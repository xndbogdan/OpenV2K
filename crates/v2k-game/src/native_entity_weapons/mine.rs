//! Type48 Proximity Mine: class34's falling and armed styles.
//!
//! 44E770's row 9 is depth-charge-shaped (`{0, 1000, 0, 48}`), so the mine is
//! dropped as a depth charge is and owned (`+60`) by its shooter. Class34's
//! first style `4C8548` (initializer `40B080`) clears Secondary then Tertiary
//! and publishes the lifetime-0 falling Primary (`FUN_00406B00 -> 406BA0`). Within
//! 300 of the higher of its cell's ground and the sea, the tick returns tag
//! 9C00, whose style `+04` (`40C750 -> 40C6B0`) installs the armed style
//! `4C8590`. Its initializer `FUN_0040B0B0` clears Secondary, publishes the shared
//! target search (`401F80(entity, 1, 0, 0)`: one visit in four, the type's
//! 0x300 range and capability mask 8, never the shooter) and replaces the
//! Primary with the floating tick (`FUN_00406E90 -> 406F20`: Sub-C lift on the
//! waved sea, then Sub-B drag). A found target runs the armed style's `+04`,
//! `40CEF0`; both styles' `+18` pair and `+1C` static hooks are `40CEF0` too,
//! and `410C10` takes the alternate class49 (`40BD20`). Both styles' `+10`,
//! `+14`, `+28` and `+2C` are null. Neither initializer draws a random word:
//! the Type has no Sub-A for `406070` to reset.
//!
//! Model variables 1 and 2 (`40A950`, the type's two-word bank) are the
//! tasks' Sub-K words; 409A80 zeroes the bank at birth.

use super::EntityWeaponBlock;
use crate::{
    entity_collision_state::{
        CommonMoverComponentTopology, CommonMoverGklPayloads, EntityTypeRuntimeMetadata,
        RetailRuntimeValue,
    },
    hover::q31_mul,
};

pub const MINE_ENTITY_TYPE: u32 = 48;
pub const MINE_MODEL_ID: u16 = 241;
/// Class34 style 0, initializer `40B080`.
pub const MINE_FALLING_STYLE_ADDRESS: u32 = 0x004c_8548;
/// Class34 style 1, initializer `40B0B0`.
pub const MINE_ARMED_STYLE_ADDRESS: u32 = 0x004c_8590;
/// `406BA0` takes its tag once the clearance is at most this.
pub const MINE_ARMING_CLEARANCE_RAW: i32 = 299;

pub(crate) fn authenticate_mine_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), EntityWeaponBlock> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(EntityWeaponBlock::Metadata);
    };
    let topology = CommonMoverComponentTopology {
        sub_b: true,
        sub_c: true,
        sub_k: true,
        ..CommonMoverComponentTopology::default()
    };
    if metadata.model_slots != [MINE_MODEL_ID; 4]
        || metadata.mass_raw != 10
        || metadata.capability_flags != 0x40
        || metadata.initial_health_raw != Some(1)
        || metadata.damage_profile != Some(super::grenade::GRENADE_DAMAGE_PROFILE)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(topology)
        || metadata.model_variable_count_raw != RetailRuntimeValue::Known(2)
        || metadata.common_mover_gkl_payloads
            != RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some([1, 2]),
                sub_l: None,
            })
        || !matches!(
            metadata.sub_b_lateral_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        // 406F20 calls 41F1C0 at the centre; an offset sample needs the body-up
        // point this owner does not take.
        || !matches!(
            metadata.sub_c_lift_descriptor,
            RetailRuntimeValue::Known(Some(c)) if c.offset_sample_raw == 0
        )
        || initializer.initializer_state_flags_raw != 8
        || initializer.common_axis_descriptor.strict_axis_limit_raw != 0x300
        || initializer.common_axis_descriptor.raw_word_at_0x04 != 8
        || initializer.behavior_choices.len() != 1
        || initializer.behavior_choices[0].weight_rule_id != 1
        || initializer.behavior_choices[0].weight_multiplier != 1
        || initializer.behavior_choices[0].behavior_class_id != 34
        || initializer.alternate_behavior_class_ref != 49
    {
        return Err(EntityWeaponBlock::Metadata);
    }
    Ok(())
}

/// The class34 Primary's tick, by style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinePrimary {
    /// `406BA0`, published by `406B00`.
    Falling,
    /// `406F20`, published by `406E90`.
    Armed,
}

/// `(3 * pitch + q31(elapsed << 11, goal - pitch)) / 4`, as both ticks ease it.
fn eased_pitch(pitch_raw: i16, goal_raw: i32, elapsed_micros: u32) -> i16 {
    let pitch = i32::from(pitch_raw);
    let sum = q31_mul(
        (elapsed_micros as i32).wrapping_shl(11),
        goal_raw.wrapping_sub(pitch),
    )
    .wrapping_add(pitch.wrapping_mul(3));
    (sum / 4) as i16
}

/// `FUN_00406BA0` on the body's heading/pitch and Sub-K words. Returns whether it
/// took tag 9C00: within 300 of the higher of its cell's ground byte << 5
/// and the sea.
pub fn step_falling_mine(
    rotation_raw: &mut [i16; 3],
    sub_k_raw: &mut [u16; 2],
    position_raw: [i16; 3],
    ground_raw: i16,
    sea_level_raw: i16,
    elapsed_micros: u32,
) -> bool {
    rotation_raw[0] = rotation_raw[0].wrapping_add((elapsed_micros >> 4) as i16);
    rotation_raw[1] = eased_pitch(rotation_raw[1], 0x1000, elapsed_micros);
    sub_k_raw[0] = sub_k_raw[0].wrapping_add((elapsed_micros >> 2) as u16);
    sub_k_raw[1] = 0;
    i32::from(position_raw[1]) - i32::from(ground_raw.max(sea_level_raw))
        <= MINE_ARMING_CLEARANCE_RAW
}

/// `FUN_00406F20`'s attitude and Sub-K prefix, before its Sub-C and Sub-B forces.
pub fn step_armed_mine_words(
    rotation_raw: &mut [i16; 3],
    sub_k_raw: &mut [u16; 2],
    elapsed_micros: u32,
) {
    rotation_raw[1] = eased_pitch(rotation_raw[1], 0, elapsed_micros);
    sub_k_raw[0] = sub_k_raw[0].wrapping_add((elapsed_micros >> 4) as u16);
    // Variable 2 is read unsigned.
    let step = q31_mul(
        (elapsed_micros as i32).wrapping_shl(11),
        0x4000 - i32::from(sub_k_raw[1]),
    );
    sub_k_raw[1] = sub_k_raw[1].wrapping_add(step as u16);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falling_turns_eases_and_tags_within_300_of_ground_or_sea() {
        let mut rotation = [0x7ff0, -0x400, 5];
        let mut k = [0xfff0, 77];
        let tagged = step_falling_mine(&mut rotation, &mut k, [0, 699, 0], 100, 400, 20_000);
        // 20000 >> 4 = 1250 wraps the heading; roll is untouched.
        assert_eq!(rotation[0], 0x7ff0i16.wrapping_add(1250));
        assert_eq!(rotation[2], 5);
        // q31(20000 << 11, 0x1400) = 0x1400 * 40960000 >> 31 = 97; (97 - 3072) / 4.
        assert_eq!(rotation[1], -743);
        assert_eq!(k, [0xfff0u16.wrapping_add(5000), 0]);
        assert!(tagged, "299 above the sea arms");
        let mut k = [0, 0];
        assert!(!step_falling_mine(
            &mut rotation,
            &mut k,
            [0, 700, 0],
            -50,
            400,
            0
        ));
        assert!(step_falling_mine(
            &mut rotation,
            &mut k,
            [0, 699, 0],
            400,
            -4096,
            0
        ));
    }

    #[test]
    fn armed_eases_pitch_to_zero_and_variable_two_unsigned_to_0x4000() {
        let mut rotation = [9, 0x800, 3];
        let mut k = [1, 0xc000];
        step_armed_mine_words(&mut rotation, &mut k, 20_000);
        // (q31(40960000, -0x800) + 0x1800) / 4: -40 + 6144 = 6104 / 4.
        assert_eq!(rotation, [9, 1526, 3]);
        assert_eq!(k[0], 1 + 1250);
        // 0x4000 - 0xC000 = -0x8000 unsigned-read: q31 is -625.
        assert_eq!(k[1], 0xc000u16.wrapping_sub(625));
    }
}
