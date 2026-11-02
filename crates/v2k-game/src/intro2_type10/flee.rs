//! Type29's class10 Run Away on the Type10 owner. Style `4C7618` runs
//! Search's `B6C0` (slot1 `02050/02080`, slot0 `02B10`'s 500 ms); `02080`'s
//! target goes to `C7D0`, whose style `4C7660` runs `B3F0`: the 2000-ms
//! `03E20/03F40` flee with `06070`'s Sub-G word ([`crate::native_ground_actor::run_away`]).

use super::{mover::MoverFrame, Intro2Type10Block};
use crate::{
    entity::EntityManager, entity_collision_state::EntityTypeRuntimeMetadata,
    native_ground_actor::run_away as shared, world_fx::WorldFx,
};

fn map<E>(
    error: shared::NativeRunAwayBlock<E>,
    mover: impl FnOnce(E) -> Intro2Type10Block,
) -> Intro2Type10Block {
    match error {
        shared::NativeRunAwayBlock::Allocation => Intro2Type10Block::Allocation,
        shared::NativeRunAwayBlock::Graph => Intro2Type10Block::Graph,
        shared::NativeRunAwayBlock::Runtime(reason) => Intro2Type10Block::Runtime(reason),
        shared::NativeRunAwayBlock::Acquisition(reason) => Intro2Type10Block::Acquisition(reason),
        shared::NativeRunAwayBlock::Mover(reason) => mover(reason),
    }
}

/// One `02080` visit of style `4C7618`'s search.
pub(super) fn acquire(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    world_fx: &mut WorldFx,
) -> Result<(), Intro2Type10Block> {
    shared::acquire(manager, id, metadata, world_fx)
        .map_err(|error| map(error, |never| match never {}))
}

/// One `403F40` visit; returns whether `C690` must reselect.
pub(super) fn tick_primary(
    manager: &mut EntityManager,
    id: u32,
    frame: MoverFrame<'_>,
    world_fx: &mut WorldFx,
) -> Result<bool, Intro2Type10Block> {
    shared::tick_primary(
        manager,
        id,
        shared::NativeRunAwayFrame {
            elapsed_micros: frame.elapsed_micros,
            dispatch_mode: frame.dispatch_mode,
        },
        world_fx,
        |request| {
            super::mover::run(
                request.entity,
                frame,
                request.target,
                request.tracked_target,
                request.world_fx,
            )
        },
    )
    .map_err(|error| map(error, |error| error))
}
