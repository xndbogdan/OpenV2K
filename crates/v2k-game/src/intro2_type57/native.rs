//! Own Type57 104B0/09A80 allocations, then AC60 -> B6C0 publication.

use super::*;
use crate::{
    actor_task_dispatcher::{prepare_target_acquisition_runtime_task, ActorTaskRuntime},
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    common_mover::type9_attitude::Type9BodyBasis,
    damage::DamageProfile,
    entity_behavior::{
        behavior_program, initial_behavior_state_policy, select_initial_behavior,
        BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        BehaviorWeightRule,
    },
    entity_collision_state::{CommonMoverGklPayloads, EntityTypeRuntimeMetadata, DYING_STATE_BIT},
    search_attack::{
        search_attack_variant_setup, SearchAttackCandidateFilter, SearchAttackRadius,
        SearchAttackTaskRole, SearchAttackVariant,
    },
    search_attack_owner::{
        apply_search_attack_task_setup, SearchAttackTaskPreparation, SearchAttackTaskSetupRequest,
    },
    shared_retarget_mover::SharedRetargetTaskState,
    sub_g_runtime::SubG06070RuntimeState,
};
use v2k_formats::collision::CommonAxisDescriptor;

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type57Error> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2Type57Error::Metadata);
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(5_000)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 10_000, 1000, 0, 0, 200, 0],
                multipliers_q8: [0, 256, 256, 1024, 0, 64, 0],
            })
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: Some(SUB_G),
                sub_k: Some(SUB_K),
                sub_l: Some(SUB_L),
            })
        || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(SUB_D))
        || metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(EMITTER))
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_h_external_frame_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.status_component_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.accepted_hit_presentation_sound_id != RetailRuntimeValue::Known(None)
        || metadata.infected_model_presentation_sound_id != RetailRuntimeValue::Known(None)
        || metadata.generic_hit_sound_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(None)
        || metadata.target_warning_sound_id != RetailRuntimeValue::Known(None)
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || !matches!(metadata.common_world_effects, RetailRuntimeValue::Known(profile)
            if profile.surface_selectors == [0, 0]
                && profile.surface_lifetime_ms == 0
                && profile.low_health_effect_words == [0, 0, 0])
        || initializer.initializer_state_flags_raw != 8
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 11
    {
        return Err(Intro2Type57Error::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_intro2_type57(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type57Publication, Intro2Type57Error> {
    if entity.authored_spawn_index != Some(1) {
        return Err(Intro2Type57Error::Identity);
    }
    if !entity.active
        || entity.entity_type != 57
        || entity.model_slots != [Some(MODEL); 4]
        || entity.model_index != Some(MODEL)
        || entity.position_raw() != [0xB400u16 as i16, 0x0200, 0x0600]
        || entity.rotation_heading_pitch_roll_raw() != [0, 0, 0]
    {
        return Err(Intro2Type57Error::Identity);
    }
    authenticate_metadata(metadata)?;
    if entity.intro2_type57_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type57Error::AlreadyPublished);
    }
    let expected = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(7).ok_or(Intro2Type57Error::Selection)?,
    };
    if entity.initial_behavior != RetailRuntimeValue::Known(Some(expected))
        || entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(AXIS)
        || entity.sub_g_06070_runtime
            != RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()))
        || entity.physical_body_basis_q31
            != RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0))
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(8)
        || entity.collision.health_raw != RetailRuntimeValue::Known(5_000)
        || entity.collision.pre_health_damage_buffer_raw != RetailRuntimeValue::Known(0)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
    {
        return Err(Intro2Type57Error::ComponentStorage);
    }

    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(
        SubG06070RuntimeState::from_1b8c0_constructor(&SUB_G, next_random() as u16),
    ));
    entity.intro2_type57_runtime = Some(Intro2Type57Runtime {
        entity_id: entity.id,
        spawn_index: 1,
        sub_d_runtime: Type9SubDRuntime::from_constructor(),
        sub_d_frame_owner: Type9SubDFrameOwner::pending_constructor_origin(0x01),
        sub_e_runtime: GenericEmitterRuntime {
            joint_bindings: [None; 2],
            projectile_method: 24,
            emitter_selector: 0,
            sound_id: 93,
            direct_mode: 0,
            remaining_time_raw: 0,
            manual_step_raw: 0,
            remaining_bursts_raw: 0,
            cadence_raw: 0,
            basis_adjustment_identity: None,
        },
        sub_k_smoothed_raw: 0,
        sub_k_output_raw: [0; 2],
        sub_l_target_raw: [0; 3],
        sub_l_exact_raw: 0,
        sub_l_output_raw: [0; 2],
    });
    // D4A0 bit20 is clear: retain authored Y and the zero-Euler constructor
    // basis. Do not invent constructor-zero B2; 12DA0's dormant exit owns it.
    let (selection, selector_word) = select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type57Error::Selection)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let initialized = publish_acquiring(entity, selection, context, next_random);
    Ok(Intro2Type57Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

fn select(
    next_random: &mut impl FnMut() -> u32,
) -> Result<(BehaviorSelection, u32), Intro2Type57Error> {
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &INITIAL_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2Type57Error::Selection)?
    .ok_or(Intro2Type57Error::Selection)?;
    Ok((selection, selector_word))
}

