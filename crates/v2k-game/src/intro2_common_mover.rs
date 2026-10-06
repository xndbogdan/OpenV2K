//! Shared native A/B/C/D mover with optional H/K/L from retail FUN_00401430.
//!
//! Cohort wrappers authenticate their own metadata/allocation and retain the
//! Sub-D pair and, when present, the native K/L variable bank. This helper
//! commits the target, steering, H/K/L and C/A/B prefixes
//! in source order, including on a later block. It never publishes a body
//! matrix: 20360 only changes angle words, so C/A/B read the incoming nine
//! Q31 dwords. DCA0/E870 rebuild 13F70 after all task visits, optionally after
//! E640. Normal mode admits H/K/L; restricted mode skips their callbacks but
//! retains target/Sub-D writes and C/A/B. A null target skips the complete
//! target/Sub-D prefix while retaining H/K/L component state.

mod kl_components;
pub use kl_components::{Intro2KlComponents, Intro2KlConstructionError};

use crate::common_mover::{
    component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase},
    frame_machine::*,
    sub_c::{sample_sub_c_world_surface, SubCWaveClock},
    sub_d::{apply_type9_sub_d, Type9SubDFrameOwner, Type9SubDRuntime, Type9SubDStep},
    target_prelude::{CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot},
};
use crate::entity::Entity;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::hover::HoverLiftConfig;
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

// J is an attachment owner outside01430. Types16/58 additionally own E, whose
// separate Aim callback invokes24650; neither01430 nor018A0 dispatches E.
const ABCDHJ_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: true,
    sub_b: true,
    sub_c: true,
    sub_d: true,
    sub_e: false,
    sub_f: false,
    sub_g: false,
    sub_h: true,
    sub_i: false,
    sub_j: true,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
const ABCDEHJ_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_e: true,
    ..ABCDHJ_TOPOLOGY
};

const ABCDEH_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_j: false,
    ..ABCDEHJ_TOPOLOGY
};
const ABCD_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_h: false,
    sub_j: false,
    ..ABCDHJ_TOPOLOGY
};
pub(crate) const ABCDEHKL_TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_k: true,
    sub_l: true,
    ..ABCDEH_TOPOLOGY
};

pub(crate) struct Intro2CommonMoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub topology: CommonMoverComponentTopology,
    pub terrain: &'a TerrainGrid,
    /// Required only when the authored Sub-C requests water sampling.
    pub wave_tick_50hz: Option<i32>,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub sub_d_runtime: &'a mut Type9SubDRuntime,
    pub sub_d_owner: &'a mut Type9SubDFrameOwner,
    /// Successful native K/L construction and its independently zeroed bank.
    /// Required exactly for the ABCDEHKL component shape.
    pub kl_components: Option<&'a mut Intro2KlComponents>,
    /// Retail's target-private pointer. Class12 passes a true null pointer.
    pub target: Option<&'a mut WanderNearPrivateState>,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Intro2CommonMoverBlock {
    UnsupportedTopology,
    Runtime(&'static str),
    SubDFirstQuery { seed: u8 },
    SubD(Type9SubDStep),
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    Mover(CommonMoverFrameBlock),
    MoverAction(CommonMoverFrameAction),
    MoverAdvance(CommonMoverFrameAdvanceError),
}

