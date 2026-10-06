//! Receipt-bound full-health Working Factory production frames.
//!
//! Retail `FUN_00419010` commits timer and phase prefixes before it crosses
//! entity allocation, relation, presentation, positional-sound, and
//! publication boundaries.
//! The older [`crate::factory_production::FactoryProductionRuntime`]
//! observational replay intentionally rolls an unresolved frame back, so it
//! cannot safely drive a live world. This module owns the complementary
//! non-replayable transaction for the fully staffed, full-health phase-0..4
//! path.
//!
//! Damaged factories, cached-health changes, progressive death, and
//! understaffed updates are explicit admission failures. They precede or
//! replace production in retail and must be composed by an outer factory
//! owner; treating any of them as an ordinary production frame would silently
//! skip staffing loss, repair, or death.

use std::num::{NonZeroU32, NonZeroU64};

use crate::factory_production::{
    FactoryProductionPhase, FactoryProductionRuntime, FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
    FACTORY_MATERIALISER_ENTITY_TYPE, FACTORY_OUTPUT_CONVERSION_GATE_RAW,
    FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID, FACTORY_OUTPUT_CONVERSION_SOUND_ID,
    FACTORY_PICKUP_ENTITY_TYPE,
};

/// Retail Working Factory update.
pub const FACTORY_PRODUCTION_UPDATE_ADDRESS: u32 = 0x0041_9010;
/// Retail entity allocator used by both production spawn paths.
pub const FACTORY_PRODUCTION_SPAWN_ADDRESS: u32 = 0x0043_8080;
/// Retail output/materialiser relation installer.
pub const FACTORY_MATERIALISER_LINK_ADDRESS: u32 = 0x0040_8F00;
/// Exact lifetime word cleared after the phase-0 spawn attempt.
pub const FACTORY_ENTITY_LIFETIME_OFFSET: u32 = 0x74;
/// Entity dirty bit set after an active phase and normal status publication.
pub const FACTORY_ENTITY_DIRTY_FLAG: u32 = 0x80;
/// Phase-2 output and materialiser Z displacement in signed raw 8.8 units.
pub const FACTORY_CONVERTED_OUTPUT_Z_OFFSET_RAW: i16 = -250;

/// Caller-owned identity for one detached production frame.
///
/// IDs must remain unique among every live factory frame and retained adapter
/// journal entry. A factory entity handle is not a transaction identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryProductionTransactionId(NonZeroU64);

impl FactoryProductionTransactionId {
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

/// Allocation and mutable-state generation authenticated at frame entry.
///
/// The owner adapter reserves this version for the transaction. Acknowledged
/// actions are mutations inside that lease; an unrelated mutation must
/// durably block the pending action before performing any side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactoryProductionEntityVersion {
    pub entity_id: u32,
    pub allocation_identity: u64,
    pub state_version: u64,
}

/// Complete immutable snapshot consumed before the first action can escape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionFrameRequest {
    pub factory: FactoryProductionEntityVersion,
    pub position_raw: [i16; 3],
    /// Signed descriptor words at factory type resource Sub-M
    /// `+0x0C/+0x0E/+0x10`.
    pub pickup_spawn_offset_raw: [i16; 3],
    pub current_health_raw: i32,
    pub maximum_health_raw: i32,
    /// Runtime component `+0x94`; any nonzero value delegates to
    /// `FUN_00419B50` before production.
    pub progressive_death_elapsed_raw: i32,
    pub production: FactoryProductionRuntime,
    /// Cached three-bit presentation state at retail component `+0xB4`.
    pub animation_state_raw: u32,
    pub elapsed_micros: u32,
    /// Exact current world style consumed by `FUN_0042EB70`.
    pub world_style_raw: u32,
    /// Retail global `DAT_004F741C != 0`.
    pub phase1_presentation_enabled: bool,
    /// Nonzero retail `param_3` suppresses status publication unless phase 1
    /// dispatches its presentation callback.
    pub suppress_status_publication: bool,
}

/// A path owned by an outer factory transaction, not this production frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionAdmissionRejection {
    ProgressiveDeathActive,
    DamagedFactory,
    CachedHealthTransition,
    UnderstaffedFactory,
}

/// The request's explicit-handle field at `+0x00`.
///
/// Both recovered production paths write zero, which asks
/// `FUN_004104B0` to allocate a new handle. This is not a list anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryRequestedEntityHandle {
    Allocate,
}

/// Exact semantic contents of the zero-filled 0x4C-byte retail spawn request.
///
/// Fields not represented here are proven zero on both factory paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryEntitySpawnRequest {
    pub requested_handle: FactoryRequestedEntityHandle,
    pub entity_type: u32,
    pub position_raw: [i16; 3],
    /// Request `+0x20`, copied to newborn entity `+0x88`.
    pub spawn_parameter_6: u32,
}

/// Allocation-owned proof for one Type-61 product emitted by the bounded
/// Working Factory bridge.
///
/// Retail constructs the pickup through the ordinary entity constructor. Its
/// singleton `Always -> Power Up` behavior list still consumes one shared RNG
/// word. Retaining the complete source lease, spawn request, and sampled word
/// distinguishes that exact dynamic birth from an arbitrary runtime Type-61
/// carrying the same payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryType61BirthProvenance {
    source_factory: FactoryProductionEntityVersion,
    spawn_request: FactoryEntitySpawnRequest,
    selector_rng_word: u16,
}

impl FactoryType61BirthProvenance {
    pub(crate) const fn new(
        source_factory: FactoryProductionEntityVersion,
        spawn_request: FactoryEntitySpawnRequest,
        selector_rng_word: u16,
    ) -> Self {
        Self {
            source_factory,
            spawn_request,
            selector_rng_word,
        }
    }

    pub const fn source_factory(self) -> FactoryProductionEntityVersion {
        self.source_factory
    }

    pub const fn spawn_request(self) -> FactoryEntitySpawnRequest {
        self.spawn_request
    }