pub(super) fn reselect_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type57Publication, Intro2Type57Error> {
    if !intro2_type57_allocation_authenticates(entity) {
        return Err(Intro2Type57Error::Identity);
    }
    authenticate_metadata(metadata)?;
    if entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) != RetailRuntimeValue::Known(0)
    {
        return Err(Intro2Type57Error::AlternateBehavior);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Intro2Type57Error::Selection);
    };
    let program = behavior_program(7).ok_or(Intro2Type57Error::Selection)?;
    if previous.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || previous.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || !matches!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(_)
        )
    {
        return Err(Intro2Type57Error::ComponentStorage);
    }
    let context = previous
        .reselect_named_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(Intro2Type57Error::Selection)?;
    let (selection, selector_word) = select(next_random)?;
    let initialized = publish_acquiring(entity, selection, context, next_random);
    Ok(Intro2Type57Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

fn publish_acquiring(
    entity: &mut Entity,
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
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        unreachable!("native Type57 initializer preflight retains its axis allocation")
    };
    axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_g_06070_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_g)) = sub_g_06070_runtime else {
        unreachable!("native Type57 initializer preflight retains G")
    };
    let result = apply_search_attack_task_setup(
        actor_tasks,
        SearchAttackTaskSetupRequest::Acquiring,
        |preparation| {
            let prepared = prepare(preparation, position, axis)?;
            let word = next_random() as u16;
            sub_g.apply_shared_06070_sub_g_branch(700 + i32::from(word >> 8), 0);
            Ok::<_, Intro2Type57Error>(prepared)
        },
    );
    if result.is_err() {
        entity.publish_behavior_initializer_failure_fallback(context);
        return false;
    }
    true
}

fn prepare(
    preparation: SearchAttackTaskPreparation,
    position: [i16; 3],
    axis: CommonAxisDescriptor,
) -> Result<PreparedActorTask<ActorTaskRuntime>, Intro2Type57Error> {
    match preparation.task.role {
        SearchAttackTaskRole::AcquireTarget => prepare_target_acquisition_runtime_task(
            preparation,
            SearchAttackRadius::from_raw(axis.strict_axis_limit_raw),
            SearchAttackCandidateFilter::from_raw(axis.raw_word_at_0x04),
            0,
        )
        .map_err(|_| Intro2Type57Error::ComponentStorage),
        SearchAttackTaskRole::Wander
            if preparation.request == SearchAttackTaskSetupRequest::Acquiring
                && preparation.phase_index == 1
                && preparation.task
                    == search_attack_variant_setup(SearchAttackVariant::Acquiring)
                        .ordered_phases[1]
                        .install =>
        {
            Ok(PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                SharedRetargetTaskState::new(position, 500),
            )))
        }
        _ => Err(Intro2Type57Error::ComponentStorage),
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
pub(super) mod tests;
