//! Detached, resumable primary-hit transaction from retail `FUN_00410EB0`.
//!
//! Retail interleaves one unconditional cached-entity write with an optional
//! type callback, two helpers which resolve the target independently, and a
//! final suffix which returns to the cached entity/type pointers. A one-shot
//! planner would either replay already committed work after an opaque callback
//! or incorrectly turn those cached pointers into fresh handle lookups.
//!
//! This state machine therefore stops at every external boundary. Numeric
//! allocation/type identities are opaque adapter tokens; Rust never
//! dereferences a retail pointer. The impact-reaction and checked-damage
//! actions explicitly model their independent handle lookups, while the
//! accepted-hit suffix samples the original cached allocation only after
//! checked damage has unwound and samples it again after sound playback. A
//! deleted/reused cached allocation becomes a durable block rather than a
//! synthetic fresh binding.
//!
//! Each emitted action carries a transaction-bound, monotonically sequenced
//! receipt. Transaction IDs must be unique among all live machines. The
//! machine never reissues an outstanding action and accepts only the exact
//! receipt it emitted for the current boundary. This prevents stale or
//! cross-transaction completions from advancing the protocol. It cannot, by
//! itself, prevent an adapter from executing the same borrowed action more than
//! once before returning its receipt: adapters must journal or deduplicate
//! external execution by `(transaction_id, action_sequence)`.
//!
//! A recoverable protocol rejection returns ownership of the submitted receipt
//! in [`PrimaryHitResumeFailure`]. The adapter must retain its journal entry and
//! either retry the corrected completion or route a cross-machine receipt back
//! to its source. Only `Ok(())` consumes the receipt: at that point the adapter
//! may drop the journal entry, whether the completion advanced the retail path
//! or durably blocked it.
//!
//! No live adapter is installed here. In particular, checked delivery retains
//! ownership of damage modifiers, generic hit callbacks, actor death, common
//! dying, and type-specific survivor/death behavior.

use std::num::{NonZeroI32, NonZeroU64};

use crate::checked_damage::CheckedDamageCompletion;
use crate::damage::DamageDeliveryRecord;
use crate::entity_collision_state::{
    active_model_slot_from_state_flags, RetailRuntimeValue, DYING_STATE_BIT,
};
use crate::impact_reaction::{
    ImpactReactionError, ImpactReactionNetworkRequest, ImpactReactionOutcome,
};

/// Complete primary-hit wrapper.
pub const PRIMARY_HIT_ADDRESS: u32 = 0x0041_0EB0;
/// Common type-vtable `+0x14` wrapper recovered for the retail type table.
pub const PRIMARY_HIT_TYPE_CALLBACK_ADDRESS: u32 = 0x0040_DAC0;
/// Independent impulse/jolt helper called after the optional type callback.
pub const PRIMARY_HIT_REACTION_ADDRESS: u32 = 0x0041_1030;
/// Independently resolving checked-damage helper.
pub const PRIMARY_HIT_CHECKED_DAMAGE_ADDRESS: u32 = 0x0041_5040;
/// Final capability flag at cached entity byte `+0x64`.
pub const PRIMARY_HIT_CAPABILITY_EFFECT_BIT: u8 = 0x08;
/// Particle class submitted by the final capability branch.
pub const PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS: u8 = 5;
/// Fixed particle scale submitted by the final capability branch.
pub const PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW: u16 = 0x0800;

/// Cached outer pointers resolved before any callback in `FUN_00410EB0`.
///
/// Identity values are host-safe opaque tokens. They must identify allocation
/// instances, not just entity handles or type numbers, so a delete/reuse cycle
/// cannot be mistaken for the cached retail pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryHitCachedEntry {
    pub target_allocation_identity: u64,
    pub type_record_identity: u64,
    /// Type-vtable `+0x14`, read through the cached initial type record after
    /// the tick stamp. The common binding is [`PRIMARY_HIT_TYPE_CALLBACK_ADDRESS`].
    /// The callback itself freshly resolves the target and current style.
    pub type_callback_address: RetailRuntimeValue<Option<u32>>,
}

/// Immutable call arguments captured at primary-hit entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryHitRequest {
    pub target_handle: u32,
    /// Complete six-dword delivery record. The impact sum uses only its
    /// four-word packet view; checked delivery retains both provenance words.
    pub delivery: DamageDeliveryRecord,
    pub source_provenance: u32,
    pub direction_q15: [i16; 3],
    pub retail_tick: u32,
    pub cached_entry: RetailRuntimeValue<Option<PrimaryHitCachedEntry>>,
}

/// Selected model binding needed by the cached capability-effect suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryHitModelBinding {
    pub global_model_id: u16,
    /// Signed model-resource word `+0x08`, subtracted from entity Z.
    pub base_z_raw: i16,
}

/// In-place state read through the cached outer target/type pointers at one
/// accepted-suffix sampling boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryHitCachedTargetSnapshot {
    pub target_allocation_identity: u64,
    pub cached_type_record_identity: u64,
    pub state_flags_at_0x08: u32,
    pub capability_flags_at_0x64: u8,
    pub position_raw: [i16; 3],
    pub model_by_state_slot: [RetailRuntimeValue<Option<PrimaryHitModelBinding>>; 4],
    /// Post-delivery read through the cached type-record pointer at `+0x80`.
    /// In-place mutation of that record is therefore observed; zero is
    /// represented as `Known(None)`.
    pub accepted_hit_sound_id: RetailRuntimeValue<Option<u16>>,
}

/// Exact final `FUN_00440DC0` request after model-slot selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryHitCapabilityEmission {
    pub target_handle: u32,
    pub target_allocation_identity: u64,
    pub selected_model_slot: usize,
    pub selected_global_model_id: u16,
    pub position_raw: [i16; 3],
    pub particle_class: u8,
    pub particle_scale_raw: u16,
    /// Exact `state_flags >> 31` value copied by retail.
    pub owner_sign: u32,
}

/// Independent lookup result from `FUN_00411030`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitImpactReactionResult {
    /// The helper's fresh handle lookup no longer found the target. Retail
    /// subsequently dereferences that null result, so the machine converts
    /// this observed lifetime hazard into a durable block.
    TargetMissing,
    /// The adapter has already committed any six reaction words.
    Committed(ImpactReactionOutcome),
}

/// Return from independently resolving checked delivery `FUN_00415040`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitCheckedDamageResult {
    /// The checked helper's fresh handle lookup no longer found the target.
    TargetMissing,
    /// Retail returned zero after eligibility/filter/modifier processing.
    ReturnedZero,
    /// Retail returned the exact nonzero signed bits after all target-owned
    /// health/death callbacks completed.
    ReturnedNonZero { accepted_damage_raw: NonZeroI32 },
}

impl From<CheckedDamageCompletion> for PrimaryHitCheckedDamageResult {
    fn from(completion: CheckedDamageCompletion) -> Self {
        match completion {
            CheckedDamageCompletion::TargetMissing => Self::TargetMissing,
            CheckedDamageCompletion::ReturnedZero(_) => Self::ReturnedZero,
            CheckedDamageCompletion::ReturnedNonZero(accepted_damage_raw) => {
                Self::ReturnedNonZero {
                    accepted_damage_raw,
                }
            }
        }
    }
}

/// One externally visible boundary, also used as an anti-replay tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitPhase {
    CommitTickStamp,
    TypeImpactCallback,
    ImpactReaction,
    ImpactReactionNetwork,
    CheckedDamageDelivery,
    SampleCachedTargetAfterCheckedDamage,
    AcceptedHitSound,
    SampleCachedTargetAfterAcceptedHitSound,
    CapabilityFollowUp,
}

/// Caller-supplied identity for one detached primary-hit transaction.
///
/// The host owns identity allocation. A stable entity handle is not sufficient:
/// concurrent or later hits against the same handle must receive distinct
/// transaction IDs. IDs must remain unique across every simultaneously live
/// [`PrimaryHitMachine`]; reusing an ID while an older machine or adapter
/// journal entry is live defeats cross-machine receipt isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrimaryHitTransactionId(NonZeroU64);

