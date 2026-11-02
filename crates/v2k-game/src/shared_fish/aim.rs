//! Type21's class7 Aim (method 24): fish custody over the shared transaction.

use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::{Entity, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterRuntime,
    intro2_native_ballistic_aim::{
        self as shared, NativeBallisticProfile, NativeBallisticProfileId,
    },
    world_fx::{ParticleEnvironment, WorldFx},
};

pub use shared::{
    NativeBallisticAimError as FishAimError, NativeBallisticAimRuntime as FishAimRuntime,
    NativeBallisticAimTickOutcome as FishAimTickOutcome,
    NativeBallisticShotDrainError as FishShotDrainError,
    NativeBallisticShotDrainOutcome as FishShotDrainOutcome,
};

/// ADE0's graph: `4C7A98` holding Chase (slot0) and Aim (slot2) against the
/// context's target, with slot1 cleared.
pub(crate) fn pursuing_graph_authenticates(entity: &Entity) -> bool {
    if !super::searches_and_attacks(entity.entity_type) || entity.shared_fish_runtime.is_none() {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    if context.active_style().style_address() != 0x004c_7a98 {
        return false;
    }
    let RetailRuntimeValue::Known(Some(target)) = context.target_handle_at_0x08() else {
        return false;
    };
    matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == target)
        && matches!(entity.actor_task_state(ActorTaskSlot::Tertiary), Some(ActorTaskRuntime::AimAndFire(task)) if task.private_state().target_entity_id() == target)
        && entity.actor_task_state(ActorTaskSlot::Secondary).is_none()
}

struct Type21AimProfile;
impl shared::sealed::Sealed for Type21AimProfile {}
impl NativeBallisticProfile for Type21AimProfile {
    const ID: NativeBallisticProfileId = NativeBallisticProfileId::Type21;
    fn allocation_authenticates(entity: &Entity) -> bool {
        super::searches_and_attacks(entity.entity_type) && entity.shared_fish_runtime.is_some()
    }
    fn graph_authenticates(entity: &Entity) -> bool {
        pursuing_graph_authenticates(entity)
    }
    fn metadata_authenticates(metadata: &EntityTypeRuntimeMetadata) -> bool {
        super::authenticate_metadata(metadata).is_ok()
    }
    fn emitter(entity: &Entity) -> Option<&GenericEmitterRuntime> {
        entity.shared_fish_runtime.as_ref()?.sub_e.as_ref()
    }
    fn emitter_mut(entity: &mut Entity) -> Option<&mut GenericEmitterRuntime> {
        entity.shared_fish_runtime.as_mut()?.sub_e.as_mut()
    }
    fn queue(entity: &Entity) -> Option<&FishAimRuntime> {
        entity.shared_fish_aim_runtime.as_ref()
    }
    fn queue_slot_mut(entity: &mut Entity) -> &mut Option<FishAimRuntime> {
        &mut entity.shared_fish_aim_runtime
    }
}

pub(crate) fn tick_fish_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<FishAimTickOutcome, FishAimError> {
    // Even Restricted visits age the task wrapper: authenticate first.
    if !super::allocation_authenticates(manager, entity_id) {
        return Err(FishAimError::EntityUnavailable);
    }
    shared::tick_native_aim::<Type21AimProfile>(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        Some(metadata),
    )
}

pub fn drain_fish_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    source_entity_id: u32,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<FishShotDrainOutcome, FishShotDrainError> {
    // A queued command outlives its task graph, but never its allocation.
    if !super::allocation_authenticates(manager, source_entity_id) {
        return Err(FishShotDrainError::EntityUnavailable);
    }
    shared::drain_native_shots::<Type21AimProfile>(
        manager,
        world_fx,
        source_entity_id,
        environment,
        retail_tick,
    )
}
