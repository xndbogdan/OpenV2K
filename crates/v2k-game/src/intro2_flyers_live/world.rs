//! The flyers' normal 12DA0 prefix and DCA0/E870 post-task suffix.
//!
//! Callback timing and callback mass belong to the common scheduler. A task
//! evidence block parks that already-entered visit instead of replaying its
//! prefix/RNG on another frame. Unsupported relation, wind and surface effect
//! histories fail before entry to this bounded B/D/E/G, B/D/G and B/D/E/G/K
//! route.

use super::*;
use crate::common_mover::type9_surface::{
    decay_actor_surface_timer_ms, ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT,
};
use crate::common_mover::type9_tail::{
    plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
};
use crate::entity::{apply_type13_common_environment_raw, commit_common_master_motion};
use crate::entity_collision_state::{
    CommonWorldEffectProfile, BODY_BASIS_REBUILT_STATE_BIT, REMOTE_OWNED_STATE_BIT,
};
use crate::entity_scheduler::{
    commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
    common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerWorldBlock {
    SchedulerStateUnavailable,
    RemoteOwnerUnsupported,
    UnsupportedRelation,
    UnsupportedSoundAttachment,
    UnsupportedMetadata,
    UnsupportedEffectiveFlags,
    UnsupportedWindMode { actual: u32 },
    AnimationOffsetUnavailable,
    SurfaceStateUnavailable,
    SurfaceTimerUnavailable,
    MasterMotionStateUnavailable,
    DetailedSoundUnavailable,
}

fn blocked(
    owner: Intro2FlyerSchedulerOwner,
    reason: Intro2FlyerRuntimeBlock,
) -> Intro2FlyerSchedulerOwnerTick {
    Intro2FlyerSchedulerOwnerTick {
        outcome: Intro2FlyerSchedulerProductionOutcome::Blocked {
            entity_id: owner.entity_id,
            reason,
        },
        retained_owner: Some(owner),
    }
}

fn world_block(
    owner: Intro2FlyerSchedulerOwner,
    reason: Intro2FlyerWorldBlock,
) -> Intro2FlyerSchedulerOwnerTick {
    blocked(owner, Intro2FlyerRuntimeBlock::World(reason))
}

fn effective_flags(entity: &Entity) -> Result<u32, Intro2FlyerWorldBlock> {
    let RetailRuntimeValue::Known(default) = entity.collision.default_state_flags_at_0xc8 else {
        return Err(Intro2FlyerWorldBlock::UnsupportedEffectiveFlags);
    };
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Intro2FlyerWorldBlock::UnsupportedEffectiveFlags);
    };
    // Style +38. Move About Aimlessly and Type14's Defecate Virus and both
    // Flocking styles mask nothing.
    let disable = match context.active_style().style_address() {
        0x004c_7a50 => 0x0002_1080,
        0x004c_7a98 => 0x80,
        0x004c_7930 | 0x004c_7e88 | 0x004c_7df8 | 0x004c_7e40 => 0,
        _ => return Err(Intro2FlyerWorldBlock::UnsupportedEffectiveFlags),
    };
    let flags = default & !disable;
    // Both authored flyer types use gravity and mode-zero drag, with no
    // terrain attitude, extra mover, Sub-C water spring or attachment route.
    if flags != 8 {
        return Err(Intro2FlyerWorldBlock::UnsupportedEffectiveFlags);
    }
    Ok(flags)
}

fn motion_bits(entity: &Entity) -> Result<u32, Intro2FlyerWorldBlock> {
    match entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    {
        RetailRuntimeValue::Known(bits) => Ok(bits),
        RetailRuntimeValue::Unresolved => Err(Intro2FlyerWorldBlock::MasterMotionStateUnavailable),
    }
}

