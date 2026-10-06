//! Live Go-To-Job visit for Main-Base cargo-converted type-8 scientists.
//!
//! Adoption requires the `V200002.run` conversion seed `0x38` Sub-D owner and
//! the published D720/13F70 birth basis. The visit drives `FUN_00403780`:
//! live target lookup, `FUN_00423030`, then the shared actor D/I/A/B kernel.
//! Tagged returns and strict task expiry re-enter the type-default selector
//! after unwind, retaining either class-54 Go-To-Job or class-6 Wander custody.
//! A completed visit then applies the common `FUN_00412DA0` master-motion
//! suffix. After a successful callback this owner rebuilds `FUN_00413F70`
//! from the post-task angle words (no `0x10` E640 gate). Ordinary Type-9
//! selected custody and the Type-9 E370 surface suffix stay out of this
//! owner. Factory-ejected type-8 has no `0x38` mint and stays out of custody.

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskVisit};
use crate::common_mover::actor_abdi::{
    advance_actor_abdi_common_mover, ActorAbdiEntityKind, ActorAbdiFrameBlock,
    ActorAbdiFrameOutcome, ActorAbdiFrameRequest, ActorAbdiFrameState, ActorAbdiTopology,
    ActorAbdiTopologyError, FIRST_WORLD_SCIENTIST_ENTITY_TYPE,
};
use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{commit_common_master_motion, Entity, EntityManager};
use crate::entity_behavior::BehaviorContextRuntime;
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT, DYING_STATE_BIT,
};
use crate::go_to_job_owner::{
    evaluate_go_to_job_callback, go_to_job_transition_after_unwind, GoToJobCallbackError,
    GoToJobCallbackPrefix, GoToJobCallbackRequest, GoToJobCallbackResult, GoToJobCommonMoverReturn,
    GoToJobRoutePredicate, GoToJobTaggedSingleton, GoToJobTargetRuntimeState,
};
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange};
use v2k_formats::collision::CommonAxisDescriptor;
use v2k_formats::terrain::TerrainGrid;

