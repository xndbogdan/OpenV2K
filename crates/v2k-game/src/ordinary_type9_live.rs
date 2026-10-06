//! Fresh-Level-1 admission and retained state for ordinary type-9 actors.
//!
//! This module begins with the exact construction sidecar. The accepted retail
//! captures prove the six authored Level-1 allocations, their process-owned
//! Sub-D stagger seeds, and the mandatory first-query classifier reset. Static
//! retail code and `1X3XX.OVL` also close the four selector candidates:
//!
//! - Baddie Nearby (`0x08`) * 10 -> Run Away;
//! - Player Nearby (`0x01`) * 3 -> Attract Attention;
//! - Base Nearby (`0x20`) * 200 -> Go To Job;
//! - Always * 1 -> Wander Near Location.
//!
//! Fresh-New-Game construction now owns one shared `WorldFx` stream, resolves
//! candidates in authored order, runs `FUN_00425680`'s selector, dispatches
//! exactly one bounded initializer, and completes the native link/body-basis/
//! state-bit suffix. The resulting branch and task owners remain in manager
//! custody until their branch-specific production schedulers adopt them.
//!
//! The live seam in this module is correspondingly narrow. It can bind an
//! already-published ordinary-Wander task to the exact admitted entity and
//! atomically persist one detached D/I/A/B mover action. The bounded frame
//! coordinator also ticks an already-published Wander wrapper through the
//! shared three-slot owner. The exact construction receipt owns the immutable
//! anchor, while the entity's retained physical matrix owns the pre-mover
//! basis; the caller supplies only terrain, frame timing, and the shared RNG
//! stream. This module also owns the selected Wander callback authority minted
//! by the initializer. It does not yet attach that authority to the frame
//! scheduler; Run Away, Go-To-Job, and multi-task Attract Attention likewise
//! retain manager-side owners pending their dedicated scheduling bridges.

use crate::actor_animation::ActorAnimationController;
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{
    ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskVisitControl, ActorTaskWrapperFlags,
};
use crate::common_mover::sub_d::{
    FreshLevel1Type9ClassifierAdmission, Type9SubDFrameOwner, Type9SubDRuntime,
};
use crate::common_mover::type9::{
    OrdinaryType9FrameBlock, OrdinaryType9FrameRequest, OrdinaryType9FrameState,
    OrdinaryType9Topology, OrdinaryType9TopologyError,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_transaction::{
    OrdinaryType9Action, OrdinaryType9AppliedState, OrdinaryType9ExternalBlock,
    OrdinaryType9FrameLease, OrdinaryType9Poll, OrdinaryType9Resume, OrdinaryType9Transaction,
    OrdinaryType9TransactionId,
};
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity::{
    binary_angle_to_radians, radians_to_binary_angle, raw_position_world, world_position_raw,
    Entity,
};
use crate::entity_behavior::{
    behavior_program, BehaviorChoiceListSource, BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::EntityTypeRuntimeMetadata;
use crate::entity_collision_state::{
    RetailRuntimeValue, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
};
use crate::main_base_type9_abort::{
    exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR,
};
use crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID;
use crate::ordinary_type9_wander_owner::{
    ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
    OrdinaryType9WanderPostUnwind, OrdinaryType9WanderTransitionRequest,
};
use crate::wander_near_location::{
    map_common_mover_return, WanderNearCommonMoverReturn, WanderNearLifetimeStatus,
    WanderNearPrivateState, WanderNearRetarget,
};
use v2k_formats::terrain::TerrainGrid;

pub const FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES: [usize; 6] = [9, 10, 14, 15, 16, 22];
pub const FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS: [u8; 6] = [0x29, 0x2A, 0x2E, 0x2F, 0x30, 0x35];
pub const FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID: usize = 558;
/// Fixed state bits at the exact pre-publication boundary inside
/// `FUN_004104B0`. Only the two construction-time surface classifier bits are
/// outside this mask.
pub const FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK: u32 =
    !crate::entity_collision_state::SURFACE_STATE_MASK;
/// `0x06078801` constructor base, zero authored spawn parameter, current
/// resource domain, and Type-9 policy `0x2f` clearing `0x00010000`. Wrapper
/// finalization has not yet set bit four.
pub const FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE: u32 = 0x0606_8801;
/// First `FUN_00412DA0` state for the six authenticated fresh-Level-1 type-9
/// births. Independent consume-site traces `20260817-061455` and
/// `20260817-062155` both observe this exact word: callback set, wait-disable
/// clear. Both sampled first visits wait-cleared `+0xB2`; the randomized gate
/// can also admit a callback immediately under another process RNG history.
pub const FRESH_LEVEL1_ORDINARY_TYPE9_FIRST_SCHEDULER_STATE_VALUE: u32 = 0x0046_8805;
const ORDINARY_TYPE9_ENTITY_TYPE: u32 = 9;

/// Consume the authenticated native birth's first-scheduler publication once.
/// The transient `+0xB2` contribution is independent: neither a known value nor
/// later unresolved evidence can arm or replay this startup phase.
pub fn apply_fresh_level1_ordinary_type9_first_scheduler_state(
    collision: &mut crate::entity_collision_state::EntityCollisionRuntimeState,
) {
    if !collision.fresh_level1_type9_first_scheduler_pending {
        return;
    }
    collision.fresh_level1_type9_first_scheduler_pending = false;
    collision.state_flags_at_0x08 = crate::entity_collision_state::RetailStateWord::exact(
        FRESH_LEVEL1_ORDINARY_TYPE9_FIRST_SCHEDULER_STATE_VALUE,
    );
}

pub fn fresh_level1_ordinary_type9_pre_publication_state_matches(
    state: crate::entity_collision_state::RetailStateWord,
) -> bool {
    state.masked(FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_MASK)
        == RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE9_PRE_PUBLICATION_STATE_VALUE)
}

/// Construction facts shared by the collision and future task-runtime owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1OrdinaryType9SpawnFacts {
    pub retail_first_world: bool,
    pub authored_spawn_index: usize,
    pub entity_type: u32,
    pub active_model_slot: RetailRuntimeValue<usize>,
    pub active_model: Option<usize>,
    pub rotation: [u16; 3],
    /// Final constructor position retained at retail entity `+0x90`.
    ///
    /// Fresh Level-1 type 9 has Section-12 `+0xC0 = 0x2f`, so
    /// `FUN_0040D4A0` terrain-snaps current Y and then copies the complete
    /// `+0x96..+0x9a` position over the provisional constructor anchor. A
    /// missing construction terrain owner therefore remains unresolved.
    pub immutable_anchor_raw_at_0x90: RetailRuntimeValue<[i16; 3]>,
}

/// Private receipt proving that one authored allocation is among the six
/// captured fresh-Level-1 ordinary type-9 actors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1OrdinaryType9Admission {
    sub_d_stagger_seed: u8,
    immutable_anchor_raw_at_0x90: RetailRuntimeValue<[i16; 3]>,
    native: Option<crate::ordinary_type9_construction::OrdinaryType9NativeReceipt>,
}

impl FreshLevel1OrdinaryType9Admission {
    /// A native constructor owns its real process seed and allocation state.
    /// Captured admission remains a distinct policy for existing RE fixtures.
    pub(crate) const fn from_native(
        receipt: crate::ordinary_type9_construction::OrdinaryType9NativeReceipt,
    ) -> Self {
        Self {
            sub_d_stagger_seed: receipt
                .initial_components()
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            immutable_anchor_raw_at_0x90: receipt
                .initial_components()
                .immutable_anchor_raw_at_0x90(),
            native: Some(receipt),
        }
    }

    pub(crate) const fn native_receipt(
        self,
    ) -> Option<crate::ordinary_type9_construction::OrdinaryType9NativeReceipt> {
        self.native
    }

    pub const fn sub_d_stagger_seed(self) -> u8 {
        self.sub_d_stagger_seed
    }

    /// Seed the linear pre-selection component custody consumed by fresh
    /// production after common construction and before live-list linkage.
    pub const fn pending_initial_selection(self) -> OrdinaryType9PendingInitialSelection {
        if let Some(native) = self.native {
            return native.initial_components();
        }
        OrdinaryType9PendingInitialSelection {
            immutable_anchor_raw_at_0x90: self.immutable_anchor_raw_at_0x90,
            sub_d_frame_owner: Type9SubDFrameOwner::pending_fresh_level1_first_query_reset(
                self.sub_d_stagger_seed,
                FreshLevel1Type9ClassifierAdmission::ACCEPTED_FIRST_CONSUMER_TRACE,
            ),
            sub_d_runtime: Type9SubDRuntime::from_constructor(),
        }
    }
}

/// Per-entity state retained only across common construction, nearby
/// evaluation, selector RNG, and native context allocation. The selected
/// initializer consumes this value into explicit selected/fallback custody
/// before the entity is linked; generic/direct construction never receives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9PendingInitialSelection {
    /// Immutable final birth position copied from retail `+0x96..+0x9a` by
    /// `FUN_0040D4A0` after Type 9's mandatory terrain snap.
    immutable_anchor_raw_at_0x90: RetailRuntimeValue<[i16; 3]>,
    pub sub_d_frame_owner: Type9SubDFrameOwner,
    /// Deterministic steering words zeroed by `FUN_004203D0`.
    ///
    /// This remains beside the process-sensitive cache owner so live
    /// activation can move both into one frame state without reconstructing
    /// component state after behavior selection.
    pub sub_d_runtime: Type9SubDRuntime,
}

impl OrdinaryType9PendingInitialSelection {
    pub(crate) const fn from_native_components(
        immutable_anchor_raw: [i16; 3],
        sub_d_frame_owner: Type9SubDFrameOwner,
        sub_d_runtime: Type9SubDRuntime,
    ) -> Self {
        Self {
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(immutable_anchor_raw),
            sub_d_frame_owner,
            sub_d_runtime,
        }
    }

    /// Share the component data shape with a separately authenticated native
    /// constructor. The supplied frame owner retains that route's own origin
    /// evidence; this does not grant the Level-1 first-query reset policy.
    pub(crate) const fn from_native_constructor(
        immutable_anchor_raw: [i16; 3],
        sub_d_frame_owner: Type9SubDFrameOwner,
    ) -> Self {
        Self {
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(immutable_anchor_raw),
            sub_d_frame_owner,
            sub_d_runtime: Type9SubDRuntime::from_constructor(),
        }
    }

    pub const fn immutable_anchor_raw_at_0x90(&self) -> RetailRuntimeValue<[i16; 3]> {
        self.immutable_anchor_raw_at_0x90
    }
}

/// Terminal owner of the component state moved out of the dormant
/// initial-selection sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9SelectedRuntimeKind {
    RunAwayAcquiringPublished,
    RunAwayFleeingPublished,
    AttractAttentionPublished,
    AttractAttentionTargetRoutePublished,
    WanderNearPublished,
    GoToJobPublished,
    /// `FUN_0040CD70` has installed the carrying style and real None Primary.
    Carried,
    InitializerFailureFallbackPublished,
}

/// Component custody after one bounded selected initializer has completed.
///
/// This is deliberately separate from [`OrdinaryType9PendingInitialSelection`]:
/// a published task graph or terminal initializer fallback must never remain
/// labelled as awaiting initial behavior selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9SelectedComponentRuntime {
    components: OrdinaryType9PendingInitialSelection,
    kind: OrdinaryType9SelectedRuntimeKind,
}

impl OrdinaryType9SelectedComponentRuntime {
    pub(crate) const fn new(
        components: OrdinaryType9PendingInitialSelection,
        kind: OrdinaryType9SelectedRuntimeKind,
    ) -> Self {
        Self { components, kind }
    }

    pub const fn components(self) -> OrdinaryType9PendingInitialSelection {
        self.components
    }

    pub const fn kind(self) -> OrdinaryType9SelectedRuntimeKind {
        self.kind
    }

    #[cfg(test)]
    pub(crate) fn set_immutable_anchor_for_test(&mut self, anchor: RetailRuntimeValue<[i16; 3]>) {
        self.components.immutable_anchor_raw_at_0x90 = anchor;
    }

    /// Relabel component custody only after a selected behavior initializer
    /// has synchronously replaced its complete task/style graph.
    pub(crate) fn set_kind(&mut self, kind: OrdinaryType9SelectedRuntimeKind) {
        self.kind = kind;
    }

    /// `FUN_00409030` overwrites entity `+0x90` from its releasing proxy.
    /// This anchor is immutable during movement, but is not a lifetime birth
    /// constant across a materialiser release.
    pub(crate) fn apply_materialiser_release_anchor(&mut self, anchor: [i16; 3]) {
        self.components.immutable_anchor_raw_at_0x90 = RetailRuntimeValue::Known(anchor);
    }
}

/// Why a live owner could not be authenticated from retained construction and
/// exact Section-12 evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9LiveOwnerError {
    EntityNotAdmitted,
    EntityInactive,
    Topology(OrdinaryType9TopologyError),
}

