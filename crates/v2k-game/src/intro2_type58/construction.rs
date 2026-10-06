//! Shared authored104B0/09A80/D4A0 entry with actual process Sub-D custody.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::{sub_d::NativeSubDConstruction, SubAPropulsionRuntime},
    entity::EntityConstructionResources,
    entity_behavior::translate_state_policy,
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
};
use v2k_formats::levels::EntitySpawn;

pub(crate) struct Type58AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_type58(
    request: Type58AuthoredConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type58Publication, Intro2Type58Error> {
    let Type58AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        resources,
        constructor_surface_bits,
        sub_d,
    } = request;
    authenticate_metadata(metadata)?;
    let slots = std::array::from_fn(|slot| {
        let model = if spawn.model_overrides[slot] == 0 {
            u32::from(metadata.model_slots[slot])
        } else {
            spawn.model_overrides[slot]
        };
        (model != 0).then_some(model as usize)
    });
    if !entity.active
        || entity.entity_type != 58
        || spawn.entity_type != 58
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.capability_flags != metadata.capability_flags
        || entity.model_slots != slots
        || slots != [Some(MODEL); 4]
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Intro2Type58Error::Identity);
    }
    if entity.intro2_type58_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type58Error::AlreadyPublished);
    }
    // A persistent arriving player has no authored index and participates in
    // PlayerNearby before the current world's first authored allocation.
    if preceding.iter().any(|candidate| {
        candidate.id == entity.id
            || candidate
                .authored_spawn_index
                .is_some_and(|index| index >= spawn.index)
    }) || preceding.windows(2).any(|pair| {
        matches!((pair[0].authored_spawn_index, pair[1].authored_spawn_index),
            (Some(a), Some(b)) if a >= b)
    }) {
        return Err(Intro2Type58Error::Prefix);
    }
    let RetailRuntimeValue::Known(Some(a)) = metadata.sub_a_propulsion_descriptor else {
        unreachable!()
    };
    if entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(AXIS)
        || entity.sub_a_propulsion_runtime
            != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(a)))
        || !native::initializer_storage_authenticates(entity)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
        || entity.collision.health_raw != RetailRuntimeValue::Known(7000)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Intro2Type58Error::ComponentStorage);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Intro2Type58Error::Runtime("constructor surface bits"));
    }
    let terrain = resources
        .terrain
        .ok_or(Intro2Type58Error::Runtime("constructor terrain"))?;
    let objects = resources
        .terrain_objects
        .ok_or(Intro2Type58Error::Runtime("constructor terrain objects"))?;
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Intro2Type58Error::Runtime("constructor terrain cell"))?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        spawn.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type58Error::Metadata);
    };
    let mut state =
        RetailStateWord::exact(0x0607_8801 | if spawn.param != 0 { 0x0100_0000 } else { 0 });
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    state.overwrite(
        ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        if cell.terrain_type & 0x10 != 0 {
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
        } else {
            0
        },
    );
    let policy = translate_state_policy(
        metadata
            .initializer
            .as_ref()
            .unwrap()
            .initializer_state_flags_raw,
    );
    state.overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.model_index = Some(MODEL);
    // Common104B0 +44 and Type58 type-vtable+30 are both null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    let runtime = Intro2Type58Runtime {
        entity_id: entity.id,
        spawn_index: spawn.index,
        model_slots: slots,
        anchor_raw,
        origin: Type58ConstructionOrigin::Native(allocation),
        sub_d_runtime: sub_d.runtime,
        sub_d_owner: sub_d.frame_owner,
        sub_e_runtime: native::emitter_constructor(),
    };
    native::publish_birth(
        entity,
        metadata,
        preceding,
        terrain,
        objects,
        runtime,
        next_random,
    )
}
