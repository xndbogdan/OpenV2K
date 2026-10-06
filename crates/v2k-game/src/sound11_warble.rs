//! Detached core for constructor-attached sound 11's playback-rate warble.
//!
//! Retail `FUN_0044CD90` and demo `FUN_0044C590` construct the same 0x24-byte
//! state. Their update helpers (`FUN_0044CCA0` / `FUN_0044C4A0`) also match:
//! elapsed time advances only while the logical mixer finds the emitter
//! audible, interpolation consumes no random words, and a segment rollover
//! consumes exactly two words from the process-global retail RNG.
//!
//! [`crate::entity_positional_audio`] now advances this core for types 15, 44,
//! 87, and 108 only after a `FUN_0044C920` snapshot mixes as audible, and
//! feeds it `WorldFx::next_shared_retail_random_u16`.

const UPPER_RATE_Q16: i32 = 0x1_2000;
const LOWER_RATE_Q16: i32 = 0xE000;
const MAX_STEP_Q16: i32 = 0x2000;
const DURATION_SPAN_US: i32 = 40_000;
const DURATION_BASE_US: i32 = 30_000;
const INITIAL_RATE_Q16: i32 = 0x1_0000;

/// Exact dynamic and configuration state allocated for a sound-11 warbler.
///
/// The field order mirrors the original nine-dword allocation. Configuration
/// remains in each row because that is part of the recovered lifetime shape,
/// even though sound 11 currently has one fixed parameter set.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sound11WarbleState {
    upper_rate_q16: i32,
    lower_rate_q16: i32,
    max_step_q16: i32,
    duration_span_us: i32,
    duration_base_us: i32,
    segment_start_rate_q16: i32,
    segment_delta_rate_q16: i32,
    segment_duration_us: i32,
    segment_elapsed_us: i32,
}

impl Default for Sound11WarbleState {
    fn default() -> Self {
        Self::new()
    }
}

impl Sound11WarbleState {
    /// Construct retail sound 11's exact initial state without consuming RNG.
    pub const fn new() -> Self {
        Self {
            upper_rate_q16: UPPER_RATE_Q16,
            lower_rate_q16: LOWER_RATE_Q16,
            max_step_q16: MAX_STEP_Q16,
            duration_span_us: DURATION_SPAN_US,
            duration_base_us: DURATION_BASE_US,
            segment_start_rate_q16: INITIAL_RATE_Q16,
            segment_delta_rate_q16: 0,
            segment_duration_us: 0,
            segment_elapsed_us: 0,
        }
    }

    /// Advance one audible mixer visit and return the Q16 playback rate.
    ///
    /// The original logical mixer does not call the update helper while the
    /// emitter is outside its positional radius. Consequently callers must not
    /// call this method for an inaudible row: both elapsed state and RNG cadence
    /// freeze during physical culling. `elapsed_us` is added with x86 wrapping
    /// semantics. Overshoot is discarded and at most one rollover occurs per
    /// call, matching the helper rather than a catch-up loop.
    pub fn advance_audible(
        &mut self,
        elapsed_us: u32,
        next_shared_retail_random_u16: &mut impl FnMut() -> u16,
    ) -> i32 {
        self.segment_elapsed_us = self.segment_elapsed_us.wrapping_add(elapsed_us as i32);

        if self.segment_elapsed_us < self.segment_duration_us {
            let progress_q31 =
                retail_signed_ratio_q31(self.segment_elapsed_us, self.segment_duration_us);
            return self
                .segment_start_rate_q16
                .wrapping_add(q31_mul(progress_q31, self.segment_delta_rate_q16));
        }

        let previous_start = self.segment_start_rate_q16;
        let previous_delta = self.segment_delta_rate_q16;
        let current_rate = previous_start.wrapping_add(previous_delta);
        self.segment_start_rate_q16 = current_rate;

        let direction_and_step_word = next_shared_retail_random_u16();
        let step =
            (i32::from(direction_and_step_word) >> 4).wrapping_mul(self.max_step_q16 >> 4) >> 8;
        let target_rate = if direction_and_step_word & 1 == 0 {
            reflect_downward(current_rate, step, self.lower_rate_q16, self.upper_rate_q16)
        } else {
            reflect_upward(current_rate, step, self.lower_rate_q16, self.upper_rate_q16)
        };

        self.segment_elapsed_us = 0;
        self.segment_delta_rate_q16 = target_rate.wrapping_sub(current_rate);

        let duration_word = next_shared_retail_random_u16();
        let duration_jitter =
            (i32::from(duration_word) >> 4).wrapping_mul(self.duration_span_us >> 4) >> 8;
        self.segment_duration_us = duration_jitter.wrapping_add(self.duration_base_us);

        previous_start.wrapping_add(previous_delta)
    }
}