/// Why an authenticated owner could not borrow the exact live callback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9RuntimeBindError {
    OwnerRuntimeMismatch {
        planned_owner_id: u32,
        runtime_owner_id: u32,
    },
    ProvenanceUnavailable,
    TaskLeaseUnavailable {
        visit: ActorTaskVisit,
    },
    TaskFamilyMismatch {
        visit: ActorTaskVisit,
    },
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationRuntimeUnavailable,
    ImmutableAnchorUnavailable,
    PhysicalBodyBasisUnavailable,
}

/// Proven live owner for one of the six fresh-Level-1 ordinary type-9 actors.
///
/// This proof is deliberately entity-specific. The retained pending-selection
/// receipt is the result of the rotation-aware construction admission; the
/// runtime entity does not retain generic pitch/roll words which could safely
/// recreate that admission later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9LiveOwner {
    entity_id: u32,
    topology: OrdinaryType9Topology,
}

impl OrdinaryType9LiveOwner {
    /// Authenticate an exact admitted entity and its A/B/D/I Section-12 route.
    pub fn from_entity_metadata(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, OrdinaryType9LiveOwnerError> {
        if !entity.active {
            return Err(OrdinaryType9LiveOwnerError::EntityInactive);
        }
        if !has_live_provenance(entity)
            || metadata.model_slots[0] as usize != FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID
        {
            return Err(OrdinaryType9LiveOwnerError::EntityNotAdmitted);
        }
        let topology =
            OrdinaryType9Topology::from_metadata(ORDINARY_TYPE9_ENTITY_TYPE as u16, metadata)
                .map_err(OrdinaryType9LiveOwnerError::Topology)?;
        Ok(Self {
            entity_id: entity.id,
            topology,
        })
    }

    /// Authenticate descriptor contact without broadening the legacy
    /// pending-custody Wander tick owner. Fresh production may expose this
    /// callback only after exact selected-Wander finalization.
    pub(crate) fn from_entity_metadata_for_descriptor_contact(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, OrdinaryType9LiveOwnerError> {
        if entity.ordinary_type9_pending_initial_selection.is_some() {
            return Self::from_entity_metadata(entity, metadata);
        }
        if !entity.active {
            return Err(OrdinaryType9LiveOwnerError::EntityInactive);
        }
        let topology =
            OrdinaryType9Topology::from_metadata(ORDINARY_TYPE9_ENTITY_TYPE as u16, metadata)
                .map_err(OrdinaryType9LiveOwnerError::Topology)?;
        let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
            return Err(OrdinaryType9LiveOwnerError::EntityNotAdmitted);
        };
        let selected_owner = OrdinaryType9SelectedWanderLiveOwner {
            entity_id: entity.id,
            visit: ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            },
            topology,
        };
        validate_selected_wander_live_owner(
            &selected_owner,
            entity,
            metadata,
            OrdinaryType9SelectedWanderValidationMode::Live,
        )
        .map_err(|_| OrdinaryType9LiveOwnerError::EntityNotAdmitted)?;
        Ok(Self {
            entity_id: entity.id,
            topology,
        })
    }

    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn topology(self) -> OrdinaryType9Topology {
        self.topology
    }

    /// Bind the exact surviving Wander wrapper and every mutable D/I/A/B
    /// destination through one live entity borrow.
    ///
    /// The constructor-randomized Sub-A speed must already be resolved. This
    /// legacy pending-custody seam neither installs a task nor fills a missing
    /// RNG result; fresh production uses its selected-custody owner instead.
    pub fn bind_entity_runtime<'a>(
        self,
        entity: &'a mut Entity,
        visit: ActorTaskVisit,
    ) -> Result<OrdinaryType9BoundRuntime<'a>, OrdinaryType9RuntimeBindError> {
        if entity.id != self.entity_id {
            return Err(OrdinaryType9RuntimeBindError::OwnerRuntimeMismatch {
                planned_owner_id: self.entity_id,
                runtime_owner_id: entity.id,
            });
        }
        if !entity.active || !has_live_provenance(entity) {
            return Err(OrdinaryType9RuntimeBindError::ProvenanceUnavailable);
        }
        validate_runtime_state(entity, visit)?;
        Ok(OrdinaryType9BoundRuntime {
            entity,
            visit,
            topology: self.topology,
        })
    }

    /// Tick one exact, already-published Wander callback in the caller's
    /// retail scheduler mode.
    ///
    /// This is not a behavior activator. The caller must already own the
    /// process RNG stream, while the admitted entity must retain both the
    /// immutable retail `entity + 0x90` anchor and the callback-entry physical
    /// body basis. Fresh production has a separate selected-custody Wander
    /// owner; this legacy pending owner cannot impersonate it.
    ///
    /// Errors reached after callback entry carry the committed
    /// lifetime/retarget prefix. Such a result has consumed the frame and must
    /// not be retried with the same timing or RNG position.
    /// `TransitionPending` is likewise a successful consumed frame; the caller
    /// must resolve its owner transition instead of ticking the task again.
    pub fn tick_published_wander(
        self,
        entity: &mut Entity,
        request: OrdinaryType9LiveFrameRequest<'_>,
        next_random: impl FnMut() -> u32,
    ) -> Result<OrdinaryType9LiveFrameOutcome, OrdinaryType9LiveFrameError> {
        if entity.id != self.entity_id {
            return Err(OrdinaryType9LiveFrameError::RuntimeBind(
                OrdinaryType9RuntimeBindError::OwnerRuntimeMismatch {
                    planned_owner_id: self.entity_id,
                    runtime_owner_id: entity.id,
                },
            ));
        }
        if !entity.active || !has_live_provenance(entity) {
            return Err(OrdinaryType9LiveFrameError::RuntimeBind(
                OrdinaryType9RuntimeBindError::ProvenanceUnavailable,
            ));
        }
        validate_runtime_state(entity, request.visit)
            .map_err(OrdinaryType9LiveFrameError::RuntimeBind)?;
        validate_single_published_wander(entity, request.visit)?;

        // Both values are callback-entry entity state in retail. Resolve them
        // before visit_slots advances lifetime, invokes the retarget callback,
        // or consumes a shared RNG word.
        let immutable_anchor_raw = match entity
            .ordinary_type9_pending_initial_selection
            .expect("live provenance must retain the Type-9 construction receipt")
            .immutable_anchor_raw_at_0x90()
        {
            RetailRuntimeValue::Known(anchor) => anchor,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9LiveFrameError::RuntimeBind(
                    OrdinaryType9RuntimeBindError::ImmutableAnchorUnavailable,
                ));
            }
        };
        let pre_mover_basis = match entity.physical_body_basis_q31() {
            RetailRuntimeValue::Known(basis) => basis,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9LiveFrameError::RuntimeBind(
                    OrdinaryType9RuntimeBindError::PhysicalBodyBasisUnavailable,
                ));
            }
        };

        tick_authenticated_published_wander(
            entity,
            self.topology,
            request,
            immutable_anchor_raw,
            pre_mover_basis,
            OrdinaryType9LiveComponentCustodyKind::Pending,
            next_random,
        )
    }
}

/// Linear live authority minted by a successful fresh-Level-1 class-6
/// publication.
///
/// The owner captures the exact Primary wrapper identity. It intentionally is
/// neither `Clone` nor `Copy`: once callback entry has consumed scheduler/RNG
/// state, only an explicit continue or transition-suppression result can
/// return the behavior authority. Before another tick, the caller must still
/// run the outer post-task suffix which rebuilds the physical body basis for
/// the newly committed heading.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9SelectedWanderLiveOwner {
    entity_id: u32,
    visit: ActorTaskVisit,
    topology: OrdinaryType9Topology,
}

impl OrdinaryType9SelectedWanderLiveOwner {
    /// Duplicate this lease only while the original manager is inaccessible
    /// inside the isolated Main Base abort transaction.
    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            visit: self.visit,
            topology: self.topology,
        }
    }

    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub const fn visit(&self) -> ActorTaskVisit {
        self.visit
    }

    /// Reauthenticate the exact initializer-minted task/private state after
    /// the outer constructor has published its body basis and state bit. This
    /// is used once when manager construction custody enters the scheduler;
    /// progressed live owners and Main Base no-op resumes deliberately use the
    /// normal live validation path instead.
    pub(crate) fn authenticates_finalized_mint(
        &self,
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> bool {
        validate_selected_wander_live_owner(
            self,
            entity,
            metadata,
            OrdinaryType9SelectedWanderValidationMode::FinalizedMint,
        )
        .is_ok()
    }

    pub(crate) const fn topology(&self) -> OrdinaryType9Topology {
        self.topology
    }

    /// Tick the exact selected-custody Wander wrapper captured at publication.
    ///
    /// Preflight failures precede elapsed-time and RNG consumption and return
    /// this linear owner. Any error after callback entry consumes it. A normal
    /// continue or a known live `0x1000` owner-transition suppression returns
    /// the behavior owner. That owner is not immediately tick-ready: the
    /// outer frame coordinator must first run the post-task body-basis rebuild
    /// corresponding to the committed heading.
    pub fn tick(
        self,
        entity: &mut Entity,
        metadata: &EntityTypeRuntimeMetadata,
        request: OrdinaryType9SelectedWanderLiveFrameRequest<'_>,
        next_random: impl FnMut() -> u32,
    ) -> Result<
        OrdinaryType9SelectedWanderLiveFrameOutcome,
        OrdinaryType9SelectedWanderLiveFrameFailure,
    > {
        let preflight = match validate_selected_wander_live_owner(
            &self,
            entity,
            metadata,
            OrdinaryType9SelectedWanderValidationMode::Live,
        ) {
            Ok(preflight) => preflight,
            Err(error) => {
                return Err(OrdinaryType9SelectedWanderLiveFrameFailure {
                    error: OrdinaryType9SelectedWanderLiveFrameError::Preflight(error),
                    owner: Some(self),
                });
            }
        };
        let visit = self.visit;
        let outcome = tick_authenticated_published_wander(
            entity,
            self.topology,
            request.with_visit(visit),
            preflight.immutable_anchor_raw,
            preflight
                .pre_mover_basis
                .expect("live validation requires the retained physical basis"),
            OrdinaryType9LiveComponentCustodyKind::SelectedWander,
            next_random,
        )
        .map_err(|error| OrdinaryType9SelectedWanderLiveFrameFailure {
            error: OrdinaryType9SelectedWanderLiveFrameError::Frame(error),
            owner: None,
        })?;

        match outcome {
            OrdinaryType9LiveFrameOutcome::Continue {
                committed_prefix,
                mover_return,
                ..
            } => Ok(OrdinaryType9SelectedWanderLiveFrameOutcome::Continue {
                owner: self,
                committed_prefix,
                mover_return,
            }),
            OrdinaryType9LiveFrameOutcome::TransitionPending {
                request,
                mover_return,
            } => {
                let transition = OrdinaryType9SelectedWanderTransitionRequest {
                    entity_id: self.entity_id,
                    owner_request: request,
                    selection: OrdinaryType9SelectedWanderTransitionSelection::TypeDefaultRootFirst,
                };
                match entity
                    .collision
                    .state_flags_at_0x08
                    .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
                {
                    RetailRuntimeValue::Known(0) => Ok(
                        OrdinaryType9SelectedWanderLiveFrameOutcome::TransitionPending {
                            transition,
                            mover_return,
                        },
                    ),
                    RetailRuntimeValue::Known(_) => Ok(
                        OrdinaryType9SelectedWanderLiveFrameOutcome::TransitionSuppressed {
                            owner: self,
                            transition,
                            mover_return,
                        },
                    ),
                    RetailRuntimeValue::Unresolved => {
                        Err(OrdinaryType9SelectedWanderLiveFrameFailure {
                            error: OrdinaryType9SelectedWanderLiveFrameError::TransitionGateUnresolved {
                                transition,
                                mover_return,
                            },
                            owner: None,
                        })
                    }
                }
            }
        }
    }
}

/// Mint the selected Wander owner at the successful initializer boundary.
/// Wrapper finalization deliberately has not happened yet, so the mint checks
/// the complete publication graph while leaving the body-basis/bit-four gate
/// to the first live tick.
pub(crate) fn issue_selected_wander_live_owner(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> OrdinaryType9SelectedWanderLiveOwner {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .expect("successful class-6 publication retains its Primary wrapper");
    let topology =
        OrdinaryType9Topology::from_metadata(ORDINARY_TYPE9_ENTITY_TYPE as u16, metadata)
            .expect("the selected initializer authenticated exact Type-9 topology");
    let owner = OrdinaryType9SelectedWanderLiveOwner {
        entity_id: entity.id,
        visit: ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        },
        topology,
    };
    validate_selected_wander_live_owner(
        &owner,
        entity,
        metadata,
        OrdinaryType9SelectedWanderValidationMode::Mint,
    )
    .expect("successful class-6 publication must mint one exact live owner");
    owner
}

/// Complete external inputs whose ownership is not represented by [`Entity`].
#[derive(Debug, Clone, Copy)]
pub struct OrdinaryType9LiveFrameRequest<'a> {
    pub visit: ActorTaskVisit,
    pub transaction_id: OrdinaryType9TransactionId,
    pub tracked_target: RetailRuntimeValue<
        Option<crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot>,
    >,
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    /// Retail's independent global frame delta consumed by Sub-D yaw.
    pub global_elapsed_micros: u32,
    /// Zero dispatches Sub-I; every nonzero retail word suppresses it while
    /// preserving Sub-D and the Sub-A/B tail.
    pub scheduler_mode: i32,
}

