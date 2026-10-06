//! 104B0/09A80/18A90 -> D4A0 -> AC60 -> 257C0/25BD0 construction.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    damage::DamageProfile,
    entity::FactoryLiveOwnerRuntimeState,
    entity_behavior::{
        behavior_program, select_initial_behavior, BehaviorContextRuntime, BehaviorWeightRule,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailStateWord, SURFACE_STATE_MASK},
    entity_initializer::{
        resolve_entity_initializer_with_selected_behavior, EntityInitializerRequest,
        ResourceDomainRelation,
    },
    factory_production::{FactoryProductionRuntime, FactorySection13Config},
    main_base_type66_abort::WorkingFactoryTaskState,
};
use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type66Error> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(Intro2Type66Error::Metadata);
    };
    if metadata.model_slots != [210, 225, 210, 225]
        || metadata.mass_raw != 1000
        || metadata.capability_flags != 0x84
        || metadata.initial_health_raw != Some(INITIAL_HEALTH_RAW)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 2000, 200, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            })
        || metadata.accepted_hit_presentation_sound_id != RetailRuntimeValue::Known(Some(7))
        || metadata.death_sound_id != RetailRuntimeValue::Known(Some(62))
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.generic_hit_sound_id != RetailRuntimeValue::Known(None)
        || metadata.infected_model_presentation_sound_id != RetailRuntimeValue::Known(None)
        || !matches!(metadata.common_world_effects, RetailRuntimeValue::Known(profile)
            if profile.surface_selectors == [0, 0]
                && profile.surface_lifetime_ms == 0
                && profile.low_health_effect_words == [0, 0, 0])
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.status_component_descriptor
            != RetailRuntimeValue::Known(Some(STATUS_DESCRIPTOR))
        || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(None)
        || metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(None)
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_h_external_frame_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_n_payload.is_some()
        || initializer.initializer_state_flags_raw != INITIALIZER_STATE_RAW
        || initializer.common_axis_descriptor != Default::default()
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 0
    {
        return Err(Intro2Type66Error::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_working_factory(
    entity: &mut Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    spawn: &EntitySpawn,
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type66Publication, Intro2Type66Error> {
    let authored_position = spawn.position_raw();
    let xz = [authored_position[0], authored_position[2]];
    let models = std::array::from_fn(|slot| {
        let model = if spawn.model_overrides[slot] != 0 {
            spawn.model_overrides[slot] as usize
        } else {
            usize::from(metadata.model_slots[slot])
        };
        (model != 0).then_some(model)
    });
    if !entity.active
        || entity.entity_type != 66
        || allocation.entity_id != entity.id
        || allocation.allocation_identity == 0
        || entity.authored_spawn_index != Some(spawn.index)
        || spawn.entity_type != 66
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|angle| angle as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || entity.model_slots != models
        || entity.model_index != models[0]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
    {
        return Err(Intro2Type66Error::Identity);
    }
    authenticate_metadata(metadata)?;
    let config = spawn.config.as_ref().ok_or(Intro2Type66Error::Config)?;
    let config = FactorySection13Config::decode(config);
    if entity.intro2_type66_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type66Error::AlreadyPublished);
    }
    let RetailRuntimeValue::Known(Some(mut factory)) = entity.base_factory_runtime else {
        return Err(Intro2Type66Error::ComponentStorage);
    };
    let expected_production =
        FactoryProductionRuntime::from_retail_template(config, INITIAL_HEALTH_RAW);
    let initial_selection = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(39).ok_or(Intro2Type66Error::Selection)?,
    };
    let resolution = resolve_entity_initializer_with_selected_behavior(
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: spawn.param,
            authored_position_raw: authored_position,
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        Some(initial_selection),
    );
    if factory.status_descriptor != STATUS_DESCRIPTOR
        || factory.control_value_raw != config.output_payload_packed() as u16
        || factory.required_scientists != config.scientist_capacity_raw() as u16
        || factory.current_scientists != 0
        || factory.lifter_progress_raw != 0
        || factory.production_progress_raw != 0
        || factory.recovery_progress_raw != 0
        || factory.production != Some(expected_production)
        || factory.live_owner.is_some()
        || factory.progressive_death.elapsed_micros_raw != 0
        || factory.progressive_death.config_flags_at_0x18 != config.raw_words()[6] as u8
        || entity.collision.health_raw != RetailRuntimeValue::Known(INITIAL_HEALTH_RAW)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.default_state_flags_at_0xc8
            != RetailRuntimeValue::Known(INITIALIZER_STATE_RAW)
        || entity.initial_behavior != RetailRuntimeValue::Known(Some(initial_selection))
        || RetailStateWord::from_known_bits(
            entity.collision.state_flags_at_0x08.known_value_bits() & !SURFACE_STATE_MASK,
            entity.collision.state_flags_at_0x08.known_mask() & !SURFACE_STATE_MASK,
        ) != RetailStateWord::from_known_bits(
            resolution.state_flags.known_value_bits() & !SURFACE_STATE_MASK,
            resolution.state_flags.known_mask() & !SURFACE_STATE_MASK,
        )
    {
        return Err(Intro2Type66Error::ComponentStorage);
    }
    // All fallible admission precedes AC60's one unconditional selector word.
    // This singleton has no component constructor RNG and 25BD0 uses01020,
    // not the generic06070 component-reset constructor.
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &INITIAL_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2Type66Error::Selection)?
    .ok_or(Intro2Type66Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type66Error::Selection)?;
    // D4A0 bit20 grounds this descriptor; bit40 model-radius and Sub-C
    // clearance additions are absent. Each authored spawn supplies its own XZ.
    entity.set_position_raw([xz[0], terrain.bilinear_height_raw(xz[0], xz[1]), xz[1]]);
    // 104B0 classifies the authored Y against the constructor sea/wave before
    // 09A80/D4A0 relocates this body. The initializer resolution owns every
    // other known state bit; admission above checks those domains separately.
    // Preserve the earlier surface write, including its legacy unknown state.
    // Explicit native policy for104B0's unwritten transient mass contribution.
    // 12DA0 and future contributors retain ownership after this one birth write.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let live_owner = FactoryLiveOwnerRuntimeState::from_constructor(entity.id, INITIAL_HEALTH_RAW);
    factory.live_owner = Some(live_owner);
    entity.base_factory_runtime = RetailRuntimeValue::Known(Some(factory));
    entity.intro2_type66_runtime = Some(Intro2Type66Runtime {
        entity_id: entity.id,
        spawn_index: spawn.index,
        allocation,
        model_slots: models,
        config,
        factory_allocation_identity: live_owner.allocation_identity,
        voice_position_raw: authored_position,
    });
    publish_working_factory_graph(entity, context, live_owner.allocation_identity);
    Ok(Intro2Type66Publication {
        selection,
        selector_word,
    })
}

