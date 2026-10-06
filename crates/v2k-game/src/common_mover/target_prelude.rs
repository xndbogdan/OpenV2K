//! Pure target/reversal prelude shared by retail's common mover.
//!
//! `FUN_00401430` performs this phase before Sub-D and the component
//! dispatcher. The phase validates and optionally extrapolates a tracked
//! target, reconciles the task direction with Sub-A, expires the reversal
//! timer, and publishes the final target to Sub-L. The no-Sub-I mismatch
//! branch also owns a deliberately asymmetric randomized retarget and an
//! optional immediate Sub-D reversal write.
//!
//! This module plans those writes without mutating entity/component owners.
//! Live adapters remain responsible for committing only the effects their
//! complete Section-12 topology supports.

use super::SubAPropulsionRuntime;
use crate::entity_collision_state::{RetailRuntimeValue, RetailStateWord, DYING_STATE_BIT};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::collision::SubAPropulsionDescriptor;

/// Reversal interval installed by `FUN_00401A20`.
pub const COMMON_MOVER_REVERSAL_TIMER_MS: i32 = 1_500;
/// Heading delta applied by the Sub-I mismatch path and passed to Sub-D's
/// immediate no-I reversal writer.
pub const COMMON_MOVER_REVERSAL_ANGLE_RAW: i32 = 0x2000;
/// Minimum randomized Sub-A target speed in the no-I mismatch path.
pub const COMMON_MOVER_MINIMUM_REVERSE_SPEED_RAW: i32 = 50;
/// Per-axis near/far boundary used by the no-I randomized retarget.
pub const COMMON_MOVER_REVERSE_NEAR_RADIUS_RAW: i32 = 0x600;

/// Tracked-entity fields consumed before the common mover dispatch.
/// `FUN_00401430` reads only the dying bit and whether +08 is zero; unrelated
/// surface-state bits may remain unknown without preventing target motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTrackedTargetSnapshot {
    pub state_flags: RetailStateWord,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
}

/// Sub-A's authored descriptor and current allocation state.
///
/// A present runtime with no descriptor preserves the raw executable's
/// defensive branch: direction propagation still occurs, but speed
/// randomization is skipped. Normal Section-12 records provide both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverPreludeSubA {
    pub descriptor: Option<SubAPropulsionDescriptor>,
    pub runtime: SubAPropulsionRuntime,
}

/// The two Sub-D inputs used by `FUN_004204C0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverPreludeSubD {
    pub steering_divisor_raw: i32,
    /// Runtime dword `+0x0C`, initialized from descriptor byte `+0x04`.
    pub couple_yaw_into_roll: bool,
}

/// Resolved Section-12 shape needed by the target/reversal prelude.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTargetPreludeTopology {
    pub sub_a: Option<CommonMoverPreludeSubA>,
    pub sub_d: Option<CommonMoverPreludeSubD>,
    pub sub_f: bool,
    pub sub_g: bool,
    pub sub_i: bool,
    pub sub_l: bool,
}

/// Inputs to one pure target/reversal prelude transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTargetPreludeRequest {
    pub target_state: WanderNearPrivateState,
    /// Lookup result for `target_state.tracked_entity_handle`.
    ///
    /// The value is ignored for retail's zero static-target sentinel. A
    /// missing resolved lookup is distinct from an unresolved lookup: retail
    /// continues with the retained target when the handle no longer resolves.
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub controlled_position_raw: [i16; 3],
    pub heading_raw: u16,
    pub roll_raw: u16,
    pub elapsed_micros: u32,
    pub topology: RetailRuntimeValue<CommonMoverTargetPreludeTopology>,
}

/// Retail's intentional zero return before any component dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverTargetPreludeZero {
    TrackedEntityDying { state_flags: RetailStateWord },
    TrackedEntityInactive,
}

/// Evidence boundaries that must be closed before the planner consumes RNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverTargetPreludeBlock {
    UnresolvedTrackedTarget { handle: u32 },
    UnresolvedTopology,
    InvalidSubDReversalDivisor { steering_divisor_raw: i32 },
}

/// Inputs consumed by retail descriptor effect `FUN_00401A20`.
///
/// The common-mover prefix calls this routine only after detecting a
/// direction mismatch. Component pair callback `FUN_00402DA0` calls the same
/// routine directly after its forward-half-space gate. Keeping the effect
/// separate prevents either caller from duplicating its deliberately
/// asymmetric RNG cadence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverDescriptorEffectRequest {
    pub target_state: WanderNearPrivateState,
    pub controlled_position_raw: [i16; 3],
    pub heading_raw: u16,
    pub roll_raw: u16,
    pub topology: CommonMoverTargetPreludeTopology,
}

/// Planned immediate no-I `FUN_004204C0` write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverSubDReversalWrite {
    /// Sign-extended runtime dword `+0x10`.
    pub step_raw: i32,
    pub heading_raw: u16,
    pub roll_raw: u16,
}

