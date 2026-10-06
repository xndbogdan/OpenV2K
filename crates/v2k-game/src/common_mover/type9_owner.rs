//! Detached ordinary type-9 outer callback and master-motion owner.
//!
//! Retail keeps the task/common-mover callback, the normal callback
//! (`FUN_0040E870`), and the master owner (`FUN_00412DA0`) as distinct
//! mutation boundaries.  This transaction starts from the fresh state sampled
//! after task dispatch and preserves the remaining order for ordinary Type 9:
//!
//! 1. direct body-basis rebuild (`FUN_00413F70`);
//! 2. environment drag (`FUN_0040E100`);
//! 3. ground snap at the old X/Z (`FUN_0040DF70`);
//! 4. the type-9 surface/lifetime owner (`FUN_0040E370`);
//! 5. master motion (`FUN_00412DA0`).
//!
//! `FUN_0040E870` latches ordinary Type 9's effective flags as `0x2f`
//! before task dispatch. Bit `0x10` is clear, so this path must not invoke
//! `FUN_0040E640` or alter the post-task pitch/roll words before the rebuild.
//!
//! Shared RNG is not consumed during construction. The machine commits the
//! no-RNG surface timer classification after the basis and drag/snap commits,
//! then journals the bubble and independent sound draws separately. Thus a
//! blocked timer commit cannot advance the stream, and a bubble allocation is
//! acknowledged before the sound word is consumed.
//!
//! Particle, sound, and lifecycle work remains adapter-owned.  Every such
//! effect is receipt-bound and returns a fresh entity snapshot before the next
//! phase.  The lifecycle action means exactly retail's synchronous
//! `FUN_00416750` then `FUN_00410C10` pair; it is deliberately one journaled
//! action so either call cannot be replayed independently after a lost
//! acknowledgement.  This closes detached ownership only.  It does not attach
//! ordinary actors to the live scheduler or manufacture the missing entity,
//! component, RNG, effect, and lifecycle adapters.

use std::num::NonZeroU64;

use v2k_formats::terrain::TerrainGrid;

use super::type9::ORDINARY_TYPE9_ENTITY_TYPE;
use super::type9_attitude::Type9BodyBasis;
use super::type9_surface::{
    classify_actor_surface_timer_phase, plan_actor_surface_bubble,
    plan_ordinary_type9_surface_sound, ActorSurfaceBubbleFrame, ActorSurfaceBubbleRequest,
    ActorSurfacePlanError, ActorSurfaceTimerFrame, ActorSurfaceTimerPhase, Type9LifecycleRequest,
    Type9SurfaceFrame, Type9SurfaceRuntime, Type9SurfaceSoundRequest,
};
use super::type9_tail::{
    apply_common_master_motion_raw, apply_type9_environment_drag_raw, apply_type9_ground_snap_raw,
};
use crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT;

/// Caller-owned identity for one live detached owner transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrdinaryType9OwnerTransactionId(NonZeroU64);

impl OrdinaryType9OwnerTransactionId {
    pub const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Stable identity of the entity frame controlled by this transaction.
///
/// `frame_token` is allocated by the adapter.  It must change when an entity
/// allocation is recycled or a new outer callback starts, even when the
/// 32-bit retail handle happens to be reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrdinaryType9OwnerLease {
    pub controlled_entity_id: u32,
    pub frame_token: NonZeroU64,
}

/// Complete mutable state re-read between detached owner phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9OwnerState {
    pub state_flags: u32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: i16,
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub body_basis: Type9BodyBasis,
    pub surface_runtime: Type9SurfaceRuntime,
}

/// Runtime-owned scalar inputs which remain stable for one outer frame.
#[derive(Debug, Clone, Copy)]
pub struct OrdinaryType9OwnerFrame<'a> {
    pub terrain: &'a TerrainGrid,
    pub effective_elapsed_micros: u32,
    pub mode_zero_drag: super::type9_tail::Type9ModeZeroDrag,
    pub active_model_extent_raw: u16,
    pub flat_surface_y_raw: i16,
    pub authored_lifetime_ms: u32,
}

/// Atomic admission failure for the entry after an already-published
/// `FUN_00413F70` body basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerStartAfterBodyBasisError {
    BodyBasisRebuiltStateBitClear {
        actual_state_flags: u32,
    },
    BodyBasisDoesNotMatchPostTaskAngles {
        expected: Type9BodyBasis,
        actual: Type9BodyBasis,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrdinaryType9OwnerContext {
    effective_elapsed_micros: u32,
    active_model_extent_raw: u16,
    flat_surface_y_raw: i16,
    authored_lifetime_ms: u32,
    tail_after_basis: OrdinaryType9OwnerState,
}

impl OrdinaryType9OwnerContext {
    fn from_frame(
        frame: OrdinaryType9OwnerFrame<'_>,
        tail_after_basis: OrdinaryType9OwnerState,
    ) -> Self {
        Self {
            effective_elapsed_micros: frame.effective_elapsed_micros,
            active_model_extent_raw: frame.active_model_extent_raw,
            flat_surface_y_raw: frame.flat_surface_y_raw,
            authored_lifetime_ms: frame.authored_lifetime_ms,
            tail_after_basis,
        }
    }
}

/// Exact state-write boundary in the outer callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerCommitPhase {
    BodyBasis,
    EnvironmentDragAndOldPositionSnap,
    SurfaceRuntime,
    MasterMotion,
}

/// One external operation in retail order.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9OwnerAction {
    CommitState {
        phase: OrdinaryType9OwnerCommitPhase,
        before: OrdinaryType9OwnerState,
        after: OrdinaryType9OwnerState,
    },
    /// Consume one receipt-bounded surface RNG phase through
    /// [`OrdinaryType9OwnerTransaction::plan_surface`].
    PlanSurface {
        phase: OrdinaryType9OwnerSurfacePlanPhase,
        frame: Type9SurfaceFrame,
    },
    SpawnBubble(ActorSurfaceBubbleRequest),
    PlaySound(Type9SurfaceSoundRequest),
    /// Run `FUN_00416750` then `FUN_00410C10` synchronously and return a
    /// fresh snapshot of the surviving/rebound allocation.
    RunLifecycleContinuation(Type9LifecycleRequest),
}

/// The two process-RNG phases in retail E370 order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerSurfacePlanPhase {
    Bubble { remaining_percent: u32 },
    Sound,
}

/// Linear receipt for one outstanding action.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9OwnerReceipt {
    transaction_id: OrdinaryType9OwnerTransactionId,
    lease: OrdinaryType9OwnerLease,
    action_sequence: u64,
    action_kind: OrdinaryType9OwnerActionKind,
}

impl OrdinaryType9OwnerReceipt {
    pub const fn transaction_id(&self) -> OrdinaryType9OwnerTransactionId {
        self.transaction_id
    }

    pub const fn lease(&self) -> OrdinaryType9OwnerLease {
        self.lease
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }

    fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            transaction_id: self.transaction_id,
            lease: self.lease,
            action_sequence: self.action_sequence,
            action_kind: self.action_kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerActionKind {
    Commit(OrdinaryType9OwnerCommitPhase),
    PlanSurface,
    SpawnBubble,
    PlaySound,
    Lifecycle,
}

impl OrdinaryType9OwnerAction {
    const fn kind(&self) -> OrdinaryType9OwnerActionKind {
        match self {
            Self::CommitState { phase, .. } => OrdinaryType9OwnerActionKind::Commit(*phase),
            Self::PlanSurface { .. } => OrdinaryType9OwnerActionKind::PlanSurface,
            Self::SpawnBubble(_) => OrdinaryType9OwnerActionKind::SpawnBubble,
            Self::PlaySound(_) => OrdinaryType9OwnerActionKind::PlaySound,
            Self::RunLifecycleContinuation(_) => OrdinaryType9OwnerActionKind::Lifecycle,
        }
    }

