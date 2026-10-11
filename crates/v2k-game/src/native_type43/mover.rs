//! Type43's 01430: with Sub-E alone, only the target prelude can run.
//!
//! No A, D, F, G, I, L or O descriptor exists, so the prelude cannot reverse,
//! retarget or propagate a direction into a component, and 018A0/the C/A/B
//! tail have no phase. A tracked target that is dying or inactive returns zero;
//! otherwise the private target may be extrapolated and 01430 returns one.

use super::*;
use crate::{
    common_mover::{
        component_dispatch::CommonMoverDispatchMode,
        frame_machine::{
            CommonMoverFrameAction, CommonMoverFrameCommitPhase, CommonMoverFrameConfiguration,
            CommonMoverFrameMachine, CommonMoverFramePoll, CommonMoverFrameRequest,
            CommonMoverFrameResume, CommonMoverFrameSnapshot,
        },
        target_prelude::CommonMoverTrackedTargetSnapshot,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    wander_near_location::WanderNearPrivateState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type43MoverBlock {
    Allocation,
    Metadata,
    Runtime(&'static str),
    Mover(crate::common_mover::frame_machine::CommonMoverFrameBlock),
    MoverAdvance(crate::common_mover::frame_machine::CommonMoverFrameAdvanceError),
    UnexpectedAction(CommonMoverFrameAction),
}

pub(super) fn run(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    dispatch_mode: CommonMoverDispatchMode,
    elapsed_micros: u32,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Type43MoverBlock> {
    use Type43MoverBlock as Block;
    if !allocation_authenticates(entity) {
        return Err(Block::Allocation);
    }
    authenticate_metadata(metadata).map_err(|_| Block::Metadata)?;
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("physical basis"));
    };
    let [heading, _, roll] = entity.rotation_heading_pitch_roll_raw();
    let snapshot = CommonMoverFrameSnapshot {
        controlled_entity_lookup: None,
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        heading_raw: heading as u16,
        roll_raw: roll as u16,
        body_up_q31: basis.up,
        body_right_q31: basis.lateral,
        body_forward_q31: basis.forward,
        attached_cargo_mass: 0,
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
                controlled_entity_handle: entity.id,
                topology: TOPOLOGY,
                dispatch_mode,
                elapsed_micros,
                sub_a_descriptor: None,
                sub_b_descriptor: None,
                sub_c_descriptor: None,
                sub_d_descriptor: None,
                target_resource_context_present: true,
            },
            initial_snapshot: snapshot,
            target_private: Some(*target),
            tracked_target,
        },
        next_random,
    )
    .map_err(Block::Mover)?;
    // The prelude commit is the only boundary this topology can reach.
    for _ in 0..2 {
        match machine.poll() {
            CommonMoverFramePoll::ReturnZero(_) => return Ok(false),
            CommonMoverFramePoll::ReturnOne => return Ok(true),
            CommonMoverFramePoll::Blocked(reason) => return Err(Block::Mover(reason)),
            CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitTargetPrelude(plan)) => {
                if plan.sub_a_runtime.is_some()
                    || plan.sub_d_reversal_write.is_some()
                    || plan.sub_f_reverse_write.is_some()
                    || plan.sub_g_reverse_write.is_some()
                    || plan.sub_l_target_write.is_some()
                    || plan.heading_raw != snapshot.heading_raw
                    || plan.roll_raw != snapshot.roll_raw
                {
                    return Err(Block::Runtime("component write without a component"));
                }
                *target = plan.target_state;
                machine
                    .resume(CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::TargetPrelude,
                        snapshot,
                    })
                    .map_err(Block::MoverAdvance)?;
            }
            CommonMoverFramePoll::Action(action) => return Err(Block::UnexpectedAction(action)),
        }
    }
    Err(Block::Runtime("mover did not return"))
}
