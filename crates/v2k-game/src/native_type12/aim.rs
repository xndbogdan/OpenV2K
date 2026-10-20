//! Native Type12-family method24 Aim: actor-specific custody over the shared transaction.

use super::{manager_allocation_authenticates, type12_row, Type12Row};
use crate::native_ground_actor::search::pursuing_graph_authenticates;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::{Entity, EntityManager},
    entity_collision_state::EntityTypeRuntimeMetadata,
    generic_projectile_emitter::GenericEmitterRuntime,
    intro2_native_ballistic_aim::{
        self as shared, NativeBallisticProfile, NativeBallisticProfileId,
    },
    world_fx::{ParticleEnvironment, WorldFx},
};

pub use shared::{
    NativeBallisticAimError as Type12AimError, NativeBallisticAimRuntime as Type12AimRuntime,
    NativeBallisticAimTickOutcome as Type12AimTickOutcome,
    NativeBallisticShotDrainError as Type12ShotDrainError,
    NativeBallisticShotDrainOutcome as Type12ShotDrainOutcome,
};

/// Each row's emitter is its own source profile; the FIFO records which.
macro_rules! row_aim_profile {
    ($name:ident, $id:ident, $row:expr, $ground:ty) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type12_row(entity) == Some($row)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates::<$ground>(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::authenticate_metadata($row, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .native_type12_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .native_type12_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Type12AimRuntime> {
                entity.native_type12_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type12AimRuntime> {
                &mut entity.native_type12_aim_runtime
            }
        }
    };
}

row_aim_profile!(
    Type12AimProfile,
    Type12,
    Type12Row::Type12,
    super::profile::Type12Profile
);
row_aim_profile!(
    Type127AimProfile,
    Type127,
    Type12Row::Type127,
    super::profile::Type127Profile
);

fn manager_row(manager: &EntityManager, entity_id: u32) -> Option<Type12Row> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type12_row)
}

pub fn ensure_type12_family_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type12AimError> {
    match type12_row(entity) {
        Some(Type12Row::Type127) => {
            shared::ensure_native_aim_runtime::<Type127AimProfile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type12AimProfile>(entity, metadata),
    }
}

pub fn tick_type12_family_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type12AimTickOutcome, Type12AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Type12AimError::GraphMismatch
        } else {
            Type12AimError::EntityUnavailable
        });
    }
    match manager_row(manager, entity_id) {
        Some(Type12Row::Type127) => shared::tick_native_aim::<Type127AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        _ => shared::tick_native_aim::<Type12AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
    }
}

pub fn drain_type12_family_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type12ShotDrainOutcome, Type12ShotDrainError> {
    match manager_row(manager, source_entity_id) {
        Some(Type12Row::Type127) => shared::drain_native_shots::<Type127AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        _ => shared::drain_native_shots::<Type12AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
    }
}
