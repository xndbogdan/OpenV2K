//! Ordinary authored Power Ups: 104B0 -> singleton AC60 -> 257F0 (demo256C0).
//! Class23 is infallible and has no actor task; its native allocation still
//! owns contact and the shared class49 death across every authored world.
//! A dying class63 carrier's `40BC90` builds the same body from its own
//! position and `+88` payload; that allocation is native in the same sense.

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
    origin: NativeType61Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeType61Origin {
    Authored { spawn_index: usize },
    AutoPilot(AutoPilotPowerUpBirth),
}

/// `40BC90`'s zero-filled request: type 61 at `+08`, the carrier's `+96..+9A`
/// position words at `+0C`, and its `+88` payload at `+20`. Handle, rotation,
/// objective flag, damage buffer and model overrides all stay zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutoPilotPowerUpBirth {
    pub carrier: MainBaseAbortActorLease,
    pub carrier_type: u32,
    pub position_raw: [i16; 3],
    pub payload_packed: u32,
}

impl NativeType61Allocation {
    #[cfg(test)]
    pub(crate) const fn auto_pilot_birth(self) -> Option<AutoPilotPowerUpBirth> {
        match self.origin {
            NativeType61Origin::AutoPilot(birth) => Some(birth),
            NativeType61Origin::Authored { .. } => None,
        }
    }
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
    // +1C payload/model overrides remain authored.
    publish_power_up(
        entity,
        metadata,
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: spawn.param,
            authored_position_raw: spawn.position_raw(),
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        constructor_surface_bits,
        world_fx,
    )?;
    entity.native_type61_allocation = Some(NativeType61Allocation {
        allocation,
        origin: NativeType61Origin::Authored {
            spawn_index: spawn.index,
        },
    });
    Ok(())
}

pub(crate) struct AutoPilotPowerUpConstruction<'a> {
    pub entity: &'a mut Entity,
    pub allocation: MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub birth: AutoPilotPowerUpBirth,
    pub terrain: &'a TerrainGrid,
    pub constructor_surface_bits: u32,
}

/// Publish `40BC90`'s Type61 on a fresh `104B0` body. The request carries no
/// model overrides, so the row's model slots stand; `+88` is the carrier's.
pub(crate) fn publish_auto_pilot_power_up(
    request: AutoPilotPowerUpConstruction<'_>,
    world_fx: &mut WorldFx,
) -> Result<(), &'static str> {
    let AutoPilotPowerUpConstruction {
        entity,
        allocation,
        metadata,
        birth,
        terrain,
        constructor_surface_bits,
    } = request;
    if !exact_level_one_type61_metadata(metadata)
        || !entity.active
        || entity.entity_type != 61
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index.is_some()
        || entity.factory_type61_birth_provenance().is_some()
        || entity.native_type61_allocation.is_some()
        || entity.position_raw() != birth.position_raw
        || constructor_surface_bits & !SURFACE_STATE_MASK != 0
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err("Type61 auto-pilot construction identity");
    }
    publish_power_up(
        entity,
        metadata,
        EntityInitializerRequest {
            metadata: Some(metadata),
            spawn_param: 0,
            authored_position_raw: birth.position_raw,
            terrain: Some(terrain),
            resource_domain: ResourceDomainRelation::Current,
        },
        constructor_surface_bits,
        world_fx,
    )?;
    entity.power_up_payload_packed = Some(birth.payload_packed);
    entity.native_type61_allocation = Some(NativeType61Allocation {
        allocation,
        origin: NativeType61Origin::AutoPilot(birth),
    });
    Ok(())
}

fn publish_power_up(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    construction: EntityInitializerRequest<'_>,
    constructor_surface_bits: u32,
    world_fx: &mut WorldFx,
) -> Result<(), &'static str> {
    // Even a singleton AC60 selector consumes one process RNG word. 257F0
    // adds no task and cannot fail.
    let selection = select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| i32::from(rule == BehaviorWeightRule::Always),
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| "Type61 selector")?
    .ok_or("Type61 empty selector")?;
    let initializer =
        resolve_entity_initializer_with_selected_behavior(construction, Some(selection));
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
                && match receipt.origin {
                    NativeType61Origin::Authored { spawn_index } => {
                        entity.authored_spawn_index == Some(spawn_index)
                    }
                    NativeType61Origin::AutoPilot(_) => {
                        entity.authored_spawn_index.is_none()
                            && entity.factory_type61_birth_provenance().is_none()
                    }
                }
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
