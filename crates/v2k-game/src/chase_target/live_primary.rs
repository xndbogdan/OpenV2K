//! Live 03490 wrapper, target custody and Sub-H-gated proximity control.
//!
//! The caller authenticates its behavior graph and supplies its own 01430
//! mover. EXE 40354E reads type+C8+24 (Sub-H at type+EC), not Sub-G.
//! A successful in-range mover precedes this gate and every controller read.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::DYING_STATE_BIT,
};

#[derive(Clone, Copy)]
pub struct ChaseTargetLivePrimaryFrame {
    pub entity_id: u32,
    pub elapsed_micros: u32,
    pub dispatch_mode: CommonMoverDispatchMode,
}

pub struct ChaseTargetLiveMoverRequest<'a> {
    pub entity: &'a mut Entity,
    pub target: &'a mut WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub elapsed_micros: u32,
    pub dispatch_mode: CommonMoverDispatchMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaseTargetLivePrimaryError<E> {
    Allocation,
    Graph,
    Runtime(&'static str),
    Mover(E),
}

pub fn tick_chase_target_live_primary<E>(
    manager: &mut EntityManager,
    frame: ChaseTargetLivePrimaryFrame,
    mut mover: impl FnMut(ChaseTargetLiveMoverRequest<'_>) -> Result<bool, E>,
) -> Result<Option<ChaseTargetTransitionRequest>, ChaseTargetLivePrimaryError<E>> {
    use ChaseTargetLivePrimaryError as Error;
    let id = frame.entity_id;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Error::Allocation)?;
    let entity_type = entity.entity_type;
    if !matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(_))
    ) {
        return Err(Error::Graph);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .ok_or(Error::Graph)?,
    };
    let entity = manager.entity_mut(id).ok_or(Error::Allocation)?;
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::ChaseTarget(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(frame.elapsed_micros),
                task.stage_callback(),
            )
        })
        .ok_or(Error::Graph)?;
    // 01120's age write precedes target/axis reads. Always unwind this exact
    // wrapper, including a later target or mover failure.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let target_id = stage.private_state_mut().tracked_entity_handle;
        if target_id == WANDER_NEAR_NO_TRACKED_ENTITY {
            return Ok(ChaseTargetCallbackResult::Tagged(
                ChaseTargetTaggedSingleton::InvalidTarget,
            ));
        }
        let (target_state, tracked_target) = target_snapshot(manager, target_id)?;
        if !matches!(target_state, ChaseTargetTargetRuntimeState::Live { .. }) {
            return Ok(ChaseTargetCallbackResult::Tagged(
                ChaseTargetTaggedSingleton::InvalidTarget,
            ));
        }
        let entity = manager.entity_mut(id).ok_or(Error::Allocation)?;
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Err(Error::Runtime("Chase actor axis"));
        };
        let result = evaluate_chase_target_callback(
            &mut stage,
            ChaseTargetCallbackRequest {
                visit,
                entity_id: id,
                owner_position_raw: entity.position_raw(),
                route_range: WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
                proximity_control: ChaseTargetProximityControl::Disabled,
                movement_state: &mut (),
                controller_context: &mut (),
                elapsed_micros: frame.elapsed_micros,
                scheduler_mode: match frame.dispatch_mode {
                    CommonMoverDispatchMode::Normal => 0,
                    CommonMoverDispatchMode::Restricted => 1,
                },
            },
            |_| Ok::<_, ()>(target_state),
            |request| {
                mover(ChaseTargetLiveMoverRequest {
                    entity,
                    target: request.target_state,
                    tracked_target,
                    elapsed_micros: request.elapsed_micros,
                    dispatch_mode: frame.dispatch_mode,
                })
                .map(|moved| {
                    if moved {
                        ChaseTargetCommonMoverReturn::NonZero
                    } else {
                        ChaseTargetCommonMoverReturn::Zero
                    }
                })
            },
            |_, _| unreachable!("live post-mover controller is resolved after the mover borrow"),
        )
        .map_err(|error| match error {
            ChaseTargetCallbackError::CommonMover { error, .. } => Error::Mover(error),
            _ => Error::Graph,
        })?;
        match result {
            ChaseTargetCallbackResult::Continue { .. } => {
                apply_post_mover_controller(manager, id, entity_type, target_id, &mut stage)
            }
            tagged => Ok(tagged),
        }
    }));
    let entity = manager.entity_mut(id).ok_or(Error::Allocation)?;
    if let Some(ActorTaskRuntime::ChaseTarget(task)) =
        entity.actor_tasks.task_state_mut(visit.task_id)
    {
        stage.commit(task);
    }
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_none_or(|flags| !flags.in_callback)
    {
        return Err(Error::Graph);
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    let result = match result {
        Ok(result) => result?,
        Err(payload) => std::panic::resume_unwind(payload),
    };
    Ok(survived
        .then(|| chase_target_transition_after_unwind(visit, prefix, result))
        .flatten())
}

