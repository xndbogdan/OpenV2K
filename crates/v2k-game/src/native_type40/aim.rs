//! Method1 allocation-owned SubE cadence and FIFO; packet1 uses the retail row.
use super::*;
use crate::{
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_native_ballistic_aim::{
        self as ballistic, NativeBallisticProfile, NativeBallisticProfileId,
    },
    world_fx::ParticleEnvironment,
};
pub type Type40AimRuntime = ballistic::NativeBallisticAimRuntime;

pub use ballistic::{
    NativeBallisticAimError as Type40AimError,
    NativeBallisticAimTickOutcome as Type40AimTickOutcome,
    NativeBallisticShotDrainError as Type40ShotDrainError,
    NativeBallisticShotDrainOutcome as Type40ShotDrainOutcome,
};
struct Type40BallisticProfile;
impl ballistic::sealed::Sealed for Type40BallisticProfile {}
impl NativeBallisticProfile for Type40BallisticProfile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type40;
    fn allocation_authenticates(entity: &Entity) -> bool {
        super::allocation_authenticates(entity)
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        shared::search::pursuing_graph_authenticates::<super::profile::Type40Profile>(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity
            .native_type40_runtime
            .as_ref()
            .map(|runtime| &runtime.sub_e_runtime)
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity
            .native_type40_runtime
            .as_mut()
            .map(|runtime| &mut runtime.sub_e_runtime)
    }
    fn queue(entity: &Entity) -> Option<&ballistic::NativeBallisticAimRuntime> {
        entity.native_type40_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<ballistic::NativeBallisticAimRuntime> {
        &mut entity.native_type40_aim_runtime
    }
}
pub fn tick_type40_aim(
    mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    id: u32,
    dt: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type40AimTickOutcome, Type40AimError> {
    if !manager_allocation_authenticates(manager, id) {
        return Err(Type40AimError::GraphMismatch);
    }
    ballistic::tick_native_aim::<Type40BallisticProfile>(mode, manager, fx, id, dt, metadata)
}
pub fn drain_type40_shots(
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    id: u32,
    environment: ParticleEnvironment<'_>,
    tick: u32,
) -> Result<Type40ShotDrainOutcome, Type40ShotDrainError> {
    if !manager_allocation_authenticates(manager, id) {
        return Err(Type40ShotDrainError::RuntimeContractMismatch);
    }
    ballistic::drain_native_shots::<Type40BallisticProfile>(manager, fx, id, environment, tick)
}
