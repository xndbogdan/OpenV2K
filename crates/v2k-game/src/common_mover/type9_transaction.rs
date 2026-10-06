//! Receipt-bound persistence boundary for one ordinary type-9 mover frame.
//!
//! [`super::type9::advance_ordinary_type9_common_mover`] is an atomic evidence
//! oracle: it stages the deterministic D -> I -> A -> B frame on copied
//! component state and either produces one complete result, returns retail
//! zero, or fails closed. This module adds the detached runtime transaction
//! needed to persist an accepted result without replaying it.
//!
//! Scheduler lifetime, Wander retargeting, timeout dispatch, and task-owner
//! unwind remain outside this boundary. The lease deliberately binds only the
//! controlled entity and the exact actor-task visit whose callback planned the
//! frame.

use std::num::NonZeroU64;

use crate::actor_task_owner::ActorTaskVisit;
use crate::wander_near_location::WanderNearPrivateState;

use super::actor_abdi::ActorAbdiAnimationPolicy;
use super::target_prelude::CommonMoverTargetPreludeZero;
use super::type9::{
    advance_ordinary_type9_common_mover, advance_ordinary_type9_common_mover_with_animation_policy,
    OrdinaryType9FrameBlock, OrdinaryType9FrameOutcome, OrdinaryType9FrameRequest,
    OrdinaryType9FrameState, OrdinaryType9FrameStep,
};

/// Caller-owned identity for one detached ordinary type-9 frame transaction.
///
/// The host must not reuse an identity while either the machine or its adapter
/// journal entry remains live. Entity and task identities are leases, not
/// transaction identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrdinaryType9TransactionId(NonZeroU64);

impl OrdinaryType9TransactionId {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl From<NonZeroU64> for OrdinaryType9TransactionId {
    fn from(value: NonZeroU64) -> Self {
        Self::new(value)
    }
}

/// Runtime ownership which must still identify the callback at commit time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9FrameLease {
    pub controlled_entity_id: u32,
    pub task_visit: ActorTaskVisit,
}

/// Complete mutable state covered by the one atomic frame commit.
///
/// Position, basis, elapsed time, terrain, and tracked-target samples are
/// immutable frame inputs. The component allocation, Wander target state,
/// heading, and velocity are the values which the adapter must compare and
/// replace as one lease-validated write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AppliedState {
    pub frame: OrdinaryType9FrameState,
    pub wander: WanderNearPrivateState,
    pub heading_raw: u16,
    pub velocity_raw: [i16; 3],
}

/// The sole external action of an accepted ordinary type-9 frame.
///
/// Deliberately not `Clone`: an adapter receives this action together with one
/// linear receipt and must journal execution under that receipt.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9Action {
    CommitApplied {
        lease: OrdinaryType9FrameLease,
        before: OrdinaryType9AppliedState,
        after: OrdinaryType9AppliedState,
        step: OrdinaryType9FrameStep,
    },
}

/// Linear proof that one exact commit action was issued.
///
/// Fields are private and the type is deliberately neither `Copy` nor `Clone`.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9Receipt {
    transaction_id: OrdinaryType9TransactionId,
    lease: OrdinaryType9FrameLease,
    action_sequence: u64,
}

impl OrdinaryType9Receipt {
    pub const fn transaction_id(&self) -> OrdinaryType9TransactionId {
        self.transaction_id
    }

    pub const fn lease(&self) -> OrdinaryType9FrameLease {
        self.lease
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }
}

/// One non-replayable action and its linear completion proof.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedOrdinaryType9Action {
    pub receipt: OrdinaryType9Receipt,
    pub action: OrdinaryType9Action,
}

/// Why the adapter could not perform the atomic state replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9ExternalBlock {
    LeaseUnavailable,
    StateCommitUnavailable,
}

/// Completion supplied for the one outstanding commit.
///
/// An acknowledgement echoes the state actually persisted. This makes a
/// stale, partial, or otherwise wrong adapter response rejectable without
/// consuming the receipt or advancing the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9Resume {
    Acknowledged {
        committed: OrdinaryType9AppliedState,
    },
    Blocked {
        reason: OrdinaryType9ExternalBlock,
    },
}

