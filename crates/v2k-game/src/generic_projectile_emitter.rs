//! Detached, resumable common projectile emitter from retail `FUN_00424650`.
//!
//! This is the complete control transaction, not a replacement for the older
//! type-47 presentation policy. Retail mutates cadence and burst state around
//! shared RNG draws, entity lookups, aim/forward-half-space tests, fixed-point launch
//! construction, transient-list appends, joint writes, and audio dispatch.
//! Replaying any of those boundaries after an adapter has committed it changes
//! the world, so this machine issues every action once and waits for an
//! explicitly phase-tagged response carrying its transaction-scoped,
//! single-use receipt.
//!
//! Fixed-point target lead/gravity, body-matrix construction, runtime joint
//! bindings, and the intrusive 0x2C list allocator stay adapter-owned. Their
//! inputs and ordering are proven, but reproducing their internals here would
//! duplicate unresolved runtime state. Missing pointers on retail paths which
//! immediately dereference them become durable blocks instead of guesses or
//! fresh handle rebinding.

use std::num::{NonZeroU32, NonZeroU64};

use v2k_formats::collision::ProjectileEmitterDescriptor;

/// Retail common projectile emitter.
pub const GENERIC_PROJECTILE_EMITTER_ADDRESS: u32 = 0x0042_4650;
/// Transient 0x2C-list append helper.
pub const GENERIC_PROJECTILE_APPEND_ADDRESS: u32 = 0x0041_47A0;
/// Target aim-error helper.
pub const GENERIC_PROJECTILE_AIM_ADDRESS: u32 = 0x0041_E4D0;
/// Target forward-half-space dot-product helper; it does not test terrain occlusion.
pub const GENERIC_PROJECTILE_FORWARD_HALF_SPACE_ADDRESS: u32 = 0x0041_E930;
/// Manual launch matrix helper.
pub const GENERIC_PROJECTILE_MANUAL_BASIS_ADDRESS: u32 = 0x0041_3D40;
/// Runtime A/B joint writer.
pub const GENERIC_PROJECTILE_JOINT_WRITE_ADDRESS: u32 = 0x0042_4EE0;
/// Method-31 rejected-shot effect class.
pub const GENERIC_PROJECTILE_METHOD_31_REJECTION_EFFECT: u32 = 0x1C;
/// Auxiliary command method emitted after a successful primary append.
pub const GENERIC_PROJECTILE_AUXILIARY_METHOD: u32 = 0x0F;
/// Source state bit which permits method 31's direct-mode firing route.
pub const GENERIC_PROJECTILE_METHOD_31_PERMISSION_BIT: u32 = 0x0020_0000;

/// Caller-supplied identity for one detached emitter transaction.
///
/// The host owns allocation and must keep IDs unique among all live machines;
/// do not reuse an ID while a receipt from its previous transaction can still
/// exist. A source handle is not sufficient because two concurrent emitters
/// can share it and later entities can reuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericEmitterTransactionId(NonZeroU64);

impl GenericEmitterTransactionId {
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

/// Entity fields observed at one host-side lookup or cached-pointer refresh.
///
/// `allocation_identity` identifies this allocation, not merely its handle.
/// The machine retains only the entry source's allocation identity; actions at
/// each proven raw dereference point refresh the mutable fields from that same
/// allocation, matching retail's cached-pointer semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterEntitySnapshot {
    pub handle: u32,
    pub allocation_identity: u64,
    pub state_flags_at_0x08: u32,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub orientation_raw: [i16; 3],
}

/// Mutable 0x3C-byte emitter state, represented by the fields read or written
/// by `FUN_00424650`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterRuntime {
    /// Opaque identities of the two optional u16 joint bindings at dwords 0/1.
    pub joint_bindings: [Option<u64>; 2],
    /// Projectile method at dword 4.
    pub projectile_method: u32,
    /// A/B selector at dword 7. Retail normalizes it to 0 or 1 only when the
    /// descriptor asks for alternation.
    pub emitter_selector: u32,
    /// Positional sound id at dword 8.
    pub sound_id: u32,
    /// Zero is target-aim mode; nonzero is direct/manual mode.
    pub direct_mode: u32,
    /// Direct-mode time/budget accumulator at dword 10.
    pub remaining_time_raw: i32,
    /// Direct-mode decrement/catch-up interval at dword 11.
    pub manual_step_raw: i32,
    /// Direct-mode remaining burst count at dword 12.
    pub remaining_bursts_raw: i32,
    /// Target-mode cadence accumulator at dword 13.
    pub cadence_raw: i32,
    /// Optional extra manual-basis transform at dword 14.
    pub basis_adjustment_identity: Option<u64>,
}

impl GenericEmitterRuntime {
    const fn target_mode(self) -> bool {
        self.direct_mode == 0
    }
}

/// Immutable arguments captured at emitter entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterRequest {
    /// Retail `param_1`: the source entity resolved and cached at entry.
    pub source_handle: u32,
    /// Retail `param_2`: the owner copied into each transient command.
    pub owner_handle: u32,
    pub target_handle: u32,
    pub explicit_target_position_raw: Option<[i16; 3]>,
    pub descriptor: ProjectileEmitterDescriptor,
    pub runtime: GenericEmitterRuntime,
    pub elapsed_us: u32,
}

impl
    From<
        crate::aim_and_fire::InvokeGenericEmitter<
            ProjectileEmitterDescriptor,
            GenericEmitterRuntime,
        >,
    > for GenericEmitterRequest
{
    fn from(
        invocation: crate::aim_and_fire::InvokeGenericEmitter<
            ProjectileEmitterDescriptor,
            GenericEmitterRuntime,
        >,
    ) -> Self {
        Self {
            source_handle: invocation.owner_entity_id,
            owner_handle: invocation.secondary_owner_entity_id,
            target_handle: invocation.target_entity_id,
            explicit_target_position_raw: invocation.explicit_position_raw,
            descriptor: invocation.sub_e_descriptor,
            runtime: invocation.emitter_runtime,
            elapsed_us: invocation.elapsed_micros,
        }
    }
}

/// Exact state write performed before the first firing gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterInitialCadenceWrite {
    pub before_raw: i32,
    pub elapsed_us: u32,
    pub after_raw: i32,
}

/// One direct-mode method-31 rejection decrement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterSuppressionWrite {
    pub before_raw: i32,
    pub decrement_raw: i32,
    pub after_raw: i32,
}

/// One A/B selector toggle after the primary/auxiliary append pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterSelectorWrite {
    pub before_raw: u32,
    pub after_raw: u32,
}

/// Target-mode catch-up write performed after one emitted pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterTargetCadenceWrite {
    pub cadence_before_raw: i32,
    pub interval_us: u32,
    pub cadence_after_raw: i32,
    pub time_offset_before_raw: i32,
    pub time_offset_after_raw: i32,
}

/// Direct-mode time/count catch-up write performed after all appends in one
/// iteration, including the special second pair for methods 10/22/26.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterManualCounterWrite {
    pub remaining_time_before_raw: i32,
    pub remaining_time_after_raw: i32,
    pub remaining_bursts_before_raw: i32,
    pub remaining_bursts_after_raw: i32,
    pub step_raw: i32,
    pub time_offset_before_raw: i32,
    pub time_offset_after_raw: i32,
}

/// Exact semantic fields handed to the transient 0x2C-list append helper.
///
/// The helper owns linkage and internal padding. `time_offset_raw` is the
/// catch-up timestamp accumulated in retail local `-0x2C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterAppendRequest {
    /// `FUN_004147A0` resolves this handle afresh. It does not use or validate
    /// the allocation pointer cached by `FUN_00424650`.
    pub source_handle: u32,
    pub owner_handle: u32,
    pub projectile_method: u32,
    pub emitter_selector: u16,
    pub direction_raw: [i32; 3],
    pub time_offset_raw: i32,
    pub speed_field: GenericEmitterSpeedField,
    pub auxiliary: bool,
}

/// Retail's signed transient-record word at offset `+0x28`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterSpeedField {
    /// Primary records always receive the resolved/explicit launch speed.
    Explicit(i16),
    /// The auxiliary method-15 stack record does not visibly initialize this
    /// word before `FUN_004147A0` copies all 0x2C bytes. Preserve the
    /// indeterminacy rather than fabricating a zero.
    IndeterminateAuxiliaryStackWord,
}

/// Proven inputs to target-mode fixed-point lead, speed and gravity work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterTargetLaunchRequest {
    pub cached_source: GenericEmitterEntitySnapshot,
    pub target_position_raw: [i16; 3],
    /// A fresh lookup is performed for every catch-up shot. Missing is valid:
    /// retail then omits target-velocity lead while retaining the accepted
    /// target position.
    pub current_target: Option<GenericEmitterEntitySnapshot>,
    pub projectile_method: u32,
    pub speed_override_raw: i16,
}

/// Proven inputs to direct/manual body-matrix construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterManualBasisRequest {
    pub cached_source: GenericEmitterEntitySnapshot,
    pub manual_angle_raw: Option<u16>,
    pub basis_adjustment_identity: Option<u64>,
}

/// Adapter-computed target-mode launch state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterTargetLaunchSolution {
    pub direction_raw: [i32; 3],
    /// Target mode resolves a zero descriptor override through the retail
    /// method-speed table. This exact signed word is copied to record `+0x28`.
    pub speed_raw: i16,
    /// Retail mutates its local target coordinates while applying target
    /// velocity lead and gravity. Catch-up shots start from those mutated
    /// coordinates rather than the initially accepted/spread target.
    pub target_position_after_raw: [i16; 3],
}

/// Adapter-computed direct/manual basis and trigonometric locals.
///
/// These values are copied into the machine before the joint write so a
/// binding which aliases later runtime state cannot retroactively change the
/// current shot's pre-joint basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterManualBasisSolution {
    pub basis_raw: [i32; 9],
    pub sine_raw: i32,
    pub cosine_raw: i32,
}

/// Proven inputs to the final post-joint direct/manual vector computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterManualLaunchRequest {
    pub basis: GenericEmitterManualBasisSolution,
    pub projectile_method: u32,
}

/// Adapter-computed direct/manual launch direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEmitterManualLaunchSolution {
    pub direction_raw: [i32; 3],
}

/// Exact cached-pointer dereference being refreshed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterSourceRefreshPoint {
    Method31Gate,
    Method31Rejection,
    TargetAxisGate,
    TargetShot,
    ManualShot,
}

/// One externally visible boundary, also used as the anti-replay tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterPhase {
    LookupEntrySource,
    CommitInitialCadence,
    DrawCadenceRandom,
    RefreshCachedSource,
    EmitMethod31Rejection,
    CommitMethod31Suppression,
    LookupTargetPresence,
    LookupTargetPosition,
    EvaluateAim,
    EvaluateForwardHalfSpace,
    DrawSpreadX,
    DrawSpreadZ,
    LookupTargetForLead,
    ReadManualAngleBinding,
    ResolveTargetLaunch,
    ResolveManualBasis,
    CommitGunJoint,
    ResolveManualLaunch,
    AppendPrimary,
    AppendAuxiliary,
    CommitAlternateSelector,
    AppendManualRepeatPrimary,
    AppendManualRepeatAuxiliary,
    CommitTargetCadence,
    CommitManualCounters,
    LookupSourceForSound,
    PlaySound,
}

