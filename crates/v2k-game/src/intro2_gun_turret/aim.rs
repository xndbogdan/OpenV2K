//! Turret-only425160 firing, distinct from the2300/24650 Aim transaction.
//!
//! Native E owns cadence and the alternating joint selector. The queue owns
//! immutable allocation/descriptor custody and4147A0 commands; presentation
//! resolves their actual model muzzle later, never at the actor centre here.

use super::{intro2_gun_turret_allocation_authenticates, profile_for_entity};
use crate::{
    entity::{Entity, EntityManager},
    entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    generic_projectile_emitter::{GenericEmitterAppendRequest, GenericEmitterSpeedField},
    ordinary_type47_shot_math::normalize_retail_vector_q31,
    world_fx::WorldFx,
};
use std::collections::VecDeque;
use v2k_formats::collision::ProjectileEmitterDescriptor;

mod drain;
pub use drain::{
    drain_intro2_gun_turret_shots, Intro2GunTurretShotDrainError, Intro2GunTurretShotDrainOutcome,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2GunTurretAimRuntime {
    entity_id: u32,
    spawn_index: usize,
    origin: super::GunTurretConstructionOrigin,
    descriptor: ProjectileEmitterDescriptor,
    shots: VecDeque<GenericEmitterAppendRequest>,
}

impl Intro2GunTurretAimRuntime {
    pub const fn projectile_descriptor(&self) -> ProjectileEmitterDescriptor {
        self.descriptor
    }
    pub fn queued_shot_count(&self) -> usize {
        self.shots.len()
    }
    pub fn transient_shots(&self) -> &VecDeque<GenericEmitterAppendRequest> {
        &self.shots
    }
    pub(crate) fn pop_front(&mut self) -> Option<GenericEmitterAppendRequest> {
        self.shots.pop_front()
    }
    pub(crate) fn authenticates(&self, entity: &Entity) -> bool {
        intro2_gun_turret_allocation_authenticates(entity)
            && entity.id == self.entity_id
            && entity.authored_spawn_index == Some(self.spawn_index)
            && entity
                .intro2_gun_turret_runtime
                .as_ref()
                .is_some_and(|runtime| runtime.origin == self.origin)
            && profile_for_entity(entity)
                .is_some_and(|profile| self.descriptor == profile.emitter())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2GunTurretAimError {
    Allocation,
    Descriptor,
    QueueCustody,
    Runtime(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Intro2GunTurretAimTickOutcome {
    pub queued_shots: usize,
    ///425160 dispatches4147A0 failure and continues alternation/catch-up/audio.
    pub rejected_appends: usize,
}

pub struct Intro2GunTurretAimFrame<'a> {
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub notification_phase: GameplayNotificationPhase,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn tick_intro2_gun_turret_aim(
    manager: &mut EntityManager,
    id: u32,
    manual_direction_raw: Option<[i32; 3]>,
    frame: Intro2GunTurretAimFrame<'_>,
) -> Result<Intro2GunTurretAimTickOutcome, Intro2GunTurretAimError> {
    let Intro2GunTurretAimFrame {
        world_fx,
        notifications,
        notification_phase,
        global_elapsed_micros,
        retail_tick,
    } = frame;
    if !super::intro2_gun_turret_manager_allocation_authenticates(manager, id) {
        return Err(Intro2GunTurretAimError::Allocation);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Intro2GunTurretAimError::Allocation)?;
    if !super::task::graph_authenticates(entity) {
        return Err(Intro2GunTurretAimError::Allocation);
    }
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Intro2GunTurretAimError::Descriptor)?;
    let descriptor = profile_for_entity(entity)
        .ok_or(Intro2GunTurretAimError::Descriptor)?
        .emitter();
    if metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(descriptor)) {
        return Err(Intro2GunTurretAimError::Descriptor);
    }
    if entity
        .intro2_gun_turret_aim_runtime
        .as_ref()
        .is_some_and(|queue| !queue.authenticates(entity))
    {
        return Err(Intro2GunTurretAimError::QueueCustody);
    }
    let entity = manager
        .entity_mut(id)
        .ok_or(Intro2GunTurretAimError::Allocation)?;
    let spawn_index = entity
        .authored_spawn_index
        .ok_or(Intro2GunTurretAimError::Allocation)?;
    let origin = entity
        .intro2_gun_turret_runtime
        .as_ref()
        .ok_or(Intro2GunTurretAimError::Allocation)?
        .origin;
    let queue =
        entity
            .intro2_gun_turret_aim_runtime
            .get_or_insert_with(|| Intro2GunTurretAimRuntime {
                entity_id: id,
                spawn_index,
                origin,
                descriptor,
                shots: VecDeque::new(),
            });
    let runtime = entity
        .intro2_gun_turret_runtime
        .as_mut()
        .ok_or(Intro2GunTurretAimError::Allocation)?;
    let emitter = &mut runtime.sub_e_runtime;
    // This write occurs even with no target, disabled tracking, or a failed
    // alignment test. Source uses global frame delta, not callback carry.
    emitter.cadence_raw = if emitter.cadence_raw <= 0 {
        (global_elapsed_micros as i32).wrapping_neg()
    } else {
        emitter
            .cadence_raw
            .wrapping_sub(global_elapsed_micros as i32)
    };
    let Some(direction) = manual_direction_raw else {
        return Ok(Intro2GunTurretAimTickOutcome::default());
    };
    if global_elapsed_micros == 0 {
        return Err(Intro2GunTurretAimError::Runtime(
            "425160 global delta divisor",
        ));
    }
    let divisor = (descriptor.random_interval_us / global_elapsed_micros).max(1);
    if u32::from(world_fx.next_shared_retail_random_u16()) % divisor != 0 {
        return Ok(Intro2GunTurretAimTickOutcome::default());
    }
    if emitter.projectile_method == 0 {
        return Ok(Intro2GunTurretAimTickOutcome::default());
    }
    //425160 submits all three methods through4147A0. Type115's method16
    //and native Type97's method12 retain their own descriptor rows.
    if emitter.projectile_method != u32::from(descriptor.projectile_method)
        || !matches!(emitter.projectile_method, 12 | 14 | 16)
    {
        return Err(Intro2GunTurretAimError::Runtime("425160 projectile method"));
    }
    let direction = normalize_retail_vector_q31(direction);
    let mut time_offset = 0_i32;
    let mut outcome = Intro2GunTurretAimTickOutcome::default();
    loop {
        //24EE0 reverses the selector's binding index.09A80 maps authored
        //byte+19=3 to runtime+00, so selector1 writes the retained word.
        let binding_index = if emitter.emitter_selector == 0 { 1 } else { 0 };
        if let Some(binding) = emitter.joint_bindings[binding_index] {
            if binding != 3 {
                return Err(Intro2GunTurretAimError::Runtime("425160 joint binding"));
            }
            runtime.sub_e_joint_word_raw = u16::MAX;
        }
        let request = GenericEmitterAppendRequest {
            source_handle: id,
            owner_handle: id,
            projectile_method: emitter.projectile_method,
            emitter_selector: emitter.emitter_selector as u16,
            direction_raw: direction,
            time_offset_raw: time_offset,
            speed_field: GenericEmitterSpeedField::Explicit(descriptor.speed_override_raw),
            auxiliary: false,
        };
        if queue.shots.try_reserve(1).is_ok() {
            let firing_feedback = if entity.capability_flags & 8 != 0 {
                match entity.collision.state_flags_at_0x08.masked(0x0400_0000) {
                    RetailRuntimeValue::Known(bits) => bits != 0,
                    RetailRuntimeValue::Unresolved => {
                        return Err(Intro2GunTurretAimError::Runtime(
                            "4147A0 firing feedback state",
                        ))
                    }
                }
            } else {
                false
            };
            queue.shots.push_back(request);
            outcome.queued_shots += 1;
            //41483C follows successful append only.1B9D0 finds no Sub-G
            //in this E/L allocation;4568B0(9) uses the actual controller
            //phase and retains the session's deduplicated resource slot.
            if firing_feedback && notification_phase == GameplayNotificationPhase::Playing {
                notifications.queue_infected_firing_hint(retail_tick as i32);
            }
        } else {
            outcome.rejected_appends += 1;
        }
        // 4252EA toggles only with a nonzero alternate muzzle word. Type92
        // therefore keeps selector zero, even across a multi-shot catch-up.
        if descriptor.alternate_emitter_raw != 0 {
            emitter.emitter_selector = u32::from(emitter.emitter_selector == 0);
        }
        emitter.cadence_raw = emitter
            .cadence_raw
            .wrapping_add(descriptor.random_interval_us as i32);
        time_offset = time_offset.wrapping_add(descriptor.random_interval_us as i32);
        if emitter.cadence_raw > 0 {
            break;
        }
    }
    let sound = emitter.sound_id;
    if sound != 0 {
        let sound =
            u16::try_from(sound).map_err(|_| Intro2GunTurretAimError::Runtime("425160 sound"))?;
        world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests;
