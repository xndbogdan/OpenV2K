//! Native Type18 method20 Aim: actor-specific custody over the shared transaction.

use super::{allocation_authenticates, manager_allocation_authenticates};
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
    NativeBallisticAimError as Type18AimError, NativeBallisticAimRuntime as Type18AimRuntime,
    NativeBallisticAimTickOutcome as Type18AimTickOutcome,
    NativeBallisticShotDrainError as Type18ShotDrainError,
    NativeBallisticShotDrainOutcome as Type18ShotDrainOutcome,
};

struct Type18AimProfile;
impl shared::sealed::Sealed for Type18AimProfile {}
impl NativeBallisticProfile for Type18AimProfile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type18;
    fn allocation_authenticates(entity: &Entity) -> bool {
        allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        crate::native_ground_actor::search::pursuing_graph_authenticates::<
            super::profile::Type18Profile,
        >(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity
            .native_type18_runtime
            .as_ref()
            .map(|runtime| &runtime.sub_e_runtime)
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity
            .native_type18_runtime
            .as_mut()
            .map(|runtime| &mut runtime.sub_e_runtime)
    }
    fn queue(entity: &Entity) -> Option<&Type18AimRuntime> {
        entity.native_type18_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type18AimRuntime> {
        &mut entity.native_type18_aim_runtime
    }
}

pub fn ensure_native_type18_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type18AimError> {
    shared::ensure_native_aim_runtime::<Type18AimProfile>(entity, metadata)
}
pub fn tick_type18_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type18AimTickOutcome, Type18AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Type18AimError::GraphMismatch
        } else {
            Type18AimError::EntityUnavailable
        });
    }
    shared::tick_native_aim::<Type18AimProfile>(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        metadata,
    )
}
pub fn drain_type18_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type18ShotDrainOutcome, Type18ShotDrainError> {
    // A queued command outlives its task graph, but never its allocation.
    if !manager_allocation_authenticates(manager, source_entity_id) {
        return Err(
            if manager
                .iter_all()
                .any(|entity| entity.id == source_entity_id)
            {
                Type18ShotDrainError::RuntimeContractMismatch
            } else {
                Type18ShotDrainError::EntityUnavailable
            },
        );
    }
    shared::drain_native_shots::<Type18AimProfile>(
        manager,
        world_fx,
        source_entity_id,
        environment,
        retail_tick,
    )
}
