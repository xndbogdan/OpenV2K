//! Type53 allocation/mover binding for native Intro2 class9 callbacks.

use super::*;
use crate::{
    entity::EntityManager,
    intro2_capture_pursuit::{
        self as capture, Intro2CaptureBlock, Intro2CapturePrimaryFrame, Intro2CaptureProfile,
    },
    shared_target_route::SharedTargetRouteTransitionRequest,
    world_fx::WorldFx,
};

pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    retail_tick: u32,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type53Block> {
    capture::acquire(
        manager,
        id,
        retail_tick,
        world_fx,
        Intro2CaptureProfile::Type53,
    )
    .map_err(|block| map_block(block, |never| match never {}))
}

pub(super) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: super::mover::MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<Option<SharedTargetRouteTransitionRequest>, Intro2Type53Block> {
    capture::tick_primary(
        manager,
        Intro2CapturePrimaryFrame {
            entity_id: id,
            elapsed_micros: frame.elapsed_micros,
            dispatch_mode: frame.dispatch_mode,
        },
        |entity, target, tracked_target| {
            super::mover::run(entity, frame, target, tracked_target, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
        },
    )
    .map_err(|block| map_block(block, |block| block))
}

fn map_block<E>(
    block: Intro2CaptureBlock<E>,
    mover: impl FnOnce(E) -> Intro2Type53Block,
) -> Intro2Type53Block {
    match block {
        Intro2CaptureBlock::Allocation => Intro2Type53Block::Allocation,
        Intro2CaptureBlock::Graph => Intro2Type53Block::Graph,
        Intro2CaptureBlock::Metadata => Intro2Type53Block::Metadata,
        Intro2CaptureBlock::Runtime(reason) => Intro2Type53Block::Runtime(reason),
        Intro2CaptureBlock::Acquisition(reason) => Intro2Type53Block::Acquisition(reason),
        Intro2CaptureBlock::Mover(reason) => mover(reason),
    }
}

#[cfg(test)]
mod tests;
