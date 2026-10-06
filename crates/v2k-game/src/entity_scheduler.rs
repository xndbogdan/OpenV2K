//! Exact common entity-scheduler transition recovered from `FUN_00412DA0`.
//!
//! This module covers the normal-world prefix shared by live entity types. The
//! earlier network/global exceptional branch and the type callbacks after this
//! prefix remain outside its scope. Unknown initializer-owned inputs therefore
//! remain explicit: an unresolved prerequisite consumes no RNG and changes no
//! retained state.

#[cfg(test)]
use crate::entity_collision_state::RetailStateWord;
use crate::entity_collision_state::{EntityCollisionRuntimeState, RetailRuntimeValue};

/// `FUN_00412DA0` skips both randomized scheduler waits while this bit is set.
pub const SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT: u32 = 0x0200_0000;
/// Random span used by the subject-scan gate at entity `+0x70`.
pub const SUBJECT_SCAN_RANDOM_SPAN_US: u32 = 250_000;
/// Random span and maximum callback delta used by entity `+0x6c`.
pub const CALLBACK_SCHEDULER_RANDOM_SPAN_US: u32 = 125_000;
/// Normal `FUN_00412DA0` callback-enable bit at entity `+0x08`.
pub const COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT: u32 = 0x0002_0000;

/// Whether retail returns at the secondary gate or continues to callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonSchedulerPrefixFlow {
    WaitingAtCallbackGate,
    Continue { callback_elapsed_us: u32 },
}

/// Fully planned retained-state publication for retail's common scheduler
/// prefix.
///
/// Production owners plan this transition on a clone so all callback-local
/// blockers can be closed before process-shared RNG is consumed. Committing a
/// waiting plan also performs retail's early `+0xB2 = 0` write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedCommonSchedulerPrefix {
    pub(crate) flow: CommonSchedulerPrefixFlow,
    pub(crate) recent_relation_elapsed_us_at_0x68: u32,
    pub(crate) callback_scheduler_accumulator_us_at_0x6c: u32,
    pub(crate) subject_scan_gate_at_0x70: u32,
}

/// Plan the exact common scheduler prefix without changing retained state.
///
/// An unresolved input consumes no RNG. Once all inputs are known, draw order
/// and branch locality are inherited directly from
/// [`EntityCollisionRuntimeState::advance_common_scheduler_prefix`].
pub(crate) fn plan_common_scheduler_prefix(
    collision: &EntityCollisionRuntimeState,
    elapsed_us: u32,
    next_random: &mut impl FnMut() -> u32,
) -> RetailRuntimeValue<PlannedCommonSchedulerPrefix> {
    let mut planned = collision.clone();
    let RetailRuntimeValue::Known(flow) =
        planned.advance_common_scheduler_prefix(elapsed_us, next_random)
    else {
        return RetailRuntimeValue::Unresolved;
    };
    let (
        RetailRuntimeValue::Known(recent_relation_elapsed_us_at_0x68),
        RetailRuntimeValue::Known(callback_scheduler_accumulator_us_at_0x6c),
        RetailRuntimeValue::Known(subject_scan_gate_at_0x70),
    ) = (
        planned.recent_relation_elapsed_us_at_0x68,
        planned.callback_scheduler_accumulator_us_at_0x6c,
        planned.subject_scan_gate_at_0x70,
    )
    else {
        return RetailRuntimeValue::Unresolved;
    };
    RetailRuntimeValue::Known(PlannedCommonSchedulerPrefix {
        flow,
        recent_relation_elapsed_us_at_0x68,
        callback_scheduler_accumulator_us_at_0x6c,
        subject_scan_gate_at_0x70,
    })
}

/// Publish one already-planned common scheduler prefix.
pub(crate) fn commit_common_scheduler_prefix(
    collision: &mut EntityCollisionRuntimeState,
    plan: PlannedCommonSchedulerPrefix,
) {
    collision.recent_relation_elapsed_us_at_0x68 =
        RetailRuntimeValue::Known(plan.recent_relation_elapsed_us_at_0x68);
    collision.callback_scheduler_accumulator_us_at_0x6c =
        RetailRuntimeValue::Known(plan.callback_scheduler_accumulator_us_at_0x6c);
    collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(plan.subject_scan_gate_at_0x70);
    if plan.flow == CommonSchedulerPrefixFlow::WaitingAtCallbackGate {
        collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    }
}