/// Adapter action in exact retail order.
///
/// Deliberately neither `Copy` nor `Clone`: the action travels with a linear
/// receipt and must not be duplicated by a safe adapter.
#[derive(Debug, PartialEq, Eq)]
pub enum GenericEmitterAction {
    LookupEntity {
        phase: GenericEmitterPhase,
        handle: u32,
    },
    CommitInitialCadence {
        phase: GenericEmitterPhase,
        write: GenericEmitterInitialCadenceWrite,
    },
    DrawSharedRandom {
        phase: GenericEmitterPhase,
    },
    /// Reread mutable fields through the allocation cached at entry.
    ///
    /// `source_handle` is context, not permission to perform a fresh handle
    /// lookup or silently rebind to a replacement allocation.
    RefreshCachedSource {
        phase: GenericEmitterPhase,
        source_handle: u32,
        allocation_identity: u64,
        point: GenericEmitterSourceRefreshPoint,
    },
    EmitMethod31Rejection {
        phase: GenericEmitterPhase,
        source_handle: u32,
        source_position_raw: [i16; 3],
        effect_class: u32,
    },
    CommitMethod31Suppression {
        phase: GenericEmitterPhase,
        write: GenericEmitterSuppressionWrite,
    },
    EvaluateAim {
        phase: GenericEmitterPhase,
        source_handle: u32,
        target_position_raw: [i16; 3],
    },
    /// Evaluate whether the target is on or ahead of the source's forward plane.
    EvaluateForwardHalfSpace {
        phase: GenericEmitterPhase,
        source_handle: u32,
        target_position_raw: [i16; 3],
    },
    /// Reread runtime binding dword 2 for this direct-mode iteration.
    ReadManualAngleBinding {
        phase: GenericEmitterPhase,
    },
    ResolveTargetLaunch {
        phase: GenericEmitterPhase,
        request: GenericEmitterTargetLaunchRequest,
    },
    ResolveManualBasis {
        phase: GenericEmitterPhase,
        request: GenericEmitterManualBasisRequest,
    },
    CommitGunJoint {
        phase: GenericEmitterPhase,
        binding_identity: u64,
        value_raw: u16,
        emitter_selector: u32,
    },
    /// Compute the final vector after the selected joint write, using the
    /// already-copied pre-joint basis.
    ResolveManualLaunch {
        phase: GenericEmitterPhase,
        request: GenericEmitterManualLaunchRequest,
    },
    AppendTransient {
        phase: GenericEmitterPhase,
        request: GenericEmitterAppendRequest,
    },
    CommitAlternateSelector {
        phase: GenericEmitterPhase,
        write: GenericEmitterSelectorWrite,
    },
    CommitTargetCadence {
        phase: GenericEmitterPhase,
        write: GenericEmitterTargetCadenceWrite,
    },
    CommitManualCounters {
        phase: GenericEmitterPhase,
        write: GenericEmitterManualCounterWrite,
    },
    PlayPositionalSound {
        phase: GenericEmitterPhase,
        sound_id: u32,
        source_position_raw: Option<[i16; 3]>,
        frequency_multiplier_16_16: u32,
        volume_multiplier_16_16: u32,
    },
}

impl GenericEmitterAction {
    pub const fn phase(&self) -> GenericEmitterPhase {
        match self {
            Self::LookupEntity { phase, .. }
            | Self::CommitInitialCadence { phase, .. }
            | Self::DrawSharedRandom { phase }
            | Self::RefreshCachedSource { phase, .. }
            | Self::EmitMethod31Rejection { phase, .. }
            | Self::CommitMethod31Suppression { phase, .. }
            | Self::EvaluateAim { phase, .. }
            | Self::EvaluateForwardHalfSpace { phase, .. }
            | Self::ReadManualAngleBinding { phase }
            | Self::ResolveTargetLaunch { phase, .. }
            | Self::ResolveManualBasis { phase, .. }
            | Self::CommitGunJoint { phase, .. }
            | Self::ResolveManualLaunch { phase, .. }
            | Self::AppendTransient { phase, .. }
            | Self::CommitAlternateSelector { phase, .. }
            | Self::CommitTargetCadence { phase, .. }
            | Self::CommitManualCounters { phase, .. }
            | Self::PlayPositionalSound { phase, .. } => *phase,
        }
    }
}

/// Linear proof that one specific machine action was issued.
///
/// Fields are private and the type is deliberately neither `Copy` nor `Clone`.
/// The only production constructor is [`GenericProjectileEmitterMachine::poll`].
#[derive(Debug, PartialEq, Eq)]
pub struct GenericEmitterReceipt {
    transaction_id: GenericEmitterTransactionId,
    action_sequence: u64,
}

impl GenericEmitterReceipt {
    pub const fn transaction_id(&self) -> GenericEmitterTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }
}

/// One non-duplicable action/receipt pair returned to the adapter.
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedGenericEmitterAction {
    pub receipt: GenericEmitterReceipt,
    pub action: GenericEmitterAction,
}

/// Adapter-side failure at a proven external boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterExternalBlock {
    EntityLookupUnavailable,
    StateCommitUnavailable,
    SharedRandomUnavailable,
    RejectionEffectUnavailable,
    AimUnavailable,
    ForwardHalfSpaceUnavailable,
    TargetLaunchUnavailable,
    ManualBasisUnavailable,
    GunJointUnavailable,
    TransientListUnavailable,
    SoundUnavailable,
}

/// Durable evidence or lifetime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterBlock {
    MissingCachedSourceAllocation {
        point: GenericEmitterSourceRefreshPoint,
    },
    CachedSourceUnavailableAtRefresh {
        point: GenericEmitterSourceRefreshPoint,
        allocation_identity: u64,
    },
    CachedSourceAllocationIdentityMismatch {
        point: GenericEmitterSourceRefreshPoint,
        expected: u64,
        actual: u64,
    },
    MissingTargetWithoutExplicitPosition,
    MissingTargetOnSecondLookup,
    ZeroElapsedCadenceDivision,
    ZeroSuppressionStepWouldNotTerminate,
    External {
        phase: GenericEmitterPhase,
        reason: GenericEmitterExternalBlock,
    },
}

/// Phase-tagged completion supplied by the adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterResume {
    EntityLookupResolved {
        phase: GenericEmitterPhase,
        entity: Option<GenericEmitterEntitySnapshot>,
    },
    CachedSourceRefreshed {
        phase: GenericEmitterPhase,
        entity: Option<GenericEmitterEntitySnapshot>,
    },
    RandomDrawn {
        phase: GenericEmitterPhase,
        value: u32,
    },
    AimEvaluated {
        phase: GenericEmitterPhase,
        error_raw: i32,
    },
    /// Result of retail `FUN_0041E930`'s signed forward-basis projection.
    ForwardHalfSpaceEvaluated {
        phase: GenericEmitterPhase,
        in_forward_half_space: bool,
    },
    ManualAngleBindingRead {
        phase: GenericEmitterPhase,
        angle_raw: Option<u16>,
    },
    TargetLaunchResolved {
        phase: GenericEmitterPhase,
        solution: GenericEmitterTargetLaunchSolution,
    },
    ManualBasisResolved {
        phase: GenericEmitterPhase,
        solution: GenericEmitterManualBasisSolution,
    },
    ManualLaunchResolved {
        phase: GenericEmitterPhase,
        solution: GenericEmitterManualLaunchSolution,
    },
    AppendReturned {
        phase: GenericEmitterPhase,
        result_raw: u32,
    },
    Acknowledged {
        phase: GenericEmitterPhase,
    },
    Blocked {
        phase: GenericEmitterPhase,
        reason: GenericEmitterExternalBlock,
    },
}