/// Terminal planning failure. No external action was issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9PlanningBlock {
    pub lease: OrdinaryType9FrameLease,
    pub initial: OrdinaryType9AppliedState,
    pub reason: OrdinaryType9FrameBlock,
}

/// Terminal retail-zero result. No external action was issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9ReturnedZero {
    pub lease: OrdinaryType9FrameLease,
    pub unchanged: OrdinaryType9AppliedState,
    pub reason: CommonMoverTargetPreludeZero,
}

/// Durable terminal adapter block.
///
/// The action receipt has been consumed. Reconstructing the transaction could
/// replay an adapter write whose outcome was already classified as blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9CommitBlock {
    pub lease: OrdinaryType9FrameLease,
    pub before: OrdinaryType9AppliedState,
    pub intended_after: OrdinaryType9AppliedState,
    pub step: OrdinaryType9FrameStep,
    pub reason: OrdinaryType9ExternalBlock,
}

/// Retail's successful nonzero callback return after the state commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9ReturnOne {
    pub lease: OrdinaryType9FrameLease,
    pub committed: OrdinaryType9AppliedState,
    pub step: OrdinaryType9FrameStep,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9Poll {
    Action(IssuedOrdinaryType9Action),
    Awaiting,
    PlanningBlocked(OrdinaryType9PlanningBlock),
    ReturnedZero(OrdinaryType9ReturnedZero),
    Blocked(OrdinaryType9CommitBlock),
    ReturnOne(OrdinaryType9ReturnOne),
}

/// Invalid adapter completion which leaves the outstanding action unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9ProtocolError {
    NoOutstandingAction {
        transaction_id: OrdinaryType9TransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: OrdinaryType9TransactionId,
        actual: OrdinaryType9TransactionId,
    },
    ReceiptLeaseMismatch {
        expected: OrdinaryType9FrameLease,
        actual: OrdinaryType9FrameLease,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    AcknowledgedStateMismatch {
        expected: OrdinaryType9AppliedState,
        actual: OrdinaryType9AppliedState,
    },
}

/// Recoverable rejection carrying the still-linear receipt back to its owner.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9ResumeFailure {
    pub receipt: OrdinaryType9Receipt,
    pub error: OrdinaryType9ProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrdinaryType9Stage {
    PlanningBlocked(OrdinaryType9PlanningBlock),
    ReturnedZero(OrdinaryType9ReturnedZero),
    CommitApplied {
        before: OrdinaryType9AppliedState,
        after: OrdinaryType9AppliedState,
        step: OrdinaryType9FrameStep,
    },
    Blocked(OrdinaryType9CommitBlock),
    ReturnOne(OrdinaryType9ReturnOne),
}

/// Detached one-action ordinary type-9 frame transaction.
///
/// Deliberately neither `Copy` nor `Clone`: duplicating it could issue the same
/// atomic frame replacement more than once.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9Transaction {
    transaction_id: OrdinaryType9TransactionId,
    lease: OrdinaryType9FrameLease,
    stage: OrdinaryType9Stage,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
}

impl OrdinaryType9Transaction {
    /// Plan the complete mover frame atomically on copied state.
    ///
    /// The terrain and other borrowed request inputs are consumed only during
    /// construction; no borrowed live state survives inside the transaction.
    pub fn start(
        transaction_id: OrdinaryType9TransactionId,
        lease: OrdinaryType9FrameLease,
        initial_frame: OrdinaryType9FrameState,
        request: OrdinaryType9FrameRequest<'_>,
        next_random: impl FnMut() -> u32,
    ) -> Self {
        Self::start_with_planner(
            transaction_id,
            lease,
            initial_frame,
            request,
            |state, request| advance_ordinary_type9_common_mover(state, request, next_random),
        )
    }

