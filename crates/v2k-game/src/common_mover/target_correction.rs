//! Detached target-correction phase from retail's common mover.
//!
//! `FUN_00401BB0` is called twice by `FUN_00401430`. Both calls share the
//! correction primitive, but their surrounding policies are intentionally
//! different:
//!
//! - the first call starts with a zero correction, applies half of the result
//!   to Sub-G runtime word `+0x1E`, then clamps that word to `[-4000, 4000]`;
//! - the second call clears Sub-O runtime dword `+0x10`, starts with the
//!   sentinel correction `1000`, and conditionally restores the saved dword.
//!
//! This module preserves those policies as pure plans. It does not resolve
//! handles, mutate component allocations, or activate the phase for live
//! actors.

/// Strict per-axis X/Z boundary for the normalized correction branch.
pub const TARGET_CORRECTION_NEAR_RADIUS_RAW: i32 = 0x0f00;
/// Dword written through the Sub-G descriptor at `+0x1C` on every active call.
pub const TARGET_CORRECTION_SUB_G_DESCRIPTOR_DWORD_1C_RAW: u32 = 0x8000;
/// Clamp applied by the first `FUN_00401430` caller after the correction.
pub const TARGET_CORRECTION_SUB_G_ANGLE_LIMIT_RAW: i16 = 4_000;
/// Initial output retained by the second caller when the primitive does not
/// take its normalized near-target branch.
pub const TARGET_CORRECTION_SECONDARY_SENTINEL_RAW: i32 = 1_000;

/// Controlled-entity fields read by `FUN_00401BB0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTargetCorrectionEntity {
    pub position_raw: [i16; 3],
    /// Entity dword `+0x28`, compared with normalized target Y.
    pub attitude_raw_28: i32,
}

/// Complete input to one invocation of retail `FUN_00401BB0`.
///
/// `entity == None` models a failed handle lookup. `target_context_present`
/// models the independent null check of the third function argument; retail
/// does not otherwise read that pointer in this function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTargetCorrectionRequest {
    pub entity: Option<CommonMoverTargetCorrectionEntity>,
    pub target_context_present: bool,
    pub target_position_raw: [i16; 3],
    /// Sub-G runtime word `+0x1E`.
    pub sub_g_runtime_angle_raw: i16,
}

/// Retail path selected by one target-correction invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverTargetCorrectionPath {
    MissingEntity,
    MissingTargetContext,
    Near,
    Far,
}

/// Writes and output override produced by one `FUN_00401BB0` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTargetCorrectionPlan {
    pub path: CommonMoverTargetCorrectionPath,
    /// `None` means retail returned before reaching the descriptor write.
    pub sub_g_descriptor_dword_1c_write: Option<u32>,
    /// Sub-G runtime word `+0x1E` after the call. Only the far path changes it.
    pub sub_g_runtime_angle_raw: i16,
    /// The near path overwrites the caller-owned output dword. Every other
    /// path preserves the caller's initializer.
    pub output_override_raw: Option<i32>,
}

/// Complete first-call policy at `0x0040167A..0x004016CD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverPrimaryTargetCorrectionPlan {
    pub correction: CommonMoverTargetCorrectionPlan,
    pub output_raw: i32,
    pub sub_g_runtime_angle_raw: i16,
}

/// Complete second-call policy at `0x0040182A..0x00401881`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverSecondaryTargetCorrectionPlan {
    pub correction: CommonMoverTargetCorrectionPlan,
    pub output_raw: i32,
    /// The caller writes zero before invoking `FUN_00401BB0`, then either
    /// leaves zero or restores this saved retail pointer/token.
    pub sub_o_link_raw: u32,
    pub restored_saved_sub_o_link: bool,
}