fn reflect_downward(current: i32, step: i32, lower: i32, upper: i32) -> i32 {
    let candidate = current.wrapping_sub(step);
    if lower <= candidate {
        return candidate;
    }

    let reflected = candidate.wrapping_add(step.wrapping_mul(2));
    if reflected <= upper {
        reflected
    } else {
        upper
    }
}

fn reflect_upward(current: i32, step: i32, lower: i32, upper: i32) -> i32 {
    let candidate = current.wrapping_add(step);
    if candidate <= upper {
        return candidate;
    }

    let reflected = candidate.wrapping_sub(step.wrapping_mul(2));
    if lower <= reflected {
        reflected
    } else {
        lower
    }
}

/// `FUN_00457680` / demo `FUN_00456FE0`, including signed saturation.
fn retail_signed_ratio_q31(numerator: i32, denominator: i32) -> i32 {
    let sign = numerator ^ denominator;
    let mut numerator = if numerator < 0 {
        numerator.wrapping_neg()
    } else {
        numerator
    };
    let mut denominator = if denominator < 0 {
        denominator.wrapping_neg()
    } else {
        denominator
    };

    if denominator <= numerator {
        return if sign < 0 { -i32::MAX } else { i32::MAX };
    }

    let mut quotient_bit = 0x8000_0000_u32;
    let mut quotient = 0_u32;
    loop {
        denominator >>= 1;
        quotient_bit >>= 1;
        if denominator <= numerator {
            numerator = numerator.wrapping_sub(denominator);
            quotient |= quotient_bit;
        }
        if numerator == 0 || denominator == 0 {
            break;
        }
    }

    if sign < 0 {
        quotient = quotient.wrapping_neg();
    }
    quotient as i32
}

