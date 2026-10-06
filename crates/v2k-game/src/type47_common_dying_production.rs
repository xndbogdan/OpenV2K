//! Exact Common-Dying scheduler owner for fresh Level-1 Type 47.
//!
//! The authenticated standard-death publisher and dying-bit C690 /
//! `FUN_00425660` class-12 install both return allocation/task receipts
//! for authored spawns 11, 12, and 13. This owner adopts those receipts,
//! visits them in current manager live-list order, and runs class 12 then
//! E870's `0x28` suffix through the later same-pass deferred-destroy
//! boundary. Live construction publishes `0x07068805` (both `FUN_00411400`
//! detail bits set). `FUN_00412DA0` then skips randomized waits; this owner
//! must still invoke class 12. A fail-closed "coarse-only" gate left killed
//! newants at health 0 with class-5 smoke and no Flip Over And Die. Type-17
//! recovers the separate detailed DCA0 mover; Type-47 keeps this E870
//! suffix until that fork is evidenced for model 302.
//!
//! All callback and suffix blockers are closed before process-shared RNG is
//! touched. The common scheduler therefore retains exact conditional `+0x70`
//! then `+0x6C` draw order, wait/carry/cap behavior, callback mass, and `+0xB2`
//! writes. A continuation invokes class 12 in named coarse mode 1, consumes its
//! `0x9C01` transition tag before timeout, publishes variant one, then still
//! runs E870's entry-snapshot `0x28` suffix: body-basis publication, E100,
//! E370, the authenticated empty Sub-J tail, and the outer `+0xB2` clear. The
//! terminal policy makes the final master-motion reread inactive. The entity
//! remains live until the manager's later sweep; this scheduler never unlinks
//! it inline.

use v2k_formats::collision::{SubJAttachmentDescriptor, SubJAttachmentSlotDescriptor};
use v2k_formats::terrain::TerrainGrid;

use crate::actor_death::COMMON_ACTOR_DYING_ACTIVE_STYLE;
use crate::actor_task_dispatcher::{ActorTaskRuntime, ActorTaskRuntimeFamily};
use crate::actor_task_owner::{
    ActorTaskSlot, ActorTaskVisit, ActorTaskVisitControl, ActorTaskWrapperFlags,
};
use crate::common_dying::{
    common_dying_after_unwind, CommonDyingAfterUnwindOutcome, CommonDyingCallbackPrefix,
    CommonDyingCallbackResult, CommonDyingFrameRequest, CommonDyingTaggedResult,
};
use crate::common_dying_live::stage_common_dying_terminal_transition;
use crate::common_mover::type9_surface::{
    classify_actor_surface_timer_phase, plan_actor_surface_bubble, ActorSurfaceBubbleFrame,
    ActorSurfaceBubbleRequest, ActorSurfaceTimerFrame, ActorSurfaceTimerPhase,
    ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
};
use crate::common_mover::type9_tail::{plan_common_master_motion, PlannedCommonMasterMotion};
use crate::entity::{
    apply_first_world_common_dying_environment_raw, commit_common_master_motion, Entity,
    EntityManager,
};
use crate::entity_behavior::{
    audited_behavior_program, ActiveBehaviorStyle, BehaviorDescriptorIdentity,
};
use crate::entity_collision_state::{
    RetailRuntimeValue, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
    DEFERRED_DESTROY_PENDING_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_relation_release::relation_release_state_word_after;
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    PlannedCommonSchedulerPrefix, COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
};
use crate::hover::{HoverBasis, RETAIL_FRAME_DELTA_MAX_US};
use crate::ordinary_type47_death_live::{
    FreshLevelOneType47CommonDyingOwner, TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
    TYPE47_COMMON_DYING_ENTITY_TYPE, TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW,
    TYPE47_COMMON_DYING_MASS_RAW,
};
use crate::ordinary_type47_live::{
    FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
};

/// Model 302 Section-8 header `+0x08`, distinct from collision radius 140.
pub const LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW: u16 = 182;
/// Type-47 Section-12 bytes `+0x72=1,+0x73=0,+0x74=2000`.
pub const LEVEL_ONE_TYPE47_SURFACE_LIFETIME_MS: u32 = 2_000;

const BODY_BASIS_REBUILT_STATE_BIT: u32 = 0x0000_0004;
const MASTER_MOTION_ENABLE_STATE_BIT: u32 = 0x0004_0000;

const LEVEL_ONE_TYPE47_SUB_J_SLOT: SubJAttachmentSlotDescriptor = SubJAttachmentSlotDescriptor {
    policy_word_raw: 1,
    local_offset_raw: [0; 3],
};

/// One capped outer-frame invocation.
#[derive(Debug, Clone, Copy)]
pub struct Type47CommonDyingProductionFrame<'a> {
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
}

/// Static/live evidence which blocks a frame before mutation or shared RNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47CommonDyingProductionBlock {
    MissingTypeMetadata,
    UnexpectedMetadataModelSlots {
        actual: [u16; 4],
    },
    UnexpectedMetadataMass {
        actual: u16,
    },
    UnexpectedMetadataTopology,
    MissingTypeInitializer,
    UnexpectedInitializerStateFlags {
        actual: u32,
    },
    UnexpectedAlternateBehaviorClass {
        actual: u32,
    },
    UnresolvedSubJDescriptor,
    MissingSubJDescriptor,
    UnexpectedSubJDescriptor,
    UnresolvedSubJRuntime,
    MissingSubJRuntime,
    UnexpectedSubJRuntimeShape {
        authored_slots: usize,
        capacity: usize,
        live_rows: usize,
        outer_policy_raw: u32,
    },
    ElapsedExceedsRetailCap {
        actual: u32,
    },
    UnresolvedNormalSchedulerOwnerState,
    RemoteSchedulerOwnerUnsupported,
    SchedulerCallbackDisabled,
    UnresolvedViewDetailState,
    UnresolvedRelationOwnerState,
    RelationOwnerUnsupported,
    UnexpectedRelationAttachment {
        actual: Option<u32>,
    },
    UnresolvedAnimationOffset,
    UnresolvedSchedulerState,
    UnresolvedSurfaceOwnerState,
    UnexpectedSurfaceModelState,
    UnresolvedSurfaceLifetimeTimer,
}

/// Why a receipt was stale and was discarded without RNG or mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47CommonDyingProductionDrop {
    EntityUnavailable,
    ReceiptMismatch,
    Owner(Type47CommonDyingOwnerError),
}

/// Dynamic allocation/task lease mismatch for an adopted receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47CommonDyingOwnerError {
    NotFreshNewGameFirstWorld,
    EntityInactive,
    UnauthenticatedSpawn,
    WrongEntityType,
    EntityModelMismatch,
    WrongActiveModelSlot,
    ActiveStyleMismatch,
    DeferredDestroyAlreadyPending,
    WrongTaskSlot,
    AdditionalPublishedTask { slot: ActorTaskSlot },
    TaskLeaseUnavailable { visit: ActorTaskVisit },
    TaskFamilyMismatch { visit: ActorTaskVisit },
    TaskWrapperNotRunnable { visit: ActorTaskVisit },
}

