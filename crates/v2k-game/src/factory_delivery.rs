//! Receipt-bound Working Factory scientist delivery.
//!
//! Retail `FUN_00425850` is the behavior callback used when an eligible
//! scientist reaches the Working Factory/lifter.  Its accepted branch is
//! deliberately sequential and non-rollback:
//!
//! 1. emit controller operation `0x33` at the scientist position, sourced by
//!    the lifter/factory;
//! 2. queue deduplicated HUD resource event `5`;
//! 3. apply `FUN_00418C20(factory, 1)`, including any capacity presentation;
//! 4. for a remote-owned scientist, emit entity feedback `(1, 8)`;
//! 5. queue deferred scientist destruction;
//! 6. return the static callback object tagged `0xA300`.
//!
//! This module closes every callback gate against one authenticated
//! factory/scientist identity-and-version snapshot before issuing the first
//! action.  Once accepted, actions are not rolled back if a later external
//! boundary blocks.  Linear receipts prevent replay and bind every
//! acknowledgement to the same transaction and pair snapshot.
//!
//! The terminal `0xA300` value is only the behavior-callback return.  The
//! surrounding active-pair coordinator still owns return interpretation,
//! component traversal, candidate behavior, and the physical/damage suffix.

use std::num::NonZeroU64;

use crate::entity_collision_state::RetailRuntimeValue;
use crate::entity_pair_callbacks::{
    plan_lifter_contact, EntityPairCallbackAction, EntityPairCallbackUnresolved,
    EntityPairTaggedEffect, LifterContactInput, LifterDeliverySnapshot, LifterFactoryState,
    LifterSourceSnapshot, PairBehaviorCallbackReturn, PairCallbackEntitySnapshot,
};
use crate::factory_production::{FactoryProductionRuntime, FactoryStaffingChange};

/// Deduplicated HUD resource event selected by the accepted callback.
pub const FACTORY_DELIVERY_HUD_RESOURCE_EVENT_ID: u8 = 5;
/// Deduplicated HUD resource queued when a visible staffing change fills the
/// factory.
pub const FACTORY_CAPACITY_HUD_RESOURCE_EVENT_ID: u8 = 2;
/// Remote-entity feedback channel emitted before deferred destruction.
pub const FACTORY_DELIVERY_REMOTE_FEEDBACK_CHANNEL: u8 = 1;
/// Remote-entity feedback code emitted before deferred destruction.
pub const FACTORY_DELIVERY_REMOTE_FEEDBACK_CODE: u8 = 8;

/// Caller-owned identity for one detached callback transaction.
///
/// The host must keep this identity unique across every simultaneously live
/// factory-delivery machine and any adapter journal entries retained for it.
/// A stable entity pair is not a transaction identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryDeliveryTransactionId(NonZeroU64);

impl FactoryDeliveryTransactionId {
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

/// One entity allocation and the state generation authenticated by preflight.
///
/// The runtime bridge chooses the version domain. It may be a collision-state
/// generation, an entity-table epoch, or another monotonically changing token,
/// but it must change whenever fields used by this callback can become stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryDeliveryEntityVersion {
    pub entity_id: u32,
    pub allocation_identity: u64,
    pub state_version: u64,
}

/// Pair identity stamped into every linear receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryDeliveryPairVersion {
    pub factory: FactoryDeliveryEntityVersion,
    pub scientist: FactoryDeliveryEntityVersion,
}

/// Authenticated scientist state consumed by `FUN_00425850`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryDeliveryScientistSnapshot {
    pub entity: PairCallbackEntitySnapshot,
    pub allocation_identity: u64,
    pub state_version: u64,
}

impl FactoryDeliveryScientistSnapshot {
    pub const fn version(self) -> FactoryDeliveryEntityVersion {
        FactoryDeliveryEntityVersion {
            entity_id: self.entity.id,
            allocation_identity: self.allocation_identity,
            state_version: self.state_version,
        }
    }
}

/// Authenticated Working Factory/lifter state consumed by the callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryDeliveryFactorySnapshot {
    pub lifter: LifterSourceSnapshot,
    pub allocation_identity: u64,
    pub state_version: u64,
    pub type_extension_present: bool,
    pub behavior_state_present: bool,
    pub delivery_enabled: bool,
    pub production: FactoryProductionRuntime,
}

impl FactoryDeliveryFactorySnapshot {
    pub const fn version(self) -> FactoryDeliveryEntityVersion {
        FactoryDeliveryEntityVersion {
            entity_id: self.lifter.id,
            allocation_identity: self.allocation_identity,
            state_version: self.state_version,
        }
    }
}

/// Complete immutable admission snapshot for one scientist delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryDeliveryRequest {
    pub factory: FactoryDeliveryFactorySnapshot,
    pub scientist: FactoryDeliveryScientistSnapshot,
}

impl FactoryDeliveryRequest {
    pub const fn pair_version(self) -> FactoryDeliveryPairVersion {
        FactoryDeliveryPairVersion {
            factory: self.factory.version(),
            scientist: self.scientist.version(),
        }
    }
}

/// Admission result before any non-rollback action has escaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryDeliveryAdmissionRejection {
    /// The retail callback returned null after one of its ordinary capability,
    /// lifetime, ownership, helper, enablement, or capacity gates.
    CallbackReturnedNull,
    /// A future planner change exposed an unresolved boundary despite the
    /// complete concrete request supplied here.
    CallbackUnresolved(EntityPairCallbackUnresolved),
    /// The shared callback planner no longer describes the exact bounded
    /// journal audited for this detached executor.
    UnexpectedCallbackJournal,
}

/// Externally visible phase in retail action order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactoryDeliveryPhase {
    EmitOperation33,
    QueueDeliveryHudResource,
    CommitStaffing,
    PublishCapacityCountdown,
    EmitCapacityDirectText,
    QueueCapacityHudResource,
    EmitRemoteScientistFeedback,
    QueueDeferredScientistDestroy,
}

