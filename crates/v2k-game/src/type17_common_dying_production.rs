//! Bounded production scheduler for captured Level-1 Type-17 Common-Dying.
//!
//! Retail's detailed actor callback ticks an already-published class-12 task
//! before the later same-manager-tick deferred sweep.  This adapter retains
//! only authenticated publication receipts and visits them in current retail
//! live-list order. A set detailed-update bit closes the statically proven
//! callback prefix: terrain attitude, normal null-target common mover
//! H -> C -> A -> B, and class-12 X/Z damping/timeout ownership.  A clear bit
//! selects retail's distinct coarse owner. Class-12 style word `0x2015` is its
//! `+0x38` clear mask, not effective flags; Type 17 therefore still dispatches
//! the task in mode 1 before E870's post-task suffix. The bounded adapter now
//! executes that exact tagged transition plus basis/environment/surface suffix
//! whenever the transition gate and E370 branch are fully preflightable.
//!
//! The Type-17 Sub-C selector is authored zero, so both the direct attitude
//! probes and Sub-C use terrain.  Its Sub-H runtime is disabled by the
//! class-12 constructor; updating it only clears transient record flags and
//! cannot consume WorldFx RNG or emit completion cues. Detailed frames now
//! continue through the statically matched DCA0 suffix and the enclosing
//! master-motion gates. Coarse mode either publishes its same-tick deferred
//! unlink or retains the relation-suppressed task before those same gates.
//! The entry `0x1000` relation prelude and the callback-gated solo Sub-J tail
//! are bound too: parent order is authoritative, stale rows compact stably,
//! and valid children consume the rebuilt post-callback pose before outer
//! master motion. Natural coarse visits now retain
//! the exact common-scheduler waits and branch-local draw order on the
//! process-shared RNG. Deep-water class-42 emission is now exact on that same
//! stream. The matched retail/demo lifecycle helper is bound too: exact expiry
//! releases the relation, refreshes the type-default state copy, enters the
//! divisor-two bubble gate, and a normal overshoot returns without RNG.

use std::cell::Cell;

use v2k_formats::terrain::TerrainGrid;

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::ActorTaskSlot;
use crate::common_dying::{
    damp_common_dying_axis, select_common_dying_effect_raw, CommonDyingCallbackError,
    CommonDyingEffectInputs, COMMON_DYING_EFFECT_ENABLED, COMMON_DYING_EFFECT_KIND,
    COMMON_DYING_TASK_LIFETIME_MS,
};
use crate::common_dying_live::{
    LevelOneType17CommonDyingOwner, Type17CommonDyingCallbackEvidence,
    Type17CommonDyingLiveFrameCommit, Type17CommonDyingLiveFrameError,
    Type17CommonDyingLiveFrameOutcome, Type17CommonDyingLiveFrameRequest,
    Type17CommonDyingLiveOwnerError, TYPE17_COMMON_DYING_ENTITY_TYPE,
};
use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameAdvanceError, CommonMoverFrameBlock,
    CommonMoverFrameCommitPhase, CommonMoverFrameConfiguration, CommonMoverFrameMachine,
    CommonMoverFramePoll, CommonMoverFrameRequest, CommonMoverFrameResume,
    CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_c::SubCSurfaceSample;
use crate::common_mover::target_prelude::CommonMoverTargetPreludeZero;
use crate::common_mover::type9_attitude::{
    plan_common_dying_terrain_attitude_raw, CommonDyingTerrainAttitudeInput, Type9BodyBasis,
    TERRAIN_ATTITUDE_BILINEAR_STATE_BIT,
};
use crate::common_mover::type9_surface::{
    classify_actor_surface_timer_phase, plan_actor_surface_bubble, ActorSurfaceBubbleFrame,
    ActorSurfaceBubbleRequest, ActorSurfaceTimerFrame, ActorSurfaceTimerPhase,
    ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
};
use crate::common_mover::type9_tail::{
    plan_common_master_motion, PlannedCommonMasterMotion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity::{
    apply_first_world_type17_environment_raw, commit_common_master_motion, Entity, EntityManager,
    Type17MaterialiserAttempt, Type17MissingRelationMaterialiserBlock,
    Type17MissingRelationMaterialiserPlan, LEVEL_ONE_TYPE17_SELF_MASS_RAW,
};
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityCollisionRuntimeState, RetailRuntimeValue, RetailStateWord,
    ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT,
    REMOTE_OWNED_STATE_BIT,
};
use crate::entity_relation_release::relation_release_state_word_after;
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    PlannedCommonSchedulerPrefix, COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
};
use crate::entity_view_detail::RetailViewDetailContext;
use crate::hover::{q31_mul, HoverBasis, HoverLiftConfig, RETAIL_FRAME_DELTA_MAX_US};
use crate::sub_h_external_frame::{SubHRuntimeState, SubHUpdateError};
use v2k_formats::collision::{
    SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    SubHExternalFrameDescriptor, SubJAttachmentDescriptor,
};

/// The exact optional-component shape of the captured Level-1 Type-17 actors.
pub const LEVEL_ONE_TYPE17_COMMON_MOVER_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: true,
        sub_b: true,
        sub_c: true,
        sub_d: true,
        sub_e: false,
        sub_f: false,
        sub_g: false,
        sub_h: true,
        sub_i: false,
        sub_j: true,
        sub_k: false,
        sub_l: false,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

/// Model 256 (`spider`) Section-8 header `+0x08` in the first-world pool.
pub const LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW: u16 = 315;
/// Type-17 Section-12 bytes `+0x72=1,+0x73=0,+0x74=2000`.
pub const LEVEL_ONE_TYPE17_SURFACE_LIFETIME_MS: u32 = 2_000;
/// Type-17 Section-12 `+0xC0`, copied to entity `+0xC8` by `FUN_0040D3C0`.
pub const LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW: u32 = 0x39;
/// One scheduler invocation at the detailed actor-callback boundary.
#[derive(Debug, Clone, Copy)]
pub struct Type17CommonDyingProductionFrame<'a> {
    pub terrain: &'a TerrainGrid,
    /// Capped gameplay step supplied by the outer loop. The common scheduler
    /// derives the callback step from this value and its retained carry fields.
    pub elapsed_micros: u32,
}

/// Static/live evidence that prevented an exact frame before task elapsed or
/// entity state was mutated.  The receipt remains registered for a later
/// retry unless its task/allocation lease itself proved stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17CommonDyingProductionBlock {
    MissingTypeMetadata,
    UnresolvedTopology,
    UnexpectedTopology {
        actual: CommonMoverComponentTopology,
    },
    UnresolvedSubADescriptor,
    MissingSubADescriptor,
    UnresolvedSubBDescriptor,
    MissingSubBDescriptor,
    UnresolvedSubCDescriptor,
    MissingSubCDescriptor,
    UnexpectedSubCSurfaceMode {
        actual: i8,
    },
    UnresolvedSubHDescriptor,
    MissingSubHDescriptor,
    UnresolvedSubARuntime,
    MissingSubARuntime,
    UnresolvedSubATargetSpeed,
    ZeroSubATargetSpeed,
    UnresolvedSubHRuntime,
    MissingSubHRuntime,
    SubHUnexpectedlyEnabled,
    UnresolvedSubJAttachmentRuntime,
    MissingSubJAttachmentRuntime,
    UnresolvedSubJAttachmentDescriptor,
    MissingSubJAttachmentDescriptor,
    UnexpectedType17SubJAttachmentDescriptor,
    UnexpectedSubJAttachmentShape {
        authored_slots: usize,
        runtime_slots: usize,
        live_rows: usize,
    },
    UnresolvedDetailedUpdateState,
    UnresolvedNormalSchedulerOwnerState,
    RemoteSchedulerOwnerUnimplemented,
    SchedulerCallbackDisabled,
    Type93Materialiser(Type17MissingRelationMaterialiserBlock),
    UnresolvedRelationOwnerState {
        relation_owner_id: u32,
    },
    UnresolvedRelationOwnerAttachmentRuntime {
        relation_owner_id: u32,
    },
    UnresolvedDirectAttachmentState {
        child_entity_id: u32,
    },
    DirectAttachmentSelfReference {
        child_entity_id: u32,
    },
    DetailedElapsedExceedsRetailCap {
        actual: u32,
    },
    CoarseElapsedExceedsRetailCap {
        actual: u32,
    },
    UnresolvedAnimationOffset,
    UnresolvedDetailedSchedulerState,
    UnresolvedCoarseSchedulerState,
    UnexpectedDetailedSchedulerRandomDraw,
    UnexpectedDetailedSchedulerWait,
    UnresolvedMasterMotionState,
    UnresolvedSurfaceOwnerState,
    UnresolvedSurfaceLifetimeTimer,
    MissingTypeInitializer,
    UnexpectedType17DefaultStateFlags {
        actual: u32,
    },
    UnexpectedSurfaceLifecycleModelState,
    UnexpectedType17Mass {
        actual: u16,
    },
    EffectSelection(CommonDyingCallbackError),
    SubHUpdate(SubHUpdateError),
    UnexpectedSubHCompletionCues {
        count: usize,
    },
    UnexpectedMoverRandomDraw,
    MoverStart(CommonMoverFrameBlock),
    MoverAdvance(CommonMoverFrameAdvanceError),
    MoverBlocked(CommonMoverFrameBlock),
    UnexpectedMoverAction(CommonMoverFrameAction),
    UnexpectedMoverReturnZero(CommonMoverTargetPreludeZero),
    MoverDidNotTerminate,
}

/// Why a registered receipt was discarded instead of retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17CommonDyingProductionDrop {
    EntityUnavailable,
    ReceiptMismatch,
    Owner(Type17CommonDyingLiveOwnerError),
}

/// Result for one registered owner, emitted in retail live-list order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17CommonDyingProductionOutcome {
    /// The common scheduler consumed its exact prefix and returned before the
    /// Type-17 callback. The authenticated receipt remains live for retry.
    SchedulerWaiting { entity_id: u32 },
    Advanced {
        entity_id: u32,
        live_outcome: Type17CommonDyingLiveFrameOutcome,
    },
    Blocked {
        entity_id: u32,
        reason: Type17CommonDyingProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Type17CommonDyingProductionDrop,
    },
    /// The low-level wrapper consumed whatever prefix is described by the
    /// error.  Retryability follows that error's retained-task semantics.
    LiveError {
        entity_id: u32,
        error: Type17CommonDyingLiveFrameError,
        retained: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type17CommonDyingProductionPass {
    pub outcomes: Vec<Type17CommonDyingProductionOutcome>,
    /// Class-42 allocator requests emitted by successful callbacks, in the
    /// same retail live-list order as `outcomes`.
    pub surface_bubbles: Vec<ActorSurfaceBubbleRequest>,
}

/// Result of ticking one authenticated receipt without choosing global actor
/// order for the caller.
///
/// The retained owner is the sole authority for a later frame. A missing
/// value means the task completed or its allocation/task lease proved stale.
/// The optional bubble belongs beside this outcome in the caller's global
/// live-list action order.
#[derive(Debug)]
pub(crate) struct Type17CommonDyingOwnerTick {
    pub(crate) outcome: Type17CommonDyingProductionOutcome,
    pub(crate) retained_owner: Option<LevelOneType17CommonDyingOwner>,
    pub(crate) surface_bubble: Option<ActorSurfaceBubbleRequest>,
}

/// Small specialized receipt scheduler; it does not activate the generic
/// actor dispatcher or infer ownership for any other task family.
#[derive(Debug, Default)]
pub struct Type17CommonDyingScheduler {
    owners: Vec<LevelOneType17CommonDyingOwner>,
}

impl Type17CommonDyingScheduler {
    pub const fn new() -> Self {
        Self { owners: Vec::new() }
    }

    /// Register or replace the sole current Common-Dying receipt for an
    /// entity.  Tick order is derived from the manager live list, never this
    /// publication vector.
    pub fn register(&mut self, owner: LevelOneType17CommonDyingOwner) {
        if let Some(existing) = self
            .owners
            .iter_mut()
            .find(|existing| existing.entity_id() == owner.entity_id())
        {
            *existing = owner;
        } else {
            self.owners.push(owner);
        }
    }

    pub fn clear(&mut self) {
        self.owners.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.owners.is_empty()
    }

    pub fn registered_len(&self) -> usize {
        self.owners.len()
    }

    /// Publish the presentation-phase `FUN_00411400` detail result for the
    /// authenticated receipts which survived this frame's scheduler/sweep.
    ///
    /// Retail classifies during the later presentation traversal, after
    /// `FUN_00413500` has already dispatched the current simulation tick. The
    /// resulting broader-detail bit therefore selects the next scheduler
    /// callback. This bounded seam deliberately does not claim the adjacent
    /// full-detail callback or attachment traversal.
    pub fn publish_presented_view_detail(
        &self,
        manager: &mut EntityManager,
        context: RetailViewDetailContext,
    ) {
        for owner in &self.owners {
            publish_type17_common_dying_owner_presented_view_detail(manager, *owner, context);
        }
    }

    /// Tick receipts that existed when this manager phase began.
    ///
    /// The production caller runs this before particle traversal, then runs
    /// `EntityManager::cleanup_pending_actor_deferred_destroys` at the later
    /// same-tick sweep boundary. Common-Dying publications performed
    /// by the subsequent particle pass are registered afterwards and thus
    /// first become eligible on the next scheduler invocation. A clear
    /// detailed-update bit selects the separately preflighted mode-1 task and
    /// coarse owner suffix. `next_shared_random` must draw from the caller's
    /// process-global retail stream. Unsupported coarse suffix states remain
    /// conservatively frozen before the scheduler determines a live wait.
    pub fn tick(
        &mut self,
        manager: &mut EntityManager,
        frame: Type17CommonDyingProductionFrame<'_>,
        next_shared_random: &mut impl FnMut() -> u32,
    ) -> Type17CommonDyingProductionPass {
        let live_order = manager.retail_live_order_ids().collect::<Vec<_>>();
        let mut pending = std::mem::take(&mut self.owners);
        let mut retained = Vec::with_capacity(pending.len());
        let mut outcomes = Vec::with_capacity(pending.len());
        let mut surface_bubbles = Vec::new();

        for entity_id in live_order {
            let Some(index) = pending
                .iter()
                .position(|owner| owner.entity_id() == entity_id)
            else {
                continue;
            };
            let owner = pending.remove(index);
            let tick = tick_type17_common_dying_owner(manager, owner, frame, next_shared_random);
            if let Some(owner) = tick.retained_owner {
                retained.push(owner);
            }
            if let Some(request) = tick.surface_bubble {
                surface_bubbles.push(request);
            }
            outcomes.push(tick.outcome);
        }

        // Receipts whose allocation is no longer in the manager live list are
        // stale.  They are reported after all live-list-ordered outcomes.
        outcomes.extend(pending.into_iter().map(|owner| {
            Type17CommonDyingProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type17CommonDyingProductionDrop::EntityUnavailable,
            }
        }));
        self.owners = retained;
        Type17CommonDyingProductionPass {
            outcomes,
            surface_bubbles,
        }
    }
}

/// Publish the later presentation traversal's view-detail result for one
/// surviving Type-17 receipt.
///
/// A heterogeneous scheduler can call this for only its Type-17 owners while
/// retaining the current next-simulation-tick policy. Missing or inactive
/// allocations remain the same silent presentation no-op as the public family
/// scheduler.
pub(crate) fn publish_type17_common_dying_owner_presented_view_detail(
    manager: &mut EntityManager,
    owner: LevelOneType17CommonDyingOwner,
    context: RetailViewDetailContext,
) {
    let Some(entity) = manager.common_actor_dying_entity_mut(owner.entity_id()) else {
        return;
    };
    if !entity.active {
        return;
    }
    let position_raw = entity.position_raw();
    let _ = context.publish(position_raw, &mut entity.collision.state_flags_at_0x08);
}

/// Tick one Type-17 Common-Dying owner after an outer scheduler has selected
/// its position in the current retail live-list pass.
///
/// This primitive deliberately does not inspect or advance any sibling owner.
/// It therefore composes into a heterogeneous actor scheduler without
/// changing this family's callback, RNG, retry, or deferred-destruction
/// semantics. Callers must invoke it at most once for the owner in one manager
/// pass; some error outcomes retain a prefix which must not be replayed.
pub(crate) fn tick_type17_common_dying_owner(
    manager: &mut EntityManager,
    owner: LevelOneType17CommonDyingOwner,
    frame: Type17CommonDyingProductionFrame<'_>,
    next_shared_random: &mut impl FnMut() -> u32,
) -> Type17CommonDyingOwnerTick {
    let entity_id = owner.entity_id();
    let prepared = preflight_production_frame(manager, owner, frame);
    let planned =
        prepared.map(|prepared| resolve_production_frame(prepared, manager, next_shared_random));
    match planned {
        Ok(plan) => match execute_preplanned_frame(owner, manager, frame.elapsed_micros, plan) {
            Ok(ExecutedProductionFrame::SchedulerWaiting) => Type17CommonDyingOwnerTick {
                outcome: Type17CommonDyingProductionOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
                surface_bubble: None,
            },
            Ok(ExecutedProductionFrame::Advanced {
                live_outcome,
                surface_bubble,
            }) => {
                let retained_owner = (!matches!(
                    live_outcome,
                    Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged { .. }
                ))
                .then_some(owner);
                Type17CommonDyingOwnerTick {
                    outcome: Type17CommonDyingProductionOutcome::Advanced {
                        entity_id,
                        live_outcome,
                    },
                    retained_owner,
                    surface_bubble,
                }
            }
            Err(error) => {
                let retained = live_error_retains_owner(error);
                Type17CommonDyingOwnerTick {
                    outcome: Type17CommonDyingProductionOutcome::LiveError {
                        entity_id,
                        error,
                        retained,
                    },
                    retained_owner: retained.then_some(owner),
                    surface_bubble: None,
                }
            }
        },
        Err(ProductionPlanFailure::Blocked(reason)) => Type17CommonDyingOwnerTick {
            outcome: Type17CommonDyingProductionOutcome::Blocked { entity_id, reason },
            retained_owner: Some(owner),
            surface_bubble: None,
        },
        Err(ProductionPlanFailure::Dropped(reason)) => Type17CommonDyingOwnerTick {
            outcome: Type17CommonDyingProductionOutcome::Dropped { entity_id, reason },
            retained_owner: None,
            surface_bubble: None,
        },
    }
}

