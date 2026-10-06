//! Native Type58 method20 Aim: actor-specific custody over the shared transaction.

use super::{
    intro2_type58_allocation_authenticates, search::pursuing_graph_authenticates,
    type58_manager_allocation_authenticates,
};
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
    NativeBallisticAimError as Intro2Type58AimError,
    NativeBallisticAimRuntime as Intro2Type58AimRuntime,
    NativeBallisticAimTickOutcome as Intro2Type58AimTickOutcome,
    NativeBallisticShotDrainError as Intro2Type58ShotDrainError,
    NativeBallisticShotDrainOutcome as Intro2Type58ShotDrainOutcome,
};

struct Type58Profile;
impl shared::sealed::Sealed for Type58Profile {}
impl NativeBallisticProfile for Type58Profile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type58;
    fn allocation_authenticates(entity: &Entity) -> bool {
        intro2_type58_allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        pursuing_graph_authenticates(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::native::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity
            .intro2_type58_runtime
            .as_ref()
            .map(|runtime| &runtime.sub_e_runtime)
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity
            .intro2_type58_runtime
            .as_mut()
            .map(|runtime| &mut runtime.sub_e_runtime)
    }
    fn queue(entity: &Entity) -> Option<&Intro2Type58AimRuntime> {
        entity.intro2_type58_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Intro2Type58AimRuntime> {
        &mut entity.intro2_type58_aim_runtime
    }
}

pub fn ensure_intro2_type58_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Intro2Type58AimError> {
    shared::ensure_native_aim_runtime::<Type58Profile>(entity, metadata)
}
pub fn tick_intro2_type58_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Intro2Type58AimTickOutcome, Intro2Type58AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !type58_manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Intro2Type58AimError::GraphMismatch
        } else {
            Intro2Type58AimError::EntityUnavailable
        });
    }
    shared::tick_native_aim::<Type58Profile>(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        metadata,
    )
}
pub fn drain_intro2_type58_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2Type58ShotDrainOutcome, Intro2Type58ShotDrainError> {
    // A queued command outlives its task graph, but never its allocation.
    if !type58_manager_allocation_authenticates(manager, source_entity_id) {
        return Err(
            if manager
                .iter_all()
                .any(|entity| entity.id == source_entity_id)
            {
                Intro2Type58ShotDrainError::RuntimeContractMismatch
            } else {
                Intro2Type58ShotDrainError::EntityUnavailable
            },
        );
    }
    shared::drain_native_shots::<Type58Profile>(
        manager,
        world_fx,
        source_entity_id,
        environment,
        retail_tick,
    )
}

#[cfg(test)]
mod authored_tests;
#[cfg(test)]
mod tests;
