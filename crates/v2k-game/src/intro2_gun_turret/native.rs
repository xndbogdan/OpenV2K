//! 09A80 -> D4A0 -> AC60/D190 -> 4086D0/408DF0 native publication.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, PreparedActorTask},
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{
        behavior_program, initial_behavior_state_policy, select_initial_behavior,
        BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverGklPayloads, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
};
use v2k_formats::terrain::TerrainGrid;

pub(crate) fn authenticate_metadata(
    profile: Intro2GunTurretProfile,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2GunTurretError> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2GunTurretError::Metadata);
    };
    if metadata.model_slots != profile.model_slots()
        || metadata.model_variable_count_raw
            != RetailRuntimeValue::Known(profile.model_variable_count())
        || metadata.mass_raw != 100
        || metadata.capability_flags != profile.capability()
        || metadata.initial_health_raw != Some(profile.health())
        || metadata.damage_profile != Some(profile.damage())
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: Some(profile.sub_l()),
            })
        || metadata.sub_a_propulsion_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(None)
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
            if profile.surface_selectors == [0,0] && profile.surface_lifetime_ms == 0
                && profile.low_health_effect_words == [0,0,0])
        || initializer.initializer_state_flags_raw != 0x25025
        || initializer.common_axis_descriptor != profile.axis()
        || initializer.behavior_choices.as_ref() != profile.choices()
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != profile.alternate_behavior_class()
    {
        return Err(Intro2GunTurretError::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_intro2_gun_turret(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2GunTurretPublication, Intro2GunTurretError> {
    let profile = profile_for_entity(entity).ok_or(Intro2GunTurretError::Identity)?;
    let spawn_index = entity
        .authored_spawn_index
        .ok_or(Intro2GunTurretError::Identity)?;
    let xz = profile
        .authored_xz(spawn_index)
        .ok_or(Intro2GunTurretError::Identity)?;
    publish_gun_turret_at(
        entity,
        metadata,
        GunTurretBirth {
            profile,
            spawn_index,
            xz,
            origin: GunTurretConstructionOrigin::Intro2Authored,
            damage_buffer_raw: 0,
            constructor_surface_bits: None,
        },
        terrain,
        next_random,
    )
}

/// Ordinary class-29 E/L miss: same D190 birth at an explicit pose.
/// The Intro2 spawn-52 XZ table is not the admission key.
pub(crate) fn publish_ordinary_class29_turret_at_position(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    position_raw: [i16; 3],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2GunTurretPublication, Intro2GunTurretError> {
    let profile = Intro2GunTurretProfile::for_ordinary_cargo(entity.entity_type)
        .ok_or(Intro2GunTurretError::Identity)?;
    publish_gun_turret_at(
        entity,
        metadata,
        GunTurretBirth {
            profile,
            spawn_index: entity.authored_spawn_index.unwrap_or(0),
            xz: [position_raw[0], position_raw[2]],
            origin: GunTurretConstructionOrigin::CampaignReconstruction,
            damage_buffer_raw: 0,
            constructor_surface_bits: None,
        },
        terrain,
        next_random,
    )
}

pub(crate) struct GunTurretAuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub allocation: MainBaseAbortActorLease,
    pub spawn: &'a v2k_formats::levels::EntitySpawn,
    pub terrain: &'a TerrainGrid,
    pub constructor_surface_bits: u32,
}

/// The ordinary104B0 allocation carries its actual authored pose and param.
/// It is independent of Intro2's authored identities and451C00 zero records.
pub(crate) fn publish_authored_gun_turret(
    request: GunTurretAuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2GunTurretPublication, Intro2GunTurretError> {
    let GunTurretAuthoredConstruction {
        entity,
        metadata,
        allocation,
        spawn,
        terrain,
        constructor_surface_bits,
    } = request;
    let profile = Intro2GunTurretProfile::for_native_authored(entity.entity_type)
        .ok_or(Intro2GunTurretError::Identity)?;
    if u32::from(spawn.entity_type) != entity.entity_type
        || allocation.entity_id != entity.id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|value| value as i16)
        || spawn.model_overrides != [0; 4]
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
        || entity.collision.state_flags_at_0x08.masked(0x0100_0000)
            != RetailRuntimeValue::Known(if spawn.param != 0 { 0x0100_0000 } else { 0 })
    {
        return Err(Intro2GunTurretError::Identity);
    }
    if constructor_surface_bits & !crate::entity_collision_state::SURFACE_STATE_MASK != 0 {
        return Err(Intro2GunTurretError::ComponentStorage);
    }
    let [x, _, z] = spawn.position_raw();
    publish_gun_turret_at(
        entity,
        metadata,
        GunTurretBirth {
            profile,
            spawn_index: spawn.index,
            xz: [x, z],
            origin: GunTurretConstructionOrigin::NativeOrdinary(allocation),
            damage_buffer_raw: spawn.initial_damage_buffer_raw,
            constructor_surface_bits: Some(constructor_surface_bits),
        },
        terrain,
        next_random,
    )
}

struct GunTurretBirth {
    profile: Intro2GunTurretProfile,
    spawn_index: usize,
    xz: [i16; 2],
    origin: GunTurretConstructionOrigin,
    damage_buffer_raw: i32,
    constructor_surface_bits: Option<u32>,
}

fn publish_gun_turret_at(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    birth: GunTurretBirth,
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2GunTurretPublication, Intro2GunTurretError> {
    let GunTurretBirth {
        profile,
        spawn_index,
        xz,
        origin,
        damage_buffer_raw,
        constructor_surface_bits,
    } = birth;
    if !entity.active
        || entity.entity_type != profile.entity_type()
        || entity.model_slots != profile.model_slots().map(|model| Some(usize::from(model)))
        || !matches!(entity.collision.active_model_slot(), RetailRuntimeValue::Known(slot)
            if entity.model_index == entity.model_in_slot(usize::from(slot)))
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
        || (matches!(origin, GunTurretConstructionOrigin::Intro2Authored)
            && profile.authored_xz(spawn_index).is_some()
            && entity.rotation_heading_pitch_roll_raw() != [0; 3])
    {
        return Err(Intro2GunTurretError::Identity);
    }
    authenticate_metadata(profile, metadata)?;
    if entity.intro2_gun_turret_runtime.is_some()
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2GunTurretError::AlreadyPublished);
    }
    let expected = BehaviorSelection {
        choice_index: 0,
        program: behavior_program(29).ok_or(Intro2GunTurretError::Selection)?,
    };
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    if (profile != Intro2GunTurretProfile::Type115
        && entity.initial_behavior != RetailRuntimeValue::Known(Some(expected)))
        || entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(profile.axis())
        || entity.physical_body_basis_q31
            != RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(0x25025)
        || entity.collision.health_raw != RetailRuntimeValue::Known(profile.health())
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.capability_flags != profile.capability()
        || entity.collision.state_flags_at_0x08.masked(0x2000) == RetailRuntimeValue::Unresolved
    {
        return Err(Intro2GunTurretError::ComponentStorage);
    }
    if let Some(surface_bits) = constructor_surface_bits {
        entity.collision.state_flags_at_0x08.overwrite(
            crate::entity_collision_state::SURFACE_STATE_MASK,
            surface_bits,
        );
        //104B0+44 and common type-vtable+30 are null for these E/L rows.
        entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
        entity
            .collision
            .pair_callbacks
            .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
        entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    }
    // 09A80 zeroes the authored model bank, allocates L1BB80, axis235A0,
    // then E24E30. Neither component allocates a random value. E+00 binds
    // selector3 for Type102; Type92's four pointers are null. Descriptor+12
    // is a muzzle slot, not a binding.
    let emitter = profile.emitter();
    entity.intro2_gun_turret_runtime = Some(Intro2GunTurretRuntime {
        entity_id: entity.id,
        spawn_index,
        profile,
        origin,
        sub_e_runtime: GenericEmitterRuntime {
            joint_bindings: [
                (emitter.variable_bindings[1] != 0)
                    .then_some(u64::from(emitter.variable_bindings[1])),
                (emitter.variable_bindings[0] != 0)
                    .then_some(u64::from(emitter.variable_bindings[0])),
            ],
            projectile_method: u32::from(emitter.projectile_method),
            emitter_selector: 0,
            sound_id: u32::from(emitter.sound_id),
            direct_mode: 0,
            remaining_time_raw: 0,
            manual_step_raw: 0,
            remaining_bursts_raw: 0,
            cadence_raw: 0,
            basis_adjustment_identity: None,
        },
        sub_e_joint_word_raw: 0,
        sub_l_target_raw: [0; 3],
        sub_l_exact_raw: 0,
        sub_l_output_raw: [0; 2],
    });
    // D4A0's default bit20 grounds before the selector; bit40 and SubC are
    // absent. D720/13F70 then rebuilds Q31 from the spawn Euler words.
    entity.set_position_raw([xz[0], terrain.bilinear_height_raw(xz[0], xz[1]), xz[1]]);
    entity.apply_d720_euler_body_basis();
    // Explicit deterministic native policy for104B0's unwritten B2 residue.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    let publication = select(entity, profile, next_random)?;
    if profile == Intro2GunTurretProfile::Type115 {
        //The passive weighted-domain projection merged D190 success with
        //C490. Fresh104B0 starts these setup bits clear; the selected
        //initializer below owns their exact publication.
        entity.collision.state_flags_at_0x08.overwrite(0x28, 0);
    }
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(publication.selection)
        .ok_or(Intro2GunTurretError::Selection)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(publication.selection));
    publish_graph(entity, publication.selection, context);
    Ok(publication)
}

fn select(
    entity: &Entity,
    profile: Intro2GunTurretProfile,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2GunTurretPublication, Intro2GunTurretError> {
    let RetailRuntimeValue::Known(mutated) = entity.collision.state_flags_at_0x08.masked(0x2000)
    else {
        return Err(Intro2GunTurretError::ComponentStorage);
    };
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        profile.choices(),
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::Mutated => i32::from(mutated != 0),
            _ => 0,
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2GunTurretError::Selection)?
    .ok_or(Intro2GunTurretError::Selection)?;
    Ok(Intro2GunTurretPublication {
        selection,
        selector_word,
    })
}

/// C690's caller owns suppression and completed-task custody. AC60 retains
/// the allocated context's target/auxiliary and all E/L state on reentry.
pub(crate) fn reselect_intro2_gun_turret(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2GunTurretPublication, Intro2GunTurretError> {
    if !intro2_gun_turret_allocation_authenticates(entity) {
        return Err(Intro2GunTurretError::Identity);
    }
    authenticate_metadata(
        profile_for_entity(entity).ok_or(Intro2GunTurretError::Identity)?,
        metadata,
    )?;
    if entity.collision.state_flags_at_0x08.masked(0x4000) != RetailRuntimeValue::Known(0) {
        return Err(Intro2GunTurretError::AlternateBehavior);
    }
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Intro2GunTurretError::Selection);
    };
    if previous.choice_list_source()
        != RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        || entity.collision.state_flags_at_0x08.masked(0x2000) == RetailRuntimeValue::Unresolved
    {
        return Err(Intro2GunTurretError::ComponentStorage);
    }
    let profile = profile_for_entity(entity).ok_or(Intro2GunTurretError::Identity)?;
    let publication = select(entity, profile, next_random)?;
    let program = publication.selection.program;
    let context = previous
        .reselect_named_type_default(
            program,
            program.initial_style_table_index_raw,
            program.initial_style,
        )
        .ok_or(Intro2GunTurretError::Selection)?;
    publish_graph(entity, publication.selection, context);
    Ok(publication)
}

