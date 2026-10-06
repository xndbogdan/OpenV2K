//! Type94's own 104B0/09A80 construction and AC60 initial task publication.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_acquiring_runtime_task, prepare_shared_acquiring_runtime_task,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::{intro2_type94_first_query_owner_for_birth, INTRO2_TYPE94_SUB_D},
        SubAPropulsionRuntime,
    },
    entity_behavior::initial_behavior_state_policy,
    follow_beacons::apply_follow_beacons_acquiring_task_setup,
    generic_projectile_emitter::GenericEmitterRuntime,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
    sub_h_external_frame::SubHRuntimeState,
};
use v2k_formats::collision::{
    ProjectileEmitterDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    SubCLiftDescriptor,
};

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type94Error> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2Type94Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Intro2Type94Error::Metadata);
    };
    let refs = [
        [114, 58, 64],
        [115, 59, 65],
        [112, 56, 62],
        [113, 57, 63],
        [66, 120, 118],
        [67, 121, 119],
    ];
    let dependencies = [
        [1, 2, 3, 4],
        [0, 2, 3, 5],
        [0, 1, 4, 5],
        [0, 1, 4, 5],
        [0, 2, 3, 5],
        [1, 2, 3, 4],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(10_000)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 400,
            }))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1000,
            }))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 50,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 1,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0, 0],
            }))
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(INTRO2_TYPE94_SUB_D))
        || metadata.projectile_emitter_descriptor
            != RetailRuntimeValue::Known(Some(ProjectileEmitterDescriptor {
                projectile_method: 20,
                random_interval_us: 700_000,
                spread_raw: 256,
                aim_threshold_raw: 12_000,
                speed_override_raw: 0,
                target_axis_tolerance_raw: 2304,
                sound_id: 69,
                raw_word_at_0x12: 102,
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
            record.resolver_flags_raw != 0x20000000
                || record.phase_rate_raw != 0x20000000
                || record.vertex_refs != refs[i]
                || record.axis_mode_raw != 0
                || record.dependencies != dependencies[i]
        })
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2Type94Error::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_intro2_type94(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type94Publication, Intro2Type94Error> {
    let Some(spawn) = entity.authored_spawn_index else {
        return Err(Intro2Type94Error::Identity);
    };
    let xz = match spawn {
        43 => [-16384, -32256],
        _ => return Err(Intro2Type94Error::Identity),
    };
    if !entity.active
        || entity.entity_type != 94
        || entity.model_slots != [Some(MODEL); 4]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
    {
        return Err(Intro2Type94Error::Identity);
    }
    authenticate_metadata(metadata)?;
    if entity.intro2_type94_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type94Error::AlreadyPublished);
    }
    if entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || !initializer_storage_authenticates(entity)
        || !matches!(entity.sub_a_propulsion_runtime, RetailRuntimeValue::Known(Some(a)) if a.drive_scale_percent() == 100)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
        || entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(AXIS)
    {
        return Err(Intro2Type94Error::ComponentStorage);
    }
    if preceding.iter().any(|candidate| {
        candidate
            .authored_spawn_index
            .is_none_or(|index| index >= spawn)
    }) {
        return Err(Intro2Type94Error::Prefix);
    }
    // Own 1FAB0 -> first 41FCB0 receipt: spawn43/seed1C, zero origin before
    // the full reset at C1FF/82C8. Type58's equal descriptor grants no custody.
    let sub_d_owner =
        intro2_type94_first_query_owner_for_birth(spawn, 0x1c).ok_or(Intro2Type94Error::Prefix)?;
    let mut selector_owner = behavior::candidate(entity);
    // D4A0's type-default bit0x20 terrain snap precedes the AC60 range reads.
    let RetailRuntimeValue::Known(position) = crate::entity_initializer::constructor_position_raw(
        metadata,
        selector_owner.position_raw,
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type94Error::Metadata);
    };
    selector_owner.position_raw = position;
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(behavior::candidate)
        .collect();
    let evaluators = behavior::evaluate(selector_owner, &prefix, AXIS)?;
    let mut sub_h = SubHRuntimeState::new(6).map_err(|_| Intro2Type94Error::ComponentStorage)?;
    sub_h.set_surface_policy(crate::sub_h_external_frame::SubHSurfacePolicy::TerrainAndWater);
    let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    // 09A80 constructs H, D, then A20450. Only A consumes a word. E24E30
    // subsequently zeroes all0x3c bytes and copies method/sound; its four
    // bindings (+18..1B) are null. Descriptor+12=102 is not a binding.
    let sub_a = SubAPropulsionRuntime::from_20450_constructor(descriptor, next_random() as u16);
    let sub_e_runtime = GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: 20,
        emitter_selector: 0,
        sound_id: 69,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    };
    let (selection, selector_word) = evaluators.select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type94Error::Selection)?;
    entity.set_position_raw(selector_owner.position_raw);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.intro2_type94_runtime = Some(Intro2Type94Runtime {
        entity_id: entity.id,
        spawn_index: spawn,
        sub_d_runtime: Type9SubDRuntime::from_constructor(),
        // The first-query helper must authenticate this allocation's own
        // recorded constructor and origin read; other cohorts cannot supply it.
        sub_d_owner,
        sub_e_runtime,
    });
    // Native deterministic policy for the allocator's unwritten transient B2;
    // later animation/mass contributions must never be reset by reselection.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_initial_style(entity, metadata, selection, context, next_random);
    Ok(Intro2Type94Publication {
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
    ) && matches!(&entity.sub_h_external_frame_runtime, RetailRuntimeValue::Known(Some(h)) if h.records().len() == 6)
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
    debug_assert!(matches!(selection.program.class_id, 7 | 9 | 33));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if selection.program.class_id == 7 {
        // B6C0 copies authored axis+04; the live range word remains unchanged.
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
    let mut apply = |effect| match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { value } => sub_h.set_enabled(value != 0),
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw, ..
        } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
    };
    let succeeded = if selection.program.class_id == 33 {
        apply_follow_beacons_acquiring_task_setup(actor_tasks, |preparation| {
            prepare_follow_beacons_acquiring_runtime_task(preparation, position, metadata)
                .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
        })
        .is_ok()
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

#[cfg(test)]
#[path = "native_tests.rs"]
pub(super) mod tests;