fn live_error_retains_owner(error: Type17CommonDyingLiveFrameError) -> bool {
    matches!(
        error,
        Type17CommonDyingLiveFrameError::FrameBlocked { .. }
            | Type17CommonDyingLiveFrameError::TransitionGateUnresolved { .. }
    )
}

enum ProductionPlanFailure {
    Blocked(Type17CommonDyingProductionBlock),
    Dropped(Type17CommonDyingProductionDrop),
}

impl From<Type17CommonDyingProductionBlock> for ProductionPlanFailure {
    fn from(value: Type17CommonDyingProductionBlock) -> Self {
        Self::Blocked(value)
    }
}

#[derive(Debug)]
struct PreflightSnapshot {
    entity_id: u32,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    rotation_raw: [i16; 3],
    state_flags: u32,
    master_motion_state_flags: u32,
    terminal_transition_expected: bool,
    attached_mass: u32,
    topology: CommonMoverComponentTopology,
    sub_a_descriptor: SubAPropulsionDescriptor,
    sub_b_descriptor: SubBLateralDescriptor,
    sub_c_descriptor: SubCLiftDescriptor,
    sub_h_descriptor: SubHExternalFrameDescriptor,
    sub_a_runtime: SubAPropulsionRuntime,
    sub_h_runtime: SubHRuntimeState,
}

#[derive(Debug)]
enum PlannedProductionFrame {
    Detailed(PlannedDetailedProductionFrame),
    Coarse(PlannedCoarseProductionFrame),
    SchedulerWait(PlannedCommonSchedulerPrefix),
}

/// All fallible callback reads are closed before the committed scheduler,
/// construction stamp and selector/surface random consumers run.
#[derive(Debug)]
enum PreflightedProductionFrame {
    Detailed(PlannedDetailedProductionFrame<PreflightedType17PostTaskSuffix>),
    Coarse {
        collision: EntityCollisionRuntimeState,
        elapsed_micros: u32,
        callback: PlannedCoarseProductionFrame<PreflightedType17PostTaskSuffix>,
    },
}

fn resolve_production_frame(
    prepared: PreflightedProductionFrame,
    manager: &mut EntityManager,
    next_shared_random: &mut impl FnMut() -> u32,
) -> PlannedProductionFrame {
    match prepared {
        PreflightedProductionFrame::Detailed(prepared) => {
            let PlannedDetailedProductionFrame {
                scheduler_prefix,
                relation_prelude,
                callback_mass_raw,
                effect_inputs,
                selected_effect_raw,
                mover_velocity_raw,
                live_commit,
                post_task_suffix,
                direct_attachment_tail,
                master_motion,
                terminal_transition_expected,
            } = prepared;
            let relation_prelude =
                relation_prelude.begin_type93_constructor(manager, next_shared_random);
            let post_task_suffix =
                resolve_type17_post_task_suffix(post_task_suffix, next_shared_random);
            PlannedProductionFrame::Detailed(PlannedDetailedProductionFrame {
                scheduler_prefix,
                relation_prelude,
                callback_mass_raw,
                effect_inputs,
                selected_effect_raw,
                mover_velocity_raw,
                live_commit,
                post_task_suffix,
                direct_attachment_tail,
                master_motion,
                terminal_transition_expected,
            })
        }
        PreflightedProductionFrame::Coarse {
            collision,
            elapsed_micros,
            callback,
        } => {
            let RetailRuntimeValue::Known(scheduler_prefix) =
                plan_common_scheduler_prefix(&collision, elapsed_micros, next_shared_random)
            else {
                unreachable!("immutable coarse preflight closed every scheduler input")
            };
            let CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us,
            } = scheduler_prefix.flow
            else {
                return PlannedProductionFrame::SchedulerWait(scheduler_prefix);
            };
            let PlannedCoarseProductionFrame {
                scheduler_prefix: preview_prefix,
                relation_prelude,
                callback_mass_raw,
                post_task_suffix,
                direct_attachment_tail,
                master_motion,
                terminal_transition_expected,
            } = callback;
            debug_assert_eq!(
                preview_prefix.flow,
                CommonSchedulerPrefixFlow::Continue {
                    callback_elapsed_us
                }
            );
            // A coarse wait returned above; only an actual relation callback
            // enters104B0, before its selector and the later surface random.
            let relation_prelude =
                relation_prelude.begin_type93_constructor(manager, next_shared_random);
            let post_task_suffix =
                resolve_type17_post_task_suffix(post_task_suffix, next_shared_random);
            PlannedProductionFrame::Coarse(PlannedCoarseProductionFrame {
                scheduler_prefix,
                relation_prelude,
                callback_mass_raw,
                post_task_suffix,
                direct_attachment_tail,
                master_motion,
                terminal_transition_expected,
            })
        }
    }
}

/// The local-subject relation reconciliation from retail `FUN_00412DA0`'s
/// entry-state `0x1000` branch, including the retail/demo-matched Type-93
/// materialiser transaction for a missing local relation owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannedType17RelationPrelude {
    None,
    RetainLocal {
        relation_owner_id: u32,
    },
    ReleaseLocal {
        relation_owner_id: u32,
    },
    FollowRemote {
        relation_owner_id: u32,
        position_raw: [i16; 3],
    },
    MaterialiseMissing {
        relation_owner_id: Option<u32>,
        target_raw: [i16; 3],
        materialiser: Type17MissingRelationMaterialiserPlan,
        attempt: Option<Type17MaterialiserAttempt>,
    },
}

impl PlannedType17RelationPrelude {
    fn state_after(
        self,
        mut before: RetailStateWord,
        type_default_state_flags_at_0xc0: u32,
    ) -> RetailStateWord {
        match self {
            Self::ReleaseLocal { .. } => {
                relation_release_state_word_after(before, type_default_state_flags_at_0xc0)
            }
            Self::MaterialiseMissing { .. } => {
                before =
                    relation_release_state_word_after(before, type_default_state_flags_at_0xc0);
                // Type-93 slot policy zero clears 0x800, relation attach sets
                // 0x1000, and FUN_00408F00 finally clears master-motion 0x40000.
                before.overwrite(0x0004_1800, 0x0000_1000);
                before
            }
            Self::FollowRemote { .. } => {
                before.overwrite(0x0000_0020, 0x0000_0020);
                before
            }
            Self::None | Self::RetainLocal { .. } => before,
        }
    }

    fn position_after(self, before: [i16; 3]) -> [i16; 3] {
        match self {
            Self::FollowRemote { position_raw, .. } => position_raw,
            Self::None
            | Self::RetainLocal { .. }
            | Self::ReleaseLocal { .. }
            | Self::MaterialiseMissing { .. } => before,
        }
    }

    /// `FUN_00425680` draws unconditionally for Type 93's singleton positive
    /// Always -> Materialiser list. Its identity is already authenticated by
    /// preflight, but the process-shared stream must still advance here.
    fn begin_type93_constructor(
        mut self,
        manager: &mut EntityManager,
        next_shared_random: &mut impl FnMut() -> u32,
    ) -> Self {
        if let Self::MaterialiseMissing {
            materialiser,
            attempt,
            ..
        } = &mut self
        {
            assert!(
                attempt.is_none(),
                "a committed Type93 attempt cannot be replayed"
            );
            *attempt = Some(materialiser.begin_attempt(manager));
            let _ = next_shared_random();
        }
        self
    }
}

/// Child evidence resolved before scheduler RNG is consumed. Retail's solo
/// path does not inspect or repair `attached_to`; that branch is gated by the
/// multiplayer global `DAT_004F741C`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreflightedType17DirectAttachment {
    child_entity_id: u32,
    local_offset_raw: [i16; 3],
    apply_local_offset: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreflightedType17DirectAttachmentTail {
    retained_entity_ids: Vec<u32>,
    children: Vec<PreflightedType17DirectAttachment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannedType17DirectAttachmentUpdate {
    child_entity_id: u32,
    position_raw: [i16; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlannedType17DirectAttachmentTail {
    retained_entity_ids: Vec<u32>,
    updates: Vec<PlannedType17DirectAttachmentUpdate>,
}

#[derive(Debug, Clone)]
struct PreflightedType17SchedulerTail {
    relation_prelude: PlannedType17RelationPrelude,
    state_after_relation: RetailStateWord,
    position_after_relation_raw: [i16; 3],
    attached_mass: u32,
    direct_attachments: PreflightedType17DirectAttachmentTail,
}

#[derive(Debug)]
struct PlannedDetailedProductionFrame<S = PlannedType17PostTaskSuffix> {
    scheduler_prefix: PlannedCommonSchedulerPrefix,
    relation_prelude: PlannedType17RelationPrelude,
    callback_mass_raw: u16,
    effect_inputs: CommonDyingEffectInputs,
    selected_effect_raw: i32,
    mover_velocity_raw: [i16; 3],
    live_commit: Type17CommonDyingLiveFrameCommit,
    post_task_suffix: S,
    direct_attachment_tail: PlannedType17DirectAttachmentTail,
    master_motion: PlannedCommonMasterMotion,
    terminal_transition_expected: bool,
}

#[derive(Debug, Clone)]
struct PlannedCoarseProductionFrame<S = PlannedType17PostTaskSuffix> {
    scheduler_prefix: PlannedCommonSchedulerPrefix,
    relation_prelude: PlannedType17RelationPrelude,
    callback_mass_raw: u16,
    post_task_suffix: S,
    direct_attachment_tail: PlannedType17DirectAttachmentTail,
    master_motion: PlannedCommonMasterMotion,
    terminal_transition_expected: bool,
}

#[derive(Debug)]
enum ExecutedProductionFrame {
    SchedulerWaiting,
    Advanced {
        live_outcome: Type17CommonDyingLiveFrameOutcome,
        surface_bubble: Option<ActorSurfaceBubbleRequest>,
    },
}

#[derive(Debug, Clone, Copy)]
struct PlannedType17PostTaskSuffix {
    body_basis_q31: Type9BodyBasis,
    velocity_after_environment_raw: [i16; 3],
    surface_timer_after_ms: Option<u32>,
    surface_lifecycle: Option<PlannedType17SurfaceLifecycle>,
    surface_bubble: Option<ActorSurfaceBubbleRequest>,
}

#[derive(Debug, Clone, Copy)]
struct PreflightedType17PostTaskSuffix {
    body_basis_q31: Type9BodyBasis,
    velocity_after_environment_raw: [i16; 3],
    surface_timer_after_ms: Option<u32>,
    surface_lifecycle: Option<PlannedType17SurfaceLifecycle>,
    surface_random: Option<PreflightedType17SurfaceRandom>,
}

/// Deterministic Type-17 specialization of the synchronous
/// `FUN_00416750 -> FUN_00410C10` pair.
///
/// Both Common-Dying styles have a null nested release callback, while the
/// already-selected model bit `0x4000` makes the standard-death call a no-op.
/// The executor therefore owns only the release prefix's live masked writes.
#[derive(Debug, Clone, Copy)]
struct PlannedType17SurfaceLifecycle {
    type_default_state_flags_at_0xc0: u32,
}

#[derive(Debug, Clone, Copy)]
struct PreflightedType17SurfaceRandom {
    remaining_percent: u32,
    frame: ActorSurfaceBubbleFrame,
}

#[derive(Debug, Clone, Copy)]
struct Type17PostTaskOwnerSnapshot {
    entity_id: u32,
    self_mass_raw: u16,
    surface_owner_disabled: bool,
    surface_state_flags: u32,
    state_flags_at_0x08: RetailStateWord,
    surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue<u32>,
}

const DIRECT_ATTACHMENT_LOCAL_OFFSET_STATE_BIT: u32 = 0x0000_0800;
const DIRECT_ATTACHMENT_STALE_STATE_BIT: u32 = 0x0000_4000;

fn preflight_type17_scheduler_tail(
    manager: &EntityManager,
    entity: &Entity,
    descriptor: &RetailRuntimeValue<Option<SubJAttachmentDescriptor>>,
    type_default_state_flags_at_0xc0: u32,
    terrain: &TerrainGrid,
) -> Result<PreflightedType17SchedulerTail, Type17CommonDyingProductionBlock> {
    let relation_prelude =
        plan_type17_relation_prelude(manager, entity, terrain, type_default_state_flags_at_0xc0)?;
    let state_after_relation = relation_prelude.state_after(
        entity.collision.state_flags_at_0x08,
        type_default_state_flags_at_0xc0,
    );
    let position_after_relation_raw = relation_prelude.position_after(entity.position_raw());

    let descriptor = match descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingProductionBlock::MissingSubJAttachmentDescriptor)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSubJAttachmentDescriptor)
        }
    };
    let runtime = match &entity.sub_j_attachment_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingProductionBlock::MissingSubJAttachmentRuntime)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSubJAttachmentRuntime)
        }
    };
    if descriptor.reserved_at_0x01 != 0
        || descriptor.slots.len() != 1
        || descriptor.slots[0].policy_word_raw != 1
        || descriptor.slots[0].local_offset_raw != [0, 10, 110]
    {
        return Err(Type17CommonDyingProductionBlock::UnexpectedType17SubJAttachmentDescriptor);
    }
    if runtime.authored_slot_count() != descriptor.slots.len()
        || runtime.len() > descriptor.slots.len()
    {
        return Err(
            Type17CommonDyingProductionBlock::UnexpectedSubJAttachmentShape {
                authored_slots: descriptor.slots.len(),
                runtime_slots: runtime.authored_slot_count(),
                live_rows: runtime.len(),
            },
        );
    }
    let direct_attachments = preflight_type17_direct_attachments(
        manager,
        entity.id,
        runtime.ordered_entity_ids(),
        descriptor,
    )?;
    let RetailRuntimeValue::Known(attached_mass) =
        manager.attached_mass_for_entity_state(entity.id)
    else {
        return Err(Type17CommonDyingProductionBlock::UnresolvedSubJAttachmentRuntime);
    };
    Ok(PreflightedType17SchedulerTail {
        relation_prelude,
        state_after_relation,
        position_after_relation_raw,
        attached_mass,
        direct_attachments,
    })
}

fn plan_type17_relation_prelude(
    manager: &EntityManager,
    entity: &Entity,
    terrain: &TerrainGrid,
    type_default_state_flags_at_0xc0: u32,
) -> Result<PlannedType17RelationPrelude, Type17CommonDyingProductionBlock> {
    match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => return Ok(PlannedType17RelationPrelude::None),
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState)
        }
    }

    let Some(relation_owner_id) = entity.attached_to else {
        let materialiser = manager
            .preflight_type17_missing_relation_materialiser(
                entity.id,
                None,
                terrain,
                type_default_state_flags_at_0xc0,
            )
            .map_err(Type17CommonDyingProductionBlock::Type93Materialiser)?;
        return Ok(PlannedType17RelationPrelude::MaterialiseMissing {
            relation_owner_id: None,
            target_raw: materialiser.target_raw(),
            materialiser,
            attempt: None,
        });
    };
    let Some(relation_owner) = manager
        .iter_all()
        .find(|candidate| candidate.id == relation_owner_id)
    else {
        let materialiser = manager
            .preflight_type17_missing_relation_materialiser(
                entity.id,
                Some(relation_owner_id),
                terrain,
                type_default_state_flags_at_0xc0,
            )
            .map_err(Type17CommonDyingProductionBlock::Type93Materialiser)?;
        return Ok(PlannedType17RelationPrelude::MaterialiseMissing {
            relation_owner_id: Some(relation_owner_id),
            target_raw: materialiser.target_raw(),
            materialiser,
            attempt: None,
        });
    };
    match relation_owner
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            Ok(PlannedType17RelationPrelude::FollowRemote {
                relation_owner_id,
                position_raw: relation_owner.position_raw(),
            })
        }
        RetailRuntimeValue::Known(0) => match &relation_owner.sub_j_attachment_runtime {
            RetailRuntimeValue::Known(Some(runtime)) if runtime.contains(entity.id) => {
                Ok(PlannedType17RelationPrelude::RetainLocal { relation_owner_id })
            }
            RetailRuntimeValue::Known(Some(_)) | RetailRuntimeValue::Known(None) => {
                Ok(PlannedType17RelationPrelude::ReleaseLocal { relation_owner_id })
            }
            RetailRuntimeValue::Unresolved => Err(
                Type17CommonDyingProductionBlock::UnresolvedRelationOwnerAttachmentRuntime {
                    relation_owner_id,
                },
            ),
        },
        RetailRuntimeValue::Unresolved => Err(
            Type17CommonDyingProductionBlock::UnresolvedRelationOwnerState { relation_owner_id },
        ),
        RetailRuntimeValue::Known(_) => unreachable!("the masked bit has only zero/nonzero cases"),
    }
}