fn publish_graph(
    entity: &mut Entity,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
) {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if selection.program.class_id == 0 {
        //C490 clears Secondary then Tertiary before installing02800's
        //9000ms Primary timer. E/L and the presentation FIFO survive.
        entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        entity.actor_tasks.replace_prepared(
            ActorTaskSlot::Primary,
            PreparedActorTask::new(ActorTaskRuntime::Class0Timer(
                crate::class0_timer::Class0TimerTaskState::new(),
            )),
        );
        return;
    }
    // D190 clears Secondary then Primary;4086D0 prepares the new private
    // allocation before A7A0 retires the former Tertiary wrapper.
    entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
    entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
    let runtime = entity
        .intro2_gun_turret_runtime
        .as_ref()
        .expect("authenticated E/L allocation");
    //44EA60(method12/14) returns zero; flower method16 returns one.
    let state = task::GunTurretTaskState::from_408df0(
        entity.capability_flags,
        runtime.sub_l_output_raw,
        runtime.profile.gravity_lead(),
    );
    if entity.collision.state_flags_at_0x08.masked(0x2000) == RetailRuntimeValue::Known(0x2000) {
        entity.capability_flags = (entity.capability_flags & !0x1004) | 8;
    }
    entity.collision.state_flags_at_0x08.overwrite(0x100, 0x100);
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Tertiary,
        PreparedActorTask::new(ActorTaskRuntime::Intro2GunTurret(state)),
    );
    entity.collision.state_flags_at_0x08.overwrite(0x28, 0x28);
}

#[cfg(test)]
#[path = "native_tests.rs"]
pub(crate) mod tests;