mod root_transition;
mod wander;
use root_transition::resume_root_transition;
pub use root_transition::{Type8RootTransitionBlock, Type8RootTransitionReason};
use wander::tick_wander;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScientistTask {
    GoToJob { target_id: Option<u32> },
    Wander,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitPhase {
    Ready,
    RootPending {
        reason: Type8RootTransitionReason,
        elapsed_micros: u32,
    },
    CallbackIncomplete,
}

/// Ordinary Level-1 `FUN_00401430` scheduler-mode word for type 8.
///
/// Type-9 selected Go-To-Job and Follow Beacons also use mode 0. Intro2
/// restricted mode 1 is recorded only for types 13/26.
pub const TYPE8_GO_TO_JOB_SCHEDULER_MODE: i32 = 0;

#[derive(Debug, PartialEq, Eq)]
pub struct Type8GoToJobSchedulerOwner {
    entity_id: u32,
    primary_task_id: ActorTaskId,
    task: ScientistTask,
    context: BehaviorContextRuntime,
    immutable_anchor_raw: [i16; 3],
    phase: VisitPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type8GoToJobAdoptionError {
    WrongEntityType { actual: u32 },
    SubDFirstQueryUnavailable,
    BodyBasisUnavailable,
    PrimaryTaskMismatch,
    ConstructorAnchorUnavailable,
    BehaviorContextUnavailable,
}

/// Type-8 E370/attitude is a separate owner. This visit does not invent one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type8SurfaceSuffix {
    Unclaimed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type8GoToJobSchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type8GoToJobSchedulerProductionBlock {
    VisitUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type8GoToJobMoverBlock {
    Topology(ActorAbdiTopologyError),
    SubDFirstQueryUnavailable,
    BodyBasisUnavailable,
    SubARuntimeUnavailable,
    AnimationUnavailable,
    TerrainUnavailable,
    AxisUnavailable,
    MasterMotionStateUnavailable,
    ActorAbdi(ActorAbdiFrameBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type8GoToJobVisitResult {
    Tagged(GoToJobTaggedSingleton),
    Continue,
    CommonMoverBlocked(Type8GoToJobMoverBlock),
    TargetValidationUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type8GoToJobVisit {
    pub prefix: GoToJobCallbackPrefix,
    pub result: Type8GoToJobVisitResult,
    pub surface_suffix: Type8SurfaceSuffix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type8GoToJobSchedulerProductionOutcome {
    Visit {
        entity_id: u32,
        target_id: u32,
        visit: Type8GoToJobVisit,
    },
    Blocked {
        entity_id: u32,
        reason: Type8GoToJobSchedulerProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Type8GoToJobSchedulerProductionDrop,
    },
    WanderVisit {
        entity_id: u32,
        prefix: crate::ordinary_type9_wander_owner::OrdinaryType9WanderCallbackPrefix,
        result: crate::wander_near_location::WanderNearTaskCallbackResult,
    },
    WanderBlocked {
        entity_id: u32,
        prefix: crate::ordinary_type9_wander_owner::OrdinaryType9WanderCallbackPrefix,
        error: Type8GoToJobMoverBlock,
    },
    Transition {
        entity_id: u32,
        reason: Type8RootTransitionReason,
        selection: crate::entity_behavior::BehaviorSelection,
        predecessor_task_id: ActorTaskId,
        published_task_id: ActorTaskId,
    },
    TransitionSuppressed {
        entity_id: u32,
        reason: Type8RootTransitionReason,
    },
    TransitionBlocked {
        entity_id: u32,
        reason: Type8RootTransitionReason,
        block: Type8RootTransitionBlock,
    },
    CallbackIncomplete {
        entity_id: u32,
    },
}

impl Type8GoToJobSchedulerProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Visit { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. }
            | Self::WanderVisit { entity_id, .. }
            | Self::WanderBlocked { entity_id, .. }
            | Self::Transition { entity_id, .. }
            | Self::TransitionSuppressed { entity_id, .. }
            | Self::TransitionBlocked { entity_id, .. }
            | Self::CallbackIncomplete { entity_id } => *entity_id,
        }
    }
}

pub struct Type8GoToJobSchedulerOwnerTick {
    pub outcome: Type8GoToJobSchedulerProductionOutcome,
    pub retained_owner: Option<Type8GoToJobSchedulerOwner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Type8GoToJobControllerContext {
    axis_at_0x28: CommonAxisDescriptor,
}

impl Type8GoToJobSchedulerOwner {
    pub const fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub const fn target_id(&self) -> Option<u32> {
        match self.task {
            ScientistTask::GoToJob { target_id } => target_id,
            ScientistTask::Wander => None,
        }
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            entity_id: self.entity_id,
            primary_task_id: self.primary_task_id,
            task: self.task,
            context: self.context,
            immutable_anchor_raw: self.immutable_anchor_raw,
            phase: self.phase,
        }
    }

    pub fn adopt_published(
        entity: &Entity,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type8GoToJobAdoptionError> {
        if entity.entity_type != u32::from(FIRST_WORLD_SCIENTIST_ENTITY_TYPE) {
            return Err(Type8GoToJobAdoptionError::WrongEntityType {
                actual: entity.entity_type,
            });
        }
        let _ = ActorAbdiTopology::from_metadata(FIRST_WORLD_SCIENTIST_ENTITY_TYPE, metadata)
            .map_err(|_| Type8GoToJobAdoptionError::WrongEntityType {
                actual: entity.entity_type,
            })?;
        if entity.type8_sub_d_frame_owner.is_none() || entity.type8_sub_d_runtime.is_none() {
            return Err(Type8GoToJobAdoptionError::SubDFirstQueryUnavailable);
        }
        if !matches!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(_)
        ) {
            return Err(Type8GoToJobAdoptionError::BodyBasisUnavailable);
        }
        let Some(primary_task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
            return Err(Type8GoToJobAdoptionError::PrimaryTaskMismatch);
        };
        let task = match entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary) {
            Some(ActorTaskRuntime::GoToJob(state)) => ScientistTask::GoToJob {
                target_id: state.target_id(),
            },
            Some(ActorTaskRuntime::OrdinaryType9Wander(_)) => ScientistTask::Wander,
            _ => return Err(Type8GoToJobAdoptionError::PrimaryTaskMismatch),
        };
        let RetailRuntimeValue::Known(immutable_anchor_raw) =
            entity.type8_wander_anchor_raw_at_0x90
        else {
            return Err(Type8GoToJobAdoptionError::ConstructorAnchorUnavailable);
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Type8GoToJobAdoptionError::BehaviorContextUnavailable);
        };
        if !root_transition::context_matches_task(context, task) || !later_slots_empty(entity) {
            return Err(Type8GoToJobAdoptionError::BehaviorContextUnavailable);
        }
        Ok(Self {
            entity_id: entity.id,
            primary_task_id,
            task,
            context,
            immutable_anchor_raw,
            phase: VisitPhase::Ready,
        })
    }
}

pub fn tick_type8_go_to_job_scheduler_owner(
    manager: &mut EntityManager,
    mut owner: Type8GoToJobSchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    elapsed_micros: u32,
) -> Type8GoToJobSchedulerOwnerTick {
    let metadata = manager
        .type_runtime_metadata(u32::from(FIRST_WORLD_SCIENTIST_ENTITY_TYPE))
        .cloned();
    let Some(metadata) = metadata else {
        return drop_owner(owner, Type8GoToJobSchedulerProductionDrop::GraphMismatch);
    };
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
    else {
        return drop_owner(
            owner,
            Type8GoToJobSchedulerProductionDrop::EntityUnavailable,
        );
    };
    if !owner_matches_entity(&owner, entity) {
        return drop_owner(owner, Type8GoToJobSchedulerProductionDrop::GraphMismatch);
    }
    match owner.phase {
        VisitPhase::RootPending { .. } => {
            return resume_root_transition(manager, owner, world_fx, &metadata)
        }
        VisitPhase::CallbackIncomplete => {
            return Type8GoToJobSchedulerOwnerTick {
                outcome: Type8GoToJobSchedulerProductionOutcome::CallbackIncomplete {
                    entity_id: owner.entity_id(),
                },
                retained_owner: Some(owner),
            }
        }
        VisitPhase::Ready => {}
    }
    if owner.task == ScientistTask::Wander {
        return tick_wander(manager, owner, world_fx, terrain, elapsed_micros, &metadata);
    }
    let (target_state, tracked_target, target_position_raw) = owner.target_id().map_or(
        (
            Ok(GoToJobTargetRuntimeState::Missing),
            RetailRuntimeValue::Known(None),
            None,
        ),
        |target_id| type8_go_to_job_target_runtime_state(manager, target_id),
    );

    let Some(entity) = manager.type8_go_to_job_entity_mut(owner.entity_id()) else {
        return drop_owner(
            owner,
            Type8GoToJobSchedulerProductionDrop::EntityUnavailable,
        );
    };
    if entity.entity_type != u32::from(FIRST_WORLD_SCIENTIST_ENTITY_TYPE)
        || entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) != Some(owner.primary_task_id)
    {
        return drop_owner(owner, Type8GoToJobSchedulerProductionDrop::GraphMismatch);
    }

    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: owner.primary_task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::GoToJob(state) = runtime else {
            unreachable!("adopted Primary is GoToJob")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Type8GoToJobSchedulerOwnerTick {
            outcome: Type8GoToJobSchedulerProductionOutcome::Blocked {
                entity_id: owner.entity_id(),
                reason: Type8GoToJobSchedulerProductionBlock::VisitUnavailable,
            },
            retained_owner: Some(owner),
        };
    };
    let Some(ActorTaskRuntime::GoToJob(state)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        finish_visit(entity, visit);
        return drop_owner(owner, Type8GoToJobSchedulerProductionDrop::GraphMismatch);
    };
    let mut stage = state.stage_callback();
    let mut movement = ();
    let RetailRuntimeValue::Known(axis_at_0x28) = entity.actor_common_axis_descriptor else {
        finish_visit(entity, visit);
        owner.phase = VisitPhase::CallbackIncomplete;
        return visit_tick(
            owner,
            prefix,
            Type8GoToJobVisitResult::CommonMoverBlocked(Type8GoToJobMoverBlock::AxisUnavailable),
        );
    };
    let mut controller = Type8GoToJobControllerContext { axis_at_0x28 };
    let owner_position_raw = entity.position_raw();
    let master_gate = match entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    {
        RetailRuntimeValue::Known(bits) => bits,
        RetailRuntimeValue::Unresolved => {
            finish_visit(entity, visit);
            owner.phase = VisitPhase::CallbackIncomplete;
            return visit_tick(
                owner,
                prefix,
                Type8GoToJobVisitResult::CommonMoverBlocked(
                    Type8GoToJobMoverBlock::MasterMotionStateUnavailable,
                ),
            );
        }
    };
    let evaluated = evaluate_go_to_job_callback(
        &mut stage,
        GoToJobCallbackRequest {
            visit,
            entity_id: owner.entity_id(),
            movement_state: &mut movement,
            controller_context: &mut controller,
            elapsed_micros,
            scheduler_mode: TYPE8_GO_TO_JOB_SCHEDULER_MODE as u32,
        },
        |_| target_state,
        |_| {
            Ok::<_, ()>(match target_position_raw {
                Some(target_position_raw)
                    if within_wrapped_axis_range(
                        WrappedAxisRange::from_raw(axis_at_0x28.strict_axis_limit_raw),
                        owner_position_raw,
                        target_position_raw,
                    ) =>
                {
                    GoToJobRoutePredicate::NonZero
                }
                _ => GoToJobRoutePredicate::Zero,
            })
        },
        |request| {
            run_type8_go_to_job_common_mover(
                entity,
                &metadata,
                terrain,
                request.target_state,
                tracked_target,
                elapsed_micros,
                || u32::from(world_fx.next_shared_retail_random_u16()),
            )
        },
    );
    let should_commit = evaluated.is_ok();
    if should_commit {
        if let Some(ActorTaskRuntime::GoToJob(surviving)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        {
            stage.commit(surviving);
        }
    }
    finish_visit(entity, visit);
    if let Ok(callback_result) = &evaluated {
        if let Some(request) = go_to_job_transition_after_unwind(visit, prefix, *callback_result) {
            owner.phase = VisitPhase::RootPending {
                reason: Type8RootTransitionReason::GoToJob(request.reason),
                elapsed_micros,
            };
            return resume_root_transition(manager, owner, world_fx, &metadata);
        }
    }
    let result = match evaluated {
        Ok(GoToJobCallbackResult::Tagged(_)) => unreachable!("tagged result enters the root owner"),
        Ok(GoToJobCallbackResult::Continue) => {
            publish_type8_post_task_body_basis(entity);
            let plan = plan_common_master_motion(
                entity.position_raw(),
                entity.velocity_raw(),
                master_gate,
                elapsed_micros,
            );
            commit_common_master_motion(entity, plan);
            Type8GoToJobVisitResult::Continue
        }
        Err(GoToJobCallbackError::CommonMover { error, .. }) => {
            owner.phase = VisitPhase::CallbackIncomplete;
            Type8GoToJobVisitResult::CommonMoverBlocked(error)
        }
        Err(GoToJobCallbackError::TargetValidation { .. })
        | Err(GoToJobCallbackError::RoutePredicate { .. }) => {
            owner.phase = VisitPhase::CallbackIncomplete;
            Type8GoToJobVisitResult::TargetValidationUnresolved
        }
    };
    visit_tick(owner, prefix, result)
}

fn run_type8_go_to_job_common_mover(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: Option<&TerrainGrid>,
    target_state: &mut crate::wander_near_location::WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    elapsed_micros: u32,
    next_random: impl FnMut() -> u32,
) -> Result<GoToJobCommonMoverReturn, Type8GoToJobMoverBlock> {
    if entity.type8_sub_d_frame_owner.is_none() || entity.type8_sub_d_runtime.is_none() {
        return Err(Type8GoToJobMoverBlock::SubDFirstQueryUnavailable);
    }
    let topology = ActorAbdiTopology::from_metadata(FIRST_WORLD_SCIENTIST_ENTITY_TYPE, metadata)
        .map_err(Type8GoToJobMoverBlock::Topology)?;
    debug_assert_eq!(topology.entity_kind(), ActorAbdiEntityKind::Scientist);
    let sub_a = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a))
            if sub_a.target_speed_raw() != RetailRuntimeValue::Unresolved =>
        {
            sub_a
        }
        _ => return Err(Type8GoToJobMoverBlock::SubARuntimeUnavailable),
    };
    let animation = match entity.actor_animation_runtime {
        RetailRuntimeValue::Known(Some(animation)) => animation,
        _ => return Err(Type8GoToJobMoverBlock::AnimationUnavailable),
    };
    let basis = match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(Type9BodyBasis {
            lateral, forward, ..
        }) => (lateral, forward),
        RetailRuntimeValue::Unresolved => return Err(Type8GoToJobMoverBlock::BodyBasisUnavailable),
    };
    let terrain = terrain.ok_or(Type8GoToJobMoverBlock::TerrainUnavailable)?;
    let mut state = ActorAbdiFrameState {
        sub_d_frame_owner: entity
            .type8_sub_d_frame_owner
            .expect("first-query owner preflight"),
        sub_d_runtime: entity
            .type8_sub_d_runtime
            .expect("first-query runtime preflight"),
        sub_a_runtime: sub_a,
        actor_animation: animation,
    };
    let outcome = advance_actor_abdi_common_mover(
        &mut state,
        ActorAbdiFrameRequest {
            topology,
            target_state: *target_state,
            tracked_target,
            terrain,
            position_raw: entity.position_raw(),
            pre_mover_right_q31: basis.0,
            pre_mover_forward_q31: basis.1,
            heading_raw: entity.heading_raw(),
            velocity_raw: entity.velocity_raw(),
            elapsed_micros,
            global_elapsed_micros: elapsed_micros,
            scheduler_mode: TYPE8_GO_TO_JOB_SCHEDULER_MODE,
        },
        next_random,
    )
    .map_err(Type8GoToJobMoverBlock::ActorAbdi)?;
    match outcome {
        ActorAbdiFrameOutcome::ReturnedZero(_) => Ok(GoToJobCommonMoverReturn::Zero),
        ActorAbdiFrameOutcome::Applied(step) => {
            entity.type8_sub_d_frame_owner = Some(state.sub_d_frame_owner);
            entity.type8_sub_d_runtime = Some(state.sub_d_runtime);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(state.sub_a_runtime));
            entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(state.actor_animation));
            entity.set_heading_raw(step.heading_raw);
            entity.set_motion_raw(entity.position_raw(), step.velocity_raw);
            *target_state = step.target_state;
            Ok(GoToJobCommonMoverReturn::NonZero)
        }
    }
}

