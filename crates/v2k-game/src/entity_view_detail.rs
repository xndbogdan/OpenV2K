//! Retail view-detail classification recovered from `FUN_00411400`.
//!
//! The classifier supplies both the presentation-phase detail writer and the
//! main-model scan window. Its `0x02000000` result selects the detailed callback
//! on the next common scheduler visit, while `0x04000000` additionally admits
//! the full-detail presentation callback in the current traversal. Geometry
//! alone does not reproduce the separate `0x800` callback-admission gate.

use crate::entity_collision_state::{RetailRuntimeValue, RetailStateWord};

/// State bit which admits `FUN_00411400` classification.
pub const VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT: u32 = 0x0000_0800;
/// Broader-detail state bit consumed by `FUN_00412DA0`.
pub const BROADER_DETAIL_STATE_BIT: u32 = 0x0200_0000;
/// Inner/full-detail state bit consumed by the presentation callback.
pub const FULL_DETAIL_STATE_BIT: u32 = 0x0400_0000;
/// Complete state subset replaced by one admitted classification.
pub const VIEW_DETAIL_STATE_MASK: u32 = BROADER_DETAIL_STATE_BIT | FULL_DETAIL_STATE_BIT;

const RETAIL_FIXED_SCALE: f32 = 256.0;
const RETAIL_Q31_SCALE: f64 = 2_147_483_648.0;
const RETAIL_MAX_SCAN_COLUMNS: u32 = 52;
const RETAIL_MAX_SCAN_ROWS: u32 = 30;
const BROADER_DETAIL_MARGIN_CELLS: i32 = 3;
const BROADER_DETAIL_REAR_SLACK_RAW: i32 = 0x300;

/// The three persistent tiers written by `FUN_00411400`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetailViewDetail {
    Coarse,
    Broader,
    Full,
}

impl RetailViewDetail {
    pub const fn state_bits(self) -> u32 {
        match self {
            Self::Coarse => 0,
            Self::Broader => BROADER_DETAIL_STATE_BIT,
            Self::Full => VIEW_DETAIL_STATE_MASK,
        }
    }

    pub const fn uses_detailed_update(self) -> bool {
        !matches!(self, Self::Coarse)
    }
}

/// Exact camera and scan-window inputs retained by the classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetailViewDetailContext {
    /// X/Z are consumed as wrapping signed words; Y remains the full dword
    /// stored in the retail presentation context.
    camera_position_raw: [i32; 3],
    camera_true_up_z_q31: i32,
    tight_half_columns: i32,
    tight_rows: i32,
}

impl RetailViewDetailContext {
    /// Quantize the port camera into retail's signed-8.8/Q31 context.
    pub fn from_world(
        camera_position: [f32; 3],
        camera_true_up_z: f32,
        scan_dimensions: (u32, u32),
    ) -> Self {
        let camera_position_raw =
            camera_position.map(|coordinate| (coordinate * RETAIL_FIXED_SCALE).round() as i32);
        let camera_true_up_z_q31 = if camera_true_up_z >= 1.0 {
            i32::MAX
        } else if camera_true_up_z <= -1.0 {
            i32::MIN
        } else {
            (f64::from(camera_true_up_z) * RETAIL_Q31_SCALE).round() as i32
        };
        Self::from_raw(camera_position_raw, camera_true_up_z_q31, scan_dimensions)
    }

    /// Build an exact raw context. X/Z are narrowed at their retail word reads;
    /// Y deliberately remains a signed dword.
    pub const fn from_raw(
        camera_position_raw: [i32; 3],
        camera_true_up_z_q31: i32,
        scan_dimensions: (u32, u32),
    ) -> Self {
        let scan_columns = if scan_dimensions.0 > RETAIL_MAX_SCAN_COLUMNS {
            RETAIL_MAX_SCAN_COLUMNS
        } else {
            scan_dimensions.0
        };
        let scan_rows = if scan_dimensions.1 > RETAIL_MAX_SCAN_ROWS {
            RETAIL_MAX_SCAN_ROWS
        } else {
            scan_dimensions.1
        };
        Self {
            camera_position_raw,
            camera_true_up_z_q31,
            tight_half_columns: (scan_columns / 2) as i32,
            tight_rows: scan_rows as i32,
        }
    }

    /// Classify one entity using the exact tight-first, expanded-second order.
    pub fn classify(self, entity_position_raw: [i16; 3]) -> RetailViewDetail {
        let camera_x_raw = self.camera_position_raw[0] as i16;
        let camera_z_raw = self.camera_position_raw[2] as i16;
        let dx = i32::from(entity_position_raw[0].wrapping_sub(camera_x_raw));
        let dz = i32::from(entity_position_raw[2].wrapping_sub(camera_z_raw));
        let dy = i32::from(entity_position_raw[1]).wrapping_sub(self.camera_position_raw[1]);
        let true_up_z_q16 = self.camera_true_up_z_q31 >> 15;
        let rear_plane_raw = dy.wrapping_mul(true_up_z_q16) >> 16;

        let tight_x_limit_raw = self.tight_half_columns << 8;
        let tight_z_limit_raw = self.tight_rows << 8;
        if dx >= -tight_x_limit_raw
            && dx <= tight_x_limit_raw
            && dz <= tight_z_limit_raw
            && (dz >= 0 || dz >= rear_plane_raw)
        {
            return RetailViewDetail::Full;
        }

        let broad_x_limit_raw = (self.tight_half_columns + BROADER_DETAIL_MARGIN_CELLS) << 8;
        let broad_z_limit_raw = (self.tight_rows + BROADER_DETAIL_MARGIN_CELLS) << 8;
        if dx >= -broad_x_limit_raw
            && dx <= broad_x_limit_raw
            && dz <= broad_z_limit_raw
            && (dz >= 0 || dz.wrapping_add(BROADER_DETAIL_REAR_SLACK_RAW) >= rear_plane_raw)
        {
            RetailViewDetail::Broader
        } else {
            RetailViewDetail::Coarse
        }
    }