/// Complete detached output of one `FUN_00401A20` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverDescriptorEffectPlan {
    pub target_state: WanderNearPrivateState,
    pub heading_raw: u16,
    pub roll_raw: u16,
    pub sub_a_runtime: Option<SubAPropulsionRuntime>,
    pub sub_d_reversal_write: Option<CommonMoverSubDReversalWrite>,
    pub sub_f_reverse_write: Option<bool>,
    pub sub_g_reverse_write: Option<bool>,
    pub rng_draw_count: u8,
}

/// Complete successful prelude state and detached component writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverTargetPreludePlan {
    pub target_state: WanderNearPrivateState,
    pub heading_raw: u16,
    pub roll_raw: u16,
    pub sub_a_runtime: Option<SubAPropulsionRuntime>,
    pub sub_d_reversal_write: Option<CommonMoverSubDReversalWrite>,
    pub sub_f_reverse_write: Option<bool>,
    pub sub_g_reverse_write: Option<bool>,
    pub sub_l_target_write: Option<[i16; 3]>,
    pub tracked_target_extrapolated: bool,
    pub mismatch_applied: bool,
    pub timer_expired: bool,
    /// Number of direction propagations in retail order. This is two when a
    /// mismatch installs reverse and that new timer expires in the same call.
    pub direction_propagation_count: u8,
    pub rng_draw_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonMoverTargetPreludeOutcome {
    Continue(CommonMoverTargetPreludePlan),
    ReturnZero(CommonMoverTargetPreludeZero),
}

/// Plan retail descriptor effect `FUN_00401A20` without mutating component
/// allocations.
///
/// The Sub-I branch consumes no RNG: it adds `0x2000` to heading and
/// propagates the retained direction through A/F/G. Without Sub-I, the effect
/// applies the optional A speed quirk, optional D reversal, negates direction,
/// installs the 1500-ms timer, retargets X/Y/Z, and then propagates direction.
///
/// All fallible validation precedes the first RNG draw, so an error is RNG
/// atomic.
pub fn plan_common_mover_descriptor_effect(
    request: CommonMoverDescriptorEffectRequest,
    mut next_random: impl FnMut() -> u32,
) -> Result<CommonMoverDescriptorEffectPlan, CommonMoverTargetPreludeBlock> {
    let topology = request.topology;
    if !topology.sub_i {
        if let Some(sub_d) = topology.sub_d {
            let quarter = truncated_divisor_quarter(sub_d.steering_divisor_raw);
            if quarter == 0 {
                return Err(CommonMoverTargetPreludeBlock::InvalidSubDReversalDivisor {
                    steering_divisor_raw: sub_d.steering_divisor_raw,
                });
            }
        }
    }

    let mut target_state = request.target_state;
    let mut rng_draw_count = 0u8;
    let mut heading_raw = request.heading_raw;
    let mut roll_raw = request.roll_raw;
    let mut sub_a_runtime = topology.sub_a.map(|value| value.runtime);
    let mut sub_d_reversal_write = None;
    let mut sub_f_reverse_write = None;
    let mut sub_g_reverse_write = None;

    if topology.sub_i {
        heading_raw = heading_raw.wrapping_add(COMMON_MOVER_REVERSAL_ANGLE_RAW as u16);
    } else {
        if let Some(CommonMoverPreludeSubA {
            descriptor: Some(descriptor),
            runtime,
        }) = topology.sub_a
        {
            let first_word = draw(&mut next_random, &mut rng_draw_count);
            let first_candidate = i32::from(descriptor.target_speed_base_raw)
                .wrapping_sub(i32::from((first_word as u16) >> 7));
            let target_speed_raw = if first_candidate < COMMON_MOVER_MINIMUM_REVERSE_SPEED_RAW {
                COMMON_MOVER_MINIMUM_REVERSE_SPEED_RAW
            } else {
                let second_word = draw(&mut next_random, &mut rng_draw_count);
                i32::from(descriptor.target_speed_base_raw)
                    .wrapping_sub(i32::from((second_word as u16) >> 7))
            };
            sub_a_runtime = Some(SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(target_speed_raw),
                runtime.direction_multiplier(),
                runtime.drive_scale_percent(),
            ));
        }

        if let Some(sub_d) = topology.sub_d {
            let write =
                plan_sub_d_reversal(sub_d, heading_raw, roll_raw).expect("preflight checked");
            heading_raw = write.heading_raw;
            roll_raw = write.roll_raw;
            sub_d_reversal_write = Some(write);
        }

        target_state.reversal_timer_ms = COMMON_MOVER_REVERSAL_TIMER_MS;
        target_state.direction = target_state.direction.wrapping_neg();
        target_state.target_position_raw = reverse_retarget(
            target_state.target_position_raw,
            request.controlled_position_raw,
            &mut next_random,
            &mut rng_draw_count,
        );
    }

    propagate_direction(
        target_state.direction,
        topology,
        &mut sub_a_runtime,
        &mut sub_f_reverse_write,
        &mut sub_g_reverse_write,
    );

    Ok(CommonMoverDescriptorEffectPlan {
        target_state,
        heading_raw,
        roll_raw,
        sub_a_runtime,
        sub_d_reversal_write,
        sub_f_reverse_write,
        sub_g_reverse_write,
        rng_draw_count,
    })
}

