//! Native Type38-family method10 Aim: actor-specific custody over the shared transaction.

use super::{manager_allocation_authenticates, type38_row, Type38Row};
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
    NativeBallisticAimError as Type38AimError, NativeBallisticAimRuntime as Type38AimRuntime,
    NativeBallisticAimTickOutcome as Type38AimTickOutcome,
    NativeBallisticShotDrainError as Type38ShotDrainError,
    NativeBallisticShotDrainOutcome as Type38ShotDrainOutcome,
};

/// Each row's emitter is its own source profile; the FIFO records which.
macro_rules! row_aim_profile {
    ($name:ident, $id:ident, $row:expr, $ground:ty) => {
        struct $name;
        impl shared::sealed::Sealed for $name {}
        impl NativeBallisticProfile for $name {
            const ID: NativeBallisticProfileId = NativeBallisticProfileId::$id;
            fn allocation_authenticates(entity: &Entity) -> bool {
                type38_row(entity) == Some($row)
            }
            fn graph_authenticates(entity: &Entity) -> bool {
                pursuing_graph_authenticates::<$ground>(entity)
            }
            fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
                super::authenticate_metadata($row, metadata).is_ok()
            }
            fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
                entity
                    .native_type38_runtime
                    .as_ref()
                    .map(|runtime| &runtime.sub_e_runtime)
            }
            fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
                entity
                    .native_type38_runtime
                    .as_mut()
                    .map(|runtime| &mut runtime.sub_e_runtime)
            }
            fn queue(entity: &Entity) -> Option<&Type38AimRuntime> {
                entity.native_type38_aim_runtime.as_ref()
            }
            fn queue_slot_mut(entity: &mut Entity) -> &mut Option<Type38AimRuntime> {
                &mut entity.native_type38_aim_runtime
            }
        }
    };
}

row_aim_profile!(
    Type38AimProfile,
    Type38,
    Type38Row::Type38,
    super::profile::Type38Profile
);
row_aim_profile!(
    Type129AimProfile,
    Type129,
    Type38Row::Type129,
    super::profile::Type129Profile
);
row_aim_profile!(
    Type119AimProfile,
    Type119,
    Type38Row::Type119,
    super::profile::Type119Profile
);
row_aim_profile!(
    Type32AimProfile,
    Type32,
    Type38Row::Type32,
    super::profile::Type32Profile
);

row_aim_profile!(
    Type2AimProfile,
    Type2,
    Type38Row::Type2,
    super::profile::Type2Profile
);

fn manager_row(manager: &EntityManager, entity_id: u32) -> Option<Type38Row> {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(type38_row)
}

pub fn ensure_type38_family_aim_runtime(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<(), Type38AimError> {
    match type38_row(entity) {
        Some(Type38Row::Type129) => {
            shared::ensure_native_aim_runtime::<Type129AimProfile>(entity, metadata)
        }
        Some(Type38Row::Type119) => {
            shared::ensure_native_aim_runtime::<Type119AimProfile>(entity, metadata)
        }
        Some(Type38Row::Type32) => {
            shared::ensure_native_aim_runtime::<Type32AimProfile>(entity, metadata)
        }
        Some(Type38Row::Type2) => {
            shared::ensure_native_aim_runtime::<Type2AimProfile>(entity, metadata)
        }
        _ => shared::ensure_native_aim_runtime::<Type38AimProfile>(entity, metadata),
    }
}

pub fn tick_type38_family_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
) -> Result<Type38AimTickOutcome, Type38AimError> {
    // Even Restricted visits age the task wrapper. Authenticate its issuing
    // manager before allowing any part of that prefix to commit.
    if !manager_allocation_authenticates(manager, entity_id) {
        return Err(if manager.iter_all().any(|entity| entity.id == entity_id) {
            Type38AimError::GraphMismatch
        } else {
            Type38AimError::EntityUnavailable
        });
    }
    match manager_row(manager, entity_id) {
        Some(Type38Row::Type129) => shared::tick_native_aim::<Type129AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        Some(Type38Row::Type119) => shared::tick_native_aim::<Type119AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        Some(Type38Row::Type32) => shared::tick_native_aim::<Type32AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        Some(Type38Row::Type2) => shared::tick_native_aim::<Type2AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
        _ => shared::tick_native_aim::<Type38AimProfile>(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            elapsed_micros,
            metadata,
        ),
    }
}

pub fn drain_type38_family_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Type38ShotDrainOutcome, Type38ShotDrainError> {
    match manager_row(manager, source_entity_id) {
        Some(Type38Row::Type129) => shared::drain_native_shots::<Type129AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        Some(Type38Row::Type119) => shared::drain_native_shots::<Type119AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        Some(Type38Row::Type32) => shared::drain_native_shots::<Type32AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
        Some(Type38Row::Type2) => Err(shared::NativeBallisticShotDrainError::ActorBirthMethod),
        _ => shared::drain_native_shots::<Type38AimProfile>(
            manager,
            world_fx,
            source_entity_id,
            environment,
            retail_tick,
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type2ShotDrainError {
    Drain(Type38ShotDrainError),
    /// Every command was consumed; this beam's 438080 failed.
    Birth(crate::native_type69::Type69Error),
}

/// 11400 -> 14870 -> 44E770 for each queued method-25 command: the shot's
/// position and velocity (the shooter's plus speed 1), the shooter's current
/// Euler words, then 438080 builds a Type69 with the shooter at +60. The
/// scheduler registers each beam's owner before its next pass.
pub fn drain_type2_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    resources: &crate::resource_cache::ResourceCache,
    source_entity_id: u32,
    retail_tick: u32,
) -> Result<usize, Type2ShotDrainError> {
    let solutions =
        shared::take_native_shot_solutions::<Type2AimProfile>(manager, source_entity_id)
            .map_err(Type2ShotDrainError::Drain)?
            .solutions;
    let mut blocked = None;
    for solution in &solutions {
        let rotation_raw = manager
            .iter_all()
            .find(|entity| entity.id == source_entity_id)
            .map(Entity::rotation_heading_pitch_roll_raw)
            .ok_or(Type2ShotDrainError::Birth(
                crate::native_type69::Type69Error::SourceAllocation,
            ))?;
        match manager.construct_native_type69(
            crate::entity::Type69BirthRequest {
                shooter_id: source_entity_id,
                position_raw: solution.position_raw,
                velocity_raw: solution.velocity_raw,
                rotation_raw,
            },
            resources,
            world_fx,
            retail_tick,
        ) {
            Ok(owner) => manager.queue_born_native_type69(owner),
            Err(reason) => {
                blocked.get_or_insert(reason);
            }
        }
    }
    match blocked {
        Some(reason) => Err(Type2ShotDrainError::Birth(reason)),
        None => Ok(solutions.len()),
    }
}
