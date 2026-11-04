//! 11400/13BE0 model callbacks ->424F20 muzzle stamps ->14870/4E770 FIFO.

use super::*;
use crate::{
    ordinary_type47_shot_math::{drain_transient_with_method_row, Type47ShotMathError},
    projectile_emitter::projectile_class_row,
    resource_cache::ResourceCache,
    world_fx::{DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth},
};

pub use crate::intro2_native_ballistic_aim::NativeBallisticShotDrainOutcome as Intro2GunTurretShotDrainOutcome;

const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2GunTurretShotDrainError {
    Allocation,
    Queue,
    State,
    BodyBasis,
    ModelBoundary(&'static str),
    Muzzle(crate::intro2_projectiles::MuzzlePassError),
    MalformedRequest { index: usize },
    ShotMath(Type47ShotMathError),
}

/// Queues outlive the task which authored them. Model callbacks stamp every
/// matching selector before11400 drains in FIFO order; no callback restarts
/// the task or consumes actor RNG. A rejected particle birth still consumes
/// the corresponding command, as in the other native presentation drains.
pub fn drain_intro2_gun_turret_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    id: u32,
    resources: &ResourceCache,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2GunTurretShotDrainOutcome, Intro2GunTurretShotDrainError> {
    if !crate::intro2_gun_turret::intro2_gun_turret_manager_allocation_authenticates(manager, id) {
        return Err(Intro2GunTurretShotDrainError::Allocation);
    }
    // 11400 stamps through 13BE0 first when the queue's markers ask for it.
    crate::intro2_projectiles::stamp_unstamped_muzzles(
        manager,
        world_fx,
        resources,
        retail_tick,
        id,
    )
    .map_err(Intro2GunTurretShotDrainError::Muzzle)?;
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Intro2GunTurretShotDrainError::Allocation)?;
    let queue = entity
        .intro2_gun_turret_aim_runtime
        .as_ref()
        .ok_or(Intro2GunTurretShotDrainError::Queue)?;
    if !queue.authenticates(entity) {
        return Err(Intro2GunTurretShotDrainError::Queue);
    }
    let descriptor = queue.projectile_descriptor();
    let source_type = entity.entity_type as u8;
    if queue.queued_shot_count() == 0 {
        return Ok(Intro2GunTurretShotDrainOutcome {
            consumed_requests: 0,
            materialized_particle_classes: Vec::new(),
        });
    }
    let suppression = match entity
        .collision
        .state_flags_at_0x08
        .masked(IMPACT_SUPPRESSION_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) => bits != 0,
        RetailRuntimeValue::Unresolved => return Err(Intro2GunTurretShotDrainError::State),
    };
    let muzzles = entity.emitter_muzzles;
    // EXE4D0380: method12={leading1,speed6000,particle49,entity0}.
    // Type97's command retains its explicit4000 override during the drain.
    let method = projectile_class_row(u32::from(descriptor.projectile_method))
        .ok_or(Intro2GunTurretShotDrainError::Queue)?;
    let mut solutions = Vec::with_capacity(queue.queued_shot_count());
    let auxiliary_method = projectile_class_row(
        crate::generic_projectile_emitter::GENERIC_PROJECTILE_AUXILIARY_METHOD,
    )
    .ok_or(Intro2GunTurretShotDrainError::Queue)?;
    for (index, request) in queue.transient_shots().iter().copied().enumerate() {
        let shape = if request.auxiliary {
            descriptor.auxiliary_command != 0
                && request.projectile_method
                    == crate::generic_projectile_emitter::GENERIC_PROJECTILE_AUXILIARY_METHOD
                && request.direction_raw == [0; 3]
                && request.time_offset_raw == 0
                && request.speed_field == GenericEmitterSpeedField::IndeterminateAuxiliaryStackWord
        } else {
            request.projectile_method == u32::from(descriptor.projectile_method)
                && request.speed_field
                    == GenericEmitterSpeedField::Explicit(descriptor.speed_override_raw)
        };
        if request.source_handle != id
            || request.owner_handle != id
            || request.emitter_selector > 1
            || (request.emitter_selector != 0 && descriptor.alternate_emitter_raw == 0)
            || !shape
        {
            return Err(Intro2GunTurretShotDrainError::MalformedRequest { index });
        }
        // An unstamped offline command follows11400's explicit marker1
        // source-position branch. Missing/unsupported models on a required
        // detailed traversal remain errors instead of changing that branch.
        let position =
            muzzles[usize::from(request.emitter_selector)].unwrap_or(entity.position_raw());
        // Method 15's row {0, 1, 32, 0} adds the source's velocity to a zero
        // direction, so its unwritten speed word never reaches the shot.
        solutions.push(if request.auxiliary {
            (
                auxiliary_method.particle_class,
                crate::ordinary_type47_shot_math::Type47TransientDrainSolution {
                    position_raw: position,
                    velocity_raw: entity.velocity_raw(),
                },
            )
        } else {
            (
                method.particle_class,
                drain_transient_with_method_row(request, position, entity.velocity_raw(), method)
                    .map_err(Intro2GunTurretShotDrainError::ShotMath)?,
            )
        });
    }
    let mut outcome = Intro2GunTurretShotDrainOutcome {
        consumed_requests: 0,
        materialized_particle_classes: Vec::new(),
    };
    for (source_class, solution) in solutions {
        //40A60 cannot invoke an actor callback; the allocated source/queue
        //remains the same across this materialization and list-node removal.
        let particle_request = DescriptorParticleRequest {
            source_class,
            position_raw: solution.position_raw,
            velocity_raw: solution.velocity_raw,
            owner: Some(ParticleOwnerAtBirth {
                entity_id: id,
                entity_type: source_type,
            }),
            suppresses_impact_damage: suppression,
        };
        //4E770 method16 reaches410B0's unchanged-class50 fallthrough to
        //40A60, with no defensive-bolt underwater descriptor replacement.
        let particle_class = if descriptor.projectile_method == 16 && source_class != 32 {
            world_fx
                .materialize_descriptor_particle_request(particle_request, environment, retail_tick)
                .map(|_| method.particle_class)
        } else {
            world_fx
                .materialize_combat_projectile_410b0(particle_request, environment, retail_tick)
                .map(|birth| birth.particle_class)
        };
        if let Some(particle_class) = particle_class {
            outcome.materialized_particle_classes.push(particle_class);
        }
        let entity = manager
            .entity_mut(id)
            .ok_or(Intro2GunTurretShotDrainError::Allocation)?;
        entity
            .intro2_gun_turret_aim_runtime
            .as_mut()
            .ok_or(Intro2GunTurretShotDrainError::Queue)?
            .pop_front();
        outcome.consumed_requests += 1;
    }
    Ok(outcome)
}

/// The muzzles `FUN_00413BE0` stamps for this turret now.
#[cfg(test)]
fn resolve_muzzles(
    entity: &Entity,
    resources: &ResourceCache,
    tick: u32,
) -> Result<[Option<[i16; 3]>; 2], Intro2GunTurretShotDrainError> {
    let RetailRuntimeValue::Known(body) = entity.physical_body_basis_q31() else {
        return Err(Intro2GunTurretShotDrainError::BodyBasis);
    };
    let model_id = entity
        .model_index
        .ok_or(Intro2GunTurretShotDrainError::ModelBoundary("active model"))?;
    crate::model_tree::ActorTransformPass {
        cache: resources,
        model_id,
        vars: &entity.presentation_anim_vars(tick),
        origin_raw: entity.position_raw(),
        body_axes_q31: [body.lateral, body.up, body.forward],
        surface: None,
        retail_tick: tick,
        sub_h: None,
        emitter: Some(
            profile_for_entity(entity)
                .ok_or(Intro2GunTurretShotDrainError::Allocation)?
                .emitter(),
        ),
        fallback: None,
    }
    .run()
    .map_err(|_| Intro2GunTurretShotDrainError::ModelBoundary("transform pass"))
}

#[cfg(test)]
mod tests;
