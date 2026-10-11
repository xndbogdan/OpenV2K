//! Shared DEGKL common-mover frame, preserving per-cohort descriptors.
//!
//! Type13 and Type10 execute the same 01430 -> 20360 -> Sub-G/K/L callbacks.
//! Native publication owns allocation/binding admission; this frame retains
//! target-first exits, the pre-Sub-D basis, dispatch-mode asymmetry and the
//! actual Sub-D/Sub-G/Sub-L parameters. It does not authenticate a birth.

use crate::chase_target::ChaseTargetCommonMoverReturn;
use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameAdvanceError, CommonMoverFrameBlock,
    CommonMoverFrameCommitPhase, CommonMoverFrameConfiguration, CommonMoverFrameMachine,
    CommonMoverFramePoll, CommonMoverFrameProtocolError, CommonMoverFrameRequest,
    CommonMoverFrameResume, CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_d::{
    apply_type9_sub_d, Type9SubDFrameOwner, Type9SubDRuntime, Type9SubDStep,
};
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity_collision_state::{
    CommonMoverComponentTopology, CommonMoverGklPayloads, EntityTypeRuntimeMetadata,
    RetailRuntimeValue,
};
use crate::sub_g_runtime::{
    plan_sub_g_frame_with_prefix, SubG06070RuntimeState, Type13SubGFrameBlock,
    Type13SubGFrameRequest, Type13SubGSound,
};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

use v2k_formats::collision::SubDSteeringDescriptor;

#[path = "type13_common_mover/component_outputs.rs"]
pub(crate) mod component_outputs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GklCommonMoverTopologyError {
    WrongEntityType { actual: u32 },
    UnresolvedComponentTopology,
    UnsupportedComponentTopology,
    UnresolvedGklPayloads,
    UnsupportedGklPayloads,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GklCommonMoverProfile {
    pub topology: CommonMoverComponentTopology,
    pub sub_d: SubDSteeringDescriptor,
    pub gkl: CommonMoverGklPayloads,
}

impl GklCommonMoverProfile {
    pub(crate) fn authenticate_gkl_payloads(
        self,
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<(), GklCommonMoverTopologyError> {
        match metadata.common_mover_gkl_payloads {
            RetailRuntimeValue::Known(payloads) if payloads == self.gkl => Ok(()),
            RetailRuntimeValue::Known(_) => {
                Err(GklCommonMoverTopologyError::UnsupportedGklPayloads)
            }
            RetailRuntimeValue::Unresolved => {
                Err(GklCommonMoverTopologyError::UnresolvedGklPayloads)
            }
        }
    }
}

/// DEGKL component state staged across one detached common-mover call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GklCommonMoverRuntime {
    pub sub_d_runtime: Type9SubDRuntime,
    pub sub_d_frame_owner: Type9SubDFrameOwner,
    pub sub_k_smoothed_raw: i32,
    /// Signed animation words bound by Sub-K `+0/+4` to selectors 11/10.
    pub sub_k_output_raw: [i16; 2],
    pub sub_l_target_raw: [i16; 3],
    pub sub_l_exact_raw: i32,
    /// Signed animation words bound by Sub-L `+0/+4` to selectors 6/7.
    pub sub_l_output_raw: [i16; 2],
}

impl GklCommonMoverRuntime {
    /// Exact Intro2 spawn-0 constructor image admitted by `V200001.run`.
    ///
    /// The capture joins handle `04fc0001` to Sub-D seed `0`, cache origin
    /// `[0, 0]`, and eight cleared rows. Sub-K/Sub-L allocations begin zeroed.
    /// Their pointed-to output words are independently zeroed: FUN_00409A80
    /// allocates the entity animation bank (`type +0x110` words), clears its
    /// full byte span with FUN_00457370, then binds K/L through FUN_0040A950.
    pub const fn from_intro2_constructor_capture() -> Self {
        Self {
            sub_d_runtime: Type9SubDRuntime::from_constructor(),
            sub_d_frame_owner: Type9SubDFrameOwner::from_retail_state([0; 8], [0; 2], 0),
            sub_k_smoothed_raw: 0,
            sub_k_output_raw: [0; 2],
            sub_l_target_raw: [0; 3],
            sub_l_exact_raw: 0,
            sub_l_output_raw: [0; 2],
        }
    }

