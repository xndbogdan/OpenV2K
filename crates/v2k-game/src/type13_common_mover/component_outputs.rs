//! Authenticated Type-13 Sub-K/Sub-L animation output callbacks.
//!
//! `FUN_00409A80` binds the two K words to selectors 11/10 and the two L
//! words to selectors 6/7. These callbacks advance the retained signed words;
//! the live entity owner publishes them to those presentation bindings.
//! Before component binding, 09A80 separately allocates and zero-fills the
//! `type +0x110`-word entity bank. The output words therefore start at zero
//! independently of the cleared K/L allocation bytes.

use crate::common_mover::sub_d::target_body_projection_raw;
use crate::common_mover::type9_attitude::Type9BodyBasis;

/// `FUN_004243B0`: steering and post-Sub-G vertical-velocity smoothing.
pub(crate) fn advance_sub_k(
    output_raw: [i16; 2],
    smoothed_raw: i32,
    velocity_y_raw: i16,
) -> [i16; 2] {
    // The input at +0x0C is a dword written by the common-mover prelude, but
    // this callback reads only its signed low word. Both stores narrow before
    // the signed clamps; clamping the wide intermediate changes retail wrap.
    let steering = ((i32::from(output_raw[0]) - i32::from(smoothed_raw as i16) * 8) / 2) as i16;
    let vertical = ((i32::from(output_raw[1]) * 3 - i32::from(velocity_y_raw) * 8) / 4) as i16;
    [
        steering.clamp(-0x1800, 0x1800),
        vertical.clamp(-0x1200, 0x1200),
    ]
}

/// `FUN_0041BA00`: target/body projection or exact-turn drive followed by
/// the authored signed descriptor limits. Elapsed time is not consumed here.
pub(crate) fn advance_sub_l(
    output_raw: [i16; 2],
    exact_raw: i32,
    position_raw: [i16; 3],
    target_raw: [i16; 3],
    retained_body_basis: Type9BodyBasis,
    descriptor: [u8; 6],
) -> [i16; 2] {
    // FUN_0041E700 reads the retained entity matrix even though preceding
    // Sub-D/Sub-G callbacks may already have changed the Euler angle words.
    let projection = target_body_projection_raw(position_raw, target_raw, retained_body_basis);
    let old_lateral = i32::from(output_raw[0]);
    let lateral_numerator = if exact_raw == 0 {
        (i32::from(projection.lateral_raw) - old_lateral).clamp(-0x3000, 0x3000) + old_lateral * 4
    } else {
        // The branch tests the whole +0x10 dword, then reads a signed word
        // for the drive. A nonzero high word with low word zero still eases.
        (i32::from(exact_raw as i16) * 0x18).clamp(-0x3000, 0x3000) + old_lateral * 3
    };
    let lateral = (lateral_numerator / 4) as i16;
    let vertical = if target_raw[1] == 0 {
        // This is the absolute target-Y word at allocation +0x0A, not the
        // relative target height calculated by FUN_0041E700.
        0
    } else {
        let old_vertical = i32::from(output_raw[1]);
        let step = (-(i32::from(projection.vertical_raw) + old_vertical)).clamp(-0x2000, 0x2000);
        ((step + old_vertical * 3) / 4) as i16
    };
    let lateral_limit = i16::from_le_bytes([descriptor[2], descriptor[3]]);
    let vertical_limit = i16::from_le_bytes([descriptor[4], descriptor[5]]);
    [
        lateral.clamp(-lateral_limit, lateral_limit),
        vertical.clamp(-vertical_limit, vertical_limit),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_L;

    const BASIS: Type9BodyBasis = Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    };

    #[test]
    fn sub_k_uses_signed_truncation_low_word_input_and_store_before_clamp() {
        assert_eq!(advance_sub_k([-3, -3], 0, 0), [-1, -2]);
        assert_eq!(advance_sub_k([0; 2], 0x1_0000, 0), [0; 2]);
        assert_eq!(advance_sub_k([0; 2], 32_767, 32_767), [4, 2]);
        assert_eq!(advance_sub_k([0; 2], 10_000, 10_000), [0x1800, -0x1200]);
    }

    #[test]
    fn sub_l_distinguishes_zero_dword_projection_from_nonzero_low_word_drive() {
        assert_eq!(
            advance_sub_l(
                [0; 2],
                0,
                [0; 3],
                [100, 100, 100],
                BASIS,
                TYPE13_SEARCH_ATTACK_SUB_L
            ),
            [0x0c00, -0x0800]
        );
        assert_eq!(
            advance_sub_l(
                [0; 2],
                0x1_0000,
                [0; 3],
                [100, 100, 100],
                BASIS,
                TYPE13_SEARCH_ATTACK_SUB_L
            ),
            [0, -0x0800]
        );
        assert_eq!(
            advance_sub_l(
                [0; 2],
                -1,
                [0; 3],
                [100, 100, 100],
                BASIS,
                TYPE13_SEARCH_ATTACK_SUB_L
            ),
            [-6, -0x0800]
        );
    }

    #[test]
    fn sub_l_resets_on_absolute_target_y_zero_and_truncates_toward_zero() {
        assert_eq!(
            advance_sub_l(
                [-3, 1234],
                0x1_0000,
                [0, 100, 0],
                [100, 0, 100],
                BASIS,
                TYPE13_SEARCH_ATTACK_SUB_L
            ),
            [-2, 0]
        );
        let no_vertical_projection = Type9BodyBasis {
            up: [0; 3],
            ..BASIS
        };
        assert_eq!(
            advance_sub_l(
                [-3, -3],
                0x1_0000,
                [0; 3],
                [100, 100, 100],
                no_vertical_projection,
                TYPE13_SEARCH_ATTACK_SUB_L
            ),
            [-2, -1]
        );
    }

    #[test]
    fn sub_l_applies_authored_signed_limits_after_easing() {
        assert_eq!(
            advance_sub_l(
                [32_000, -32_000],
                0x1_0000,
                [0; 3],
                [100, 100, 100],
                BASIS,
                TYPE13_SEARCH_ATTACK_SUB_L
            ),
            [8_000, -8_000]
        );
    }
}