    /// Materialize the one value handed to the adapter while the machine keeps
    /// the private stage description needed to validate its response.
    ///
    /// This is intentionally private rather than a `Clone` implementation:
    /// [`OrdinaryType9OwnerTransaction::poll`] calls it only after proving no
    /// receipt is outstanding.
    fn issue_value(&self) -> Self {
        match *self {
            Self::CommitState {
                phase,
                before,
                after,
            } => Self::CommitState {
                phase,
                before,
                after,
            },
            Self::PlanSurface { phase, frame } => Self::PlanSurface { phase, frame },
            Self::SpawnBubble(request) => Self::SpawnBubble(request),
            Self::PlaySound(request) => Self::PlaySound(request),
            Self::RunLifecycleContinuation(request) => Self::RunLifecycleContinuation(request),
        }
    }

    fn fork_for_main_base_abort_transaction(&self) -> Self {
        self.issue_value()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IssuedOrdinaryType9OwnerAction {
    pub receipt: OrdinaryType9OwnerReceipt,
    pub action: OrdinaryType9OwnerAction,
}

impl IssuedOrdinaryType9OwnerAction {
    /// Preserve an already-issued action only while speculatively forking the
    /// scheduler for a Main Base transaction that may abort.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            receipt: self.receipt.fork_for_main_base_abort_transaction(),
            action: self.action.fork_for_main_base_abort_transaction(),
        }
    }
}

/// Durable adapter failure before an action, or a post-action observation
/// failure whose execution state is recorded explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerExternalBlock {
    StateCommitUnavailable,
    SurfaceRngUnavailable,
    BubbleAllocatorUnavailable,
    PositionalSoundUnavailable,
    LifecycleContinuationUnavailable,
    PostLifecycleEntityUnavailable,
    InvalidSurfaceConfiguration(ActorSurfacePlanError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9OwnerBlock {
    pub lease: OrdinaryType9OwnerLease,
    pub action_kind: OrdinaryType9OwnerActionKind,
    pub state_before_action: OrdinaryType9OwnerState,
    pub reason: OrdinaryType9OwnerExternalBlock,
    /// True only when the action already completed and must never be replayed.
    pub action_executed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9OwnerCompletion {
    pub lease: OrdinaryType9OwnerLease,
    pub state: OrdinaryType9OwnerState,
    pub callback_result: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9OwnerPoll {
    Action(IssuedOrdinaryType9OwnerAction),
    Awaiting,
    Complete(OrdinaryType9OwnerCompletion),
    Blocked(OrdinaryType9OwnerBlock),
}

/// Accepted adapter response for state/effect/lifecycle actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerResume {
    StateCommitted {
        committed: OrdinaryType9OwnerState,
    },
    EffectCompleted {
        fresh_state: OrdinaryType9OwnerState,
    },
    LifecycleCompleted {
        fresh_state: Option<OrdinaryType9OwnerState>,
    },
    Blocked {
        reason: OrdinaryType9OwnerExternalBlock,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9OwnerProtocolError {
    NoOutstandingAction,
    ReceiptTransactionMismatch {
        expected: OrdinaryType9OwnerTransactionId,
        actual: OrdinaryType9OwnerTransactionId,
    },
    ReceiptLeaseMismatch {
        expected: OrdinaryType9OwnerLease,
        actual: OrdinaryType9OwnerLease,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    ReceiptActionMismatch,
    SurfacePlannerRequired,
    SurfacePlannerNotExpected,
    WrongResponseForAction,
    AcknowledgedStateMismatch {
        expected: OrdinaryType9OwnerState,
        actual: OrdinaryType9OwnerState,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9OwnerResumeFailure {
    pub error: OrdinaryType9OwnerProtocolError,
    pub receipt: OrdinaryType9OwnerReceipt,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdinaryType9OwnerStage {
    Action {
        action: OrdinaryType9OwnerAction,
        state_before_action: OrdinaryType9OwnerState,
        surface_continuation: Option<OrdinaryType9OwnerSurfaceContinuation>,
    },
    Complete(OrdinaryType9OwnerCompletion),
    Blocked(OrdinaryType9OwnerBlock),
}

impl OrdinaryType9OwnerStage {
    fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::Action {
                action,
                state_before_action,
                surface_continuation,
            } => Self::Action {
                action: action.fork_for_main_base_abort_transaction(),
                state_before_action: *state_before_action,
                surface_continuation: *surface_continuation,
            },
            Self::Complete(completion) => Self::Complete(*completion),
            Self::Blocked(block) => Self::Blocked(*block),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrdinaryType9OwnerSurfaceContinuation {
    Complete,
    RandomEffects {
        remaining_percent: u32,
    },
    Lifecycle {
        request: Type9LifecycleRequest,
        remaining_percent_after_lifecycle: u32,
    },
    AfterLifecycle {
        remaining_percent: u32,
    },
}

/// Receipt-bound detached outer owner.
///
/// This type is deliberately not `Clone`: duplicating an awaiting machine
/// would duplicate the right to acknowledge an external side effect.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9OwnerTransaction {
    transaction_id: OrdinaryType9OwnerTransactionId,
    lease: OrdinaryType9OwnerLease,
    context: OrdinaryType9OwnerContext,
    stage: OrdinaryType9OwnerStage,
    next_action_sequence: u64,
    outstanding: Option<OrdinaryType9OwnerOutstanding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrdinaryType9OwnerOutstanding {
    transaction_id: OrdinaryType9OwnerTransactionId,
    lease: OrdinaryType9OwnerLease,
    action_sequence: u64,
    action_kind: OrdinaryType9OwnerActionKind,
}

enum OrdinaryType9OwnerStartPhase {
    PublishBodyBasis { before: OrdinaryType9OwnerState },
    ContinueAfterBodyBasis,
}

impl OrdinaryType9OwnerTransaction {
    /// Duplicate this private owner solely for the scheduler snapshot used by
    /// a speculative Main Base transaction. If that transaction aborts, the
    /// snapshot must retain the exact outstanding receipt and continuation.
    ///
    /// This deliberately remains crate-private and narrowly named instead of
    /// making detached owner transactions generally cloneable.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            transaction_id: self.transaction_id,
            lease: self.lease,
            context: self.context,
            stage: self.stage.fork_for_main_base_abort_transaction(),
            next_action_sequence: self.next_action_sequence,
            outstanding: self.outstanding,
        }
    }

    /// Start immediately after the task/common-mover callback's fresh state
    /// sample. Construction performs no RNG draw and no external action. Do
    /// not use this entry after a production owner has already published F70;
    /// use [`Self::start_after_body_basis`] for that retained boundary.
    pub fn start(
        transaction_id: OrdinaryType9OwnerTransactionId,
        lease: OrdinaryType9OwnerLease,
        initial: OrdinaryType9OwnerState,
        frame: OrdinaryType9OwnerFrame<'_>,
    ) -> Self {
        let mut after = initial;
        // Retail E870 latches effective flags before A800. Ordinary Type 9's
        // exact 0x2f value has no 0x10 attitude gate, so F70 consumes the
        // post-task angle words verbatim.
        after.body_basis = Type9BodyBasis::from_angle_words(
            initial.heading_raw,
            initial.pitch_raw,
            initial.roll_raw,
        );
        // FUN_00413F70 publishes the complete matrix and then its retained
        // completion bit before E100, E370, or master motion can observe it.
        after.state_flags |= BODY_BASIS_REBUILT_STATE_BIT;
        Self::start_from_body_basis_state(
            transaction_id,
            lease,
            after,
            frame,
            OrdinaryType9OwnerStartPhase::PublishBodyBasis { before: initial },
        )
    }

    /// Resume the exact outer callback after the production owner has already
    /// published `FUN_00413F70` and its retained completion bit.
    ///
    /// Admission is atomic: the complete live basis must still equal the
    /// matrix derived from the retained post-task angles and state bit `0x4`
    /// must still be set.  A successful entry issues E100/DF70 first; it never
    /// emits another body-basis commit.  The supplied lease remains bound to
    /// every later receipt through the existing transaction protocol.
    pub fn start_after_body_basis(
        transaction_id: OrdinaryType9OwnerTransactionId,
        lease: OrdinaryType9OwnerLease,
        initial: OrdinaryType9OwnerState,
        frame: OrdinaryType9OwnerFrame<'_>,
    ) -> Result<Self, OrdinaryType9OwnerStartAfterBodyBasisError> {
        if initial.state_flags & BODY_BASIS_REBUILT_STATE_BIT == 0 {
            return Err(
                OrdinaryType9OwnerStartAfterBodyBasisError::BodyBasisRebuiltStateBitClear {
                    actual_state_flags: initial.state_flags,
                },
            );
        }
        let expected = Type9BodyBasis::from_angle_words(
            initial.heading_raw,
            initial.pitch_raw,
            initial.roll_raw,
        );
        if initial.body_basis != expected {
            return Err(
                OrdinaryType9OwnerStartAfterBodyBasisError::BodyBasisDoesNotMatchPostTaskAngles {
                    expected,
                    actual: initial.body_basis,
                },
            );
        }
        Ok(Self::start_from_body_basis_state(
            transaction_id,
            lease,
            initial,
            frame,
            OrdinaryType9OwnerStartPhase::ContinueAfterBodyBasis,
        ))
    }

    fn start_from_body_basis_state(
        transaction_id: OrdinaryType9OwnerTransactionId,
        lease: OrdinaryType9OwnerLease,
        body_basis_state: OrdinaryType9OwnerState,
        frame: OrdinaryType9OwnerFrame<'_>,
        phase: OrdinaryType9OwnerStartPhase,
    ) -> Self {
        let mut tail_after_basis = body_basis_state;
        apply_type9_environment_drag_raw(
            &mut tail_after_basis.velocity_raw,
            frame.effective_elapsed_micros,
            frame.mode_zero_drag,
        );
        apply_type9_ground_snap_raw(
            &mut tail_after_basis.position_raw,
            &mut tail_after_basis.velocity_raw,
            &mut tail_after_basis.state_flags,
            frame.terrain,
        );

        let (action, state_before_action) = match phase {
            OrdinaryType9OwnerStartPhase::PublishBodyBasis { before } => (
                OrdinaryType9OwnerAction::CommitState {
                    phase: OrdinaryType9OwnerCommitPhase::BodyBasis,
                    before,
                    after: body_basis_state,
                },
                before,
            ),
            OrdinaryType9OwnerStartPhase::ContinueAfterBodyBasis => (
                OrdinaryType9OwnerAction::CommitState {
                    phase: OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
                    before: body_basis_state,
                    after: tail_after_basis,
                },
                body_basis_state,
            ),
        };

        Self {
            transaction_id,
            lease,
            context: OrdinaryType9OwnerContext::from_frame(frame, tail_after_basis),
            stage: OrdinaryType9OwnerStage::Action {
                action,
                state_before_action,
                surface_continuation: None,
            },
            next_action_sequence: 1,
            outstanding: None,
        }
    }

    pub fn poll(&mut self) -> OrdinaryType9OwnerPoll {
        match &self.stage {
            OrdinaryType9OwnerStage::Complete(completion) => {
                OrdinaryType9OwnerPoll::Complete(*completion)
            }
            OrdinaryType9OwnerStage::Blocked(block) => OrdinaryType9OwnerPoll::Blocked(*block),
            OrdinaryType9OwnerStage::Action { action, .. } => {
                if self.outstanding.is_some() {
                    return OrdinaryType9OwnerPoll::Awaiting;
                }
                let receipt = OrdinaryType9OwnerReceipt {
                    transaction_id: self.transaction_id,
                    lease: self.lease,
                    action_sequence: self.next_action_sequence,
                    action_kind: action.kind(),
                };
                self.next_action_sequence = self.next_action_sequence.wrapping_add(1);
                self.outstanding = Some(OrdinaryType9OwnerOutstanding {
                    transaction_id: receipt.transaction_id,
                    lease: receipt.lease,
                    action_sequence: receipt.action_sequence,
                    action_kind: receipt.action_kind,
                });
                OrdinaryType9OwnerPoll::Action(IssuedOrdinaryType9OwnerAction {
                    receipt,
                    action: action.issue_value(),
                })
            }
        }
    }

    /// Consume the process-shared RNG at one exact surface phase.
    ///
    /// Bubble planning and the independent sound gate have distinct receipts;
    /// an accepted bubble is completed before the sound receipt is issued.
    pub fn plan_surface(
        &mut self,
        receipt: OrdinaryType9OwnerReceipt,
        mut next_random: impl FnMut() -> u32,
    ) -> Result<(), OrdinaryType9OwnerResumeFailure> {
        let receipt = self.validate_receipt(receipt)?;
        let (phase, frame, state_before_action) = match &self.stage {
            OrdinaryType9OwnerStage::Action {
                action: OrdinaryType9OwnerAction::PlanSurface { phase, frame },
                state_before_action,
                ..
            } => (*phase, *frame, *state_before_action),
            _ => {
                return Err(OrdinaryType9OwnerResumeFailure {
                    error: OrdinaryType9OwnerProtocolError::SurfacePlannerNotExpected,
                    receipt,
                });
            }
        };

        self.outstanding = None;
        match phase {
            OrdinaryType9OwnerSurfacePlanPhase::Bubble { remaining_percent } => {
                let request = plan_actor_surface_bubble(
                    remaining_percent,
                    ActorSurfaceBubbleFrame {
                        entity_id: frame.entity_id,
                        entity_type: frame.entity_type,
                        state_flags: frame.state_flags,
                        position_raw: frame.position_raw,
                        emission_axis_q31: frame.emission_axis_q31,
                        active_model_extent_raw: frame.active_model_extent_raw,
                    },
                    &mut next_random,
                );
                if let Some(request) = request {
                    self.stage = OrdinaryType9OwnerStage::Action {
                        action: OrdinaryType9OwnerAction::SpawnBubble(request),
                        state_before_action,
                        surface_continuation: None,
                    };
                } else {
                    self.stage_surface_plan(
                        state_before_action,
                        OrdinaryType9OwnerSurfacePlanPhase::Sound,
                    );
                }
            }
            OrdinaryType9OwnerSurfacePlanPhase::Sound => {
                let request = plan_ordinary_type9_surface_sound(frame.into(), &mut next_random);
                if let Some(request) = request {
                    self.stage = OrdinaryType9OwnerStage::Action {
                        action: OrdinaryType9OwnerAction::PlaySound(request),
                        state_before_action,
                        surface_continuation: None,
                    };
                } else {
                    self.stage_master_motion(state_before_action, 0);
                }
            }
        }
        Ok(())
    }

    pub fn resume(
        &mut self,
        receipt: OrdinaryType9OwnerReceipt,
        response: OrdinaryType9OwnerResume,
    ) -> Result<(), OrdinaryType9OwnerResumeFailure> {
        let receipt = self.validate_receipt(receipt)?;
        let (action, state_before_action, surface_continuation) = match &self.stage {
            OrdinaryType9OwnerStage::Action {
                action,
                state_before_action,
                surface_continuation,
            } => (
                action.issue_value(),
                *state_before_action,
                *surface_continuation,
            ),
            _ => unreachable!("an outstanding receipt belongs to an action"),
        };

        if let OrdinaryType9OwnerResume::Blocked { reason } = response {
            self.outstanding = None;
            self.stage = OrdinaryType9OwnerStage::Blocked(OrdinaryType9OwnerBlock {
                lease: self.lease,
                action_kind: action.kind(),
                state_before_action,
                reason,
                action_executed: false,
            });
            return Ok(());
        }
        if matches!(&action, OrdinaryType9OwnerAction::PlanSurface { .. }) {
            return Err(OrdinaryType9OwnerResumeFailure {
                error: OrdinaryType9OwnerProtocolError::SurfacePlannerRequired,
                receipt,
            });
        }

        let fresh_state = match (&action, response) {
            (
                OrdinaryType9OwnerAction::CommitState { after, .. },
                OrdinaryType9OwnerResume::StateCommitted { committed },
            ) => {
                if committed != *after {
                    return Err(OrdinaryType9OwnerResumeFailure {
                        error: OrdinaryType9OwnerProtocolError::AcknowledgedStateMismatch {
                            expected: *after,
                            actual: committed,
                        },
                        receipt,
                    });
                }
                committed
            }
            (
                OrdinaryType9OwnerAction::SpawnBubble(_) | OrdinaryType9OwnerAction::PlaySound(_),
                OrdinaryType9OwnerResume::EffectCompleted { fresh_state },
            ) => fresh_state,
            (
                OrdinaryType9OwnerAction::RunLifecycleContinuation(_),
                OrdinaryType9OwnerResume::LifecycleCompleted {
                    fresh_state: Some(fresh_state),
                },
            ) => fresh_state,
            (
                OrdinaryType9OwnerAction::RunLifecycleContinuation(_),
                OrdinaryType9OwnerResume::LifecycleCompleted { fresh_state: None },
            ) => {
                self.outstanding = None;
                self.stage = OrdinaryType9OwnerStage::Blocked(OrdinaryType9OwnerBlock {
                    lease: self.lease,
                    action_kind: action.kind(),
                    state_before_action,
                    reason: OrdinaryType9OwnerExternalBlock::PostLifecycleEntityUnavailable,
                    action_executed: true,
                });
                return Ok(());
            }
            _ => {
                return Err(OrdinaryType9OwnerResumeFailure {
                    error: OrdinaryType9OwnerProtocolError::WrongResponseForAction,
                    receipt,
                });
            }
        };

        self.outstanding = None;
        self.advance_after_action(action, fresh_state, surface_continuation);
        Ok(())
    }

    fn validate_receipt(
        &self,
        receipt: OrdinaryType9OwnerReceipt,
    ) -> Result<OrdinaryType9OwnerReceipt, OrdinaryType9OwnerResumeFailure> {
        let Some(expected) = self.outstanding else {
            return Err(OrdinaryType9OwnerResumeFailure {
                error: OrdinaryType9OwnerProtocolError::NoOutstandingAction,
                receipt,
            });
        };
        let error = if receipt.transaction_id != expected.transaction_id {
            Some(
                OrdinaryType9OwnerProtocolError::ReceiptTransactionMismatch {
                    expected: expected.transaction_id,
                    actual: receipt.transaction_id,
                },
            )
        } else if receipt.lease != expected.lease {
            Some(OrdinaryType9OwnerProtocolError::ReceiptLeaseMismatch {
                expected: expected.lease,
                actual: receipt.lease,
            })
        } else if receipt.action_sequence != expected.action_sequence {
            Some(OrdinaryType9OwnerProtocolError::ReceiptSequenceMismatch {
                expected: expected.action_sequence,
                actual: receipt.action_sequence,
            })
        } else if receipt.action_kind != expected.action_kind {
            Some(OrdinaryType9OwnerProtocolError::ReceiptActionMismatch)
        } else {
            None
        };
        match error {
            Some(error) => Err(OrdinaryType9OwnerResumeFailure { error, receipt }),
            None => Ok(receipt),
        }
    }

    fn advance_after_action(
        &mut self,
        completed_action: OrdinaryType9OwnerAction,
        fresh_state: OrdinaryType9OwnerState,
        surface_continuation: Option<OrdinaryType9OwnerSurfaceContinuation>,
    ) {
        match completed_action {
            OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::BodyBasis,
                ..
            } => {
                self.stage = OrdinaryType9OwnerStage::Action {
                    action: OrdinaryType9OwnerAction::CommitState {
                        phase: OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
                        before: fresh_state,
                        after: self.context.tail_after_basis,
                    },
                    state_before_action: fresh_state,
                    surface_continuation: None,
                };
            }
            OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
                ..
            } => {
                self.stage_surface_runtime_commit(fresh_state);
            }
            OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
                ..
            } => match surface_continuation.expect("surface continuation") {
                OrdinaryType9OwnerSurfaceContinuation::Complete => {
                    self.stage_master_motion(fresh_state, 0);
                }
                OrdinaryType9OwnerSurfaceContinuation::RandomEffects { remaining_percent } => self
                    .stage_surface_plan(
                        fresh_state,
                        OrdinaryType9OwnerSurfacePlanPhase::Bubble { remaining_percent },
                    ),
                OrdinaryType9OwnerSurfaceContinuation::Lifecycle {
                    request,
                    remaining_percent_after_lifecycle,
                } => {
                    self.stage = OrdinaryType9OwnerStage::Action {
                        action: OrdinaryType9OwnerAction::RunLifecycleContinuation(request),
                        state_before_action: fresh_state,
                        surface_continuation: Some(
                            OrdinaryType9OwnerSurfaceContinuation::AfterLifecycle {
                                remaining_percent: remaining_percent_after_lifecycle,
                            },
                        ),
                    };
                }
                OrdinaryType9OwnerSurfaceContinuation::AfterLifecycle { .. } => {
                    unreachable!("post-lifecycle continuation belongs to the lifecycle action")
                }
            },
            OrdinaryType9OwnerAction::SpawnBubble(_) => {
                self.stage_surface_plan(fresh_state, OrdinaryType9OwnerSurfacePlanPhase::Sound);
            }
            OrdinaryType9OwnerAction::PlaySound(_) => {
                self.stage_master_motion(fresh_state, 0);
            }
            OrdinaryType9OwnerAction::RunLifecycleContinuation(_) => {
                let OrdinaryType9OwnerSurfaceContinuation::AfterLifecycle { remaining_percent } =
                    surface_continuation.expect("post-lifecycle continuation")
                else {
                    unreachable!("lifecycle action must retain its E370 continuation")
                };
                if remaining_percent <= 74 {
                    self.stage_surface_plan(
                        fresh_state,
                        OrdinaryType9OwnerSurfacePlanPhase::Bubble { remaining_percent },
                    );
                } else {
                    self.stage_master_motion(fresh_state, 0);
                }
            }
            OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::MasterMotion,
                ..
            } => {
                self.stage = OrdinaryType9OwnerStage::Complete(OrdinaryType9OwnerCompletion {
                    lease: self.lease,
                    state: fresh_state,
                    callback_result: 0,
                });
            }
            OrdinaryType9OwnerAction::PlanSurface { .. } => {
                unreachable!("surface planning has its own resume method")
            }
        }
    }

    fn stage_surface_runtime_commit(&mut self, before: OrdinaryType9OwnerState) {
        let frame = self.surface_frame(before);
        let phase = classify_actor_surface_timer_phase(
            before.surface_runtime.lifetime_timer_ms_at_0x48,
            ActorSurfaceTimerFrame {
                state_flags: frame.state_flags,
                position_y_raw: frame.position_raw[1],
                active_model_extent_raw: frame.active_model_extent_raw,
                flat_surface_y_raw: frame.flat_surface_y_raw,
                elapsed_us: frame.elapsed_us,
                authored_lifetime_ms: frame.authored_lifetime_ms,
            },
        );
        let (timer_after_ms, continuation) = match phase {
            Ok(ActorSurfaceTimerPhase::OwnerDisabled) => (
                before.surface_runtime.lifetime_timer_ms_at_0x48,
                OrdinaryType9OwnerSurfaceContinuation::Complete,
            ),
            Ok(ActorSurfaceTimerPhase::NonDeep { timer_after_ms })
            | Ok(ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. }) => (
                timer_after_ms,
                OrdinaryType9OwnerSurfaceContinuation::Complete,
            ),
            Ok(ActorSurfaceTimerPhase::DeepRandomEffects {
                timer_after_ms,
                remaining_percent,
            }) => (
                timer_after_ms,
                OrdinaryType9OwnerSurfaceContinuation::RandomEffects { remaining_percent },
            ),
            Ok(ActorSurfaceTimerPhase::DeepLifecycle {
                timer_after_ms,
                remaining_percent_after_lifecycle,
            }) => (
                timer_after_ms,
                OrdinaryType9OwnerSurfaceContinuation::Lifecycle {
                    request: Type9LifecycleRequest {
                        entity_id: self.lease.controlled_entity_id,
                    },
                    remaining_percent_after_lifecycle,
                },
            ),
            Err(error) => {
                self.stage = OrdinaryType9OwnerStage::Blocked(OrdinaryType9OwnerBlock {
                    lease: self.lease,
                    action_kind: OrdinaryType9OwnerActionKind::Commit(
                        OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
                    ),
                    state_before_action: before,
                    reason: surface_plan_error_block(error),
                    action_executed: false,
                });
                return;
            }
        };
        let mut after = before;
        after.surface_runtime.lifetime_timer_ms_at_0x48 = timer_after_ms;
        self.stage = OrdinaryType9OwnerStage::Action {
            action: OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
                before,
                after,
            },
            state_before_action: before,
            surface_continuation: Some(continuation),
        };
    }

    fn stage_surface_plan(
        &mut self,
        state: OrdinaryType9OwnerState,
        phase: OrdinaryType9OwnerSurfacePlanPhase,
    ) {
        self.stage = OrdinaryType9OwnerStage::Action {
            action: OrdinaryType9OwnerAction::PlanSurface {
                phase,
                frame: self.surface_frame(state),
            },
            state_before_action: state,
            surface_continuation: None,
        };
    }

    fn surface_frame(&self, state: OrdinaryType9OwnerState) -> Type9SurfaceFrame {
        Type9SurfaceFrame {
            entity_id: self.lease.controlled_entity_id,
            entity_type: ORDINARY_TYPE9_ENTITY_TYPE as u8,
            state_flags: state.state_flags,
            position_raw: state.position_raw,
            emission_axis_q31: state.body_basis.forward,
            active_model_extent_raw: self.context.active_model_extent_raw,
            flat_surface_y_raw: self.context.flat_surface_y_raw,
            elapsed_us: self.context.effective_elapsed_micros,
            authored_lifetime_ms: self.context.authored_lifetime_ms,
        }
    }

    fn stage_master_motion(&mut self, before: OrdinaryType9OwnerState, callback_result: u32) {
        let mut after = before;
        let callback_result = apply_common_master_motion_raw(
            &mut after.position_raw,
            &mut after.velocity_raw,
            &mut after.state_flags,
            self.context.effective_elapsed_micros,
            callback_result,
        );
        self.stage = OrdinaryType9OwnerStage::Action {
            action: OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::MasterMotion,
                before,
                after,
            },
            state_before_action: before,
            surface_continuation: None,
        };
        debug_assert_eq!(callback_result, 0);
    }
}

fn surface_plan_error_block(error: ActorSurfacePlanError) -> OrdinaryType9OwnerExternalBlock {
    OrdinaryType9OwnerExternalBlock::InvalidSurfaceConfiguration(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hover::HoverBasis;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    const MASTER_MOTION_ENABLE_STATE_BIT: u32 = 0x0004_0000;

    fn transaction_id(raw: u64) -> OrdinaryType9OwnerTransactionId {
        OrdinaryType9OwnerTransactionId::new(NonZeroU64::new(raw).unwrap())
    }

    fn lease() -> OrdinaryType9OwnerLease {
        OrdinaryType9OwnerLease {
            controlled_entity_id: 0x04ac_0001,
            frame_token: NonZeroU64::new(17).unwrap(),
        }
    }

    fn flat_terrain(height: i8, sea_y_raw: i16) -> TerrainGrid {
        let mut header = [0; 5];
        header[0] = i32::from(sea_y_raw) << 8;
        TerrainGrid {
            header,
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn set_height(terrain: &mut TerrainGrid, x: usize, z: usize, height: i8) {
        terrain.cells[x * GRID_SIZE + z].height = height as u8;
    }

    fn state() -> OrdinaryType9OwnerState {
        let basis = HoverBasis::from_angle_words(0, 0, 0);
        OrdinaryType9OwnerState {
            state_flags: MASTER_MOTION_ENABLE_STATE_BIT,
            position_raw: [0, 0, 0],
            velocity_raw: [0, 0, 0],
            heading_raw: 0,
            pitch_raw: 0,
            roll_raw: 0,
            body_basis: Type9BodyBasis {
                lateral: basis.lateral,
                up: basis.up,
                forward: basis.forward,
            },
            surface_runtime: Type9SurfaceRuntime::default(),
        }
    }

    fn post_body_basis_state() -> OrdinaryType9OwnerState {
        let mut state = state();
        state.heading_raw = 0x1234;
        state.pitch_raw = -0x0222;
        state.roll_raw = 0x0111;
        state.body_basis =
            Type9BodyBasis::from_angle_words(state.heading_raw, state.pitch_raw, state.roll_raw);
        state.state_flags |= BODY_BASIS_REBUILT_STATE_BIT;
        state
    }

    fn frame(terrain: &TerrainGrid, elapsed: u32) -> OrdinaryType9OwnerFrame<'_> {
        OrdinaryType9OwnerFrame {
            terrain,
            effective_elapsed_micros: elapsed,
            mode_zero_drag: super::super::type9_tail::Type9ModeZeroDrag {
                callback_mass_raw: std::num::NonZeroU16::new(10).unwrap(),
                strength: 3,
            },
            active_model_extent_raw: 0,
            flat_surface_y_raw: terrain.sea_level_raw(),
            authored_lifetime_ms: 5_000,
        }
    }

    fn issue(transaction: &mut OrdinaryType9OwnerTransaction) -> IssuedOrdinaryType9OwnerAction {
        let OrdinaryType9OwnerPoll::Action(issued) = transaction.poll() else {
            panic!("expected action")
        };
        assert_eq!(transaction.poll(), OrdinaryType9OwnerPoll::Awaiting);
        issued
    }

    fn acknowledge_commit(
        transaction: &mut OrdinaryType9OwnerTransaction,
        phase: OrdinaryType9OwnerCommitPhase,
    ) -> OrdinaryType9OwnerState {
        let issued = issue(transaction);
        let OrdinaryType9OwnerAction::CommitState {
            phase: actual,
            after,
            ..
        } = issued.action
        else {
            panic!("expected state commit")
        };
        assert_eq!(actual, phase);
        transaction
            .resume(
                issued.receipt,
                OrdinaryType9OwnerResume::StateCommitted { committed: after },
            )
            .unwrap();
        after
    }

    #[test]
    fn ordinary_0x2f_owner_skips_e640_and_rebuilds_from_post_task_angles() {
        let mut terrain = flat_terrain(0, -4_096);
        set_height(&mut terrain, 1, 1, 0);
        set_height(&mut terrain, 1, 2, 10);
        set_height(&mut terrain, 2, 1, 20);
        set_height(&mut terrain, 2, 2, 30);
        let mut initial = state();
        initial.position_raw = [0x0180, 0, 0x0180];
        initial.state_flags |= super::super::type9_attitude::TERRAIN_ATTITUDE_BILINEAR_STATE_BIT;
        initial.heading_raw = 0x1234;
        initial.pitch_raw = 0;
        initial.roll_raw = 0;
        initial.body_basis = Type9BodyBasis {
            lateral: [0, 0, i32::MIN],
            up: [0, i32::MAX, 0],
            forward: [i32::MAX, 0, 0],
        };
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(7),
            lease(),
            initial,
            frame(&terrain, 32_768),
        );

        let issued = issue(&mut transaction);
        let OrdinaryType9OwnerAction::CommitState {
            phase: OrdinaryType9OwnerCommitPhase::BodyBasis,
            before,
            after,
        } = issued.action
        else {
            panic!("the first exact outer phase must publish F70")
        };
        assert_eq!(before, initial);
        assert_eq!(
            [after.heading_raw, after.pitch_raw, after.roll_raw],
            [initial.heading_raw, initial.pitch_raw, initial.roll_raw],
            "the sloped terrain would alter pitch/roll if the clear 0x10 gate were ignored"
        );
        assert_eq!(
            after.body_basis,
            Type9BodyBasis::from_angle_words(
                initial.heading_raw,
                initial.pitch_raw,
                initial.roll_raw,
            )
        );
        assert_ne!(after.body_basis, initial.body_basis);
        assert_ne!(after.state_flags & BODY_BASIS_REBUILT_STATE_BIT, 0);
    }

    #[test]
    fn post_body_basis_entry_starts_at_drag_snap_without_replaying_f70() {
        let terrain = flat_terrain(10, -4_096);
        let mut initial = post_body_basis_state();
        initial.position_raw = [0, 99, 0];
        initial.velocity_raw = [8_192, 777, -4_096];
        let retained_basis = initial.body_basis;
        let mut transaction = OrdinaryType9OwnerTransaction::start_after_body_basis(
            transaction_id(8),
            lease(),
            initial,
            frame(&terrain, 20_000),
        )
        .unwrap();

        let first = issue(&mut transaction);
        let OrdinaryType9OwnerAction::CommitState {
            phase,
            before,
            after,
        } = first.action
        else {
            panic!("the post-F70 entry must issue the E100/DF70 commit")
        };
        assert_eq!(
            phase,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap
        );
        assert_eq!(before, initial);
        assert_eq!(after.body_basis, retained_basis);
        assert_eq!(
            after.state_flags & BODY_BASIS_REBUILT_STATE_BIT,
            initial.state_flags & BODY_BASIS_REBUILT_STATE_BIT,
            "the retained F70 completion bit is not republished or cleared"
        );
        assert_eq!(
            [after.heading_raw, after.pitch_raw, after.roll_raw],
            [initial.heading_raw, initial.pitch_raw, initial.roll_raw]
        );
        assert_eq!(after.velocity_raw, [8_005, 0, -4_002]);
        assert_eq!(after.position_raw, [0, 320, 0]);
        transaction
            .resume(
                first.receipt,
                OrdinaryType9OwnerResume::StateCommitted { committed: after },
            )
            .unwrap();

        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );
        let completed = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::MasterMotion,
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Complete(OrdinaryType9OwnerCompletion {
                lease: lease(),
                state: completed,
                callback_result: 0,
            }),
            "no later phase may replay the omitted BodyBasis commit"
        );
    }

