//! Type40 B6C0 exact class7/9 acquisition, each publication owns406070.
use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_shared_acquiring_runtime_task, SharedGenericConstructorEffect,
    },
    entity_behavior::initial_behavior_state_policy,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
};
pub(crate) fn publish_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    {
        // B6C0 copies only authored axis+04. Keep the current range after a
        // carrying task or a previous selector changed it; B740 copies neither.
        let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
            unreachable!("authenticated common axis storage")
        };
        axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    let position_raw = entity.position_raw();

    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
        unreachable!()
    };
    let mut apply = |effect| match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { value } => sub_h.set_enabled(value != 0),
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw, ..
        } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
    };
    let RetailRuntimeValue::Known(filter) = selection.program.initializer_argument_raw else {
        return false;
    };
    let succeeded = apply_run_away_task_setup(
        actor_tasks,
        RunAwayTaskSetupRequest::Acquiring,
        |preparation| {
            prepare_shared_acquiring_runtime_task(preparation, position_raw, metadata, filter)
                .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
        },
    )
    .is_ok();
    if !succeeded {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    succeeded
}