    /// Plan with a task-owner-authenticated Sub-I policy while retaining the
    /// same one-action receipt protocol. Existing callers remain neutral-only
    /// through [`Self::start`].
    pub(crate) fn start_with_animation_policy(
        transaction_id: OrdinaryType9TransactionId,
        lease: OrdinaryType9FrameLease,
        initial_frame: OrdinaryType9FrameState,
        request: OrdinaryType9FrameRequest<'_>,
        animation_policy: ActorAbdiAnimationPolicy,
        next_random: impl FnMut() -> u32,
    ) -> Self {
        Self::start_with_planner(
            transaction_id,
            lease,
            initial_frame,
            request,
            |state, request| {
                advance_ordinary_type9_common_mover_with_animation_policy(
                    state,
                    request,
                    animation_policy,
                    next_random,
                )
            },
        )
    }

    fn start_with_planner(
        transaction_id: OrdinaryType9TransactionId,
        lease: OrdinaryType9FrameLease,
        initial_frame: OrdinaryType9FrameState,
        request: OrdinaryType9FrameRequest<'_>,
        planner: impl FnOnce(
            &mut OrdinaryType9FrameState,
            OrdinaryType9FrameRequest<'_>,
        ) -> Result<OrdinaryType9FrameOutcome, OrdinaryType9FrameBlock>,
    ) -> Self {
        let initial = OrdinaryType9AppliedState {
            frame: initial_frame,
            wander: request.wander,
            heading_raw: request.heading_raw,
            velocity_raw: request.velocity_raw,
        };
        let mut staged_frame = initial_frame;
        let stage = match planner(&mut staged_frame, request) {
            Err(reason) => OrdinaryType9Stage::PlanningBlocked(OrdinaryType9PlanningBlock {
                lease,
                initial,
                reason,
            }),
            Ok(OrdinaryType9FrameOutcome::ReturnedZero(reason)) => {
                OrdinaryType9Stage::ReturnedZero(OrdinaryType9ReturnedZero {
                    lease,
                    unchanged: initial,
                    reason,
                })
            }
            Ok(OrdinaryType9FrameOutcome::Applied(step)) => {
                let after = OrdinaryType9AppliedState {
                    frame: staged_frame,
                    wander: step.wander,
                    heading_raw: step.heading_raw,
                    velocity_raw: step.velocity_raw,
                };
                OrdinaryType9Stage::CommitApplied {
                    before: initial,
                    after,
                    step,
                }
            }
        };
        Self {
            transaction_id,
            lease,
            stage,
            next_action_sequence: 1,
            issued_action_sequence: None,
        }
    }

    /// Issue the atomic commit once, or report the durable terminal state.
    pub fn poll(&mut self) -> OrdinaryType9Poll {
        if self.issued_action_sequence.is_some() {
            return OrdinaryType9Poll::Awaiting;
        }
        let (before, after, step) = match self.stage {
            OrdinaryType9Stage::PlanningBlocked(block) => {
                return OrdinaryType9Poll::PlanningBlocked(block);
            }
            OrdinaryType9Stage::ReturnedZero(result) => {
                return OrdinaryType9Poll::ReturnedZero(result);
            }
            OrdinaryType9Stage::CommitApplied {
                before,
                after,
                step,
            } => (before, after, step),
            OrdinaryType9Stage::Blocked(block) => {
                return OrdinaryType9Poll::Blocked(block);
            }
            OrdinaryType9Stage::ReturnOne(completion) => {
                return OrdinaryType9Poll::ReturnOne(completion);
            }
        };

        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one type-9 frame cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        OrdinaryType9Poll::Action(IssuedOrdinaryType9Action {
            receipt: OrdinaryType9Receipt {
                transaction_id: self.transaction_id,
                lease: self.lease,
                action_sequence,
            },
            action: OrdinaryType9Action::CommitApplied {
                lease: self.lease,
                before,
                after,
                step,
            },
        })
    }

