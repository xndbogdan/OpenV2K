//! Native Hive child:438080's zero0x4C request ->104B0 ->class7 acquiring.
//!
//! The manager owns the common body, position/objective bit, allocation stamp,
//! and sound11. This phase consumes exactly four shared words:1B8C0, then
//! the actual203D0 process allocation,25680, and06070/B940 for both tasks.
//! 1BEB0 owns the later child+60 source/FIFO/count and1C830 ejection writes.

use super::*;

#[cfg(test)]
mod tests;

pub(crate) struct NativeType15ConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
}

/// Metadata admission is query/RNG-free and can run before the104B0 attempt.
pub(crate) fn authenticate_native_type15_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2FlyerPublicationError> {
    authenticate_flyer_mover_metadata(15, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    authenticate_flyer_choices(metadata)?;
    if metadata.model_slots != [INTRO2_TYPE15_MODEL_ID; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw.is_none()
        || metadata.damage_profile.is_none()
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(Some(11))
        || metadata
            .initializer
            .as_ref()
            .unwrap()
            .alternate_behavior_class_ref
            != 2
        || !crate::intro2_flyer_aim::flyer_projectile_descriptor_authenticates(15, metadata)
    {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }
    Ok(())
}

pub(crate) fn publish_native_type15(
    request: NativeType15ConstructionRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<Intro2FlyerSchedulerOwner, Intro2FlyerPublicationError> {
    let NativeType15ConstructionRequest {
        entity,
        allocation,
        metadata,
    } = request;
    authenticate_native_type15_metadata(metadata)?;
    authenticate_native_type15_body(entity, allocation, metadata)?;
    //09A80 reaches G before D: a successful1B8C0 consumes its word before
    //203D0 advances the independent process byte. Both precede25680/C6B0.
    let descriptor = authenticate_flyer_mover_metadata(15, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    entity.sub_g_06070_runtime =
        RetailRuntimeValue::Known(Some(SubG06070RuntimeState::from_1b8c0_constructor(
            descriptor,
            world_fx.next_shared_retail_random_u16(),
        )));
    let sub_d = world_fx.construct_entity_sub_d(FLYER_SUB_D);
    publish_flyer_behavior_phase(
        entity,
        metadata,
        FlyerBirthProvenance::NativeType15(NativeType15Birth { allocation }),
        sub_d.runtime,
        sub_d.frame_owner,
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
    )?;
    Intro2FlyerSchedulerOwner::adopt_published(entity)
        .map_err(|_| Intro2FlyerPublicationError::TaskSetupMismatch)
}

pub(crate) fn authenticate_native_type15_body(
    entity: &Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2FlyerPublicationError> {
    let initial_behavior_supported = match entity.initial_behavior {
        RetailRuntimeValue::Unresolved => true,
        RetailRuntimeValue::Known(Some(selection)) => {
            selection.program.class_id == SEARCH_ATTACK_BEHAVIOR_CLASS_ID
                && selection.choice_index == 0
        }
        RetailRuntimeValue::Known(None) => false,
    };
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.entity_type != 15
        || entity.authored_spawn_index.is_some()
        || entity.intro2_flyer_frame_owner.is_some()
        || entity.intro2_flyer_aim_runtime.is_some()
        || entity.attached_to.is_some()
        || entity.model_slots != [Some(usize::from(INTRO2_TYPE15_MODEL_ID)); 4]
        || entity.model_index != Some(usize::from(INTRO2_TYPE15_MODEL_ID))
        || entity.mass_raw != metadata.mass_raw
        || entity.capability_flags != metadata.capability_flags
        || entity.collision.health_raw
            != RetailRuntimeValue::Known(metadata.initial_health_raw.unwrap())
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(Some(11))
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || !initial_behavior_supported
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .any(|slot| entity.actor_tasks.task_in_slot(slot).is_some())
    {
        return Err(Intro2FlyerPublicationError::EntityIdentityMismatch);
    }
    if entity.sub_g_06070_runtime
        != RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()))
    {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }
    Ok(())
}