/// Pure `FUN_00418C20(factory, 1)` mutation prepared at the exact staffing
/// point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryDeliveryStaffingCommit {
    pub before: FactoryProductionRuntime,
    pub after: FactoryProductionRuntime,
    pub change: FactoryStaffingChange,
}

/// One ordered external action.
#[derive(Debug, PartialEq, Eq)]
pub enum FactoryDeliveryAction {
    EmitOperation33 {
        phase: FactoryDeliveryPhase,
        position_raw: [i16; 3],
        source_factory_entity_id: u32,
        remote_owned: bool,
    },
    QueueDeliveryHudResource {
        phase: FactoryDeliveryPhase,
        event_id: u8,
    },
    CommitStaffing {
        phase: FactoryDeliveryPhase,
        factory: FactoryDeliveryEntityVersion,
        commit: FactoryDeliveryStaffingCommit,
    },
    /// `FUN_00456BD0(0)` precedes direct text when a factory with an active
    /// understaffed limit becomes full.
    PublishCapacityCountdown {
        phase: FactoryDeliveryPhase,
        remaining_seconds_raw: i32,
    },
    EmitCapacityDirectText {
        phase: FactoryDeliveryPhase,
        direct_text_id: u16,
        sub_parameter: u32,
    },
    QueueCapacityHudResource {
        phase: FactoryDeliveryPhase,
        event_id: u8,
    },
    EmitRemoteScientistFeedback {
        phase: FactoryDeliveryPhase,
        scientist_entity_id: u32,
        channel: u8,
        code: u8,
    },
    QueueDeferredScientistDestroy {
        phase: FactoryDeliveryPhase,
        scientist_entity_id: u32,
    },
}

impl FactoryDeliveryAction {
    pub const fn phase(&self) -> FactoryDeliveryPhase {
        match self {
            Self::EmitOperation33 { phase, .. }
            | Self::QueueDeliveryHudResource { phase, .. }
            | Self::CommitStaffing { phase, .. }
            | Self::PublishCapacityCountdown { phase, .. }
            | Self::EmitCapacityDirectText { phase, .. }
            | Self::QueueCapacityHudResource { phase, .. }
            | Self::EmitRemoteScientistFeedback { phase, .. }
            | Self::QueueDeferredScientistDestroy { phase, .. } => *phase,
        }
    }
}

/// Linear proof that one exact pair action was issued.
///
/// Private fields and the absence of `Copy`/`Clone` make a production receipt
/// non-duplicable. The only production constructor is
/// [`FactoryDeliveryMachine::poll`].
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryDeliveryReceipt {
    transaction_id: FactoryDeliveryTransactionId,
    action_sequence: u64,
    pair_version: FactoryDeliveryPairVersion,
}

impl FactoryDeliveryReceipt {
    pub const fn transaction_id(&self) -> FactoryDeliveryTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }

    pub const fn pair_version(&self) -> FactoryDeliveryPairVersion {
        self.pair_version
    }
}

/// One action paired with its non-duplicable acknowledgement token.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedFactoryDeliveryAction {
    pub receipt: FactoryDeliveryReceipt,
    pub action: FactoryDeliveryAction,
}

/// Adapter-side boundary which could not apply an already-preflighted action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryDeliveryExternalBlock {
    Operation33Unavailable,
    DeliveryHudResourceUnavailable,
    StaffingCommitUnavailable,
    CapacityCountdownUnavailable,
    CapacityDirectTextUnavailable,
    CapacityHudResourceUnavailable,
    RemoteScientistFeedbackUnavailable,
    DeferredDestroyUnavailable,
}

/// Terminal block after zero or more earlier actions became durable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryDeliveryBlock {
    pub pair_version: FactoryDeliveryPairVersion,
    pub phase: FactoryDeliveryPhase,
    pub reason: FactoryDeliveryExternalBlock,
}

/// Adapter response to one issued action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryDeliveryResume {
    Acknowledged {
        phase: FactoryDeliveryPhase,
    },
    Blocked {
        phase: FactoryDeliveryPhase,
        reason: FactoryDeliveryExternalBlock,
    },
}

impl FactoryDeliveryResume {
    pub const fn phase(self) -> FactoryDeliveryPhase {
        match self {
            Self::Acknowledged { phase } | Self::Blocked { phase, .. } => phase,
        }
    }
}

/// The callback completed, but the active-pair suffix remains unclaimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryDeliveryPairSuffix {
    Unclaimed,
}

/// Exact behavior-callback completion after deferred destruction was queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryDeliveryCompletion {
    pub pair_version: FactoryDeliveryPairVersion,
    pub callback_return: PairBehaviorCallbackReturn,
    pub pair_suffix: FactoryDeliveryPairSuffix,
    pub factory_runtime: FactoryProductionRuntime,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FactoryDeliveryPoll {
    Action(IssuedFactoryDeliveryAction),
    Awaiting(FactoryDeliveryPhase),
    Blocked(FactoryDeliveryBlock),
    Complete(FactoryDeliveryCompletion),
}

/// Recoverable receipt/protocol rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryDeliveryProtocolError {
    NoOutstandingAction {
        transaction_id: FactoryDeliveryTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: FactoryDeliveryTransactionId,
        actual: FactoryDeliveryTransactionId,
    },
    ReceiptPairMismatch {
        expected: FactoryDeliveryPairVersion,
        actual: FactoryDeliveryPairVersion,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: FactoryDeliveryPhase,
        actual: FactoryDeliveryPhase,
    },
}