fn preflight_type17_direct_attachments(
    manager: &EntityManager,
    owner_entity_id: u32,
    ordered_entity_ids: &[u32],
    descriptor: &SubJAttachmentDescriptor,
) -> Result<PreflightedType17DirectAttachmentTail, Type17CommonDyingProductionBlock> {
    let mut retained_entity_ids = Vec::with_capacity(ordered_entity_ids.len());
    let mut children = Vec::with_capacity(ordered_entity_ids.len());
    let mut descriptor_ordinal = 0usize;
    for &child_entity_id in ordered_entity_ids {
        if child_entity_id == owner_entity_id {
            return Err(
                Type17CommonDyingProductionBlock::DirectAttachmentSelfReference { child_entity_id },
            );
        }
        let Some(child) = manager
            .iter_all()
            .find(|candidate| candidate.id == child_entity_id)
        else {
            // Retail compacts an unresolved handle and reuses this descriptor
            // ordinal for the row shifted into its place.
            continue;
        };
        let state = child.collision.state_flags_at_0x08;
        match state.masked(DIRECT_ATTACHMENT_STALE_STATE_BIT) {
            RetailRuntimeValue::Known(bits) if bits != 0 => continue,
            RetailRuntimeValue::Known(0) => {}
            RetailRuntimeValue::Unresolved => {
                return Err(
                    Type17CommonDyingProductionBlock::UnresolvedDirectAttachmentState {
                        child_entity_id,
                    },
                )
            }
            RetailRuntimeValue::Known(_) => unreachable!("the masked bit is zero or nonzero"),
        }
        if state.known_value_bits() == 0 {
            if state.known_mask() == u32::MAX {
                // The exact-zero state word is stale independently of bit
                // 0x4000, matching the second retail invalidity test.
                continue;
            }
            return Err(
                Type17CommonDyingProductionBlock::UnresolvedDirectAttachmentState {
                    child_entity_id,
                },
            );
        }
        let apply_local_offset = match state.masked(DIRECT_ATTACHMENT_LOCAL_OFFSET_STATE_BIT) {
            RetailRuntimeValue::Known(bits) => bits != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(
                    Type17CommonDyingProductionBlock::UnresolvedDirectAttachmentState {
                        child_entity_id,
                    },
                )
            }
        };
        let Some(slot) = descriptor.slots.get(descriptor_ordinal) else {
            return Err(
                Type17CommonDyingProductionBlock::UnexpectedSubJAttachmentShape {
                    authored_slots: descriptor.slots.len(),
                    runtime_slots: descriptor.slots.len(),
                    live_rows: ordered_entity_ids.len(),
                },
            );
        };
        retained_entity_ids.push(child_entity_id);
        children.push(PreflightedType17DirectAttachment {
            child_entity_id,
            local_offset_raw: slot.local_offset_raw,
            apply_local_offset,
        });
        descriptor_ordinal += 1;
    }
    Ok(PreflightedType17DirectAttachmentTail {
        retained_entity_ids,
        children,
    })
}

fn resolve_type17_direct_attachment_tail(
    preflight: PreflightedType17DirectAttachmentTail,
    owner_position_raw: [i16; 3],
    owner_basis: Type9BodyBasis,
) -> PlannedType17DirectAttachmentTail {
    let updates = preflight
        .children
        .into_iter()
        .map(|child| {
            let mut position_raw = owner_position_raw;
            if child.apply_local_offset {
                let [offset_x, offset_y, offset_z] = child.local_offset_raw.map(i32::from);
                for axis in 0..3 {
                    let delta = q31_mul(owner_basis.lateral[axis], offset_x)
                        .wrapping_add(q31_mul(owner_basis.up[axis], offset_y))
                        .wrapping_add(q31_mul(owner_basis.forward[axis], offset_z));
                    position_raw[axis] = position_raw[axis].wrapping_add(delta as i16);
                }
            }
            PlannedType17DirectAttachmentUpdate {
                child_entity_id: child.child_entity_id,
                position_raw,
            }
        })
        .collect();
    PlannedType17DirectAttachmentTail {
        retained_entity_ids: preflight.retained_entity_ids,
        updates,
    }
}

fn preflight_production_frame(
    manager: &EntityManager,
    owner: LevelOneType17CommonDyingOwner,
    frame: Type17CommonDyingProductionFrame<'_>,
) -> Result<PreflightedProductionFrame, ProductionPlanFailure> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .ok_or(ProductionPlanFailure::Dropped(
            Type17CommonDyingProductionDrop::EntityUnavailable,
        ))?;
    let metadata = manager
        .type_runtime_metadata(TYPE17_COMMON_DYING_ENTITY_TYPE)
        .ok_or(Type17CommonDyingProductionBlock::MissingTypeMetadata)?;
    let type_default_state_flags_at_0xc0 = metadata
        .initializer
        .as_ref()
        .ok_or(Type17CommonDyingProductionBlock::MissingTypeInitializer)?
        .initializer_state_flags_raw;
    if type_default_state_flags_at_0xc0 != LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW {
        return Err(
            Type17CommonDyingProductionBlock::UnexpectedType17DefaultStateFlags {
                actual: type_default_state_flags_at_0xc0,
            }
            .into(),
        );
    }
    let authenticated = LevelOneType17CommonDyingOwner::from_entity_metadata(
        manager.is_fresh_new_game_first_world(),
        entity,
        metadata,
        owner.visit(),
    )
    .map_err(|reason| {
        ProductionPlanFailure::Dropped(Type17CommonDyingProductionDrop::Owner(reason))
    })?;
    if authenticated != owner {
        return Err(ProductionPlanFailure::Dropped(
            Type17CommonDyingProductionDrop::ReceiptMismatch,
        ));
    }
    preflight_normal_scheduler_owner(entity)?;
    let scheduler_tail = preflight_type17_scheduler_tail(
        manager,
        entity,
        &metadata.sub_j_attachment_descriptor,
        type_default_state_flags_at_0xc0,
        frame.terrain,
    )?;
    let callback_mass_raw = plan_type17_callback_mass(entity, metadata.mass_raw)?;

    // FUN_00412DA0 chooses detailed DCA0 only while this bit is set. A clear
    // bit selects E870, whose effective Type-17/class-12 flags are 0x28:
    // style +0x38 value 0x2015 is a clear mask, not the effective word. E870
    // therefore dispatches A800(mode 1) before its own suffix. Plan that
    // distinct path before inspecting detailed-only component evidence.
    let state_flags = match entity
        .collision
        .state_flags_at_0x08
        .masked(TERRAIN_ATTITUDE_BILINEAR_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {
            return preflight_coarse_production_frame(
                entity,
                callback_mass_raw,
                scheduler_tail,
                frame,
            )
            .map_err(ProductionPlanFailure::Blocked)
        }
        RetailRuntimeValue::Known(bits) => bits,
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedDetailedUpdateState.into())
        }
    };
    let scheduler_prefix = plan_detailed_scheduler_prefix(entity, frame.elapsed_micros)?;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = scheduler_prefix.flow
    else {
        return Err(Type17CommonDyingProductionBlock::UnexpectedDetailedSchedulerWait.into());
    };
    let frame = Type17CommonDyingProductionFrame {
        terrain: frame.terrain,
        elapsed_micros: callback_elapsed_us,
    };

    let topology = match metadata.common_mover_topology {
        RetailRuntimeValue::Known(topology) => topology,
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedTopology.into())
        }
    };
    if topology != LEVEL_ONE_TYPE17_COMMON_MOVER_TOPOLOGY {
        return Err(
            Type17CommonDyingProductionBlock::UnexpectedTopology { actual: topology }.into(),
        );
    }
    let sub_a_descriptor = required_copy_descriptor(
        metadata.sub_a_propulsion_descriptor,
        Type17CommonDyingProductionBlock::UnresolvedSubADescriptor,
        Type17CommonDyingProductionBlock::MissingSubADescriptor,
    )?;
    let sub_b_descriptor = required_copy_descriptor(
        metadata.sub_b_lateral_descriptor,
        Type17CommonDyingProductionBlock::UnresolvedSubBDescriptor,
        Type17CommonDyingProductionBlock::MissingSubBDescriptor,
    )?;
    let sub_c_descriptor = required_copy_descriptor(
        metadata.sub_c_lift_descriptor,
        Type17CommonDyingProductionBlock::UnresolvedSubCDescriptor,
        Type17CommonDyingProductionBlock::MissingSubCDescriptor,
    )?;
    if sub_c_descriptor.surface_mode_raw != 0 {
        return Err(
            Type17CommonDyingProductionBlock::UnexpectedSubCSurfaceMode {
                actual: sub_c_descriptor.surface_mode_raw,
            }
            .into(),
        );
    }
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => descriptor.clone(),
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingProductionBlock::MissingSubHDescriptor.into())
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSubHDescriptor.into())
        }
    };

    let sub_a_runtime = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingProductionBlock::MissingSubARuntime.into())
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSubARuntime.into())
        }
    };
    match sub_a_runtime.target_speed_raw() {
        RetailRuntimeValue::Known(0) => {
            return Err(Type17CommonDyingProductionBlock::ZeroSubATargetSpeed.into())
        }
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSubATargetSpeed.into())
        }
    }
    let sub_h_runtime = match &entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime.clone(),
        RetailRuntimeValue::Known(None) => {
            return Err(Type17CommonDyingProductionBlock::MissingSubHRuntime.into())
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSubHRuntime.into())
        }
    };
    if sub_h_runtime.is_enabled() {
        return Err(Type17CommonDyingProductionBlock::SubHUnexpectedlyEnabled.into());
    }

    // The relation prelude runs before the callback, but the callback selector
    // still comes from the entry snapshot. A retained parent membership keeps
    // bit 0x1000 set and suppresses only a due post-unwind transition.
    let transition_suppressed = match scheduler_tail
        .state_after_relation
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) => bits != 0,
        RetailRuntimeValue::Unresolved => {
            return Err(
                Type17CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState.into(),
            )
        }
    };
    let terminal_transition_expected =
        transition_due_this_frame(entity, frame.elapsed_micros) && !transition_suppressed;
    // An allowed terminal transition clears master-motion enable before
    // FUN_00412DA0 rereads state, so no other master gate is observable on
    // that frame. Continuing and suppressed-terminal frames retain the word
    // and must resolve every gate before the task is allowed to mutate.
    let master_motion_state_flags = if terminal_transition_expected {
        0
    } else {
        match scheduler_tail
            .state_after_relation
            .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
        {
            RetailRuntimeValue::Known(bits) => bits,
            RetailRuntimeValue::Unresolved => {
                return Err(Type17CommonDyingProductionBlock::UnresolvedMasterMotionState.into())
            }
        }
    };

    let snapshot = PreflightSnapshot {
        entity_id: entity.id,
        position_raw: scheduler_tail.position_after_relation_raw,
        velocity_raw: entity.velocity_raw(),
        rotation_raw: entity.rotation_heading_pitch_roll_raw(),
        state_flags,
        master_motion_state_flags,
        terminal_transition_expected,
        attached_mass: scheduler_tail.attached_mass,
        topology,
        sub_a_descriptor,
        sub_b_descriptor,
        sub_c_descriptor,
        sub_h_descriptor,
        sub_a_runtime,
        sub_h_runtime,
    };
    let post_task_owner = snapshot_type17_post_task_owner_with_state(
        entity,
        callback_mass_raw,
        scheduler_tail.state_after_relation,
    )?;
    preflight_exact_frame(
        snapshot,
        post_task_owner,
        scheduler_prefix,
        scheduler_tail.relation_prelude,
        scheduler_tail.direct_attachments,
        frame,
    )
    .map(PreflightedProductionFrame::Detailed)
    .map_err(ProductionPlanFailure::Blocked)
}

fn preflight_normal_scheduler_owner(
    entity: &Entity,
) -> Result<(), Type17CommonDyingProductionBlock> {
    let mask = REMOTE_OWNED_STATE_BIT
        | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
        | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT;
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(bits) if bits & REMOTE_OWNED_STATE_BIT != 0 => {
            Err(Type17CommonDyingProductionBlock::RemoteSchedulerOwnerUnimplemented)
        }
        RetailRuntimeValue::Known(bits)
            if bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 =>
        {
            Err(Type17CommonDyingProductionBlock::SchedulerCallbackDisabled)
        }
        RetailRuntimeValue::Known(_) => Ok(()),
        RetailRuntimeValue::Unresolved => {
            Err(Type17CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState)
        }
    }
}

fn plan_type17_callback_mass(
    entity: &Entity,
    authored_mass_raw: u16,
) -> Result<u16, Type17CommonDyingProductionBlock> {
    if authored_mass_raw != LEVEL_ONE_TYPE17_SELF_MASS_RAW {
        return Err(Type17CommonDyingProductionBlock::UnexpectedType17Mass {
            actual: authored_mass_raw,
        });
    }
    let RetailRuntimeValue::Known(mass_raw) = common_scheduler_callback_mass(
        authored_mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Type17CommonDyingProductionBlock::UnresolvedAnimationOffset);
    };
    Ok(mass_raw)
}

fn plan_detailed_scheduler_prefix(
    entity: &Entity,
    elapsed_micros: u32,
) -> Result<PlannedCommonSchedulerPrefix, Type17CommonDyingProductionBlock> {
    if elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(
            Type17CommonDyingProductionBlock::DetailedElapsedExceedsRetailCap {
                actual: elapsed_micros,
            },
        );
    }
    let random_drawn = Cell::new(false);
    let plan = plan_common_scheduler_prefix(&entity.collision, elapsed_micros, &mut || {
        random_drawn.set(true);
        0
    });
    if random_drawn.get() {
        return Err(Type17CommonDyingProductionBlock::UnexpectedDetailedSchedulerRandomDraw);
    }
    let RetailRuntimeValue::Known(plan) = plan else {
        return Err(Type17CommonDyingProductionBlock::UnresolvedDetailedSchedulerState);
    };
    if plan.flow == CommonSchedulerPrefixFlow::WaitingAtCallbackGate {
        return Err(Type17CommonDyingProductionBlock::UnexpectedDetailedSchedulerWait);
    }
    Ok(plan)
}

fn preflight_coarse_production_frame(
    entity: &Entity,
    callback_mass_raw: u16,
    scheduler_tail: PreflightedType17SchedulerTail,
    frame: Type17CommonDyingProductionFrame<'_>,
) -> Result<PreflightedProductionFrame, Type17CommonDyingProductionBlock> {
    if frame.elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(
            Type17CommonDyingProductionBlock::CoarseElapsedExceedsRetailCap {
                actual: frame.elapsed_micros,
            },
        );
    }

    // The callback delta is independent of the random thresholds: RNG only
    // decides whether this visit waits. Preview with zero samples so every
    // possible callback continuation can be proven before the process-global
    // stream is touched.
    let RetailRuntimeValue::Known(preview_prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || 0)
    else {
        return Err(Type17CommonDyingProductionBlock::UnresolvedCoarseSchedulerState);
    };
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = preview_prefix.flow
    else {
        unreachable!("a zero-threshold callback preview cannot wait")
    };
    let callback_frame = Type17CommonDyingProductionFrame {
        terrain: frame.terrain,
        elapsed_micros: callback_elapsed_us,
    };
    let PreflightedType17SchedulerTail {
        relation_prelude,
        state_after_relation,
        position_after_relation_raw,
        attached_mass: _,
        direct_attachments,
    } = scheduler_tail;
    let post_task_suffix = plan_coarse_frame_after_relation(
        entity,
        callback_mass_raw,
        state_after_relation,
        position_after_relation_raw,
        callback_frame,
    )?;
    let direct_attachment_tail = resolve_type17_direct_attachment_tail(
        direct_attachments,
        position_after_relation_raw,
        post_task_suffix.body_basis_q31,
    );
    let transition_suppressed =
        match state_after_relation.masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT) {
            RetailRuntimeValue::Known(bits) => bits != 0,
            RetailRuntimeValue::Unresolved => {
                return Err(Type17CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState)
            }
        };
    let terminal_transition_expected = !transition_suppressed;
    let master_motion_state_flags = if terminal_transition_expected {
        0
    } else {
        match state_after_relation.masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK) {
            RetailRuntimeValue::Known(bits) => bits,
            RetailRuntimeValue::Unresolved => {
                return Err(Type17CommonDyingProductionBlock::UnresolvedMasterMotionState)
            }
        }
    };
    let master_motion = plan_common_master_motion(
        position_after_relation_raw,
        post_task_suffix.velocity_after_environment_raw,
        master_motion_state_flags,
        callback_elapsed_us,
    );

    Ok(PreflightedProductionFrame::Coarse {
        collision: entity.collision.clone(),
        elapsed_micros: frame.elapsed_micros,
        callback: PlannedCoarseProductionFrame {
            scheduler_prefix: preview_prefix,
            relation_prelude,
            callback_mass_raw,
            post_task_suffix,
            direct_attachment_tail,
            master_motion,
            terminal_transition_expected,
        },
    })
}

#[cfg(test)]
fn plan_coarse_frame(
    entity: &Entity,
    callback_mass_raw: u16,
    frame: Type17CommonDyingProductionFrame<'_>,
) -> Result<PreflightedType17PostTaskSuffix, Type17CommonDyingProductionBlock> {
    plan_coarse_frame_after_relation(
        entity,
        callback_mass_raw,
        entity.collision.state_flags_at_0x08,
        entity.position_raw(),
        frame,
    )
}

