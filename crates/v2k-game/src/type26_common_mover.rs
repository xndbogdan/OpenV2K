//! Captured restricted Type-26 fixture binding of shared `FUN_00401430`.
//!
//! The accepted sampled Intro2 transcript records scheduler mode 1.
//! First-world type 26 is A/B/C/D/H, so Restricted dispatch selects neither
//! F nor G. This owner starts the detached frame machine, commits the
//! recovered target prelude, and applies the captured first `FUN_0041FCB0`
//! full-reset plus shared `FUN_0041F660` steering for `classifier_flags`
//! `0x12` (steepness then water, divisor 128). The ordinary C -> A -> B tail
//! uses the authored terrain-only Sub-C and Sub-A/B descriptors. Restricted
//! `FUN_004018A0` does not invoke Sub-H. Presentation owns D360 writeback
//! for actual visible model submissions. Native Type26 production instead uses
//! `intro2_type26_defecate_virus::world`, with both callback modes selected from
//! current presentation state and the complete common scheduler/world phases.

use crate::chase_target::ChaseTargetCommonMoverReturn;
use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameBlock, CommonMoverFrameCommitPhase,
    CommonMoverFrameConfiguration, CommonMoverFrameMachine, CommonMoverFramePoll,
    CommonMoverFrameRequest, CommonMoverFrameResume, CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_c::SubCSurfaceSample;