/// A rejected response returns the still-live receipt intact. No stage,
/// staffing, or sequence state changes on this path.
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryDeliveryResumeFailure {
    pub receipt: FactoryDeliveryReceipt,
    pub error: FactoryDeliveryProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactoryDeliveryStage {
    EmitOperation33,
    QueueDeliveryHudResource,
    CommitStaffing(FactoryDeliveryStaffingCommit),
    PublishCapacityCountdown,
    EmitCapacityDirectText,
    QueueCapacityHudResource,
    EmitRemoteScientistFeedback,
    QueueDeferredScientistDestroy,
    Blocked(FactoryDeliveryBlock),
    Complete,
}

impl FactoryDeliveryStage {
    const fn phase(self) -> Option<FactoryDeliveryPhase> {
        Some(match self {
            Self::EmitOperation33 => FactoryDeliveryPhase::EmitOperation33,
            Self::QueueDeliveryHudResource => FactoryDeliveryPhase::QueueDeliveryHudResource,
            Self::CommitStaffing(_) => FactoryDeliveryPhase::CommitStaffing,
            Self::PublishCapacityCountdown => FactoryDeliveryPhase::PublishCapacityCountdown,
            Self::EmitCapacityDirectText => FactoryDeliveryPhase::EmitCapacityDirectText,
            Self::QueueCapacityHudResource => FactoryDeliveryPhase::QueueCapacityHudResource,
            Self::EmitRemoteScientistFeedback => FactoryDeliveryPhase::EmitRemoteScientistFeedback,
            Self::QueueDeferredScientistDestroy => {
                FactoryDeliveryPhase::QueueDeferredScientistDestroy
            }
            Self::Blocked(_) | Self::Complete => return None,
        })
    }
}

/// Non-replayable detached `FUN_00425850` accepted transaction.
///
/// Deliberately neither `Copy` nor `Clone`. Duplicating the machine would
/// duplicate operation `0x33`, feedback, staffing, and destruction.
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryDeliveryMachine {
    transaction_id: FactoryDeliveryTransactionId,
    request: FactoryDeliveryRequest,
    pair_version: FactoryDeliveryPairVersion,
    factory_runtime: FactoryProductionRuntime,
    staffing_commit: Option<FactoryDeliveryStaffingCommit>,
    stage: FactoryDeliveryStage,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
}

impl FactoryDeliveryMachine {
    /// Close the complete callback journal before any action can escape.
    pub fn preflight(
        transaction_id: FactoryDeliveryTransactionId,
        request: FactoryDeliveryRequest,
    ) -> Result<Self, FactoryDeliveryAdmissionRejection> {
        let plan = plan_lifter_contact(LifterContactInput {
            target: RetailRuntimeValue::Known(Some(request.scientist.entity)),
            delivery: RetailRuntimeValue::Known(LifterDeliverySnapshot {
                lifter: request.factory.lifter,
                type_extension_present: request.factory.type_extension_present,
                behavior_state_present: request.factory.behavior_state_present,
                delivery_enabled: request.factory.delivery_enabled,
                factory_state: Some(LifterFactoryState {
                    occupancy_raw: request.factory.production.current_scientists_raw,
                    capacity_raw: request.factory.production.scientist_capacity_raw,
                }),
            }),
        })
        .map_err(FactoryDeliveryAdmissionRejection::CallbackUnresolved)?;

        let mut expected_actions = vec![
            EntityPairCallbackAction::EmitOperation33 {
                position_raw: request.scientist.entity.position_raw,
                source_entity_id: request.factory.lifter.id,
                remote_owned: false,
            },
            EntityPairCallbackAction::Feedback {
                feedback_id: u32::from(FACTORY_DELIVERY_HUD_RESOURCE_EVENT_ID),
            },
            EntityPairCallbackAction::AdvanceFactoryState {
                lifter_entity_id: request.factory.lifter.id,
                amount: 1,
            },
        ];
        if request.scientist.entity.state_flags_raw & 0x8000_0000 != 0 {
            expected_actions.push(EntityPairCallbackAction::RemoteEntityFeedback {
                entity_id: request.scientist.entity.id,
                channel: FACTORY_DELIVERY_REMOTE_FEEDBACK_CHANNEL,
                code: FACTORY_DELIVERY_REMOTE_FEEDBACK_CODE,
            });
        }
        expected_actions.push(EntityPairCallbackAction::QueueDeferredDestroy {
            entity_id: request.scientist.entity.id,
        });

        if plan.actions.is_empty() && plan.return_value == PairBehaviorCallbackReturn::Null {
            return Err(FactoryDeliveryAdmissionRejection::CallbackReturnedNull);
        }
        if plan.actions != expected_actions
            || plan.return_value
                != PairBehaviorCallbackReturn::TaggedA300(EntityPairTaggedEffect::LifterAccepted)
        {
            return Err(FactoryDeliveryAdmissionRejection::UnexpectedCallbackJournal);
        }

        Ok(Self {
            transaction_id,
            request,
            pair_version: request.pair_version(),
            factory_runtime: request.factory.production,
            staffing_commit: None,
            stage: FactoryDeliveryStage::EmitOperation33,
            next_action_sequence: 1,
            issued_action_sequence: None,
        })
    }

    pub const fn pair_version(&self) -> FactoryDeliveryPairVersion {
        self.pair_version
    }

    /// Runtime committed by acknowledged staffing. Before the staffing action
    /// is acknowledged this remains the admission value.
    pub const fn factory_runtime(&self) -> FactoryProductionRuntime {
        self.factory_runtime
    }

    pub fn poll(&mut self) -> FactoryDeliveryPoll {
        let Some(phase) = self.stage.phase() else {
            return match self.stage {
                FactoryDeliveryStage::Blocked(block) => FactoryDeliveryPoll::Blocked(block),
                FactoryDeliveryStage::Complete => FactoryDeliveryPoll::Complete(self.completion()),
                _ => unreachable!("active stages always expose a phase"),
            };
        };
        if self.issued_action_sequence.is_some() {
            return FactoryDeliveryPoll::Awaiting(phase);
        }

        let action = match self.stage {
            FactoryDeliveryStage::EmitOperation33 => FactoryDeliveryAction::EmitOperation33 {
                phase,
                position_raw: self.request.scientist.entity.position_raw,
                source_factory_entity_id: self.request.factory.lifter.id,
                remote_owned: false,
            },
            FactoryDeliveryStage::QueueDeliveryHudResource => {
                FactoryDeliveryAction::QueueDeliveryHudResource {
                    phase,
                    event_id: FACTORY_DELIVERY_HUD_RESOURCE_EVENT_ID,
                }
            }
            FactoryDeliveryStage::CommitStaffing(commit) => FactoryDeliveryAction::CommitStaffing {
                phase,
                factory: self.pair_version.factory,
                commit,
            },
            FactoryDeliveryStage::PublishCapacityCountdown => {
                FactoryDeliveryAction::PublishCapacityCountdown {
                    phase,
                    remaining_seconds_raw: 0,
                }
            }
            FactoryDeliveryStage::EmitCapacityDirectText => {
                FactoryDeliveryAction::EmitCapacityDirectText {
                    phase,
                    direct_text_id: self
                        .staffing_commit
                        .expect("capacity text requires the staffing result")
                        .change
                        .capacity_direct_text_id
                        .expect("text stage requires a direct-text id"),
                    sub_parameter: 0,
                }
            }
            FactoryDeliveryStage::QueueCapacityHudResource => {
                FactoryDeliveryAction::QueueCapacityHudResource {
                    phase,
                    event_id: FACTORY_CAPACITY_HUD_RESOURCE_EVENT_ID,
                }
            }
            FactoryDeliveryStage::EmitRemoteScientistFeedback => {
                FactoryDeliveryAction::EmitRemoteScientistFeedback {
                    phase,
                    scientist_entity_id: self.request.scientist.entity.id,
                    channel: FACTORY_DELIVERY_REMOTE_FEEDBACK_CHANNEL,
                    code: FACTORY_DELIVERY_REMOTE_FEEDBACK_CODE,
                }
            }
            FactoryDeliveryStage::QueueDeferredScientistDestroy => {
                FactoryDeliveryAction::QueueDeferredScientistDestroy {
                    phase,
                    scientist_entity_id: self.request.scientist.entity.id,
                }
            }
            FactoryDeliveryStage::Blocked(_) | FactoryDeliveryStage::Complete => {
                unreachable!("terminal stages returned above")
            }
        };

        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one factory delivery cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        FactoryDeliveryPoll::Action(IssuedFactoryDeliveryAction {
            receipt: FactoryDeliveryReceipt {
                transaction_id: self.transaction_id,
                action_sequence,
                pair_version: self.pair_version,
            },
            action,
        })
    }

    pub fn resume(
        &mut self,
        receipt: FactoryDeliveryReceipt,
        resume: FactoryDeliveryResume,
    ) -> Result<(), FactoryDeliveryResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            return Err(FactoryDeliveryResumeFailure {
                error: FactoryDeliveryProtocolError::NoOutstandingAction {
                    transaction_id: receipt.transaction_id,
                    action_sequence: receipt.action_sequence,
                },
                receipt,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(FactoryDeliveryResumeFailure {
                error: FactoryDeliveryProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.pair_version != self.pair_version {
            return Err(FactoryDeliveryResumeFailure {
                error: FactoryDeliveryProtocolError::ReceiptPairMismatch {
                    expected: self.pair_version,
                    actual: receipt.pair_version,
                },
                receipt,
            });
        }
        if receipt.action_sequence != expected_sequence {
            return Err(FactoryDeliveryResumeFailure {
                error: FactoryDeliveryProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }

        let expected_phase = self
            .stage
            .phase()
            .expect("an outstanding receipt belongs to an active phase");
        let actual_phase = resume.phase();
        if expected_phase != actual_phase {
            return Err(FactoryDeliveryResumeFailure {
                error: FactoryDeliveryProtocolError::PhaseMismatch {
                    expected: expected_phase,
                    actual: actual_phase,
                },
                receipt,
            });
        }

        self.issued_action_sequence = None;
        match resume {
            FactoryDeliveryResume::Blocked { reason, .. } => {
                self.stage = FactoryDeliveryStage::Blocked(FactoryDeliveryBlock {
                    pair_version: self.pair_version,
                    phase: expected_phase,
                    reason,
                });
            }
            FactoryDeliveryResume::Acknowledged { .. } => self.advance_after_acknowledgement(),
        }
        Ok(())
    }

    fn advance_after_acknowledgement(&mut self) {
        self.stage = match self.stage {
            FactoryDeliveryStage::EmitOperation33 => FactoryDeliveryStage::QueueDeliveryHudResource,
            FactoryDeliveryStage::QueueDeliveryHudResource => {
                let before = self.factory_runtime;
                let mut after = before;
                let change = after.adjust_staffing(1);
                let commit = FactoryDeliveryStaffingCommit {
                    before,
                    after,
                    change,
                };
                self.staffing_commit = Some(commit);
                FactoryDeliveryStage::CommitStaffing(commit)
            }
            FactoryDeliveryStage::CommitStaffing(commit) => {
                self.factory_runtime = commit.after;
                self.first_post_staffing_stage(commit)
            }
            FactoryDeliveryStage::PublishCapacityCountdown => {
                if self
                    .staffing_commit
                    .expect("capacity publication requires staffing")
                    .change
                    .capacity_direct_text_id
                    .is_some()
                {
                    FactoryDeliveryStage::EmitCapacityDirectText
                } else {
                    self.first_post_capacity_presentation_stage()
                }
            }
            FactoryDeliveryStage::EmitCapacityDirectText => {
                self.first_post_capacity_presentation_stage()
            }
            FactoryDeliveryStage::QueueCapacityHudResource => {
                self.first_post_staffing_feedback_stage()
            }
            FactoryDeliveryStage::EmitRemoteScientistFeedback => {
                FactoryDeliveryStage::QueueDeferredScientistDestroy
            }
            FactoryDeliveryStage::QueueDeferredScientistDestroy => FactoryDeliveryStage::Complete,
            FactoryDeliveryStage::Blocked(_) | FactoryDeliveryStage::Complete => {
                unreachable!("terminal stages cannot have outstanding receipts")
            }
        };
    }

    fn first_post_staffing_stage(
        &self,
        commit: FactoryDeliveryStaffingCommit,
    ) -> FactoryDeliveryStage {
        if commit.change.reached_capacity && commit.before.understaffed_limit_micros_raw != 0 {
            FactoryDeliveryStage::PublishCapacityCountdown
        } else if commit.change.capacity_direct_text_id.is_some() {
            FactoryDeliveryStage::EmitCapacityDirectText
        } else if commit.change.refreshes_status {
            FactoryDeliveryStage::QueueCapacityHudResource
        } else {
            self.first_post_staffing_feedback_stage()
        }
    }

    fn first_post_capacity_presentation_stage(&self) -> FactoryDeliveryStage {
        if self
            .staffing_commit
            .expect("capacity presentation requires staffing")
            .change
            .refreshes_status
        {
            FactoryDeliveryStage::QueueCapacityHudResource
        } else {
            self.first_post_staffing_feedback_stage()
        }
    }

    fn first_post_staffing_feedback_stage(&self) -> FactoryDeliveryStage {
        if self.request.scientist.entity.state_flags_raw & 0x8000_0000 != 0 {
            FactoryDeliveryStage::EmitRemoteScientistFeedback
        } else {
            FactoryDeliveryStage::QueueDeferredScientistDestroy
        }
    }

    fn completion(&self) -> FactoryDeliveryCompletion {
        FactoryDeliveryCompletion {
            pair_version: self.pair_version,
            callback_return: PairBehaviorCallbackReturn::TaggedA300(
                EntityPairTaggedEffect::LifterAccepted,
            ),
            pair_suffix: FactoryDeliveryPairSuffix::Unclaimed,
            factory_runtime: self.factory_runtime,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory_production::{
        FactorySection13Config, FACTORY_CAPACITY_DIRECT_TEXT_ID, FACTORY_CONFIG_BYTES,
    };

    const FACTORY_ID: u32 = 0x04B7_0001;
    const SCIENTIST_ID: u32 = 0x04C2_0001;
    const SCIENTIST_POSITION: [i16; 3] = [0x50BA, -0x02FE, 0x39EC];
    const ACTIVE: u32 = 1;
    const DYING: u32 = 0x0000_4000;
    const REMOTE: u32 = 0x8000_0000;
    const SCIENTIST_CAPABILITY: u32 = 0x0000_0400;

    fn transaction_id(raw: u64) -> FactoryDeliveryTransactionId {
        FactoryDeliveryTransactionId::new(raw).unwrap()
    }

    fn write_i32(raw: &mut [u8; FACTORY_CONFIG_BYTES], offset: usize, value: i32) {
        raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn production(
        capacity: i32,
        scientists: i32,
        understaffed_limit: i32,
    ) -> FactoryProductionRuntime {
        let mut raw = [0; FACTORY_CONFIG_BYTES];
        write_i32(&mut raw, 0x04, capacity);
        write_i32(&mut raw, 0x08, 6_000_000);
        write_i32(&mut raw, 0x0C, 5_000_000);
        write_i32(&mut raw, 0x10, -1_000);
        write_i32(&mut raw, 0x14, understaffed_limit);
        let mut runtime = FactoryProductionRuntime::from_retail_template(
            FactorySection13Config::decode(&raw),
            99_999,
        );
        runtime.current_scientists_raw = scientists;
        runtime.understaffed_countdown_micros_raw = understaffed_limit;
        runtime
    }

    fn request_with(
        capacity: i32,
        scientists: i32,
        understaffed_limit: i32,
    ) -> FactoryDeliveryRequest {
        FactoryDeliveryRequest {
            factory: FactoryDeliveryFactorySnapshot {
                lifter: LifterSourceSnapshot {
                    id: FACTORY_ID,
                    state_flags_raw: ACTIVE,
                },
                allocation_identity: 0x1111,
                state_version: 7,
                type_extension_present: true,
                behavior_state_present: true,
                delivery_enabled: true,
                production: production(capacity, scientists, understaffed_limit),
            },
            scientist: FactoryDeliveryScientistSnapshot {
                entity: PairCallbackEntitySnapshot {
                    id: SCIENTIST_ID,
                    position_raw: SCIENTIST_POSITION,
                    state_flags_raw: ACTIVE,
                    capability_flags_raw: SCIENTIST_CAPABILITY,
                },
                allocation_identity: 0x2222,
                state_version: 9,
            },
        }
    }

    fn issue(machine: &mut FactoryDeliveryMachine) -> IssuedFactoryDeliveryAction {
        match machine.poll() {
            FactoryDeliveryPoll::Action(issued) => issued,
            other => panic!("expected action, got {other:?}"),
        }
    }

    fn acknowledge(machine: &mut FactoryDeliveryMachine, issued: IssuedFactoryDeliveryAction) {
        let phase = issued.action.phase();
        machine
            .resume(
                issued.receipt,
                FactoryDeliveryResume::Acknowledged { phase },
            )
            .unwrap();
    }

    fn acknowledge_next(machine: &mut FactoryDeliveryMachine) {
        let issued = issue(machine);
        acknowledge(machine, issued);
    }

    fn next_phase(machine: &mut FactoryDeliveryMachine) -> FactoryDeliveryPhase {
        let issued = issue(machine);
        let phase = issued.action.phase();
        acknowledge(machine, issued);
        phase
    }

    #[test]
    fn accepted_under_capacity_delivery_preserves_exact_order_and_tag() {
        let request = request_with(3, 0, 0);
        let before = request.factory.production;
        let mut machine = FactoryDeliveryMachine::preflight(transaction_id(1), request).unwrap();

        let operation = issue(&mut machine);
        assert_eq!(
            operation.action,
            FactoryDeliveryAction::EmitOperation33 {
                phase: FactoryDeliveryPhase::EmitOperation33,
                position_raw: SCIENTIST_POSITION,
                source_factory_entity_id: FACTORY_ID,
                remote_owned: false,
            }
        );
        assert_eq!(
            machine.poll(),
            FactoryDeliveryPoll::Awaiting(FactoryDeliveryPhase::EmitOperation33)
        );
        acknowledge(&mut machine, operation);

        let feedback = issue(&mut machine);
        assert_eq!(
            feedback.action,
            FactoryDeliveryAction::QueueDeliveryHudResource {
                phase: FactoryDeliveryPhase::QueueDeliveryHudResource,
                event_id: FACTORY_DELIVERY_HUD_RESOURCE_EVENT_ID,
            }
        );
        acknowledge(&mut machine, feedback);

        let staffing = issue(&mut machine);
        let FactoryDeliveryAction::CommitStaffing {
            factory, commit, ..
        } = staffing.action
        else {
            panic!("expected staffing commit")
        };
        assert_eq!(factory, request.factory.version());
        assert_eq!(commit.before, before);
        assert_eq!(commit.after.current_scientists_raw, 1);
        assert_eq!(commit.change.previous_scientists_raw, 0);
        assert_eq!(commit.change.current_scientists_raw, 1);
        assert!(!commit.change.reached_capacity);
        assert_eq!(
            machine.factory_runtime(),
            before,
            "the detached copy commits only when the host acknowledges its write"
        );
        acknowledge(&mut machine, staffing);
        assert_eq!(machine.factory_runtime(), commit.after);

        let destroy = issue(&mut machine);
        assert_eq!(
            destroy.action,
            FactoryDeliveryAction::QueueDeferredScientistDestroy {
                phase: FactoryDeliveryPhase::QueueDeferredScientistDestroy,
                scientist_entity_id: SCIENTIST_ID,
            }
        );
        acknowledge(&mut machine, destroy);

        let FactoryDeliveryPoll::Complete(done) = machine.poll() else {
            panic!("expected tagged completion")
        };
        assert_eq!(
            done.callback_return,
            PairBehaviorCallbackReturn::TaggedA300(EntityPairTaggedEffect::LifterAccepted)
        );
        let PairBehaviorCallbackReturn::TaggedA300(effect) = done.callback_return else {
            unreachable!()
        };
        assert_eq!(effect.tag(), 0xA300);
        assert_eq!(done.pair_suffix, FactoryDeliveryPairSuffix::Unclaimed);
        assert_eq!(done.factory_runtime.current_scientists_raw, 1);
    }

    #[test]
    fn filling_factory_emits_complete_staffing_presentation_before_destroy() {
        let mut request = request_with(2, 1, 120);
        request.factory.production.understaffed_countdown_micros_raw = 60;
        let mut machine = FactoryDeliveryMachine::preflight(transaction_id(2), request).unwrap();

        assert_eq!(
            [
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
            ],
            [
                FactoryDeliveryPhase::EmitOperation33,
                FactoryDeliveryPhase::QueueDeliveryHudResource,
                FactoryDeliveryPhase::CommitStaffing,
                FactoryDeliveryPhase::PublishCapacityCountdown,
                FactoryDeliveryPhase::EmitCapacityDirectText,
                FactoryDeliveryPhase::QueueCapacityHudResource,
                FactoryDeliveryPhase::QueueDeferredScientistDestroy,
            ]
        );
        let FactoryDeliveryPoll::Complete(done) = machine.poll() else {
            panic!("expected completion")
        };
        assert_eq!(done.factory_runtime.current_scientists_raw, 2);
        assert_eq!(done.factory_runtime.understaffed_countdown_micros_raw, 0);

        let mut inspect = FactoryDeliveryMachine::preflight(transaction_id(3), request).unwrap();
        acknowledge_next(&mut inspect);
        acknowledge_next(&mut inspect);
        let staffing = issue(&mut inspect);
        let FactoryDeliveryAction::CommitStaffing { commit, .. } = staffing.action else {
            unreachable!()
        };
        assert_eq!(
            commit.change.capacity_direct_text_id,
            Some(FACTORY_CAPACITY_DIRECT_TEXT_ID)
        );
        assert!(commit.change.refreshes_status);
        acknowledge(&mut inspect, staffing);
        let countdown = issue(&mut inspect);
        assert_eq!(
            countdown.action,
            FactoryDeliveryAction::PublishCapacityCountdown {
                phase: FactoryDeliveryPhase::PublishCapacityCountdown,
                remaining_seconds_raw: 0,
            }
        );
        acknowledge(&mut inspect, countdown);
        let text = issue(&mut inspect);
        assert_eq!(
            text.action,
            FactoryDeliveryAction::EmitCapacityDirectText {
                phase: FactoryDeliveryPhase::EmitCapacityDirectText,
                direct_text_id: FACTORY_CAPACITY_DIRECT_TEXT_ID,
                sub_parameter: 0,
            }
        );
        acknowledge(&mut inspect, text);
        let resource = issue(&mut inspect);
        assert_eq!(
            resource.action,
            FactoryDeliveryAction::QueueCapacityHudResource {
                phase: FactoryDeliveryPhase::QueueCapacityHudResource,
                event_id: FACTORY_CAPACITY_HUD_RESOURCE_EVENT_ID,
            }
        );
    }

    #[test]
    fn zero_understaffed_limit_skips_countdown_and_text_but_keeps_hud_refresh() {
        let mut machine =
            FactoryDeliveryMachine::preflight(transaction_id(4), request_with(1, 0, 0)).unwrap();
        assert_eq!(
            [
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
                next_phase(&mut machine),
            ],
            [
                FactoryDeliveryPhase::EmitOperation33,
                FactoryDeliveryPhase::QueueDeliveryHudResource,
                FactoryDeliveryPhase::CommitStaffing,
                FactoryDeliveryPhase::QueueCapacityHudResource,
                FactoryDeliveryPhase::QueueDeferredScientistDestroy,
            ]
        );
        assert!(matches!(machine.poll(), FactoryDeliveryPoll::Complete(_)));
    }

    #[test]
    fn remote_feedback_follows_all_capacity_presentation_and_precedes_destroy() {
        let mut request = request_with(1, 0, 10);
        request.scientist.entity.state_flags_raw = REMOTE;
        let mut machine = FactoryDeliveryMachine::preflight(transaction_id(5), request).unwrap();
        let mut phases = Vec::new();
        loop {
            match machine.poll() {
                FactoryDeliveryPoll::Action(issued) => {
                    phases.push(issued.action.phase());
                    acknowledge(&mut machine, issued);
                }
                FactoryDeliveryPoll::Complete(_) => break,
                other => panic!("unexpected state {other:?}"),
            }
        }
        assert_eq!(
            phases,
            [
                FactoryDeliveryPhase::EmitOperation33,
                FactoryDeliveryPhase::QueueDeliveryHudResource,
                FactoryDeliveryPhase::CommitStaffing,
                FactoryDeliveryPhase::PublishCapacityCountdown,
                FactoryDeliveryPhase::EmitCapacityDirectText,
                FactoryDeliveryPhase::QueueCapacityHudResource,
                FactoryDeliveryPhase::EmitRemoteScientistFeedback,
                FactoryDeliveryPhase::QueueDeferredScientistDestroy,
            ]
        );
    }

    #[test]
    fn every_retail_rejection_gate_stops_before_the_first_action() {
        let baseline = request_with(2, 0, 10);
        let mut rejected = Vec::new();

        let mut missing_capability = baseline;
        missing_capability.scientist.entity.capability_flags_raw = 0;
        rejected.push(missing_capability);

        let mut remote_factory = baseline;
        remote_factory.factory.lifter.state_flags_raw = REMOTE;
        rejected.push(remote_factory);

        let mut missing_extension = baseline;
        missing_extension.factory.type_extension_present = false;
        rejected.push(missing_extension);

        let mut missing_behavior = baseline;
        missing_behavior.factory.behavior_state_present = false;
        rejected.push(missing_behavior);

        let mut dying_scientist = baseline;
        dying_scientist.scientist.entity.state_flags_raw = ACTIVE | DYING;
        rejected.push(dying_scientist);

        let mut inactive_scientist = baseline;
        inactive_scientist.scientist.entity.state_flags_raw = 0;
        rejected.push(inactive_scientist);

        let mut dying_factory = baseline;
        dying_factory.factory.lifter.state_flags_raw = ACTIVE | DYING;
        rejected.push(dying_factory);

        let mut inactive_factory = baseline;
        inactive_factory.factory.lifter.state_flags_raw = 0;
        rejected.push(inactive_factory);

        let mut disabled = baseline;
        disabled.factory.delivery_enabled = false;
        rejected.push(disabled);

        let at_capacity = request_with(2, 2, 10);
        rejected.push(at_capacity);

        for (index, request) in rejected.into_iter().enumerate() {
            let before = request.factory.production;
            assert_eq!(
                FactoryDeliveryMachine::preflight(transaction_id(100 + index as u64), request,),
                Err(FactoryDeliveryAdmissionRejection::CallbackReturnedNull)
            );
            assert_eq!(request.factory.production, before);
        }
    }

    #[test]
    fn wrong_phase_returns_receipt_intact_and_changes_nothing() {
        let request = request_with(2, 0, 10);
        let mut machine = FactoryDeliveryMachine::preflight(transaction_id(6), request).unwrap();
        let issued = issue(&mut machine);
        let before_runtime = machine.factory_runtime();
        let failure = machine
            .resume(
                issued.receipt,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::QueueDeliveryHudResource,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            FactoryDeliveryProtocolError::PhaseMismatch {
                expected: FactoryDeliveryPhase::EmitOperation33,
                actual: FactoryDeliveryPhase::QueueDeliveryHudResource,
            }
        );
        assert_eq!(failure.receipt.transaction_id(), transaction_id(6));
        assert_eq!(failure.receipt.action_sequence(), 1);
        assert_eq!(machine.factory_runtime(), before_runtime);
        assert_eq!(
            machine.poll(),
            FactoryDeliveryPoll::Awaiting(FactoryDeliveryPhase::EmitOperation33)
        );
        machine
            .resume(
                failure.receipt,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::EmitOperation33,
                },
            )
            .unwrap();
    }

    #[test]
    fn first_phase_block_commits_nothing_and_becomes_terminal() {
        let request = request_with(2, 0, 10);
        let before = request.factory.production;
        let mut machine = FactoryDeliveryMachine::preflight(transaction_id(12), request).unwrap();
        let operation = issue(&mut machine);

        machine
            .resume(
                operation.receipt,
                FactoryDeliveryResume::Blocked {
                    phase: FactoryDeliveryPhase::EmitOperation33,
                    reason: FactoryDeliveryExternalBlock::Operation33Unavailable,
                },
            )
            .unwrap();

        assert_eq!(machine.factory_runtime(), before);
        assert_eq!(
            machine.poll(),
            FactoryDeliveryPoll::Blocked(FactoryDeliveryBlock {
                pair_version: request.pair_version(),
                phase: FactoryDeliveryPhase::EmitOperation33,
                reason: FactoryDeliveryExternalBlock::Operation33Unavailable,
            })
        );
    }

    #[test]
    fn cross_pair_and_cross_transaction_receipts_are_recoverable() {
        let request = request_with(2, 0, 10);
        let mut other_target = request;
        other_target.scientist.entity.id = SCIENTIST_ID + 1;
        other_target.scientist.allocation_identity += 1;

        let transaction = transaction_id(7);
        let mut first = FactoryDeliveryMachine::preflight(transaction, request).unwrap();
        let mut second = FactoryDeliveryMachine::preflight(transaction, other_target).unwrap();
        let first_action = issue(&mut first);
        let second_action = issue(&mut second);
        let failure = second
            .resume(
                first_action.receipt,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::EmitOperation33,
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            FactoryDeliveryProtocolError::ReceiptPairMismatch { .. }
        ));
        assert_eq!(failure.receipt.pair_version(), request.pair_version());
        let recovered_first_receipt = failure.receipt;
        assert_eq!(
            second.poll(),
            FactoryDeliveryPoll::Awaiting(FactoryDeliveryPhase::EmitOperation33)
        );
        acknowledge(&mut second, second_action);

        let mut third = FactoryDeliveryMachine::preflight(transaction_id(8), request).unwrap();
        let third_action = issue(&mut third);
        let failure = first
            .resume(
                third_action.receipt,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::EmitOperation33,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            FactoryDeliveryProtocolError::ReceiptTransactionMismatch {
                expected: transaction,
                actual: transaction_id(8),
            }
        );
        assert_eq!(
            first.poll(),
            FactoryDeliveryPoll::Awaiting(FactoryDeliveryPhase::EmitOperation33)
        );
        first
            .resume(
                recovered_first_receipt,
                FactoryDeliveryResume::Acknowledged {
                    phase: first_action.action.phase(),
                },
            )
            .unwrap();
    }

    #[test]
    fn state_version_alone_prevents_cross_machine_receipt_acceptance() {
        let request = request_with(2, 0, 10);
        let mut newer = request;
        newer.scientist.state_version = newer.scientist.state_version.wrapping_add(1);
        let transaction = transaction_id(13);
        let mut first = FactoryDeliveryMachine::preflight(transaction, request).unwrap();
        let mut second = FactoryDeliveryMachine::preflight(transaction, newer).unwrap();
        let first_action = issue(&mut first);
        let second_action = issue(&mut second);

        let failure = second
            .resume(
                first_action.receipt,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::EmitOperation33,
                },
            )
            .unwrap_err();

        assert_eq!(
            failure.error,
            FactoryDeliveryProtocolError::ReceiptPairMismatch {
                expected: newer.pair_version(),
                actual: request.pair_version(),
            }
        );
        assert_eq!(
            second.poll(),
            FactoryDeliveryPoll::Awaiting(FactoryDeliveryPhase::EmitOperation33)
        );
        acknowledge(&mut second, second_action);
    }

    #[test]
    fn stale_and_already_consumed_receipts_never_advance_twice() {
        let request = request_with(2, 0, 10);
        let transaction = transaction_id(9);
        let mut machine = FactoryDeliveryMachine::preflight(transaction, request).unwrap();
        let first = issue(&mut machine);
        let pair = first.receipt.pair_version();
        acknowledge(&mut machine, first);

        let consumed = FactoryDeliveryReceipt {
            transaction_id: transaction,
            action_sequence: 1,
            pair_version: pair,
        };
        let failure = machine
            .resume(
                consumed,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::EmitOperation33,
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            FactoryDeliveryProtocolError::NoOutstandingAction { .. }
        ));

        let second = issue(&mut machine);
        let stale = FactoryDeliveryReceipt {
            transaction_id: transaction,
            action_sequence: 1,
            pair_version: pair,
        };
        let failure = machine
            .resume(
                stale,
                FactoryDeliveryResume::Acknowledged {
                    phase: FactoryDeliveryPhase::QueueDeliveryHudResource,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            FactoryDeliveryProtocolError::ReceiptSequenceMismatch {
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(failure.receipt.action_sequence(), 1);
        assert_eq!(
            machine.poll(),
            FactoryDeliveryPoll::Awaiting(FactoryDeliveryPhase::QueueDeliveryHudResource)
        );
        acknowledge(&mut machine, second);
    }

    #[test]
    fn late_block_preserves_prior_staffing_while_commit_block_does_not_apply_it() {
        let request = request_with(1, 0, 10);
        let before = request.factory.production;

        let mut commit_block =
            FactoryDeliveryMachine::preflight(transaction_id(10), request).unwrap();
        acknowledge_next(&mut commit_block);
        acknowledge_next(&mut commit_block);
        let staffing = issue(&mut commit_block);
        commit_block
            .resume(
                staffing.receipt,
                FactoryDeliveryResume::Blocked {
                    phase: FactoryDeliveryPhase::CommitStaffing,
                    reason: FactoryDeliveryExternalBlock::StaffingCommitUnavailable,
                },
            )
            .unwrap();
        assert_eq!(commit_block.factory_runtime(), before);
        assert_eq!(
            commit_block.poll(),
            FactoryDeliveryPoll::Blocked(FactoryDeliveryBlock {
                pair_version: request.pair_version(),
                phase: FactoryDeliveryPhase::CommitStaffing,
                reason: FactoryDeliveryExternalBlock::StaffingCommitUnavailable,
            })
        );

        let mut late_block =
            FactoryDeliveryMachine::preflight(transaction_id(11), request).unwrap();
        acknowledge_next(&mut late_block);
        acknowledge_next(&mut late_block);
        acknowledge_next(&mut late_block);
        assert_eq!(late_block.factory_runtime().current_scientists_raw, 1);
        let countdown = issue(&mut late_block);
        late_block
            .resume(
                countdown.receipt,
                FactoryDeliveryResume::Blocked {
                    phase: FactoryDeliveryPhase::PublishCapacityCountdown,
                    reason: FactoryDeliveryExternalBlock::CapacityCountdownUnavailable,
                },
            )
            .unwrap();
        assert_eq!(late_block.factory_runtime().current_scientists_raw, 1);
        assert!(matches!(
            late_block.poll(),
            FactoryDeliveryPoll::Blocked(FactoryDeliveryBlock {
                phase: FactoryDeliveryPhase::PublishCapacityCountdown,
                ..
            })
        ));
    }
}
