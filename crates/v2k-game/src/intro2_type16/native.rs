//! Type16's own 104B0/09A80 construction and AC60 initial task publication.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_defecate_virus_runtime_task, prepare_shared_acquiring_runtime_task,
        prepare_shared_generic_constructor_suffix, ActorTaskRuntime,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    common_mover::{
        sub_d::{intro2_type16_first_query_owner_for_birth, INTRO2_TYPE16_SUB_D},
        SubAPropulsionRuntime,
    },
    defecate_virus_owner::{
        apply_defecate_virus_setup, DefecateVirusSetupRequest, DefecateVirusSubATopology,
    },
    entity_behavior::initial_behavior_state_policy,
    generic_projectile_emitter::GenericEmitterRuntime,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
    shared_retarget_mover::SharedRetargetTaskState,
    sub_h_external_frame::SubHRuntimeState,
};
use v2k_formats::collision::{
    ProjectileEmitterDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    SubCLiftDescriptor,
};

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type16Error> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2Type16Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Intro2Type16Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_j)) = &metadata.sub_j_attachment_descriptor else {
        return Err(Intro2Type16Error::Metadata);
    };
    let refs = [
        [0, 56, 64],
        [1, 57, 65],
        [52, 58, 66],
        [53, 59, 67],
        [54, 60, 68],
        [55, 61, 69],
        [150, 62, 70],
        [151, 63, 71],
    ];
    let dependencies = [
        [1, 2, 3, 3],
        [0, 2, 3, 3],
        [0, 1, 3, 3],
        [0, 1, 2, 2],
        [5, 6, 7, 7],
        [4, 6, 7, 7],
        [4, 5, 7, 7],
        [4, 5, 6, 6],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(12_000)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 200,
            }))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1000,
            }))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 75,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0, 0],
            }))
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(INTRO2_TYPE16_SUB_D))
        || metadata.projectile_emitter_descriptor
            != RetailRuntimeValue::Known(Some(ProjectileEmitterDescriptor {
                projectile_method: 20,
                random_interval_us: 300_000,
                spread_raw: 100,
                aim_threshold_raw: 16_000,
                speed_override_raw: 0,
                target_axis_tolerance_raw: 2560,
                sound_id: 81,
                raw_word_at_0x12: 158,
                alternate_emitter_raw: 0,
                stochastic_gate_mode: 0,
                auxiliary_command: 0,
                variable_bindings: [0; 4],
            }))
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || initializer.initializer_state_flags_raw != 0x439
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || sub_h.completion_sound_id.is_some()
        || sub_h.records.len() != refs.len()
        || sub_h.records.iter().enumerate().any(|(i, record)| {
            record.resolver_flags_raw != 0x40000000
                || record.phase_rate_raw != 0x0c000000
                || record.vertex_refs != refs[i]
                || record.axis_mode_raw != 0
                || record.dependencies != dependencies[i]
        })
        || sub_j.slots.len() != 1
        || sub_j.slots[0].policy_word_raw != 1
        || sub_j.slots[0].local_offset_raw != [0, 10, 110]
    {
        return Err(Intro2Type16Error::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_intro2_type16(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type16Publication, Intro2Type16Error> {
    let Some(spawn) = entity.authored_spawn_index else {
        return Err(Intro2Type16Error::Identity);
    };
    let (xz, sub_d_seed) = match spawn {
        5 => ([-27648, -29184], 0x05),
        42 => ([-17664, 30976], 0x1b),
        _ => return Err(Intro2Type16Error::Identity),
    };
    if !entity.active
        || entity.entity_type != 16
        || entity.model_slots != [Some(MODEL); 4]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
    {
        return Err(Intro2Type16Error::Identity);
    }
    authenticate_metadata(metadata)?;
    authenticate_birth_storage(entity)?;
    if preceding.iter().any(|candidate| {
        candidate
            .authored_spawn_index
            .is_none_or(|index| index >= spawn)
    }) {
        return Err(Intro2Type16Error::Prefix);
    }
    // Each receipt joins this own allocation to its first41F7A8 full reset
    // in V200001. No Type17 or other cohort's origin is substituted.
    let sub_d_owner = intro2_type16_first_query_owner_for_birth(spawn, sub_d_seed)
        .expect("authenticated Type16 spawn/seed owns its first-query receipt");
    publish_birth(
        entity,
        metadata,
        preceding,
        terrain,
        Type9SubDRuntime::from_constructor(),
        sub_d_owner,
        None,
        next_random,
    )
}

/// Ordinary 104B0 publication of one authored Type16.
pub(crate) struct Type16AuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a v2k_formats::levels::EntitySpawn,
    /// The already-linked live list, read by AC60's nearby predicates.
    pub preceding: &'a [Entity],
    pub terrain: &'a TerrainGrid,
    pub constructor_surface_bits: u32,
    pub sub_d: crate::common_mover::sub_d::NativeSubDConstruction,
}

/// Ordinary worlds build the same birth as Intro2 spawns5/42, except that
/// the process owns the Sub-D seed and the manager the allocation identity.
pub(crate) fn publish_authored_type16(
    request: Type16AuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type16Publication, Intro2Type16Error> {
    let Type16AuthoredConstruction {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        terrain,
        constructor_surface_bits,
        sub_d,
    } = request;
    if !entity.active
        || entity.entity_type != 16
        || spawn.entity_type != 16
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(MODEL); 4]
        || spawn
            .model_overrides
            .iter()
            .any(|&model| model != 0 && model as usize != MODEL)
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Intro2Type16Error::Identity);
    }
    authenticate_metadata(metadata)?;
    authenticate_birth_storage(entity)?;
    if sub_d.descriptor != INTRO2_TYPE16_SUB_D
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
        || constructor_surface_bits & !crate::entity_collision_state::SURFACE_STATE_MASK != 0
    {
        return Err(Intro2Type16Error::ComponentStorage);
    }
    // 104B0 classifies the surface at this tick; D4A0 has already run.
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_initializer::CONSTRUCTOR_SURFACE_STATE_MASK,
        constructor_surface_bits,
    );
    // Common type-vtable+30 and 104B0's initial +44 modifier are null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    publish_birth(
        entity,
        metadata,
        preceding,
        terrain,
        sub_d.runtime,
        sub_d.frame_owner,
        Some(allocation),
        next_random,
    )
}

