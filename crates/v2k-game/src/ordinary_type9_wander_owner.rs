//! Mutation-safe owner bridge for ordinary type-9 `"Wander Near Location"`.
//!
//! This joins the recovered task wrapper, Wander private state, and scheduler
//! contracts without pretending the still-unwired live entity dispatcher is
//! known. It preserves two retail transactions:
//!
//! - `FUN_0040AD10` clears slot 2, clears slot 1, then prepares and installs
//!   slot 0; and
//! - `FUN_00401120` increments elapsed time, runs `FUN_00402EB0`, unwinds the
//!   wrapper, interprets its result, and only then checks the strict 5,000-ms
//!   timeout.

use crate::actor_task_owner::{
    ActorTaskId, ActorTaskMutation, ActorTaskOwner, ActorTaskPrepareError, ActorTaskSlot,
    ActorTaskVisitControl, PreparedActorTask,
};
use crate::common_mover::SubAPropulsionRuntime;
use crate::wander_near_location::{
    construct_ordinary_type9_wander_near, map_common_mover_return, WanderNearCommonMoverReturn,
    WanderNearLifetime, WanderNearLifetimeStatus, WanderNearPrivateState, WanderNearRetarget,
    WanderNearTaggedResult, WanderNearTaskCallbackResult,
};

/// Exact slot-0 inner state owned by the ordinary class-6 task wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderTaskState {
    private_state: WanderNearPrivateState,
    lifetime: WanderNearLifetime,
}

impl OrdinaryType9WanderTaskState {
    /// Private record after `FUN_004012E0` and before the shared
    /// `FUN_00406070` suffix. Type-47 class-6 / Guard slot-0 construction
    /// must not consume the type-9 Sub-A-only constructor word here.
    pub(crate) const fn from_allocated_anchor(current_position_raw: [i16; 3]) -> Self {
        Self {
            private_state: WanderNearPrivateState::ordinary_type9(current_position_raw),
            lifetime: WanderNearLifetime::new(),
        }
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.lifetime.elapsed_ms()
    }

    /// Replace only the private record after the live adapter has authenticated
    /// the exact surviving task visit and compared the complete mover
    /// before-state.
    ///
    /// This is crate-private because ordinary callback code should continue to
    /// use the staged [`OrdinaryType9WanderCallbackStage`] transaction.
    pub(crate) fn replace_private_state(&mut self, private_state: WanderNearPrivateState) {
        self.private_state = private_state;
    }

    /// Run generic-owner phase 1 for this task only.
    pub fn before_callback(&mut self, elapsed_micros: u32) -> WanderNearLifetimeStatus {
        self.lifetime.advance_frame(elapsed_micros)
    }

    /// Commit the callback's RNG-owned retarget and stage subsequent
    /// common-mover writes.
    ///
    /// Retail performs the retarget against live task-private state before
    /// calling the common mover. The returned stage starts from that committed
    /// prefix, but later mover writes remain detached until
    /// [`OrdinaryType9WanderCallbackStage::commit`] is called for the same
    /// surviving wrapper.
    pub fn stage_callback(
        &mut self,
        immutable_anchor_raw: [i16; 3],
        next_random: impl FnMut() -> u32,
    ) -> OrdinaryType9WanderCallbackStage {
        let retarget = self
            .private_state
            .retarget_tick(immutable_anchor_raw, next_random);
        OrdinaryType9WanderCallbackStage {
            retarget,
            private_state: self.private_state,
        }
    }
}

/// Proven inputs to `FUN_00402E20` after wrapper/private allocation succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderTaskSpec {
    current_position_raw: [i16; 3],
    sub_a_target_speed_base_raw: i16,
}

impl OrdinaryType9WanderTaskSpec {
    pub const fn current_position_raw(self) -> [i16; 3] {
        self.current_position_raw
    }

    pub const fn sub_a_target_speed_base_raw(self) -> i16 {
        self.sub_a_target_speed_base_raw
    }

