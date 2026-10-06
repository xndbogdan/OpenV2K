//! Detached checked-damage transaction recovered from retail
//! `FUN_00415040 -> FUN_00414E90 -> FUN_00410C10`.
//!
//! The retail chain is not one atomic "subtract health" operation. It caches
//! an admission target, resolves the same handle again for the damage profile,
//! lets an optional entity-owned modifier mutate all six packet dwords, and
//! then enters a generic-damage helper which performs two more lookups. The
//! generic helper caches its first target and second type pointers across
//! sounds and callbacks. A lethal result invokes a death helper which resolves
//! another target/type pair, then returns to the *original generic target*
//! after death processing. Re-resolving any of those cached pointers after a
//! callback would silently change retail lifetime semantics.
//!
//! This module therefore exposes every live callback, sound, network send,
//! resource release, write, and cached-pointer sample as a receipt-bound
//! adapter action. Opaque allocation/type identities identify pointer
//! instances without ever dereferencing retail memory in Rust. A missing,
//! reused, or unresolved cached pointer durably blocks the transaction instead
//! of fabricating a fresh handle binding.
//!
//! Actions are issued once. The adapter must journal external execution by
//! `(transaction_id, action_sequence)` until [`CheckedDamageMachine::resume`]
//! accepts the exact receipt. Recoverable protocol errors return the receipt
//! intact, so a corrected completion can be retried without replaying the
//! already executed side effect.

use std::num::{NonZeroI32, NonZeroU64};

use crate::damage::{DamageDeliveryRecord, DamageProfile, DAMAGE_CHANNEL_COUNT};
use crate::entity_collision_state::{
    RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};

/// Checked admission/filter/modifier wrapper.
pub const CHECKED_DAMAGE_ADDRESS: u32 = 0x0041_5040;
/// Generic remote/local buffer, hit-effect, health, and death dispatcher.
pub const GENERIC_DAMAGE_ADDRESS: u32 = 0x0041_4E90;
/// Optional local death helper entered by a lethal generic-health write.
pub const GENERIC_DEATH_ADDRESS: u32 = 0x0041_0C10;
/// Type-vtable offset of the generic hit callback.
pub const GENERIC_HIT_CALLBACK_VTABLE_OFFSET: u32 = 0x30;
/// Type-vtable offset of the death callback.
pub const GENERIC_DEATH_CALLBACK_VTABLE_OFFSET: u32 = 0x08;
/// Initial-target capability which enables filter-zero feedback.
pub const ZERO_DAMAGE_FEEDBACK_CAPABILITY_BIT: u8 = 0x08;
/// Generic/death network capability at target byte `+0x64`.
pub const DAMAGE_NETWORK_CAPABILITY_BIT: u8 = 0x01;
/// Death-helper capability which enters the managed-death finalizer after a
/// non-null death callback leaves the target dying.
pub const MANAGED_DEATH_CAPABILITY_BIT: u8 = 0x20;
/// Original generic-target state bit which selects player-kill feedback after
/// a successful death helper.
pub const PLAYER_KILL_FEEDBACK_STATE_BIT: u32 = 0x0100_0000;
/// Retail entity type used as player-source provenance.
pub const PLAYER_SOURCE_ENTITY_TYPE_RAW: u32 = 0x2e;
/// Filter-zero selector submitted before the authored feedback sound.
pub const ZERO_DAMAGE_FEEDBACK_SELECTOR: u8 = 0x17;
/// Filter-zero authored sound.
pub const ZERO_DAMAGE_FEEDBACK_SOUND_ID: u16 = 0x00db;
/// Successful player-kill selector.
pub const PLAYER_KILL_FEEDBACK_SELECTOR: u8 = 4;
/// Fallback sound used inside the managed-death finalizer.
pub const MANAGED_DEATH_FALLBACK_SOUND_ID: u16 = 0x00de;

/// Initial target cached by `FUN_00415040`.
///
/// The modifier and filter-zero branch continue to use this pointer even
/// though profile selection performs a second independent handle lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedDamageAdmissionTarget {
    pub target_allocation_identity: u64,
    pub state_flags_at_0x08: u32,
    pub capability_flags_at_0x64: u8,
    /// Entity callback at `+0x44`. It is read only after a nonzero filter.
    pub modifier_address: RetailRuntimeValue<Option<u32>>,
}

/// Fresh profile binding selected by the second lookup in `FUN_00415040`.
///
/// Neither identity is required to match the admission target: delete/rebind
/// behavior between retail lookups is intentionally observable. The profile
/// pointer is not retained after the pure filter call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedDamageFilterBinding {
    pub target_allocation_identity: u64,
    pub type_record_identity: u64,
    pub profile: RetailRuntimeValue<DamageProfile>,
}

/// Immutable arguments and call-entry observations for one checked delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedDamageRequest {
    pub target_handle: u32,
    /// `None` represents retail's null packet pointer.
    pub delivery: Option<DamageDeliveryRecord>,
    pub ratio_numerator: u32,
    pub ratio_denominator: u16,
    pub admission_target: RetailRuntimeValue<Option<CheckedDamageAdmissionTarget>>,
    /// The second lookup is consulted only after the admission, state-bit, and
    /// non-null packet gates have passed.
    pub filter_binding: RetailRuntimeValue<Option<CheckedDamageFilterBinding>>,
}

/// Target state returned by the first lookup in `FUN_00414E90`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericDamageEntryTarget {
    pub target_allocation_identity: u64,
    pub state_flags_at_0x08: u32,
    pub pre_health_buffer_raw: i32,
}

/// Two generic-damage lookups plus the global byte evaluated as the fourth
/// argument immediately before `FUN_00414E90`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericDamageEntryObservation {
    pub target: RetailRuntimeValue<Option<GenericDamageEntryTarget>>,
    /// Opaque cached type pointer selected by the second lookup. A remote or
    /// already-dying path can finish without dereferencing it.
    pub type_record_identity: RetailRuntimeValue<Option<u64>>,
    pub local_player_type_at_call: RetailRuntimeValue<u8>,
}

/// Same-allocation target sample after committing a positive pre-health buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericTargetStateSample {
    pub target_allocation_identity: u64,
    pub state_flags_at_0x08: u32,
}

/// Cached target/type sample at the generic-hit sound read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericHitSoundObservation {
    pub target_allocation_identity: u64,
    pub type_record_identity: u64,
    pub position_raw: [i16; 3],
    pub sound_id: RetailRuntimeValue<Option<u16>>,
}

/// Cached type sample at the generic-hit callback read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericHitCallbackObservation {
    pub type_record_identity: u64,
    pub callback_address: RetailRuntimeValue<Option<u32>>,
}

/// Health is intentionally sampled only after the generic hit callback has
/// returned. The callback may change it through the still-cached target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericHealthObservation {
    pub target_allocation_identity: u64,
    pub health_raw: i32,
}

/// Same-allocation proof returned by a target write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedTargetWriteObservation {
    pub target_allocation_identity: u64,
}

/// Post-health survivor network inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurvivorNetworkTailObservation {
    pub target_allocation_identity: u64,
    pub capability_flags_at_0x64: u8,
    pub network_session_active: bool,
    /// Read only if the other survivor-network predicates all pass.
    pub current_local_player_type: RetailRuntimeValue<u8>,
}

/// First target lookup in `FUN_00410C10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathEntryTarget {
    pub target_allocation_identity: u64,
    pub state_flags_at_0x08: u32,
}

/// Fresh death-helper target/type lookup pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathEntryObservation {
    pub target: RetailRuntimeValue<Option<DeathEntryTarget>>,
    /// A remote, already-dying, or missing death target does not dereference
    /// this cached type pointer.
    pub type_record_identity: RetailRuntimeValue<Option<u64>>,
}

/// Cached death target/type sample at the authored death-sound read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathSoundObservation {
    pub target_allocation_identity: u64,
    pub type_record_identity: u64,
    pub position_raw: [i16; 3],
    pub sound_id: RetailRuntimeValue<Option<u16>>,
}

/// Cached death target sample at attached-resource inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathAttachmentObservation {
    pub target_allocation_identity: u64,
    pub attached_resource_identity: RetailRuntimeValue<Option<u64>>,
}

/// Cached death type sample at the vtable `+0x08` callback read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathCallbackObservation {
    pub type_record_identity: u64,
    pub callback_address: RetailRuntimeValue<Option<u32>>,
}

/// Cached death target state inspected only after a non-null death callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathTargetAfterCallbackObservation {
    pub target_allocation_identity: u64,
    pub state_flags_at_0x08: u32,
    pub capability_flags_at_0x64: u8,
}

/// Original generic target sampled after `FUN_00410C10` returned nonzero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalTargetAfterDeathObservation {
    pub target_allocation_identity: u64,
    pub state_flags_at_0x08: u32,
}

/// Original generic target and globals read by the post-death network suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathNetworkTailObservation {
    pub target_allocation_identity: u64,
    pub capability_flags_at_0x64: u8,
    pub network_session_active: bool,
    /// Read only when the target/network capability predicates pass.
    pub current_local_player_type: RetailRuntimeValue<u8>,
}

/// Exact remote-owned generic-damage packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteDamageForwardRequest {
    pub target_handle: u32,
    pub damage_low_u16: u16,
    pub source_entity_type_raw: u32,
}

/// Exact positive-buffer write before dying-state inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreHealthBufferCommit {
    pub target_handle: u32,
    pub target_allocation_identity: u64,
    pub before_raw: i32,
    pub after_raw: i32,
    pub residual_damage_raw: i32,
}

/// Exact cached-health write after the optional generic hit callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericHealthCommit {
    pub target_handle: u32,
    pub target_allocation_identity: u64,
    pub health_before_raw: i32,
    pub damage_after_buffer_raw: i32,
    pub health_after_raw: i32,
}

/// Two-byte state packet sent by survivor/death network helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageLifecycleNetworkRequest {
    pub current_local_player_type_raw: u8,
    /// Survivor messages always use the byte captured at generic call entry.
    /// Death messages use zero unless source provenance is player type `0x2e`.
    pub source_local_player_type_raw: u8,
}

/// One externally visible boundary and anti-replay phase tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamagePhase {
    SelectZeroDamageFeedback,
    PlayZeroDamageFeedback,
    DamageModifier,
    ResolveGenericDamageEntry,
    ForwardRemoteDamage,
    CommitPreHealthBuffer,
    SampleGenericHitSound,
    PlayGenericHitSound,
    SampleGenericHitCallback,
    GenericHitCallback,
    SampleGenericHealth,
    CommitGenericHealth,
    SampleSurvivorNetworkTail,
    SubmitSurvivorNetwork,
    ResolveDeathEntry,
    CommitDeathState,
    SampleDeathSound,
    PlayDeathSound,
    SampleDeathAttachment,
    ReleaseDeathAttachment,
    CommitDeathAttachmentClear,
    SampleDeathCallback,
    DeathCallback,
    SampleDeathTargetAfterCallback,
    FinalizeManagedDeath,
    SampleOriginalTargetAfterDeath,
    SelectPlayerKillFeedback,
    SampleDeathNetworkTail,
    SubmitDeathNetwork,
}

/// Adapter action in exact retail order.
#[derive(Debug, PartialEq, Eq)]
pub enum CheckedDamageAction {
    SelectFeedback {
        phase: CheckedDamagePhase,
        selector: u8,
    },
    PlayNonPositionalSound {
        phase: CheckedDamagePhase,
        sound_id: u16,
    },
    InvokeDamageModifier {
        phase: CheckedDamagePhase,
        callback_address: u32,
        target_handle: u32,
        delivery: DamageDeliveryRecord,
        filtered_damage_raw: i32,
    },
    ResolveGenericDamageEntry {
        phase: CheckedDamagePhase,
        target_handle: u32,
    },
    ForwardRemoteDamage {
        phase: CheckedDamagePhase,
        request: RemoteDamageForwardRequest,
    },
    CommitPreHealthBuffer {
        phase: CheckedDamagePhase,
        commit: PreHealthBufferCommit,
    },
    SampleCachedGenericHitSound {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
        type_record_identity: u64,
    },
    PlayPositionalSound {
        phase: CheckedDamagePhase,
        sound_id: u16,
        position_raw: [i16; 3],
    },
    SampleCachedGenericHitCallback {
        phase: CheckedDamagePhase,
        target_handle: u32,
        type_record_identity: u64,
    },
    InvokeGenericHitCallback {
        phase: CheckedDamagePhase,
        callback_address: u32,
        target_handle: u32,
        damage_after_buffer_raw: i32,
    },
    SampleCachedGenericHealth {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
    },
    CommitGenericHealth {
        phase: CheckedDamagePhase,
        commit: GenericHealthCommit,
    },
    SampleSurvivorNetworkTail {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
    },
    SubmitSurvivorNetwork {
        phase: CheckedDamagePhase,
        request: DamageLifecycleNetworkRequest,
    },
    ResolveDeathEntry {
        phase: CheckedDamagePhase,
        target_handle: u32,
    },
    CommitDeathState {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
        state_flags_before: u32,
        state_flags_after: u32,
        health_after_raw: i32,
    },
    SampleCachedDeathSound {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
        type_record_identity: u64,
    },
    SampleCachedDeathAttachment {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
    },
    ReleaseDeathAttachment {
        phase: CheckedDamagePhase,
        attached_resource_identity: u64,
    },
    CommitDeathAttachmentClear {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
        attached_resource_identity: u64,
    },
    SampleCachedDeathCallback {
        phase: CheckedDamagePhase,
        target_handle: u32,
        type_record_identity: u64,
    },
    InvokeDeathCallback {
        phase: CheckedDamagePhase,
        callback_address: u32,
        target_handle: u32,
    },
    SampleCachedDeathTargetAfterCallback {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
    },
    /// Adapter-owned exact `FUN_00456CB0`, optional `FUN_00456CD0`,
    /// optional sound `0xDE`, then unconditional `FUN_00456DB0` sequence.
    FinalizeManagedDeath {
        phase: CheckedDamagePhase,
        fallback_sound_id: u16,
    },
    SampleOriginalTargetAfterDeath {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
    },
    SampleDeathNetworkTail {
        phase: CheckedDamagePhase,
        target_handle: u32,
        target_allocation_identity: u64,
    },
    SubmitDeathNetwork {
        phase: CheckedDamagePhase,
        request: DamageLifecycleNetworkRequest,
    },
}

