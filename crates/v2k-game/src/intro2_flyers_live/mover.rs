//! Descriptor-authenticated Intro2 flyer binding of the shared 01430 mover.
//! The caller owns task lifetime, publication and the 12DA0 world suffix.

use super::{
    Intro2FlyerFrameOwner, FLYER_COMMON_MOVER_TOPOLOGY, FLYER_SUB_B, INTRO2_TYPE15_ENTITY_TYPE,
    INTRO2_TYPE87_ENTITY_TYPE, TYPE15_SUB_G_DESCRIPTOR, TYPE87_SUB_G_DESCRIPTOR,
};
use crate::chase_target::ChaseTargetCommonMoverReturn;
use crate::common_mover::component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase};
use crate::common_mover::frame_machine::{
    CommonMoverFrameAction, CommonMoverFrameAdvanceError, CommonMoverFrameBlock,
    CommonMoverFrameCommitPhase, CommonMoverFrameConfiguration, CommonMoverFrameMachine,
    CommonMoverFramePoll, CommonMoverFrameRequest, CommonMoverFrameResume,
    CommonMoverFrameSnapshot,
};
use crate::common_mover::sub_d::{apply_type9_sub_d, Type9SubDStep, FLYER_SUB_D};
use crate::common_mover::target_prelude::{
    CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot,
};
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::sub_g_runtime::{
    plan_type13_sub_g_frame, SubG06070RuntimeState, Type13SubGFrameBlock, Type13SubGFrameRequest,
    Type13SubGSound,
};
use crate::wander_near_location::WanderNearPrivateState;
use v2k_formats::terrain::TerrainGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2FlyerCommonMoverOutcome {
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub frame_owner: Intro2FlyerFrameOwner,
    pub sub_g_runtime: SubG06070RuntimeState,
    pub target_private: WanderNearPrivateState,
    pub sound: Option<Type13SubGSound>,
    pub result: ChaseTargetCommonMoverReturn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerCommonMoverBlock {
    MissingFrameOwner,
    MissingTarget,
    MetadataMismatch,
    BodyBasisUnavailable,
    SubGRuntimeUnavailable,
    Frame(CommonMoverFrameBlock),
    FrameAdvance(CommonMoverFrameAdvanceError),
    SubD(Type9SubDStep),
    SubG(Type13SubGFrameBlock),
    UnexpectedFrameAction,
}

#[derive(Debug, Clone, Copy)]
pub struct Intro2FlyerCommonMoverRequest<'a> {
    pub dispatch_mode: CommonMoverDispatchMode,
    pub entity_id: u32,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub position_raw: [i16; 3],
    pub velocity_raw: [i16; 3],
    pub heading_raw: u16,
    pub pitch_raw: i16,
    pub roll_raw: i16,
    pub body_basis: RetailRuntimeValue<Type9BodyBasis>,
    pub frame_owner: Intro2FlyerFrameOwner,
    pub sub_g_runtime: SubG06070RuntimeState,
    pub target_private: WanderNearPrivateState,
    pub tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    pub terrain: &'a TerrainGrid,
    pub active_model_extent_raw: u16,
    pub self_mass_raw: u16,
    pub attached_cargo_mass: u32,
    pub capability_flags: u32,
    pub retail_tick: u32,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

pub(super) fn flyer_sub_g_descriptor(entity_type: u32) -> Option<&'static [u8; 104]> {
    match entity_type {
        INTRO2_TYPE15_ENTITY_TYPE => Some(&TYPE15_SUB_G_DESCRIPTOR),
        INTRO2_TYPE87_ENTITY_TYPE => Some(&TYPE87_SUB_G_DESCRIPTOR),
        _ => None,
    }
}

