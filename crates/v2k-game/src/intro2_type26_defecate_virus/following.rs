//! Source class33 acquisition/handoff for the native Type26 allocation.
use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode, entity::EntityManager,
    follow_beacons::FollowBeaconsFollowingPostUnwind,
};
pub(super) fn secondary(
    manager: &mut EntityManager,
    id: u32,
    fx: &mut WorldFx,
) -> Result<(), Intro2Type26WorldBlock> {
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Intro2Type26WorldBlock::Allocation)?;
    match entity.actor_task_state(ActorTaskSlot::Secondary) {
        None => Ok(()),
        Some(ActorTaskRuntime::FollowBeaconAcquisition(_)) => {
            use crate::follow_beacons::live_acquisition::*;
            let metadata = manager
                .type_runtime_metadata(26)
                .cloned()
                .ok_or(Intro2Type26WorldBlock::Metadata)?;
            tick_follow_beacons_live_acquisition(
                manager,
                FollowBeaconsLiveAcquisitionRequest {
                    entity_id: id,
                    metadata: &metadata,
                },
                &mut || u32::from(fx.next_shared_retail_random_u16()),
            )
            .map(|_| ())
            .map_err(Intro2Type26WorldBlock::FollowAcquisition)
        }
        _ => Err(Intro2Type26WorldBlock::Graph),
    }
}

pub(super) fn primary(
    manager: &mut EntityManager,
    id: u32,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    dt: u32,
    global_dt: u32,
    mode: CommonMoverDispatchMode,
    fx: &mut WorldFx,
) -> Result<bool, Intro2Type26WorldBlock> {
    use crate::follow_beacons::live_primary::*;
    let result = tick_follow_beacons_live_primary(
        manager,
        FollowBeaconsLivePrimaryRequest {
            entity_id: id,
            callback_elapsed_micros: dt,
        },
        |request| {
            super::mover::run(
                request.entity,
                super::mover::MoverFrame {
                    metadata,
                    terrain,
                    dispatch_mode: mode,
                    elapsed_micros: dt,
                    global_elapsed_micros: global_dt,
                },
                request.target,
                request.tracked_target,
                &mut || u32::from(fx.next_shared_retail_random_u16()),
            )
        },
    )
    .map_err(|error| match error {
        FollowBeaconsLivePrimaryError::Mover(error) => error,
        _ => Intro2Type26WorldBlock::Runtime("following task phase"),
    })?;
    Ok(matches!(
        result,
        FollowBeaconsFollowingPostUnwind::Transition(_)
    ))
}
