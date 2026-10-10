//! Native Type10-family method10 Aim: actor-specific custody over the shared transaction.

use super::{search::pursuing_graph_authenticates, type10_profile, Type10Profile};
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
    NativeBallisticAimError as Intro2Type10AimError, NativeBallisticAimRuntime as Type10AimRuntime,
    NativeBallisticAimTickOutcome as Intro2Type10AimTickOutcome,
    NativeBallisticShotDrainError as Intro2Type10ShotDrainError,
    NativeBallisticShotDrainOutcome as Intro2Type10ShotDrainOutcome,
};

/// Each row's emitter is its own source profile; the FIFO records which.
macro_rules! family_aim_profile {
    ($name:ident, $id:ident, $profile:expr) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type10_profile(entity) == Some($profile)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::native::authenticate_metadata($profile, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .intro2_type10_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .intro2_type10_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Type10AimRuntime> {
                entity.intro2_type10_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type10AimRuntime> {
                &mut entity.intro2_type10_aim_runtime
            }
        }
    };
}

family_aim_profile!(Type10AimProfile, Type10, Type10Profile::Type10);
family_aim_profile!(Type5AimProfile, Type5, Type10Profile::Type5);

fn manager_profile(manager: &EntityManager, entity_id: u32) -> Option<Type10Profile> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type10_profile)
}

pub fn ensure_intro2_type10_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Intro2Type10AimError> {
    match type10_profile(entity) {
        Some(Type10Profile::Type5) => {
            shared::ensure_native_aim_runtime::<Type5AimProfile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type10AimProfile>(entity, metadata),
    }
}
pub fn tick_intro2_type10_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Intro2Type10AimTickOutcome, Intro2Type10AimError> {
    match manager_profile(manager, entity_id) {
        Some(Type10Profile::Type5) => shared::tick_native_aim::<Type5AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        _ => shared::tick_native_aim::<Type10AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
    }
}
pub fn drain_intro2_type10_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2Type10ShotDrainOutcome, Intro2Type10ShotDrainError> {
    match manager_profile(manager, source_entity_id) {
        Some(Type10Profile::Type5) => shared::drain_native_shots::<Type5AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        _ => shared::drain_native_shots::<Type10AimProfile>(
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