    #[test]
    fn post_body_basis_entry_rejects_incomplete_or_tampered_publication_atomically() {
        let terrain = flat_terrain(0, -4_096);
        let valid = post_body_basis_state();

        let mut missing_bit = valid;
        missing_bit.state_flags &= !BODY_BASIS_REBUILT_STATE_BIT;
        assert_eq!(
            OrdinaryType9OwnerTransaction::start_after_body_basis(
                transaction_id(9),
                lease(),
                missing_bit,
                frame(&terrain, 20_000),
            )
            .unwrap_err(),
            OrdinaryType9OwnerStartAfterBodyBasisError::BodyBasisRebuiltStateBitClear {
                actual_state_flags: missing_bit.state_flags,
            }
        );

        let mut tampered_basis = valid;
        tampered_basis.body_basis.forward[0] ^= 1;
        assert_eq!(
            OrdinaryType9OwnerTransaction::start_after_body_basis(
                transaction_id(10),
                lease(),
                tampered_basis,
                frame(&terrain, 20_000),
            )
            .unwrap_err(),
            OrdinaryType9OwnerStartAfterBodyBasisError::BodyBasisDoesNotMatchPostTaskAngles {
                expected: valid.body_basis,
                actual: tampered_basis.body_basis,
            }
        );

        let mut tampered_angles = valid;
        tampered_angles.heading_raw = tampered_angles.heading_raw.wrapping_add(0x0100);
        assert!(matches!(
            OrdinaryType9OwnerTransaction::start_after_body_basis(
                transaction_id(11),
                lease(),
                tampered_angles,
                frame(&terrain, 20_000),
            )
            .unwrap_err(),
            OrdinaryType9OwnerStartAfterBodyBasisError::BodyBasisDoesNotMatchPostTaskAngles {
                actual,
                ..
            } if actual == valid.body_basis
        ));
        assert_eq!(valid, post_body_basis_state());
    }

