//! Native Hive child:438080's zero0x4C request ->104B0 ->class7 acquiring.
//!
//! The manager owns the common body, position/objective bit, allocation stamp,
//! and sound11. This phase consumes exactly four shared words:1B8C0, then
//! the actual203D0 process allocation,25680, and06070/B940 for both tasks.
//! 1BEB0 owns the later child+60 source/FIFO/count and1C830 ejection writes.

use super::*;

#[cfg(test)]
mod tests;

pub(crate) struct NativeFlyerConstructionRequest<'a> {
    pub entity: &'a mut Entity,
    pub allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    /// An ordinary world's authored spawn and the Sub-D its loop allocated;
    /// a Hive child has neither and allocates its own.
    pub authored: Option<(usize, crate::common_mover::sub_d::NativeSubDConstruction)>,
}

/// The authored B/D/E/G rows: Type15 (wasp), Type87 (deathwas) and Type108;
/// Type14's B/D/G row and Type39's B/D/E/G/K row.
pub(crate) const fn native_flyer_model_id(entity_type: u32) -> Option<u16> {
    match entity_type {
        INTRO2_TYPE15_ENTITY_TYPE => Some(INTRO2_TYPE15_MODEL_ID),
        INTRO2_TYPE87_ENTITY_TYPE => Some(INTRO2_TYPE87_MODEL_ID),
        TYPE108_ENTITY_TYPE => Some(TYPE108_MODEL_ID),
        TYPE14_ENTITY_TYPE => Some(TYPE14_MODEL_ID),
        TYPE39_ENTITY_TYPE => Some(TYPE39_MODEL_ID),
        TYPE41_ENTITY_TYPE => Some(TYPE41_MODEL_ID),
        _ => None,
    }
}

/// Metadata admission is query/RNG-free and can run before the104B0 attempt.
pub(crate) fn authenticate_native_flyer_metadata(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2FlyerPublicationError> {
    let models =
        flyer_model_slots(entity_type).ok_or(Intro2FlyerPublicationError::MetadataMismatch)?;
    authenticate_flyer_mover_metadata(entity_type, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    authenticate_flyer_choices(entity_type, metadata)?;
    if metadata.model_slots != models
        || metadata.mass_raw != flyer_mass(entity_type)
        || metadata.capability_flags != flyer_capability(entity_type)
        || metadata.initial_health_raw.is_none()
        || metadata.damage_profile.is_none()
        || metadata.constructor_sound_attachment_id
            != RetailRuntimeValue::Known(flyer_attachment_sound(entity_type))
        || metadata
            .initializer
            .as_ref()
            .unwrap()
            .alternate_behavior_class_ref
            != flyer_alternate_class(entity_type)
        || !if flyer_has_emitter(entity_type) {
            crate::intro2_flyer_aim::flyer_projectile_descriptor_authenticates(
                entity_type,
                metadata,
            )
        } else {
            metadata.projectile_emitter_descriptor == RetailRuntimeValue::Known(None)
        }
    {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }
    Ok(())
}

pub(crate) fn publish_native_flyer(
    request: NativeFlyerConstructionRequest<'_>,
    world_fx: &mut WorldFx,
) -> Result<Intro2FlyerSchedulerOwner, Intro2FlyerPublicationError> {
    let NativeFlyerConstructionRequest {
        entity,
        allocation,
        metadata,
        authored,
    } = request;
    let authored_spawn_index = authored.map(|(spawn_index, _)| spawn_index);
    authenticate_native_flyer_metadata(entity.entity_type, metadata)?;
    authenticate_native_flyer_body(entity, allocation, metadata, authored_spawn_index)?;
    //09A80 reaches G before D: a successful1B8C0 consumes its word before
    //203D0 advances the independent process byte. Both precede25680/C6B0.
    let descriptor = authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    entity.sub_g_06070_runtime =
        RetailRuntimeValue::Known(Some(SubG06070RuntimeState::from_1b8c0_constructor(
            descriptor,
            world_fx.next_shared_retail_random_u16(),
        )));
    // 09A80 reaches A after G: 20450 draws the craft's forward target.
    if flyer_topology(entity.entity_type).sub_a {
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
            crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(
                TYPE46_SUB_A,
                world_fx.next_shared_retail_random_u16(),
            ),
        ));
    }
    let sub_d = match authored {
        Some((_, sub_d)) => sub_d,
        None => world_fx.construct_entity_sub_d(flyer_sub_d(entity.entity_type)),
    };
    publish_flyer_behavior_phase(
        entity,
        metadata,
        FlyerBirthProvenance::NativeFlyer(NativeFlyerBirth {
            allocation,
            authored_spawn_index,
        }),
        sub_d.runtime,
        sub_d.frame_owner,
        &mut || u32::from(world_fx.next_shared_retail_random_u16()),
    )?;
    Intro2FlyerSchedulerOwner::adopt_published(entity)
        .map_err(|_| Intro2FlyerPublicationError::TaskSetupMismatch)
}

