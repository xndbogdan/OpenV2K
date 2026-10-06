//! Receipt-bound outer Working Factory update.
//!
//! Retail `FUN_00419010` is more than its full-health production switch. Its
//! owner first delegates progressive death, reconciles cached health (including
//! the apparent double scientist decrement), advances the understaffed clock,
//! and either repairs a damaged Factory or enters production. The common
//! animation/status tail runs only after those branches.
//!
//! This module owns that ordering as a detached transaction. It deliberately
//! does not attach the result to a live entity: every callback, presentation,
//! destruction, and publication boundary is a receipt-bound adapter action.

use std::num::NonZeroU64;

use crate::base_factory_progression::{
    ProgressiveDeathAdvance, ProgressiveDeathBegin, ProgressiveDeathState, ProgressiveModelEffect,
    PROGRESSION_REVIVE_HEALTH_RAW,
};
use crate::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FACTORY_CAPACITY_DIRECT_TEXT_ID,
    FACTORY_UNDERSTAFFED_DIRECT_TEXT_ID,
};
use crate::factory_production_live::{
    animation_transition, published_status, FactoryAnimationTransition, FactoryEntitySpawnResult,
    FactoryPickupPresence, FactoryProductionAction, FactoryProductionActionPhase,
    FactoryProductionDetachedState, FactoryProductionEntityVersion, FactoryProductionExternalBlock,
    FactoryProductionFrameRequest, FactoryProductionMachine, FactoryProductionPoll,
    FactoryProductionProtocolError, FactoryProductionReceipt, FactoryProductionResume,
    FactoryProductionTransactionId, FactoryStatusPublication,
};

/// Retail Working Factory owner update.
pub const FACTORY_PRODUCTION_OWNER_UPDATE_ADDRESS: u32 = 0x0041_9010;
/// Direct-text identifier emitted before understaffed destruction.
pub const FACTORY_OWNER_UNDERSTAFFED_DIRECT_TEXT_ID: u16 = FACTORY_UNDERSTAFFED_DIRECT_TEXT_ID;
/// Direct-text identifier emitted when repair re-enters a full Factory.
pub const FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID: u16 = FACTORY_CAPACITY_DIRECT_TEXT_ID;
/// HUD resource emitted after repair clamps staffing at capacity.
pub const FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID: u16 = 2;
/// Entry write applied to the entity runtime byte at `+0x84`.
pub const FACTORY_OWNER_ENTRY_FLAGS_AT_0X84_OR_MASK: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryProductionOwnerTransactionId(NonZeroU64);