/// Plan one exact invocation of `FUN_00401BB0`.
pub fn plan_common_mover_target_correction(
    request: CommonMoverTargetCorrectionRequest,
) -> CommonMoverTargetCorrectionPlan {
    let Some(entity) = request.entity else {
        return inactive_plan(
            CommonMoverTargetCorrectionPath::MissingEntity,
            request.sub_g_runtime_angle_raw,
        );
    };
    if !request.target_context_present {
        return inactive_plan(
            CommonMoverTargetCorrectionPath::MissingTargetContext,
            request.sub_g_runtime_angle_raw,
        );
    }

    let displacement = retail_displacement(entity.position_raw, request.target_position_raw);
    let near = signed_low_word_abs(displacement[0]) < TARGET_CORRECTION_NEAR_RADIUS_RAW
        && signed_low_word_abs(displacement[2]) < TARGET_CORRECTION_NEAR_RADIUS_RAW;
    if !near {
        let quarter = request.sub_g_runtime_angle_raw / 4;
        return CommonMoverTargetCorrectionPlan {
            path: CommonMoverTargetCorrectionPath::Far,
            sub_g_descriptor_dword_1c_write: Some(TARGET_CORRECTION_SUB_G_DESCRIPTOR_DWORD_1C_RAW),
            sub_g_runtime_angle_raw: request.sub_g_runtime_angle_raw.wrapping_sub(quarter),
            output_override_raw: None,
        };
    }

    let normalized = normalize_retail_q31(displacement);
    let target_half = normalized[1] / 2;
    let entity_half = entity.attitude_raw_28 / 2;
    let output_raw = target_half.wrapping_sub(entity_half) >> 17;
    CommonMoverTargetCorrectionPlan {
        path: CommonMoverTargetCorrectionPath::Near,
        sub_g_descriptor_dword_1c_write: Some(TARGET_CORRECTION_SUB_G_DESCRIPTOR_DWORD_1C_RAW),
        sub_g_runtime_angle_raw: request.sub_g_runtime_angle_raw,
        output_override_raw: Some(output_raw),
    }
}

/// Apply the exact first caller policy around `FUN_00401BB0`.
///
/// The output initializer is zero. Retail truncates the resulting correction
/// toward zero when halving it, applies only the low 16 bits to Sub-G, then
/// performs signed 16-bit clamps.
pub fn plan_primary_common_mover_target_correction(
    request: CommonMoverTargetCorrectionRequest,
) -> CommonMoverPrimaryTargetCorrectionPlan {
    let correction = plan_common_mover_target_correction(request);
    let output_raw = correction.output_override_raw.unwrap_or(0);
    let half_low = (output_raw / 2) as i16;
    let sub_g_runtime_angle_raw = correction
        .sub_g_runtime_angle_raw
        .wrapping_sub(half_low)
        .clamp(
            -TARGET_CORRECTION_SUB_G_ANGLE_LIMIT_RAW,
            TARGET_CORRECTION_SUB_G_ANGLE_LIMIT_RAW,
        );
    CommonMoverPrimaryTargetCorrectionPlan {
        correction,
        output_raw,
        sub_g_runtime_angle_raw,
    }
}

/// Apply the exact second caller policy around `FUN_00401BB0`.
///
/// `elapsed_micros / 500` and the absolute correction are compared as
/// unsigned values. Equality does not restore the saved Sub-O link.
pub fn plan_secondary_common_mover_target_correction(
    request: CommonMoverTargetCorrectionRequest,
    saved_sub_o_link_raw: u32,
    elapsed_micros: u32,
    sub_g_state_byte_3c: u8,
) -> CommonMoverSecondaryTargetCorrectionPlan {
    let correction = plan_common_mover_target_correction(request);
    let output_raw = correction
        .output_override_raw
        .unwrap_or(TARGET_CORRECTION_SECONDARY_SENTINEL_RAW);
    let restored_saved_sub_o_link =
        unsigned_abs_i32(output_raw) < elapsed_micros / 500 && sub_g_state_byte_3c == 0;
    CommonMoverSecondaryTargetCorrectionPlan {
        correction,
        output_raw,
        sub_o_link_raw: if restored_saved_sub_o_link {
            saved_sub_o_link_raw
        } else {
            0
        },
        restored_saved_sub_o_link,
    }
}

fn inactive_plan(
    path: CommonMoverTargetCorrectionPath,
    sub_g_runtime_angle_raw: i16,
) -> CommonMoverTargetCorrectionPlan {
    CommonMoverTargetCorrectionPlan {
        path,
        sub_g_descriptor_dword_1c_write: None,
        sub_g_runtime_angle_raw,
        output_override_raw: None,
    }
}

