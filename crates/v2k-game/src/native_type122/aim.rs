//! Native Type122 method24 Aim: actor-specific custody over the shared transaction.

use super::{type122_allocation_authenticates, type122_manager_allocation_authenticates};
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
    NativeBallisticAimError as Type122AimError, NativeBallisticAimRuntime as Type122AimRuntime,
    NativeBallisticAimTickOutcome as Type122AimTickOutcome,
    NativeBallisticShotDrainError as Type122ShotDrainError,
    NativeBallisticShotDrainOutcome as Type122ShotDrainOutcome,
};

struct Type122Profile;
impl shared::sealed::Sealed for Type122Profile {}
impl NativeBallisticProfile for Type122Profile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type122;
    fn allocation_authenticates(entity: &Entity) -> bool {
        type122_allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        crate::native_ground_actor::search::pursuing_graph_authenticates::<
            super::profile::Type122Profile,
        >(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity
            .native_type122_runtime
            .as_ref()
            .map(|runtime| &runtime.sub_e_runtime)
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity
            .native_type122_runtime
            .as_mut()
            .map(|runtime| &mut runtime.sub_e_runtime)
    }
    fn queue(entity: &Entity) -> Option<&Type122AimRuntime> {
        entity.native_type122_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type122AimRuntime> {
        &mut entity.native_type122_aim_runtime
    }
}

pub fn ensure_native_type122_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type122AimError> {
    shared::ensure_native_aim_runtime::<Type122Profile>(entity, metadata)
}
pub fn tick_type122_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type122AimTickOutcome, Type122AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !type122_manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Type122AimError::GraphMismatch
        } else {
            Type122AimError::EntityUnavailable
        });
    }
    shared::tick_native_aim::<Type122Profile>(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        metadata,
    )
}
pub fn drain_type122_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type122ShotDrainOutcome, Type122ShotDrainError> {
    // A queued command outlives its task graph, but never its allocation.
    if !type122_manager_allocation_authenticates(manager, source_entity_id) {
        return Err(
            if manager
                .iter_all()
                .any(|entity| entity.id == source_entity_id)
            {
                Type122ShotDrainError::RuntimeContractMismatch
            } else {
                Type122ShotDrainError::EntityUnavailable
            },
        );
    }
    shared::drain_native_shots::<Type122Profile>(
        manager,
        world_fx,
        source_entity_id,
        environment,
        retail_tick,
    )
}