fn plan_coarse_frame_after_relation(
    entity: &Entity,
    callback_mass_raw: u16,
    state_after_relation: RetailStateWord,
    position_after_relation_raw: [i16; 3],
    frame: Type17CommonDyingProductionFrame<'_>,
) -> Result<PreflightedType17PostTaskSuffix, Type17CommonDyingProductionBlock> {
    let owner = snapshot_type17_post_task_owner_with_state(
        entity,
        callback_mass_raw,
        state_after_relation,
    )?;

    // E870 rebuilds all nine basis dwords before E100; E370 consumes its
    // forward column when the random class-42 branch hits. The transition does
    // not mutate the angle words, so derive the exact post-task matrix now.
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    let body_basis_q31 = Type9BodyBasis::from_angle_words(heading_raw, pitch_raw, roll_raw);
    preflight_type17_post_task_suffix(
        owner,
        position_after_relation_raw,
        entity.velocity_raw(),
        body_basis_q31,
        frame,
    )
}

#[cfg(test)]
fn snapshot_type17_post_task_owner(
    entity: &Entity,
    self_mass_raw: u16,
) -> Result<Type17PostTaskOwnerSnapshot, Type17CommonDyingProductionBlock> {
    snapshot_type17_post_task_owner_with_state(
        entity,
        self_mass_raw,
        entity.collision.state_flags_at_0x08,
    )
}

fn snapshot_type17_post_task_owner_with_state(
    entity: &Entity,
    self_mass_raw: u16,
    state_flags_at_0x08: RetailStateWord,
) -> Result<Type17PostTaskOwnerSnapshot, Type17CommonDyingProductionBlock> {
    let surface_state_flags = match state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT | REMOTE_OWNED_STATE_BIT | 0x0000_4000)
    {
        RetailRuntimeValue::Known(bits) => bits,
        RetailRuntimeValue::Unresolved => {
            return Err(Type17CommonDyingProductionBlock::UnresolvedSurfaceOwnerState)
        }
    };
    if surface_state_flags & 0x0000_4000 == 0 {
        return Err(Type17CommonDyingProductionBlock::UnexpectedSurfaceLifecycleModelState);
    }
    Ok(Type17PostTaskOwnerSnapshot {
        entity_id: entity.id,
        self_mass_raw,
        surface_owner_disabled: surface_state_flags & ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT != 0,
        surface_state_flags,
        state_flags_at_0x08,
        surface_lifetime_timer_ms_at_0x48: entity.surface_lifetime_timer_ms_at_0x48,
    })
}

fn preflight_type17_post_task_suffix(
    owner: Type17PostTaskOwnerSnapshot,
    position_after_task_raw: [i16; 3],
    mut velocity_after_task_raw: [i16; 3],
    body_basis_q31: Type9BodyBasis,
    frame: Type17CommonDyingProductionFrame<'_>,
) -> Result<PreflightedType17PostTaskSuffix, Type17CommonDyingProductionBlock> {
    let (surface_timer_after_ms, surface_lifecycle, surface_random) =
        if owner.surface_owner_disabled {
            // E370 returns before reading entity +0x48.
            (None, None, None)
        } else {
            let timer_ms_at_0x48 = match owner.surface_lifetime_timer_ms_at_0x48 {
                RetailRuntimeValue::Known(timer) => timer,
                RetailRuntimeValue::Unresolved => {
                    return Err(Type17CommonDyingProductionBlock::UnresolvedSurfaceLifetimeTimer)
                }
            };
            match classify_actor_surface_timer_phase(
                timer_ms_at_0x48,
                ActorSurfaceTimerFrame {
                    // The disabled bit was resolved above. No other state bit is
                    // read before the classifier returns its RNG boundary.
                    state_flags: 0,
                    position_y_raw: position_after_task_raw[1],
                    active_model_extent_raw: LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW,
                    flat_surface_y_raw: frame.terrain.sea_level_raw(),
                    elapsed_us: frame.elapsed_micros,
                    authored_lifetime_ms: LEVEL_ONE_TYPE17_SURFACE_LIFETIME_MS,
                },
            )
            .expect("Type 17 has a nonzero statically authenticated lifetime")
            {
                ActorSurfaceTimerPhase::NonDeep { timer_after_ms }
                | ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. } => {
                    (Some(timer_after_ms), None, None)
                }
                ActorSurfaceTimerPhase::DeepRandomEffects {
                    timer_after_ms,
                    remaining_percent,
                } => (
                    Some(timer_after_ms),
                    None,
                    Some(preflight_type17_surface_random(
                        owner,
                        position_after_task_raw,
                        body_basis_q31.forward,
                        remaining_percent,
                    )),
                ),
                ActorSurfaceTimerPhase::DeepLifecycle {
                    timer_after_ms,
                    remaining_percent_after_lifecycle,
                } => (
                    Some(timer_after_ms),
                    Some(plan_type17_surface_lifecycle(owner)),
                    ((remaining_percent_after_lifecycle as i32) <= 74).then(|| {
                        preflight_type17_surface_random(
                            owner,
                            position_after_task_raw,
                            body_basis_q31.forward,
                            remaining_percent_after_lifecycle,
                        )
                    }),
                ),
                ActorSurfaceTimerPhase::OwnerDisabled => {
                    unreachable!("the resolved enabled owner cannot classify as disabled")
                }
            }
        };

    apply_first_world_type17_environment_raw(
        &mut velocity_after_task_raw,
        frame.elapsed_micros,
        owner.self_mass_raw,
    );
    Ok(PreflightedType17PostTaskSuffix {
        body_basis_q31,
        velocity_after_environment_raw: velocity_after_task_raw,
        surface_timer_after_ms,
        surface_lifecycle,
        surface_random,
    })
}

fn preflight_type17_surface_random(
    owner: Type17PostTaskOwnerSnapshot,
    position_raw: [i16; 3],
    emission_axis_q31: [i32; 3],
    remaining_percent: u32,
) -> PreflightedType17SurfaceRandom {
    PreflightedType17SurfaceRandom {
        remaining_percent,
        frame: ActorSurfaceBubbleFrame {
            entity_id: owner.entity_id,
            entity_type: TYPE17_COMMON_DYING_ENTITY_TYPE as u8,
            // The normal scheduler-owner preflight proves the remote/impact-
            // suppression bit clear. Relation release does not alter it.
            state_flags: owner.surface_state_flags,
            position_raw,
            emission_axis_q31,
            active_model_extent_raw: LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW,
        },
    }
}

fn plan_type17_surface_lifecycle(
    owner: Type17PostTaskOwnerSnapshot,
) -> PlannedType17SurfaceLifecycle {
    let state_after = relation_release_state_word_after(
        owner.state_flags_at_0x08,
        LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
    );
    debug_assert_eq!(
        state_after.masked(REMOTE_OWNED_STATE_BIT | 0x0000_4000),
        RetailRuntimeValue::Known(0x0000_4000),
        "release preserves Type 17's selected-model bit, so FUN_00410C10 is a no-op"
    );
    PlannedType17SurfaceLifecycle {
        type_default_state_flags_at_0xc0: LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
    }
}

fn resolve_type17_post_task_suffix(
    preflight: PreflightedType17PostTaskSuffix,
    next_shared_random: &mut impl FnMut() -> u32,
) -> PlannedType17PostTaskSuffix {
    let surface_bubble = preflight.surface_random.and_then(|random| {
        plan_actor_surface_bubble(random.remaining_percent, random.frame, next_shared_random)
    });
    PlannedType17PostTaskSuffix {
        body_basis_q31: preflight.body_basis_q31,
        velocity_after_environment_raw: preflight.velocity_after_environment_raw,
        surface_timer_after_ms: preflight.surface_timer_after_ms,
        surface_lifecycle: preflight.surface_lifecycle,
        surface_bubble,
    }
}

fn required_copy_descriptor<T: Copy>(
    value: RetailRuntimeValue<Option<T>>,
    unresolved: Type17CommonDyingProductionBlock,
    missing: Type17CommonDyingProductionBlock,
) -> Result<T, ProductionPlanFailure> {
    match value {
        RetailRuntimeValue::Known(Some(value)) => Ok(value),
        RetailRuntimeValue::Known(None) => Err(missing.into()),
        RetailRuntimeValue::Unresolved => Err(unresolved.into()),
    }
}

fn transition_due_this_frame(entity: &Entity, elapsed_micros: u32) -> bool {
    let Some(ActorTaskRuntime::CommonDying(state)) =
        entity.actor_task_state(ActorTaskSlot::Primary)
    else {
        return false;
    };
    COMMON_DYING_TASK_LIFETIME_MS < state.elapsed_ms().wrapping_add(elapsed_micros / 1_000)
}

fn preflight_exact_frame(
    snapshot: PreflightSnapshot,
    post_task_owner: Type17PostTaskOwnerSnapshot,
    scheduler_prefix: PlannedCommonSchedulerPrefix,
    relation_prelude: PlannedType17RelationPrelude,
    direct_attachments: PreflightedType17DirectAttachmentTail,
    frame: Type17CommonDyingProductionFrame<'_>,
) -> Result<
    PlannedDetailedProductionFrame<PreflightedType17PostTaskSuffix>,
    Type17CommonDyingProductionBlock,
> {
    let effect_inputs = CommonDyingEffectInputs {
        descriptor_effect_raw: RetailRuntimeValue::Known(Some(
            snapshot.sub_c_descriptor.surface_mode_raw,
        )),
        attached_mass: RetailRuntimeValue::Known(snapshot.attached_mass as i32),
    };
    let selected_effect_raw = select_common_dying_effect_raw(effect_inputs)
        .map_err(Type17CommonDyingProductionBlock::EffectSelection)?;
    debug_assert_eq!(selected_effect_raw, 0);

    // FUN_0041EC70 and the following common mover both consume the basis
    // captured before the effect mutates pitch/roll.
    let [heading_raw, pitch_raw, roll_raw] = snapshot.rotation_raw;
    let pre_effect_basis = HoverBasis::from_angle_words(heading_raw, pitch_raw, roll_raw);
    let attitude = plan_common_dying_terrain_attitude_raw(CommonDyingTerrainAttitudeInput {
        terrain: frame.terrain,
        position_raw: snapshot.position_raw,
        pitch_raw,
        roll_raw,
        lateral_basis_q31: pre_effect_basis.lateral,
        forward_basis_q31: pre_effect_basis.forward,
        body_up_y_q31: pre_effect_basis.up[1],
        state_flags: snapshot.state_flags,
        resolved_surface_mode_raw: selected_effect_raw as i8,
        water_enabled: false,
        wave_tick_50hz: 0,
        effective_elapsed_micros: frame.elapsed_micros,
    });

    let mover = plan_null_target_mover(
        &snapshot,
        pre_effect_basis,
        frame.terrain,
        frame.elapsed_micros,
    )?;
    let mut velocity_after_task_raw = mover.snapshot.velocity_raw;
    velocity_after_task_raw[0] =
        damp_common_dying_axis(velocity_after_task_raw[0], frame.elapsed_micros);
    velocity_after_task_raw[2] =
        damp_common_dying_axis(velocity_after_task_raw[2], frame.elapsed_micros);
    let post_effect_basis =
        Type9BodyBasis::from_angle_words(heading_raw, attitude.pitch_raw, attitude.roll_raw);
    let post_task_suffix = preflight_type17_post_task_suffix(
        post_task_owner,
        mover.snapshot.position_raw,
        velocity_after_task_raw,
        post_effect_basis,
        frame,
    )?;
    let master_motion = plan_common_master_motion(
        mover.snapshot.position_raw,
        post_task_suffix.velocity_after_environment_raw,
        snapshot.master_motion_state_flags,
        frame.elapsed_micros,
    );
    let direct_attachment_tail = resolve_type17_direct_attachment_tail(
        direct_attachments,
        mover.snapshot.position_raw,
        post_effect_basis,
    );
    Ok(PlannedDetailedProductionFrame {
        scheduler_prefix,
        relation_prelude,
        callback_mass_raw: post_task_owner.self_mass_raw,
        effect_inputs,
        selected_effect_raw,
        mover_velocity_raw: mover.snapshot.velocity_raw,
        live_commit: Type17CommonDyingLiveFrameCommit {
            position_raw: mover.snapshot.position_raw,
            pitch_raw: attitude.pitch_raw,
            roll_raw: attitude.roll_raw,
            sub_h_runtime: mover.sub_h_runtime,
        },
        post_task_suffix,
        direct_attachment_tail,
        master_motion,
        terminal_transition_expected: snapshot.terminal_transition_expected,
    })
}

#[derive(Debug)]
struct PlannedMover {
    snapshot: CommonMoverFrameSnapshot,
    sub_h_runtime: SubHRuntimeState,
}

fn plan_null_target_mover(
    input: &PreflightSnapshot,
    pre_effect_basis: HoverBasis,
    terrain: &TerrainGrid,
    elapsed_micros: u32,
) -> Result<PlannedMover, Type17CommonDyingProductionBlock> {
    if input.sub_h_runtime.is_enabled() {
        return Err(Type17CommonDyingProductionBlock::SubHUnexpectedlyEnabled);
    }
    let mut snapshot = CommonMoverFrameSnapshot {
        controlled_entity_lookup: None,
        position_raw: input.position_raw,
        velocity_raw: input.velocity_raw,
        heading_raw: input.rotation_raw[0] as u16,
        roll_raw: input.rotation_raw[2] as u16,
        body_up_q31: pre_effect_basis.up,
        body_right_q31: pre_effect_basis.lateral,
        body_forward_q31: pre_effect_basis.forward,
        attached_cargo_mass: input.attached_mass,
        sub_a_runtime: RetailRuntimeValue::Known(Some(input.sub_a_runtime)),
        sub_f_smoothed_raw: None,
        sub_g_runtime_angle_raw: 0,
        sub_g_state_byte_3c: 0,
        sub_k_smoothed_raw: None,
        sub_n_accumulator_raw: None,
        sub_o_link_raw: 0,
    };
    let random_drawn = Cell::new(false);
    let mut machine = CommonMoverFrameMachine::start(
        CommonMoverFrameRequest {
            configuration: CommonMoverFrameConfiguration {
                controlled_entity_handle: input.entity_id,
                topology: input.topology,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros,
                sub_a_descriptor: Some(input.sub_a_descriptor),
                sub_b_descriptor: Some(input.sub_b_descriptor),
                sub_c_descriptor: Some(HoverLiftConfig::from(input.sub_c_descriptor)),
                // A null target skips Sub-D before its descriptor is read.
                sub_d_descriptor: None,
                target_resource_context_present: false,
            },
            initial_snapshot: snapshot,
            target_private: None,
            tracked_target: RetailRuntimeValue::Unresolved,
        },
        || {
            random_drawn.set(true);
            0
        },
    )
    .map_err(Type17CommonDyingProductionBlock::MoverStart)?;
    if random_drawn.get() {
        return Err(Type17CommonDyingProductionBlock::UnexpectedMoverRandomDraw);
    }

    let mut sub_h_runtime = input.sub_h_runtime.clone();
    for _ in 0..16 {
        match machine.poll() {
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubH,
                controlled_entity_handle,
                elapsed_micros: action_elapsed,
            }) if controlled_entity_handle == input.entity_id
                && action_elapsed == elapsed_micros =>
            {
                let cues = sub_h_runtime
                    .update(&input.sub_h_descriptor, elapsed_micros as i32)
                    .map_err(Type17CommonDyingProductionBlock::SubHUpdate)?;
                if !cues.is_empty() {
                    return Err(
                        Type17CommonDyingProductionBlock::UnexpectedSubHCompletionCues {
                            count: cues.len(),
                        },
                    );
                }
                machine
                    .resume(CommonMoverFrameResume::ComponentReturned {
                        phase: CommonMoverDispatchPhase::SubH,
                        snapshot,
                    })
                    .map_err(Type17CommonDyingProductionBlock::MoverAdvance)?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface: false,
            }) => {
                machine
                    .resume(CommonMoverFrameResume::SubCSurfaceSampled(
                        SubCSurfaceSample::Terrain {
                            terrain_y_raw: terrain.bilinear_height_raw(point_raw[0], point_raw[1]),
                        },
                    ))
                    .map_err(Type17CommonDyingProductionBlock::MoverAdvance)?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC {
                position_raw,
                velocity_raw,
                ..
            }) => {
                snapshot.position_raw = position_raw;
                snapshot.velocity_raw = velocity_raw;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::SubC,
                        snapshot,
                    })
                    .map_err(Type17CommonDyingProductionBlock::MoverAdvance)?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubA {
                velocity_raw,
                ..
            }) => {
                snapshot.velocity_raw = velocity_raw;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::SubA,
                        snapshot,
                    })
                    .map_err(Type17CommonDyingProductionBlock::MoverAdvance)?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { velocity_raw }) => {
                snapshot.velocity_raw = velocity_raw;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::SubB,
                        snapshot,
                    })
                    .map_err(Type17CommonDyingProductionBlock::MoverAdvance)?;
            }
            CommonMoverFramePoll::Action(action) => {
                return Err(Type17CommonDyingProductionBlock::UnexpectedMoverAction(
                    action,
                ))
            }
            CommonMoverFramePoll::Blocked(reason) => {
                return Err(Type17CommonDyingProductionBlock::MoverBlocked(reason))
            }
            CommonMoverFramePoll::ReturnZero(reason) => {
                return Err(Type17CommonDyingProductionBlock::UnexpectedMoverReturnZero(
                    reason,
                ))
            }
            CommonMoverFramePoll::ReturnOne => {
                return Ok(PlannedMover {
                    snapshot,
                    sub_h_runtime,
                })
            }
        }
    }
    Err(Type17CommonDyingProductionBlock::MoverDidNotTerminate)
}