/// Evidence from one successful E870 mode-1 callback and terminal staging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47CommonDyingTerminalFrame {
    pub entity_id: u32,
    pub visit: ActorTaskVisit,
    pub committed_prefix: CommonDyingCallbackPrefix,
    pub tagged_result: CommonDyingTaggedResult,
    pub callback_mass_raw: u16,
}

/// One registered receipt's result in manager live-list order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47CommonDyingProductionOutcome {
    SchedulerWaiting {
        entity_id: u32,
    },
    DeferredDestroyStaged(Type47CommonDyingTerminalFrame),
    Blocked {
        entity_id: u32,
        reason: Type47CommonDyingProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Type47CommonDyingProductionDrop,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type47CommonDyingProductionPass {
    pub outcomes: Vec<Type47CommonDyingProductionOutcome>,
    /// E370 class-42 requests in the same order as their owner outcomes.
    pub surface_bubbles: Vec<ActorSurfaceBubbleRequest>,
}

/// One exact Type-47 receipt visit, detached from whole-manager traversal.
///
/// A future heterogeneous scheduler can invoke this boundary at the receipt's
/// current live-list position without giving this family ownership of any
/// other actor. Waiting and evidence-blocked visits retain the same linear
/// receipt; terminal and stale visits release it. Deferred entity cleanup
/// remains with the caller's later manager-wide sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Type47CommonDyingOwnerTick {
    pub(crate) outcome: Type47CommonDyingProductionOutcome,
    pub(crate) retained_owner: Option<FreshLevelOneType47CommonDyingOwner>,
    pub(crate) surface_bubble: Option<ActorSurfaceBubbleRequest>,
}

/// Tick exactly one authenticated Type-47 Common-Dying owner.
///
/// This primitive deliberately performs no live-list traversal and no
/// deferred sweep. The caller owns cross-family ordering and must visit a
/// retained receipt at most once per manager pass.
pub(crate) fn tick_type47_common_dying_owner(
    manager: &mut EntityManager,
    owner: FreshLevelOneType47CommonDyingOwner,
    frame: Type47CommonDyingProductionFrame<'_>,
    next_shared_random: &mut impl FnMut() -> u32,
) -> Type47CommonDyingOwnerTick {
    let entity_id = owner.entity_id();
    match plan_production_frame(manager, owner, frame, next_shared_random) {
        Ok(PlannedType47Frame::SchedulerWait(prefix)) => {
            let entity = manager
                .common_actor_dying_entity_mut(entity_id)
                .expect("a preflighted live allocation cannot disappear synchronously");
            commit_common_scheduler_prefix(&mut entity.collision, prefix);
            Type47CommonDyingOwnerTick {
                outcome: Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
                surface_bubble: None,
            }
        }
        Ok(PlannedType47Frame::Callback(plan)) => {
            let (terminal, surface_bubble) = execute_callback_frame(manager, owner, plan);
            Type47CommonDyingOwnerTick {
                outcome: Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal),
                retained_owner: None,
                surface_bubble,
            }
        }
        Err(PlanFailure::Blocked(reason)) => Type47CommonDyingOwnerTick {
            outcome: Type47CommonDyingProductionOutcome::Blocked { entity_id, reason },
            retained_owner: Some(owner),
            surface_bubble: None,
        },
        Err(PlanFailure::Dropped(reason)) => Type47CommonDyingOwnerTick {
            outcome: Type47CommonDyingProductionOutcome::Dropped { entity_id, reason },
            retained_owner: None,
            surface_bubble: None,
        },
    }
}

/// Receipt scheduler for the three authenticated fresh-Level-1 allocations.
#[derive(Debug, Default)]
pub struct Type47CommonDyingScheduler {
    owners: Vec<FreshLevelOneType47CommonDyingOwner>,
}

impl Type47CommonDyingScheduler {
    pub const fn new() -> Self {
        Self { owners: Vec::new() }
    }

    /// Register or replace the sole current receipt for an allocation.
    /// Publication order never determines tick order.
    pub fn register(&mut self, owner: FreshLevelOneType47CommonDyingOwner) {
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

    /// Tick receipts which existed at entry.
    ///
    /// Deferred entities remain allocated after this returns. The production
    /// manager owner must run `cleanup_pending_actor_deferred_destroys` at its
    /// established later boundary in the same pass.
    pub fn tick(
        &mut self,
        manager: &mut EntityManager,
        frame: Type47CommonDyingProductionFrame<'_>,
        next_shared_random: &mut impl FnMut() -> u32,
    ) -> Type47CommonDyingProductionPass {
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
            let tick = tick_type47_common_dying_owner(manager, owner, frame, next_shared_random);
            if let Some(owner) = tick.retained_owner {
                retained.push(owner);
            }
            if let Some(bubble) = tick.surface_bubble {
                surface_bubbles.push(bubble);
            }
            outcomes.push(tick.outcome);
        }

        outcomes.extend(pending.into_iter().map(|owner| {
            Type47CommonDyingProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Type47CommonDyingProductionDrop::EntityUnavailable,
            }
        }));
        self.owners = retained;
        Type47CommonDyingProductionPass {
            outcomes,
            surface_bubbles,
        }
    }
}

enum PlanFailure {
    Blocked(Type47CommonDyingProductionBlock),
    Dropped(Type47CommonDyingProductionDrop),
}

impl From<Type47CommonDyingProductionBlock> for PlanFailure {
    fn from(value: Type47CommonDyingProductionBlock) -> Self {
        Self::Blocked(value)
    }
}

enum PlannedType47Frame {
    SchedulerWait(PlannedCommonSchedulerPrefix),
    Callback(PlannedType47Callback),
}

struct PlannedType47Callback {
    scheduler_prefix: PlannedCommonSchedulerPrefix,
    callback_elapsed_us: u32,
    callback_mass_raw: u16,
    suffix: PlannedType47Suffix,
    master_motion: PlannedCommonMasterMotion,
}

#[derive(Debug, Clone, Copy)]
struct PlannedType47Suffix {
    velocity_after_environment_raw: [i16; 3],
    surface_timer_after_ms: Option<u32>,
    run_surface_lifecycle: bool,
    surface_bubble: Option<ActorSurfaceBubbleRequest>,
}

#[derive(Debug, Clone, Copy)]
struct PreflightedType47Suffix {
    velocity_after_environment_raw: [i16; 3],
    surface_timer_after_ms: Option<u32>,
    run_surface_lifecycle: bool,
    surface_random: Option<PreflightedType47SurfaceRandom>,
}

#[derive(Debug, Clone, Copy)]
struct PreflightedType47SurfaceRandom {
    remaining_percent: u32,
    frame: ActorSurfaceBubbleFrame,
}