/// Plan `FUN_00401430`'s complete target/reversal prelude.
///
/// All fallible evidence gates are evaluated before the first possible RNG
/// draw. Consequently an `Err` never consumes RNG, and callers can keep the
/// entity/task/component transaction atomic by committing only `Continue`.
pub fn plan_common_mover_target_prelude(
    request: CommonMoverTargetPreludeRequest,
    mut next_random: impl FnMut() -> u32,
) -> Result<CommonMoverTargetPreludeOutcome, CommonMoverTargetPreludeBlock> {
    let mut target_state = request.target_state;
    let mut tracked_target_extrapolated = false;

    let tracked_target = if target_state.tracked_entity_handle == 0 {
        None
    } else {
        match request.tracked_target {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                return Err(CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
                    handle: target_state.tracked_entity_handle,
                });
            }
        }
    };

    if let Some(tracked_target) = tracked_target {
        let unresolved_state = CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
            handle: target_state.tracked_entity_handle,
        };
        match tracked_target.state_flags.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(bits) if bits != 0 => {
                return Ok(CommonMoverTargetPreludeOutcome::ReturnZero(
                    CommonMoverTargetPreludeZero::TrackedEntityDying {
                        state_flags: tracked_target.state_flags,
                    },
                ));
            }
            RetailRuntimeValue::Known(_) => {}
            RetailRuntimeValue::Unresolved => return Err(unresolved_state),
        }
        // One known set bit proves the whole word nonzero. Proving zero
        // requires every bit; normalized unknown bits are never read as zero.
        let state = tracked_target.state_flags;
        if state.known_value_bits() == 0 {
            if state.known_mask() != u32::MAX {
                return Err(unresolved_state);
            }
            return Ok(CommonMoverTargetPreludeOutcome::ReturnZero(
                CommonMoverTargetPreludeZero::TrackedEntityInactive,
            ));
        }
        if target_state.direction == 1 && target_state.reversal_timer_ms == 0 {
            target_state.target_position_raw = extrapolate_tracked_target(
                tracked_target.position_raw,
                tracked_target.velocity_raw,
                request.elapsed_micros,
            );
            tracked_target_extrapolated = true;
        }
    }

    let topology = match request.topology {
        RetailRuntimeValue::Known(value) => value,
        RetailRuntimeValue::Unresolved => {
            return Err(CommonMoverTargetPreludeBlock::UnresolvedTopology);
        }
    };

    let mismatch_applied = topology
        .sub_a
        .is_some_and(|sub_a| target_state.direction != sub_a.runtime.direction_multiplier());
    let mut rng_draw_count = 0u8;
    let mut heading_raw = request.heading_raw;
    let mut roll_raw = request.roll_raw;
    let mut sub_a_runtime = topology.sub_a.map(|value| value.runtime);
    let mut sub_d_reversal_write = None;
    let mut sub_f_reverse_write = None;
    let mut sub_g_reverse_write = None;
    let mut direction_propagation_count = 0u8;

    if mismatch_applied {
        let effect = plan_common_mover_descriptor_effect(
            CommonMoverDescriptorEffectRequest {
                target_state,
                controlled_position_raw: request.controlled_position_raw,
                heading_raw,
                roll_raw,
                topology,
            },
            &mut next_random,
        )?;
        target_state = effect.target_state;
        heading_raw = effect.heading_raw;
        roll_raw = effect.roll_raw;
        sub_a_runtime = effect.sub_a_runtime;
        sub_d_reversal_write = effect.sub_d_reversal_write;
        sub_f_reverse_write = effect.sub_f_reverse_write;
        sub_g_reverse_write = effect.sub_g_reverse_write;
        rng_draw_count = rng_draw_count.wrapping_add(effect.rng_draw_count);
        direction_propagation_count = direction_propagation_count.wrapping_add(1);
    }

    let mut timer_expired = false;
    if target_state.reversal_timer_ms != 0 {
        target_state.reversal_timer_ms = target_state
            .reversal_timer_ms
            .wrapping_sub((request.elapsed_micros / 1_000) as i32);
        if target_state.reversal_timer_ms < 1 {
            timer_expired = true;
            target_state.reversal_timer_ms = 0;
            target_state.direction = 1;
            if let Some(CommonMoverPreludeSubA {
                descriptor: Some(descriptor),
                runtime,
            }) = topology.sub_a
            {
                let word = draw(&mut next_random, &mut rng_draw_count);
                let base = i32::from(descriptor.target_speed_base_raw);
                let random_scale = i32::from((word as u16) >> 8);
                let target_speed_raw = base.wrapping_add(random_scale.wrapping_mul(base) / 0x0a00);
                sub_a_runtime = Some(SubAPropulsionRuntime::from_retail_words(
                    RetailRuntimeValue::Known(target_speed_raw),
                    runtime.direction_multiplier(),
                    runtime.drive_scale_percent(),
                ));
            }
            propagate_direction(
                target_state.direction,
                topology,
                &mut sub_a_runtime,
                &mut sub_f_reverse_write,
                &mut sub_g_reverse_write,
            );
            direction_propagation_count = direction_propagation_count.wrapping_add(1);
        }
    }

    Ok(CommonMoverTargetPreludeOutcome::Continue(
        CommonMoverTargetPreludePlan {
            target_state,
            heading_raw,
            roll_raw,
            sub_a_runtime,
            sub_d_reversal_write,
            sub_f_reverse_write,
            sub_g_reverse_write,
            sub_l_target_write: topology.sub_l.then_some(target_state.target_position_raw),
            tracked_target_extrapolated,
            mismatch_applied,
            timer_expired,
            direction_propagation_count,
            rng_draw_count,
        },
    ))
}

