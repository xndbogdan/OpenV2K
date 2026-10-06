//! Native ground-actor class7 graph and shared 03490 Chase binding.

use super::{mover::MoverFrame, NativeGroundActorBlock, NativeGroundActorProfile};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    chase_target::live_primary::{
        tick_chase_target_live_primary, ChaseTargetLivePrimaryError, ChaseTargetLivePrimaryFrame,
    },
    entity::{Entity, EntityManager},
    entity_collision_state::RetailRuntimeValue,
    world_fx::WorldFx,
};

/// ADE0 publishes Chase and Aim against the same target. The actual emitter
/// belongs to the profile; Aim runs at the later Tertiary visit.
pub(crate) fn pursuing_graph_authenticates<P: NativeGroundActorProfile>(entity: &Entity) -> bool {
    if !P::allocation_authenticates(entity) {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    if context.active_style().style_address() != 0x004c_7a98 {
        return false;
    }
    let RetailRuntimeValue::Known(Some(target)) = context.target_handle_at_0x08() else {
        return false;
    };
    matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == target)
        && matches!(entity.actor_task_state(ActorTaskSlot::Tertiary), Some(ActorTaskRuntime::AimAndFire(task)) if task.private_state().target_entity_id() == target)
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
}

pub(crate) fn tick_primary<P: NativeGroundActorProfile>(
    manager: &mut EntityManager,
    id: u32,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, NativeGroundActorBlock> {
    use NativeGroundActorBlock as Block;
    if !manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(pursuing_graph_authenticates::<P>)
    {
        return Err(Block::Graph);
    }
    tick_chase_target_live_primary(
        manager,
        ChaseTargetLivePrimaryFrame {
            entity_id: id,
            elapsed_micros: frame.elapsed_micros,
            dispatch_mode: frame.dispatch_mode,
        },
        |request| {
            super::mover::run::<P>(
                request.entity,
                MoverFrame {
                    elapsed_micros: request.elapsed_micros,
                    dispatch_mode: request.dispatch_mode,
                    ..frame
                },
                request.target,
                request.tracked_target,
                &mut || u32::from(world_fx.next_shared_retail_random_u16()),
            )
        },
    )
    .map(|transition| transition.is_some())
    .map_err(|error| match error {
        ChaseTargetLivePrimaryError::Allocation => Block::Allocation,
        ChaseTargetLivePrimaryError::Graph => Block::Graph,
        ChaseTargetLivePrimaryError::Runtime(reason) => Block::Runtime(reason),
        ChaseTargetLivePrimaryError::Mover(reason) => reason,
    })
}
