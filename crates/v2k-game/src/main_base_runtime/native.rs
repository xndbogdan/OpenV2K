use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    damage::DamageProfile,
    entity_behavior::{
        behavior_program, select_initial_behavior, BehaviorContextRuntime, BehaviorWeightRule,
    },
    entity_collision_state::EntityTypeRuntimeMetadata,
    entity_initializer::{
        resolve_entity_initializer_with_selected_behavior, EntityInitializerRequest,
        ResourceDomainRelation,
    },
};
use v2k_formats::{
    collision::{BehaviorChoice, StatusComponentDescriptor},
    levels::EntitySpawn,
    terrain::TerrainGrid,
};

pub(super) const INITIAL_HEALTH: i32 = 99_999;
pub(super) const INITIALIZER_STATE: u32 = 0x25027;
const CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 41,
}];
const STATUS: StatusComponentDescriptor = StatusComponentDescriptor {
    raw_word_at_0x00: 0,
    variable_bindings: [0, 0, 1, 0, 0, 0],
    raw_tail: [0; 10],
};

pub(super) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), MainBaseError> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(MainBaseError::Metadata);
    };
    if metadata.model_slots != [286, 225, 286, 225]
        || metadata.mass_raw != 1000
        || metadata.capability_flags != 0x20
        || metadata.initial_health_raw != Some(INITIAL_HEALTH)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 2000, 200, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            })
        || metadata.accepted_hit_presentation_sound_id != RetailRuntimeValue::Known(Some(7))
        || metadata.death_sound_id != RetailRuntimeValue::Known(None)
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.generic_hit_sound_id != RetailRuntimeValue::Known(None)
        || metadata.infected_model_presentation_sound_id != RetailRuntimeValue::Known(None)
        || !matches!(metadata.common_world_effects, RetailRuntimeValue::Known(profile) if profile.surface_selectors == [0, 0] && profile.surface_lifetime_ms == 0 && profile.low_health_effect_words == [0, 0, 0])
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || metadata.common_mover_topology
            != RetailRuntimeValue::Known(crate::intro2_type66::TOPOLOGY)
        || metadata.status_component_descriptor != RetailRuntimeValue::Known(Some(STATUS))
        || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(None)
        || metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(None)
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_h_external_frame_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_n_payload.is_some()
        || initializer.initializer_state_flags_raw != INITIALIZER_STATE
        || initializer.common_axis_descriptor != Default::default()
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 0
    {
        return Err(MainBaseError::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_main_base(
    entity: &mut Entity,
    allocation: MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    spawn: &EntitySpawn,
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<MainBasePublication, MainBaseError> {
    authenticate_metadata(metadata)?;
    let models = std::array::from_fn(|slot| {
        let value = if spawn.model_overrides[slot] != 0 {
            spawn.model_overrides[slot] as usize
        } else {
            usize::from(metadata.model_slots[slot])
        };
        (value != 0).then_some(value)
    });
    let [x, _, z] = spawn.position_raw();
    if !entity.active
        || entity.entity_type != 6
        || spawn.entity_type != 6
        || allocation.entity_id != entity.id
        || allocation.allocation_identity == 0
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.model_slots != models
        || entity.model_index != models[0]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|angle| angle as i16)
        || [entity.position_raw()[0], entity.position_raw()[2]] != [x, z]
        || spawn.has_animation
        || spawn.animation.is_some()
    {
        return Err(MainBaseError::Identity);
    }
    let config =
        FactorySection13Config::decode(spawn.config.as_ref().ok_or(MainBaseError::Config)?);
    // All28 authored ordinary Type6 templates are zero. Their Sub-M has no
    // production output or C830 voice; retain the whole template for19750.
    if config.raw_words() != [0; 22] {
        return Err(MainBaseError::Config);
    }
    if entity.main_base_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(MainBaseError::AlreadyPublished);
    }
    let RetailRuntimeValue::Known(Some(base)) = entity.base_factory_runtime else {
        return Err(MainBaseError::ComponentStorage);
    };
    let selection = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(41).ok_or(MainBaseError::Selection)?,
    };
    let resolution = resolve_entity_initializer_with_selected_behavior(
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: spawn.param,
            authored_position_raw: spawn.position_raw(),
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        Some(selection),
    );
    if base.status_descriptor != STATUS
        || base.control_value_raw != 0
        || base.required_scientists != 0
        || base.current_scientists != 0
        || base.lifter_progress_raw != 0
        || base.production_progress_raw != 0
        || base.recovery_progress_raw != 0
        || base.production.is_some()
        || base.live_owner.is_some()
        || base.progressive_death != crate::base_factory_progression::ProgressiveDeathState::idle(0)
        || entity.collision.health_raw != RetailRuntimeValue::Known(INITIAL_HEALTH)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(INITIALIZER_STATE)
        || entity.initial_behavior != RetailRuntimeValue::Known(Some(selection))
        || entity.collision.state_flags_at_0x08 != resolution.state_flags
    {
        return Err(MainBaseError::ComponentStorage);
    }
    let publication = select(next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(publication.selection)
        .ok_or(MainBaseError::Selection)?;
    entity.set_position_raw([x, terrain.bilinear_height_raw(x, z), z]);
    entity.collision.state_flags_at_0x08 = resolution.state_flags;
    // Explicit native birth policy for104B0's unwritten transient mass word.
    // Subsequent12DA0 visits retain their own source clear/contributor order.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(publication.selection));
    entity.main_base_runtime = Some(MainBaseRuntime {
        allocation,
        spawn_index: spawn.index,
        model_slots: models,
        config,
    });
    publish_graph(entity, context, allocation.allocation_identity);
    Ok(publication)
}

fn select(next_random: &mut impl FnMut() -> u32) -> Result<MainBasePublication, MainBaseError> {
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| MainBaseError::Selection)?
    .ok_or(MainBaseError::Selection)?;
    Ok(MainBasePublication {
        selection,
        selector_word,
    })
}

fn publish_graph(entity: &mut Entity, context: BehaviorContextRuntime, allocation_identity: u64) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::MainBase(MainBaseTaskState::new(
            allocation_identity,
        ))),
    );
}

pub(super) fn reselect_main_base(
    entity: &mut Entity,
    next_random: &mut impl FnMut() -> u32,
) -> Result<MainBasePublication, MainBaseError> {
    if !main_base_allocation_authenticates(entity) {
        return Err(MainBaseError::Identity);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(MainBaseError::ComponentStorage);
    };
    let program = behavior_program(41).ok_or(MainBaseError::Selection)?;
    let context = previous
        .reselect_named_type_default(program, 0, program.initial_style)
        .ok_or(MainBaseError::ComponentStorage)?;
    let publication = select(next_random)?;
    publish_graph(
        entity,
        context,
        entity
            .main_base_runtime
            .unwrap()
            .allocation
            .allocation_identity,
    );
    Ok(publication)
}