fn authenticate_birth_storage(entity: &Entity) -> Result<(), Intro2Type16Error> {
    if entity.intro2_type16_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type16Error::AlreadyPublished);
    }
    if entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || !initializer_storage_authenticates(entity)
        || !matches!(entity.sub_a_propulsion_runtime, RetailRuntimeValue::Known(Some(a)) if a.drive_scale_percent() == 100)
        || !matches!(&entity.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(j)) if j.is_empty() && j.authored_slot_count() == 1)
        || entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(AXIS)
    {
        return Err(Intro2Type16Error::ComponentStorage);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    sub_d_runtime: Type9SubDRuntime,
    sub_d_owner: Type9SubDFrameOwner,
    ordinary_allocation: Option<crate::main_base_abort::MainBaseAbortActorLease>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type16Publication, Intro2Type16Error> {
    let spawn = entity
        .authored_spawn_index
        .ok_or(Intro2Type16Error::Identity)?;
    let mut selector_owner = behavior::candidate(entity);
    // D4A0's type-default bit0x20 terrain snap precedes the AC60 range reads.
    let RetailRuntimeValue::Known(position) = crate::entity_initializer::constructor_position_raw(
        metadata,
        selector_owner.position_raw,
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type16Error::Metadata);
    };
    selector_owner.position_raw = position;
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(behavior::candidate)
        .collect();
    let evaluators = behavior::evaluate(selector_owner, &prefix, 0, 0)?;
    let sub_h = SubHRuntimeState::new(8).map_err(|_| Intro2Type16Error::ComponentStorage)?;
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    // 09A80 constructs H, D, then A20450. Only A consumes a word. E24E30
    // subsequently zeroes all0x3c bytes and copies method/sound; its four
    // bindings (+18..1B) are null. Descriptor+12=158 is not a binding.
    let sub_a = SubAPropulsionRuntime::from_20450_constructor(descriptor, next_random() as u16);
    let sub_e_runtime = GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: 20,
        emitter_selector: 0,
        sound_id: 81,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    };
    let (selection, selector_word) = evaluators.select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type16Error::Selection)?;
    entity.set_position_raw(selector_owner.position_raw);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.intro2_type16_runtime = Some(Intro2Type16Runtime {
        entity_id: entity.id,
        spawn_index: spawn,
        sub_d_runtime,
        sub_d_owner,
        sub_e_runtime,
        ordinary_allocation,
    });
    // Native deterministic policy for the allocator's unwritten transient B2;
    // later animation/mass contributions must never be reset by reselection.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_initial_style(entity, metadata, selection, context, next_random);
    Ok(Intro2Type16Publication {
        selection,
        selector_word,
        people_nearby: evaluators.people_nearby,
        player_nearby: evaluators.player_nearby,
        initializer_fallback: !succeeded,
    })
}

pub(super) fn initializer_storage_authenticates(entity: &Entity) -> bool {
    matches!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(_)
    ) && matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) && matches!(&entity.sub_h_external_frame_runtime, RetailRuntimeValue::Known(Some(h)) if h.records().len() == 8)
}

/// ABB0/C6B0 initializer shared by birth and C690, with each class's own
/// task order. It does not reconstruct A/H/D/E or reset transient B2.
pub(super) fn publish_initial_style(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    debug_assert!(matches!(selection.program.class_id, 4 | 5 | 7 | 9));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if matches!(selection.program.class_id, 7 | 9) {
        // Only B6C0 copies authored axis+04. ACD0/B9E0 preserve both words.
        let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
            unreachable!()
        };
        axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
    let position = entity.position_raw();
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
    let succeeded = if selection.program.class_id == 4 {
        // B9E0: clear1, publish mode5 slot2, prepare shared mover slot0,
        // reset A target to literal1, publish0. Neither phase calls06070.
        let RetailRuntimeValue::Known(lifetime) = metadata.terrain_contact_task_lifetime_ms else {
            unreachable!()
        };
        apply_defecate_virus_setup(
            actor_tasks,
            DefecateVirusSetupRequest {
                terrain_task_lifetime_ms: lifetime,
            },
            DefecateVirusSubATopology::Authored(sub_a),
            |preparation| prepare_defecate_virus_runtime_task(preparation, position),
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
        if selection.program.class_id == 5 {
            // ACD0: clear2, clear1, then2B10(5000). Its06070 suffix runs once.
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
                        prepared.apply_suffix(&mut *next_random, &mut apply),
                    );
                    true
                }
                Err(_) => false,
            }
        } else {
            let RetailRuntimeValue::Known(filter) = selection.program.initializer_argument_raw
            else {
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
        }
    };
    if !succeeded {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    succeeded
}

#[cfg(test)]
#[path = "native_tests.rs"]
pub(super) mod tests;