    /// Complete the infallible post-allocation constructor suffix.
    ///
    /// Call this only after allocation/private initialization succeeds. It
    /// consumes exactly one shared RNG draw and returns the synchronized task
    /// state plus external Sub-A reset. Retail has no fallible operation
    /// between that reset and publishing the new slot.
    pub fn prepare_after_allocation(
        self,
        mut next_random: impl FnMut() -> u32,
    ) -> PreparedOrdinaryType9WanderTask {
        let construction = construct_ordinary_type9_wander_near(
            self.current_position_raw,
            self.sub_a_target_speed_base_raw,
            &mut next_random,
        );
        PreparedOrdinaryType9WanderTask {
            task: PreparedActorTask::new(OrdinaryType9WanderTaskState {
                private_state: construction.private_state,
                lifetime: construction.lifetime,
            }),
            sub_a_reset: construction.sub_a_reset,
        }
    }
}

/// Successful post-allocation result. Private fields prevent task state and
/// Sub-A reset from being desynchronized.
#[derive(Debug)]
pub struct PreparedOrdinaryType9WanderTask<T = OrdinaryType9WanderTaskState> {
    task: PreparedActorTask<T>,
    sub_a_reset: crate::wander_near_location::WanderNearSubAReset,
}

impl PreparedOrdinaryType9WanderTask {
    /// Wrap the fully initialized family state for a heterogeneous task owner
    /// without separating it from the constructor's Sub-A reset.
    pub fn map_task<T>(
        self,
        map: impl FnOnce(OrdinaryType9WanderTaskState) -> T,
    ) -> PreparedOrdinaryType9WanderTask<T> {
        PreparedOrdinaryType9WanderTask {
            task: self.task.map(map),
            sub_a_reset: self.sub_a_reset,
        }
    }
}

/// One-shot `FUN_0040AD10` task transaction.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9WanderSetupPlan {
    ordered_mutations: [ActorTaskMutation<OrdinaryType9WanderTaskSpec>; 3],
}

impl OrdinaryType9WanderSetupPlan {
    /// Apply clear-tertiary, clear-secondary, then try-install-primary.
    ///
    /// On `prepare` error the old primary survives, earlier clears remain
    /// committed, and no reset is applied. On success the reset is applied
    /// before the prepared wrapper replaces the old primary, matching
    /// `FUN_00406030/00406070`.
    pub fn apply<T, E>(
        self,
        owner: &mut ActorTaskOwner<T>,
        sub_a: &mut SubAPropulsionRuntime,
        prepare: impl FnMut(
            OrdinaryType9WanderTaskSpec,
        ) -> Result<PreparedOrdinaryType9WanderTask<T>, E>,
    ) -> Result<(), ActorTaskPrepareError<E>> {
        self.apply_with_retirement(owner, sub_a, prepare, |_| {})
    }

    pub fn apply_with_retirement<T, E>(
        self,
        owner: &mut ActorTaskOwner<T>,
        sub_a: &mut SubAPropulsionRuntime,
        mut prepare: impl FnMut(
            OrdinaryType9WanderTaskSpec,
        ) -> Result<PreparedOrdinaryType9WanderTask<T>, E>,
        retire: impl FnMut(&T),
    ) -> Result<(), ActorTaskPrepareError<E>> {
        owner.apply_ordered_mutations_with_retirement(
            self.ordered_mutations,
            |specification| {
                let prepared = prepare(specification)?;
                sub_a.apply_wander_near_reset(prepared.sub_a_reset);
                Ok(prepared.task)
            },
            retire,
        )
    }
}

/// Staged private state for one ordinary Wander callback invocation.
///
/// `retarget` was already committed to the live task before callback entry.
/// The copied private record additionally carries any common-mover writes and
/// is published only when the same wrapper survives and the mover return is
/// resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderCallbackStage {
    retarget: WanderNearRetarget,
    private_state: WanderNearPrivateState,
}

impl OrdinaryType9WanderCallbackStage {
    pub const fn retarget(self) -> WanderNearRetarget {
        self.retarget
    }

    pub const fn private_state(self) -> WanderNearPrivateState {
        self.private_state
    }

    pub fn private_state_mut(&mut self) -> &mut WanderNearPrivateState {
        &mut self.private_state
    }

    pub fn commit(self, surviving_state: &mut OrdinaryType9WanderTaskState) {
        surviving_state.private_state = self.private_state;
    }

    pub(crate) const fn retained(private_state: WanderNearPrivateState) -> Self {
        Self {
            retarget: WanderNearRetarget::Retained,
            private_state,
        }
    }
}