fn execute_preplanned_frame(
    owner: LevelOneType17CommonDyingOwner,
    manager: &mut EntityManager,
    elapsed_micros: u32,
    plan: PlannedProductionFrame,
) -> Result<ExecutedProductionFrame, Type17CommonDyingLiveFrameError> {
    match plan {
        PlannedProductionFrame::Detailed(plan) => {
            execute_preplanned_detailed_frame(owner, manager, elapsed_micros, plan).map(
                |(live_outcome, surface_bubble)| ExecutedProductionFrame::Advanced {
                    live_outcome,
                    surface_bubble,
                },
            )
        }
        PlannedProductionFrame::Coarse(plan) => {
            execute_preplanned_coarse_frame(owner, manager, plan).map(
                |(live_outcome, surface_bubble)| ExecutedProductionFrame::Advanced {
                    live_outcome,
                    surface_bubble,
                },
            )
        }
        PlannedProductionFrame::SchedulerWait(prefix) => {
            let entity = manager
                .common_actor_dying_entity_mut(owner.entity_id())
                .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                    entity_id: owner.entity_id(),
                })?;
            commit_common_scheduler_prefix(&mut entity.collision, prefix);
            Ok(ExecutedProductionFrame::SchedulerWaiting)
        }
    }
}

fn execute_preplanned_detailed_frame(
    owner: LevelOneType17CommonDyingOwner,
    manager: &mut EntityManager,
    _outer_elapsed_micros: u32,
    plan: PlannedDetailedProductionFrame,
) -> Result<
    (
        Type17CommonDyingLiveFrameOutcome,
        Option<ActorSurfaceBubbleRequest>,
    ),
    Type17CommonDyingLiveFrameError,
> {
    let PlannedDetailedProductionFrame {
        scheduler_prefix,
        relation_prelude,
        callback_mass_raw,
        effect_inputs,
        selected_effect_raw,
        mover_velocity_raw,
        live_commit,
        post_task_suffix,
        direct_attachment_tail,
        master_motion,
        terminal_transition_expected,
    } = plan;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: elapsed_micros,
    } = scheduler_prefix.flow
    else {
        unreachable!("a detailed frame cannot carry a waiting prefix")
    };
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner.entity_id(),
            })?;
        commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
    }
    commit_type17_relation_prelude(manager, owner.entity_id(), relation_prelude)?;
    manager
        .common_actor_dying_entity_mut(owner.entity_id())
        .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
            entity_id: owner.entity_id(),
        })?
        .mass_raw = callback_mass_raw;
    let effect_invoked = Cell::new(false);
    let mover_invoked = Cell::new(false);
    let outcome = owner.tick_published_normal_with_preplanned_commit(
        manager,
        Type17CommonDyingLiveFrameRequest {
            visit: owner.visit(),
            elapsed_micros,
            type_runtime: (),
            component_runtime: (),
        },
        || effect_inputs,
        |effect| {
            debug_assert_eq!(effect.owner_entity_id, owner.entity_id());
            debug_assert_eq!(effect.effect_kind, COMMON_DYING_EFFECT_KIND);
            debug_assert_eq!(effect.signed_effect_raw, selected_effect_raw);
            debug_assert_eq!(effect.enabled, COMMON_DYING_EFFECT_ENABLED);
            debug_assert_eq!(effect.elapsed_micros, elapsed_micros);
            effect_invoked.set(true);
        },
        |mover, velocity_raw| {
            debug_assert_eq!(mover.task_wrapper, owner.visit());
            debug_assert_eq!(mover.owner_entity_id, owner.entity_id());
            debug_assert_eq!(mover.explicit_target_raw, None);
            debug_assert_eq!(mover.elapsed_micros, elapsed_micros);
            debug_assert_eq!(mover.scheduler_mode, 0);
            *velocity_raw = mover_velocity_raw;
            mover_invoked.set(true);
            1
        },
        live_commit,
    )?;
    // Retail DCA0 reaches this suffix after the detailed task, including after
    // a timeout-triggered variant-one transition. Type 17's low-health +0xA0
    // selector is zero, so the intervening branch returns before cadence/RNG.
    // Commit FUN_00413F70, E100, then the preflighted E370 timer write.
    let surface_bubble = post_task_suffix.surface_bubble;
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner.entity_id(),
            })?;
        commit_type17_post_task_suffix(entity, post_task_suffix);
    }
    commit_type17_direct_attachment_tail(manager, owner.entity_id(), direct_attachment_tail)?;
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner.entity_id(),
            })?;
        commit_common_scheduler_post_callback(&mut entity.collision);
        commit_common_master_motion(entity, master_motion);
    }
    debug_assert_eq!(
        matches!(
            outcome,
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged { .. }
        ),
        terminal_transition_expected
    );
    debug_assert!(effect_invoked.get());
    debug_assert!(mover_invoked.get());
    Ok((outcome, surface_bubble))
}

fn execute_preplanned_coarse_frame(
    owner: LevelOneType17CommonDyingOwner,
    manager: &mut EntityManager,
    plan: PlannedCoarseProductionFrame,
) -> Result<
    (
        Type17CommonDyingLiveFrameOutcome,
        Option<ActorSurfaceBubbleRequest>,
    ),
    Type17CommonDyingLiveFrameError,
> {
    let PlannedCoarseProductionFrame {
        scheduler_prefix,
        relation_prelude,
        callback_mass_raw,
        post_task_suffix,
        direct_attachment_tail,
        master_motion,
        terminal_transition_expected,
    } = plan;
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: elapsed_micros,
    } = scheduler_prefix.flow
    else {
        unreachable!("a coarse callback frame cannot carry a waiting prefix")
    };
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner.entity_id(),
            })?;
        commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
    }
    commit_type17_relation_prelude(manager, owner.entity_id(), relation_prelude)?;
    manager
        .common_actor_dying_entity_mut(owner.entity_id())
        .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
            entity_id: owner.entity_id(),
        })?
        .mass_raw = callback_mass_raw;
    let outcome = owner.tick_published_coarse(manager, elapsed_micros)?;
    debug_assert!(matches!(
        (&outcome, terminal_transition_expected),
        (
            Type17CommonDyingLiveFrameOutcome::DeferredDestroyStaged {
                callback_evidence: Type17CommonDyingCallbackEvidence::Coarse { .. },
                ..
            },
            true
        ) | (
            Type17CommonDyingLiveFrameOutcome::TransitionSuppressed {
                callback_evidence: Type17CommonDyingCallbackEvidence::Coarse { .. },
                ..
            },
            false
        )
    ));

    // Retail enters E870's suffix after the mode-1 tag. An admitted transition
    // has already published variant 1, cleared the task slots, and staged
    // FUN_00410B70; a retained relation suppresses those mutations but still
    // reaches this suffix. Commit the preflighted E100 and E370 writes in that
    // same order before the later same-tick sweep. Only an admitted transition
    // makes master motion impossible.
    let surface_bubble = post_task_suffix.surface_bubble;
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner.entity_id(),
            })?;
        commit_type17_post_task_suffix(entity, post_task_suffix);
    }
    commit_type17_direct_attachment_tail(manager, owner.entity_id(), direct_attachment_tail)?;
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner.entity_id(),
            })?;
        commit_common_scheduler_post_callback(&mut entity.collision);
        commit_common_master_motion(entity, master_motion);
        if terminal_transition_expected {
            debug_assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x0004_0000),
                RetailRuntimeValue::Known(0)
            );
        }
    }
    Ok((outcome, surface_bubble))
}

fn commit_type17_relation_prelude(
    manager: &mut EntityManager,
    entity_id: u32,
    plan: PlannedType17RelationPrelude,
) -> Result<(), Type17CommonDyingLiveFrameError> {
    match plan {
        PlannedType17RelationPrelude::None | PlannedType17RelationPrelude::RetainLocal { .. } => {}
        PlannedType17RelationPrelude::ReleaseLocal { .. } => {
            let entity = manager
                .common_actor_dying_entity_mut(entity_id)
                .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable { entity_id })?;
            commit_type17_relation_release(entity, LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW);
        }
        PlannedType17RelationPrelude::FollowRemote { position_raw, .. } => {
            let entity = manager
                .common_actor_dying_entity_mut(entity_id)
                .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable { entity_id })?;
            entity.set_position_raw(position_raw);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x0000_0020, 0x0000_0020);
        }
        PlannedType17RelationPrelude::MaterialiseMissing { attempt, .. } => {
            if !manager.commit_type17_missing_relation_materialiser(
                attempt.expect("committed Type93 construction precedes relation publication"),
            ) {
                return Err(Type17CommonDyingLiveFrameError::EntityUnavailable { entity_id });
            }
        }
    }
    Ok(())
}

fn commit_type17_direct_attachment_tail(
    manager: &mut EntityManager,
    owner_entity_id: u32,
    plan: PlannedType17DirectAttachmentTail,
) -> Result<(), Type17CommonDyingLiveFrameError> {
    // No callback or external action occurs inside FUN_00418640. Validate the
    // preflighted child leases before publishing its callback-free compaction.
    if let Some(missing) = plan.updates.iter().find(|update| {
        !manager
            .iter_all()
            .any(|entity| entity.id == update.child_entity_id)
    }) {
        return Err(Type17CommonDyingLiveFrameError::EntityUnavailable {
            entity_id: missing.child_entity_id,
        });
    }
    {
        let owner = manager
            .common_actor_dying_entity_mut(owner_entity_id)
            .ok_or(Type17CommonDyingLiveFrameError::EntityUnavailable {
                entity_id: owner_entity_id,
            })?;
        let RetailRuntimeValue::Known(Some(runtime)) = &mut owner.sub_j_attachment_runtime else {
            unreachable!("the preplanned Type-17 owner must retain its authenticated Sub-J runtime")
        };
        runtime.commit_stable_compaction(plan.retained_entity_ids);
    }
    for update in plan.updates {
        let child = manager
            .type17_direct_attachment_entity_mut(update.child_entity_id)
            .expect("every direct child lease was revalidated before compaction");
        child.set_motion_raw(update.position_raw, [0; 3]);
    }
    Ok(())
}

fn commit_type17_post_task_suffix(entity: &mut Entity, plan: PlannedType17PostTaskSuffix) {
    // FUN_00413F70 writes the full basis and then publishes state bit 0x4.
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(plan.body_basis_q31);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    entity.set_velocity_raw(plan.velocity_after_environment_raw);
    if let Some(timer_after_ms) = plan.surface_timer_after_ms {
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer_after_ms);
    }
    if let Some(lifecycle) = plan.surface_lifecycle {
        commit_type17_surface_lifecycle(entity, lifecycle);
    }
}

fn commit_type17_surface_lifecycle(entity: &mut Entity, lifecycle: PlannedType17SurfaceLifecycle) {
    // FUN_00416750 first applies its entry edit, then FUN_0040D3C0 refreshes
    // +0xC8 and applies the type policy, and finally +0x80 is overwritten with
    // the zero g_entity_db sentinel. The common Type-17 trampoline reaches a
    // null class-12 release hook. The following FUN_00410C10 sees preserved
    // bit 0x4000 and returns one without another mutation or side effect.
    commit_type17_relation_release(entity, lifecycle.type_default_state_flags_at_0xc0);
}

fn commit_type17_relation_release(entity: &mut Entity, type_default_state_flags_at_0xc0: u32) {
    entity.collision.state_flags_at_0x08 = relation_release_state_word_after(
        entity.collision.state_flags_at_0x08,
        type_default_state_flags_at_0xc0,
    );
    entity.collision.default_state_flags_at_0xc8 =
        RetailRuntimeValue::Known(type_default_state_flags_at_0xc0);
    entity.attached_to = None;
}

