//! Type17's actual metadata/allocation binding for shared native class10.
use super::*;
use crate::{entity::EntityManager, native_ground_actor::run_away as shared, world_fx::WorldFx};
pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
) -> Result<(), Intro2Type17Block> {
    let metadata = manager
        .type_runtime_metadata(17)
        .cloned()
        .ok_or(Intro2Type17Block::Metadata)?;
    shared::acquire(manager, id, &metadata, fx)
        .map_err(|error| map_error(error, |never| match never {}))
}
pub(super) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: super::mover::MoverFrame<'_>,
    fx: &mut WorldFx,
) -> Result<bool, Intro2Type17Block> {
    shared::tick_primary(
        manager,
        id,
        shared::NativeRunAwayFrame {
            elapsed_micros: frame.elapsed_micros,
            dispatch_mode: frame.dispatch_mode,
        },
        fx,
        |request| {
            super::mover::run(
                request.entity,
                frame,
                request.target,
                request.tracked_target,
                &mut || u32::from(request.world_fx.next_shared_retail_random_u16()),
            )
        },
    )
    .map_err(|error| map_error(error, |error| error))
}
fn map_error<E>(
    error: shared::NativeRunAwayBlock<E>,
    mover: impl FnOnce(E) -> Intro2Type17Block,
) -> Intro2Type17Block {
    match error {
        shared::NativeRunAwayBlock::Allocation => Intro2Type17Block::Allocation,
        shared::NativeRunAwayBlock::Graph => Intro2Type17Block::Graph,
        shared::NativeRunAwayBlock::Runtime(reason) => Intro2Type17Block::Runtime(reason),
        shared::NativeRunAwayBlock::Acquisition(reason) => Intro2Type17Block::Acquisition(reason),
        shared::NativeRunAwayBlock::Mover(reason) => mover(reason),
    }
}
