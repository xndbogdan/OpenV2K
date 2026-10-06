//! Type17 binding to the shared Class9 carrying task kernels.

use super::*;
use crate::native_actor_capture::carry_tasks as shared;
use crate::world_fx::WorldFx;
pub(crate) use shared::carrying_variant;
#[cfg(test)]
pub(crate) use shared::CaptureTargetWrite;

#[cfg(test)]
pub(crate) fn publish_style(
    manager: &mut EntityManager,
    id: u32,
    variant: u8,
    target: CaptureTargetWrite,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type17Block> {
    shared::publish_style(manager, id, variant, target, world_fx)
        .map_err(|error| Intro2Type17Block::CaptureTask(Box::new(error)))
}

pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type17Block> {
    shared::acquire(manager, id, world_fx)
        .map_err(|error| Intro2Type17Block::CaptureTask(Box::new(error)))
}

pub(super) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: super::mover::MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type17Block> {
    shared::tick_primary(
        manager,
        id,
        shared::CaptureTaskFrame {
            metadata: frame.metadata,
            terrain: frame.terrain,
            dispatch_mode: frame.dispatch_mode,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
        },
        world_fx,
    )
    .map_err(|error| Intro2Type17Block::CaptureTask(Box::new(error)))
}

#[cfg(test)]
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    entity_behavior::BehaviorDescriptorIdentity, shared_target_route::SharedTargetRouteLifetime,
};
#[cfg(test)]
use shared::select_beacon;
#[cfg(test)]
mod tests;