fn draw(next_random: &mut impl FnMut() -> u32, count: &mut u8) -> u32 {
    let value = next_random();
    *count = count.wrapping_add(1);
    value
}

fn extrapolate_tracked_target(
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    elapsed_micros: u32,
) -> [i16; 3] {
    let elapsed_q12 = (elapsed_micros as i32).wrapping_shl(12);
    std::array::from_fn(|axis| {
        let product = i64::from(elapsed_q12) * i64::from(velocity_raw[axis]);
        position_raw[axis].wrapping_add((product >> 31) as i16)
    })
}

fn reverse_retarget(
    retained_target_raw: [i16; 3],
    controlled_position_raw: [i16; 3],
    next_random: &mut impl FnMut() -> u32,
    rng_draw_count: &mut u8,
) -> [i16; 3] {
    let x_word = draw(next_random, rng_draw_count);
    let z_word = draw(next_random, rng_draw_count);
    [
        reverse_retarget_axis(
            retained_target_raw[0],
            controlled_position_raw[0],
            x_word,
            true,
        ),
        controlled_position_raw[1],
        reverse_retarget_axis(
            retained_target_raw[2],
            controlled_position_raw[2],
            z_word,
            false,
        ),
    ]
}

fn reverse_retarget_axis(
    retained_target_raw: i16,
    controlled_position_raw: i16,
    random_word: u32,
    is_x: bool,
) -> i16 {
    if (i32::from(retained_target_raw) - i32::from(controlled_position_raw)).abs()
        < COMMON_MOVER_REVERSE_NEAR_RADIUS_RAW
    {
        retained_target_raw.wrapping_sub(((random_word >> 8) & 0xff) as i16)
    } else {
        let random_offset = ((random_word >> 6) & 0x03ff) as i16;
        let origin = controlled_position_raw.wrapping_add(random_offset);
        if is_x {
            origin.wrapping_sub(0x0200)
        } else {
            origin
        }
    }
}

fn propagate_direction(
    direction: i32,
    topology: CommonMoverTargetPreludeTopology,
    sub_a_runtime: &mut Option<SubAPropulsionRuntime>,
    sub_f_reverse_write: &mut Option<bool>,
    sub_g_reverse_write: &mut Option<bool>,
) {
    if let Some(runtime) = sub_a_runtime {
        runtime.set_direction_multiplier(direction);
    }
    if topology.sub_f {
        *sub_f_reverse_write = Some(direction == -1);
    }
    if topology.sub_g {
        *sub_g_reverse_write = Some(direction == -1);
    }
}

fn truncated_divisor_quarter(steering_divisor_raw: i32) -> i32 {
    steering_divisor_raw.wrapping_add((steering_divisor_raw >> 31) & 3) >> 2
}

fn plan_sub_d_reversal(
    sub_d: CommonMoverPreludeSubD,
    heading_raw: u16,
    roll_raw: u16,
) -> Option<CommonMoverSubDReversalWrite> {
    let quarter = truncated_divisor_quarter(sub_d.steering_divisor_raw);
    if quarter == 0 {
        return None;
    }
    let quotient = (125_000_i64 / i64::from(quarter)) as i32;
    let step_raw = (quotient.wrapping_mul(COMMON_MOVER_REVERSAL_ANGLE_RAW) >> 15) as i16;
    Some(CommonMoverSubDReversalWrite {
        step_raw: i32::from(step_raw),
        heading_raw: heading_raw.wrapping_sub(step_raw as u16),
        roll_raw: if sub_d.couple_yaw_into_roll {
            roll_raw.wrapping_sub(step_raw as u16)
        } else {
            roll_raw
        },
    })
}

