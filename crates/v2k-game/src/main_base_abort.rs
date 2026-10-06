//! Main Base abort list-routing contract.
//!
//! Retail `FUN_0042F1A0` enters `FUN_004170A0` before the terrain and player
//! handoff. The latter walks the intrusive live-entity list in order and
//! selects one of two callback families from entity `+0x64`. Ordinary entities
//! are further gated by type and live-state bits. This module records that
//! selection without pretending that the still-unported callbacks are generic
//! deletion operations.

use std::num::NonZeroU64;

use v2k_formats::levels::LevelDescriptor;

use crate::entity_collision_state::RetailRuntimeValue;
use crate::power_up_contact::{CampaignControlSlotError, PlayerCampaignProgress};
use crate::time_trophy::{
    TimeTrophyLoadOutcome, TimeTrophyRuntime, TimeTrophySound, TimeTrophyState,
    TimeTrophyWorldCompletionOutcome,
};

/// Entity `+0x64` bits which route directly through `FUN_0041CF90`.
pub const ALTERNATE_CLEANUP_CAPABILITY_MASK: u32 = 0x0000_0011;

/// Entity `+0x08` bits which suppress ordinary `FUN_00410C10` dispatch.
pub const ORDINARY_DEATH_STATE_SKIP_MASK: u32 = 0x1000_1000;

/// Type `0x6F` is explicitly excluded from ordinary death dispatch.
pub const ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE: u32 = 0x6f;

/// A deferred-removal queue which may already own an actor when the late Main
/// Base abort sweep reaches its ordinary-death callback.
///
/// Retail uses one shared state bit/count helper for these independently
/// scheduled producers. The queue identity remains explicit in the port so a
/// callback cannot accidentally duplicate or steal another phase's removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseExternalDeferredDestroyOwner {
    /// The type-61 queue shared by accepted Power-Up contact and Working
    /// Factory product cleanup.
    Type61DeferredQueue,
    MainBaseConversion,
    FactoryScientist,
}

/// Deferred-removal state admitted at an ordinary Main Base death callback.
///
/// `Unstaged` leaves the callback family responsible for its normal direct or
/// task-terminal staging. `AlreadyPending` means exactly one earlier
/// same-frame producer owns the later splice; the callback still runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseDeferredDestroyEntry {
    Unstaged,
    AlreadyPending(MainBaseExternalDeferredDestroyOwner),
}

/// Complete state/queue snapshot used to reject contradictory deferred-
/// removal custody before an ordinary death callback mutates the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseDeferredDestroyQueueFacts {
    pub state_pending: bool,
    pub power_up_queued: bool,
    pub main_base_conversion_queued: bool,
    pub factory_scientist_queued: bool,
    pub actor_terminal_queued: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseDeferredDestroyCustodyBlock {
    StateUnresolved,
    StateQueueMismatch(MainBaseDeferredDestroyQueueFacts),
}

/// Result of calling the shared idempotent staging helper at callback exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseDeferredDestroyCommit {
    StagedByCallback,
    AlreadyPending(MainBaseExternalDeferredDestroyOwner),
}

/// Fresh-Level-1 authored actors whose generic death continuation selects the
/// class-2 `"Die Quietly"` alternate.
pub const LEVEL_ONE_QUIET_DEATH_ENTITY_TYPES: [u32; 3] = [52, 62, 68];

/// Section-13 identities of the eleven fresh-Level-1 type-52 flags.
pub const LEVEL_ONE_TYPE52_QUIET_DEATH_SPAWN_INDICES: [usize; 11] =
    [0, 1, 2, 3, 7, 21, 27, 28, 29, 30, 31];
/// Section-13 identities of the three fresh-Level-1 type-62 furniture actors.
pub const LEVEL_ONE_TYPE62_QUIET_DEATH_SPAWN_INDICES: [usize; 3] = [4, 25, 26];
/// Section-13 identity of the fresh-Level-1 type-68 loose weight.
pub const LEVEL_ONE_TYPE68_QUIET_DEATH_SPAWN_INDICES: [usize; 1] = [5];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortActorFacts {
    pub entity_id: u32,
    pub entity_type: u32,
    pub capability_flags: u32,
    pub state_flags: RetailRuntimeValue<u32>,
}

/// Exact action selected by retail's Main Base abort sweep.
///
/// The callback names are deliberate boundaries. `FUN_0041CF90` mutates
/// linked state and terrain context; `FUN_00410C10` owns authored death audio
/// and a type-specific callback. Neither may be replaced by an unconditional
/// vector removal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortActorRoute {
    AlternateCleanup { entity_id: u32 },
    OrdinaryDeath { entity_id: u32 },
    ExcludedType { entity_id: u32, entity_type: u32 },
    SuppressedByState { entity_id: u32, masked_state: u32 },
    UnresolvedState { entity_id: u32 },
}

/// Classify one live-list member exactly as `FUN_004170A0`.
pub fn classify_main_base_abort_actor(facts: MainBaseAbortActorFacts) -> MainBaseAbortActorRoute {
    if facts.capability_flags & ALTERNATE_CLEANUP_CAPABILITY_MASK != 0 {
        return MainBaseAbortActorRoute::AlternateCleanup {
            entity_id: facts.entity_id,
        };
    }

    if facts.entity_type == ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE {
        return MainBaseAbortActorRoute::ExcludedType {
            entity_id: facts.entity_id,
            entity_type: facts.entity_type,
        };
    }

    match facts.state_flags {
        RetailRuntimeValue::Known(state_flags) => {
            let masked_state = state_flags & ORDINARY_DEATH_STATE_SKIP_MASK;
            if masked_state == 0 {
                MainBaseAbortActorRoute::OrdinaryDeath {
                    entity_id: facts.entity_id,
                }
            } else {
                MainBaseAbortActorRoute::SuppressedByState {
                    entity_id: facts.entity_id,
                    masked_state,
                }
            }
        }
        RetailRuntimeValue::Unresolved => MainBaseAbortActorRoute::UnresolvedState {
            entity_id: facts.entity_id,
        },
    }
}

/// Callback installed into the full-frame request by retail `FUN_0042F1A0`.
pub const MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS: u32 = 0x0042_E860;
/// Player-controller state selected after the actor sweep and terrain change.
pub const MAIN_BASE_ABORT_PLAYER_STATE: u32 = TimeTrophyState::ResultsActive as u32;

/// Caller-owned identity for one detached systemic Main Base abort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainBaseAbortTransactionId(NonZeroU64);

impl MainBaseAbortTransactionId {
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

/// Authenticated allocation identity for one current intrusive-list member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainBaseAbortActorLease {
    pub entity_id: u32,
    pub allocation_identity: u64,
}

/// Linear proof that the terminal Main Base callback committed before the
/// systemic abort entered its nested live-list sweep.
///
/// The private payload prevents callers from manufacturing this authority
/// from a merely current actor observation. Failure paths return the origin so
/// a blocked owner preserves linear custody; it may retry only after resolving
/// a recoverable cause. Successful processing consumes it exactly once.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseTerminalAbortOrigin {
    actor_lease: MainBaseAbortActorLease,
}

impl MainBaseTerminalAbortOrigin {
    pub(crate) const fn issue(actor_lease: MainBaseAbortActorLease) -> Self {
        Self { actor_lease }
    }

