//! Class12's null-target 01430 phase, with the shared component machine.

use super::*;
use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::*;
use crate::common_mover::sub_c::{sample_sub_c_world_surface, SubCWaveClock};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::hover::HoverLiftConfig;
use crate::sub_h_external_frame::SubHRuntimeState;
use v2k_formats::terrain::TerrainGrid;

pub(super) struct MoverPlan {
    pub snapshot: CommonMoverFrameSnapshot,
    pub sub_h: SubHRuntimeState,
}

/// Existing no-K/L families retain their planned component phase. Native
/// ABCDEHKL owns a live variable bank whose ordered callback prefix must survive
/// a later C/A block, so it executes against that retained allocation.
pub(super) enum MoverExecution {
    Planned(MoverPlan),
    RetainedKl,
}

pub(super) fn run_retained_kl_mover(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    elapsed_micros: u32,
    global_elapsed_micros: u32,
) -> Result<(), Intro2CommonDyingBlock> {
    use crate::intro2_common_mover::{
        run_intro2_common_mover, Intro2CommonMoverBlock, Intro2CommonMoverFrame,
    };
    use Intro2CommonDyingBlock as Block;
    authenticate_components(entity, metadata)?;
    if !crate::native_type30::allocation_authenticates(entity) {
        return Err(Block::UnauthenticatedAllocation);
    }
    if !matches!(&entity.sub_h_external_frame_runtime, RetailRuntimeValue::Known(Some(h)) if !h.is_enabled())
    {
        return Err(Block::Runtime("class12 Sub-H must be disabled"));
    }
    let mut runtime = entity.native_type30_runtime.take().unwrap();
    let result = run_intro2_common_mover(
        entity,
        Intro2CommonMoverFrame {
            metadata,
            topology: crate::native_type30::TOPOLOGY,
            terrain,
            // Canonical Type30 C samples solid terrain, with both wave and offset
            // flags clear. No world clock is read by this source component phase.
            wave_tick_50hz: None,
            dispatch_mode: CommonMoverDispatchMode::Normal,
            elapsed_micros,
            global_elapsed_micros,
            sub_d_runtime: &mut runtime.sub_d_runtime,
            sub_d_owner: &mut runtime.sub_d_owner,
            kl_components: Some(&mut runtime.kl_components),
            target: None,
            tracked_target: RetailRuntimeValue::Known(None),
        },
        &mut || panic!("class12 NULL target cannot enter target RNG"),
    );
    // The same bank and K/L inputs survive task replacement and callback
    // errors. No constructor, smoothing reset or captured identity is used.
    entity.native_type30_runtime = Some(runtime);
    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(Block::Runtime("null-target mover returned zero")),
        Err(error) => Err(match error {
            Intro2CommonMoverBlock::UnsupportedTopology => Block::Metadata("native K/L topology"),
            Intro2CommonMoverBlock::Runtime(reason) => Block::Runtime(reason),
            Intro2CommonMoverBlock::SubH(_) => Block::Runtime("Sub-H update"),
            Intro2CommonMoverBlock::Mover(reason) => Block::Mover(reason),
            Intro2CommonMoverBlock::MoverAction(reason) => Block::MoverAction(reason),
            Intro2CommonMoverBlock::MoverAdvance(reason) => Block::MoverAdvance(reason),
            Intro2CommonMoverBlock::SubDFirstQuery { .. } | Intro2CommonMoverBlock::SubD(_) => {
                Block::Runtime("null-target mover entered Sub-D")
            }
        }),
    }
}

