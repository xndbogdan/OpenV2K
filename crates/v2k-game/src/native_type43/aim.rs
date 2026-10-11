//! Native Type43 method10 Aim: actor-specific custody over the shared transaction.

use super::{allocation_authenticates, search::pursuing_graph_authenticates};
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
    NativeBallisticAimError as Type43AimError, NativeBallisticAimRuntime as Type43AimRuntime,
    NativeBallisticAimTickOutcome as Type43AimTickOutcome,
    NativeBallisticShotDrainError as Type43ShotDrainError,
    NativeBallisticShotDrainOutcome as Type43ShotDrainOutcome,
};

struct Type43AimProfile;
impl shared::sealed::Sealed for Type43AimProfile {}
impl NativeBallisticProfile for Type43AimProfile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type43;
    fn allocation_authenticates(entity: &Entity) -> bool {
        allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        pursuing_graph_authenticates(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
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

pub fn ensure_type43_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type43AimError> {
    shared::ensure_native_aim_runtime::<Type43AimProfile>(entity, metadata)
}

pub fn tick_type43_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type43AimTickOutcome, Type43AimError> {
    shared::tick_native_aim::<Type43AimProfile>(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        metadata,
    )
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