    /// Duplicate the proof only inside the isolated production transaction.
    ///
    /// The original remains parked at the wrapper boundary while this fork is
    /// consumed by the speculative actor walk.  Success drops the parked proof
    /// while failure drops the speculative world and returns the original, so
    /// no two usable origins ever escape the call.
    pub(crate) const fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            actor_lease: self.actor_lease,
        }
    }

    pub const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.actor_lease
    }

    pub const fn target_id(&self) -> u32 {
        self.actor_lease.entity_id
    }
}

/// Current node observation used by the exact actor-list dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortActorObservation {
    pub lease: MainBaseAbortActorLease,
    pub entity_type: u32,
    pub capability_flags: u32,
    pub state_flags: RetailRuntimeValue<u32>,
}

/// Completed callback-free branch of retail's live-list sweep.
///
/// `FUN_004170A0` advances past excluded type `0x6F` and actors carrying one
/// of the ordinary-death suppression bits without invoking either callback
/// family. The route is retained so a whole-list owner can distinguish those
/// two proven branches without reconstructing facts from the successor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortNoCallbackAdvance {
    pub route: MainBaseAbortActorRoute,
    pub next_actor: Option<MainBaseAbortActorObservation>,
}

/// The current lease or live route did not authorize callback-free advance.
/// No actor or world state changes on any of these paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortNoCallbackBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotCallbackFreeRoute(MainBaseAbortActorRoute),
}

/// Exact nested generic-death result for the terminal Main Base itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortTerminalSelfOutcome {
    /// The outer progression callback already retained bit `0x4000`, so the
    /// nested `FUN_00410C10` returns one before audio, relation release, or
    /// type-specific callback dispatch.
    AlreadyDyingNoOp { entity_id: u32, entity_type: u32 },
}

/// Post-callback traversal result for the authenticated terminal Main Base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortTerminalSelfAdvance {
    Advanced {
        outcome: MainBaseAbortTerminalSelfOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseAbortTerminalSelfOutcome,
    },
}

/// Evidence or live custody missing before the terminal self no-op can be
/// authenticated. No entity or presentation state has changed on these paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortTerminalSelfBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    OriginActorMismatch {
        origin: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    NotFreshNewGameFirstWorld,
    UnsupportedEntityType {
        actual: u32,
    },
    UnauthenticatedSpawn {
        actual: Option<usize>,
    },
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    UnexpectedGenericDeathEntry(GenericDeathEntry),
}

/// Terminal-self preflight failure. Returning the non-clone origin makes the
/// ownership rule explicit without weakening the one-shot success path.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseAbortTerminalSelfFailure {
    pub origin: MainBaseTerminalAbortOrigin,
    pub error: MainBaseAbortTerminalSelfBlock,
}

/// Completed bounded `FUN_00410C10 -> FUN_0040DB80 -> FUN_0040C470`
/// result for one native Type52/68 or bounded fresh-Level-1 class-2 actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortQuietDeathOutcome {
    /// Generic death returns zero immediately for a high-bit/remote entity.
    RemoteOwnedNoOp { entity_id: u32, entity_type: u32 },
    /// Generic death returns one without repeating any prefix when `0x4000`
    /// was already set.
    AlreadyDyingNoOp { entity_id: u32, entity_type: u32 },
    /// Health/dying state, optional cue, class-2 publication, ordered task
    /// clears, and the shared deferred-destroy mark/count all committed.
    DeferredDestroyStaged {
        entity_id: u32,
        entity_type: u32,
        death_sound_id: Option<u16>,
    },
}

/// Post-callback traversal result. Retail samples `current->next` only after
/// the complete actor callback returns; the future whole-list owner must never
/// reuse a successor captured during preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortQuietDeathAdvance {
    Advanced {
        outcome: MainBaseAbortQuietDeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    /// The death prefix is already committed, so this is a consumed durable
    /// block rather than a retryable preflight error.
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseAbortQuietDeathOutcome,
    },
}

/// Evidence or live custody missing before the bounded ordinary-death adapter
/// can make its first mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortQuietDeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    NotFreshNewGameFirstWorld,
    NativeConstructorReceiptMismatch,
    PendingNativePrefix,
    UnsupportedEntityType {
        actual: u32,
    },
    UnauthenticatedSpawn {
        entity_type: u32,
        actual: Option<usize>,
    },
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    CurrentBehaviorContextAbsent,
    CurrentBehaviorContextMismatch,
    UnexpectedPublishedTask,
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    DeferredDestroyStateUnresolved,
    DeferredDestroyAlreadyPending,
    DeferredDestroyAlreadyQueued,
    DeathSoundStateMismatch,
    ConstructorSoundAttachmentNotExactNull,
    ManagedDeathNetworkSuffixUnsupported,
}

/// Completed generic-death/class-2 result for one dynamically allocated
/// Type-93 Materialiser reached by the Main Base actor sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType93DeathOutcome {
    /// Generic death returns zero before consulting the current Materialiser
    /// behavior or its Sub-J child row.
    RemoteOwnedNoOp { entity_id: u32 },
    /// Generic death returns one before replaying health, behavior, task, or
    /// deferred-destroy publication.
    AlreadyDyingNoOp { entity_id: u32 },
    /// Health/dying state, class-2 selection, ordered task clears, and the
    /// shared deferred-destroy mark/count all committed. The authoritative
    /// Sub-J row remains intact for the later abnormal-teardown callback.
    DeferredDestroyStaged { entity_id: u32, cargo_id: u32 },
}

/// Post-callback traversal result for an authenticated Type-93 actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType93DeathAdvance {
    Advanced {
        outcome: MainBaseType93DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    /// The callback has committed, but its post-callback intrusive successor
    /// can no longer be authenticated.
    SuccessorUnavailableAfterCommit { outcome: MainBaseType93DeathOutcome },
}

/// Evidence or live custody missing before the bounded Type-93 death adapter
/// can make its first mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType93DeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    UnsupportedEntityType {
        actual: u32,
    },
    ExpectedDynamicAllocation {
        actual: Option<usize>,
    },
    /// Geometry-only proxy projection lacks the shared native constructor.
    UnsupportedConstructionProvenance,
    ConstructorReceiptMismatch,
    RemoteOwnershipUnresolved,
    DyingStateUnresolved,
    TypeMetadataUnavailable,
    TypeMetadataMismatch,
    InitialBehaviorMismatch,
    CurrentBehaviorContextMismatch,
    RuntimeTopologyMismatch,
    CargoRelationMismatch,
    SidecarMismatch,
    UnexpectedPublishedTask,
    DeferredDestroyCustody(MainBaseDeferredDestroyCustodyBlock),
    DeferredDestroyAlreadyPending(MainBaseExternalDeferredDestroyOwner),
}

/// Prefix branch selected by generic death `FUN_00410C10` after its type
/// record lookup. Remote ownership wins over the already-dying short circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericDeathEntry {
    RemoteOwnedNoOp,
    AlreadyDyingNoOp,
    Dispatch,
}

pub const fn classify_generic_death_entry(
    remote_owned: bool,
    already_dying: bool,
) -> GenericDeathEntry {
    if remote_owned {
        GenericDeathEntry::RemoteOwnedNoOp
    } else if already_dying {
        GenericDeathEntry::AlreadyDyingNoOp
    } else {
        GenericDeathEntry::Dispatch
    }
}

impl MainBaseAbortActorObservation {
    pub const fn facts(self) -> MainBaseAbortActorFacts {
        MainBaseAbortActorFacts {
            entity_id: self.lease.entity_id,
            entity_type: self.entity_type,
            capability_flags: self.capability_flags,
            state_flags: self.state_flags,
        }
    }
}