/// External live-frame inputs for the selected-custody owner. The task visit
/// is intentionally absent because it is linear authority captured by
/// [`OrdinaryType9SelectedWanderLiveOwner`].
#[derive(Debug, Clone, Copy)]
pub struct OrdinaryType9SelectedWanderLiveFrameRequest<'a> {
    pub transaction_id: OrdinaryType9TransactionId,
    pub tracked_target: RetailRuntimeValue<
        Option<crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot>,
    >,
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub scheduler_mode: i32,
}

impl<'a> OrdinaryType9SelectedWanderLiveFrameRequest<'a> {
    fn with_visit(self, visit: ActorTaskVisit) -> OrdinaryType9LiveFrameRequest<'a> {
        OrdinaryType9LiveFrameRequest {
            visit,
            transaction_id: self.transaction_id,
            tracked_target: self.tracked_target,
            terrain: self.terrain,
            elapsed_micros: self.elapsed_micros,
            global_elapsed_micros: self.global_elapsed_micros,
            scheduler_mode: self.scheduler_mode,
        }
    }
}

/// A fully consumed live frame.
///
/// `TransitionPending` is successful frame completion, not a retryable error:
/// a nonzero mover has already persisted its receipt-bound state before the
/// owner transition request is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9LiveFrameOutcome {
    Continue {
        visit: ActorTaskVisit,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
        mover_return: WanderNearCommonMoverReturn,
    },
    TransitionPending {
        request: OrdinaryType9WanderTransitionRequest,
        mover_return: WanderNearCommonMoverReturn,
    },
}

