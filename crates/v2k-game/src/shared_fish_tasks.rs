//! Native fish ACD0/AD10/B640 graphs and C7D0/AF50 target handoff.
//!
//! Task algorithms remain in their shared detached owners. This module joins
//! them to the fish's B/D/F allocation, preserving source slot order, the
//! no-RNG Sub-F constructor reset and exact-wrapper callback custody.

use std::convert::Infallible;

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit, PreparedActorTask},
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    entity::{Entity, EntityManager},
    entity_behavior::{
        audited_behavior_style, behavior_program, initial_behavior_state_policy,
        select_initial_behavior, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT},
    guard_location_owner::acquisition::{
        self, GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionTaskState,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection,
        GuardLocationCandidateSelectionError, GuardLocationEntityRef, GuardLocationSearchContext,
    },
    ordinary_type9_wander_owner::{
        ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
        OrdinaryType9WanderPostUnwind, OrdinaryType9WanderTaskState,
    },
    search_attack::SearchAttackCandidateFilter,
    shared_retarget_mover::{
        shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
        SharedRetargetTaskState,
    },
    shared_target_route::*,
    wander_near_location::{
        map_common_mover_return, WanderNearCommonMoverReturn, WanderNearPrivateState,
    },
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FishTaskError<E = Infallible> {
    Allocation,
    Metadata,
    Graph,
    Runtime(&'static str),
    Acquisition(GuardLocationCandidateSelectionError),
    Mover(E),
}

impl<E> FishTaskError<E> {
    pub(crate) fn map_mover<F>(self, map: impl FnOnce(E) -> F) -> FishTaskError<F> {
        match self {
            Self::Allocation => FishTaskError::Allocation,
            Self::Metadata => FishTaskError::Metadata,
            Self::Graph => FishTaskError::Graph,
            Self::Runtime(value) => FishTaskError::Runtime(value),
            Self::Acquisition(value) => FishTaskError::Acquisition(value),
            Self::Mover(value) => FishTaskError::Mover(map(value)),
        }
    }
}

/// All authenticated fish choices are Always; AC60 consumes exactly one word.
pub(crate) fn select_behavior(
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, FishTaskError> {
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(FishTaskError::Metadata)?;
    if initializer
        .behavior_choices
        .iter()
        .any(|choice| choice.weight_rule_id != 1 || !matches!(choice.behavior_class_id, 5 | 6 | 13))
    {
        return Err(FishTaskError::Metadata);
    }
    select_initial_behavior(
        &initializer.behavior_choices,
        |rule| {
            debug_assert_eq!(rule, BehaviorWeightRule::Always);
            1
        },
        next_random,
    )
    .map_err(|_| FishTaskError::Metadata)?
    .ok_or(FishTaskError::Metadata)
}

fn reset_and_publish(
    entity: &mut Entity,
    slot: ActorTaskSlot,
    prepared: PreparedActorTask<ActorTaskRuntime>,
    target_route: bool,
) -> Result<(), FishTaskError> {
    let fish = entity
        .shared_fish_runtime
        .as_mut()
        .ok_or(FishTaskError::Allocation)?;
    // 06070: 424380(F,0), 424390(F,0); B/D/F has no Sub-A RNG suffix.
    fish.sub_f.reset_shared_task();
    if target_route {
        // 03650's distinct post-06070 suffix, before A7A0 publication.
        fish.sub_f.set_follow_target_mode_raw(1);
    }
    entity.actor_tasks.replace_prepared(slot, prepared);
    Ok(())
}

pub(crate) fn publish_selection(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
) -> Result<(), FishTaskError> {
    publish_selection_with(entity, metadata, selection, context, |runtime| {
        Ok(PreparedActorTask::new(runtime))
    })
}

fn publish_selection_with(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    mut prepare: impl FnMut(
        ActorTaskRuntime,
    ) -> Result<PreparedActorTask<ActorTaskRuntime>, FishTaskError>,
) -> Result<(), FishTaskError> {
    let class = selection.program.class_id;
    if !matches!(class, 5 | 6 | 13)
        || context.descriptor() != BehaviorDescriptorIdentity::Named(selection.program)
        || context.style_table_index_raw_at_0x10() != 0
        || entity.shared_fish_runtime.is_none()
    {
        return Err(FishTaskError::Graph);
    }
    let initializer = metadata
        .initializer
        .as_ref()
        .ok_or(FishTaskError::Metadata)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let position = entity.position_raw();
    let result = (|| {
        // B640 copies only authored common-axis +4 before clearing slot2.
        if class == 13 {
            let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
                return Err(FishTaskError::Runtime("Flocking axis"));
            };
            axis.raw_word_at_0x04 = initializer.common_axis_descriptor.raw_word_at_0x04;
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
        }
        entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        if class == 13 {
            let prepared = prepare(ActorTaskRuntime::GuardLocationAcquisition(
                GuardLocationAcquisitionTaskState::new(u32::MAX),
            ))?;
            reset_and_publish(entity, ActorTaskSlot::Secondary, prepared, false)?;
        } else {
            entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        }
        let runtime = if class == 6 {
            ActorTaskRuntime::OrdinaryType9Wander(
                OrdinaryType9WanderTaskState::from_allocated_anchor(position),
            )
        } else {
            ActorTaskRuntime::SharedRetarget(SharedRetargetTaskState::new(position, 5000))
        };
        let prepared = prepare(runtime)?;
        reset_and_publish(entity, ActorTaskSlot::Primary, prepared, false)
    })();
    if result.is_err() {
        // C6B0's failed initializer fallback preserves earlier writes, then
        // replaces the context and clears Secondary, Tertiary, Primary.
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    result
}

pub(crate) fn reselect_after_task_result(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), FishTaskError> {
    // 416410's suppression read precedes C690/416460's dying-state read.
    // A known-set suppression bit must not require unrelated bit evidence.
    let RetailRuntimeValue::Known(suppressed) = entity.collision.state_flags_at_0x08.masked(0x1000)
    else {
        return Err(FishTaskError::Runtime("fish task-result suppression"));
    };
    if suppressed != 0 {
        return Ok(());
    }
    reselect_behavior(entity, metadata, next_random)
}

/// Direct C690/AC60 entry from a style callback. Unlike task-result dispatch,
/// this does not first call416410 or suppress selection for attachment bit1000.
pub(crate) fn reselect_behavior(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), FishTaskError> {
    let RetailRuntimeValue::Known(flags) =
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
    else {
        return Err(FishTaskError::Runtime("fish C690 state"));
    };
    if flags & DYING_STATE_BIT != 0 {
        return Err(FishTaskError::Runtime("fish alternate dying owner"));
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(FishTaskError::Graph);
    };
    let selection = select_behavior(metadata, next_random)?;
    let context = previous
        .reselect_named_type_default(selection.program, 0, selection.program.initial_style)
        .ok_or(FishTaskError::Graph)?;
    publish_selection(entity, metadata, selection, context)
}

fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

fn publish_target_route(entity: &mut Entity, target_id: u32) -> Result<(), FishTaskError> {
    publish_target_route_with(entity, target_id, |runtime| {
        Ok(PreparedActorTask::new(runtime))
    })
}

fn publish_target_route_with(
    entity: &mut Entity,
    target_id: u32,
    prepare: impl FnOnce(ActorTaskRuntime) -> Result<PreparedActorTask<ActorTaskRuntime>, FishTaskError>,
) -> Result<(), FishTaskError> {
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(FishTaskError::Graph);
    };
    let program = behavior_program(13).unwrap();
    if previous.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || previous.style_table_index_raw_at_0x10() != 0
    {
        return Err(FishTaskError::Graph);
    }
    let context = BehaviorContextRuntime::named_audited(
        program,
        1,
        previous.choice_list_source(),
        RetailRuntimeValue::Known(Some(target_id)),
        previous.auxiliary_word_at_0x0c(),
        *audited_behavior_style(13, 1).unwrap(),
    )
    .ok_or(FishTaskError::Graph)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    // AF50 has no target-warning call (unlike Capture's AEE0 wrapper).
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    let prepared = prepare(ActorTaskRuntime::FishTargetRoute(
        SharedTargetRouteTaskState::after_allocation(entity.position_raw(), target_id),
    ));
    let result = prepared
        .and_then(|prepared| reset_and_publish(entity, ActorTaskSlot::Primary, prepared, true));
    if result.is_err() {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    result
}

/// Fresh Secondary read after Primary/reselection; a new Primary waits for the
/// next A800 pass. C7D0 clears the executing wrapper before its tag is consumed.
pub(crate) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), FishTaskError> {
    let entity = manager.entity_mut(id).ok_or(FishTaskError::Allocation)?;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(FishTaskError::Graph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(behavior_program(13).unwrap())
        || context.style_table_index_raw_at_0x10() != 0
    {
        return Err(FishTaskError::Graph);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Secondary)
        .ok_or(FishTaskError::Graph)?;
    if !matches!(
        entity.actor_tasks.task_state(task_id),
        Some(ActorTaskRuntime::GuardLocationAcquisition(_))
    ) {
        return Err(FishTaskError::Graph);
    }
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        return Err(FishTaskError::Runtime("fish acquisition axis"));
    };
    let mut search = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        SearchAttackCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::GuardLocationAcquisition(task) = runtime else {
                unreachable!()
            };
            task.before_callback(&mut search, next_random())
        })
        .ok_or(FishTaskError::Graph)?;
    if let GuardLocationAcquisitionCallbackPrefix::Acquire {
        filter_write: Some(filter),
        ..
    } = prefix
    {
        axis.raw_word_at_0x04 = filter.raw();
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    let result = match prefix {
        GuardLocationAcquisitionCallbackPrefix::ChanceRejected { .. } => Ok(None),
        GuardLocationAcquisitionCallbackPrefix::Acquire { search_context, .. } => {
            let owner = candidate(entity);
            let candidates: Vec<_> = manager
                .iter_all()
                .filter(|e| e.active)
                .map(candidate)
                .collect();
            acquisition::select_guard_location_candidate(GuardLocationCandidateRequest {
                owner,
                candidates_in_intrusive_order: &candidates,
                search_context,
            })
            .map(|selection| match selection {
                GuardLocationCandidateSelection::Selected(target) => Some(target.id),
                GuardLocationCandidateSelection::TaggedNoCandidate { .. } => None,
            })
            .map_err(FishTaskError::Acquisition)
        }
    };
    let entity = manager.entity_mut(id).ok_or(FishTaskError::Allocation)?;
    let result = result.and_then(|target| match target {
        Some(target) => publish_target_route(entity, target),
        None => Ok(()),
    });
    entity.actor_tasks.finish_exact_visit(visit);
    result
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FishTaskFrame {
    pub entity_id: u32,
    pub elapsed_micros: u32,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub immutable_anchor_raw: [i16; 3],
}

/// Returns whether the surviving task requests its style+00 C690. The caller
/// applies C690 before freshly reading Secondary; suppressed transitions do
/// not manufacture another callback or repeat the movement prefix.
pub(crate) fn tick_primary<R, E>(
    manager: &mut EntityManager,
    frame: FishTaskFrame,
    next_random: &mut R,
    mut mover: impl FnMut(
        &mut Entity,
        &mut WanderNearPrivateState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
        &mut R,
    ) -> Result<bool, E>,
) -> Result<bool, FishTaskError<E>>
where
    R: FnMut() -> u32,
{
    let entity = manager
        .entity_mut(frame.entity_id)
        .ok_or(FishTaskError::Allocation)?;
    if entity.shared_fish_runtime.is_none() {
        return Err(FishTaskError::Allocation);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(FishTaskError::Graph);
    };
    let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
        return Err(FishTaskError::Graph);
    };
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(FishTaskError::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let position = entity.position_raw();
    match entity.actor_tasks.task_state(task_id) {
        Some(ActorTaskRuntime::SharedRetarget(_)) => {
            if !matches!(program.class_id, 5 | 13) || context.style_table_index_raw_at_0x10() != 0 {
                return Err(FishTaskError::Graph);
            }
            let (prefix, mut stage) = entity
                .actor_tasks
                .begin_exact_visit_with(visit, |runtime| {
                    let ActorTaskRuntime::SharedRetarget(task) = runtime else {
                        unreachable!()
                    };
                    let lifetime = task.before_callback(frame.elapsed_micros);
                    let stage = task.stage_callback(position, || next_random() as u16);
                    (
                        SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget()),
                        stage,
                    )
                })
                .ok_or(FishTaskError::Graph)?;
            let moved = mover(
                entity,
                stage.private_state_mut(),
                RetailRuntimeValue::Known(None),
                next_random,
            );
            if let Some(ActorTaskRuntime::SharedRetarget(task)) =
                entity.actor_tasks.task_state_mut(task_id)
            {
                task.commit_callback_stage(stage);
            }
            let survived = entity.actor_tasks.finish_exact_visit(visit);
            if !survived {
                return Ok(false);
            }
            let moved = moved.map_err(FishTaskError::Mover)?;
            Ok(matches!(
                shared_retarget_after_unwind(visit, prefix, mover_return(moved)),
                SharedRetargetPostUnwind::Transition(_)
            ))
        }
        Some(ActorTaskRuntime::OrdinaryType9Wander(_)) => {
            if program.class_id != 6 || context.style_table_index_raw_at_0x10() != 0 {
                return Err(FishTaskError::Graph);
            }
            let (prefix, mut stage) = entity
                .actor_tasks
                .begin_exact_visit_with(visit, |runtime| {
                    let ActorTaskRuntime::OrdinaryType9Wander(task) = runtime else {
                        unreachable!()
                    };
                    let lifetime_status = task.before_callback(frame.elapsed_micros);
                    let stage = task.stage_callback(frame.immutable_anchor_raw, &mut *next_random);
                    (
                        OrdinaryType9WanderCallbackPrefix {
                            lifetime_status,
                            retarget: stage.retarget(),
                        },
                        stage,
                    )
                })
                .ok_or(FishTaskError::Graph)?;
            let moved = mover(
                entity,
                stage.private_state_mut(),
                RetailRuntimeValue::Known(None),
                next_random,
            );
            if let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
                entity.actor_tasks.task_state_mut(task_id)
            {
                stage.commit(task);
            }
            let survived = entity.actor_tasks.finish_exact_visit(visit);
            if !survived {
                return Ok(false);
            }
            let moved = moved.map_err(FishTaskError::Mover)?;
            Ok(matches!(
                ordinary_type9_wander_after_unwind(
                    visit,
                    prefix,
                    map_common_mover_return(mover_return(moved))
                ),
                OrdinaryType9WanderPostUnwind::Transition(_)
            ))
        }
        Some(ActorTaskRuntime::FishTargetRoute(_)) => {
            tick_route(manager, frame, visit, next_random, mover)
        }
        _ => Err(FishTaskError::Graph),
    }
}