fn plan_production_frame(
    manager: &EntityManager,
    owner: FreshLevelOneType47CommonDyingOwner,
    frame: Type47CommonDyingProductionFrame<'_>,
    next_shared_random: &mut impl FnMut() -> u32,
) -> Result<PlannedType47Frame, PlanFailure> {
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .ok_or(PlanFailure::Dropped(
            Type47CommonDyingProductionDrop::EntityUnavailable,
        ))?;
    let metadata = manager
        .type_runtime_metadata(TYPE47_COMMON_DYING_ENTITY_TYPE)
        .ok_or(Type47CommonDyingProductionBlock::MissingTypeMetadata)?;
    authenticate_type47_profile(metadata)?;
    authenticate_owner(manager, entity, owner)
        .map_err(|reason| PlanFailure::Dropped(Type47CommonDyingProductionDrop::Owner(reason)))?;
    preflight_empty_sub_j(entity, &metadata.sub_j_attachment_descriptor)?;
    preflight_outer_owner(entity)?;
    if frame.elapsed_micros > RETAIL_FRAME_DELTA_MAX_US {
        return Err(Type47CommonDyingProductionBlock::ElapsedExceedsRetailCap {
            actual: frame.elapsed_micros,
        }
        .into());
    }
    let RetailRuntimeValue::Known(callback_mass_raw) = common_scheduler_callback_mass(
        TYPE47_COMMON_DYING_MASS_RAW,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return Err(Type47CommonDyingProductionBlock::UnresolvedAnimationOffset.into());
    };

    // Threshold samples cannot change the callback delta, only whether the
    // callback runs. A zero-threshold preview therefore closes every E870
    // suffix blocker before the process stream is touched.
    let RetailRuntimeValue::Known(preview) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || 0)
    else {
        return Err(Type47CommonDyingProductionBlock::UnresolvedSchedulerState.into());
    };
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = preview.flow
    else {
        unreachable!("a zero-threshold scheduler preview cannot wait")
    };
    let suffix = preflight_type47_suffix(
        entity,
        frame.terrain,
        callback_elapsed_us,
        callback_mass_raw,
    )?;

    let RetailRuntimeValue::Known(scheduler_prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, next_shared_random)
    else {
        return Err(Type47CommonDyingProductionBlock::UnresolvedSchedulerState.into());
    };
    match scheduler_prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            Ok(PlannedType47Frame::SchedulerWait(scheduler_prefix))
        }
        CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us: actual_callback_elapsed_us,
        } => {
            debug_assert_eq!(actual_callback_elapsed_us, callback_elapsed_us);
            let master_motion = plan_common_master_motion(
                entity.position_raw(),
                suffix.velocity_after_environment_raw,
                0,
                callback_elapsed_us,
            );
            Ok(PlannedType47Frame::Callback(PlannedType47Callback {
                scheduler_prefix,
                callback_elapsed_us,
                callback_mass_raw,
                suffix: resolve_type47_suffix(suffix, next_shared_random),
                master_motion,
            }))
        }
    }
}

fn authenticate_type47_profile(
    metadata: &crate::entity_collision_state::EntityTypeRuntimeMetadata,
) -> Result<(), Type47CommonDyingProductionBlock> {
    let expected_models = [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4];
    if metadata.model_slots != expected_models {
        return Err(
            Type47CommonDyingProductionBlock::UnexpectedMetadataModelSlots {
                actual: metadata.model_slots,
            },
        );
    }
    if metadata.mass_raw != TYPE47_COMMON_DYING_MASS_RAW {
        return Err(Type47CommonDyingProductionBlock::UnexpectedMetadataMass {
            actual: metadata.mass_raw,
        });
    }
    if metadata.common_mover_topology
        != RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY)
    {
        return Err(Type47CommonDyingProductionBlock::UnexpectedMetadataTopology);
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(Type47CommonDyingProductionBlock::MissingTypeInitializer)?;
    if initializer.initializer_state_flags_raw != TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW {
        return Err(
            Type47CommonDyingProductionBlock::UnexpectedInitializerStateFlags {
                actual: initializer.initializer_state_flags_raw,
            },
        );
    }
    if initializer.alternate_behavior_class_ref != TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID {
        return Err(
            Type47CommonDyingProductionBlock::UnexpectedAlternateBehaviorClass {
                actual: initializer.alternate_behavior_class_ref,
            },
        );
    }
    Ok(())
}

fn authenticate_owner(
    manager: &EntityManager,
    entity: &Entity,
    owner: FreshLevelOneType47CommonDyingOwner,
) -> Result<(), Type47CommonDyingOwnerError> {
    if !manager.is_fresh_new_game_first_world()
        && !crate::intro2_type47_live::intro2_type47_cohort_runtime_authenticates(entity)
    {
        return Err(Type47CommonDyingOwnerError::NotFreshNewGameFirstWorld);
    }
    if !entity.active {
        return Err(Type47CommonDyingOwnerError::EntityInactive);
    }
    if entity.authored_spawn_index != Some(owner.authored_spawn_index())
        || crate::type47_initial_behavior_live::live_type47_cohort(entity).is_none()
    {
        return Err(Type47CommonDyingOwnerError::UnauthenticatedSpawn);
    }
    if entity.entity_type != TYPE47_COMMON_DYING_ENTITY_TYPE {
        return Err(Type47CommonDyingOwnerError::WrongEntityType);
    }
    let expected_model = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
    if entity.model_slots != [expected_model; 4] || entity.model_index != expected_model {
        return Err(Type47CommonDyingOwnerError::EntityModelMismatch);
    }
    if !matches!(
        entity.collision.active_model_slot(),
        RetailRuntimeValue::Known(1) | RetailRuntimeValue::Known(3)
    ) {
        return Err(Type47CommonDyingOwnerError::WrongActiveModelSlot);
    }
    // Health 0 is the generic death prefix, not class-12 scheduler
    // admission. C690 / FUN_00425660 installs class 12 without that prefix.
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Type47CommonDyingOwnerError::ActiveStyleMismatch);
    };
    let program = audited_behavior_program(TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID)
        .expect("class-12 behavior is statically catalogued");
    if context.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || context.active_style() != ActiveBehaviorStyle::Audited(COMMON_ACTOR_DYING_ACTIVE_STYLE)
    {
        return Err(Type47CommonDyingOwnerError::ActiveStyleMismatch);
    }
    if entity
        .collision
        .state_flags_at_0x08
        .masked(DEFERRED_DESTROY_PENDING_STATE_BIT)
        != RetailRuntimeValue::Known(0)
    {
        return Err(Type47CommonDyingOwnerError::DeferredDestroyAlreadyPending);
    }
    let visit = owner.visit();
    if visit.slot != ActorTaskSlot::Primary {
        return Err(Type47CommonDyingOwnerError::WrongTaskSlot);
    }
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        if entity.actor_tasks.task_in_slot(slot).is_some() {
            return Err(Type47CommonDyingOwnerError::AdditionalPublishedTask { slot });
        }
    }
    if entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(visit.task_id) {
        return Err(Type47CommonDyingOwnerError::TaskLeaseUnavailable { visit });
    }
    match entity.actor_tasks.task_state(visit.task_id) {
        Some(state) if state.family() == ActorTaskRuntimeFamily::CommonDying => {}
        Some(_) => return Err(Type47CommonDyingOwnerError::TaskFamilyMismatch { visit }),
        None => return Err(Type47CommonDyingOwnerError::TaskLeaseUnavailable { visit }),
    }
    if entity.actor_tasks.wrapper_flags(visit.task_id)
        != Some(ActorTaskWrapperFlags {
            alive: true,
            in_callback: false,
        })
    {
        return Err(Type47CommonDyingOwnerError::TaskWrapperNotRunnable { visit });
    }
    Ok(())
}