    /// Complete the outstanding atomic commit.
    ///
    /// Protocol errors return the receipt intact and do not change the stage.
    /// A successful acknowledgement advances to retail return one. An adapter
    /// block consumes the receipt and becomes terminal.
    pub fn resume(
        &mut self,
        receipt: OrdinaryType9Receipt,
        response: OrdinaryType9Resume,
    ) -> Result<(), OrdinaryType9ResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            return Err(OrdinaryType9ResumeFailure {
                error: OrdinaryType9ProtocolError::NoOutstandingAction {
                    transaction_id: receipt.transaction_id,
                    action_sequence: receipt.action_sequence,
                },
                receipt,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(OrdinaryType9ResumeFailure {
                error: OrdinaryType9ProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.lease != self.lease {
            return Err(OrdinaryType9ResumeFailure {
                error: OrdinaryType9ProtocolError::ReceiptLeaseMismatch {
                    expected: self.lease,
                    actual: receipt.lease,
                },
                receipt,
            });
        }
        if receipt.action_sequence != expected_sequence {
            return Err(OrdinaryType9ResumeFailure {
                error: OrdinaryType9ProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }

        let OrdinaryType9Stage::CommitApplied {
            before,
            after,
            step,
        } = self.stage
        else {
            unreachable!("an outstanding receipt belongs to CommitApplied")
        };
        if let OrdinaryType9Resume::Acknowledged { committed } = response {
            if committed != after {
                return Err(OrdinaryType9ResumeFailure {
                    error: OrdinaryType9ProtocolError::AcknowledgedStateMismatch {
                        expected: after,
                        actual: committed,
                    },
                    receipt,
                });
            }
        }

        self.issued_action_sequence = None;
        self.stage = match response {
            OrdinaryType9Resume::Acknowledged { committed } => {
                OrdinaryType9Stage::ReturnOne(OrdinaryType9ReturnOne {
                    lease: self.lease,
                    committed,
                    step,
                })
            }
            OrdinaryType9Resume::Blocked { reason } => {
                OrdinaryType9Stage::Blocked(OrdinaryType9CommitBlock {
                    lease: self.lease,
                    before,
                    intended_after: after,
                    step,
                    reason,
                })
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_animation::ActorAnimationController;
    use crate::actor_task_owner::{
        ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, PreparedActorTask,
    };
    use crate::common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime, ORDINARY_TYPE9_SUB_D};
    use crate::common_mover::target_prelude::{
        CommonMoverTargetPreludeBlock, CommonMoverTrackedTargetSnapshot,
    };
    use crate::common_mover::type9::{OrdinaryType9Topology, ORDINARY_TYPE9_ENTITY_TYPE};
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    };
    use v2k_formats::collision::{
        ActorAnimationDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    };
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
        acceleration_raw: 1_500,
        overspeed_correction_raw: -3_000,
        target_speed_base_raw: 250,
    };
    const SUB_B: SubBLateralDescriptor = SubBLateralDescriptor {
        projection_threshold_rate_raw: 10_000,
        correction_rate_raw: 1_000,
    };
    const SUB_I: ActorAnimationDescriptor = ActorAnimationDescriptor {
        capability_bit_3_sound_id: 72,
        capability_mask_0x201_sound_id: 0,
        attention_stop_sound_id: 72,
        variable_binding: 1,
        frames_per_direction: 4,
    };

    fn transaction_id(raw: u64) -> OrdinaryType9TransactionId {
        OrdinaryType9TransactionId::new(NonZeroU64::new(raw).unwrap())
    }

    fn lease(controlled_entity_id: u32, task_ordinal: usize) -> OrdinaryType9FrameLease {
        let mut owner = ActorTaskOwner::new();
        let mut task_id =
            owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        for _ in 1..task_ordinal {
            task_id = owner.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        }
        OrdinaryType9FrameLease {
            controlled_entity_id,
            task_visit: ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            },
        }
    }

    fn topology() -> OrdinaryType9Topology {
        OrdinaryType9Topology::from_metadata(
            ORDINARY_TYPE9_ENTITY_TYPE,
            &EntityTypeRuntimeMetadata {
                sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(SUB_A)),
                sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(SUB_B)),
                sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D)),
                actor_animation_descriptor: RetailRuntimeValue::Known(Some(SUB_I)),
                common_mover_topology: RetailRuntimeValue::Known(CommonMoverComponentTopology {
                    sub_a: true,
                    sub_b: true,
                    sub_d: true,
                    sub_i: true,
                    ..CommonMoverComponentTopology::default()
                }),
                ..EntityTypeRuntimeMetadata::default()
            },
        )
        .unwrap()
    }