/// Current post-Sub-D component words consumed by lines `331..344` of
/// `FUN_00401430`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverPostSubDWriteRequest {
    pub sub_d_result_raw: i16,
    pub target_position_raw: [i16; 3],
    pub sub_f_smoothed_raw: Option<i32>,
    pub sub_k_smoothed_raw: Option<i32>,
    pub sub_l: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverSubFWrite {
    pub smoothed_raw: i32,
    pub target_position_raw: [i16; 3],
}

/// Detached F/K/L fan-out immediately following a normal Sub-D call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverPostSubDWritePlan {
    pub sub_f: Option<CommonMoverSubFWrite>,
    pub sub_k_smoothed_raw: Option<i32>,
    pub sub_l_exact_raw: Option<i32>,
}

pub fn plan_common_mover_post_sub_d_writes(
    request: CommonMoverPostSubDWriteRequest,
) -> CommonMoverPostSubDWritePlan {
    let result = i32::from(request.sub_d_result_raw);
    CommonMoverPostSubDWritePlan {
        sub_f: request.sub_f_smoothed_raw.map(|old| CommonMoverSubFWrite {
            smoothed_raw: smooth_sub_d_result(old, result),
            target_position_raw: request.target_position_raw,
        }),
        sub_k_smoothed_raw: request
            .sub_k_smoothed_raw
            .map(|old| smooth_sub_d_result(old, result)),
        sub_l_exact_raw: request.sub_l.then_some(result),
    }
}