    #[test]
    fn post_body_basis_entry_receipt_rejects_lease_tamper_without_advancing() {
        let terrain = flat_terrain(0, -4_096);
        let mut transaction = OrdinaryType9OwnerTransaction::start_after_body_basis(
            transaction_id(12),
            lease(),
            post_body_basis_state(),
            frame(&terrain, 20_000),
        )
        .unwrap();
        let issued = issue(&mut transaction);
        let OrdinaryType9OwnerAction::CommitState { after, .. } = issued.action else {
            panic!("expected the first post-basis commit")
        };
        let wrong_lease = OrdinaryType9OwnerLease {
            controlled_entity_id: lease().controlled_entity_id,
            frame_token: NonZeroU64::new(lease().frame_token.get() + 1).unwrap(),
        };
        let wrong = OrdinaryType9OwnerReceipt {
            transaction_id: issued.receipt.transaction_id(),
            lease: wrong_lease,
            action_sequence: issued.receipt.action_sequence(),
            action_kind: OrdinaryType9OwnerActionKind::Commit(
                OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
            ),
        };
        let failure = transaction
            .resume(
                wrong,
                OrdinaryType9OwnerResume::StateCommitted { committed: after },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9OwnerProtocolError::ReceiptLeaseMismatch {
                expected: lease(),
                actual: wrong_lease,
            }
        );
        assert_eq!(transaction.poll(), OrdinaryType9OwnerPoll::Awaiting);
        transaction
            .resume(
                issued.receipt,
                OrdinaryType9OwnerResume::StateCommitted { committed: after },
            )
            .unwrap();
    }