impl GenericEmitterResume {
    pub const fn phase(self) -> GenericEmitterPhase {
        match self {
            Self::EntityLookupResolved { phase, .. }
            | Self::CachedSourceRefreshed { phase, .. }
            | Self::RandomDrawn { phase, .. }
            | Self::AimEvaluated { phase, .. }
            | Self::ForwardHalfSpaceEvaluated { phase, .. }
            | Self::ManualAngleBindingRead { phase, .. }
            | Self::TargetLaunchResolved { phase, .. }
            | Self::ManualBasisResolved { phase, .. }
            | Self::ManualLaunchResolved { phase, .. }
            | Self::AppendReturned { phase, .. }
            | Self::Acknowledged { phase }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterCompletion {
    ReturnZero,
    /// Exact nonzero bits returned by `FUN_004147A0`.
    ReturnNonZero(NonZeroU32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum GenericEmitterPoll {
    Action(IssuedGenericEmitterAction),
    Awaiting(GenericEmitterPhase),
    Blocked(GenericEmitterBlock),
    Complete(GenericEmitterCompletion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericEmitterProtocolError {
    NoOutstandingAction {
        transaction_id: GenericEmitterTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: GenericEmitterTransactionId,
        actual: GenericEmitterTransactionId,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    LookupHandleMismatch {
        phase: GenericEmitterPhase,
        expected: u32,
        actual: u32,
    },
    PhaseMismatch {
        expected: GenericEmitterPhase,
        actual: GenericEmitterPhase,
    },
    UnexpectedResumeKind {
        phase: GenericEmitterPhase,
    },
}

/// Recoverable protocol rejection.
///
/// The outstanding action remains pending and the linear receipt is returned
/// so the adapter can correct a malformed response without deadlocking the
/// transaction.
#[derive(Debug, PartialEq, Eq)]
pub struct GenericEmitterResumeFailure {
    pub receipt: GenericEmitterReceipt,
    pub error: GenericEmitterProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenericEmitterStage {
    LookupEntrySource,
    CommitInitialCadence(GenericEmitterInitialCadenceWrite),
    DrawCadenceRandom {
        divisor: u32,
    },
    RefreshCachedSource(GenericEmitterSourceRefreshPoint),
    EmitMethod31Rejection,
    CommitMethod31Suppression(GenericEmitterSuppressionWrite),
    LookupTargetPresence,
    LookupTargetPosition,
    EvaluateAim,
    EvaluateForwardHalfSpace,
    DrawSpreadX,
    DrawSpreadZ,
    LookupTargetForLead,
    ReadManualAngleBinding,
    ResolveTargetLaunch,
    ResolveManualBasis,
    CommitGunJoint {
        binding_identity: u64,
        selector: u32,
    },
    ResolveManualLaunch,
    AppendPrimary,
    AppendAuxiliary,
    CommitAlternateSelector(GenericEmitterSelectorWrite),
    AppendManualRepeatPrimary,
    AppendManualRepeatAuxiliary,
    CommitTargetCadence(GenericEmitterTargetCadenceWrite),
    CommitManualCounters(GenericEmitterManualCounterWrite),
    LookupSourceForSound,
    PlaySound(Option<[i16; 3]>),
    Blocked(GenericEmitterBlock),
    Complete(GenericEmitterCompletion),
}

impl GenericEmitterStage {
    const fn phase(self) -> Option<GenericEmitterPhase> {
        Some(match self {
            Self::LookupEntrySource => GenericEmitterPhase::LookupEntrySource,
            Self::CommitInitialCadence(_) => GenericEmitterPhase::CommitInitialCadence,
            Self::DrawCadenceRandom { .. } => GenericEmitterPhase::DrawCadenceRandom,
            Self::RefreshCachedSource(_) => GenericEmitterPhase::RefreshCachedSource,
            Self::EmitMethod31Rejection => GenericEmitterPhase::EmitMethod31Rejection,
            Self::CommitMethod31Suppression(_) => GenericEmitterPhase::CommitMethod31Suppression,
            Self::LookupTargetPresence => GenericEmitterPhase::LookupTargetPresence,
            Self::LookupTargetPosition => GenericEmitterPhase::LookupTargetPosition,
            Self::EvaluateAim => GenericEmitterPhase::EvaluateAim,
            Self::EvaluateForwardHalfSpace => GenericEmitterPhase::EvaluateForwardHalfSpace,
            Self::DrawSpreadX => GenericEmitterPhase::DrawSpreadX,
            Self::DrawSpreadZ => GenericEmitterPhase::DrawSpreadZ,
            Self::LookupTargetForLead => GenericEmitterPhase::LookupTargetForLead,
            Self::ReadManualAngleBinding => GenericEmitterPhase::ReadManualAngleBinding,
            Self::ResolveTargetLaunch => GenericEmitterPhase::ResolveTargetLaunch,
            Self::ResolveManualBasis => GenericEmitterPhase::ResolveManualBasis,
            Self::CommitGunJoint { .. } => GenericEmitterPhase::CommitGunJoint,
            Self::ResolveManualLaunch => GenericEmitterPhase::ResolveManualLaunch,
            Self::AppendPrimary => GenericEmitterPhase::AppendPrimary,
            Self::AppendAuxiliary => GenericEmitterPhase::AppendAuxiliary,
            Self::CommitAlternateSelector(_) => GenericEmitterPhase::CommitAlternateSelector,
            Self::AppendManualRepeatPrimary => GenericEmitterPhase::AppendManualRepeatPrimary,
            Self::AppendManualRepeatAuxiliary => GenericEmitterPhase::AppendManualRepeatAuxiliary,
            Self::CommitTargetCadence(_) => GenericEmitterPhase::CommitTargetCadence,
            Self::CommitManualCounters(_) => GenericEmitterPhase::CommitManualCounters,
            Self::LookupSourceForSound => GenericEmitterPhase::LookupSourceForSound,
            Self::PlaySound(_) => GenericEmitterPhase::PlaySound,
            Self::Blocked(_) | Self::Complete(_) => return None,
        })
    }
}

/// Non-replayable detached `FUN_00424650` transaction.
///
/// Deliberately neither `Copy` nor `Clone`: duplicating the machine could
/// replay RNG, list appends, joint writes, counters, effects or sound.
#[derive(Debug, PartialEq, Eq)]
pub struct GenericProjectileEmitterMachine {
    transaction_id: GenericEmitterTransactionId,
    request: GenericEmitterRequest,
    runtime: GenericEmitterRuntime,
    cached_source_allocation_identity: Option<u64>,
    refreshed_source: Option<GenericEmitterEntitySnapshot>,
    current_target: Option<GenericEmitterEntitySnapshot>,
    current_manual_angle_raw: Option<u16>,
    manual_basis: Option<GenericEmitterManualBasisSolution>,
    target_position_raw: Option<[i16; 3]>,
    direction_raw: [i32; 3],
    speed_raw: i16,
    time_offset_raw: i32,
    stage: GenericEmitterStage,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
}

impl GenericProjectileEmitterMachine {
    pub const fn start(
        transaction_id: GenericEmitterTransactionId,
        request: GenericEmitterRequest,
    ) -> Self {
        Self {
            transaction_id,
            runtime: request.runtime,
            request,
            cached_source_allocation_identity: None,
            refreshed_source: None,
            current_target: None,
            current_manual_angle_raw: None,
            manual_basis: None,
            target_position_raw: None,
            direction_raw: [0; 3],
            speed_raw: 0,
            time_offset_raw: 0,
            stage: GenericEmitterStage::LookupEntrySource,
            next_action_sequence: 1,
            issued_action_sequence: None,
        }
    }

    pub const fn runtime(&self) -> GenericEmitterRuntime {
        self.runtime
    }

    pub const fn time_offset_raw(&self) -> i32 {
        self.time_offset_raw
    }

    pub fn poll(&mut self) -> GenericEmitterPoll {
        let Some(phase) = self.stage.phase() else {
            return match self.stage {
                GenericEmitterStage::Blocked(block) => GenericEmitterPoll::Blocked(block),
                GenericEmitterStage::Complete(done) => GenericEmitterPoll::Complete(done),
                _ => unreachable!("active stages always expose a phase"),
            };
        };
        if self.issued_action_sequence.is_some() {
            return GenericEmitterPoll::Awaiting(phase);
        }

        let action = match self.stage {
            GenericEmitterStage::LookupEntrySource => GenericEmitterAction::LookupEntity {
                phase,
                handle: self.request.source_handle,
            },
            GenericEmitterStage::CommitInitialCadence(write) => {
                GenericEmitterAction::CommitInitialCadence { phase, write }
            }
            GenericEmitterStage::DrawCadenceRandom { .. }
            | GenericEmitterStage::DrawSpreadX
            | GenericEmitterStage::DrawSpreadZ => GenericEmitterAction::DrawSharedRandom { phase },
            GenericEmitterStage::RefreshCachedSource(point) => {
                GenericEmitterAction::RefreshCachedSource {
                    phase,
                    source_handle: self.request.source_handle,
                    allocation_identity: self
                        .cached_source_allocation_identity
                        .expect("source refresh requires the entry allocation identity"),
                    point,
                }
            }
            GenericEmitterStage::EmitMethod31Rejection => {
                let source = self
                    .refreshed_source
                    .expect("method-31 rejection requires refreshed source fields");
                GenericEmitterAction::EmitMethod31Rejection {
                    phase,
                    source_handle: self.request.source_handle,
                    source_position_raw: source.position_raw,
                    effect_class: GENERIC_PROJECTILE_METHOD_31_REJECTION_EFFECT,
                }
            }
            GenericEmitterStage::CommitMethod31Suppression(write) => {
                GenericEmitterAction::CommitMethod31Suppression { phase, write }
            }
            GenericEmitterStage::LookupTargetPresence
            | GenericEmitterStage::LookupTargetPosition
            | GenericEmitterStage::LookupTargetForLead => GenericEmitterAction::LookupEntity {
                phase,
                handle: self.request.target_handle,
            },
            GenericEmitterStage::EvaluateAim => GenericEmitterAction::EvaluateAim {
                phase,
                source_handle: self.request.source_handle,
                target_position_raw: self
                    .target_position_raw
                    .expect("aim requires an accepted target position"),
            },
            GenericEmitterStage::EvaluateForwardHalfSpace => {
                GenericEmitterAction::EvaluateForwardHalfSpace {
                    phase,
                    source_handle: self.request.source_handle,
                    target_position_raw: self
                        .target_position_raw
                        .expect("forward-half-space test requires an accepted target position"),
                }
            }
            GenericEmitterStage::ReadManualAngleBinding => {
                GenericEmitterAction::ReadManualAngleBinding { phase }
            }
            GenericEmitterStage::ResolveTargetLaunch => GenericEmitterAction::ResolveTargetLaunch {
                phase,
                request: GenericEmitterTargetLaunchRequest {
                    cached_source: self
                        .refreshed_source
                        .expect("target launch requires refreshed source fields"),
                    target_position_raw: self
                        .target_position_raw
                        .expect("target launch requires target position"),
                    current_target: self.current_target,
                    projectile_method: self.runtime.projectile_method,
                    speed_override_raw: self.request.descriptor.speed_override_raw,
                },
            },
            GenericEmitterStage::ResolveManualBasis => GenericEmitterAction::ResolveManualBasis {
                phase,
                request: GenericEmitterManualBasisRequest {
                    cached_source: self
                        .refreshed_source
                        .expect("manual basis requires refreshed source fields"),
                    manual_angle_raw: self.current_manual_angle_raw,
                    basis_adjustment_identity: self.runtime.basis_adjustment_identity,
                },
            },
            GenericEmitterStage::CommitGunJoint {
                binding_identity,
                selector,
            } => GenericEmitterAction::CommitGunJoint {
                phase,
                binding_identity,
                value_raw: u16::MAX,
                emitter_selector: selector,
            },
            GenericEmitterStage::ResolveManualLaunch => GenericEmitterAction::ResolveManualLaunch {
                phase,
                request: GenericEmitterManualLaunchRequest {
                    basis: self
                        .manual_basis
                        .expect("manual launch requires its pre-joint basis"),
                    projectile_method: self.runtime.projectile_method,
                },
            },
            GenericEmitterStage::AppendPrimary | GenericEmitterStage::AppendManualRepeatPrimary => {
                GenericEmitterAction::AppendTransient {
                    phase,
                    request: self.append_request(false),
                }
            }
            GenericEmitterStage::AppendAuxiliary
            | GenericEmitterStage::AppendManualRepeatAuxiliary => {
                GenericEmitterAction::AppendTransient {
                    phase,
                    request: self.append_request(true),
                }
            }
            GenericEmitterStage::CommitAlternateSelector(write) => {
                GenericEmitterAction::CommitAlternateSelector { phase, write }
            }
            GenericEmitterStage::CommitTargetCadence(write) => {
                GenericEmitterAction::CommitTargetCadence { phase, write }
            }
            GenericEmitterStage::CommitManualCounters(write) => {
                GenericEmitterAction::CommitManualCounters { phase, write }
            }
            GenericEmitterStage::LookupSourceForSound => GenericEmitterAction::LookupEntity {
                phase,
                handle: self.request.source_handle,
            },
            GenericEmitterStage::PlaySound(source_position_raw) => {
                GenericEmitterAction::PlayPositionalSound {
                    phase,
                    sound_id: self.runtime.sound_id,
                    source_position_raw,
                    frequency_multiplier_16_16: 0x0001_0000,
                    volume_multiplier_16_16: 0x0001_0000,
                }
            }
            GenericEmitterStage::Blocked(_) | GenericEmitterStage::Complete(_) => {
                unreachable!("terminal stages returned above")
            }
        };
        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one emitter transaction cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        GenericEmitterPoll::Action(IssuedGenericEmitterAction {
            receipt: GenericEmitterReceipt {
                transaction_id: self.transaction_id,
                action_sequence,
            },
            action,
        })
    }

    pub fn resume(
        &mut self,
        receipt: GenericEmitterReceipt,
        resume: GenericEmitterResume,
    ) -> Result<(), GenericEmitterResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            return Err(GenericEmitterResumeFailure {
                error: GenericEmitterProtocolError::NoOutstandingAction {
                    transaction_id: receipt.transaction_id,
                    action_sequence: receipt.action_sequence,
                },
                receipt,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(GenericEmitterResumeFailure {
                error: GenericEmitterProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.action_sequence != expected_sequence {
            return Err(GenericEmitterResumeFailure {
                error: GenericEmitterProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }
        let expected = self
            .stage
            .phase()
            .expect("an outstanding receipt always belongs to an active stage");
        let actual = resume.phase();
        if actual != expected {
            return Err(GenericEmitterResumeFailure {
                error: GenericEmitterProtocolError::PhaseMismatch { expected, actual },
                receipt,
            });
        }
        if let Some(actual_handle) = returned_entity_handle(resume) {
            if let Some(expected_handle) = self.expected_lookup_handle(resume) {
                if actual_handle != expected_handle {
                    return Err(GenericEmitterResumeFailure {
                        error: GenericEmitterProtocolError::LookupHandleMismatch {
                            phase: expected,
                            expected: expected_handle,
                            actual: actual_handle,
                        },
                        receipt,
                    });
                }
            }
        }
        if let GenericEmitterResume::Blocked { reason, .. } = resume {
            self.stage = GenericEmitterStage::Blocked(GenericEmitterBlock::External {
                phase: expected,
                reason,
            });
            self.issued_action_sequence = None;
            return Ok(());
        }

        let accepted = match (self.stage, resume) {
            (
                GenericEmitterStage::LookupEntrySource,
                GenericEmitterResume::EntityLookupResolved { entity, .. },
            ) => {
                self.cached_source_allocation_identity =
                    entity.map(|entity| entity.allocation_identity);
                let before_raw = self.runtime.cadence_raw;
                let after_raw = if before_raw < 1 {
                    0i32.wrapping_sub(self.request.elapsed_us as i32)
                } else {
                    before_raw.wrapping_sub(self.request.elapsed_us as i32)
                };
                self.stage =
                    GenericEmitterStage::CommitInitialCadence(GenericEmitterInitialCadenceWrite {
                        before_raw,
                        elapsed_us: self.request.elapsed_us,
                        after_raw,
                    });
                true
            }
            (
                GenericEmitterStage::CommitInitialCadence(write),
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                self.runtime.cadence_raw = write.after_raw;
                self.enter_initial_gate();
                true
            }
            (
                GenericEmitterStage::DrawCadenceRandom { divisor },
                GenericEmitterResume::RandomDrawn { value, .. },
            ) => {
                if u32::from(value as u16) % divisor == 0 {
                    self.stage = GenericEmitterStage::LookupTargetPresence;
                } else {
                    self.complete_zero();
                }
                true
            }
            (
                GenericEmitterStage::RefreshCachedSource(point),
                GenericEmitterResume::CachedSourceRefreshed { entity, .. },
            ) => {
                self.accept_source_refresh(point, entity);
                true
            }
            (
                GenericEmitterStage::EmitMethod31Rejection,
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                let before_raw = self.runtime.remaining_time_raw;
                let after_raw = before_raw.wrapping_sub(self.runtime.manual_step_raw);
                self.stage = GenericEmitterStage::CommitMethod31Suppression(
                    GenericEmitterSuppressionWrite {
                        before_raw,
                        decrement_raw: self.runtime.manual_step_raw,
                        after_raw,
                    },
                );
                true
            }
            (
                GenericEmitterStage::CommitMethod31Suppression(write),
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                self.runtime.remaining_time_raw = write.after_raw;
                if write.after_raw > 0 {
                    self.enter_source_refresh(GenericEmitterSourceRefreshPoint::Method31Rejection);
                } else {
                    self.complete_zero();
                }
                true
            }
            (
                GenericEmitterStage::LookupTargetPresence,
                GenericEmitterResume::EntityLookupResolved { entity, .. },
            ) => {
                if entity.is_some() {
                    // Retail deliberately performs a second lookup before it
                    // reads +0x96 rather than reusing the presence result.
                    self.stage = GenericEmitterStage::LookupTargetPosition;
                } else if let Some(position) = self.request.explicit_target_position_raw {
                    self.target_position_raw = Some(position);
                    self.enter_after_target_position();
                } else {
                    self.stage = GenericEmitterStage::Blocked(
                        GenericEmitterBlock::MissingTargetWithoutExplicitPosition,
                    );
                }
                true
            }
            (
                GenericEmitterStage::LookupTargetPosition,
                GenericEmitterResume::EntityLookupResolved { entity, .. },
            ) => {
                if let Some(entity) = entity {
                    self.target_position_raw = Some(entity.position_raw);
                    self.enter_after_target_position();
                } else {
                    self.stage = GenericEmitterStage::Blocked(
                        GenericEmitterBlock::MissingTargetOnSecondLookup,
                    );
                }
                true
            }
            (
                GenericEmitterStage::EvaluateAim,
                GenericEmitterResume::AimEvaluated { error_raw, .. },
            ) => {
                let threshold = self.request.descriptor.aim_threshold_raw;
                if threshold > i16::MAX as u16 {
                    self.stage = GenericEmitterStage::DrawSpreadX;
                } else {
                    let error = i32::from(error_raw as i16);
                    let threshold = i32::from(threshold);
                    if error < -threshold || error >= threshold {
                        self.complete_zero();
                    } else {
                        self.stage = GenericEmitterStage::EvaluateForwardHalfSpace;
                    }
                }
                true
            }
            (
                GenericEmitterStage::EvaluateForwardHalfSpace,
                GenericEmitterResume::ForwardHalfSpaceEvaluated {
                    in_forward_half_space,
                    ..
                },
            ) => {
                if in_forward_half_space {
                    self.stage = GenericEmitterStage::DrawSpreadX;
                } else {
                    self.complete_zero();
                }
                true
            }
            (GenericEmitterStage::DrawSpreadX, GenericEmitterResume::RandomDrawn { value, .. }) => {
                let mut position = self
                    .target_position_raw
                    .expect("spread requires target position");
                position[0] = position[0].wrapping_add(spread_offset(
                    value as u16,
                    self.request.descriptor.spread_raw,
                ));
                self.target_position_raw = Some(position);
                self.stage = GenericEmitterStage::DrawSpreadZ;
                true
            }
            (GenericEmitterStage::DrawSpreadZ, GenericEmitterResume::RandomDrawn { value, .. }) => {
                let mut position = self
                    .target_position_raw
                    .expect("spread requires target position");
                position[2] = position[2].wrapping_add(spread_offset(
                    value as u16,
                    self.request.descriptor.spread_raw,
                ));
                self.target_position_raw = Some(position);
                if self.runtime.projectile_method == 0 {
                    self.complete_zero();
                } else {
                    self.enter_shot_iteration();
                }
                true
            }
            (
                GenericEmitterStage::LookupTargetForLead,
                GenericEmitterResume::EntityLookupResolved { entity, .. },
            ) => {
                self.current_target = entity;
                self.enter_source_refresh(GenericEmitterSourceRefreshPoint::TargetShot);
                true
            }
            (
                GenericEmitterStage::ReadManualAngleBinding,
                GenericEmitterResume::ManualAngleBindingRead { angle_raw, .. },
            ) => {
                self.current_manual_angle_raw = angle_raw;
                self.enter_source_refresh(GenericEmitterSourceRefreshPoint::ManualShot);
                true
            }
            (
                GenericEmitterStage::ResolveTargetLaunch,
                GenericEmitterResume::TargetLaunchResolved { solution, .. },
            ) => {
                self.direction_raw = solution.direction_raw;
                self.speed_raw = solution.speed_raw;
                self.target_position_raw = Some(solution.target_position_after_raw);
                self.stage = GenericEmitterStage::AppendPrimary;
                true
            }
            (
                GenericEmitterStage::ResolveManualBasis,
                GenericEmitterResume::ManualBasisResolved { solution, .. },
            ) => {
                self.manual_basis = Some(solution);
                self.enter_manual_joint_or_launch();
                true
            }
            (
                GenericEmitterStage::CommitGunJoint { .. },
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                self.stage = GenericEmitterStage::ResolveManualLaunch;
                true
            }
            (
                GenericEmitterStage::ResolveManualLaunch,
                GenericEmitterResume::ManualLaunchResolved { solution, .. },
            ) => {
                self.direction_raw = solution.direction_raw;
                // Direct/manual mode copies the descriptor word as-is; unlike
                // target mode it does not resolve zero through FUN_0044EA50.
                self.speed_raw = self.request.descriptor.speed_override_raw;
                self.stage = GenericEmitterStage::AppendPrimary;
                true
            }
            (
                GenericEmitterStage::AppendPrimary,
                GenericEmitterResume::AppendReturned { result_raw, .. },
            ) => {
                if self.accept_append_result(result_raw) {
                    if self.request.descriptor.auxiliary_command != 0 {
                        self.stage = GenericEmitterStage::AppendAuxiliary;
                    } else {
                        self.enter_after_first_append_pair();
                    }
                }
                true
            }
            (
                GenericEmitterStage::AppendAuxiliary,
                GenericEmitterResume::AppendReturned { result_raw, .. },
            ) => {
                if self.accept_append_result(result_raw) {
                    self.enter_after_first_append_pair();
                }
                true
            }
            (
                GenericEmitterStage::CommitAlternateSelector(write),
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                self.runtime.emitter_selector = write.after_raw;
                self.enter_after_selector_toggle();
                true
            }
            (
                GenericEmitterStage::AppendManualRepeatPrimary,
                GenericEmitterResume::AppendReturned { result_raw, .. },
            ) => {
                if self.accept_append_result(result_raw) {
                    if self.request.descriptor.auxiliary_command != 0 {
                        self.stage = GenericEmitterStage::AppendManualRepeatAuxiliary;
                    } else {
                        self.enter_iteration_counter_commit();
                    }
                }
                true
            }
            (
                GenericEmitterStage::AppendManualRepeatAuxiliary,
                GenericEmitterResume::AppendReturned { result_raw, .. },
            ) => {
                if self.accept_append_result(result_raw) {
                    self.enter_iteration_counter_commit();
                }
                true
            }
            (
                GenericEmitterStage::CommitTargetCadence(write),
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                self.runtime.cadence_raw = write.cadence_after_raw;
                self.time_offset_raw = write.time_offset_after_raw;
                if self.runtime.cadence_raw < 1 {
                    self.enter_shot_iteration();
                } else {
                    self.enter_sound_suffix();
                }
                true
            }
            (
                GenericEmitterStage::CommitManualCounters(write),
                GenericEmitterResume::Acknowledged { .. },
            ) => {
                self.runtime.remaining_time_raw = write.remaining_time_after_raw;
                self.runtime.remaining_bursts_raw = write.remaining_bursts_after_raw;
                self.time_offset_raw = write.time_offset_after_raw;
                if self.runtime.remaining_time_raw > 0 && self.runtime.remaining_bursts_raw > 0 {
                    self.enter_shot_iteration();
                } else {
                    self.enter_sound_suffix();
                }
                true
            }
            (
                GenericEmitterStage::LookupSourceForSound,
                GenericEmitterResume::EntityLookupResolved { entity, .. },
            ) => {
                self.stage =
                    GenericEmitterStage::PlaySound(entity.map(|entity| entity.position_raw));
                true
            }
            (GenericEmitterStage::PlaySound(_), GenericEmitterResume::Acknowledged { .. }) => {
                self.complete_zero();
                true
            }
            _ => false,
        };

        if !accepted {
            return Err(GenericEmitterResumeFailure {
                error: GenericEmitterProtocolError::UnexpectedResumeKind { phase: expected },
                receipt,
            });
        }
        self.issued_action_sequence = None;
        Ok(())
    }

    fn expected_lookup_handle(&self, resume: GenericEmitterResume) -> Option<u32> {
        match (self.stage, resume) {
            (
                GenericEmitterStage::LookupEntrySource | GenericEmitterStage::LookupSourceForSound,
                GenericEmitterResume::EntityLookupResolved { .. },
            )
            | (
                GenericEmitterStage::RefreshCachedSource(_),
                GenericEmitterResume::CachedSourceRefreshed { .. },
            ) => Some(self.request.source_handle),
            (
                GenericEmitterStage::LookupTargetPresence
                | GenericEmitterStage::LookupTargetPosition
                | GenericEmitterStage::LookupTargetForLead,
                GenericEmitterResume::EntityLookupResolved { .. },
            ) => Some(self.request.target_handle),
            _ => None,
        }
    }

    fn enter_initial_gate(&mut self) {
        if self.runtime.target_mode() {
            if self.request.descriptor.stochastic_gate_mode == 0 || self.runtime.cadence_raw > 0 {
                if self.request.elapsed_us == 0 {
                    self.stage = GenericEmitterStage::Blocked(
                        GenericEmitterBlock::ZeroElapsedCadenceDivision,
                    );
                    return;
                }
                let divisor =
                    (self.request.descriptor.random_interval_us / self.request.elapsed_us).max(1);
                self.stage = GenericEmitterStage::DrawCadenceRandom { divisor };
            } else {
                self.stage = GenericEmitterStage::LookupTargetPresence;
            }
            return;
        }

        if self.runtime.remaining_time_raw < 1 {
            self.complete_zero();
            return;
        }
        if self.runtime.projectile_method == 0 {
            self.complete_zero();
            return;
        }
        if self.runtime.projectile_method != 31 {
            self.enter_shot_iteration();
            return;
        }
        self.enter_source_refresh(GenericEmitterSourceRefreshPoint::Method31Gate);
    }

    fn enter_after_method31_gate_refresh(&mut self) {
        let source = self
            .refreshed_source
            .expect("method-31 gate requires refreshed source fields");
        if source.state_flags_at_0x08 & GENERIC_PROJECTILE_METHOD_31_PERMISSION_BIT != 0 {
            self.enter_shot_iteration();
        } else if self.runtime.manual_step_raw == 0 {
            self.stage = GenericEmitterStage::Blocked(
                GenericEmitterBlock::ZeroSuppressionStepWouldNotTerminate,
            );
        } else {
            self.stage = GenericEmitterStage::EmitMethod31Rejection;
        }
    }

    fn enter_after_target_position(&mut self) {
        self.enter_source_refresh(GenericEmitterSourceRefreshPoint::TargetAxisGate);
    }

    fn evaluate_target_axis(&mut self) {
        let source = self
            .refreshed_source
            .expect("target-axis gate requires refreshed source fields");
        let target = self
            .target_position_raw
            .expect("target position was accepted before axis tests");
        let tolerance = i32::from(self.request.descriptor.target_axis_tolerance_raw);
        let x = i32::from(source.position_raw[0].wrapping_sub(target[0])).abs();
        let z = i32::from(source.position_raw[2].wrapping_sub(target[2])).abs();
        if x < tolerance && z < tolerance {
            self.stage = GenericEmitterStage::EvaluateAim;
        } else {
            self.complete_zero();
        }
    }

    fn enter_shot_iteration(&mut self) {
        if self.runtime.target_mode() {
            self.stage = GenericEmitterStage::LookupTargetForLead;
        } else {
            self.stage = GenericEmitterStage::ReadManualAngleBinding;
        }
    }

    fn enter_source_refresh(&mut self, point: GenericEmitterSourceRefreshPoint) {
        if self.cached_source_allocation_identity.is_none() {
            self.stage =
                GenericEmitterStage::Blocked(GenericEmitterBlock::MissingCachedSourceAllocation {
                    point,
                });
            return;
        }
        self.refreshed_source = None;
        self.stage = GenericEmitterStage::RefreshCachedSource(point);
    }

    fn accept_source_refresh(
        &mut self,
        point: GenericEmitterSourceRefreshPoint,
        entity: Option<GenericEmitterEntitySnapshot>,
    ) {
        let expected = self
            .cached_source_allocation_identity
            .expect("source refresh stage requires cached allocation identity");
        let Some(entity) = entity else {
            self.stage = GenericEmitterStage::Blocked(
                GenericEmitterBlock::CachedSourceUnavailableAtRefresh {
                    point,
                    allocation_identity: expected,
                },
            );
            return;
        };
        if entity.allocation_identity != expected {
            self.stage = GenericEmitterStage::Blocked(
                GenericEmitterBlock::CachedSourceAllocationIdentityMismatch {
                    point,
                    expected,
                    actual: entity.allocation_identity,
                },
            );
            return;
        }
        self.refreshed_source = Some(entity);
        match point {
            GenericEmitterSourceRefreshPoint::Method31Gate => {
                self.enter_after_method31_gate_refresh();
            }
            GenericEmitterSourceRefreshPoint::Method31Rejection => {
                self.stage = GenericEmitterStage::EmitMethod31Rejection;
            }
            GenericEmitterSourceRefreshPoint::TargetAxisGate => self.evaluate_target_axis(),
            GenericEmitterSourceRefreshPoint::TargetShot => {
                self.stage = GenericEmitterStage::ResolveTargetLaunch;
            }
            GenericEmitterSourceRefreshPoint::ManualShot => {
                self.stage = GenericEmitterStage::ResolveManualBasis;
            }
        }
    }

    fn enter_manual_joint_or_launch(&mut self) {
        let selector = self.runtime.emitter_selector;
        // FUN_00424EE0 writes binding 1 when selector is zero, otherwise
        // binding 0. A null selected binding is a legitimate no-op.
        let binding_index = if selector == 0 { 1 } else { 0 };
        if let Some(binding_identity) = self.runtime.joint_bindings[binding_index] {
            self.stage = GenericEmitterStage::CommitGunJoint {
                binding_identity,
                selector,
            };
        } else {
            self.stage = GenericEmitterStage::ResolveManualLaunch;
        }
    }

    fn append_request(&self, auxiliary: bool) -> GenericEmitterAppendRequest {
        GenericEmitterAppendRequest {
            source_handle: self.request.source_handle,
            owner_handle: self.request.owner_handle,
            projectile_method: if auxiliary {
                GENERIC_PROJECTILE_AUXILIARY_METHOD
            } else {
                self.runtime.projectile_method
            },
            emitter_selector: self.runtime.emitter_selector as u16,
            direction_raw: if auxiliary {
                [0; 3]
            } else {
                self.direction_raw
            },
            time_offset_raw: if auxiliary { 0 } else { self.time_offset_raw },
            speed_field: if auxiliary {
                GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord
            } else {
                GenericEmitterSpeedField::Explicit(self.speed_raw)
            },
            auxiliary,
        }
    }

    /// Returns true only when the append succeeded and the transaction may
    /// continue. Every nonzero u32 is propagated with its bits unchanged.
    fn accept_append_result(&mut self, result_raw: u32) -> bool {
        if let Some(result) = NonZeroU32::new(result_raw) {
            self.stage =
                GenericEmitterStage::Complete(GenericEmitterCompletion::ReturnNonZero(result));
            false
        } else {
            true
        }
    }

    fn enter_after_first_append_pair(&mut self) {
        if self.request.descriptor.alternate_emitter_raw != 0 {
            let before_raw = self.runtime.emitter_selector;
            self.stage =
                GenericEmitterStage::CommitAlternateSelector(GenericEmitterSelectorWrite {
                    before_raw,
                    after_raw: u32::from(before_raw == 0),
                });
        } else {
            self.enter_after_selector_toggle();
        }
    }

    fn enter_after_selector_toggle(&mut self) {
        if !self.runtime.target_mode() && matches!(self.runtime.projectile_method, 10 | 0x16 | 0x1A)
        {
            self.stage = GenericEmitterStage::AppendManualRepeatPrimary;
        } else {
            self.enter_iteration_counter_commit();
        }
    }

    fn enter_iteration_counter_commit(&mut self) {
        if self.runtime.target_mode() {
            let cadence_before_raw = self.runtime.cadence_raw;
            let cadence_after_raw =
                cadence_before_raw.wrapping_add(self.request.descriptor.random_interval_us as i32);
            let time_offset_before_raw = self.time_offset_raw;
            let time_offset_after_raw = time_offset_before_raw
                .wrapping_add(self.request.descriptor.random_interval_us as i32);
            self.stage =
                GenericEmitterStage::CommitTargetCadence(GenericEmitterTargetCadenceWrite {
                    cadence_before_raw,
                    interval_us: self.request.descriptor.random_interval_us,
                    cadence_after_raw,
                    time_offset_before_raw,
                    time_offset_after_raw,
                });
        } else {
            let remaining_time_before_raw = self.runtime.remaining_time_raw;
            let remaining_bursts_before_raw = self.runtime.remaining_bursts_raw;
            let time_offset_before_raw = self.time_offset_raw;
            self.stage =
                GenericEmitterStage::CommitManualCounters(GenericEmitterManualCounterWrite {
                    remaining_time_before_raw,
                    remaining_time_after_raw: remaining_time_before_raw
                        .wrapping_sub(self.runtime.manual_step_raw),
                    remaining_bursts_before_raw,
                    remaining_bursts_after_raw: remaining_bursts_before_raw.wrapping_sub(1),
                    step_raw: self.runtime.manual_step_raw,
                    time_offset_before_raw,
                    time_offset_after_raw: time_offset_before_raw
                        .wrapping_add(self.runtime.manual_step_raw),
                });
        }
    }

    fn enter_sound_suffix(&mut self) {
        if self.runtime.sound_id == 0 {
            self.complete_zero();
        } else {
            self.stage = GenericEmitterStage::LookupSourceForSound;
        }
    }

    fn complete_zero(&mut self) {
        self.stage = GenericEmitterStage::Complete(GenericEmitterCompletion::ReturnZero);
    }
}

fn returned_entity_handle(resume: GenericEmitterResume) -> Option<u32> {
    match resume {
        GenericEmitterResume::EntityLookupResolved {
            entity: Some(entity),
            ..
        }
        | GenericEmitterResume::CachedSourceRefreshed {
            entity: Some(entity),
            ..
        } => Some(entity.handle),
        _ => None,
    }
}

fn spread_offset(sample: u16, spread_raw: u16) -> i16 {
    let scaled = ((u32::from(sample) * u32::from(spread_raw)) >> 15) as i16;
    scaled.wrapping_sub(spread_raw as i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSACTION_ID_RAW: u64 = 0xE117_0000_0000_0001;

    fn transaction_id(raw: u64) -> GenericEmitterTransactionId {
        GenericEmitterTransactionId::new(raw).expect("test transaction IDs are nonzero")
    }

    fn descriptor() -> ProjectileEmitterDescriptor {
        ProjectileEmitterDescriptor {
            projectile_method: 30,
            random_interval_us: 750_000,
            spread_raw: 100,
            aim_threshold_raw: 16_000,
            speed_override_raw: 0,
            target_axis_tolerance_raw: 0x0600,
            sound_id: 70,
            raw_word_at_0x12: 150,
            alternate_emitter_raw: 0,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        }
    }

    fn runtime() -> GenericEmitterRuntime {
        GenericEmitterRuntime {
            joint_bindings: [Some(0xAA), Some(0xBB)],
            projectile_method: 30,
            emitter_selector: 0,
            sound_id: 70,
            direct_mode: 0,
            remaining_time_raw: 0,
            manual_step_raw: 20_000,
            remaining_bursts_raw: 0,
            cadence_raw: 10_000,
            basis_adjustment_identity: None,
        }
    }

    fn request() -> GenericEmitterRequest {
        GenericEmitterRequest {
            source_handle: 0x04AA_0001,
            owner_handle: 0x04AA_0001,
            target_handle: 0x04BB_0001,
            explicit_target_position_raw: None,
            descriptor: descriptor(),
            runtime: runtime(),
            elapsed_us: 20_000,
        }
    }

    fn entity(handle: u32, position_raw: [i16; 3]) -> GenericEmitterEntitySnapshot {
        GenericEmitterEntitySnapshot {
            handle,
            allocation_identity: u64::from(handle) << 16 | 1,
            state_flags_at_0x08: 0,
            position_raw,
            velocity_raw: [10, 20, 30],
            orientation_raw: [0x100, 0x200, 0x300],
        }
    }

    fn start_machine(request: GenericEmitterRequest) -> GenericProjectileEmitterMachine {
        GenericProjectileEmitterMachine::start(transaction_id(TRANSACTION_ID_RAW), request)
    }

    #[test]
    fn aim_and_fire_invocation_maps_all_seven_arguments_without_allocating_identity() {
        let descriptor = descriptor();
        let runtime = runtime();
        let request: GenericEmitterRequest = crate::aim_and_fire::InvokeGenericEmitter {
            owner_entity_id: 0x0401_0001,
            secondary_owner_entity_id: 0x0402_0001,
            target_entity_id: 0x0403_0001,
            explicit_position_raw: Some([101, -202, 303]),
            sub_e_descriptor: descriptor,
            emitter_runtime: runtime,
            elapsed_micros: 0x1020_3040,
        }
        .into();
        assert_eq!(
            request,
            GenericEmitterRequest {
                source_handle: 0x0401_0001,
                owner_handle: 0x0402_0001,
                target_handle: 0x0403_0001,
                explicit_target_position_raw: Some([101, -202, 303]),
                descriptor,
                runtime,
                elapsed_us: 0x1020_3040,
            }
        );

        let caller_allocated_id = transaction_id(0xE117_0000_0000_0077);
        let mut machine = GenericProjectileEmitterMachine::start(caller_allocated_id, request);
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(receipt.transaction_id(), caller_allocated_id);
        assert!(matches!(
            action,
            GenericEmitterAction::LookupEntity {
                phase: GenericEmitterPhase::LookupEntrySource,
                handle: 0x0401_0001,
            }
        ));
    }

    fn issue(machine: &mut GenericProjectileEmitterMachine) -> IssuedGenericEmitterAction {
        let GenericEmitterPoll::Action(issued) = machine.poll() else {
            panic!("expected action, got {:?}", machine.poll());
        };
        issued
    }

    fn acknowledge(machine: &mut GenericProjectileEmitterMachine, phase: GenericEmitterPhase) {
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        assert_eq!(action.phase(), phase);
        machine
            .resume(receipt, GenericEmitterResume::Acknowledged { phase })
            .unwrap();
    }

    fn lookup(
        machine: &mut GenericProjectileEmitterMachine,
        phase: GenericEmitterPhase,
        result: Option<GenericEmitterEntitySnapshot>,
    ) {
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        assert_eq!(action.phase(), phase);
        machine
            .resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase,
                    entity: result,
                },
            )
            .unwrap();
    }

    fn draw(machine: &mut GenericProjectileEmitterMachine, phase: GenericEmitterPhase, value: u32) {
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        assert_eq!(action.phase(), phase);
        machine
            .resume(receipt, GenericEmitterResume::RandomDrawn { phase, value })
            .unwrap();
    }

    fn refresh_source(
        machine: &mut GenericProjectileEmitterMachine,
        point: GenericEmitterSourceRefreshPoint,
        result: Option<GenericEmitterEntitySnapshot>,
    ) {
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        let GenericEmitterAction::RefreshCachedSource {
            phase,
            source_handle,
            allocation_identity,
            point: action_point,
        } = action
        else {
            panic!("expected source refresh, got {action:?}");
        };
        assert_eq!(phase, GenericEmitterPhase::RefreshCachedSource);
        assert_eq!(source_handle, machine.request.source_handle);
        assert_eq!(
            allocation_identity,
            entity(machine.request.source_handle, [0; 3]).allocation_identity
        );
        assert_eq!(action_point, point);
        machine
            .resume(
                receipt,
                GenericEmitterResume::CachedSourceRefreshed {
                    phase,
                    entity: result,
                },
            )
            .unwrap();
    }

    fn expect_protocol_error(
        result: Result<(), GenericEmitterResumeFailure>,
        expected: GenericEmitterProtocolError,
    ) -> GenericEmitterReceipt {
        let failure = result.expect_err("malformed response must be rejected");
        assert_eq!(failure.error, expected);
        failure.receipt
    }

    fn append(
        machine: &mut GenericProjectileEmitterMachine,
        phase: GenericEmitterPhase,
        result_raw: u32,
    ) -> GenericEmitterAppendRequest {
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        let GenericEmitterAction::AppendTransient {
            phase: action_phase,
            request,
        } = action
        else {
            panic!("expected append, got {action:?}");
        };
        assert_eq!(action_phase, phase);
        machine
            .resume(
                receipt,
                GenericEmitterResume::AppendReturned { phase, result_raw },
            )
            .unwrap();
        request
    }

    fn enter_accepted_target_path(machine: &mut GenericProjectileEmitterMachine) {
        lookup(
            machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(machine.request.source_handle, [0, 200, 0])),
        );
        acknowledge(machine, GenericEmitterPhase::CommitInitialCadence);
        draw(machine, GenericEmitterPhase::DrawCadenceRandom, 0);
        lookup(
            machine,
            GenericEmitterPhase::LookupTargetPresence,
            Some(entity(machine.request.target_handle, [1_000, 300, -1_000])),
        );
        lookup(
            machine,
            GenericEmitterPhase::LookupTargetPosition,
            Some(entity(machine.request.target_handle, [1_000, 300, -1_000])),
        );
        refresh_source(
            machine,
            GenericEmitterSourceRefreshPoint::TargetAxisGate,
            Some(entity(machine.request.source_handle, [0, 200, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        assert_eq!(action.phase(), GenericEmitterPhase::EvaluateAim);
        machine
            .resume(
                receipt,
                GenericEmitterResume::AimEvaluated {
                    phase: GenericEmitterPhase::EvaluateAim,
                    error_raw: 0,
                },
            )
            .unwrap();
        let IssuedGenericEmitterAction { receipt, action } = issue(machine);
        assert_eq!(
            action.phase(),
            GenericEmitterPhase::EvaluateForwardHalfSpace
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::ForwardHalfSpaceEvaluated {
                    phase: GenericEmitterPhase::EvaluateForwardHalfSpace,
                    in_forward_half_space: true,
                },
            )
            .unwrap();
        draw(machine, GenericEmitterPhase::DrawSpreadX, 0x8000);
        draw(machine, GenericEmitterPhase::DrawSpreadZ, 0x4000);
    }

    #[test]
    fn action_is_issued_once_and_resume_requires_the_issued_phase() {
        let mut machine = start_machine(request());
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(
            action,
            GenericEmitterAction::LookupEntity {
                phase: GenericEmitterPhase::LookupEntrySource,
                handle: request().source_handle,
            }
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Awaiting(GenericEmitterPhase::LookupEntrySource)
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            )
            .unwrap();
        let unissued = GenericEmitterReceipt {
            transaction_id: transaction_id(TRANSACTION_ID_RAW),
            action_sequence: 2,
        };
        expect_protocol_error(
            machine.resume(
                unissued,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::CommitInitialCadence,
                },
            ),
            GenericEmitterProtocolError::NoOutstandingAction {
                transaction_id: transaction_id(TRANSACTION_ID_RAW),
                action_sequence: 2,
            },
        );
    }

    #[test]
    fn phase_and_resume_kind_errors_return_the_same_pending_receipt() {
        let mut machine = start_machine(request());
        let IssuedGenericEmitterAction { receipt, .. } = issue(&mut machine);
        let sequence = receipt.action_sequence();
        let receipt = expect_protocol_error(
            machine.resume(
                receipt,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::LookupEntrySource,
                },
            ),
            GenericEmitterProtocolError::UnexpectedResumeKind {
                phase: GenericEmitterPhase::LookupEntrySource,
            },
        );
        assert_eq!(receipt.action_sequence(), sequence);
        let receipt = expect_protocol_error(
            machine.resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::CommitInitialCadence,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            ),
            GenericEmitterProtocolError::PhaseMismatch {
                expected: GenericEmitterPhase::LookupEntrySource,
                actual: GenericEmitterPhase::CommitInitialCadence,
            },
        );
        assert_eq!(receipt.action_sequence(), sequence);
        machine
            .resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            )
            .unwrap();
    }

    #[test]
    fn receipt_from_another_transaction_cannot_advance_the_machine() {
        let mut first =
            GenericProjectileEmitterMachine::start(transaction_id(TRANSACTION_ID_RAW), request());
        let mut second = GenericProjectileEmitterMachine::start(
            transaction_id(TRANSACTION_ID_RAW + 1),
            request(),
        );
        let IssuedGenericEmitterAction {
            receipt: first_receipt,
            ..
        } = issue(&mut first);
        let IssuedGenericEmitterAction {
            receipt: second_receipt,
            ..
        } = issue(&mut second);

        let returned_first_receipt = expect_protocol_error(
            second.resume(
                first_receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            ),
            GenericEmitterProtocolError::ReceiptTransactionMismatch {
                expected: transaction_id(TRANSACTION_ID_RAW + 1),
                actual: transaction_id(TRANSACTION_ID_RAW),
            },
        );
        first
            .resume(
                returned_first_receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            )
            .unwrap();
        assert_eq!(
            second.poll(),
            GenericEmitterPoll::Awaiting(GenericEmitterPhase::LookupEntrySource)
        );
        second
            .resume(
                second_receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            )
            .unwrap();
    }

    #[test]
    fn consumed_receipt_cannot_complete_the_next_action() {
        let mut machine = start_machine(request());
        let IssuedGenericEmitterAction { receipt, .. } = issue(&mut machine);
        let consumed_sequence = receipt.action_sequence();
        machine
            .resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            )
            .unwrap();
        let IssuedGenericEmitterAction {
            receipt: current_receipt,
            ..
        } = issue(&mut machine);
        let duplicate = GenericEmitterReceipt {
            transaction_id: transaction_id(TRANSACTION_ID_RAW),
            action_sequence: consumed_sequence,
        };
        expect_protocol_error(
            machine.resume(
                duplicate,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::CommitInitialCadence,
                },
            ),
            GenericEmitterProtocolError::ReceiptSequenceMismatch {
                expected: current_receipt.action_sequence(),
                actual: consumed_sequence,
            },
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Awaiting(GenericEmitterPhase::CommitInitialCadence)
        );
        machine
            .resume(
                current_receipt,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::CommitInitialCadence,
                },
            )
            .unwrap();
    }

    #[test]
    fn cadence_write_precedes_random_and_rejection_returns_zero() {
        let mut machine = start_machine(request());
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request().source_handle, [0, 0, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        let GenericEmitterAction::CommitInitialCadence { write, .. } = action else {
            panic!("expected cadence write");
        };
        assert_eq!(
            write,
            GenericEmitterInitialCadenceWrite {
                before_raw: 10_000,
                elapsed_us: 20_000,
                after_raw: -10_000,
            }
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::CommitInitialCadence,
                },
            )
            .unwrap();
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(
            action,
            GenericEmitterAction::DrawSharedRandom {
                phase: GenericEmitterPhase::DrawCadenceRandom
            }
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::RandomDrawn {
                    phase: GenericEmitterPhase::DrawCadenceRandom,
                    value: 1,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
        assert_eq!(machine.runtime().cadence_raw, -10_000);
    }

    #[test]
    fn method_zero_target_still_aims_and_spreads_before_returning_zero() {
        let mut request = request();
        request.runtime.projectile_method = 0;
        let mut machine = start_machine(request);
        enter_accepted_target_path(&mut machine);
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
    }

    #[test]
    fn method_zero_direct_returns_before_reading_manual_state() {
        let mut request = request();
        request.runtime.direct_mode = 1;
        request.runtime.projectile_method = 0;
        request.runtime.remaining_time_raw = 20_000;
        request.runtime.remaining_bursts_raw = 1;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [0, 0, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
    }

    #[test]
    fn wrong_lookup_handle_returns_the_receipt_for_corrected_response() {
        let mut machine = start_machine(request());
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::LookupEntrySource);
        let receipt = expect_protocol_error(
            machine.resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(0x04CC_0001, [0, 0, 0])),
                },
            ),
            GenericEmitterProtocolError::LookupHandleMismatch {
                phase: GenericEmitterPhase::LookupEntrySource,
                expected: request().source_handle,
                actual: 0x04CC_0001,
            },
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Awaiting(GenericEmitterPhase::LookupEntrySource)
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    entity: Some(entity(request().source_handle, [0, 0, 0])),
                },
            )
            .unwrap();
    }

    #[test]
    fn wrong_target_handle_is_recoverable_at_target_lookup() {
        let mut machine = start_machine(request());
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request().source_handle, [0, 0, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        draw(&mut machine, GenericEmitterPhase::DrawCadenceRandom, 0);
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::LookupTargetPresence);
        let receipt = expect_protocol_error(
            machine.resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupTargetPresence,
                    entity: Some(entity(0x04CC_0001, [100, 0, 100])),
                },
            ),
            GenericEmitterProtocolError::LookupHandleMismatch {
                phase: GenericEmitterPhase::LookupTargetPresence,
                expected: request().target_handle,
                actual: 0x04CC_0001,
            },
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::EntityLookupResolved {
                    phase: GenericEmitterPhase::LookupTargetPresence,
                    entity: Some(entity(request().target_handle, [100, 0, 100])),
                },
            )
            .unwrap();
        assert_eq!(
            issue(&mut machine).action.phase(),
            GenericEmitterPhase::LookupTargetPosition
        );
    }

    #[test]
    fn wrong_refresh_handle_does_not_consume_the_pending_receipt() {
        let mut machine = start_machine(request());
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request().source_handle, [0, 200, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        draw(&mut machine, GenericEmitterPhase::DrawCadenceRandom, 0);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPresence,
            Some(entity(request().target_handle, [1_000, 300, -1_000])),
        );
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPosition,
            Some(entity(request().target_handle, [1_000, 300, -1_000])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::RefreshCachedSource);
        let receipt = expect_protocol_error(
            machine.resume(
                receipt,
                GenericEmitterResume::CachedSourceRefreshed {
                    phase: GenericEmitterPhase::RefreshCachedSource,
                    entity: Some(entity(0x04CC_0001, [0, 200, 0])),
                },
            ),
            GenericEmitterProtocolError::LookupHandleMismatch {
                phase: GenericEmitterPhase::RefreshCachedSource,
                expected: request().source_handle,
                actual: 0x04CC_0001,
            },
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::CachedSourceRefreshed {
                    phase: GenericEmitterPhase::RefreshCachedSource,
                    entity: Some(entity(request().source_handle, [0, 200, 0])),
                },
            )
            .unwrap();
        assert_eq!(
            issue(&mut machine).action.phase(),
            GenericEmitterPhase::EvaluateAim
        );
    }

    #[test]
    fn source_refresh_rejects_handle_reuse_with_a_new_allocation_identity() {
        let mut machine = start_machine(request());
        let original = entity(request().source_handle, [0, 200, 0]);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(original),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        draw(&mut machine, GenericEmitterPhase::DrawCadenceRandom, 0);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPresence,
            Some(entity(request().target_handle, [1_000, 300, -1_000])),
        );
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPosition,
            Some(entity(request().target_handle, [1_000, 300, -1_000])),
        );
        let mut replacement = original;
        replacement.allocation_identity = original.allocation_identity + 1;
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::TargetAxisGate,
            Some(replacement),
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Blocked(
                GenericEmitterBlock::CachedSourceAllocationIdentityMismatch {
                    point: GenericEmitterSourceRefreshPoint::TargetAxisGate,
                    expected: original.allocation_identity,
                    actual: replacement.allocation_identity,
                }
            )
        );
    }

    #[test]
    fn accepted_target_path_draws_spread_then_appends_and_sounds() {
        let mut machine = start_machine(request());
        enter_accepted_target_path(&mut machine);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetForLead,
            Some(entity(request().target_handle, [1_000, 300, -1_000])),
        );
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::TargetShot,
            Some(entity(request().source_handle, [0, 210, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        let GenericEmitterAction::ResolveTargetLaunch {
            request: launch, ..
        } = action
        else {
            panic!("expected target launch");
        };
        assert_eq!(launch.target_position_raw, [1_000, 300, -1_050]);
        assert_eq!(launch.cached_source.position_raw, [0, 210, 0]);
        machine
            .resume(
                receipt,
                GenericEmitterResume::TargetLaunchResolved {
                    phase: GenericEmitterPhase::ResolveTargetLaunch,
                    solution: GenericEmitterTargetLaunchSolution {
                        direction_raw: [100, 200, -300],
                        speed_raw: 3_000,
                        target_position_after_raw: [1_020, 310, -1_070],
                    },
                },
            )
            .unwrap();
        let primary = append(&mut machine, GenericEmitterPhase::AppendPrimary, 0);
        assert_eq!(primary.projectile_method, 30);
        assert_eq!(primary.direction_raw, [100, 200, -300]);
        assert_eq!(primary.source_handle, request().source_handle);
        assert_eq!(
            primary.speed_field,
            GenericEmitterSpeedField::Explicit(3_000)
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitTargetCadence);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupSourceForSound,
            Some(entity(request().source_handle, [9, 8, 7])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(
            action,
            GenericEmitterAction::PlayPositionalSound {
                phase: GenericEmitterPhase::PlaySound,
                sound_id: 70,
                source_position_raw: Some([9, 8, 7]),
                frequency_multiplier_16_16: 0x10000,
                volume_multiplier_16_16: 0x10000,
            }
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::PlaySound,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
    }

    #[test]
    fn target_catch_up_reuses_the_previous_launch_mutation() {
        let mut request = request();
        request.descriptor.random_interval_us = 5_000;
        request.runtime.sound_id = 0;
        let mut machine = start_machine(request);
        enter_accepted_target_path(&mut machine);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetForLead,
            Some(entity(request.target_handle, [1_000, 300, -1_000])),
        );
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::TargetShot,
            Some(entity(request.source_handle, [0, 200, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        let GenericEmitterAction::ResolveTargetLaunch {
            request: first_launch,
            ..
        } = action
        else {
            panic!("expected first target launch");
        };
        assert_eq!(first_launch.target_position_raw, [1_000, 300, -1_050]);
        let mutated_target = [1_125, 360, -900];
        machine
            .resume(
                receipt,
                GenericEmitterResume::TargetLaunchResolved {
                    phase: GenericEmitterPhase::ResolveTargetLaunch,
                    solution: GenericEmitterTargetLaunchSolution {
                        direction_raw: [10, 20, 30],
                        speed_raw: 2_500,
                        target_position_after_raw: mutated_target,
                    },
                },
            )
            .unwrap();
        append(&mut machine, GenericEmitterPhase::AppendPrimary, 0);
        acknowledge(&mut machine, GenericEmitterPhase::CommitTargetCadence);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetForLead,
            Some(entity(request.target_handle, [1_010, 310, -990])),
        );
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::TargetShot,
            Some(entity(request.source_handle, [1, 201, 2])),
        );
        let IssuedGenericEmitterAction { action, .. } = issue(&mut machine);
        let GenericEmitterAction::ResolveTargetLaunch {
            request: second_launch,
            ..
        } = action
        else {
            panic!("expected second target launch");
        };
        assert_eq!(second_launch.target_position_raw, mutated_target);
        assert_eq!(second_launch.cached_source.position_raw, [1, 201, 2]);
    }

    #[test]
    fn high_aim_threshold_still_calls_aim_but_bypasses_forward_half_space() {
        let mut request = request();
        request.descriptor.aim_threshold_raw = 0x8000;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [0, 0, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        draw(&mut machine, GenericEmitterPhase::DrawCadenceRandom, 0);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPresence,
            Some(entity(request.target_handle, [100, 0, 100])),
        );
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPosition,
            Some(entity(request.target_handle, [100, 0, 100])),
        );
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::TargetAxisGate,
            Some(entity(request.source_handle, [0, 0, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::EvaluateAim);
        machine
            .resume(
                receipt,
                GenericEmitterResume::AimEvaluated {
                    phase: GenericEmitterPhase::EvaluateAim,
                    error_raw: i32::MAX,
                },
            )
            .unwrap();
        let IssuedGenericEmitterAction { action, .. } = issue(&mut machine);
        assert_eq!(
            action,
            GenericEmitterAction::DrawSharedRandom {
                phase: GenericEmitterPhase::DrawSpreadX
            }
        );
    }

    #[test]
    fn missing_target_without_explicit_position_blocks_after_presence_lookup() {
        let mut machine = start_machine(request());
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request().source_handle, [0, 0, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        draw(&mut machine, GenericEmitterPhase::DrawCadenceRandom, 0);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupTargetPresence,
            None,
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Blocked(GenericEmitterBlock::MissingTargetWithoutExplicitPosition)
        );
    }

    #[test]
    fn auxiliary_append_failure_propagates_exact_bits_before_toggle_counter_and_sound() {
        let mut request = request();
        request.descriptor.auxiliary_command = 1;
        let mut machine = start_machine(request);
        enter_accepted_target_path(&mut machine);
        lookup(&mut machine, GenericEmitterPhase::LookupTargetForLead, None);
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::TargetShot,
            Some(entity(request.source_handle, [0, 200, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::ResolveTargetLaunch);
        machine
            .resume(
                receipt,
                GenericEmitterResume::TargetLaunchResolved {
                    phase: GenericEmitterPhase::ResolveTargetLaunch,
                    solution: GenericEmitterTargetLaunchSolution {
                        direction_raw: [1, 2, 3],
                        speed_raw: 4_000,
                        target_position_after_raw: [1_000, 300, -1_050],
                    },
                },
            )
            .unwrap();
        append(&mut machine, GenericEmitterPhase::AppendPrimary, 0);
        append(
            &mut machine,
            GenericEmitterPhase::AppendAuxiliary,
            0xDEAD_BEEF,
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnNonZero(
                NonZeroU32::new(0xDEAD_BEEF).unwrap()
            ))
        );
        assert_eq!(machine.runtime().emitter_selector, 0);
        assert_eq!(machine.time_offset_raw(), 0);
    }

    #[test]
    fn method_31_permission_enters_the_normal_manual_shot_path() {
        let mut request = request();
        request.runtime.direct_mode = 1;
        request.runtime.projectile_method = 31;
        request.runtime.remaining_time_raw = 20;
        request.runtime.remaining_bursts_raw = 1;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [7, 8, 9])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        let mut source = entity(request.source_handle, [7, 8, 9]);
        source.state_flags_at_0x08 = GENERIC_PROJECTILE_METHOD_31_PERMISSION_BIT;
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::Method31Gate,
            Some(source),
        );
        assert_eq!(
            issue(&mut machine).action,
            GenericEmitterAction::ReadManualAngleBinding {
                phase: GenericEmitterPhase::ReadManualAngleBinding,
            }
        );
    }

    #[test]
    fn rejected_method_31_with_zero_step_blocks_instead_of_looping() {
        let mut request = request();
        request.runtime.direct_mode = 1;
        request.runtime.projectile_method = 31;
        request.runtime.remaining_time_raw = 20;
        request.runtime.manual_step_raw = 0;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [7, 8, 9])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::Method31Gate,
            Some(entity(request.source_handle, [7, 8, 9])),
        );
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Blocked(GenericEmitterBlock::ZeroSuppressionStepWouldNotTerminate)
        );
    }

    #[test]
    fn rejected_manual_method_31_repeats_effect_then_countdown_and_stops() {
        let mut request = request();
        request.runtime.direct_mode = 1;
        request.runtime.projectile_method = 31;
        request.runtime.remaining_time_raw = 45;
        request.runtime.manual_step_raw = 20;
        request.runtime.remaining_bursts_raw = 9;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [7, 8, 9])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        for (index, expected_after) in [25, 5, -15].into_iter().enumerate() {
            let position = [7 + index as i16, 8, 9];
            refresh_source(
                &mut machine,
                if index == 0 {
                    GenericEmitterSourceRefreshPoint::Method31Gate
                } else {
                    GenericEmitterSourceRefreshPoint::Method31Rejection
                },
                Some(entity(request.source_handle, position)),
            );
            let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
            assert_eq!(
                action,
                GenericEmitterAction::EmitMethod31Rejection {
                    phase: GenericEmitterPhase::EmitMethod31Rejection,
                    source_handle: request.source_handle,
                    source_position_raw: position,
                    effect_class: 0x1C,
                }
            );
            machine
                .resume(
                    receipt,
                    GenericEmitterResume::Acknowledged {
                        phase: GenericEmitterPhase::EmitMethod31Rejection,
                    },
                )
                .unwrap();
            let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
            let GenericEmitterAction::CommitMethod31Suppression { write, .. } = action else {
                panic!("expected suppression write");
            };
            assert_eq!(write.after_raw, expected_after);
            machine
                .resume(
                    receipt,
                    GenericEmitterResume::Acknowledged {
                        phase: GenericEmitterPhase::CommitMethod31Suppression,
                    },
                )
                .unwrap();
        }
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
        assert_eq!(machine.runtime().remaining_time_raw, -15);
    }

    #[test]
    fn negative_method_31_step_preserves_wrapping_retail_subtraction() {
        let mut request = request();
        request.runtime.direct_mode = 1;
        request.runtime.projectile_method = 31;
        request.runtime.remaining_time_raw = 1;
        request.runtime.manual_step_raw = i32::MIN;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [7, 8, 9])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::Method31Gate,
            Some(entity(request.source_handle, [7, 8, 9])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::EmitMethod31Rejection);
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        let GenericEmitterAction::CommitMethod31Suppression { write, .. } = action else {
            panic!("expected suppression write");
        };
        assert_eq!(write.after_raw, i32::MIN.wrapping_neg().wrapping_add(1));
        machine
            .resume(
                receipt,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::CommitMethod31Suppression,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
    }

    #[test]
    fn manual_special_method_orders_joint_pair_toggle_repeat_pair_then_counters() {
        let mut request = request();
        request.descriptor.auxiliary_command = 1;
        request.descriptor.alternate_emitter_raw = 1;
        request.runtime.direct_mode = 1;
        request.runtime.projectile_method = 10;
        request.runtime.sound_id = 0;
        request.runtime.remaining_time_raw = 20;
        request.runtime.manual_step_raw = 20;
        request.runtime.remaining_bursts_raw = 1;
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [0, 0, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(
            action,
            GenericEmitterAction::ReadManualAngleBinding {
                phase: GenericEmitterPhase::ReadManualAngleBinding,
            }
        );
        machine
            .resume(
                receipt,
                GenericEmitterResume::ManualAngleBindingRead {
                    phase: GenericEmitterPhase::ReadManualAngleBinding,
                    angle_raw: Some(0x1234),
                },
            )
            .unwrap();
        refresh_source(
            &mut machine,
            GenericEmitterSourceRefreshPoint::ManualShot,
            Some(entity(request.source_handle, [0, 0, 0])),
        );
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::ResolveManualBasis);
        let basis = GenericEmitterManualBasisSolution {
            basis_raw: [1, 2, 3, 4, 5, 6, 7, 8, 9],
            sine_raw: 0x1111,
            cosine_raw: 0x2222,
        };
        machine
            .resume(
                receipt,
                GenericEmitterResume::ManualBasisResolved {
                    phase: GenericEmitterPhase::ResolveManualBasis,
                    solution: basis,
                },
            )
            .unwrap();
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        let GenericEmitterAction::CommitGunJoint {
            binding_identity,
            emitter_selector,
            ..
        } = action
        else {
            panic!("expected joint write");
        };
        assert_eq!((binding_identity, emitter_selector), (0xBB, 0));
        machine
            .resume(
                receipt,
                GenericEmitterResume::Acknowledged {
                    phase: GenericEmitterPhase::CommitGunJoint,
                },
            )
            .unwrap();
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        let GenericEmitterAction::ResolveManualLaunch {
            request: launch, ..
        } = action
        else {
            panic!("expected post-joint manual launch");
        };
        assert_eq!(launch.basis, basis);
        machine
            .resume(
                receipt,
                GenericEmitterResume::ManualLaunchResolved {
                    phase: GenericEmitterPhase::ResolveManualLaunch,
                    solution: GenericEmitterManualLaunchSolution {
                        direction_raw: [4, 5, 6],
                    },
                },
            )
            .unwrap();
        let primary = append(&mut machine, GenericEmitterPhase::AppendPrimary, 0);
        assert!(!primary.auxiliary);
        assert_eq!(primary.direction_raw, [4, 5, 6]);
        assert_eq!(primary.speed_field, GenericEmitterSpeedField::Explicit(0));
        let auxiliary = append(&mut machine, GenericEmitterPhase::AppendAuxiliary, 0);
        assert!(auxiliary.auxiliary);
        assert_eq!(
            auxiliary.speed_field,
            GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitAlternateSelector);
        let repeat = append(
            &mut machine,
            GenericEmitterPhase::AppendManualRepeatPrimary,
            0,
        );
        assert_eq!(repeat.emitter_selector, 1);
        assert!(
            append(
                &mut machine,
                GenericEmitterPhase::AppendManualRepeatAuxiliary,
                0
            )
            .auxiliary
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitManualCounters);
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
        assert_eq!(machine.runtime().emitter_selector, 1);
        assert_eq!(machine.runtime().remaining_time_raw, 0);
        assert_eq!(machine.runtime().remaining_bursts_raw, 0);
        assert_eq!(machine.time_offset_raw(), 20);
    }

    #[test]
    fn manual_angle_is_reread_each_iteration_and_null_joint_is_a_no_op() {
        let mut request = request();
        request.runtime.direct_mode = 1;
        request.runtime.sound_id = 0;
        request.runtime.remaining_time_raw = 40;
        request.runtime.manual_step_raw = 20;
        request.runtime.remaining_bursts_raw = 2;
        request.runtime.joint_bindings = [None, None];
        let mut machine = start_machine(request);
        lookup(
            &mut machine,
            GenericEmitterPhase::LookupEntrySource,
            Some(entity(request.source_handle, [0, 0, 0])),
        );
        acknowledge(&mut machine, GenericEmitterPhase::CommitInitialCadence);

        for (iteration, angle_raw) in [0x1111, 0x2222].into_iter().enumerate() {
            let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
            assert_eq!(
                action,
                GenericEmitterAction::ReadManualAngleBinding {
                    phase: GenericEmitterPhase::ReadManualAngleBinding,
                }
            );
            machine
                .resume(
                    receipt,
                    GenericEmitterResume::ManualAngleBindingRead {
                        phase: GenericEmitterPhase::ReadManualAngleBinding,
                        angle_raw: Some(angle_raw),
                    },
                )
                .unwrap();
            refresh_source(
                &mut machine,
                GenericEmitterSourceRefreshPoint::ManualShot,
                Some(entity(request.source_handle, [iteration as i16, 0, 0])),
            );
            let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
            let GenericEmitterAction::ResolveManualBasis {
                request: basis_request,
                ..
            } = action
            else {
                panic!("expected manual basis");
            };
            assert_eq!(basis_request.manual_angle_raw, Some(angle_raw));
            machine
                .resume(
                    receipt,
                    GenericEmitterResume::ManualBasisResolved {
                        phase: GenericEmitterPhase::ResolveManualBasis,
                        solution: GenericEmitterManualBasisSolution {
                            basis_raw: [iteration as i32; 9],
                            sine_raw: 1,
                            cosine_raw: 2,
                        },
                    },
                )
                .unwrap();
            let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
            assert_eq!(action.phase(), GenericEmitterPhase::ResolveManualLaunch);
            machine
                .resume(
                    receipt,
                    GenericEmitterResume::ManualLaunchResolved {
                        phase: GenericEmitterPhase::ResolveManualLaunch,
                        solution: GenericEmitterManualLaunchSolution {
                            direction_raw: [iteration as i32, 2, 3],
                        },
                    },
                )
                .unwrap();
            append(&mut machine, GenericEmitterPhase::AppendPrimary, 0);
            acknowledge(&mut machine, GenericEmitterPhase::CommitManualCounters);
        }
        assert_eq!(
            machine.poll(),
            GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero)
        );
    }

    #[test]
    fn external_failure_is_durable() {
        let mut machine = start_machine(request());
        let IssuedGenericEmitterAction { receipt, action } = issue(&mut machine);
        assert_eq!(action.phase(), GenericEmitterPhase::LookupEntrySource);
        machine
            .resume(
                receipt,
                GenericEmitterResume::Blocked {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
                },
            )
            .unwrap();
        let block = GenericEmitterBlock::External {
            phase: GenericEmitterPhase::LookupEntrySource,
            reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
        };
        assert_eq!(machine.poll(), GenericEmitterPoll::Blocked(block));
        let stale = GenericEmitterReceipt {
            transaction_id: transaction_id(TRANSACTION_ID_RAW),
            action_sequence: 1,
        };
        expect_protocol_error(
            machine.resume(
                stale,
                GenericEmitterResume::Blocked {
                    phase: GenericEmitterPhase::LookupEntrySource,
                    reason: GenericEmitterExternalBlock::EntityLookupUnavailable,
                },
            ),
            GenericEmitterProtocolError::NoOutstandingAction {
                transaction_id: transaction_id(TRANSACTION_ID_RAW),
                action_sequence: 1,
            },
        );
    }
}