/// `FUN_004438A0`'s phase-4 craft: the request's type 46 under type 51's
/// list. `self_righting` is controller `+0x20C`, which `FUN_0041B9C0` writes
/// to Sub-G `+0x41`; Sub-J's capacity is `FUN_00418620`'s min(5,
/// `+0x199`). Sub-O's `+0x10`/`+0x14` (1 for controller `+0x86` zero) are
/// read only by the fan updates class 7 never runs.
pub(crate) fn publish_attract_craft(
    entity: &mut Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    self_righting: u8,
    cargo_unlock: u8,
    world_fx: &mut WorldFx,
) -> Result<Intro2FlyerSchedulerOwner, Intro2FlyerPublicationError> {
    if entity.entity_type != TYPE46_ENTITY_TYPE {
        return Err(Intro2FlyerPublicationError::UnexpectedEntityType);
    }
    let RetailRuntimeValue::Known(Some(sub_j)) = &metadata.sub_j_attachment_descriptor else {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    };
    let mut attachments = crate::sub_j_attachment::SubJAttachmentRuntime::from_descriptor(sub_j)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    attachments.set_capacity_clamped(usize::from(cargo_unlock));
    entity.sub_j_attachment_runtime = RetailRuntimeValue::Known(Some(attachments));
    let owner = publish_native_flyer(
        NativeFlyerConstructionRequest {
            entity: &mut *entity,
            allocation,
            metadata,
            authored: None,
        },
        world_fx,
    )?;
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        return Err(Intro2FlyerPublicationError::SubGRuntimeUnavailable);
    };
    sub_g.set_self_righting_mode(self_righting);
    Ok(owner)
}

pub(crate) fn authenticate_native_flyer_body(
    entity: &Entity,
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
    metadata: &EntityTypeRuntimeMetadata,
    authored_spawn_index: Option<usize>,
) -> Result<(), Intro2FlyerPublicationError> {
    let initial_behavior_supported = match entity.initial_behavior {
        RetailRuntimeValue::Unresolved => true,
        RetailRuntimeValue::Known(Some(selection)) => {
            selection.program.class_id == SEARCH_ATTACK_BEHAVIOR_CLASS_ID
                && selection.choice_index == 0
        }
        RetailRuntimeValue::Known(None) => false,
    };
    let Some(models) = flyer_model_slots(entity.entity_type) else {
        return Err(Intro2FlyerPublicationError::EntityIdentityMismatch);
    };
    if !entity.active
        || entity.id != allocation.entity_id
        || entity.authored_spawn_index != authored_spawn_index
        || entity.intro2_flyer_frame_owner.is_some()
        || entity.intro2_flyer_aim_runtime.is_some()
        || entity.attached_to.is_some()
        || entity.model_slots != models.map(|model| Some(usize::from(model)))
        || entity.model_index != Some(usize::from(models[0]))
        || entity.mass_raw != metadata.mass_raw
        || entity.capability_flags != metadata.capability_flags
        || entity.collision.health_raw
            != RetailRuntimeValue::Known(metadata.initial_health_raw.unwrap())
        || entity.collision.constructor_sound_attachment_id_at_0x8c
            != RetailRuntimeValue::Known(flyer_attachment_sound(entity.entity_type))
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
