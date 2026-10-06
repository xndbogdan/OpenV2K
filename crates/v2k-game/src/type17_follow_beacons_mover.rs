//! Type-17 Follow Beacons binding of shared `FUN_00401430`.
//!
//! Variant one's `FUN_00403CE0` calls the common mover first. First-world
//! type 17 is A/B/C/D/H/J, so normal dispatch selects Sub-H rather than
//! Type-9's D/I/A/B route or Type-47's A/B/C/D/E/H/J path. This owner starts
//! the detached frame machine, commits the recovered target prelude, and
//! applies the `V200002.run` first `FUN_0041FCB0` full-reset for the
//! accepted type-17 seeds, then the authored Sub-H writer and C -> A -> B
//! tail. After Sub-D yaw, `FUN_00413F70` rebuilds the live Q31 basis so
//! Sub-A thrusts along the new heading (the same 01430 policy as Type-47;
//! keeping the constructor matrix walks spawn-heading). The production
//! owner commits the applied pose, E100, `FUN_00412DA0`, and live D360.
//! Authored Sub-D flags `0x13` match Type-47's bytes; they do not
//! authorize Type-9's first-query reset. Level-1 Type-47 `0x2B/0x2C/0x2D`
//! never enter.

use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameBlock, CommonMoverFrameCommitPhase,
    CommonMoverFrameConfiguration, CommonMoverFrameMachine, CommonMoverFramePoll,
    CommonMoverFrameRequest, CommonMoverFrameResume, CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_c::SubCSurfaceSample;
use crate::common_mover::sub_d::{
    apply_type9_sub_d, type17_first_query_owner_for_seed, Type9SubDFrameOwner, Type9SubDRuntime,
    Type9SubDStep,
};
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::follow_beacons::FollowBeaconsFollowingCommonMoverReturn;
use crate::hover::HoverLiftConfig;
use crate::sub_h_external_frame::SubHRuntimeState;
use crate::type17_impact_live::{TYPE17_MODEL256_COMPONENT_TOPOLOGY, TYPE17_MODEL256_SUB_D};
use crate::type17_impact_reselection::TYPE17_IMPACT_ENTITY_TYPE;
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