fn publish_working_factory_graph(
    entity: &mut Entity,
    context: BehaviorContextRuntime,
    allocation_identity: u64,
) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    // 257C0 clears T thenS, then25BD0 publishes its zero-lifetime Primary.
    // The class39 style has zero+34/+38 policy words and no06070 suffix.
    entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::WorkingFactory(
            WorkingFactoryTaskState::new(allocation_identity),
        )),
    );
}

/// Intro2's post-load actor setup, `4519BE..4519D7` in `451710`.
///
/// After the complete `2E570` constructor/radar phase, mode<4 writes current
/// health1 for Type66 (the preceding Type34/capability8 branch is distinct).
/// This does not change the type's99999 maximum, Sub-M's health cache, task
/// clocks, or RNG. Keep it separate from104B0/18A90 construction and reject
/// repeated or post-callback application. The caller owns the Intro2 phase.
pub fn apply_intro2_type66_post_load_health(entity: &mut Entity) -> Result<(), Intro2Type66Error> {
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(Intro2Type66Error::Identity);
    }
    let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
        return Err(Intro2Type66Error::ComponentStorage);
    };
    let owner = factory
        .live_owner
        .ok_or(Intro2Type66Error::ComponentStorage)?;
    let primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2Type66Error::ComponentStorage)?;
    if entity.collision.health_raw != RetailRuntimeValue::Known(INITIAL_HEALTH_RAW)
        || factory.progressive_death.elapsed_micros_raw != 0
        || owner.state_version != 1
        || owner.next_transaction_id_raw != 1
        || !matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
            if context.active_style().style_address() == 0x004C_9558)
        || entity.actor_tasks.task_state(primary)
            != Some(&ActorTaskRuntime::WorkingFactory(
                WorkingFactoryTaskState::new(owner.allocation_identity),
            ))
        || entity.actor_tasks.wrapper_flags(primary)
            != Some(crate::actor_task_owner::ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
        || entity.actor_task_state(ActorTaskSlot::Tertiary).is_some()
    {
        return Err(Intro2Type66Error::ComponentStorage);
    }
    entity.collision.health_raw = RetailRuntimeValue::Known(1);
    Ok(())
}

/// AC60/257C0 reentry retains the Sub-M allocation and context payload.
/// The death/impact caller owns every mutation before this selector.
pub(crate) fn reselect_working_factory(
    entity: &mut Entity,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type66Publication, Intro2Type66Error> {
    if !intro2_type66_allocation_authenticates(entity) {
        return Err(Intro2Type66Error::Identity);
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2Type66Error::ComponentStorage);
    };
    let program = behavior_program(39).ok_or(Intro2Type66Error::Selection)?;
    let context = context
        .reselect_named_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(Intro2Type66Error::ComponentStorage)?;
    let allocation_identity = entity
        .intro2_type66_runtime
        .ok_or(Intro2Type66Error::Identity)?
        .factory_allocation_identity;
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &INITIAL_CHOICES,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2Type66Error::Selection)?
    .ok_or(Intro2Type66Error::Selection)?;
    publish_working_factory_graph(entity, context, allocation_identity);
    Ok(Intro2Type66Publication {
        selection,
        selector_word,
    })
}

#[cfg(test)]
#[path = "native_tests.rs"]
pub(super) mod tests;
