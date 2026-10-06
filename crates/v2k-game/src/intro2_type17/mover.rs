//! Native Type17 allocation binding for the shared ABCDH 01430 phase.

use super::*;
use crate::{
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    intro2_common_mover::{
        run_intro2_common_mover, Intro2CommonMoverBlock, Intro2CommonMoverFrame,
    },
    type17_impact_live::TYPE17_MODEL256_COMPONENT_TOPOLOGY,
    wander_near_location::WanderNearPrivateState,
};

#[derive(Clone, Copy)]
pub(crate) struct MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

pub(crate) fn run(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2Type17Block> {
    authenticate_metadata(frame.metadata).map_err(|_| Intro2Type17Block::Metadata)?;
    if !intro2_type17_allocation_authenticates(entity) {
        return Err(Intro2Type17Block::Allocation);
    }
    let (Some(mut sub_d_runtime), Some(mut sub_d_owner)) =
        (entity.type17_sub_d_runtime, entity.type17_sub_d_frame_owner)
    else {
        return Err(Intro2Type17Block::Runtime("Sub-D custody"));
    };
    let result = run_intro2_common_mover(
        entity,
        Intro2CommonMoverFrame {
            metadata: frame.metadata,
            topology: TYPE17_MODEL256_COMPONENT_TOPOLOGY,
            terrain: frame.terrain,
            wave_tick_50hz: None,
            dispatch_mode: frame.dispatch_mode,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
            sub_d_runtime: &mut sub_d_runtime,
            sub_d_owner: &mut sub_d_owner,
            kl_components: None,
            target: Some(target),
            tracked_target,
        },
        next_random,
    );
    // All committed classifier rows/cadence and target-prefix writes survive
    // a later mover block. Birth identity is never republished at this seam.
    entity.type17_sub_d_runtime = Some(sub_d_runtime);
    entity.type17_sub_d_frame_owner = Some(sub_d_owner);
    result.map_err(|block| match block {
        Intro2CommonMoverBlock::UnsupportedTopology => Intro2Type17Block::Metadata,
        Intro2CommonMoverBlock::Runtime(reason) => Intro2Type17Block::Runtime(reason),
        Intro2CommonMoverBlock::SubDFirstQuery { seed } => Intro2Type17Block::SubDFirstQuery {
            spawn_index: entity.authored_spawn_index.unwrap(),
            seed,
        },
        Intro2CommonMoverBlock::SubD(reason) => Intro2Type17Block::SubD(reason),
        Intro2CommonMoverBlock::SubH(reason) => Intro2Type17Block::SubH(reason),
        Intro2CommonMoverBlock::Mover(reason) => Intro2Type17Block::Mover(reason),
        Intro2CommonMoverBlock::MoverAction(reason) => Intro2Type17Block::MoverAction(reason),
        Intro2CommonMoverBlock::MoverAdvance(reason) => Intro2Type17Block::MoverAdvance(reason),
    })
}
