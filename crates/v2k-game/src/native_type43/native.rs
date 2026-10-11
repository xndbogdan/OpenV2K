//! Own Type43 104B0/09A80 allocations, then 25680 -> AC60 -> B6C0 publication.

use super::*;
use crate::{
    actor_task_dispatcher::{prepare_target_acquisition_runtime_task, ActorTaskRuntime},
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{
        behavior_program, initial_behavior_state_policy, select_initial_behavior,
        BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverGklPayloads, CommonWorldEffectProfile, EntityTypeRuntimeMetadata,
        RetailRuntimeValue, DYING_STATE_BIT, SURFACE_STATE_MASK,
    },
    search_attack::{
        search_attack_variant_setup, SearchAttackCandidateFilter, SearchAttackRadius,
        SearchAttackTaskRole, SearchAttackVariant,
    },
    search_attack_owner::{
        apply_search_attack_task_setup, SearchAttackTaskPreparation, SearchAttackTaskSetupRequest,
    },
    shared_retarget_mover::SharedRetargetTaskState,
};
use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type43Error> {
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type43Error::Metadata);
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.model_variable_count_raw != Known(0)
        || metadata.mass_raw != MASS
        || metadata.capability_flags != CAPABILITY
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile != Some(DAMAGE)
        || metadata.common_mover_topology != Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: None,
            })
        || metadata.sub_a_propulsion_descriptor != Known(None)
        || metadata.sub_b_lateral_descriptor != Known(None)
        || metadata.sub_c_lift_descriptor != Known(None)
        || metadata.sub_d_steering_descriptor != Known(None)
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.projectile_emitter_descriptor != Known(Some(EMITTER))
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.sub_h_external_frame_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.accepted_hit_presentation_sound_id != Known(None)
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.cured_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.death_sound_id != Known(None)
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(None)
        || metadata.search_attack_aim_sound_id != Known(None)
        || metadata.search_attack_aim_sound_period_raw != Known(0)
        || metadata.terrain_contact_task_lifetime_ms != Known(0)
        || metadata.common_world_effects != Known(CommonWorldEffectProfile::default())
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != BEHAVIOR_RULE_REF
        || initializer.alternate_behavior_class_ref != ALTERNATE_BEHAVIOR_CLASS
    {
        return Err(Type43Error::Metadata);
    }
    Ok(())
}

/// Ordinary 104B0 publication of one authored Type43 row.
pub(crate) struct Type43AuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub terrain: &'a TerrainGrid,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_authored_type43(
    request: Type43AuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type43Publication, Type43Error> {
    let Type43AuthoredConstruction {
        entity,
        allocation,
        metadata,
        spawn,
        terrain,
        constructor_surface_bits,
    } = request;
    let [x, _, z] = spawn.position_raw();
    if !entity.active
        || entity.entity_type != ENTITY_TYPE
        || spawn.entity_type != ENTITY_TYPE
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != [Some(MODEL); 4]
        || entity.model_index != Some(MODEL)
        || spawn
            .model_overrides
            .iter()
            .any(|&slot| slot != 0 && slot as usize != MODEL)
        // D4A0's +C0 bit20 grounds at the authored X/Z; bit40 is absent.
        || entity.position_raw() != [x, terrain.bilinear_height_raw(x, z), z]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
        || entity.collision.state_flags_at_0x08.masked(0x0100_0000)
            != RetailRuntimeValue::Known(if spawn.param != 0 { 0x0100_0000 } else { 0 })
    {
        return Err(Type43Error::Identity);
    }
    authenticate_metadata(metadata)?;
    authenticate_birth_storage(entity, spawn.initial_damage_buffer_raw)?;
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Type43Error::ComponentStorage);
    }
    // 104B0 classifies the authored surface at this tick; D4A0 has run.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    // Common type-vtable+30 and 104B0's initial +44 modifier are null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    // 09A80 zeroes the model bank, then allocates E24E30 alone: it copies the
    // row's method and sound and draws no random word. Axis235A0 follows.
    entity.native_type43_runtime = Some(Type43Runtime {
        entity_id: entity.id,
        spawn_index: spawn.index,
        allocation,
        sub_e_runtime: GenericEmitterRuntime {
            joint_bindings: [None; 2],
            projectile_method: EMITTER.projectile_method,
            emitter_selector: 0,
            sound_id: u32::from(EMITTER.sound_id),
            direct_mode: 0,
            remaining_time_raw: 0,
            manual_step_raw: 0,
            remaining_bursts_raw: 0,
            cadence_raw: 0,
            basis_adjustment_identity: None,
        },
    });
    // Explicit native policy for 104B0's unwritten transient B2 heap residue,
    // once at birth. Reselection preserves every later contribution.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let (selection, selector_word) = select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type43Error::Selection)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let initialized = publish_acquiring(entity, selection, context);
    Ok(Type43Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

fn authenticate_birth_storage(entity: &Entity, damage_buffer_raw: i32) -> Result<(), Type43Error> {
    use RetailRuntimeValue::Known;
    if entity.native_type43_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Type43Error::AlreadyPublished);
    }
    let expected = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(7).ok_or(Type43Error::Selection)?,
    };
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    if entity.initial_behavior != Known(Some(expected))
        || entity.actor_common_axis_descriptor != Known(AXIS)
        || entity.physical_body_basis_q31
            != Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
        || entity.collision.default_state_flags_at_0xc8 != Known(DEFAULT_FLAGS)
        || entity.collision.health_raw != Known(HEALTH)
        || entity.collision.pre_health_damage_buffer_raw != Known(damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != Known(0)
        || entity.capability_flags != CAPABILITY
        || entity.mass_raw != MASS
        // D3C0's +C0 drops terrain/water 0x10000 and fixes the body with
        // 0x08000000; class7's reversed +38 (0x21080) then frees it again,
        // enabling motion 0x40000 and pair collision 0x8000.
        || entity.collision.state_flags_at_0x08.masked(0x0805_8000) != Known(0x0004_8000)
    {
        return Err(Type43Error::ComponentStorage);
    }
    Ok(())
}