/// Authenticated world-control slot retained across the actor and terrain
/// phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MainBaseAbortWorldControlLease {
    pub allocation_identity: u64,
}

/// World-control fields sampled after the actor sweep and terrain transform,
/// then copied into player-controller offsets `+0xB0..+0xBC` by
/// `FUN_0042F1A0`.
///
/// The opaque offset names are intentional. Their presentation semantics are
/// owned by `FUN_00433130`; this boundary only preserves the exact copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortWorldControl {
    pub allocation_identity: u64,
    pub word_0x58: u16,
    pub word_0x5a: u16,
    pub dword_0x4c: u32,
    pub dword_0x88: u32,
    pub dword_0x8c: u32,
}

impl MainBaseAbortWorldControl {
    pub const fn lease(self) -> MainBaseAbortWorldControlLease {
        MainBaseAbortWorldControlLease {
            allocation_identity: self.allocation_identity,
        }
    }
}

/// Complete six-dword request consumed from player-controller offsets
/// `+0xA8..+0xBC`.
///
/// Retail rewrites `+0xA8` and `+0xB0..+0xBC` but deliberately retains the
/// existing dword at `+0xAC`; `FUN_00433130` then copies all six dwords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortFrameRequest {
    pub callback_address: u32,
    pub retained_dword_0xac: u32,
    pub word_0xb0: u16,
    pub word_0xb2: u16,
    pub dword_0xb4: u32,
    pub dword_0xb8: u32,
    pub dword_0xbc: u32,
}

impl MainBaseAbortFrameRequest {
    pub const fn from_post_transform_observation(
        control: MainBaseAbortWorldControl,
        retained_dword_0xac: u32,
    ) -> Self {
        Self {
            callback_address: MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS,
            retained_dword_0xac,
            word_0xb0: control.word_0x58,
            word_0xb2: control.word_0x5a,
            dword_0xb4: control.dword_0x4c,
            dword_0xb8: control.dword_0x88,
            dword_0xbc: control.dword_0x8c,
        }
    }
}

/// Live owner for the retail world-controller state and its current submitted
/// full-frame request.
///
/// `FUN_0042E3B0` clears the complete `0x1FC`-byte controller and explicitly
/// initializes `+0xAC` to zero. `FUN_0042E570` submits the normal Section-13
/// `+0x54/+0x56` frame and initializes time-trophy fields `+0x1EC/+0x1F0`,
/// while `FUN_0042F1A0` later replaces only the callback and `+0xB0..+0xBC`
/// from `+0x58/+0x5A`, `+0x4C`, `+0x88`, and `+0x8C` and commits state 5.
/// The host identity is a per-load generation lease, never a fabricated retail
/// pointer or campaign control-slot number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortControllerStorage {
    world_control: MainBaseAbortWorldControl,
    time_trophy: TimeTrophyRuntime,
    stored_frame_request: MainBaseAbortFrameRequest,
    submitted_frame_request: MainBaseAbortFrameRequest,
    abort_frame_submitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortControllerLeaseMismatch {
    pub expected: MainBaseAbortWorldControlLease,
    pub actual: MainBaseAbortWorldControlLease,
}

impl MainBaseAbortControllerStorage {
    /// Construct the same normal frame state installed for a freshly loaded
    /// world. The generation must be unique for this host-side level load.
    pub fn from_loaded_level(
        load_generation: NonZeroU64,
        level: &LevelDescriptor,
        results_active: bool,
        progress: &mut PlayerCampaignProgress,
    ) -> Result<(Self, TimeTrophyLoadOutcome), CampaignControlSlotError> {
        let world_control = MainBaseAbortWorldControl {
            allocation_identity: load_generation.get(),
            word_0x58: level.main_base_abort_sky_color_index,
            word_0x5a: level.main_base_abort_sky_model,
            dword_0x4c: level.terrain_sprite_base,
            dword_0x88: level.terrain_draw_depth,
            dword_0x8c: level
                .raw_u32(0x8C)
                .expect("parsed Section 13 retains its complete fixed header"),
        };
        let normal_request = MainBaseAbortFrameRequest {
            callback_address: MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS,
            retained_dword_0xac: 0,
            word_0xb0: level.sky_color_index,
            word_0xb2: level.sky_model,
            dword_0xb4: world_control.dword_0x4c,
            dword_0xb8: world_control.dword_0x88,
            dword_0xbc: world_control.dword_0x8c,
        };
        let (time_trophy, load_outcome) = TimeTrophyRuntime::from_loaded_world(
            level.time_trophy_deadline_seconds(),
            results_active,
            progress,
        )?;
        Ok((
            Self {
                world_control,
                time_trophy,
                stored_frame_request: normal_request,
                submitted_frame_request: normal_request,
                abort_frame_submitted: false,
            },
            load_outcome,
        ))
    }

    pub const fn world_control_lease(&self) -> MainBaseAbortWorldControlLease {
        self.world_control.lease()
    }

    /// Observe the exact retained world-control words for a detached abort
    /// transaction. The controller remains the mutation owner; this copy is
    /// used only to authenticate the state handed back to
    /// [`MainBaseAbortMachine`] after the terrain phase.
    pub const fn world_control(&self) -> MainBaseAbortWorldControl {
        self.world_control
    }

    pub const fn player_state(&self) -> RetailRuntimeValue<u32> {
        RetailRuntimeValue::Known(self.time_trophy.state() as u32)
    }

    pub const fn time_trophy_state(&self) -> TimeTrophyState {
        self.time_trophy.state()
    }

    pub const fn time_trophy_runtime(&self) -> TimeTrophyRuntime {
        self.time_trophy
    }

    pub const fn time_trophy_remaining_millis_raw(&self) -> i32 {
        self.time_trophy.remaining_millis_raw()
    }

    pub fn advance_time_trophy(&mut self, elapsed_micros: u32) -> Option<TimeTrophySound> {
        self.time_trophy.advance(elapsed_micros)
    }

    pub fn stop_time_trophy_countdown_for_hidden_pickup(&mut self) {
        self.time_trophy.stop_countdown_for_hidden_pickup();
    }

    /// Campaign completion hook for the ordinary victory/results owner.
    /// This is not the user save-to-disk path.
    pub fn complete_current_world(
        &mut self,
        progress: &mut PlayerCampaignProgress,
    ) -> Result<TimeTrophyWorldCompletionOutcome, CampaignControlSlotError> {
        self.time_trophy.complete_current_world(progress)
    }

    /// Overlay 51 / results-screen occupancy of controller `+0x1F0`.
    pub fn enter_results_active(&mut self) {
        self.time_trophy.enter_results_active();
    }

    pub const fn stored_frame_request(&self) -> MainBaseAbortFrameRequest {
        self.stored_frame_request
    }

    pub const fn submitted_frame_request(&self) -> MainBaseAbortFrameRequest {
        self.submitted_frame_request
    }

    pub const fn abort_frame_submitted(&self) -> bool {
        self.abort_frame_submitted
    }