/// Compute the wrapping callback-time `+0xB0` mass write.
pub(crate) const fn common_scheduler_callback_mass(
    authored_mass_raw: u16,
    animation_offset_raw: RetailRuntimeValue<u16>,
) -> RetailRuntimeValue<u16> {
    match animation_offset_raw {
        RetailRuntimeValue::Known(animation_offset_raw) => {
            let mass_raw = authored_mass_raw.wrapping_add(animation_offset_raw);
            RetailRuntimeValue::Known(if mass_raw == 0 { 1 } else { mass_raw })
        }
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
    }
}

/// Publish the common scheduler's unconditional post-callback `+0xB2` clear.
pub(crate) fn commit_common_scheduler_post_callback(collision: &mut EntityCollisionRuntimeState) {
    collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KnownSchedulerState {
    state_flags: u32,
    recent_relation_elapsed_us: u32,
    callback_scheduler_accumulator_us: u32,
    subject_scan_gate_us: u32,
    scheduler_unit_delta_flag: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KnownSchedulerTransition {
    state: KnownSchedulerState,
    flow: CommonSchedulerPrefixFlow,
}

impl EntityCollisionRuntimeState {
    /// Advance the common, fully evidenced prefix of retail `FUN_00412DA0`.
    ///
    /// If any required live field is unresolved, this performs no mutation and
    /// consumes no RNG. A callback-gate wait also reproduces retail's exact
    /// `+0xb2 = 0` write even when that field's prior value is unresolved.
    pub fn advance_common_scheduler_prefix(
        &mut self,
        elapsed_us: u32,
        next_random: &mut impl FnMut() -> u32,
    ) -> RetailRuntimeValue<CommonSchedulerPrefixFlow> {
        let RetailRuntimeValue::Known(state_flags) = self
            .state_flags_at_0x08
            .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
        else {
            return RetailRuntimeValue::Unresolved;
        };
        let (
            RetailRuntimeValue::Known(recent_relation_elapsed_us),
            RetailRuntimeValue::Known(callback_scheduler_accumulator_us),
            RetailRuntimeValue::Known(subject_scan_gate_us),
            RetailRuntimeValue::Known(scheduler_unit_delta_flag),
        ) = (
            &self.recent_relation_elapsed_us_at_0x68,
            &self.callback_scheduler_accumulator_us_at_0x6c,
            &self.subject_scan_gate_at_0x70,
            &self.scheduler_unit_delta_flag_at_0xb6,
        )
        else {
            return RetailRuntimeValue::Unresolved;
        };

        let transition = advance_known_scheduler(
            KnownSchedulerState {
                state_flags,
                recent_relation_elapsed_us: *recent_relation_elapsed_us,
                callback_scheduler_accumulator_us: *callback_scheduler_accumulator_us,
                subject_scan_gate_us: *subject_scan_gate_us,
                scheduler_unit_delta_flag: *scheduler_unit_delta_flag,
            },
            elapsed_us,
            next_random,
        );

        self.recent_relation_elapsed_us_at_0x68 =
            RetailRuntimeValue::Known(transition.state.recent_relation_elapsed_us);
        self.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(transition.state.callback_scheduler_accumulator_us);
        self.subject_scan_gate_at_0x70 =
            RetailRuntimeValue::Known(transition.state.subject_scan_gate_us);
        if transition.flow == CommonSchedulerPrefixFlow::WaitingAtCallbackGate {
            self.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        }

        RetailRuntimeValue::Known(transition.flow)
    }
}

/// Reproduce retail's low-16-bit random scaling without floating point.
const fn scaled_random_us(random_sample: u32, span_us: u32) -> u32 {
    (((random_sample & 0xffff) as u64 * span_us as u64) >> 16) as u32
}

/// Advance fully known state while preserving retail's branch-specific RNG.
fn advance_known_scheduler(
    mut state: KnownSchedulerState,
    elapsed_us: u32,
    next_random: &mut impl FnMut() -> u32,
) -> KnownSchedulerTransition {
    let random_waits_disabled = state.state_flags & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0;

    let subject_scan_elapsed = state.subject_scan_gate_us.wrapping_add(elapsed_us);
    if subject_scan_elapsed < SUBJECT_SCAN_RANDOM_SPAN_US && !random_waits_disabled {
        let threshold = scaled_random_us(next_random(), SUBJECT_SCAN_RANDOM_SPAN_US);
        state.subject_scan_gate_us = if threshold > subject_scan_elapsed {
            subject_scan_elapsed
        } else {
            0
        };
    } else {
        state.subject_scan_gate_us = 0;
    }

    let callback_elapsed = state
        .callback_scheduler_accumulator_us
        .wrapping_add(elapsed_us);
    let mut continuing_elapsed_us;
    if callback_elapsed <= CALLBACK_SCHEDULER_RANDOM_SPAN_US {
        if !random_waits_disabled {
            let threshold = scaled_random_us(next_random(), CALLBACK_SCHEDULER_RANDOM_SPAN_US);
            if callback_elapsed < threshold {
                state.callback_scheduler_accumulator_us = callback_elapsed;
                return KnownSchedulerTransition {
                    state,
                    flow: CommonSchedulerPrefixFlow::WaitingAtCallbackGate,
                };
            }
        }
        state.callback_scheduler_accumulator_us = 0;
        continuing_elapsed_us = callback_elapsed;
    } else {
        state.callback_scheduler_accumulator_us =
            callback_elapsed - CALLBACK_SCHEDULER_RANDOM_SPAN_US;
        continuing_elapsed_us = CALLBACK_SCHEDULER_RANDOM_SPAN_US;
    }

    if state.scheduler_unit_delta_flag != 0 {
        continuing_elapsed_us = 1;
    }
    state.recent_relation_elapsed_us = state
        .recent_relation_elapsed_us
        .wrapping_add(continuing_elapsed_us);

    KnownSchedulerTransition {
        state,
        flow: CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us: continuing_elapsed_us,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known_state() -> KnownSchedulerState {
        KnownSchedulerState {
            state_flags: 0,
            recent_relation_elapsed_us: 0,
            callback_scheduler_accumulator_us: 125_001,
            subject_scan_gate_us: 250_000,
            scheduler_unit_delta_flag: 0,
        }
    }

    fn known_runtime_state() -> EntityCollisionRuntimeState {
        let mut state = EntityCollisionRuntimeState::unresolved_port_entity(0);
        state.state_flags_at_0x08 = RetailStateWord::exact(0);
        state.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        state.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        state.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        state.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        state.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
        state
    }

    #[test]
    fn random_scaling_uses_only_the_low_sixteen_bits() {
        assert_eq!(scaled_random_us(0, 250_000), 0);
        assert_eq!(scaled_random_us(0xffff, 250_000), 249_996);
        assert_eq!(scaled_random_us(0xffff, 125_000), 124_998);
        assert_eq!(scaled_random_us(0x1234_ffff, 125_000), 124_998);
        assert_eq!(scaled_random_us(0x8000, 250_000), 125_000);
    }

    #[test]
    fn subject_scan_gate_matches_threshold_and_timer_boundaries() {
        for (scan_gate, sample, expected) in [
            (249_995, 0xffff, 249_995),
            (125_000, 0x8000, 0),
            (249_999, 0xffff, 0),
        ] {
            let mut state = known_state();
            state.subject_scan_gate_us = scan_gate;
            let mut calls = 0;
            let transition = advance_known_scheduler(state, 0, &mut || {
                calls += 1;
                sample
            });
            assert_eq!(transition.state.subject_scan_gate_us, expected);
            assert_eq!(calls, 1);
        }

        let mut calls = 0;
        let transition = advance_known_scheduler(known_state(), 0, &mut || {
            calls += 1;
            0xffff
        });
        assert_eq!(transition.state.subject_scan_gate_us, 0);
        assert_eq!(calls, 0);
    }

    #[test]
    fn disabled_random_waits_and_wrapping_scan_addition_match_retail() {
        let mut disabled = known_state();
        disabled.state_flags = SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
        disabled.subject_scan_gate_us = 10;
        disabled.callback_scheduler_accumulator_us = 20;
        let mut calls = 0;
        let transition = advance_known_scheduler(disabled, 5, &mut || {
            calls += 1;
            0xffff
        });
        assert_eq!(transition.state.subject_scan_gate_us, 0);
        assert_eq!(transition.state.callback_scheduler_accumulator_us, 0);
        assert_eq!(calls, 0);

        let mut wrapping = known_state();
        wrapping.subject_scan_gate_us = u32::MAX;
        let mut calls = 0;
        let transition = advance_known_scheduler(wrapping, 2, &mut || {
            calls += 1;
            0xffff
        });
        assert_eq!(transition.state.subject_scan_gate_us, 1);
        assert_eq!(calls, 1);
    }

    #[test]
    fn callback_gate_matches_wait_equality_and_cap_boundaries() {
        for (accumulator, flow, remainder, age, calls) in [
            (
                124_997,
                CommonSchedulerPrefixFlow::WaitingAtCallbackGate,
                124_997,
                7,
                1,
            ),
            (
                124_998,
                CommonSchedulerPrefixFlow::Continue {
                    callback_elapsed_us: 124_998,
                },
                0,
                125_005,
                1,
            ),
            (
                125_000,
                CommonSchedulerPrefixFlow::Continue {
                    callback_elapsed_us: 125_000,
                },
                0,
                125_007,
                1,
            ),
            (
                125_001,
                CommonSchedulerPrefixFlow::Continue {
                    callback_elapsed_us: 125_000,
                },
                1,
                125_007,
                0,
            ),
        ] {
            let mut state = known_state();
            state.callback_scheduler_accumulator_us = accumulator;
            state.recent_relation_elapsed_us = 7;
            let mut actual_calls = 0;
            let transition = advance_known_scheduler(state, 0, &mut || {
                actual_calls += 1;
                0xffff
            });
            assert_eq!(transition.flow, flow);
            assert_eq!(
                transition.state.callback_scheduler_accumulator_us,
                remainder
            );
            assert_eq!(transition.state.recent_relation_elapsed_us, age);
            assert_eq!(actual_calls, calls);
        }
    }

    #[test]
    fn unit_delta_and_wrapping_relation_age_match_retail() {
        let mut state = known_state();
        state.state_flags = SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
        state.callback_scheduler_accumulator_us = 100;
        state.subject_scan_gate_us = 100;
        state.recent_relation_elapsed_us = u32::MAX;
        state.scheduler_unit_delta_flag = 1;
        let transition = advance_known_scheduler(state, 23, &mut || {
            panic!("disabled random waits must not consume RNG")
        });
        assert_eq!(
            transition.flow,
            CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us: 1
            }
        );
        assert_eq!(transition.state.recent_relation_elapsed_us, 0);
    }

    #[test]
    fn rng_draw_order_and_branch_counts_are_exact() {
        let mut state = known_state();
        state.subject_scan_gate_us = 1;
        state.callback_scheduler_accumulator_us = 1;
        let mut samples = [0xffff, 0].into_iter();
        let mut calls = 0;
        let transition = advance_known_scheduler(state, 0, &mut || {
            calls += 1;
            samples.next().unwrap()
        });
        assert_eq!(calls, 2);
        assert_eq!(transition.state.subject_scan_gate_us, 1);
        assert_eq!(
            transition.flow,
            CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us: 1
            }
        );
    }

    #[test]
    fn unresolved_inputs_neither_mutate_state_nor_consume_rng() {
        for unresolved_field in 0..5 {
            let mut state = known_runtime_state();
            match unresolved_field {
                0 => state
                    .state_flags_at_0x08
                    .invalidate(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT),
                1 => state.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Unresolved,
                2 => {
                    state.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Unresolved
                }
                3 => state.subject_scan_gate_at_0x70 = RetailRuntimeValue::Unresolved,
                4 => state.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Unresolved,
                _ => unreachable!(),
            }
            let before = state.clone();
            let mut calls = 0;
            let result = state.advance_common_scheduler_prefix(100, &mut || {
                calls += 1;
                0xffff
            });
            assert_eq!(result, RetailRuntimeValue::Unresolved);
            assert_eq!(state, before);
            assert_eq!(calls, 0);
        }
    }

    #[test]
    fn unknown_surface_bits_do_not_block_the_scheduler_domain() {
        let mut state = known_runtime_state();
        state.state_flags_at_0x08.invalidate(0x0060_0000);
        state.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
        state.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(1);
        let mut samples = [0xffff, 0].into_iter();
        let mut calls = 0;
        assert_eq!(
            state.advance_common_scheduler_prefix(0, &mut || {
                calls += 1;
                samples.next().unwrap()
            }),
            RetailRuntimeValue::Known(CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us: 1
            })
        );
        assert_eq!(calls, 2);
        assert_eq!(state.state_flags_at_0x08.known_mask(), !0x0060_0000);
    }

    #[test]
    fn wrapper_applies_only_the_branch_specific_b2_write() {
        let mut waiting = known_runtime_state();
        waiting.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
        waiting.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(124_997);
        waiting.animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            waiting.advance_common_scheduler_prefix(0, &mut || 0xffff),
            RetailRuntimeValue::Known(CommonSchedulerPrefixFlow::WaitingAtCallbackGate)
        );
        assert_eq!(
            waiting.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );

        let mut continuing = known_runtime_state();
        continuing.state_flags_at_0x08 =
            RetailStateWord::exact(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT);
        continuing.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(20);
        assert_eq!(
            continuing.advance_common_scheduler_prefix(5, &mut || unreachable!()),
            RetailRuntimeValue::Known(CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us: 25
            })
        );
        assert_eq!(
            continuing.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(77)
        );
    }
}
