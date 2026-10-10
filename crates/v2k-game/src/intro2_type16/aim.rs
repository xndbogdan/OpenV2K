//! Native Type16 method20 Aim: actor-specific custody over the shared transaction.

use super::{search::pursuing_graph_authenticates, type16_row, Type16Row};
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
    NativeBallisticAimError as Intro2Type16AimError,
    NativeBallisticAimRuntime as Intro2Type16AimRuntime,
    NativeBallisticAimTickOutcome as Intro2Type16AimTickOutcome,
    NativeBallisticShotDrainError as Intro2Type16ShotDrainError,
    NativeBallisticShotDrainOutcome as Intro2Type16ShotDrainOutcome,
};

/// Each row's emitter is its own source profile; the FIFO records which.
macro_rules! row_aim_profile {
    ($name:ident, $id:ident, $row:expr) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type16_row(entity) == Some($row)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::native::authenticate_metadata($row, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .intro2_type16_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .intro2_type16_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Intro2Type16AimRuntime> {
                entity.intro2_type16_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Intro2Type16AimRuntime> {
                &mut entity.intro2_type16_aim_runtime
            }
        }
    };
}

row_aim_profile!(Type16AimProfile, Type16, Type16Row::Type16);
row_aim_profile!(Type128AimProfile, Type128, Type16Row::Type128);

fn manager_row(manager: &EntityManager, entity_id: u32) -> Option<Type16Row> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type16_row)
}

pub fn ensure_intro2_type16_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Intro2Type16AimError> {
    match type16_row(entity) {
        Some(Type16Row::Type128) => {
            shared::ensure_native_aim_runtime::<Type128AimProfile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type16AimProfile>(entity, metadata),
    }
}
pub fn tick_intro2_type16_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Intro2Type16AimTickOutcome, Intro2Type16AimError> {
    match manager_row(manager, entity_id) {
        Some(Type16Row::Type128) => shared::tick_native_aim::<Type128AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        _ => shared::tick_native_aim::<Type16AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
    }
}
pub fn drain_intro2_type16_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2Type16ShotDrainOutcome, Intro2Type16ShotDrainError> {
    match manager_row(manager, source_entity_id) {
        Some(Type16Row::Type128) => shared::drain_native_shots::<Type128AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        _ => shared::drain_native_shots::<Type16AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
    }
}

#[cfg(test)]
mod tests;