    /// Atomically apply the post-terrain player handoff and frame submission.
    /// A stale lease leaves every controller field untouched.
    pub fn commit_post_terrain_abort(
        &mut self,
        lease: MainBaseAbortWorldControlLease,
    ) -> Result<MainBaseAbortFrameRequest, MainBaseAbortControllerLeaseMismatch> {
        let expected = self.world_control.lease();
        if lease != expected {
            return Err(MainBaseAbortControllerLeaseMismatch {
                expected,
                actual: lease,
            });
        }

        let request = MainBaseAbortFrameRequest::from_post_transform_observation(
            self.world_control,
            self.stored_frame_request.retained_dword_0xac,
        );
        self.time_trophy.enter_results_active();
        self.stored_frame_request = request;
        self.submitted_frame_request = request;
        self.abort_frame_submitted = true;
        Ok(request)
    }
}

/// Exact externally owned phase of the systemic `FUN_0042F1A0` body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortPhase {
    ActorSweep,
    TerrainTransform,
    PlayerHandoff,
    FullFrameRequest,
}

/// One external boundary in retail order.
///
/// `ProcessActor` deliberately returns the successor only after its selected
/// callback has returned. Retail reads `current->next` at that point, so a
/// pre-snapshotted list would be observably wrong when a callback rewires it.
#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseAbortAction {
    ProcessActor {
        phase: MainBaseAbortPhase,
        actor: MainBaseAbortActorObservation,
        route: MainBaseAbortActorRoute,
    },
    TransformTerrain {
        phase: MainBaseAbortPhase,
        world_control_identity: u64,
    },
    CommitPlayerHandoff {
        phase: MainBaseAbortPhase,
        player_state: u32,
        world_control: MainBaseAbortWorldControlLease,
        callback_address: u32,
    },
    SubmitFullFrame {
        phase: MainBaseAbortPhase,
        request: MainBaseAbortFrameRequest,
    },
}

impl MainBaseAbortAction {
    pub const fn phase(&self) -> MainBaseAbortPhase {
        match self {
            Self::ProcessActor { phase, .. }
            | Self::TransformTerrain { phase, .. }
            | Self::CommitPlayerHandoff { phase, .. }
            | Self::SubmitFullFrame { phase, .. } => *phase,
        }
    }
}

/// Linear proof that exactly one action has been issued.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseAbortReceipt {
    transaction_id: MainBaseAbortTransactionId,
    action_sequence: u64,
    phase: MainBaseAbortPhase,
}

impl MainBaseAbortReceipt {
    pub const fn transaction_id(&self) -> MainBaseAbortTransactionId {
        self.transaction_id
    }

    pub const fn action_sequence(&self) -> u64 {
        self.action_sequence
    }