impl PrimaryHitTransactionId {
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

/// Adapter action in exact retail order.
#[derive(Debug, PartialEq, Eq)]
pub enum PrimaryHitAction {
    CommitTickStamp {
        phase: PrimaryHitPhase,
        target_handle: u32,
        target_allocation_identity: u64,
        retail_tick: u32,
    },
    InvokeTypeImpactCallback {
        phase: PrimaryHitPhase,
        callback_address: u32,
        target_handle: u32,
        impact_sum_raw: i32,
        source_provenance: u32,
        direction_q15: [i16; 3],
    },
    ApplyImpactReaction {
        phase: PrimaryHitPhase,
        target_handle: u32,
        impact_sum_raw: i32,
        direction_q15: [i16; 3],
    },
    SubmitImpactReactionNetwork {
        phase: PrimaryHitPhase,
        request: ImpactReactionNetworkRequest,
    },
    DeliverCheckedDamage {
        phase: PrimaryHitPhase,
        target_handle: u32,
        delivery: DamageDeliveryRecord,
        ratio_numerator: u32,
        ratio_denominator: u16,
    },
    /// Read-only sample through the outer target/type pointers cached at
    /// primary-hit entry. This is intentionally a distinct boundary after
    /// checked damage has fully unwound and again after an accepted-hit sound.
    SampleCachedOuterTarget {
        phase: PrimaryHitPhase,
        target_handle: u32,
        target_allocation_identity: u64,
        cached_type_record_identity: u64,
    },
    PlayAcceptedHitSound {
        phase: PrimaryHitPhase,
        sound_id: u16,
        position_raw: [i16; 3],
    },
    EmitCapabilityFollowUp {
        phase: PrimaryHitPhase,
        emission: PrimaryHitCapabilityEmission,
    },
}

impl PrimaryHitAction {
    pub const fn phase(&self) -> PrimaryHitPhase {
        match self {
            Self::CommitTickStamp { phase, .. }
            | Self::InvokeTypeImpactCallback { phase, .. }
            | Self::ApplyImpactReaction { phase, .. }
            | Self::SubmitImpactReactionNetwork { phase, .. }
            | Self::DeliverCheckedDamage { phase, .. }
            | Self::SampleCachedOuterTarget { phase, .. }
            | Self::PlayAcceptedHitSound { phase, .. }
            | Self::EmitCapabilityFollowUp { phase, .. } => *phase,
        }
    }
}

/// Linear proof that one specific machine action was issued.
///
/// Fields are private and the type is deliberately neither `Copy` nor `Clone`.
/// The only production constructor is [`PrimaryHitMachine::poll`].
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryHitReceipt {
    transaction_id: PrimaryHitTransactionId,
    action_sequence: u64,
}

impl PrimaryHitReceipt {
    pub const fn transaction_id(&self) -> PrimaryHitTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }
}

/// One non-`Clone` action/receipt pair returned to the adapter.
///
/// The adapter must journal external execution by the receipt identity. Rust
/// ownership prevents accidental value duplication, but cannot stop an adapter
/// from executing the same borrowed action twice before consuming its receipt.
/// Keep that journal entry until [`PrimaryHitMachine::resume`] returns `Ok(())`.
/// On [`PrimaryHitResumeFailure`], use the returned receipt to retry or route
/// the completion; do not discard the journal entry as though the boundary had
/// been accepted.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedPrimaryHitAction {
    pub receipt: PrimaryHitReceipt,
    pub action: PrimaryHitAction,
}

/// Adapter-side reason that an otherwise proven boundary cannot complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitExternalBlock {
    TickStampUnavailable,
    TypeImpactCallbackUnavailable,
    ImpactReactionUnavailable,
    ImpactReactionError(ImpactReactionError),
    ImpactReactionNetworkUnavailable,
    CheckedDamageUnavailable,
    CachedOuterTargetSampleUnavailable,
    AcceptedHitSoundUnavailable,
    CapabilityFollowUpUnavailable,
}

/// Durable evidence/runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitBlock {
    UnresolvedInitialCachedEntry,
    MissingInitialCachedTarget,
    UnresolvedTypeImpactCallback,
    /// A non-common type-vtable binding has not been audited against the
    /// `FUN_00410EB0` callback ABI. Retail would invoke it; this is not a
    /// suppression rule.
    UnsupportedTypeImpactCallback(u32),
    TargetMissingDuringImpactReaction,
    MissingCachedOuterTargetAfterAcceptedDamage,
    UnresolvedCachedOuterTargetAfterAcceptedDamage,
    MissingCachedOuterTargetAfterAcceptedHitSound,
    UnresolvedCachedOuterTargetAfterAcceptedHitSound,
    CachedTargetAllocationIdentityMismatch {
        expected: u64,
        actual: u64,
    },
    CachedTypeRecordIdentityMismatch {
        expected: u64,
        actual: u64,
    },
    UnresolvedAcceptedHitSound,
    MissingSelectedModel {
        slot: usize,
    },
    UnresolvedSelectedModel {
        slot: usize,
    },
    External {
        phase: PrimaryHitPhase,
        reason: PrimaryHitExternalBlock,
    },
}

/// Phase-tagged completion supplied by the adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitResume {
    Acknowledged {
        phase: PrimaryHitPhase,
    },
    ImpactReactionResolved {
        phase: PrimaryHitPhase,
        result: PrimaryHitImpactReactionResult,
    },
    CheckedDamageReturned {
        phase: PrimaryHitPhase,
        result: PrimaryHitCheckedDamageResult,
    },
    CachedOuterTargetSampled {
        phase: PrimaryHitPhase,
        cached_outer_target: RetailRuntimeValue<Option<PrimaryHitCachedTargetSnapshot>>,
    },
    Blocked {
        phase: PrimaryHitPhase,
        reason: PrimaryHitExternalBlock,
    },
}

