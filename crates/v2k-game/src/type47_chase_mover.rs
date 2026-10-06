//! Type-47 Chase binding of shared `FUN_00401430`.
//!
//! Guard Location's pursuing style calls the common mover with scheduler
//! mode 0. The authored first-world Type-47 record is A/B/C/D/E/H/J, so
//! normal dispatch selects Sub-H rather than Type-9's D/I/A/B route. This
//! owner starts the detached frame machine, commits the recovered target
//! prelude, and applies the `V200003.run` first `FUN_0041FCB0` full-reset
//! for seeds `0x2B/0x2C/0x2D`. Shared `FUN_0041F660` yaw uses
//! `classifier_flags` `0x13`. Normal dispatch then runs the detached
//! six-record Sub-H writer and the authored terrain-only C -> A -> B tail.
//! Intro2 Type-47 seeds `0x06/0x07/0x08` belong to the separate Intro2
//! Guard-anchor bind. Replay-level seeds `0x3C/0x3D/0x3E` stay fail-closed.

use crate::chase_target::ChaseTargetCommonMoverReturn;
use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameAdvanceError, CommonMoverFrameBlock,
    CommonMoverFrameCommitPhase, CommonMoverFrameConfiguration, CommonMoverFrameMachine,
    CommonMoverFramePoll, CommonMoverFrameRequest, CommonMoverFrameResume,
    CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_c::SubCSurfaceSample;
use crate::common_mover::sub_d::{
    apply_type9_sub_d, level1_type47_first_query_owner_for_seed, Type9SubDFrameOwner,
    Type9SubDRuntime, Type9SubDStep, TYPE47_SUB_D,
};
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::common_mover::SubAPropulsionRuntime;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::hover::HoverLiftConfig;
use crate::ordinary_type47_death_live::{
    TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
    TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_H_RECORDS,
};
use crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY;
use crate::sub_h_external_frame::{SubHRuntimeState, SubHUpdateError};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

