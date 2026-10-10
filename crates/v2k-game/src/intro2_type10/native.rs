//! Own Type10 104B0/09A80 allocations, then AC60 -> B6C0 publication.

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

pub(crate) fn authenticate_metadata(
    profile: Type10Profile,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type10Error> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2Type10Error::Metadata);
    };
    if metadata.model_slots != [profile.model() as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(32_000)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: profile.damage_thresholds_raw(),
                multipliers_q8: [0, 256, 256, 0, 750, 512, 0],
            })
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: Some(*profile.sub_g()),
                sub_k: Some(SUB_K),
                sub_l: Some(SUB_L),
            })
        || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(profile.sub_d()))
        || metadata.projectile_emitter_descriptor
            != RetailRuntimeValue::Known(Some(profile.emitter()))
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
        || initializer.common_axis_descriptor != profile.axis()
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 11
    {
        return Err(Intro2Type10Error::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_intro2_type10(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type10Publication, Intro2Type10Error> {
    let (spawn_index, position, seed) = match entity.authored_spawn_index {
        Some(55) => (55, [0x7300, 0x0700, 0x0900], 0x22),
        Some(56) => (56, [0x5a00, 0x0600, 0x0800], 0x23),
        _ => return Err(Intro2Type10Error::Identity),
    };
    if !entity.active
        || entity.entity_type != 10
        || entity.model_slots != [Some(MODEL); 4]
        || entity.model_index != Some(MODEL)
        || entity.position_raw() != position
        || entity.rotation_heading_pitch_roll_raw() != [0xc000u16 as i16, 0, 0]
    {
        return Err(Intro2Type10Error::Identity);
    }
    let profile = Type10Profile::Type10;
    authenticate_metadata(profile, metadata)?;
    authenticate_birth_storage(profile, entity)?;
    publish_birth(
        profile,
        entity,
        spawn_index,
        Type9SubDRuntime::from_constructor(),
        // Source203D0 increments once for each earlier successful D allocation:
        // the normal Intro2 authored prefix supplies own seeds22/23. Flags0
        // never read the allocator's unwritten origin, so no capture is borrowed.
        Type9SubDFrameOwner::pending_constructor_origin(seed),
        None,
        next_random,
    )
}

/// Ordinary 104B0 publication of one authored Type10-family row.
pub(crate) struct Type10AuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a v2k_formats::levels::EntitySpawn,
    pub constructor_surface_bits: u32,
    pub sub_d: crate::common_mover::sub_d::NativeSubDConstruction,
}

/// Ordinary births run the same 09A80/AC60/B6C0 transaction as Intro2's
/// dragons; the process owns the Sub-D seed and the manager the receipt.
pub(crate) fn publish_authored_type10_family(
    request: Type10AuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type10Publication, Intro2Type10Error> {
    let Type10AuthoredConstruction {
        entity,
        allocation,
        metadata,
        spawn,
        constructor_surface_bits,
        sub_d,
    } = request;
    let profile =
        Type10Profile::from_entity_type(spawn.entity_type).ok_or(Intro2Type10Error::Identity)?;
    let model = profile.model();
    if !entity.active
        || entity.entity_type != profile.entity_type()
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(model); 4]
        || entity.model_index != Some(model)
        || spawn
            .model_overrides
            .iter()
            .any(|&slot| slot != 0 && slot as usize != model)
        || entity.position_raw() != spawn.position_raw()
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Intro2Type10Error::Identity);
    }
    authenticate_metadata(profile, metadata)?;
    authenticate_birth_storage(profile, entity)?;
    // Flags-zero Sub-D never queries its cache; the process seed is retained.
    if sub_d.descriptor != profile.sub_d()
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
        || constructor_surface_bits & !crate::entity_collision_state::SURFACE_STATE_MASK != 0
    {
        return Err(Intro2Type10Error::ComponentStorage);
    }
    // 104B0 classifies the authored surface at this tick; D4A0 has run.
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
        profile,
        entity,
        spawn.index,
        sub_d.runtime,
        sub_d.frame_owner,
        Some(allocation),
        next_random,
    )
}

