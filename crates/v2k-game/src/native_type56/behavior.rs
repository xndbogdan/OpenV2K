//! Source-selected7/4/10 initializers; no assumed class4 birth branch.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_defecate_virus_runtime_task, prepare_shared_acquiring_runtime_task,
        prepare_shared_generic_constructor_suffix, SharedGenericConstructorEffect,
    },
    defecate_virus_owner::{
        apply_defecate_virus_setup_with_component_suffix, DefecateVirusSetupRequest,
        DefecateVirusSubATopology,
    },
    entity_behavior::initial_behavior_state_policy,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
};

pub(crate) fn publish_selection(
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
    let position = entity.position_raw();
    let succeeded = match selection.program.class_id {
        4 => {
            let RetailRuntimeValue::Known(lifetime) = metadata.terrain_contact_task_lifetime_ms
            else {
                return false;
            };
            let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
                return false;
            };
            let RetailRuntimeValue::Known(Some(h)) = &mut entity.sub_h_external_frame_runtime
            else {
                return false;
            };
            apply_defecate_virus_setup_with_component_suffix(
                &mut entity.actor_tasks,
                DefecateVirusSetupRequest {
                    terrain_task_lifetime_ms: lifetime,
                },
                DefecateVirusSubATopology::Authored(a),
                |preparation, topology| {
                    let task = prepare_defecate_virus_runtime_task(preparation, position)
                        .map_err(|_| ())?;
                    let prepared = prepare_shared_generic_constructor_suffix(task, metadata)
                        .map_err(|_| ())?;
                    let DefecateVirusSubATopology::Authored(a) = topology else {
                        return Err(());
                    };
                    Ok(
                        prepared.apply_suffix(&mut *next_random, |effect| match effect {
                            SharedGenericConstructorEffect::WriteSubHState08 { value } => {
                                h.set_enabled(value != 0)
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
        }
        7 | 10 => {
            let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
                return false;
            };
            // B6C0 restores only+04; prior acquisition's range remains live.
            axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
            let RetailRuntimeValue::Known(filter) = selection.program.initializer_argument_raw
            else {
                return false;
            };
            let RetailRuntimeValue::Known(Some(a)) = &mut entity.sub_a_propulsion_runtime else {
                return false;
            };
            let RetailRuntimeValue::Known(Some(h)) = &mut entity.sub_h_external_frame_runtime
            else {
                return false;
            };
            apply_run_away_task_setup(
                &mut entity.actor_tasks,
                RunAwayTaskSetupRequest::Acquiring,
                |preparation| {
                    prepare_shared_acquiring_runtime_task(preparation, position, metadata, filter)
                        .map(|prepared| {
                            prepared.apply_suffix(&mut *next_random, |effect| match effect {
                                SharedGenericConstructorEffect::WriteSubHState08 { value } => {
                                    h.set_enabled(value != 0)
                                }
                                SharedGenericConstructorEffect::WriteSubADirection {
                                    direction_multiplier,
                                } => a.set_direction_multiplier(direction_multiplier),
                                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                                    target_speed_raw,
                                    ..
                                } => {
                                    a.apply_shared_initializer_target_speed_write(target_speed_raw)
                                }
                            })
                        })
                },
            )
            .is_ok()
        }
        _ => false,
    };
    if !succeeded {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    succeeded
}
