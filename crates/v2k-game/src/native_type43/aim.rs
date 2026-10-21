//! Native Type43 method10 and Type35 method11 Aim: actor-specific custody over
//! the shared transaction. Type35's drain builds a Type34 meteor per shot.

use super::{search::pursuing_graph_authenticates, type43_row, Type43Row};
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::{Entity, EntityManager},
    entity_collision_state::EntityTypeRuntimeMetadata,
    generic_projectile_emitter::GenericEmitterRuntime,
    intro2_native_ballistic_aim::{
        self as shared, NativeBallisticProfile, NativeBallisticProfileId,
    },
    native_entity_weapons::{
        meteor::meteor_launch_raw, EntityWeaponBlock, EntityWeaponConstructionRequest,
        EntityWeaponKind,
    },
    resource_cache::ResourceCache,
    world_fx::{ParticleEnvironment, WorldFx},
};

pub use shared::{
    NativeBallisticAimError as Type43AimError, NativeBallisticAimRuntime as Type43AimRuntime,
    NativeBallisticAimTickOutcome as Type43AimTickOutcome,
    NativeBallisticShotDrainError as Type43ShotDrainError,
    NativeBallisticShotDrainOutcome as Type43ShotDrainOutcome,
};

macro_rules! aim_profile {
    ($name:ident, $id:ident, $row:expr) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type43_row(entity) == Some($row)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::native::authenticate_row_metadata($row, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .native_type43_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .native_type43_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Type43AimRuntime> {
                entity.native_type43_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type43AimRuntime> {
                &mut entity.native_type43_aim_runtime
            }
        }
    };
}

aim_profile!(Type43AimProfile, Type43, Type43Row::Type43);
aim_profile!(Type35AimProfile, Type35, Type43Row::Type35);

fn row(manager: &EntityManager, entity_id: u32) -> Option<Type43Row> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type43_row)
}

pub fn ensure_type43_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type43AimError> {
    match type43_row(entity) {
        Some(Type43Row::Type35) => {
            shared::ensure_native_aim_runtime::<Type35AimProfile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type43AimProfile>(entity, metadata),
    }
}

pub fn tick_type43_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type43AimTickOutcome, Type43AimError> {
    match row(manager, entity_id) {
        Some(Type43Row::Type35) => shared::tick_native_aim::<Type35AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        _ => shared::tick_native_aim::<Type43AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
    }
}

pub fn drain_type43_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type43ShotDrainOutcome, Type43ShotDrainError> {
    shared::drain_native_shots::<Type43AimProfile>(
        manager,
        world_fx,
        source_entity_id,
        environment,
        retail_tick,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type35ShotDrainError {
    Drain(Type43ShotDrainError),
    /// Every command was consumed; this birth's 438080 failed.
    Birth(EntityWeaponBlock),
}

/// 11400 -> 14870 -> 44E770 for each queued method-11 command: Y+300, the
/// 0x960 floor, then 438080 with the launcher's current Euler words. The
/// scheduler registers each meteor's owner before its next pass.
pub fn drain_type35_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    resources: &ResourceCache,
    source_entity_id: u32,
    retail_tick: u32,
) -> Result<usize, Type35ShotDrainError> {
    let solutions =
        shared::take_native_shot_solutions::<Type35AimProfile>(manager, source_entity_id)
            .map_err(Type35ShotDrainError::Drain)?
            .solutions;
    let mut blocked = None;
    for solution in &solutions {
        let rotation_raw = manager
            .iter_all()
            .find(|entity| entity.id == source_entity_id)
            .map(|source| source.rotation_heading_pitch_roll_raw())
            .ok_or(Type35ShotDrainError::Birth(
                EntityWeaponBlock::SourceAllocation,
            ))?;
        let (position_raw, velocity_raw) =
            meteor_launch_raw(solution.position_raw, solution.velocity_raw);
        match manager.construct_entity_weapon(
            EntityWeaponConstructionRequest {
                kind: EntityWeaponKind::Meteor,
                source_actor_id: source_entity_id,
                position_raw,
                velocity_raw,
                rotation_raw,
            },
            resources,
            world_fx,
            retail_tick,
        ) {
            Ok(owner) => manager.queue_born_native_weapon(owner),
            Err(reason) => {
                blocked.get_or_insert(reason);
            }
        }
    }
    match blocked {
        Some(reason) => Err(Type35ShotDrainError::Birth(reason)),
        None => Ok(solutions.len()),
    }
}