fn smooth_sub_d_result(old: i32, result: i32) -> i32 {
    old.wrapping_add(result.wrapping_sub(old) >> 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 1_000,
    };

    fn sub_a(
        descriptor: Option<SubAPropulsionDescriptor>,
        direction: i32,
    ) -> CommonMoverPreludeSubA {
        CommonMoverPreludeSubA {
            descriptor,
            runtime: SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(777),
                direction,
                73,
            ),
        }
    }

    fn topology(sub_i: bool) -> CommonMoverTargetPreludeTopology {
        CommonMoverTargetPreludeTopology {
            sub_a: Some(sub_a(Some(SUB_A), 1)),
            sub_d: Some(CommonMoverPreludeSubD {
                steering_divisor_raw: 20,
                couple_yaw_into_roll: false,
            }),
            sub_f: false,
            sub_g: false,
            sub_i,
            sub_l: false,
        }
    }

    fn request(
        topology: RetailRuntimeValue<CommonMoverTargetPreludeTopology>,
    ) -> CommonMoverTargetPreludeRequest {
        CommonMoverTargetPreludeRequest {
            target_state: WanderNearPrivateState {
                target_position_raw: [100, 200, 2_000],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            },
            tracked_target: RetailRuntimeValue::Unresolved,
            controlled_position_raw: [0, 50, 0],
            heading_raw: 10_000,
            roll_raw: 20_000,
            elapsed_micros: 20_000,
            topology,
        }
    }

    fn continued(outcome: CommonMoverTargetPreludeOutcome) -> CommonMoverTargetPreludePlan {
        let CommonMoverTargetPreludeOutcome::Continue(plan) = outcome else {
            panic!("expected continuing prelude");
        };
        plan
    }

    fn run_with_words(
        request: CommonMoverTargetPreludeRequest,
        words: &[u32],
    ) -> CommonMoverTargetPreludePlan {
        let mut words = words.iter().copied();
        let outcome =
            plan_common_mover_target_prelude(request, || words.next().expect("missing RNG word"))
                .unwrap();
        assert!(
            words.next().is_none(),
            "planner consumed fewer RNG words than expected"
        );
        continued(outcome)
    }

    #[test]
    fn tracked_target_validation_distinguishes_missing_inactive_and_dying() {
        let mut missing = request(RetailRuntimeValue::Known(topology(true)));
        missing.target_state.tracked_entity_handle = 0x04ac_0001;
        missing.tracked_target = RetailRuntimeValue::Known(None);
        let plan = run_with_words(missing, &[]);
        assert!(!plan.tracked_target_extrapolated);
        assert_eq!(plan.target_state.target_position_raw, [100, 200, 2_000]);

        let mut inactive = missing;
        inactive.topology = RetailRuntimeValue::Unresolved;
        inactive.tracked_target =
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw: [1, 2, 3],
                velocity_raw: [4, 5, 6],
            }));
        assert_eq!(
            plan_common_mover_target_prelude(inactive, || panic!("zero path drew RNG")).unwrap(),
            CommonMoverTargetPreludeOutcome::ReturnZero(
                CommonMoverTargetPreludeZero::TrackedEntityInactive
            ),
            "target rejection precedes the unresolved-topology gate"
        );

        inactive.tracked_target =
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(
                    DYING_STATE_BIT | 7,
                ),
                position_raw: [1, 2, 3],
                velocity_raw: [4, 5, 6],
            }));
        assert_eq!(
            plan_common_mover_target_prelude(inactive, || panic!("zero path drew RNG")).unwrap(),
            CommonMoverTargetPreludeOutcome::ReturnZero(
                CommonMoverTargetPreludeZero::TrackedEntityDying {
                    state_flags: RetailStateWord::exact(DYING_STATE_BIT | 7)
                }
            )
        );
    }

    #[test]
    fn tracked_target_reads_only_dying_and_evidence_of_a_nonzero_word() {
        let mut frame = request(RetailRuntimeValue::Known(topology(true)));
        frame.target_state.tracked_entity_handle = 123;
        // Actual fresh Main Base at the failed Go-To-Job visit: only the two
        // surface bits are unknown, while every 01430 state read is resolved.
        let main_base_state = RetailStateWord::from_known_bits(0x0e02_8805, 0xff9f_ffff);
        frame.tracked_target = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: main_base_state,
            position_raw: [100, 200, 300],
            velocity_raw: [0; 3],
        }));
        let plan = run_with_words(frame, &[]);
        assert!(plan.tracked_target_extrapolated);
        assert_eq!(plan.target_state.target_position_raw, [100, 200, 300]);

        let dying = RetailStateWord::from_known_bits(DYING_STATE_BIT, DYING_STATE_BIT);
        frame.tracked_target = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: dying,
            position_raw: [100, 200, 300],
            velocity_raw: [0; 3],
        }));
        frame.topology = RetailRuntimeValue::Unresolved;
        assert_eq!(
            plan_common_mover_target_prelude(frame, || panic!("dying target cannot draw")),
            Ok(CommonMoverTargetPreludeOutcome::ReturnZero(
                CommonMoverTargetPreludeZero::TrackedEntityDying { state_flags: dying }
            ))
        );

        for state in [
            RetailStateWord::unknown(),
            RetailStateWord::from_known_bits(1, 1), // nonzero, dying unknown
            RetailStateWord::from_known_bits(0, DYING_STATE_BIT), // dying clear, zero unknown
            RetailStateWord::from_known_bits(0, 0xff9f_ffff), // unknown surface bits can be nonzero
        ] {
            frame.tracked_target =
                RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: state,
                    position_raw: [100, 200, 300],
                    velocity_raw: [0; 3],
                }));
            assert_eq!(
                plan_common_mover_target_prelude(frame, || panic!("unresolved target cannot draw")),
                Err(CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget { handle: 123 })
            );
        }
    }

    #[test]
    fn tracked_target_extrapolation_preserves_signed_shift_and_word_wrap() {
        let mut frame = request(RetailRuntimeValue::Known(topology(true)));
        frame.target_state.tracked_entity_handle = 0x04ac_0001;
        frame.tracked_target = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: crate::entity_collision_state::RetailStateWord::exact(1),
            position_raw: [32_767, -32_760, 123],
            velocity_raw: [256, -256, 0],
        }));
        let plan = run_with_words(frame, &[]);
        assert!(plan.tracked_target_extrapolated);
        assert_eq!(
            plan.target_state.target_position_raw,
            [-32_760, 32_766, 123],
            "20ms produces +9 and arithmetic-shift -10 before i16 wrapping"
        );
    }

    #[test]
    fn tracked_target_extrapolation_requires_forward_direction_and_zero_timer() {
        for (direction, timer) in [(-1, 0), (1, 1)] {
            let mut frame = request(RetailRuntimeValue::Known(topology(true)));
            frame.target_state.tracked_entity_handle = 0x04ac_0001;
            frame.target_state.direction = direction;
            frame.target_state.reversal_timer_ms = timer;
            frame.elapsed_micros = 0;
            frame.tracked_target =
                RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: crate::entity_collision_state::RetailStateWord::exact(1),
                    position_raw: [9, 8, 7],
                    velocity_raw: [1_000; 3],
                }));
            let plan = run_with_words(frame, &[]);
            assert!(!plan.tracked_target_extrapolated);
            assert_eq!(plan.target_state.target_position_raw, [100, 200, 2_000]);
        }
    }

    #[test]
    fn sub_i_mismatch_turns_and_propagates_without_rng() {
        let mut exact = topology(true);
        exact.sub_a = Some(sub_a(Some(SUB_A), -1));
        exact.sub_f = true;
        exact.sub_g = true;
        let plan = run_with_words(request(RetailRuntimeValue::Known(exact)), &[]);

        assert!(plan.mismatch_applied);
        assert_eq!(plan.heading_raw, 10_000u16.wrapping_add(0x2000));
        assert_eq!(
            plan.sub_a_runtime.unwrap().direction_multiplier(),
            plan.target_state.direction
        );
        assert_eq!(plan.sub_f_reverse_write, Some(false));
        assert_eq!(plan.sub_g_reverse_write, Some(false));
        assert_eq!(plan.direction_propagation_count, 1);
        assert_eq!(plan.rng_draw_count, 0);
        assert_eq!(plan.sub_d_reversal_write, None);
    }

    #[test]
    fn no_i_clamp_path_consumes_one_speed_word_then_x_and_z() {
        let mut no_i = topology(false);
        no_i.sub_a = Some(sub_a(
            Some(SubAPropulsionDescriptor {
                target_speed_base_raw: 100,
                ..SUB_A
            }),
            -1,
        ));
        no_i.sub_d = None;
        let plan = run_with_words(
            request(RetailRuntimeValue::Known(no_i)),
            &[0x0000_ffff, 0x0000_1200, 0x0000_3400],
        );

        assert_eq!(plan.rng_draw_count, 3);
        assert_eq!(
            plan.sub_a_runtime.unwrap().target_speed_raw(),
            RetailRuntimeValue::Known(COMMON_MOVER_MINIMUM_REVERSE_SPEED_RAW)
        );
        assert_eq!(plan.target_state.direction, -1);
        assert_eq!(
            plan.target_state.target_position_raw,
            [82, 50, 208],
            "near X subtracts RNG bits 8..15; far Z replaces the target with bits 6..15"
        );
    }

    #[test]
    fn no_i_non_clamp_path_discards_first_candidate_and_draws_speed_again() {
        let mut no_i = topology(false);
        no_i.sub_a = Some(sub_a(Some(SUB_A), -1));
        no_i.sub_d = None;
        let plan = run_with_words(
            request(RetailRuntimeValue::Known(no_i)),
            &[0, 0x0000_ffff, 0x100, 0x200],
        );

        assert_eq!(plan.rng_draw_count, 4);
        assert_eq!(
            plan.sub_a_runtime.unwrap().target_speed_raw(),
            RetailRuntimeValue::Known(489),
            "the first candidate (1000) is deliberately discarded"
        );
    }

    #[test]
    fn no_i_runtime_without_descriptor_skips_speed_rng_but_still_retargets() {
        let mut no_i = topology(false);
        no_i.sub_a = Some(sub_a(None, -1));
        no_i.sub_d = None;
        let plan = run_with_words(request(RetailRuntimeValue::Known(no_i)), &[0x0100, 0x0200]);

        assert_eq!(plan.rng_draw_count, 2);
        assert_eq!(
            plan.sub_a_runtime.unwrap().target_speed_raw(),
            RetailRuntimeValue::Known(777)
        );
        assert_eq!(plan.target_state.direction, -1);
    }

    #[test]
    fn no_i_far_retarget_preserves_the_retail_x_z_asymmetry() {
        let mut no_i = topology(false);
        no_i.sub_a = Some(sub_a(None, -1));
        no_i.sub_d = None;
        let mut frame = request(RetailRuntimeValue::Known(no_i));
        frame.target_state.target_position_raw = [2_000, 200, -2_000];
        frame.controlled_position_raw = [0, 50, 0];
        let plan = run_with_words(frame, &[0x0000_4000, 0x0000_4000]);

        assert_eq!(
            plan.target_state.target_position_raw,
            [-256, 50, 256],
            "far X subtracts 0x200 while far Z does not"
        );
    }

    #[test]
    fn timer_uses_integer_milliseconds_and_expires_at_less_than_one() {
        let mut frame = request(RetailRuntimeValue::Known(topology(true)));
        frame.target_state.reversal_timer_ms = 1;
        frame.elapsed_micros = 999;
        let retained = run_with_words(frame, &[]);
        assert_eq!(retained.target_state.reversal_timer_ms, 1);
        assert!(!retained.timer_expired);

        frame.elapsed_micros = 1_000;
        let expired = run_with_words(frame, &[0x0000_ff00]);
        assert_eq!(expired.target_state.reversal_timer_ms, 0);
        assert_eq!(expired.target_state.direction, 1);
        assert!(expired.timer_expired);
        assert_eq!(
            expired.sub_a_runtime.unwrap().target_speed_raw(),
            RetailRuntimeValue::Known(1_099)
        );

        frame.target_state.reversal_timer_ms = -7;
        frame.elapsed_micros = 0;
        let negative = run_with_words(frame, &[0]);
        assert!(negative.timer_expired);
        assert_eq!(negative.target_state.reversal_timer_ms, 0);
    }

    #[test]
    fn mismatch_runs_before_same_frame_timer_expiry() {
        let mut exact = topology(true);
        exact.sub_a = Some(sub_a(Some(SUB_A), 1));
        exact.sub_f = true;
        exact.sub_g = true;
        let mut frame = request(RetailRuntimeValue::Known(exact));
        frame.target_state.direction = -1;
        frame.target_state.reversal_timer_ms = 1;
        frame.elapsed_micros = 1_000;
        let plan = run_with_words(frame, &[0]);

        assert!(plan.mismatch_applied);
        assert!(plan.timer_expired);
        assert_eq!(plan.heading_raw, 10_000u16.wrapping_add(0x2000));
        assert_eq!(plan.target_state.direction, 1);
        assert_eq!(plan.sub_a_runtime.unwrap().direction_multiplier(), 1);
        assert_eq!(plan.sub_f_reverse_write, Some(false));
        assert_eq!(plan.sub_g_reverse_write, Some(false));
        assert_eq!(plan.direction_propagation_count, 2);
    }

    #[test]
    fn sub_l_receives_the_final_post_retarget_target() {
        let mut no_i = topology(false);
        no_i.sub_a = Some(sub_a(None, -1));
        no_i.sub_d = None;
        no_i.sub_l = true;
        let plan = run_with_words(request(RetailRuntimeValue::Known(no_i)), &[0x0100, 0x0200]);
        assert_eq!(
            plan.sub_l_target_write,
            Some(plan.target_state.target_position_raw)
        );
    }

    #[test]
    fn no_i_sub_d_reversal_writes_heading_roll_and_sign_extended_step() {
        let mut no_i = topology(false);
        no_i.sub_a = Some(sub_a(None, -1));
        no_i.sub_d = Some(CommonMoverPreludeSubD {
            steering_divisor_raw: 20,
            couple_yaw_into_roll: true,
        });
        let plan = run_with_words(request(RetailRuntimeValue::Known(no_i)), &[0x0100, 0x0200]);
        assert_eq!(
            plan.sub_d_reversal_write,
            Some(CommonMoverSubDReversalWrite {
                step_raw: 6_250,
                heading_raw: 3_750,
                roll_raw: 13_750,
            })
        );
        assert_eq!(plan.heading_raw, 3_750);
        assert_eq!(plan.roll_raw, 13_750);

        no_i.sub_d = Some(CommonMoverPreludeSubD {
            steering_divisor_raw: -19,
            couple_yaw_into_roll: false,
        });
        let negative = run_with_words(request(RetailRuntimeValue::Known(no_i)), &[0x0100, 0x0200]);
        assert_eq!(
            negative.sub_d_reversal_write.unwrap().step_raw,
            -7_813,
            "-19 quarters toward zero to -4 before signed fixed-point shift"
        );
        assert_eq!(negative.roll_raw, 20_000);
    }

    #[test]
    fn every_error_path_is_rng_atomic() {
        let mut calls = 0;
        let mut unresolved_target = request(RetailRuntimeValue::Known(topology(false)));
        unresolved_target.target_state.tracked_entity_handle = 0x04ac_0001;
        assert_eq!(
            plan_common_mover_target_prelude(unresolved_target, || {
                calls += 1;
                0
            }),
            Err(CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
                handle: 0x04ac_0001
            })
        );
        assert_eq!(calls, 0);

        assert_eq!(
            plan_common_mover_target_prelude(request(RetailRuntimeValue::Unresolved), || {
                calls += 1;
                0
            }),
            Err(CommonMoverTargetPreludeBlock::UnresolvedTopology)
        );
        assert_eq!(calls, 0);

        let mut invalid_d = topology(false);
        invalid_d.sub_a = Some(sub_a(None, -1));
        invalid_d.sub_d = Some(CommonMoverPreludeSubD {
            steering_divisor_raw: 3,
            couple_yaw_into_roll: true,
        });
        assert_eq!(
            plan_common_mover_target_prelude(request(RetailRuntimeValue::Known(invalid_d)), || {
                calls += 1;
                0
            }),
            Err(CommonMoverTargetPreludeBlock::InvalidSubDReversalDivisor {
                steering_divisor_raw: 3
            })
        );
        assert_eq!(calls, 0);
    }

    #[test]
    fn post_sub_d_fanout_smooths_f_and_k_but_writes_l_exactly() {
        assert_eq!(
            plan_common_mover_post_sub_d_writes(CommonMoverPostSubDWriteRequest {
                sub_d_result_raw: -300,
                target_position_raw: [1, 2, 3],
                sub_f_smoothed_raw: Some(500),
                sub_k_smoothed_raw: Some(-500),
                sub_l: true,
            }),
            CommonMoverPostSubDWritePlan {
                sub_f: Some(CommonMoverSubFWrite {
                    smoothed_raw: 450,
                    target_position_raw: [1, 2, 3],
                }),
                sub_k_smoothed_raw: Some(-488),
                sub_l_exact_raw: Some(-300),
            }
        );

        assert_eq!(
            smooth_sub_d_result(i32::MIN, i32::MAX),
            i32::MIN.wrapping_add(-1 >> 4),
            "the subtract and add retain x86 wrapping semantics"
        );
    }
}