impl CheckedDamageAction {
    pub const fn phase(&self) -> CheckedDamagePhase {
        match self {
            Self::SelectFeedback { phase, .. }
            | Self::PlayNonPositionalSound { phase, .. }
            | Self::InvokeDamageModifier { phase, .. }
            | Self::ResolveGenericDamageEntry { phase, .. }
            | Self::ForwardRemoteDamage { phase, .. }
            | Self::CommitPreHealthBuffer { phase, .. }
            | Self::SampleCachedGenericHitSound { phase, .. }
            | Self::PlayPositionalSound { phase, .. }
            | Self::SampleCachedGenericHitCallback { phase, .. }
            | Self::InvokeGenericHitCallback { phase, .. }
            | Self::SampleCachedGenericHealth { phase, .. }
            | Self::CommitGenericHealth { phase, .. }
            | Self::SampleSurvivorNetworkTail { phase, .. }
            | Self::SubmitSurvivorNetwork { phase, .. }
            | Self::ResolveDeathEntry { phase, .. }
            | Self::CommitDeathState { phase, .. }
            | Self::SampleCachedDeathSound { phase, .. }
            | Self::SampleCachedDeathAttachment { phase, .. }
            | Self::ReleaseDeathAttachment { phase, .. }
            | Self::CommitDeathAttachmentClear { phase, .. }
            | Self::SampleCachedDeathCallback { phase, .. }
            | Self::InvokeDeathCallback { phase, .. }
            | Self::SampleCachedDeathTargetAfterCallback { phase, .. }
            | Self::FinalizeManagedDeath { phase, .. }
            | Self::SampleOriginalTargetAfterDeath { phase, .. }
            | Self::SampleDeathNetworkTail { phase, .. }
            | Self::SubmitDeathNetwork { phase, .. } => *phase,
        }
    }
}

/// Caller-supplied identity for one live checked-damage transaction.
///
/// The caller must keep this value unique among concurrently live
/// [`CheckedDamageMachine`] instances. Receipts deliberately use this identity
/// plus their action sequence as their cross-adapter provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CheckedDamageTransactionId(NonZeroU64);

impl CheckedDamageTransactionId {
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

/// Linear proof that one specific action was issued.
///
/// Deliberately neither `Copy` nor `Clone`.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckedDamageReceipt {
    transaction_id: CheckedDamageTransactionId,
    action_sequence: u64,
}

impl CheckedDamageReceipt {
    pub const fn transaction_id(&self) -> CheckedDamageTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }
}

/// One non-cloneable action/receipt pair.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedCheckedDamageAction {
    pub receipt: CheckedDamageReceipt,
    pub action: CheckedDamageAction,
}

/// Private proof of the one death callback currently waiting on its issued
/// receipt.
///
/// This deliberately exposes no general machine internals. The primary-hit
/// owner uses the snapshot to distinguish the genuine, already-polled
/// callback from an action value reconstructed around a valid public receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutstandingCheckedDamageDeathCallback {
    pub(crate) transaction_id: CheckedDamageTransactionId,
    pub(crate) action_sequence: u64,
    pub(crate) callback_address: u32,
    pub(crate) target_handle: u32,
}

/// Adapter-side reason an otherwise proven boundary cannot complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamageExternalBlock {
    CallbackUnavailable,
    RuntimeReadUnavailable,
    RuntimeWriteUnavailable,
    SoundUnavailable,
    NetworkUnavailable,
    FeedbackUnavailable,
    ResourceReleaseUnavailable,
    DeathFinalizerUnavailable,
    AdapterRejected(u32),
}

/// Durable evidence/lifetime/runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamageBlock {
    UnresolvedAdmissionTarget,
    UnresolvedFilterBinding,
    MissingFilterBinding,
    UnresolvedDamageProfile,
    InvalidDamageChannel {
        slot: usize,
        channel: i32,
    },
    UnresolvedModifier,
    UnresolvedGenericCallLocalPlayerType,
    UnresolvedGenericTarget,
    UnresolvedGenericTypeRecord,
    MissingGenericTypeRecord,
    UnresolvedGenericHitSound,
    UnresolvedGenericHitCallback,
    UnresolvedSurvivorLocalPlayerType,
    UnresolvedDeathTarget,
    UnresolvedDeathTypeRecord,
    MissingDeathTypeRecord,
    UnresolvedDeathSound,
    UnresolvedDeathAttachment,
    UnresolvedDeathCallback,
    UnresolvedDeathLocalPlayerType,
    MissingCachedGenericTarget {
        phase: CheckedDamagePhase,
    },
    UnresolvedCachedGenericTarget {
        phase: CheckedDamagePhase,
    },
    MissingCachedGenericType {
        phase: CheckedDamagePhase,
    },
    UnresolvedCachedGenericType {
        phase: CheckedDamagePhase,
    },
    CachedGenericTargetIdentityMismatch {
        phase: CheckedDamagePhase,
        expected: u64,
        actual: u64,
    },
    CachedGenericTypeIdentityMismatch {
        phase: CheckedDamagePhase,
        expected: u64,
        actual: u64,
    },
    MissingCachedDeathTarget {
        phase: CheckedDamagePhase,
    },
    UnresolvedCachedDeathTarget {
        phase: CheckedDamagePhase,
    },
    MissingCachedDeathType {
        phase: CheckedDamagePhase,
    },
    UnresolvedCachedDeathType {
        phase: CheckedDamagePhase,
    },
    CachedDeathTargetIdentityMismatch {
        phase: CheckedDamagePhase,
        expected: u64,
        actual: u64,
    },
    CachedDeathTypeIdentityMismatch {
        phase: CheckedDamagePhase,
        expected: u64,
        actual: u64,
    },
    MissingOriginalGenericTargetAfterDeath,
    UnresolvedOriginalGenericTargetAfterDeath,
    OriginalGenericTargetIdentityMismatch {
        expected: u64,
        actual: u64,
    },
    External {
        phase: CheckedDamagePhase,
        reason: CheckedDamageExternalBlock,
    },
}

/// Adapter completion for one exact phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamageResume {
    Acknowledged {
        phase: CheckedDamagePhase,
    },
    ModifierReturned {
        phase: CheckedDamagePhase,
        damage_raw: i32,
        delivery_after_callback: DamageDeliveryRecord,
    },
    GenericDamageEntryResolved {
        phase: CheckedDamagePhase,
        observation: GenericDamageEntryObservation,
    },
    PreHealthBufferCommitted {
        phase: CheckedDamagePhase,
        target: RetailRuntimeValue<Option<GenericTargetStateSample>>,
    },
    GenericHitSoundSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<GenericHitSoundObservation>>,
    },
    GenericHitCallbackSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<GenericHitCallbackObservation>>,
    },
    GenericHealthSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<GenericHealthObservation>>,
    },
    GenericHealthCommitted {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<CachedTargetWriteObservation>>,
    },
    SurvivorNetworkTailSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<SurvivorNetworkTailObservation>>,
    },
    DeathEntryResolved {
        phase: CheckedDamagePhase,
        observation: DeathEntryObservation,
    },
    DeathStateCommitted {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<CachedTargetWriteObservation>>,
    },
    DeathSoundSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<DeathSoundObservation>>,
    },
    DeathAttachmentSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<DeathAttachmentObservation>>,
    },
    DeathAttachmentCleared {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<CachedTargetWriteObservation>>,
    },
    DeathCallbackSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<DeathCallbackObservation>>,
    },
    DeathTargetAfterCallbackSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<DeathTargetAfterCallbackObservation>>,
    },
    OriginalTargetAfterDeathSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<OriginalTargetAfterDeathObservation>>,
    },
    DeathNetworkTailSampled {
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<DeathNetworkTailObservation>>,
    },
    Blocked {
        phase: CheckedDamagePhase,
        reason: CheckedDamageExternalBlock,
    },
}

