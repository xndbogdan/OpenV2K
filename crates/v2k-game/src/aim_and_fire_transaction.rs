//! Receipt-bound composition of one Aim-and-Fire callback with its generic
//! projectile-emitter child.
//!
//! The generic actor-task scheduler owns elapsed accounting before this
//! boundary and wrapper unwind after it. This coordinator retains the exact
//! [`ActorTaskVisit`] and committed callback prefix while the optional-sound
//! RNG/sound actions and the resumable `FUN_00424650` child run. It deliberately
//! does not retain a mutable [`crate::actor_task_owner::ActorTaskOwner`] borrow
//! across those external actions.
//!
//! This remains detached infrastructure. A live adapter must validate the
//! lease before every action, journal execution by the linear receipt, and
//! route the terminal callback result back through the shared task owner.

use std::num::NonZeroU64;

use v2k_formats::collision::ProjectileEmitterDescriptor;

use crate::{
    actor_task_owner::ActorTaskVisit,
    aim_and_fire::{
        classify_invalid_target, optional_sound_uses_shared_random,
        plan_optional_sound_from_sample, AimAndFireCallbackPrefix, AimAndFireCallbackResult,
        AimAndFireFrameRequest, AimAndFireOptionalSoundEffect, AimAndFirePrivateState,
        AimAndFireTaggedSingleton, InvokeGenericEmitter,
    },
    generic_projectile_emitter::{
        GenericEmitterAction, GenericEmitterBlock, GenericEmitterCompletion, GenericEmitterPhase,
        GenericEmitterPoll, GenericEmitterProtocolError, GenericEmitterReceipt,
        GenericEmitterRequest, GenericEmitterResume, GenericEmitterRuntime,
        GenericEmitterTransactionId, GenericProjectileEmitterMachine,
    },
};

/// Caller-owned identity for one parent callback transaction.
///
/// This identity and the child emitter identity must be distinct. Neither may
/// be reused while a machine or adapter journal entry remains live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimAndFireEmitterTransactionId(NonZeroU64);

impl AimAndFireEmitterTransactionId {
    pub const fn new(raw: u64) -> Option<Self> {
        match NonZeroU64::new(raw) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Runtime ownership which must still identify the callback when an action is
/// committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireEmitterLease {
    pub owner_entity_id: u32,
    pub task_visit: ActorTaskVisit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireEmitterStartError {
    ParentAndChildTransactionIdsMustDiffer {
        shared_raw: u64,
    },
    LeaseOwnerMismatch {
        lease_owner: u32,
        request_owner: u32,
    },
}

/// One externally visible parent boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireEmitterPhase {
    DrawOptionalSoundRandom,
    PlayOptionalSound,
    GenericEmitter(GenericEmitterPhase),
}

/// Parent action in exact callback order.
///
/// Deliberately neither `Copy` nor `Clone`: the nested generic action and its
/// parent receipt form one linear adapter boundary.
#[derive(Debug, PartialEq, Eq)]
pub enum AimAndFireEmitterAction {
    DrawSharedRandom {
        phase: AimAndFireEmitterPhase,
        lease: AimAndFireEmitterLease,
    },
    PlayOptionalSound {
        phase: AimAndFireEmitterPhase,
        lease: AimAndFireEmitterLease,
        effect: AimAndFireOptionalSoundEffect,
    },
    GenericEmitter {
        phase: AimAndFireEmitterPhase,
        lease: AimAndFireEmitterLease,
        child_transaction_id: GenericEmitterTransactionId,
        action: GenericEmitterAction,
    },
}

impl AimAndFireEmitterAction {
    pub const fn phase(&self) -> AimAndFireEmitterPhase {
        match self {
            Self::DrawSharedRandom { phase, .. }
            | Self::PlayOptionalSound { phase, .. }
            | Self::GenericEmitter { phase, .. } => *phase,
        }
    }

    pub const fn lease(&self) -> AimAndFireEmitterLease {
        match self {
            Self::DrawSharedRandom { lease, .. }
            | Self::PlayOptionalSound { lease, .. }
            | Self::GenericEmitter { lease, .. } => *lease,
        }
    }
}

/// Linear proof that one exact parent action was issued.
///
/// Fields are private and the type is deliberately neither `Copy` nor `Clone`.
#[derive(Debug, PartialEq, Eq)]
pub struct AimAndFireEmitterReceipt {
    transaction_id: AimAndFireEmitterTransactionId,
    lease: AimAndFireEmitterLease,
    action_sequence: u64,
}

impl AimAndFireEmitterReceipt {
    pub const fn transaction_id(&self) -> AimAndFireEmitterTransactionId {
        self.transaction_id
    }

