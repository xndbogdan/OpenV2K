//! Native Type94 binding for the shared 01430 mover; E fires in Aim instead.

use super::*;
use crate::{
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    intro2_common_mover::{
        run_intro2_common_mover, Intro2CommonMoverBlock, Intro2CommonMoverFrame,
    },
    wander_near_location::WanderNearPrivateState,
};

#[derive(Clone, Copy)]
pub(super) struct MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

pub(super) fn run(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2Type94Block> {
    authenticate_metadata(frame.metadata).map_err(|_| Intro2Type94Block::Metadata)?;
    if !intro2_type94_allocation_authenticates(entity) {
        return Err(Intro2Type94Block::Allocation);
    }
    let Some(mut runtime) = entity.intro2_type94_runtime else {
        return Err(Intro2Type94Block::Runtime("Sub-D custody"));
    };
    let result = run_intro2_common_mover(
        entity,
        Intro2CommonMoverFrame {
            metadata: frame.metadata,
            topology: TOPOLOGY,
            terrain: frame.terrain,
            wave_tick_50hz: Some(frame.retail_tick as i32),
            dispatch_mode: frame.dispatch_mode,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
            sub_d_runtime: &mut runtime.sub_d_runtime,
            sub_d_owner: &mut runtime.sub_d_owner,
            kl_components: None,
            target: Some(target),
            tracked_target,
        },
        next_random,
    );
    // All committed classifier rows/cadence and target-prefix writes survive
    // a later mover block. Birth identity is never republished at this seam.
    entity.intro2_type94_runtime = Some(runtime);
    result.map_err(|block| match block {
        Intro2CommonMoverBlock::UnsupportedTopology => Intro2Type94Block::Metadata,
        Intro2CommonMoverBlock::Runtime(reason) => Intro2Type94Block::Runtime(reason),
        Intro2CommonMoverBlock::SubDFirstQuery { seed } => Intro2Type94Block::SubDFirstQuery {
            spawn_index: entity.authored_spawn_index.unwrap(),
            seed,
        },
        Intro2CommonMoverBlock::SubD(reason) => Intro2Type94Block::SubD(reason),
        Intro2CommonMoverBlock::SubH(reason) => Intro2Type94Block::SubH(reason),
        Intro2CommonMoverBlock::Mover(reason) => Intro2Type94Block::Mover(reason),
        Intro2CommonMoverBlock::MoverAction(reason) => Intro2Type94Block::MoverAction(reason),
        Intro2CommonMoverBlock::MoverAdvance(reason) => Intro2Type94Block::MoverAdvance(reason),
    })
}
