//! Intro2 Type-47 binding of shared `FUN_00401430`.
//!
//! The accepted Intro2 transcript records scheduler mode 0 for type 47.
//! Authored type 47 is A/B/C/D/E/H/J, so Normal dispatch selects Sub-H
//! after the outer Sub-D query. This owner starts the detached frame
//! machine, commits the recovered target prelude, and applies the captured
//! first `FUN_0041FCB0` full-reset when the caller supplies one of the
//! TTD-closed Intro2 seeds `0x06/0x07/0x08`. Shared `FUN_0041F660` yaw
//! uses `classifier_flags` `0x13` (steepness, water, then object) and
//! divisor 64. Normal dispatch then runs the detached six-record Sub-H
//! writer and the authored terrain-only C -> A -> B tail. The live Intro2
//! visit commits that applied pose. Level-1 seeds `0x2B/0x2C/0x2D` and
//! type 13 never enter. It does not invent a mover return.

use crate::chase_target::ChaseTargetCommonMoverReturn;
use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameBlock, CommonMoverFrameCommitPhase,
    CommonMoverFrameConfiguration, CommonMoverFrameMachine, CommonMoverFramePoll,
    CommonMoverFrameRequest, CommonMoverFrameResume, CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_c::SubCSurfaceSample;
use crate::common_mover::sub_d::{
    apply_type9_sub_d, intro2_type47_first_query_owner_for_seed, Type9SubDFrameOwner,
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

/// Intro2 `FUN_00401430` scheduler-mode word for type 47.
const TYPE47_INTRO2_SCHEDULER_MODE: i32 = 0;
const INTRO2_TYPE47_ENTITY_TYPE: u32 = 47;

/// TTD `V200001.run` Intro2 Type-47 identities, construction order.
pub const INTRO2_TYPE47_TTD_HANDLES: [u32; 3] = [0x04F6_0001, 0x04F5_0001, 0x04F4_0001];
pub const INTRO2_TYPE47_TTD_POSITIONS_RAW: [[i16; 3]; 3] = [
    [0xC000u16 as i16, 0, 0x1600],
    [0xBB00u16 as i16, 0, 0x7B00],
    [0xBE00u16 as i16, 0, 0x7C00],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47CommonMoverTopologyError {
    WrongEntityType { actual: u32 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type47CommonMoverTopology {
    components: CommonMoverComponentTopology,
}

impl Intro2Type47CommonMoverTopology {
    pub fn from_metadata(
        entity_type: u32,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Intro2Type47CommonMoverTopologyError> {
        if entity_type != INTRO2_TYPE47_ENTITY_TYPE {
            return Err(Intro2Type47CommonMoverTopologyError::WrongEntityType {
                actual: entity_type,
            });
        }
        let components = match metadata.common_mover_topology {
            RetailRuntimeValue::Unresolved => {
                return Err(Intro2Type47CommonMoverTopologyError::UnresolvedComponentTopology);
            }
            RetailRuntimeValue::Known(components) => components,
        };
        if components != FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY {
            return Err(Intro2Type47CommonMoverTopologyError::UnsupportedComponentTopology);
        }
        Ok(Self { components })
    }

    pub const fn components(self) -> CommonMoverComponentTopology {
        self.components
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47CommonMoverBlock {
    Topology(Intro2Type47CommonMoverTopologyError),
    TypeMetadataUnavailable,
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
    NativeRuntime(&'static str),
    NativeAction(CommonMoverFrameAction),
    NativeAdvance(crate::common_mover::frame_machine::CommonMoverFrameAdvanceError),
}

pub(crate) struct NativeType47MoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub target: &'a mut WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
}

/// Native 01430 shares the component interpreter that commits each reached
/// body prefix. 20360 changes angle words; C/A/B still consume the incoming
/// body basis until the outer DCA0 rebuild after all task visits.
pub(crate) fn run_native_type47_common_mover(
    entity: &mut crate::entity::Entity,
    frame: NativeType47MoverFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type47CommonMoverOutcome, Intro2Type47CommonMoverBlock> {
    use crate::intro2_common_mover::{
        run_intro2_common_mover, Intro2CommonMoverBlock, Intro2CommonMoverFrame,
    };
    crate::intro2_type47_live::authenticate_type47_metadata(frame.metadata)
        .map_err(|_| Intro2Type47CommonMoverBlock::TypeMetadataUnavailable)?;
    if !entity
        .native_type47_construction
        .as_ref()
        .is_some_and(|receipt| receipt.entity_authenticates(entity))
    {
        return Err(Intro2Type47CommonMoverBlock::NativeRuntime("allocation"));
    }
    let (Some(mut sub_d_owner), Some(mut sub_d_runtime)) = (
        entity.intro2_type47_sub_d_frame_owner,
        entity.intro2_type47_sub_d_runtime,
    ) else {
        return Err(Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable);
    };
    let result = run_intro2_common_mover(
        entity,
        Intro2CommonMoverFrame {
            metadata: frame.metadata,
            topology: FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
            terrain: frame.terrain,
            wave_tick_50hz: None,
            dispatch_mode: CommonMoverDispatchMode::Normal,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
            sub_d_runtime: &mut sub_d_runtime,
            sub_d_owner: &mut sub_d_owner,
            kl_components: None,
            target: Some(frame.target),
            tracked_target: frame.tracked_target,
        },
        next_random,
    );
    entity.intro2_type47_sub_d_frame_owner = Some(sub_d_owner);
    entity.intro2_type47_sub_d_runtime = Some(sub_d_runtime);
    let nonzero = result.map_err(|error| match error {
        Intro2CommonMoverBlock::UnsupportedTopology => Intro2Type47CommonMoverBlock::Topology(
            Intro2Type47CommonMoverTopologyError::UnsupportedComponentTopology,
        ),
        Intro2CommonMoverBlock::Runtime(reason) => {
            Intro2Type47CommonMoverBlock::NativeRuntime(reason)
        }
        Intro2CommonMoverBlock::SubDFirstQuery { .. } => {
            Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable
        }
        Intro2CommonMoverBlock::SubD(error) => Intro2Type47CommonMoverBlock::SubD(error),
        Intro2CommonMoverBlock::SubH(error) => Intro2Type47CommonMoverBlock::SubH(error),
        Intro2CommonMoverBlock::Mover(error) => Intro2Type47CommonMoverBlock::Frame(error),
        Intro2CommonMoverBlock::MoverAction(action) => {
            Intro2Type47CommonMoverBlock::NativeAction(action)
        }
        Intro2CommonMoverBlock::MoverAdvance(error) => {
            Intro2Type47CommonMoverBlock::NativeAdvance(error)
        }
    })?;
    Ok(Intro2Type47CommonMoverOutcome {
        result: if nonzero {
            ChaseTargetCommonMoverReturn::NonZero
        } else {
            ChaseTargetCommonMoverReturn::Zero
        },
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        heading_raw: entity.heading_raw(),
        sub_a_runtime: entity.sub_a_propulsion_runtime,
    })
}

#[derive(Debug)]
pub struct Intro2Type47CommonMoverRequest<'a> {
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
pub struct Intro2Type47CommonMoverOutcome {
    pub result: ChaseTargetCommonMoverReturn,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub sub_a_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
}

pub fn evaluate_intro2_type47_common_mover(
    mut request: Intro2Type47CommonMoverRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Intro2Type47CommonMoverOutcome, Intro2Type47CommonMoverBlock> {
    let metadata = request
        .metadata
        .ok_or(Intro2Type47CommonMoverBlock::TypeMetadataUnavailable)?;
    let topology = Intro2Type47CommonMoverTopology::from_metadata(request.entity_type, metadata)
        .map_err(Intro2Type47CommonMoverBlock::Topology)?;
    let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
        RetailRuntimeValue::Unresolved => {
            return Err(Intro2Type47CommonMoverBlock::SubDUnresolved);
        }
        RetailRuntimeValue::Known(None) => {
            return Err(Intro2Type47CommonMoverBlock::UnexpectedSubD);
        }
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor == TYPE47_SUB_D => {
            CommonMoverPreludeSubD {
                steering_divisor_raw: descriptor.steering_divisor_raw,
                couple_yaw_into_roll: descriptor.couple_yaw_into_roll_raw != 0,
            }
        }
        RetailRuntimeValue::Known(Some(_)) => {
            return Err(Intro2Type47CommonMoverBlock::UnexpectedSubD);
        }
    };
    let sub_a_descriptor = match metadata.sub_a_propulsion_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor == TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR =>
        {
            descriptor
        }
        _ => return Err(Intro2Type47CommonMoverBlock::UnexpectedSubA),
    };
    let sub_b_descriptor = match metadata.sub_b_lateral_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor == TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR =>
        {
            descriptor
        }
        _ => return Err(Intro2Type47CommonMoverBlock::UnexpectedSubB),
    };
    let sub_c_descriptor = match metadata.sub_c_lift_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor == TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR =>
        {
            HoverLiftConfig::from(descriptor)
        }
        _ => return Err(Intro2Type47CommonMoverBlock::UnexpectedSubC),
    };
    let sub_h_descriptor = match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.records.as_slice() == TYPE47_COMMON_DYING_SUB_H_RECORDS =>
        {
            descriptor
        }
        _ => return Err(Intro2Type47CommonMoverBlock::UnexpectedSubH),
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
                    TYPE47_INTRO2_SCHEDULER_MODE,
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
    .map_err(Intro2Type47CommonMoverBlock::Frame)?;

    let mut live = snapshot;
    let mut sub_d_frame_owner = request.sub_d_frame_owner;
    let mut sub_d_runtime = request.sub_d_runtime;
    loop {
        match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) => {
                return Ok(Intro2Type47CommonMoverOutcome {
                    result: ChaseTargetCommonMoverReturn::Zero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    sub_a_runtime: live.sub_a_runtime,
                });
            }
            CommonMoverFramePoll::ReturnOne => {
                return Ok(Intro2Type47CommonMoverOutcome {
                    result: ChaseTargetCommonMoverReturn::NonZero,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    sub_a_runtime: live.sub_a_runtime,
                });
            }
            CommonMoverFramePoll::Blocked(block) => {
                return Err(Intro2Type47CommonMoverBlock::Frame(block));
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD { .. }) => {
                let Some(owner) = sub_d_frame_owner.as_deref_mut() else {
                    return Err(Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable);
                };
                if intro2_type47_first_query_owner_for_seed(request.sub_d_stagger_seed.unwrap_or(0))
                    .is_none()
                    || !owner.classifier_cache().can_classify()
                {
                    return Err(Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable);
                }
                let Some(runtime) = sub_d_runtime.as_deref_mut() else {
                    return Err(Intro2Type47CommonMoverBlock::SubDSteeringUnavailable);
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
                    return Err(Intro2Type47CommonMoverBlock::BodyBasisUnavailable);
                };
                let Some(terrain) = request.terrain else {
                    return Err(Intro2Type47CommonMoverBlock::SubDSteeringUnavailable);
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
                    blocked => return Err(Intro2Type47CommonMoverBlock::SubD(blocked)),
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubH,
                ..
            }) => {
                let Some(runtime) = request.sub_h_runtime.as_deref_mut() else {
                    return Err(Intro2Type47CommonMoverBlock::SubHUnavailable);
                };
                let cues = runtime
                    .update(sub_h_descriptor, request.elapsed_micros as i32)
                    .map_err(Intro2Type47CommonMoverBlock::SubH)?;
                if !cues.is_empty() {
                    return Err(Intro2Type47CommonMoverBlock::SubHCompletionCue);
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubHUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::SampleSubCSurface {
                point_raw,
                use_wave_surface,
            }) => {
                if use_wave_surface {
                    return Err(Intro2Type47CommonMoverBlock::SubCWaveUnexpected);
                }
                let Some(terrain) = request.terrain else {
                    return Err(Intro2Type47CommonMoverBlock::SubDSteeringUnavailable);
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubDSteeringUnavailable,
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubDSteeringUnavailable,
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubDSteeringUnavailable,
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
                        ) => Intro2Type47CommonMoverBlock::Frame(block),
                        crate::common_mover::frame_machine::CommonMoverFrameAdvanceError::Protocol(
                            _,
                        ) => Intro2Type47CommonMoverBlock::SubDSteeringUnavailable,
                    }
                    })?;
            }
            CommonMoverFramePoll::Action(_) => {
                return Err(Intro2Type47CommonMoverBlock::SubDSteeringUnavailable);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::component_dispatch::CommonMoverDispatchPlan;
    use crate::common_mover::sub_d::{INTRO2_TYPE26_SUB_D, ORDINARY_TYPE9_SUB_D};
    use crate::intro2_type26_defecate_virus::INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY;
    use crate::ordinary_type47_live::FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    #[v2k_test_support::retail_test]
    fn native_mover_keeps_target_and_sub_d_prefix_when_later_sub_h_is_unavailable() {
        let Some((session, mut manager, id)) = crate::shared_type47::tests::native_fixture() else {
            return;
        };
        let metadata = manager.type_runtime_metadata(47).unwrap().clone();
        let entity = manager.entity_mut(id).unwrap();
        let origin = entity.position_raw();
        let basis = entity.physical_body_basis_q31;
        let cache_before = entity
            .intro2_type47_sub_d_frame_owner
            .unwrap()
            .classifier_cache()
            .stagger_counter();
        assert_eq!(
            cache_before, 0xd3,
            "native process seed lies outside both replay groups"
        );
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
        let target_position = [
            origin[0].wrapping_add(3_000),
            origin[1],
            origin[2].wrapping_add(4_000),
        ];
        let mut target = WanderNearPrivateState::tracked_entity(origin, 123);
        let result = run_native_type47_common_mover(
            entity,
            NativeType47MoverFrame {
                metadata: &metadata,
                terrain: session.cache.terrain().unwrap(),
                elapsed_micros: 20_000,
                global_elapsed_micros: 31_415,
                target: &mut target,
                tracked_target: RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
                    state_flags: crate::entity_collision_state::RetailStateWord::exact(4),
                    position_raw: target_position,
                    velocity_raw: [0; 3],
                })),
            },
            &mut || panic!("matching direction prelude and native classifier do not consume RNG"),
        );
        assert_eq!(
            result,
            Err(Intro2Type47CommonMoverBlock::NativeRuntime(
                "Sub-H allocation"
            ))
        );
        assert_eq!(target.target_position_raw, target_position);
        let cache = entity.intro2_type47_sub_d_frame_owner.unwrap();
        assert_eq!(
            cache.classifier_cache().stagger_counter(),
            cache_before.wrapping_add(1)
        );
        assert!(matches!(
            cache.classifier_cache().origin(),
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(
            entity.physical_body_basis_q31, basis,
            "01430 retains the incoming matrix"
        );
        assert_eq!(
            entity.position_raw(),
            origin,
            "blocked before C/A/B and master integration"
        );
    }

    #[v2k_test_support::retail_test]
    fn native_mover_rejects_lost_receipt_before_target_or_sub_d_mutation() {
        let Some((session, mut manager, id)) = crate::shared_type47::tests::native_fixture() else {
            return;
        };
        let metadata = manager.type_runtime_metadata(47).unwrap().clone();
        let entity = manager.entity_mut(id).unwrap();
        entity.native_type47_construction = None;
        let before = entity.intro2_type47_sub_d_frame_owner;
        let mut target = WanderNearPrivateState::tracked_entity(entity.position_raw(), 123);
        let target_before = target;
        let result = run_native_type47_common_mover(
            entity,
            NativeType47MoverFrame {
                metadata: &metadata,
                terrain: session.cache.terrain().unwrap(),
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                target: &mut target,
                tracked_target: RetailRuntimeValue::Known(None),
            },
            &mut || panic!("unowned allocation cannot enter the mover"),
        );
        assert_eq!(
            result,
            Err(Intro2Type47CommonMoverBlock::NativeRuntime("allocation"))
        );
        assert_eq!(target, target_before);
        assert_eq!(entity.intro2_type47_sub_d_frame_owner, before);
    }

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
        assert!(Intro2Type47CommonMoverTopology::from_metadata(47, &metadata).is_ok());
        assert_eq!(
            Intro2Type47CommonMoverTopology::from_metadata(26, &metadata),
            Err(Intro2Type47CommonMoverTopologyError::WrongEntityType { actual: 26 })
        );

        let mut type26_shape = metadata;
        type26_shape.common_mover_topology =
            RetailRuntimeValue::Known(INTRO2_TYPE26_COMMON_MOVER_TOPOLOGY);
        assert_eq!(
            Intro2Type47CommonMoverTopology::from_metadata(47, &type26_shape),
            Err(Intro2Type47CommonMoverTopologyError::UnsupportedComponentTopology)
        );
    }

    #[test]
    fn intro2_mode_0_selects_type47_sub_h_after_outer_sub_d() {
        assert_eq!(
            CommonMoverDispatchMode::from_retail_word(TYPE47_INTRO2_SCHEDULER_MODE),
            CommonMoverDispatchMode::Normal
        );
        assert_eq!(
            CommonMoverDispatchPlan::from_topology(
                FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
                CommonMoverDispatchMode::Normal,
            )
            .phases(),
            [
                Some(crate::common_mover::component_dispatch::CommonMoverDispatchPhase::SubH),
                None,
                None
            ]
        );
    }

    #[test]
    fn type9_or_type26_descriptors_do_not_authorize_intro2_type47_first_query() {
        let mut metadata = type47_metadata();
        metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(ORDINARY_TYPE9_SUB_D));
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_intro2_type47_common_mover(
            Intro2Type47CommonMoverRequest {
                entity_id: INTRO2_TYPE47_TTD_HANDLES[1],
                entity_type: 47,
                metadata: Some(&metadata),
                position_raw: INTRO2_TYPE47_TTD_POSITIONS_RAW[1],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(None),
                sub_d_stagger_seed: Some(0x07),
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
            || panic!("Type-9 Sub-D must not start Intro2 Type-47 FUN_00401430"),
        );
        assert_eq!(result, Err(Intro2Type47CommonMoverBlock::UnexpectedSubD));

        metadata.sub_d_steering_descriptor = RetailRuntimeValue::Known(Some(INTRO2_TYPE26_SUB_D));
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_intro2_type47_common_mover(
            Intro2Type47CommonMoverRequest {
                entity_id: INTRO2_TYPE47_TTD_HANDLES[1],
                entity_type: 47,
                metadata: Some(&metadata),
                position_raw: INTRO2_TYPE47_TTD_POSITIONS_RAW[1],
                velocity_raw: [10, 0, 0],
                heading_raw: 0,
                roll_raw: 0,
                sub_a_runtime: RetailRuntimeValue::Known(None),
                sub_d_stagger_seed: Some(0x07),
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
            || panic!("type-26 Sub-D must not start Intro2 Type-47 FUN_00401430"),
        );
        assert_eq!(result, Err(Intro2Type47CommonMoverBlock::UnexpectedSubD));
    }

    #[test]
    fn level1_type47_seeds_cannot_apply_intro2_first_query() {
        let metadata = type47_metadata();
        let terrain = empty_terrain();
        let mut owner = intro2_type47_first_query_owner_for_seed(0x07).expect("Intro2 seed");
        let mut target_private = WanderNearPrivateState::tracked_entity([0, 0, 0], 0x047F_0001);
        let result = evaluate_intro2_type47_common_mover(
            Intro2Type47CommonMoverRequest {
                entity_id: 0x042F_000B,
                entity_type: 47,
                metadata: Some(&metadata),
                position_raw: [0xB700u16 as i16, 0, 0x7F00u16 as i16],
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
            result,
            Err(Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable)
        );
        assert_eq!(
            owner.classifier_cache().origin(),
            RetailRuntimeValue::Unresolved
        );
    }
}
