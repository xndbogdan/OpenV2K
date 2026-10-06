//! Shared authored104B0/09A80/D4A0 entry, independent of replay coordinates.

use super::*;
use crate::{
    common_mover::{
        sub_d::{NativeSubDConstruction, Type9SubDRuntime},
        SubAPropulsionRuntime,
    },
    entity::EntityConstructionResources,
    entity_behavior::translate_state_policy,
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
};
use v2k_formats::levels::EntitySpawn;

/// Sub-D is the actual process allocation, charged by the common constructor.
/// The surface comparison uses authored Y and the caller's current world wave
/// policy before D4A0 replaces Y with the integer interpolated ground height.
pub(crate) struct Type17AuthoredConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub preceding: &'a [Entity],
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub retail_tick: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_type17(
    request: Type17AuthoredConstructionRequest<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type17Publication, Intro2Type17Error> {
    let Type17AuthoredConstructionRequest {
        entity,
        allocation,
        metadata,
        spawn,
        preceding,
        resources,
        constructor_surface_bits,
        retail_tick,
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
        || entity.entity_type != 17
        || entity.id != allocation.entity_id
        || spawn.entity_type != 17
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
        return Err(Intro2Type17Error::Identity);
    }
    native::validate_unpublished(entity)?;
    native::validate_component_storage(entity)?;
    // The persistent player has no authored index but really precedes the
    // world's authored allocations. Do not erase it from PlayerNearby.
    if preceding.iter().any(|candidate| {
        candidate.id == entity.id
            || candidate
                .authored_spawn_index
                .is_some_and(|index| index >= spawn.index)
    }) || preceding.windows(2).any(|pair| {
        matches!(
            (pair[0].authored_spawn_index, pair[1].authored_spawn_index),
            (Some(a), Some(b)) if a >= b
        )
    }) {
        return Err(Intro2Type17Error::Prefix);
    }
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("authenticated Sub-A descriptor");
    };
    if entity.sub_a_propulsion_runtime
        != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
            sub_a_descriptor,
        )))
        || entity.collision.health_raw != RetailRuntimeValue::Known(5000)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(sub_d.descriptor))
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
    {
        return Err(Intro2Type17Error::ComponentStorage);
    }
    if constructor_surface_bits & !SURFACE_STATE_MASK != 0 {
        return Err(Intro2Type17Error::Runtime("constructor surface bits"));
    }
    let terrain = resources
        .terrain
        .ok_or(Intro2Type17Error::Runtime("constructor terrain"))?;
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from((x as u16) >> 8), usize::from((z as u16) >> 8))
        .ok_or(Intro2Type17Error::Runtime("constructor terrain cell"))?;
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        spawn.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type17Error::Metadata);
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
    // The common104B0 default is null+44; Type17's type-vtable+30 is also
    // null. Pair contact/orientation retain their separately owned evidence.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    let receipt = Intro2Type17Runtime {
        entity_id: entity.id,
        spawn_index: spawn.index,
        model_slots: entity.model_slots,
        anchor_raw,
        origin: Type17ConstructionOrigin::Native(allocation),
    };
    native::publish_birth(
        native::Type17PreparedBirth {
            entity,
            metadata,
            preceding,
            receipt,
            sub_d_runtime: sub_d.runtime,
            sub_d_frame_owner: sub_d.frame_owner,
            retail_tick,
        },
        next_random,
    )
}