impl CheckedDamageResume {
    pub const fn phase(self) -> CheckedDamagePhase {
        match self {
            Self::Acknowledged { phase }
            | Self::ModifierReturned { phase, .. }
            | Self::GenericDamageEntryResolved { phase, .. }
            | Self::PreHealthBufferCommitted { phase, .. }
            | Self::GenericHitSoundSampled { phase, .. }
            | Self::GenericHitCallbackSampled { phase, .. }
            | Self::GenericHealthSampled { phase, .. }
            | Self::GenericHealthCommitted { phase, .. }
            | Self::SurvivorNetworkTailSampled { phase, .. }
            | Self::DeathEntryResolved { phase, .. }
            | Self::DeathStateCommitted { phase, .. }
            | Self::DeathSoundSampled { phase, .. }
            | Self::DeathAttachmentSampled { phase, .. }
            | Self::DeathAttachmentCleared { phase, .. }
            | Self::DeathCallbackSampled { phase, .. }
            | Self::DeathTargetAfterCallbackSampled { phase, .. }
            | Self::OriginalTargetAfterDeathSampled { phase, .. }
            | Self::DeathNetworkTailSampled { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

/// Retail zero-return origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamageZeroReason {
    AdmissionDisabled,
    NullDelivery,
    FilteredOut,
    ModifierReturnedZero,
}

/// Stable outer return from `FUN_00415040`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamageCompletion {
    /// Only the first admission lookup maps to this distinction.
    TargetMissing,
    ReturnedZero(CheckedDamageZeroReason),
    /// Exact signed bits returned by the filter/modifier, independent of late
    /// generic target loss, remote forwarding, or death restoration.
    ReturnedNonZero(NonZeroI32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckedDamagePoll {
    Action(IssuedCheckedDamageAction),
    Awaiting(CheckedDamagePhase),
    Blocked(CheckedDamageBlock),
    Complete(CheckedDamageCompletion),
}

/// Invalid adapter response which never advances or blocks the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckedDamageProtocolError {
    NoOutstandingAction {
        transaction_id: CheckedDamageTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: CheckedDamageTransactionId,
        actual: CheckedDamageTransactionId,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: CheckedDamagePhase,
        actual: CheckedDamagePhase,
    },
    UnexpectedResumeKind {
        phase: CheckedDamagePhase,
    },
}

/// Recoverable completion rejection. The outstanding receipt remains linear
/// and is returned to the adapter for retry or cross-machine routing.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckedDamageResumeFailure {
    pub receipt: CheckedDamageReceipt,
    pub error: CheckedDamageProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckedDamageStage {
    SelectZeroDamageFeedback,
    PlayZeroDamageFeedback,
    DamageModifier(u32),
    ResolveGenericDamageEntry,
    ForwardRemoteDamage(RemoteDamageForwardRequest),
    CommitPreHealthBuffer(PreHealthBufferCommit),
    SampleGenericHitSound,
    PlayGenericHitSound {
        sound_id: u16,
        position_raw: [i16; 3],
    },
    SampleGenericHitCallback,
    GenericHitCallback(u32),
    SampleGenericHealth,
    CommitGenericHealth(GenericHealthCommit),
    SampleSurvivorNetworkTail,
    SubmitSurvivorNetwork(DamageLifecycleNetworkRequest),
    ResolveDeathEntry,
    CommitDeathState {
        state_flags_before: u32,
    },
    SampleDeathSound,
    PlayDeathSound {
        sound_id: u16,
        position_raw: [i16; 3],
    },
    SampleDeathAttachment,
    ReleaseDeathAttachment(u64),
    CommitDeathAttachmentClear(u64),
    SampleDeathCallback,
    DeathCallback(u32),
    SampleDeathTargetAfterCallback,
    FinalizeManagedDeath,
    SampleOriginalTargetAfterDeath,
    SelectPlayerKillFeedback,
    SampleDeathNetworkTail,
    SubmitDeathNetwork(DamageLifecycleNetworkRequest),
    Blocked(CheckedDamageBlock),
    Complete(CheckedDamageCompletion),
}

impl CheckedDamageStage {
    const fn phase(self) -> Option<CheckedDamagePhase> {
        match self {
            Self::SelectZeroDamageFeedback => Some(CheckedDamagePhase::SelectZeroDamageFeedback),
            Self::PlayZeroDamageFeedback => Some(CheckedDamagePhase::PlayZeroDamageFeedback),
            Self::DamageModifier(_) => Some(CheckedDamagePhase::DamageModifier),
            Self::ResolveGenericDamageEntry => Some(CheckedDamagePhase::ResolveGenericDamageEntry),
            Self::ForwardRemoteDamage(_) => Some(CheckedDamagePhase::ForwardRemoteDamage),
            Self::CommitPreHealthBuffer(_) => Some(CheckedDamagePhase::CommitPreHealthBuffer),
            Self::SampleGenericHitSound => Some(CheckedDamagePhase::SampleGenericHitSound),
            Self::PlayGenericHitSound { .. } => Some(CheckedDamagePhase::PlayGenericHitSound),
            Self::SampleGenericHitCallback => Some(CheckedDamagePhase::SampleGenericHitCallback),
            Self::GenericHitCallback(_) => Some(CheckedDamagePhase::GenericHitCallback),
            Self::SampleGenericHealth => Some(CheckedDamagePhase::SampleGenericHealth),
            Self::CommitGenericHealth(_) => Some(CheckedDamagePhase::CommitGenericHealth),
            Self::SampleSurvivorNetworkTail => Some(CheckedDamagePhase::SampleSurvivorNetworkTail),
            Self::SubmitSurvivorNetwork(_) => Some(CheckedDamagePhase::SubmitSurvivorNetwork),
            Self::ResolveDeathEntry => Some(CheckedDamagePhase::ResolveDeathEntry),
            Self::CommitDeathState { .. } => Some(CheckedDamagePhase::CommitDeathState),
            Self::SampleDeathSound => Some(CheckedDamagePhase::SampleDeathSound),
            Self::PlayDeathSound { .. } => Some(CheckedDamagePhase::PlayDeathSound),
            Self::SampleDeathAttachment => Some(CheckedDamagePhase::SampleDeathAttachment),
            Self::ReleaseDeathAttachment(_) => Some(CheckedDamagePhase::ReleaseDeathAttachment),
            Self::CommitDeathAttachmentClear(_) => {
                Some(CheckedDamagePhase::CommitDeathAttachmentClear)
            }
            Self::SampleDeathCallback => Some(CheckedDamagePhase::SampleDeathCallback),
            Self::DeathCallback(_) => Some(CheckedDamagePhase::DeathCallback),
            Self::SampleDeathTargetAfterCallback => {
                Some(CheckedDamagePhase::SampleDeathTargetAfterCallback)
            }
            Self::FinalizeManagedDeath => Some(CheckedDamagePhase::FinalizeManagedDeath),
            Self::SampleOriginalTargetAfterDeath => {
                Some(CheckedDamagePhase::SampleOriginalTargetAfterDeath)
            }
            Self::SelectPlayerKillFeedback => Some(CheckedDamagePhase::SelectPlayerKillFeedback),
            Self::SampleDeathNetworkTail => Some(CheckedDamagePhase::SampleDeathNetworkTail),
            Self::SubmitDeathNetwork(_) => Some(CheckedDamagePhase::SubmitDeathNetwork),
            Self::Blocked(_) | Self::Complete(_) => None,
        }
    }
}

/// Receipt-bound detached checked-damage transaction.
///
/// Deliberately neither `Copy` nor `Clone`: duplicating it could replay a
/// modifier, callback, write, sound, network packet, release, or finalizer.
#[derive(Debug, PartialEq, Eq)]
pub struct CheckedDamageMachine {
    transaction_id: CheckedDamageTransactionId,
    request: CheckedDamageRequest,
    delivery: Option<DamageDeliveryRecord>,
    filtered_damage_raw: i32,
    accepted_damage_raw: i32,
    generic_target_allocation_identity: Option<u64>,
    generic_type_record_runtime: RetailRuntimeValue<Option<u64>>,
    generic_type_record_identity: Option<u64>,
    local_player_type_at_generic_call: Option<u8>,
    damage_after_buffer_raw: i32,
    death_target_allocation_identity: Option<u64>,
    death_type_record_identity: Option<u64>,
    stage: CheckedDamageStage,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
}

impl CheckedDamageMachine {
    pub fn start(
        transaction_id: CheckedDamageTransactionId,
        request: CheckedDamageRequest,
    ) -> Self {
        let mut machine = Self {
            transaction_id,
            request,
            delivery: request.delivery,
            filtered_damage_raw: 0,
            accepted_damage_raw: 0,
            generic_target_allocation_identity: None,
            generic_type_record_runtime: RetailRuntimeValue::Unresolved,
            generic_type_record_identity: None,
            local_player_type_at_generic_call: None,
            damage_after_buffer_raw: 0,
            death_target_allocation_identity: None,
            death_type_record_identity: None,
            stage: CheckedDamageStage::Complete(CheckedDamageCompletion::ReturnedZero(
                CheckedDamageZeroReason::AdmissionDisabled,
            )),
            next_action_sequence: 1,
            issued_action_sequence: None,
        };
        machine.enter_admission();
        machine
    }

    pub const fn filtered_damage_raw(&self) -> i32 {
        self.filtered_damage_raw
    }

    pub const fn accepted_damage_raw(&self) -> i32 {
        self.accepted_damage_raw
    }

    pub const fn current_delivery(&self) -> Option<DamageDeliveryRecord> {
        self.delivery
    }

    /// Snapshot only a death callback which has actually been issued.
    ///
    /// A staged callback without an outstanding sequence is intentionally not
    /// enough: adapters may act only after [`Self::poll`] has created the
    /// linear receipt for this exact boundary.
    pub(crate) const fn outstanding_death_callback(
        &self,
    ) -> Option<OutstandingCheckedDamageDeathCallback> {
        match (self.stage, self.issued_action_sequence) {
            (CheckedDamageStage::DeathCallback(callback_address), Some(action_sequence)) => {
                Some(OutstandingCheckedDamageDeathCallback {
                    transaction_id: self.transaction_id,
                    action_sequence,
                    callback_address,
                    target_handle: self.request.target_handle,
                })
            }
            _ => None,
        }
    }

    /// Issue the active boundary once. Repeated polling while a receipt is
    /// outstanding returns [`CheckedDamagePoll::Awaiting`].
    pub fn poll(&mut self) -> CheckedDamagePoll {
        if self.issued_action_sequence.is_some() {
            return CheckedDamagePoll::Awaiting(
                self.stage
                    .phase()
                    .expect("issued checked-damage action has an active phase"),
            );
        }

        let action = match self.stage {
            CheckedDamageStage::SelectZeroDamageFeedback => CheckedDamageAction::SelectFeedback {
                phase: CheckedDamagePhase::SelectZeroDamageFeedback,
                selector: ZERO_DAMAGE_FEEDBACK_SELECTOR,
            },
            CheckedDamageStage::PlayZeroDamageFeedback => {
                CheckedDamageAction::PlayNonPositionalSound {
                    phase: CheckedDamagePhase::PlayZeroDamageFeedback,
                    sound_id: ZERO_DAMAGE_FEEDBACK_SOUND_ID,
                }
            }
            CheckedDamageStage::DamageModifier(callback_address) => {
                CheckedDamageAction::InvokeDamageModifier {
                    phase: CheckedDamagePhase::DamageModifier,
                    callback_address,
                    target_handle: self.request.target_handle,
                    delivery: self
                        .delivery
                        .expect("modifier is reachable only with a non-null delivery"),
                    filtered_damage_raw: self.filtered_damage_raw,
                }
            }
            CheckedDamageStage::ResolveGenericDamageEntry => {
                CheckedDamageAction::ResolveGenericDamageEntry {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    target_handle: self.request.target_handle,
                }
            }
            CheckedDamageStage::ForwardRemoteDamage(request) => {
                CheckedDamageAction::ForwardRemoteDamage {
                    phase: CheckedDamagePhase::ForwardRemoteDamage,
                    request,
                }
            }
            CheckedDamageStage::CommitPreHealthBuffer(commit) => {
                CheckedDamageAction::CommitPreHealthBuffer {
                    phase: CheckedDamagePhase::CommitPreHealthBuffer,
                    commit,
                }
            }
            CheckedDamageStage::SampleGenericHitSound => {
                CheckedDamageAction::SampleCachedGenericHitSound {
                    phase: CheckedDamagePhase::SampleGenericHitSound,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.generic_target_identity(),
                    type_record_identity: self.generic_type_identity(),
                }
            }
            CheckedDamageStage::PlayGenericHitSound {
                sound_id,
                position_raw,
            } => CheckedDamageAction::PlayPositionalSound {
                phase: CheckedDamagePhase::PlayGenericHitSound,
                sound_id,
                position_raw,
            },
            CheckedDamageStage::SampleGenericHitCallback => {
                CheckedDamageAction::SampleCachedGenericHitCallback {
                    phase: CheckedDamagePhase::SampleGenericHitCallback,
                    target_handle: self.request.target_handle,
                    type_record_identity: self.generic_type_identity(),
                }
            }
            CheckedDamageStage::GenericHitCallback(callback_address) => {
                CheckedDamageAction::InvokeGenericHitCallback {
                    phase: CheckedDamagePhase::GenericHitCallback,
                    callback_address,
                    target_handle: self.request.target_handle,
                    damage_after_buffer_raw: self.damage_after_buffer_raw,
                }
            }
            CheckedDamageStage::SampleGenericHealth => {
                CheckedDamageAction::SampleCachedGenericHealth {
                    phase: CheckedDamagePhase::SampleGenericHealth,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.generic_target_identity(),
                }
            }
            CheckedDamageStage::CommitGenericHealth(commit) => {
                CheckedDamageAction::CommitGenericHealth {
                    phase: CheckedDamagePhase::CommitGenericHealth,
                    commit,
                }
            }
            CheckedDamageStage::SampleSurvivorNetworkTail => {
                CheckedDamageAction::SampleSurvivorNetworkTail {
                    phase: CheckedDamagePhase::SampleSurvivorNetworkTail,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.generic_target_identity(),
                }
            }
            CheckedDamageStage::SubmitSurvivorNetwork(request) => {
                CheckedDamageAction::SubmitSurvivorNetwork {
                    phase: CheckedDamagePhase::SubmitSurvivorNetwork,
                    request,
                }
            }
            CheckedDamageStage::ResolveDeathEntry => CheckedDamageAction::ResolveDeathEntry {
                phase: CheckedDamagePhase::ResolveDeathEntry,
                target_handle: self.request.target_handle,
            },
            CheckedDamageStage::CommitDeathState { state_flags_before } => {
                CheckedDamageAction::CommitDeathState {
                    phase: CheckedDamagePhase::CommitDeathState,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.death_target_identity(),
                    state_flags_before,
                    state_flags_after: state_flags_before | DYING_STATE_BIT,
                    health_after_raw: 0,
                }
            }
            CheckedDamageStage::SampleDeathSound => CheckedDamageAction::SampleCachedDeathSound {
                phase: CheckedDamagePhase::SampleDeathSound,
                target_handle: self.request.target_handle,
                target_allocation_identity: self.death_target_identity(),
                type_record_identity: self.death_type_identity(),
            },
            CheckedDamageStage::PlayDeathSound {
                sound_id,
                position_raw,
            } => CheckedDamageAction::PlayPositionalSound {
                phase: CheckedDamagePhase::PlayDeathSound,
                sound_id,
                position_raw,
            },
            CheckedDamageStage::SampleDeathAttachment => {
                CheckedDamageAction::SampleCachedDeathAttachment {
                    phase: CheckedDamagePhase::SampleDeathAttachment,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.death_target_identity(),
                }
            }
            CheckedDamageStage::ReleaseDeathAttachment(attached_resource_identity) => {
                CheckedDamageAction::ReleaseDeathAttachment {
                    phase: CheckedDamagePhase::ReleaseDeathAttachment,
                    attached_resource_identity,
                }
            }
            CheckedDamageStage::CommitDeathAttachmentClear(attached_resource_identity) => {
                CheckedDamageAction::CommitDeathAttachmentClear {
                    phase: CheckedDamagePhase::CommitDeathAttachmentClear,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.death_target_identity(),
                    attached_resource_identity,
                }
            }
            CheckedDamageStage::SampleDeathCallback => {
                CheckedDamageAction::SampleCachedDeathCallback {
                    phase: CheckedDamagePhase::SampleDeathCallback,
                    target_handle: self.request.target_handle,
                    type_record_identity: self.death_type_identity(),
                }
            }
            CheckedDamageStage::DeathCallback(callback_address) => {
                CheckedDamageAction::InvokeDeathCallback {
                    phase: CheckedDamagePhase::DeathCallback,
                    callback_address,
                    target_handle: self.request.target_handle,
                }
            }
            CheckedDamageStage::SampleDeathTargetAfterCallback => {
                CheckedDamageAction::SampleCachedDeathTargetAfterCallback {
                    phase: CheckedDamagePhase::SampleDeathTargetAfterCallback,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.death_target_identity(),
                }
            }
            CheckedDamageStage::FinalizeManagedDeath => CheckedDamageAction::FinalizeManagedDeath {
                phase: CheckedDamagePhase::FinalizeManagedDeath,
                fallback_sound_id: MANAGED_DEATH_FALLBACK_SOUND_ID,
            },
            CheckedDamageStage::SampleOriginalTargetAfterDeath => {
                CheckedDamageAction::SampleOriginalTargetAfterDeath {
                    phase: CheckedDamagePhase::SampleOriginalTargetAfterDeath,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.generic_target_identity(),
                }
            }
            CheckedDamageStage::SelectPlayerKillFeedback => CheckedDamageAction::SelectFeedback {
                phase: CheckedDamagePhase::SelectPlayerKillFeedback,
                selector: PLAYER_KILL_FEEDBACK_SELECTOR,
            },
            CheckedDamageStage::SampleDeathNetworkTail => {
                CheckedDamageAction::SampleDeathNetworkTail {
                    phase: CheckedDamagePhase::SampleDeathNetworkTail,
                    target_handle: self.request.target_handle,
                    target_allocation_identity: self.generic_target_identity(),
                }
            }
            CheckedDamageStage::SubmitDeathNetwork(request) => {
                CheckedDamageAction::SubmitDeathNetwork {
                    phase: CheckedDamagePhase::SubmitDeathNetwork,
                    request,
                }
            }
            CheckedDamageStage::Blocked(block) => return CheckedDamagePoll::Blocked(block),
            CheckedDamageStage::Complete(completion) => {
                return CheckedDamagePoll::Complete(completion);
            }
        };

        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one checked-damage transaction cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        CheckedDamagePoll::Action(IssuedCheckedDamageAction {
            receipt: CheckedDamageReceipt {
                transaction_id: self.transaction_id,
                action_sequence,
            },
            action,
        })
    }

    /// Complete one outstanding boundary.
    ///
    /// A protocol rejection leaves the machine and outstanding action intact
    /// and returns ownership of the submitted receipt.
    pub fn resume(
        &mut self,
        receipt: CheckedDamageReceipt,
        completion: CheckedDamageResume,
    ) -> Result<(), CheckedDamageResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            let error = CheckedDamageProtocolError::NoOutstandingAction {
                transaction_id: receipt.transaction_id,
                action_sequence: receipt.action_sequence,
            };
            return Err(CheckedDamageResumeFailure { receipt, error });
        };
        if receipt.transaction_id != self.transaction_id {
            let error = CheckedDamageProtocolError::ReceiptTransactionMismatch {
                expected: self.transaction_id,
                actual: receipt.transaction_id,
            };
            return Err(CheckedDamageResumeFailure { receipt, error });
        }
        if receipt.action_sequence != expected_sequence {
            let error = CheckedDamageProtocolError::ReceiptSequenceMismatch {
                expected: expected_sequence,
                actual: receipt.action_sequence,
            };
            return Err(CheckedDamageResumeFailure { receipt, error });
        }

        let expected = self
            .stage
            .phase()
            .expect("outstanding checked-damage receipt has an active phase");
        let actual = completion.phase();
        if actual != expected {
            return Err(CheckedDamageResumeFailure {
                receipt,
                error: CheckedDamageProtocolError::PhaseMismatch { expected, actual },
            });
        }
        if let CheckedDamageResume::Blocked { reason, .. } = completion {
            self.stage = CheckedDamageStage::Blocked(CheckedDamageBlock::External {
                phase: expected,
                reason,
            });
            self.issued_action_sequence = None;
            return Ok(());
        }

        match (self.stage, completion) {
            (
                CheckedDamageStage::SelectZeroDamageFeedback,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::SelectZeroDamageFeedback,
                },
            ) => {
                self.stage = CheckedDamageStage::PlayZeroDamageFeedback;
            }
            (
                CheckedDamageStage::PlayZeroDamageFeedback,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::PlayZeroDamageFeedback,
                },
            ) => {
                self.stage = CheckedDamageStage::Complete(CheckedDamageCompletion::ReturnedZero(
                    CheckedDamageZeroReason::FilteredOut,
                ));
            }
            (
                CheckedDamageStage::DamageModifier(_),
                CheckedDamageResume::ModifierReturned {
                    damage_raw,
                    delivery_after_callback,
                    ..
                },
            ) => {
                self.accepted_damage_raw = damage_raw;
                self.delivery = Some(delivery_after_callback);
                self.stage = CheckedDamageStage::ResolveGenericDamageEntry;
            }
            (
                CheckedDamageStage::ResolveGenericDamageEntry,
                CheckedDamageResume::GenericDamageEntryResolved { observation, .. },
            ) => self.enter_generic_damage(observation),
            (
                CheckedDamageStage::ForwardRemoteDamage(_),
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::ForwardRemoteDamage,
                },
            ) => self.complete_outer_return(),
            (
                CheckedDamageStage::CommitPreHealthBuffer(_),
                CheckedDamageResume::PreHealthBufferCommitted { target, .. },
            ) => self.enter_after_buffer_commit(target),
            (
                CheckedDamageStage::SampleGenericHitSound,
                CheckedDamageResume::GenericHitSoundSampled { observation, .. },
            ) => self.enter_after_generic_hit_sound_sample(observation),
            (
                CheckedDamageStage::PlayGenericHitSound { .. },
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::PlayGenericHitSound,
                },
            ) => {
                self.stage = CheckedDamageStage::SampleGenericHitCallback;
            }
            (
                CheckedDamageStage::SampleGenericHitCallback,
                CheckedDamageResume::GenericHitCallbackSampled { observation, .. },
            ) => self.enter_after_generic_hit_callback_sample(observation),
            (
                CheckedDamageStage::GenericHitCallback(_),
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::GenericHitCallback,
                },
            ) => {
                self.stage = CheckedDamageStage::SampleGenericHealth;
            }
            (
                CheckedDamageStage::SampleGenericHealth,
                CheckedDamageResume::GenericHealthSampled { observation, .. },
            ) => self.enter_after_generic_health_sample(observation),
            (
                CheckedDamageStage::CommitGenericHealth(commit),
                CheckedDamageResume::GenericHealthCommitted { observation, .. },
            ) => self.enter_after_generic_health_commit(commit, observation),
            (
                CheckedDamageStage::SampleSurvivorNetworkTail,
                CheckedDamageResume::SurvivorNetworkTailSampled { observation, .. },
            ) => self.enter_after_survivor_network_tail(observation),
            (
                CheckedDamageStage::SubmitSurvivorNetwork(_),
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::SubmitSurvivorNetwork,
                },
            ) => self.complete_outer_return(),
            (
                CheckedDamageStage::ResolveDeathEntry,
                CheckedDamageResume::DeathEntryResolved { observation, .. },
            ) => self.enter_death_helper(observation),
            (
                CheckedDamageStage::CommitDeathState { .. },
                CheckedDamageResume::DeathStateCommitted { observation, .. },
            ) => {
                if self.accept_death_target_write(CheckedDamagePhase::CommitDeathState, observation)
                {
                    self.stage = CheckedDamageStage::SampleDeathSound;
                }
            }
            (
                CheckedDamageStage::SampleDeathSound,
                CheckedDamageResume::DeathSoundSampled { observation, .. },
            ) => self.enter_after_death_sound_sample(observation),
            (
                CheckedDamageStage::PlayDeathSound { .. },
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::PlayDeathSound,
                },
            ) => {
                self.stage = CheckedDamageStage::SampleDeathAttachment;
            }
            (
                CheckedDamageStage::SampleDeathAttachment,
                CheckedDamageResume::DeathAttachmentSampled { observation, .. },
            ) => self.enter_after_death_attachment_sample(observation),
            (
                CheckedDamageStage::ReleaseDeathAttachment(resource),
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::ReleaseDeathAttachment,
                },
            ) => {
                self.stage = CheckedDamageStage::CommitDeathAttachmentClear(resource);
            }
            (
                CheckedDamageStage::CommitDeathAttachmentClear(_),
                CheckedDamageResume::DeathAttachmentCleared { observation, .. },
            ) => {
                if self.accept_death_target_write(
                    CheckedDamagePhase::CommitDeathAttachmentClear,
                    observation,
                ) {
                    self.stage = CheckedDamageStage::SampleDeathCallback;
                }
            }
            (
                CheckedDamageStage::SampleDeathCallback,
                CheckedDamageResume::DeathCallbackSampled { observation, .. },
            ) => self.enter_after_death_callback_sample(observation),
            (
                CheckedDamageStage::DeathCallback(_),
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::DeathCallback,
                },
            ) => {
                self.stage = CheckedDamageStage::SampleDeathTargetAfterCallback;
            }
            (
                CheckedDamageStage::SampleDeathTargetAfterCallback,
                CheckedDamageResume::DeathTargetAfterCallbackSampled { observation, .. },
            ) => self.enter_after_death_callback_target_sample(observation),
            (
                CheckedDamageStage::FinalizeManagedDeath,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::FinalizeManagedDeath,
                },
            ) => self.enter_successful_death_suffix(),
            (
                CheckedDamageStage::SampleOriginalTargetAfterDeath,
                CheckedDamageResume::OriginalTargetAfterDeathSampled { observation, .. },
            ) => self.enter_after_original_target_sample(observation),
            (
                CheckedDamageStage::SelectPlayerKillFeedback,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::SelectPlayerKillFeedback,
                },
            ) => {
                self.stage = CheckedDamageStage::SampleDeathNetworkTail;
            }
            (
                CheckedDamageStage::SampleDeathNetworkTail,
                CheckedDamageResume::DeathNetworkTailSampled { observation, .. },
            ) => self.enter_after_death_network_tail(observation),
            (
                CheckedDamageStage::SubmitDeathNetwork(_),
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::SubmitDeathNetwork,
                },
            ) => self.complete_outer_return(),
            _ => {
                return Err(CheckedDamageResumeFailure {
                    receipt,
                    error: CheckedDamageProtocolError::UnexpectedResumeKind { phase: expected },
                });
            }
        }

        self.issued_action_sequence = None;
        Ok(())
    }

    fn enter_admission(&mut self) {
        let admission = match self.request.admission_target {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedAdmissionTarget);
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::Complete(CheckedDamageCompletion::TargetMissing);
                return;
            }
            RetailRuntimeValue::Known(Some(admission)) => admission,
        };
        if admission.state_flags_at_0x08 & CHECKED_DAMAGE_ENABLED_STATE_BIT == 0 {
            self.stage = CheckedDamageStage::Complete(CheckedDamageCompletion::ReturnedZero(
                CheckedDamageZeroReason::AdmissionDisabled,
            ));
            return;
        }
        let Some(delivery) = self.delivery else {
            self.stage = CheckedDamageStage::Complete(CheckedDamageCompletion::ReturnedZero(
                CheckedDamageZeroReason::NullDelivery,
            ));
            return;
        };

        let binding = match self.request.filter_binding {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedFilterBinding);
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::Blocked(CheckedDamageBlock::MissingFilterBinding);
                return;
            }
            RetailRuntimeValue::Known(Some(binding)) => binding,
        };
        let profile = match binding.profile {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDamageProfile);
                return;
            }
            RetailRuntimeValue::Known(profile) => profile,
        };

        let filtered = match filter_checked_damage(
            delivery,
            profile,
            self.request.ratio_numerator,
            self.request.ratio_denominator,
        ) {
            Ok(filtered) => filtered,
            Err(block) => {
                self.stage = CheckedDamageStage::Blocked(block);
                return;
            }
        };
        self.filtered_damage_raw = filtered;
        self.accepted_damage_raw = filtered;

        if filtered == 0 {
            if zero_damage_feedback_enabled(admission, delivery) {
                self.stage = CheckedDamageStage::SelectZeroDamageFeedback;
            } else {
                self.stage = CheckedDamageStage::Complete(CheckedDamageCompletion::ReturnedZero(
                    CheckedDamageZeroReason::FilteredOut,
                ));
            }
            return;
        }

        match admission.modifier_address {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedModifier);
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::ResolveGenericDamageEntry;
            }
            RetailRuntimeValue::Known(Some(callback_address)) => {
                self.stage = CheckedDamageStage::DamageModifier(callback_address);
            }
        }
    }

    fn enter_generic_damage(&mut self, observation: GenericDamageEntryObservation) {
        let local_player_type = match observation.local_player_type_at_call {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedGenericCallLocalPlayerType,
                );
                return;
            }
        };
        self.local_player_type_at_generic_call = Some(local_player_type);

        let target = match observation.target {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedGenericTarget);
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.complete_outer_return();
                return;
            }
            RetailRuntimeValue::Known(Some(target)) => target,
        };
        self.generic_target_allocation_identity = Some(target.target_allocation_identity);

        if target.state_flags_at_0x08 & REMOTE_OWNED_STATE_BIT != 0 {
            let delivery = self
                .delivery
                .expect("generic damage follows a non-null delivery");
            self.stage = CheckedDamageStage::ForwardRemoteDamage(RemoteDamageForwardRequest {
                target_handle: self.request.target_handle,
                damage_low_u16: self.accepted_damage_raw as u16,
                source_entity_type_raw: delivery.source_entity_type_raw,
            });
            return;
        }

        self.generic_type_record_runtime = observation.type_record_identity;
        self.generic_type_record_identity = match observation.type_record_identity {
            RetailRuntimeValue::Known(identity) => identity,
            RetailRuntimeValue::Unresolved => None,
        };

        if target.pre_health_buffer_raw > 0 {
            let remaining = target
                .pre_health_buffer_raw
                .wrapping_sub(self.accepted_damage_raw);
            let (after_raw, residual_damage_raw) = if remaining < 0 {
                (0, remaining.wrapping_neg())
            } else {
                (remaining, 0)
            };
            self.damage_after_buffer_raw = residual_damage_raw;
            self.stage = CheckedDamageStage::CommitPreHealthBuffer(PreHealthBufferCommit {
                target_handle: self.request.target_handle,
                target_allocation_identity: target.target_allocation_identity,
                before_raw: target.pre_health_buffer_raw,
                after_raw,
                residual_damage_raw,
            });
        } else {
            self.damage_after_buffer_raw = self.accepted_damage_raw;
            self.enter_live_generic_effects(target.state_flags_at_0x08);
        }
    }

    fn enter_after_buffer_commit(
        &mut self,
        target: RetailRuntimeValue<Option<GenericTargetStateSample>>,
    ) {
        let Some(target) =
            self.accept_generic_target_state(CheckedDamagePhase::CommitPreHealthBuffer, target)
        else {
            return;
        };
        self.enter_live_generic_effects(target.state_flags_at_0x08);
    }

    fn enter_live_generic_effects(&mut self, state_flags: u32) {
        if state_flags & DYING_STATE_BIT != 0 {
            self.complete_outer_return();
            return;
        }
        match self.generic_type_record_identity {
            Some(_) => {
                self.stage = CheckedDamageStage::SampleGenericHitSound;
            }
            None => {
                // Preserve the distinction between an unknown second lookup
                // and an observed null type binding.
                self.stage = match self.current_generic_type_runtime_value() {
                    RetailRuntimeValue::Unresolved => {
                        CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedGenericTypeRecord)
                    }
                    RetailRuntimeValue::Known(None) => {
                        CheckedDamageStage::Blocked(CheckedDamageBlock::MissingGenericTypeRecord)
                    }
                    RetailRuntimeValue::Known(Some(_)) => {
                        unreachable!("known generic type identity was retained")
                    }
                };
            }
        }
    }

    fn enter_after_generic_hit_sound_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<GenericHitSoundObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedCachedGenericTarget {
                        phase: CheckedDamagePhase::SampleGenericHitSound,
                    },
                );
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedGenericTarget {
                        phase: CheckedDamagePhase::SampleGenericHitSound,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_generic_target_identity(
            CheckedDamagePhase::SampleGenericHitSound,
            observation.target_allocation_identity,
        ) || !self.validate_generic_type_identity(
            CheckedDamagePhase::SampleGenericHitSound,
            observation.type_record_identity,
        ) {
            return;
        }

        match observation.sound_id {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedGenericHitSound);
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::SampleGenericHitCallback;
            }
            RetailRuntimeValue::Known(Some(sound_id)) => {
                self.stage = CheckedDamageStage::PlayGenericHitSound {
                    sound_id,
                    position_raw: observation.position_raw,
                };
            }
        }
    }

    fn enter_after_generic_hit_callback_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<GenericHitCallbackObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedCachedGenericType {
                        phase: CheckedDamagePhase::SampleGenericHitCallback,
                    });
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedGenericType {
                        phase: CheckedDamagePhase::SampleGenericHitCallback,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_generic_type_identity(
            CheckedDamagePhase::SampleGenericHitCallback,
            observation.type_record_identity,
        ) {
            return;
        }
        match observation.callback_address {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedGenericHitCallback);
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::SampleGenericHealth;
            }
            RetailRuntimeValue::Known(Some(callback_address)) => {
                self.stage = CheckedDamageStage::GenericHitCallback(callback_address);
            }
        }
    }

    fn enter_after_generic_health_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<GenericHealthObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedCachedGenericTarget {
                        phase: CheckedDamagePhase::SampleGenericHealth,
                    },
                );
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedGenericTarget {
                        phase: CheckedDamagePhase::SampleGenericHealth,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_generic_target_identity(
            CheckedDamagePhase::SampleGenericHealth,
            observation.target_allocation_identity,
        ) {
            return;
        }
        self.stage = CheckedDamageStage::CommitGenericHealth(GenericHealthCommit {
            target_handle: self.request.target_handle,
            target_allocation_identity: observation.target_allocation_identity,
            health_before_raw: observation.health_raw,
            damage_after_buffer_raw: self.damage_after_buffer_raw,
            health_after_raw: observation
                .health_raw
                .wrapping_sub(self.damage_after_buffer_raw),
        });
    }

    fn enter_after_generic_health_commit(
        &mut self,
        commit: GenericHealthCommit,
        observation: RetailRuntimeValue<Option<CachedTargetWriteObservation>>,
    ) {
        if !self.accept_generic_target_write(CheckedDamagePhase::CommitGenericHealth, observation) {
            return;
        }
        if commit.health_after_raw > 0 {
            self.stage = CheckedDamageStage::SampleSurvivorNetworkTail;
        } else {
            self.stage = CheckedDamageStage::ResolveDeathEntry;
        }
    }

    fn enter_after_survivor_network_tail(
        &mut self,
        observation: RetailRuntimeValue<Option<SurvivorNetworkTailObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedCachedGenericTarget {
                        phase: CheckedDamagePhase::SampleSurvivorNetworkTail,
                    },
                );
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedGenericTarget {
                        phase: CheckedDamagePhase::SampleSurvivorNetworkTail,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_generic_target_identity(
            CheckedDamagePhase::SampleSurvivorNetworkTail,
            observation.target_allocation_identity,
        ) {
            return;
        }
        let source_type = self.current_delivery_record().source_entity_type_raw;
        if !observation.network_session_active
            || observation.capability_flags_at_0x64 & DAMAGE_NETWORK_CAPABILITY_BIT == 0
            || source_type != PLAYER_SOURCE_ENTITY_TYPE_RAW
        {
            self.complete_outer_return();
            return;
        }
        let current_local = match observation.current_local_player_type {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedSurvivorLocalPlayerType,
                );
                return;
            }
        };
        self.stage = CheckedDamageStage::SubmitSurvivorNetwork(DamageLifecycleNetworkRequest {
            current_local_player_type_raw: current_local,
            source_local_player_type_raw: self.local_player_type_at_generic_call(),
        });
    }

    fn enter_death_helper(&mut self, observation: DeathEntryObservation) {
        let target = match observation.target {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDeathTarget);
                return;
            }
            RetailRuntimeValue::Known(None) => {
                // FUN_00410C10 initializes its return to one before lookup.
                self.enter_successful_death_suffix();
                return;
            }
            RetailRuntimeValue::Known(Some(target)) => target,
        };
        self.death_target_allocation_identity = Some(target.target_allocation_identity);

        if target.state_flags_at_0x08 & REMOTE_OWNED_STATE_BIT != 0 {
            // Death helper returns zero; generic damage stops before its
            // original-cached-target suffix.
            self.complete_outer_return();
            return;
        }
        if target.state_flags_at_0x08 & DYING_STATE_BIT != 0 {
            self.enter_successful_death_suffix();
            return;
        }

        self.death_type_record_identity = match observation.type_record_identity {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDeathTypeRecord);
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingDeathTypeRecord);
                return;
            }
            RetailRuntimeValue::Known(Some(identity)) => Some(identity),
        };
        self.stage = CheckedDamageStage::CommitDeathState {
            state_flags_before: target.state_flags_at_0x08,
        };
    }

    fn enter_after_death_sound_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<DeathSoundObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedCachedDeathTarget {
                        phase: CheckedDamagePhase::SampleDeathSound,
                    });
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedDeathTarget {
                        phase: CheckedDamagePhase::SampleDeathSound,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_death_target_identity(
            CheckedDamagePhase::SampleDeathSound,
            observation.target_allocation_identity,
        ) || !self.validate_death_type_identity(
            CheckedDamagePhase::SampleDeathSound,
            observation.type_record_identity,
        ) {
            return;
        }
        match observation.sound_id {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDeathSound);
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::SampleDeathAttachment;
            }
            RetailRuntimeValue::Known(Some(sound_id)) => {
                self.stage = CheckedDamageStage::PlayDeathSound {
                    sound_id,
                    position_raw: observation.position_raw,
                };
            }
        }
    }

    fn enter_after_death_attachment_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<DeathAttachmentObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedCachedDeathTarget {
                        phase: CheckedDamagePhase::SampleDeathAttachment,
                    });
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedDeathTarget {
                        phase: CheckedDamagePhase::SampleDeathAttachment,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_death_target_identity(
            CheckedDamagePhase::SampleDeathAttachment,
            observation.target_allocation_identity,
        ) {
            return;
        }
        match observation.attached_resource_identity {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDeathAttachment);
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::SampleDeathCallback;
            }
            RetailRuntimeValue::Known(Some(identity)) => {
                self.stage = CheckedDamageStage::ReleaseDeathAttachment(identity);
            }
        }
    }

    fn enter_after_death_callback_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<DeathCallbackObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedCachedDeathType {
                        phase: CheckedDamagePhase::SampleDeathCallback,
                    });
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedDeathType {
                        phase: CheckedDamagePhase::SampleDeathCallback,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_death_type_identity(
            CheckedDamagePhase::SampleDeathCallback,
            observation.type_record_identity,
        ) {
            return;
        }
        match observation.callback_address {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDeathCallback);
            }
            RetailRuntimeValue::Known(None) => {
                // Retail does not inspect the post-callback dying/capability
                // fields when the callback pointer itself is null.
                self.enter_successful_death_suffix();
            }
            RetailRuntimeValue::Known(Some(callback_address)) => {
                self.stage = CheckedDamageStage::DeathCallback(callback_address);
            }
        }
    }

    fn enter_after_death_callback_target_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<DeathTargetAfterCallbackObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedCachedDeathTarget {
                        phase: CheckedDamagePhase::SampleDeathTargetAfterCallback,
                    });
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedDeathTarget {
                        phase: CheckedDamagePhase::SampleDeathTargetAfterCallback,
                    });
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if !self.validate_death_target_identity(
            CheckedDamagePhase::SampleDeathTargetAfterCallback,
            observation.target_allocation_identity,
        ) {
            return;
        }
        if observation.state_flags_at_0x08 & DYING_STATE_BIT == 0 {
            // Callback restored/cancelled death; helper returns zero.
            self.complete_outer_return();
        } else if observation.capability_flags_at_0x64 & MANAGED_DEATH_CAPABILITY_BIT != 0 {
            self.stage = CheckedDamageStage::FinalizeManagedDeath;
        } else {
            self.enter_successful_death_suffix();
        }
    }

    fn enter_successful_death_suffix(&mut self) {
        self.stage = CheckedDamageStage::SampleOriginalTargetAfterDeath;
    }

    fn enter_after_original_target_sample(
        &mut self,
        observation: RetailRuntimeValue<Option<OriginalTargetAfterDeathObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedOriginalGenericTargetAfterDeath,
                );
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::MissingOriginalGenericTargetAfterDeath,
                );
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if observation.target_allocation_identity != self.generic_target_identity() {
            self.stage = CheckedDamageStage::Blocked(
                CheckedDamageBlock::OriginalGenericTargetIdentityMismatch {
                    expected: self.generic_target_identity(),
                    actual: observation.target_allocation_identity,
                },
            );
            return;
        }
        if self.current_delivery_record().source_entity_type_raw == PLAYER_SOURCE_ENTITY_TYPE_RAW
            && observation.state_flags_at_0x08 & PLAYER_KILL_FEEDBACK_STATE_BIT != 0
        {
            self.stage = CheckedDamageStage::SelectPlayerKillFeedback;
        } else {
            self.stage = CheckedDamageStage::SampleDeathNetworkTail;
        }
    }

    fn enter_after_death_network_tail(
        &mut self,
        observation: RetailRuntimeValue<Option<DeathNetworkTailObservation>>,
    ) {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedOriginalGenericTargetAfterDeath,
                );
                return;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::MissingOriginalGenericTargetAfterDeath,
                );
                return;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        if observation.target_allocation_identity != self.generic_target_identity() {
            self.stage = CheckedDamageStage::Blocked(
                CheckedDamageBlock::OriginalGenericTargetIdentityMismatch {
                    expected: self.generic_target_identity(),
                    actual: observation.target_allocation_identity,
                },
            );
            return;
        }
        if !observation.network_session_active
            || observation.capability_flags_at_0x64 & DAMAGE_NETWORK_CAPABILITY_BIT == 0
        {
            self.complete_outer_return();
            return;
        }
        let current_local = match observation.current_local_player_type {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedDeathLocalPlayerType);
                return;
            }
        };
        let source_local = if self.current_delivery_record().source_entity_type_raw
            == PLAYER_SOURCE_ENTITY_TYPE_RAW
        {
            self.local_player_type_at_generic_call()
        } else {
            0
        };
        self.stage = CheckedDamageStage::SubmitDeathNetwork(DamageLifecycleNetworkRequest {
            current_local_player_type_raw: current_local,
            source_local_player_type_raw: source_local,
        });
    }

    fn accept_generic_target_state(
        &mut self,
        phase: CheckedDamagePhase,
        target: RetailRuntimeValue<Option<GenericTargetStateSample>>,
    ) -> Option<GenericTargetStateSample> {
        let target = match target {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedCachedGenericTarget { phase },
                );
                return None;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedGenericTarget {
                        phase,
                    });
                return None;
            }
            RetailRuntimeValue::Known(Some(target)) => target,
        };
        self.validate_generic_target_identity(phase, target.target_allocation_identity)
            .then_some(target)
    }

    fn accept_generic_target_write(
        &mut self,
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<CachedTargetWriteObservation>>,
    ) -> bool {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage = CheckedDamageStage::Blocked(
                    CheckedDamageBlock::UnresolvedCachedGenericTarget { phase },
                );
                return false;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedGenericTarget {
                        phase,
                    });
                return false;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        self.validate_generic_target_identity(phase, observation.target_allocation_identity)
    }

    fn accept_death_target_write(
        &mut self,
        phase: CheckedDamagePhase,
        observation: RetailRuntimeValue<Option<CachedTargetWriteObservation>>,
    ) -> bool {
        let observation = match observation {
            RetailRuntimeValue::Unresolved => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::UnresolvedCachedDeathTarget {
                        phase,
                    });
                return false;
            }
            RetailRuntimeValue::Known(None) => {
                self.stage =
                    CheckedDamageStage::Blocked(CheckedDamageBlock::MissingCachedDeathTarget {
                        phase,
                    });
                return false;
            }
            RetailRuntimeValue::Known(Some(observation)) => observation,
        };
        self.validate_death_target_identity(phase, observation.target_allocation_identity)
    }

    fn validate_generic_target_identity(&mut self, phase: CheckedDamagePhase, actual: u64) -> bool {
        let expected = self.generic_target_identity();
        if actual == expected {
            true
        } else {
            self.stage = CheckedDamageStage::Blocked(
                CheckedDamageBlock::CachedGenericTargetIdentityMismatch {
                    phase,
                    expected,
                    actual,
                },
            );
            false
        }
    }

    fn validate_generic_type_identity(&mut self, phase: CheckedDamagePhase, actual: u64) -> bool {
        let expected = self.generic_type_identity();
        if actual == expected {
            true
        } else {
            self.stage = CheckedDamageStage::Blocked(
                CheckedDamageBlock::CachedGenericTypeIdentityMismatch {
                    phase,
                    expected,
                    actual,
                },
            );
            false
        }
    }

    fn validate_death_target_identity(&mut self, phase: CheckedDamagePhase, actual: u64) -> bool {
        let expected = self.death_target_identity();
        if actual == expected {
            true
        } else {
            self.stage = CheckedDamageStage::Blocked(
                CheckedDamageBlock::CachedDeathTargetIdentityMismatch {
                    phase,
                    expected,
                    actual,
                },
            );
            false
        }
    }

    fn validate_death_type_identity(&mut self, phase: CheckedDamagePhase, actual: u64) -> bool {
        let expected = self.death_type_identity();
        if actual == expected {
            true
        } else {
            self.stage =
                CheckedDamageStage::Blocked(CheckedDamageBlock::CachedDeathTypeIdentityMismatch {
                    phase,
                    expected,
                    actual,
                });
            false
        }
    }

    fn complete_outer_return(&mut self) {
        self.stage =
            CheckedDamageStage::Complete(match NonZeroI32::new(self.accepted_damage_raw) {
                Some(nonzero) => CheckedDamageCompletion::ReturnedNonZero(nonzero),
                None => CheckedDamageCompletion::ReturnedZero(
                    CheckedDamageZeroReason::ModifierReturnedZero,
                ),
            });
    }

    fn generic_target_identity(&self) -> u64 {
        self.generic_target_allocation_identity
            .expect("active generic stage has a cached target identity")
    }

    fn generic_type_identity(&self) -> u64 {
        self.generic_type_record_identity
            .expect("active generic type stage has a cached type identity")
    }

    fn death_target_identity(&self) -> u64 {
        self.death_target_allocation_identity
            .expect("active death stage has a cached target identity")
    }

    fn death_type_identity(&self) -> u64 {
        self.death_type_record_identity
            .expect("active death type stage has a cached type identity")
    }

    fn local_player_type_at_generic_call(&self) -> u8 {
        self.local_player_type_at_generic_call
            .expect("generic entry captured the fourth argument")
    }

    fn current_delivery_record(&self) -> DamageDeliveryRecord {
        self.delivery
            .expect("active checked-damage path retains a non-null delivery")
    }

    fn current_generic_type_runtime_value(&self) -> RetailRuntimeValue<Option<u64>> {
        self.generic_type_record_runtime
    }
}