fn preflight_outer_owner(entity: &Entity) -> Result<(), Type47CommonDyingProductionBlock> {
    match entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) if bits & REMOTE_OWNED_STATE_BIT != 0 => {
            return Err(Type47CommonDyingProductionBlock::RemoteSchedulerOwnerUnsupported)
        }
        RetailRuntimeValue::Known(bits)
            if bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 =>
        {
            return Err(Type47CommonDyingProductionBlock::SchedulerCallbackDisabled)
        }
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingProductionBlock::UnresolvedNormalSchedulerOwnerState)
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
    {
        // Set: FUN_00412DA0 skips +0x70/+0x6C waits (live Type-47 birth
        // `0x07068805`). Clear: coarse waits. Both still run class 12.
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingProductionBlock::UnresolvedViewDetailState)
        }
    }
    match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => {
            return Err(Type47CommonDyingProductionBlock::RelationOwnerUnsupported)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingProductionBlock::UnresolvedRelationOwnerState)
        }
    }
    if entity.attached_to.is_some() {
        return Err(
            Type47CommonDyingProductionBlock::UnexpectedRelationAttachment {
                actual: entity.attached_to,
            },
        );
    }
    Ok(())
}

fn preflight_empty_sub_j(
    entity: &Entity,
    descriptor: &RetailRuntimeValue<Option<SubJAttachmentDescriptor>>,
) -> Result<(), Type47CommonDyingProductionBlock> {
    match descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.reserved_at_0x01 == 0
                && descriptor.slots.as_ref() == [LEVEL_ONE_TYPE47_SUB_J_SLOT] => {}
        RetailRuntimeValue::Known(Some(_)) => {
            return Err(Type47CommonDyingProductionBlock::UnexpectedSubJDescriptor)
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingProductionBlock::MissingSubJDescriptor)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingProductionBlock::UnresolvedSubJDescriptor)
        }
    }
    let runtime = match &entity.sub_j_attachment_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => runtime,
        RetailRuntimeValue::Known(None) => {
            return Err(Type47CommonDyingProductionBlock::MissingSubJRuntime)
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingProductionBlock::UnresolvedSubJRuntime)
        }
    };
    if runtime.authored_slot_count() != 1
        || runtime.capacity() != 1
        || !runtime.is_empty()
        || runtime.policy_raw_at_0x0c() != 0
    {
        return Err(
            Type47CommonDyingProductionBlock::UnexpectedSubJRuntimeShape {
                authored_slots: runtime.authored_slot_count(),
                capacity: runtime.capacity(),
                live_rows: runtime.len(),
                outer_policy_raw: runtime.policy_raw_at_0x0c(),
            },
        );
    }
    Ok(())
}

fn preflight_type47_suffix(
    entity: &Entity,
    terrain: &TerrainGrid,
    elapsed_micros: u32,
    callback_mass_raw: u16,
) -> Result<PreflightedType47Suffix, Type47CommonDyingProductionBlock> {
    let surface_state = match entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT | REMOTE_OWNED_STATE_BIT | 0x0000_4000)
    {
        RetailRuntimeValue::Known(bits) => bits,
        RetailRuntimeValue::Unresolved => {
            return Err(Type47CommonDyingProductionBlock::UnresolvedSurfaceOwnerState)
        }
    };
    if surface_state & 0x0000_4000 == 0 {
        return Err(Type47CommonDyingProductionBlock::UnexpectedSurfaceModelState);
    }
    let position_raw = entity.position_raw();
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    let emission_axis_q31 = HoverBasis::from_angle_words(heading_raw, pitch_raw, roll_raw).forward;
    let (surface_timer_after_ms, run_surface_lifecycle, surface_random) =
        if surface_state & ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT != 0 {
            (None, false, None)
        } else {
            let RetailRuntimeValue::Known(timer_ms_at_0x48) =
                entity.surface_lifetime_timer_ms_at_0x48
            else {
                return Err(Type47CommonDyingProductionBlock::UnresolvedSurfaceLifetimeTimer);
            };
            match classify_actor_surface_timer_phase(
                timer_ms_at_0x48,
                ActorSurfaceTimerFrame {
                    state_flags: 0,
                    position_y_raw: position_raw[1],
                    active_model_extent_raw: LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW,
                    flat_surface_y_raw: terrain.sea_level_raw(),
                    elapsed_us: elapsed_micros,
                    authored_lifetime_ms: LEVEL_ONE_TYPE47_SURFACE_LIFETIME_MS,
                },
            )
            .expect("the exact Type-47 authored lifetime is nonzero")
            {
                ActorSurfaceTimerPhase::NonDeep { timer_after_ms }
                | ActorSurfaceTimerPhase::DeepBeforeRandomEffects { timer_after_ms, .. } => {
                    (Some(timer_after_ms), false, None)
                }
                ActorSurfaceTimerPhase::DeepRandomEffects {
                    timer_after_ms,
                    remaining_percent,
                } => (
                    Some(timer_after_ms),
                    false,
                    Some(PreflightedType47SurfaceRandom {
                        remaining_percent,
                        frame: ActorSurfaceBubbleFrame {
                            entity_id: entity.id,
                            entity_type: TYPE47_COMMON_DYING_ENTITY_TYPE as u8,
                            state_flags: surface_state,
                            position_raw,
                            emission_axis_q31,
                            active_model_extent_raw: LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW,
                        },
                    }),
                ),
                ActorSurfaceTimerPhase::DeepLifecycle {
                    timer_after_ms,
                    remaining_percent_after_lifecycle,
                } => (
                    Some(timer_after_ms),
                    true,
                    ((remaining_percent_after_lifecycle as i32) <= 74).then_some(
                        PreflightedType47SurfaceRandom {
                            remaining_percent: remaining_percent_after_lifecycle,
                            frame: ActorSurfaceBubbleFrame {
                                entity_id: entity.id,
                                entity_type: TYPE47_COMMON_DYING_ENTITY_TYPE as u8,
                                state_flags: surface_state,
                                position_raw,
                                emission_axis_q31,
                                active_model_extent_raw: LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW,
                            },
                        },
                    ),
                ),
                ActorSurfaceTimerPhase::OwnerDisabled => {
                    unreachable!("the owner-disabled branch was handled before classification")
                }
            }
        };

    let mut velocity_after_environment_raw = entity.velocity_raw();
    // Effective flags are the E870 entry snapshot (0x28). The task transition
    // has not happened during preflight and the executor never recomputes them
    // from completion style 0x004C7F18.
    apply_first_world_common_dying_environment_raw(
        &mut velocity_after_environment_raw,
        elapsed_micros,
        callback_mass_raw,
    );
    Ok(PreflightedType47Suffix {
        velocity_after_environment_raw,
        surface_timer_after_ms,
        run_surface_lifecycle,
        surface_random,
    })
}