fn publish_type8_post_task_body_basis(entity: &mut Entity) {
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
        heading_raw,
        pitch_raw,
        roll_raw,
    ));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
}

fn type8_go_to_job_target_runtime_state(
    manager: &EntityManager,
    target_id: u32,
) -> (
    Result<GoToJobTargetRuntimeState, ()>,
    RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    Option<[i16; 3]>,
) {
    let Some(target) = manager.iter_all().find(|entity| entity.id == target_id) else {
        return (
            Ok(GoToJobTargetRuntimeState::Missing),
            RetailRuntimeValue::Known(None),
            None,
        );
    };
    if !target.active {
        return (
            Ok(GoToJobTargetRuntimeState::Inactive),
            RetailRuntimeValue::Known(None),
            Some(target.position_raw()),
        );
    }
    let state = target.collision.state_flags_at_0x08;
    let snapshot = CommonMoverTrackedTargetSnapshot {
        state_flags: state,
        position_raw: target.position_raw(),
        velocity_raw: target.velocity_raw(),
    };
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => (
            Err(()),
            RetailRuntimeValue::Unresolved,
            Some(target.position_raw()),
        ),
        RetailRuntimeValue::Known(bits) if bits != 0 => (
            Ok(GoToJobTargetRuntimeState::Dying),
            RetailRuntimeValue::Known(Some(snapshot)),
            Some(target.position_raw()),
        ),
        RetailRuntimeValue::Known(_) if state.known_value_bits() != 0 => (
            Ok(GoToJobTargetRuntimeState::Live),
            RetailRuntimeValue::Known(Some(snapshot)),
            Some(target.position_raw()),
        ),
        RetailRuntimeValue::Known(_) if state.known_mask() == u32::MAX => (
            Ok(GoToJobTargetRuntimeState::Inactive),
            RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                state_flags: crate::entity_collision_state::RetailStateWord::exact(0),
                position_raw: target.position_raw(),
                velocity_raw: target.velocity_raw(),
            })),
            Some(target.position_raw()),
        ),
        RetailRuntimeValue::Known(_) => (
            Ok(GoToJobTargetRuntimeState::Live),
            RetailRuntimeValue::Unresolved,
            Some(target.position_raw()),
        ),
    }
}

