//! Ordinary-world Type13 births through 104B0/09A80.
//!
//! Authored Type13 records in worlds 24..48 match Intro2 spawn0 apart from
//! position: model 291 with no override, param 1, zero angles, and no
//! animation, config or damage buffer. D4A0 has no bit 0x20 for the type, so
//! the authored Y is kept. The birth is spawn0's G/D/K/L transaction, except
//! that the process owns the Sub-D seed and the manager the allocation lease.

use super::*;
use crate::actor_task_owner::ActorTaskSlot;
use crate::common_mover::sub_d::{NativeSubDConstruction, Type9SubDRuntime};
use crate::entity::EntityManager;
use crate::main_base_abort::MainBaseAbortActorLease;
use crate::search_attack_live::{TYPE13_SEARCH_ATTACK_COMMON_AXIS, TYPE13_SEARCH_ATTACK_SUB_D};
use crate::type13_initial_behavior::{
    authenticate_type13_behavior_list, authenticate_type13_metadata,
};
use v2k_formats::levels::EntitySpawn;

/// Ordinary 104B0 publication of one authored Type13.
pub(crate) struct Type13AuthoredConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub constructor_surface_bits: u32,
    pub sub_d: NativeSubDConstruction,
}

pub(crate) fn publish_authored_type13(
    request: Type13AuthoredConstruction<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type13Admission, Intro2Type13PublicationError> {
    use Intro2Type13PublicationError as Error;
    let Type13AuthoredConstruction {
        entity,
        allocation,
        metadata,
        spawn,
        constructor_surface_bits,
        sub_d,
    } = request;
    if !entity.active
        || entity.entity_type != TYPE13_ENTITY_TYPE
        || spawn.entity_type != TYPE13_ENTITY_TYPE
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || spawn
            .model_overrides
            .iter()
            .any(|&model| model != 0 && model as usize != INTRO2_TYPE13_MODEL_ID)
        || entity.position_raw() != spawn.position_raw()
        || entity.rotation_heading_pitch_roll_raw() != spawn.rotation.map(|word| word as i16)
        || spawn.has_animation
        || spawn.animation.is_some()
        || spawn.has_config
        || spawn.config.is_some()
    {
        return Err(Error::EntityIdentityMismatch);
    }
    if entity.model_slots != [Some(INTRO2_TYPE13_MODEL_ID); 4]
        || entity.model_index != Some(INTRO2_TYPE13_MODEL_ID)
    {
        return Err(Error::UnexpectedModel);
    }
    // Preflight every later reader before 1B8C0 consumes the first word.
    authenticate_type13_metadata(metadata).map_err(Error::Constructor)?;
    authenticate_type13_behavior_list(metadata).map_err(Error::Selection)?;
    if metadata.model_slots != [INTRO2_TYPE13_MODEL_ID as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(TYPE13_SEARCH_ATTACK_SUB_D))
    {
        return Err(Error::ComponentStorage);
    }
    if entity.native_type13_allocation.is_some()
        || entity.intro2_type13_common_mover_runtime.is_some()
        || entity.intro2_type13_aim_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Error::AlreadyPublished);
    }
    // Type13 Sub-D flags are zero: no classifier query ever reads the cache,
    // so the process seed is retained but never consulted.
    if sub_d.descriptor != TYPE13_SEARCH_ATTACK_SUB_D
        || sub_d.runtime != Type9SubDRuntime::from_constructor()
        || sub_d.frame_owner.classifier_cache().stagger_counter() != sub_d.seed
        || entity.sub_g_06070_runtime
            != RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()))
        || entity.actor_common_axis_descriptor
            != RetailRuntimeValue::Known(TYPE13_SEARCH_ATTACK_COMMON_AXIS)
        || entity.collision.default_state_flags_at_0xc8 != RetailRuntimeValue::Known(8)
        || entity.collision.pre_health_damage_buffer_raw != RetailRuntimeValue::Known(0)
        || entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || constructor_surface_bits & !crate::entity_collision_state::SURFACE_STATE_MASK != 0
    {
        return Err(Error::ComponentStorage);
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
        entity,
        metadata,
        Intro2Type13BirthSelection::Weighted,
        GklCommonMoverRuntime::from_native_constructor(sub_d.runtime, sub_d.frame_owner),
        next_random,
    )?;
    entity.native_type13_allocation = Some(allocation);
    Ok(Intro2Type13Admission {
        spawn_index: spawn.index,
    })
}

/// Entity identity plus, for an ordinary receipt, the issuing generation.
pub(crate) fn type13_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            authenticate_intro2_type13(entity).is_ok()
                && entity.native_type13_allocation.is_none_or(|lease| {
                    manager
                        .main_base_abort_actor_observation(id)
                        .is_some_and(|observation| observation.lease == lease)
                })
        })
}

#[cfg(test)]
#[path = "ordinary_tests.rs"]
mod tests;
