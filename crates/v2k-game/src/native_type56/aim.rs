//! Method30 allocation-owned SubE cadence and FIFO; packet87 uses the retail row.
use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_native_ballistic_aim::{
        self as ballistic, NativeBallisticProfile, NativeBallisticProfileId,
    },
    world_fx::ParticleEnvironment,
};
pub use ballistic::{
    NativeBallisticAimError as Type56AimError,
    NativeBallisticAimTickOutcome as Type56AimTickOutcome,
    NativeBallisticShotDrainError as Type56ShotDrainError,
    NativeBallisticShotDrainOutcome as Type56ShotDrainOutcome,
};
struct Type56BallisticProfile;
impl ballistic::sealed::Sealed for Type56BallisticProfile {}
impl NativeBallisticProfile for Type56BallisticProfile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type56;
    fn allocation_authenticates(entity: &Entity) -> bool {
        super::allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        shared::search::pursuing_graph_authenticates::<super::profile::Type56Profile>(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity
            .native_type56_runtime
            .as_ref()
            .map(|runtime| &runtime.sub_e_runtime)
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity
            .native_type56_runtime
            .as_mut()
            .map(|runtime| &mut runtime.sub_e_runtime)
    }
    fn queue(entity: &Entity) -> Option<&ballistic::NativeBallisticAimRuntime> {
        entity.native_type56_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<ballistic::NativeBallisticAimRuntime> {
        &mut entity.native_type56_aim_runtime
    }
}
pub fn tick_type56_aim(
    mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    id: u32,
    dt: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type56AimTickOutcome, Type56AimError> {
    if !manager_allocation_authenticates(manager, id) {
        return Err(Type56AimError::GraphMismatch);
    }
    ballistic::tick_native_aim::<Type56BallisticProfile>(mode, manager, fx, id, dt, metadata)
}
pub fn drain_type56_shots(
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    id: u32,
    environment: ParticleEnvironment<'_>,
    tick: u32,
) -> Result<Type56ShotDrainOutcome, Type56ShotDrainError> {
    if !manager_allocation_authenticates(manager, id) {
        return Err(Type56ShotDrainError::RuntimeContractMismatch);
    }
    ballistic::drain_native_shots::<Type56BallisticProfile>(manager, fx, id, environment, tick)
}