fn drop_owner(
    owner: Type8GoToJobSchedulerOwner,
    reason: Type8GoToJobSchedulerProductionDrop,
) -> Type8GoToJobSchedulerOwnerTick {
    Type8GoToJobSchedulerOwnerTick {
        outcome: Type8GoToJobSchedulerProductionOutcome::Dropped {
            entity_id: owner.entity_id(),
            reason,
        },
        retained_owner: None,
    }
}

fn visit_tick(
    owner: Type8GoToJobSchedulerOwner,
    prefix: GoToJobCallbackPrefix,
    result: Type8GoToJobVisitResult,
) -> Type8GoToJobSchedulerOwnerTick {
    Type8GoToJobSchedulerOwnerTick {
        outcome: Type8GoToJobSchedulerProductionOutcome::Visit {
            entity_id: owner.entity_id(),
            target_id: owner.target_id().unwrap_or(0),
            visit: Type8GoToJobVisit {
                prefix,
                result,
                surface_suffix: Type8SurfaceSuffix::Unclaimed,
            },
        },
        retained_owner: Some(owner),
    }
}

fn later_slots_empty(entity: &Entity) -> bool {
    [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary]
        .into_iter()
        .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none())
}

fn owner_matches_entity(owner: &Type8GoToJobSchedulerOwner, entity: &Entity) -> bool {
    entity.active
        && entity.entity_type == u32::from(FIRST_WORLD_SCIENTIST_ENTITY_TYPE)
        && entity.type8_sub_d_frame_owner.is_some()
        && entity.type8_sub_d_runtime.is_some()
        && entity.type8_wander_anchor_raw_at_0x90
            == RetailRuntimeValue::Known(owner.immutable_anchor_raw)
        && entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) == Some(owner.primary_task_id)
        && entity.current_behavior_context == RetailRuntimeValue::Known(Some(owner.context))
        && later_slots_empty(entity)
        && match (
            owner.task,
            entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary),
        ) {
            (ScientistTask::GoToJob { target_id }, Some(ActorTaskRuntime::GoToJob(state))) => {
                state.target_id() == target_id
            }
            (ScientistTask::Wander, Some(ActorTaskRuntime::OrdinaryType9Wander(_))) => true,
            _ => false,
        }
}

fn finish_completed_frame(entity: &mut Entity, elapsed_micros: u32, master_gate: u32) {
    // E870's Type8 route skips E640 and executes F70 after the task owner,
    // then 12DA0 integrates the resulting pose. E370 remains separate.
    publish_type8_post_task_body_basis(entity);
    let plan = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        master_gate,
        elapsed_micros,
    );
    commit_common_master_motion(entity, plan);
}

fn finish_visit(entity: &mut Entity, visit: ActorTaskVisit) {
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
}