fn preflight_suffix(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
    wind_mode: u32,
) -> Result<(), Intro2FlyerWorldBlock> {
    authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Intro2FlyerWorldBlock::UnsupportedMetadata)?;
    // 1BEB0 writes a Hive child's source handle to+60 after104B0 returns.
    // It is not the+80 attachment route. With state1000 already excluded,
    // 12DA0's admitted flight suffix does not dereference this source word.
    let relation_supported = match entity
        .intro2_flyer_frame_owner
        .map(|owner| owner.birth_provenance)
    {
        Some(super::FlyerBirthProvenance::NativeFlyer(_)) => {
            matches!(
                entity.collision.recent_relation_id_at_0x60,
                RetailRuntimeValue::Known(_)
            )
        }
        _ => entity.collision.recent_relation_id_at_0x60 == RetailRuntimeValue::Known(None),
    };
    if entity.attached_to.is_some() || !relation_supported {
        return Err(Intro2FlyerWorldBlock::UnsupportedRelation);
    }
    // DCA0's sound gate reads the +94/+96/+A0 words; E370's surface words
    // must stay zero.
    if !matches!(metadata.common_world_effects, RetailRuntimeValue::Known(effects)
            if CommonWorldEffectProfile {
                low_health_effect_words: [0; 3],
                ..effects
            } == CommonWorldEffectProfile::default())
        || metadata.constructor_sound_attachment_id
            != RetailRuntimeValue::Known(flyer_attachment_sound(entity.entity_type))
        || metadata.sub_c_lift_descriptor != RetailRuntimeValue::Known(None)
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || entity.actor_animation_runtime != RetailRuntimeValue::Known(None)
        || entity.sub_j_attachment_runtime != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2FlyerWorldBlock::UnsupportedMetadata);
    }
    effective_flags(entity)?;
    if wind_mode != 0 {
        return Err(Intro2FlyerWorldBlock::UnsupportedWindMode { actual: wind_mode });
    }
    let RetailRuntimeValue::Known(disabled) = entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)
    else {
        return Err(Intro2FlyerWorldBlock::SurfaceStateUnavailable);
    };
    if disabled == 0 && entity.surface_lifetime_timer_ms_at_0x48 == RetailRuntimeValue::Unresolved {
        return Err(Intro2FlyerWorldBlock::SurfaceTimerUnavailable);
    }
    motion_bits(entity)?;
    Ok(())
}