pub(crate) fn authenticate_flyer_mover_metadata(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<&'static [u8; 104], Intro2FlyerCommonMoverBlock> {
    let descriptor =
        flyer_sub_g_descriptor(entity_type).ok_or(Intro2FlyerCommonMoverBlock::MetadataMismatch)?;
    if metadata.common_mover_topology != RetailRuntimeValue::Known(FLYER_COMMON_MOVER_TOPOLOGY)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(Some(FLYER_SUB_B))
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(FLYER_SUB_D))
        || !matches!(metadata.common_mover_gkl_payloads, RetailRuntimeValue::Known(payloads)
            if payloads.sub_g == Some(*descriptor) && payloads.sub_k.is_none() && payloads.sub_l.is_none())
    {
        return Err(Intro2FlyerCommonMoverBlock::MetadataMismatch);
    }
    Ok(descriptor)
}

/// Bind the two authored B/D/E/G shapes to the shared 01430 frame machine.
///
/// 02BA0 contributes only its target prefix. 01430 performs target reversal,
/// classifier-free Sub-D and the normal 1A690/AA60/AC40/B210 flight chain.
/// Sub-G suppresses the C/A/B tail and consumes the retained pre-D matrix, including
/// after yaw has coupled into roll. Position integration belongs to the outer
/// world suffix and is deliberately absent from this callback.
pub fn evaluate_intro2_flyer_common_mover(
    request: Intro2FlyerCommonMoverRequest<'_>,
    next_random: impl FnMut() -> u32,
) -> Result<Intro2FlyerCommonMoverOutcome, Intro2FlyerCommonMoverBlock> {
    let descriptor =
        authenticate_flyer_mover_metadata(request.frame_owner.entity_type, request.metadata)?;
    let RetailRuntimeValue::Known(basis) = request.body_basis else {
        return Err(Intro2FlyerCommonMoverBlock::BodyBasisUnavailable);
    };
    let mut live = CommonMoverFrameSnapshot {
        controlled_entity_lookup: None,
        position_raw: request.position_raw,
        velocity_raw: request.velocity_raw,
        heading_raw: request.heading_raw,
        roll_raw: request.roll_raw as u16,
        body_up_q31: basis.up,
        body_right_q31: basis.lateral,
        body_forward_q31: basis.forward,
        attached_cargo_mass: request.attached_cargo_mass,
        sub_a_runtime: RetailRuntimeValue::Known(None),
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
                topology: FLYER_COMMON_MOVER_TOPOLOGY,
                dispatch_mode: request.dispatch_mode,
                elapsed_micros: request.elapsed_micros,
                sub_a_descriptor: None,
                sub_b_descriptor: Some(FLYER_SUB_B),
                sub_c_descriptor: None,
                sub_d_descriptor: Some(CommonMoverPreludeSubD {
                    steering_divisor_raw: FLYER_SUB_D.steering_divisor_raw,
                    couple_yaw_into_roll: FLYER_SUB_D.couple_yaw_into_roll_raw != 0,
                }),
                target_resource_context_present: true,
            },
            initial_snapshot: live,
            target_private: Some(request.target_private),
            tracked_target: request.tracked_target,
        },
        next_random,
    )
    .map_err(Intro2FlyerCommonMoverBlock::Frame)?;
    let mut frame_owner = request.frame_owner;
    let mut sub_g_runtime = request.sub_g_runtime;
    let mut target_private = request.target_private;
    let mut reverse_write = None;
    let mut pitch_raw = request.pitch_raw;
    let mut sound = None;
    loop {
        let resume = match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) | CommonMoverFramePoll::ReturnOne => {
                return Ok(Intro2FlyerCommonMoverOutcome {
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    heading_raw: live.heading_raw,
                    pitch_raw,
                    roll_raw: live.roll_raw as i16,
                    frame_owner,
                    sub_g_runtime,
                    target_private,
                    sound,
                    result: if matches!(machine.poll(), CommonMoverFramePoll::ReturnOne) {
                        ChaseTargetCommonMoverReturn::NonZero
                    } else {
                        ChaseTargetCommonMoverReturn::Zero
                    },
                });
            }
            CommonMoverFramePoll::Blocked(block) => {
                return Err(Intro2FlyerCommonMoverBlock::Frame(block));
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(plan)) => {
                target_private = plan.target_state;
                live.heading_raw = plan.heading_raw;
                live.roll_raw = plan.roll_raw;
                if let Some(write) = plan.sub_d_reversal_write {
                    frame_owner.sub_d_runtime.last_yaw_step_raw = write.step_raw as i16;
                }
                reverse_write = plan.sub_g_reverse_write;
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::TargetPrelude,
                    snapshot: live,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeSubD {
                target_position_raw,
                ..
            }) => {
                let evidence = frame_owner
                    .sub_d_frame_owner
                    .evidence_for_classifier_free_frame(
                        FLYER_SUB_D,
                        live.position_raw,
                        target_position_raw,
                        basis.lateral,
                        basis.forward,
                    )
                    .ok_or(Intro2FlyerCommonMoverBlock::UnexpectedFrameAction)?;
                let yaw_step_raw = match apply_type9_sub_d(
                    FLYER_SUB_D,
                    &mut frame_owner.sub_d_runtime,
                    evidence,
                    request.elapsed_micros,
                    request.global_elapsed_micros,
                ) {
                    Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                    block => return Err(Intro2FlyerCommonMoverBlock::SubD(block)),
                };
                live.heading_raw = live.heading_raw.wrapping_sub(yaw_step_raw as u16);
                live.roll_raw = live.roll_raw.wrapping_sub(yaw_step_raw as u16);
                CommonMoverFrameResume::SubDReturned {
                    result_raw: yaw_step_raw,
                    snapshot: live,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(plan)) => {
                if plan.sub_f.is_some()
                    || plan.sub_k_smoothed_raw.is_some()
                    || plan.sub_l_exact_raw.is_some()
                {
                    return Err(Intro2FlyerCommonMoverBlock::UnexpectedFrameAction);
                }
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::PostSubDWrites,
                    snapshot: live,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                phase: CommonMoverDispatchPhase::SubG,
                elapsed_micros,
                ..
            }) => {
                // The existing descriptor-bearing normal Sub-G planner has
                // no type-13 constants: these flyers retain their own force,
                // clearance, rate, speed cap, and sole selector-one binding.
                let outcome = plan_type13_sub_g_frame(Type13SubGFrameRequest {
                    descriptor,
                    runtime: sub_g_runtime,
                    reverse_write,
                    position_raw: live.position_raw,
                    velocity_raw: live.velocity_raw,
                    pitch_raw,
                    roll_raw: live.roll_raw,
                    retained_body_basis: basis,
                    active_model_extent_raw: request.active_model_extent_raw,
                    self_mass_raw: request.self_mass_raw,
                    attached_cargo_mass: request.attached_cargo_mass,
                    capability_flags: request.capability_flags,
                    terrain: request.terrain,
                    retail_tick: request.retail_tick,
                    elapsed_micros,
                })
                .map_err(Intro2FlyerCommonMoverBlock::SubG)?;
                sub_g_runtime = outcome.runtime;
                live.velocity_raw = outcome.velocity_raw;
                pitch_raw = outcome.pitch_raw;
                live.roll_raw = outcome.roll_raw;
                sound = outcome.sound;
                let RetailRuntimeValue::Known(outputs) = sub_g_runtime.animation_outputs_raw()
                else {
                    return Err(Intro2FlyerCommonMoverBlock::SubGRuntimeUnavailable);
                };
                frame_owner.wing_var1 = i32::from(outputs[0]);
                CommonMoverFrameResume::ComponentReturned {
                    phase: CommonMoverDispatchPhase::SubG,
                    snapshot: live,
                }
            }
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB { velocity_raw }) => {
                live.velocity_raw = velocity_raw;
                CommonMoverFrameResume::Committed {
                    phase: CommonMoverFrameCommitPhase::SubB,
                    snapshot: live,
                }
            }
            CommonMoverFramePoll::Action(_) => {
                return Err(Intro2FlyerCommonMoverBlock::UnexpectedFrameAction);
            }
        };
        machine
            .resume(resume)
            .map_err(Intro2FlyerCommonMoverBlock::FrameAdvance)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::apply_sub_b_lateral_raw;
    use crate::common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime};
    use crate::entity_collision_state::CommonMoverGklPayloads;
    use v2k_formats::terrain::TerrainCell;

    fn terrain() -> TerrainGrid {
        TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0
                };
                256 * 256
            ],
        }
    }

    fn metadata(entity_type: u32) -> EntityTypeRuntimeMetadata {
        EntityTypeRuntimeMetadata {
            common_mover_topology: RetailRuntimeValue::Known(FLYER_COMMON_MOVER_TOPOLOGY),
            common_mover_gkl_payloads: RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: flyer_sub_g_descriptor(entity_type).copied(),
                sub_k: None,
                sub_l: None,
            }),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(Some(FLYER_SUB_B)),
            sub_d_steering_descriptor: RetailRuntimeValue::Known(Some(FLYER_SUB_D)),
            ..EntityTypeRuntimeMetadata::default()
        }
    }

    fn request<'a>(
        metadata: &'a EntityTypeRuntimeMetadata,
        terrain: &'a TerrainGrid,
        entity_type: u32,
    ) -> Intro2FlyerCommonMoverRequest<'a> {
        let position = [1000, 600, -2000];
        let mut target_private = WanderNearPrivateState::ordinary_type9(position);
        target_private.target_position_raw = [4000, 600, 3000];
        let descriptor = flyer_sub_g_descriptor(entity_type).unwrap();
        let mut sub_g = SubG06070RuntimeState::from_1b8c0_constructor(descriptor, 0);
        sub_g.apply_shared_06070_sub_g_branch(217, 20);
        Intro2FlyerCommonMoverRequest {
            dispatch_mode: CommonMoverDispatchMode::Normal,
            entity_id: 44,
            metadata,
            position_raw: position,
            velocity_raw: [477, -150, 923],
            heading_raw: 3000,
            pitch_raw: 2000,
            roll_raw: -1000,
            // Deliberately distinct from the current angle words: Sub-D and
            // Sub-G read the existing matrix, not a reconstructed substitute.
            body_basis: RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                -6000, 5000, 4000,
            )),
            frame_owner: Intro2FlyerFrameOwner {
                birth_provenance: super::super::FlyerBirthProvenance::AuthoredIntro2 {
                    spawn_index: if entity_type == 15 { 44 } else { 46 },
                },
                entity_type,
                sub_d_runtime: Type9SubDRuntime::from_constructor(),
                sub_d_frame_owner: Type9SubDFrameOwner::flyer(),
                wing_var1: 0,
            },
            sub_g_runtime: sub_g,
            target_private,
            tracked_target: RetailRuntimeValue::Known(None),
            terrain,
            active_model_extent_raw: 160,
            self_mass_raw: 100,
            attached_cargo_mass: 0,
            capability_flags: 8,
            retail_tick: 12,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
        }
    }

    #[test]
    fn flyer_chain_retains_pre_d_matrix_couples_roll_and_suppresses_lateral_tail() {
        let terrain = terrain();
        for entity_type in [15, 87] {
            let metadata = metadata(entity_type);
            let request = request(&metadata, &terrain, entity_type);
            let RetailRuntimeValue::Known(basis) = request.body_basis else {
                unreachable!()
            };
            let outcome =
                evaluate_intro2_flyer_common_mover(request, || panic!("no direction mismatch"))
                    .unwrap();
            let step = outcome.frame_owner.sub_d_runtime.last_yaw_step_raw;
            assert_ne!(step, 0);
            assert_eq!(
                outcome.heading_raw,
                request.heading_raw.wrapping_sub(step as u16)
            );
            let flight = plan_type13_sub_g_frame(Type13SubGFrameRequest {
                descriptor: flyer_sub_g_descriptor(entity_type).unwrap(),
                runtime: request.sub_g_runtime,
                reverse_write: None,
                position_raw: request.position_raw,
                velocity_raw: request.velocity_raw,
                pitch_raw: request.pitch_raw,
                roll_raw: request.roll_raw.wrapping_sub(step) as u16,
                retained_body_basis: basis,
                active_model_extent_raw: request.active_model_extent_raw,
                self_mass_raw: request.self_mass_raw,
                attached_cargo_mass: 0,
                capability_flags: 8,
                terrain: &terrain,
                retail_tick: request.retail_tick,
                elapsed_micros: request.elapsed_micros,
            })
            .unwrap();
            let mut extra_lateral_velocity = flight.velocity_raw;
            apply_sub_b_lateral_raw(
                FLYER_SUB_B,
                basis.lateral,
                &mut extra_lateral_velocity,
                request.elapsed_micros,
            );
            assert_ne!(
                extra_lateral_velocity, flight.velocity_raw,
                "incorrectly running Sub-B would change this frame"
            );
            assert_eq!(outcome.velocity_raw, flight.velocity_raw);
            assert_eq!(outcome.pitch_raw, flight.pitch_raw);
            assert_eq!(outcome.roll_raw, flight.roll_raw as i16);
            assert_eq!(
                outcome.position_raw, request.position_raw,
                "01430 does not integrate the entity position"
            );
            assert_eq!(
                outcome.frame_owner.wing_var1, -32768,
                "AA60 uses the prior zero rate before AC40 publishes this frame's rate"
            );
            let RetailRuntimeValue::Known(outputs) = outcome.sub_g_runtime.animation_outputs_raw()
            else {
                panic!()
            };
            assert_eq!(outputs[0], outcome.frame_owner.wing_var1 as i16);
        }
    }

    #[test]
    fn flyer_asset_speed_cap_and_force_policy_stay_distinct() {
        let terrain = terrain();
        let wasp_metadata = metadata(15);
        let deathwas_metadata = metadata(87);
        let wasp_request = request(&wasp_metadata, &terrain, 15);
        let deathwas_request = request(&deathwas_metadata, &terrain, 87);
        let wasp = evaluate_intro2_flyer_common_mover(wasp_request, || 0).unwrap();
        let deathwas = evaluate_intro2_flyer_common_mover(deathwas_request, || 0).unwrap();
        assert_ne!(
            wasp.velocity_raw, deathwas.velocity_raw,
            "500 and 750 are distinct authored speed caps"
        );
        assert_eq!(
            wasp.sub_g_runtime.source_raw_at_0x20(),
            RetailRuntimeValue::Known(20)
        );
        assert_eq!(
            deathwas.sub_g_runtime.source_raw_at_0x20(),
            RetailRuntimeValue::Known(20)
        );
    }

    #[test]
    fn unresolved_basis_and_cross_type_descriptor_fail_before_rng_or_publication() {
        let terrain = terrain();
        let wasp_metadata = metadata(15);
        let mut request = request(&wasp_metadata, &terrain, 15);
        request.body_basis = RetailRuntimeValue::Unresolved;
        assert_eq!(
            evaluate_intro2_flyer_common_mover(request, || panic!("blocked before RNG")),
            Err(Intro2FlyerCommonMoverBlock::BodyBasisUnavailable)
        );
        let deathwas_metadata = metadata(87);
        request.metadata = &deathwas_metadata;
        assert_eq!(
            evaluate_intro2_flyer_common_mover(request, || panic!("blocked before RNG")),
            Err(Intro2FlyerCommonMoverBlock::MetadataMismatch)
        );
    }
}