fn retail_displacement(entity: [i16; 3], target: [i16; 3]) -> [i32; 3] {
    [
        i32::from(target[0] as u16).wrapping_sub(i32::from(entity[0] as u16)),
        i32::from(target[1]).wrapping_sub(i32::from(entity[1])),
        i32::from(target[2] as u16).wrapping_sub(i32::from(entity[2] as u16)),
    ]
}

fn signed_low_word_abs(value: i32) -> i32 {
    i32::from(value as i16).wrapping_abs()
}

/// Exact in-place normalization performed by `FUN_00457960`.
pub(crate) fn normalize_retail_q31(mut vector: [i32; 3]) -> [i32; 3] {
    let mut magnitude_or = vector.into_iter().fold(0i32, |combined, component| {
        combined | component.wrapping_abs()
    });
    while magnitude_or > 0x6882 {
        magnitude_or >>= 1;
        for component in &mut vector {
            *component >>= 1;
        }
    }

    let length_squared = vector.into_iter().fold(0u32, |sum, component| {
        sum.wrapping_add(component.wrapping_mul(component) as u32)
    });
    let length = (integer_sqrt(length_squared) as i32).wrapping_add(1);
    vector.map(|component| normalized_component_q31(component, length))
}

fn integer_sqrt(mut value: u32) -> u32 {
    let mut root = 0u32;
    let mut bit = 1u32 << 30;
    while bit > value {
        bit >>= 2;
    }
    while bit != 0 {
        if value >= root.wrapping_add(bit) {
            value = value.wrapping_sub(root.wrapping_add(bit));
            root = (root >> 1).wrapping_add(bit);
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

fn normalized_component_q31(component: i32, length: i32) -> i32 {
    if component.wrapping_abs() < length.wrapping_abs() {
        ((i64::from(component) << 31) / i64::from(length)) as i32
    } else {
        (component ^ length) | i32::MAX
    }
}

fn unsigned_abs_i32(value: i32) -> u32 {
    let sign = value >> 31;
    (value ^ sign).wrapping_sub(sign) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        position_raw: [i16; 3],
        attitude_raw_28: i32,
        target_position_raw: [i16; 3],
        sub_g_runtime_angle_raw: i16,
    ) -> CommonMoverTargetCorrectionRequest {
        CommonMoverTargetCorrectionRequest {
            entity: Some(CommonMoverTargetCorrectionEntity {
                position_raw,
                attitude_raw_28,
            }),
            target_context_present: true,
            target_position_raw,
            sub_g_runtime_angle_raw,
        }
    }

    #[test]
    fn inactive_paths_return_before_descriptor_and_output_writes() {
        let mut missing_entity = request([0; 3], 0, [10, 20, 30], 5_000);
        missing_entity.entity = None;
        let first = plan_primary_common_mover_target_correction(missing_entity);
        assert_eq!(
            first.correction.path,
            CommonMoverTargetCorrectionPath::MissingEntity
        );
        assert_eq!(first.correction.sub_g_descriptor_dword_1c_write, None);
        assert_eq!(first.output_raw, 0);
        // The primitive is inactive, but the first caller's clamp still runs.
        assert_eq!(first.sub_g_runtime_angle_raw, 4_000);

        let mut missing_context = request([0; 3], 0, [10, 20, 30], -12);
        missing_context.target_context_present = false;
        let core = plan_common_mover_target_correction(missing_context);
        assert_eq!(
            core.path,
            CommonMoverTargetCorrectionPath::MissingTargetContext
        );
        assert_eq!(core.sub_g_descriptor_dword_1c_write, None);
        assert_eq!(core.sub_g_runtime_angle_raw, -12);
        assert_eq!(core.output_override_raw, None);
    }

    #[test]
    fn near_path_uses_retail_normalization_with_length_plus_one() {
        let core = plan_common_mover_target_correction(request([0; 3], 0, [100, 100, 0], 321));
        let component = ((100i64 << 31) / 142) as i32;
        let expected = (component / 2) >> 17;
        assert_eq!(core.path, CommonMoverTargetCorrectionPath::Near);
        assert_eq!(
            core.sub_g_descriptor_dword_1c_write,
            Some(TARGET_CORRECTION_SUB_G_DESCRIPTOR_DWORD_1C_RAW)
        );
        assert_eq!(core.sub_g_runtime_angle_raw, 321);
        assert_eq!(core.output_override_raw, Some(expected));
    }

    #[test]
    fn xz_near_test_uses_low_words_but_normalization_keeps_full_deltas() {
        let displacement = retail_displacement([-1, 0, 0], [0, 0, 0]);
        assert_eq!(displacement, [-65_535, 0, 0]);
        assert_eq!(signed_low_word_abs(displacement[0]), 1);
        assert_eq!(
            normalize_retail_q31(displacement)[0],
            ((-16_384i64 << 31) / 16_385) as i32
        );

        let core = plan_common_mover_target_correction(request([-1, 0, 0], 0, [0, 0, 0], 0));
        assert_eq!(core.path, CommonMoverTargetCorrectionPath::Near);
    }

    #[test]
    fn exact_radius_is_far_and_negative_damping_truncates_toward_zero() {
        let core = plan_common_mover_target_correction(request([0; 3], 0, [0x0f00, 0, 0], -7));
        assert_eq!(core.path, CommonMoverTargetCorrectionPath::Far);
        assert_eq!(core.sub_g_runtime_angle_raw, -6);
        assert_eq!(core.output_override_raw, None);
    }

    #[test]
    fn primary_policy_applies_half_correction_then_signed_clamp() {
        let near = request([0; 3], 0, [0, 100, 0], 0);
        let planned = plan_primary_common_mover_target_correction(near);
        let normalized_y = ((100i64 << 31) / 101) as i32;
        let output = (normalized_y / 2) >> 17;
        assert_eq!(planned.output_raw, output);
        assert_eq!(planned.sub_g_runtime_angle_raw, -4_000);

        let clamped =
            plan_primary_common_mover_target_correction(request([0; 3], 0, [0; 3], -5_000));
        assert_eq!(clamped.sub_g_runtime_angle_raw, -4_000);
    }

    #[test]
    fn secondary_policy_preserves_sentinel_and_uses_strict_unsigned_threshold() {
        let far = request([0; 3], 0, [0x0f00, 0, 0], 100);
        let equal = plan_secondary_common_mover_target_correction(far, 0x1234_5678, 500_000, 0);
        assert_eq!(equal.output_raw, TARGET_CORRECTION_SECONDARY_SENTINEL_RAW);
        assert!(!equal.restored_saved_sub_o_link);
        assert_eq!(equal.sub_o_link_raw, 0);

        let greater = plan_secondary_common_mover_target_correction(far, 0x1234_5678, 500_500, 0);
        assert!(greater.restored_saved_sub_o_link);
        assert_eq!(greater.sub_o_link_raw, 0x1234_5678);

        let blocked_by_sub_g =
            plan_secondary_common_mover_target_correction(far, 0x1234_5678, 500_500, 1);
        assert!(!blocked_by_sub_g.restored_saved_sub_o_link);
        assert_eq!(blocked_by_sub_g.sub_o_link_raw, 0);
    }

    #[test]
    fn both_retail_calls_apply_far_damping_independently() {
        let first =
            plan_primary_common_mover_target_correction(request([0; 3], 0, [0x0f00, 0, 0], 100));
        assert_eq!(first.sub_g_runtime_angle_raw, 75);

        let second_request = request([0; 3], 0, [0x0f00, 0, 0], first.sub_g_runtime_angle_raw);
        let second =
            plan_secondary_common_mover_target_correction(second_request, 0xdead_beef, 0, 0);
        assert_eq!(second.correction.sub_g_runtime_angle_raw, 57);
    }

    #[test]
    fn unsigned_abs_matches_x86_minimum_value_behavior() {
        assert_eq!(unsigned_abs_i32(-7), 7);
        assert_eq!(unsigned_abs_i32(i32::MIN), 0x8000_0000);
    }
}
