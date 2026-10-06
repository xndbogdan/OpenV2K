//! Actor-specific allocation binding for the shared native Intro2 01430 phase.

use super::*;
use crate::common_mover::{
    component_dispatch::CommonMoverDispatchMode, target_prelude::CommonMoverTrackedTargetSnapshot,
};
use crate::intro2_common_mover::{
    run_intro2_common_mover, Intro2CommonMoverBlock, Intro2CommonMoverFrame,
};
use crate::wander_near_location::WanderNearPrivateState;

#[derive(Clone, Copy)]
pub struct MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

pub(crate) fn run<P: NativeGroundActorProfile>(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, NativeGroundActorBlock> {
    if !P::metadata_authenticates(frame.metadata) {
        return Err(NativeGroundActorBlock::Metadata);
    }
    if !P::allocation_authenticates(entity) {
        return Err(NativeGroundActorBlock::Allocation);
    }
    let runtime = P::sub_d_state(entity).ok_or(NativeGroundActorBlock::Allocation)?;
    if !runtime.origin.authenticates_entity(entity) {
        return Err(NativeGroundActorBlock::Allocation);
    }
    let mut sub_d_runtime = runtime.runtime;
    let mut sub_d_owner = runtime.owner;
    let mut kl_components = P::kl_components(entity);
    let result = run_intro2_common_mover(
        entity,
        Intro2CommonMoverFrame {
            metadata: frame.metadata,
            topology: P::TOPOLOGY,
            terrain: frame.terrain,
            wave_tick_50hz: None,
            dispatch_mode: frame.dispatch_mode,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
            sub_d_runtime: &mut sub_d_runtime,
            sub_d_owner: &mut sub_d_owner,
            kl_components: kl_components.as_mut(),
            target: Some(target),
            tracked_target,
        },
        next_random,
    );
    // Restore both words and the first-query/cadence owner on success or a
    // later block. The shared helper does not own allocation identity.
    P::store_sub_d_state(
        entity,
        NativeGroundSubDState {
            runtime: sub_d_runtime,
            owner: sub_d_owner,
            ..runtime
        },
    );
    // K/L share the allocation's native variable bank. Preserve their target,
    // smoothing and output writes even when a later H/C/A boundary blocks.
    if let Some(components) = kl_components {
        P::store_kl_components(entity, components);
    }
    result.map_err(|block| map_block(block, runtime.origin))
}

fn map_block(
    block: Intro2CommonMoverBlock,
    origin: NativeGroundAllocationOrigin,
) -> NativeGroundActorBlock {
    match block {
        Intro2CommonMoverBlock::UnsupportedTopology => NativeGroundActorBlock::Metadata,
        Intro2CommonMoverBlock::Runtime(reason) => NativeGroundActorBlock::Runtime(reason),
        Intro2CommonMoverBlock::SubDFirstQuery { seed } => {
            NativeGroundActorBlock::SubDFirstQuery { origin, seed }
        }
        Intro2CommonMoverBlock::SubD(reason) => NativeGroundActorBlock::SubD(reason),
        Intro2CommonMoverBlock::SubH(reason) => NativeGroundActorBlock::SubH(reason),
        Intro2CommonMoverBlock::Mover(reason) => NativeGroundActorBlock::Mover(reason),
        Intro2CommonMoverBlock::MoverAction(reason) => NativeGroundActorBlock::MoverAction(reason),
        Intro2CommonMoverBlock::MoverAdvance(reason) => {
            NativeGroundActorBlock::MoverAdvance(reason)
        }
    }
}