/// Fully interpreted selected-custody frame result.
#[derive(Debug, PartialEq, Eq)]
pub enum OrdinaryType9SelectedWanderLiveFrameOutcome {
    Continue {
        owner: OrdinaryType9SelectedWanderLiveOwner,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
        mover_return: WanderNearCommonMoverReturn,
    },
    TransitionSuppressed {
        owner: OrdinaryType9SelectedWanderLiveOwner,
        transition: OrdinaryType9SelectedWanderTransitionRequest,
        mover_return: WanderNearCommonMoverReturn,
    },
    TransitionPending {
        transition: OrdinaryType9SelectedWanderTransitionRequest,
        mover_return: WanderNearCommonMoverReturn,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9SelectedWanderTransitionSelection {
    TypeDefaultRootFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9SelectedWanderTransitionRequest {
    pub entity_id: u32,
    pub owner_request: OrdinaryType9WanderTransitionRequest,
    pub selection: OrdinaryType9SelectedWanderTransitionSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9SelectedWanderLivePreflightError {
    OwnerEntityMismatch {
        expected: u32,
        actual: u32,
    },
    EntityInactive,
    MetadataContractMismatch,
    SpawnOrModelProvenanceMismatch,
    PendingComponentCustodyPresent,
    SelectedComponentCustodyUnavailable,
    UnexpectedSelectedRuntimeKind {
        actual: OrdinaryType9SelectedRuntimeKind,
    },
    InitialSelectionMismatch,
    BehaviorContextMismatch,
    TaskLeaseChanged,
    TaskFamilyMismatch,
    AdditionalPublishedTask {
        slot: ActorTaskSlot,
    },
    TaskWrapperNotRunnable,
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationRuntimeUnavailable,
    ImmutableAnchorUnavailable,
    MintTaskStateMismatch,
    MintPhysicalBodyBasisAlreadyResolved,
    MintBodyBasisRebuiltStateBitNotKnownClear,
    PhysicalBodyBasisUnavailable,
    BodyBasisRebuiltStateBitClear,
    BodyBasisRebuiltStateBitUnresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9SelectedWanderLiveFrameError {
    Preflight(OrdinaryType9SelectedWanderLivePreflightError),
    Frame(OrdinaryType9LiveFrameError),
    TransitionGateUnresolved {
        transition: OrdinaryType9SelectedWanderTransitionRequest,
        mover_return: WanderNearCommonMoverReturn,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9SelectedWanderLiveFrameFailure {
    pub error: OrdinaryType9SelectedWanderLiveFrameError,
    owner: Option<OrdinaryType9SelectedWanderLiveOwner>,
}

impl OrdinaryType9SelectedWanderLiveFrameFailure {
    pub fn into_owner(self) -> Option<OrdinaryType9SelectedWanderLiveOwner> {
        self.owner
    }

    pub const fn owner(&self) -> Option<&OrdinaryType9SelectedWanderLiveOwner> {
        self.owner.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9LiveFrameError {
    RuntimeBind(OrdinaryType9RuntimeBindError),
    WrongTaskSlot {
        actual: ActorTaskSlot,
    },
    AdditionalPublishedTask {
        slot: ActorTaskSlot,
    },
    TaskAlreadyInCallback {
        visit: ActorTaskVisit,
    },
    /// The scheduler lifetime/retarget prefix was consumed before the mover
    /// reached an unresolved evidence boundary. Retrying this frame would
    /// advance scheduler time and RNG a second time.
    FrameBlocked {
        visit: ActorTaskVisit,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
        reason: OrdinaryType9FrameBlock,
    },
    /// The scheduler prefix was consumed, but the detached mover action could
    /// not be committed. The prefix makes the non-retryable partial outcome
    /// explicit to the caller.
    CommitBlocked {
        visit: ActorTaskVisit,
        committed_prefix: OrdinaryType9WanderCallbackPrefix,
        reason: OrdinaryType9ExternalBlock,
    },
    ExpectedVisitNotTicked,
}

/// Mutable component custody accepted by the selected Type-9 common-mover
/// binding.
///
/// Production adapters can construct this only from the selected component
/// owner. The private pending-custody constructor exists solely so the older
/// published-Wander seam can share the same transaction/commit path while it
/// is being retired in favor of branch-specific production custody.
pub(crate) struct OrdinaryType9SelectedCommonMoverComponentCustody<'a> {
    components: &'a mut OrdinaryType9PendingInitialSelection,
}

impl<'a> OrdinaryType9SelectedCommonMoverComponentCustody<'a> {
    pub(crate) fn from_selected(selected: &'a mut OrdinaryType9SelectedComponentRuntime) -> Self {
        Self {
            components: &mut selected.components,
        }
    }

    fn from_pending(components: &'a mut OrdinaryType9PendingInitialSelection) -> Self {
        Self { components }
    }

    fn reborrow(&mut self) -> OrdinaryType9SelectedCommonMoverComponentCustody<'_> {
        OrdinaryType9SelectedCommonMoverComponentCustody {
            components: &mut *self.components,
        }
    }
}

/// Complete selected-custody binding for one exact D -> I -> A -> B mover
/// invocation.
///
/// `staged_wander` may be a Run Away callback stage: reflected-target
/// substitution and later restoration therefore remain owned by that caller.
/// The mutable D/I/A/B values may likewise be caller-owned stage copies. This
/// helper publishes them together only after the detached transaction issues
/// and acknowledges its one linear commit receipt.
pub(crate) struct OrdinaryType9SelectedCommonMoverRequest<'state, 'terrain> {
    pub(crate) transaction_id: OrdinaryType9TransactionId,
    pub(crate) lease: OrdinaryType9FrameLease,
    pub(crate) topology: OrdinaryType9Topology,
    pub(crate) component_custody: OrdinaryType9SelectedCommonMoverComponentCustody<'state>,
    pub(crate) staged_wander: &'state mut WanderNearPrivateState,
    pub(crate) sub_a_runtime: &'state mut SubAPropulsionRuntime,
    pub(crate) actor_animation: &'state mut ActorAnimationController,
    pub(crate) heading_raw: &'state mut u16,
    pub(crate) velocity_raw: &'state mut [i16; 3],
    pub(crate) tracked_target: RetailRuntimeValue<
        Option<crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot>,
    >,
    pub(crate) terrain: &'terrain TerrainGrid,
    pub(crate) position_raw: [i16; 3],
    pub(crate) pre_mover_basis: Type9BodyBasis,
    pub(crate) elapsed_micros: u32,
    pub(crate) global_elapsed_micros: u32,
    pub(crate) scheduler_mode: i32,
}

/// A selected common-mover attempt either fails before issuing an external
/// action or consumes that action receipt into a durable commit block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdinaryType9SelectedCommonMoverError {
    FrameBlocked(OrdinaryType9FrameBlock),
    CommitBlocked(OrdinaryType9ExternalBlock),
}

/// Run and persist one selected Type-9 common-mover transaction.
///
/// The caller must authenticate the task wrapper and behavior owner before
/// constructing the request. This boundary owns only the exact component,
/// staged-private-state, and motion replacement covered by
/// [`OrdinaryType9AppliedState`]. A retail-zero or planning block leaves every
/// supplied mutable value unchanged.
pub(crate) fn run_selected_ordinary_type9_common_mover(
    request: OrdinaryType9SelectedCommonMoverRequest<'_, '_>,
    next_random: impl FnMut() -> u32,
) -> Result<WanderNearCommonMoverReturn, OrdinaryType9SelectedCommonMoverError> {
    run_selected_ordinary_type9_common_mover_with_animation_policy(
        request,
        crate::common_mover::actor_abdi::ActorAbdiAnimationPolicy::Neutral,
        next_random,
    )
}

pub(crate) fn run_selected_ordinary_type9_common_mover_with_animation_policy(
    mut request: OrdinaryType9SelectedCommonMoverRequest<'_, '_>,
    animation_policy: crate::common_mover::actor_abdi::ActorAbdiAnimationPolicy,
    next_random: impl FnMut() -> u32,
) -> Result<WanderNearCommonMoverReturn, OrdinaryType9SelectedCommonMoverError> {
    let initial = selected_common_mover_snapshot(&request);
    let mut transaction = OrdinaryType9Transaction::start_with_animation_policy(
        request.transaction_id,
        request.lease,
        initial.frame,
        OrdinaryType9FrameRequest {
            topology: request.topology,
            wander: initial.wander,
            tracked_target: request.tracked_target,
            terrain: request.terrain,
            position_raw: request.position_raw,
            pre_mover_right_q31: request.pre_mover_basis.lateral,
            pre_mover_forward_q31: request.pre_mover_basis.forward,
            heading_raw: initial.heading_raw,
            velocity_raw: initial.velocity_raw,
            elapsed_micros: request.elapsed_micros,
            global_elapsed_micros: request.global_elapsed_micros,
            scheduler_mode: request.scheduler_mode,
        },
        animation_policy,
        next_random,
    );

    match transaction.poll() {
        OrdinaryType9Poll::PlanningBlocked(block) => Err(
            OrdinaryType9SelectedCommonMoverError::FrameBlocked(block.reason),
        ),
        OrdinaryType9Poll::ReturnedZero(_) => Ok(WanderNearCommonMoverReturn::Zero),
        OrdinaryType9Poll::Action(issued) => {
            let response = commit_selected_common_mover_action(&mut request, issued.action);
            let blocked_reason = match response {
                OrdinaryType9Resume::Acknowledged { .. } => None,
                OrdinaryType9Resume::Blocked { reason } => Some(reason),
            };
            transaction
                .resume(issued.receipt, response)
                .expect("the selected live binding resumes its own unmodified linear receipt");
            match (blocked_reason, transaction.poll()) {
                (None, OrdinaryType9Poll::ReturnOne(_)) => {
                    Ok(WanderNearCommonMoverReturn::NonZero)
                }
                (Some(reason), OrdinaryType9Poll::Blocked(block)) => {
                    debug_assert_eq!(block.reason, reason);
                    Err(OrdinaryType9SelectedCommonMoverError::CommitBlocked(
                        reason,
                    ))
                }
                _ => unreachable!(
                    "acknowledged and blocked selected commits have exact terminal transaction states"
                ),
            }
        }
        OrdinaryType9Poll::Awaiting
        | OrdinaryType9Poll::Blocked(_)
        | OrdinaryType9Poll::ReturnOne(_) => {
            unreachable!("a fresh transaction has exactly one initial poll state")
        }
    }
}

fn selected_common_mover_snapshot(
    request: &OrdinaryType9SelectedCommonMoverRequest<'_, '_>,
) -> OrdinaryType9AppliedState {
    OrdinaryType9AppliedState {
        frame: OrdinaryType9FrameState {
            sub_d_frame_owner: request.component_custody.components.sub_d_frame_owner,
            sub_d_runtime: request.component_custody.components.sub_d_runtime,
            sub_a_runtime: *request.sub_a_runtime,
            actor_animation: *request.actor_animation,
        },
        wander: *request.staged_wander,
        heading_raw: *request.heading_raw,
        velocity_raw: *request.velocity_raw,
    }
}

fn commit_selected_common_mover_action(
    request: &mut OrdinaryType9SelectedCommonMoverRequest<'_, '_>,
    action: OrdinaryType9Action,
) -> OrdinaryType9Resume {
    let OrdinaryType9Action::CommitApplied {
        lease,
        before,
        after,
        step: _,
    } = action;
    if lease != request.lease {
        return OrdinaryType9Resume::Blocked {
            reason: OrdinaryType9ExternalBlock::LeaseUnavailable,
        };
    }
    if selected_common_mover_snapshot(request) != before {
        return OrdinaryType9Resume::Blocked {
            reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
        };
    }

    request.component_custody.components.sub_d_frame_owner = after.frame.sub_d_frame_owner;
    request.component_custody.components.sub_d_runtime = after.frame.sub_d_runtime;
    *request.sub_a_runtime = after.frame.sub_a_runtime;
    *request.actor_animation = after.frame.actor_animation;
    *request.staged_wander = after.wander;
    *request.heading_raw = after.heading_raw;
    *request.velocity_raw = after.velocity_raw;
    OrdinaryType9Resume::Acknowledged { committed: after }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrdinaryType9LiveComponentCustodyKind {
    Pending,
    SelectedWander,
}

fn tick_authenticated_published_wander(
    entity: &mut Entity,
    topology: OrdinaryType9Topology,
    request: OrdinaryType9LiveFrameRequest<'_>,
    immutable_anchor_raw: [i16; 3],
    pre_mover_basis: Type9BodyBasis,
    custody_kind: OrdinaryType9LiveComponentCustodyKind,
    mut next_random: impl FnMut() -> u32,
) -> Result<OrdinaryType9LiveFrameOutcome, OrdinaryType9LiveFrameError> {
    let position_raw = entity.position_raw();
    let entity_id = entity.id;
    let result = {
        let Entity {
            actor_tasks,
            ordinary_type9_pending_initial_selection,
            ordinary_type9_selected_component_runtime,
            sub_a_propulsion_runtime,
            actor_animation_runtime,
            heading,
            velocity,
            ..
        } = entity;
        let component_custody = match custody_kind {
            OrdinaryType9LiveComponentCustodyKind::Pending => {
                OrdinaryType9SelectedCommonMoverComponentCustody::from_pending(
                    ordinary_type9_pending_initial_selection
                        .as_mut()
                        .expect("legacy live preflight retained pending component custody"),
                )
            }
            OrdinaryType9LiveComponentCustodyKind::SelectedWander => {
                OrdinaryType9SelectedCommonMoverComponentCustody::from_selected(
                    ordinary_type9_selected_component_runtime
                        .as_mut()
                        .expect("selected live preflight retained selected component custody"),
                )
            }
        };
        let mut context = OrdinaryType9LiveFrameContext {
            entity_id,
            topology,
            request,
            position_raw,
            immutable_anchor_raw,
            pre_mover_basis,
            heading,
            velocity,
            component_custody,
            sub_a: sub_a_propulsion_runtime,
            animation: actor_animation_runtime,
            next_random: &mut next_random,
        };

        let result = actor_tasks.visit_slots_fresh_phased_with(
            &mut context,
            |context, state, visit| context.before_callback(state, visit),
            |context, owner, visit| context.callback(owner, visit),
            |context, _owner, visit, lifetime_status, callback| {
                context.after_unwind(visit, lifetime_status, callback)
            },
        );
        result.ok_or(OrdinaryType9LiveFrameError::ExpectedVisitNotTicked)
    };
    result?
}

#[derive(Debug)]
struct OrdinaryType9LiveCallback {
    retarget: WanderNearRetarget,
    mover: Result<WanderNearCommonMoverReturn, OrdinaryType9SelectedCommonMoverError>,
}

struct OrdinaryType9LiveFrameContext<'entity, 'request, Random> {
    entity_id: u32,
    topology: OrdinaryType9Topology,
    request: OrdinaryType9LiveFrameRequest<'request>,
    position_raw: [i16; 3],
    immutable_anchor_raw: [i16; 3],
    pre_mover_basis: Type9BodyBasis,
    heading: &'entity mut f32,
    velocity: &'entity mut [f32; 3],
    component_custody: OrdinaryType9SelectedCommonMoverComponentCustody<'entity>,
    sub_a: &'entity mut RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    animation: &'entity mut RetailRuntimeValue<Option<ActorAnimationController>>,
    next_random: &'entity mut Random,
}

impl<Random: FnMut() -> u32> OrdinaryType9LiveFrameContext<'_, '_, Random> {
    fn before_callback(
        &mut self,
        state: &mut ActorTaskRuntime,
        visit: ActorTaskVisit,
    ) -> WanderNearLifetimeStatus {
        debug_assert_eq!(visit, self.request.visit);
        let ActorTaskRuntime::OrdinaryType9Wander(state) = state else {
            unreachable!("the exact Wander visit was prevalidated")
        };
        state.before_callback(self.request.elapsed_micros)
    }

    fn callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
    ) -> OrdinaryType9LiveCallback {
        debug_assert_eq!(visit, self.request.visit);
        let (retarget, wander) = {
            let state = owner
                .task_state_mut(visit.task_id)
                .expect("the executing Wander wrapper must retain its state");
            let ActorTaskRuntime::OrdinaryType9Wander(state) = state else {
                unreachable!("the exact Wander visit was prevalidated")
            };
            let stage = state.stage_callback(self.immutable_anchor_raw, || (self.next_random)());
            (stage.retarget(), stage.private_state())
        };
        let mover = self.run_mover(owner, visit, wander);
        OrdinaryType9LiveCallback { retarget, mover }
    }

    fn after_unwind(
        &mut self,
        visit: ActorTaskVisit,
        lifetime_status: WanderNearLifetimeStatus,
        callback: OrdinaryType9LiveCallback,
    ) -> ActorTaskVisitControl<Result<OrdinaryType9LiveFrameOutcome, OrdinaryType9LiveFrameError>>
    {
        let committed_prefix = OrdinaryType9WanderCallbackPrefix {
            lifetime_status,
            retarget: callback.retarget,
        };
        let mover_return = match callback.mover {
            Ok(value) => value,
            Err(OrdinaryType9SelectedCommonMoverError::FrameBlocked(reason)) => {
                return ActorTaskVisitControl::Propagate(Err(
                    OrdinaryType9LiveFrameError::FrameBlocked {
                        visit,
                        committed_prefix,
                        reason,
                    },
                ));
            }
            Err(OrdinaryType9SelectedCommonMoverError::CommitBlocked(reason)) => {
                return ActorTaskVisitControl::Propagate(Err(
                    OrdinaryType9LiveFrameError::CommitBlocked {
                        visit,
                        committed_prefix,
                        reason,
                    },
                ));
            }
        };
        match ordinary_type9_wander_after_unwind(
            visit,
            committed_prefix,
            map_common_mover_return(mover_return),
        ) {
            OrdinaryType9WanderPostUnwind::Continue => {
                ActorTaskVisitControl::Propagate(Ok(OrdinaryType9LiveFrameOutcome::Continue {
                    visit,
                    committed_prefix,
                    mover_return,
                }))
            }
            OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => {
                unreachable!("the live coordinator never fabricates an unresolved mover return")
            }
            OrdinaryType9WanderPostUnwind::Transition(request) => ActorTaskVisitControl::Propagate(
                Ok(OrdinaryType9LiveFrameOutcome::TransitionPending {
                    request,
                    mover_return,
                }),
            ),
        }
    }

    fn run_mover(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        wander: WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, OrdinaryType9SelectedCommonMoverError> {
        let Some(ActorTaskRuntime::OrdinaryType9Wander(task_state)) =
            owner.task_state_mut(visit.task_id)
        else {
            return Err(OrdinaryType9SelectedCommonMoverError::CommitBlocked(
                OrdinaryType9ExternalBlock::LeaseUnavailable,
            ));
        };
        if task_state.private_state() != wander {
            return Err(OrdinaryType9SelectedCommonMoverError::CommitBlocked(
                OrdinaryType9ExternalBlock::StateCommitUnavailable,
            ));
        }

        let sub_a_runtime = match &mut *self.sub_a {
            RetailRuntimeValue::Known(Some(runtime)) => runtime,
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                unreachable!("prevalidated ordinary type-9 runtime must retain Sub-A")
            }
        };
        if sub_a_runtime.target_speed_raw() == RetailRuntimeValue::Unresolved {
            unreachable!("prevalidated ordinary type-9 Sub-A speed must be resolved")
        }
        let actor_animation = match &mut *self.animation {
            RetailRuntimeValue::Known(Some(runtime)) => runtime,
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                unreachable!("prevalidated ordinary type-9 runtime must retain Sub-I")
            }
        };
        let request = self.request;
        let mut staged_wander = wander;
        let mut heading_raw = radians_to_binary_angle(*self.heading);
        let mut velocity_raw = world_position_raw(*self.velocity);
        let next_random = &mut *self.next_random;
        let result = run_selected_ordinary_type9_common_mover(
            OrdinaryType9SelectedCommonMoverRequest {
                transaction_id: request.transaction_id,
                lease: OrdinaryType9FrameLease {
                    controlled_entity_id: self.entity_id,
                    task_visit: visit,
                },
                topology: self.topology,
                component_custody: self.component_custody.reborrow(),
                staged_wander: &mut staged_wander,
                sub_a_runtime,
                actor_animation,
                heading_raw: &mut heading_raw,
                velocity_raw: &mut velocity_raw,
                tracked_target: request.tracked_target,
                terrain: request.terrain,
                position_raw: self.position_raw,
                pre_mover_basis: self.pre_mover_basis,
                elapsed_micros: request.elapsed_micros,
                global_elapsed_micros: request.global_elapsed_micros,
                scheduler_mode: request.scheduler_mode,
            },
            || next_random(),
        );

        if result == Ok(WanderNearCommonMoverReturn::NonZero) {
            // The helper has completed every fallible lookup and acknowledged
            // the receipt-bound D/I/A/B stage. The wrapper borrow proves the
            // same task lease still owns this private record, so the remaining
            // raw-to-world conversions cannot split the commit.
            task_state.replace_private_state(staged_wander);
            *self.heading = binary_angle_to_radians(heading_raw);
            *self.velocity = raw_position_world(velocity_raw);
        }
        result
    }
}

#[derive(Debug, Clone, Copy)]
struct OrdinaryType9SelectedWanderLivePreflight {
    immutable_anchor_raw: [i16; 3],
    pre_mover_basis: Option<Type9BodyBasis>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrdinaryType9SelectedWanderValidationMode {
    Mint,
    FinalizedMint,
    Live,
}

fn validate_selected_wander_live_owner(
    owner: &OrdinaryType9SelectedWanderLiveOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    mode: OrdinaryType9SelectedWanderValidationMode,
) -> Result<OrdinaryType9SelectedWanderLivePreflight, OrdinaryType9SelectedWanderLivePreflightError>
{
    if owner.entity_id != entity.id {
        return Err(
            OrdinaryType9SelectedWanderLivePreflightError::OwnerEntityMismatch {
                expected: owner.entity_id,
                actual: entity.id,
            },
        );
    }
    if !entity.active {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::EntityInactive);
    }
    if entity.entity_type != ORDINARY_TYPE9_ENTITY_TYPE
        || !exact_level_one_type9_metadata(metadata)
        || entity.actor_common_axis_descriptor
            != RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR)
    {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::MetadataContractMismatch);
    }
    let native = entity
        .ordinary_type9_native_receipt
        .filter(|receipt| receipt.authenticates_entity(entity));
    if (entity.ordinary_type9_native_receipt.is_some() && native.is_none())
        || (!entity
            .authored_spawn_index
            .is_some_and(|index| FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES.contains(&index))
            && native.is_none())
        || entity.model_slots != [Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID); 4]
        || entity.collision.active_model_slot()
            != RetailRuntimeValue::Known(native.map_or(0, |receipt| receipt.active_slot()))
        || entity.model_index != Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID)
    {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::SpawnOrModelProvenanceMismatch);
    }
    if entity.ordinary_type9_pending_initial_selection.is_some() {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::PendingComponentCustodyPresent);
    }
    let selected = entity.ordinary_type9_selected_component_runtime.ok_or(
        OrdinaryType9SelectedWanderLivePreflightError::SelectedComponentCustodyUnavailable,
    )?;
    if selected.kind() != OrdinaryType9SelectedRuntimeKind::WanderNearPublished {
        return Err(
            OrdinaryType9SelectedWanderLivePreflightError::UnexpectedSelectedRuntimeKind {
                actual: selected.kind(),
            },
        );
    }

    let program = behavior_program(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        .expect("class-6 Wander Near is statically audited");
    let expected_selection = crate::entity_behavior::BehaviorSelection {
        choice_index: 3,
        program,
    };
    if entity.initial_behavior != RetailRuntimeValue::Known(Some(expected_selection)) {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::InitialSelectionMismatch);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::BehaviorContextMismatch);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.style_table_index_raw_at_0x10() != program.initial_style_table_index_raw
        || context.active_style().audited() != Some(program.initial_style)
        || program.initial_style.variant != 0
        || context.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || context.target_handle_at_0x08() != RetailRuntimeValue::Known(None)
        || context.auxiliary_word_at_0x0c() != RetailRuntimeValue::Known(0)
    {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::BehaviorContextMismatch);
    }

    if owner.visit.slot != ActorTaskSlot::Primary
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(owner.visit.task_id)
    {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::TaskLeaseChanged);
    }
    match entity.actor_tasks.task_state(owner.visit.task_id) {
        Some(ActorTaskRuntime::OrdinaryType9Wander(_)) => {}
        Some(_) => {
            return Err(OrdinaryType9SelectedWanderLivePreflightError::TaskFamilyMismatch);
        }
        None => return Err(OrdinaryType9SelectedWanderLivePreflightError::TaskLeaseChanged),
    }
    if entity.actor_tasks.wrapper_flags(owner.visit.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9SelectedWanderLivePreflightError::TaskWrapperNotRunnable);
    }
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        if entity.actor_tasks.task_in_slot(slot).is_some() {
            return Err(
                OrdinaryType9SelectedWanderLivePreflightError::AdditionalPublishedTask { slot },
            );
        }
    }

    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => {
            if sub_a.target_speed_raw() == RetailRuntimeValue::Unresolved {
                return Err(
                    OrdinaryType9SelectedWanderLivePreflightError::SubATargetSpeedUnresolved,
                );
            }
        }
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9SelectedWanderLivePreflightError::SubARuntimeUnavailable);
        }
    }
    if !matches!(
        entity.actor_animation_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(
            OrdinaryType9SelectedWanderLivePreflightError::ActorAnimationRuntimeUnavailable,
        );
    }
    let immutable_anchor_raw = match selected.components.immutable_anchor_raw_at_0x90() {
        RetailRuntimeValue::Known(anchor) => anchor,
        RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9SelectedWanderLivePreflightError::ImmutableAnchorUnavailable);
        }
    };
    if matches!(
        mode,
        OrdinaryType9SelectedWanderValidationMode::Mint
            | OrdinaryType9SelectedWanderValidationMode::FinalizedMint
    ) {
        let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
            entity.actor_tasks.task_state(owner.visit.task_id)
        else {
            unreachable!("the selected Wander task family was already validated")
        };
        if wander.elapsed_ms() != 0
            || wander.private_state()
                != WanderNearPrivateState::ordinary_type9(immutable_anchor_raw)
        {
            return Err(OrdinaryType9SelectedWanderLivePreflightError::MintTaskStateMismatch);
        }
    }
    let (pre_mover_basis, basis_state) = match mode {
        OrdinaryType9SelectedWanderValidationMode::Mint => {
            if entity.physical_body_basis_q31() != RetailRuntimeValue::Unresolved {
                return Err(
                    OrdinaryType9SelectedWanderLivePreflightError::MintPhysicalBodyBasisAlreadyResolved,
                );
            }
            (
                None,
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(BODY_BASIS_REBUILT_STATE_BIT),
            )
        }
        OrdinaryType9SelectedWanderValidationMode::FinalizedMint
        | OrdinaryType9SelectedWanderValidationMode::Live => {
            let basis = match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => basis,
                RetailRuntimeValue::Unresolved => {
                    return Err(
                        OrdinaryType9SelectedWanderLivePreflightError::PhysicalBodyBasisUnavailable,
                    );
                }
            };
            (
                Some(basis),
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(BODY_BASIS_REBUILT_STATE_BIT),
            )
        }
    };
    match (mode, basis_state) {
        (OrdinaryType9SelectedWanderValidationMode::Mint, RetailRuntimeValue::Known(0)) => {}
        (OrdinaryType9SelectedWanderValidationMode::Mint, _) => {
            return Err(
                OrdinaryType9SelectedWanderLivePreflightError::MintBodyBasisRebuiltStateBitNotKnownClear,
            );
        }
        (
            OrdinaryType9SelectedWanderValidationMode::FinalizedMint,
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT),
        ) => {}
        (
            OrdinaryType9SelectedWanderValidationMode::FinalizedMint,
            RetailRuntimeValue::Known(_),
        ) => {
            return Err(
                OrdinaryType9SelectedWanderLivePreflightError::BodyBasisRebuiltStateBitClear,
            );
        }
        (
            OrdinaryType9SelectedWanderValidationMode::Live,
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT),
        ) => {}
        (OrdinaryType9SelectedWanderValidationMode::Live, RetailRuntimeValue::Known(_)) => {
            return Err(
                OrdinaryType9SelectedWanderLivePreflightError::BodyBasisRebuiltStateBitClear,
            );
        }
        (
            OrdinaryType9SelectedWanderValidationMode::FinalizedMint
            | OrdinaryType9SelectedWanderValidationMode::Live,
            RetailRuntimeValue::Unresolved,
        ) => {
            return Err(
                OrdinaryType9SelectedWanderLivePreflightError::BodyBasisRebuiltStateBitUnresolved,
            );
        }
    }

    Ok(OrdinaryType9SelectedWanderLivePreflight {
        immutable_anchor_raw,
        pre_mover_basis,
    })
}