    fn flat_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [-1 << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn frame_state() -> OrdinaryType9FrameState {
        OrdinaryType9FrameState {
            sub_d_frame_owner: Type9SubDFrameOwner::from_retail_state([0; 8], [0, 0], 5),
            sub_d_runtime: Type9SubDRuntime::default(),
            sub_a_runtime: SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(250),
                1,
                100,
            ),
            actor_animation: ActorAnimationController::from_descriptor(SUB_I).unwrap(),
        }
    }

    fn request(terrain: &TerrainGrid) -> OrdinaryType9FrameRequest<'_> {
        OrdinaryType9FrameRequest {
            topology: topology(),
            wander: WanderNearPrivateState {
                target_position_raw: [0, 0, 100],
                tracked_entity_handle: 0,
                direction: 1,
                reversal_timer_ms: 0,
            },
            tracked_target: RetailRuntimeValue::Unresolved,
            terrain,
            position_raw: [0, 0, 0],
            pre_mover_right_q31: [0, 0, i32::MAX],
            pre_mover_forward_q31: [i32::MAX, 0, 0],
            heading_raw: 0x1000,
            velocity_raw: [0, 0, 0],
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            scheduler_mode: 0,
        }
    }

    fn applied_transaction() -> OrdinaryType9Transaction {
        let terrain = flat_terrain();
        OrdinaryType9Transaction::start(
            transaction_id(1),
            lease(0x04ac_0001, 1),
            frame_state(),
            request(&terrain),
            || panic!("the ordinary type-9 steady branch must not consume RNG"),
        )
    }

    fn issue(
        transaction: &mut OrdinaryType9Transaction,
    ) -> (
        OrdinaryType9Receipt,
        OrdinaryType9AppliedState,
        OrdinaryType9AppliedState,
        OrdinaryType9FrameStep,
    ) {
        let OrdinaryType9Poll::Action(IssuedOrdinaryType9Action {
            receipt,
            action:
                OrdinaryType9Action::CommitApplied {
                    before,
                    after,
                    step,
                    ..
                },
        }) = transaction.poll()
        else {
            panic!("expected CommitApplied")
        };
        (receipt, before, after, step)
    }

    #[test]
    fn applied_frame_issues_one_complete_commit_then_returns_one() {
        let initial = frame_state();
        let mut transaction = applied_transaction();
        let (receipt, before, after, step) = issue(&mut transaction);

        assert_eq!(before.frame, initial);
        assert_eq!(before.wander.target_position_raw, [0, 0, 100]);
        assert_eq!(before.heading_raw, 0x1000);
        assert_eq!(before.velocity_raw, [0, 0, 0]);
        assert_eq!(
            after.frame.actor_animation.output(),
            step.actor_animation_selector
        );
        assert_eq!(after.wander, step.wander);
        assert_eq!(after.heading_raw, step.heading_raw);
        assert_eq!(after.velocity_raw, step.velocity_raw);
        assert_ne!(after, before);
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Awaiting);

        transaction
            .resume(
                receipt,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap();
        let expected = OrdinaryType9ReturnOne {
            lease: lease(0x04ac_0001, 1),
            committed: after,
            step,
        };
        assert_eq!(transaction.poll(), OrdinaryType9Poll::ReturnOne(expected));
        assert_eq!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnOne(expected),
            "terminal polling cannot replay the commit"
        );
    }

    #[test]
    fn exploding_special_policy_uses_the_same_linear_commit_protocol() {
        let terrain = flat_terrain();
        let mut initial = frame_state();
        initial.actor_animation.apply_exploding_person_reset();
        let mut transaction = OrdinaryType9Transaction::start_with_animation_policy(
            transaction_id(11),
            lease(0x04ac_0011, 1),
            initial,
            request(&terrain),
            ActorAbdiAnimationPolicy::ExplodingPersonSpecial,
            || panic!("the exact special-mode mover branch consumes no RNG"),
        );
        let (receipt, before, after, step) = issue(&mut transaction);

        assert!(before.frame.actor_animation.special_mode());
        assert_eq!(before.frame.actor_animation.phase(), 0);
        assert_eq!(after.frame.actor_animation.phase(), 1);
        assert_eq!(after.frame.actor_animation.output(), 39);
        assert_eq!(step.actor_animation_selector, 39);
        transaction
            .resume(
                receipt,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap();
        assert!(matches!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnOne(_)
        ));
    }

    #[test]
    fn planning_block_is_terminal_and_never_issues_an_action() {
        let terrain = flat_terrain();
        let initial_frame = frame_state();
        let mut blocked_request = request(&terrain);
        blocked_request.wander.tracked_entity_handle = 0x04ac_0012;
        let expected_initial = OrdinaryType9AppliedState {
            frame: initial_frame,
            wander: blocked_request.wander,
            heading_raw: blocked_request.heading_raw,
            velocity_raw: blocked_request.velocity_raw,
        };
        let mut transaction = OrdinaryType9Transaction::start(
            transaction_id(2),
            lease(0x04ac_0002, 1),
            initial_frame,
            blocked_request,
            || panic!("an early planning block must not consume RNG"),
        );
        let expected = OrdinaryType9PlanningBlock {
            lease: lease(0x04ac_0002, 1),
            initial: expected_initial,
            reason: OrdinaryType9FrameBlock::TargetPrelude(
                CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
                    handle: 0x04ac_0012,
                },
            ),
        };
        assert_eq!(
            transaction.poll(),
            OrdinaryType9Poll::PlanningBlocked(expected)
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9Poll::PlanningBlocked(expected)
        );
    }

    #[test]
    fn retail_zero_is_terminal_and_never_issues_an_action() {
        let terrain = flat_terrain();
        let initial_frame = frame_state();
        let mut zero_request = request(&terrain);
        zero_request.wander.tracked_entity_handle = 0x04bd_0001;
        zero_request.tracked_target =
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw: [100, 200, 300],
                velocity_raw: [256, -256, 0],
            }));
        let expected_unchanged = OrdinaryType9AppliedState {
            frame: initial_frame,
            wander: zero_request.wander,
            heading_raw: zero_request.heading_raw,
            velocity_raw: zero_request.velocity_raw,
        };
        let mut transaction = OrdinaryType9Transaction::start(
            transaction_id(3),
            lease(0x04ac_0003, 1),
            initial_frame,
            zero_request,
            || panic!("the tracked-target return-zero branch must not consume RNG"),
        );
        let expected = OrdinaryType9ReturnedZero {
            lease: lease(0x04ac_0003, 1),
            unchanged: expected_unchanged,
            reason: CommonMoverTargetPreludeZero::TrackedEntityInactive,
        };
        assert_eq!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnedZero(expected)
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnedZero(expected)
        );
    }

    #[test]
    fn wrong_transaction_receipt_is_returned_without_progress() {
        let mut transaction = applied_transaction();
        let (receipt, _, after, _) = issue(&mut transaction);
        let OrdinaryType9Receipt {
            lease,
            action_sequence,
            ..
        } = receipt;
        let wrong = OrdinaryType9Receipt {
            transaction_id: transaction_id(99),
            lease,
            action_sequence,
        };

        let failure = transaction
            .resume(
                wrong,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9ProtocolError::ReceiptTransactionMismatch {
                expected: transaction_id(1),
                actual: transaction_id(99),
            }
        );
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Awaiting);
        let receipt = OrdinaryType9Receipt {
            transaction_id: transaction_id(1),
            lease: failure.receipt.lease,
            action_sequence: failure.receipt.action_sequence,
        };
        transaction
            .resume(
                receipt,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap();
        assert!(matches!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnOne(_)
        ));
    }

    #[test]
    fn wrong_lease_receipt_is_returned_without_progress() {
        let mut transaction = applied_transaction();
        let (receipt, _, after, _) = issue(&mut transaction);
        let OrdinaryType9Receipt {
            transaction_id,
            action_sequence,
            ..
        } = receipt;
        let wrong_lease = lease(0x04ac_0001, 2);
        let wrong = OrdinaryType9Receipt {
            transaction_id,
            lease: wrong_lease,
            action_sequence,
        };

        let failure = transaction
            .resume(
                wrong,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9ProtocolError::ReceiptLeaseMismatch {
                expected: lease(0x04ac_0001, 1),
                actual: wrong_lease,
            }
        );
        assert_eq!(failure.receipt.lease(), wrong_lease);
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Awaiting);
    }

    #[test]
    fn wrong_sequence_receipt_is_returned_without_progress() {
        let mut transaction = applied_transaction();
        let (receipt, _, after, _) = issue(&mut transaction);
        let OrdinaryType9Receipt {
            transaction_id,
            lease,
            action_sequence,
        } = receipt;
        let wrong = OrdinaryType9Receipt {
            transaction_id,
            lease,
            action_sequence: action_sequence + 1,
        };

        let failure = transaction
            .resume(
                wrong,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9ProtocolError::ReceiptSequenceMismatch {
                expected: 1,
                actual: 2,
            }
        );
        assert_eq!(failure.receipt.action_sequence(), 2);
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Awaiting);
    }

    #[test]
    fn wrong_acknowledgement_state_returns_receipt_without_progress() {
        let mut transaction = applied_transaction();
        let (receipt, _, after, _) = issue(&mut transaction);
        let mut wrong = after;
        wrong.heading_raw = wrong.heading_raw.wrapping_add(1);

        let failure = transaction
            .resume(
                receipt,
                OrdinaryType9Resume::Acknowledged { committed: wrong },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9ProtocolError::AcknowledgedStateMismatch {
                expected: after,
                actual: wrong,
            }
        );
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Awaiting);
        transaction
            .resume(
                failure.receipt,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap();
        assert!(matches!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnOne(_)
        ));
    }

    #[test]
    fn adapter_block_consumes_receipt_and_is_durable_terminal() {
        let mut transaction = applied_transaction();
        let (receipt, before, after, step) = issue(&mut transaction);
        transaction
            .resume(
                receipt,
                OrdinaryType9Resume::Blocked {
                    reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
                },
            )
            .unwrap();
        let expected = OrdinaryType9CommitBlock {
            lease: lease(0x04ac_0001, 1),
            before,
            intended_after: after,
            step,
            reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
        };
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Blocked(expected));
        assert_eq!(transaction.poll(), OrdinaryType9Poll::Blocked(expected));
    }

    #[test]
    fn completion_without_an_outstanding_action_returns_the_receipt() {
        let mut transaction = applied_transaction();
        let (receipt, _, after, _) = issue(&mut transaction);
        transaction
            .resume(
                receipt,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap();
        let fabricated = OrdinaryType9Receipt {
            transaction_id: transaction_id(1),
            lease: lease(0x04ac_0001, 1),
            action_sequence: 1,
        };
        let failure = transaction
            .resume(
                fabricated,
                OrdinaryType9Resume::Acknowledged { committed: after },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9ProtocolError::NoOutstandingAction {
                transaction_id: transaction_id(1),
                action_sequence: 1,
            }
        );
        assert_eq!(failure.receipt.action_sequence(), 1);
        assert!(matches!(
            transaction.poll(),
            OrdinaryType9Poll::ReturnOne(_)
        ));
    }
}