pub(super) fn tick_world(
    manager: &mut EntityManager,
    owner: Intro2FlyerSchedulerOwner,
    frame: Intro2FlyerFrame<'_>,
    world_fx: &mut WorldFx,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2FlyerSchedulerOwnerTick {
    if let Some(reason) = owner.parked {
        return blocked(owner, reason);
    }
    let entity_id = owner.entity_id;
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2FlyerSchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    if !super::flyer_manager_identity_authenticates(manager, entity_id)
        || entity
            .intro2_flyer_frame_owner
            .map(|frame_owner| frame_owner.birth_provenance)
            != Some(owner.birth_provenance)
    {
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2FlyerSchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let RetailRuntimeValue::Known(state) = entity.collision.state_flags_at_0x08.masked(
        REMOTE_OWNED_STATE_BIT
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            | 0x1000,
    ) else {
        return world_block(owner, Intro2FlyerWorldBlock::SchedulerStateUnavailable);
    };
    if state & REMOTE_OWNED_STATE_BIT != 0 {
        return world_block(owner, Intro2FlyerWorldBlock::RemoteOwnerUnsupported);
    }
    if state & 0x1000 != 0 {
        return world_block(owner, Intro2FlyerWorldBlock::UnsupportedRelation);
    }
    if entity.collision.constructor_sound_attachment_id_at_0x8c
        != RetailRuntimeValue::Known(flyer_attachment_sound(entity.entity_type))
    {
        return world_block(owner, Intro2FlyerWorldBlock::UnsupportedSoundAttachment);
    }
    if let Err(reason) = motion_bits(entity) {
        return world_block(owner, reason);
    }
    let callback_enabled = state & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT != 0;
    let metadata = manager.type_runtime_metadata(entity.entity_type).cloned();
    let environment = manager.intro2_type13_environment();
    // This latch precedes A800. The environment policy is read again after
    // all slots; both acquiring/pursuing styles currently retain bit8.
    let basis_flags = if callback_enabled {
        let Some(metadata) = metadata.as_ref() else {
            return blocked(owner, Intro2FlyerRuntimeBlock::MetadataMismatch);
        };
        if let Err(reason) = preflight_suffix(entity, metadata, environment.0) {
            return world_block(owner, reason);
        }
        Some(effective_flags(entity).unwrap())
    } else {
        None
    };
    let frame_inputs = if callback_enabled {
        let extent = match entity.collision.active_model_slot() {
            RetailRuntimeValue::Known(slot) => entity.model_in_slot(slot),
            RetailRuntimeValue::Unresolved => None,
        }
        .and_then(|id| frame.resources.global_model(id))
        .map(|model| model.radius);
        match (
            frame.resources.level_terrain(),
            extent,
            manager.attached_mass_for_entity_state(entity_id),
        ) {
            (Some(terrain), Some(extent), RetailRuntimeValue::Known(mass)) => {
                Some((terrain, extent, mass))
            }
            _ => return blocked(owner, Intro2FlyerRuntimeBlock::FrameUnavailable),
        }
    } else {
        None
    };
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            next_random(world_fx)
        })
    else {
        return world_block(owner, Intro2FlyerWorldBlock::SchedulerStateUnavailable);
    };
    let entity = manager.intro2_flyer_entity_mut(entity_id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us,
    } = prefix.flow
    else {
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::SchedulerWaiting { entity_id },
            retained_owner: Some(owner),
        };
    };
    if !callback_enabled {
        let motion = plan_common_master_motion(
            entity.position_raw(),
            entity.velocity_raw(),
            motion_bits(entity).unwrap(),
            callback_elapsed_us,
        );
        commit_common_scheduler_post_callback(&mut entity.collision);
        commit_common_master_motion(entity, motion);
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::CallbackDisabled {
                entity_id,
                callback_elapsed_micros: callback_elapsed_us,
            },
            retained_owner: Some(owner),
        };
    }
    let metadata = metadata.unwrap();
    let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
        metadata.mass_raw,
        entity.collision.animation_offset_at_0xb2,
    ) else {
        return park_flyer(
            owner,
            Intro2FlyerRuntimeBlock::World(Intro2FlyerWorldBlock::AnimationOffsetUnavailable),
        );
    };
    entity.mass_raw = mass;
    let (terrain, active_model_extent_raw, attached_cargo_mass) = frame_inputs.unwrap();
    let task_frame = FlyerTaskFrame {
        resources: frame.resources,
        metadata: &metadata,
        terrain,
        active_model_extent_raw,
        attached_cargo_mass,
        elapsed_micros: callback_elapsed_us,
        global_elapsed_micros: frame.global_elapsed_micros,
        retail_tick: frame.retail_tick,
        dispatch_mode: if state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
            CommonMoverDispatchMode::Normal
        } else {
            CommonMoverDispatchMode::Restricted
        },
    };
    let tick = tick_flyer_tasks(manager, owner, task_frame, world_fx, next_random);
    // Every completed visit, aimless and Type14 ones included, owns the suffix.
    let completed = matches!(
        tick.outcome,
        Intro2FlyerSchedulerProductionOutcome::B6c0Visit { .. }
            | Intro2FlyerSchedulerProductionOutcome::PursuingVisit { .. }
            | Intro2FlyerSchedulerProductionOutcome::AimlessVisit { .. }
            | Intro2FlyerSchedulerProductionOutcome::Type14Visit { .. }
    );
    if !completed {
        return tick;
    }
    let entity = manager.intro2_flyer_entity_mut(entity_id).unwrap();
    if let Err(reason) = preflight_suffix(entity, &metadata, environment.0) {
        return park_flyer(
            tick.retained_owner.unwrap(),
            Intro2FlyerRuntimeBlock::World(reason),
        );
    }
    if state & SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT != 0 {
        // DCA0's post-task sound gate; E870's coarse suffix has none.
        let (Some(record), RetailRuntimeValue::Known(health), RetailRuntimeValue::Known(visible)) = (
            frame
                .resources
                .global_entity_type(entity.entity_type as usize),
            entity.collision.health_raw,
            entity.collision.state_flags_at_0x08.masked(0x800),
        ) else {
            return park_flyer(
                tick.retained_owner.unwrap(),
                Intro2FlyerRuntimeBlock::World(Intro2FlyerWorldBlock::DetailedSoundUnavailable),
            );
        };
        if let Some(sound) = crate::actor_detailed_sound::plan_actor_detailed_sound(
            crate::actor_detailed_sound::ActorDetailedSoundFrame {
                type_record: record,
                health_raw: health,
                visible: visible != 0,
                callback_elapsed_micros: callback_elapsed_us,
            },
            &mut || next_random(world_fx),
        ) {
            world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
        }
    }
    if basis_flags.unwrap() & 0x4000 == 0 {
        let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    }
    let mut velocity = entity.velocity_raw();
    // The shared no-SubC effective8 profile has no type-specific constants.
    apply_type13_common_environment_raw(
        &mut velocity,
        callback_elapsed_us,
        entity.mass_raw,
        environment.0,
        environment.1,
    );
    entity.set_velocity_raw(velocity);
    if entity
        .collision
        .state_flags_at_0x08
        .masked(ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT)
        == RetailRuntimeValue::Known(0)
    {
        let RetailRuntimeValue::Known(timer) = entity.surface_lifetime_timer_ms_at_0x48 else {
            unreachable!()
        };
        entity.surface_lifetime_timer_ms_at_0x48 =
            RetailRuntimeValue::Known(decay_actor_surface_timer_ms(timer, callback_elapsed_us));
    }
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        motion_bits(entity).unwrap(),
        callback_elapsed_us,
    );
    commit_common_scheduler_post_callback(&mut entity.collision);
    commit_common_master_motion(entity, motion);
    tick
}