fn validate_single_published_wander(
    entity: &Entity,
    expected: ActorTaskVisit,
) -> Result<(), OrdinaryType9LiveFrameError> {
    if expected.slot != ActorTaskSlot::Primary {
        return Err(OrdinaryType9LiveFrameError::WrongTaskSlot {
            actual: expected.slot,
        });
    }
    if entity.actor_tasks.wrapper_flags(expected.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(OrdinaryType9LiveFrameError::TaskAlreadyInCallback { visit: expected });
    }
    for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
        let Some(task_id) = entity.actor_tasks.task_in_slot(slot) else {
            continue;
        };
        if slot != expected.slot || task_id != expected.task_id {
            return Err(OrdinaryType9LiveFrameError::AdditionalPublishedTask { slot });
        }
    }
    Ok(())
}

/// One exclusive live borrow capable of applying one detached mover action.
pub struct OrdinaryType9BoundRuntime<'a> {
    entity: &'a mut Entity,
    visit: ActorTaskVisit,
    topology: OrdinaryType9Topology,
}

impl OrdinaryType9BoundRuntime<'_> {
    pub const fn task_visit(&self) -> ActorTaskVisit {
        self.visit
    }

    pub const fn topology(&self) -> OrdinaryType9Topology {
        self.topology
    }

    /// Snapshot every field covered by the detached transaction's `before`
    /// comparison.
    pub fn current_state(
        &self,
    ) -> Result<OrdinaryType9AppliedState, OrdinaryType9RuntimeBindError> {
        snapshot_runtime_state(self.entity, self.visit)
    }

    /// Consume and either atomically commit or reject one detached action.
    ///
    /// No mutation occurs unless both the entity/task lease and the complete
    /// before-state still match. The action and this exclusive binding are
    /// consumed, so the same issued action cannot be replayed through this
    /// seam.
    pub fn commit_action(self, action: OrdinaryType9Action) -> OrdinaryType9Resume {
        let OrdinaryType9Action::CommitApplied {
            lease,
            before,
            after,
            step: _,
        } = action;

        if lease
            != (OrdinaryType9FrameLease {
                controlled_entity_id: self.entity.id,
                task_visit: self.visit,
            })
        {
            return OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::LeaseUnavailable,
            };
        }

        let Ok(current) = snapshot_runtime_state(self.entity, self.visit) else {
            return OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::LeaseUnavailable,
            };
        };
        if current != before {
            return OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
            };
        }

        // Every fallible lookup above has completed. With the exclusive entity
        // borrow held, none of these destinations can disappear between the
        // comparison and this complete replacement.
        {
            let pending = self
                .entity
                .ordinary_type9_pending_initial_selection
                .as_mut()
                .expect("validated ordinary type-9 provenance must retain Sub-D state");
            pending.sub_d_frame_owner = after.frame.sub_d_frame_owner;
            pending.sub_d_runtime = after.frame.sub_d_runtime;

            let RetailRuntimeValue::Known(Some(sub_a)) = &mut self.entity.sub_a_propulsion_runtime
            else {
                unreachable!("validated ordinary type-9 runtime must retain Sub-A");
            };
            *sub_a = after.frame.sub_a_runtime;

            let RetailRuntimeValue::Known(Some(animation)) =
                &mut self.entity.actor_animation_runtime
            else {
                unreachable!("validated ordinary type-9 runtime must retain Sub-I");
            };
            *animation = after.frame.actor_animation;

            let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
                self.entity.actor_tasks.task_state_mut(self.visit.task_id)
            else {
                unreachable!("validated ordinary type-9 task lease must survive commit");
            };
            wander.replace_private_state(after.wander);
        }
        self.entity.set_heading_raw(after.heading_raw);
        self.entity.set_velocity_raw(after.velocity_raw);

        OrdinaryType9Resume::Acknowledged { committed: after }
    }
}

fn has_live_provenance(entity: &Entity) -> bool {
    let native = entity
        .ordinary_type9_native_receipt
        .filter(|receipt| receipt.authenticates_entity(entity));
    entity.entity_type == ORDINARY_TYPE9_ENTITY_TYPE
        && (entity.ordinary_type9_native_receipt.is_none() || native.is_some())
        && (entity
            .authored_spawn_index
            .is_some_and(|index| FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES.contains(&index))
            || native.is_some())
        && entity.collision.active_model_slot()
            == RetailRuntimeValue::Known(native.map_or(0, |receipt| receipt.active_slot()))
        && entity.model_slots[0] == Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID)
        && entity.model_index == Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID)
        && entity.ordinary_type9_pending_initial_selection.is_some()
        && entity.ordinary_type9_selected_component_runtime.is_none()
        && entity.main_base_type9_death_component_runtime.is_none()
}

fn validate_runtime_state(
    entity: &Entity,
    visit: ActorTaskVisit,
) -> Result<(), OrdinaryType9RuntimeBindError> {
    if entity.actor_tasks.task_in_slot(visit.slot) != Some(visit.task_id) {
        return Err(OrdinaryType9RuntimeBindError::TaskLeaseUnavailable { visit });
    }
    match entity.actor_tasks.task_state(visit.task_id) {
        Some(ActorTaskRuntime::OrdinaryType9Wander(_)) => {}
        Some(_) => return Err(OrdinaryType9RuntimeBindError::TaskFamilyMismatch { visit }),
        None => return Err(OrdinaryType9RuntimeBindError::TaskLeaseUnavailable { visit }),
    }
    match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => {
            if sub_a.target_speed_raw() == RetailRuntimeValue::Unresolved {
                return Err(OrdinaryType9RuntimeBindError::SubATargetSpeedUnresolved);
            }
        }
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RuntimeBindError::SubARuntimeUnavailable);
        }
    }
    match entity.actor_animation_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return Err(OrdinaryType9RuntimeBindError::ActorAnimationRuntimeUnavailable);
        }
    }
    Ok(())
}

fn snapshot_runtime_state(
    entity: &Entity,
    visit: ActorTaskVisit,
) -> Result<OrdinaryType9AppliedState, OrdinaryType9RuntimeBindError> {
    validate_runtime_state(entity, visit)?;
    let pending = entity
        .ordinary_type9_pending_initial_selection
        .ok_or(OrdinaryType9RuntimeBindError::ProvenanceUnavailable)?;
    let RetailRuntimeValue::Known(Some(sub_a_runtime)) = entity.sub_a_propulsion_runtime else {
        unreachable!("validated ordinary type-9 runtime must retain Sub-A");
    };
    let RetailRuntimeValue::Known(Some(actor_animation)) = entity.actor_animation_runtime else {
        unreachable!("validated ordinary type-9 runtime must retain Sub-I");
    };
    let Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) =
        entity.actor_tasks.task_state(visit.task_id)
    else {
        unreachable!("validated ordinary type-9 task lease must retain Wander");
    };
    Ok(OrdinaryType9AppliedState {
        frame: OrdinaryType9FrameState {
            sub_d_frame_owner: pending.sub_d_frame_owner,
            sub_d_runtime: pending.sub_d_runtime,
            sub_a_runtime,
            actor_animation,
        },
        wander: wander.private_state(),
        heading_raw: entity.heading_raw(),
        velocity_raw: entity.velocity_raw(),
    })
}