fn authenticate_birth_storage(
    profile: Type10Profile,
    entity: &Entity,
) -> Result<(), Intro2Type10Error> {
    if entity.intro2_type10_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type10Error::AlreadyPublished);
    }
    let expected = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(7).ok_or(Intro2Type10Error::Selection)?,
    };
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    if entity.initial_behavior != RetailRuntimeValue::Known(Some(expected))
        || entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(profile.axis())
        || entity.sub_g_06070_runtime
            != RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()))
        || entity.physical_body_basis_q31
            != RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(8)
        || entity.collision.health_raw != RetailRuntimeValue::Known(32_000)
        || entity.collision.pre_health_damage_buffer_raw != RetailRuntimeValue::Known(0)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
    {
        return Err(Intro2Type10Error::ComponentStorage);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn publish_birth(
    profile: Type10Profile,
    entity: &mut Entity,
    spawn_index: usize,
    sub_d_runtime: Type9SubDRuntime,
    sub_d_frame_owner: Type9SubDFrameOwner,
    ordinary_allocation: Option<crate::main_base_abort::MainBaseAbortActorLease>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type10Publication, Intro2Type10Error> {
    // 09A80: zero animation bank, G1B8C0 (one word), D203D0, K24450,
    // L1BB80 (both zero their allocations), axis235A0, then E24E30 (zero
    // allocation; no RNG).
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(
        SubG06070RuntimeState::from_1b8c0_constructor(profile.sub_g(), next_random() as u16),
    ));
    entity.intro2_type10_runtime = Some(Intro2Type10Runtime {
        entity_id: entity.id,
        spawn_index,
        profile,
        ordinary_allocation,
        sub_d_runtime,
        sub_d_frame_owner,
        sub_e_runtime: GenericEmitterRuntime {
            joint_bindings: [None; 2],
            projectile_method: 10,
            emitter_selector: 0,
            sound_id: 81,
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
    // D4A0 bit20 is absent: retain authored Y and incoming full Euler basis.
    // Explicit native policy for the unwritten transient B2 heap residue,
    // once at birth. Reselection preserves every later contribution.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let (selection, selector_word) = select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type10Error::Selection)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let initialized = publish_acquiring(profile, entity, selection, context, next_random);
    Ok(Intro2Type10Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

fn select(
    next_random: &mut impl FnMut() -> u32,
) -> Result<(BehaviorSelection, u32), Intro2Type10Error> {
    let mut selector_word = 0;
    // 425680 consumes a word even for the singleton Always list.
    let selection = select_initial_behavior(
        &INITIAL_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2Type10Error::Selection)?
    .ok_or(Intro2Type10Error::Selection)?;
    Ok((selection, selector_word))
}

/// Living C690/AC60 reentry; caller owns task-result suppression and completed
/// task custody. The allocated behavior context and DEGKL state survive.
pub(super) fn reselect_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type10Publication, Intro2Type10Error> {
    let profile = type10_profile(entity).ok_or(Intro2Type10Error::Identity)?;
    authenticate_metadata(profile, metadata)?;
    if entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) != RetailRuntimeValue::Known(0)
    {
        return Err(Intro2Type10Error::AlternateBehavior);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Intro2Type10Error::Selection);
    };
    let program = behavior_program(7).ok_or(Intro2Type10Error::Selection)?;
    if previous.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || previous.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || !matches!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(_)
        )
    {
        return Err(Intro2Type10Error::ComponentStorage);
    }
    // This canonical program/style exists before selector consumption.
    let context = previous
        .reselect_named_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(Intro2Type10Error::Selection)?;
    let (selection, selector_word) = select(next_random)?;
    let initialized = publish_acquiring(profile, entity, selection, context, next_random);
    Ok(Intro2Type10Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

fn publish_acquiring(
    profile: Type10Profile,
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
        unreachable!("native Type10 initializer preflight retains its axis allocation")
    };
    // 40B707 copies only live axis+4. A changed strict radius survives C690.
    axis.raw_word_at_0x04 = profile.axis().raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    let position = entity.position_raw();
    let Entity {
        actor_tasks,
        sub_g_06070_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_g)) = sub_g_06070_runtime else {
        unreachable!("native Type10 initializer preflight retains G")
    };
    let result = apply_search_attack_task_setup(
        actor_tasks,
        SearchAttackTaskSetupRequest::Acquiring,
        |preparation| {
            let prepared = prepare(preparation, position, axis)?;
            // Each successful6030 enters06070: G1B970(0), G1B940(0)
            // draws one word, G1B980(0), then24380(0). E/K/L untouched.
            let word = next_random() as u16;
            sub_g.apply_shared_06070_sub_g_branch(
                profile.sub_g_randomized_target_base() + i32::from(word >> 8),
                0,
            );
            Ok::<_, Intro2Type10Error>(prepared)
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
) -> Result<PreparedActorTask<ActorTaskRuntime>, Intro2Type10Error> {
    match preparation.task.role {
        SearchAttackTaskRole::AcquireTarget => prepare_target_acquisition_runtime_task(
            preparation,
            SearchAttackRadius::from_raw(axis.strict_axis_limit_raw),
            SearchAttackCandidateFilter::from_raw(axis.raw_word_at_0x04),
            0,
        )
        .map_err(|_| Intro2Type10Error::ComponentStorage),
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
        _ => Err(Intro2Type10Error::ComponentStorage),
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
pub(super) mod tests;
