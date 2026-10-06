//! Type30's AC60/C690 classes5/7/26: actual task order, retained components.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_shared_acquiring_runtime_task, prepare_shared_generic_constructor_suffix,
        ActorTaskRuntime, SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    entity_behavior::{initial_behavior_state_policy, select_initial_behavior, BehaviorWeightRule},
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
    shared_retarget_mover::SharedRetargetTaskState,
    trash_furniture::TrashFurnitureTargetHeightPolicy,
};

pub(super) fn select(
    player_nearby: bool,
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, Type30Block> {
    select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::PlayerNearby => i32::from(player_nearby),
            _ => unreachable!("authenticated Type30 rule list"),
        },
        next_random,
    )
    .map_err(|_| Type30Block::Runtime("weighted selector"))?
    .ok_or(Type30Block::Runtime("empty weighted selector"))
}

pub(super) fn emitter_constructor() -> GenericEmitterRuntime {
    // 24E30 zeroes0x3c and copies method/sound. E+12 is not a model binding.
    GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: 20,
        emitter_selector: 0,
        sound_id: 75,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    }
}

pub(super) fn initializer_storage_authenticates(entity: &Entity) -> bool {
    matches!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(_)
    ) && matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) && matches!(&entity.sub_h_external_frame_runtime, RetailRuntimeValue::Known(Some(h)) if h.records().len() == 10)
}

/// ACD0/B6C0/B7C0 initialize tasks only. Reselection never reconstructs the
/// four-word bank, K/L, D cadence, H outputs, E command queue or transient B2.
pub(super) fn publish_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    if !matches!(selection.program.class_id, 5 | 7 | 26)
        || !initializer_storage_authenticates(entity)
    {
        return false;
    }
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if selection.program.class_id == 7 {
        let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
            unreachable!()
        };
        axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    if selection.program.class_id == 26 {
        let succeeded = crate::trash_furniture::publish_trash_furniture(
            entity,
            metadata,
            -1,
            TrashFurnitureTargetHeightPolicy::InitialActorHeightApproximation,
            next_random,
        )
        .is_ok();
        if !succeeded {
            entity.publish_behavior_initializer_failure_fallback(context);
        }
        return succeeded;
    }
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(a)) = sub_a_propulsion_runtime else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(h)) = sub_h_external_frame_runtime else {
        unreachable!()
    };
    let mut apply = |effect| match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { value } => h.set_enabled(value != 0),
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw, ..
        } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
    };
    let succeeded = if selection.program.class_id == 5 {
        // 40ACD0 clear2 -> clear1 -> 402B10(owner,0,5000), then6070 once.
        actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        match prepare_shared_generic_constructor_suffix(
            PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(position, 5000),
            )),
            metadata,
        ) {
            Ok(prepared) => {
                actor_tasks.replace_prepared(
                    ActorTaskSlot::Primary,
                    prepared.apply_suffix(next_random, &mut apply),
                );
                true
            }
            Err(_) => false,
        }
    } else {
        let RetailRuntimeValue::Known(filter) = selection.program.initializer_argument_raw else {
            unreachable!()
        };
        apply_run_away_task_setup(
            actor_tasks,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                prepare_shared_acquiring_runtime_task(preparation, position, metadata, filter)
                    .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
            },
        )
        .is_ok()
    };
    if !succeeded {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    succeeded
}
