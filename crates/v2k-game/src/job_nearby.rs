//! Pure rule-13 `"Job Nearby"` evaluation and job-aware initial selection.
//!
//! `FUN_004165C0` walks the live intrusive entity list in order, applies
//! `FUN_00423030`'s strict wrapped-coordinate cube, and accepts the first live
//! factory-like component with free capacity for a capable subject. This
//! module closes that read-only decision. It does not append a scientist,
//! install scheduler tasks, or own retail's process-global RNG state.

use v2k_formats::collision::BehaviorChoice;

use crate::entity_behavior::{
    select_initial_behavior, BehaviorSelection, BehaviorSelectionError, BehaviorWeightRule,
};
use crate::entity_collision_state::{RetailRuntimeValue, RetailStateWord, DYING_STATE_BIT};
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};

pub const JOB_NEARBY_RULE_ID: u32 = 13;
pub const JOB_NEARBY_EVALUATOR_ADDRESS: u32 = 0x0041_65C0;
pub const JOB_NEARBY_CANDIDATE_HELPER_ADDRESS: u32 = 0x0041_8EB0;
pub const JOB_NEARBY_SOURCE_CAPABILITY_BIT: u32 = 0x0000_0400;

#[derive(Debug, Clone, Copy)]
pub struct JobNearbyOwner {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub capability_flags: RetailRuntimeValue<u32>,
}

/// Signed fields read from candidate job state `+0x68` and `+0x04`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobCapacityState {
    pub current_jobs_raw: i32,
    pub capacity_raw: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct JobNearbyCandidate {
    pub id: u32,
    pub position_raw: [i16; 3],
    pub state_flags: RetailStateWord,
    /// `Known(None)` proves the `+0x4C -> +0x0C -> +0x30` component is absent.
    pub capacity: RetailRuntimeValue<Option<JobCapacityState>>,
}

#[derive(Debug, Clone, Copy)]
pub struct JobNearbyEvaluationRequest<'a> {
    pub owner: JobNearbyOwner,
    /// Resolved entries in retail intrusive-list order, without its terminal
    /// sentinel. The first accepted entry ends the scan.
    pub candidates_in_intrusive_order: &'a [JobNearbyCandidate],
    pub range: WrappedAxisRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobNearbyEvaluationError {
    CandidateStateUnresolved { id: u32 },
    CandidateCapacityUnresolved { id: u32 },
    OwnerCapabilityUnresolved,
}