fn q31_mul(lhs: i32, rhs: i32) -> i32 {
    ((i64::from(lhs) * i64::from(rhs)) >> 31) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    fn scripted_rng(words: &[u16]) -> (impl FnMut() -> u16 + '_, Rc<Cell<usize>>) {
        let cursor = Rc::new(Cell::new(0));
        let rng_cursor = Rc::clone(&cursor);
        let rng = move || {
            let index = rng_cursor.get();
            rng_cursor.set(index + 1);
            words[index]
        };
        (rng, cursor)
    }

    #[test]
    fn constructor_matches_the_nine_dword_allocation_without_rng() {
        assert_eq!(std::mem::size_of::<Sound11WarbleState>(), 0x24);
        assert_eq!(
            Sound11WarbleState::new(),
            Sound11WarbleState {
                upper_rate_q16: 0x1_2000,
                lower_rate_q16: 0xE000,
                max_step_q16: 0x2000,
                duration_span_us: 40_000,
                duration_base_us: 30_000,
                segment_start_rate_q16: 0x1_0000,
                segment_delta_rate_q16: 0,
                segment_duration_us: 0,
                segment_elapsed_us: 0,
            }
        );
    }

    #[test]
    fn first_audible_visit_rolls_even_at_zero_dt_and_draws_exactly_twice() {
        let mut state = Sound11WarbleState::new();
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0xFFFF]);

        assert_eq!(state.advance_audible(0, &mut rng), 0x1_0000);
        assert_eq!(draws.get(), 2);
        assert_eq!(state.segment_start_rate_q16, 0x1_0000);
        assert_eq!(state.segment_delta_rate_q16, 0x1FFE);
        assert_eq!(state.segment_duration_us, 69_990);
        assert_eq!(state.segment_elapsed_us, 0);
    }

    #[test]
    fn interpolation_uses_q31_and_consumes_no_random_words() {
        let mut state = Sound11WarbleState::new();
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0xFFFF]);
        state.advance_audible(0, &mut rng);

        assert_eq!(state.advance_audible(34_995, &mut rng), 0x1_0FFF);
        assert_eq!(draws.get(), 2);
        assert_eq!(state.segment_elapsed_us, 34_995);
    }

    #[test]
    fn negative_delta_interpolation_keeps_the_helpers_arithmetic_shift() {
        let mut state = Sound11WarbleState {
            segment_start_rate_q16: UPPER_RATE_Q16,
            ..Sound11WarbleState::new()
        };
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0xFFFF]);
        state.advance_audible(0, &mut rng);
        assert_eq!(state.segment_delta_rate_q16, -0x1FFE);

        assert_eq!(state.advance_audible(1, &mut rng), UPPER_RATE_Q16 - 1);
        assert_eq!(draws.get(), 2);
    }

    #[test]
    fn exact_endpoint_rolls_and_returns_the_completed_target() {
        let mut state = Sound11WarbleState::new();
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0xFFFF, 0, 0]);
        state.advance_audible(0, &mut rng);

        assert_eq!(state.advance_audible(69_990, &mut rng), 0x1_1FFE);
        assert_eq!(draws.get(), 4);
        assert_eq!(state.segment_start_rate_q16, 0x1_1FFE);
        assert_eq!(state.segment_delta_rate_q16, 0);
        assert_eq!(state.segment_duration_us, 30_000);
        assert_eq!(state.segment_elapsed_us, 0);
    }

    #[test]
    fn overshoot_is_discarded_and_cannot_roll_more_than_once_per_visit() {
        let mut state = Sound11WarbleState::new();
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0xFFFF, 0, 0]);
        state.advance_audible(0, &mut rng);

        assert_eq!(state.advance_audible(125_000, &mut rng), 0x1_1FFE);
        assert_eq!(draws.get(), 4);
        assert_eq!(state.segment_elapsed_us, 0);
        assert_eq!(state.segment_duration_us, 30_000);
    }

    #[test]
    fn low_direction_bit_reflects_at_both_rate_bounds() {
        let mut lower = Sound11WarbleState {
            segment_start_rate_q16: LOWER_RATE_Q16,
            ..Sound11WarbleState::new()
        };
        let (mut lower_rng, lower_draws) = scripted_rng(&[0xFFFE, 0]);
        assert_eq!(lower.advance_audible(0, &mut lower_rng), LOWER_RATE_Q16);
        assert_eq!(lower.segment_delta_rate_q16, 0x1FFE);
        assert_eq!(lower_draws.get(), 2);

        let mut upper = Sound11WarbleState {
            segment_start_rate_q16: UPPER_RATE_Q16,
            ..Sound11WarbleState::new()
        };
        let (mut upper_rng, upper_draws) = scripted_rng(&[0xFFFF, 0]);
        assert_eq!(upper.advance_audible(0, &mut upper_rng), UPPER_RATE_Q16);
        assert_eq!(upper.segment_delta_rate_q16, -0x1FFE);
        assert_eq!(upper_draws.get(), 2);
    }

    #[test]
    fn random_endpoints_select_exact_step_and_duration_ranges() {
        let mut minimum = Sound11WarbleState::new();
        let (mut minimum_rng, _) = scripted_rng(&[0, 0]);
        minimum.advance_audible(0, &mut minimum_rng);
        assert_eq!(minimum.segment_delta_rate_q16, 0);
        assert_eq!(minimum.segment_duration_us, 30_000);

        let mut maximum = Sound11WarbleState::new();
        let (mut maximum_rng, _) = scripted_rng(&[0xFFFF, 0xFFFF]);
        maximum.advance_audible(0, &mut maximum_rng);
        assert_eq!(maximum.segment_delta_rate_q16, 0x1FFE);
        assert_eq!(maximum.segment_duration_us, 69_990);
    }

    #[test]
    fn multiple_warblers_consume_one_shared_stream_in_visit_order() {
        let mut first = Sound11WarbleState::new();
        let mut second = Sound11WarbleState::new();
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0, 0xFFFE, 0xFFFF]);

        first.advance_audible(0, &mut rng);
        second.advance_audible(0, &mut rng);

        assert_eq!(draws.get(), 4);
        assert_eq!(first.segment_delta_rate_q16, 0x1FFE);
        assert_eq!(first.segment_duration_us, 30_000);
        assert_eq!(second.segment_delta_rate_q16, -0x1FFE);
        assert_eq!(second.segment_duration_us, 69_990);
    }

    #[test]
    fn physical_cull_can_freeze_and_resume_the_same_logical_state() {
        let mut state = Sound11WarbleState::new();
        let (mut rng, draws) = scripted_rng(&[0xFFFF, 0xFFFF, 0, 0]);
        state.advance_audible(0, &mut rng);
        state.advance_audible(10_000, &mut rng);
        let culled_snapshot = state;

        // An inaudible owner deliberately makes no core call, regardless of
        // wall-clock time spent outside the physical mixer radius.
        assert_eq!(state, culled_snapshot);
        assert_eq!(draws.get(), 2);

        assert_eq!(state.advance_audible(24_995, &mut rng), 0x1_0FFF);
        assert_eq!(state.segment_elapsed_us, 34_995);
        assert_eq!(draws.get(), 2);
    }

    #[test]
    fn signed_ratio_helper_matches_saturation_and_fraction_boundaries() {
        assert_eq!(retail_signed_ratio_q31(0, 7), 0);
        assert_eq!(retail_signed_ratio_q31(1, 2), 0x4000_0000);
        // This is the helper's actual divisor-halving quantization, not ideal
        // `(numerator << 31) / denominator` division: 3 first shifts to 1.
        assert_eq!(retail_signed_ratio_q31(1, 3), 0x4000_0000);
        assert_eq!(retail_signed_ratio_q31(8_000, 30_000), 0x2224_0000);
        assert_eq!(retail_signed_ratio_q31(-1, 2), -0x4000_0000);
        assert_eq!(retail_signed_ratio_q31(2, 2), i32::MAX);
        assert_eq!(retail_signed_ratio_q31(-2, 2), -i32::MAX);
    }
}
