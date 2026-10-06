//! B/D/F adapter for retail01430/018A0. The callback retains the incoming
//! physical matrix; DCA0 publishes the next matrix only after all task visits.

use super::live::{bits, SharedFishBlock as Block};
use crate::{
    common_mover::{
        component_dispatch::{CommonMoverDispatchMode, CommonMoverDispatchPhase},
        frame_machine::*,
        sub_d::{apply_type9_sub_d, Type9SubDStep},
        sub_f::{apply_sub_f_swimming_raw, SubFSwimmingBody, SubFSwimmingFrame},
        target_prelude::{CommonMoverPreludeSubD, CommonMoverTrackedTargetSnapshot},
    },
    entity::Entity,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    wander_near_location::WanderNearPrivateState,
};
use v2k_formats::terrain::TerrainGrid;

#[derive(Clone, Copy)]
pub(super) struct FishMoverFrame<'a> {
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub terrain: &'a TerrainGrid,
    pub dispatch_mode: CommonMoverDispatchMode,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub active_model_extent_raw: u16,
}

pub(super) fn run(
    entity: &mut Entity,
    frame: FishMoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Block> {
    super::authenticate_metadata(frame.metadata).map_err(|_| Block::Metadata)?;
    let RetailRuntimeValue::Known(topology) = frame.metadata.common_mover_topology else {
        return Err(Block::Metadata);
    };
    let RetailRuntimeValue::Known(Some(b)) = frame.metadata.sub_b_lateral_descriptor else {
        return Err(Block::Metadata);
    };
    let RetailRuntimeValue::Known(Some(d)) = frame.metadata.sub_d_steering_descriptor else {
        return Err(Block::Metadata);
    };
    let RetailRuntimeValue::Known(Some(f)) = frame.metadata.sub_f_swimming_descriptor else {
        return Err(Block::Metadata);
    };
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(Block::Runtime("fish physical basis"));
    };
    let [heading, _, roll] = entity.rotation_heading_pitch_roll_raw();
    let mut fish = entity.shared_fish_runtime.take().ok_or(Block::Allocation)?;
    // The local component allocation is restored even when a later callback
    // blocks. Earlier Sub-D/cache, target and pose writes are real prefixes.
    let result = (|| {
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
            sub_a_runtime: RetailRuntimeValue::Known(None),
            sub_f_smoothed_raw: Some(fish.sub_f.smoothed_turn_raw),
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
                    dispatch_mode: frame.dispatch_mode,
                    elapsed_micros: frame.elapsed_micros,
                    sub_a_descriptor: None,
                    sub_b_descriptor: Some(b),
                    sub_c_descriptor: None,
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
                    if let Some(write) = plan.sub_d_reversal_write {
                        fish.sub_d_runtime.last_yaw_step_raw = write.step_raw as i16;
                    }
                    if let Some(reversed) = plan.sub_f_reverse_write {
                        fish.sub_f.set_reversal_raw(u8::from(reversed));
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
                    // Type62 uses center-depth classifier20; other neutral
                    // fish use flags0. Both retain native cache maintenance.
                    // 41F660 defaults probe direction to1 without Sub-A,
                    // independently of the private task's reversal direction.
                    let evidence = fish.sub_d_owner.evidence_for_frame_with_descriptor(
                        d,
                        frame.terrain,
                        snapshot.position_raw,
                        target_position_raw,
                        basis.lateral,
                        basis.forward,
                        1,
                    );
                    let yaw = match apply_type9_sub_d(
                        d,
                        &mut fish.sub_d_runtime,
                        evidence,
                        frame.elapsed_micros,
                        frame.global_elapsed_micros,
                    ) {
                        Type9SubDStep::Applied { yaw_step_raw } => yaw_step_raw,
                        reason => return Err(Block::SubD(reason)),
                    };
                    snapshot.heading_raw = snapshot.heading_raw.wrapping_sub(yaw as u16);
                    commit_snapshot(entity, snapshot);
                    CommonMoverFrameResume::SubDReturned {
                        result_raw: yaw,
                        snapshot,
                    }
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitPostSubDWrites(
                    plan,
                )) => {
                    let write = plan.sub_f.ok_or(Block::Runtime("fish post-D F write"))?;
                    fish.sub_f.smoothed_turn_raw = write.smoothed_raw;
                    fish.sub_f.target_position_raw = write.target_position_raw;
                    snapshot.sub_f_smoothed_raw = Some(write.smoothed_raw);
                    CommonMoverFrameResume::Committed {
                        phase: CommonMoverFrameCommitPhase::PostSubDWrites,
                        snapshot,
                    }
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::InvokeComponent {
                    phase: CommonMoverDispatchPhase::SubF,
                    ..
                }) => {
                    bits(entity, 0x4000)?;
                    let state_flags = entity.collision.state_flags_at_0x08.known_value_bits();
                    if state_flags == 0
                        && entity.collision.state_flags_at_0x08.known_mask() != u32::MAX
                    {
                        return Err(Block::Runtime("fish F active state"));
                    }
                    let mut body = SubFSwimmingBody {
                        state_flags,
                        position_raw: snapshot.position_raw,
                        velocity_raw: snapshot.velocity_raw,
                        pitch_raw: entity.rotation_heading_pitch_roll_raw()[1],
                        roll_raw: snapshot.roll_raw as i16,
                        forward_q31: basis.forward,
                    };
                    apply_sub_f_swimming_raw(
                        f,
                        &mut fish.sub_f,
                        &mut body,
                        SubFSwimmingFrame {
                            terrain: frame.terrain,
                            elapsed_micros: frame.elapsed_micros,
                            active_model_extent_raw: frame.active_model_extent_raw,
                        },
                        &mut fish.variables,
                    )
                    .map_err(Block::SubF)?;
                    entity.set_rotation_heading_pitch_roll_raw([
                        snapshot.heading_raw as i16,
                        body.pitch_raw,
                        body.roll_raw,
                    ]);
                    snapshot.roll_raw = body.roll_raw as u16;
                    snapshot.position_raw = body.position_raw;
                    snapshot.velocity_raw = body.velocity_raw;
                    commit_snapshot(entity, snapshot);
                    CommonMoverFrameResume::ComponentReturned {
                        phase: CommonMoverDispatchPhase::SubF,
                        snapshot,
                    }
                }
                CommonMoverFramePoll::Action(CommonMoverFrameAction::CommitSubB {
                    velocity_raw,
                }) => {
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
        Err(Block::Runtime("fish01430 did not terminate"))
    })();
    entity.shared_fish_runtime = Some(fish);
    result
}

fn commit_snapshot(entity: &mut Entity, snapshot: CommonMoverFrameSnapshot) {
    let pitch = entity.rotation_heading_pitch_roll_raw()[1];
    entity.set_rotation_heading_pitch_roll_raw([
        snapshot.heading_raw as i16,
        pitch,
        snapshot.roll_raw as i16,
    ]);
    entity.set_motion_raw(snapshot.position_raw, snapshot.velocity_raw);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::type9_attitude::Type9BodyBasis;

    #[v2k_test_support::retail_test]
    fn distinct_global_clock_changes_steering_without_changing_swimming_delta() {
        let (session, mut manager, _) = super::super::tests::fixture(30);
        let id = manager.iter_all().find(|e| e.entity_type == 22).unwrap().id;
        let metadata = manager.type_runtime_metadata(22).unwrap().clone();
        let entity = manager.entity_mut(id).unwrap();
        entity.set_rotation_heading_pitch_roll_raw([0; 3]);
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
        entity.set_velocity_raw([0, 0, 100]);
        let sub_f = &mut entity.shared_fish_runtime.as_mut().unwrap().sub_f;
        sub_f.forward_acceleration_raw = 20;
        sub_f.phase_raw = 0x1234;
        let position = entity.position_raw();
        let extent = session
            .cache
            .global_model(entity.model_index.unwrap())
            .unwrap()
            .radius;
        let target = WanderNearPrivateState::ordinary_type9([
            position[0].wrapping_add(4096),
            position[1],
            position[2].wrapping_add(4096),
        ]);
        let mut other = manager.fork_for_main_base_abort_transaction();
        for (world, global_elapsed_micros) in [(&mut manager, 0), (&mut other, 40_000)] {
            let mut private = target;
            assert_eq!(
                run(
                    world.entity_mut(id).unwrap(),
                    FishMoverFrame {
                        metadata: &metadata,
                        terrain: session.cache.terrain().unwrap(),
                        dispatch_mode: CommonMoverDispatchMode::Normal,
                        elapsed_micros: 20_000,
                        global_elapsed_micros,
                        active_model_extent_raw: extent,
                    },
                    &mut private,
                    RetailRuntimeValue::Known(None),
                    &mut || panic!("static fish mover consumes no RNG")
                ),
                Ok(true)
            );
        }
        let stopped = manager.entity_mut(id).unwrap();
        let turning = other.entity_mut(id).unwrap();
        let a = stopped.shared_fish_runtime.as_ref().unwrap();
        let b = turning.shared_fish_runtime.as_ref().unwrap();
        assert_eq!(a.sub_d_runtime.yaw_rate_raw, b.sub_d_runtime.yaw_rate_raw);
        assert_ne!(b.sub_d_runtime.yaw_rate_raw, 0);
        assert_eq!(a.sub_d_runtime.last_yaw_step_raw, 0);
        assert_ne!(b.sub_d_runtime.last_yaw_step_raw, 0);
        assert_eq!(stopped.rotation_heading_pitch_roll_raw()[0], 0);
        assert_eq!(
            turning.rotation_heading_pitch_roll_raw()[0],
            0i16.wrapping_sub(b.sub_d_runtime.last_yaw_step_raw)
        );
        assert_eq!(a.sub_f.phase_raw, b.sub_f.phase_raw);
        assert_eq!(a.sub_f.phase_raw, 0x1234 + (20_000 >> 8) * 20);
        assert_eq!(a.sub_f.smoothed_turn_raw, 0);
        assert_eq!(
            b.sub_f.smoothed_turn_raw,
            i32::from(b.sub_d_runtime.last_yaw_step_raw) >> 4
        );
        assert_eq!(stopped.position_raw(), turning.position_raw());
        assert_eq!(
            stopped.physical_body_basis_q31(),
            turning.physical_body_basis_q31(),
            "B/D/F must retain the incoming basis until the outer callback publishes it"
        );
    }
}