impl PrimaryHitResume {
    pub const fn phase(self) -> PrimaryHitPhase {
        match self {
            Self::Acknowledged { phase }
            | Self::ImpactReactionResolved { phase, .. }
            | Self::CheckedDamageReturned { phase, .. }
            | Self::CachedOuterTargetSampled { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

/// Stable terminal distinction from the checked-damage helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitCompletion {
    TargetMissingDuringCheckedDamage,
    CheckedDamageReturnedZero,
    Accepted(NonZeroI32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrimaryHitPoll {
    Action(IssuedPrimaryHitAction),
    /// The current action has already been issued and must be completed before
    /// it can be observed again. Returning the action again would permit an
    /// adapter to replay an already committed retail side effect.
    Awaiting(PrimaryHitPhase),
    Blocked(PrimaryHitBlock),
    Complete(PrimaryHitCompletion),
}

/// Invalid response which never advances or blocks the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryHitProtocolError {
    NoOutstandingAction {
        transaction_id: PrimaryHitTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: PrimaryHitTransactionId,
        actual: PrimaryHitTransactionId,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: PrimaryHitPhase,
        actual: PrimaryHitPhase,
    },
    UnexpectedResumeKind {
        phase: PrimaryHitPhase,
    },
}

/// Recoverable rejection of an adapter completion.
///
/// The receipt is returned intact because the machine is still awaiting that
/// boundary. The adapter must retain the corresponding execution journal entry
/// and use this receipt to retry a corrected completion or, after a
/// cross-machine mismatch, route it back to the machine that issued it. A
/// successful or explicitly blocked completion returns `Ok(())`, consumes the
/// receipt, and permits the adapter to drop that journal entry.
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryHitResumeFailure {
    pub receipt: PrimaryHitReceipt,
    pub error: PrimaryHitProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimaryHitStage {
    CommitTickStamp,
    TypeImpactCallback(u32),
    ImpactReaction,
    ImpactReactionNetwork(ImpactReactionNetworkRequest),
    CheckedDamageDelivery,
    SampleCachedTargetAfterCheckedDamage,
    AcceptedHitSound {
        sound_id: u16,
        position_raw: [i16; 3],
    },
    SampleCachedTargetAfterAcceptedHitSound,
    CapabilityFollowUp(PrimaryHitCapabilityEmission),
    Blocked(PrimaryHitBlock),
    Complete(PrimaryHitCompletion),
}

impl PrimaryHitStage {
    const fn phase(self) -> Option<PrimaryHitPhase> {
        match self {
            Self::CommitTickStamp => Some(PrimaryHitPhase::CommitTickStamp),
            Self::TypeImpactCallback(_) => Some(PrimaryHitPhase::TypeImpactCallback),
            Self::ImpactReaction => Some(PrimaryHitPhase::ImpactReaction),
            Self::ImpactReactionNetwork(_) => Some(PrimaryHitPhase::ImpactReactionNetwork),
            Self::CheckedDamageDelivery => Some(PrimaryHitPhase::CheckedDamageDelivery),
            Self::SampleCachedTargetAfterCheckedDamage => {
                Some(PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage)
            }
            Self::AcceptedHitSound { .. } => Some(PrimaryHitPhase::AcceptedHitSound),
            Self::SampleCachedTargetAfterAcceptedHitSound => {
                Some(PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound)
            }
            Self::CapabilityFollowUp(_) => Some(PrimaryHitPhase::CapabilityFollowUp),
            Self::Blocked(_) | Self::Complete(_) => None,
        }
    }
}

/// Receipt-bound detached `FUN_00410EB0` transaction.
///
/// Deliberately neither `Copy` nor `Clone`: duplicating a machine could replay
/// a committed tick stamp, callback, RNG draw, impulse, damage delivery, sound,
/// or particle request.
#[derive(Debug, PartialEq, Eq)]
pub struct PrimaryHitMachine {
    transaction_id: PrimaryHitTransactionId,
    request: PrimaryHitRequest,
    cached_entry: Option<PrimaryHitCachedEntry>,
    impact_sum_raw: i32,
    accepted_damage_raw: Option<NonZeroI32>,
    final_cached_target: Option<PrimaryHitCachedTargetSnapshot>,
    stage: PrimaryHitStage,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
}

impl PrimaryHitMachine {
    pub fn start(transaction_id: PrimaryHitTransactionId, request: PrimaryHitRequest) -> Self {
        let impact_sum_raw = request.delivery.packet.impact_sum_raw();
        let (cached_entry, stage) = match request.cached_entry {
            RetailRuntimeValue::Unresolved => (
                None,
                PrimaryHitStage::Blocked(PrimaryHitBlock::UnresolvedInitialCachedEntry),
            ),
            RetailRuntimeValue::Known(None) => (
                None,
                PrimaryHitStage::Blocked(PrimaryHitBlock::MissingInitialCachedTarget),
            ),
            RetailRuntimeValue::Known(Some(entry)) => {
                (Some(entry), PrimaryHitStage::CommitTickStamp)
            }
        };
        Self {
            transaction_id,
            request,
            cached_entry,
            impact_sum_raw,
            accepted_damage_raw: None,
            final_cached_target: None,
            stage,
            next_action_sequence: 1,
            issued_action_sequence: None,
        }
    }

    pub const fn impact_sum_raw(&self) -> i32 {
        self.impact_sum_raw
    }

    pub(crate) const fn transaction_id(&self) -> PrimaryHitTransactionId {
        self.transaction_id
    }

    pub(crate) const fn outstanding_action_sequence(&self) -> Option<u64> {
        self.issued_action_sequence
    }

    pub(crate) const fn active_phase(&self) -> Option<PrimaryHitPhase> {
        self.stage.phase()
    }

    /// Issues the current action exactly once for this machine.
    ///
    /// The returned receipt is the adapter's idempotency key. Until that exact
    /// receipt is resumed, later polls return [`PrimaryHitPoll::Awaiting`].
    pub fn poll(&mut self) -> PrimaryHitPoll {
        if self.issued_action_sequence.is_some() {
            return PrimaryHitPoll::Awaiting(
                self.stage
                    .phase()
                    .expect("an issued action always has an active phase"),
            );
        }
        let action = match self.stage {
            PrimaryHitStage::CommitTickStamp => {
                let entry = self.cached_entry.expect("entry exists for active stage");
                PrimaryHitAction::CommitTickStamp {
                    phase: PrimaryHitPhase::CommitTickStamp,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: entry.target_allocation_identity,
                    retail_tick: self.request.retail_tick,
                }
            }
            PrimaryHitStage::TypeImpactCallback(callback_address) => {
                PrimaryHitAction::InvokeTypeImpactCallback {
                    phase: PrimaryHitPhase::TypeImpactCallback,
                    callback_address,
                    target_handle: self.request.target_handle,
                    impact_sum_raw: self.impact_sum_raw,
                    source_provenance: self.request.source_provenance,
                    direction_q15: self.request.direction_q15,
                }
            }
            PrimaryHitStage::ImpactReaction => PrimaryHitAction::ApplyImpactReaction {
                phase: PrimaryHitPhase::ImpactReaction,
                target_handle: self.request.target_handle,
                impact_sum_raw: self.impact_sum_raw,
                direction_q15: self.request.direction_q15,
            },
            PrimaryHitStage::ImpactReactionNetwork(request) => {
                PrimaryHitAction::SubmitImpactReactionNetwork {
                    phase: PrimaryHitPhase::ImpactReactionNetwork,
                    request,
                }
            }
            PrimaryHitStage::CheckedDamageDelivery => PrimaryHitAction::DeliverCheckedDamage {
                phase: PrimaryHitPhase::CheckedDamageDelivery,
                target_handle: self.request.target_handle,
                delivery: self.request.delivery,
                ratio_numerator: 0,
                ratio_denominator: 0,
            },
            PrimaryHitStage::SampleCachedTargetAfterCheckedDamage => self
                .cached_target_sample_action(PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage),
            PrimaryHitStage::AcceptedHitSound {
                sound_id,
                position_raw,
            } => PrimaryHitAction::PlayAcceptedHitSound {
                phase: PrimaryHitPhase::AcceptedHitSound,
                sound_id,
                position_raw,
            },
            PrimaryHitStage::SampleCachedTargetAfterAcceptedHitSound => self
                .cached_target_sample_action(
                    PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
                ),
            PrimaryHitStage::CapabilityFollowUp(emission) => {
                PrimaryHitAction::EmitCapabilityFollowUp {
                    phase: PrimaryHitPhase::CapabilityFollowUp,
                    emission,
                }
            }
            PrimaryHitStage::Blocked(block) => return PrimaryHitPoll::Blocked(block),
            PrimaryHitStage::Complete(completion) => return PrimaryHitPoll::Complete(completion),
        };
        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one transaction cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        PrimaryHitPoll::Action(IssuedPrimaryHitAction {
            receipt: PrimaryHitReceipt {
                transaction_id: self.transaction_id,
                action_sequence,
            },
            action,
        })
    }

    /// Completes one outstanding boundary.
    ///
    /// Protocol errors do not consume the linear receipt: it is returned in
    /// [`PrimaryHitResumeFailure`] so the adapter can recover without replaying
    /// the already executed action. `Ok(())` consumes the receipt, including
    /// when [`PrimaryHitResume::Blocked`] durably stops the transaction.
    pub fn resume(
        &mut self,
        receipt: PrimaryHitReceipt,
        completion: PrimaryHitResume,
    ) -> Result<(), PrimaryHitResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            let error = PrimaryHitProtocolError::NoOutstandingAction {
                transaction_id: receipt.transaction_id,
                action_sequence: receipt.action_sequence,
            };
            return Err(PrimaryHitResumeFailure { receipt, error });
        };
        if receipt.transaction_id != self.transaction_id {
            let error = PrimaryHitProtocolError::ReceiptTransactionMismatch {
                expected: self.transaction_id,
                actual: receipt.transaction_id,
            };
            return Err(PrimaryHitResumeFailure { receipt, error });
        }
        if receipt.action_sequence != expected_sequence {
            let error = PrimaryHitProtocolError::ReceiptSequenceMismatch {
                expected: expected_sequence,
                actual: receipt.action_sequence,
            };
            return Err(PrimaryHitResumeFailure { receipt, error });
        }
        let expected = self
            .stage
            .phase()
            .expect("an outstanding receipt always belongs to an active stage");
        let actual = completion.phase();
        if actual != expected {
            return Err(PrimaryHitResumeFailure {
                receipt,
                error: PrimaryHitProtocolError::PhaseMismatch { expected, actual },
            });
        }
        if let PrimaryHitResume::Blocked { reason, .. } = completion {
            self.stage = PrimaryHitStage::Blocked(PrimaryHitBlock::External {
                phase: expected,
                reason,
            });
            self.issued_action_sequence = None;
            return Ok(());
        }

        match (self.stage, completion) {
            (
                PrimaryHitStage::CommitTickStamp,
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::CommitTickStamp,
                },
            ) => self.enter_after_tick_stamp(),
            (
                PrimaryHitStage::TypeImpactCallback(_),
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::TypeImpactCallback,
                },
            ) => {
                self.stage = PrimaryHitStage::ImpactReaction;
            }
            (
                PrimaryHitStage::ImpactReaction,
                PrimaryHitResume::ImpactReactionResolved { result, .. },
            ) => self.enter_after_impact_reaction(result),
            (
                PrimaryHitStage::ImpactReactionNetwork(_),
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::ImpactReactionNetwork,
                },
            ) => {
                self.stage = PrimaryHitStage::CheckedDamageDelivery;
            }
            (
                PrimaryHitStage::CheckedDamageDelivery,
                PrimaryHitResume::CheckedDamageReturned { result, .. },
            ) => self.enter_after_checked_damage(result),
            (
                PrimaryHitStage::SampleCachedTargetAfterCheckedDamage,
                PrimaryHitResume::CachedOuterTargetSampled {
                    cached_outer_target,
                    ..
                },
            ) => {
                if self.accept_cached_target_snapshot(
                    PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage,
                    cached_outer_target,
                ) {
                    self.enter_accepted_hit_sound();
                }
            }
            (
                PrimaryHitStage::AcceptedHitSound { .. },
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::AcceptedHitSound,
                },
            ) => {
                self.stage = PrimaryHitStage::SampleCachedTargetAfterAcceptedHitSound;
            }
            (
                PrimaryHitStage::SampleCachedTargetAfterAcceptedHitSound,
                PrimaryHitResume::CachedOuterTargetSampled {
                    cached_outer_target,
                    ..
                },
            ) => {
                if self.accept_cached_target_snapshot(
                    PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
                    cached_outer_target,
                ) {
                    self.enter_capability_follow_up();
                }
            }
            (
                PrimaryHitStage::CapabilityFollowUp(_),
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::CapabilityFollowUp,
                },
            ) => self.complete_accepted(),
            _ => {
                return Err(PrimaryHitResumeFailure {
                    receipt,
                    error: PrimaryHitProtocolError::UnexpectedResumeKind { phase: expected },
                });
            }
        }
        self.issued_action_sequence = None;
        Ok(())
    }

    fn enter_after_tick_stamp(&mut self) {
        let entry = self
            .cached_entry
            .expect("tick action requires cached entry");
        match entry.type_callback_address {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    PrimaryHitStage::Blocked(PrimaryHitBlock::UnresolvedTypeImpactCallback);
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = PrimaryHitStage::ImpactReaction;
            }
            RetailRuntimeValue::Known(Some(address))
                if address == PRIMARY_HIT_TYPE_CALLBACK_ADDRESS =>
            {
                self.stage = PrimaryHitStage::TypeImpactCallback(address);
            }
            RetailRuntimeValue::Known(Some(address)) => {
                self.stage = PrimaryHitStage::Blocked(
                    PrimaryHitBlock::UnsupportedTypeImpactCallback(address),
                );
            }
        }
    }

    fn enter_after_impact_reaction(&mut self, result: PrimaryHitImpactReactionResult) {
        self.stage = match result {
            PrimaryHitImpactReactionResult::TargetMissing => {
                PrimaryHitStage::Blocked(PrimaryHitBlock::TargetMissingDuringImpactReaction)
            }
            PrimaryHitImpactReactionResult::Committed(ImpactReactionOutcome::Applied(applied))
                if applied.network_request.is_some() =>
            {
                PrimaryHitStage::ImpactReactionNetwork(
                    applied
                        .network_request
                        .expect("guarded applied network request"),
                )
            }
            PrimaryHitImpactReactionResult::Committed(_) => PrimaryHitStage::CheckedDamageDelivery,
        };
    }

    fn enter_after_checked_damage(&mut self, result: PrimaryHitCheckedDamageResult) {
        match result {
            PrimaryHitCheckedDamageResult::TargetMissing => {
                self.stage = PrimaryHitStage::Complete(
                    PrimaryHitCompletion::TargetMissingDuringCheckedDamage,
                );
            }
            PrimaryHitCheckedDamageResult::ReturnedZero => {
                self.stage =
                    PrimaryHitStage::Complete(PrimaryHitCompletion::CheckedDamageReturnedZero);
            }
            PrimaryHitCheckedDamageResult::ReturnedNonZero {
                accepted_damage_raw,
            } => {
                self.accepted_damage_raw = Some(accepted_damage_raw);
                self.stage = PrimaryHitStage::SampleCachedTargetAfterCheckedDamage;
            }
        }
    }

    fn cached_target_sample_action(&self, phase: PrimaryHitPhase) -> PrimaryHitAction {
        let entry = self
            .cached_entry
            .expect("cached-target sampling requires the entry-time identities");
        PrimaryHitAction::SampleCachedOuterTarget {
            phase,
            target_handle: self.request.target_handle,
            target_allocation_identity: entry.target_allocation_identity,
            cached_type_record_identity: entry.type_record_identity,
        }
    }

    fn accept_cached_target_snapshot(
        &mut self,
        phase: PrimaryHitPhase,
        cached_outer_target: RetailRuntimeValue<Option<PrimaryHitCachedTargetSnapshot>>,
    ) -> bool {
        let target = match cached_outer_target {
            RetailRuntimeValue::Unresolved => {
                self.stage = PrimaryHitStage::Blocked(match phase {
                    PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage => {
                        PrimaryHitBlock::UnresolvedCachedOuterTargetAfterAcceptedDamage
                    }
                    PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound => {
                        PrimaryHitBlock::UnresolvedCachedOuterTargetAfterAcceptedHitSound
                    }
                    _ => unreachable!("cached-target snapshots have dedicated phases"),
                });
                return false;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = PrimaryHitStage::Blocked(match phase {
                    PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage => {
                        PrimaryHitBlock::MissingCachedOuterTargetAfterAcceptedDamage
                    }
                    PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound => {
                        PrimaryHitBlock::MissingCachedOuterTargetAfterAcceptedHitSound
                    }
                    _ => unreachable!("cached-target snapshots have dedicated phases"),
                });
                return false;
            }
            RetailRuntimeValue::Known(Some(target)) => target,
        };
        let entry = self.cached_entry.expect("accepted path has cached entry");
        if target.target_allocation_identity != entry.target_allocation_identity {
            self.stage =
                PrimaryHitStage::Blocked(PrimaryHitBlock::CachedTargetAllocationIdentityMismatch {
                    expected: entry.target_allocation_identity,
                    actual: target.target_allocation_identity,
                });
            return false;
        }
        if target.cached_type_record_identity != entry.type_record_identity {
            self.stage =
                PrimaryHitStage::Blocked(PrimaryHitBlock::CachedTypeRecordIdentityMismatch {
                    expected: entry.type_record_identity,
                    actual: target.cached_type_record_identity,
                });
            return false;
        }
        self.final_cached_target = Some(target);
        true
    }

    fn enter_accepted_hit_sound(&mut self) {
        let target = self
            .final_cached_target
            .expect("accepted suffix requires cached target");
        if target.state_flags_at_0x08 & DYING_STATE_BIT != 0 {
            self.enter_capability_follow_up();
            return;
        }
        match target.accepted_hit_sound_id {
            RetailRuntimeValue::Unresolved => {
                self.stage = PrimaryHitStage::Blocked(PrimaryHitBlock::UnresolvedAcceptedHitSound);
            }
            RetailRuntimeValue::Known(None) => self.enter_capability_follow_up(),
            RetailRuntimeValue::Known(Some(sound_id)) => {
                self.stage = PrimaryHitStage::AcceptedHitSound {
                    sound_id,
                    position_raw: target.position_raw,
                };
            }
        }
    }

    fn enter_capability_follow_up(&mut self) {
        let target = self
            .final_cached_target
            .expect("capability suffix requires cached target");
        if target.capability_flags_at_0x64 & PRIMARY_HIT_CAPABILITY_EFFECT_BIT == 0 {
            self.complete_accepted();
            return;
        }
        let slot = active_model_slot_from_state_flags(target.state_flags_at_0x08);
        let model = match target.model_by_state_slot[slot] {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    PrimaryHitStage::Blocked(PrimaryHitBlock::UnresolvedSelectedModel { slot });
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    PrimaryHitStage::Blocked(PrimaryHitBlock::MissingSelectedModel { slot });
                return;
            }
            RetailRuntimeValue::Known(Some(model)) => model,
        };
        let entry = self
            .cached_entry
            .expect("capability suffix has cached entry");
        let mut position_raw = target.position_raw;
        position_raw[2] = position_raw[2].wrapping_sub(model.base_z_raw);
        self.stage = PrimaryHitStage::CapabilityFollowUp(PrimaryHitCapabilityEmission {
            target_handle: self.request.target_handle,
            target_allocation_identity: entry.target_allocation_identity,
            selected_model_slot: slot,
            selected_global_model_id: model.global_model_id,
            position_raw,
            particle_class: PRIMARY_HIT_CAPABILITY_PARTICLE_CLASS,
            particle_scale_raw: PRIMARY_HIT_CAPABILITY_PARTICLE_SCALE_RAW,
            owner_sign: target.state_flags_at_0x08 >> 31,
        });
    }

    fn complete_accepted(&mut self) {
        self.stage = PrimaryHitStage::Complete(PrimaryHitCompletion::Accepted(
            self.accepted_damage_raw
                .expect("accepted suffix retains nonzero helper return"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checked_damage::{CheckedDamageCompletion, CheckedDamageZeroReason};
    use crate::damage::{
        checked_local_damage_transition_after_null_modifier, CheckedLocalDamageState,
        CheckedLocalDamageTransition, DamageDeliveryRecord, DamagePacket, DamageProfile,
        GenericEntityDamageState,
    };
    use crate::impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, ImpactReactionSuppression,
        IMPACT_REACTION_ENABLED_STATE_BIT, IMPACT_REACTION_NETWORKED_STATE_BIT,
        IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    };

    const TARGET_HANDLE: u32 = 0x04FC_0001;
    const TARGET_IDENTITY: u64 = 0x1234_0000_04FC_0001;
    const TYPE_IDENTITY: u64 = 0x5678_0000_0000_002E;
    const TRANSACTION_ID_RAW: u64 = 0xA110_0000_0000_0001;

    fn transaction_id(raw: u64) -> PrimaryHitTransactionId {
        PrimaryHitTransactionId::new(raw).expect("test transaction IDs are nonzero")
    }

    fn cached_entry(
        type_callback_address: RetailRuntimeValue<Option<u32>>,
    ) -> PrimaryHitCachedEntry {
        PrimaryHitCachedEntry {
            target_allocation_identity: TARGET_IDENTITY,
            type_record_identity: TYPE_IDENTITY,
            type_callback_address,
        }
    }

    fn request(entry: PrimaryHitCachedEntry) -> PrimaryHitRequest {
        PrimaryHitRequest {
            target_handle: TARGET_HANDLE,
            delivery: DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 2],
                    amounts_raw: [400, 600],
                },
                source_entity_type_raw: 0x0000_002F,
                owner_handle: 0x04AA_0001,
            },
            source_provenance: 0xCAFE_BABE,
            direction_q15: [0x4000, -0x2000, 0x1000],
            retail_tick: 0xDEAD_BEEF,
            cached_entry: RetailRuntimeValue::Known(Some(entry)),
        }
    }

    fn model(
        global_model_id: u16,
        base_z_raw: i16,
    ) -> RetailRuntimeValue<Option<PrimaryHitModelBinding>> {
        RetailRuntimeValue::Known(Some(PrimaryHitModelBinding {
            global_model_id,
            base_z_raw,
        }))
    }

    fn final_target(
        state_flags_at_0x08: u32,
        capability_flags_at_0x64: u8,
        sound_id: RetailRuntimeValue<Option<u16>>,
    ) -> PrimaryHitCachedTargetSnapshot {
        PrimaryHitCachedTargetSnapshot {
            target_allocation_identity: TARGET_IDENTITY,
            cached_type_record_identity: TYPE_IDENTITY,
            state_flags_at_0x08,
            capability_flags_at_0x64,
            position_raw: [100, -200, 1_000],
            model_by_state_slot: [model(10, 10), model(11, 20), model(12, 30), model(13, 40)],
            accepted_hit_sound_id: sound_id,
        }
    }

    fn start(entry: PrimaryHitCachedEntry) -> PrimaryHitMachine {
        PrimaryHitMachine::start(transaction_id(TRANSACTION_ID_RAW), request(entry))
    }

    #[test]
    fn checked_damage_completion_bridge_preserves_outer_return_semantics() {
        assert_eq!(
            PrimaryHitCheckedDamageResult::from(CheckedDamageCompletion::TargetMissing),
            PrimaryHitCheckedDamageResult::TargetMissing,
        );
        for reason in [
            CheckedDamageZeroReason::AdmissionDisabled,
            CheckedDamageZeroReason::NullDelivery,
            CheckedDamageZeroReason::FilteredOut,
            CheckedDamageZeroReason::ModifierReturnedZero,
        ] {
            assert_eq!(
                PrimaryHitCheckedDamageResult::from(CheckedDamageCompletion::ReturnedZero(reason)),
                PrimaryHitCheckedDamageResult::ReturnedZero,
            );
        }

        let high_bit = NonZeroI32::new(i32::MIN).unwrap();
        assert_eq!(
            PrimaryHitCheckedDamageResult::from(CheckedDamageCompletion::ReturnedNonZero(high_bit)),
            PrimaryHitCheckedDamageResult::ReturnedNonZero {
                accepted_damage_raw: high_bit,
            },
        );
    }

    fn issue(machine: &mut PrimaryHitMachine) -> IssuedPrimaryHitAction {
        match machine.poll() {
            PrimaryHitPoll::Action(issued) => issued,
            other => panic!("expected issued action, got {other:?}"),
        }
    }

    fn expect_action(
        machine: &mut PrimaryHitMachine,
        expected: PrimaryHitAction,
    ) -> PrimaryHitReceipt {
        let IssuedPrimaryHitAction { receipt, action } = issue(machine);
        assert_eq!(action, expected);
        receipt
    }

    fn acknowledge(
        machine: &mut PrimaryHitMachine,
        receipt: PrimaryHitReceipt,
        phase: PrimaryHitPhase,
    ) {
        machine
            .resume(receipt, PrimaryHitResume::Acknowledged { phase })
            .unwrap();
    }

    fn advance_to_reaction(machine: &mut PrimaryHitMachine) -> PrimaryHitReceipt {
        let tick_receipt = expect_action(
            machine,
            PrimaryHitAction::CommitTickStamp {
                phase: PrimaryHitPhase::CommitTickStamp,
                target_handle: TARGET_HANDLE,
                target_allocation_identity: TARGET_IDENTITY,
                retail_tick: 0xDEAD_BEEF,
            },
        );
        acknowledge(machine, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        let IssuedPrimaryHitAction { receipt, action } = issue(machine);
        match action {
            PrimaryHitAction::InvokeTypeImpactCallback { .. } => {
                acknowledge(machine, receipt, PrimaryHitPhase::TypeImpactCallback);
                let IssuedPrimaryHitAction { receipt, action } = issue(machine);
                assert!(matches!(
                    action,
                    PrimaryHitAction::ApplyImpactReaction {
                        phase: PrimaryHitPhase::ImpactReaction,
                        impact_sum_raw: 1_000,
                        ..
                    }
                ));
                receipt
            }
            PrimaryHitAction::ApplyImpactReaction {
                phase: PrimaryHitPhase::ImpactReaction,
                impact_sum_raw: 1_000,
                ..
            } => receipt,
            other => panic!("unexpected post-tick action: {other:?}"),
        }
    }

    fn advance_to_delivery(machine: &mut PrimaryHitMachine) -> PrimaryHitReceipt {
        let reaction_receipt = advance_to_reaction(machine);
        machine
            .resume(
                reaction_receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::Committed(
                        ImpactReactionOutcome::Suppressed(
                            ImpactReactionSuppression::SuppressionBitSet,
                        ),
                    ),
                },
            )
            .unwrap();
        expect_action(
            machine,
            PrimaryHitAction::DeliverCheckedDamage {
                phase: PrimaryHitPhase::CheckedDamageDelivery,
                target_handle: TARGET_HANDLE,
                delivery: request(cached_entry(RetailRuntimeValue::Known(None))).delivery,
                ratio_numerator: 0,
                ratio_denominator: 0,
            },
        )
    }

    fn return_accepted_damage(
        machine: &mut PrimaryHitMachine,
        receipt: PrimaryHitReceipt,
        amount: i32,
    ) {
        machine
            .resume(
                receipt,
                PrimaryHitResume::CheckedDamageReturned {
                    phase: PrimaryHitPhase::CheckedDamageDelivery,
                    result: PrimaryHitCheckedDamageResult::ReturnedNonZero {
                        accepted_damage_raw: NonZeroI32::new(amount).unwrap(),
                    },
                },
            )
            .unwrap();
    }

    fn expect_cached_sample_action(
        machine: &mut PrimaryHitMachine,
        phase: PrimaryHitPhase,
    ) -> PrimaryHitReceipt {
        expect_action(
            machine,
            PrimaryHitAction::SampleCachedOuterTarget {
                phase,
                target_handle: TARGET_HANDLE,
                target_allocation_identity: TARGET_IDENTITY,
                cached_type_record_identity: TYPE_IDENTITY,
            },
        )
    }

    fn submit_cached_sample(
        machine: &mut PrimaryHitMachine,
        receipt: PrimaryHitReceipt,
        phase: PrimaryHitPhase,
        cached_outer_target: RetailRuntimeValue<Option<PrimaryHitCachedTargetSnapshot>>,
    ) {
        machine
            .resume(
                receipt,
                PrimaryHitResume::CachedOuterTargetSampled {
                    phase,
                    cached_outer_target,
                },
            )
            .unwrap();
    }

    fn accepted(
        machine: &mut PrimaryHitMachine,
        delivery_receipt: PrimaryHitReceipt,
        amount: i32,
        target: PrimaryHitCachedTargetSnapshot,
    ) {
        return_accepted_damage(machine, delivery_receipt, amount);
        let sample_receipt = expect_cached_sample_action(
            machine,
            PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage,
        );
        submit_cached_sample(
            machine,
            sample_receipt,
            PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage,
            RetailRuntimeValue::Known(Some(target)),
        );
    }

    #[test]
    fn tick_stamp_precedes_callback_and_is_retained_when_callback_is_unresolved() {
        let entry = cached_entry(RetailRuntimeValue::Unresolved);
        let mut machine = start(entry);
        let tick_receipt = expect_action(
            &mut machine,
            PrimaryHitAction::CommitTickStamp {
                phase: PrimaryHitPhase::CommitTickStamp,
                target_handle: TARGET_HANDLE,
                target_allocation_identity: TARGET_IDENTITY,
                retail_tick: 0xDEAD_BEEF,
            },
        );
        acknowledge(&mut machine, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(PrimaryHitBlock::UnresolvedTypeImpactCallback)
        );
    }

    #[test]
    fn unaudited_type_callback_is_blocked_after_the_unconditional_tick_stamp() {
        const UNSUPPORTED_CALLBACK: u32 = 0x0040_D860;
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(Some(
            UNSUPPORTED_CALLBACK,
        ))));
        let IssuedPrimaryHitAction {
            receipt: tick_receipt,
            action,
        } = issue(&mut machine);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        acknowledge(&mut machine, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(PrimaryHitBlock::UnsupportedTypeImpactCallback(
                UNSUPPORTED_CALLBACK
            ))
        );
    }

    #[test]
    fn null_type_slot_skips_callback_while_common_wrapper_keeps_order() {
        let mut absent = start(cached_entry(RetailRuntimeValue::Known(None)));
        let IssuedPrimaryHitAction {
            receipt: tick_receipt,
            action,
        } = issue(&mut absent);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        acknowledge(&mut absent, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        assert!(matches!(
            issue(&mut absent).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));

        let mut present = start(cached_entry(RetailRuntimeValue::Known(Some(
            PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
        ))));
        let IssuedPrimaryHitAction {
            receipt: tick_receipt,
            action,
        } = issue(&mut present);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        acknowledge(&mut present, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        let callback_receipt = expect_action(
            &mut present,
            PrimaryHitAction::InvokeTypeImpactCallback {
                phase: PrimaryHitPhase::TypeImpactCallback,
                callback_address: PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
                target_handle: TARGET_HANDLE,
                impact_sum_raw: 1_000,
                source_provenance: 0xCAFE_BABE,
                direction_q15: [0x4000, -0x2000, 0x1000],
            },
        );
        acknowledge(
            &mut present,
            callback_receipt,
            PrimaryHitPhase::TypeImpactCallback,
        );
        assert!(matches!(
            issue(&mut present).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
    }

    #[test]
    fn callback_removal_exposes_fresh_reaction_lookup_retail_fault_without_rebinding() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(Some(
            PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
        ))));
        let reaction_receipt = advance_to_reaction(&mut machine);
        machine
            .resume(
                reaction_receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::TargetMissing,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(PrimaryHitBlock::TargetMissingDuringImpactReaction)
        );
    }

    #[test]
    fn suppressed_reaction_consumes_no_rng_and_advances_to_checked_delivery() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let reaction_receipt = advance_to_reaction(&mut machine);
        let mut body = ImpactReactionBody {
            state_flags_at_0x08: IMPACT_REACTION_ENABLED_STATE_BIT
                | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
            mass_raw_at_0xb0: 0,
            linear_velocity_xyz_raw: [1, 2, 3],
            angular_heading_pitch_roll_raw: [4, 5, 6],
        };
        let before = body;
        let mut draws = 0;
        let outcome = apply_impact_reaction(
            &mut body,
            TARGET_HANDLE,
            machine.impact_sum_raw(),
            request(cached_entry(RetailRuntimeValue::Known(None))).direction_q15,
            || {
                draws += 1;
                0
            },
        )
        .unwrap();
        assert_eq!(draws, 0);
        assert_eq!(body, before);
        machine
            .resume(
                reaction_receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::Committed(outcome),
                },
            )
            .unwrap();
        assert!(matches!(
            issue(&mut machine).action,
            PrimaryHitAction::DeliverCheckedDamage { .. }
        ));
    }

    #[test]
    fn checked_delivery_preserves_both_non_arithmetic_provenance_words() {
        let expected = request(cached_entry(RetailRuntimeValue::Known(None))).delivery;
        assert_eq!(
            expected.raw_dwords(),
            [1, 2, 400, 600, 0x0000_002F, 0x04AA_0001]
        );
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        advance_to_delivery(&mut machine);
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CheckedDamageDelivery)
        );
    }

    #[test]
    fn eligible_reaction_draws_exactly_three_and_stages_network_after_body_commit() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let reaction_receipt = advance_to_reaction(&mut machine);
        let mut body = ImpactReactionBody {
            state_flags_at_0x08: IMPACT_REACTION_ENABLED_STATE_BIT
                | IMPACT_REACTION_NETWORKED_STATE_BIT,
            mass_raw_at_0xb0: 0x40,
            linear_velocity_xyz_raw: [10, 20, 30],
            angular_heading_pitch_roll_raw: [40, 50, 60],
        };
        let before = body;
        let mut draws = [0x1000_u32, 0x2000, 0x3000].into_iter();
        let outcome = apply_impact_reaction(
            &mut body,
            TARGET_HANDLE,
            machine.impact_sum_raw(),
            request(cached_entry(RetailRuntimeValue::Known(None))).direction_q15,
            || draws.next().expect("exactly three RNG words"),
        )
        .unwrap();
        assert_ne!(
            body, before,
            "six reaction words commit before network staging"
        );
        assert_eq!(draws.next(), None);
        let ImpactReactionOutcome::Applied(applied) = outcome else {
            panic!("enabled reaction must apply");
        };
        let request = applied.network_request.unwrap();
        machine
            .resume(
                reaction_receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::Committed(outcome),
                },
            )
            .unwrap();
        let network_receipt = expect_action(
            &mut machine,
            PrimaryHitAction::SubmitImpactReactionNetwork {
                phase: PrimaryHitPhase::ImpactReactionNetwork,
                request,
            },
        );
        acknowledge(
            &mut machine,
            network_receipt,
            PrimaryHitPhase::ImpactReactionNetwork,
        );
        assert!(matches!(
            issue(&mut machine).action,
            PrimaryHitAction::DeliverCheckedDamage { .. }
        ));
    }

    #[test]
    fn zero_checked_return_suppresses_all_final_feedback() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let delivery_receipt = advance_to_delivery(&mut machine);
        machine
            .resume(
                delivery_receipt,
                PrimaryHitResume::CheckedDamageReturned {
                    phase: PrimaryHitPhase::CheckedDamageDelivery,
                    result: PrimaryHitCheckedDamageResult::ReturnedZero,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Complete(PrimaryHitCompletion::CheckedDamageReturnedZero)
        );
    }

    #[test]
    fn accepted_alive_hit_plays_final_cached_sound_then_capability_effect() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let delivery_receipt = advance_to_delivery(&mut machine);
        accepted(
            &mut machine,
            delivery_receipt,
            -77,
            final_target(
                0x8000_2000,
                PRIMARY_HIT_CAPABILITY_EFFECT_BIT,
                RetailRuntimeValue::Known(Some(19)),
            ),
        );
        let sound_receipt = expect_action(
            &mut machine,
            PrimaryHitAction::PlayAcceptedHitSound {
                phase: PrimaryHitPhase::AcceptedHitSound,
                sound_id: 19,
                position_raw: [100, -200, 1_000],
            },
        );
        acknowledge(
            &mut machine,
            sound_receipt,
            PrimaryHitPhase::AcceptedHitSound,
        );
        let sample_receipt = expect_cached_sample_action(
            &mut machine,
            PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
        );
        let mut refreshed = final_target(
            0x8000_4000,
            PRIMARY_HIT_CAPABILITY_EFFECT_BIT,
            RetailRuntimeValue::Known(Some(99)),
        );
        refreshed.position_raw = [300, -400, 2_000];
        submit_cached_sample(
            &mut machine,
            sample_receipt,
            PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
            RetailRuntimeValue::Known(Some(refreshed)),
        );
        let capability_receipt = expect_action(
            &mut machine,
            PrimaryHitAction::EmitCapabilityFollowUp {
                phase: PrimaryHitPhase::CapabilityFollowUp,
                emission: PrimaryHitCapabilityEmission {
                    target_handle: TARGET_HANDLE,
                    target_allocation_identity: TARGET_IDENTITY,
                    selected_model_slot: 1,
                    selected_global_model_id: 11,
                    position_raw: [300, -400, 1_980],
                    particle_class: 5,
                    particle_scale_raw: 0x800,
                    owner_sign: 1,
                },
            },
        );
        acknowledge(
            &mut machine,
            capability_receipt,
            PrimaryHitPhase::CapabilityFollowUp,
        );
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Complete(PrimaryHitCompletion::Accepted(
                NonZeroI32::new(-77).unwrap()
            ))
        );
    }

    #[test]
    fn accepted_suffix_requires_a_distinct_post_damage_cached_pointer_sample() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let delivery_receipt = advance_to_delivery(&mut machine);
        return_accepted_damage(&mut machine, delivery_receipt, 10);
        let sample_receipt = expect_cached_sample_action(
            &mut machine,
            PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage,
        );
        submit_cached_sample(
            &mut machine,
            sample_receipt,
            PrimaryHitPhase::SampleCachedTargetAfterCheckedDamage,
            RetailRuntimeValue::Unresolved,
        );
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(
                PrimaryHitBlock::UnresolvedCachedOuterTargetAfterAcceptedDamage
            )
        );
    }

    #[test]
    fn accepted_sound_requires_a_second_cached_pointer_sample() {
        for (sample, expected) in [
            (
                RetailRuntimeValue::Unresolved,
                PrimaryHitBlock::UnresolvedCachedOuterTargetAfterAcceptedHitSound,
            ),
            (
                RetailRuntimeValue::Known(None),
                PrimaryHitBlock::MissingCachedOuterTargetAfterAcceptedHitSound,
            ),
        ] {
            let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
            let delivery_receipt = advance_to_delivery(&mut machine);
            accepted(
                &mut machine,
                delivery_receipt,
                10,
                final_target(0, 0, RetailRuntimeValue::Known(Some(19))),
            );
            let IssuedPrimaryHitAction {
                receipt: sound_receipt,
                action,
            } = issue(&mut machine);
            assert!(matches!(
                action,
                PrimaryHitAction::PlayAcceptedHitSound { .. }
            ));
            acknowledge(
                &mut machine,
                sound_receipt,
                PrimaryHitPhase::AcceptedHitSound,
            );
            let sample_receipt = expect_cached_sample_action(
                &mut machine,
                PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
            );
            submit_cached_sample(
                &mut machine,
                sample_receipt,
                PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
                sample,
            );
            assert_eq!(machine.poll(), PrimaryHitPoll::Blocked(expected));
        }
    }

    #[test]
    fn post_sound_sample_rejects_cached_type_identity_replacement() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let delivery_receipt = advance_to_delivery(&mut machine);
        accepted(
            &mut machine,
            delivery_receipt,
            10,
            final_target(0, 0, RetailRuntimeValue::Known(Some(19))),
        );
        let IssuedPrimaryHitAction {
            receipt: sound_receipt,
            action,
        } = issue(&mut machine);
        assert!(matches!(
            action,
            PrimaryHitAction::PlayAcceptedHitSound { .. }
        ));
        acknowledge(
            &mut machine,
            sound_receipt,
            PrimaryHitPhase::AcceptedHitSound,
        );
        let sample_receipt = expect_cached_sample_action(
            &mut machine,
            PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
        );
        let mut replaced = final_target(0, 0, RetailRuntimeValue::Known(None));
        replaced.cached_type_record_identity = TYPE_IDENTITY + 1;
        submit_cached_sample(
            &mut machine,
            sample_receipt,
            PrimaryHitPhase::SampleCachedTargetAfterAcceptedHitSound,
            RetailRuntimeValue::Known(Some(replaced)),
        );
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(PrimaryHitBlock::CachedTypeRecordIdentityMismatch {
                expected: TYPE_IDENTITY,
                actual: TYPE_IDENTITY + 1,
            })
        );
    }

    #[test]
    fn dying_hit_suppresses_sound_but_not_independent_capability_effect() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let delivery_receipt = advance_to_delivery(&mut machine);
        accepted(
            &mut machine,
            delivery_receipt,
            1,
            final_target(
                DYING_STATE_BIT,
                PRIMARY_HIT_CAPABILITY_EFFECT_BIT,
                RetailRuntimeValue::Unresolved,
            ),
        );
        let _capability_receipt = expect_action(
            &mut machine,
            PrimaryHitAction::EmitCapabilityFollowUp {
                phase: PrimaryHitPhase::CapabilityFollowUp,
                emission: PrimaryHitCapabilityEmission {
                    target_handle: TARGET_HANDLE,
                    target_allocation_identity: TARGET_IDENTITY,
                    selected_model_slot: 1,
                    selected_global_model_id: 11,
                    position_raw: [100, -200, 980],
                    particle_class: 5,
                    particle_scale_raw: 0x800,
                    owner_sign: 0,
                },
            },
        );
    }

    #[test]
    fn phase_mismatch_never_advances_pending_action() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let IssuedPrimaryHitAction { receipt, action } = issue(&mut machine);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        let receipt_transaction_id = receipt.transaction_id();
        let receipt_sequence = receipt.action_sequence();
        let failure = machine
            .resume(
                receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::TargetMissing,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitProtocolError::PhaseMismatch {
                expected: PrimaryHitPhase::CommitTickStamp,
                actual: PrimaryHitPhase::ImpactReaction,
            }
        );
        assert_eq!(failure.receipt.transaction_id(), receipt_transaction_id);
        assert_eq!(failure.receipt.action_sequence(), receipt_sequence);
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CommitTickStamp)
        );
        acknowledge(
            &mut machine,
            failure.receipt,
            PrimaryHitPhase::CommitTickStamp,
        );
        assert!(matches!(
            issue(&mut machine).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
    }

    #[test]
    fn wrong_resume_kind_returns_receipt_for_corrected_retry() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let IssuedPrimaryHitAction { receipt, action } = issue(&mut machine);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        let failure = machine
            .resume(
                receipt,
                PrimaryHitResume::CheckedDamageReturned {
                    phase: PrimaryHitPhase::CommitTickStamp,
                    result: PrimaryHitCheckedDamageResult::ReturnedZero,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitProtocolError::UnexpectedResumeKind {
                phase: PrimaryHitPhase::CommitTickStamp,
            }
        );
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CommitTickStamp)
        );
        acknowledge(
            &mut machine,
            failure.receipt,
            PrimaryHitPhase::CommitTickStamp,
        );
        assert!(matches!(
            issue(&mut machine).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
    }

    #[test]
    fn no_outstanding_action_returns_the_submitted_receipt() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let IssuedPrimaryHitAction { receipt, action } = issue(&mut machine);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        let replay_receipt = PrimaryHitReceipt {
            transaction_id: receipt.transaction_id(),
            action_sequence: receipt.action_sequence(),
        };
        acknowledge(&mut machine, receipt, PrimaryHitPhase::CommitTickStamp);

        let failure = machine
            .resume(
                replay_receipt,
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::CommitTickStamp,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitProtocolError::NoOutstandingAction {
                transaction_id: transaction_id(TRANSACTION_ID_RAW),
                action_sequence: 1,
            }
        );
        assert_eq!(
            failure.receipt.transaction_id(),
            transaction_id(TRANSACTION_ID_RAW)
        );
        assert_eq!(failure.receipt.action_sequence(), 1);
        assert!(matches!(
            issue(&mut machine).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
    }

    #[test]
    fn cross_machine_receipt_is_rejected_even_at_the_same_phase() {
        let first_id = transaction_id(TRANSACTION_ID_RAW);
        let second_id = transaction_id(TRANSACTION_ID_RAW + 1);
        let entry = cached_entry(RetailRuntimeValue::Known(None));
        let mut first = PrimaryHitMachine::start(first_id, request(entry));
        let mut second = PrimaryHitMachine::start(second_id, request(entry));
        let IssuedPrimaryHitAction {
            receipt: first_receipt,
            action: first_action,
        } = issue(&mut first);
        let IssuedPrimaryHitAction {
            receipt: second_receipt,
            action: second_action,
        } = issue(&mut second);
        assert_eq!(first_action.phase(), PrimaryHitPhase::CommitTickStamp);
        assert_eq!(second_action.phase(), PrimaryHitPhase::CommitTickStamp);
        let failure = second
            .resume(
                first_receipt,
                PrimaryHitResume::Acknowledged {
                    phase: PrimaryHitPhase::CommitTickStamp,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitProtocolError::ReceiptTransactionMismatch {
                expected: second_id,
                actual: first_id,
            }
        );
        assert_eq!(
            second.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CommitTickStamp)
        );
        acknowledge(
            &mut first,
            failure.receipt,
            PrimaryHitPhase::CommitTickStamp,
        );
        assert!(matches!(
            issue(&mut first).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
        acknowledge(
            &mut second,
            second_receipt,
            PrimaryHitPhase::CommitTickStamp,
        );
        assert!(matches!(
            issue(&mut second).action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
    }

    #[test]
    fn sequence_mismatch_returns_the_rejected_receipt_identity() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let IssuedPrimaryHitAction {
            receipt: tick_receipt,
            action,
        } = issue(&mut machine);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        let stale_receipt = PrimaryHitReceipt {
            transaction_id: tick_receipt.transaction_id(),
            action_sequence: tick_receipt.action_sequence(),
        };
        acknowledge(&mut machine, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        let IssuedPrimaryHitAction {
            receipt: reaction_receipt,
            action,
        } = issue(&mut machine);
        assert!(matches!(
            action,
            PrimaryHitAction::ApplyImpactReaction { .. }
        ));
        let failure = machine
            .resume(
                stale_receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::TargetMissing,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            PrimaryHitProtocolError::ReceiptSequenceMismatch {
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(
            failure.receipt.transaction_id(),
            transaction_id(TRANSACTION_ID_RAW)
        );
        assert_eq!(failure.receipt.action_sequence(), 1);
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::ImpactReaction)
        );
        machine
            .resume(
                reaction_receipt,
                PrimaryHitResume::ImpactReactionResolved {
                    phase: PrimaryHitPhase::ImpactReaction,
                    result: PrimaryHitImpactReactionResult::TargetMissing,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(PrimaryHitBlock::TargetMissingDuringImpactReaction)
        );
    }

    #[test]
    fn repeated_poll_cannot_reissue_an_unacknowledged_action() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        assert!(matches!(
            issue(&mut machine).action,
            PrimaryHitAction::CommitTickStamp { .. }
        ));
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CommitTickStamp)
        );
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Awaiting(PrimaryHitPhase::CommitTickStamp)
        );
    }

    #[test]
    fn adapter_failure_is_durable_and_callback_cannot_replay() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(Some(
            PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
        ))));
        let IssuedPrimaryHitAction {
            receipt: tick_receipt,
            action,
        } = issue(&mut machine);
        assert!(matches!(action, PrimaryHitAction::CommitTickStamp { .. }));
        acknowledge(&mut machine, tick_receipt, PrimaryHitPhase::CommitTickStamp);
        let IssuedPrimaryHitAction {
            receipt: callback_receipt,
            action,
        } = issue(&mut machine);
        assert!(matches!(
            action,
            PrimaryHitAction::InvokeTypeImpactCallback { .. }
        ));
        machine
            .resume(
                callback_receipt,
                PrimaryHitResume::Blocked {
                    phase: PrimaryHitPhase::TypeImpactCallback,
                    reason: PrimaryHitExternalBlock::TypeImpactCallbackUnavailable,
                },
            )
            .unwrap();
        let block = PrimaryHitBlock::External {
            phase: PrimaryHitPhase::TypeImpactCallback,
            reason: PrimaryHitExternalBlock::TypeImpactCallbackUnavailable,
        };
        assert_eq!(machine.poll(), PrimaryHitPoll::Blocked(block));
    }

    #[test]
    fn accepted_tail_rejects_handle_reuse_instead_of_rebinding_cached_pointer() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(None)));
        let delivery_receipt = advance_to_delivery(&mut machine);
        let mut reused = final_target(0, 0, RetailRuntimeValue::Known(None));
        reused.target_allocation_identity = TARGET_IDENTITY + 1;
        accepted(&mut machine, delivery_receipt, 10, reused);
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Blocked(PrimaryHitBlock::CachedTargetAllocationIdentityMismatch {
                expected: TARGET_IDENTITY,
                actual: TARGET_IDENTITY + 1,
            })
        );
    }

    #[test]
    fn base_factory_style_survivor_uses_existing_damage_contract_without_actor_dispatch() {
        let mut machine = start(cached_entry(RetailRuntimeValue::Known(Some(
            PRIMARY_HIT_TYPE_CALLBACK_ADDRESS,
        ))));
        let delivery_receipt = advance_to_delivery(&mut machine);

        let profile = DamageProfile {
            thresholds_raw: [0, 0, 200, 0, 0, 0, 0],
            multipliers_q8: [0, 0, 256, 0, 0, 0, 0],
        };
        let accepted_raw = request(cached_entry(RetailRuntimeValue::Known(None)))
            .delivery
            .packet
            .filtered_raw(Some(&profile));
        assert_eq!(accepted_raw, 400);
        let CheckedLocalDamageTransition::Survived(local) =
            checked_local_damage_transition_after_null_modifier(
                CheckedLocalDamageState {
                    generic: GenericEntityDamageState {
                        health_raw: 10_000,
                        pre_health_buffer_raw: 0,
                        already_dying: false,
                    },
                    generic_hit_sound_id: None,
                },
                NonZeroI32::new(accepted_raw).unwrap(),
            )
        else {
            panic!("bounded base/factory-compatible hit must survive");
        };
        assert_eq!(local.generic.health_after_subtraction_raw, 9_600);
        accepted(
            &mut machine,
            delivery_receipt,
            local.accepted_damage_raw,
            final_target(0, 0, RetailRuntimeValue::Known(None)),
        );
        assert_eq!(
            machine.poll(),
            PrimaryHitPoll::Complete(PrimaryHitCompletion::Accepted(
                NonZeroI32::new(400).unwrap()
            ))
        );
    }
}