fn filter_checked_damage(
    delivery: DamageDeliveryRecord,
    profile: DamageProfile,
    ratio_numerator: u32,
    ratio_denominator: u16,
) -> Result<i32, CheckedDamageBlock> {
    for (slot, channel) in delivery.packet.channels.into_iter().enumerate() {
        if !(0..DAMAGE_CHANNEL_COUNT as i32).contains(&channel) {
            return Err(CheckedDamageBlock::InvalidDamageChannel { slot, channel });
        }
    }
    let filtered = profile.filter(delivery.packet);
    if ratio_denominator == 0 {
        return Ok(filtered);
    }
    Ok(filtered.wrapping_mul((ratio_numerator & 0xffff) as i32) / i32::from(ratio_denominator))
}

fn zero_damage_feedback_enabled(
    admission: CheckedDamageAdmissionTarget,
    delivery: DamageDeliveryRecord,
) -> bool {
    admission.capability_flags_at_0x64 & ZERO_DAMAGE_FEEDBACK_CAPABILITY_BIT != 0
        && delivery.source_entity_type_raw == PLAYER_SOURCE_ENTITY_TYPE_RAW
        && (delivery.packet.channels[0] != 1 || delivery.packet.channels[1] != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::DamagePacket;

    const TARGET_HANDLE: u32 = 0x04b7_0001;
    const ADMISSION_ALLOCATION: u64 = 0x1001;
    const FILTER_ALLOCATION: u64 = 0x2002;
    const FILTER_TYPE: u64 = 0x3003;
    const GENERIC_ALLOCATION: u64 = 0x4004;
    const GENERIC_TYPE: u64 = 0x5005;
    const DEATH_ALLOCATION: u64 = 0x6006;
    const DEATH_TYPE: u64 = 0x7007;

    fn transaction_id(raw: u64) -> CheckedDamageTransactionId {
        CheckedDamageTransactionId::new(raw).unwrap()
    }

    fn identity_profile() -> DamageProfile {
        DamageProfile {
            thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
            multipliers_q8: [256; DAMAGE_CHANNEL_COUNT],
        }
    }

    fn delivery(
        channels: [i32; 2],
        amounts_raw: [i32; 2],
        source_entity_type_raw: u32,
    ) -> DamageDeliveryRecord {
        DamageDeliveryRecord {
            packet: DamagePacket {
                channels,
                amounts_raw,
            },
            source_entity_type_raw,
            owner_handle: 0xaabb_ccdd,
        }
    }

    fn request_with(
        delivery: Option<DamageDeliveryRecord>,
        modifier_address: RetailRuntimeValue<Option<u32>>,
    ) -> CheckedDamageRequest {
        CheckedDamageRequest {
            target_handle: TARGET_HANDLE,
            delivery,
            ratio_numerator: 0,
            ratio_denominator: 0,
            admission_target: RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
                target_allocation_identity: ADMISSION_ALLOCATION,
                state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
                capability_flags_at_0x64: 0,
                modifier_address,
            })),
            filter_binding: RetailRuntimeValue::Known(Some(CheckedDamageFilterBinding {
                target_allocation_identity: FILTER_ALLOCATION,
                type_record_identity: FILTER_TYPE,
                profile: RetailRuntimeValue::Known(identity_profile()),
            })),
        }
    }

    fn normal_request(amount_raw: i32) -> CheckedDamageRequest {
        request_with(
            Some(delivery(
                [1, 0],
                [amount_raw, 0],
                PLAYER_SOURCE_ENTITY_TYPE_RAW,
            )),
            RetailRuntimeValue::Known(None),
        )
    }

    fn machine(request: CheckedDamageRequest) -> CheckedDamageMachine {
        CheckedDamageMachine::start(transaction_id(1), request)
    }

    fn issue(machine: &mut CheckedDamageMachine) -> IssuedCheckedDamageAction {
        match machine.poll() {
            CheckedDamagePoll::Action(issued) => issued,
            other => panic!("expected issued action, got {other:?}"),
        }
    }

    fn resume(
        machine: &mut CheckedDamageMachine,
        issued: IssuedCheckedDamageAction,
        completion: CheckedDamageResume,
    ) {
        machine.resume(issued.receipt, completion).unwrap();
    }

    fn acknowledge(machine: &mut CheckedDamageMachine, issued: IssuedCheckedDamageAction) {
        let phase = issued.action.phase();
        resume(machine, issued, CheckedDamageResume::Acknowledged { phase });
    }

    fn assert_complete(machine: &mut CheckedDamageMachine, expected: CheckedDamageCompletion) {
        assert_eq!(machine.poll(), CheckedDamagePoll::Complete(expected));
    }

    fn resolve_generic(
        machine: &mut CheckedDamageMachine,
        target: RetailRuntimeValue<Option<GenericDamageEntryTarget>>,
        type_record_identity: RetailRuntimeValue<Option<u64>>,
        local_player_type_at_call: RetailRuntimeValue<u8>,
    ) {
        let issued = issue(machine);
        assert_eq!(
            issued.action,
            CheckedDamageAction::ResolveGenericDamageEntry {
                phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                target_handle: TARGET_HANDLE,
            }
        );
        resume(
            machine,
            issued,
            CheckedDamageResume::GenericDamageEntryResolved {
                phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                observation: GenericDamageEntryObservation {
                    target,
                    type_record_identity,
                    local_player_type_at_call,
                },
            },
        );
    }

    fn sample_no_generic_sound(machine: &mut CheckedDamageMachine) {
        let issued = issue(machine);
        assert!(matches!(
            issued.action,
            CheckedDamageAction::SampleCachedGenericHitSound {
                phase: CheckedDamagePhase::SampleGenericHitSound,
                target_allocation_identity: GENERIC_ALLOCATION,
                type_record_identity: GENERIC_TYPE,
                ..
            }
        ));
        resume(
            machine,
            issued,
            CheckedDamageResume::GenericHitSoundSampled {
                phase: CheckedDamagePhase::SampleGenericHitSound,
                observation: RetailRuntimeValue::Known(Some(GenericHitSoundObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    type_record_identity: GENERIC_TYPE,
                    position_raw: [1, 2, 3],
                    sound_id: RetailRuntimeValue::Known(None),
                })),
            },
        );
    }

    fn sample_no_generic_callback(machine: &mut CheckedDamageMachine) {
        let issued = issue(machine);
        assert!(matches!(
            issued.action,
            CheckedDamageAction::SampleCachedGenericHitCallback {
                phase: CheckedDamagePhase::SampleGenericHitCallback,
                type_record_identity: GENERIC_TYPE,
                ..
            }
        ));
        resume(
            machine,
            issued,
            CheckedDamageResume::GenericHitCallbackSampled {
                phase: CheckedDamagePhase::SampleGenericHitCallback,
                observation: RetailRuntimeValue::Known(Some(GenericHitCallbackObservation {
                    type_record_identity: GENERIC_TYPE,
                    callback_address: RetailRuntimeValue::Known(None),
                })),
            },
        );
    }

    fn sample_generic_health(
        machine: &mut CheckedDamageMachine,
        health_raw: i32,
    ) -> GenericHealthCommit {
        let issued = issue(machine);
        assert!(matches!(
            issued.action,
            CheckedDamageAction::SampleCachedGenericHealth {
                phase: CheckedDamagePhase::SampleGenericHealth,
                target_allocation_identity: GENERIC_ALLOCATION,
                ..
            }
        ));
        resume(
            machine,
            issued,
            CheckedDamageResume::GenericHealthSampled {
                phase: CheckedDamagePhase::SampleGenericHealth,
                observation: RetailRuntimeValue::Known(Some(GenericHealthObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    health_raw,
                })),
            },
        );
        let issued = issue(machine);
        let commit = match &issued.action {
            CheckedDamageAction::CommitGenericHealth { commit, .. } => *commit,
            other => panic!("expected generic-health commit, got {other:?}"),
        };
        resume(
            machine,
            issued,
            CheckedDamageResume::GenericHealthCommitted {
                phase: CheckedDamagePhase::CommitGenericHealth,
                observation: RetailRuntimeValue::Known(Some(CachedTargetWriteObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                })),
            },
        );
        commit
    }

    fn drive_to_death_entry(
        machine: &mut CheckedDamageMachine,
        health_raw: i32,
    ) -> IssuedCheckedDamageAction {
        resolve_generic(
            machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
                pre_health_buffer_raw: 0,
            })),
            RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
            RetailRuntimeValue::Known(0x22),
        );
        sample_no_generic_sound(machine);
        sample_no_generic_callback(machine);
        let commit = sample_generic_health(machine, health_raw);
        assert!(commit.health_after_raw <= 0);
        let issued = issue(machine);
        assert!(matches!(
            issued.action,
            CheckedDamageAction::ResolveDeathEntry {
                phase: CheckedDamagePhase::ResolveDeathEntry,
                ..
            }
        ));
        issued
    }

    #[test]
    fn admission_gates_do_not_demand_later_evidence() {
        let mut missing = normal_request(10);
        missing.admission_target = RetailRuntimeValue::Known(None);
        missing.filter_binding = RetailRuntimeValue::Unresolved;
        assert_complete(
            &mut machine(missing),
            CheckedDamageCompletion::TargetMissing,
        );

        let mut disabled = normal_request(10);
        disabled.admission_target = RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
            target_allocation_identity: ADMISSION_ALLOCATION,
            state_flags_at_0x08: 0,
            capability_flags_at_0x64: 0,
            modifier_address: RetailRuntimeValue::Unresolved,
        }));
        disabled.filter_binding = RetailRuntimeValue::Unresolved;
        assert_complete(
            &mut machine(disabled),
            CheckedDamageCompletion::ReturnedZero(CheckedDamageZeroReason::AdmissionDisabled),
        );

        let mut null_delivery = normal_request(10);
        null_delivery.delivery = None;
        null_delivery.filter_binding = RetailRuntimeValue::Unresolved;
        assert_complete(
            &mut machine(null_delivery),
            CheckedDamageCompletion::ReturnedZero(CheckedDamageZeroReason::NullDelivery),
        );

        let mut unresolved = normal_request(10);
        unresolved.admission_target = RetailRuntimeValue::Unresolved;
        assert_eq!(
            machine(unresolved).poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::UnresolvedAdmissionTarget,)
        );
    }

    #[test]
    fn fresh_filter_binding_may_differ_and_ratio_uses_low_sixteen_bits() {
        let mut request = normal_request(30);
        request.ratio_numerator = 0xabcd_0003;
        request.ratio_denominator = 2;
        let mut machine = machine(request);
        assert_eq!(machine.filtered_damage_raw(), 45);
        assert_eq!(machine.accepted_damage_raw(), 45);
        assert!(matches!(
            machine.poll(),
            CheckedDamagePoll::Action(IssuedCheckedDamageAction {
                action: CheckedDamageAction::ResolveGenericDamageEntry { .. },
                ..
            })
        ));
    }

    #[test]
    fn missing_profile_and_invalid_channel_block_instead_of_guessing() {
        let mut missing = normal_request(10);
        missing.filter_binding = RetailRuntimeValue::Known(None);
        assert_eq!(
            machine(missing).poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::MissingFilterBinding)
        );

        let mut unresolved = normal_request(10);
        unresolved.filter_binding = RetailRuntimeValue::Known(Some(CheckedDamageFilterBinding {
            target_allocation_identity: FILTER_ALLOCATION,
            type_record_identity: FILTER_TYPE,
            profile: RetailRuntimeValue::Unresolved,
        }));
        assert_eq!(
            machine(unresolved).poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::UnresolvedDamageProfile,)
        );

        let invalid = request_with(
            Some(delivery(
                [DAMAGE_CHANNEL_COUNT as i32, 0],
                [10, 0],
                PLAYER_SOURCE_ENTITY_TYPE_RAW,
            )),
            RetailRuntimeValue::Known(None),
        );
        assert_eq!(
            machine(invalid).poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::InvalidDamageChannel {
                slot: 0,
                channel: DAMAGE_CHANNEL_COUNT as i32,
            },)
        );
    }

    #[test]
    fn filter_zero_feedback_has_exact_gate_and_order() {
        let mut request = request_with(
            Some(delivery([2, 0], [10, 0], PLAYER_SOURCE_ENTITY_TYPE_RAW)),
            RetailRuntimeValue::Unresolved,
        );
        request.admission_target = RetailRuntimeValue::Known(Some(CheckedDamageAdmissionTarget {
            target_allocation_identity: ADMISSION_ALLOCATION,
            state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
            capability_flags_at_0x64: ZERO_DAMAGE_FEEDBACK_CAPABILITY_BIT,
            modifier_address: RetailRuntimeValue::Unresolved,
        }));
        request.filter_binding = RetailRuntimeValue::Known(Some(CheckedDamageFilterBinding {
            target_allocation_identity: FILTER_ALLOCATION,
            type_record_identity: FILTER_TYPE,
            profile: RetailRuntimeValue::Known(DamageProfile {
                thresholds_raw: [0; DAMAGE_CHANNEL_COUNT],
                multipliers_q8: [0; DAMAGE_CHANNEL_COUNT],
            }),
        }));

        let mut transaction = machine(request);
        let selector = issue(&mut transaction);
        assert_eq!(
            selector.action,
            CheckedDamageAction::SelectFeedback {
                phase: CheckedDamagePhase::SelectZeroDamageFeedback,
                selector: ZERO_DAMAGE_FEEDBACK_SELECTOR,
            }
        );
        acknowledge(&mut transaction, selector);
        let sound = issue(&mut transaction);
        assert_eq!(
            sound.action,
            CheckedDamageAction::PlayNonPositionalSound {
                phase: CheckedDamagePhase::PlayZeroDamageFeedback,
                sound_id: ZERO_DAMAGE_FEEDBACK_SOUND_ID,
            }
        );
        acknowledge(&mut transaction, sound);
        assert_complete(
            &mut transaction,
            CheckedDamageCompletion::ReturnedZero(CheckedDamageZeroReason::FilteredOut),
        );

        let mut suppressed = request;
        suppressed.delivery = Some(delivery([1, 0], [10, 0], PLAYER_SOURCE_ENTITY_TYPE_RAW));
        assert_complete(
            &mut machine(suppressed),
            CheckedDamageCompletion::ReturnedZero(CheckedDamageZeroReason::FilteredOut),
        );
    }

    #[test]
    fn modifier_zero_still_runs_generic_callback_and_refreshes_health() {
        let mut machine = machine(request_with(
            Some(delivery([1, 0], [50, 0], PLAYER_SOURCE_ENTITY_TYPE_RAW)),
            RetailRuntimeValue::Known(Some(0x0042_1234)),
        ));
        let modifier = issue(&mut machine);
        assert!(matches!(
            modifier.action,
            CheckedDamageAction::InvokeDamageModifier {
                filtered_damage_raw: 50,
                ..
            }
        ));
        let mutated_delivery = delivery([6, 5], [77, 88], 0x66);
        resume(
            &mut machine,
            modifier,
            CheckedDamageResume::ModifierReturned {
                phase: CheckedDamagePhase::DamageModifier,
                damage_raw: 0,
                delivery_after_callback: mutated_delivery,
            },
        );
        assert_eq!(machine.current_delivery(), Some(mutated_delivery));

        resolve_generic(
            &mut machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: CHECKED_DAMAGE_ENABLED_STATE_BIT,
                pre_health_buffer_raw: 0,
            })),
            RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
            RetailRuntimeValue::Known(0x44),
        );
        sample_no_generic_sound(&mut machine);

        let callback_sample = issue(&mut machine);
        resume(
            &mut machine,
            callback_sample,
            CheckedDamageResume::GenericHitCallbackSampled {
                phase: CheckedDamagePhase::SampleGenericHitCallback,
                observation: RetailRuntimeValue::Known(Some(GenericHitCallbackObservation {
                    type_record_identity: GENERIC_TYPE,
                    callback_address: RetailRuntimeValue::Known(Some(0x0040_9999)),
                })),
            },
        );
        let callback = issue(&mut machine);
        assert_eq!(
            callback.action,
            CheckedDamageAction::InvokeGenericHitCallback {
                phase: CheckedDamagePhase::GenericHitCallback,
                callback_address: 0x0040_9999,
                target_handle: TARGET_HANDLE,
                damage_after_buffer_raw: 0,
            }
        );
        acknowledge(&mut machine, callback);

        // The callback-owned value is sampled afterward and becomes the exact
        // basis of the cached health write.
        let commit = sample_generic_health(&mut machine, 321);
        assert_eq!(commit.health_before_raw, 321);
        assert_eq!(commit.damage_after_buffer_raw, 0);
        assert_eq!(commit.health_after_raw, 321);

        let tail = issue(&mut machine);
        resume(
            &mut machine,
            tail,
            CheckedDamageResume::SurvivorNetworkTailSampled {
                phase: CheckedDamagePhase::SampleSurvivorNetworkTail,
                observation: RetailRuntimeValue::Known(Some(SurvivorNetworkTailObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    capability_flags_at_0x64: DAMAGE_NETWORK_CAPABILITY_BIT,
                    network_session_active: true,
                    current_local_player_type: RetailRuntimeValue::Unresolved,
                })),
            },
        );
        // Packet dword four changed away from player provenance, so retail
        // does not require the current-local-type byte or send a survivor
        // packet.
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedZero(CheckedDamageZeroReason::ModifierReturnedZero),
        );
    }

    #[test]
    fn modifier_high_bit_return_and_mutated_provenance_survive_remote_path() {
        let mut machine = machine(request_with(
            Some(delivery([1, 0], [1, 0], PLAYER_SOURCE_ENTITY_TYPE_RAW)),
            RetailRuntimeValue::Known(Some(0x0042_2222)),
        ));
        let modifier = issue(&mut machine);
        let mutated = delivery([3, 4], [5, 6], 0x1234_5678);
        resume(
            &mut machine,
            modifier,
            CheckedDamageResume::ModifierReturned {
                phase: CheckedDamagePhase::DamageModifier,
                damage_raw: i32::MIN,
                delivery_after_callback: mutated,
            },
        );
        resolve_generic(
            &mut machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: REMOTE_OWNED_STATE_BIT,
                pre_health_buffer_raw: 99,
            })),
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Known(0x22),
        );
        let remote = issue(&mut machine);
        assert_eq!(
            remote.action,
            CheckedDamageAction::ForwardRemoteDamage {
                phase: CheckedDamagePhase::ForwardRemoteDamage,
                request: RemoteDamageForwardRequest {
                    target_handle: TARGET_HANDLE,
                    damage_low_u16: 0,
                    source_entity_type_raw: 0x1234_5678,
                },
            }
        );
        acknowledge(&mut machine, remote);
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(i32::MIN).unwrap()),
        );
    }

    #[test]
    fn positive_buffer_commits_before_dying_gate_and_type_dereference() {
        let mut machine = machine(normal_request(10));
        resolve_generic(
            &mut machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: DYING_STATE_BIT,
                pre_health_buffer_raw: 50,
            })),
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Known(0x22),
        );
        let commit = issue(&mut machine);
        assert_eq!(
            commit.action,
            CheckedDamageAction::CommitPreHealthBuffer {
                phase: CheckedDamagePhase::CommitPreHealthBuffer,
                commit: PreHealthBufferCommit {
                    target_handle: TARGET_HANDLE,
                    target_allocation_identity: GENERIC_ALLOCATION,
                    before_raw: 50,
                    after_raw: 40,
                    residual_damage_raw: 0,
                },
            }
        );
        resume(
            &mut machine,
            commit,
            CheckedDamageResume::PreHealthBufferCommitted {
                phase: CheckedDamagePhase::CommitPreHealthBuffer,
                target: RetailRuntimeValue::Known(Some(GenericTargetStateSample {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    state_flags_at_0x08: DYING_STATE_BIT,
                })),
            },
        );
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(10).unwrap()),
        );
    }

    #[test]
    fn buffer_sound_callback_then_health_follow_retail_order() {
        let mut machine = machine(normal_request(100));
        resolve_generic(
            &mut machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: 0,
                pre_health_buffer_raw: 30,
            })),
            RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
            RetailRuntimeValue::Known(0x22),
        );
        let buffer = issue(&mut machine);
        assert!(matches!(
            buffer.action,
            CheckedDamageAction::CommitPreHealthBuffer {
                commit: PreHealthBufferCommit {
                    after_raw: 0,
                    residual_damage_raw: 70,
                    ..
                },
                ..
            }
        ));
        resume(
            &mut machine,
            buffer,
            CheckedDamageResume::PreHealthBufferCommitted {
                phase: CheckedDamagePhase::CommitPreHealthBuffer,
                target: RetailRuntimeValue::Known(Some(GenericTargetStateSample {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    state_flags_at_0x08: 0,
                })),
            },
        );

        let sound_sample = issue(&mut machine);
        resume(
            &mut machine,
            sound_sample,
            CheckedDamageResume::GenericHitSoundSampled {
                phase: CheckedDamagePhase::SampleGenericHitSound,
                observation: RetailRuntimeValue::Known(Some(GenericHitSoundObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    type_record_identity: GENERIC_TYPE,
                    position_raw: [7, 8, 9],
                    sound_id: RetailRuntimeValue::Known(Some(0x98)),
                })),
            },
        );
        let sound = issue(&mut machine);
        assert_eq!(
            sound.action,
            CheckedDamageAction::PlayPositionalSound {
                phase: CheckedDamagePhase::PlayGenericHitSound,
                sound_id: 0x98,
                position_raw: [7, 8, 9],
            }
        );
        acknowledge(&mut machine, sound);

        let callback_sample = issue(&mut machine);
        resume(
            &mut machine,
            callback_sample,
            CheckedDamageResume::GenericHitCallbackSampled {
                phase: CheckedDamagePhase::SampleGenericHitCallback,
                observation: RetailRuntimeValue::Known(Some(GenericHitCallbackObservation {
                    type_record_identity: GENERIC_TYPE,
                    callback_address: RetailRuntimeValue::Known(Some(0x0040_aaaa)),
                })),
            },
        );
        let callback = issue(&mut machine);
        assert!(matches!(
            callback.action,
            CheckedDamageAction::InvokeGenericHitCallback {
                damage_after_buffer_raw: 70,
                ..
            }
        ));
        acknowledge(&mut machine, callback);

        let commit = sample_generic_health(&mut machine, 50);
        assert_eq!(commit.health_before_raw, 50);
        assert_eq!(commit.health_after_raw, -20);
        assert!(matches!(
            machine.poll(),
            CheckedDamagePoll::Action(IssuedCheckedDamageAction {
                action: CheckedDamageAction::ResolveDeathEntry { .. },
                ..
            })
        ));
    }

    #[test]
    fn survivor_network_uses_current_and_call_entry_player_bytes() {
        let mut machine = machine(normal_request(10));
        resolve_generic(
            &mut machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: 0,
                pre_health_buffer_raw: 0,
            })),
            RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
            RetailRuntimeValue::Known(0xaa),
        );
        sample_no_generic_sound(&mut machine);
        sample_no_generic_callback(&mut machine);
        let commit = sample_generic_health(&mut machine, 100);
        assert_eq!(commit.health_after_raw, 90);
        let tail = issue(&mut machine);
        resume(
            &mut machine,
            tail,
            CheckedDamageResume::SurvivorNetworkTailSampled {
                phase: CheckedDamagePhase::SampleSurvivorNetworkTail,
                observation: RetailRuntimeValue::Known(Some(SurvivorNetworkTailObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    capability_flags_at_0x64: DAMAGE_NETWORK_CAPABILITY_BIT,
                    network_session_active: true,
                    current_local_player_type: RetailRuntimeValue::Known(0xbb),
                })),
            },
        );
        let network = issue(&mut machine);
        assert_eq!(
            network.action,
            CheckedDamageAction::SubmitSurvivorNetwork {
                phase: CheckedDamagePhase::SubmitSurvivorNetwork,
                request: DamageLifecycleNetworkRequest {
                    current_local_player_type_raw: 0xbb,
                    source_local_player_type_raw: 0xaa,
                },
            }
        );
        acknowledge(&mut machine, network);
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(10).unwrap()),
        );
    }

    #[test]
    fn full_death_path_uses_fresh_binding_then_original_cached_suffix() {
        let mut machine = machine(normal_request(10));
        let death_entry = drive_to_death_entry(&mut machine, 5);
        resume(
            &mut machine,
            death_entry,
            CheckedDamageResume::DeathEntryResolved {
                phase: CheckedDamagePhase::ResolveDeathEntry,
                observation: DeathEntryObservation {
                    target: RetailRuntimeValue::Known(Some(DeathEntryTarget {
                        // Fresh death binding deliberately differs from
                        // the original generic cached allocation.
                        target_allocation_identity: DEATH_ALLOCATION,
                        state_flags_at_0x08: 0x80,
                    })),
                    type_record_identity: RetailRuntimeValue::Known(Some(DEATH_TYPE)),
                },
            },
        );

        let commit_death = issue(&mut machine);
        assert_eq!(
            commit_death.action,
            CheckedDamageAction::CommitDeathState {
                phase: CheckedDamagePhase::CommitDeathState,
                target_handle: TARGET_HANDLE,
                target_allocation_identity: DEATH_ALLOCATION,
                state_flags_before: 0x80,
                state_flags_after: 0x80 | DYING_STATE_BIT,
                health_after_raw: 0,
            }
        );
        resume(
            &mut machine,
            commit_death,
            CheckedDamageResume::DeathStateCommitted {
                phase: CheckedDamagePhase::CommitDeathState,
                observation: RetailRuntimeValue::Known(Some(CachedTargetWriteObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                })),
            },
        );

        let death_sound_sample = issue(&mut machine);
        resume(
            &mut machine,
            death_sound_sample,
            CheckedDamageResume::DeathSoundSampled {
                phase: CheckedDamagePhase::SampleDeathSound,
                observation: RetailRuntimeValue::Known(Some(DeathSoundObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                    type_record_identity: DEATH_TYPE,
                    position_raw: [10, 11, 12],
                    sound_id: RetailRuntimeValue::Known(Some(0x90)),
                })),
            },
        );
        let death_sound = issue(&mut machine);
        assert!(matches!(
            death_sound.action,
            CheckedDamageAction::PlayPositionalSound {
                phase: CheckedDamagePhase::PlayDeathSound,
                sound_id: 0x90,
                ..
            }
        ));
        acknowledge(&mut machine, death_sound);

        let attachment_sample = issue(&mut machine);
        resume(
            &mut machine,
            attachment_sample,
            CheckedDamageResume::DeathAttachmentSampled {
                phase: CheckedDamagePhase::SampleDeathAttachment,
                observation: RetailRuntimeValue::Known(Some(DeathAttachmentObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                    attached_resource_identity: RetailRuntimeValue::Known(Some(0x8888)),
                })),
            },
        );
        let release = issue(&mut machine);
        assert_eq!(
            release.action,
            CheckedDamageAction::ReleaseDeathAttachment {
                phase: CheckedDamagePhase::ReleaseDeathAttachment,
                attached_resource_identity: 0x8888,
            }
        );
        acknowledge(&mut machine, release);
        let clear = issue(&mut machine);
        assert!(matches!(
            clear.action,
            CheckedDamageAction::CommitDeathAttachmentClear {
                target_allocation_identity: DEATH_ALLOCATION,
                attached_resource_identity: 0x8888,
                ..
            }
        ));
        resume(
            &mut machine,
            clear,
            CheckedDamageResume::DeathAttachmentCleared {
                phase: CheckedDamagePhase::CommitDeathAttachmentClear,
                observation: RetailRuntimeValue::Known(Some(CachedTargetWriteObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                })),
            },
        );

        let callback_sample = issue(&mut machine);
        resume(
            &mut machine,
            callback_sample,
            CheckedDamageResume::DeathCallbackSampled {
                phase: CheckedDamagePhase::SampleDeathCallback,
                observation: RetailRuntimeValue::Known(Some(DeathCallbackObservation {
                    type_record_identity: DEATH_TYPE,
                    callback_address: RetailRuntimeValue::Known(Some(0x0040_bbbb)),
                })),
            },
        );
        let callback = issue(&mut machine);
        assert_eq!(
            callback.action,
            CheckedDamageAction::InvokeDeathCallback {
                phase: CheckedDamagePhase::DeathCallback,
                callback_address: 0x0040_bbbb,
                target_handle: TARGET_HANDLE,
            }
        );
        acknowledge(&mut machine, callback);
        let post_callback = issue(&mut machine);
        resume(
            &mut machine,
            post_callback,
            CheckedDamageResume::DeathTargetAfterCallbackSampled {
                phase: CheckedDamagePhase::SampleDeathTargetAfterCallback,
                observation: RetailRuntimeValue::Known(Some(DeathTargetAfterCallbackObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                    state_flags_at_0x08: DYING_STATE_BIT,
                    capability_flags_at_0x64: MANAGED_DEATH_CAPABILITY_BIT,
                })),
            },
        );
        let finalizer = issue(&mut machine);
        assert_eq!(
            finalizer.action,
            CheckedDamageAction::FinalizeManagedDeath {
                phase: CheckedDamagePhase::FinalizeManagedDeath,
                fallback_sound_id: MANAGED_DEATH_FALLBACK_SOUND_ID,
            }
        );
        acknowledge(&mut machine, finalizer);

        let original = issue(&mut machine);
        assert!(matches!(
            original.action,
            CheckedDamageAction::SampleOriginalTargetAfterDeath {
                target_allocation_identity: GENERIC_ALLOCATION,
                ..
            }
        ));
        resume(
            &mut machine,
            original,
            CheckedDamageResume::OriginalTargetAfterDeathSampled {
                phase: CheckedDamagePhase::SampleOriginalTargetAfterDeath,
                observation: RetailRuntimeValue::Known(Some(OriginalTargetAfterDeathObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    state_flags_at_0x08: PLAYER_KILL_FEEDBACK_STATE_BIT,
                })),
            },
        );
        let selector = issue(&mut machine);
        assert_eq!(
            selector.action,
            CheckedDamageAction::SelectFeedback {
                phase: CheckedDamagePhase::SelectPlayerKillFeedback,
                selector: PLAYER_KILL_FEEDBACK_SELECTOR,
            }
        );
        acknowledge(&mut machine, selector);

        let network_tail = issue(&mut machine);
        resume(
            &mut machine,
            network_tail,
            CheckedDamageResume::DeathNetworkTailSampled {
                phase: CheckedDamagePhase::SampleDeathNetworkTail,
                observation: RetailRuntimeValue::Known(Some(DeathNetworkTailObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    capability_flags_at_0x64: DAMAGE_NETWORK_CAPABILITY_BIT,
                    network_session_active: true,
                    current_local_player_type: RetailRuntimeValue::Known(0xcc),
                })),
            },
        );
        let death_network = issue(&mut machine);
        assert_eq!(
            death_network.action,
            CheckedDamageAction::SubmitDeathNetwork {
                phase: CheckedDamagePhase::SubmitDeathNetwork,
                request: DamageLifecycleNetworkRequest {
                    current_local_player_type_raw: 0xcc,
                    source_local_player_type_raw: 0x22,
                },
            }
        );
        acknowledge(&mut machine, death_network);
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(10).unwrap()),
        );
    }

    #[test]
    fn death_callback_can_restore_target_and_skip_original_suffix() {
        let mut machine = machine(normal_request(10));
        let death_entry = drive_to_death_entry(&mut machine, 5);
        resume(
            &mut machine,
            death_entry,
            CheckedDamageResume::DeathEntryResolved {
                phase: CheckedDamagePhase::ResolveDeathEntry,
                observation: DeathEntryObservation {
                    target: RetailRuntimeValue::Known(Some(DeathEntryTarget {
                        target_allocation_identity: DEATH_ALLOCATION,
                        state_flags_at_0x08: 0,
                    })),
                    type_record_identity: RetailRuntimeValue::Known(Some(DEATH_TYPE)),
                },
            },
        );
        let commit = issue(&mut machine);
        resume(
            &mut machine,
            commit,
            CheckedDamageResume::DeathStateCommitted {
                phase: CheckedDamagePhase::CommitDeathState,
                observation: RetailRuntimeValue::Known(Some(CachedTargetWriteObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                })),
            },
        );
        let sound = issue(&mut machine);
        resume(
            &mut machine,
            sound,
            CheckedDamageResume::DeathSoundSampled {
                phase: CheckedDamagePhase::SampleDeathSound,
                observation: RetailRuntimeValue::Known(Some(DeathSoundObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                    type_record_identity: DEATH_TYPE,
                    position_raw: [0; 3],
                    sound_id: RetailRuntimeValue::Known(None),
                })),
            },
        );
        let attachment = issue(&mut machine);
        resume(
            &mut machine,
            attachment,
            CheckedDamageResume::DeathAttachmentSampled {
                phase: CheckedDamagePhase::SampleDeathAttachment,
                observation: RetailRuntimeValue::Known(Some(DeathAttachmentObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                    attached_resource_identity: RetailRuntimeValue::Known(None),
                })),
            },
        );
        let callback_sample = issue(&mut machine);
        resume(
            &mut machine,
            callback_sample,
            CheckedDamageResume::DeathCallbackSampled {
                phase: CheckedDamagePhase::SampleDeathCallback,
                observation: RetailRuntimeValue::Known(Some(DeathCallbackObservation {
                    type_record_identity: DEATH_TYPE,
                    callback_address: RetailRuntimeValue::Known(Some(0x0040_cccc)),
                })),
            },
        );
        let callback = issue(&mut machine);
        acknowledge(&mut machine, callback);
        let post = issue(&mut machine);
        resume(
            &mut machine,
            post,
            CheckedDamageResume::DeathTargetAfterCallbackSampled {
                phase: CheckedDamagePhase::SampleDeathTargetAfterCallback,
                observation: RetailRuntimeValue::Known(Some(DeathTargetAfterCallbackObservation {
                    target_allocation_identity: DEATH_ALLOCATION,
                    state_flags_at_0x08: 0,
                    capability_flags_at_0x64: MANAGED_DEATH_CAPABILITY_BIT,
                })),
            },
        );
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(10).unwrap()),
        );
    }

    #[test]
    fn successful_death_suffix_rejects_reused_original_allocation() {
        let mut machine = machine(normal_request(10));
        let death_entry = drive_to_death_entry(&mut machine, 5);
        // A missing death target makes FUN_00410C10 return one.
        resume(
            &mut machine,
            death_entry,
            CheckedDamageResume::DeathEntryResolved {
                phase: CheckedDamagePhase::ResolveDeathEntry,
                observation: DeathEntryObservation {
                    target: RetailRuntimeValue::Known(None),
                    type_record_identity: RetailRuntimeValue::Unresolved,
                },
            },
        );
        let sample = issue(&mut machine);
        resume(
            &mut machine,
            sample,
            CheckedDamageResume::OriginalTargetAfterDeathSampled {
                phase: CheckedDamagePhase::SampleOriginalTargetAfterDeath,
                observation: RetailRuntimeValue::Known(Some(OriginalTargetAfterDeathObservation {
                    target_allocation_identity: GENERIC_ALLOCATION + 1,
                    state_flags_at_0x08: 0,
                })),
            },
        );
        assert_eq!(
            machine.poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::OriginalGenericTargetIdentityMismatch {
                expected: GENERIC_ALLOCATION,
                actual: GENERIC_ALLOCATION + 1,
            },)
        );
    }

    #[test]
    fn cached_generic_type_reuse_blocks_at_first_dereference() {
        let mut machine = machine(normal_request(10));
        resolve_generic(
            &mut machine,
            RetailRuntimeValue::Known(Some(GenericDamageEntryTarget {
                target_allocation_identity: GENERIC_ALLOCATION,
                state_flags_at_0x08: 0,
                pre_health_buffer_raw: 0,
            })),
            RetailRuntimeValue::Known(Some(GENERIC_TYPE)),
            RetailRuntimeValue::Known(0x22),
        );
        let sample = issue(&mut machine);
        resume(
            &mut machine,
            sample,
            CheckedDamageResume::GenericHitSoundSampled {
                phase: CheckedDamagePhase::SampleGenericHitSound,
                observation: RetailRuntimeValue::Known(Some(GenericHitSoundObservation {
                    target_allocation_identity: GENERIC_ALLOCATION,
                    type_record_identity: GENERIC_TYPE + 1,
                    position_raw: [0; 3],
                    sound_id: RetailRuntimeValue::Known(None),
                })),
            },
        );
        assert_eq!(
            machine.poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::CachedGenericTypeIdentityMismatch {
                phase: CheckedDamagePhase::SampleGenericHitSound,
                expected: GENERIC_TYPE,
                actual: GENERIC_TYPE + 1,
            },)
        );
    }

    #[test]
    fn receipts_are_linear_recoverable_and_external_blocks_are_durable() {
        let mut machine = machine(normal_request(10));
        let issued = issue(&mut machine);
        let transaction = issued.receipt.transaction_id();
        let sequence = issued.receipt.action_sequence();
        assert_eq!(
            machine.poll(),
            CheckedDamagePoll::Awaiting(CheckedDamagePhase::ResolveGenericDamageEntry,)
        );

        let failure = machine
            .resume(
                issued.receipt,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            CheckedDamageProtocolError::UnexpectedResumeKind {
                phase: CheckedDamagePhase::ResolveGenericDamageEntry,
            }
        );
        assert_eq!(failure.receipt.transaction_id(), transaction);
        assert_eq!(failure.receipt.action_sequence(), sequence);

        machine
            .resume(
                failure.receipt,
                CheckedDamageResume::Blocked {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    reason: CheckedDamageExternalBlock::RuntimeReadUnavailable,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            CheckedDamagePoll::Blocked(CheckedDamageBlock::External {
                phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                reason: CheckedDamageExternalBlock::RuntimeReadUnavailable,
            })
        );
    }

    #[test]
    fn wrong_phase_returns_receipt_without_advancing() {
        let mut machine = machine(normal_request(10));
        let issued = issue(&mut machine);
        let failure = machine
            .resume(
                issued.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveDeathEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            CheckedDamageProtocolError::PhaseMismatch {
                expected: CheckedDamagePhase::ResolveGenericDamageEntry,
                actual: CheckedDamagePhase::ResolveDeathEntry,
            }
        );
        machine
            .resume(
                failure.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap();
        assert_complete(
            &mut machine,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(10).unwrap()),
        );
    }

    #[test]
    fn cross_machine_and_wrong_sequence_receipts_are_returned() {
        let mut first = CheckedDamageMachine::start(transaction_id(1), normal_request(10));
        let mut second = CheckedDamageMachine::start(transaction_id(2), normal_request(10));
        let first_action = issue(&mut first);
        let second_action = issue(&mut second);

        let failure = second
            .resume(
                first_action.receipt,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            CheckedDamageProtocolError::ReceiptTransactionMismatch {
                expected: transaction_id(2),
                actual: transaction_id(1),
            }
        );

        let wrong_sequence = CheckedDamageReceipt {
            transaction_id: transaction_id(2),
            action_sequence: second_action.receipt.action_sequence() + 1,
        };
        let failure = second
            .resume(
                wrong_sequence,
                CheckedDamageResume::GenericDamageEntryResolved {
                    phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                    observation: GenericDamageEntryObservation {
                        target: RetailRuntimeValue::Known(None),
                        type_record_identity: RetailRuntimeValue::Known(None),
                        local_player_type_at_call: RetailRuntimeValue::Known(0),
                    },
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            CheckedDamageProtocolError::ReceiptSequenceMismatch {
                expected: second_action.receipt.action_sequence(),
                actual: second_action.receipt.action_sequence() + 1,
            }
        );

        // The valid second-machine receipt remains accepted after both
        // recoverable rejections.
        resume(
            &mut second,
            second_action,
            CheckedDamageResume::GenericDamageEntryResolved {
                phase: CheckedDamagePhase::ResolveGenericDamageEntry,
                observation: GenericDamageEntryObservation {
                    target: RetailRuntimeValue::Known(None),
                    type_record_identity: RetailRuntimeValue::Known(None),
                    local_player_type_at_call: RetailRuntimeValue::Known(0),
                },
            },
        );
        assert_complete(
            &mut second,
            CheckedDamageCompletion::ReturnedNonZero(NonZeroI32::new(10).unwrap()),
        );
    }

    #[test]
    fn no_outstanding_action_returns_submitted_receipt() {
        let mut terminal_request = normal_request(10);
        terminal_request.admission_target = RetailRuntimeValue::Known(None);
        let mut terminal = CheckedDamageMachine::start(transaction_id(9), terminal_request);
        let receipt = CheckedDamageReceipt {
            transaction_id: transaction_id(9),
            action_sequence: 77,
        };
        let failure = terminal
            .resume(
                receipt,
                CheckedDamageResume::Acknowledged {
                    phase: CheckedDamagePhase::ForwardRemoteDamage,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            CheckedDamageProtocolError::NoOutstandingAction {
                transaction_id: transaction_id(9),
                action_sequence: 77,
            }
        );
        assert_eq!(failure.receipt.action_sequence(), 77);
    }
}
