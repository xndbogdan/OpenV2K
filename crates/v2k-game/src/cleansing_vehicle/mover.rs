//! Type49 binds A/B/C/D to the shared01430 machine with its authored water lift.
use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_common_mover::{
        run_intro2_common_mover, Intro2CommonMoverBlock, Intro2CommonMoverFrame,
    },
    wander_near_location::WanderNearPrivateState,
};
use v2k_formats::terrain::TerrainGrid;

pub(crate) struct MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub wave_tick_50hz: Option<i32>,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

pub(crate) fn run(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2CommonMoverBlock> {
    if !entity_authenticates(entity) || authenticate_metadata(frame.metadata).is_err() {
        return Err(Intro2CommonMoverBlock::Runtime(
            "cleansing allocation or metadata",
        ));
    }
    let mut runtime = entity.cleansing_vehicle_runtime.unwrap();
    let result = run_intro2_common_mover(
        entity,
        Intro2CommonMoverFrame {
            metadata: frame.metadata,
            topology: TOPOLOGY,
            terrain: frame.terrain,
            wave_tick_50hz: frame.wave_tick_50hz,
            dispatch_mode: frame.dispatch_mode,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
            sub_d_runtime: &mut runtime.sub_d_runtime,
            sub_d_owner: &mut runtime.sub_d_owner,
            kl_components: None,
            target: Some(target),
            tracked_target: RetailRuntimeValue::Known(None),
        },
        next_random,
    );
    entity.cleansing_vehicle_runtime = Some(runtime);
    result
}
