//! Shared `FUN_00423030` wrapped-coordinate range policy.
//!
//! Retail uses this exact predicate both for class-7 target acquisition and
//! the rule-13 `"Job Nearby"` initializer evaluator. Keeping the recovered
//! arithmetic here prevents those two proven call sites from drifting.

use std::num::NonZeroI32;

/// A zero limit is retail's explicit unbounded mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrappedAxisRange {
    Unbounded,
    Strict(NonZeroI32),
}

impl WrappedAxisRange {
    pub const fn from_raw(raw: i32) -> Self {
        match NonZeroI32::new(raw) {
            Some(limit) => Self::Strict(limit),
            None => Self::Unbounded,
        }
    }

    pub const fn strict(raw: i32) -> Option<Self> {
        match NonZeroI32::new(raw) {
            Some(limit) => Some(Self::Strict(limit)),
            None => None,
        }
    }

    pub const fn raw(self) -> i32 {
        match self {
            Self::Unbounded => 0,
            Self::Strict(limit) => limit.get(),
        }
    }
}

/// Exact `FUN_00423030` strict cube test.
///
/// Each subtraction wraps as signed 16-bit 8.8 world position before being
/// sign-extended and absolutized. Equality with a nonzero limit is rejected.
pub fn within_wrapped_axis_range(
    range: WrappedAxisRange,
    source_raw: [i16; 3],
    target_raw: [i16; 3],
) -> bool {
    match range {
        WrappedAxisRange::Unbounded => true,
        WrappedAxisRange::Strict(limit) => source_raw
            .into_iter()
            .zip(target_raw)
            .all(|(source, target)| i32::from(source.wrapping_sub(target)).abs() < limit.get()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_unbounded_and_nonzero_limits_are_strict() {
        assert!(within_wrapped_axis_range(
            WrappedAxisRange::from_raw(0),
            [0, 0, 0],
            [i16::MIN, i16::MAX, 123],
        ));
        let range = WrappedAxisRange::strict(0x0F00).unwrap();
        assert!(within_wrapped_axis_range(
            range,
            [0, 0, 0],
            [0x0EFF, -0x0EFF, 0],
        ));
        assert!(!within_wrapped_axis_range(range, [0, 0, 0], [0x0F00, 0, 0],));
    }

    #[test]
    fn subtraction_wraps_before_the_absolute_value() {
        let range = WrappedAxisRange::strict(2).unwrap();
        assert!(within_wrapped_axis_range(
            range,
            [i16::MIN, 0, 0],
            [i16::MAX, 0, 0],
        ));
        assert!(!within_wrapped_axis_range(
            range,
            [0, 0, 0],
            [i16::MIN, 0, 0],
        ));
    }
}
