//! Type26's native A/B/C/D/H binding of 01430. Geometry and integration are outer phases.

use super::*;
use crate::common_mover::{
    component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase},
    frame_machine::*,
    sub_c::SubCSurfaceSample,
    sub_d::{apply_type9_sub_d, Type9SubDStep},
    target_prelude::{CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot},
};
use crate::hover::HoverLiftConfig;
use crate::wander_near_location::WanderNearPrivateState;

pub(super) struct MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

pub(super) fn run(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2Type26WorldBlock> {
    use Intro2Type26WorldBlock as Block;
    let metadata = frame.metadata;
    crate::type26_common_mover::Type26CommonMoverTopology::from_metadata(26, metadata)
        .map_err(|_| Block::Metadata)?;
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(b)) = metadata.sub_b_lateral_descriptor else {
        return Err(Block::Runtime("Sub-B descriptor"));
    };
    let RetailRuntimeValue::Known(Some(c)) = metadata.sub_c_lift_descriptor else {
        return Err(Block::Runtime("Sub-C descriptor"));
    };
    let RetailRuntimeValue::Known(Some(d)) = metadata.sub_d_steering_descriptor else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        unreachable!()
    };
    if c.surface_mode_raw != 0 {
        return Err(Block::Runtime("Sub-C surface mode"));
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    let [heading, _, roll] = entity.rotation_heading_pitch_roll_raw();
    let mut snapshot = CommonMoverFrameSnapshot {
        controlled_entity_lookup: None,
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        heading_raw: heading as u16,
        roll_raw: roll as u16,
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
                topology: INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY,
                dispatch_mode: frame.dispatch_mode,
                elapsed_micros: frame.elapsed_micros,
                sub_a_descriptor: Some(a),
                sub_b_descriptor: Some(b),
                sub_c_descriptor: Some(HoverLiftConfig::from(c)),
                sub_d_descriptor: Some(CommonMoverPreludeSubD {
                    steering_divisor_raw: d.steering_divisor_raw,
                    couple_yaw_into_roll: d.couple_yaw_into_roll_raw != 0,
                }),
                target_resource_context_present: true,
            },
            initial_snapshot: snapshot,
            target_private: Some(*target),
            tracked_target,
        },
        next_random,
    )
    .map_err(Block::Mover)?;
    for _ in 0..24 {
        let resume = match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) => return Ok(false),
            CommonMoverFramePoll::ReturnOne => return Ok(true),
            CommonMoverFramePoll::Blocked(reason) => return Err(Block::Mover(reason)),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(plan)) => {
                *target = plan.target_state;
                snapshot.heading_raw = plan.heading_raw;
                snapshot.roll_raw = plan.roll_raw;
                if let Some(a) = plan.sub_a_runtime {
                    snapshot.sub_a_runtime = RetailRuntimeValue::Known(Some(a));
                }
                if let Some(write) = plan.sub_d_reversal_write {
                    entity
                        .intro2_type26_sub_d_runtime
                        .as_mut()
                        .unwrap()
                        .last_yaw_step_raw = write.step_raw as i16;
                }
                commit_snapshot(entity, snapshot);
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::TargetPrelude,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD {
                target_position_raw,
                ..
            }) => {
                let runtime = entity
                    .intro2_type26_sub_d_runtime
                    .as_mut()
                    .ok_or(Block::Allocation)?;
                let owner = entity
                    .intro2_type26_sub_d_frame_owner
                    .as_mut()
                    .ok_or(Block::Allocation)?;
                if !owner.classifier_cache().can_classify() {
                    return Err(Block::SubDFirstQuery {
                        spawn_index: entity.authored_spawn_index.unwrap(),
                        seed: owner.classifier_cache().stagger_counter(),
                    });
                }
                let RetailRuntimeValue::Known(Some(a)) = snapshot.sub_a_runtime else {
                    return Err(Block::Runtime("Sub-A allocation"));
                };
                let evidence = owner.evidence_for_frame_with_descriptor(
                    d,
                    frame.terrain,
                    snapshot.position_raw,
                    target_position_raw,
                    basis.lateral,
                    basis.forward,
                    a.direction_multiplier(),
                );
                let yaw = match apply_type9_sub_d(
                    d,
                    runtime,
                    evidence,
                    frame.elapsed_micros,
                    frame.global_elapsed_micros,
                ) {
                    Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                    reason => return Err(Block::SubD(reason)),
                };
                snapshot.heading_raw = snapshot.heading_raw.wrapping_sub(yaw as u16);
                // 20360 changes angles, not the matrix read by the C/A/B tail.
                commit_snapshot(entity, snapshot);
                CommonMoverFrameResume::SubDReturned {
                    result_raw: yaw,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubH,
                ..
            }) => {
                let RetailRuntimeValue::Known(Some(runtime)) =
                    &mut entity.sub_h_external_frame_runtime
                else {
                    return Err(Block::Runtime("Sub-H allocation"));
                };
                let cues = runtime
                    .update(h, frame.elapsed_micros as i32)
                    .map_err(Block::SubH)?;
                if !cues.is_empty() {
                    return Err(Block::Runtime("unexpected Sub-H cue"));
                }
                CommonMoverFrameResume::ComponentReturned {
                    phase: CommonMoverDispatchPhase::SubH,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface: false,
            }) => CommonMoverFrameResume::SubCSurfaceSampled(SubCSurfaceSample::Terrain {
                terrain_y_raw: frame
                    .terrain
                    .bilinear_height_raw(point_raw[0], point_raw[1]),
            }),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC {
                position_raw,
                velocity_raw,
                ..
            }) => {
                snapshot.position_raw = position_raw;
                snapshot.velocity_raw = velocity_raw;
                commit_snapshot(entity, snapshot);
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
                commit_snapshot(entity, snapshot);
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::SubA,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { velocity_raw }) => {
                snapshot.velocity_raw = velocity_raw;
                commit_snapshot(entity, snapshot);
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::SubB,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(action) => return Err(Block::MoverAction(action)),
        };
        machine.resume(resume).map_err(Block::MoverAdvance)?;
    }
    Err(Block::Runtime("01430 did not terminate"))
}

fn commit_snapshot(entity: &mut Entity, snapshot: CommonMoverFrameSnapshot) {
    let rotation = entity.rotation_heading_pitch_roll_raw();
    entity.set_rotation_heading_pitch_roll_raw([
        snapshot.heading_raw as i16,
        rotation[1],
        snapshot.roll_raw as i16,
    ]);
    entity.set_motion_raw(snapshot.position_raw, snapshot.velocity_raw);
    entity.sub_a_propulsion_runtime = snapshot.sub_a_runtime;
}
