//! Ordinary Type26 stag beetles use 104B0/09A80/D4A0 and the current process RNG.

use super::*;
use crate::{
    common_mover::{sub_d::NativeSubDConstruction, SubAPropulsionRuntime},
    entity::EntityConstructionResources,
    entity_behavior::translate_state_policy,
    entity_collision_state::{
        RetailStateWord, ACTIVE_MODEL_SLOT_HIGH_STATE_BIT, SURFACE_STATE_MASK,
    },
    main_base_abort::MainBaseAbortActorLease,
};
use v2k_formats::levels::EntitySpawn;

pub(crate) struct Type26AuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub resources: EntityConstructionResources<'a>,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_type26(
    request: Type26AuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type26Publication, Intro2Type26DefecateVirusPublicationError> {
    use Intro2Type26DefecateVirusPublicationError as Error;
    let Type26AuthoredConstruction {
        entity,
        allocation,
        metadata,
        spawn,
        resources,
        constructor_surface_bits,
        sub_d,
    } = request;
    authenticate_metadata(metadata)?;
    if !matches!(&metadata.sub_h_external_frame_descriptor,
        RetailRuntimeValue::Known(Some(descriptor)) if descriptor.records.len() == TYPE26_SUB_H_RECORD_COUNT)
    {
        return Err(Error::MetadataMismatch);
    }
    if !entity.active
        || entity.entity_type != 26
        || spawn.entity_type != 26
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.capability_flags != 8
        || entity.model_slots != [Some(267); 4]
        || spawn
            .model_overrides
            .iter()
            .any(|model| !matches!(*model, 0 | 267))
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Error::EntityIdentityMismatch);
    }
    if entity.native_type26_allocation.is_some()
        || entity.intro2_type26_sub_d_frame_owner.is_some()
        || entity.intro2_type26_sub_d_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Error::InitialBehaviorAlreadyResolved);
    }
    if entity.sub_a_propulsion_runtime
        != RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::pending_constructor_rng(
            INTRO2_TYPE26_SUB_A,
        )))
        || entity.collision.health_raw != RetailRuntimeValue::Known(5000)
        || entity.collision.pre_health_damage_buffer_raw
            != RetailRuntimeValue::Known(spawn.initial_damage_buffer_raw)
        || entity.actor_common_axis_descriptor
            != RetailRuntimeValue::Known(
                metadata
                    .initializer
                    .as_ref()
                    .unwrap()
                    .common_axis_descriptor,
            )
        || sub_d.descriptor != INTRO2_TYPE26_SUB_D
        || sub_d.runtime != crate::common_mover::sub_d::Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
        || constructor_surface_bits & !SURFACE_STATE_MASK != 0
    {
        return Err(Error::MetadataMismatch);
    }
    let terrain = resources.terrain.ok_or(Error::TerrainUnavailable)?;
    let objects = resources.terrain_objects.ok_or(Error::TerrainUnavailable)?;
    let [x, _, z] = spawn.position_raw();
    let cell = terrain
        .cell(usize::from(x as u16 >> 8), usize::from(z as u16 >> 8))
        .ok_or(Error::TerrainUnavailable)?;
    // 104B0 domain/default setup precedes D4A0 and the weighted initializer.
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
    // Common type-vtable+30 and 104B0's initial +44 modifier are null.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    entity.native_type26_allocation = Some(NativeType26Allocation {
        allocation,
        spawn_index: spawn.index,
    });
    native::publish_retained_birth(
        entity,
        metadata,
        sub_d.frame_owner,
        terrain,
        Some(objects),
        next_random,
    )
}