impl FactoryProductionOwnerTransactionId {
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

/// Complete immutable frame snapshot consumed before any owner action escapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionOwnerFrameRequest {
    pub factory: FactoryProductionEntityVersion,
    pub position_raw: [i16; 3],
    pub pickup_spawn_offset_raw: [i16; 3],
    pub current_health_raw: i32,
    pub maximum_health_raw: i32,
    pub progressive_death: ProgressiveDeathState,
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub elapsed_micros: u32,
    pub world_style_raw: u32,
    pub phase1_presentation_enabled: bool,
    pub suppress_status_publication: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionOwnerAdmissionRejection {
    TransactionIdsMustBeDistinct,
    NonPositiveHealthLossThreshold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactoryProductionOwnerActionPhase {
    ProgressiveModelEffect,
    QueueProgressivePickupDeferredDestroy,
    InvokeProgressiveFactoryDeath,
    DispatchProgressiveDeathPresentation,
    EmitUnderstaffedDirectText,
    InvokeUnderstaffedDeath,
    PublishUnderstaffedCountdown,
    PublishDamageLossCapacityCountdown,
    EmitDamageLossCapacityDirectText,
    QueueDamageLossCapacityHudResource,
    PublishRepairCompleteCountdown,
    EmitRepairCapacityDirectText,
    QueueRepairCapacityHudResource,
    Production(FactoryProductionActionPhase),
    ApplyAnimationTransition,
    PublishStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FactoryProductionOwnerAction {
    DispatchProgressiveModelEffect {
        phase: FactoryProductionOwnerActionPhase,
        factory: FactoryProductionEntityVersion,
        effect: ProgressiveModelEffect,
    },
    /// Call retail's deferred-destruction helper. A lookup miss, including
    /// entity ID zero, is an acknowledged no-op rather than a protocol block.
    QueueDeferredDestroy {
        phase: FactoryProductionOwnerActionPhase,
        entity_id: u32,
    },
    InvokeProgressiveFactoryDeath {
        phase: FactoryProductionOwnerActionPhase,
        factory: FactoryProductionEntityVersion,
    },
    DispatchProgressiveDeathPresentation {
        phase: FactoryProductionOwnerActionPhase,
    },
    EmitDirectText {
        phase: FactoryProductionOwnerActionPhase,
        direct_text_id: u16,
        sub_parameter: u32,
    },
    InvokeUnderstaffedDeath {
        phase: FactoryProductionOwnerActionPhase,
        factory: FactoryProductionEntityVersion,
    },
    PublishCountdownSeconds {
        phase: FactoryProductionOwnerActionPhase,
        remaining_seconds_raw: i32,
    },
    QueueHudResource {
        phase: FactoryProductionOwnerActionPhase,
        resource_id: u16,
    },
    Production {
        phase: FactoryProductionOwnerActionPhase,
        action: FactoryProductionAction,
    },
    ApplyAnimationTransition {
        phase: FactoryProductionOwnerActionPhase,
        factory: FactoryProductionEntityVersion,
        transition: FactoryAnimationTransition,
    },
    PublishStatus {
        phase: FactoryProductionOwnerActionPhase,
        factory: FactoryProductionEntityVersion,
        status: FactoryStatusPublication,
    },
}

impl FactoryProductionOwnerAction {
    pub const fn phase(&self) -> FactoryProductionOwnerActionPhase {
        match self {
            Self::DispatchProgressiveModelEffect { phase, .. }
            | Self::QueueDeferredDestroy { phase, .. }
            | Self::InvokeProgressiveFactoryDeath { phase, .. }
            | Self::DispatchProgressiveDeathPresentation { phase }
            | Self::EmitDirectText { phase, .. }
            | Self::InvokeUnderstaffedDeath { phase, .. }
            | Self::PublishCountdownSeconds { phase, .. }
            | Self::QueueHudResource { phase, .. }
            | Self::Production { phase, .. }
            | Self::ApplyAnimationTransition { phase, .. }
            | Self::PublishStatus { phase, .. } => *phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionOwnerPreActionState {
    pub factory: FactoryProductionEntityVersion,
    /// OR-mask for the entry write to entity byte `+0x84`.
    pub entry_flags_at_0x84_or_mask: u8,
    pub current_health_raw: i32,
    pub progressive_death: ProgressiveDeathState,
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub status_published: bool,
    pub dirty_flag_written: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum FactoryProductionOwnerReceiptKind {
    Owner,
    Production(FactoryProductionReceipt),
}

/// Linear proof for one exact owner or nested production action.
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryProductionOwnerReceipt {
    transaction_id: FactoryProductionOwnerTransactionId,
    action_sequence: u64,
    factory: FactoryProductionEntityVersion,
    kind: FactoryProductionOwnerReceiptKind,
}

impl FactoryProductionOwnerReceipt {
    pub const fn transaction_id(&self) -> FactoryProductionOwnerTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }

    pub const fn factory(&self) -> FactoryProductionEntityVersion {
        self.factory
    }

    pub const fn is_nested_production(&self) -> bool {
        matches!(self.kind, FactoryProductionOwnerReceiptKind::Production(_))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IssuedFactoryProductionOwnerAction {
    pub receipt: FactoryProductionOwnerReceipt,
    pub before_action: FactoryProductionOwnerPreActionState,
    pub action: FactoryProductionOwnerAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionOwnerExternalBlock {
    FactoryStateCommitUnavailable,
    ProgressiveModelEffectUnavailable,
    EntityDestructionUnavailable,
    ProgressiveDeathPresentationUnavailable,
    DirectTextUnavailable,
    DeathCallbackUnavailable,
    CountdownPublicationUnavailable,
    HudResourceUnavailable,
    AnimationUnavailable,
    StatusPublicationUnavailable,
    Production(FactoryProductionExternalBlock),
}

impl FactoryProductionOwnerExternalBlock {
    const fn matches_owner_phase(self, phase: FactoryProductionOwnerActionPhase) -> bool {
        if matches!(self, Self::FactoryStateCommitUnavailable) {
            return true;
        }
        matches!(
            (self, phase),
            (
                Self::ProgressiveModelEffectUnavailable,
                FactoryProductionOwnerActionPhase::ProgressiveModelEffect
            ) | (
                Self::EntityDestructionUnavailable,
                FactoryProductionOwnerActionPhase::QueueProgressivePickupDeferredDestroy
            ) | (
                Self::ProgressiveDeathPresentationUnavailable,
                FactoryProductionOwnerActionPhase::DispatchProgressiveDeathPresentation
            ) | (
                Self::DirectTextUnavailable,
                FactoryProductionOwnerActionPhase::EmitUnderstaffedDirectText
                    | FactoryProductionOwnerActionPhase::EmitDamageLossCapacityDirectText
                    | FactoryProductionOwnerActionPhase::EmitRepairCapacityDirectText
            ) | (
                Self::DeathCallbackUnavailable,
                FactoryProductionOwnerActionPhase::InvokeProgressiveFactoryDeath
                    | FactoryProductionOwnerActionPhase::InvokeUnderstaffedDeath
            ) | (
                Self::CountdownPublicationUnavailable,
                FactoryProductionOwnerActionPhase::PublishUnderstaffedCountdown
                    | FactoryProductionOwnerActionPhase::PublishDamageLossCapacityCountdown
                    | FactoryProductionOwnerActionPhase::PublishRepairCompleteCountdown
            ) | (
                Self::HudResourceUnavailable,
                FactoryProductionOwnerActionPhase::QueueRepairCapacityHudResource
                    | FactoryProductionOwnerActionPhase::QueueDamageLossCapacityHudResource
            ) | (
                Self::AnimationUnavailable,
                FactoryProductionOwnerActionPhase::ApplyAnimationTransition
            ) | (
                Self::StatusPublicationUnavailable,
                FactoryProductionOwnerActionPhase::PublishStatus
            ) | (
                Self::Production(_),
                FactoryProductionOwnerActionPhase::Production(_)
            )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionOwnerResume {
    Acknowledged {
        phase: FactoryProductionOwnerActionPhase,
    },
    SpawnCompleted {
        phase: FactoryProductionOwnerActionPhase,
        result: FactoryEntitySpawnResult,
    },
    PickupPresence {
        phase: FactoryProductionOwnerActionPhase,
        presence: FactoryPickupPresence,
    },
    Blocked {
        phase: FactoryProductionOwnerActionPhase,
        reason: FactoryProductionOwnerExternalBlock,
    },
}

impl FactoryProductionOwnerResume {
    pub const fn phase(self) -> FactoryProductionOwnerActionPhase {
        match self {
            Self::Acknowledged { phase }
            | Self::SpawnCompleted { phase, .. }
            | Self::PickupPresence { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionOwnerBlock {
    pub factory: FactoryProductionEntityVersion,
    pub entry_flags_at_0x84_or_mask: u8,
    pub phase: FactoryProductionOwnerActionPhase,
    pub reason: FactoryProductionOwnerExternalBlock,
    pub current_health_raw: i32,
    pub progressive_death: ProgressiveDeathState,
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub status_published: bool,
    pub dirty_flag_written: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionOwnerCompletion {
    pub factory: FactoryProductionEntityVersion,
    pub entry_flags_at_0x84_or_mask: u8,
    pub current_health_raw: i32,
    pub progressive_death: ProgressiveDeathState,
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub status_published: bool,
    pub dirty_flag_written: bool,
    pub callback_result: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FactoryProductionOwnerPoll {
    Action(IssuedFactoryProductionOwnerAction),
    Awaiting(FactoryProductionOwnerActionPhase),
    Blocked(FactoryProductionOwnerBlock),
    Complete(FactoryProductionOwnerCompletion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionOwnerProtocolError {
    NoOutstandingAction {
        transaction_id: FactoryProductionOwnerTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: FactoryProductionOwnerTransactionId,
        actual: FactoryProductionOwnerTransactionId,
    },
    ReceiptFactoryMismatch {
        expected: FactoryProductionEntityVersion,
        actual: FactoryProductionEntityVersion,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: FactoryProductionOwnerActionPhase,
        actual: FactoryProductionOwnerActionPhase,
    },
    ResponseKindMismatch {
        phase: FactoryProductionOwnerActionPhase,
    },
    ChildProtocol(FactoryProductionProtocolError),
}

#[derive(Debug, PartialEq, Eq)]
pub struct FactoryProductionOwnerResumeFailure {
    pub receipt: FactoryProductionOwnerReceipt,
    pub error: FactoryProductionOwnerProtocolError,
}

#[derive(Debug, PartialEq, Eq)]
enum FactoryProductionOwnerStage {
    Start,
    ProgressiveModelEffect,
    QueueProgressivePickupDeferredDestroy,
    InvokeProgressiveFactoryDeath,
    DispatchProgressiveDeathPresentation,
    EmitUnderstaffedDirectText,
    InvokeUnderstaffedDeath,
    PublishUnderstaffedCountdown,
    PublishDamageLossCapacityCountdown,
    EmitDamageLossCapacityDirectText,
    QueueDamageLossCapacityHudResource,
    PublishRepairCompleteCountdown,
    EmitRepairCapacityDirectText,
    QueueRepairCapacityHudResource,
    Production(FactoryProductionMachine),
    ApplyAnimationTransition {
        transition: FactoryAnimationTransition,
        completes_frame: bool,
    },
    PublishStatus(FactoryStatusPublication),
    Blocked(FactoryProductionOwnerBlock),
    Complete,
}

impl FactoryProductionOwnerStage {
    const fn owner_action_phase(&self) -> Option<FactoryProductionOwnerActionPhase> {
        Some(match self {
            Self::ProgressiveModelEffect => {
                FactoryProductionOwnerActionPhase::ProgressiveModelEffect
            }
            Self::QueueProgressivePickupDeferredDestroy => {
                FactoryProductionOwnerActionPhase::QueueProgressivePickupDeferredDestroy
            }
            Self::InvokeProgressiveFactoryDeath => {
                FactoryProductionOwnerActionPhase::InvokeProgressiveFactoryDeath
            }
            Self::DispatchProgressiveDeathPresentation => {
                FactoryProductionOwnerActionPhase::DispatchProgressiveDeathPresentation
            }
            Self::EmitUnderstaffedDirectText => {
                FactoryProductionOwnerActionPhase::EmitUnderstaffedDirectText
            }
            Self::InvokeUnderstaffedDeath => {
                FactoryProductionOwnerActionPhase::InvokeUnderstaffedDeath
            }
            Self::PublishUnderstaffedCountdown => {
                FactoryProductionOwnerActionPhase::PublishUnderstaffedCountdown
            }
            Self::PublishDamageLossCapacityCountdown => {
                FactoryProductionOwnerActionPhase::PublishDamageLossCapacityCountdown
            }
            Self::EmitDamageLossCapacityDirectText => {
                FactoryProductionOwnerActionPhase::EmitDamageLossCapacityDirectText
            }
            Self::QueueDamageLossCapacityHudResource => {
                FactoryProductionOwnerActionPhase::QueueDamageLossCapacityHudResource
            }
            Self::PublishRepairCompleteCountdown => {
                FactoryProductionOwnerActionPhase::PublishRepairCompleteCountdown
            }
            Self::EmitRepairCapacityDirectText => {
                FactoryProductionOwnerActionPhase::EmitRepairCapacityDirectText
            }
            Self::QueueRepairCapacityHudResource => {
                FactoryProductionOwnerActionPhase::QueueRepairCapacityHudResource
            }
            Self::ApplyAnimationTransition { .. } => {
                FactoryProductionOwnerActionPhase::ApplyAnimationTransition
            }
            Self::PublishStatus(_) => FactoryProductionOwnerActionPhase::PublishStatus,
            Self::Start | Self::Production(_) | Self::Blocked(_) | Self::Complete => return None,
        })
    }
}

/// Non-replayable detached owner frame.
///
/// Deliberately neither `Copy` nor `Clone`: it may own a child production
/// transaction and linear receipts for destructive or presentation effects.
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryProductionOwnerMachine {
    transaction_id: FactoryProductionOwnerTransactionId,
    child_transaction_id: FactoryProductionTransactionId,
    request: FactoryProductionOwnerFrameRequest,
    current_health_raw: i32,
    progressive_death: ProgressiveDeathState,
    production: FactoryProductionRuntime,
    animation_state_raw: u32,
    stage: FactoryProductionOwnerStage,
    progressive_effects: Vec<ProgressiveModelEffect>,
    progressive_effect_index: usize,
    progressive_reached_terminal_death: bool,
    damaged_branch: bool,
    staffed_branch: bool,
    repair_animation_enabled: bool,
    status_published: bool,
    dirty_flag_written: bool,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
    issued_phase: Option<FactoryProductionOwnerActionPhase>,
}

impl FactoryProductionOwnerMachine {
    pub fn preflight(
        transaction_id: FactoryProductionOwnerTransactionId,
        child_transaction_id: FactoryProductionTransactionId,
        request: FactoryProductionOwnerFrameRequest,
    ) -> Result<Self, FactoryProductionOwnerAdmissionRejection> {
        if transaction_id.get() == child_transaction_id.get() {
            return Err(FactoryProductionOwnerAdmissionRejection::TransactionIdsMustBeDistinct);
        }
        let reconciled_loss_accumulator =
            request.production.health_loss_accumulator_raw.wrapping_add(
                request
                    .production
                    .cached_health_raw
                    .wrapping_sub(request.current_health_raw),
            );
        if request.progressive_death.elapsed_micros_raw == 0
            && request.production.cached_health_raw != request.current_health_raw
            && request.production.current_scientists_raw != 0
            && request.production.health_per_scientist_raw <= 0
            && request.production.health_per_scientist_raw < reconciled_loss_accumulator
        {
            return Err(FactoryProductionOwnerAdmissionRejection::NonPositiveHealthLossThreshold);
        }

        Ok(Self {
            transaction_id,
            child_transaction_id,
            current_health_raw: request.current_health_raw,
            progressive_death: request.progressive_death,
            production: request.production,
            animation_state_raw: request.animation_state_raw,
            request,
            stage: FactoryProductionOwnerStage::Start,
            progressive_effects: Vec::new(),
            progressive_effect_index: 0,
            progressive_reached_terminal_death: false,
            damaged_branch: false,
            staffed_branch: false,
            repair_animation_enabled: true,
            status_published: false,
            dirty_flag_written: false,
            next_action_sequence: 1,
            issued_action_sequence: None,
            issued_phase: None,
        })
    }

    pub const fn current_health_raw(&self) -> i32 {
        self.current_health_raw
    }

    pub const fn progressive_death(&self) -> ProgressiveDeathState {
        self.progressive_death
    }

    pub const fn production(&self) -> FactoryProductionRuntime {
        self.production
    }

    pub fn poll(&mut self) -> FactoryProductionOwnerPoll {
        if let Some(phase) = self.issued_phase {
            return FactoryProductionOwnerPoll::Awaiting(phase);
        }
        self.advance_internal_stages();

        if matches!(&self.stage, FactoryProductionOwnerStage::Production(_)) {
            return self.poll_child();
        }
        match &mut self.stage {
            FactoryProductionOwnerStage::Production(_) => {
                unreachable!("the nested production stage is polled above")
            }
            FactoryProductionOwnerStage::Blocked(block) => {
                FactoryProductionOwnerPoll::Blocked(*block)
            }
            FactoryProductionOwnerStage::Complete => {
                FactoryProductionOwnerPoll::Complete(self.completion())
            }
            _ => self.issue_owner_action(),
        }
    }

    pub fn resume(
        &mut self,
        receipt: FactoryProductionOwnerReceipt,
        resume: FactoryProductionOwnerResume,
    ) -> Result<(), FactoryProductionOwnerResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::NoOutstandingAction {
                    transaction_id: receipt.transaction_id,
                    action_sequence: receipt.action_sequence,
                },
                receipt,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.factory != self.request.factory {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::ReceiptFactoryMismatch {
                    expected: self.request.factory,
                    actual: receipt.factory,
                },
                receipt,
            });
        }
        if receipt.action_sequence != expected_sequence {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }
        let expected_phase = self
            .issued_phase
            .expect("issued sequence and phase are installed together");
        if resume.phase() != expected_phase {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::PhaseMismatch {
                    expected: expected_phase,
                    actual: resume.phase(),
                },
                receipt,
            });
        }

        if matches!(
            expected_phase,
            FactoryProductionOwnerActionPhase::Production(_)
        ) {
            return self.resume_child(receipt, resume);
        }
        if !matches!(
            resume,
            FactoryProductionOwnerResume::Acknowledged { .. }
                | FactoryProductionOwnerResume::Blocked { .. }
        ) {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::ResponseKindMismatch {
                    phase: expected_phase,
                },
                receipt,
            });
        }
        if !matches!(&receipt.kind, FactoryProductionOwnerReceiptKind::Owner) {
            return Err(FactoryProductionOwnerResumeFailure {
                error: FactoryProductionOwnerProtocolError::ResponseKindMismatch {
                    phase: expected_phase,
                },
                receipt,
            });
        }

        self.issued_action_sequence = None;
        self.issued_phase = None;
        match resume {
            FactoryProductionOwnerResume::Blocked { reason, .. } => {
                if !reason.matches_owner_phase(expected_phase) {
                    self.issued_action_sequence = Some(expected_sequence);
                    self.issued_phase = Some(expected_phase);
                    return Err(FactoryProductionOwnerResumeFailure {
                        error: FactoryProductionOwnerProtocolError::ResponseKindMismatch {
                            phase: expected_phase,
                        },
                        receipt,
                    });
                }
                self.stage =
                    FactoryProductionOwnerStage::Blocked(self.block(expected_phase, reason));
            }
            FactoryProductionOwnerResume::Acknowledged { .. } => {
                self.advance_after_owner_ack();
            }
            _ => unreachable!("owner response kind was checked before consuming the receipt"),
        }
        Ok(())
    }

    fn poll_child(&mut self) -> FactoryProductionOwnerPoll {
        let (poll, detached_state) = match &mut self.stage {
            FactoryProductionOwnerStage::Production(child) => {
                let poll = child.poll();
                (poll, child.detached_state())
            }
            _ => unreachable!("nested polling requires a production child"),
        };
        self.apply_child_state(detached_state);
        match poll {
            FactoryProductionPoll::Action(issued) => {
                let crate::factory_production_live::IssuedFactoryProductionAction {
                    receipt: child_receipt,
                    before_action,
                    action,
                } = issued;
                self.production = before_action.production;
                self.animation_state_raw = before_action.animation_state_raw;
                let child_phase = action.phase();
                let phase = FactoryProductionOwnerActionPhase::Production(child_phase);
                let action_sequence = self.take_next_action_sequence(phase);
                FactoryProductionOwnerPoll::Action(IssuedFactoryProductionOwnerAction {
                    receipt: FactoryProductionOwnerReceipt {
                        transaction_id: self.transaction_id,
                        action_sequence,
                        factory: self.request.factory,
                        kind: FactoryProductionOwnerReceiptKind::Production(child_receipt),
                    },
                    before_action: self.pre_action_state(),
                    action: FactoryProductionOwnerAction::Production { phase, action },
                })
            }
            FactoryProductionPoll::Awaiting(phase) => FactoryProductionOwnerPoll::Awaiting(
                FactoryProductionOwnerActionPhase::Production(phase),
            ),
            FactoryProductionPoll::Blocked(block) => {
                self.production = block.production;
                self.animation_state_raw = block.animation_state_raw;
                self.status_published = block.status_published;
                self.dirty_flag_written = block.dirty_flag_written;
                let block = self.block(
                    FactoryProductionOwnerActionPhase::Production(block.phase),
                    FactoryProductionOwnerExternalBlock::Production(block.reason),
                );
                self.stage = FactoryProductionOwnerStage::Blocked(block);
                FactoryProductionOwnerPoll::Blocked(block)
            }
            FactoryProductionPoll::Complete(completion) => {
                self.production = completion.production;
                self.animation_state_raw = completion.animation_state_raw;
                self.status_published = completion.status_published;
                self.dirty_flag_written = completion.dirty_flag_written;
                self.stage = FactoryProductionOwnerStage::Complete;
                FactoryProductionOwnerPoll::Complete(self.completion())
            }
        }
    }

    fn resume_child(
        &mut self,
        receipt: FactoryProductionOwnerReceipt,
        resume: FactoryProductionOwnerResume,
    ) -> Result<(), FactoryProductionOwnerResumeFailure> {
        let FactoryProductionOwnerReceipt {
            transaction_id,
            action_sequence,
            factory,
            kind,
        } = receipt;
        let FactoryProductionOwnerReceiptKind::Production(child_receipt) = kind else {
            return Err(FactoryProductionOwnerResumeFailure {
                receipt: FactoryProductionOwnerReceipt {
                    transaction_id,
                    action_sequence,
                    factory,
                    kind,
                },
                error: FactoryProductionOwnerProtocolError::ResponseKindMismatch {
                    phase: resume.phase(),
                },
            });
        };
        let FactoryProductionOwnerActionPhase::Production(child_phase) = resume.phase() else {
            unreachable!("the outer phase was validated before nested resumption")
        };

        if let FactoryProductionOwnerResume::Blocked {
            reason: FactoryProductionOwnerExternalBlock::FactoryStateCommitUnavailable,
            ..
        } = resume
        {
            let detached_state = match &self.stage {
                FactoryProductionOwnerStage::Production(child) => child.detached_state(),
                _ => unreachable!("an issued nested receipt retains its production child"),
            };
            self.apply_child_state(detached_state);
            self.issued_action_sequence = None;
            self.issued_phase = None;
            let phase = FactoryProductionOwnerActionPhase::Production(child_phase);
            let block = self.block(
                phase,
                FactoryProductionOwnerExternalBlock::FactoryStateCommitUnavailable,
            );
            self.stage = FactoryProductionOwnerStage::Blocked(block);
            return Ok(());
        }

        let child_resume = match resume {
            FactoryProductionOwnerResume::Acknowledged { .. } => {
                FactoryProductionResume::Acknowledged { phase: child_phase }
            }
            FactoryProductionOwnerResume::SpawnCompleted { result, .. } => {
                FactoryProductionResume::SpawnCompleted {
                    phase: child_phase,
                    result,
                }
            }
            FactoryProductionOwnerResume::PickupPresence { presence, .. } => {
                FactoryProductionResume::PickupPresence {
                    phase: child_phase,
                    presence,
                }
            }
            FactoryProductionOwnerResume::Blocked { reason, .. } => {
                let FactoryProductionOwnerExternalBlock::Production(reason) = reason else {
                    return Err(FactoryProductionOwnerResumeFailure {
                        receipt: FactoryProductionOwnerReceipt {
                            transaction_id,
                            action_sequence,
                            factory,
                            kind: FactoryProductionOwnerReceiptKind::Production(child_receipt),
                        },
                        error: FactoryProductionOwnerProtocolError::ResponseKindMismatch {
                            phase: FactoryProductionOwnerActionPhase::Production(child_phase),
                        },
                    });
                };
                FactoryProductionResume::Blocked {
                    phase: child_phase,
                    reason,
                }
            }
        };

        let child_result = match &mut self.stage {
            FactoryProductionOwnerStage::Production(child) => {
                child.resume(child_receipt, child_resume)
            }
            _ => unreachable!("an issued nested receipt retains its production child"),
        };
        if let Err(failure) = child_result {
            return Err(FactoryProductionOwnerResumeFailure {
                receipt: FactoryProductionOwnerReceipt {
                    transaction_id,
                    action_sequence,
                    factory,
                    kind: FactoryProductionOwnerReceiptKind::Production(failure.receipt),
                },
                error: FactoryProductionOwnerProtocolError::ChildProtocol(failure.error),
            });
        }

        let detached_state = match &self.stage {
            FactoryProductionOwnerStage::Production(child) => child.detached_state(),
            _ => unreachable!("a successful nested resume retains its production child"),
        };
        self.apply_child_state(detached_state);
        self.issued_action_sequence = None;
        self.issued_phase = None;
        Ok(())
    }

    fn apply_child_state(&mut self, state: FactoryProductionDetachedState) {
        self.production = state.production;
        self.animation_state_raw = state.animation_state_raw;
        self.status_published = state.status_published;
        self.dirty_flag_written = state.dirty_flag_written;
    }

    fn advance_internal_stages(&mut self) {
        if matches!(&self.stage, FactoryProductionOwnerStage::Start) {
            self.begin_frame();
        }
    }

    fn begin_frame(&mut self) {
        if self.progressive_death.elapsed_micros_raw != 0 {
            let advanced_elapsed_micros_raw = self
                .progressive_death
                .elapsed_micros_raw
                .wrapping_add(self.request.elapsed_micros as i32);
            let ProgressiveDeathAdvance {
                state,
                effects,
                reached_terminal_death,
            } = self.progressive_death.advance(self.request.elapsed_micros);
            self.progressive_death = if reached_terminal_death {
                // FUN_00419B50 stores the advanced positive clock before its
                // stage walk. A crossed stage-31 model effect therefore sees
                // old+dt; the stage-32 branch writes -1 only after those
                // synchronous effects have returned.
                ProgressiveDeathState {
                    elapsed_micros_raw: advanced_elapsed_micros_raw,
                    ..state
                }
            } else {
                state
            };
            self.progressive_effects = effects;
            self.progressive_effect_index = 0;
            self.progressive_reached_terminal_death = reached_terminal_death;
            if self.progressive_effects.is_empty() {
                self.enter_after_progressive_effects();
            } else {
                self.stage = FactoryProductionOwnerStage::ProgressiveModelEffect;
            }
            return;
        }

        self.damaged_branch = self.current_health_raw != self.request.maximum_health_raw;
        if self.reconcile_cached_health() {
            self.enter_staffing_branch();
        }
    }

    fn reconcile_cached_health(&mut self) -> bool {
        let old_cached_health = self.production.cached_health_raw;
        if old_cached_health == self.current_health_raw {
            return true;
        }
        self.production.cached_health_raw = self.current_health_raw;
        if self.production.current_scientists_raw == 0 {
            if self.production.health_per_scientist_raw != 0 {
                self.production.scientist_capacity_raw =
                    self.request.maximum_health_raw / self.production.health_per_scientist_raw;
            }
            return true;
        }

        self.production.health_loss_accumulator_raw = self
            .production
            .health_loss_accumulator_raw
            .wrapping_add(old_cached_health.wrapping_sub(self.current_health_raw));
        self.continue_health_loss_loop()
    }

    fn continue_health_loss_loop(&mut self) -> bool {
        while self.production.health_per_scientist_raw < self.production.health_loss_accumulator_raw
        {
            self.production.health_loss_accumulator_raw = self
                .production
                .health_loss_accumulator_raw
                .wrapping_sub(self.production.health_per_scientist_raw);

            // Retail first calls FUN_00418C20(factory, -1) and then performs
            // a second explicit decrement at +0x68. This apparent double
            // decrement is preserved intentionally.
            let candidate = self.production.current_scientists_raw.wrapping_sub(1);
            if candidate < self.production.scientist_capacity_raw {
                self.production.current_scientists_raw = candidate;
                if self.production.understaffed_limit_micros_raw > 0 {
                    self.production.understaffed_countdown_micros_raw = self
                        .production
                        .understaffed_countdown_micros_raw
                        .wrapping_add(self.production.understaffed_step_per_scientist_raw)
                        .max(0);
                }
                self.finish_health_loss_iteration();
            } else {
                self.production.current_scientists_raw = self.production.scientist_capacity_raw;
                self.stage = if self.production.understaffed_limit_micros_raw != 0 {
                    FactoryProductionOwnerStage::PublishDamageLossCapacityCountdown
                } else {
                    FactoryProductionOwnerStage::QueueDamageLossCapacityHudResource
                };
                return false;
            }
        }
        true
    }

    fn finish_health_loss_iteration(&mut self) {
        self.production.current_scientists_raw =
            self.production.current_scientists_raw.wrapping_sub(1);
        if self.production.current_scientists_raw == 0 {
            self.production.health_loss_accumulator_raw = 0;
        }
    }

    fn enter_staffing_branch(&mut self) {
        self.staffed_branch =
            self.production.current_scientists_raw >= self.production.scientist_capacity_raw;
        if self.staffed_branch {
            if self.damaged_branch {
                self.decay_staffed_countdown();
                self.repair_damaged_factory();
            } else {
                self.start_production_child();
            }
            return;
        }

        if self.production.understaffed_limit_micros_raw != 0 {
            self.production.understaffed_countdown_micros_raw = self
                .production
                .understaffed_countdown_micros_raw
                .wrapping_add(self.request.elapsed_micros as i32);
            if self.production.understaffed_limit_micros_raw
                <= self.production.understaffed_countdown_micros_raw
            {
                self.stage = FactoryProductionOwnerStage::EmitUnderstaffedDirectText;
            } else {
                self.stage = FactoryProductionOwnerStage::PublishUnderstaffedCountdown;
            }
        } else {
            self.enter_after_staffing();
        }
    }

    fn decay_staffed_countdown(&mut self) {
        if self.request.elapsed_micros < self.production.understaffed_countdown_micros_raw as u32 {
            self.production.understaffed_countdown_micros_raw = self
                .production
                .understaffed_countdown_micros_raw
                .wrapping_sub(self.request.elapsed_micros as i32);
        } else {
            self.production.understaffed_countdown_micros_raw = 0;
        }
    }

    fn enter_after_staffing(&mut self) {
        if self.damaged_branch {
            self.repair_damaged_factory();
        } else {
            self.enter_common_tail();
        }
    }

    fn start_production_child(&mut self) {
        let child_request = FactoryProductionFrameRequest {
            factory: self.request.factory,
            position_raw: self.request.position_raw,
            pickup_spawn_offset_raw: self.request.pickup_spawn_offset_raw,
            current_health_raw: self.current_health_raw,
            maximum_health_raw: self.request.maximum_health_raw,
            progressive_death_elapsed_raw: self.progressive_death.elapsed_micros_raw,
            production: self.production,
            animation_state_raw: self.animation_state_raw,
            elapsed_micros: self.request.elapsed_micros,
            world_style_raw: self.request.world_style_raw,
            phase1_presentation_enabled: self.request.phase1_presentation_enabled,
            suppress_status_publication: self.request.suppress_status_publication,
        };
        let child = FactoryProductionMachine::preflight(self.child_transaction_id, child_request)
            .expect("the outer owner admits a child only after proving its invariants");
        self.stage = FactoryProductionOwnerStage::Production(child);
    }

    fn repair_damaged_factory(&mut self) {
        self.repair_animation_enabled = true;
        let repair_increment_base = (self.request.elapsed_micros >> 8) as i32;
        if self.production.scientist_capacity_raw == 0 {
            self.production.repair_rate_remainder_raw = self
                .production
                .repair_rate_remainder_raw
                .wrapping_add(repair_increment_base.wrapping_mul(self.production.repair_rate_raw));
        } else if self.production.current_scientists_raw != 0 {
            self.production.repair_rate_remainder_raw =
                self.production.repair_rate_remainder_raw.wrapping_add(
                    repair_increment_base
                        .wrapping_mul(self.production.repair_rate_raw)
                        .wrapping_mul(self.production.current_scientists_raw),
                );
        } else {
            self.repair_animation_enabled = false;
        }

        let repair_delta = self.production.repair_rate_remainder_raw >> 12;
        if repair_delta != 0 {
            self.production.repair_rate_remainder_raw = self
                .production
                .repair_rate_remainder_raw
                .wrapping_sub(repair_delta.wrapping_mul(0x1000));
            self.production.health_loss_accumulator_raw = self
                .production
                .health_loss_accumulator_raw
                .wrapping_sub(repair_delta)
                .max(0);
            self.current_health_raw = self.current_health_raw.wrapping_add(repair_delta);
            if self.request.maximum_health_raw <= self.current_health_raw {
                self.current_health_raw = self.request.maximum_health_raw;
                self.damaged_branch = false;
                self.production.cached_health_raw = self.current_health_raw;
                self.begin_repair_capacity_reconciliation();
                return;
            }
            self.production.cached_health_raw = self.current_health_raw;
        }
        self.enter_common_tail();
    }

    fn begin_repair_capacity_reconciliation(&mut self) {
        let candidate = self.production.current_scientists_raw;
        if candidate < self.production.scientist_capacity_raw {
            self.enter_common_tail();
            return;
        }
        self.production.current_scientists_raw = self.production.scientist_capacity_raw;
        if self.production.understaffed_limit_micros_raw != 0 {
            self.stage = FactoryProductionOwnerStage::PublishRepairCompleteCountdown;
        } else {
            self.stage = FactoryProductionOwnerStage::QueueRepairCapacityHudResource;
        }
    }

    fn enter_after_progressive_effects(&mut self) {
        if self.progressive_reached_terminal_death {
            // Stage 32 selects FUN_00419750's terminal re-entry only after any
            // earlier crossed-stage model effects have completed.
            self.progressive_death.elapsed_micros_raw = -1;
            self.stage = FactoryProductionOwnerStage::QueueProgressivePickupDeferredDestroy;
        } else {
            self.enter_progressive_animation_tail();
        }
    }

    fn enter_progressive_animation_tail(&mut self) {
        if self.animation_state_raw != 0 {
            self.stage = FactoryProductionOwnerStage::ApplyAnimationTransition {
                transition: animation_transition(self.animation_state_raw, 0),
                completes_frame: true,
            };
        } else {
            self.stage = FactoryProductionOwnerStage::Complete;
        }
    }

    fn enter_common_tail(&mut self) {
        let mut next_animation_state = u32::from(self.staffed_branch);
        if !self.damaged_branch {
            if matches!(
                self.production.phase,
                FactoryProductionPhase::Delivering | FactoryProductionPhase::Cooldown
            ) {
                next_animation_state |= 2;
            }
        } else if self.repair_animation_enabled {
            next_animation_state |= 4;
        }

        if next_animation_state != self.animation_state_raw {
            self.stage = FactoryProductionOwnerStage::ApplyAnimationTransition {
                transition: animation_transition(self.animation_state_raw, next_animation_state),
                completes_frame: false,
            };
        } else {
            self.enter_status_tail();
        }
    }

    fn enter_status_tail(&mut self) {
        if self.request.suppress_status_publication {
            self.stage = FactoryProductionOwnerStage::Complete;
        } else {
            self.stage =
                FactoryProductionOwnerStage::PublishStatus(published_status(self.production));
        }
    }

    fn issue_owner_action(&mut self) -> FactoryProductionOwnerPoll {
        let phase = self
            .stage
            .owner_action_phase()
            .expect("an owner action stage exposes a phase");
        let action = match &self.stage {
            FactoryProductionOwnerStage::ProgressiveModelEffect => {
                FactoryProductionOwnerAction::DispatchProgressiveModelEffect {
                    phase,
                    factory: self.request.factory,
                    effect: self.progressive_effects[self.progressive_effect_index],
                }
            }
            FactoryProductionOwnerStage::QueueProgressivePickupDeferredDestroy => {
                FactoryProductionOwnerAction::QueueDeferredDestroy {
                    phase,
                    entity_id: self.production.spawned_pickup_handle,
                }
            }
            FactoryProductionOwnerStage::InvokeProgressiveFactoryDeath => {
                FactoryProductionOwnerAction::InvokeProgressiveFactoryDeath {
                    phase,
                    factory: self.request.factory,
                }
            }
            FactoryProductionOwnerStage::DispatchProgressiveDeathPresentation => {
                FactoryProductionOwnerAction::DispatchProgressiveDeathPresentation { phase }
            }
            FactoryProductionOwnerStage::EmitUnderstaffedDirectText => {
                FactoryProductionOwnerAction::EmitDirectText {
                    phase,
                    direct_text_id: FACTORY_OWNER_UNDERSTAFFED_DIRECT_TEXT_ID,
                    sub_parameter: 0,
                }
            }
            FactoryProductionOwnerStage::InvokeUnderstaffedDeath => {
                FactoryProductionOwnerAction::InvokeUnderstaffedDeath {
                    phase,
                    factory: self.request.factory,
                }
            }
            FactoryProductionOwnerStage::PublishUnderstaffedCountdown => {
                FactoryProductionOwnerAction::PublishCountdownSeconds {
                    phase,
                    remaining_seconds_raw: self
                        .production
                        .understaffed_limit_micros_raw
                        .wrapping_sub(self.production.understaffed_countdown_micros_raw)
                        / 1_000_000,
                }
            }
            FactoryProductionOwnerStage::PublishDamageLossCapacityCountdown => {
                FactoryProductionOwnerAction::PublishCountdownSeconds {
                    phase,
                    remaining_seconds_raw: 0,
                }
            }
            FactoryProductionOwnerStage::EmitDamageLossCapacityDirectText => {
                FactoryProductionOwnerAction::EmitDirectText {
                    phase,
                    direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                    sub_parameter: 0,
                }
            }
            FactoryProductionOwnerStage::QueueDamageLossCapacityHudResource => {
                FactoryProductionOwnerAction::QueueHudResource {
                    phase,
                    resource_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
                }
            }
            FactoryProductionOwnerStage::PublishRepairCompleteCountdown => {
                FactoryProductionOwnerAction::PublishCountdownSeconds {
                    phase,
                    remaining_seconds_raw: 0,
                }
            }
            FactoryProductionOwnerStage::EmitRepairCapacityDirectText => {
                FactoryProductionOwnerAction::EmitDirectText {
                    phase,
                    direct_text_id: FACTORY_OWNER_CAPACITY_DIRECT_TEXT_ID,
                    sub_parameter: 0,
                }
            }
            FactoryProductionOwnerStage::QueueRepairCapacityHudResource => {
                FactoryProductionOwnerAction::QueueHudResource {
                    phase,
                    resource_id: FACTORY_OWNER_CAPACITY_HUD_RESOURCE_ID,
                }
            }
            FactoryProductionOwnerStage::ApplyAnimationTransition { transition, .. } => {
                FactoryProductionOwnerAction::ApplyAnimationTransition {
                    phase,
                    factory: self.request.factory,
                    transition: *transition,
                }
            }
            FactoryProductionOwnerStage::PublishStatus(status) => {
                FactoryProductionOwnerAction::PublishStatus {
                    phase,
                    factory: self.request.factory,
                    status: *status,
                }
            }
            _ => unreachable!("non-owner stages are handled before action issue"),
        };
        let before_action = self.pre_action_state();
        let action_sequence = self.take_next_action_sequence(phase);
        FactoryProductionOwnerPoll::Action(IssuedFactoryProductionOwnerAction {
            receipt: FactoryProductionOwnerReceipt {
                transaction_id: self.transaction_id,
                action_sequence,
                factory: self.request.factory,
                kind: FactoryProductionOwnerReceiptKind::Owner,
            },
            before_action,
            action,
        })
    }

    fn advance_after_owner_ack(&mut self) {
        match self.stage {
            FactoryProductionOwnerStage::ProgressiveModelEffect => {
                self.progressive_effect_index += 1;
                if self.progressive_effect_index < self.progressive_effects.len() {
                    self.stage = FactoryProductionOwnerStage::ProgressiveModelEffect;
                } else {
                    self.enter_after_progressive_effects();
                }
            }
            FactoryProductionOwnerStage::QueueProgressivePickupDeferredDestroy => {
                self.stage = FactoryProductionOwnerStage::InvokeProgressiveFactoryDeath;
            }
            FactoryProductionOwnerStage::InvokeProgressiveFactoryDeath => {
                // The acknowledged FUN_00410C10 writes terminal health before
                // entering FUN_00419750; that callback then clears both live
                // staffing words and publishes the resulting component
                // variables before FUN_00419B50 resumes.
                self.current_health_raw = 0;
                self.production.current_scientists_raw = 0;
                self.production.scientist_capacity_raw = 0;
                self.status_published = true;
                if self.request.suppress_status_publication {
                    self.enter_progressive_animation_tail();
                } else {
                    self.stage = FactoryProductionOwnerStage::DispatchProgressiveDeathPresentation;
                }
            }
            FactoryProductionOwnerStage::DispatchProgressiveDeathPresentation => {
                self.enter_progressive_animation_tail();
            }
            FactoryProductionOwnerStage::EmitUnderstaffedDirectText => {
                self.stage = FactoryProductionOwnerStage::InvokeUnderstaffedDeath;
            }
            FactoryProductionOwnerStage::InvokeUnderstaffedDeath => {
                if let ProgressiveDeathBegin::Revived { state } = self.progressive_death.begin() {
                    self.progressive_death = state;
                }
                self.current_health_raw = PROGRESSION_REVIVE_HEALTH_RAW;
                self.stage = FactoryProductionOwnerStage::PublishUnderstaffedCountdown;
            }
            FactoryProductionOwnerStage::PublishUnderstaffedCountdown => {
                self.enter_after_staffing();
            }
            FactoryProductionOwnerStage::PublishDamageLossCapacityCountdown => {
                self.stage = FactoryProductionOwnerStage::EmitDamageLossCapacityDirectText;
            }
            FactoryProductionOwnerStage::EmitDamageLossCapacityDirectText => {
                self.production.understaffed_countdown_micros_raw = 0;
                self.stage = FactoryProductionOwnerStage::QueueDamageLossCapacityHudResource;
            }
            FactoryProductionOwnerStage::QueueDamageLossCapacityHudResource => {
                self.finish_health_loss_iteration();
                if self.continue_health_loss_loop() {
                    self.enter_staffing_branch();
                }
            }
            FactoryProductionOwnerStage::PublishRepairCompleteCountdown => {
                self.stage = FactoryProductionOwnerStage::EmitRepairCapacityDirectText;
            }
            FactoryProductionOwnerStage::EmitRepairCapacityDirectText => {
                self.production.understaffed_countdown_micros_raw = 0;
                self.stage = FactoryProductionOwnerStage::QueueRepairCapacityHudResource;
            }
            FactoryProductionOwnerStage::QueueRepairCapacityHudResource => {
                self.enter_common_tail();
            }
            FactoryProductionOwnerStage::ApplyAnimationTransition {
                transition,
                completes_frame,
            } => {
                //18F60's acknowledged C8E0 writes belong to the retained
                // production component as well as its animation state.
                self.production.retune_voices_from_animation_state(
                    transition.next_state_raw,
                    transition.previous_state_raw,
                );
                self.animation_state_raw = transition.next_state_raw;
                if completes_frame {
                    self.stage = FactoryProductionOwnerStage::Complete;
                } else {
                    self.enter_status_tail();
                }
            }
            FactoryProductionOwnerStage::PublishStatus(_) => {
                self.status_published = true;
                self.stage = FactoryProductionOwnerStage::Complete;
            }
            _ => unreachable!("only an owner action stage accepts an owner acknowledgement"),
        }
    }

    fn pre_action_state(&self) -> FactoryProductionOwnerPreActionState {
        FactoryProductionOwnerPreActionState {
            factory: self.request.factory,
            entry_flags_at_0x84_or_mask: FACTORY_OWNER_ENTRY_FLAGS_AT_0X84_OR_MASK,
            current_health_raw: self.current_health_raw,
            progressive_death: self.progressive_death,
            production: self.production,
            animation_state_raw: self.animation_state_raw,
            status_published: self.status_published,
            dirty_flag_written: self.dirty_flag_written,
        }
    }

    fn take_next_action_sequence(&mut self, phase: FactoryProductionOwnerActionPhase) -> u64 {
        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one factory owner frame cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        self.issued_phase = Some(phase);
        action_sequence
    }

    fn block(
        &self,
        phase: FactoryProductionOwnerActionPhase,
        reason: FactoryProductionOwnerExternalBlock,
    ) -> FactoryProductionOwnerBlock {
        FactoryProductionOwnerBlock {
            factory: self.request.factory,
            entry_flags_at_0x84_or_mask: FACTORY_OWNER_ENTRY_FLAGS_AT_0X84_OR_MASK,
            phase,
            reason,
            current_health_raw: self.current_health_raw,
            progressive_death: self.progressive_death,
            production: self.production,
            animation_state_raw: self.animation_state_raw,
            status_published: self.status_published,
            dirty_flag_written: self.dirty_flag_written,
        }
    }

    fn completion(&self) -> FactoryProductionOwnerCompletion {
        FactoryProductionOwnerCompletion {
            factory: self.request.factory,
            entry_flags_at_0x84_or_mask: FACTORY_OWNER_ENTRY_FLAGS_AT_0X84_OR_MASK,
            current_health_raw: self.current_health_raw,
            progressive_death: self.progressive_death,
            production: self.production,
            animation_state_raw: self.animation_state_raw,
            status_published: self.status_published,
            dirty_flag_written: self.dirty_flag_written,
            callback_result: 0,
        }
    }
}