    /// Apply the masked retail state write when the classification gate is
    /// known set. A known-clear or unresolved gate leaves all state untouched.
    pub fn publish(
        self,
        entity_position_raw: [i16; 3],
        state: &mut RetailStateWord,
    ) -> RetailRuntimeValue<Option<RetailViewDetail>> {
        match state.masked(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT) {
            RetailRuntimeValue::Known(0) => RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(_) => {
                let detail = self.classify(entity_position_raw);
                state.overwrite(VIEW_DETAIL_STATE_MASK, detail.state_bits());
                RetailRuntimeValue::Known(Some(detail))
            }
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(camera_position_raw: [i32; 3], up_z_q31: i32) -> RetailViewDetailContext {
        RetailViewDetailContext::from_raw(camera_position_raw, up_z_q31, (52, 30))
    }

    #[test]
    fn tight_and_expanded_boundaries_publish_all_three_tiers() {
        let eye = [32_000_i32, 1_000, 32_000];
        let context = context(eye, 0);
        let eye_words = [eye[0] as i16, eye[1] as i16, eye[2] as i16];
        let tight_x = (52 / 2) * 256;
        let broad_x = (52 / 2 + 3) * 256;
        let tight_z = 30 * 256;
        let broad_z = (30 + 3) * 256;

        assert_eq!(
            context.classify([
                eye_words[0].wrapping_add(tight_x as i16),
                eye_words[1],
                eye_words[2],
            ]),
            RetailViewDetail::Full
        );
        assert_eq!(
            context.classify([
                eye_words[0].wrapping_add(tight_x as i16 + 1),
                eye_words[1],
                eye_words[2],
            ]),
            RetailViewDetail::Broader
        );
        assert_eq!(
            context.classify([
                eye_words[0].wrapping_add(broad_x as i16 + 1),
                eye_words[1],
                eye_words[2],
            ]),
            RetailViewDetail::Coarse
        );
        assert_eq!(
            context.classify([
                eye_words[0],
                eye_words[1],
                eye_words[2].wrapping_add(tight_z as i16 + 1),
            ]),
            RetailViewDetail::Broader
        );
        assert_eq!(
            context.classify([
                eye_words[0],
                eye_words[1],
                eye_words[2].wrapping_add(broad_z as i16 + 1),
            ]),
            RetailViewDetail::Coarse
        );

        assert_eq!(
            context.classify([eye_words[0], eye_words[1], eye_words[2].wrapping_sub(0x300),]),
            RetailViewDetail::Broader,
            "the expanded rear slack is inclusive"
        );
        assert_eq!(
            context.classify([eye_words[0], eye_words[1], eye_words[2].wrapping_sub(0x301),]),
            RetailViewDetail::Coarse
        );
    }

    #[test]
    fn retail_has_no_negative_z_box_cutoff() {
        let context = context([0, 0, 0], i32::MIN);
        assert_eq!(
            context.classify([0, i16::MAX, -20_000]),
            RetailViewDetail::Full,
            "the rear plane, not -rows, is the authored lower boundary"
        );
    }

    #[test]
    fn camera_y_is_a_dword_and_plane_multiply_wraps_like_x86() {
        assert_eq!(
            context([0, 40_000, 0], 0x4000_0000).classify([0, 0, -1_000]),
            RetailViewDetail::Full,
            "camera Y must not be narrowed to the wrapping X/Z word domain"
        );
        assert_eq!(
            context([0, -32_768, 0], i32::MAX).classify([0, i16::MAX, -1]),
            RetailViewDetail::Full,
            "65535 * 65535 wraps before retail's arithmetic shift"
        );
    }

    #[test]
    fn gate_controls_one_masked_atomic_state_publication() {
        let context = context([20_000, 0, 0], 0);
        let unrelated = 0x8000_0040;
        let mut admitted = RetailStateWord::exact(
            unrelated | VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT | VIEW_DETAIL_STATE_MASK,
        );
        assert_eq!(
            context.publish([0, 0, 0], &mut admitted),
            RetailRuntimeValue::Known(Some(RetailViewDetail::Coarse))
        );
        assert_eq!(
            admitted.masked(unrelated | VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT),
            RetailRuntimeValue::Known(unrelated | VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT)
        );
        assert_eq!(
            admitted.masked(VIEW_DETAIL_STATE_MASK),
            RetailRuntimeValue::Known(0)
        );

        let mut disabled = RetailStateWord::exact(unrelated | VIEW_DETAIL_STATE_MASK);
        let disabled_before = disabled;
        assert_eq!(
            context.publish([0, 0, 0], &mut disabled),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(disabled, disabled_before);

        let mut unresolved = RetailStateWord::from_known_bits(
            unrelated | VIEW_DETAIL_STATE_MASK,
            !VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT,
        );
        let unresolved_before = unresolved;
        assert_eq!(
            context.publish([0, 0, 0], &mut unresolved),
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(unresolved, unresolved_before);
    }
}