/// EXE 40354E..40361F: only a nonzero, in-range mover reaches the H/A
/// controller. Fresh positions and component storage are deliberately read
/// here so a later failure retains the wrapper age and mover/private prefix.
fn apply_post_mover_controller<E>(
    manager: &mut EntityManager,
    id: u32,
    entity_type: u32,
    target_id: u32,
    stage: &mut ChaseTargetCallbackStage,
) -> Result<ChaseTargetCallbackResult, ChaseTargetLivePrimaryError<E>> {
    use ChaseTargetLivePrimaryError as Error;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Error::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .ok_or(Error::Runtime("Chase component topology"))?;
    let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
        return Err(Error::Runtime("Chase component topology"));
    };
    if !topology.sub_h {
        return Ok(ChaseTargetCallbackResult::Continue {
            controller_write: None,
        });
    }
    let owner_position = entity.position_raw();
    let target_position = manager
        .iter_all()
        .find(|entity| entity.id == target_id)
        .ok_or(Error::Runtime("Chase post-mover target"))?
        .position_raw();
    if !topology.sub_a {
        return Ok(ChaseTargetCallbackResult::Continue {
            controller_write: None,
        });
    }
    let delta_x = wrapped_absolute_delta(owner_position[0], target_position[0]);
    let delta_z = wrapped_absolute_delta(owner_position[2], target_position[2]);
    let in_annulus = delta_x < CHASE_TARGET_OUTER_PROXIMITY_RAW
        && delta_z < CHASE_TARGET_OUTER_PROXIMITY_RAW
        && (delta_x >= CHASE_TARGET_INNER_PROXIMITY_RAW
            || delta_z >= CHASE_TARGET_INNER_PROXIMITY_RAW);
    let source_value_raw = if in_annulus {
        // 4035F0 only checks the A pointer and writes literal one. The pure
        // planner does not consume its source value on this branch.
        0
    } else {
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor
        else {
            return Err(Error::Runtime("Chase Sub-A descriptor"));
        };
        descriptor.target_speed_base_raw
    };
    let entity = manager.entity_mut(id).ok_or(Error::Allocation)?;
    let RetailRuntimeValue::Known(Some(sub_a)) = &mut entity.sub_a_propulsion_runtime else {
        return Err(Error::Runtime("Chase Sub-A runtime"));
    };
    let controller_write = chase_target_controller_write(
        stage,
        ChaseTargetProximityControl::EnabledWithSource { source_value_raw },
        owner_position,
        target_position,
    );
    if let Some(write) = controller_write {
        sub_a.apply_shared_initializer_target_speed_write(write.primary_raw);
        if let Some(direction) = write.secondary_raw {
            sub_a.set_direction_multiplier(direction);
        }
    }
    Ok(ChaseTargetCallbackResult::Continue { controller_write })
}

fn target_snapshot<E>(
    manager: &EntityManager,
    id: u32,
) -> Result<
    (
        ChaseTargetTargetRuntimeState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ),
    ChaseTargetLivePrimaryError<E>,
> {
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == id && entity.active)
    else {
        return Ok((
            ChaseTargetTargetRuntimeState::Missing,
            RetailRuntimeValue::Known(None),
        ));
    };
    let flags = entity.collision.state_flags_at_0x08;
    let tracked = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
        state_flags: flags,
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
    }));
    match flags.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return Ok((ChaseTargetTargetRuntimeState::Dying, tracked))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(ChaseTargetLivePrimaryError::Runtime(
                "Chase target dying bit",
            ))
        }
        _ => {}
    }
    let state = if flags.known_value_bits() != 0 {
        ChaseTargetTargetRuntimeState::Live {
            position_raw: entity.position_raw(),
        }
    } else if flags.known_mask() == u32::MAX {
        ChaseTargetTargetRuntimeState::Inactive
    } else {
        return Err(ChaseTargetLivePrimaryError::Runtime("Chase target state"));
    };
    Ok((state, tracked))
}

#[cfg(test)]
mod tests;