/// Evaluate `FUN_004165C0` without consuming RNG or mutating either entity.
pub fn evaluate_job_nearby(
    request: JobNearbyEvaluationRequest<'_>,
) -> Result<bool, JobNearbyEvaluationError> {
    for candidate in request.candidates_in_intrusive_order {
        if candidate.id == request.owner.id {
            continue;
        }
        if !candidate_state_is_live(candidate.id, candidate.state_flags)? {
            continue;
        }
        if !within_wrapped_axis_range(
            request.range,
            request.owner.position_raw,
            candidate.position_raw,
        ) {
            continue;
        }
        if candidate_accepts_job(candidate, request.owner.capability_flags)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Exact `FUN_00418EB0` capacity/capability predicate shared by the rule-13
/// initializer gate and class-54's later nearest-job selector.
pub(crate) fn candidate_accepts_job(
    candidate: &JobNearbyCandidate,
    owner_capability_flags: RetailRuntimeValue<u32>,
) -> Result<bool, JobNearbyEvaluationError> {
    // FUN_00418EB0 deliberately repeats the caller's live/dying gates.
    if !candidate_state_is_live(candidate.id, candidate.state_flags)? {
        return Ok(false);
    }
    let has_capacity = match candidate.capacity {
        RetailRuntimeValue::Known(Some(capacity)) => {
            capacity.current_jobs_raw < capacity.capacity_raw
        }
        RetailRuntimeValue::Known(None) => false,
        RetailRuntimeValue::Unresolved => {
            return Err(JobNearbyEvaluationError::CandidateCapacityUnresolved { id: candidate.id });
        }
    };
    if !has_capacity {
        return Ok(false);
    }
    match owner_capability_flags {
        RetailRuntimeValue::Known(flags) => Ok(flags & JOB_NEARBY_SOURCE_CAPABILITY_BIT != 0),
        RetailRuntimeValue::Unresolved => Err(JobNearbyEvaluationError::OwnerCapabilityUnresolved),
    }
}

pub(crate) fn candidate_state_is_live(
    id: u32,
    state: RetailStateWord,
) -> Result<bool, JobNearbyEvaluationError> {
    if state.known_value_bits() == 0 {
        if state.known_mask() == u32::MAX {
            return Ok(false);
        }
        return Err(JobNearbyEvaluationError::CandidateStateUnresolved { id });
    }
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(0) => Ok(true),
        RetailRuntimeValue::Known(_) => Ok(false),
        RetailRuntimeValue::Unresolved => {
            Err(JobNearbyEvaluationError::CandidateStateUnresolved { id })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobAwareBehaviorSelectionError {
    JobNearby(JobNearbyEvaluationError),
    UnsupportedWeightRule { raw: u32 },
    Selection(BehaviorSelectionError),
}

/// Run the recovered `Always`/`JobNearby` rule set and retail weighted selector.
///
/// The caller supplies the shared-process RNG callback. It is invoked exactly
/// once after successful world evaluation, including the no-job fallback.
pub fn select_job_aware_initial_behavior(
    choices: &[BehaviorChoice],
    request: JobNearbyEvaluationRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Option<BehaviorSelection>, JobAwareBehaviorSelectionError> {
    for choice in choices {
        match BehaviorWeightRule::from_raw(choice.weight_rule_id) {
            Some(BehaviorWeightRule::Always | BehaviorWeightRule::JobNearby) => {}
            _ => {
                return Err(JobAwareBehaviorSelectionError::UnsupportedWeightRule {
                    raw: choice.weight_rule_id,
                });
            }
        }
    }

    let job_nearby =
        evaluate_job_nearby(request).map_err(JobAwareBehaviorSelectionError::JobNearby)?;
    select_initial_behavior(
        choices,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::JobNearby => i32::from(job_nearby),
            _ => unreachable!("rule set validated before RNG ownership begins"),
        },
        next_random,
    )
    .map_err(JobAwareBehaviorSelectionError::Selection)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn owner(capability_flags: RetailRuntimeValue<u32>) -> JobNearbyOwner {
        JobNearbyOwner {
            id: 7,
            position_raw: [0, 0, 0],
            capability_flags,
        }
    }

    fn candidate(
        id: u32,
        position_raw: [i16; 3],
        state: RetailStateWord,
        capacity: RetailRuntimeValue<Option<JobCapacityState>>,
    ) -> JobNearbyCandidate {
        JobNearbyCandidate {
            id,
            position_raw,
            state_flags: state,
            capacity,
        }
    }

    fn open_capacity() -> RetailRuntimeValue<Option<JobCapacityState>> {
        RetailRuntimeValue::Known(Some(JobCapacityState {
            current_jobs_raw: 1,
            capacity_raw: 2,
        }))
    }

    fn request<'a>(
        owner: JobNearbyOwner,
        candidates: &'a [JobNearbyCandidate],
    ) -> JobNearbyEvaluationRequest<'a> {
        JobNearbyEvaluationRequest {
            owner,
            candidates_in_intrusive_order: candidates,
            range: WrappedAxisRange::strict(0x0F00).unwrap(),
        }
    }

    #[test]
    fn strict_range_self_state_capacity_and_capability_gates_match_retail() {
        let candidates = [
            candidate(7, [0, 0, 0], RetailStateWord::exact(1), open_capacity()),
            candidate(
                8,
                [0x0F00, 0, 0],
                RetailStateWord::exact(1),
                open_capacity(),
            ),
            candidate(
                9,
                [0x0EFF, 0, 0],
                RetailStateWord::exact(DYING_STATE_BIT | 1),
                open_capacity(),
            ),
            candidate(
                10,
                [0x0EFF, 0, 0],
                RetailStateWord::exact(1),
                RetailRuntimeValue::Known(None),
            ),
            candidate(
                11,
                [0x0EFF, 0, 0],
                RetailStateWord::exact(1),
                RetailRuntimeValue::Known(Some(JobCapacityState {
                    current_jobs_raw: 2,
                    capacity_raw: 2,
                })),
            ),
            candidate(
                12,
                [0x0EFF, -0x0EFF, 0],
                RetailStateWord::exact(1),
                open_capacity(),
            ),
        ];
        assert!(evaluate_job_nearby(request(
            owner(RetailRuntimeValue::Known(0x1404)),
            &candidates,
        ))
        .unwrap());
        assert!(!evaluate_job_nearby(request(
            owner(RetailRuntimeValue::Known(0x1004)),
            &candidates,
        ))
        .unwrap());
    }

    #[test]
    fn zero_range_is_unbounded_and_negative_nonzero_rejects_every_candidate() {
        let candidates = [candidate(
            8,
            [i16::MIN, i16::MAX, 123],
            RetailStateWord::exact(1),
            open_capacity(),
        )];
        let mut query = request(owner(RetailRuntimeValue::Known(0x400)), &candidates);
        query.range = WrappedAxisRange::Unbounded;
        assert!(evaluate_job_nearby(query).unwrap());
        query.range = WrappedAxisRange::strict(-1).unwrap();
        assert!(!evaluate_job_nearby(query).unwrap());
    }

    #[test]
    fn unresolved_consumed_fields_fail_closed_but_later_entries_are_not_read() {
        let accepted = candidate(8, [0, 0, 0], RetailStateWord::exact(1), open_capacity());
        let unresolved = candidate(
            9,
            [0, 0, 0],
            RetailStateWord::unknown(),
            RetailRuntimeValue::Unresolved,
        );
        assert!(evaluate_job_nearby(request(
            owner(RetailRuntimeValue::Known(0x400)),
            &[accepted, unresolved],
        ))
        .unwrap());

        assert_eq!(
            evaluate_job_nearby(request(
                owner(RetailRuntimeValue::Known(0x400)),
                &[unresolved],
            )),
            Err(JobNearbyEvaluationError::CandidateStateUnresolved { id: 9 })
        );
        let unresolved_capacity = candidate(
            10,
            [0, 0, 0],
            RetailStateWord::exact(1),
            RetailRuntimeValue::Unresolved,
        );
        assert_eq!(
            evaluate_job_nearby(request(
                owner(RetailRuntimeValue::Known(0x400)),
                &[unresolved_capacity],
            )),
            Err(JobNearbyEvaluationError::CandidateCapacityUnresolved { id: 10 })
        );
        assert_eq!(
            evaluate_job_nearby(request(owner(RetailRuntimeValue::Unresolved), &[accepted])),
            Err(JobNearbyEvaluationError::OwnerCapabilityUnresolved)
        );
    }

    #[test]
    fn type8_selection_preserves_authored_order_thresholds_and_one_rng_draw() {
        let choices = [
            BehaviorChoice {
                weight_rule_id: JOB_NEARBY_RULE_ID,
                weight_multiplier: 100,
                behavior_class_id: 54,
            },
            BehaviorChoice {
                weight_rule_id: 1,
                weight_multiplier: 1,
                behavior_class_id: 6,
            },
        ];
        let candidates = [candidate(
            8,
            [0, 0, 0],
            RetailStateWord::exact(1),
            open_capacity(),
        )];
        for (random, expected_index, expected_class) in
            [(64_239_u32, 0_usize, 54_u8), (64_888_u32, 1_usize, 6_u8)]
        {
            let calls = Cell::new(0);
            let selection = select_job_aware_initial_behavior(
                &choices,
                request(owner(RetailRuntimeValue::Known(0x1404)), &candidates),
                || {
                    calls.set(calls.get() + 1);
                    random
                },
            )
            .unwrap()
            .unwrap();
            assert_eq!(calls.get(), 1);
            assert_eq!(selection.choice_index, expected_index);
            assert_eq!(selection.program.class_id, expected_class);
        }
        assert_eq!(
            select_job_aware_initial_behavior(
                &choices,
                request(owner(RetailRuntimeValue::Known(0x1404)), &[]),
                || 0,
            )
            .unwrap()
            .unwrap()
            .program
            .class_id,
            6
        );
        assert_eq!(
            select_job_aware_initial_behavior(
                &choices,
                request(owner(RetailRuntimeValue::Known(0x1404)), &candidates),
                || 0,
            )
            .unwrap()
            .unwrap()
            .program
            .initial_style
            .frame_address,
            0x004C_8788
        );
    }
}