fn resolve_type47_suffix(
    preflight: PreflightedType47Suffix,
    next_shared_random: &mut impl FnMut() -> u32,
) -> PlannedType47Suffix {
    PlannedType47Suffix {
        velocity_after_environment_raw: preflight.velocity_after_environment_raw,
        surface_timer_after_ms: preflight.surface_timer_after_ms,
        run_surface_lifecycle: preflight.run_surface_lifecycle,
        surface_bubble: preflight.surface_random.and_then(|random| {
            plan_actor_surface_bubble(random.remaining_percent, random.frame, next_shared_random)
        }),
    }
}

fn execute_callback_frame(
    manager: &mut EntityManager,
    owner: FreshLevelOneType47CommonDyingOwner,
    plan: PlannedType47Callback,
) -> (
    Type47CommonDyingTerminalFrame,
    Option<ActorSurfaceBubbleRequest>,
) {
    let PlannedType47Callback {
        scheduler_prefix,
        callback_elapsed_us,
        callback_mass_raw,
        suffix,
        master_motion,
    } = plan;
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .expect("the preflighted allocation remains live");
        commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
        entity.mass_raw = callback_mass_raw;
    }
    let (committed_prefix, tagged_result) = {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .expect("the preflighted allocation remains live");
        tick_type47_coarse_task(entity, owner.visit(), callback_elapsed_us)
    };
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .expect("the preflighted allocation remains live");
        stage_common_dying_terminal_transition(entity);
    }
    manager.queue_actor_deferred_destroy(owner.entity_id());

    let surface_bubble = suffix.surface_bubble;
    {
        let entity = manager
            .common_actor_dying_entity_mut(owner.entity_id())
            .expect("the staged allocation survives until the later sweep");
        // FUN_00413F70 publishes the basis-complete bit. The basis itself is
        // derived from retained angle words rather than cached on Entity.
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        entity.set_velocity_raw(suffix.velocity_after_environment_raw);
        if let Some(timer_after_ms) = suffix.surface_timer_after_ms {
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(timer_after_ms);
        }
        if suffix.run_surface_lifecycle {
            commit_type47_surface_lifecycle(entity);
        }
        // The exact Type-47 Sub-J runtime was preflighted empty, so the
        // callback-free updater has no rows to mutate here.
        commit_common_scheduler_post_callback(&mut entity.collision);
        commit_common_master_motion(entity, master_motion);
        debug_assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(MASTER_MOTION_ENABLE_STATE_BIT),
            RetailRuntimeValue::Known(0),
            "the terminal plan must preserve the disabled master-motion gate"
        );
    }
    (
        Type47CommonDyingTerminalFrame {
            entity_id: owner.entity_id(),
            visit: owner.visit(),
            committed_prefix,
            tagged_result,
            callback_mass_raw,
        },
        surface_bubble,
    )
}

fn tick_type47_coarse_task(
    entity: &mut Entity,
    expected_visit: ActorTaskVisit,
    elapsed_micros: u32,
) -> (CommonDyingCallbackPrefix, CommonDyingTaggedResult) {
    let owner_entity_id = entity.id;
    let linear_velocity_raw = entity.velocity_raw();
    entity
        .actor_tasks
        .visit_slots_fresh_phased(
            |state, visit| {
                debug_assert_eq!(visit, expected_visit);
                let ActorTaskRuntime::CommonDying(state) = state else {
                    unreachable!("the receipt's Common-Dying family was prevalidated")
                };
                state.before_callback(elapsed_micros)
            },
            |_tasks, visit| {
                debug_assert_eq!(visit, expected_visit);
                crate::common_dying::tick_common_dying(
                    CommonDyingFrameRequest {
                        task_wrapper: visit,
                        owner_entity_id,
                        type_runtime: (),
                        component_runtime: (),
                        elapsed_micros,
                        scheduler_mode: 1,
                        linear_velocity_raw,
                    },
                    || unreachable!("mode 1 returns before effect resolution"),
                    |_| unreachable!("mode 1 returns before effect invocation"),
                    |_, _| -> () { unreachable!("mode 1 returns before the common mover") },
                )
                .expect("mode 1 has no fallible callback evidence")
            },
            |_tasks, visit, prefix, resolution| {
                debug_assert_eq!(visit, expected_visit);
                debug_assert_eq!(resolution.linear_velocity_raw, linear_velocity_raw);
                debug_assert_eq!(resolution.selected_effect_raw, None);
                let CommonDyingCallbackResult::TaggedOwnerTransition(tagged_result) =
                    resolution.callback_result
                else {
                    unreachable!("mode 1 always returns the class-12 transition tag")
                };
                debug_assert!(matches!(
                    common_dying_after_unwind(prefix, resolution.callback_result),
                    CommonDyingAfterUnwindOutcome::RequestOwnerTransition { .. }
                ));
                ActorTaskVisitControl::Propagate((prefix, tagged_result))
            },
        )
        .expect("the prevalidated primary visit must execute exactly once")
}

fn commit_type47_surface_lifecycle(entity: &mut Entity) {
    // The class-12 release hook is null. The preserved dying/model bit 0x4000
    // makes the following generic standard-death call an exact no-op.
    entity.collision.state_flags_at_0x08 = relation_release_state_word_after(
        entity.collision.state_flags_at_0x08,
        TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW,
    );
    entity.collision.default_state_flags_at_0xc8 =
        RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW);
    entity.attached_to = None;
}

#[cfg(test)]
pub(crate) use tests::exact_type47_single_owner_fixture;

#[cfg(test)]
mod tests {

    use super::*;
    use v2k_formats::collision::SubAPropulsionDescriptor;
    use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

    use crate::actor_task_owner::ActorTaskSlot;
    use crate::common_dying::{
        CommonDyingComponentDescriptors, CommonDyingTaskState, COMMON_DYING_OWNER_TRANSITION_TAG,
        COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
    };
    use crate::entity::{EntityKind, EntityManager};
    use crate::entity_behavior::{BehaviorChoiceListSource, BehaviorContextRuntime};
    use crate::entity_collision_state::{
        EntityInitializerSpec, EntityTypeRuntimeMetadata, RetailStateWord,
    };
    use crate::session::GameSession;
    use crate::sub_j_attachment::SubJAttachmentRuntime;

    const STATE_SLOT1: u32 = 0x0143_C805;
    const STATE_SLOT3: u32 = 0x0143_E805;
    const TERMINAL_SLOT1: u32 = 0x0151_C805;
    const TERMINAL_SLOT3: u32 = 0x0151_E805;

