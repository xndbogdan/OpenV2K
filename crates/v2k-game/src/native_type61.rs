//! Ordinary authored Power Ups: 104B0 -> singleton AC60 -> 257F0 (demo256C0).
//! Class23 is infallible and has no actor task; its native allocation still
//! owns contact and the shared class49 death across every authored world.

use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::{Entity, EntityManager},
    entity_behavior::{select_initial_behavior, BehaviorWeightRule},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, SURFACE_STATE_MASK},
    entity_initializer::{
        resolve_entity_initializer_with_selected_behavior, EntityInitializerRequest,
        ResourceDomainRelation,
    },
    main_base_abort::MainBaseAbortActorLease,
    main_base_type61_abort::exact_level_one_type61_metadata,
    world_fx::WorldFx,
};
use v2k_formats::{levels::EntitySpawn, terrain::TerrainGrid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeType61Allocation {
    allocation: MainBaseAbortActorLease,
    spawn_index: usize,
}

pub(crate) struct AuthoredPowerUpConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub spawn: &'a EntitySpawn,
    pub terrain: &'a TerrainGrid,
    pub constructor_surface_bits: u32,
}

pub(crate) fn publish_authored_power_up(
    request: AuthoredPowerUpConstruction<'_>,
    world_fx: &mut WorldFx,
) -> Result<(), &'static str> {
    let AuthoredPowerUpConstruction {
        entity,
        allocation,
        metadata,
        spawn,
        terrain,
        constructor_surface_bits,
    } = request;
    if !exact_level_one_type61_metadata(metadata)
        || !entity.active
        || entity.entity_type != 61
        || spawn.entity_type != 61
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != Some(spawn.index)
        || entity.factory_type61_birth_provenance().is_some()
        || entity.native_type61_allocation.is_some()
        || constructor_surface_bits & !SURFACE_STATE_MASK != 0
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err("Type61 native construction identity");
    }
    // Even a singleton AC60 selector consumes one process RNG word. 257F0
    // adds no task and cannot fail; +1C payload/model overrides remain authored.
    let selection = select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| "Type61 selector")?
    .ok_or("Type61 empty selector")?;
    let initializer = resolve_entity_initializer_with_selected_behavior(
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: spawn.param,
            authored_position_raw: spawn.position_raw(),
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        Some(selection),
    );
    let mut state = initializer.state_flags;
    state.overwrite(SURFACE_STATE_MASK, constructor_surface_bits);
    entity.collision.state_flags_at_0x08 = state;
    entity.initial_behavior = initializer.initial_behavior;
    entity.current_behavior_context = initializer.current_behavior_context;
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    entity
        .collision
        .pair_callbacks
        .damage_modifier_identity_context_empty = RetailRuntimeValue::Known(true);
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    entity.native_type61_allocation = Some(NativeType61Allocation {
        allocation,
        spawn_index: spawn.index,
    });
    Ok(())
}

pub(crate) fn has_native_allocation(entity: &Entity) -> bool {
    entity.native_type61_allocation.is_some()
}

pub(crate) fn entity_authenticates(entity: &Entity) -> bool {
    entity.active
        && entity.entity_type == 61
        && entity.capability_flags == 0x40
        && entity.native_type61_allocation.is_some_and(|receipt| {
            receipt.allocation.entity_id == entity.id
                && entity.authored_spawn_index == Some(receipt.spawn_index)
        })
}

pub(crate) fn allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    entity_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observed| {
                Some(observed.lease) == entity.native_type61_allocation.map(|r| r.allocation)
            })
}

#[cfg(test)]
mod tests;
