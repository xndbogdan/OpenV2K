//! Own Type43/Type35 104B0/09A80 allocations, then 25680 -> AC60 -> B6C0.

use super::*;
use crate::{
    actor_task_dispatcher::{prepare_target_acquisition_runtime_task, ActorTaskRuntime},
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{
        behavior_program, initial_behavior_state_policy, select_initial_behavior,
        BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
        BehaviorProgram, BehaviorWeightRule, DeathCallbackPolicy,
    },
    entity_collision_state::{
        CommonMoverGklPayloads, CommonWorldEffectProfile, EntityTypeRuntimeMetadata,
        RetailRuntimeValue, DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT, SURFACE_STATE_MASK,
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

/// Type43's own row, for callers outside this owner.
pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type43Error> {
    authenticate_row_metadata(Type43Row::Type43, metadata)
}

pub(crate) fn authenticate_row_metadata(
    row: Type43Row,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type43Error> {
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type43Error::Metadata);
    };
    let model_word = row.model().map_or(0, |model| model as u16);
    if metadata.model_slots != [model_word; 4]
        || metadata.model_variable_count_raw != Known(0)
        || metadata.mass_raw != row.mass()
        || metadata.capability_flags != row.capability()
        || metadata.initial_health_raw != Some(row.health())
        || metadata.damage_profile != Some(row.damage())
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
        || metadata.projectile_emitter_descriptor != Known(Some(row.emitter()))
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
        || initializer.initializer_state_flags_raw != row.default_flags()
        || initializer.common_axis_descriptor != row.axis()
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != row.behavior_rule_ref()
        || initializer.alternate_behavior_class_ref != row.alternate_behavior_class()
    {
        return Err(Type43Error::Metadata);
    }
    Ok(())
}

/// Ordinary 104B0 publication of one authored Type43 or Type35 row.
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
    let row =
        Type43Row::from_entity_type(u32::from(spawn.entity_type)).ok_or(Type43Error::Identity)?;
    let [x, _, z] = spawn.position_raw();
    if !entity.active
        || entity.entity_type != row.entity_type()
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != row.model_slots()
        || entity.model_index != row.model()
        || spawn
            .model_overrides
            .iter()
            .any(|&slot| slot != 0 && Some(slot as usize) != row.model())
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
    authenticate_row_metadata(row, metadata)?;
    authenticate_birth_storage(entity, row, spawn.initial_damage_buffer_raw)?;
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
    let emitter = row.emitter();
    entity.native_type43_runtime = Some(Type43Runtime {
        entity_id: entity.id,
        spawn_index: spawn.index,
        row,
        allocation,
        sub_e_runtime: GenericEmitterRuntime {
            joint_bindings: [None; 2],
            projectile_method: emitter.projectile_method,
            emitter_selector: 0,
            sound_id: u32::from(emitter.sound_id),
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
    let initialized = publish_acquiring(entity, row, selection.program, context);
    Ok(Type43Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

fn authenticate_birth_storage(
    entity: &Entity,
    row: Type43Row,
    damage_buffer_raw: i32,
) -> Result<(), Type43Error> {
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
        || entity.actor_common_axis_descriptor != Known(row.axis())
        || entity.physical_body_basis_q31
            != Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
        || entity.collision.default_state_flags_at_0xc8 != Known(row.default_flags())
        || entity.collision.health_raw != Known(row.health())
        || entity.collision.pre_health_damage_buffer_raw != Known(damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != Known(0)
        || entity.capability_flags != row.capability()
        || entity.mass_raw != row.mass()
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

/// The class7 context to reuse for AC60, from the current named context.
fn reused_class7_context(entity: &Entity) -> Result<BehaviorContextRuntime, Type43Error> {
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
    previous
        .reselect_named_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(Type43Error::Selection)
}

/// C690/AC60 reentry; the caller owns task-result suppression and completed
/// task custody. The allocated context and E state survive. AC60 asks 16460
/// whether the body is dying: a living one takes 25680's weighted selector
/// word, a dying one 25660's alternate without a word. Only Type35's
/// alternate is class7; a dying Type43 never keeps a class7 graph.
pub(super) fn reselect_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type43Publication, Type43Error> {
    let row = type43_row(entity).ok_or(Type43Error::Identity)?;
    authenticate_row_metadata(row, metadata)?;
    let RetailRuntimeValue::Known(dying) =
        entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
    else {
        return Err(Type43Error::ComponentStorage);
    };
    let context = reused_class7_context(entity)?;
    let program = behavior_program(7).ok_or(Type43Error::Selection)?;
    if dying != 0 {
        if row.alternate_behavior_class() != 7 {
            return Err(Type43Error::AlternateBehavior);
        }
        let initialized = publish_acquiring(entity, row, program, context);
        return Ok(Type43Publication {
            selection: BehaviorSelection {
                choice_index: 0,
                program,
            },
            selector_word: 0,
            initializer_fallback: !initialized,
        });
    }
    let (selection, selector_word) = select(next_random)?;
    let initialized = publish_acquiring(entity, row, selection.program, context);
    Ok(Type43Publication {
        selection,
        selector_word,
        initializer_fallback: !initialized,
    })
}

/// 10C10 on a living Type35: health0 and dying, a null type+90 sound and
/// attached sound, then DB80. The Search styles' +2C is null, and the
/// alternate publisher AC40 reaches AC60 on the dying body: 25660 republishes
/// class7 without a word. `None` reports 10C10's remote or already-dying
/// return, which changes nothing.
pub(crate) fn publish_type35_standard_death(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<Option<Type43Publication>, Type43Error> {
    if type43_row(entity) != Some(Type43Row::Type35) {
        return Err(Type43Error::Identity);
    }
    authenticate_row_metadata(Type43Row::Type35, metadata)?;
    let RetailRuntimeValue::Known(state) = entity
        .collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT)
    else {
        return Err(Type43Error::ComponentStorage);
    };
    if state != 0 {
        return Ok(None);
    }
    let RetailRuntimeValue::Known(Some(current)) = entity.current_behavior_context else {
        return Err(Type43Error::Selection);
    };
    if current.active_style().death_callback_policy() != DeathCallbackPolicy::None
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(None)
    {
        return Err(Type43Error::AlternateBehavior);
    }
    let context = reused_class7_context(entity)?;
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
    let program = behavior_program(7).ok_or(Type43Error::Selection)?;
    let initialized = publish_acquiring(entity, Type43Row::Type35, program, context);
    Ok(Some(Type43Publication {
        selection: BehaviorSelection {
            choice_index: 0,
            program,
        },
        selector_word: 0,
        initializer_fallback: !initialized,
    }))
}

/// B6C0: copy the type's axis+04, clear Tertiary, then install the slot-1
/// acquisition and the 500-ms slot-0 02BA0 retarget. Neither 06070 suffix
/// finds an H, G, F or A descriptor, so construction draws no random word.
fn publish_acquiring(
    entity: &mut Entity,
    row: Type43Row,
    program: &'static BehaviorProgram,
    context: BehaviorContextRuntime,
) -> bool {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
        unreachable!("native Type43 initializer preflight retains its axis allocation")
    };
    // 40B707 copies only live axis+4. A changed strict radius survives C690.
    axis.raw_word_at_0x04 = row.axis().raw_word_at_0x04;
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