    pub const fn selector_rng_word(self) -> u16 {
        self.selector_rng_word
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorySpawnRole {
    Pickup,
    ConvertedOutput,
    Materialiser,
}

/// Stable identity returned by a successful host allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactorySpawnedEntity {
    pub entity_id: NonZeroU32,
    pub allocation_identity: u64,
}

/// A spawn failure means the adapter has also dispatched retail's returned
/// error object. It is an ordinary branch result, not a protocol block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryEntitySpawnResult {
    Spawned(FactorySpawnedEntity),
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryPickupPresence {
    Active,
    Gone,
}

/// Exact low-word status values written by `FUN_00419630`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryStatusPublication {
    pub current_scientists_raw: u16,
    pub scientist_capacity_raw: u16,
    /// Zero-extended selector byte at factory runtime `+0x00`.
    ///
    /// This is deliberately not the packed output descriptor. Retail
    /// `FUN_00419630` reads one byte even though the destination is a word.
    pub output_selector_raw: u16,
    pub production_or_cooldown_ratio_raw: u16,
    pub delivery_or_cooldown_ratio_raw: u16,
    pub understaffed_ratio_raw: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryAnimationRange {
    pub start_raw_16_16: i32,
    pub end_raw_16_16: i32,
}

/// Exact `FUN_00418F60` channel update derived from the old/new `+0xB4` bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryAnimationTransition {
    pub previous_state_raw: u32,
    pub next_state_raw: u32,
    pub primary: Option<FactoryAnimationRange>,
    pub secondary: Option<FactoryAnimationRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryPositionalSoundRequest {
    pub sound_id: u16,
    pub position_raw: [i16; 3],
    pub gain_raw_16_16: i32,
    pub rate_raw_16_16: i32,
}

/// Externally visible action boundary in exact retail order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactoryProductionActionPhase {
    SpawnPickup,
    LinkPickupOwner,
    EmitHighThresholdDirectText,
    ClearFactoryLifetime,
    DispatchPhase1Presentation,
    QueryPickupPresence,
    QueueConversionHudResource,
    SpawnConvertedOutput,
    LinkConvertedOutputOwner,
    SpawnMaterialiser,
    LinkMaterialiserOutput,
    PlayConversionPositionalSound,
    ApplyAnimationTransition,
    PublishStatus,
    MarkEntityDirty,
}

/// One non-replayable adapter action.
#[derive(Debug, PartialEq, Eq)]
pub enum FactoryProductionAction {
    SpawnEntity {
        phase: FactoryProductionActionPhase,
        role: FactorySpawnRole,
        request: FactoryEntitySpawnRequest,
    },
    /// Perform retail's fresh child lookup and write `+0x60` only when found.
    ///
    /// A normal lookup miss is an acknowledged no-op, not a block: retail
    /// still commits the handle/stock or staffing prefix that follows.
    LinkOwner {
        phase: FactoryProductionActionPhase,
        spawned: FactorySpawnedEntity,
        owner_factory: FactoryProductionEntityVersion,
    },
    EmitDirectText {
        phase: FactoryProductionActionPhase,
        direct_text_id: u16,
        sub_parameter: u32,
    },
    ClearFactoryLifetime {
        phase: FactoryProductionActionPhase,
        factory: FactoryProductionEntityVersion,
        lifetime_offset: u32,
        value_raw: u32,
    },
    /// Invoke the phase-1 presentation callback, dispatch the returned
    /// presentation object, and dispose it before acknowledging the action.
    ///
    /// Acknowledgement means that complete call sequence ran; adapters must
    /// not acknowledge after performing only a prefix of it.
    DispatchPhase1Presentation {
        phase: FactoryProductionActionPhase,
        factory: FactoryProductionEntityVersion,
    },
    QueryPickupPresence {
        phase: FactoryProductionActionPhase,
        pickup_handle: u32,
    },
    QueueHudResource {
        phase: FactoryProductionActionPhase,
        resource_id: u16,
    },
    /// Invoke `FUN_00408F00(materialiser, output)` and dispose its returned
    /// object as required. Retail ignores the call's result and proceeds to
    /// the positional sound, so a nonzero result is still acknowledged.
    LinkMaterialiserOutput {
        phase: FactoryProductionActionPhase,
        materialiser: FactorySpawnedEntity,
        output: FactorySpawnedEntity,
    },
    /// Play retail/demo's fixed positional sound through
    /// `FUN_0044F480`/`FUN_0044EC80` and dispose the returned object before
    /// acknowledging the action.
    ///
    /// As with every compound action in this protocol, adapters must preflight
    /// the whole operation: `Blocked` means none of its externally visible
    /// work was performed.
    PlayConversionPositionalSound {
        phase: FactoryProductionActionPhase,
        request: FactoryPositionalSoundRequest,
    },
    /// Apply both recovered animation ranges and write `next_state_raw` to the
    /// Factory's cached animation-state field (`+0xB4`) before acknowledging.
    ApplyAnimationTransition {
        phase: FactoryProductionActionPhase,
        factory: FactoryProductionEntityVersion,
        transition: FactoryAnimationTransition,
    },
    PublishStatus {
        phase: FactoryProductionActionPhase,
        factory: FactoryProductionEntityVersion,
        status: FactoryStatusPublication,
    },
    MarkEntityDirty {
        phase: FactoryProductionActionPhase,
        factory: FactoryProductionEntityVersion,
        flag: u32,
    },
}

impl FactoryProductionAction {
    pub const fn phase(&self) -> FactoryProductionActionPhase {
        match self {
            Self::SpawnEntity { phase, .. }
            | Self::LinkOwner { phase, .. }
            | Self::EmitDirectText { phase, .. }
            | Self::ClearFactoryLifetime { phase, .. }
            | Self::DispatchPhase1Presentation { phase, .. }
            | Self::QueryPickupPresence { phase, .. }
            | Self::QueueHudResource { phase, .. }
            | Self::LinkMaterialiserOutput { phase, .. }
            | Self::PlayConversionPositionalSound { phase, .. }
            | Self::ApplyAnimationTransition { phase, .. }
            | Self::PublishStatus { phase, .. }
            | Self::MarkEntityDirty { phase, .. } => *phase,
        }
    }
}