    pub const fn phase(&self) -> MainBaseAbortPhase {
        self.phase
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct IssuedMainBaseAbortAction {
    pub receipt: MainBaseAbortReceipt,
    pub action: MainBaseAbortAction,
}

/// Adapter-side inability to execute one exact external boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortExternalBlock {
    ActorProcessingUnavailable,
    ActorSuccessorUnavailable,
    TerrainTransformUnavailable,
    PlayerHandoffUnavailable,
    FullFrameSubmissionUnavailable,
}

impl MainBaseAbortExternalBlock {
    pub const fn phase(self) -> MainBaseAbortPhase {
        match self {
            Self::ActorProcessingUnavailable | Self::ActorSuccessorUnavailable => {
                MainBaseAbortPhase::ActorSweep
            }
            Self::TerrainTransformUnavailable => MainBaseAbortPhase::TerrainTransform,
            Self::PlayerHandoffUnavailable => MainBaseAbortPhase::PlayerHandoff,
            Self::FullFrameSubmissionUnavailable => MainBaseAbortPhase::FullFrameRequest,
        }
    }
}

/// Durable terminal block after any already acknowledged prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortBlock {
    pub phase: MainBaseAbortPhase,
    pub actors_processed: usize,
    pub terrain_transformed: bool,
    pub player_handoff_committed: bool,
    pub reason: MainBaseAbortExternalBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseAbortCompletion {
    /// False only when retail's world-control slot was known null.
    pub systemic_body_entered: bool,
    pub actors_processed: usize,
    pub terrain_transformed: bool,
    pub player_handoff_committed: bool,
    pub full_frame_submitted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortPlanningBlock {
    WorldControlUnresolved,
    InitialActorUnresolved,
    ActorStateUnresolved { entity_id: u32 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum MainBaseAbortPoll {
    Action(IssuedMainBaseAbortAction),
    Awaiting(MainBaseAbortPhase),
    PlanningBlocked(MainBaseAbortPlanningBlock),
    Blocked(MainBaseAbortBlock),
    Complete(MainBaseAbortCompletion),
}

/// Result supplied by the adapter for the outstanding action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortResume {
    ActorProcessed {
        phase: MainBaseAbortPhase,
        actor: MainBaseAbortActorLease,
        successor: RetailRuntimeValue<Option<MainBaseAbortActorObservation>>,
    },
    TerrainTransformed {
        phase: MainBaseAbortPhase,
    },
    PlayerHandoffCommitted {
        phase: MainBaseAbortPhase,
        player_state: u32,
        world_control: MainBaseAbortWorldControl,
        request: MainBaseAbortFrameRequest,
    },
    FullFrameSubmitted {
        phase: MainBaseAbortPhase,
        request: MainBaseAbortFrameRequest,
    },
    Blocked {
        phase: MainBaseAbortPhase,
        reason: MainBaseAbortExternalBlock,
    },
}

impl MainBaseAbortResume {
    pub const fn phase(self) -> MainBaseAbortPhase {
        match self {
            Self::ActorProcessed { phase, .. }
            | Self::TerrainTransformed { phase }
            | Self::PlayerHandoffCommitted { phase, .. }
            | Self::FullFrameSubmitted { phase, .. }
            | Self::Blocked { phase, .. } => phase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseAbortProtocolError {
    NoOutstandingAction,
    ReceiptTransactionMismatch {
        expected: MainBaseAbortTransactionId,
        actual: MainBaseAbortTransactionId,
    },
    ReceiptSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    PhaseMismatch {
        expected: MainBaseAbortPhase,
        actual: MainBaseAbortPhase,
    },
    UnexpectedResumeKind {
        phase: MainBaseAbortPhase,
    },
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    WorldControlLeaseMismatch {
        expected: MainBaseAbortWorldControlLease,
        actual: MainBaseAbortWorldControlLease,
    },
    PlayerStateMismatch {
        expected: u32,
        actual: u32,
    },
    FrameRequestMismatch {
        expected: MainBaseAbortFrameRequest,
        actual: MainBaseAbortFrameRequest,
    },
    BlockReasonPhaseMismatch {
        phase: MainBaseAbortPhase,
        reason: MainBaseAbortExternalBlock,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseAbortResumeFailure {
    pub receipt: MainBaseAbortReceipt,
    pub error: MainBaseAbortProtocolError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainBaseAbortStage {
    Actor(MainBaseAbortActorObservation),
    TerrainTransform,
    PlayerHandoff,
    FullFrameRequest,
    PlanningBlocked(MainBaseAbortPlanningBlock),
    Blocked(MainBaseAbortBlock),
    Complete(MainBaseAbortCompletion),
}

/// Non-replayable detached `FUN_0042F1A0` systemic-body transaction.
///
/// The surrounding `FUN_00456960` sound/session prefix and final session
/// full-frame flag remain separate owners. This machine begins at the
/// world-control gate and ends after `FUN_00433130` has accepted its request.
#[derive(Debug, PartialEq, Eq)]
pub struct MainBaseAbortMachine {
    transaction_id: MainBaseAbortTransactionId,
    world_control: Option<MainBaseAbortWorldControlLease>,
    frame_request: Option<MainBaseAbortFrameRequest>,
    stage: MainBaseAbortStage,
    actors_processed: usize,
    terrain_transformed: bool,
    player_handoff_committed: bool,
    next_action_sequence: u64,
    outstanding: Option<(u64, MainBaseAbortPhase)>,
}

impl MainBaseAbortMachine {
    pub fn start(
        transaction_id: MainBaseAbortTransactionId,
        world_control: RetailRuntimeValue<Option<MainBaseAbortWorldControlLease>>,
        first_actor: RetailRuntimeValue<Option<MainBaseAbortActorObservation>>,
    ) -> Self {
        let (world_control, frame_request, stage) = match world_control {
            RetailRuntimeValue::Unresolved => (
                None,
                None,
                MainBaseAbortStage::PlanningBlocked(
                    MainBaseAbortPlanningBlock::WorldControlUnresolved,
                ),
            ),
            RetailRuntimeValue::Known(None) => (
                None,
                None,
                MainBaseAbortStage::Complete(MainBaseAbortCompletion {
                    systemic_body_entered: false,
                    actors_processed: 0,
                    terrain_transformed: false,
                    player_handoff_committed: false,
                    full_frame_submitted: false,
                }),
            ),
            RetailRuntimeValue::Known(Some(control)) => {
                let stage = match first_actor {
                    RetailRuntimeValue::Unresolved => MainBaseAbortStage::PlanningBlocked(
                        MainBaseAbortPlanningBlock::InitialActorUnresolved,
                    ),
                    RetailRuntimeValue::Known(Some(actor)) => {
                        stage_for_actor(actor).unwrap_or_else(MainBaseAbortStage::PlanningBlocked)
                    }
                    RetailRuntimeValue::Known(None) => MainBaseAbortStage::TerrainTransform,
                };
                (Some(control), None, stage)
            }
        };

        Self {
            transaction_id,
            world_control,
            frame_request,
            stage,
            actors_processed: 0,
            terrain_transformed: false,
            player_handoff_committed: false,
            next_action_sequence: 1,
            outstanding: None,
        }
    }

    pub fn poll(&mut self) -> MainBaseAbortPoll {
        if let Some((_, phase)) = self.outstanding {
            return MainBaseAbortPoll::Awaiting(phase);
        }

        let action = match self.stage {
            MainBaseAbortStage::Actor(actor) => MainBaseAbortAction::ProcessActor {
                phase: MainBaseAbortPhase::ActorSweep,
                actor,
                route: classify_main_base_abort_actor(actor.facts()),
            },
            MainBaseAbortStage::TerrainTransform => MainBaseAbortAction::TransformTerrain {
                phase: MainBaseAbortPhase::TerrainTransform,
                world_control_identity: self
                    .world_control
                    .expect("systemic body retains world control")
                    .allocation_identity,
            },
            MainBaseAbortStage::PlayerHandoff => MainBaseAbortAction::CommitPlayerHandoff {
                phase: MainBaseAbortPhase::PlayerHandoff,
                player_state: MAIN_BASE_ABORT_PLAYER_STATE,
                world_control: self
                    .world_control
                    .expect("systemic body retains world-control lease"),
                callback_address: MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS,
            },
            MainBaseAbortStage::FullFrameRequest => MainBaseAbortAction::SubmitFullFrame {
                phase: MainBaseAbortPhase::FullFrameRequest,
                request: self
                    .frame_request
                    .expect("systemic body retains frame request"),
            },
            MainBaseAbortStage::PlanningBlocked(reason) => {
                return MainBaseAbortPoll::PlanningBlocked(reason);
            }
            MainBaseAbortStage::Blocked(block) => return MainBaseAbortPoll::Blocked(block),
            MainBaseAbortStage::Complete(completion) => {
                return MainBaseAbortPoll::Complete(completion);
            }
        };

        let sequence = self.next_action_sequence;
        self.next_action_sequence = self.next_action_sequence.wrapping_add(1);
        let phase = action.phase();
        self.outstanding = Some((sequence, phase));
        MainBaseAbortPoll::Action(IssuedMainBaseAbortAction {
            receipt: MainBaseAbortReceipt {
                transaction_id: self.transaction_id,
                action_sequence: sequence,
                phase,
            },
            action,
        })
    }

    pub fn resume(
        &mut self,
        receipt: MainBaseAbortReceipt,
        resume: MainBaseAbortResume,
    ) -> Result<(), MainBaseAbortResumeFailure> {
        let Some((expected_sequence, expected_phase)) = self.outstanding else {
            return Err(MainBaseAbortResumeFailure {
                receipt,
                error: MainBaseAbortProtocolError::NoOutstandingAction,
            });
        };
        if receipt.transaction_id != self.transaction_id {
            return Err(MainBaseAbortResumeFailure {
                error: MainBaseAbortProtocolError::ReceiptTransactionMismatch {
                    expected: self.transaction_id,
                    actual: receipt.transaction_id,
                },
                receipt,
            });
        }
        if receipt.action_sequence != expected_sequence {
            return Err(MainBaseAbortResumeFailure {
                error: MainBaseAbortProtocolError::ReceiptSequenceMismatch {
                    expected: expected_sequence,
                    actual: receipt.action_sequence,
                },
                receipt,
            });
        }
        if receipt.phase != expected_phase || resume.phase() != expected_phase {
            return Err(MainBaseAbortResumeFailure {
                error: MainBaseAbortProtocolError::PhaseMismatch {
                    expected: expected_phase,
                    actual: resume.phase(),
                },
                receipt,
            });
        }

        let next_stage = match (self.stage, resume) {
            (
                MainBaseAbortStage::Actor(expected_actor),
                MainBaseAbortResume::ActorProcessed {
                    actor, successor, ..
                },
            ) => {
                if actor != expected_actor.lease {
                    return Err(MainBaseAbortResumeFailure {
                        error: MainBaseAbortProtocolError::ActorLeaseMismatch {
                            expected: expected_actor.lease,
                            actual: actor,
                        },
                        receipt,
                    });
                }
                self.actors_processed += 1;
                match successor {
                    RetailRuntimeValue::Unresolved => MainBaseAbortStage::Blocked(self.block(
                        MainBaseAbortPhase::ActorSweep,
                        MainBaseAbortExternalBlock::ActorSuccessorUnavailable,
                    )),
                    RetailRuntimeValue::Known(Some(actor)) => match stage_for_actor(actor) {
                        Ok(stage) => stage,
                        Err(MainBaseAbortPlanningBlock::ActorStateUnresolved { .. }) => {
                            MainBaseAbortStage::Blocked(self.block(
                                MainBaseAbortPhase::ActorSweep,
                                MainBaseAbortExternalBlock::ActorProcessingUnavailable,
                            ))
                        }
                        Err(reason) => MainBaseAbortStage::PlanningBlocked(reason),
                    },
                    RetailRuntimeValue::Known(None) => MainBaseAbortStage::TerrainTransform,
                }
            }
            (
                MainBaseAbortStage::TerrainTransform,
                MainBaseAbortResume::TerrainTransformed { .. },
            ) => {
                self.terrain_transformed = true;
                MainBaseAbortStage::PlayerHandoff
            }
            (
                MainBaseAbortStage::PlayerHandoff,
                MainBaseAbortResume::PlayerHandoffCommitted {
                    player_state,
                    world_control,
                    request,
                    ..
                },
            ) => {
                if player_state != MAIN_BASE_ABORT_PLAYER_STATE {
                    return Err(MainBaseAbortResumeFailure {
                        error: MainBaseAbortProtocolError::PlayerStateMismatch {
                            expected: MAIN_BASE_ABORT_PLAYER_STATE,
                            actual: player_state,
                        },
                        receipt,
                    });
                }
                let expected_lease = self
                    .world_control
                    .expect("player handoff retains world-control lease");
                if world_control.lease() != expected_lease {
                    return Err(MainBaseAbortResumeFailure {
                        error: MainBaseAbortProtocolError::WorldControlLeaseMismatch {
                            expected: expected_lease,
                            actual: world_control.lease(),
                        },
                        receipt,
                    });
                }
                let expected = MainBaseAbortFrameRequest::from_post_transform_observation(
                    world_control,
                    request.retained_dword_0xac,
                );
                if request != expected {
                    return Err(MainBaseAbortResumeFailure {
                        error: MainBaseAbortProtocolError::FrameRequestMismatch {
                            expected,
                            actual: request,
                        },
                        receipt,
                    });
                }
                self.frame_request = Some(expected);
                self.player_handoff_committed = true;
                MainBaseAbortStage::FullFrameRequest
            }
            (
                MainBaseAbortStage::FullFrameRequest,
                MainBaseAbortResume::FullFrameSubmitted { request, .. },
            ) => {
                let expected = self
                    .frame_request
                    .expect("full-frame stage retains frame request");
                if request != expected {
                    return Err(MainBaseAbortResumeFailure {
                        error: MainBaseAbortProtocolError::FrameRequestMismatch {
                            expected,
                            actual: request,
                        },
                        receipt,
                    });
                }
                MainBaseAbortStage::Complete(MainBaseAbortCompletion {
                    systemic_body_entered: true,
                    actors_processed: self.actors_processed,
                    terrain_transformed: self.terrain_transformed,
                    player_handoff_committed: self.player_handoff_committed,
                    full_frame_submitted: true,
                })
            }
            (_, MainBaseAbortResume::Blocked { phase, reason }) => {
                if reason.phase() != phase {
                    return Err(MainBaseAbortResumeFailure {
                        error: MainBaseAbortProtocolError::BlockReasonPhaseMismatch {
                            phase,
                            reason,
                        },
                        receipt,
                    });
                }
                MainBaseAbortStage::Blocked(self.block(phase, reason))
            }
            _ => {
                return Err(MainBaseAbortResumeFailure {
                    error: MainBaseAbortProtocolError::UnexpectedResumeKind {
                        phase: expected_phase,
                    },
                    receipt,
                });
            }
        };

        self.stage = next_stage;
        self.outstanding = None;
        Ok(())
    }

    const fn block(
        &self,
        phase: MainBaseAbortPhase,
        reason: MainBaseAbortExternalBlock,
    ) -> MainBaseAbortBlock {
        MainBaseAbortBlock {
            phase,
            actors_processed: self.actors_processed,
            terrain_transformed: self.terrain_transformed,
            player_handoff_committed: self.player_handoff_committed,
            reason,
        }
    }
}

fn stage_for_actor(
    actor: MainBaseAbortActorObservation,
) -> Result<MainBaseAbortStage, MainBaseAbortPlanningBlock> {
    match classify_main_base_abort_actor(actor.facts()) {
        MainBaseAbortActorRoute::UnresolvedState { entity_id } => {
            Err(MainBaseAbortPlanningBlock::ActorStateUnresolved { entity_id })
        }
        _ => Ok(MainBaseAbortStage::Actor(actor)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        entity_id: u32,
        entity_type: u32,
        capability_flags: u32,
        state_flags: RetailRuntimeValue<u32>,
    ) -> MainBaseAbortActorFacts {
        MainBaseAbortActorFacts {
            entity_id,
            entity_type,
            capability_flags,
            state_flags,
        }
    }

    #[test]
    fn capability_route_precedes_type_and_state_gates() {
        assert_eq!(
            classify_main_base_abort_actor(facts(
                0x0459_0001,
                ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE,
                0x11,
                RetailRuntimeValue::Known(ORDINARY_DEATH_STATE_SKIP_MASK),
            )),
            MainBaseAbortActorRoute::AlternateCleanup {
                entity_id: 0x0459_0001,
            }
        );
    }

    #[test]
    fn ordinary_route_preserves_both_static_suppression_gates() {
        assert_eq!(
            classify_main_base_abort_actor(facts(
                0x0442_0001,
                17,
                8,
                RetailRuntimeValue::Known(0x0146_8825),
            )),
            MainBaseAbortActorRoute::OrdinaryDeath {
                entity_id: 0x0442_0001,
            }
        );
        assert_eq!(
            classify_main_base_abort_actor(facts(
                7,
                ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE,
                0,
                RetailRuntimeValue::Known(0),
            )),
            MainBaseAbortActorRoute::ExcludedType {
                entity_id: 7,
                entity_type: ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE,
            }
        );
        assert_eq!(
            classify_main_base_abort_actor(
                facts(8, 17, 0, RetailRuntimeValue::Known(0x1000_0000),)
            ),
            MainBaseAbortActorRoute::SuppressedByState {
                entity_id: 8,
                masked_state: 0x1000_0000,
            }
        );
    }

    #[test]
    fn unknown_live_state_never_becomes_an_assumed_death_callback() {
        assert_eq!(
            classify_main_base_abort_actor(facts(9, 17, 0, RetailRuntimeValue::Unresolved,)),
            MainBaseAbortActorRoute::UnresolvedState { entity_id: 9 }
        );
    }

    #[test]
    fn accepted_main_base_capture_representatives_take_ordinary_death() {
        // 20260724-201244-friendly-main-base-destruction, sample 5564:
        // these are the pre-transition words for the Main Base, villager,
        // spider, marker, scientist, Power Up, teleporter, Factory, and weight.
        let captured = [
            (0x0450_0001, 6, 0x20, 0x0EC2_8805),
            (0x0440_0001, 9, 0x1804, 0x00C6_8825),
            (0x0442_0001, 17, 8, 0x0146_8825),
            (0x0437_0001, 52, 0x100, 0x0640_0005),
            (0x044E_0001, 54, 0, 0x0042_8805),
            (0x0434_0001, 61, 0x40, 0x0820_8805),
            (0x043C_0001, 62, 0, 0x0003_8805),
            (0x043F_0001, 66, 0x84, 0x0EC2_8805),
            (0x0451_0001, 68, 0x1040, 0x0E40_8805),
        ];

        for (entity_id, entity_type, capability_flags, state_flags) in captured {
            assert_eq!(
                classify_main_base_abort_actor(facts(
                    entity_id,
                    entity_type,
                    capability_flags,
                    RetailRuntimeValue::Known(state_flags),
                )),
                MainBaseAbortActorRoute::OrdinaryDeath { entity_id },
                "type {entity_type} capture representative",
            );
        }
    }

    fn control() -> MainBaseAbortWorldControl {
        MainBaseAbortWorldControl {
            allocation_identity: 0xAA55,
            word_0x58: 0x1234,
            word_0x5a: 0x5678,
            dword_0x4c: 0x1122_3344,
            dword_0x88: 0x5566_7788,
            dword_0x8c: 0x99AA_BBCC,
        }
    }

    fn loaded_level_frame_descriptor() -> LevelDescriptor {
        let mut data = vec![0u8; 0xD0];
        data[0x4C..0x50].copy_from_slice(&0x650u32.to_le_bytes());
        data[0x54..0x56].copy_from_slice(&27u16.to_le_bytes());
        data[0x56..0x58].copy_from_slice(&305u16.to_le_bytes());
        data[0x58..0x5A].copy_from_slice(&32u16.to_le_bytes());
        data[0x5A..0x5C].copy_from_slice(&0u16.to_le_bytes());
        data[0x88..0x8C].copy_from_slice(&21u32.to_le_bytes());
        data[0x8C..0x90].copy_from_slice(&8u32.to_le_bytes());
        v2k_formats::levels::parse_level(&data).unwrap()
    }

    fn loaded_controller(generation: u64) -> MainBaseAbortControllerStorage {
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        MainBaseAbortControllerStorage::from_loaded_level(
            NonZeroU64::new(generation).unwrap(),
            &loaded_level_frame_descriptor(),
            false,
            &mut progress,
        )
        .unwrap()
        .0
    }

    #[test]
    fn loaded_controller_owns_the_exact_normal_frame_request() {
        let controller = loaded_controller(7);
        assert_eq!(controller.world_control_lease().allocation_identity, 7);
        assert_eq!(
            controller.player_state(),
            RetailRuntimeValue::Known(TimeTrophyState::Secured as u32)
        );
        assert_eq!(controller.time_trophy_remaining_millis_raw(), 0);
        assert!(!controller.abort_frame_submitted());
        assert_eq!(
            controller.submitted_frame_request(),
            MainBaseAbortFrameRequest {
                callback_address: MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS,
                retained_dword_0xac: 0,
                word_0xb0: 27,
                word_0xb2: 305,
                dword_0xb4: 0x650,
                dword_0xb8: 21,
                dword_0xbc: 8,
            }
        );
    }

    #[test]
    fn ordinary_hive_completion_marks_saved_without_occupying_results() {
        let mut controller = loaded_controller(8);
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(1));
        let outcome = controller.complete_current_world(&mut progress).unwrap();
        assert!(outcome.world_saved_now);
        assert!(!outcome.time_trophy_claimed_now);
        assert_eq!(progress.control_slot_world_saved(1), Some(true));
        assert_eq!(
            controller.time_trophy_state(),
            TimeTrophyState::WorldSavedOutsideActiveWindow
        );
    }

    #[test]
    fn post_terrain_commit_preserves_ac_and_atomically_submits_abort_frame() {
        let mut controller = loaded_controller(9);
        controller.stored_frame_request.retained_dword_0xac = 0xDEAD_BEEF;
        let request = controller
            .commit_post_terrain_abort(controller.world_control_lease())
            .unwrap();
        assert_eq!(
            controller.player_state(),
            RetailRuntimeValue::Known(MAIN_BASE_ABORT_PLAYER_STATE)
        );
        assert!(controller.abort_frame_submitted());
        assert_eq!(
            request,
            MainBaseAbortFrameRequest {
                callback_address: MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS,
                retained_dword_0xac: 0xDEAD_BEEF,
                word_0xb0: 32,
                word_0xb2: 0,
                dword_0xb4: 0x650,
                dword_0xb8: 21,
                dword_0xbc: 8,
            }
        );
        assert_eq!(controller.stored_frame_request(), request);
        assert_eq!(controller.submitted_frame_request(), request);
    }

    #[test]
    fn stale_world_control_lease_leaves_controller_untouched() {
        let mut controller = loaded_controller(11);
        let before = controller;
        assert_eq!(
            controller.commit_post_terrain_abort(MainBaseAbortWorldControlLease {
                allocation_identity: 12,
            }),
            Err(MainBaseAbortControllerLeaseMismatch {
                expected: before.world_control_lease(),
                actual: MainBaseAbortWorldControlLease {
                    allocation_identity: 12,
                },
            })
        );
        assert_eq!(controller, before);
    }

    fn actor(
        entity_id: u32,
        entity_type: u32,
        capability_flags: u32,
        state_flags: RetailRuntimeValue<u32>,
    ) -> MainBaseAbortActorObservation {
        MainBaseAbortActorObservation {
            lease: MainBaseAbortActorLease {
                entity_id,
                allocation_identity: u64::from(entity_id) << 12 | 0xA5,
            },
            entity_type,
            capability_flags,
            state_flags,
        }
    }

    fn issue(machine: &mut MainBaseAbortMachine) -> IssuedMainBaseAbortAction {
        match machine.poll() {
            MainBaseAbortPoll::Action(issued) => issued,
            other => panic!("expected action, got {other:?}"),
        }
    }

    #[test]
    fn null_world_control_suppresses_the_complete_systemic_body() {
        let id = MainBaseAbortTransactionId::new(1).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Unresolved,
        );
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Complete(MainBaseAbortCompletion {
                systemic_body_entered: false,
                actors_processed: 0,
                terrain_transformed: false,
                player_handoff_committed: false,
                full_frame_submitted: false,
            })
        );
    }

    #[test]
    fn unresolved_control_or_actor_state_never_issues_an_action() {
        let id = MainBaseAbortTransactionId::new(2).unwrap();
        let mut unresolved_control = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Known(None),
        );
        assert_eq!(
            unresolved_control.poll(),
            MainBaseAbortPoll::PlanningBlocked(MainBaseAbortPlanningBlock::WorldControlUnresolved)
        );

        let mut unresolved_actor = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(actor(9, 17, 0, RetailRuntimeValue::Unresolved))),
        );
        assert_eq!(
            unresolved_actor.poll(),
            MainBaseAbortPoll::PlanningBlocked(MainBaseAbortPlanningBlock::ActorStateUnresolved {
                entity_id: 9
            })
        );
    }

    #[test]
    fn systemic_body_preserves_actor_terrain_player_and_frame_order() {
        let actors = [
            actor(
                0x04A0_0001,
                46,
                ALTERNATE_CLEANUP_CAPABILITY_MASK,
                RetailRuntimeValue::Known(0),
            ),
            actor(0x0442_0001, 17, 8, RetailRuntimeValue::Known(0x0146_8825)),
            actor(
                0x0430_0001,
                ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE,
                0,
                RetailRuntimeValue::Known(0),
            ),
            actor(0x0431_0001, 9, 0, RetailRuntimeValue::Known(0x1000_0000)),
        ];
        let id = MainBaseAbortTransactionId::new(3).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(actors[0])),
        );

        let expected_routes = [
            MainBaseAbortActorRoute::AlternateCleanup {
                entity_id: actors[0].lease.entity_id,
            },
            MainBaseAbortActorRoute::OrdinaryDeath {
                entity_id: actors[1].lease.entity_id,
            },
            MainBaseAbortActorRoute::ExcludedType {
                entity_id: actors[2].lease.entity_id,
                entity_type: ORDINARY_DEATH_EXCLUDED_ENTITY_TYPE,
            },
            MainBaseAbortActorRoute::SuppressedByState {
                entity_id: actors[3].lease.entity_id,
                masked_state: 0x1000_0000,
            },
        ];
        for index in 0..actors.len() {
            let issued = issue(&mut machine);
            assert_eq!(
                issued.action,
                MainBaseAbortAction::ProcessActor {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: actors[index],
                    route: expected_routes[index],
                }
            );
            assert_eq!(
                machine.poll(),
                MainBaseAbortPoll::Awaiting(MainBaseAbortPhase::ActorSweep)
            );
            machine
                .resume(
                    issued.receipt,
                    MainBaseAbortResume::ActorProcessed {
                        phase: MainBaseAbortPhase::ActorSweep,
                        actor: actors[index].lease,
                        successor: RetailRuntimeValue::Known(actors.get(index + 1).copied()),
                    },
                )
                .unwrap();
        }

        let issued = issue(&mut machine);
        assert_eq!(
            issued.action,
            MainBaseAbortAction::TransformTerrain {
                phase: MainBaseAbortPhase::TerrainTransform,
                world_control_identity: control().allocation_identity,
            }
        );
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::TerrainTransformed {
                    phase: MainBaseAbortPhase::TerrainTransform,
                },
            )
            .unwrap();

        let request =
            MainBaseAbortFrameRequest::from_post_transform_observation(control(), 0xDEAD_BEEF);
        let issued = issue(&mut machine);
        assert_eq!(
            issued.action,
            MainBaseAbortAction::CommitPlayerHandoff {
                phase: MainBaseAbortPhase::PlayerHandoff,
                player_state: MAIN_BASE_ABORT_PLAYER_STATE,
                world_control: control().lease(),
                callback_address: MAIN_BASE_ABORT_FRAME_CALLBACK_ADDRESS,
            }
        );
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::PlayerHandoffCommitted {
                    phase: MainBaseAbortPhase::PlayerHandoff,
                    player_state: MAIN_BASE_ABORT_PLAYER_STATE,
                    world_control: control(),
                    request,
                },
            )
            .unwrap();

        let issued = issue(&mut machine);
        assert_eq!(
            issued.action,
            MainBaseAbortAction::SubmitFullFrame {
                phase: MainBaseAbortPhase::FullFrameRequest,
                request,
            }
        );
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::FullFrameSubmitted {
                    phase: MainBaseAbortPhase::FullFrameRequest,
                    request,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Complete(MainBaseAbortCompletion {
                systemic_body_entered: true,
                actors_processed: actors.len(),
                terrain_transformed: true,
                player_handoff_committed: true,
                full_frame_submitted: true,
            })
        );
    }