    pub const fn lease(&self) -> AimAndFireEmitterLease {
        self.lease
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IssuedAimAndFireEmitterAction {
    pub receipt: AimAndFireEmitterReceipt,
    pub action: AimAndFireEmitterAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireEmitterExternalBlock {
    SharedRandomUnavailable,
    OptionalSoundUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireEmitterBlockReason {
    External {
        phase: AimAndFireEmitterPhase,
        reason: AimAndFireEmitterExternalBlock,
    },
    GenericEmitter(GenericEmitterBlock),
}

/// Durable parent block, retaining the callback lease and committed prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireEmitterBlock {
    pub lease: AimAndFireEmitterLease,
    pub committed_prefix: AimAndFireCallbackPrefix,
    pub optional_sound_committed: bool,
    pub reason: AimAndFireEmitterBlockReason,
}

/// Callback result ready for the shared task owner to unwind and interpret.
///
/// This is intentionally not an [`crate::aim_and_fire::AimAndFireFrameOutcome`]:
/// tag/timeout owner transitions happen only after the wrapper has unwound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimAndFireEmitterCompletion {
    pub lease: AimAndFireEmitterLease,
    pub committed_prefix: AimAndFireCallbackPrefix,
    pub callback_result: AimAndFireCallbackResult,
    pub optional_sound_committed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AimAndFireEmitterPoll {
    Action(IssuedAimAndFireEmitterAction),
    Awaiting(AimAndFireEmitterPhase),
    Blocked(AimAndFireEmitterBlock),
    Complete(AimAndFireEmitterCompletion),
}

/// Phase-tagged adapter completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireEmitterResume {
    RandomDrawn {
        phase: AimAndFireEmitterPhase,
        value: u32,
    },
    Acknowledged {
        phase: AimAndFireEmitterPhase,
    },
    GenericEmitter {
        phase: AimAndFireEmitterPhase,
        child_transaction_id: GenericEmitterTransactionId,
        response: GenericEmitterResume,
    },
    Blocked {
        phase: AimAndFireEmitterPhase,
        reason: AimAndFireEmitterExternalBlock,
    },
}

impl AimAndFireEmitterResume {
    pub const fn phase(self) -> AimAndFireEmitterPhase {
        match self {
            Self::RandomDrawn { phase, .. }
            | Self::Acknowledged { phase }
            | Self::GenericEmitter { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AimAndFireEmitterProtocolError {
    NoOutstandingAction {
        transaction_id: AimAndFireEmitterTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: AimAndFireEmitterTransactionId,
        actual: AimAndFireEmitterTransactionId,
    },
    ReceiptLeaseMismatch {
        expected: AimAndFireEmitterLease,
        actual: AimAndFireEmitterLease,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: AimAndFireEmitterPhase,
        actual: AimAndFireEmitterPhase,
    },
    ChildTransactionMismatch {
        expected: GenericEmitterTransactionId,
        actual: GenericEmitterTransactionId,
    },
    UnexpectedResumeKind {
        phase: AimAndFireEmitterPhase,
    },
    ChildProtocol(GenericEmitterProtocolError),
}

/// Recoverable rejection carrying the still-linear parent receipt.
#[derive(Debug, PartialEq, Eq)]
pub struct AimAndFireEmitterResumeFailure {
    pub receipt: AimAndFireEmitterReceipt,
    pub error: AimAndFireEmitterProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AimAndFireEmitterStage {
    DrawOptionalSoundRandom,
    PlayOptionalSound(AimAndFireOptionalSoundEffect),
    GenericEmitter,
    Blocked(AimAndFireEmitterBlock),
    Complete(AimAndFireEmitterCompletion),
}

#[derive(Debug, PartialEq, Eq)]
struct OutstandingAimAndFireEmitterAction {
    sequence: u64,
    phase: AimAndFireEmitterPhase,
    child_receipt: Option<GenericEmitterReceipt>,
}

/// Non-replayable detached callback/child coordinator.
///
/// Deliberately neither `Copy` nor `Clone`: duplicating it could replay the
/// optional-sound RNG draw, sound, or any child emitter effect.
#[derive(Debug, PartialEq, Eq)]
pub struct AimAndFireEmitterMachine {
    transaction_id: AimAndFireEmitterTransactionId,
    child_transaction_id: GenericEmitterTransactionId,
    lease: AimAndFireEmitterLease,
    committed_prefix: AimAndFireCallbackPrefix,
    private_state: AimAndFirePrivateState,
    elapsed_micros: u32,
    owner_sound_position_raw: Option<[i16; 3]>,
    mapped_child_request: Option<GenericEmitterRequest>,
    child: Option<GenericProjectileEmitterMachine>,
    optional_sound_committed: bool,
    stage: AimAndFireEmitterStage,
    next_action_sequence: u64,
    outstanding: Option<OutstandingAimAndFireEmitterAction>,
}

impl AimAndFireEmitterMachine {
    /// Start after the generic scheduler has committed `committed_prefix`.
    ///
    /// Scheduler suppression and invalid-target paths become immediate callback
    /// completions. A valid target performs the optional-sound path before the
    /// Sub-E null gate and child construction, exactly like retail.
    pub fn start(
        transaction_id: AimAndFireEmitterTransactionId,
        child_transaction_id: GenericEmitterTransactionId,
        lease: AimAndFireEmitterLease,
        committed_prefix: AimAndFireCallbackPrefix,
        private_state: AimAndFirePrivateState,
        request: AimAndFireFrameRequest<ProjectileEmitterDescriptor, GenericEmitterRuntime>,
    ) -> Result<Self, AimAndFireEmitterStartError> {
        if transaction_id.get() == child_transaction_id.get() {
            return Err(
                AimAndFireEmitterStartError::ParentAndChildTransactionIdsMustDiffer {
                    shared_raw: transaction_id.get(),
                },
            );
        }
        if lease.owner_entity_id != request.owner_entity_id {
            return Err(AimAndFireEmitterStartError::LeaseOwnerMismatch {
                lease_owner: lease.owner_entity_id,
                request_owner: request.owner_entity_id,
            });
        }

        let callback_result = if request.scheduler_mode != 0 {
            Some(AimAndFireCallbackResult::Zero)
        } else {
            classify_invalid_target(request.target_state).map(|reason| {
                AimAndFireCallbackResult::TaggedInvalidTarget {
                    singleton: AimAndFireTaggedSingleton::InvalidTarget,
                    reason,
                }
            })
        };

        let mapped_child_request = if callback_result.is_none() {
            request.sub_e_descriptor.map(|descriptor| {
                GenericEmitterRequest::from(InvokeGenericEmitter {
                    owner_entity_id: request.owner_entity_id,
                    secondary_owner_entity_id: request.owner_entity_id,
                    target_entity_id: private_state.target_entity_id(),
                    explicit_position_raw: None,
                    sub_e_descriptor: descriptor,
                    emitter_runtime: request.emitter_runtime,
                    elapsed_micros: request.elapsed_micros,
                })
            })
        } else {
            None
        };

        let initial_stage = match callback_result {
            Some(callback_result) => {
                AimAndFireEmitterStage::Complete(AimAndFireEmitterCompletion {
                    lease,
                    committed_prefix,
                    callback_result,
                    optional_sound_committed: false,
                })
            }
            None if optional_sound_uses_shared_random(private_state) => {
                AimAndFireEmitterStage::DrawOptionalSoundRandom
            }
            None => AimAndFireEmitterStage::GenericEmitter,
        };

        let mut machine = Self {
            transaction_id,
            child_transaction_id,
            lease,
            committed_prefix,
            private_state,
            elapsed_micros: request.elapsed_micros,
            owner_sound_position_raw: request.owner_sound_position_raw,
            mapped_child_request,
            child: None,
            optional_sound_committed: false,
            stage: initial_stage,
            next_action_sequence: 1,
            outstanding: None,
        };
        if matches!(machine.stage, AimAndFireEmitterStage::GenericEmitter) {
            machine.start_child_or_complete();
        }
        Ok(machine)
    }

    pub const fn lease(&self) -> AimAndFireEmitterLease {
        self.lease
    }

    pub const fn committed_prefix(&self) -> AimAndFireCallbackPrefix {
        self.committed_prefix
    }

    pub const fn child_transaction_id(&self) -> GenericEmitterTransactionId {
        self.child_transaction_id
    }

    /// Exact seven-argument mapping retained for the child transaction.
    pub const fn mapped_child_request(&self) -> Option<GenericEmitterRequest> {
        self.mapped_child_request
    }

    pub fn poll(&mut self) -> AimAndFireEmitterPoll {
        if let Some(outstanding) = &self.outstanding {
            return AimAndFireEmitterPoll::Awaiting(outstanding.phase);
        }

        loop {
            match self.stage {
                AimAndFireEmitterStage::DrawOptionalSoundRandom => {
                    return self.issue(
                        AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                        AimAndFireEmitterAction::DrawSharedRandom {
                            phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                            lease: self.lease,
                        },
                        None,
                    );
                }
                AimAndFireEmitterStage::PlayOptionalSound(effect) => {
                    return self.issue(
                        AimAndFireEmitterPhase::PlayOptionalSound,
                        AimAndFireEmitterAction::PlayOptionalSound {
                            phase: AimAndFireEmitterPhase::PlayOptionalSound,
                            lease: self.lease,
                            effect,
                        },
                        None,
                    );
                }
                AimAndFireEmitterStage::GenericEmitter => {
                    let child_poll = self
                        .child
                        .as_mut()
                        .expect("generic-emitter stage owns a child")
                        .poll();
                    match child_poll {
                        GenericEmitterPoll::Action(issued) => {
                            let child_phase = issued.action.phase();
                            debug_assert_eq!(
                                issued.receipt.transaction_id(),
                                self.child_transaction_id
                            );
                            return self.issue(
                                AimAndFireEmitterPhase::GenericEmitter(child_phase),
                                AimAndFireEmitterAction::GenericEmitter {
                                    phase: AimAndFireEmitterPhase::GenericEmitter(child_phase),
                                    lease: self.lease,
                                    child_transaction_id: self.child_transaction_id,
                                    action: issued.action,
                                },
                                Some(issued.receipt),
                            );
                        }
                        GenericEmitterPoll::Awaiting(_) => {
                            unreachable!(
                                "a child receipt is always wrapped by a parent outstanding action"
                            )
                        }
                        GenericEmitterPoll::Blocked(reason) => {
                            self.stage = AimAndFireEmitterStage::Blocked(AimAndFireEmitterBlock {
                                lease: self.lease,
                                committed_prefix: self.committed_prefix,
                                optional_sound_committed: self.optional_sound_committed,
                                reason: AimAndFireEmitterBlockReason::GenericEmitter(reason),
                            });
                        }
                        GenericEmitterPoll::Complete(completion) => {
                            let callback_result = match completion {
                                GenericEmitterCompletion::ReturnZero => {
                                    AimAndFireCallbackResult::Zero
                                }
                                GenericEmitterCompletion::ReturnNonZero(result) => {
                                    AimAndFireCallbackResult::ReturnGenericEmitterResult(result)
                                }
                            };
                            self.stage =
                                AimAndFireEmitterStage::Complete(AimAndFireEmitterCompletion {
                                    lease: self.lease,
                                    committed_prefix: self.committed_prefix,
                                    callback_result,
                                    optional_sound_committed: self.optional_sound_committed,
                                });
                        }
                    }
                }
                AimAndFireEmitterStage::Blocked(block) => {
                    return AimAndFireEmitterPoll::Blocked(block);
                }
                AimAndFireEmitterStage::Complete(completion) => {
                    return AimAndFireEmitterPoll::Complete(completion);
                }
            }
        }
    }

    pub fn resume(
        &mut self,
        receipt: AimAndFireEmitterReceipt,
        response: AimAndFireEmitterResume,
    ) -> Result<(), AimAndFireEmitterResumeFailure> {
        let Some(outstanding) = self.outstanding.as_ref() else {
            return Err(AimAndFireEmitterResumeFailure {
                error: AimAndFireEmitterProtocolError::NoOutstandingAction {
                    transaction_id: receipt.transaction_id,
                    action_sequence: receipt.action_sequence,
                },
                receipt,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(AimAndFireEmitterResumeFailure {
                error: AimAndFireEmitterProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.lease != self.lease {
            return Err(AimAndFireEmitterResumeFailure {
                error: AimAndFireEmitterProtocolError::ReceiptLeaseMismatch {
                    expected: self.lease,
                    actual: receipt.lease,
                },
                receipt,
            });
        }
        if receipt.action_sequence != outstanding.sequence {
            return Err(AimAndFireEmitterResumeFailure {
                error: AimAndFireEmitterProtocolError::ReceiptSequenceMismatch {
                    expected: outstanding.sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }
        let expected_phase = outstanding.phase;
        let actual_phase = response.phase();
        if actual_phase != expected_phase {
            return Err(AimAndFireEmitterResumeFailure {
                error: AimAndFireEmitterProtocolError::PhaseMismatch {
                    expected: expected_phase,
                    actual: actual_phase,
                },
                receipt,
            });
        }

        match (self.stage, response) {
            (
                AimAndFireEmitterStage::DrawOptionalSoundRandom,
                AimAndFireEmitterResume::RandomDrawn { value, .. },
            ) => {
                self.outstanding = None;
                if let Some(effect) = plan_optional_sound_from_sample(
                    self.private_state,
                    self.elapsed_micros,
                    self.owner_sound_position_raw,
                    value,
                ) {
                    self.stage = AimAndFireEmitterStage::PlayOptionalSound(effect);
                } else {
                    self.start_child_or_complete();
                }
                Ok(())
            }
            (
                AimAndFireEmitterStage::DrawOptionalSoundRandom,
                AimAndFireEmitterResume::Blocked { reason, .. },
            )
            | (
                AimAndFireEmitterStage::PlayOptionalSound(_),
                AimAndFireEmitterResume::Blocked { reason, .. },
            ) => {
                self.outstanding = None;
                self.stage = AimAndFireEmitterStage::Blocked(AimAndFireEmitterBlock {
                    lease: self.lease,
                    committed_prefix: self.committed_prefix,
                    optional_sound_committed: self.optional_sound_committed,
                    reason: AimAndFireEmitterBlockReason::External {
                        phase: expected_phase,
                        reason,
                    },
                });
                Ok(())
            }
            (
                AimAndFireEmitterStage::PlayOptionalSound(_),
                AimAndFireEmitterResume::Acknowledged { .. },
            ) => {
                self.outstanding = None;
                self.optional_sound_committed = true;
                self.start_child_or_complete();
                Ok(())
            }
            (
                AimAndFireEmitterStage::GenericEmitter,
                AimAndFireEmitterResume::GenericEmitter {
                    child_transaction_id,
                    response,
                    ..
                },
            ) => {
                if child_transaction_id != self.child_transaction_id {
                    return Err(AimAndFireEmitterResumeFailure {
                        error: AimAndFireEmitterProtocolError::ChildTransactionMismatch {
                            expected: self.child_transaction_id,
                            actual: child_transaction_id,
                        },
                        receipt,
                    });
                }
                let child_receipt = self
                    .outstanding
                    .as_mut()
                    .and_then(|outstanding| outstanding.child_receipt.take())
                    .expect("a wrapped child action retains its child receipt");
                match self
                    .child
                    .as_mut()
                    .expect("generic-emitter stage owns a child")
                    .resume(child_receipt, response)
                {
                    Ok(()) => {
                        self.outstanding = None;
                        Ok(())
                    }
                    Err(failure) => {
                        self.outstanding
                            .as_mut()
                            .expect("failed child response leaves parent outstanding")
                            .child_receipt = Some(failure.receipt);
                        Err(AimAndFireEmitterResumeFailure {
                            receipt,
                            error: AimAndFireEmitterProtocolError::ChildProtocol(failure.error),
                        })
                    }
                }
            }
            _ => Err(AimAndFireEmitterResumeFailure {
                error: AimAndFireEmitterProtocolError::UnexpectedResumeKind {
                    phase: expected_phase,
                },
                receipt,
            }),
        }
    }

    fn start_child_or_complete(&mut self) {
        let Some(request) = self.mapped_child_request else {
            self.stage = AimAndFireEmitterStage::Complete(AimAndFireEmitterCompletion {
                lease: self.lease,
                committed_prefix: self.committed_prefix,
                callback_result: AimAndFireCallbackResult::Zero,
                optional_sound_committed: self.optional_sound_committed,
            });
            return;
        };
        self.child = Some(GenericProjectileEmitterMachine::start(
            self.child_transaction_id,
            request,
        ));
        self.stage = AimAndFireEmitterStage::GenericEmitter;
    }

    fn issue(
        &mut self,
        phase: AimAndFireEmitterPhase,
        action: AimAndFireEmitterAction,
        child_receipt: Option<GenericEmitterReceipt>,
    ) -> AimAndFireEmitterPoll {
        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one Aim-and-Fire callback cannot issue u64::MAX actions");
        self.outstanding = Some(OutstandingAimAndFireEmitterAction {
            sequence: action_sequence,
            phase,
            child_receipt,
        });
        AimAndFireEmitterPoll::Action(IssuedAimAndFireEmitterAction {
            receipt: AimAndFireEmitterReceipt {
                transaction_id: self.transaction_id,
                lease: self.lease,
                action_sequence,
            },
            action,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::{
        actor_task_owner::{
            ActorTaskOwner, ActorTaskSlot, ActorTaskVisitControl, PreparedActorTask,
        },
        aim_and_fire::{
            AimAndFireInvalidTargetReason, AimAndFireLifetimeStatus, AimAndFireTargetRuntimeState,
            AIM_AND_FIRE_DYING_STATE_BIT,
        },
        generic_projectile_emitter::{
            GenericEmitterEntitySnapshot, GenericEmitterExternalBlock,
            GenericEmitterManualBasisSolution, GenericEmitterManualLaunchSolution,
        },
    };

    const OWNER: u32 = 0x0497_0001;
    const TARGET: u32 = 0x047F_0001;
    const PARENT_ID_RAW: u64 = 0xAA10_0000_0000_0001;
    const CHILD_ID_RAW: u64 = 0xEE10_0000_0000_0001;

    fn parent_id(raw: u64) -> AimAndFireEmitterTransactionId {
        AimAndFireEmitterTransactionId::new(raw).unwrap()
    }

    fn child_id(raw: u64) -> GenericEmitterTransactionId {
        GenericEmitterTransactionId::new(raw).unwrap()
    }

    fn visit(slot: ActorTaskSlot) -> ActorTaskVisit {
        let mut owner = ActorTaskOwner::new();
        owner.replace_prepared(slot, PreparedActorTask::new(()));
        let mut captured = None;
        owner.visit_slots_fresh(|_, visit| {
            captured = Some(visit);
            ActorTaskVisitControl::<()>::Continue
        });
        captured.unwrap()
    }

    fn lease() -> AimAndFireEmitterLease {
        AimAndFireEmitterLease {
            owner_entity_id: OWNER,
            task_visit: visit(ActorTaskSlot::Tertiary),
        }
    }

    fn prefix() -> AimAndFireCallbackPrefix {
        AimAndFireCallbackPrefix {
            elapsed_ms: 123,
            lifetime_status: AimAndFireLifetimeStatus::WithinLifetime,
        }
    }

    fn private(sound_id: u32, period_us: i32) -> AimAndFirePrivateState {
        AimAndFirePrivateState::new([10, 20, 30], TARGET, sound_id, period_us)
    }

    fn descriptor() -> ProjectileEmitterDescriptor {
        ProjectileEmitterDescriptor {
            projectile_method: 30,
            random_interval_us: 750_000,
            spread_raw: 100,
            aim_threshold_raw: 16_000,
            speed_override_raw: 321,
            target_axis_tolerance_raw: 0x0600,
            sound_id: 70,
            raw_word_at_0x12: 150,
            alternate_emitter_raw: 1,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [1, 2, 3, 4],
        }
    }

    fn runtime(direct_mode: u32, remaining_time_raw: i32) -> GenericEmitterRuntime {
        GenericEmitterRuntime {
            joint_bindings: [None; 2],
            projectile_method: 30,
            emitter_selector: 1,
            sound_id: 0,
            direct_mode,
            remaining_time_raw,
            manual_step_raw: 20_000,
            remaining_bursts_raw: 1,
            cadence_raw: 10_000,
            basis_adjustment_identity: Some(0xABCD),
        }
    }

    fn request(
        target_state: AimAndFireTargetRuntimeState,
        descriptor: Option<ProjectileEmitterDescriptor>,
        runtime: GenericEmitterRuntime,
    ) -> AimAndFireFrameRequest<ProjectileEmitterDescriptor, GenericEmitterRuntime> {
        AimAndFireFrameRequest {
            owner_entity_id: OWNER,
            elapsed_micros: 20_000,
            scheduler_mode: 0,
            target_state,
            owner_sound_position_raw: Some([100, 200, 300]),
            sub_e_descriptor: descriptor,
            emitter_runtime: runtime,
        }
    }

    fn start(
        sound_id: u32,
        period_us: i32,
        descriptor: Option<ProjectileEmitterDescriptor>,
        runtime: GenericEmitterRuntime,
    ) -> AimAndFireEmitterMachine {
        AimAndFireEmitterMachine::start(
            parent_id(PARENT_ID_RAW),
            child_id(CHILD_ID_RAW),
            lease(),
            prefix(),
            private(sound_id, period_us),
            request(
                AimAndFireTargetRuntimeState::Present { state_flags: 1 },
                descriptor,
                runtime,
            ),
        )
        .unwrap()
    }

    fn issue(machine: &mut AimAndFireEmitterMachine) -> IssuedAimAndFireEmitterAction {
        match machine.poll() {
            AimAndFireEmitterPoll::Action(issued) => issued,
            other => panic!("expected action, got {other:?}"),
        }
    }

    fn child_response(
        phase: GenericEmitterPhase,
        response: GenericEmitterResume,
    ) -> AimAndFireEmitterResume {
        AimAndFireEmitterResume::GenericEmitter {
            phase: AimAndFireEmitterPhase::GenericEmitter(phase),
            child_transaction_id: child_id(CHILD_ID_RAW),
            response,
        }
    }

    fn entity(handle: u32) -> GenericEmitterEntitySnapshot {
        GenericEmitterEntitySnapshot {
            handle,
            allocation_identity: 0x1000_0000_0000 | u64::from(handle),
            state_flags_at_0x08: 1,
            position_raw: [100, 200, 300],
            velocity_raw: [4, 5, 6],
            orientation_raw: [7, 8, 9],
        }
    }

    #[test]
    fn scheduler_and_invalid_target_paths_never_start_or_issue_a_child() {
        let mut scheduler_request = request(
            AimAndFireTargetRuntimeState::Present { state_flags: 1 },
            Some(descriptor()),
            runtime(1, 1),
        );
        scheduler_request.scheduler_mode = 7;
        let scheduler_lease = lease();
        let mut scheduler = AimAndFireEmitterMachine::start(
            parent_id(PARENT_ID_RAW),
            child_id(CHILD_ID_RAW),
            scheduler_lease,
            prefix(),
            private(70, 0x400),
            scheduler_request,
        )
        .unwrap();
        assert_eq!(scheduler.mapped_child_request(), None);
        assert_eq!(
            scheduler.poll(),
            AimAndFireEmitterPoll::Complete(AimAndFireEmitterCompletion {
                lease: scheduler_lease,
                committed_prefix: prefix(),
                callback_result: AimAndFireCallbackResult::Zero,
                optional_sound_committed: false,
            })
        );

        let cases = [
            (
                AimAndFireTargetRuntimeState::Missing,
                AimAndFireInvalidTargetReason::Missing,
            ),
            (
                AimAndFireTargetRuntimeState::Present { state_flags: 0 },
                AimAndFireInvalidTargetReason::ZeroStateFlags,
            ),
            (
                AimAndFireTargetRuntimeState::Present {
                    state_flags: AIM_AND_FIRE_DYING_STATE_BIT | 1,
                },
                AimAndFireInvalidTargetReason::Dying,
            ),
        ];
        for (target_state, reason) in cases {
            let current_lease = lease();
            let mut machine = AimAndFireEmitterMachine::start(
                parent_id(PARENT_ID_RAW),
                child_id(CHILD_ID_RAW),
                current_lease,
                prefix(),
                private(70, 0x400),
                request(target_state, Some(descriptor()), runtime(1, 1)),
            )
            .unwrap();
            assert_eq!(machine.mapped_child_request(), None);
            assert_eq!(
                machine.poll(),
                AimAndFireEmitterPoll::Complete(AimAndFireEmitterCompletion {
                    lease: current_lease,
                    committed_prefix: prefix(),
                    callback_result: AimAndFireCallbackResult::TaggedInvalidTarget {
                        singleton: AimAndFireTaggedSingleton::InvalidTarget,
                        reason,
                    },
                    optional_sound_committed: false,
                })
            );
        }
    }

    #[test]
    fn start_rejects_shared_identity_and_owner_lease_mismatch() {
        let shared = 0x1234;
        assert_eq!(
            AimAndFireEmitterMachine::start(
                parent_id(shared),
                child_id(shared),
                lease(),
                prefix(),
                private(0, 0),
                request(
                    AimAndFireTargetRuntimeState::Present { state_flags: 1 },
                    None,
                    runtime(1, 0),
                ),
            )
            .unwrap_err(),
            AimAndFireEmitterStartError::ParentAndChildTransactionIdsMustDiffer {
                shared_raw: shared
            }
        );

        let bad_lease = AimAndFireEmitterLease {
            owner_entity_id: OWNER + 1,
            task_visit: visit(ActorTaskSlot::Tertiary),
        };
        assert_eq!(
            AimAndFireEmitterMachine::start(
                parent_id(PARENT_ID_RAW),
                child_id(CHILD_ID_RAW),
                bad_lease,
                prefix(),
                private(0, 0),
                request(
                    AimAndFireTargetRuntimeState::Present { state_flags: 1 },
                    None,
                    runtime(1, 0),
                ),
            )
            .unwrap_err(),
            AimAndFireEmitterStartError::LeaseOwnerMismatch {
                lease_owner: OWNER + 1,
                request_owner: OWNER,
            }
        );
    }

    #[test]
    fn seven_argument_mapping_is_exact_and_child_identity_remains_explicit() {
        let mut machine = start(0, 0, Some(descriptor()), runtime(1, 0));
        assert_eq!(
            machine.mapped_child_request(),
            Some(GenericEmitterRequest {
                source_handle: OWNER,
                owner_handle: OWNER,
                target_handle: TARGET,
                explicit_target_position_raw: None,
                descriptor: descriptor(),
                runtime: runtime(1, 0),
                elapsed_us: 20_000,
            })
        );
        assert_eq!(machine.child_transaction_id(), child_id(CHILD_ID_RAW));

        let issued = issue(&mut machine);
        assert_eq!(issued.receipt.lease(), machine.lease());
        assert!(matches!(
            issued.action,
            AimAndFireEmitterAction::GenericEmitter {
                child_transaction_id,
                action: GenericEmitterAction::LookupEntity {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    handle: OWNER,
                },
                ..
            } if child_transaction_id == child_id(CHILD_ID_RAW)
        ));
    }

    #[test]
    fn selected_optional_sound_is_durable_and_strictly_precedes_child() {
        let mut machine = start(70, 0x400, Some(descriptor()), runtime(1, 0));
        let random = issue(&mut machine);
        assert!(matches!(
            random.action,
            AimAndFireEmitterAction::DrawSharedRandom { .. }
        ));
        assert_eq!(
            machine.poll(),
            AimAndFireEmitterPoll::Awaiting(AimAndFireEmitterPhase::DrawOptionalSoundRandom)
        );
        machine
            .resume(
                random.receipt,
                AimAndFireEmitterResume::RandomDrawn {
                    phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                    value: 0,
                },
            )
            .unwrap();

        let sound = issue(&mut machine);
        assert!(matches!(
            sound.action,
            AimAndFireEmitterAction::PlayOptionalSound {
                effect: AimAndFireOptionalSoundEffect {
                    owner_position_raw: Some([100, 200, 300]),
                    ..
                },
                ..
            }
        ));
        machine
            .resume(
                sound.receipt,
                AimAndFireEmitterResume::Acknowledged {
                    phase: AimAndFireEmitterPhase::PlayOptionalSound,
                },
            )
            .unwrap();

        let child = issue(&mut machine);
        assert!(matches!(
            child.action,
            AimAndFireEmitterAction::GenericEmitter {
                action: GenericEmitterAction::LookupEntity {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    ..
                },
                ..
            }
        ));
        machine
            .resume(
                child.receipt,
                child_response(
                    GenericEmitterPhase::LookupEntrySource,
                    GenericEmitterResume::Blocked {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
                    },
                ),
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            AimAndFireEmitterPoll::Blocked(AimAndFireEmitterBlock {
                lease: machine.lease(),
                committed_prefix: prefix(),
                optional_sound_committed: true,
                reason: AimAndFireEmitterBlockReason::GenericEmitter(
                    GenericEmitterBlock::External {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
                    }
                ),
            })
        );
    }

    #[test]
    fn accepted_child_block_is_preserved_as_a_durable_parent_block() {
        let mut machine = start(0, 0, Some(descriptor()), runtime(1, 0));
        let child = issue(&mut machine);
        assert!(matches!(
            child.action,
            AimAndFireEmitterAction::GenericEmitter {
                action: GenericEmitterAction::LookupEntity {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    ..
                },
                ..
            }
        ));
        machine
            .resume(
                child.receipt,
                child_response(
                    GenericEmitterPhase::LookupEntrySource,
                    GenericEmitterResume::Blocked {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
                    },
                ),
            )
            .unwrap();

        let block = AimAndFireEmitterBlock {
            lease: machine.lease(),
            committed_prefix: prefix(),
            optional_sound_committed: false,
            reason: AimAndFireEmitterBlockReason::GenericEmitter(GenericEmitterBlock::External {
                phase: GenericEmitterPhase::LookupEntrySource,
                reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
            }),
        };
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Blocked(block));
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Blocked(block));
    }

    #[test]
    fn rejected_sound_draw_starts_child_without_a_sound_action() {
        let mut machine = start(70, 750_000, Some(descriptor()), runtime(1, 0));
        let random = issue(&mut machine);
        machine
            .resume(
                random.receipt,
                AimAndFireEmitterResume::RandomDrawn {
                    phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                    value: u16::MAX.into(),
                },
            )
            .unwrap();
        assert!(matches!(
            issue(&mut machine).action,
            AimAndFireEmitterAction::GenericEmitter { .. }
        ));
    }

    #[test]
    fn sound_can_commit_without_sub_e_then_callback_returns_zero() {
        let mut machine = start(70, 0x400, None, runtime(1, 0));
        let random = issue(&mut machine);
        machine
            .resume(
                random.receipt,
                AimAndFireEmitterResume::RandomDrawn {
                    phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                    value: 0,
                },
            )
            .unwrap();
        let sound = issue(&mut machine);
        machine
            .resume(
                sound.receipt,
                AimAndFireEmitterResume::Acknowledged {
                    phase: AimAndFireEmitterPhase::PlayOptionalSound,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            AimAndFireEmitterPoll::Complete(AimAndFireEmitterCompletion {
                lease: machine.lease(),
                committed_prefix: prefix(),
                callback_result: AimAndFireCallbackResult::Zero,
                optional_sound_committed: true,
            })
        );
    }

    #[test]
    fn child_zero_maps_once_to_callback_zero_before_unwind() {
        let mut machine = start(0, 0, Some(descriptor()), runtime(1, 0));
        let lookup = issue(&mut machine);
        machine
            .resume(
                lookup.receipt,
                child_response(
                    GenericEmitterPhase::LookupEntrySource,
                    GenericEmitterResume::EntityLookupResolved {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        entity: None,
                    },
                ),
            )
            .unwrap();
        let cadence = issue(&mut machine);
        machine
            .resume(
                cadence.receipt,
                child_response(
                    GenericEmitterPhase::CommitInitialCadence,
                    GenericEmitterResume::Acknowledged {
                        phase: GenericEmitterPhase::CommitInitialCadence,
                    },
                ),
            )
            .unwrap();

        let completion = AimAndFireEmitterCompletion {
            lease: machine.lease(),
            committed_prefix: prefix(),
            callback_result: AimAndFireCallbackResult::Zero,
            optional_sound_committed: false,
        };
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Complete(completion));
        assert_eq!(
            machine.poll(),
            AimAndFireEmitterPoll::Complete(completion),
            "terminal observation cannot restart the child"
        );
    }

    #[test]
    fn child_nonzero_bits_propagate_once_before_parent_unwind() {
        let mut machine = start(0, 0, Some(descriptor()), runtime(1, 1));
        let source = entity(OWNER);

        let lookup = issue(&mut machine);
        machine
            .resume(
                lookup.receipt,
                child_response(
                    GenericEmitterPhase::LookupEntrySource,
                    GenericEmitterResume::EntityLookupResolved {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        entity: Some(source),
                    },
                ),
            )
            .unwrap();
        let cadence = issue(&mut machine);
        machine
            .resume(
                cadence.receipt,
                child_response(
                    GenericEmitterPhase::CommitInitialCadence,
                    GenericEmitterResume::Acknowledged {
                        phase: GenericEmitterPhase::CommitInitialCadence,
                    },
                ),
            )
            .unwrap();
        let angle = issue(&mut machine);
        machine
            .resume(
                angle.receipt,
                child_response(
                    GenericEmitterPhase::ReadManualAngleBinding,
                    GenericEmitterResume::ManualAngleBindingRead {
                        phase: GenericEmitterPhase::ReadManualAngleBinding,
                        angle_raw: None,
                    },
                ),
            )
            .unwrap();
        let refresh = issue(&mut machine);
        machine
            .resume(
                refresh.receipt,
                child_response(
                    GenericEmitterPhase::RefreshCachedSource,
                    GenericEmitterResume::CachedSourceRefreshed {
                        phase: GenericEmitterPhase::RefreshCachedSource,
                        entity: Some(source),
                    },
                ),
            )
            .unwrap();
        let basis = issue(&mut machine);
        machine
            .resume(
                basis.receipt,
                child_response(
                    GenericEmitterPhase::ResolveManualBasis,
                    GenericEmitterResume::ManualBasisResolved {
                        phase: GenericEmitterPhase::ResolveManualBasis,
                        solution: GenericEmitterManualBasisSolution {
                            basis_raw: [1; 9],
                            sine_raw: 2,
                            cosine_raw: 3,
                        },
                    },
                ),
            )
            .unwrap();
        let launch = issue(&mut machine);
        machine
            .resume(
                launch.receipt,
                child_response(
                    GenericEmitterPhase::ResolveManualLaunch,
                    GenericEmitterResume::ManualLaunchResolved {
                        phase: GenericEmitterPhase::ResolveManualLaunch,
                        solution: GenericEmitterManualLaunchSolution {
                            direction_raw: [11, 22, 33],
                        },
                    },
                ),
            )
            .unwrap();
        let append = issue(&mut machine);
        let result = NonZeroU32::new(0x00AB_CDEF).unwrap();
        machine
            .resume(
                append.receipt,
                child_response(
                    GenericEmitterPhase::AppendPrimary,
                    GenericEmitterResume::AppendReturned {
                        phase: GenericEmitterPhase::AppendPrimary,
                        result_raw: result.get(),
                    },
                ),
            )
            .unwrap();

        let completion = AimAndFireEmitterCompletion {
            lease: machine.lease(),
            committed_prefix: prefix(),
            callback_result: AimAndFireCallbackResult::ReturnGenericEmitterResult(result),
            optional_sound_committed: false,
        };
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Complete(completion));
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Complete(completion));
    }

    #[test]
    fn protocol_mismatches_keep_both_parent_and_child_receipts_retryable() {
        let mut machine = start(0, 0, Some(descriptor()), runtime(1, 0));
        let issued = issue(&mut machine);

        let failure = machine
            .resume(
                issued.receipt,
                AimAndFireEmitterResume::GenericEmitter {
                    phase: AimAndFireEmitterPhase::GenericEmitter(
                        GenericEmitterPhase::LookupEntrySource,
                    ),
                    child_transaction_id: child_id(CHILD_ID_RAW + 1),
                    response: GenericEmitterResume::EntityLookupResolved {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        entity: None,
                    },
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            AimAndFireEmitterProtocolError::ChildTransactionMismatch {
                expected: child_id(CHILD_ID_RAW),
                actual: child_id(CHILD_ID_RAW + 1),
            }
        );

        let failure = machine
            .resume(
                failure.receipt,
                child_response(
                    GenericEmitterPhase::LookupEntrySource,
                    GenericEmitterResume::Acknowledged {
                        phase: GenericEmitterPhase::LookupEntrySource,
                    },
                ),
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            AimAndFireEmitterProtocolError::ChildProtocol(
                GenericEmitterProtocolError::UnexpectedResumeKind {
                    phase: GenericEmitterPhase::LookupEntrySource
                }
            )
        ));

        machine
            .resume(
                failure.receipt,
                child_response(
                    GenericEmitterPhase::LookupEntrySource,
                    GenericEmitterResume::EntityLookupResolved {
                        phase: GenericEmitterPhase::LookupEntrySource,
                        entity: None,
                    },
                ),
            )
            .unwrap();
        assert!(matches!(
            issue(&mut machine).action,
            AimAndFireEmitterAction::GenericEmitter {
                action: GenericEmitterAction::CommitInitialCadence { .. },
                ..
            }
        ));
    }

    #[test]
    fn parent_transaction_lease_sequence_phase_and_replay_mismatches_fail_closed() {
        let mut machine = start(70, 0x400, None, runtime(1, 0));
        let issued = issue(&mut machine);
        let duplicate = AimAndFireEmitterReceipt {
            transaction_id: issued.receipt.transaction_id,
            lease: issued.receipt.lease,
            action_sequence: issued.receipt.action_sequence,
        };

        let wrong_transaction = AimAndFireEmitterReceipt {
            transaction_id: parent_id(PARENT_ID_RAW + 1),
            lease: issued.receipt.lease,
            action_sequence: issued.receipt.action_sequence,
        };
        assert!(matches!(
            machine
                .resume(
                    wrong_transaction,
                    AimAndFireEmitterResume::RandomDrawn {
                        phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                        value: 0,
                    },
                )
                .unwrap_err()
                .error,
            AimAndFireEmitterProtocolError::ReceiptTransactionMismatch { .. }
        ));

        let wrong_lease = AimAndFireEmitterReceipt {
            transaction_id: issued.receipt.transaction_id,
            lease: AimAndFireEmitterLease {
                owner_entity_id: OWNER,
                task_visit: visit(ActorTaskSlot::Secondary),
            },
            action_sequence: issued.receipt.action_sequence,
        };
        assert!(matches!(
            machine
                .resume(
                    wrong_lease,
                    AimAndFireEmitterResume::RandomDrawn {
                        phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                        value: 0,
                    },
                )
                .unwrap_err()
                .error,
            AimAndFireEmitterProtocolError::ReceiptLeaseMismatch { .. }
        ));

        let wrong_sequence = AimAndFireEmitterReceipt {
            transaction_id: issued.receipt.transaction_id,
            lease: issued.receipt.lease,
            action_sequence: issued.receipt.action_sequence + 1,
        };
        assert!(matches!(
            machine
                .resume(
                    wrong_sequence,
                    AimAndFireEmitterResume::RandomDrawn {
                        phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                        value: 0,
                    },
                )
                .unwrap_err()
                .error,
            AimAndFireEmitterProtocolError::ReceiptSequenceMismatch { .. }
        ));

        let failure = machine
            .resume(
                issued.receipt,
                AimAndFireEmitterResume::Acknowledged {
                    phase: AimAndFireEmitterPhase::PlayOptionalSound,
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            AimAndFireEmitterProtocolError::PhaseMismatch { .. }
        ));
        machine
            .resume(
                failure.receipt,
                AimAndFireEmitterResume::RandomDrawn {
                    phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                    value: u16::MAX.into(),
                },
            )
            .unwrap();

        assert!(matches!(
            machine
                .resume(
                    duplicate,
                    AimAndFireEmitterResume::RandomDrawn {
                        phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                        value: 0,
                    },
                )
                .unwrap_err()
                .error,
            AimAndFireEmitterProtocolError::NoOutstandingAction { .. }
        ));
    }

    #[test]
    fn parent_external_block_is_durable_and_never_constructs_child() {
        let mut machine = start(70, 0x400, Some(descriptor()), runtime(1, 1));
        let issued = issue(&mut machine);
        machine
            .resume(
                issued.receipt,
                AimAndFireEmitterResume::Blocked {
                    phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                    reason: AimAndFireEmitterExternalBlock::SharedRandomUnavailable,
                },
            )
            .unwrap();
        let block = AimAndFireEmitterBlock {
            lease: machine.lease(),
            committed_prefix: prefix(),
            optional_sound_committed: false,
            reason: AimAndFireEmitterBlockReason::External {
                phase: AimAndFireEmitterPhase::DrawOptionalSoundRandom,
                reason: AimAndFireEmitterExternalBlock::SharedRandomUnavailable,
            },
        };
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Blocked(block));
        assert_eq!(machine.poll(), AimAndFireEmitterPoll::Blocked(block));
    }
}
