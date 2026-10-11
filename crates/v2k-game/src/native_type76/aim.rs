//! Native Type76-family Aim (Type76 method24, Type77 method30): actor-specific
//! custody over the shared transaction.

use super::{manager_allocation_authenticates, type76_row, Type76Row};
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
    NativeBallisticAimError as Type76AimError, NativeBallisticAimRuntime as Type76AimRuntime,
    NativeBallisticAimTickOutcome as Type76AimTickOutcome,
    NativeBallisticShotDrainError as Type76ShotDrainError,
    NativeBallisticShotDrainOutcome as Type76ShotDrainOutcome,
};

/// Each row's emitter is its own source profile; the FIFO records which.
macro_rules! row_aim_profile {
    ($name:ident, $id:ident, $row:expr, $ground:ty) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type76_row(entity) == Some($row)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates::<$ground>(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::authenticate_metadata($row, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .native_type76_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .native_type76_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Type76AimRuntime> {
                entity.native_type76_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type76AimRuntime> {
                &mut entity.native_type76_aim_runtime
            }
        }
    };
}

row_aim_profile!(
    Type76AimProfile,
    Type76,
    Type76Row::Type76,
    super::profile::Type76Profile
);
row_aim_profile!(
    Type77AimProfile,
    Type77,
    Type76Row::Type77,
    super::profile::Type77Profile
);

fn manager_row(manager: &EntityManager, entity_id: u32) -> Option<Type76Row> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type76_row)
}

pub fn ensure_type76_family_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type76AimError> {
    match type76_row(entity) {
        Some(Type76Row::Type77) => {
            shared::ensure_native_aim_runtime::<Type77AimProfile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type76AimProfile>(entity, metadata),
    }
}

pub fn tick_type76_family_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
) -> Result<Type76AimTickOutcome, Type76AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Type76AimError::GraphMismatch
        } else {
            Type76AimError::EntityUnavailable
        });
    }
    let row = manager_row(manager, entity_id).ok_or(Type76AimError::GraphMismatch)?;
    let metadata = manager.type_runtime_metadata(row.entity_type()).cloned();
    match row {
        Type76Row::Type77 => shared::tick_native_aim::<Type77AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata.as_ref(),
        ),
        Type76Row::Type76 => shared::tick_native_aim::<Type76AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata.as_ref(),
        ),
    }
}

pub fn drain_type76_family_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type76ShotDrainOutcome, Type76ShotDrainError> {
    match manager_row(manager, source_entity_id) {
        Some(Type76Row::Type77) => shared::drain_native_shots::<Type77AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        _ => shared::drain_native_shots::<Type76AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
    }
}
