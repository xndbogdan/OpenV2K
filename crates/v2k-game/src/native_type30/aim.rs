//! Native Type30 method20 and Type33 method24 Aim: actor-specific custody over
//! the shared transaction, one source profile per row.

use super::{manager_allocation_authenticates, type30_row, Type30Row};
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
    NativeBallisticAimError as Type30AimError, NativeBallisticAimRuntime as Type30AimRuntime,
    NativeBallisticAimTickOutcome as Type30AimTickOutcome,
    NativeBallisticShotDrainError as Type30ShotDrainError,
    NativeBallisticShotDrainOutcome as Type30ShotDrainOutcome,
};

macro_rules! row_aim_profile {
    ($name:ident, $id:ident, $row:expr, $ground:ty) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type30_row(entity) == Some($row)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates::<$ground>(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::authenticate_row_metadata($row, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .native_type30_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .native_type30_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Type30AimRuntime> {
                entity.native_type30_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type30AimRuntime> {
                &mut entity.native_type30_aim_runtime
            }
        }
    };
}

row_aim_profile!(
    Type30Profile,
    Type30,
    Type30Row::Type30,
    super::profile::Type30Profile
);
row_aim_profile!(
    Type33Profile,
    Type33,
    Type30Row::Type33,
    super::profile::Type33Profile
);

fn manager_row(manager: &EntityManager, entity_id: u32) -> Option<Type30Row> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type30_row)
}

pub fn ensure_native_type30_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type30AimError> {
    match type30_row(entity) {
        Some(Type30Row::Type33) => {
            shared::ensure_native_aim_runtime::<Type33Profile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type30Profile>(entity, metadata),
    }
}
pub fn tick_type30_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type30AimTickOutcome, Type30AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Type30AimError::GraphMismatch
        } else {
            Type30AimError::EntityUnavailable
        });
    }
    match manager_row(manager, entity_id) {
        Some(Type30Row::Type33) => shared::tick_native_aim::<Type33Profile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        _ => shared::tick_native_aim::<Type30Profile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
    }
}
pub fn drain_type30_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type30ShotDrainOutcome, Type30ShotDrainError> {
    // A queued command outlives its task graph, but never its allocation.
    if !manager_allocation_authenticates(manager, source_entity_id) {
        return Err(
            if manager
                .iter_all()
                .any(|entity| entity.id == source_entity_id)
            {
                Type30ShotDrainError::RuntimeContractMismatch
            } else {
                Type30ShotDrainError::EntityUnavailable
            },
        );
    }
    match manager_row(manager, source_entity_id) {
        Some(Type30Row::Type33) => shared::drain_native_shots::<Type33Profile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        _ => shared::drain_native_shots::<Type30Profile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
    }
}