    #[test]
    fn post_callback_successor_controls_the_next_visited_allocation() {
        let first = actor(1, 17, 0, RetailRuntimeValue::Known(0));
        let removed_middle = actor(2, 17, 0, RetailRuntimeValue::Known(0));
        let rewired_tail = actor(3, 17, 0, RetailRuntimeValue::Known(0));
        let id = MainBaseAbortTransactionId::new(4).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(first)),
        );

        let issued = issue(&mut machine);
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::ActorProcessed {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: first.lease,
                    successor: RetailRuntimeValue::Known(Some(rewired_tail)),
                },
            )
            .unwrap();

        let issued = issue(&mut machine);
        assert_eq!(
            issued.action,
            MainBaseAbortAction::ProcessActor {
                phase: MainBaseAbortPhase::ActorSweep,
                actor: rewired_tail,
                route: MainBaseAbortActorRoute::OrdinaryDeath {
                    entity_id: rewired_tail.lease.entity_id,
                },
            }
        );
        assert_ne!(rewired_tail.lease, removed_middle.lease);
    }

    #[test]
    fn unresolved_post_callback_successor_blocks_after_the_committed_prefix() {
        let first = actor(1, 17, 0, RetailRuntimeValue::Known(0));
        let id = MainBaseAbortTransactionId::new(5).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(first)),
        );
        let issued = issue(&mut machine);
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::ActorProcessed {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: first.lease,
                    successor: RetailRuntimeValue::Unresolved,
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Blocked(MainBaseAbortBlock {
                phase: MainBaseAbortPhase::ActorSweep,
                actors_processed: 1,
                terrain_transformed: false,
                player_handoff_committed: false,
                reason: MainBaseAbortExternalBlock::ActorSuccessorUnavailable,
            })
        );
    }

    #[test]
    fn unresolved_successor_state_blocks_after_the_committed_prefix() {
        let first = actor(1, 17, 0, RetailRuntimeValue::Known(0));
        let unresolved_next = actor(2, 17, 0, RetailRuntimeValue::Unresolved);
        let id = MainBaseAbortTransactionId::new(8).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(first)),
        );
        let issued = issue(&mut machine);
        machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::ActorProcessed {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: first.lease,
                    successor: RetailRuntimeValue::Known(Some(unresolved_next)),
                },
            )
            .unwrap();
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Blocked(MainBaseAbortBlock {
                phase: MainBaseAbortPhase::ActorSweep,
                actors_processed: 1,
                terrain_transformed: false,
                player_handoff_committed: false,
                reason: MainBaseAbortExternalBlock::ActorProcessingUnavailable,
            })
        );
    }

    #[test]
    fn wrong_receipts_and_acknowledgements_do_not_advance_or_consume_the_action() {
        let first = actor(1, 17, 0, RetailRuntimeValue::Known(0));
        let id = MainBaseAbortTransactionId::new(6).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(first)),
        );
        let issued = issue(&mut machine);
        let forged = MainBaseAbortReceipt {
            transaction_id: MainBaseAbortTransactionId::new(7).unwrap(),
            action_sequence: issued.receipt.action_sequence,
            phase: issued.receipt.phase,
        };
        let failure = machine
            .resume(
                forged,
                MainBaseAbortResume::ActorProcessed {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: first.lease,
                    successor: RetailRuntimeValue::Known(None),
                },
            )
            .unwrap_err();
        assert!(matches!(
            failure.error,
            MainBaseAbortProtocolError::ReceiptTransactionMismatch { .. }
        ));
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Awaiting(MainBaseAbortPhase::ActorSweep)
        );

        let wrong_actor = MainBaseAbortActorLease {
            entity_id: 99,
            allocation_identity: 99,
        };
        let failure = machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::ActorProcessed {
                    phase: MainBaseAbortPhase::ActorSweep,
                    actor: wrong_actor,
                    successor: RetailRuntimeValue::Known(None),
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            MainBaseAbortProtocolError::ActorLeaseMismatch {
                expected: first.lease,
                actual: wrong_actor,
            }
        );
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Awaiting(MainBaseAbortPhase::ActorSweep)
        );
    }

    #[test]
    fn phase_incompatible_external_block_does_not_consume_the_action() {
        let first = actor(1, 17, 0, RetailRuntimeValue::Known(0));
        let id = MainBaseAbortTransactionId::new(9).unwrap();
        let mut machine = MainBaseAbortMachine::start(
            id,
            RetailRuntimeValue::Known(Some(control().lease())),
            RetailRuntimeValue::Known(Some(first)),
        );
        let issued = issue(&mut machine);
        let failure = machine
            .resume(
                issued.receipt,
                MainBaseAbortResume::Blocked {
                    phase: MainBaseAbortPhase::ActorSweep,
                    reason: MainBaseAbortExternalBlock::FullFrameSubmissionUnavailable,
                },
            )
            .unwrap_err();
        assert_eq!(
            failure.error,
            MainBaseAbortProtocolError::BlockReasonPhaseMismatch {
                phase: MainBaseAbortPhase::ActorSweep,
                reason: MainBaseAbortExternalBlock::FullFrameSubmissionUnavailable,
            }
        );
        assert_eq!(
            machine.poll(),
            MainBaseAbortPoll::Awaiting(MainBaseAbortPhase::ActorSweep)
        );
    }
}