fn select(next_random: &mut impl FnMut() -> u32) -> Result<(BehaviorSelection, u32), Type43Error> {
    let mut selector_word = 0;
    // 25680 consumes a word even for the singleton Always list.
    let selection = select_initial_behavior(
        &INITIAL_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type43Error::Selection)?
    .ok_or(Type43Error::Selection)?;
    Ok((selection, selector_word))
}

/// Living C690/AC60 reentry; the caller owns task-result suppression and
/// completed task custody. The allocated context and E state survive.
pub(super) fn reselect_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type43Publication, Type43Error> {
    if !allocation_authenticates(entity) {
        return Err(Type43Error::Identity);
    }
    authenticate_metadata(metadata)?;
    if entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT) != RetailRuntimeValue::Known(0)
    {
        return Err(Type43Error::AlternateBehavior);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Type43Error::Selection);
    };
    let program = behavior_program(7).ok_or(Type43Error::Selection)?;
    if previous.descriptor() != BehaviorDescriptorIdentity::Named(program)
        || previous.choice_list_source()
            != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || !matches!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(_)
        )
    {
        return Err(Type43Error::ComponentStorage);
    }
    // This canonical program/style exists before selector consumption.
    let context = previous
        .reselect_named_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(Type43Error::Selection)?;
    let (selection, selector_word) = select(next_random)?;
    let initialized = publish_acquiring(entity, selection, context);
    Ok(Type43Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

/// B6C0: copy the type's axis+04, clear Tertiary, then install the slot-1
/// acquisition and the 500-ms slot-0 02BA0 retarget. Neither 06070 suffix
/// finds an H, G, F or A descriptor, so construction draws no random word.
fn publish_acquiring(
    entity: &mut Entity,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
) -> bool {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        unreachable!("native Type43 initializer preflight retains its axis allocation")
    };
    // 40B707 copies only live axis+4. A changed strict radius survives C690.
    axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    let position = entity.position_raw();
    let result = apply_search_attack_task_setup(
        &mut entity.actor_tasks,
        SearchAttackTaskSetupRequest::Acquiring,
        |preparation| prepare(preparation, position, axis),
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
) -> Result<PreparedActorTask<ActorTaskRuntime>, Type43Error> {
    match preparation.task.role {
        SearchAttackTaskRole::AcquireTarget => prepare_target_acquisition_runtime_task(
            preparation,
            SearchAttackRadius::from_raw(axis.strict_axis_limit_raw),
            SearchAttackCandidateFilter::from_raw(axis.raw_word_at_0x04),
            0,
        )
        .map_err(|_| Type43Error::ComponentStorage),
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
        _ => Err(Type43Error::ComponentStorage),
    }
}