    #[test]
    fn ground_snap_uses_acknowledged_old_x_before_master_integrates_x() {
        let mut terrain = flat_terrain(10, -4_096);
        set_height(&mut terrain, 1, 0, 20);
        let mut initial = state();
        initial.velocity_raw = [8_192, 777, 0];
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(1),
            lease(),
            initial,
            frame(&terrain, 32_768),
        );

        let attitude =
            acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        assert_ne!(
            attitude.state_flags & BODY_BASIS_REBUILT_STATE_BIT,
            0,
            "FUN_00413F70 publishes the retained matrix-complete bit"
        );
        let snapped = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        assert_eq!(snapped.position_raw, [0, 320, 0]);
        assert_eq!(snapped.velocity_raw[1], 0);

        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );
        let moved = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::MasterMotion,
        );
        assert_eq!(moved.position_raw[0], 0x0f6);
        assert_eq!(
            moved.position_raw[1], 320,
            "master motion must not resample the terrain after moving X"
        );
        assert_eq!(
            super::super::type9_tail::bilinear_terrain_height_raw(
                &terrain,
                moved.position_raw[0],
                moved.position_raw[2],
            ),
            627,
            "the new X would have produced a visibly different snap height"
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Complete(OrdinaryType9OwnerCompletion {
                lease: lease(),
                state: moved,
                callback_result: 0,
            })
        );
    }

    #[test]
    fn surface_timer_commit_precedes_separate_bubble_and_sound_draws() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_900;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(2),
            lease(),
            initial,
            frame(&terrain, 0),
        );
        let mut draws = 0;

        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        assert_eq!(draws, 0);

        let timer = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );
        assert_eq!(timer.surface_runtime.lifetime_timer_ms_at_0x48, 4_900);
        assert_eq!(draws, 0, "the +0x48 commit precedes the first RNG word");

        let bubble = issue(&mut transaction);
        assert!(matches!(
            bubble.action,
            OrdinaryType9OwnerAction::PlanSurface {
                phase: OrdinaryType9OwnerSurfacePlanPhase::Bubble { .. },
                ..
            }
        ));
        transaction
            .plan_surface(bubble.receipt, || {
                draws += 1;
                1
            })
            .unwrap();
        assert_eq!(draws, 1, "a bubble miss consumes only its gate word");

        let sound = issue(&mut transaction);
        assert!(matches!(
            sound.action,
            OrdinaryType9OwnerAction::PlanSurface {
                phase: OrdinaryType9OwnerSurfacePlanPhase::Sound,
                ..
            }
        ));
        transaction
            .plan_surface(sound.receipt, || {
                draws += 1;
                1
            })
            .unwrap();
        assert_eq!(
            draws, 2,
            "failed bubble gate and independent sound gate consume two words"
        );
    }

    #[test]
    fn blocked_surface_timer_commit_consumes_no_rng_and_never_reissues() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_900;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(14),
            lease(),
            initial,
            frame(&terrain, 20_000),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );

        let timer = issue(&mut transaction);
        let OrdinaryType9OwnerAction::CommitState {
            phase: OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
            before,
            ..
        } = timer.action
        else {
            panic!("the no-RNG timer write must be offered before random effects")
        };
        transaction
            .resume(
                timer.receipt,
                OrdinaryType9OwnerResume::Blocked {
                    reason: OrdinaryType9OwnerExternalBlock::StateCommitUnavailable,
                },
            )
            .unwrap();
        let expected = OrdinaryType9OwnerBlock {
            lease: lease(),
            action_kind: OrdinaryType9OwnerActionKind::Commit(
                OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
            ),
            state_before_action: before,
            reason: OrdinaryType9OwnerExternalBlock::StateCommitUnavailable,
            action_executed: false,
        };
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Blocked(expected)
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Blocked(expected),
            "terminal polling cannot replay the timer commit or reach either RNG phase"
        );
    }

    #[test]
    fn unavailable_surface_rng_blocks_once_without_reissuing_the_plan() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_900;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(6),
            lease(),
            initial,
            frame(&terrain, 20_000),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        let committed_timer = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );

        let planning = issue(&mut transaction);
        assert!(matches!(
            &planning.action,
            OrdinaryType9OwnerAction::PlanSurface {
                phase: OrdinaryType9OwnerSurfacePlanPhase::Bubble { .. },
                ..
            }
        ));
        transaction
            .resume(
                planning.receipt,
                OrdinaryType9OwnerResume::Blocked {
                    reason: OrdinaryType9OwnerExternalBlock::SurfaceRngUnavailable,
                },
            )
            .unwrap();

        let expected = OrdinaryType9OwnerBlock {
            lease: lease(),
            action_kind: OrdinaryType9OwnerActionKind::PlanSurface,
            state_before_action: committed_timer,
            reason: OrdinaryType9OwnerExternalBlock::SurfaceRngUnavailable,
            action_executed: false,
        };
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Blocked(expected)
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Blocked(expected),
            "terminal polling cannot reissue the shared-RNG planning boundary"
        );
    }

    #[test]
    fn effect_completion_fresh_state_drives_later_master_motion() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_900;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(3),
            lease(),
            initial,
            frame(&terrain, 32_768),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );

        let planning = issue(&mut transaction);
        assert!(matches!(
            planning.action,
            OrdinaryType9OwnerAction::PlanSurface {
                phase: OrdinaryType9OwnerSurfacePlanPhase::Bubble { .. },
                ..
            }
        ));
        let words = [0, 0, 0, 0, 0, 0, 1];
        let mut cursor = 0;
        transaction
            .plan_surface(planning.receipt, || {
                let value = words[cursor];
                cursor += 1;
                value
            })
            .unwrap();
        assert_eq!(cursor, 6, "sound RNG must wait for the bubble allocator");

        let bubble = issue(&mut transaction);
        assert!(matches!(
            &bubble.action,
            OrdinaryType9OwnerAction::SpawnBubble(_)
        ));
        let mut post_effect = initial;
        post_effect.position_raw = [1_000, 200, 3_000];
        post_effect.velocity_raw = [8_192, 0, 0];
        post_effect.state_flags = MASTER_MOTION_ENABLE_STATE_BIT;
        transaction
            .resume(
                bubble.receipt,
                OrdinaryType9OwnerResume::EffectCompleted {
                    fresh_state: post_effect,
                },
            )
            .unwrap();

        let sound = issue(&mut transaction);
        assert!(matches!(
            sound.action,
            OrdinaryType9OwnerAction::PlanSurface {
                phase: OrdinaryType9OwnerSurfacePlanPhase::Sound,
                ..
            }
        ));
        assert_eq!(cursor, 6, "bubble acknowledgement itself consumes no RNG");
        transaction
            .plan_surface(sound.receipt, || {
                let value = words[cursor];
                cursor += 1;
                value
            })
            .unwrap();
        assert_eq!(cursor, 7);

        let moved = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::MasterMotion,
        );
        assert_eq!(moved.position_raw, [1_256, 200, 3_000]);
    }

    #[test]
    fn lifecycle_is_one_journaled_action_and_missing_post_state_never_replays_it() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_999;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(4),
            lease(),
            initial,
            frame(&terrain, 1_000),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        let surface = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );

        let lifecycle = issue(&mut transaction);
        assert_eq!(
            lifecycle.receipt.action_sequence(),
            4,
            "expiry commits +0x48 and reaches lifecycle without any RNG receipt"
        );
        assert_eq!(
            lifecycle.action,
            OrdinaryType9OwnerAction::RunLifecycleContinuation(Type9LifecycleRequest {
                entity_id: lease().controlled_entity_id,
            })
        );
        transaction
            .resume(
                lifecycle.receipt,
                OrdinaryType9OwnerResume::LifecycleCompleted { fresh_state: None },
            )
            .unwrap();
        let expected = OrdinaryType9OwnerBlock {
            lease: lease(),
            action_kind: OrdinaryType9OwnerActionKind::Lifecycle,
            state_before_action: surface,
            reason: OrdinaryType9OwnerExternalBlock::PostLifecycleEntityUnavailable,
            action_executed: true,
        };
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Blocked(expected)
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Blocked(expected),
            "terminal polling cannot reissue either lifecycle call"
        );
    }

    #[test]
    fn exact_expiry_reaches_bubble_rng_only_after_lifecycle_acknowledgement() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_999;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(15),
            lease(),
            initial,
            frame(&terrain, 1_000),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        let surface = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );
        assert_eq!(surface.surface_runtime.lifetime_timer_ms_at_0x48, 5_000);

        let lifecycle = issue(&mut transaction);
        assert!(matches!(
            lifecycle.action,
            OrdinaryType9OwnerAction::RunLifecycleContinuation(_)
        ));
        let mut draws = 0;
        assert_eq!(transaction.poll(), OrdinaryType9OwnerPoll::Awaiting);
        assert_eq!(draws, 0, "expiry must not consume RNG before lifecycle ack");

        transaction
            .resume(
                lifecycle.receipt,
                OrdinaryType9OwnerResume::LifecycleCompleted {
                    fresh_state: Some(surface),
                },
            )
            .unwrap();
        let bubble = issue(&mut transaction);
        assert!(matches!(
            bubble.action,
            OrdinaryType9OwnerAction::PlanSurface {
                phase: OrdinaryType9OwnerSurfacePlanPhase::Bubble {
                    remaining_percent: 0
                },
                ..
            }
        ));
        assert_eq!(draws, 0, "issuing the post-lifecycle gate consumes no RNG");
        transaction
            .plan_surface(bubble.receipt, || {
                draws += 1;
                1
            })
            .unwrap();
        assert_eq!(draws, 1, "the exact-expiry bubble gate runs after the ack");
    }

    #[test]
    fn lifecycle_overshoot_returns_to_master_without_random_effects() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 4_999;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(16),
            lease(),
            initial,
            frame(&terrain, 2_000),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        let surface = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );
        assert_eq!(surface.surface_runtime.lifetime_timer_ms_at_0x48, 5_001);

        let lifecycle = issue(&mut transaction);
        transaction
            .resume(
                lifecycle.receipt,
                OrdinaryType9OwnerResume::LifecycleCompleted {
                    fresh_state: Some(surface),
                },
            )
            .unwrap();
        let master = issue(&mut transaction);
        assert!(matches!(
            master.action,
            OrdinaryType9OwnerAction::CommitState {
                phase: OrdinaryType9OwnerCommitPhase::MasterMotion,
                ..
            }
        ));
    }

    #[test]
    fn main_base_abort_fork_preserves_outstanding_lifecycle_receipt_independently() {
        let terrain = flat_terrain(-1, 0);
        let mut initial = state();
        initial.position_raw[1] = -1;
        initial.surface_runtime.lifetime_timer_ms_at_0x48 = 5_000;
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(13),
            lease(),
            initial,
            frame(&terrain, 1_000),
        );
        acknowledge_commit(&mut transaction, OrdinaryType9OwnerCommitPhase::BodyBasis);
        acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::EnvironmentDragAndOldPositionSnap,
        );
        let surface = acknowledge_commit(
            &mut transaction,
            OrdinaryType9OwnerCommitPhase::SurfaceRuntime,
        );

        let lifecycle = issue(&mut transaction);
        assert_eq!(lifecycle.receipt.action_sequence(), 4);
        assert!(matches!(
            &lifecycle.action,
            OrdinaryType9OwnerAction::RunLifecycleContinuation(_)
        ));
        let mut fork = transaction.fork_for_main_base_abort_transaction();
        let forked_lifecycle = lifecycle.fork_for_main_base_abort_transaction();
        assert_eq!(fork, transaction);
        assert_eq!(forked_lifecycle, lifecycle);

        let mut fork_state = surface;
        fork_state.position_raw[0] = 1_000;
        fork.resume(
            forked_lifecycle.receipt,
            OrdinaryType9OwnerResume::LifecycleCompleted {
                fresh_state: Some(fork_state),
            },
        )
        .unwrap();
        assert_eq!(transaction.poll(), OrdinaryType9OwnerPoll::Awaiting);
        assert_ne!(fork, transaction);

        let fork_master = issue(&mut fork);
        assert_eq!(fork_master.receipt.action_sequence(), 5);
        let OrdinaryType9OwnerAction::CommitState {
            phase: fork_phase,
            before: fork_before,
            after: fork_after,
        } = fork_master.action
        else {
            panic!("fork must continue at master motion")
        };
        assert_eq!(fork_phase, OrdinaryType9OwnerCommitPhase::MasterMotion);
        assert_eq!(fork_before, fork_state);

        let mut original_state = surface;
        original_state.position_raw[0] = -1_000;
        transaction
            .resume(
                lifecycle.receipt,
                OrdinaryType9OwnerResume::LifecycleCompleted {
                    fresh_state: Some(original_state),
                },
            )
            .unwrap();
        let original_master = issue(&mut transaction);
        assert_eq!(original_master.receipt.action_sequence(), 5);
        let OrdinaryType9OwnerAction::CommitState {
            phase: original_phase,
            before: original_before,
            after: original_after,
        } = original_master.action
        else {
            panic!("original must retain its own master-motion continuation")
        };
        assert_eq!(original_phase, OrdinaryType9OwnerCommitPhase::MasterMotion);
        assert_eq!(original_before, original_state);
        assert_ne!(fork_after, original_after);

        fork.resume(
            fork_master.receipt,
            OrdinaryType9OwnerResume::StateCommitted {
                committed: fork_after,
            },
        )
        .unwrap();
        transaction
            .resume(
                original_master.receipt,
                OrdinaryType9OwnerResume::StateCommitted {
                    committed: original_after,
                },
            )
            .unwrap();
        assert_eq!(
            fork.poll(),
            OrdinaryType9OwnerPoll::Complete(OrdinaryType9OwnerCompletion {
                lease: lease(),
                state: fork_after,
                callback_result: 0,
            })
        );
        assert_eq!(
            transaction.poll(),
            OrdinaryType9OwnerPoll::Complete(OrdinaryType9OwnerCompletion {
                lease: lease(),
                state: original_after,
                callback_result: 0,
            })
        );
    }

    #[test]
    fn wrong_receipt_and_wrong_acknowledgement_preserve_outstanding_action() {
        let terrain = flat_terrain(0, -4_096);
        let mut transaction = OrdinaryType9OwnerTransaction::start(
            transaction_id(5),
            lease(),
            state(),
            frame(&terrain, 20_000),
        );
        let issued = issue(&mut transaction);
        let OrdinaryType9OwnerAction::CommitState { after, .. } = issued.action else {
            panic!("expected commit")
        };
        let expected_sequence = issued.receipt.action_sequence();
        let wrong_sequence = expected_sequence + 1;
        let wrong = OrdinaryType9OwnerReceipt {
            transaction_id: issued.receipt.transaction_id(),
            lease: issued.receipt.lease(),
            action_sequence: wrong_sequence,
            action_kind: OrdinaryType9OwnerActionKind::Commit(
                OrdinaryType9OwnerCommitPhase::BodyBasis,
            ),
        };
        let failure = transaction
            .resume(
                wrong,
                OrdinaryType9OwnerResume::StateCommitted { committed: after },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            OrdinaryType9OwnerProtocolError::ReceiptSequenceMismatch {
                expected: expected_sequence,
                actual: wrong_sequence,
            }
        );
        assert_eq!(transaction.poll(), OrdinaryType9OwnerPoll::Awaiting);

        let mut mismatched = after;
        mismatched.position_raw[0] = 1;
        let failure = transaction
            .resume(
                issued.receipt,
                OrdinaryType9OwnerResume::StateCommitted {
                    committed: mismatched,
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            OrdinaryType9OwnerProtocolError::AcknowledgedStateMismatch { .. }
        ));
        assert_eq!(transaction.poll(), OrdinaryType9OwnerPoll::Awaiting);
        transaction
            .resume(
                failure.receipt,
                OrdinaryType9OwnerResume::StateCommitted { committed: after },
            )
            .unwrap();
    }
}