pub(super) fn plan_mover(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    basis: Type9BodyBasis,
    terrain: &TerrainGrid,
    elapsed_micros: u32,
    wave_clock: SubCWaveClock,
) -> Result<MoverPlan, Intro2CommonDyingBlock> {
    use Intro2CommonDyingBlock as Block;
    authenticate_components(entity, metadata)?;
    let RetailRuntimeValue::Known(topology) = metadata.common_mover_topology else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(sub_a)) = metadata.sub_a_propulsion_descriptor else {
        return Err(Block::Metadata("Sub-A descriptor"));
    };
    let RetailRuntimeValue::Known(Some(sub_b)) = metadata.sub_b_lateral_descriptor else {
        return Err(Block::Metadata("Sub-B descriptor"));
    };
    let RetailRuntimeValue::Known(Some(sub_c)) = metadata.sub_c_lift_descriptor else {
        return Err(Block::Metadata("Sub-C descriptor"));
    };
    let RetailRuntimeValue::Known(Some(sub_h_descriptor)) =
        &metadata.sub_h_external_frame_descriptor
    else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
        unreachable!()
    };
    if sub_h.is_enabled() {
        return Err(Block::Runtime("class12 Sub-H must be disabled"));
    }
    let rotation = entity.rotation_heading_pitch_roll_raw();
    let mut snapshot = CommonMoverFrameSnapshot {
        controlled_entity_lookup: None,
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        heading_raw: rotation[0] as u16,
        roll_raw: rotation[2] as u16,
        body_up_q31: basis.up,
        body_right_q31: basis.lateral,
        body_forward_q31: basis.forward,
        attached_cargo_mass: 0,
        sub_a_runtime: entity.sub_a_propulsion_runtime,
        sub_f_smoothed_raw: None,
        sub_g_runtime_angle_raw: 0,
        sub_g_state_byte_3c: 0,
        sub_k_smoothed_raw: None,
        sub_n_accumulator_raw: None,
        sub_o_link_raw: 0,
    };
    let mut machine = CommonMoverFrameMachine::start(
        CommonMoverFrameRequest {
            configuration: CommonMoverFrameConfiguration {
                controlled_entity_handle: entity.id,
                topology,
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros,
                sub_a_descriptor: Some(sub_a),
                sub_b_descriptor: Some(sub_b),
                sub_c_descriptor: Some(HoverLiftConfig::from(sub_c)),
                sub_d_descriptor: None,
                target_resource_context_present: false,
            },
            initial_snapshot: snapshot,
            target_private: None,
            tracked_target: RetailRuntimeValue::Unresolved,
        },
        || panic!("class12 null target cannot enter target RNG"),
    )
    .map_err(Block::Mover)?;
    let mut sub_h = sub_h.clone();
    for _ in 0..16 {
        let resume = match machine.poll() {
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubH,
                controlled_entity_handle,
                elapsed_micros: dt,
            }) if controlled_entity_handle == entity.id && dt == elapsed_micros => {
                let cues = sub_h
                    .update(sub_h_descriptor, elapsed_micros as i32)
                    .map_err(|_| Block::Runtime("Sub-H update"))?;
                if !cues.is_empty() {
                    return Err(Block::Runtime("disabled Sub-H emitted cues"));
                }
                CommonMoverFrameResume::ComponentReturned {
                    phase: CommonMoverDispatchPhase::SubH,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface,
            }) => CommonMoverFrameResume::SubCSurfaceSampled(sample_sub_c_world_surface(
                terrain,
                point_raw,
                use_wave_surface.then_some(wave_clock),
            )),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC {
                position_raw,
                velocity_raw,
                ..
            }) => {
                snapshot.position_raw = position_raw;
                snapshot.velocity_raw = velocity_raw;
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::SubC,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubA {
                velocity_raw,
                ..
            }) => {
                snapshot.velocity_raw = velocity_raw;
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::SubA,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { velocity_raw }) => {
                snapshot.velocity_raw = velocity_raw;
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::SubB,
                    snapshot,
                }
            }
            CommonMoverFramePoll::ReturnOne => return Ok(MoverPlan { snapshot, sub_h }),
            CommonMoverFramePoll::Action(action) => return Err(Block::MoverAction(action)),
            CommonMoverFramePoll::Blocked(reason) => return Err(Block::Mover(reason)),
            CommonMoverFramePoll::ReturnZero(_) => {
                return Err(Block::Runtime("null-target mover returned zero"))
            }
        };
        machine.resume(resume).map_err(Block::MoverAdvance)?;
    }
    Err(Block::Runtime("null-target mover did not terminate"))
}