#[cfg(test)]
pub(crate) use tests::coarse_type17_common_dying_composite_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_death::COMMON_ACTOR_DYING_ACTIVE_STYLE;
    use crate::actor_task_owner::{ActorTaskOwner, ActorTaskVisit, PreparedActorTask};
    use crate::common_dying::{
        CommonDyingComponentDescriptors, CommonDyingConstructorEffect, CommonDyingTaskState,
    };
    use crate::common_dying_live::{
        COMMON_DYING_BEHAVIOR_CLASS_ID, FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES,
        TYPE17_COMMON_DYING_MODEL_ID,
    };
    use crate::common_mover::sub_c::{apply_sub_c_lift_raw, sub_c_surface_sample_point_raw};
    use crate::common_mover::type9_tail::apply_common_master_motion_raw;
    use crate::common_mover::{apply_sub_a_propulsion_raw, apply_sub_b_lateral_raw};
    use crate::entity_behavior::{audited_behavior_program, BehaviorContextRuntime};
    use crate::entity_collision_state::{EntityInitializerSpec, EntityTypeRuntimeMetadata};
    use crate::{entity::EntityKind, sub_j_attachment::SubJAttachmentRuntime};
    use v2k_formats::collision::{SubHExternalFrameRecord, SubJAttachmentSlotDescriptor};
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    // Force/surface oracle fixtures exercise both phases with unavailable
    // construction lineage. Dynamic birth controls below use a real manager.
    fn plan_exact_frame(
        snapshot: PreflightSnapshot,
        post_task_owner: Type17PostTaskOwnerSnapshot,
        scheduler_prefix: PlannedCommonSchedulerPrefix,
        relation_prelude: PlannedType17RelationPrelude,
        direct_attachments: PreflightedType17DirectAttachmentTail,
        frame: Type17CommonDyingProductionFrame<'_>,
        next_shared_random: &mut impl FnMut() -> u32,
    ) -> Result<PlannedDetailedProductionFrame, Type17CommonDyingProductionBlock> {
        let prepared = preflight_exact_frame(
            snapshot,
            post_task_owner,
            scheduler_prefix,
            relation_prelude,
            direct_attachments,
            frame,
        )?;
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        let PlannedProductionFrame::Detailed(plan) = resolve_production_frame(
            PreflightedProductionFrame::Detailed(prepared),
            &mut manager,
            next_shared_random,
        ) else {
            unreachable!()
        };
        Ok(plan)
    }

    fn plan_coarse_production_frame(
        entity: &Entity,
        callback_mass_raw: u16,
        scheduler_tail: PreflightedType17SchedulerTail,
        frame: Type17CommonDyingProductionFrame<'_>,
        next_shared_random: &mut impl FnMut() -> u32,
    ) -> Result<PlannedProductionFrame, Type17CommonDyingProductionBlock> {
        let prepared =
            preflight_coarse_production_frame(entity, callback_mass_raw, scheduler_tail, frame)?;
        let mut manager = EntityManager::from_entities_for_test(Vec::new());
        Ok(resolve_production_frame(
            prepared,
            &mut manager,
            next_shared_random,
        ))
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
    const SUB_C: SubCLiftDescriptor = SubCLiftDescriptor {
        base_clearance_raw: 150,
        lift_range_raw: 125,
        strength_raw: 0x0090_0000,
        near_boost_range_raw: 100,
        damping_range_raw: 200,
        surface_mode_raw: 0,
        offset_sample_raw: 0,
        reserved_at_0x0e: [0; 2],
    };

    fn flat_terrain(height: i8) -> TerrainGrid {
        TerrainGrid {
            header: [(-0x1800_i32) << 8, 0, 0, 0, 0],
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

    fn sub_h_descriptor() -> SubHExternalFrameDescriptor {
        SubHExternalFrameDescriptor {
            completion_sound_id: Some(71),
            records: vec![SubHExternalFrameRecord {
                resolver_flags_raw: 0,
                phase_rate_raw: i32::MAX,
                vertex_refs: [0; 3],
                axis_mode_raw: 0,
                dependencies: [0; 4],
            }],
        }
    }

    fn sub_j_descriptor(offsets: &[[i16; 3]]) -> SubJAttachmentDescriptor {
        SubJAttachmentDescriptor {
            reserved_at_0x01: 0,
            slots: offsets
                .iter()
                .copied()
                .map(|local_offset_raw| SubJAttachmentSlotDescriptor {
                    policy_word_raw: 1,
                    local_offset_raw,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Build one runnable coarse Type-17 receipt without duplicating this
    /// module's scheduler, surface, and component setup in a composite test.
    pub(crate) fn coarse_type17_common_dying_composite_fixture(
        entity_id: u32,
        authored_spawn_index: usize,
    ) -> (
        Entity,
        EntityTypeRuntimeMetadata,
        LevelOneType17CommonDyingOwner,
    ) {
        assert!(
            FRESH_LEVEL1_TYPE17_COMMON_DYING_SPAWN_INDICES.contains(&authored_spawn_index),
            "the fixture retains an authenticated first-world spawn"
        );
        let sub_j_descriptor = sub_j_descriptor(&[[0, 10, 110]]);
        let metadata = EntityTypeRuntimeMetadata {
            model_slots: [TYPE17_COMMON_DYING_MODEL_ID as u16; 4],
            mass_raw: LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(SUB_A)),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(SUB_B)),
            sub_c_lift_descriptor: RetailRuntimeValue::Known(Some(SUB_C)),
            sub_h_external_frame_descriptor: RetailRuntimeValue::Known(Some(sub_h_descriptor())),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(Some(sub_j_descriptor.clone())),
            common_mover_topology: RetailRuntimeValue::Known(
                LEVEL_ONE_TYPE17_COMMON_MOVER_TOPOLOGY,
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
                common_axis_descriptor: Default::default(),
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        };

        let mut entity = coarse_entity(0, RetailRuntimeValue::Known(125));
        entity.id = entity_id;
        entity.authored_spawn_index = Some(authored_spawn_index);
        entity.model_slots = [Some(TYPE17_COMMON_DYING_MODEL_ID); 4];
        entity.collision.state_flags_at_0x08 =
            RetailStateWord::exact(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | 0x0000_4000);
        assert_eq!(
            entity.select_active_model_slot(1),
            Some(TYPE17_COMMON_DYING_MODEL_ID)
        );
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity.current_behavior_context =
            RetailRuntimeValue::Known(Some(common_dying_active_context_for_test()));
        entity.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(
            SubJAttachmentRuntime::from_descriptor(&sub_j_descriptor)
                .expect("the exact fixture descriptor has one valid slot"),
        ));

        let mut sub_a_target_speed_raw = i32::from(SUB_A.target_speed_base_raw);
        let mut sub_a_direction_raw = 1;
        let mut vertical_velocity_raw = 500;
        let prepared = CommonDyingTaskState::prepare_after_allocation(
            entity_id,
            &metadata,
            CommonDyingComponentDescriptors::default(),
        )
        .expect("the exact fixture topology admits Common-Dying")
        .map_task(ActorTaskRuntime::CommonDying)
        .apply_suffix(
            || 0,
            |effect| match effect {
                CommonDyingConstructorEffect::WriteSubADirection {
                    direction_multiplier,
                } => sub_a_direction_raw = direction_multiplier,
                CommonDyingConstructorEffect::WriteSubATargetSpeed {
                    target_speed_raw, ..
                } => sub_a_target_speed_raw = target_speed_raw,
                CommonDyingConstructorEffect::WriteOwnerVerticalVelocity { velocity_raw } => {
                    vertical_velocity_raw = velocity_raw
                }
                _ => {}
            },
        );
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(sub_a_target_speed_raw),
                sub_a_direction_raw,
                0,
            )));
        let mut velocity_raw = entity.velocity_raw();
        velocity_raw[1] = vertical_velocity_raw;
        entity.set_velocity_raw(velocity_raw);
        let mut sub_h_runtime = SubHRuntimeState::new(1).unwrap();
        sub_h_runtime.set_enabled(false);
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h_runtime));

        let task_id = entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, prepared);
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        };
        let owner =
            LevelOneType17CommonDyingOwner::from_entity_metadata(true, &entity, &metadata, visit)
                .expect("the composite fixture must publish an authenticated receipt");
        (entity, metadata, owner)
    }

    fn common_dying_active_context_for_test() -> BehaviorContextRuntime {
        let program = audited_behavior_program(COMMON_DYING_BEHAVIOR_CLASS_ID)
            .expect("class-12 is statically audited");
        let style_offset = COMMON_ACTOR_DYING_ACTIVE_STYLE
            .frame_address
            .checked_sub(program.style_table_base_address)
            .expect("the active style follows its table base");
        assert_eq!(style_offset % 0x48, 0);
        BehaviorContextRuntime::named_audited(
            program,
            style_offset / 0x48,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            COMMON_ACTOR_DYING_ACTIVE_STYLE,
        )
        .expect("the exact class-12 context is valid")
    }

    fn entity_with_exact_state(id: u32, state_flags: u32) -> Entity {
        let mut entity = Entity::unresolved_port_entity(id, EntityKind::Enemy, 9);
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(state_flags);
        entity
    }

    fn snapshot() -> PreflightSnapshot {
        let mut sub_h_runtime = SubHRuntimeState::new(1).unwrap();
        sub_h_runtime.set_enabled(false);
        sub_h_runtime.records_mut()[0].flags_raw = 0x1f;
        sub_h_runtime.records_mut()[0].phase_raw = 0x7fff;
        PreflightSnapshot {
            entity_id: 17,
            position_raw: [0, 0, 0],
            velocity_raw: [100, -50, 25],
            rotation_raw: [0; 3],
            state_flags: TERRAIN_ATTITUDE_BILINEAR_STATE_BIT,
            master_motion_state_flags: 0x0004_0000,
            terminal_transition_expected: false,
            attached_mass: 0,
            topology: LEVEL_ONE_TYPE17_COMMON_MOVER_TOPOLOGY,
            sub_a_descriptor: SUB_A,
            sub_b_descriptor: SUB_B,
            sub_c_descriptor: SUB_C,
            sub_h_descriptor: sub_h_descriptor(),
            sub_a_runtime: SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Known(250),
                1,
                100,
            ),
            sub_h_runtime,
        }
    }

    fn coarse_entity(position_y_raw: i16, timer_ms: RetailRuntimeValue<u32>) -> Entity {
        let mut entity =
            Entity::unresolved_port_entity(17, EntityKind::Enemy, TYPE17_COMMON_DYING_ENTITY_TYPE);
        entity.mass_raw = LEVEL_ONE_TYPE17_SELF_MASS_RAW;
        entity.position = crate::entity::raw_position_world([0, position_y_raw, 0]);
        entity.set_velocity_raw([2258, 0, 658]);
        entity.surface_lifetime_timer_ms_at_0x48 = timer_ms;
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
                | ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT
                | TERRAIN_ATTITUDE_BILINEAR_STATE_BIT
                | REMOTE_OWNED_STATE_BIT
                | 0x0000_4000
                | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | 0x0000_4000,
        );
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity
    }

    fn scheduler_prefix(callback_elapsed_micros: u32) -> PlannedCommonSchedulerPrefix {
        PlannedCommonSchedulerPrefix {
            flow: CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us: callback_elapsed_micros,
            },
            recent_relation_elapsed_us_at_0x68: callback_elapsed_micros,
            callback_scheduler_accumulator_us_at_0x6c: 0,
            subject_scan_gate_at_0x70: 0,
        }
    }

    fn empty_direct_attachment_preflight() -> PreflightedType17DirectAttachmentTail {
        PreflightedType17DirectAttachmentTail {
            retained_entity_ids: Vec::new(),
            children: Vec::new(),
        }
    }

    fn empty_scheduler_tail(entity: &Entity) -> PreflightedType17SchedulerTail {
        PreflightedType17SchedulerTail {
            relation_prelude: PlannedType17RelationPrelude::None,
            state_after_relation: entity.collision.state_flags_at_0x08,
            position_after_relation_raw: entity.position_raw(),
            attached_mass: 0,
            direct_attachments: empty_direct_attachment_preflight(),
        }
    }

    fn receipt(entity_id: u32, authored_spawn_index: usize) -> LevelOneType17CommonDyingOwner {
        let mut tasks = ActorTaskOwner::new();
        let task_id = tasks.replace_prepared(ActorTaskSlot::Primary, PreparedActorTask::new(()));
        LevelOneType17CommonDyingOwner::from_authenticated_publication(
            entity_id,
            authored_spawn_index,
            ActorTaskVisit {
                slot: ActorTaskSlot::Primary,
                task_id,
            },
        )
    }

    #[test]
    fn registration_deduplicates_by_entity_id_and_keeps_the_latest_receipt() {
        let first = receipt(17, 17);
        let replacement = receipt(17, 18);
        assert_ne!(first, replacement);
        let mut scheduler = Type17CommonDyingScheduler::new();

        scheduler.register(first);
        scheduler.register(replacement);

        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(scheduler.owners, [replacement]);
    }

    #[test]
    fn presentation_phase_publishes_next_tick_detail_only_for_admitted_receipts() {
        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(receipt(17, 17));
        let mut entity = coarse_entity(0, RetailRuntimeValue::Known(0));
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_view_detail::VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT
                | crate::entity_view_detail::VIEW_DETAIL_STATE_MASK,
            crate::entity_view_detail::VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT,
        );
        let mut manager = EntityManager::from_entities_for_test(vec![entity]);

        publish_type17_common_dying_owner_presented_view_detail(
            &mut manager,
            receipt(17, 17),
            RetailViewDetailContext::from_raw([0, 0, 0], 0, (52, 30)),
        );
        let entity = manager.common_actor_dying_entity_mut(17).unwrap();
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(crate::entity_view_detail::VIEW_DETAIL_STATE_MASK),
            RetailRuntimeValue::Known(crate::entity_view_detail::VIEW_DETAIL_STATE_MASK)
        );

        scheduler.publish_presented_view_detail(
            &mut manager,
            RetailViewDetailContext::from_raw([20_000, 0, 0], 0, (52, 30)),
        );
        let entity = manager.common_actor_dying_entity_mut(17).unwrap();
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(crate::entity_view_detail::VIEW_DETAIL_STATE_MASK),
            RetailRuntimeValue::Known(0)
        );

        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_view_detail::VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT
                | crate::entity_view_detail::VIEW_DETAIL_STATE_MASK,
            crate::entity_view_detail::VIEW_DETAIL_STATE_MASK,
        );
        scheduler.publish_presented_view_detail(
            &mut manager,
            RetailViewDetailContext::from_raw([20_000, 0, 0], 0, (52, 30)),
        );
        let entity = manager.common_actor_dying_entity_mut(17).unwrap();
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(crate::entity_view_detail::VIEW_DETAIL_STATE_MASK),
            RetailRuntimeValue::Known(crate::entity_view_detail::VIEW_DETAIL_STATE_MASK),
            "a clear 0x800 gate preserves the previous detail result"
        );
    }

    #[test]
    fn scheduler_reports_preflight_blocks_in_current_manager_live_order() {
        let terrain = flat_terrain(0);
        let mut scheduler = Type17CommonDyingScheduler::new();
        scheduler.register(receipt(1, 17));
        scheduler.register(receipt(2, 18));
        let mut manager = EntityManager::from_entities_for_test(vec![
            Entity::unresolved_port_entity(2, EntityKind::Enemy, 17),
            Entity::unresolved_port_entity(1, EntityKind::Enemy, 17),
        ]);

        let pass = scheduler.tick(
            &mut manager,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );

        assert_eq!(
            pass.outcomes,
            [
                Type17CommonDyingProductionOutcome::Blocked {
                    entity_id: 2,
                    reason: Type17CommonDyingProductionBlock::MissingTypeMetadata,
                },
                Type17CommonDyingProductionOutcome::Blocked {
                    entity_id: 1,
                    reason: Type17CommonDyingProductionBlock::MissingTypeMetadata,
                },
            ]
        );
        assert_eq!(scheduler.registered_len(), 2);
    }

    #[test]
    fn single_owner_tick_returns_blocked_receipt_for_outer_scheduler_retention() {
        let terrain = flat_terrain(0);
        let owner = receipt(17, 17);
        let mut manager =
            EntityManager::from_entities_for_test(vec![Entity::unresolved_port_entity(
                17,
                EntityKind::Enemy,
                17,
            )]);

        let tick = tick_type17_common_dying_owner(
            &mut manager,
            owner,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("missing metadata must block before shared RNG"),
        );

        assert_eq!(tick.retained_owner, Some(owner));
        assert_eq!(tick.surface_bubble, None);
        assert_eq!(
            tick.outcome,
            Type17CommonDyingProductionOutcome::Blocked {
                entity_id: 17,
                reason: Type17CommonDyingProductionBlock::MissingTypeMetadata,
            }
        );
    }

    #[test]
    fn single_owner_tick_drops_a_missing_allocation_without_rng() {
        let terrain = flat_terrain(0);
        let owner = receipt(17, 17);
        let mut manager = EntityManager::from_entities_for_test(Vec::new());

        let tick = tick_type17_common_dying_owner(
            &mut manager,
            owner,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("a missing allocation cannot consume shared RNG"),
        );

        assert_eq!(tick.retained_owner, None);
        assert_eq!(tick.surface_bubble, None);
        assert_eq!(
            tick.outcome,
            Type17CommonDyingProductionOutcome::Dropped {
                entity_id: 17,
                reason: Type17CommonDyingProductionDrop::EntityUnavailable,
            }
        );
    }

    #[test]
    fn composite_fixture_ticks_one_live_owner_without_family_scheduler_state() {
        let terrain = flat_terrain(0);
        let (entity, fixture_metadata, owner) =
            super::coarse_type17_common_dying_composite_fixture(17, 17);
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 18];
        metadata[TYPE17_COMMON_DYING_ENTITY_TYPE as usize] = fixture_metadata;
        let mut manager =
            EntityManager::from_entities_with_type_metadata_for_test(vec![entity], metadata, true);
        let mut samples = [0x0026, 0x1e27].into_iter();
        let mut draws = 0;

        let tick = tick_type17_common_dying_owner(
            &mut manager,
            owner,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                draws += 1;
                samples
                    .next()
                    .expect("the exact scheduler wait consumes two samples")
            },
        );

        assert_eq!(draws, 2);
        assert_eq!(tick.retained_owner, Some(owner));
        assert_eq!(tick.surface_bubble, None);
        assert_eq!(
            tick.outcome,
            Type17CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 17 }
        );
    }

    #[test]
    fn null_target_plan_matches_disabled_h_then_c_a_b_and_never_emits_a_cue() {
        let terrain = flat_terrain(0);
        let input = snapshot();
        let basis = HoverBasis::from_angle_words(0, 0, 0);
        let planned = plan_null_target_mover(&input, basis, &terrain, 20_000).unwrap();

        let mut expected_position = input.position_raw;
        let mut expected_velocity = input.velocity_raw;
        let point = sub_c_surface_sample_point_raw(
            expected_position,
            basis.up,
            HoverLiftConfig::from(SUB_C).offset_sample,
        );
        apply_sub_c_lift_raw(
            HoverLiftConfig::from(SUB_C),
            &mut expected_position,
            &mut expected_velocity,
            20_000,
            SubCSurfaceSample::Terrain {
                terrain_y_raw: terrain.bilinear_height_raw(point[0], point[1]),
            },
            basis.up,
        );
        assert!(apply_sub_a_propulsion_raw(
            SUB_A,
            250,
            1,
            100,
            basis.forward,
            &mut expected_velocity,
            20_000,
        ));
        apply_sub_b_lateral_raw(SUB_B, basis.lateral, &mut expected_velocity, 20_000);

        assert_eq!(planned.snapshot.position_raw, expected_position);
        assert_eq!(planned.snapshot.velocity_raw, expected_velocity);
        assert_eq!(planned.sub_h_runtime.cursor(), 0);
        assert!(!planned.sub_h_runtime.is_enabled());
        assert_eq!(planned.sub_h_runtime.records()[0].flags_raw, 1);
        assert_eq!(planned.sub_h_runtime.records()[0].phase_raw, 0x7fff);
    }

    #[test]
    fn enabled_sub_h_blocks_before_a_mover_plan_exists() {
        let terrain = flat_terrain(0);
        let mut input = snapshot();
        input.sub_h_runtime.set_enabled(true);
        let before = input.sub_h_runtime.clone();

        assert_eq!(
            plan_null_target_mover(
                &input,
                HoverBasis::from_angle_words(0, 0, 0),
                &terrain,
                20_000,
            )
            .unwrap_err(),
            Type17CommonDyingProductionBlock::SubHUnexpectedlyEnabled
        );
        assert_eq!(input.sub_h_runtime, before);
    }

    #[test]
    fn frame_plan_uses_pre_effect_basis_for_mover_forces() {
        let terrain = flat_terrain(0);
        let mut input = snapshot();
        input.rotation_raw = [0x1234, 0x0800, -0x1000];
        input.position_raw = [0x0180, 500, 0x0280];
        let old_basis = HoverBasis::from_angle_words(
            input.rotation_raw[0],
            input.rotation_raw[1],
            input.rotation_raw[2],
        );
        let expected_mover = plan_null_target_mover(&input, old_basis, &terrain, 20_000).unwrap();

        let plan = plan_exact_frame(
            input,
            Type17PostTaskOwnerSnapshot {
                entity_id: 17,
                self_mass_raw: LEVEL_ONE_TYPE17_SELF_MASS_RAW,
                surface_owner_disabled: false,
                surface_state_flags: 0x0000_4000,
                state_flags_at_0x08: RetailStateWord::exact(0x0000_4000),
                surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue::Known(125),
            },
            scheduler_prefix(20_000),
            PlannedType17RelationPrelude::None,
            empty_direct_attachment_preflight(),
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("the non-deep suffix must not consume RNG"),
        )
        .unwrap();
        assert_eq!(
            plan.mover_velocity_raw,
            expected_mover.snapshot.velocity_raw
        );
        assert_eq!(
            plan.live_commit.position_raw,
            expected_mover.snapshot.position_raw
        );
        assert_eq!(plan.selected_effect_raw, 0);
        let mut expected_suffix_velocity = expected_mover.snapshot.velocity_raw;
        expected_suffix_velocity[0] = damp_common_dying_axis(expected_suffix_velocity[0], 20_000);
        expected_suffix_velocity[2] = damp_common_dying_axis(expected_suffix_velocity[2], 20_000);
        apply_first_world_type17_environment_raw(
            &mut expected_suffix_velocity,
            20_000,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
        );
        assert_eq!(
            plan.post_task_suffix.velocity_after_environment_raw,
            expected_suffix_velocity
        );
        assert_eq!(plan.post_task_suffix.surface_timer_after_ms, Some(105));
        let mut expected_master_position = expected_mover.snapshot.position_raw;
        let mut expected_master_velocity = expected_suffix_velocity;
        let mut expected_master_state = 0x0004_0000;
        apply_common_master_motion_raw(
            &mut expected_master_position,
            &mut expected_master_velocity,
            &mut expected_master_state,
            20_000,
            0,
        );
        assert!(plan.master_motion.active);
        assert_eq!(
            plan.master_motion.position_after_raw,
            expected_master_position
        );
        assert_eq!(
            plan.master_motion.velocity_after_raw,
            expected_master_velocity
        );
        assert_eq!(plan.master_motion.state_write_mask, 0x20);
        assert_eq!(plan.master_motion.state_write_bits, 0x20);
        assert!(!plan.terminal_transition_expected);
    }

    #[test]
    fn detailed_surface_bubble_uses_the_post_attitude_forward_column() {
        let mut terrain = flat_terrain(0);
        terrain.header[0] = (0x7fff_i32) << 8;
        let mut input = snapshot();
        input.rotation_raw = [0x1234, 0x0800, -0x1000];
        input.master_motion_state_flags = 0;
        let pre_effect_forward = HoverBasis::from_angle_words(
            input.rotation_raw[0],
            input.rotation_raw[1],
            input.rotation_raw[2],
        )
        .forward;
        let mut samples = [0, 0, 0, 0, 0, 0].into_iter();
        let mut draws = 0;

        let plan = plan_exact_frame(
            input,
            Type17PostTaskOwnerSnapshot {
                entity_id: 17,
                self_mass_raw: LEVEL_ONE_TYPE17_SELF_MASS_RAW,
                surface_owner_disabled: false,
                surface_state_flags: 0x0000_4000,
                state_flags_at_0x08: RetailStateWord::exact(0x0000_4000),
                surface_lifetime_timer_ms_at_0x48: RetailRuntimeValue::Known(500),
            },
            scheduler_prefix(20_000),
            PlannedType17RelationPrelude::None,
            empty_direct_attachment_preflight(),
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || {
                draws += 1;
                samples.next().unwrap()
            },
        )
        .unwrap();

        assert_eq!(draws, 6);
        let post_effect_forward = HoverBasis::from_angle_words(
            0x1234,
            plan.live_commit.pitch_raw,
            plan.live_commit.roll_raw,
        )
        .forward;
        assert_eq!(
            plan.post_task_suffix.body_basis_q31,
            Type9BodyBasis::from_angle_words(
                0x1234,
                plan.live_commit.pitch_raw,
                plan.live_commit.roll_raw,
            ),
            "the detailed DCA0 suffix retains all nine post-attitude basis dwords"
        );
        assert_ne!(post_effect_forward, pre_effect_forward);
        let expected_position = std::array::from_fn(|axis| {
            plan.live_commit.position_raw[axis].wrapping_add(
                ((i64::from(post_effect_forward[axis])
                    * i64::from(LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW))
                    >> 31) as i16,
            )
        });
        assert_eq!(
            plan.post_task_suffix.surface_bubble,
            Some(ActorSurfaceBubbleRequest {
                position_raw: expected_position,
                velocity_argument_raw: [0; 3],
                owner_entity_id: 17,
                owner_entity_type: TYPE17_COMMON_DYING_ENTITY_TYPE as u8,
                suppresses_impact_damage: false,
            })
        );
    }

    #[test]
    fn terminal_detailed_plan_skips_outer_master_motion_after_state_clear() {
        let plan = plan_common_master_motion([100, 200, 300], [4_000, -2_000, 1_000], 0, 20_000);

        assert!(!plan.active);
        assert_eq!(plan.position_after_raw, [100, 200, 300]);
        assert_eq!(plan.velocity_after_raw, [4_000, -2_000, 1_000]);
        assert_eq!(plan.state_write_mask, 0);
    }

    #[test]
    fn detailed_scheduler_prefix_advances_known_carry_without_rng() {
        let mut entity = coarse_entity(0, RetailRuntimeValue::Known(0));
        entity.collision.state_flags_at_0x08.overwrite(
            TERRAIN_ATTITUDE_BILINEAR_STATE_BIT,
            TERRAIN_ATTITUDE_BILINEAR_STATE_BIT,
        );
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(10);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(120_000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(777);
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        let plan = plan_detailed_scheduler_prefix(&entity, 20_000).unwrap();
        assert_eq!(
            plan.flow,
            CommonSchedulerPrefixFlow::Continue {
                callback_elapsed_us: 125_000
            }
        );
        assert_eq!(plan.recent_relation_elapsed_us_at_0x68, 125_010);
        assert_eq!(plan.callback_scheduler_accumulator_us_at_0x6c, 15_000);
        assert_eq!(plan.subject_scan_gate_at_0x70, 0);
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(10)
        );

        assert_eq!(
            plan_detailed_scheduler_prefix(&entity, RETAIL_FRAME_DELTA_MAX_US + 1),
            Err(
                Type17CommonDyingProductionBlock::DetailedElapsedExceedsRetailCap {
                    actual: RETAIL_FRAME_DELTA_MAX_US + 1,
                }
            )
        );
    }

    #[test]
    fn callback_mass_wraps_the_animation_offset_and_promotes_zero() {
        let mut entity = coarse_entity(0, RetailRuntimeValue::Known(0));
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
        assert_eq!(
            plan_type17_callback_mass(&entity, LEVEL_ONE_TYPE17_SELF_MASS_RAW),
            Ok(177)
        );

        entity.collision.animation_offset_at_0xb2 =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE17_SELF_MASS_RAW.wrapping_neg());
        assert_eq!(
            plan_type17_callback_mass(&entity, LEVEL_ONE_TYPE17_SELF_MASS_RAW),
            Ok(1)
        );

        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            plan_type17_callback_mass(&entity, LEVEL_ONE_TYPE17_SELF_MASS_RAW),
            Err(Type17CommonDyingProductionBlock::UnresolvedAnimationOffset)
        );
    }

    #[test]
    fn coarse_scheduler_wait_commits_only_the_exact_randomized_prefix() {
        let terrain = flat_terrain(0);
        let mut entity = coarse_entity(0, RetailRuntimeValue::Known(125));
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
        let callback_mass_raw =
            plan_type17_callback_mass(&entity, LEVEL_ONE_TYPE17_SELF_MASS_RAW).unwrap();
        assert_eq!(callback_mass_raw, 177);
        let mut samples = [0x0026, 0x1e27].into_iter();
        let mut draws = 0;

        let plan = plan_coarse_production_frame(
            &entity,
            callback_mass_raw,
            empty_scheduler_tail(&entity),
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                draws += 1;
                samples.next().unwrap()
            },
        )
        .unwrap();
        let PlannedProductionFrame::SchedulerWait(prefix) = plan else {
            panic!("the fresh retail stream must wait at the callback gate")
        };
        assert_eq!(draws, 2);
        commit_common_scheduler_prefix(&mut entity.collision, prefix);
        assert_eq!(
            entity.collision.subject_scan_gate_at_0x70,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(1_000)
        );
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(7)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.mass_raw, LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            "a scheduler wait clears +0xB2 without rewriting callback mass"
        );
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(125),
            "a scheduler wait cannot enter E870/E370"
        );
    }

    #[test]
    fn coarse_scheduler_draws_precede_the_surface_random_gate() {
        let terrain = flat_terrain(0);
        let deep_y = terrain
            .sea_level_raw()
            .wrapping_sub((LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW >> 2) as i16)
            .wrapping_sub(1);
        let entity = coarse_entity(deep_y, RetailRuntimeValue::Known(500));
        let mut samples = [0xabcd, 0, 1].into_iter();
        let mut consumed = Vec::new();

        let plan = plan_coarse_production_frame(
            &entity,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            empty_scheduler_tail(&entity),
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || {
                let sample = samples.next().unwrap();
                consumed.push(sample);
                sample
            },
        )
        .unwrap();

        let PlannedProductionFrame::Coarse(plan) = plan else {
            panic!("a zero callback threshold must continue to E870")
        };
        assert_eq!(consumed, [0xabcd, 0, 1]);
        assert_eq!(plan.post_task_suffix.surface_timer_after_ms, Some(520));
        assert_eq!(plan.post_task_suffix.surface_bubble, None);
    }

    #[test]
    fn coarse_scheduler_and_suffix_blockers_consume_no_shared_rng() {
        let terrain = flat_terrain(0);
        let mut draws = 0;
        let unresolved_suffix = coarse_entity(0, RetailRuntimeValue::Unresolved);
        assert_eq!(
            plan_coarse_production_frame(
                &unresolved_suffix,
                LEVEL_ONE_TYPE17_SELF_MASS_RAW,
                empty_scheduler_tail(&unresolved_suffix),
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || {
                    draws += 1;
                    0
                },
            )
            .unwrap_err(),
            Type17CommonDyingProductionBlock::UnresolvedSurfaceLifetimeTimer
        );
        assert_eq!(draws, 0);

        let mut unresolved_scheduler = coarse_entity(0, RetailRuntimeValue::Known(125));
        unresolved_scheduler
            .collision
            .callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Unresolved;
        assert_eq!(
            plan_coarse_production_frame(
                &unresolved_scheduler,
                LEVEL_ONE_TYPE17_SELF_MASS_RAW,
                empty_scheduler_tail(&unresolved_scheduler),
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || {
                    draws += 1;
                    0
                },
            )
            .unwrap_err(),
            Type17CommonDyingProductionBlock::UnresolvedCoarseSchedulerState
        );
        assert_eq!(draws, 0);
    }

    #[test]
    fn coarse_plan_applies_exact_environment_and_non_deep_timer_decay() {
        let terrain = flat_terrain(0);
        let entity = coarse_entity(0, RetailRuntimeValue::Known(125));

        let plan = plan_coarse_frame(
            &entity,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();

        assert_eq!(plan.velocity_after_environment_raw, [2253, -28, 657]);
        assert_eq!(plan.surface_timer_after_ms, Some(105));
    }

    #[test]
    fn coarse_plan_admits_early_deep_timer_without_consuming_rng() {
        let terrain = flat_terrain(0);
        let deep_y = terrain
            .sea_level_raw()
            .wrapping_sub((LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW >> 2) as i16)
            .wrapping_sub(1);
        let entity = coarse_entity(deep_y, RetailRuntimeValue::Known(0));

        let plan = plan_coarse_frame(
            &entity,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();

        assert_eq!(plan.surface_timer_after_ms, Some(20));
    }

    #[test]
    fn coarse_plan_resolves_random_window_exact_lifecycle_and_overshoot() {
        let terrain = flat_terrain(0);
        let deep_y = terrain
            .sea_level_raw()
            .wrapping_sub((LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW >> 2) as i16)
            .wrapping_sub(1);
        let random_window = coarse_entity(deep_y, RetailRuntimeValue::Known(500));
        let preflight = plan_coarse_frame(
            &random_window,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        );
        let preflight = preflight.unwrap();
        assert_eq!(preflight.surface_timer_after_ms, Some(520));
        let random = preflight.surface_random.unwrap();
        assert_eq!(random.remaining_percent, 74);
        assert_eq!(random.frame.entity_id, 17);

        let mut miss_draws = 0;
        let miss = resolve_type17_post_task_suffix(preflight, &mut || {
            miss_draws += 1;
            1
        });
        assert_eq!(miss_draws, 1);
        assert_eq!(miss.surface_bubble, None);

        let preflight = plan_coarse_frame(
            &random_window,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();
        let mut samples = [0, 0x0200, 0x0400, 0x0600, 0x0080, 0x0100].into_iter();
        let mut hit_draws = 0;
        let hit = resolve_type17_post_task_suffix(preflight, &mut || {
            hit_draws += 1;
            samples.next().unwrap()
        });
        assert_eq!(hit_draws, 6);
        let frame = preflight.surface_random.unwrap().frame;
        let expected_position = std::array::from_fn(|axis| {
            frame.position_raw[axis]
                .wrapping_add(
                    ((i64::from(frame.emission_axis_q31[axis])
                        * i64::from(frame.active_model_extent_raw))
                        >> 31) as i16,
                )
                .wrapping_add([1, 2, 3][axis])
        });
        assert_eq!(
            hit.surface_bubble,
            Some(ActorSurfaceBubbleRequest {
                position_raw: expected_position,
                velocity_argument_raw: [1, 0, 2],
                owner_entity_id: 17,
                owner_entity_type: TYPE17_COMMON_DYING_ENTITY_TYPE as u8,
                suppresses_impact_damage: false,
            })
        );

        let below_lifecycle = coarse_entity(deep_y, RetailRuntimeValue::Known(1_979));
        let below = plan_coarse_frame(
            &below_lifecycle,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();
        assert_eq!(below.surface_timer_after_ms, Some(1_999));
        assert!(below.surface_lifecycle.is_none());
        assert_eq!(
            below.surface_random.unwrap().remaining_percent,
            0,
            "integer percentage reaches zero one millisecond before expiry without releasing"
        );

        let exact_lifecycle = coarse_entity(deep_y, RetailRuntimeValue::Known(1_980));
        let exact = plan_coarse_frame(
            &exact_lifecycle,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();
        assert_eq!(exact.surface_timer_after_ms, Some(2_000));
        assert!(exact.surface_lifecycle.is_some());
        assert_eq!(exact.surface_random.unwrap().remaining_percent, 0);
        let mut exact_draws = 0;
        let exact = resolve_type17_post_task_suffix(exact, &mut || {
            exact_draws += 1;
            1
        });
        assert_eq!(exact_draws, 1, "exact expiry enters the divisor-two gate");
        assert_eq!(exact.surface_bubble, None);

        let exact_hit = plan_coarse_frame(
            &exact_lifecycle,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();
        let mut exact_hit_draws = 0;
        let exact_hit = resolve_type17_post_task_suffix(exact_hit, &mut || {
            exact_hit_draws += 1;
            0
        });
        assert_eq!(
            exact_hit_draws, 6,
            "the lifecycle hit consumes the gate plus five packet words and no sound word"
        );
        assert!(exact_hit.surface_bubble.is_some());

        let overshoot = coarse_entity(deep_y, RetailRuntimeValue::Known(1_981));
        let overshoot = plan_coarse_frame(
            &overshoot,
            LEVEL_ONE_TYPE17_SELF_MASS_RAW,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();
        assert_eq!(overshoot.surface_timer_after_ms, Some(2_001));
        assert!(overshoot.surface_lifecycle.is_some());
        assert!(overshoot.surface_random.is_none());
        let mut overshoot_draws = 0;
        let overshoot = resolve_type17_post_task_suffix(overshoot, &mut || {
            overshoot_draws += 1;
            0
        });
        assert_eq!(overshoot_draws, 0);
        assert_eq!(overshoot.surface_bubble, None);
    }

    #[test]
    fn lifecycle_commit_overwrites_relation_and_type_copy_without_widening_unknown_state() {
        let mut entity = coarse_entity(0, RetailRuntimeValue::Known(2_000));
        let unrelated_unknown = 0x0040_0000;
        let state_before = RetailStateWord::from_known_bits(0x2041_7000, !unrelated_unknown);
        entity.collision.state_flags_at_0x08 = state_before;
        entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0xDEAD_BEEF);
        entity.attached_to = Some(0x04AA_00FF);
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        let position_before = entity.position_raw();
        let velocity_before = entity.velocity_raw();
        let active_before = entity.active;

        commit_type17_surface_lifecycle(
            &mut entity,
            PlannedType17SurfaceLifecycle {
                type_default_state_flags_at_0xc0: LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
            },
        );

        assert_eq!(
            entity.collision.state_flags_at_0x08,
            relation_release_state_word_after(
                state_before,
                LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW
            )
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.known_mask() & unrelated_unknown,
            0
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x2001_5800),
            RetailRuntimeValue::Known(0x0000_4800)
        );
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW)
        );
        assert_eq!(entity.attached_to, None);
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(entity.position_raw(), position_before);
        assert_eq!(entity.velocity_raw(), velocity_before);
        assert_eq!(entity.active, active_before);
    }

    #[test]
    fn planned_exact_and_overshoot_lifecycle_suffixes_commit_all_live_storage() {
        let terrain = flat_terrain(0);
        let deep_y = terrain
            .sea_level_raw()
            .wrapping_sub((LEVEL_ONE_TYPE17_MODEL_EXTENT_RAW >> 2) as i16)
            .wrapping_sub(1);

        for (timer_before_ms, timer_after_ms, expected_draws) in
            [(1_980, 2_000, 1), (1_981, 2_001, 0)]
        {
            let mut entity = coarse_entity(deep_y, RetailRuntimeValue::Known(timer_before_ms));
            entity.set_rotation_heading_pitch_roll_raw([0x1234, -0x0800, 0x0400]);
            let expected_basis = Type9BodyBasis::from_angle_words(0x1234, -0x0800, 0x0400);
            entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0xDEAD_BEEF);
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(0x0001_0000, 0x0001_0000);
            entity.attached_to = Some(0x04AA_00FF);
            let mut expected_state = entity.collision.state_flags_at_0x08;
            expected_state.overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
            expected_state = relation_release_state_word_after(
                expected_state,
                LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
            );

            let preflight = plan_coarse_frame(
                &entity,
                LEVEL_ONE_TYPE17_SELF_MASS_RAW,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
            )
            .unwrap();
            let mut draws = 0;
            let plan = resolve_type17_post_task_suffix(preflight, &mut || {
                draws += 1;
                1
            });
            commit_type17_post_task_suffix(&mut entity, plan);

            assert_eq!(draws, expected_draws);
            assert_eq!(
                entity.physical_body_basis_q31(),
                RetailRuntimeValue::Known(expected_basis),
                "E870 publishes the complete matrix before its completion bit"
            );
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(timer_after_ms)
            );
            assert_eq!(entity.collision.state_flags_at_0x08, expected_state);
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW)
            );
            assert_eq!(entity.attached_to, None);
        }
    }

    #[test]
    fn disabled_surface_owner_skips_timer_lifecycle_storage_and_rng() {
        let terrain = flat_terrain(0);
        let mut entity = coarse_entity(-10_000, RetailRuntimeValue::Unresolved);
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
            ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
        );
        entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0xDEAD_BEEF);
        entity.attached_to = Some(0x04AA_00FF);
        let state_before = entity.collision.state_flags_at_0x08;
        let owner = snapshot_type17_post_task_owner(&entity, LEVEL_ONE_TYPE17_SELF_MASS_RAW)
            .expect("authenticated model and disabled bit are resolved");
        let preflight = preflight_type17_post_task_suffix(
            owner,
            entity.position_raw(),
            entity.velocity_raw(),
            Type9BodyBasis::from_angle_words(0, 0, 0),
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 125_000,
            },
        )
        .unwrap();
        assert_eq!(preflight.surface_timer_after_ms, None);
        assert!(preflight.surface_lifecycle.is_none());
        assert!(preflight.surface_random.is_none());
        let plan = resolve_type17_post_task_suffix(preflight, &mut || {
            panic!("disabled E370 owner cannot consume shared RNG")
        });
        commit_type17_post_task_suffix(&mut entity, plan);
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0xDEAD_BEEF)
        );
        assert_eq!(entity.attached_to, Some(0x04AA_00FF));
        let mut expected_state = state_before;
        expected_state.overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        assert_eq!(
            entity.collision.state_flags_at_0x08, expected_state,
            "the ordinary basis-complete write is the only state edit when E370 is disabled"
        );
    }

    #[test]
    fn normal_owner_admits_known_relation_branch_and_coarse_plan_blocks_unknown_timer() {
        let terrain = flat_terrain(0);
        let mut suppressed = coarse_entity(0, RetailRuntimeValue::Known(0));
        suppressed.collision.state_flags_at_0x08.overwrite(
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
        );
        assert_eq!(preflight_normal_scheduler_owner(&suppressed), Ok(()));

        suppressed
            .collision
            .state_flags_at_0x08
            .invalidate(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT);
        assert_eq!(
            preflight_normal_scheduler_owner(&suppressed),
            Err(Type17CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState)
        );

        let unknown_timer = coarse_entity(0, RetailRuntimeValue::Unresolved);
        assert_eq!(
            plan_coarse_frame(
                &unknown_timer,
                LEVEL_ONE_TYPE17_SELF_MASS_RAW,
                Type17CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
            )
            .unwrap_err(),
            Type17CommonDyingProductionBlock::UnresolvedSurfaceLifetimeTimer
        );
    }

    #[test]
    fn relation_prelude_uses_parent_order_authority_and_preserves_remote_follow() {
        let terrain = flat_terrain(0);
        let descriptor = sub_j_descriptor(&[[0, 0, 0]]);
        let mut parent = entity_with_exact_state(5, 0);
        let mut parent_runtime = SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap();
        parent_runtime.append(17).unwrap();
        parent.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(parent_runtime));
        let mut subject = entity_with_exact_state(
            17,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
                | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
        );
        subject.attached_to = Some(5);
        let mut manager = EntityManager::from_entities_for_test(vec![subject, parent]);

        let subject = manager.iter_all().find(|entity| entity.id == 17).unwrap();
        assert_eq!(
            plan_type17_relation_prelude(
                &manager,
                subject,
                &terrain,
                LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
            ),
            Ok(PlannedType17RelationPrelude::RetainLocal {
                relation_owner_id: 5,
            })
        );

        let parent = manager.entity_mut_for_test(5).unwrap();
        let RetailRuntimeValue::Known(Some(runtime)) = &mut parent.sub_j_attachment_runtime else {
            panic!("fixture parent has an ordered Sub-J runtime")
        };
        runtime.clear();
        let subject = manager.iter_all().find(|entity| entity.id == 17).unwrap();
        let release = plan_type17_relation_prelude(
            &manager,
            subject,
            &terrain,
            LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
        )
        .unwrap();
        assert_eq!(
            release,
            PlannedType17RelationPrelude::ReleaseLocal {
                relation_owner_id: 5,
            },
            "the inverse backlink cannot replace the parent's ordered row"
        );
        commit_type17_relation_prelude(&mut manager, 17, release).unwrap();
        let subject = manager.iter_all().find(|entity| entity.id == 17).unwrap();
        assert_eq!(subject.attached_to, None);
        assert_eq!(
            subject
                .collision
                .state_flags_at_0x08
                .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            subject.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW)
        );

        let mut remote_parent = entity_with_exact_state(9, REMOTE_OWNED_STATE_BIT);
        remote_parent.set_motion_raw([321, -654, 987], [0; 3]);
        let mut remote_subject = entity_with_exact_state(
            17,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
                | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
        );
        remote_subject.attached_to = Some(9);
        let mut remote_manager =
            EntityManager::from_entities_for_test(vec![remote_subject, remote_parent]);
        let remote_subject = remote_manager
            .iter_all()
            .find(|entity| entity.id == 17)
            .unwrap();
        let follow = plan_type17_relation_prelude(
            &remote_manager,
            remote_subject,
            &terrain,
            LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
        )
        .unwrap();
        assert_eq!(
            follow,
            PlannedType17RelationPrelude::FollowRemote {
                relation_owner_id: 9,
                position_raw: [321, -654, 987],
            }
        );
        commit_type17_relation_prelude(&mut remote_manager, 17, follow).unwrap();
        let remote_subject = remote_manager
            .iter_all()
            .find(|entity| entity.id == 17)
            .unwrap();
        assert_eq!(remote_subject.position_raw(), [321, -654, 987]);
        assert_eq!(remote_subject.attached_to, Some(9));
        assert_eq!(
            remote_subject
                .collision
                .state_flags_at_0x08
                .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT | 0x20),
            RetailRuntimeValue::Known(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT | 0x20)
        );
    }

    #[test]
    fn missing_relation_owner_fails_closed_without_exact_type93_metadata() {
        let terrain = flat_terrain(0);
        let mut subject = entity_with_exact_state(
            17,
            ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT
                | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
        );
        subject.attached_to = Some(0x04aa_00ff);
        let manager = EntityManager::from_entities_for_test(vec![subject]);
        let subject = manager.iter_all().next().unwrap();

        assert_eq!(
            plan_type17_relation_prelude(
                &manager,
                subject,
                &terrain,
                LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
            ),
            Err(Type17CommonDyingProductionBlock::Type93Materialiser(
                Type17MissingRelationMaterialiserBlock::MissingType93Metadata,
            ))
        );
    }

    fn missing_relation_stamp_fixture() -> (EntityManager, LevelOneType17CommonDyingOwner) {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "canonical Type93 metadata required"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let mut metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let (mut entity, fixture_metadata, owner) =
            coarse_type17_common_dying_composite_fixture(17, 17);
        metadata[17] = fixture_metadata;
        entity.attached_to = Some(0x04aa_00ff);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x1000, 0x1000);
        (
            EntityManager::from_entities_with_type_metadata_for_test(vec![entity], metadata, true),
            owner,
        )
    }

    #[v2k_test_support::retail_test]
    fn missing_relation_preflight_and_wait_are_inert_then_attempt_precedes_publication() {
        for native_lineage in [false, true] {
            let terrain = flat_terrain(0);
            let (mut manager, owner) = missing_relation_stamp_fixture();
            if native_lineage {
                manager.set_common_body_stamp_counter_for_test(27);
            }
            let before = manager.next_common_body_ordinal();
            let frame = Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            };
            let prepared = preflight_production_frame(&manager, owner, frame)
                .unwrap_or_else(|_| panic!("exact missing-relation frame preflights"));
            let _second_preview = preflight_production_frame(&manager, owner, frame)
                .unwrap_or_else(|_| panic!("repeated immutable preflight"));
            assert_eq!(manager.next_common_body_ordinal(), before);
            assert_eq!(manager.iter_all().count(), 1);
            let mut draws = 0;
            let mut samples = [0x0026, 0x1e27].into_iter();
            let wait = resolve_production_frame(prepared, &mut manager, &mut || {
                draws += 1;
                samples
                    .next()
                    .expect("wait consumes only the two scheduler samples")
            });
            assert!(matches!(wait, PlannedProductionFrame::SchedulerWait(_)));
            assert_eq!(draws, 2);
            assert_eq!(manager.next_common_body_ordinal(), before);
            assert!(matches!(
                execute_preplanned_frame(owner, &mut manager, frame.elapsed_micros, wait),
                Ok(ExecutedProductionFrame::SchedulerWaiting)
            ));

            let frame = Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            };
            let prepared = preflight_production_frame(&manager, owner, frame)
                .unwrap_or_else(|_| panic!("wait continuation preflights"));
            assert_eq!(manager.next_common_body_ordinal(), before);
            let mut committed_draws = 0;
            let plan = resolve_production_frame(prepared, &mut manager, &mut || {
                committed_draws += 1;
                0
            });
            assert_eq!(
                committed_draws, 3,
                "scheduler pair, then singleton Type93 selector"
            );
            let expected_next = if native_lineage {
                RetailRuntimeValue::Known(1)
            } else {
                RetailRuntimeValue::Unresolved
            };
            assert_eq!(
                manager.next_common_body_ordinal(),
                expected_next,
                "body attempt is committed before delayed intrusive publication"
            );
            assert_eq!(manager.iter_all().count(), 1);
            let PlannedProductionFrame::Coarse(ref callback) = plan else {
                panic!("coarse continuation")
            };
            let attempted_relation = callback.relation_prelude;
            assert!(matches!(
                attempted_relation,
                PlannedType17RelationPrelude::MaterialiseMissing {
                    attempt: Some(_),
                    ..
                }
            ));
            let executed =
                execute_preplanned_frame(owner, &mut manager, frame.elapsed_micros, plan).unwrap();
            assert!(matches!(executed, ExecutedProductionFrame::Advanced { .. }));
            let proxy = manager
                .iter_all()
                .find(|entity| entity.entity_type == 93)
                .unwrap();
            let proxy_id = proxy.id;
            assert_eq!(
                proxy.construction_stamp_at_0xb4,
                if native_lineage {
                    RetailRuntimeValue::Known(0x6c00)
                } else {
                    RetailRuntimeValue::Unresolved
                }
            );
            assert_eq!(
                manager
                    .iter_all()
                    .find(|entity| entity.id == 17)
                    .unwrap()
                    .attached_to,
                Some(proxy_id)
            );
            assert_eq!(
                manager.next_common_body_ordinal(),
                expected_next,
                "publishing the attempted body cannot charge again"
            );
            assert!(
                commit_type17_relation_prelude(&mut manager, 17, attempted_relation).is_err(),
                "stale publication cannot replay its attempted body"
            );
            assert_eq!(manager.next_common_body_ordinal(), expected_next);

            let next = tick_type17_common_dying_owner(&mut manager, owner, frame, &mut || 0);
            assert!(matches!(
                next.outcome,
                Type17CommonDyingProductionOutcome::Advanced { .. }
            ));
            assert_eq!(
                manager
                    .iter_all()
                    .filter(|entity| entity.entity_type == 93)
                    .count(),
                1
            );
            assert_eq!(
                manager.next_common_body_ordinal(),
                expected_next,
                "retained parent membership has no new constructor on the next real visit"
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn blocked_missing_relation_callback_does_not_enter_body_attempt() {
        let terrain = flat_terrain(0);
        let (mut manager, owner) = missing_relation_stamp_fixture();
        manager.set_common_body_stamp_counter_for_test(2);
        manager
            .entity_mut_for_test(17)
            .unwrap()
            .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Unresolved;
        let outcome = tick_type17_common_dying_owner(
            &mut manager,
            owner,
            Type17CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("unclosed callback consumes neither selector nor scheduler RNG"),
        );
        assert!(matches!(
            outcome.outcome,
            Type17CommonDyingProductionOutcome::Blocked { .. }
        ));
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(manager.iter_all().count(), 1);
    }

    #[test]
    fn direct_tail_compacts_stably_reuses_ordinals_and_ignores_solo_backlinks() {
        let descriptor = sub_j_descriptor(&[
            [0, 10, 110],
            [300, 400, 500],
            [600, 700, 800],
            [900, 1_000, 1_100],
        ]);
        let mut owner = entity_with_exact_state(17, 1);
        owner.set_motion_raw([32_760, 100, -200], [3, 4, 5]);
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap();
        for child_id in [99, 2, 3, 4] {
            runtime.append(child_id).unwrap();
        }
        owner.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(runtime));
        let child_dying = entity_with_exact_state(2, DIRECT_ATTACHMENT_STALE_STATE_BIT);
        let mut child_offset = entity_with_exact_state(3, DIRECT_ATTACHMENT_LOCAL_OFFSET_STATE_BIT);
        child_offset.attached_to = Some(0xdead_beef);
        child_offset.set_motion_raw([1, 2, 3], [11, 22, 33]);
        let mut child_anchor = entity_with_exact_state(4, 1);
        child_anchor.set_motion_raw([4, 5, 6], [-11, -22, -33]);
        let mut manager = EntityManager::from_entities_for_test(vec![
            owner,
            child_dying,
            child_offset,
            child_anchor,
        ]);

        let preflight =
            preflight_type17_direct_attachments(&manager, 17, &[99, 2, 3, 4], &descriptor).unwrap();
        assert_eq!(preflight.retained_entity_ids, [3, 4]);
        assert_eq!(preflight.children[0].local_offset_raw, [0, 10, 110]);
        assert_eq!(
            preflight.children[1].local_offset_raw,
            [300, 400, 500],
            "both stale rows leave the first descriptor ordinal vacant"
        );
        let plan = resolve_type17_direct_attachment_tail(
            preflight,
            [32_760, 100, -200],
            Type9BodyBasis::from_angle_words(0, 0, 0),
        );
        assert_eq!(plan.updates[0].position_raw, [-32_667, 109, -200]);
        assert_eq!(plan.updates[1].position_raw, [32_760, 100, -200]);
        commit_type17_direct_attachment_tail(&mut manager, 17, plan).unwrap();

        let owner = manager.iter_all().find(|entity| entity.id == 17).unwrap();
        let RetailRuntimeValue::Known(Some(runtime)) = &owner.sub_j_attachment_runtime else {
            panic!("owner runtime remains live")
        };
        assert_eq!(runtime.ordered_entity_ids(), [3, 4]);
        let child_offset = manager.iter_all().find(|entity| entity.id == 3).unwrap();
        assert_eq!(child_offset.position_raw(), [-32_667, 109, -200]);
        assert_eq!(child_offset.velocity_raw(), [0; 3]);
        assert_eq!(
            child_offset.attached_to,
            Some(0xdead_beef),
            "DAT_004F741C is clear in this solo adapter, so backlinks are not reconciled"
        );
        let child_anchor = manager.iter_all().find(|entity| entity.id == 4).unwrap();
        assert_eq!(child_anchor.position_raw(), [32_760, 100, -200]);
        assert_eq!(child_anchor.velocity_raw(), [0; 3]);
    }

    #[test]
    fn direct_tail_state_decisions_fail_closed_but_accept_proven_partial_nonzero() {
        let descriptor = sub_j_descriptor(&[[0, 10, 110]]);
        let mut child = entity_with_exact_state(3, 0);
        child.collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            1,
            1 | DIRECT_ATTACHMENT_STALE_STATE_BIT | DIRECT_ATTACHMENT_LOCAL_OFFSET_STATE_BIT,
        );
        let manager =
            EntityManager::from_entities_for_test(vec![entity_with_exact_state(17, 1), child]);
        assert_eq!(
            preflight_type17_direct_attachments(&manager, 17, &[3], &descriptor)
                .unwrap()
                .retained_entity_ids,
            [3]
        );

        let mut unresolved_child = entity_with_exact_state(3, 0);
        unresolved_child.collision.state_flags_at_0x08 =
            RetailStateWord::from_known_bits(1, 1 | DIRECT_ATTACHMENT_LOCAL_OFFSET_STATE_BIT);
        let unresolved = EntityManager::from_entities_for_test(vec![
            entity_with_exact_state(17, 1),
            unresolved_child,
        ]);
        assert_eq!(
            preflight_type17_direct_attachments(&unresolved, 17, &[3], &descriptor),
            Err(
                Type17CommonDyingProductionBlock::UnresolvedDirectAttachmentState {
                    child_entity_id: 3,
                }
            )
        );
    }

    #[test]
    fn attached_child_mass_reads_original_rows_before_later_compaction() {
        let terrain = flat_terrain(0);
        let descriptor = sub_j_descriptor(&[[0, 10, 110]]);
        let mut owner = entity_with_exact_state(
            17,
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | 0x0000_4000,
        );
        let mut runtime = SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap();
        runtime.append(3).unwrap();
        owner.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(runtime));
        let mut stale_child = entity_with_exact_state(3, DIRECT_ATTACHMENT_STALE_STATE_BIT);
        stale_child.mass_raw = 77;
        let manager = EntityManager::from_entities_for_test(vec![owner, stale_child]);
        let owner = manager.iter_all().find(|entity| entity.id == 17).unwrap();

        let tail = preflight_type17_scheduler_tail(
            &manager,
            owner,
            &RetailRuntimeValue::Known(Some(descriptor)),
            LEVEL_ONE_TYPE17_DEFAULT_STATE_FLAGS_RAW,
            &terrain,
        )
        .unwrap();

        assert_eq!(tail.attached_mass, 77);
        assert!(tail.direct_attachments.retained_entity_ids.is_empty());
    }
}
