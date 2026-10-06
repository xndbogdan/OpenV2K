//! Native Type94 method20 Aim: actor-specific custody over the shared transaction.

use super::{intro2_type94_allocation_authenticates, search::pursuing_graph_authenticates};
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
    NativeBallisticAimError as Intro2Type94AimError,
    NativeBallisticAimRuntime as Intro2Type94AimRuntime,
    NativeBallisticAimTickOutcome as Intro2Type94AimTickOutcome,
    NativeBallisticShotDrainError as Intro2Type94ShotDrainError,
    NativeBallisticShotDrainOutcome as Intro2Type94ShotDrainOutcome,
};

struct Type94Profile;
impl shared::sealed::Sealed for Type94Profile {}
impl NativeBallisticProfile for Type94Profile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type94;
    fn allocation_authenticates(entity: &Entity) -> bool {
        intro2_type94_allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        pursuing_graph_authenticates(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::native::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity
            .intro2_type94_runtime
            .as_ref()
            .map(|runtime| &runtime.sub_e_runtime)
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity
            .intro2_type94_runtime
            .as_mut()
            .map(|runtime| &mut runtime.sub_e_runtime)
    }
    fn queue(entity: &Entity) -> Option<&Intro2Type94AimRuntime> {
        entity.intro2_type94_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Intro2Type94AimRuntime> {
        &mut entity.intro2_type94_aim_runtime
    }
}

pub fn ensure_intro2_type94_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Intro2Type94AimError> {
    shared::ensure_native_aim_runtime::<Type94Profile>(entity, metadata)
}
pub fn tick_intro2_type94_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Intro2Type94AimTickOutcome, Intro2Type94AimError> {
    shared::tick_native_aim::<Type94Profile>(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        metadata,
    )
}
pub fn drain_intro2_type94_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2Type94ShotDrainOutcome, Intro2Type94ShotDrainError> {
    shared::drain_native_shots::<Type94Profile>(
        manager,
        world_fx,
        source_entity_id,
        environment,
        retail_tick,
    )
}

#[cfg(test)]
mod tests;
