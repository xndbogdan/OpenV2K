//! Turret-only 425160 firing, distinct from the 2300/24650 Aim transaction.
//!
//! Native E owns cadence and the alternating joint selector. The queue owns
//! immutable allocation/descriptor custody and 4147A0 commands; presentation
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
    /// 425160 dispatches 4147A0 failure and continues alternation/catch-up/audio.
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
    {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Intro2GunTurretAimError::Allocation)?;
        if !super::task::graph_authenticates(entity) {
            return Err(Intro2GunTurretAimError::Allocation);
        }
    }
    tick_gun_turret_aim::<ElTurret>(manager, id, manual_direction_raw, frame)
}

/// The E/L turret's own receipt, outputs and FIFO.
pub(crate) struct ElTurret;

impl super::host::GunTurretHost for ElTurret {
    fn manager_authenticates(manager: &EntityManager, id: u32) -> bool {
        super::intro2_gun_turret_manager_allocation_authenticates(manager, id)
    }
    fn allocation_authenticates(entity: &Entity) -> bool {
        intro2_gun_turret_allocation_authenticates(entity)
    }
    fn descriptor(entity: &Entity) -> Option<ProjectileEmitterDescriptor> {
        profile_for_entity(entity).map(|profile| profile.emitter())
    }
    fn l_outputs(entity: &Entity) -> Option<[i16; 2]> {
        entity
            .intro2_gun_turret_runtime
            .map(|runtime| runtime.sub_l_output_raw)
    }
    fn store_l_outputs(entity: &mut Entity, outputs: [i16; 2]) -> bool {
        let Some(runtime) = entity.intro2_gun_turret_runtime.as_mut() else {
            return false;
        };
        runtime.sub_l_output_raw = outputs;
        true
    }
    fn with_aim<R>(
        entity: &mut Entity,
        f: impl FnOnce(
            &mut dyn super::host::ShotQueue,
            &mut crate::generic_projectile_emitter::GenericEmitterRuntime,
            super::host::JointWriter<'_>,
        ) -> R,
    ) -> Option<R> {
        if entity
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .is_some_and(|queue| !queue.authenticates(entity))
        {
            return None;
        }
        let descriptor = Self::descriptor(entity)?;
        let id = entity.id;
        let spawn_index = entity.authored_spawn_index?;
        let origin = entity.intro2_gun_turret_runtime.as_ref()?.origin;
        let Entity {
            intro2_gun_turret_aim_runtime,
            intro2_gun_turret_runtime,
            ..
        } = entity;
        let queue =
            intro2_gun_turret_aim_runtime.get_or_insert_with(|| Intro2GunTurretAimRuntime {
                entity_id: id,
                spawn_index,
                origin,
                descriptor,
                shots: VecDeque::new(),
            });
        let super::Intro2GunTurretRuntime {
            sub_e_runtime,
            sub_e_joint_word_raw,
            ..
        } = intro2_gun_turret_runtime.as_mut()?;
        // The E/L bank's only bound joint word is selector 3.
        let mut joint = |binding: u64| {
            if binding != 3 {
                return false;
            }
            *sub_e_joint_word_raw = u16::MAX;
            true
        };
        Some(f(&mut queue.shots, sub_e_runtime, &mut joint))
    }
    fn gravity_lead(entity: &Entity) -> bool {
        entity
            .intro2_gun_turret_runtime
            .is_some_and(|runtime| runtime.profile.gravity_lead())
    }
}

/// `FUN_00425160` on any turret body: E's cadence, the stochastic gate, then per
/// catch-up shot `24EE0`'s joint flag, the primary `4147A0` command, the
/// method-15 auxiliary command when E `+17` is set, and the selector toggle.
pub(crate) fn tick_gun_turret_aim<H: super::host::GunTurretHost>(
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
    if !H::manager_authenticates(manager, id) {
        return Err(Intro2GunTurretAimError::Allocation);
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Intro2GunTurretAimError::Allocation)?;
    let metadata = manager
        .type_runtime_metadata(entity.entity_type)
        .ok_or(Intro2GunTurretAimError::Descriptor)?;
    let descriptor = H::descriptor(entity).ok_or(Intro2GunTurretAimError::Descriptor)?;
    if metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(descriptor)) {
        return Err(Intro2GunTurretAimError::Descriptor);
    }
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
    let position = entity.position_raw();
    let entity = manager
        .entity_mut(id)
        .ok_or(Intro2GunTurretAimError::Allocation)?;
    let result = H::with_aim(entity, |shots, emitter, joint| {
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
        // 425160 submits every method through 4147A0. Type115's method16, Type100's
        //method19 and the turrets' methods 12, 13, 14 and 18 keep their own rows.
        if emitter.projectile_method != u32::from(descriptor.projectile_method)
            || !matches!(emitter.projectile_method, 12 | 13 | 14 | 16 | 18 | 19)
        {
            return Err(Intro2GunTurretAimError::Runtime("425160 projectile method"));
        }
        let direction = normalize_retail_vector_q31(direction);
        let mut time_offset = 0_i32;
        let mut outcome = Intro2GunTurretAimTickOutcome::default();
        let mut append = |shots: &mut dyn super::host::ShotQueue,
                          request: GenericEmitterAppendRequest,
                          outcome: &mut Intro2GunTurretAimTickOutcome| {
            if shots.try_append(request) {
                outcome.queued_shots += 1;
                // 41483C follows each successful append. 1B9D0 finds no Sub-G
                //in these allocations; 4568B0(9) uses the actual controller
                //phase and retains the session's deduplicated resource slot.
                if firing_feedback && notification_phase == GameplayNotificationPhase::Playing {
                    notifications.queue_infected_firing_hint(retail_tick as i32);
                }
            } else {
                outcome.rejected_appends += 1;
            }
        };
        loop {
            // 24EE0: selector zero writes runtime+04's binding, otherwise +00.
            let binding_index = if emitter.emitter_selector == 0 { 1 } else { 0 };
            if let Some(binding) = emitter.joint_bindings[binding_index] {
                if !joint(binding) {
                    return Err(Intro2GunTurretAimError::Runtime("425160 joint binding"));
                }
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
            append(shots, request, &mut outcome);
            if descriptor.auxiliary_command != 0 {
                // The method-15 stack record: zero direction and time, the
                // same selector, and an unwritten speed word.
                append(
                    shots,
                    GenericEmitterAppendRequest {
                        projectile_method:
                            crate::generic_projectile_emitter::GENERIC_PROJECTILE_AUXILIARY_METHOD,
                        direction_raw: [0; 3],
                        time_offset_raw: 0,
                        speed_field: GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord,
                        auxiliary: true,
                        ..request
                    },
                    &mut outcome,
                );
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
            let sound = u16::try_from(sound)
                .map_err(|_| Intro2GunTurretAimError::Runtime("425160 sound"))?;
            world_fx.queue_networked_positional_sound_raw(sound, position);
        }
        Ok(outcome)
    });
    result.ok_or(Intro2GunTurretAimError::QueueCustody)?
}

#[cfg(test)]
mod tests;