fn mover_return(moved: bool) -> WanderNearCommonMoverReturn {
    if moved {
        WanderNearCommonMoverReturn::NonZero
    } else {
        WanderNearCommonMoverReturn::Zero
    }
}

fn tick_route<R, E>(
    manager: &mut EntityManager,
    frame: FishTaskFrame,
    visit: ActorTaskVisit,
    next_random: &mut R,
    mut mover: impl FnMut(
        &mut Entity,
        &mut WanderNearPrivateState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
        &mut R,
    ) -> Result<bool, E>,
) -> Result<bool, FishTaskError<E>>
where
    R: FnMut() -> u32,
{
    let entity = manager
        .entity_mut(frame.entity_id)
        .ok_or(FishTaskError::Allocation)?;
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(FishTaskError::Graph);
    };
    let Some(ActorTaskRuntime::FishTargetRoute(task)) =
        entity.actor_tasks.task_state(visit.task_id)
    else {
        return Err(FishTaskError::Graph);
    };
    if context.descriptor() != BehaviorDescriptorIdentity::Named(behavior_program(13).unwrap())
        || context.style_table_index_raw_at_0x10() != 1
        || context.target_handle_at_0x08() != RetailRuntimeValue::Known(task.target_id())
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(FishTaskError::Graph);
    }
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::FishTargetRoute(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(),
            )
        })
        .ok_or(FishTaskError::Graph)?;
    let target_id = stage.private_state().tracked_entity_handle;
    let target = manager
        .iter_all()
        .find(|e| e.active && e.id == target_id)
        .map(|e| CommonMoverTrackedTargetSnapshot {
            state_flags: e.collision.state_flags_at_0x08,
            position_raw: e.position_raw(),
            velocity_raw: e.velocity_raw(),
        });
    let entity = manager.entity_mut(frame.entity_id).unwrap();
    let position = entity.position_raw();
    let mut axis = entity.actor_common_axis_descriptor;
    let result = evaluate_shared_target_route_callback(
        &mut stage,
        SharedTargetRouteCallbackRequest {
            visit,
            entity_id: frame.entity_id,
            movement_state: entity,
            controller_context: &mut axis,
            elapsed_micros: frame.elapsed_micros,
            scheduler_mode: match frame.dispatch_mode {
                CommonMoverDispatchMode::Normal => 0,
                CommonMoverDispatchMode::Restricted => 1,
            },
        },
        |_| {
            let Some(target) = target else {
                return Ok(SharedTargetRouteTargetRuntimeState::Missing);
            };
            let RetailRuntimeValue::Known(dying) = target.state_flags.masked(DYING_STATE_BIT)
            else {
                return Err(FishTaskError::Runtime("fish target state"));
            };
            if dying != 0 {
                return Ok(SharedTargetRouteTargetRuntimeState::Dying);
            }
            if target.state_flags.known_value_bits() != 0 {
                return Ok(SharedTargetRouteTargetRuntimeState::Live);
            }
            if target.state_flags.known_mask() == u32::MAX {
                return Ok(SharedTargetRouteTargetRuntimeState::Inactive);
            }
            Err(FishTaskError::Runtime("fish target state"))
        },
        |request| {
            let RetailRuntimeValue::Known(axis) = *request.controller_context else {
                return Err(FishTaskError::Runtime("fish route axis"));
            };
            Ok(
                if within_wrapped_axis_range(
                    WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
                    position,
                    target.unwrap().position_raw,
                ) {
                    SharedTargetRoutePredicate::NonZero
                } else {
                    SharedTargetRoutePredicate::Zero
                },
            )
        },
        |request| {
            mover(
                request.movement_state,
                request.target_state,
                RetailRuntimeValue::Known(target),
                next_random,
            )
            .map_err(FishTaskError::Mover)
            .map(|moved| {
                if moved {
                    SharedTargetRouteCommonMoverReturn::NonZero
                } else {
                    SharedTargetRouteCommonMoverReturn::Zero
                }
            })
        },
    );
    if let Some(ActorTaskRuntime::FishTargetRoute(task)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    {
        stage.commit(task);
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    if !survived {
        return Ok(false);
    }
    let result = result.map_err(|error| match error {
        SharedTargetRouteCallbackError::TargetValidation { error, .. }
        | SharedTargetRouteCallbackError::RoutePredicate { error, .. }
        | SharedTargetRouteCallbackError::CommonMover { error, .. } => error,
    })?;
    Ok(shared_target_route_transition_after_unwind(visit, prefix, result).is_some())
}

#[cfg(test)]
mod tests;