/// Admit only the exact six captured authored allocations.
///
/// The active model/slot and authored rotation are part of the same accepted
/// Level-1 boundary already used by the collision owner. Centralizing the
/// predicate keeps task storage and collision provenance from drifting apart.
pub fn admit_fresh_level1_ordinary_type9(
    facts: FreshLevel1OrdinaryType9SpawnFacts,
) -> Option<FreshLevel1OrdinaryType9Admission> {
    if !facts.retail_first_world
        || facts.entity_type != ORDINARY_TYPE9_ENTITY_TYPE
        || facts.active_model_slot != RetailRuntimeValue::Known(0)
        || facts.active_model != Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID)
        || facts.rotation != [0, 0, 0]
    {
        return None;
    }

    FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES
        .iter()
        .zip(FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS)
        .find_map(|(&spawn_index, sub_d_stagger_seed)| {
            (facts.authored_spawn_index == spawn_index).then_some(
                FreshLevel1OrdinaryType9Admission {
                    sub_d_stagger_seed,
                    immutable_anchor_raw_at_0x90: facts.immutable_anchor_raw_at_0x90,
                    native: None,
                },
            )
        })
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::num::NonZeroU64;

    use crate::actor_animation::ActorAnimationController;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::ActorTaskSlot;
    use crate::common_mover::sub_d::ORDINARY_TYPE9_SUB_D;
    use crate::common_mover::type9::OrdinaryType9FrameStep;
    use crate::common_mover::SubAPropulsionRuntime;
    use crate::entity::{Entity, EntityKind};
    use crate::entity_collision_state::{CommonMoverComponentTopology, EntityTypeRuntimeMetadata};
    use crate::ordinary_type9_wander_owner::plan_ordinary_type9_wander_setup;
    use v2k_formats::collision::{
        ActorAnimationDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    };
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    use super::*;

    #[test]
    fn native_first_scheduler_phase_is_consumed_once_independently_of_mass_offset() {
        let mut collision =
            crate::entity_collision_state::EntityCollisionRuntimeState::unresolved_port_entity(0);
        collision.fresh_level1_type9_first_scheduler_pending = true;
        collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut collision);
        assert_eq!(
            collision.state_flags_at_0x08.known_value_bits(),
            FRESH_LEVEL1_ORDINARY_TYPE9_FIRST_SCHEDULER_STATE_VALUE
        );
        assert!(!collision.fresh_level1_type9_first_scheduler_pending);
        assert_eq!(
            collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(7)
        );

        let later_flags = crate::entity_collision_state::RetailStateWord::exact(0x0646_8805);
        collision.state_flags_at_0x08 = later_flags;
        for later_offset in [
            RetailRuntimeValue::Known(11),
            RetailRuntimeValue::Unresolved,
        ] {
            collision.animation_offset_at_0xb2 = later_offset;
            apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut collision);
            assert_eq!(collision.state_flags_at_0x08, later_flags);
            assert_eq!(collision.animation_offset_at_0xb2, later_offset);
        }
    }

    #[test]
    fn generic_unresolved_offset_does_not_authorize_fresh_scheduler_publication() {
        let mut collision =
            crate::entity_collision_state::EntityCollisionRuntimeState::from_constructor(
                None,
                0,
                crate::entity_collision_state::RetailStateWord::exact(0x0606_8805),
            );
        let before = collision.clone();
        apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut collision);
        assert_eq!(collision, before);
        assert_eq!(
            collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Unresolved
        );
    }

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
    const LIVE_TEST_ANCHOR_RAW: [i16; 3] = [100, 20, -300];

    fn live_test_basis() -> Type9BodyBasis {
        // The live fixture later publishes heading 0x1800. Keeping the retained
        // constructor matrix at a cardinal 0x4000 yaw makes accidental Euler
        // reconstruction observable in mover output.
        Type9BodyBasis::from_angle_words(0x4000, 0, 0)
    }

    fn exact_facts(spawn_index: usize) -> FreshLevel1OrdinaryType9SpawnFacts {
        FreshLevel1OrdinaryType9SpawnFacts {
            retail_first_world: true,
            authored_spawn_index: spawn_index,
            entity_type: ORDINARY_TYPE9_ENTITY_TYPE,
            active_model_slot: RetailRuntimeValue::Known(0),
            active_model: Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID),
            rotation: [0; 3],
            immutable_anchor_raw_at_0x90: RetailRuntimeValue::Known(LIVE_TEST_ANCHOR_RAW),
        }
    }

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID as u16, 0, 0, 0],
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
        }
    }

    fn admitted_entity(id: u32) -> Entity {
        let mut entity = Entity::unresolved_port_entity(
            id,
            EntityKind::Unknown(ORDINARY_TYPE9_ENTITY_TYPE),
            ORDINARY_TYPE9_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(9);
        entity.model_slots[0] = Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID);
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID);
        entity.ordinary_type9_pending_initial_selection = Some(
            admit_fresh_level1_ordinary_type9(exact_facts(9))
                .unwrap()
                .pending_initial_selection(),
        );
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(live_test_basis());
        entity
    }

    fn entity_with_runtime(id: u32) -> (Entity, ActorTaskVisit) {
        let mut entity = admitted_entity(id);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(SUB_I).unwrap(),
        ));

        let mut sub_a =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(250), 1, 100);
        plan_ordinary_type9_wander_setup([100, 20, -300], SUB_A.target_speed_base_raw)
            .apply(&mut entity.actor_tasks, &mut sub_a, |specification| {
                Ok::<_, Infallible>(
                    specification
                        .prepare_after_allocation(|| 0x1234_5678)
                        .map_task(ActorTaskRuntime::OrdinaryType9Wander),
                )
            })
            .unwrap();
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
        entity.set_motion_raw([100, 20, -300], [3, -4, 5]);
        entity.set_heading_raw(0x1800);
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap(),
        };
        (entity, visit)
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

    fn live_frame_request<'a>(
        visit: ActorTaskVisit,
        terrain: &'a TerrainGrid,
    ) -> OrdinaryType9LiveFrameRequest<'a> {
        OrdinaryType9LiveFrameRequest {
            visit,
            transaction_id: OrdinaryType9TransactionId::new(NonZeroU64::new(1).unwrap()),
            tracked_target: RetailRuntimeValue::Unresolved,
            terrain,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            scheduler_mode: 0,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct LiveAtomicSnapshot {
        pending: Option<OrdinaryType9PendingInitialSelection>,
        sub_a: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
        animation: RetailRuntimeValue<Option<ActorAnimationController>>,
        task: ActorTaskRuntime,
        wrapper_flags: ActorTaskWrapperFlags,
        position_raw: [i16; 3],
        heading_raw: u16,
        velocity_raw: [i16; 3],
        physical_body_basis_q31: RetailRuntimeValue<Type9BodyBasis>,
    }

    fn live_atomic_snapshot(entity: &Entity, visit: ActorTaskVisit) -> LiveAtomicSnapshot {
        LiveAtomicSnapshot {
            pending: entity.ordinary_type9_pending_initial_selection,
            sub_a: entity.sub_a_propulsion_runtime,
            animation: entity.actor_animation_runtime,
            task: *entity
                .actor_tasks
                .task_state(visit.task_id)
                .expect("live fixture retains its Wander task"),
            wrapper_flags: entity
                .actor_tasks
                .wrapper_flags(visit.task_id)
                .expect("live fixture retains its task wrapper"),
            position_raw: entity.position_raw(),
            heading_raw: entity.heading_raw(),
            velocity_raw: entity.velocity_raw(),
            physical_body_basis_q31: entity.physical_body_basis_q31,
        }
    }

    fn assert_live_preflight_block_is_atomic(
        mut entity: Entity,
        visit: ActorTaskVisit,
        terrain: &TerrainGrid,
        expected: OrdinaryType9RuntimeBindError,
    ) {
        let owner =
            OrdinaryType9LiveOwner::from_entity_metadata(&entity, &exact_metadata()).unwrap();
        let before = live_atomic_snapshot(&entity, visit);
        let mut draws = 0;

        assert_eq!(
            owner.tick_published_wander(&mut entity, live_frame_request(visit, terrain), || {
                draws += 1;
                1
            },),
            Err(OrdinaryType9LiveFrameError::RuntimeBind(expected))
        );
        assert_eq!(draws, 0, "preflight must precede scheduler retarget RNG");
        assert_eq!(
            live_atomic_snapshot(&entity, visit),
            before,
            "preflight failure must precede elapsed, wrapper, task, component, and motion writes"
        );
    }

    fn set_wander_target(entity: &mut Entity, visit: ActorTaskVisit, target: [i16; 3]) {
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            panic!("fixture must retain the published Wander task")
        };
        let mut private = state.private_state();
        private.target_position_raw = target;
        state.replace_private_state(private);
    }

    fn changed_after(mut before: OrdinaryType9AppliedState) -> OrdinaryType9AppliedState {
        before.frame.sub_d_frame_owner =
            Type9SubDFrameOwner::from_retail_state([1, 2, 3, 4, 5, 6, 7, 8], [9, 10], 11);
        before.frame.sub_d_runtime = Type9SubDRuntime {
            yaw_rate_raw: -700,
            last_yaw_step_raw: 19,
        };
        before.frame.sub_a_runtime =
            SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(333), -1, 90);
        before.wander.target_position_raw = [-400, 20, 600];
        before.wander.direction = -1;
        before.wander.reversal_timer_ms = 45;
        before.heading_raw = 0x2800;
        before.velocity_raw = [-11, 12, -13];
        before
    }

    fn action(
        lease: OrdinaryType9FrameLease,
        before: OrdinaryType9AppliedState,
        after: OrdinaryType9AppliedState,
    ) -> OrdinaryType9Action {
        OrdinaryType9Action::CommitApplied {
            lease,
            before,
            after,
            step: OrdinaryType9FrameStep {
                wander: after.wander,
                heading_raw: after.heading_raw,
                velocity_raw: after.velocity_raw,
                yaw_step_raw: after.frame.sub_d_runtime.last_yaw_step_raw,
                actor_animation_selector: after.frame.actor_animation.output(),
                propulsion_applied: true,
            },
        }
    }

    fn lease(entity_id: u32, visit: ActorTaskVisit) -> OrdinaryType9FrameLease {
        OrdinaryType9FrameLease {
            controlled_entity_id: entity_id,
            task_visit: visit,
        }
    }

    #[test]
    fn exact_fresh_level1_spawns_retain_captured_sub_d_seeds() {
        for ((spawn_index, expected_seed), ordinal) in FRESH_LEVEL1_ORDINARY_TYPE9_SPAWN_INDICES
            .into_iter()
            .zip(FRESH_LEVEL1_ORDINARY_TYPE9_SUB_D_SEEDS)
            .zip(0..)
        {
            let admission = admit_fresh_level1_ordinary_type9(exact_facts(spawn_index)).unwrap();
            assert_eq!(admission.sub_d_stagger_seed(), expected_seed);

            let pending = admission.pending_initial_selection();
            let cache = pending.sub_d_frame_owner.classifier_cache();
            assert_eq!(cache.stagger_counter(), expected_seed, "ordinal {ordinal}");
            assert_eq!(cache.rows(), [0; 8]);
            assert_eq!(cache.origin(), RetailRuntimeValue::Unresolved);
            assert_eq!(pending.sub_d_runtime, Type9SubDRuntime::from_constructor());
            assert_eq!(
                pending.immutable_anchor_raw_at_0x90(),
                RetailRuntimeValue::Known(LIVE_TEST_ANCHOR_RAW)
            );
            assert!(
                cache.can_classify(),
                "fresh Level-1 first use must perform the captured reset"
            );
        }

        let mut unresolved_anchor = exact_facts(9);
        unresolved_anchor.immutable_anchor_raw_at_0x90 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            admit_fresh_level1_ordinary_type9(unresolved_anchor)
                .unwrap()
                .pending_initial_selection()
                .immutable_anchor_raw_at_0x90(),
            RetailRuntimeValue::Unresolved,
            "admission retains a missing construction-terrain boundary instead of inventing Y"
        );
    }

    #[test]
    fn admission_rejects_every_unproven_boundary_dimension() {
        let exact = exact_facts(9);
        let rejected = [
            FreshLevel1OrdinaryType9SpawnFacts {
                retail_first_world: false,
                ..exact
            },
            FreshLevel1OrdinaryType9SpawnFacts {
                authored_spawn_index: 8,
                ..exact
            },
            FreshLevel1OrdinaryType9SpawnFacts {
                entity_type: 17,
                ..exact
            },
            FreshLevel1OrdinaryType9SpawnFacts {
                active_model_slot: RetailRuntimeValue::Known(1),
                ..exact
            },
            FreshLevel1OrdinaryType9SpawnFacts {
                active_model_slot: RetailRuntimeValue::Unresolved,
                ..exact
            },
            FreshLevel1OrdinaryType9SpawnFacts {
                active_model: Some(FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID + 1),
                ..exact
            },
            FreshLevel1OrdinaryType9SpawnFacts {
                rotation: [1, 0, 0],
                ..exact
            },
        ];

        for facts in rejected {
            assert_eq!(admit_fresh_level1_ordinary_type9(facts), None);
        }
    }

    #[test]
    fn live_owner_authenticates_exact_topology_and_construction_receipt() {
        let entity = admitted_entity(41);
        let owner =
            OrdinaryType9LiveOwner::from_entity_metadata(&entity, &exact_metadata()).unwrap();
        assert_eq!(owner.entity_id(), 41);

        let mut unresolved = exact_metadata();
        unresolved.common_mover_topology = RetailRuntimeValue::Unresolved;
        assert_eq!(
            OrdinaryType9LiveOwner::from_entity_metadata(&entity, &unresolved),
            Err(OrdinaryType9LiveOwnerError::Topology(
                OrdinaryType9TopologyError::UnresolvedComponentTopology,
            ))
        );

        let mut wrong_model = exact_metadata();
        wrong_model.model_slots[0] = FRESH_LEVEL1_ORDINARY_TYPE9_MODEL_ID.wrapping_add(1) as u16;
        assert_eq!(
            OrdinaryType9LiveOwner::from_entity_metadata(&entity, &wrong_model),
            Err(OrdinaryType9LiveOwnerError::EntityNotAdmitted)
        );
    }

    #[test]
    fn binding_requires_the_exact_surviving_visit_and_complete_runtime() {
        let metadata = exact_metadata();
        let (mut entity, visit) = entity_with_runtime(42);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let expected = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();

        let wrong_visit = ActorTaskVisit {
            slot: ActorTaskSlot::Secondary,
            ..visit
        };
        match owner.bind_entity_runtime(&mut entity, wrong_visit) {
            Err(error) => assert_eq!(
                error,
                OrdinaryType9RuntimeBindError::TaskLeaseUnavailable { visit: wrong_visit }
            ),
            Ok(_) => panic!("a task in another slot must not satisfy the lease"),
        }

        let rebound = owner.bind_entity_runtime(&mut entity, visit).unwrap();
        assert_eq!(rebound.current_state().unwrap(), expected);
    }

    #[test]
    fn accepted_action_commits_complete_state_once_and_replay_fails_closed() {
        let metadata = exact_metadata();
        let (mut entity, visit) = entity_with_runtime(43);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let retained_anchor = entity
            .ordinary_type9_pending_initial_selection
            .unwrap()
            .immutable_anchor_raw_at_0x90();
        let retained_basis = entity.physical_body_basis_q31;
        let bound = owner.bind_entity_runtime(&mut entity, visit).unwrap();
        let before = bound.current_state().unwrap();
        let after = changed_after(before);

        assert_eq!(
            bound.commit_action(action(lease(43, visit), before, after)),
            OrdinaryType9Resume::Acknowledged { committed: after }
        );
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .current_state()
                .unwrap(),
            after
        );
        assert_eq!(
            entity
                .ordinary_type9_pending_initial_selection
                .unwrap()
                .immutable_anchor_raw_at_0x90(),
            retained_anchor,
            "a mover receipt never owns the constructor anchor"
        );
        assert_eq!(
            entity.physical_body_basis_q31, retained_basis,
            "D/I/A/B commit must not perform the later E870 basis rebuild"
        );

        let replay = owner.bind_entity_runtime(&mut entity, visit).unwrap();
        assert_eq!(
            replay.commit_action(action(lease(43, visit), before, after)),
            OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
            }
        );
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .current_state()
                .unwrap(),
            after
        );
    }

    #[test]
    fn stale_before_or_mismatched_lease_never_partially_mutates() {
        let metadata = exact_metadata();
        let (mut entity, visit) = entity_with_runtime(44);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let bound = owner.bind_entity_runtime(&mut entity, visit).unwrap();
        let initial = bound.current_state().unwrap();
        let after = changed_after(initial);
        let mut stale = initial;
        stale.velocity_raw[1] = stale.velocity_raw[1].wrapping_add(1);
        assert_eq!(
            bound.commit_action(action(lease(44, visit), stale, after)),
            OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::StateCommitUnavailable,
            }
        );
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .current_state()
                .unwrap(),
            initial
        );

        let wrong_lease = OrdinaryType9FrameLease {
            controlled_entity_id: 44,
            task_visit: ActorTaskVisit {
                slot: ActorTaskSlot::Secondary,
                ..visit
            },
        };
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .commit_action(action(wrong_lease, initial, after)),
            OrdinaryType9Resume::Blocked {
                reason: OrdinaryType9ExternalBlock::LeaseUnavailable,
            }
        );
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .current_state()
                .unwrap(),
            initial
        );
    }

    #[test]
    fn retained_fresh_entity_stays_inert_without_a_published_task() {
        let metadata = exact_metadata();
        let mut entity = admitted_entity(45);
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(
            ActorAnimationController::from_descriptor(SUB_I).unwrap(),
        ));
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(SUB_A)));
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let (_, visit) = entity_with_runtime(99);
        match owner.bind_entity_runtime(&mut entity, visit) {
            Err(error) => assert_eq!(
                error,
                OrdinaryType9RuntimeBindError::TaskLeaseUnavailable { visit }
            ),
            Ok(_) => panic!("an unpublished task must keep the retained entity inert"),
        }
    }

    #[test]
    fn exact_wander_task_stays_inert_while_sub_a_speed_is_unresolved() {
        let metadata = exact_metadata();
        let (mut unresolved_speed, speed_visit) = entity_with_runtime(47);
        unresolved_speed.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(SUB_A)));
        let speed_owner =
            OrdinaryType9LiveOwner::from_entity_metadata(&unresolved_speed, &metadata).unwrap();
        match speed_owner.bind_entity_runtime(&mut unresolved_speed, speed_visit) {
            Err(error) => assert_eq!(
                error,
                OrdinaryType9RuntimeBindError::SubATargetSpeedUnresolved
            ),
            Ok(_) => panic!("the live seam must not invent constructor RNG state"),
        }
    }

    #[test]
    fn published_mode_zero_wander_ticks_scheduler_and_commits_one_receipt() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(48);
        set_wander_target(&mut entity, visit, [3_000, 20, -300]);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let before = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let retained_basis = entity.physical_body_basis_q31;
        let initial_seed = before
            .frame
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter();
        let mut draws = 0;

        let outcome = owner
            .tick_published_wander(&mut entity, live_frame_request(visit, &terrain), || {
                draws += 1;
                1
            })
            .unwrap();

        let OrdinaryType9LiveFrameOutcome::Continue {
            visit: outcome_visit,
            committed_prefix,
            mover_return,
        } = outcome
        else {
            panic!("an active nonzero mover must continue")
        };
        assert_eq!(outcome_visit, visit);
        assert_eq!(
            committed_prefix,
            OrdinaryType9WanderCallbackPrefix {
                lifetime_status: WanderNearLifetimeStatus::Active,
                retarget: WanderNearRetarget::Retained,
            }
        );
        assert_eq!(mover_return, WanderNearCommonMoverReturn::NonZero);
        assert_eq!(draws, 1, "the retained target consumes only its gate draw");
        let after = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        assert_ne!(after, before);
        assert_ne!(
            after.frame.actor_animation, before.frame.actor_animation,
            "normal mode must execute Sub-I"
        );
        assert_eq!(
            after
                .frame
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            initial_seed.wrapping_add(1)
        );
        assert_eq!(entity.heading_raw(), after.heading_raw);
        assert_eq!(entity.velocity_raw(), after.velocity_raw);
        assert_eq!(entity.physical_body_basis_q31, retained_basis);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("published Wander task must survive one active frame")
        };
        assert_eq!(state.elapsed_ms(), 20);
        assert_eq!(state.private_state(), after.wander);
    }

    #[test]
    fn published_mode_one_wander_commits_d_a_b_without_touching_animation() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(53);
        set_wander_target(&mut entity, visit, [3_000, 20, -300]);
        let mut mismatched_animation =
            ActorAnimationController::from_descriptor(ActorAnimationDescriptor {
                frames_per_direction: 3,
                ..SUB_I
            })
            .unwrap();
        mismatched_animation.apply_exploding_person_reset();
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(mismatched_animation));
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let before = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let retained_basis = entity.physical_body_basis_q31;
        let initial_stagger = before
            .frame
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter();
        let mut frame = live_frame_request(visit, &terrain);
        frame.scheduler_mode = 1;
        let mut draws = 0;

        let outcome = owner
            .tick_published_wander(&mut entity, frame, || {
                draws += 1;
                1
            })
            .unwrap();

        assert!(matches!(
            outcome,
            OrdinaryType9LiveFrameOutcome::Continue {
                visit: outcome_visit,
                committed_prefix: OrdinaryType9WanderCallbackPrefix {
                    lifetime_status: WanderNearLifetimeStatus::Active,
                    retarget: WanderNearRetarget::Retained,
                },
                mover_return: WanderNearCommonMoverReturn::NonZero,
            } if outcome_visit == visit
        ));
        assert_eq!(draws, 1, "mode one adds no mover RNG draw");
        let after = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        assert_eq!(after.frame.actor_animation, before.frame.actor_animation);
        assert_eq!(
            after
                .frame
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            initial_stagger.wrapping_add(1),
            "mode one must still execute Sub-D"
        );
        assert_ne!(after.velocity_raw, before.velocity_raw);
        assert_eq!(entity.physical_body_basis_q31, retained_basis);
        let RetailRuntimeValue::Known(Some(live_animation)) = entity.actor_animation_runtime else {
            panic!("published Type-9 runtime must retain its animation controller")
        };
        assert_eq!(live_animation, mismatched_animation);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("published Wander task must survive one active mode-one frame")
        };
        assert_eq!(state.elapsed_ms(), 20);
        assert_eq!(state.private_state(), after.wander);
    }

    #[test]
    fn selected_common_mover_binding_commits_one_staged_d_i_a_b_receipt() {
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(54);
        set_wander_target(&mut entity, visit, [3_000, 20, -300]);
        let components = entity
            .ordinary_type9_pending_initial_selection
            .take()
            .expect("fixture retains admitted Type-9 component state");
        let mut selected = OrdinaryType9SelectedComponentRuntime::new(
            components,
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(task_state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("fixture retains the staged private-state shape")
        };
        let mut staged_wander = task_state.private_state();
        let RetailRuntimeValue::Known(Some(mut sub_a_runtime)) = entity.sub_a_propulsion_runtime
        else {
            panic!("fixture retains Sub-A")
        };
        let RetailRuntimeValue::Known(Some(mut actor_animation)) = entity.actor_animation_runtime
        else {
            panic!("fixture retains Sub-I")
        };
        let mut heading_raw = entity.heading_raw();
        let mut velocity_raw = entity.velocity_raw();
        let before = (
            selected,
            staged_wander,
            sub_a_runtime,
            actor_animation,
            heading_raw,
            velocity_raw,
        );
        let initial_stagger = selected
            .components()
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter();

        let result = run_selected_ordinary_type9_common_mover(
            OrdinaryType9SelectedCommonMoverRequest {
                transaction_id: OrdinaryType9TransactionId::new(NonZeroU64::new(54).unwrap()),
                lease: OrdinaryType9FrameLease {
                    controlled_entity_id: entity.id,
                    task_visit: visit,
                },
                topology: OrdinaryType9Topology::from_metadata(
                    ORDINARY_TYPE9_ENTITY_TYPE as u16,
                    &exact_metadata(),
                )
                .unwrap(),
                component_custody: OrdinaryType9SelectedCommonMoverComponentCustody::from_selected(
                    &mut selected,
                ),
                staged_wander: &mut staged_wander,
                sub_a_runtime: &mut sub_a_runtime,
                actor_animation: &mut actor_animation,
                heading_raw: &mut heading_raw,
                velocity_raw: &mut velocity_raw,
                tracked_target: RetailRuntimeValue::Unresolved,
                terrain: &terrain,
                position_raw: entity.position_raw(),
                pre_mover_basis: live_test_basis(),
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                scheduler_mode: 0,
            },
            || 1,
        );

        assert_eq!(result, Ok(WanderNearCommonMoverReturn::NonZero));
        assert_eq!(
            selected.kind(),
            OrdinaryType9SelectedRuntimeKind::RunAwayAcquiringPublished,
            "the common mover borrows component state without relabelling behavior custody"
        );
        assert_eq!(
            selected
                .components()
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            initial_stagger.wrapping_add(1)
        );
        assert_ne!(
            (
                selected,
                staged_wander,
                sub_a_runtime,
                actor_animation,
                heading_raw,
                velocity_raw,
            ),
            before,
            "one acknowledged receipt must publish the complete staged mover state"
        );
    }

    #[test]
    fn retained_non_euler_basis_drives_a_b_identically_in_modes_zero_and_one() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let retained_basis = live_test_basis();
        let mut headings = [0; 2];
        let mut velocities = [[0; 3]; 2];

        for (ordinal, scheduler_mode) in [0, 1].into_iter().enumerate() {
            let (mut entity, visit) = entity_with_runtime(60 + ordinal as u32);
            set_wander_target(&mut entity, visit, [3_000, 20, -300]);
            let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
            assert_ne!(
                retained_basis,
                Type9BodyBasis::from_angle_words(heading_raw, pitch_raw, roll_raw),
                "the fixture matrix must expose accidental Euler reconstruction"
            );
            assert_eq!(
                entity.physical_body_basis_q31,
                RetailRuntimeValue::Known(retained_basis)
            );
            let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
            let mut request = live_frame_request(visit, &terrain);
            request.scheduler_mode = scheduler_mode;

            owner
                .tick_published_wander(&mut entity, request, || 1)
                .unwrap();

            headings[ordinal] = entity.heading_raw();
            velocities[ordinal] = entity.velocity_raw();
            assert_eq!(
                entity.physical_body_basis_q31,
                RetailRuntimeValue::Known(retained_basis),
                "the later E870 suffix, not D/I/A/B, owns the matrix rebuild"
            );
        }

        assert_eq!(
            (headings[0], velocities[0]),
            (headings[1], velocities[1]),
            "suppressing Sub-I must not change the shared retained basis used by D/A/B"
        );

        let (mut reconstructed, visit) = entity_with_runtime(62);
        set_wander_target(&mut reconstructed, visit, [3_000, 20, -300]);
        let [heading_raw, pitch_raw, roll_raw] = reconstructed.rotation_heading_pitch_roll_raw();
        let reconstructed_basis =
            Type9BodyBasis::from_angle_words(heading_raw, pitch_raw, roll_raw);
        reconstructed.physical_body_basis_q31 = RetailRuntimeValue::Known(reconstructed_basis);
        let owner =
            OrdinaryType9LiveOwner::from_entity_metadata(&reconstructed, &metadata).unwrap();
        let mut request = live_frame_request(visit, &terrain);
        request.scheduler_mode = 1;
        owner
            .tick_published_wander(&mut reconstructed, request, || 1)
            .unwrap();

        assert_ne!(
            reconstructed.velocity_raw(),
            velocities[1],
            "Sub-A/B output must follow the retained matrix, not the same entity angle words"
        );
    }

    #[test]
    fn stale_visit_is_rejected_before_scheduler_rng_or_state_mutation() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(49);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let state = *entity.actor_tasks.task_state(visit.task_id).unwrap();
        let replacement_id = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            crate::actor_task_owner::PreparedActorTask::new(state),
        );
        let replacement_visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: replacement_id,
        };
        let before = owner
            .bind_entity_runtime(&mut entity, replacement_visit)
            .unwrap()
            .current_state()
            .unwrap();
        let before_task = entity.actor_tasks.task_state(replacement_id).copied();
        let mut draws = 0;

        assert_eq!(
            owner.tick_published_wander(&mut entity, live_frame_request(visit, &terrain), || {
                draws += 1;
                1
            },),
            Err(OrdinaryType9LiveFrameError::RuntimeBind(
                OrdinaryType9RuntimeBindError::TaskLeaseUnavailable { visit }
            ))
        );
        assert_eq!(draws, 0);
        assert_eq!(
            entity.actor_tasks.task_state(replacement_id),
            before_task.as_ref()
        );
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, replacement_visit)
                .unwrap()
                .current_state()
                .unwrap(),
            before
        );
    }

    #[test]
    fn additional_published_task_is_rejected_before_scheduler_rng_or_state_mutation() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(50);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let duplicate = *entity.actor_tasks.task_state(visit.task_id).unwrap();
        let secondary_id = entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Secondary,
            crate::actor_task_owner::PreparedActorTask::new(duplicate),
        );
        let before = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let before_primary = entity.actor_tasks.task_state(visit.task_id).copied();
        let before_secondary = entity.actor_tasks.task_state(secondary_id).copied();
        let mut draws = 0;

        assert_eq!(
            owner.tick_published_wander(&mut entity, live_frame_request(visit, &terrain), || {
                draws += 1;
                1
            },),
            Err(OrdinaryType9LiveFrameError::AdditionalPublishedTask {
                slot: ActorTaskSlot::Secondary,
            })
        );
        assert_eq!(draws, 0);
        assert_eq!(
            entity.actor_tasks.task_state(visit.task_id),
            before_primary.as_ref()
        );
        assert_eq!(
            entity.actor_tasks.task_state(secondary_id),
            before_secondary.as_ref()
        );
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .current_state()
                .unwrap(),
            before
        );
    }

    #[test]
    fn unresolved_immutable_anchor_blocks_before_scheduler_rng_or_mutation() {
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(63);
        entity
            .ordinary_type9_pending_initial_selection
            .as_mut()
            .unwrap()
            .immutable_anchor_raw_at_0x90 = RetailRuntimeValue::Unresolved;

        assert_live_preflight_block_is_atomic(
            entity,
            visit,
            &terrain,
            OrdinaryType9RuntimeBindError::ImmutableAnchorUnavailable,
        );
    }

    #[test]
    fn unresolved_physical_basis_blocks_before_scheduler_rng_or_mutation() {
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(64);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;

        assert_live_preflight_block_is_atomic(
            entity,
            visit,
            &terrain,
            OrdinaryType9RuntimeBindError::PhysicalBodyBasisUnavailable,
        );
    }

    #[test]
    fn mover_evidence_block_keeps_scheduler_prefix_and_component_state() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(51);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            panic!("fixture must retain the published Wander task")
        };
        let mut private = state.private_state();
        private.tracked_entity_handle = 0x04AB_0001;
        state.replace_private_state(private);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let before = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let mut draws = 0;

        assert_eq!(
            owner.tick_published_wander(
                &mut entity,
                live_frame_request(visit, &terrain),
                || {
                    draws += 1;
                    1
                },
            ),
            Err(OrdinaryType9LiveFrameError::FrameBlocked {
                visit,
                committed_prefix: OrdinaryType9WanderCallbackPrefix {
                    lifetime_status: WanderNearLifetimeStatus::Active,
                    retarget: WanderNearRetarget::Retained,
                },
                reason: OrdinaryType9FrameBlock::TargetPrelude(
                    crate::common_mover::target_prelude::CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
                        handle: 0x04AB_0001,
                    },
                ),
            })
        );
        assert_eq!(draws, 1, "the scheduler retarget prefix remains committed");
        let after = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        assert_eq!(
            after, before,
            "a retained retarget changes no mover-owned word"
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("blocked mover must retain its task")
        };
        assert_eq!(
            state.elapsed_ms(),
            20,
            "lifetime commits before mover entry"
        );
    }

    #[test]
    fn replaced_retarget_and_three_draws_are_reported_when_mover_blocks() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(52);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            panic!("fixture must retain the published Wander task")
        };
        let mut private = state.private_state();
        private.target_position_raw = [3_000, 20, 4_000];
        private.tracked_entity_handle = 0x04AB_0001;
        state.replace_private_state(private);
        let retained_anchor = [0x1234, -777, -0x2345];
        entity
            .ordinary_type9_pending_initial_selection
            .as_mut()
            .unwrap()
            .immutable_anchor_raw_at_0x90 = RetailRuntimeValue::Known(retained_anchor);
        let moved_current_position = [-5_000, 999, 6_000];
        let velocity_raw = entity.velocity_raw();
        entity.set_motion_raw(moved_current_position, velocity_raw);
        assert_ne!(retained_anchor, entity.position_raw());
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let before = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let expected_target = retained_anchor;
        let words = [0, 0x8000, 0x8000];
        let mut draws = 0usize;

        assert_eq!(
            owner.tick_published_wander(
                &mut entity,
                live_frame_request(visit, &terrain),
                || {
                    let word = words[draws];
                    draws += 1;
                    word
                },
            ),
            Err(OrdinaryType9LiveFrameError::FrameBlocked {
                visit,
                committed_prefix: OrdinaryType9WanderCallbackPrefix {
                    lifetime_status: WanderNearLifetimeStatus::Active,
                    retarget: WanderNearRetarget::Replaced {
                        target_position_raw: expected_target,
                    },
                },
                reason: OrdinaryType9FrameBlock::TargetPrelude(
                    crate::common_mover::target_prelude::CommonMoverTargetPreludeBlock::UnresolvedTrackedTarget {
                        handle: 0x04AB_0001,
                    },
                ),
            })
        );
        assert_eq!(draws, 3);
        let after = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let mut expected = before;
        expected.wander.target_position_raw = expected_target;
        assert_eq!(after, expected, "only the retarget prefix may commit");
        assert_eq!(
            entity.position_raw(),
            moved_current_position,
            "retarget reads immutable +0x90 without replacing current +0x96"
        );
        assert_eq!(
            entity
                .ordinary_type9_pending_initial_selection
                .unwrap()
                .immutable_anchor_raw_at_0x90(),
            RetailRuntimeValue::Known(retained_anchor)
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("blocked mover must retain its task")
        };
        assert_eq!(state.elapsed_ms(), 20);
        assert_eq!(
            entity.actor_tasks.wrapper_flags(visit.task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn common_mover_zero_reports_transition_after_callback_unwind() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(53);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            panic!("fixture must retain the published Wander task")
        };
        let mut private = state.private_state();
        private.tracked_entity_handle = 0x04AB_0001;
        state.replace_private_state(private);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let before = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let mut request = live_frame_request(visit, &terrain);
        request.tracked_target = RetailRuntimeValue::Known(Some(
            crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw: [500, 20, 600],
                velocity_raw: [1, 0, -1],
            },
        ));
        let mut draws = 0;

        let OrdinaryType9LiveFrameOutcome::TransitionPending {
            request: transition,
            mover_return,
        } = owner
            .tick_published_wander(&mut entity, request, || {
                draws += 1;
                1
            })
            .unwrap()
        else {
            panic!("retail common-mover zero must request the primary transition")
        };
        assert_eq!(draws, 1);
        assert_eq!(mover_return, WanderNearCommonMoverReturn::Zero);
        assert_eq!(transition.slot, ActorTaskSlot::Primary);
        assert_eq!(transition.task_id, visit.task_id);
        assert_eq!(
            transition.committed_prefix,
            OrdinaryType9WanderCallbackPrefix {
                lifetime_status: WanderNearLifetimeStatus::Active,
                retarget: WanderNearRetarget::Retained,
            }
        );
        assert!(matches!(
            transition.reason,
            crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionReason::CommonMoverCompleted(_)
        ));
        assert_eq!(
            owner
                .bind_entity_runtime(&mut entity, visit)
                .unwrap()
                .current_state()
                .unwrap(),
            before,
            "retail zero commits no mover-owned state"
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("the pending transition must retain its task")
        };
        assert_eq!(state.elapsed_ms(), 20);
        assert_eq!(
            entity.actor_tasks.wrapper_flags(visit.task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn strict_lifetime_boundary_transitions_only_after_five_thousand_ms() {
        let metadata = exact_metadata();
        let terrain = flat_terrain();
        let (mut entity, visit) = entity_with_runtime(54);
        set_wander_target(&mut entity, visit, [3_000, 20, -300]);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        else {
            panic!("fixture must retain the published Wander task")
        };
        assert_eq!(
            state.before_callback(5_000_000),
            WanderNearLifetimeStatus::Active
        );
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&entity, &metadata).unwrap();
        let mut exact_request = live_frame_request(visit, &terrain);
        exact_request.elapsed_micros = 0;
        exact_request.global_elapsed_micros = 0;
        let exact = owner
            .tick_published_wander(&mut entity, exact_request, || 1)
            .unwrap();
        let OrdinaryType9LiveFrameOutcome::Continue {
            committed_prefix,
            mover_return,
            ..
        } = exact
        else {
            panic!("the exact lifetime boundary must continue")
        };
        assert_eq!(
            committed_prefix.lifetime_status,
            WanderNearLifetimeStatus::Active
        );
        assert_eq!(mover_return, WanderNearCommonMoverReturn::NonZero);
        let Some(ActorTaskRuntime::OrdinaryType9Wander(exact_state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("the exact-boundary task must remain published")
        };
        assert_eq!(exact_state.elapsed_ms(), 5_000);
        let before_expiry = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        let before_seed = before_expiry
            .frame
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter();

        let mut expired_request = live_frame_request(visit, &terrain);
        expired_request.transaction_id =
            OrdinaryType9TransactionId::new(NonZeroU64::new(2).unwrap());
        expired_request.elapsed_micros = 1_000;
        expired_request.global_elapsed_micros = 1_000;
        let OrdinaryType9LiveFrameOutcome::TransitionPending {
            request: transition,
            mover_return,
        } = owner
            .tick_published_wander(&mut entity, expired_request, || 1)
            .unwrap()
        else {
            panic!("strictly expired Wander must request the primary transition")
        };
        assert_eq!(mover_return, WanderNearCommonMoverReturn::NonZero);
        assert_eq!(
            transition.committed_prefix,
            OrdinaryType9WanderCallbackPrefix {
                lifetime_status: WanderNearLifetimeStatus::OwnerTransitionDue,
                retarget: WanderNearRetarget::Retained,
            }
        );
        assert!(matches!(
            transition.reason,
            crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionReason::LifetimeExpired
        ));
        let after_expiry = owner
            .bind_entity_runtime(&mut entity, visit)
            .unwrap()
            .current_state()
            .unwrap();
        assert_eq!(
            after_expiry
                .frame
                .sub_d_frame_owner
                .classifier_cache()
                .stagger_counter(),
            before_seed.wrapping_add(1),
            "the mover commit precedes the unresolved owner transition"
        );
        let Some(ActorTaskRuntime::OrdinaryType9Wander(expired_state)) =
            entity.actor_tasks.task_state(visit.task_id)
        else {
            panic!("the unresolved transition must retain its task")
        };
        assert_eq!(expired_state.elapsed_ms(), 5_001);
        assert_eq!(
            entity.actor_tasks.wrapper_flags(visit.task_id),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        );
    }

    #[test]
    fn live_owner_cannot_bind_another_entity_runtime() {
        let metadata = exact_metadata();
        let source = admitted_entity(45);
        let owner = OrdinaryType9LiveOwner::from_entity_metadata(&source, &metadata).unwrap();
        let (mut wrong_entity, wrong_visit) = entity_with_runtime(46);
        match owner.bind_entity_runtime(&mut wrong_entity, wrong_visit) {
            Err(error) => assert_eq!(
                error,
                OrdinaryType9RuntimeBindError::OwnerRuntimeMismatch {
                    planned_owner_id: 45,
                    runtime_owner_id: 46,
                }
            ),
            Ok(_) => panic!("an owner must not bind another entity's runtime"),
        }
    }
}
