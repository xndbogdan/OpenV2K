//! AC60's class initializers for ground rows whose root is built only from
//! the shared programs: Defecate Virus `B9E0`, Move About Aimlessly `ACD0`,
//! Search/Capture/Run Away `B6C0`, Follow Beacons `B740` and Trash Furniture
//! `B7C0`.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_defecate_virus_runtime_task, prepare_follow_beacons_acquiring_runtime_task,
        prepare_shared_acquiring_runtime_task, prepare_shared_generic_constructor_suffix,
        ActorTaskRuntime, SharedGenericConstructorEffect,
    },
    actor_task_owner::PreparedActorTask,
    defecate_virus_owner::{
        apply_defecate_virus_setup_with_component_suffix, DefecateVirusSetupRequest,
        DefecateVirusSubATopology,
    },
    entity_behavior::initial_behavior_state_policy,
    follow_beacons::apply_follow_beacons_acquiring_task_setup,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
    shared_retarget_mover::SharedRetargetTaskState,
    trash_furniture::TrashFurnitureTargetHeightPolicy,
};

/// Publish one selected class on an A/H-bearing allocation. Each `06070`
/// suffix draws its own words; none rebuilds a component. A failed
/// initializer publishes the descriptor fallback and returns false.
pub(crate) fn publish_shared_root_class(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    authored_axis: CommonAxisDescriptor,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    let class = selection.program.class_id;
    if !matches!(class, 4 | 5 | 7 | 9 | 10 | 26 | 33) {
        return false;
    }
    let RetailRuntimeValue::Known(lifetime) = metadata.terrain_contact_task_lifetime_ms else {
        return false;
    };
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if matches!(class, 7 | 9 | 10) {
        // B6C0 copies only authored axis+04; the live range word survives.
        let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
            return false;
        };
        axis.raw_word_at_0x04 = authored_axis.raw_word_at_0x04;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    if class == 26 {
        // Type26/58's source policy: a failed scan's target Y fails closed.
        let succeeded = crate::trash_furniture::publish_trash_furniture(
            entity,
            metadata,
            -1,
            TrashFurnitureTargetHeightPolicy::SourceUnresolved,
            next_random,
        )
        .is_ok();
        if !succeeded {
            entity.publish_behavior_initializer_failure_fallback(context);
        }
        return succeeded;
    }
    let position_raw = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
        return false;
    };
    let succeeded = if class == 4 {
        apply_defecate_virus_setup_with_component_suffix(
            actor_tasks,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: lifetime,
            },
            DefecateVirusSubATopology::Authored(sub_a),
            |preparation, topology| {
                let task = prepare_defecate_virus_runtime_task(preparation, position_raw)
                    .map_err(|_| ())?;
                let prepared =
                    prepare_shared_generic_constructor_suffix(task, metadata).map_err(|_| ())?;
                let DefecateVirusSubATopology::Authored(a) = topology else {
                    return Err(());
                };
                Ok(
                    prepared.apply_suffix(&mut *next_random, |effect| match effect {
                        SharedGenericConstructorEffect::WriteSubHState08 { value } => {
                            sub_h.set_enabled(value != 0)
                        }
                        SharedGenericConstructorEffect::WriteSubADirection {
                            direction_multiplier,
                        } => a.set_direction_multiplier(direction_multiplier),
                        SharedGenericConstructorEffect::WriteSubATargetSpeed {
                            target_speed_raw,
                            ..
                        } => a.apply_shared_initializer_target_speed_write(target_speed_raw),
                    }),
                )
            },
        )
        .is_ok()
    } else {
        let mut apply = |effect| match effect {
            SharedGenericConstructorEffect::WriteSubHState08 { value } => {
                sub_h.set_enabled(value != 0)
            }
            SharedGenericConstructorEffect::WriteSubADirection {
                direction_multiplier,
            } => sub_a.set_direction_multiplier(direction_multiplier),
            SharedGenericConstructorEffect::WriteSubATargetSpeed {
                target_speed_raw, ..
            } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
        };
        if class == 5 {
            // ACD0: clear2 -> clear1 -> 402B10(owner,0,5000), then 06070 once.
            actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
            actor_tasks.clear_slot(ActorTaskSlot::Secondary);
            match prepare_shared_generic_constructor_suffix(
                PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                    SharedRetargetTaskState::new(position_raw, 5000),
                )),
                metadata,
            ) {
                Ok(prepared) => {
                    actor_tasks.replace_prepared(
                        ActorTaskSlot::Primary,
                        prepared.apply_suffix(&mut *next_random, &mut apply),
                    );
                    true
                }
                Err(_) => false,
            }
        } else if class == 33 {
            apply_follow_beacons_acquiring_task_setup(actor_tasks, |preparation| {
                prepare_follow_beacons_acquiring_runtime_task(preparation, position_raw, metadata)
                    .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
            })
            .is_ok()
        } else {
            // Classes 7/9/10 share B6C0's exact two constructors; only
            // style+44 differs.
            let RetailRuntimeValue::Known(filter) = selection.program.initializer_argument_raw
            else {
                return false;
            };
            apply_run_away_task_setup(
                actor_tasks,
                RunAwayTaskSetupRequest::Acquiring,
                |preparation| {
                    prepare_shared_acquiring_runtime_task(
                        preparation,
                        position_raw,
                        metadata,
                        filter,
                    )
                    .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
                },
            )
            .is_ok()
        }
    };
    if !succeeded {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    succeeded
}