/// Ordinary Level-1 `FUN_00401430` scheduler-mode word for type 17.
///
/// Intro2 mode 1 is recorded only for types 13/26. Type 47 uses mode 0.
/// Type 17 is not on that Restricted transcript.
pub const TYPE17_FOLLOW_BEACONS_SCHEDULER_MODE: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type17FollowBeaconsMoverTopologyError {
    WrongEntityType { actual: u32 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowBeaconsMoverTopology {
    components: CommonMoverComponentTopology,
}

impl Type17FollowBeaconsMoverTopology {
    pub fn from_metadata(
        entity_type: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type17FollowBeaconsMoverTopologyError> {
        if entity_type != TYPE17_IMPACT_ENTITY_TYPE {
            return Err(Type17FollowBeaconsMoverTopologyError::WrongEntityType {
                actual: entity_type,
            });
        }
        let components = match metadata.common_mover_topology {
            RetailRuntimeValue::Unresolved => {
                return Err(Type17FollowBeaconsMoverTopologyError::UnresolvedComponentTopology);
            }
            RetailRuntimeValue::Known(components) => components,
        };
        if components != TYPE17_MODEL256_COMPONENT_TOPOLOGY {
            return Err(Type17FollowBeaconsMoverTopologyError::UnsupportedComponentTopology);
        }
        Ok(Self { components })
    }

    pub const fn components(self) -> CommonMoverComponentTopology {
        self.components
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type17FollowBeaconsMoverBlock {
    Topology(Type17FollowBeaconsMoverTopologyError),
    TypeMetadataUnavailable,
    SubARuntimeUnavailable,
    SubDUnresolved,
    UnexpectedSubD,
    Frame(CommonMoverFrameBlock),
    SubDFirstQueryUnavailable,
    SubDSteeringUnavailable,
    BodyBasisUnavailable,
    SubHUnavailable,
    SubH(crate::sub_h_external_frame::SubHUpdateError),
    SubHCompletionCue,
    SubCWaveUnexpected,
    SubD(Type9SubDStep),
}

#[derive(Debug)]
pub struct Type17FollowBeaconsMoverRequest<'a> {
    pub entity_id: u32,
    pub entity_type: u32,
    pub metadata: Option<&'a EntityTypeRuntimeMetadata>,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub roll_raw: u16,
    pub sub_a_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    pub sub_d_stagger_seed: Option<u8>,
    pub sub_d_frame_owner: Option<&'a mut Type9SubDFrameOwner>,
    pub sub_d_runtime: Option<&'a mut Type9SubDRuntime>,
    pub body_right_q31: RetailRuntimeValue<[i32; 3]>,
    pub body_forward_q31: RetailRuntimeValue<[i32; 3]>,
    pub body_up_q31: RetailRuntimeValue<[i32; 3]>,
    pub sub_h_runtime: Option<&'a mut SubHRuntimeState>,
    pub terrain: Option<&'a TerrainGrid>,
    pub target_private: &'a mut WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type17FollowBeaconsMoverOutcome {
    pub result: FollowBeaconsFollowingCommonMoverReturn,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub sub_a_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
}

pub fn evaluate_type17_follow_beacons_common_mover(
    mut request: Type17FollowBeaconsMoverRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Type17FollowBeaconsMoverOutcome, Type17FollowBeaconsMoverBlock> {
    let metadata = request
        .metadata
        .ok_or(Type17FollowBeaconsMoverBlock::TypeMetadataUnavailable)?;
    let topology = Type17FollowBeaconsMoverTopology::from_metadata(request.entity_type, metadata)
        .map_err(Type17FollowBeaconsMoverBlock::Topology)?;
    match request.sub_a_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        _ => return Err(Type17FollowBeaconsMoverBlock::SubARuntimeUnavailable),
    }
    let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Unresolved => {
            return Err(Type17FollowBeaconsMoverBlock::SubDUnresolved)
        }
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == TYPE17_MODEL256_SUB_D => {
            CommonMoverPreludeSubD {
                steering_divisor_raw: descriptor.steering_divisor_raw,
                couple_yaw_into_roll: descriptor.couple_yaw_into_roll_raw != 0,
            }
        }
        RetailRuntimeValue::Known(_) => return Err(Type17FollowBeaconsMoverBlock::UnexpectedSubD),
    };
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => Some(descriptor),
        _ => None,
    };
    let sub_b_descriptor = match metadata.sub_b_lateral_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => Some(descriptor),
        _ => None,
    };
    let sub_c_descriptor = match metadata.sub_c_lift_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) => Some(HoverLiftConfig::from(descriptor)),
        _ => None,
    };

    let snapshot = CommonMoverFrameSnapshot {
        controlled_entity_lookup: None,
        position_raw: request.position_raw,
        velocity_raw: request.velocity_raw,
        heading_raw: request.heading_raw,
        roll_raw: request.roll_raw,
        body_up_q31: [0; 3],
        body_right_q31: [0; 3],
        body_forward_q31: [0; 3],
        attached_cargo_mass: 0,
        sub_a_runtime: request.sub_a_runtime,
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
                controlled_entity_handle: request.entity_id,
                topology: topology.components(),
                dispatch_mode: CommonMoverDispatchMode::from_retail_word(
                    TYPE17_FOLLOW_BEACONS_SCHEDULER_MODE,
                ),
                elapsed_micros: request.elapsed_micros,
                sub_a_descriptor,
                sub_b_descriptor,
                sub_c_descriptor,
                sub_d_descriptor: Some(sub_d_descriptor),
                target_resource_context_present: true,
            },
            initial_snapshot: snapshot,
            target_private: Some(*request.target_private),
            tracked_target: request.tracked_target,
        },
        next_random,
    )
    .map_err(Type17FollowBeaconsMoverBlock::Frame)?;

    let mut live = snapshot;
    let mut sub_d_frame_owner = request.sub_d_frame_owner;
    let mut sub_d_runtime = request.sub_d_runtime;
    loop {
        match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) => {
                return Ok(Type17FollowBeaconsMoverOutcome {
                    result: FollowBeaconsFollowingCommonMoverReturn::Zero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    sub_a_runtime: live.sub_a_runtime,
                });
            }
            CommonMoverFramePoll::ReturnOne => {
                return Ok(Type17FollowBeaconsMoverOutcome {
                    result: FollowBeaconsFollowingCommonMoverReturn::NonZero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    sub_a_runtime: live.sub_a_runtime,
                });
            }
            CommonMoverFramePoll::Blocked(block) => {
                return Err(Type17FollowBeaconsMoverBlock::Frame(block));
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(plan)) => {
                live.heading_raw = plan.heading_raw;
                live.roll_raw = plan.roll_raw;
                if let Some(sub_a) = plan.sub_a_runtime {
                    live.sub_a_runtime = RetailRuntimeValue::Known(Some(sub_a));
                }
                let next = live;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::TargetPrelude,
                        snapshot: next,
                    })
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. }) => {
                let Some(owner) = sub_d_frame_owner.as_deref_mut() else {
                    return Err(Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable);
                };
                if type17_first_query_owner_for_seed(request.sub_d_stagger_seed.unwrap_or(0))
                    .is_none()
                    || !owner.classifier_cache().can_classify()
                {
                    return Err(Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable);
                }
                let Some(runtime) = sub_d_runtime.as_deref_mut() else {
                    return Err(Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable);
                };
                let (
                    RetailRuntimeValue::Known(right_q31),
                    RetailRuntimeValue::Known(forward_q31),
                    RetailRuntimeValue::Known(_up_q31),
                ) = (
                    request.body_right_q31,
                    request.body_forward_q31,
                    request.body_up_q31,
                )
                else {
                    return Err(Type17FollowBeaconsMoverBlock::BodyBasisUnavailable);
                };
                let Some(terrain) = request.terrain else {
                    return Err(Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable);
                };
                let direction_multiplier = match live.sub_a_runtime {
                    RetailRuntimeValue::Known(Some(sub_a)) => sub_a.direction_multiplier(),
                    _ => 1,
                };
                let target_raw = match request.tracked_target {
                    RetailRuntimeValue::Known(Some(target)) => target.position_raw,
                    _ => request.position_raw,
                };
                let evidence = owner.evidence_for_frame_with_descriptor(
                    TYPE17_MODEL256_SUB_D,
                    terrain,
                    request.position_raw,
                    target_raw,
                    right_q31,
                    forward_q31,
                    direction_multiplier,
                );
                let yaw_step_raw = match apply_type9_sub_d(
                    TYPE17_MODEL256_SUB_D,
                    runtime,
                    evidence,
                    request.elapsed_micros,
                    request.global_elapsed_micros,
                ) {
                    Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                    blocked => return Err(Type17FollowBeaconsMoverBlock::SubD(blocked)),
                };
                live.heading_raw = live.heading_raw.wrapping_sub(yaw_step_raw as u16);
                // FUN_00413F70 after Sub-D yaw so Sub-A thrusts along the new
                // heading. Copying the constructor matrix walks spawn-heading.
                let rebuilt = Type9BodyBasis::from_angle_words(
                    live.heading_raw as i16,
                    0,
                    request.roll_raw as i16,
                );
                live.body_right_q31 = rebuilt.lateral;
                live.body_forward_q31 = rebuilt.forward;
                live.body_up_q31 = rebuilt.up;
                let next = live;
                machine
                    .resume(CommonMoverFrameResume::SubDReturned {
                        result_raw: yaw_step_raw,
                        snapshot: next,
                    })
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubH,
                ..
            }) => {
                let Some(runtime) = request.sub_h_runtime.as_deref_mut() else {
                    return Err(Type17FollowBeaconsMoverBlock::SubHUnavailable);
                };
                let RetailRuntimeValue::Known(Some(descriptor)) =
                    &metadata.sub_h_external_frame_descriptor
                else {
                    return Err(Type17FollowBeaconsMoverBlock::SubHUnavailable);
                };
                let cues = runtime
                    .update(descriptor, request.elapsed_micros as i32)
                    .map_err(Type17FollowBeaconsMoverBlock::SubH)?;
                if !cues.is_empty() {
                    return Err(Type17FollowBeaconsMoverBlock::SubHCompletionCue);
                }
                machine
                    .resume(CommonMoverFrameResume::ComponentReturned {
                        phase: CommonMoverDispatchPhase::SubH,
                        snapshot: live,
                    })
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubHUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface,
            }) => {
                if use_wave_surface {
                    return Err(Type17FollowBeaconsMoverBlock::SubCWaveUnexpected);
                }
                let Some(terrain) = request.terrain else {
                    return Err(Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable);
                };
                machine
                    .resume(CommonMoverFrameResume::SubCSurfaceSampled(
                        SubCSurfaceSample::Terrain {
                            terrain_y_raw: terrain.bilinear_height_raw(point_raw[0], point_raw[1]),
                        },
                    ))
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubC {
                position_raw,
                velocity_raw,
                ..
            }) => {
                live.position_raw = position_raw;
                live.velocity_raw = velocity_raw;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::SubC,
                        snapshot: live,
                    })
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubA {
                velocity_raw,
                ..
            }) => {
                live.velocity_raw = velocity_raw;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::SubA,
                        snapshot: live,
                    })
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB {
                velocity_raw,
                ..
            }) => {
                live.velocity_raw = velocity_raw;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::SubB,
                        snapshot: live,
                    })
                    .map_err(|error| {
                        match error {
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Block(
                            block,
                        ) => Type17FollowBeaconsMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type17FollowBeaconsMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(_) => {
                return Err(Type17FollowBeaconsMoverBlock::SubDFirstQueryUnavailable);
            }
        }
    }
}
