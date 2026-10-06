//! Shared class33 variant-one 03CE0 phase; concrete owners supply their mover.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::target_prelude::CommonMoverTrackedTargetSnapshot,
    entity::{Entity, EntityManager},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowBeaconsLivePrimaryRequest {
    pub entity_id: u32,
    pub callback_elapsed_micros: u32,
}

pub struct FollowBeaconsLiveMoverRequest<'a> {
    pub entity: &'a mut Entity,
    pub target: &'a mut WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowBeaconsLivePrimaryError<E> {
    EntityUnavailable,
    GraphMismatch,
    AxisUnavailable,
    TargetStateUnavailable,
    Mover(E),
}

/// Enter only an authenticated class33 variant-one graph. The caller retains
/// allocation custody and owns 12DA0, callback mode, physical components and
/// subsequent world phases. This function never broadens a type admission.
pub fn tick_follow_beacons_live_primary<E>(
    manager: &mut EntityManager,
    request: FollowBeaconsLivePrimaryRequest,
    mover: impl FnOnce(FollowBeaconsLiveMoverRequest<'_>) -> Result<bool, E>,
) -> Result<FollowBeaconsFollowingPostUnwind, FollowBeaconsLivePrimaryError<E>> {
    use FollowBeaconsLivePrimaryError as Error;
    let id = request.entity_id;
    let entity = manager.entity_mut(id).ok_or(Error::EntityUnavailable)?;
    if !matches!(entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == FOLLOW_BEACONS_FOLLOWING_STYLE_ADDRESS)
    {
        return Err(Error::GraphMismatch);
    }
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Error::GraphMismatch)?;
    if !matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::FollowBeaconsFollowing(_))
    ) {
        return Err(Error::GraphMismatch);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (prefix, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::FollowBeaconsFollowing(task) = runtime else {
                unreachable!()
            };
            (
                task.before_callback(request.callback_elapsed_micros),
                task.stage_callback(),
            )
        })
        .ok_or(Error::GraphMismatch)?;
    // 03CE0 captures the target handle before 01430. No later side effect
    // may silently substitute a different task target for this invocation.
    let target_id = stage.private_state().tracked_entity_handle;
    let target = manager
        .iter_all()
        .find(|e| e.active && e.id == target_id)
        .map(|e| CommonMoverTrackedTargetSnapshot {
            state_flags: e.collision.state_flags_at_0x08,
            position_raw: e.position_raw(),
            velocity_raw: e.velocity_raw(),
        });
    let entity = manager.entity_mut(id).ok_or(Error::EntityUnavailable)?;
    let moved = mover(FollowBeaconsLiveMoverRequest {
        entity,
        target: stage.private_state_mut(),
        tracked_target: RetailRuntimeValue::Known(target),
    });
    if let Some(ActorTaskRuntime::FollowBeaconsFollowing(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        // 01430's target/reversal prefix survives later component boundaries.
        stage.commit(task);
    }
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    let moved = moved.map_err(Error::Mover)?;
    // 01120 discards callback results from an inner task retired in 01430.
    // It must not consult the old target or dispatch its tag/timeout afterward.
    if !survived {
        return Ok(FollowBeaconsFollowingPostUnwind::Continue);
    }
    let result = if !moved {
        FollowBeaconsFollowingCallbackResult::Tagged(
            FollowBeaconsFollowingTaggedSingleton::MoverZero,
        )
    } else if let Some(target) = target {
        let state = target.state_flags;
        let RetailRuntimeValue::Known(dying) = state.masked(DYING_STATE_BIT) else {
            return Err(Error::TargetStateUnavailable);
        };
        if dying != 0 || (state.known_mask() == u32::MAX && state.known_value_bits() == 0) {
            return Ok(follow_beacons_following_after_unwind(
                visit,
                prefix,
                FollowBeaconsFollowingCallbackResult::Tagged(
                    FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
                ),
            ));
        }
        if state.known_value_bits() == 0 {
            return Err(Error::TargetStateUnavailable);
        }
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Err(Error::AxisUnavailable);
        };
        if !within_wrapped_axis_range(
            WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            entity.position_raw(),
            target.position_raw,
        ) {
            FollowBeaconsFollowingCallbackResult::Tagged(
                FollowBeaconsFollowingTaggedSingleton::RouteRejected,
            )
        } else {
            // EXE class33 style4C7B70 +18 is null. Reached targets therefore
            // continue; timeout still invokes the primary owner C690 below.
            FollowBeaconsFollowingCallbackResult::Continue
        }
    } else {
        FollowBeaconsFollowingCallbackResult::Tagged(
            FollowBeaconsFollowingTaggedSingleton::InvalidTarget,
        )
    };
    Ok(follow_beacons_following_after_unwind(visit, prefix, result))
}
