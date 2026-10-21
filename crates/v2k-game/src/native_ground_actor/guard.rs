//! Class6 Wander Near Location and class32 Guard Location on the ground host:
//! `02EB0`'s anchor wander through the profile's own mover, and the shared
//! `01FB0` search whose `C7D0` installs Search's Chase and Aim.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskVisit,
    guard_location_owner::native::{tick_native_guard_acquisition, NativeGuardBlock},
    ordinary_type9_wander_owner::{
        ordinary_type9_wander_after_unwind, OrdinaryType9WanderCallbackPrefix,
        OrdinaryType9WanderPostUnwind,
    },
    wander_near_location::{map_common_mover_return, WanderNearCommonMoverReturn},
};

/// One `02EB0` visit around the profile's immutable `+90` anchor. Returns
/// whether the scheduler requests the root's `C690`.
pub(crate) fn tick_wander_primary<P: NativeGroundActorProfile>(
    entity: &mut Entity,
    frame: super::mover::MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let anchor = P::wander_anchor(entity).ok_or(Block::Runtime("wander anchor"))?;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Block::Graph)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let (lifetime_status, mut stage) = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::OrdinaryType9Wander(task) = runtime else {
                unreachable!()
            };
            let lifetime = task.before_callback(frame.elapsed_micros);
            let stage = task.stage_callback(anchor, || {
                u32::from(world_fx.next_shared_retail_random_u16())
            });
            (lifetime, stage)
        })
        .ok_or(Block::Graph)?;
    let moved = super::mover::run::<P>(
        entity,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
    );
    let prefix = OrdinaryType9WanderCallbackPrefix {
        lifetime_status,
        retarget: stage.retarget(),
    };
    if let Some(ActorTaskRuntime::OrdinaryType9Wander(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    {
        // The anchor retarget precedes the mover's first classifier boundary.
        stage.commit(task);
    }
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Block::Graph);
    }
    let result = map_common_mover_return(if moved? {
        WanderNearCommonMoverReturn::NonZero
    } else {
        WanderNearCommonMoverReturn::Zero
    });
    match ordinary_type9_wander_after_unwind(visit, prefix, result) {
        OrdinaryType9WanderPostUnwind::Continue => Ok(false),
        OrdinaryType9WanderPostUnwind::Transition(_) => Ok(true),
        OrdinaryType9WanderPostUnwind::UnresolvedCommonMover => Err(Block::Runtime("wander mover")),
    }
}

/// One `01FB0` visit of Guard's search; `C7D0` may take its candidate.
pub(crate) fn acquire<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    let metadata = manager
        .type_runtime_metadata(P::ENTITY_TYPE)
        .cloned()
        .ok_or(Block::Metadata)?;
    let taken =
        tick_native_guard_acquisition(manager, id, &metadata, world_fx).map_err(|reason| {
            match reason {
                NativeGuardBlock::Graph => Block::Graph,
                NativeGuardBlock::Axis => Block::Runtime("Guard search axis"),
                NativeGuardBlock::CandidateState => Block::Runtime("Guard candidate state"),
                NativeGuardBlock::Pursuit => Block::Runtime("ADE0 task publication"),
            }
        })?;
    if taken
        && !manager
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(super::search::pursuing_graph_authenticates::<P>)
    {
        return Err(Block::Runtime("ADE0 task publication"));
    }
    Ok(())
}