/// Returns retail 01430's zero/nonzero result. Caller-owned Sub-D words and
/// entity/target prefixes are committed even if a later phase blocks.
pub(crate) fn run_intro2_common_mover(
    entity: &mut Entity,
    mut frame: Intro2CommonMoverFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2CommonMoverBlock> {
    use Intro2CommonMoverBlock as Block;
    let metadata = frame.metadata;
    if !matches!(
        frame.topology,
        ABCDHJ_TOPOLOGY | ABCDEHJ_TOPOLOGY | ABCDEH_TOPOLOGY | ABCD_TOPOLOGY | ABCDEHKL_TOPOLOGY
    ) || metadata.common_mover_topology != RetailRuntimeValue::Known(frame.topology)
    {
        return Err(Block::UnsupportedTopology);
    }
    match (frame.topology.sub_k, frame.kl_components.as_deref()) {
        (true, Some(components)) if components.authenticates(metadata) => {}
        (false, None) => {}
        _ => return Err(Block::Runtime("Sub-K/L allocation or variable bindings")),
    }
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        return Err(Block::Runtime("required component descriptor"));
    };
    let RetailRuntimeValue::Known(Some(b)) = metadata.sub_b_lateral_descriptor else {
        return Err(Block::Runtime("Sub-B descriptor"));
    };
    let RetailRuntimeValue::Known(Some(c)) = metadata.sub_c_lift_descriptor else {
        return Err(Block::Runtime("Sub-C descriptor"));
    };
    let RetailRuntimeValue::Known(Some(d)) = metadata.sub_d_steering_descriptor else {
        return Err(Block::Runtime("required component descriptor"));
    };
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
        sub_k_smoothed_raw: frame
            .kl_components
            .as_deref()
            .map(Intro2KlComponents::sub_k_smoothed_raw),
        sub_n_accumulator_raw: None,
        sub_o_link_raw: 0,
    };
    let mut machine = CommonMoverFrameMachine::start(
        CommonMoverFrameRequest {
            configuration: CommonMoverFrameConfiguration {
                controlled_entity_handle: entity.id,
                topology: frame.topology,
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
            target_private: frame.target.as_deref().copied(),
            tracked_target: frame.tracked_target,
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
                *frame
                    .target
                    .as_deref_mut()
                    .ok_or(Block::Runtime("target prelude without target allocation"))? =
                    plan.target_state;
                snapshot.heading_raw = plan.heading_raw;
                snapshot.roll_raw = plan.roll_raw;
                if let Some(a) = plan.sub_a_runtime {
                    snapshot.sub_a_runtime = RetailRuntimeValue::Known(Some(a));
                }
                if let Some(write) = plan.sub_d_reversal_write {
                    frame.sub_d_runtime.last_yaw_step_raw = write.step_raw as i16;
                }
                if let Some(target_raw) = plan.sub_l_target_write {
                    frame
                        .kl_components
                        .as_deref_mut()
                        .ok_or(Block::Runtime("Sub-L target allocation"))?
                        .commit_target(target_raw);
                }
                commit_snapshot(entity, snapshot);
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::TargetPrelude,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(plan)) => {
                let (Some(smoothed_raw), Some(exact_raw), None) =
                    (plan.sub_k_smoothed_raw, plan.sub_l_exact_raw, plan.sub_f)
                else {
                    return Err(Block::Runtime("unexpected post-Sub-D writes"));
                };
                frame
                    .kl_components
                    .as_deref_mut()
                    .ok_or(Block::Runtime("Sub-K/L post-Sub-D allocation"))?
                    .commit_sub_d_writes(smoothed_raw, exact_raw);
                snapshot.sub_k_smoothed_raw = Some(smoothed_raw);
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::PostSubDWrites,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD {
                target_position_raw,
                ..
            }) => {
                if !frame.sub_d_owner.classifier_cache().can_classify() {
                    return Err(Block::SubDFirstQuery {
                        seed: frame.sub_d_owner.classifier_cache().stagger_counter(),
                    });
                }
                let RetailRuntimeValue::Known(Some(a)) = snapshot.sub_a_runtime else {
                    return Err(Block::Runtime("Sub-A allocation"));
                };
                let evidence = frame.sub_d_owner.evidence_for_frame_with_descriptor(
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
                    frame.sub_d_runtime,
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
                let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor
                else {
                    return Err(Block::Runtime("required Sub-H descriptor"));
                };
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
                use_wave_surface,
            }) => CommonMoverFrameResume::SubCSurfaceSampled(sample_sub_c_world_surface(
                frame.terrain,
                point_raw,
                if use_wave_surface {
                    Some(SubCWaveClock {
                        wave_tick_50hz: frame
                            .wave_tick_50hz
                            .ok_or(Block::Runtime("Sub-C wave clock"))?,
                        waves_enabled: frame.terrain.water_enabled(),
                    })
                } else {
                    None
                },
            )),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubK,
                ..
            }) => {
                frame
                    .kl_components
                    .as_deref_mut()
                    .ok_or(Block::Runtime("Sub-K allocation"))?
                    .advance_sub_k(snapshot.velocity_raw[1]);
                CommonMoverFrameResume::ComponentReturned {
                    phase: CommonMoverDispatchPhase::SubK,
                    snapshot,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubL,
                ..
            }) => {
                // 1BA00/1E700 read the incoming physical matrix. Sub-D changed
                // Euler words only; C has not moved the body yet.
                frame
                    .kl_components
                    .as_deref_mut()
                    .ok_or(Block::Runtime("Sub-L allocation"))?
                    .advance_sub_l(snapshot.position_raw, basis);
                CommonMoverFrameResume::ComponentReturned {
                    phase: CommonMoverDispatchPhase::SubL,
                    snapshot,
                }
            }
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod kl_tests;