    /// Ordinary 09A80 image around the process's own Sub-D allocation.
    ///
    /// K `FUN_00424450` and L `FUN_0041BB80` allocate 0x10/0x14 bytes and
    /// clear them with FUN_00457370 (Ghidra 12.1.4), so their state begins
    /// zeroed without any capture; the bound bank words are cleared as above.
    pub(crate) const fn from_native_constructor(
        sub_d_runtime: Type9SubDRuntime,
        sub_d_frame_owner: Type9SubDFrameOwner,
    ) -> Self {
        Self {
            sub_d_runtime,
            sub_d_frame_owner,
            sub_k_smoothed_raw: 0,
            sub_k_output_raw: [0; 2],
            sub_l_target_raw: [0; 3],
            sub_l_exact_raw: 0,
            sub_l_output_raw: [0; 2],
        }
    }
}

/// Complete detached result of one admitted DEGKL `FUN_00401430` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GklCommonMoverOutcome {
    pub result: ChaseTargetCommonMoverReturn,
    pub runtime: GklCommonMoverRuntime,
    pub sub_g_runtime: SubG06070RuntimeState,
    pub target_private: WanderNearPrivateState,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub pitch_raw: i16,
    pub roll_raw: u16,
    pub sound: Option<Type13SubGSound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GklCommonMoverBlock {
    Topology(GklCommonMoverTopologyError),
    TypeMetadataUnavailable,
    SubDUnresolved,
    UnexpectedSubD,
    Frame(CommonMoverFrameBlock),
    Protocol(CommonMoverFrameProtocolError),
    BodyBasisUnavailable,
    SubD(Type9SubDStep),
    SubG(Type13SubGFrameBlock),
    UnexpectedPostSubDWrites,
    UnexpectedFrameAction,
}

/// Writes actually reached before a blocked live mover. An absent field is
/// never published: in particular, a target/basis rejection cannot overwrite
/// untouched G/K/L allocations or the physical body basis.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GklCommonMoverPrefix {
    pub target_private: Option<WanderNearPrivateState>,
    pub heading_raw: Option<u16>,
    pub pitch_raw: Option<i16>,
    pub roll_raw: Option<u16>,
    pub velocity_raw: Option<[i16; 3]>,
    pub sub_d_runtime: Option<Type9SubDRuntime>,
    pub sub_d_frame_owner: Option<Type9SubDFrameOwner>,
    pub sub_g_reverse_write: Option<bool>,
    pub sub_g_runtime: Option<SubG06070RuntimeState>,
    pub sub_k_smoothed_raw: Option<i32>,
    pub sub_k_output_raw: Option<[i16; 2]>,
    pub sub_l_target_raw: Option<[i16; 3]>,
    pub sub_l_exact_raw: Option<i32>,
    pub sub_l_output_raw: Option<[i16; 2]>,
    pub sound: Option<Type13SubGSound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GklCommonMoverFailure {
    pub reason: GklCommonMoverBlock,
    pub prefix: GklCommonMoverPrefix,
}

#[derive(Debug)]
pub struct GklCommonMoverRequest<'a> {
    pub entity_id: u32,
    pub entity_type: u32,
    pub metadata: Option<&'a EntityTypeRuntimeMetadata>,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub pitch_raw: i16,
    pub roll_raw: u16,
    pub body_basis: RetailRuntimeValue<Type9BodyBasis>,
    pub runtime: GklCommonMoverRuntime,
    pub sub_g_runtime: SubG06070RuntimeState,
    pub target_private: WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub terrain: &'a TerrainGrid,
    /// Section-8 active-model header word `+0x08` (the port's `radius`).
    pub active_model_extent_raw: u16,
    pub self_mass_raw: u16,
    pub attached_cargo_mass: u32,
    pub capability_flags: u32,
    pub retail_tick: u32,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

fn map_advance(error: CommonMoverFrameAdvanceError) -> GklCommonMoverBlock {
    match error {
        CommonMoverFrameAdvanceError::Block(block) => GklCommonMoverBlock::Frame(block),
        CommonMoverFrameAdvanceError::Protocol(error) => GklCommonMoverBlock::Protocol(error),
    }
}

pub(crate) fn evaluate_gkl_common_mover(
    request: GklCommonMoverRequest<'_>,
    profile: GklCommonMoverProfile,
    next_random: impl FnMut() -> u32,
) -> Result<GklCommonMoverOutcome, GklCommonMoverFailure> {
    let mut prefix = GklCommonMoverPrefix::default();
    let result = (|| {
        let metadata = request
            .metadata
            .ok_or(GklCommonMoverBlock::TypeMetadataUnavailable)?;
        let topology = match metadata.common_mover_topology {
            RetailRuntimeValue::Known(topology) if topology == profile.topology => topology,
            RetailRuntimeValue::Known(_) => {
                return Err(GklCommonMoverBlock::Topology(
                    GklCommonMoverTopologyError::UnsupportedComponentTopology,
                ))
            }
            RetailRuntimeValue::Unresolved => {
                return Err(GklCommonMoverBlock::Topology(
                    GklCommonMoverTopologyError::UnresolvedComponentTopology,
                ))
            }
        };
        let sub_d_descriptor = match metadata.sub_d_steering_descriptor {
            RetailRuntimeValue::Unresolved => return Err(GklCommonMoverBlock::SubDUnresolved),
            RetailRuntimeValue::Known(None) => return Err(GklCommonMoverBlock::UnexpectedSubD),
            RetailRuntimeValue::Known(Some(descriptor)) if descriptor == profile.sub_d => {
                CommonMoverPreludeSubD {
                    steering_divisor_raw: descriptor.steering_divisor_raw,
                    couple_yaw_into_roll: descriptor.couple_yaw_into_roll_raw != 0,
                }
            }
            RetailRuntimeValue::Known(Some(_)) => return Err(GklCommonMoverBlock::UnexpectedSubD),
        };
        let initial_body_basis = match request.body_basis {
            RetailRuntimeValue::Known(basis) => Some(basis),
            RetailRuntimeValue::Unresolved => None,
        };
        let snapshot = CommonMoverFrameSnapshot {
            controlled_entity_lookup: None,
            position_raw: request.position_raw,
            velocity_raw: request.velocity_raw,
            heading_raw: request.heading_raw,
            roll_raw: request.roll_raw,
            body_up_q31: initial_body_basis.map_or([0; 3], |basis| basis.up),
            body_right_q31: initial_body_basis.map_or([0; 3], |basis| basis.lateral),
            body_forward_q31: initial_body_basis.map_or([0; 3], |basis| basis.forward),
            attached_cargo_mass: request.attached_cargo_mass,
            sub_a_runtime: RetailRuntimeValue::Known(None),
            sub_f_smoothed_raw: None,
            // Type-13 has no Sub-O secondary correction, so these words are not
            // consumed by the frame machine. The Sub-G planner resolves the full
            // allocation only after retail's target-first zero return.
            sub_g_runtime_angle_raw: 0,
            sub_g_state_byte_3c: 0,
            sub_k_smoothed_raw: Some(request.runtime.sub_k_smoothed_raw),
            sub_n_accumulator_raw: None,
            sub_o_link_raw: 0,
        };
        let mut machine = CommonMoverFrameMachine::start(
            CommonMoverFrameRequest {
                configuration: CommonMoverFrameConfiguration {
                    controlled_entity_handle: request.entity_id,
                    topology: topology,
                    dispatch_mode: request.dispatch_mode,
                    elapsed_micros: request.elapsed_micros,
                    sub_a_descriptor: None,
                    sub_b_descriptor: None,
                    sub_c_descriptor: None,
                    sub_d_descriptor: Some(sub_d_descriptor),
                    target_resource_context_present: true,
                },
                initial_snapshot: snapshot,
                target_private: Some(request.target_private),
                tracked_target: request.tracked_target,
            },
            next_random,
        )
        .map_err(GklCommonMoverBlock::Frame)?;

        let mut live = snapshot;
        let mut runtime = request.runtime;
        let mut sub_g_runtime = request.sub_g_runtime;
        let mut target_private = request.target_private;
        let mut sub_g_reverse_write = None;
        let mut retained_body_basis = None;
        let mut pitch_raw = request.pitch_raw;
        let mut sound = None;

        loop {
            match machine.poll() {
                CommonMoverFramePoll::ReturnZero(_) => {
                    return Ok(GklCommonMoverOutcome {
                        result: ChaseTargetCommonMoverReturn::Zero,
                        runtime,
                        sub_g_runtime,
                        target_private,
                        position_raw: live.position_raw,
                        velocity_raw: live.velocity_raw,
                        heading_raw: live.heading_raw,
                        pitch_raw,
                        roll_raw: live.roll_raw,
                        sound,
                    });
                }
                CommonMoverFramePoll::ReturnOne => {
                    return Ok(GklCommonMoverOutcome {
                        result: ChaseTargetCommonMoverReturn::NonZero,
                        runtime,
                        sub_g_runtime,
                        target_private,
                        position_raw: live.position_raw,
                        velocity_raw: live.velocity_raw,
                        heading_raw: live.heading_raw,
                        pitch_raw,
                        roll_raw: live.roll_raw,
                        sound,
                    });
                }
                CommonMoverFramePoll::Blocked(block) => {
                    return Err(GklCommonMoverBlock::Frame(block));
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(plan)) => {
                    target_private = plan.target_state;
                    prefix.target_private = Some(target_private);
                    live.heading_raw = plan.heading_raw;
                    live.roll_raw = plan.roll_raw;
                    if let Some(write) = plan.sub_d_reversal_write {
                        runtime.sub_d_runtime.last_yaw_step_raw = write.step_raw as i16;
                        prefix.sub_d_runtime = Some(runtime.sub_d_runtime);
                        prefix.heading_raw = Some(live.heading_raw);
                        prefix.roll_raw = Some(live.roll_raw);
                    }
                    let Some(sub_l_target_raw) = plan.sub_l_target_write else {
                        return Err(GklCommonMoverBlock::UnexpectedFrameAction);
                    };
                    runtime.sub_l_target_raw = sub_l_target_raw;
                    prefix.sub_l_target_raw = Some(sub_l_target_raw);
                    sub_g_reverse_write = plan.sub_g_reverse_write;
                    prefix.sub_g_reverse_write = sub_g_reverse_write;
                    machine
                        .resume(CommonMoverFrameResume::Committed {
                            phase: CommonMoverFrameCommitPhase::TargetPrelude,
                            snapshot: live,
                        })
                        .map_err(map_advance)?;
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD {
                    target_position_raw,
                    ..
                }) => {
                    let body_basis = match request.body_basis {
                        RetailRuntimeValue::Known(basis) => basis,
                        RetailRuntimeValue::Unresolved => {
                            return Err(GklCommonMoverBlock::BodyBasisUnavailable);
                        }
                    };
                    retained_body_basis = Some(body_basis);
                    live.body_up_q31 = body_basis.up;
                    live.body_right_q31 = body_basis.lateral;
                    live.body_forward_q31 = body_basis.forward;
                    let evidence = runtime
                        .sub_d_frame_owner
                        .evidence_for_classifier_free_frame(
                            profile.sub_d,
                            live.position_raw,
                            target_position_raw,
                            body_basis.lateral,
                            body_basis.forward,
                        )
                        .ok_or(GklCommonMoverBlock::UnexpectedSubD)?;
                    prefix.sub_d_frame_owner = Some(runtime.sub_d_frame_owner);
                    let yaw_step_raw = match apply_type9_sub_d(
                        profile.sub_d,
                        &mut runtime.sub_d_runtime,
                        evidence,
                        request.elapsed_micros,
                        request.global_elapsed_micros,
                    ) {
                        Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                        block => return Err(GklCommonMoverBlock::SubD(block)),
                    };
                    live.heading_raw = live.heading_raw.wrapping_sub(yaw_step_raw as u16);
                    live.roll_raw = live.roll_raw.wrapping_sub(yaw_step_raw as u16);
                    prefix.sub_d_runtime = Some(runtime.sub_d_runtime);
                    prefix.heading_raw = Some(live.heading_raw);
                    prefix.roll_raw = Some(live.roll_raw);
                    // FUN_00420360 writes only the angle words and Sub-D step.
                    // Sub-G immediately consumes the retained pre-call matrix;
                    // no FUN_00413F70 basis rebuild occurs at this boundary.
                    machine
                        .resume(CommonMoverFrameResume::SubDReturned {
                            result_raw: yaw_step_raw,
                            snapshot: live,
                        })
                        .map_err(map_advance)?;
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(
                    plan,
                )) => {
                    let (Some(sub_k_smoothed_raw), Some(sub_l_exact_raw)) =
                        (plan.sub_k_smoothed_raw, plan.sub_l_exact_raw)
                    else {
                        return Err(GklCommonMoverBlock::UnexpectedPostSubDWrites);
                    };
                    if plan.sub_f.is_some() {
                        return Err(GklCommonMoverBlock::UnexpectedPostSubDWrites);
                    }
                    runtime.sub_k_smoothed_raw = sub_k_smoothed_raw;
                    runtime.sub_l_exact_raw = sub_l_exact_raw;
                    prefix.sub_k_smoothed_raw = Some(sub_k_smoothed_raw);
                    prefix.sub_l_exact_raw = Some(sub_l_exact_raw);
                    live.sub_k_smoothed_raw = Some(sub_k_smoothed_raw);
                    machine
                        .resume(CommonMoverFrameResume::Committed {
                            phase: CommonMoverFrameCommitPhase::PostSubDWrites,
                            snapshot: live,
                        })
                        .map_err(map_advance)?;
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                    phase: CommonMoverDispatchPhase::SubG,
                    controlled_entity_handle: _,
                    elapsed_micros,
                }) => {
                    profile
                        .authenticate_gkl_payloads(metadata)
                        .map_err(GklCommonMoverBlock::Topology)?;
                    let Some(retained_body_basis) = retained_body_basis else {
                        return Err(GklCommonMoverBlock::UnexpectedFrameAction);
                    };
                    let outcome = plan_sub_g_frame_with_prefix(Type13SubGFrameRequest {
                        descriptor: profile
                            .gkl
                            .sub_g
                            .as_ref()
                            .ok_or(GklCommonMoverBlock::UnexpectedFrameAction)?,
                        runtime: sub_g_runtime,
                        reverse_write: sub_g_reverse_write,
                        position_raw: live.position_raw,
                        velocity_raw: live.velocity_raw,
                        pitch_raw,
                        roll_raw: live.roll_raw,
                        retained_body_basis,
                        active_model_extent_raw: request.active_model_extent_raw,
                        self_mass_raw: request.self_mass_raw,
                        attached_cargo_mass: request.attached_cargo_mass,
                        capability_flags: request.capability_flags,
                        terrain: request.terrain,
                        retail_tick: request.retail_tick,
                        elapsed_micros,
                    })
                    .map_err(|failure| {
                        prefix.sub_g_runtime = failure.prefix.runtime;
                        prefix.velocity_raw = failure.prefix.velocity_raw;
                        prefix.pitch_raw = failure.prefix.pitch_raw;
                        if let Some(roll) = failure.prefix.roll_raw {
                            prefix.roll_raw = Some(roll);
                        }
                        prefix.sound = failure.prefix.sound;
                        GklCommonMoverBlock::SubG(failure.reason)
                    })?;
                    sub_g_runtime = outcome.runtime;
                    live.velocity_raw = outcome.velocity_raw;
                    pitch_raw = outcome.pitch_raw;
                    live.roll_raw = outcome.roll_raw;
                    live.sub_g_state_byte_3c = match sub_g_runtime.invert_target_at_0x3c() {
                        RetailRuntimeValue::Known(value) => value,
                        RetailRuntimeValue::Unresolved => unreachable!(
                            "the successful Sub-G planner always retains its resolved phase byte"
                        ),
                    };
                    sound = outcome.sound;
                    prefix.sub_g_runtime = Some(sub_g_runtime);
                    prefix.velocity_raw = Some(live.velocity_raw);
                    prefix.pitch_raw = Some(pitch_raw);
                    prefix.roll_raw = Some(live.roll_raw);
                    prefix.sound = sound;
                    machine
                        .resume(CommonMoverFrameResume::ComponentReturned {
                            phase: CommonMoverDispatchPhase::SubG,
                            snapshot: live,
                        })
                        .map_err(map_advance)?;
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                    phase: CommonMoverDispatchPhase::SubK,
                    ..
                }) => {
                    // FUN_004018A0 invokes K after G, so its second output reads
                    // the vertical velocity already written by Sub-G.
                    runtime.sub_k_output_raw = component_outputs::advance_sub_k(
                        runtime.sub_k_output_raw,
                        runtime.sub_k_smoothed_raw,
                        live.velocity_raw[1],
                    );
                    prefix.sub_k_output_raw = Some(runtime.sub_k_output_raw);
                    machine
                        .resume(CommonMoverFrameResume::ComponentReturned {
                            phase: CommonMoverDispatchPhase::SubK,
                            snapshot: live,
                        })
                        .map_err(map_advance)?;
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                    phase: CommonMoverDispatchPhase::SubL,
                    ..
                }) => {
                    let Some(retained_body_basis) = retained_body_basis else {
                        return Err(GklCommonMoverBlock::UnexpectedFrameAction);
                    };
                    runtime.sub_l_output_raw = component_outputs::advance_sub_l(
                        runtime.sub_l_output_raw,
                        runtime.sub_l_exact_raw,
                        live.position_raw,
                        runtime.sub_l_target_raw,
                        retained_body_basis,
                        profile
                            .gkl
                            .sub_l
                            .ok_or(GklCommonMoverBlock::UnexpectedFrameAction)?,
                    );
                    prefix.sub_l_output_raw = Some(runtime.sub_l_output_raw);
                    machine
                        .resume(CommonMoverFrameResume::ComponentReturned {
                            phase: CommonMoverDispatchPhase::SubL,
                            snapshot: live,
                        })
                        .map_err(map_advance)?;
                }
                CommonMoverFramePoll::Action(_) => {
                    return Err(GklCommonMoverBlock::UnexpectedFrameAction);
                }
            }
        }
    })();
    result.map_err(|reason| GklCommonMoverFailure { reason, prefix })
}