use crate::common_mover::sub_d::{
    apply_type9_sub_d, intro2_type26_first_query_owner_for_seed, Type9SubDFrameOwner,
    Type9SubDRuntime, Type9SubDStep, INTRO2_TYPE26_SUB_D,
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
use crate::intro2_type26_defecate_virus::{
    INTRO2_DEFECATE_VIRUS_ENTITY_TYPE, INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY, INTRO2_TYPE26_SUB_A,
    INTRO2_TYPE26_SUB_B, INTRO2_TYPE26_SUB_C,
};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

/// The explicit captured fixture's `FUN_00401430` scheduler-mode word.
const CAPTURED_TYPE26_RESTRICTED_SCHEDULER_MODE: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type26CommonMoverTopologyError {
    WrongEntityType { actual: u32 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type26CommonMoverTopology {
    components: CommonMoverComponentTopology,
}

impl Type26CommonMoverTopology {
    pub fn from_metadata(
        entity_type: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Type26CommonMoverTopologyError> {
        if entity_type != INTRO2_DEFECATE_VIRUS_ENTITY_TYPE {
            return Err(Type26CommonMoverTopologyError::WrongEntityType {
                actual: entity_type,
            });
        }
        let components = match metadata.common_mover_topology {
            RetailRuntimeValue::Unresolved => {
                return Err(Type26CommonMoverTopologyError::UnresolvedComponentTopology);
            }
            RetailRuntimeValue::Known(components) => components,
        };
        if components != INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY {
            return Err(Type26CommonMoverTopologyError::UnsupportedComponentTopology);
        }
        Ok(Self { components })
    }

    pub const fn components(self) -> CommonMoverComponentTopology {
        self.components
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type26CommonMoverBlock {
    Topology(Type26CommonMoverTopologyError),
    TypeMetadataUnavailable,
    SubDUnresolved,
    UnexpectedSubD,
    UnexpectedSubA,
    UnexpectedSubB,
    UnexpectedSubC,
    Frame(CommonMoverFrameBlock),
    SubDFirstQueryUnavailable,
    SubDSteeringUnavailable,
    BodyBasisUnavailable,
    SubCWaveUnexpected,
    SubD(Type9SubDStep),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type26CommonMoverOutcome {
    pub result: ChaseTargetCommonMoverReturn,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
}

#[derive(Debug)]
pub struct Type26CommonMoverRequest<'a> {
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
    pub terrain: Option<&'a TerrainGrid>,
    pub global_elapsed_micros: u32,
    pub target_private: &'a mut WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub elapsed_micros: u32,
}

pub fn evaluate_type26_common_mover(
    request: Type26CommonMoverRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Type26CommonMoverOutcome, Type26CommonMoverBlock> {
    let metadata = request
        .metadata
        .ok_or(Type26CommonMoverBlock::TypeMetadataUnavailable)?;
    let topology = Type26CommonMoverTopology::from_metadata(request.entity_type, metadata)
        .map_err(Type26CommonMoverBlock::Topology)?;
    let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Unresolved => return Err(Type26CommonMoverBlock::SubDUnresolved),
        RetailRuntimeValue::Known(None) => return Err(Type26CommonMoverBlock::UnexpectedSubD),
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == INTRO2_TYPE26_SUB_D => {
            CommonMoverPreludeSubD {
                steering_divisor_raw: descriptor.steering_divisor_raw,
                couple_yaw_into_roll: descriptor.couple_yaw_into_roll_raw != 0,
            }
        }
        RetailRuntimeValue::Known(Some(_)) => return Err(Type26CommonMoverBlock::UnexpectedSubD),
    };
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == INTRO2_TYPE26_SUB_A => {
            descriptor
        }
        _ => return Err(Type26CommonMoverBlock::UnexpectedSubA),
    };
    let sub_b_descriptor = match metadata.sub_b_lateral_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == INTRO2_TYPE26_SUB_B => {
            descriptor
        }
        _ => return Err(Type26CommonMoverBlock::UnexpectedSubB),
    };
    let sub_c_descriptor = match metadata.sub_c_lift_descriptor {
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == INTRO2_TYPE26_SUB_C => {
            HoverLiftConfig::from(descriptor)
        }
        _ => return Err(Type26CommonMoverBlock::UnexpectedSubC),
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
                    CAPTURED_TYPE26_RESTRICTED_SCHEDULER_MODE,
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
    .map_err(Type26CommonMoverBlock::Frame)?;

    let mut live = snapshot;
    let mut sub_d_frame_owner = request.sub_d_frame_owner;
    let mut sub_d_runtime = request.sub_d_runtime;
    loop {
        match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) => {
                return Ok(Type26CommonMoverOutcome {
                    result: ChaseTargetCommonMoverReturn::Zero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                });
            }
            CommonMoverFramePoll::ReturnOne => {
                return Ok(Type26CommonMoverOutcome {
                    result: ChaseTargetCommonMoverReturn::NonZero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                });
            }
            CommonMoverFramePoll::Blocked(block) => {
                return Err(Type26CommonMoverBlock::Frame(block));
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
                        ) => Type26CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type26CommonMoverBlock::SubDFirstQueryUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. }) => {
                let Some(owner) = sub_d_frame_owner.as_deref_mut() else {
                    return Err(Type26CommonMoverBlock::SubDFirstQueryUnavailable);
                };
                if intro2_type26_first_query_owner_for_seed(request.sub_d_stagger_seed.unwrap_or(0))
                    .is_none()
                    || !owner.classifier_cache().can_classify()
                {
                    return Err(Type26CommonMoverBlock::SubDFirstQueryUnavailable);
                }
                let Some(runtime) = sub_d_runtime.as_deref_mut() else {
                    return Err(Type26CommonMoverBlock::SubDSteeringUnavailable);
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
                    return Err(Type26CommonMoverBlock::BodyBasisUnavailable);
                };
                let Some(terrain) = request.terrain else {
                    return Err(Type26CommonMoverBlock::SubDSteeringUnavailable);
                };
                let direction_multiplier = match live.sub_a_runtime {
                    RetailRuntimeValue::Known(Some(sub_a)) => sub_a.direction_multiplier(),
                    _ => 1,
                };
                let target_raw = match request.tracked_target {
                    RetailRuntimeValue::Known(Some(target)) => target.position_raw,
                    _ => request.target_private.target_position_raw,
                };
                let evidence = owner.evidence_for_frame_with_descriptor(
                    INTRO2_TYPE26_SUB_D,
                    terrain,
                    request.position_raw,
                    target_raw,
                    right_q31,
                    forward_q31,
                    direction_multiplier,
                );
                let yaw_step_raw = match apply_type9_sub_d(
                    INTRO2_TYPE26_SUB_D,
                    runtime,
                    evidence,
                    request.elapsed_micros,
                    request.global_elapsed_micros,
                ) {
                    Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                    blocked => return Err(Type26CommonMoverBlock::SubD(blocked)),
                };
                live.heading_raw = live.heading_raw.wrapping_sub(yaw_step_raw as u16);
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
                        ) => Type26CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type26CommonMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface,
            }) => {
                if use_wave_surface {
                    return Err(Type26CommonMoverBlock::SubCWaveUnexpected);
                }
                let Some(terrain) = request.terrain else {
                    return Err(Type26CommonMoverBlock::SubDSteeringUnavailable);
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
                        ) => Type26CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type26CommonMoverBlock::SubDSteeringUnavailable,
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
                        ) => Type26CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type26CommonMoverBlock::SubDSteeringUnavailable,
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
                        ) => Type26CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type26CommonMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { velocity_raw }) => {
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
                        ) => Type26CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Type26CommonMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(_) => {
                return Err(Type26CommonMoverBlock::SubDFirstQueryUnavailable);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::component_dispatch::CommonMoverDispatchPlan;
    use crate::common_mover::sub_d::ORDINARY_TYPE9_SUB_D;
    use crate::entity_collision_state::EntityTypeRuntimeMetadata;
    use crate::intro2_type26_defecate_virus::INTRO2_TYPE26_SPAWN25_SUB_D_SEED;
    use crate::ordinary_type47_live::{
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS,
    };
    use crate::search_attack_live::TYPE13_SEARCH_ATTACK_SUB_D;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    fn type26_metadata() -> EntityTypeRuntimeMetadata {
        let mut metadata = EntityTypeRuntimeMetadata::default();
        metadata.common_mover_topology =
            RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY);
        metadata.sub_a_propulsion_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_A));
        metadata.sub_b_lateral_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_B));
        metadata.sub_c_lift_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_C));
        metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D));
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
    fn admits_only_the_authored_type26_component_route() {
        let metadata = type26_metadata();
        assert!(Type26CommonMoverTopology::from_metadata(26, &metadata).is_ok());
        assert_eq!(
            Type26CommonMoverTopology::from_metadata(13, &metadata),
            Err(Type26CommonMoverTopologyError::WrongEntityType { actual: 13 })
        );

        let mut type47_shape = metadata;
        type47_shape.common_mover_topology =
            RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY);
        assert_eq!(
            Type26CommonMoverTopology::from_metadata(26, &type47_shape),
            Err(Type26CommonMoverTopologyError::UnsupportedComponentTopology)
        );
    }

    #[test]
    fn intro2_mode_1_does_not_select_type26_f_or_g() {
        assert_eq!(
            CommonMoverDispatchMode::from_retail_word(CAPTURED_TYPE26_RESTRICTED_SCHEDULER_MODE),
            CommonMoverDispatchMode::Restricted
        );
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(
                INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY,
                CommonMoverDispatchMode::Restricted,
            )
            .phases(),
            [None, None, None]
        );
    }

    #[test]
    fn type9_or_type13_descriptors_do_not_authorize_type26_first_query() {
        let mut metadata = type26_metadata();
        metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D));
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type26_common_mover(
            Type26CommonMoverRequest {
                entity_id: 0x047E_0001,
                entity_type: 26,
                metadata: Some(&metadata),
                position_raw: [0xC200u16 as i16, 0, 0x0E00],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(None),
                sub_d_stagger_seed: Some(INTRO2_TYPE26_SPAWN25_SUB_D_SEED),
                sub_d_frame_owner: None,
                sub_d_runtime: None,
                body_right_q31: RetailRuntimeValue::Unresolved,
                body_forward_q31: RetailRuntimeValue::Unresolved,
                body_up_q31: RetailRuntimeValue::Unresolved,
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
            || panic!("Type-9 Sub-D must not start type-26 FUN_00401430"),
        );
        assert_eq!(result, Err(Type26CommonMoverBlock::UnexpectedSubD));

        metadata.sub_d_steering_descriptor =
            RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_SUB_D));
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type26_common_mover(
            Type26CommonMoverRequest {
                entity_id: 0x047E_0001,
                entity_type: 26,
                metadata: Some(&metadata),
                position_raw: [0xC200u16 as i16, 0, 0x0E00],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(None),
                sub_d_stagger_seed: Some(INTRO2_TYPE26_SPAWN25_SUB_D_SEED),
                sub_d_frame_owner: None,
                sub_d_runtime: None,
                body_right_q31: RetailRuntimeValue::Unresolved,
                body_forward_q31: RetailRuntimeValue::Unresolved,
                body_up_q31: RetailRuntimeValue::Unresolved,
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
            || panic!("type-13 Sub-D must not start type-26 FUN_00401430"),
        );
        assert_eq!(result, Err(Type26CommonMoverBlock::UnexpectedSubD));
    }

    #[test]
    fn spawn25_applies_type26_cab_tail_and_returns_one() {
        let metadata = type26_metadata();
        let terrain = empty_terrain();
        let mut owner = intro2_type26_first_query_owner_for_seed(INTRO2_TYPE26_SPAWN25_SUB_D_SEED)
            .expect("spawn-25 seed");
        let mut runtime = Type9SubDRuntime::from_constructor();
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type26_common_mover(
            Type26CommonMoverRequest {
                entity_id: 0x047E_0001,
                entity_type: 26,
                metadata: Some(&metadata),
                position_raw: [0xC200u16 as i16, 0, 0x0E00],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(Some(
                    SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(1), 1, 100),
                )),
                sub_d_stagger_seed: Some(INTRO2_TYPE26_SPAWN25_SUB_D_SEED),
                sub_d_frame_owner: Some(&mut owner),
                sub_d_runtime: Some(&mut runtime),
                body_right_q31: RetailRuntimeValue::Known([i32::MAX, 0, 0]),
                body_forward_q31: RetailRuntimeValue::Known([0, 0, i32::MAX]),
                body_up_q31: RetailRuntimeValue::Known([0, i32::MAX, 0]),
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
            result.as_ref().map(|outcome| outcome.result),
            Ok(ChaseTargetCommonMoverReturn::NonZero)
        );
        assert!(matches!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Known(_)
        ));
        assert!(owner.classifier_cache().rows().iter().any(|row| *row != 0));
        assert_ne!(runtime.yaw_rate_raw, 0);
    }

    #[test]
    fn level1_type47_seed_does_not_apply_type26_first_query() {
        let metadata = type26_metadata();
        let mut owner = Type9SubDFrameOwner::pending_constructor_origin(
            FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0],
        );
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_type26_common_mover(
            Type26CommonMoverRequest {
                entity_id: 0x047E_0001,
                entity_type: 26,
                metadata: Some(&metadata),
                position_raw: [0xC200u16 as i16, 0, 0x0E00],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(Some(
                    SubAPropulsionRuntime::from_retail_words(RetailRuntimeValue::Known(1), 1, 100),
                )),
                sub_d_stagger_seed: Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[0]),
                sub_d_frame_owner: Some(&mut owner),
                sub_d_runtime: None,
                body_right_q31: RetailRuntimeValue::Unresolved,
                body_forward_q31: RetailRuntimeValue::Unresolved,
                body_up_q31: RetailRuntimeValue::Unresolved,
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
            result,
            Err(Type26CommonMoverBlock::SubDFirstQueryUnavailable)
        );
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
    }
}