/// Linear proof that one exact action was issued.
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryProductionReceipt {
    transaction_id: FactoryProductionTransactionId,
    action_sequence: u64,
    factory: FactoryProductionEntityVersion,
}

impl FactoryProductionReceipt {
    pub const fn transaction_id(&self) -> FactoryProductionTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }

    pub const fn factory(&self) -> FactoryProductionEntityVersion {
        self.factory
    }
}

/// Factory-owned state that retail has already mutated before the accompanying
/// external call.
///
/// The adapter must validate the factory lease and persist this complete
/// snapshot before executing [`IssuedFactoryProductionAction::action`]. This
/// makes prefix writes observable in their retail order and prevents a later
/// spawn or callback block from stranding them inside the detached machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionPreActionState {
    pub factory: FactoryProductionEntityVersion,
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
}

/// A linear external action together with its mandatory state-write prefix.
///
/// Adapter order is:
///
/// 1. validate `receipt` and `before_action.factory`;
/// 2. persist all of `before_action`;
/// 3. execute `action`;
/// 4. resume the machine with the same receipt.
///
/// A non-state block may be reported only after step 2. If step 2 itself is
/// unavailable, execute no part of `action` and use
/// [`FactoryProductionExternalBlock::FactoryStateCommitUnavailable`].
#[derive(Debug, PartialEq, Eq)]
pub struct IssuedFactoryProductionAction {
    pub receipt: FactoryProductionReceipt,
    pub before_action: FactoryProductionPreActionState,
    pub action: FactoryProductionAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionExternalBlock {
    FactoryStateCommitUnavailable,
    EntitySpawnUnavailable,
    EntityMutationUnavailable,
    DirectTextUnavailable,
    LifetimeWriteUnavailable,
    PresentationUnavailable,
    PickupLookupUnavailable,
    HudResourceUnavailable,
    MaterialiserLinkUnavailable,
    PositionalSoundUnavailable,
    AnimationUnavailable,
    StatusPublicationUnavailable,
    DirtyFlagUnavailable,
}

impl FactoryProductionExternalBlock {
    const fn matches_phase(self, phase: FactoryProductionActionPhase) -> bool {
        if matches!(self, Self::FactoryStateCommitUnavailable) {
            return true;
        }
        matches!(
            (self, phase),
            (
                Self::EntitySpawnUnavailable,
                FactoryProductionActionPhase::SpawnPickup
                    | FactoryProductionActionPhase::SpawnConvertedOutput
                    | FactoryProductionActionPhase::SpawnMaterialiser
            ) | (
                Self::EntityMutationUnavailable,
                FactoryProductionActionPhase::LinkPickupOwner
                    | FactoryProductionActionPhase::LinkConvertedOutputOwner
            ) | (
                Self::DirectTextUnavailable,
                FactoryProductionActionPhase::EmitHighThresholdDirectText
            ) | (
                Self::LifetimeWriteUnavailable,
                FactoryProductionActionPhase::ClearFactoryLifetime
            ) | (
                Self::PresentationUnavailable,
                FactoryProductionActionPhase::DispatchPhase1Presentation
            ) | (
                Self::PickupLookupUnavailable,
                FactoryProductionActionPhase::QueryPickupPresence
            ) | (
                Self::HudResourceUnavailable,
                FactoryProductionActionPhase::QueueConversionHudResource
            ) | (
                Self::MaterialiserLinkUnavailable,
                FactoryProductionActionPhase::LinkMaterialiserOutput
            ) | (
                Self::PositionalSoundUnavailable,
                FactoryProductionActionPhase::PlayConversionPositionalSound
            ) | (
                Self::AnimationUnavailable,
                FactoryProductionActionPhase::ApplyAnimationTransition
            ) | (
                Self::StatusPublicationUnavailable,
                FactoryProductionActionPhase::PublishStatus
            ) | (
                Self::DirtyFlagUnavailable,
                FactoryProductionActionPhase::MarkEntityDirty
            )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionResume {
    Acknowledged {
        phase: FactoryProductionActionPhase,
    },
    SpawnCompleted {
        phase: FactoryProductionActionPhase,
        result: FactoryEntitySpawnResult,
    },
    PickupPresence {
        phase: FactoryProductionActionPhase,
        presence: FactoryPickupPresence,
    },
    Blocked {
        phase: FactoryProductionActionPhase,
        reason: FactoryProductionExternalBlock,
    },
}

impl FactoryProductionResume {
    pub const fn phase(self) -> FactoryProductionActionPhase {
        match self {
            Self::Acknowledged { phase }
            | Self::SpawnCompleted { phase, .. }
            | Self::PickupPresence { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionBlock {
    pub factory: FactoryProductionEntityVersion,
    pub phase: FactoryProductionActionPhase,
    pub reason: FactoryProductionExternalBlock,
    /// Durable prefix state reached before the unavailable call.
    ///
    /// A block is terminal and guarantees that the pending action performed no
    /// side effect. For every reason except `FactoryStateCommitUnavailable`,
    /// the adapter has already persisted this state from the issued
    /// `before_action` snapshot. For that one reason this is the recovery
    /// snapshot that still needs persistence. Retain it either way;
    /// reconstructing the original frame could replay an earlier spawn.
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub status_published: bool,
    pub dirty_flag_written: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionCompletion {
    pub factory: FactoryProductionEntityVersion,
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub status_published: bool,
    pub dirty_flag_written: bool,
}

/// Authoritative detached state retained while a production transaction is
/// still in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryProductionDetachedState {
    pub production: FactoryProductionRuntime,
    pub animation_state_raw: u32,
    pub status_published: bool,
    pub dirty_flag_written: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FactoryProductionPoll {
    Action(IssuedFactoryProductionAction),
    Awaiting(FactoryProductionActionPhase),
    Blocked(FactoryProductionBlock),
    Complete(FactoryProductionCompletion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactoryProductionProtocolError {
    NoOutstandingAction {
        transaction_id: FactoryProductionTransactionId,
        action_sequence: u64,
    },
    ReceiptTransactionMismatch {
        expected: FactoryProductionTransactionId,
        actual: FactoryProductionTransactionId,
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
        expected: FactoryProductionActionPhase,
        actual: FactoryProductionActionPhase,
    },
    ResponseKindMismatch {
        phase: FactoryProductionActionPhase,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct FactoryProductionResumeFailure {
    pub receipt: FactoryProductionReceipt,
    pub error: FactoryProductionProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactoryProductionStage {
    Start,
    SpawnPickup(FactoryEntitySpawnRequest),
    LinkPickupOwner(FactorySpawnedEntity),
    EmitHighThresholdDirectText,
    ClearFactoryLifetime,
    DispatchPhase1Presentation,
    QueryPickupPresence,
    QueueConversionHudResource,
    SpawnConvertedOutput(FactoryEntitySpawnRequest),
    LinkConvertedOutputOwner(FactorySpawnedEntity),
    SpawnMaterialiser {
        output: FactorySpawnedEntity,
        request: FactoryEntitySpawnRequest,
    },
    LinkMaterialiserOutput {
        materialiser: FactorySpawnedEntity,
        output: FactorySpawnedEntity,
    },
    PlayConversionPositionalSound(FactoryPositionalSoundRequest),
    ApplyAnimationTransition(FactoryAnimationTransition),
    PublishStatus(FactoryStatusPublication),
    MarkEntityDirty,
    Blocked(FactoryProductionBlock),
    Complete,
}

impl FactoryProductionStage {
    const fn action_phase(self) -> Option<FactoryProductionActionPhase> {
        Some(match self {
            Self::SpawnPickup(_) => FactoryProductionActionPhase::SpawnPickup,
            Self::LinkPickupOwner(_) => FactoryProductionActionPhase::LinkPickupOwner,
            Self::EmitHighThresholdDirectText => {
                FactoryProductionActionPhase::EmitHighThresholdDirectText
            }
            Self::ClearFactoryLifetime => FactoryProductionActionPhase::ClearFactoryLifetime,
            Self::DispatchPhase1Presentation => {
                FactoryProductionActionPhase::DispatchPhase1Presentation
            }
            Self::QueryPickupPresence => FactoryProductionActionPhase::QueryPickupPresence,
            Self::QueueConversionHudResource => {
                FactoryProductionActionPhase::QueueConversionHudResource
            }
            Self::SpawnConvertedOutput(_) => FactoryProductionActionPhase::SpawnConvertedOutput,
            Self::LinkConvertedOutputOwner(_) => {
                FactoryProductionActionPhase::LinkConvertedOutputOwner
            }
            Self::SpawnMaterialiser { .. } => FactoryProductionActionPhase::SpawnMaterialiser,
            Self::LinkMaterialiserOutput { .. } => {
                FactoryProductionActionPhase::LinkMaterialiserOutput
            }
            Self::PlayConversionPositionalSound(_) => {
                FactoryProductionActionPhase::PlayConversionPositionalSound
            }
            Self::ApplyAnimationTransition(_) => {
                FactoryProductionActionPhase::ApplyAnimationTransition
            }
            Self::PublishStatus(_) => FactoryProductionActionPhase::PublishStatus,
            Self::MarkEntityDirty => FactoryProductionActionPhase::MarkEntityDirty,
            Self::Start | Self::Blocked(_) | Self::Complete => return None,
        })
    }
}

/// Non-replayable, detached fully staffed/full-health production frame.
///
/// Deliberately neither `Copy` nor `Clone`: duplication would duplicate spawn,
/// relation, presentation, positional-sound, and publication side effects.
#[derive(Debug, PartialEq, Eq)]
pub struct FactoryProductionMachine {
    transaction_id: FactoryProductionTransactionId,
    request: FactoryProductionFrameRequest,
    production: FactoryProductionRuntime,
    animation_state_raw: u32,
    stage: FactoryProductionStage,
    active_phase_marks_dirty: bool,
    status_publication_enabled: bool,
    status_published: bool,
    dirty_flag_written: bool,
    next_action_sequence: u64,
    issued_action_sequence: Option<u64>,
}

impl FactoryProductionMachine {
    pub fn preflight(
        transaction_id: FactoryProductionTransactionId,
        request: FactoryProductionFrameRequest,
    ) -> Result<Self, FactoryProductionAdmissionRejection> {
        if request.progressive_death_elapsed_raw != 0 {
            return Err(FactoryProductionAdmissionRejection::ProgressiveDeathActive);
        }
        if request.production.cached_health_raw != request.current_health_raw {
            return Err(FactoryProductionAdmissionRejection::CachedHealthTransition);
        }
        if request.current_health_raw != request.maximum_health_raw {
            return Err(FactoryProductionAdmissionRejection::DamagedFactory);
        }
        if request.production.current_scientists_raw < request.production.scientist_capacity_raw {
            return Err(FactoryProductionAdmissionRejection::UnderstaffedFactory);
        }

        Ok(Self {
            transaction_id,
            production: request.production,
            animation_state_raw: request.animation_state_raw,
            request,
            stage: FactoryProductionStage::Start,
            active_phase_marks_dirty: false,
            status_publication_enabled: !request.suppress_status_publication,
            status_published: false,
            dirty_flag_written: false,
            next_action_sequence: 1,
            issued_action_sequence: None,
        })
    }

    pub const fn production(&self) -> FactoryProductionRuntime {
        self.production
    }

    pub const fn detached_state(&self) -> FactoryProductionDetachedState {
        FactoryProductionDetachedState {
            production: self.production,
            animation_state_raw: self.animation_state_raw,
            status_published: self.status_published,
            dirty_flag_written: self.dirty_flag_written,
        }
    }

    pub fn poll(&mut self) -> FactoryProductionPoll {
        self.advance_internal_stages();
        let Some(phase) = self.stage.action_phase() else {
            return match self.stage {
                FactoryProductionStage::Blocked(block) => FactoryProductionPoll::Blocked(block),
                FactoryProductionStage::Complete => {
                    FactoryProductionPoll::Complete(self.completion())
                }
                FactoryProductionStage::Start => {
                    unreachable!("internal stages are exhausted before polling")
                }
                _ => unreachable!("action stages expose a phase"),
            };
        };
        if self.issued_action_sequence.is_some() {
            return FactoryProductionPoll::Awaiting(phase);
        }

        let action = self.action_for_stage(phase);
        let action_sequence = self.next_action_sequence;
        self.next_action_sequence = self
            .next_action_sequence
            .checked_add(1)
            .expect("one factory frame cannot issue u64::MAX actions");
        self.issued_action_sequence = Some(action_sequence);
        FactoryProductionPoll::Action(IssuedFactoryProductionAction {
            receipt: FactoryProductionReceipt {
                transaction_id: self.transaction_id,
                action_sequence,
                factory: self.request.factory,
            },
            before_action: FactoryProductionPreActionState {
                factory: self.request.factory,
                production: self.production,
                animation_state_raw: self.animation_state_raw,
            },
            action,
        })
    }

    pub fn resume(
        &mut self,
        receipt: FactoryProductionReceipt,
        resume: FactoryProductionResume,
    ) -> Result<(), FactoryProductionResumeFailure> {
        let Some(expected_sequence) = self.issued_action_sequence else {
            return Err(FactoryProductionResumeFailure {
                error: FactoryProductionProtocolError::NoOutstandingAction {
                    transaction_id: receipt.transaction_id,
                    action_sequence: receipt.action_sequence,
                },
                receipt,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(FactoryProductionResumeFailure {
                error: FactoryProductionProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.factory != self.request.factory {
            return Err(FactoryProductionResumeFailure {
                error: FactoryProductionProtocolError::ReceiptFactoryMismatch {
                    expected: self.request.factory,
                    actual: receipt.factory,
                },
                receipt,
            });
        }
        if receipt.action_sequence != expected_sequence {
            return Err(FactoryProductionResumeFailure {
                error: FactoryProductionProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }

        let expected_phase = self
            .stage
            .action_phase()
            .expect("an outstanding action belongs to an action stage");
        if resume.phase() != expected_phase {
            return Err(FactoryProductionResumeFailure {
                error: FactoryProductionProtocolError::PhaseMismatch {
                    expected: expected_phase,
                    actual: resume.phase(),
                },
                receipt,
            });
        }
        if !self.response_matches_stage(resume) {
            return Err(FactoryProductionResumeFailure {
                error: FactoryProductionProtocolError::ResponseKindMismatch {
                    phase: expected_phase,
                },
                receipt,
            });
        }

        self.issued_action_sequence = None;
        match resume {
            FactoryProductionResume::Blocked { reason, .. } => {
                self.stage = FactoryProductionStage::Blocked(FactoryProductionBlock {
                    factory: self.request.factory,
                    phase: expected_phase,
                    reason,
                    production: self.production,
                    animation_state_raw: self.animation_state_raw,
                    status_published: self.status_published,
                    dirty_flag_written: self.dirty_flag_written,
                });
            }
            _ => self.advance_after_response(resume),
        }
        Ok(())
    }

    fn action_for_stage(&self, phase: FactoryProductionActionPhase) -> FactoryProductionAction {
        match self.stage {
            FactoryProductionStage::SpawnPickup(request) => FactoryProductionAction::SpawnEntity {
                phase,
                role: FactorySpawnRole::Pickup,
                request,
            },
            FactoryProductionStage::LinkPickupOwner(spawned)
            | FactoryProductionStage::LinkConvertedOutputOwner(spawned) => {
                FactoryProductionAction::LinkOwner {
                    phase,
                    spawned,
                    owner_factory: self.request.factory,
                }
            }
            FactoryProductionStage::EmitHighThresholdDirectText => {
                FactoryProductionAction::EmitDirectText {
                    phase,
                    direct_text_id: FACTORY_HIGH_THRESHOLD_DIRECT_TEXT_ID,
                    sub_parameter: 0,
                }
            }
            FactoryProductionStage::ClearFactoryLifetime => {
                FactoryProductionAction::ClearFactoryLifetime {
                    phase,
                    factory: self.request.factory,
                    lifetime_offset: FACTORY_ENTITY_LIFETIME_OFFSET,
                    value_raw: 0,
                }
            }
            FactoryProductionStage::DispatchPhase1Presentation => {
                FactoryProductionAction::DispatchPhase1Presentation {
                    phase,
                    factory: self.request.factory,
                }
            }
            FactoryProductionStage::QueryPickupPresence => {
                FactoryProductionAction::QueryPickupPresence {
                    phase,
                    pickup_handle: self.production.spawned_pickup_handle,
                }
            }
            FactoryProductionStage::QueueConversionHudResource => {
                FactoryProductionAction::QueueHudResource {
                    phase,
                    resource_id: FACTORY_OUTPUT_CONVERSION_HUD_RESOURCE_ID,
                }
            }
            FactoryProductionStage::SpawnConvertedOutput(request) => {
                FactoryProductionAction::SpawnEntity {
                    phase,
                    role: FactorySpawnRole::ConvertedOutput,
                    request,
                }
            }
            FactoryProductionStage::SpawnMaterialiser { request, .. } => {
                FactoryProductionAction::SpawnEntity {
                    phase,
                    role: FactorySpawnRole::Materialiser,
                    request,
                }
            }
            FactoryProductionStage::LinkMaterialiserOutput {
                materialiser,
                output,
            } => FactoryProductionAction::LinkMaterialiserOutput {
                phase,
                materialiser,
                output,
            },
            FactoryProductionStage::PlayConversionPositionalSound(request) => {
                FactoryProductionAction::PlayConversionPositionalSound { phase, request }
            }
            FactoryProductionStage::ApplyAnimationTransition(transition) => {
                FactoryProductionAction::ApplyAnimationTransition {
                    phase,
                    factory: self.request.factory,
                    transition,
                }
            }
            FactoryProductionStage::PublishStatus(status) => {
                FactoryProductionAction::PublishStatus {
                    phase,
                    factory: self.request.factory,
                    status,
                }
            }
            FactoryProductionStage::MarkEntityDirty => FactoryProductionAction::MarkEntityDirty {
                phase,
                factory: self.request.factory,
                flag: FACTORY_ENTITY_DIRTY_FLAG,
            },
            FactoryProductionStage::Start
            | FactoryProductionStage::Blocked(_)
            | FactoryProductionStage::Complete => {
                unreachable!("non-action stage cannot build an action")
            }
        }
    }

    fn response_matches_stage(&self, resume: FactoryProductionResume) -> bool {
        if let FactoryProductionResume::Blocked { phase, reason } = resume {
            return reason.matches_phase(phase);
        }
        match self.stage {
            FactoryProductionStage::SpawnPickup(_)
            | FactoryProductionStage::SpawnConvertedOutput(_)
            | FactoryProductionStage::SpawnMaterialiser { .. } => {
                matches!(resume, FactoryProductionResume::SpawnCompleted { .. })
            }
            FactoryProductionStage::QueryPickupPresence => {
                matches!(resume, FactoryProductionResume::PickupPresence { .. })
            }
            FactoryProductionStage::LinkPickupOwner(_)
            | FactoryProductionStage::EmitHighThresholdDirectText
            | FactoryProductionStage::ClearFactoryLifetime
            | FactoryProductionStage::DispatchPhase1Presentation
            | FactoryProductionStage::QueueConversionHudResource
            | FactoryProductionStage::LinkConvertedOutputOwner(_)
            | FactoryProductionStage::LinkMaterialiserOutput { .. }
            | FactoryProductionStage::PlayConversionPositionalSound(_)
            | FactoryProductionStage::ApplyAnimationTransition(_)
            | FactoryProductionStage::PublishStatus(_)
            | FactoryProductionStage::MarkEntityDirty => {
                matches!(resume, FactoryProductionResume::Acknowledged { .. })
            }
            FactoryProductionStage::Start
            | FactoryProductionStage::Blocked(_)
            | FactoryProductionStage::Complete => false,
        }
    }

    fn advance_after_response(&mut self, resume: FactoryProductionResume) {
        match (self.stage, resume) {
            (
                FactoryProductionStage::SpawnPickup(_),
                FactoryProductionResume::SpawnCompleted { result, .. },
            ) => match result {
                FactoryEntitySpawnResult::Spawned(spawned) => {
                    self.stage = FactoryProductionStage::LinkPickupOwner(spawned);
                }
                FactoryEntitySpawnResult::Failed => self.finish_phase0_spawn_attempt(),
            },
            (
                FactoryProductionStage::LinkPickupOwner(spawned),
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.production.spawned_pickup_handle = spawned.entity_id.get();
                self.production.remaining_stock_raw =
                    self.production.remaining_stock_raw.wrapping_sub(1);
                if self.production.production_threshold_micros_raw > 4_000_000 {
                    self.stage = FactoryProductionStage::EmitHighThresholdDirectText;
                } else {
                    self.finish_phase0_spawn_attempt();
                }
            }
            (
                FactoryProductionStage::EmitHighThresholdDirectText,
                FactoryProductionResume::Acknowledged { .. },
            ) => self.finish_phase0_spawn_attempt(),
            (
                FactoryProductionStage::ClearFactoryLifetime,
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                // Retail clears entity +0x74 before writing phase 1.
                self.production.phase = FactoryProductionPhase::Delivering;
                self.enter_common_tail();
            }
            (
                FactoryProductionStage::DispatchPhase1Presentation,
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.status_publication_enabled = true;
                self.enter_common_tail();
            }
            (
                FactoryProductionStage::QueryPickupPresence,
                FactoryProductionResume::PickupPresence { presence, .. },
            ) => {
                if presence == FactoryPickupPresence::Gone {
                    self.production.cooldown_remaining_micros_raw =
                        self.production.cooldown_duration_micros_raw;
                    self.production.production_progress_micros_raw = 0;
                    self.production.delivery_progress_micros_raw = 0;
                    self.production.phase = FactoryProductionPhase::Cooldown;
                }
                self.enter_common_tail();
            }
            (
                FactoryProductionStage::QueueConversionHudResource,
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.stage = FactoryProductionStage::SpawnConvertedOutput(
                    self.converted_output_spawn_request(),
                );
            }
            (
                FactoryProductionStage::SpawnConvertedOutput(_),
                FactoryProductionResume::SpawnCompleted { result, .. },
            ) => match result {
                FactoryEntitySpawnResult::Spawned(spawned) => {
                    self.stage = FactoryProductionStage::LinkConvertedOutputOwner(spawned);
                }
                FactoryEntitySpawnResult::Failed => self.enter_common_tail(),
            },
            (
                FactoryProductionStage::LinkConvertedOutputOwner(output),
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.production.current_scientists_raw =
                    self.production.current_scientists_raw.wrapping_sub(1);
                self.production.scientist_capacity_raw =
                    self.production.scientist_capacity_raw.wrapping_sub(1);
                self.stage = FactoryProductionStage::SpawnMaterialiser {
                    output,
                    request: self.materialiser_spawn_request(),
                };
            }
            (
                FactoryProductionStage::SpawnMaterialiser { output, .. },
                FactoryProductionResume::SpawnCompleted { result, .. },
            ) => match result {
                FactoryEntitySpawnResult::Spawned(materialiser) => {
                    self.stage = FactoryProductionStage::LinkMaterialiserOutput {
                        materialiser,
                        output,
                    };
                }
                FactoryEntitySpawnResult::Failed => self.enter_common_tail(),
            },
            (
                FactoryProductionStage::LinkMaterialiserOutput {
                    materialiser: _,
                    output: _,
                },
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.stage = FactoryProductionStage::PlayConversionPositionalSound(
                    self.conversion_sound_request(),
                );
            }
            (
                FactoryProductionStage::PlayConversionPositionalSound(_),
                FactoryProductionResume::Acknowledged { .. },
            ) => self.enter_common_tail(),
            (
                FactoryProductionStage::ApplyAnimationTransition(transition),
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                // The acknowledged18F60 callback also changed C8E0 controls.
                // Retain them in later prefixes instead of restoring the
                // pre-callback gain/rate over the live component.
                self.production.retune_voices_from_animation_state(
                    transition.next_state_raw,
                    transition.previous_state_raw,
                );
                self.animation_state_raw = transition.next_state_raw;
                self.enter_status_tail();
            }
            (
                FactoryProductionStage::PublishStatus(_),
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.status_published = true;
                if self.active_phase_marks_dirty {
                    self.stage = FactoryProductionStage::MarkEntityDirty;
                } else {
                    self.stage = FactoryProductionStage::Complete;
                }
            }
            (
                FactoryProductionStage::MarkEntityDirty,
                FactoryProductionResume::Acknowledged { .. },
            ) => {
                self.dirty_flag_written = true;
                self.stage = FactoryProductionStage::Complete;
            }
            _ => unreachable!("response kind was checked before consuming its receipt"),
        }
    }

    fn advance_internal_stages(&mut self) {
        if self.stage != FactoryProductionStage::Start {
            return;
        }

        let elapsed = self.request.elapsed_micros as i32;
        if self.request.elapsed_micros < self.production.understaffed_countdown_micros_raw as u32 {
            self.production.understaffed_countdown_micros_raw = self
                .production
                .understaffed_countdown_micros_raw
                .wrapping_sub(elapsed);
        } else {
            self.production.understaffed_countdown_micros_raw = 0;
        }

        match self.production.phase {
            FactoryProductionPhase::Producing => {
                self.active_phase_marks_dirty = true;
                if self.production.production_progress_micros_raw
                    < self.production.production_threshold_micros_raw
                {
                    self.production.production_progress_micros_raw = self
                        .production
                        .production_progress_micros_raw
                        .wrapping_add(elapsed);
                    if self.production.production_threshold_micros_raw
                        <= self.production.production_progress_micros_raw
                    {
                        self.stage =
                            FactoryProductionStage::SpawnPickup(self.pickup_spawn_request());
                        return;
                    }
                }
                self.enter_common_tail();
            }
            FactoryProductionPhase::Delivering => {
                self.active_phase_marks_dirty = true;
                if self.production.delivery_progress_micros_raw
                    < self.production.delivery_duration_micros_raw
                {
                    self.production.delivery_progress_micros_raw = self
                        .production
                        .delivery_progress_micros_raw
                        .wrapping_add(elapsed);
                    if self.production.delivery_duration_micros_raw
                        <= self.production.delivery_progress_micros_raw
                    {
                        self.production.delivery_progress_micros_raw =
                            self.production.delivery_duration_micros_raw;
                        self.production.phase = FactoryProductionPhase::WaitingForPickup;
                    }
                    if self.request.phase1_presentation_enabled {
                        self.stage = FactoryProductionStage::DispatchPhase1Presentation;
                        return;
                    }
                }
                self.enter_common_tail();
            }
            FactoryProductionPhase::WaitingForPickup => {
                if self.production.remaining_stock_raw == 0 {
                    if self.production.current_scientists_raw == 0 {
                        self.production.phase = FactoryProductionPhase::IdleEmpty;
                        self.production.production_progress_micros_raw = 0;
                    } else if self
                        .production
                        .production_progress_micros_raw
                        .wrapping_sub(self.production.production_threshold_micros_raw)
                        < FACTORY_OUTPUT_CONVERSION_GATE_RAW
                    {
                        self.production.production_progress_micros_raw = self
                            .production
                            .production_progress_micros_raw
                            .wrapping_add(elapsed);
                    } else {
                        self.stage = FactoryProductionStage::QueueConversionHudResource;
                        return;
                    }
                    self.enter_common_tail();
                } else {
                    self.stage = FactoryProductionStage::QueryPickupPresence;
                }
            }
            FactoryProductionPhase::Cooldown => {
                self.active_phase_marks_dirty = true;
                if elapsed < self.production.cooldown_remaining_micros_raw {
                    self.production.cooldown_remaining_micros_raw = self
                        .production
                        .cooldown_remaining_micros_raw
                        .wrapping_sub(elapsed);
                } else {
                    self.production.cooldown_remaining_micros_raw = 0;
                    self.production.phase = FactoryProductionPhase::Producing;
                }
                self.enter_common_tail();
            }
            FactoryProductionPhase::IdleEmpty => {
                if self.production.current_scientists_raw != 0 {
                    self.production.phase = FactoryProductionPhase::WaitingForPickup;
                }
                self.enter_common_tail();
            }
        }
    }

    fn finish_phase0_spawn_attempt(&mut self) {
        self.production.production_progress_micros_raw =
            self.production.production_threshold_micros_raw;
        self.stage = FactoryProductionStage::ClearFactoryLifetime;
    }

    fn enter_common_tail(&mut self) {
        let next_state = self.derived_animation_state();
        if next_state != self.animation_state_raw {
            self.stage = FactoryProductionStage::ApplyAnimationTransition(animation_transition(
                self.animation_state_raw,
                next_state,
            ));
        } else {
            self.enter_status_tail();
        }
    }

    fn enter_status_tail(&mut self) {
        if self.status_publication_enabled {
            self.stage = FactoryProductionStage::PublishStatus(published_status(self.production));
        } else {
            self.stage = FactoryProductionStage::Complete;
        }
    }

    fn derived_animation_state(&self) -> u32 {
        let mut state = u32::from(
            self.production.current_scientists_raw >= self.production.scientist_capacity_raw,
        );
        if matches!(
            self.production.phase,
            FactoryProductionPhase::Delivering | FactoryProductionPhase::Cooldown
        ) {
            state |= 2;
        }
        state
    }

    fn pickup_spawn_request(&self) -> FactoryEntitySpawnRequest {
        FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: FACTORY_PICKUP_ENTITY_TYPE,
            position_raw: [
                self.request.position_raw[0].wrapping_add(self.request.pickup_spawn_offset_raw[0]),
                self.request.position_raw[1].wrapping_add(self.request.pickup_spawn_offset_raw[1]),
                self.request.position_raw[2].wrapping_add(self.request.pickup_spawn_offset_raw[2]),
            ],
            spawn_parameter_6: self.production.output_payload_packed,
        }
    }

    fn converted_output_spawn_request(&self) -> FactoryEntitySpawnRequest {
        FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: converted_output_entity_type(self.request.world_style_raw),
            position_raw: self.converted_output_position(),
            spawn_parameter_6: 0,
        }
    }

    fn materialiser_spawn_request(&self) -> FactoryEntitySpawnRequest {
        FactoryEntitySpawnRequest {
            requested_handle: FactoryRequestedEntityHandle::Allocate,
            entity_type: FACTORY_MATERIALISER_ENTITY_TYPE,
            position_raw: self.converted_output_position(),
            spawn_parameter_6: 0,
        }
    }

    fn converted_output_position(&self) -> [i16; 3] {
        [
            self.request.position_raw[0],
            self.request.position_raw[1],
            self.request.position_raw[2].wrapping_add(FACTORY_CONVERTED_OUTPUT_Z_OFFSET_RAW),
        ]
    }

    fn conversion_sound_request(&self) -> FactoryPositionalSoundRequest {
        FactoryPositionalSoundRequest {
            sound_id: FACTORY_OUTPUT_CONVERSION_SOUND_ID,
            position_raw: self.converted_output_position(),
            gain_raw_16_16: 0x1_0000,
            rate_raw_16_16: 0x1_0000,
        }
    }

    fn completion(&self) -> FactoryProductionCompletion {
        FactoryProductionCompletion {
            factory: self.request.factory,
            production: self.production,
            animation_state_raw: self.animation_state_raw,
            status_published: self.status_published,
            dirty_flag_written: self.dirty_flag_written,
        }
    }
}

/// Exact `FUN_0042EB70` world-style output selector.
pub const fn converted_output_entity_type(world_style_raw: u32) -> u32 {
    match world_style_raw {
        1 => 8,
        2 => 0x5B,
        3 => 0x5A,
        4 => 0x4F,
        5 => 0x74,
        6 => 7,
        _ => 0,
    }
}

/// Exact low-word ratio from `FUN_00419710`.
pub fn factory_status_ratio(mut value_raw: u32, mut total_raw: u32) -> u16 {
    if value_raw >= total_raw || total_raw == 0 {
        return u16::MAX;
    }
    if value_raw == 0 {
        return 0;
    }
    while total_raw > u32::from(u16::MAX) {
        value_raw >>= 4;
        total_raw >>= 4;
    }
    ((value_raw * u32::from(u16::MAX)) / total_raw) as u16
}

pub fn published_status(production: FactoryProductionRuntime) -> FactoryStatusPublication {
    let (production_ratio, delivery_ratio) = if production.cooldown_remaining_micros_raw == 0 {
        (
            factory_status_ratio(
                production.production_progress_micros_raw as u32,
                production.production_threshold_micros_raw as u32,
            ),
            factory_status_ratio(
                production.delivery_progress_micros_raw as u32,
                production.delivery_duration_micros_raw as u32,
            ),
        )
    } else {
        let ratio = factory_status_ratio(
            production.cooldown_remaining_micros_raw as u32,
            production.cooldown_duration_micros_raw as u32,
        );
        (ratio, ratio)
    };

    FactoryStatusPublication {
        current_scientists_raw: production.current_scientists_raw as u16,
        scientist_capacity_raw: production.scientist_capacity_raw as u16,
        output_selector_raw: u16::from(production.output_payload_packed as u8),
        production_or_cooldown_ratio_raw: production_ratio,
        delivery_or_cooldown_ratio_raw: delivery_ratio,
        understaffed_ratio_raw: factory_status_ratio(
            production.understaffed_countdown_micros_raw as u32,
            production.understaffed_limit_micros_raw as u32,
        ),
    }
}

pub const fn animation_transition(
    previous_state_raw: u32,
    next_state_raw: u32,
) -> FactoryAnimationTransition {
    let changed = previous_state_raw ^ next_state_raw;
    let primary = if changed & 5 != 0 {
        if next_state_raw & 4 == 0 {
            Some(FactoryAnimationRange {
                start_raw_16_16: ((next_state_raw & 1) << 16) as i32,
                end_raw_16_16: 0x1_0000,
            })
        } else {
            Some(FactoryAnimationRange {
                start_raw_16_16: 0x1_0000,
                end_raw_16_16: 0x1_8000,
            })
        }
    } else {
        None
    };
    let secondary = if changed & 2 != 0 {
        Some(FactoryAnimationRange {
            start_raw_16_16: ((next_state_raw & 2) << 15) as i32,
            end_raw_16_16: 0x1_0000,
        })
    } else {
        None
    };
    FactoryAnimationTransition {
        previous_state_raw,
        next_state_raw,
        primary,
        secondary,
    }
}
