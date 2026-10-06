//! One native11AD0 visit: latch admission/model, surface, static, then active.

use super::static_objects::{
    resolve_entity_weapon_static_contact_at_entry, EntityWeaponStaticError,
    EntityWeaponStaticOutcome,
};
use super::{
    resolve_entity_weapon_surface_contact, EntityWeaponSurfaceError, EntityWeaponSurfaceOutcome,
};
use crate::{
    active_pair::ActivePairModelState,
    class49_terminal::{Class49TerminalBlock, Class49TerminalFrame, Class49WorldContext},
    entity_collision_state::RetailRuntimeValue,
    intro2_contacts::Intro2ContactFrame,
    native_actor_capture::pair::{
        resolve_native_actor_active_contacts_at_entry, CaptureFeedbackPolicy,
        NativeActorPairRequest, NativeCaptorPairOutcome, PlayingPlayerContact,
    },
    native_entity_weapons::{EntityWeaponBlock, NativeEntityWeaponOwner},
    player_active_contact::active_pair_body_from_entity,
    specialized_actor_task_production::SpecializedActorTaskWorld,
};

pub struct EntityWeaponContactFrame<'a> {
    pub common: Intro2ContactFrame<'a>,
    pub world: SpecializedActorTaskWorld<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponContactBlock {
    Owner(EntityWeaponBlock),
    Surface(EntityWeaponSurfaceError<Class49TerminalBlock>),
    Static(EntityWeaponStaticError),
    RingRegistration(
        crate::specialized_actor_task_production::SpecializedActorTaskRegistrationConflict,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityWeaponContactError {
    pub reason: EntityWeaponContactBlock,
    pub committed_prefix: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponContactOutcome {
    Ineligible,
    Applied {
        surface: EntityWeaponSurfaceOutcome,
        static_objects: EntityWeaponStaticOutcome,
        active_pairs: NativeCaptorPairOutcome,
    },
}

fn terminal_frame<'a>(
    common: &'a mut Intro2ContactFrame<'_>,
    world: &'a mut SpecializedActorTaskWorld<'_>,
) -> Class49TerminalFrame<'a> {
    let world = match world {
        SpecializedActorTaskWorld::Cinematic => Class49WorldContext::Cinematic {
            actor_tasks: common.actor_tasks,
            active_terminal_calls: Vec::new(),
        },
        SpecializedActorTaskWorld::Playing {
            player_hull,
            extra_lives,
        } => Class49WorldContext::Playing {
            scheduler: common.actor_tasks,
            player_hull,
            extra_lives: *extra_lives,
            active_terminal_calls: Vec::new(),
        },
    };
    Class49TerminalFrame {
        entities: common.entities,
        resources: common.resources,
        world_fx: common.world_fx,
        static_damage: common.static_damage,
        notifications: common.notifications,
        retail_tick: common.retail_tick,
        world,
    }
}

pub fn resolve_entity_weapon_contacts(
    mut frame: EntityWeaponContactFrame<'_>,
    owner: NativeEntityWeaponOwner,
) -> Result<EntityWeaponContactOutcome, EntityWeaponContactError> {
    let fail = |reason| EntityWeaponContactError {
        reason: EntityWeaponContactBlock::Owner(reason),
        committed_prefix: false,
    };
    let id = owner.entity_id();
    let entity = frame
        .common
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or_else(|| fail(EntityWeaponBlock::Allocation))?;
    let entry = active_pair_body_from_entity(entity, frame.common.resources);
    let RetailRuntimeValue::Known(state) = entry.collision.state_flags_at_0x08.masked(0x9000)
    else {
        return Err(fail(EntityWeaponBlock::Runtime("11AD0 entry state")));
    };
    if state != 0x8000 {
        return Ok(EntityWeaponContactOutcome::Ineligible);
    }
    match entry.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(EntityWeaponContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => {
            return Err(fail(EntityWeaponBlock::Runtime("11AD0 entry+70")))
        }
    }
    match entry.active_model {
        ActivePairModelState::Resolved(model) if model.collision_radius_raw == 0 => {
            return Ok(EntityWeaponContactOutcome::Ineligible)
        }
        ActivePairModelState::Resolved(_) => {}
        _ => return Err(fail(EntityWeaponBlock::Runtime("11AD0 entry model"))),
    }
    if frame
        .common
        .actor_tasks
        .native_weapon_owner(frame.common.entities, id)
        != Some(owner)
    {
        return Err(fail(EntityWeaponBlock::TaskChanged));
    }
    let surface = match resolve_entity_weapon_surface_contact(
        terminal_frame(&mut frame.common, &mut frame.world),
        owner,
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            if error.committed_prefix {
                frame
                    .common
                    .actor_tasks
                    .park_native_contact_prefix(frame.common.entities, id);
            }
            return Err(EntityWeaponContactError {
                committed_prefix: error.committed_prefix,
                reason: EntityWeaponContactBlock::Surface(error),
            });
        }
    };
    if let EntityWeaponSurfaceOutcome::Applied {
        ring: Some(crate::type60_exploding_ring::Type60ConstructionOutcome::ActorLinked(receipt)),
        ..
    } = &surface
    {
        if let Some(lease) = receipt.primary_task_lease() {
            if let Err(error) = frame
                .common
                .actor_tasks
                .register_type60_exploding_ring(lease)
            {
                frame
                    .common
                    .actor_tasks
                    .park_native_contact_prefix(frame.common.entities, id);
                return Err(EntityWeaponContactError {
                    committed_prefix: true,
                    reason: EntityWeaponContactBlock::RingRegistration(error.conflict),
                });
            }
        }
    }
    let static_objects = match resolve_entity_weapon_static_contact_at_entry(
        terminal_frame(&mut frame.common, &mut frame.world),
        owner,
        &entry,
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            frame
                .common
                .actor_tasks
                .park_native_contact_prefix(frame.common.entities, id);
            return Err(EntityWeaponContactError {
                committed_prefix: true,
                reason: EntityWeaponContactBlock::Static(error),
            });
        }
    };
    let (feedback, playing_player) = match &mut frame.world {
        SpecializedActorTaskWorld::Cinematic => (CaptureFeedbackPolicy::Cinematic, None),
        SpecializedActorTaskWorld::Playing {
            player_hull,
            extra_lives,
        } => (
            CaptureFeedbackPolicy::Gameplay,
            Some(PlayingPlayerContact {
                hull: player_hull,
                extra_lives: *extra_lives,
            }),
        ),
    };
    let active_pairs = resolve_native_actor_active_contacts_at_entry(
        &mut frame.common,
        NativeActorPairRequest {
            id,
            feedback,
            playing_player,
            subject_entry: Some(&entry),
        },
    );
    Ok(EntityWeaponContactOutcome::Applied {
        surface,
        static_objects,
        active_pairs,
    })
}