/// Build the exact class-6 ordered setup transaction.
pub fn plan_ordinary_type9_wander_setup(
    current_position_raw: [i16; 3],
    sub_a_target_speed_base_raw: i16,
) -> OrdinaryType9WanderSetupPlan {
    let specification = OrdinaryType9WanderTaskSpec {
        current_position_raw,
        sub_a_target_speed_base_raw,
    };
    OrdinaryType9WanderSetupPlan {
        ordered_mutations: [
            ActorTaskMutation::Clear {
                slot: ActorTaskSlot::Tertiary,
            },
            ActorTaskMutation::Clear {
                slot: ActorTaskSlot::Secondary,
            },
            ActorTaskMutation::TryInstall {
                slot: ActorTaskSlot::Primary,
                specification,
            },
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderFrameRequest {
    pub immutable_anchor_raw: [i16; 3],
    pub elapsed_micros: u32,
}

/// Mutations already committed before common-mover/result dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderCallbackPrefix {
    pub lifetime_status: WanderNearLifetimeStatus,
    pub retarget: WanderNearRetarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9WanderTransitionReason {
    CommonMoverCompleted(WanderNearTaggedResult),
    LifetimeExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9WanderTransitionRequest {
    pub slot: ActorTaskSlot,
    pub task_id: ActorTaskId,
    pub reason: OrdinaryType9WanderTransitionReason,
    pub committed_prefix: OrdinaryType9WanderCallbackPrefix,
}

/// Pure phase-3 decision after a surviving callback has unwound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9WanderPostUnwind {
    Continue,
    Transition(OrdinaryType9WanderTransitionRequest),
    UnresolvedCommonMover,
}

/// Resolve the callback result before invoking the behavior owner's transition
/// adapter.
///
/// A tagged common-mover result takes precedence over the strict timeout.
/// Unresolved mover state remains an explicit block and is never converted
/// into either retail zero or nonzero.
pub const fn ordinary_type9_wander_after_unwind(
    visit: crate::actor_task_owner::ActorTaskVisit,
    committed_prefix: OrdinaryType9WanderCallbackPrefix,
    callback_result: WanderNearTaskCallbackResult,
) -> OrdinaryType9WanderPostUnwind {
    let reason = match callback_result {
        WanderNearTaskCallbackResult::BlockedUnresolvedCommonMover => {
            return OrdinaryType9WanderPostUnwind::UnresolvedCommonMover;
        }
        WanderNearTaskCallbackResult::TaggedOwnerTransition(tagged) => {
            OrdinaryType9WanderTransitionReason::CommonMoverCompleted(tagged)
        }
        WanderNearTaskCallbackResult::Continue => match committed_prefix.lifetime_status {
            WanderNearLifetimeStatus::Active => {
                return OrdinaryType9WanderPostUnwind::Continue;
            }
            WanderNearLifetimeStatus::OwnerTransitionDue => {
                OrdinaryType9WanderTransitionReason::LifetimeExpired
            }
        },
    };
    OrdinaryType9WanderPostUnwind::Transition(OrdinaryType9WanderTransitionRequest {
        slot: visit.slot,
        task_id: visit.task_id,
        reason,
        committed_prefix,
    })
}

/// Result of resolving the behavior owner's primary transition callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9WanderTransitionOutcome<R> {
    /// The owner root has no primary callback. A tagged result then reaches the
    /// timeout check, which necessarily finds the same callback absent.
    CallbackAbsent,
    /// `FUN_00416410` rejected the transition. Tagged `0x9C01` returns
    /// immediately and does not fall through to timeout.
    SuppressedByEntityState,
    /// The callback ran. Its optional result follows ordinary owner traversal.
    Completed(Option<R>),
}

/// Fail-closed boundaries in the still-explicit live dispatch seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9WanderOwnerError<MoverError, TransitionError> {
    CommonMover {
        slot: ActorTaskSlot,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
        error: MoverError,
    },
    UnresolvedCommonMover {
        slot: ActorTaskSlot,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
    },
    OwnerTransition {
        request: OrdinaryType9WanderTransitionRequest,
        error: TransitionError,
    },
}

#[derive(Debug)]
struct CallbackOutput<MoverError> {
    retarget: WanderNearRetarget,
    result: Result<WanderNearTaskCallbackResult, MoverError>,
}

/// Tick the three-slot owner in exact scheduler order.
///
/// `common_mover` receives a staged private state after the frame's
/// one-or-three RNG retarget. Mutations are committed only for resolved
/// zero/nonzero returns. Any mover error or unresolved return is reported after
/// callback unwind with only the preceding lifetime/RNG/retarget prefix
/// committed.
///
/// `primary_transition` runs after callback unwind with
/// `in_callback == false`. Its explicit absent/suppressed/completed outcome
/// preserves tagged `0x9C01`'s asymmetric timeout behavior. A completed
/// `Some(value)` propagates that owner result. The adapter must return
/// `CallbackAbsent` or `SuppressedByEntityState` without mutating the owner;
/// only `Completed` may include transition-owned writes.
pub fn tick_ordinary_type9_wander_owner<R, MoverError, TransitionError>(
    owner: &mut ActorTaskOwner<OrdinaryType9WanderTaskState>,
    request: OrdinaryType9WanderFrameRequest,
    mut next_random: impl FnMut() -> u32,
    mut common_mover: impl FnMut(
        ActorTaskSlot,
        &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, MoverError>,
    mut primary_transition: impl FnMut(
        &mut ActorTaskOwner<OrdinaryType9WanderTaskState>,
        OrdinaryType9WanderTransitionRequest,
    ) -> Result<
        OrdinaryType9WanderTransitionOutcome<R>,
        TransitionError,
    >,
) -> Result<Option<R>, OrdinaryType9WanderOwnerError<MoverError, TransitionError>> {
    let propagated = owner.visit_slots_fresh_phased(
        |state, _visit| state.before_callback(request.elapsed_micros),
        |owner, visit| {
            let mut stage = {
                let state = owner
                    .task_state_mut(visit.task_id)
                    .expect("a live callback must retain its inner task state");
                state.stage_callback(request.immutable_anchor_raw, &mut next_random)
            };
            let retarget = stage.retarget();
            let mover_result = common_mover(visit.slot, stage.private_state_mut());
            if matches!(
                &mover_result,
                Ok(WanderNearCommonMoverReturn::NonZero | WanderNearCommonMoverReturn::Zero)
            ) {
                stage.commit(
                    owner
                        .task_state_mut(visit.task_id)
                        .expect("a live callback must retain its inner task state"),
                );
            }
            CallbackOutput {
                retarget,
                result: mover_result.map(map_common_mover_return),
            }
        },
        |owner, visit, lifetime_status, callback| {
            let committed_prefix = OrdinaryType9WanderCallbackPrefix {
                lifetime_status,
                retarget: callback.retarget,
            };
            let callback_result = match callback.result {
                Err(error) => {
                    return ActorTaskVisitControl::Propagate(Err(
                        OrdinaryType9WanderOwnerError::CommonMover {
                            slot: visit.slot,
                            committed_prefix,
                            error,
                        },
                    ));
                }
                Ok(result) => result,
            };
            let transition_request = match ordinary_type9_wander_after_unwind(
                visit,
                committed_prefix,
                callback_result,
            ) {
                OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => {
                    return ActorTaskVisitControl::Propagate(Err(
                        OrdinaryType9WanderOwnerError::UnresolvedCommonMover {
                            slot: visit.slot,
                            committed_prefix,
                        },
                    ));
                }
                OrdinaryType9WanderPostUnwind::Continue => {
                    return ActorTaskVisitControl::Continue;
                }
                OrdinaryType9WanderPostUnwind::Transition(request) => request,
            };
            match primary_transition(owner, transition_request) {
                Ok(OrdinaryType9WanderTransitionOutcome::Completed(Some(value))) => {
                    ActorTaskVisitControl::Propagate(Ok(value))
                }
                Ok(
                    OrdinaryType9WanderTransitionOutcome::CallbackAbsent
                    | OrdinaryType9WanderTransitionOutcome::SuppressedByEntityState
                    | OrdinaryType9WanderTransitionOutcome::Completed(None),
                ) => ActorTaskVisitControl::Continue,
                Err(error) => ActorTaskVisitControl::Propagate(Err(
                    OrdinaryType9WanderOwnerError::OwnerTransition {
                        request: transition_request,
                        error,
                    },
                )),
            }
        },
    );

    match propagated {
        None => Ok(None),
        Some(Ok(value)) => Ok(Some(value)),
        Some(Err(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::convert::Infallible;

    use super::*;
    use crate::actor_task_owner::ActorTaskWrapperFlags;
    use crate::entity_collision_state::RetailRuntimeValue;

    fn old_task(marker: i16) -> OrdinaryType9WanderTaskState {
        OrdinaryType9WanderTaskState {
            private_state: WanderNearPrivateState::ordinary_type9([marker, 0, 0]),
            lifetime: WanderNearLifetime::new(),
        }
    }

    fn populated_owner() -> ActorTaskOwner<OrdinaryType9WanderTaskState> {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(old_task(1)));
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(old_task(2)),
        );
        owner.replace_prepared(ActorTaskSlot::Tertiary, PreparedActorTask::new(old_task(3)));
        owner
    }

    fn sub_a() -> SubAPropulsionRuntime {
        SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(77), -1, 73)
    }

    fn primary_state(
        owner: &ActorTaskOwner<OrdinaryType9WanderTaskState>,
    ) -> OrdinaryType9WanderTaskState {
        *owner.state_in_slot(ActorTaskSlot::Primary).unwrap()
    }

    fn primary_state_mut(
        owner: &mut ActorTaskOwner<OrdinaryType9WanderTaskState>,
    ) -> &mut OrdinaryType9WanderTaskState {
        let task_id = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();
        owner.task_state_mut(task_id).unwrap()
    }

    #[test]
    fn setup_plan_uses_retail_slot_order() {
        let plan = plan_ordinary_type9_wander_setup([4, 5, 6], 2_560);
        assert_eq!(
            plan.ordered_mutations,
            [
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Tertiary,
                },
                ActorTaskMutation::Clear {
                    slot: ActorTaskSlot::Secondary,
                },
                ActorTaskMutation::TryInstall {
                    slot: ActorTaskSlot::Primary,
                    specification: OrdinaryType9WanderTaskSpec {
                        current_position_raw: [4, 5, 6],
                        sub_a_target_speed_base_raw: 2_560,
                    },
                },
            ]
        );
    }

    #[test]
    fn allocation_failure_keeps_old_primary_and_commits_earlier_clears_without_rng() {
        let mut owner = populated_owner();
        let old_primary = owner.task_in_slot(ActorTaskSlot::Primary);
        let mut propulsion = sub_a();

        let result = plan_ordinary_type9_wander_setup([4, 5, 6], 2_560).apply(
            &mut owner,
            &mut propulsion,
            |_specification| Err::<PreparedOrdinaryType9WanderTask, _>("allocation failed"),
        );

        assert!(result.is_err());
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Primary), old_primary);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Tertiary), None);
        assert_eq!(propulsion.target_speed_raw(), RetailRuntimeValue::Known(77));
        assert_eq!(propulsion.direction_multiplier(), -1);
        assert_eq!(propulsion.drive_scale_percent(), 73);
    }

    #[test]
    fn successful_setup_consumes_one_draw_resets_sub_a_and_installs_primary() {
        let mut owner = populated_owner();
        let old_primary = owner.task_in_slot(ActorTaskSlot::Primary);
        let mut propulsion = sub_a();
        let draws = Cell::new(0);

        plan_ordinary_type9_wander_setup([4, 5, 6], 2_560)
            .apply(&mut owner, &mut propulsion, |specification| {
                Ok::<_, Infallible>(specification.prepare_after_allocation(|| {
                    draws.set(draws.get() + 1);
                    0x0000_FF00
                }))
            })
            .unwrap();

        assert_eq!(draws.get(), 1);
        assert_ne!(owner.task_in_slot(ActorTaskSlot::Primary), old_primary);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Secondary), None);
        assert_eq!(owner.task_in_slot(ActorTaskSlot::Tertiary), None);
        assert_eq!(
            primary_state(&owner).private_state(),
            WanderNearPrivateState::ordinary_type9([4, 5, 6])
        );
        assert_eq!(
            propulsion.target_speed_raw(),
            RetailRuntimeValue::Known(2_815)
        );
        assert_eq!(propulsion.direction_multiplier(), 1);
        assert_eq!(propulsion.drive_scale_percent(), 73);
    }

    fn owner_with_primary() -> ActorTaskOwner<OrdinaryType9WanderTaskState> {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(old_task(10)));
        owner
    }

    #[test]
    fn mover_error_keeps_elapsed_and_one_draw_prefix_committed_after_unwind() {
        let mut owner = owner_with_primary();
        let task_id = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();
        let draws = Cell::new(0);

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [100, 200, 300],
                elapsed_micros: 1_999,
            },
            || {
                draws.set(draws.get() + 1);
                0x003F
            },
            |_slot, private| {
                assert_eq!(private.target_position_raw, [10, 0, 0]);
                private.direction = -1;
                private.reversal_timer_ms = 777;
                Err::<WanderNearCommonMoverReturn, _>("mover blocked")
            },
            |_owner, _request| {
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::<()>::CallbackAbsent)
            },
        );

        assert!(matches!(
            result,
            Err(OrdinaryType9WanderOwnerError::CommonMover {
                slot: ActorTaskSlot::Primary,
                error: "mover blocked",
                ..
            })
        ));
        assert_eq!(draws.get(), 1);
        let state = primary_state(&owner);
        assert_eq!(state.elapsed_ms(), 1);
        assert_eq!(state.private_state().direction, 1);
        assert_eq!(state.private_state().reversal_timer_ms, 0);
        assert_eq!(
            owner.wrapper_flags(task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn mover_error_keeps_three_draw_retarget_committed() {
        let mut owner = owner_with_primary();
        let words = [0x0040, 0xFFFF, 0];
        let index = Cell::new(0);

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [32_760, -123, -32_760],
                elapsed_micros: 2_000,
            },
            || {
                let current = index.get();
                index.set(current + 1);
                words[current]
            },
            |_slot, private| {
                assert_eq!(private.target_position_raw, [-31_753, -123, 31_752]);
                private.target_position_raw = [9, 8, 7];
                Err::<WanderNearCommonMoverReturn, _>("blocked")
            },
            |_owner, _request| {
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::<()>::CallbackAbsent)
            },
        );

        assert!(result.is_err());
        assert_eq!(index.get(), 3);
        let state = primary_state(&owner);
        assert_eq!(state.elapsed_ms(), 2);
        assert_eq!(
            state.private_state().target_position_raw,
            [-31_753, -123, 31_752]
        );
    }

    #[test]
    fn resolved_mover_commits_staged_private_state_before_unwind() {
        let mut owner = owner_with_primary();

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 1,
            |_slot, private| {
                private.tracked_entity_handle = 0x04FC_0001;
                private.direction = -1;
                private.reversal_timer_ms = 321;
                Ok::<_, Infallible>(WanderNearCommonMoverReturn::NonZero)
            },
            |_owner, _request| {
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::<()>::CallbackAbsent)
            },
        )
        .unwrap();

        assert_eq!(result, None);
        let private = primary_state(&owner).private_state();
        assert_eq!(private.tracked_entity_handle, 0x04FC_0001);
        assert_eq!(private.direction, -1);
        assert_eq!(private.reversal_timer_ms, 321);
    }

    #[test]
    fn strict_timeout_runs_after_successful_expiry_frame_and_after_unwind() {
        let mut owner = owner_with_primary();
        let task_id = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let first = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 5_000_000,
            },
            || 1,
            |_slot, _private| Ok::<_, Infallible>(WanderNearCommonMoverReturn::NonZero),
            |_owner, _request| {
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::Completed(Some(
                    "too early",
                )))
            },
        )
        .unwrap();
        assert_eq!(first, None);

        let second = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 1,
            |_slot, _private| Ok::<_, Infallible>(WanderNearCommonMoverReturn::NonZero),
            |owner, transition| {
                assert_eq!(
                    owner.wrapper_flags(task_id),
                    Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
                );
                assert_eq!(
                    transition.reason,
                    OrdinaryType9WanderTransitionReason::LifetimeExpired
                );
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::Completed(Some(
                    "expired",
                )))
            },
        )
        .unwrap();

        assert_eq!(second, Some("expired"));
        assert_eq!(primary_state(&owner).elapsed_ms(), 5_001);
    }

    #[test]
    fn tagged_completion_preempts_timeout_even_when_transition_is_suppressed() {
        let mut owner = owner_with_primary();
        primary_state_mut(&mut owner)
            .lifetime
            .advance_frame(5_001_000);
        let transitions = Cell::new(0);

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 1,
            |_slot, _private| Ok::<_, Infallible>(WanderNearCommonMoverReturn::Zero),
            |_owner, transition| {
                transitions.set(transitions.get() + 1);
                assert!(matches!(
                    transition.reason,
                    OrdinaryType9WanderTransitionReason::CommonMoverCompleted(_)
                ));
                Ok::<_, Infallible>(
                    OrdinaryType9WanderTransitionOutcome::<()>::SuppressedByEntityState,
                )
            },
        )
        .unwrap();

        assert_eq!(result, None);
        assert_eq!(transitions.get(), 1);
    }

    #[test]
    fn unresolved_mover_skips_due_timeout_but_keeps_prefix() {
        let mut owner = owner_with_primary();
        primary_state_mut(&mut owner)
            .lifetime
            .advance_frame(5_000_000);
        let transitions = Cell::new(0);

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 1,
            |_slot, private| {
                private.direction = -1;
                private.reversal_timer_ms = 999;
                Ok::<_, Infallible>(WanderNearCommonMoverReturn::Unresolved)
            },
            |_owner, _request| {
                transitions.set(transitions.get() + 1);
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::<()>::CallbackAbsent)
            },
        );

        assert!(matches!(
            result,
            Err(OrdinaryType9WanderOwnerError::UnresolvedCommonMover {
                slot: ActorTaskSlot::Primary,
                ..
            })
        ));
        let state = primary_state(&owner);
        assert_eq!(state.elapsed_ms(), 5_001);
        assert_eq!(state.private_state().direction, 1);
        assert_eq!(state.private_state().reversal_timer_ms, 0);
        assert_eq!(transitions.get(), 0);
    }

    #[test]
    fn completed_primary_transition_observes_freshly_replaced_secondary() {
        let mut owner = owner_with_primary();
        owner.replace_prepared(
            ActorTaskSlot::Secondary,
            PreparedActorTask::new(old_task(20)),
        );
        let mut mover_visits = Vec::new();

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 1,
            |slot, private| {
                mover_visits.push((slot, private.target_position_raw[0]));
                Ok::<_, Infallible>(if slot == ActorTaskSlot::Primary {
                    WanderNearCommonMoverReturn::Zero
                } else {
                    WanderNearCommonMoverReturn::NonZero
                })
            },
            |owner, transition| {
                assert_eq!(transition.slot, ActorTaskSlot::Primary);
                owner.replace_prepared(
                    ActorTaskSlot::Secondary,
                    PreparedActorTask::new(old_task(77)),
                );
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::<()>::Completed(None))
            },
        )
        .unwrap();

        assert_eq!(result, None);
        assert_eq!(
            mover_visits,
            vec![(ActorTaskSlot::Primary, 10), (ActorTaskSlot::Secondary, 77)]
        );
    }

    #[test]
    fn post_unwind_transition_replacement_still_propagates_result() {
        let mut owner = owner_with_primary();
        primary_state_mut(&mut owner)
            .lifetime
            .advance_frame(5_000_000);
        let old_primary = owner.task_in_slot(ActorTaskSlot::Primary).unwrap();

        let result = tick_ordinary_type9_wander_owner(
            &mut owner,
            OrdinaryType9WanderFrameRequest {
                immutable_anchor_raw: [0; 3],
                elapsed_micros: 1_000,
            },
            || 1,
            |_slot, _private| Ok::<_, Infallible>(WanderNearCommonMoverReturn::NonZero),
            |owner, _request| {
                owner
                    .replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(old_task(99)));
                Ok::<_, Infallible>(OrdinaryType9WanderTransitionOutcome::Completed(Some(
                    "transitioned",
                )))
            },
        )
        .unwrap();

        assert_eq!(result, Some("transitioned"));
        assert_eq!(owner.wrapper_flags(old_primary), None);
        assert_eq!(
            primary_state(&owner).private_state().target_position_raw[0],
            99
        );
    }
}