    fn terrain(sea_level_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_level_raw) << 8, 0, 0, 0, 0],
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

    fn exact_metadata() -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            model_slots: [FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID as u16; 4],
            mass_raw: TYPE47_COMMON_DYING_MASS_RAW,
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(Some(
                SubAPropulsionDescriptor {
                    acceleration_raw: 1_500,
                    overspeed_correction_raw: -3_000,
                    target_speed_base_raw: 300,
                },
            )),
            sub_j_attachment_descriptor: RetailRuntimeValue::Known(Some(
                SubJAttachmentDescriptor {
                    reserved_at_0x01: 0,
                    slots: vec![LEVEL_ONE_TYPE47_SUB_J_SLOT].into_boxed_slice(),
                },
            )),
            common_mover_topology: RetailRuntimeValue::Known(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            ),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW,
                common_axis_descriptor: Default::default(),
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID,
            }),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn active_context() -> BehaviorContextRuntime {
        let program = audited_behavior_program(TYPE47_COMMON_DYING_BEHAVIOR_CLASS_ID).unwrap();
        BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            COMMON_ACTOR_DYING_ACTIVE_STYLE,
        )
        .unwrap()
    }

    fn exact_entity(
        entity_id: u32,
        spawn: usize,
        state_flags: u32,
    ) -> (Entity, FreshLevelOneType47CommonDyingOwner) {
        let metadata = exact_metadata();
        let mut entity = Entity::unresolved_port_entity(
            entity_id,
            EntityKind::Enemy,
            TYPE47_COMMON_DYING_ENTITY_TYPE,
        );
        entity.authored_spawn_index = Some(spawn);
        entity.model_slots = [Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID); 4];
        entity.model_index = Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID);
        entity.mass_raw = TYPE47_COMMON_DYING_MASS_RAW;
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity.collision.state_flags_at_0x08 = RetailStateWord::exact(state_flags);
        entity.collision.default_state_flags_at_0xc8 =
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW);
        entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(7);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(active_context()));
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        let descriptor = SubJAttachmentDescriptor {
            reserved_at_0x01: 0,
            slots: vec![LEVEL_ONE_TYPE47_SUB_J_SLOT].into_boxed_slice(),
        };
        entity.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(
            SubJAttachmentRuntime::from_descriptor(&descriptor).unwrap(),
        ));
        entity.set_position_raw([0, 0, 0]);
        entity.set_velocity_raw([2_258, 0, 658]);
        let task = CommonDyingTaskState::prepare_after_allocation(
            entity_id,
            &metadata,
            CommonDyingComponentDescriptors::default(),
        )
        .unwrap()
        .map_task(ActorTaskRuntime::CommonDying)
        .apply_suffix(|| 0, |_| {});
        let task_id = entity
            .actor_tasks
            .replace_prepared(ActorTaskSlot::Primary, task);
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id,
        };
        (
            entity,
            FreshLevelOneType47CommonDyingOwner::from_authenticated_publication(
                entity_id, spawn, visit,
            ),
        )
    }

    /// Exact single-owner input for sibling composite-scheduler unit tests.
    ///
    /// Keeping the entity, Section-12 row, and receipt together prevents a
    /// mixed-family fixture from silently drifting away from this owner's
    /// production authentication contract.
    pub(crate) fn exact_type47_single_owner_fixture(
        entity_id: u32,
        spawn: usize,
    ) -> (
        Entity,
        EntityTypeRuntimeMetadata,
        FreshLevelOneType47CommonDyingOwner,
    ) {
        let state_flags = match spawn {
            11 | 12 => STATE_SLOT1,
            13 => STATE_SLOT3,
            _ => panic!("the exact Type-47 cohort contains only spawns 11 through 13"),
        };
        let metadata = exact_metadata();
        let (entity, owner) = exact_entity(entity_id, spawn, state_flags);
        (entity, metadata, owner)
    }

    fn exact_single_owner_manager(
        entity_id: u32,
        spawn: usize,
    ) -> (EntityManager, FreshLevelOneType47CommonDyingOwner) {
        let (entity, metadata, owner) = exact_type47_single_owner_fixture(entity_id, spawn);
        let mut metadata_rows = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata_rows[TYPE47_COMMON_DYING_ENTITY_TYPE as usize] = metadata;
        (
            EntityManager::from_entities_with_type_metadata_for_test(
                vec![entity],
                metadata_rows,
                true,
            ),
            owner,
        )
    }

    fn manager(
        entries: &[(u32, usize, u32)],
    ) -> (EntityManager, Vec<FreshLevelOneType47CommonDyingOwner>) {
        let (entities, owners): (Vec<_>, Vec<_>) = entries
            .iter()
            .copied()
            .map(|(entity_id, spawn, state)| exact_entity(entity_id, spawn, state))
            .unzip();
        let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 48];
        metadata[TYPE47_COMMON_DYING_ENTITY_TYPE as usize] = exact_metadata();
        (
            EntityManager::from_entities_with_type_metadata_for_test(entities, metadata, true),
            owners,
        )
    }

    fn entity(manager: &EntityManager, entity_id: u32) -> &Entity {
        manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap()
    }

    #[test]
    fn single_owner_tick_retains_waiting_receipt_without_manager_traversal() {
        let terrain = terrain(-1_000);
        let (mut manager, owner) = exact_single_owner_manager(100, 11);
        let mut words = [0x0026, 0x1e27].into_iter();
        let mut draws = 0;

        let tick = tick_type47_common_dying_owner(
            &mut manager,
            owner,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        );

        assert_eq!(
            tick,
            Type47CommonDyingOwnerTick {
                outcome: Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 100 },
                retained_owner: Some(owner),
                surface_bubble: None,
            }
        );
        assert_eq!(draws, 2);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert!(entity(&manager, 100)
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .is_some());
    }

    #[test]
    fn single_owner_tick_releases_terminal_receipt_before_later_sweep() {
        let terrain = terrain(-1_000);
        let (mut manager, owner) = exact_single_owner_manager(100, 11);

        let tick = tick_type47_common_dying_owner(
            &mut manager,
            owner,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );

        assert_eq!(tick.retained_owner, None);
        assert_eq!(tick.surface_bubble, None);
        let Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal) = tick.outcome
        else {
            panic!("exact callback must release its receipt and stage deferred destruction")
        };
        assert_eq!(terminal.entity_id, 100);
        assert_eq!(terminal.visit, owner.visit());
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [100]);
        assert!(manager.iter_all().any(|entity| entity.id == 100));
    }

    #[v2k_test_support::retail_test]
    fn normal_tier_retail_data_matches_the_complete_coarse_suffix_profile() {
        let data_dir = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data_dir).expect("init retail session");
        session
            .load_auxiliary_ovl(3, 1)
            .expect("load normal-tier first-world resources");
        let record = session
            .cache
            .global_entity_type(TYPE47_COMMON_DYING_ENTITY_TYPE as usize)
            .expect("type-47 Section-12 record");
        assert_eq!(record.raw_header[0x72], 1);
        assert_eq!(record.raw_header[0x73], 0);
        assert_eq!(
            u32::from_le_bytes(record.raw_header[0x74..0x78].try_into().unwrap()),
            LEVEL_ONE_TYPE47_SURFACE_LIFETIME_MS
        );
        assert_eq!(
            record.sub_j_attachment_descriptor().unwrap(),
            SubJAttachmentDescriptor {
                reserved_at_0x01: 0,
                slots: vec![LEVEL_ONE_TYPE47_SUB_J_SLOT].into_boxed_slice(),
            }
        );
        assert_eq!(
            session
                .cache
                .global_model(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID)
                .expect("model 302")
                .radius,
            LEVEL_ONE_TYPE47_MODEL_EXTENT_RAW
        );
    }

    #[test]
    fn slot1_and_slot3_run_full_coarse_suffix_then_wait_for_later_sweep() {
        let terrain = terrain(-1_000);
        for (spawn, state_before, state_after) in [
            (11, STATE_SLOT1, TERMINAL_SLOT1),
            (13, STATE_SLOT3, TERMINAL_SLOT3),
        ] {
            let (mut manager, owners) = manager(&[(100, spawn, state_before)]);
            let mut scheduler = Type47CommonDyingScheduler::new();
            scheduler.register(owners[0]);
            let mut draws = 0;
            let pass = scheduler.tick(
                &mut manager,
                Type47CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || {
                    draws += 1;
                    0
                },
            );

            let [Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal)] =
                pass.outcomes.as_slice()
            else {
                panic!("exact coarse owner must stage its deferred transition")
            };
            assert_eq!(draws, 2, "+0x70 must draw before +0x6C");
            assert_eq!(terminal.entity_id, 100);
            assert_eq!(terminal.visit, owners[0].visit());
            assert_eq!(terminal.committed_prefix.elapsed_ms, 20);
            assert_eq!(terminal.callback_mass_raw, TYPE47_COMMON_DYING_MASS_RAW);
            assert_eq!(
                terminal.tagged_result,
                CommonDyingTaggedResult {
                    singleton_address: COMMON_DYING_SCHEDULER_SINGLETON_ADDRESS,
                    tag: COMMON_DYING_OWNER_TRANSITION_TAG,
                }
            );
            assert!(pass.surface_bubbles.is_empty());
            assert!(scheduler.is_empty());

            let staged = entity(&manager, 100);
            assert_eq!(
                staged.collision.state_flags_at_0x08,
                RetailStateWord::exact(state_after)
            );
            assert_eq!(staged.velocity_raw(), [2_253, -28, 657]);
            assert_eq!(staged.position_raw(), [0, 0, 0]);
            assert_eq!(
                staged.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                staged.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
                .iter()
                .all(|slot| staged.actor_tasks.task_in_slot(*slot).is_none()));
            let RetailRuntimeValue::Known(Some(context)) = staged.current_behavior_context else {
                panic!("terminal style disappeared")
            };
            assert_eq!(
                context.active_style(),
                ActiveBehaviorStyle::Audited(
                    crate::actor_death::COMMON_ACTOR_DYING_COMPLETION_STYLE
                )
            );
            assert!(manager.iter_all().any(|entity| entity.id == 100));

            assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), [100]);
            assert!(!manager.iter_all().any(|entity| entity.id == 100));
        }
    }

    #[test]
    fn randomized_wait_clears_b2_without_mass_or_task_age_then_carry_caps_callback() {
        let terrain = terrain(-1_000);
        let (mut manager, owners) = manager(&[(100, 11, STATE_SLOT1)]);
        {
            let entity = manager.entity_mut_for_test(100).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(77);
        }
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[0]);
        let mut words = [0x0026, 0x1e27].into_iter();
        let mut draws = 0;
        let wait = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        );
        assert_eq!(
            wait.outcomes,
            [Type47CommonDyingProductionOutcome::SchedulerWaiting { entity_id: 100 }]
        );
        assert_eq!(draws, 2);
        assert_eq!(scheduler.registered_len(), 1);
        let waiting = entity(&manager, 100);
        assert_eq!(waiting.mass_raw, TYPE47_COMMON_DYING_MASS_RAW);
        assert_eq!(
            waiting.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            waiting
                .actor_task_state(ActorTaskSlot::Primary)
                .and_then(|task| match task {
                    ActorTaskRuntime::CommonDying(task) => Some(task.elapsed_ms()),
                    _ => None,
                }),
            Some(0)
        );
        assert_eq!(
            waiting.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(1_000)
        );
        assert_eq!(
            waiting.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(7)
        );
        assert_eq!(waiting.velocity_raw(), [2_258, 0, 658]);
        assert_eq!(
            waiting.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            waiting.collision.state_flags_at_0x08,
            RetailStateWord::exact(STATE_SLOT1)
        );
        assert!(manager.cleanup_pending_actor_deferred_destroys().is_empty());

        manager
            .entity_mut_for_test(100)
            .unwrap()
            .collision
            .callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(124_500);
        let mut continuation_draws = 0;
        let continued = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 1_000,
            },
            &mut || {
                continuation_draws += 1;
                0
            },
        );
        let [Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal)] =
            continued.outcomes.as_slice()
        else {
            panic!("the carried callback must continue")
        };
        assert_eq!(continuation_draws, 1, "carry skips the +0x6C draw");
        assert_eq!(terminal.committed_prefix.elapsed_ms, 125);
        let staged = entity(&manager, 100);
        assert_eq!(staged.mass_raw, TYPE47_COMMON_DYING_MASS_RAW);
        assert_eq!(
            staged.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(500)
        );
        assert_eq!(
            staged.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(125_007)
        );
    }

    #[test]
    fn scheduler_unit_delta_forces_one_microsecond_after_carry_math() {
        let terrain = terrain(-1_000);
        let (mut manager, owners) = manager(&[(100, 11, STATE_SLOT1)]);
        {
            let entity = manager.entity_mut_for_test(100).unwrap();
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(125_001);
            entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(1);
        }
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[0]);
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 0,
            },
            &mut || {
                draws += 1;
                0
            },
        );
        let [Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal)] =
            pass.outcomes.as_slice()
        else {
            panic!("the unit-delta callback must continue")
        };
        assert_eq!(draws, 0);
        assert_eq!(terminal.committed_prefix.elapsed_ms, 0);
        assert_eq!(
            entity(&manager, 100)
                .collision
                .recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(8)
        );
    }

    #[test]
    fn live_detail_bits_skip_scheduler_waits_and_still_stage_class12() {
        // Live construction `0x07068805` / post-10C10 `0x0707c805` keep both
        // FUN_00411400 bits. 12DA0 skips +0x70/+0x6C; class 12 still runs.
        let terrain = terrain(-1_000);
        let flags = STATE_SLOT1
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | crate::entity_view_detail::FULL_DETAIL_STATE_BIT;
        let (mut manager, owners) = manager(&[(100, 11, flags)]);
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[0]);
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || {
                draws += 1;
                panic!("live detail bits skip FUN_00412DA0 randomized waits")
            },
        );
        let [Type47CommonDyingProductionOutcome::DeferredDestroyStaged(terminal)] =
            pass.outcomes.as_slice()
        else {
            panic!(
                "live 0x02000000/0x04000000 must still Flip Over And Die: {:?}",
                pass.outcomes
            )
        };
        assert_eq!(draws, 0);
        assert_eq!(terminal.entity_id, 100);
        assert!(scheduler.is_empty());
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [100]);
    }

    #[test]
    fn unsupported_outer_owners_block_before_rng_and_retain_receipt() {
        let terrain = terrain(-1_000);
        for (state, expected) in [
            (
                STATE_SLOT1 | REMOTE_OWNED_STATE_BIT,
                Type47CommonDyingProductionBlock::RemoteSchedulerOwnerUnsupported,
            ),
            (
                STATE_SLOT1 & !COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                Type47CommonDyingProductionBlock::SchedulerCallbackDisabled,
            ),
            (
                STATE_SLOT1 | ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
                Type47CommonDyingProductionBlock::RelationOwnerUnsupported,
            ),
        ] {
            let (mut manager, owners) = manager(&[(100, 11, state)]);
            let before = entity(&manager, 100).collision.clone();
            let mut scheduler = Type47CommonDyingScheduler::new();
            scheduler.register(owners[0]);
            let pass = scheduler.tick(
                &mut manager,
                Type47CommonDyingProductionFrame {
                    terrain: &terrain,
                    elapsed_micros: 20_000,
                },
                &mut || panic!("an unsupported outer owner cannot touch shared RNG"),
            );
            assert_eq!(
                pass.outcomes,
                [Type47CommonDyingProductionOutcome::Blocked {
                    entity_id: 100,
                    reason: expected,
                }]
            );
            assert_eq!(scheduler.registered_len(), 1);
            assert_eq!(entity(&manager, 100).collision, before);
            assert!(entity(&manager, 100)
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .is_some());
        }
    }

    #[test]
    fn live_list_order_wins_over_registration_and_stale_receipts_drop_last() {
        let terrain = terrain(-1_000);
        let (mut manager, owners) = manager(&[(20, 11, STATE_SLOT1), (10, 12, STATE_SLOT1)]);
        let stale = FreshLevelOneType47CommonDyingOwner::from_authenticated_publication(
            99,
            13,
            owners[0].visit(),
        );
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[1]);
        scheduler.register(owners[0]);
        scheduler.register(owners[1]);
        scheduler.register(stale);
        assert_eq!(
            scheduler.registered_len(),
            3,
            "duplicate identity is replaced"
        );
        let pass = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || 0,
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [
                Type47CommonDyingProductionOutcome::DeferredDestroyStaged(
                    Type47CommonDyingTerminalFrame { entity_id: 20, .. }
                ),
                Type47CommonDyingProductionOutcome::DeferredDestroyStaged(
                    Type47CommonDyingTerminalFrame { entity_id: 10, .. }
                ),
                Type47CommonDyingProductionOutcome::Dropped {
                    entity_id: 99,
                    reason: Type47CommonDyingProductionDrop::EntityUnavailable,
                },
            ]
        ));
        assert!(scheduler.is_empty());
    }

    #[test]
    fn exact_deep_lifecycle_runs_after_scheduler_rng_and_preserves_deferred_mark() {
        let terrain = terrain(0);
        let (mut manager, owners) = manager(&[(100, 11, STATE_SLOT1)]);
        {
            let entity = manager.entity_mut_for_test(100).unwrap();
            entity.set_position_raw([0, -1_000, 0]);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_980);
        }
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[0]);
        let mut words = [0, 0, 1].into_iter();
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        );
        assert!(matches!(
            pass.outcomes.as_slice(),
            [Type47CommonDyingProductionOutcome::DeferredDestroyStaged(_)]
        ));
        assert_eq!(
            draws, 3,
            "scheduler draws precede the exact-boundary bubble miss"
        );
        assert!(pass.surface_bubbles.is_empty());
        let staged = entity(&manager, 100);
        assert_eq!(
            staged.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(2_000)
        );
        assert_eq!(
            staged.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(TYPE47_COMMON_DYING_INITIALIZER_STATE_FLAGS_RAW)
        );
        assert_eq!(
            staged
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
    }

    #[test]
    fn deep_bubble_hit_consumes_six_words_after_the_two_scheduler_draws() {
        let terrain = terrain(0);
        let (mut manager, owners) = manager(&[(100, 11, STATE_SLOT1)]);
        {
            let entity = manager.entity_mut_for_test(100).unwrap();
            entity.set_position_raw([0, -1_000, 0]);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(1_000);
        }
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[0]);
        let mut words = [0, 0, 0, 0, 0, 0, 0, 0].into_iter();
        let mut draws = 0;
        let pass = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || {
                draws += 1;
                words.next().unwrap()
            },
        );
        assert_eq!(draws, 8);
        let [bubble] = pass.surface_bubbles.as_slice() else {
            panic!("a zero divisor sample must emit one class-42 request")
        };
        assert_eq!(bubble.owner_entity_id, 100);
        assert_eq!(
            bubble.owner_entity_type,
            TYPE47_COMMON_DYING_ENTITY_TYPE as u8
        );
        assert!(!bubble.suppresses_impact_damage);
        assert_eq!(
            entity(&manager, 100).surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(1_020)
        );
    }

    #[test]
    fn nonempty_sub_j_blocks_before_rng_and_retains_task_and_receipt() {
        let terrain = terrain(-1_000);
        let (mut manager, owners) = manager(&[(100, 11, STATE_SLOT1)]);
        let expected_runtime = {
            let entity = manager.entity_mut_for_test(100).unwrap();
            let RetailRuntimeValue::Known(Some(runtime)) = &mut entity.sub_j_attachment_runtime
            else {
                panic!("exact fixture lost Sub-J")
            };
            runtime.append(777).unwrap();
            runtime.clone()
        };
        let mut scheduler = Type47CommonDyingScheduler::new();
        scheduler.register(owners[0]);
        let pass = scheduler.tick(
            &mut manager,
            Type47CommonDyingProductionFrame {
                terrain: &terrain,
                elapsed_micros: 20_000,
            },
            &mut || panic!("unsupported Sub-J state cannot touch scheduler RNG"),
        );
        assert_eq!(
            pass.outcomes,
            [Type47CommonDyingProductionOutcome::Blocked {
                entity_id: 100,
                reason: Type47CommonDyingProductionBlock::UnexpectedSubJRuntimeShape {
                    authored_slots: 1,
                    capacity: 1,
                    live_rows: 1,
                    outer_policy_raw: 0,
                },
            }]
        );
        assert_eq!(scheduler.registered_len(), 1);
        assert_eq!(
            entity(&manager, 100).sub_j_attachment_runtime.clone(),
            RetailRuntimeValue::Known(Some(expected_runtime))
        );
        assert!(entity(&manager, 100)
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .is_some());
    }
}