const ORDINARY_TYPE47_ENTITY_TYPE: u32 = 47;
const TYPE47_CHASE_SCHEDULER_MODE: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type47ChaseMoverTopologyError {
    WrongEntityType { actual: u32 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47ChaseMoverTopology {
    components: CommonMoverComponentTopology,
}

impl Type47ChaseMoverTopology {
    pub fn from_metadata(
        entity_type: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type47ChaseMoverTopologyError> {
        if entity_type != ORDINARY_TYPE47_ENTITY_TYPE {
            return Err(Type47ChaseMoverTopologyError::WrongEntityType {
                actual: entity_type,
            });
        }
        let components = match metadata.common_mover_topology {
            RetailRuntimeValue::Unresolved => {
                return Err(Type47ChaseMoverTopologyError::UnresolvedComponentTopology);
            }
            RetailRuntimeValue::Known(components) => components,
        };
        if components != FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY {
            return Err(Type47ChaseMoverTopologyError::UnsupportedComponentTopology);
        }
        Ok(Self { components })
    }

    pub const fn components(self) -> CommonMoverComponentTopology {
        self.components
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type47ChaseMoverBlock {
    Topology(Type47ChaseMoverTopologyError),
    TypeMetadataUnavailable,
    SubARuntimeUnavailable,
    SubDUnresolved,
    UnexpectedSubD,
    UnexpectedSubA,
    UnexpectedSubB,
    UnexpectedSubC,
    UnexpectedSubH,
    Frame(CommonMoverFrameBlock),
    SubDFirstQueryUnavailable,
    SubDSteeringUnavailable,
    BodyBasisUnavailable,
    SubD(Type9SubDStep),
    SubHUnavailable,
    SubH(SubHUpdateError),
    SubHCompletionCue,
    SubCWaveUnexpected,
}

#[derive(Debug)]
pub struct Type47ChaseMoverRequest<'a> {
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
    pub global_elapsed_micros: u32,
    pub target_private: &'a mut WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub elapsed_micros: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type47ChaseMoverOutcome {
    pub result: ChaseTargetCommonMoverReturn,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub sub_a_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
}

fn map_advance(
    error: CommonMoverFrameAdvanceError,
    protocol: Type47ChaseMoverBlock,
) -> Type47ChaseMoverBlock {
    match error {
        CommonMoverFrameAdvanceError::Block(block) => Type47ChaseMoverBlock::Frame(block),
        CommonMoverFrameAdvanceError::Protocol(_) => protocol,
    }
}

pub fn evaluate_type47_chase_common_mover(
    mut request: Type47ChaseMoverRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Type47ChaseMoverOutcome, Type47ChaseMoverBlock> {
    let metadata = request
        .metadata
        .ok_or(Type47ChaseMoverBlock::TypeMetadataUnavailable)?;
    let topology = Type47ChaseMoverTopology::from_metadata(request.entity_type, metadata)
        .map_err(Type47ChaseMoverBlock::Topology)?;
    match request.sub_a_runtime {
        RetailRuntimeValue::Known(Some(_)) => {}
        _ => return Err(Type47ChaseMoverBlock::SubARuntimeUnavailable),
    }
    let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Unresolved => {
            return Err(Type47ChaseMoverBlock::SubDUnresolved);
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Type47ChaseMoverBlock::UnexpectedSubD);
        }
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == TYPE47_SUB_D => {
            CommonMoverPreludeSubD {
                steering_divisor_raw: descriptor.steering_divisor_raw,
                couple_yaw_into_roll: descriptor.couple_yaw_into_roll_raw != 0,
            }
        }
        RetailRuntimeValue::Known(Some(_)) => {
            return Err(Type47ChaseMoverBlock::UnexpectedSubD);
        }
    };
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor == TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR =>
        {
            descriptor
        }
        _ => return Err(Type47ChaseMoverBlock::UnexpectedSubA),
    };
    let sub_b_descriptor = match metadata.sub_b_lateral_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor == TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR =>
        {
            descriptor
        }
        _ => return Err(Type47ChaseMoverBlock::UnexpectedSubB),
    };
    let sub_c_descriptor = match metadata.sub_c_lift_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor == TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR =>
        {
            HoverLiftConfig::from(descriptor)
        }
        _ => return Err(Type47ChaseMoverBlock::UnexpectedSubC),
    };
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.records.as_slice() == TYPE47_COMMON_DYING_SUB_H_RECORDS =>
        {
            descriptor
        }
        _ => return Err(Type47ChaseMoverBlock::UnexpectedSubH),
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
                    TYPE47_CHASE_SCHEDULER_MODE,
                ),
                elapsed_micros: request.elapsed_micros,
                sub_a_descriptor: Some(sub_a_descriptor),
                sub_b_descriptor: Some(sub_b_descriptor),
                sub_c_descriptor: Some(sub_c_descriptor),
                sub_d_descriptor: Some(sub_d_descriptor),
                target_resource_context_present: true,
            },
            initial_snapshot: snapshot,
            target_private: Some(*request.target_private),
            tracked_target: request.tracked_target,
        },
        next_random,
    )
    .map_err(Type47ChaseMoverBlock::Frame)?;

    let mut live = snapshot;
    let mut sub_d_frame_owner = request.sub_d_frame_owner;
    let mut sub_d_runtime = request.sub_d_runtime;
    loop {
        match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) => {
                return Ok(Type47ChaseMoverOutcome {
                    result: ChaseTargetCommonMoverReturn::Zero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    sub_a_runtime: live.sub_a_runtime,
                });
            }
            CommonMoverFramePoll::ReturnOne => {
                return Ok(Type47ChaseMoverOutcome {
                    result: ChaseTargetCommonMoverReturn::NonZero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    sub_a_runtime: live.sub_a_runtime,
                });
            }
            CommonMoverFramePoll::Blocked(block) => {
                return Err(Type47ChaseMoverBlock::Frame(block));
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
                        map_advance(error, Type47ChaseMoverBlock::SubDFirstQueryUnavailable)
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. }) => {
                let Some(owner) = sub_d_frame_owner.as_deref_mut() else {
                    return Err(Type47ChaseMoverBlock::SubDFirstQueryUnavailable);
                };
                if level1_type47_first_query_owner_for_seed(request.sub_d_stagger_seed.unwrap_or(0))
                    .is_none()
                    || !owner.classifier_cache().can_classify()
                {
                    return Err(Type47ChaseMoverBlock::SubDFirstQueryUnavailable);
                }
                let Some(runtime) = sub_d_runtime.as_deref_mut() else {
                    return Err(Type47ChaseMoverBlock::SubDSteeringUnavailable);
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
                    return Err(Type47ChaseMoverBlock::BodyBasisUnavailable);
                };
                let Some(terrain) = request.terrain else {
                    return Err(Type47ChaseMoverBlock::SubDSteeringUnavailable);
                };
                let direction_multiplier = match live.sub_a_runtime {
                    RetailRuntimeValue::Known(Some(sub_a)) => sub_a.direction_multiplier(),
                    _ => 1,
                };
                // Guard wander has no tracked entity. Aim at the slot-0
                // waypoint around +0x90, not at the body: using self made
                // Sub-D skip yaw and Sub-A walk spawn-heading forever.
                let target_raw = match request.tracked_target {
                    RetailRuntimeValue::Known(Some(target)) => target.position_raw,
                    _ => request.target_private.target_position_raw,
                };
                let evidence = owner.evidence_for_frame_with_descriptor(
                    TYPE47_SUB_D,
                    terrain,
                    request.position_raw,
                    target_raw,
                    right_q31,
                    forward_q31,
                    direction_multiplier,
                );
                let yaw_step_raw = match apply_type9_sub_d(
                    TYPE47_SUB_D,
                    runtime,
                    evidence,
                    request.elapsed_micros,
                    request.global_elapsed_micros,
                ) {
                    Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                    blocked => return Err(Type47ChaseMoverBlock::SubD(blocked)),
                };
                live.heading_raw = live.heading_raw.wrapping_sub(yaw_step_raw as u16);
                // FUN_00413F70 after Sub-D yaw so Sub-A thrusts along the new
                // heading. Keeping the constructor matrix walks spawn-heading
                // forever (hive newants marched off to water).
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
                        map_advance(error, Type47ChaseMoverBlock::SubDSteeringUnavailable)
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubH,
                ..
            }) => {
                let Some(runtime) = request.sub_h_runtime.as_deref_mut() else {
                    return Err(Type47ChaseMoverBlock::SubHUnavailable);
                };
                let cues = runtime
                    .update(sub_h_descriptor, request.elapsed_micros as i32)
                    .map_err(Type47ChaseMoverBlock::SubH)?;
                if !cues.is_empty() {
                    return Err(Type47ChaseMoverBlock::SubHCompletionCue);
                }
                machine
                    .resume(CommonMoverFrameResume::ComponentReturned {
                        phase: CommonMoverDispatchPhase::SubH,
                        snapshot: live,
                    })
                    .map_err(|error| map_advance(error, Type47ChaseMoverBlock::SubHUnavailable))?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface,
            }) => {
                if use_wave_surface {
                    return Err(Type47ChaseMoverBlock::SubCWaveUnexpected);
                }
                let Some(terrain) = request.terrain else {
                    return Err(Type47ChaseMoverBlock::SubDSteeringUnavailable);
                };
                machine
                    .resume(CommonMoverFrameResume::SubCSurfaceSampled(
                        SubCSurfaceSample::Terrain {
                            terrain_y_raw: terrain.bilinear_height_raw(point_raw[0], point_raw[1]),
                        },
                    ))
                    .map_err(|error| {
                        map_advance(error, Type47ChaseMoverBlock::SubDSteeringUnavailable)
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
                        map_advance(error, Type47ChaseMoverBlock::SubDSteeringUnavailable)
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
                        map_advance(error, Type47ChaseMoverBlock::SubDSteeringUnavailable)
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
                        map_advance(error, Type47ChaseMoverBlock::SubDSteeringUnavailable)
                    })?;
            }
            CommonMoverFramePoll::Action(_) => {
                return Err(Type47ChaseMoverBlock::SubDSteeringUnavailable);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::sub_d::{
        intro2_type47_first_query_owner_for_seed, level1_type47_first_query_owner_for_seed,
        Type9SubDRuntime,
    };
    use crate::entity_collision_state::EntityTypeRuntimeMetadata;
    use crate::ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    };
    use crate::sub_h_external_frame::SubHRuntimeState;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    fn type47_metadata() -> EntityTypeRuntimeMetadata {
        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology =
            RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY);
        metadata.sub_a_propulsion_descriptor =
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR));
        metadata.sub_b_lateral_descriptor =
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR));
        metadata.sub_c_lift_descriptor =
            RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR));
        metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(TYPE47_SUB_D));
        metadata.sub_h_external_frame_descriptor =
            RetailRuntimeValue::Known(Some(v2k_formats::collision::SubHExternalFrameDescriptor {
                completion_sound_id: None,
                records: TYPE47_COMMON_DYING_SUB_H_RECORDS.to_vec(),
            }));
        metadata
    }

    fn empty_terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    #[test]
    fn admits_only_the_authored_type47_component_route() {
        let metadata = type47_metadata();
        assert!(Type47ChaseMoverTopology::from_metadata(47, &metadata).is_ok());
        assert_eq!(
            Type47ChaseMoverTopology::from_metadata(9, &metadata),
            Err(Type47ChaseMoverTopologyError::WrongEntityType { actual: 9 })
        );

        let mut type9_shape = FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY;
        type9_shape.sub_c = false;
        type9_shape.sub_h = false;
        type9_shape.sub_i = true;
        let mut metadata = metadata;
        metadata.common_mover_topology = RetailRuntimeValue::Known(type9_shape);
        assert_eq!(
            Type47ChaseMoverTopology::from_metadata(47, &metadata),
            Err(Type47ChaseMoverTopologyError::UnsupportedComponentTopology)
        );
    }

    #[test]
    fn live_chase_starts_fun_00401430_and_blocks_without_the_v200003_owner() {
        let metadata = type47_metadata();
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type47_chase_common_mover(
            Type47ChaseMoverRequest {
                entity_id: 0x042F_000B,
                entity_type: 47,
                metadata: Some(&metadata),
                position_raw: [0, 0, 0],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(Some(
                    SubAPropulsionRuntime::from_retail_words(
                        RetailRuntimeValue::Known(400),
                        1,
                        100,
                    ),
                )),
                sub_d_stagger_seed: Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0]),
                sub_d_frame_owner: None,
                sub_d_runtime: None,
                body_right_q31: RetailRuntimeValue::Unresolved,
                body_forward_q31: RetailRuntimeValue::Unresolved,
                body_up_q31: RetailRuntimeValue::Unresolved,
                sub_h_runtime: None,
                terrain: None,
                global_elapsed_micros: 19_500,
                target_private: &mut target_private,
                tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                    position_raw: [1_000, 0, 0],
                    velocity_raw: [0; 3],
                })),
                elapsed_micros: 19_500,
            },
            || 0,
        );
        assert_eq!(
            result.unwrap_err(),
            Type47ChaseMoverBlock::SubDFirstQueryUnavailable
        );
        assert!(
            intro2_type47_first_query_owner_for_seed(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0])
                .is_none(),
            "Level-1 Type-47 seeds must not mint the Intro2 first-query owner"
        );
        assert!(level1_type47_first_query_owner_for_seed(0x3C).is_none());
        assert!(level1_type47_first_query_owner_for_seed(0x06).is_none());
    }

    #[test]
    fn classifier_owner_without_steering_runtime_blocks_before_first_query() {
        let metadata = type47_metadata();
        let terrain = empty_terrain();
        let mut owner =
            level1_type47_first_query_owner_for_seed(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[2])
                .expect("Level-1 seed");
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type47_chase_common_mover(
            Type47ChaseMoverRequest {
                entity_id: 0x042F_000B,
                entity_type: 47,
                metadata: Some(&metadata),
                position_raw: [0xBE00u16 as i16, 0, 0x7D00u16 as i16],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(Some(
                    SubAPropulsionRuntime::from_retail_words(
                        RetailRuntimeValue::Known(400),
                        1,
                        100,
                    ),
                )),
                sub_d_stagger_seed: Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[2]),
                sub_d_frame_owner: Some(&mut owner),
                sub_d_runtime: None,
                body_right_q31: RetailRuntimeValue::Unresolved,
                body_forward_q31: RetailRuntimeValue::Unresolved,
                body_up_q31: RetailRuntimeValue::Unresolved,
                sub_h_runtime: None,
                terrain: Some(&terrain),
                global_elapsed_micros: 19_500,
                target_private: &mut target_private,
                tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                    position_raw: [1_000, 0, 0],
                    velocity_raw: [0; 3],
                })),
                elapsed_micros: 19_500,
            },
            || 0,
        );
        assert_eq!(
            result.unwrap_err(),
            Type47ChaseMoverBlock::SubDSteeringUnavailable
        );
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn v200003_first_query_applies_0x13_then_sub_h_and_cab_tail() {
        let metadata = type47_metadata();
        let terrain = empty_terrain();
        let mut owner =
            level1_type47_first_query_owner_for_seed(0x2D).expect("V200003 spawn-13 seed");
        let mut runtime = Type9SubDRuntime::from_constructor();
        let mut sub_h = SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT)
            .expect("six-record Type-47 Sub-H");
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type47_chase_common_mover(
            Type47ChaseMoverRequest {
                entity_id: 0x04AB_0001,
                entity_type: 47,
                metadata: Some(&metadata),
                position_raw: [0xBE00u16 as i16, 0, 0x7D00u16 as i16],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(Some(
                    SubAPropulsionRuntime::from_retail_words(
                        RetailRuntimeValue::Known(400),
                        1,
                        100,
                    ),
                )),
                sub_d_stagger_seed: Some(0x2D),
                sub_d_frame_owner: Some(&mut owner),
                sub_d_runtime: Some(&mut runtime),
                body_right_q31: RetailRuntimeValue::Known([i32::MAX, 0, 0]),
                body_forward_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
                body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
                sub_h_runtime: Some(&mut sub_h),
                terrain: Some(&terrain),
                global_elapsed_micros: 19_500,
                target_private: &mut target_private,
                tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                    position_raw: [1_000, 0, 0],
                    velocity_raw: [0; 3],
                })),
                elapsed_micros: 19_500,
            },
            || 0,
        )
        .expect("V200003 first query then Sub-H/C/A/B");
        assert_eq!(result.result, ChaseTargetCommonMoverReturn::NonZero);
        assert!(matches!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Known(_)
        ));
        assert!(
            owner.classifier_cache().rows().iter().any(|row| *row != 0),
            "0x13 classification must fill at least one cache nibble"
        );
        assert_ne!(runtime.yaw_rate_raw, 0);
    }
}
