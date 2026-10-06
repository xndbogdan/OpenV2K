//! One authenticated13500 body visit, with synchronous terminal dispatch.
//!
//! The host lends its current world damage/death context at the callback's
//! actual slot. It must finish410C10 before this visit resumes the common
//! environment and master-motion suffix. Contacts belong to the later11A80
//! phase and do not run here.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskSlot, ActorTaskVisit},
    common_mover::{
        environment::{apply_common_wind_drag_raw, CommonWindDrag, CommonWindDragFrame},
        sub_c::sample_sub_c_world_surface,
        type9_tail::{plan_common_master_motion, COMMON_MASTER_MOTION_REQUIRED_STATE_MASK},
    },
    entity::{
        apply_common_gravity_and_underwater_raw, commit_common_master_motion, CommonUnderwaterFrame,
    },
    entity_collision_state::RetailRuntimeValue,
    entity_scheduler::{
        commit_common_scheduler_post_callback, commit_common_scheduler_prefix,
        common_scheduler_callback_mass, plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
    },
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationAcquisitionCallbackPrefix,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationEntityRef,
    },
    resource_cache::ResourceCache,
    world_fx::{ParticleEnvironment, ParticleOwnerAtBirth, TerrainCollisionContext, WorldFx},
};
use grenade::{step_grenade_primary, GrenadePrimaryStepRequest};
use rocket::{
    apply_rocket_flight_forces_raw, emit_rocket_trail, RocketFlightRequest, RocketTrailRequest,
    WorldFxRocketTrailHost,
};
use std::num::NonZeroU16;

pub struct EntityWeaponTickFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityWeaponTickOutcome {
    Waiting,
    Advanced {
        callback_elapsed_micros: u32,
        trail_attempts: u32,
    },
    ///410C10 completed synchronously; the source's common suffix also ran.
    Terminal,
}

/// Execute only a constructor-issued owner. The terminal closure receives
/// the current allocation, resources and shared RNG at the exact callback,
/// before later slots or the common environment suffix are visited.
pub fn tick_entity_weapon<E>(
    manager: &mut EntityManager,
    owner: NativeEntityWeaponOwner,
    frame: EntityWeaponTickFrame<'_>,
    mut terminal: impl FnMut(&mut EntityManager, &mut ResourceCache, &mut WorldFx, u32) -> Result<(), E>,
) -> Result<EntityWeaponTickOutcome, EntityWeaponTickError<E>> {
    let owned_error = |block| EntityWeaponTickError::Owned {
        block,
        committed_prefix: false,
    };
    if !owner.authenticates(manager) {
        return Err(owned_error(EntityWeaponBlock::Allocation));
    }
    let id = owner.entity_id();
    let metadata = manager
        .type_runtime_metadata(owner.kind.entity_type() as u32)
        .ok_or(owned_error(EntityWeaponBlock::Metadata))?
        .clone();
    authenticate_weapon_metadata(owner.kind, &metadata).map_err(owned_error)?;
    frame
        .resources
        .level_terrain()
        .ok_or(owned_error(EntityWeaponBlock::Terrain))?;
    let level = frame
        .resources
        .level_desc()
        .ok_or(owned_error(EntityWeaponBlock::Metadata))?;
    let wind = CommonWindDrag::from_level(level, manager.common_environment_physics())
        .map_err(|reason| owned_error(EntityWeaponBlock::Runtime(reason)))?;
    let extent = frame
        .resources
        .global_model(usize::from(metadata.model_slots[0]))
        .ok_or(owned_error(EntityWeaponBlock::Metadata))?
        .radius;
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let RetailRuntimeValue::Known(flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(0x8212_1000 | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    else {
        return Err(owned_error(EntityWeaponBlock::Runtime("body state")));
    };
    if flags & 0x8010_1000 != 0 {
        return Err(owned_error(EntityWeaponBlock::Runtime(
            "body is not an ordinary free projectile",
        )));
    }
    let RetailRuntimeValue::Known(mut basis) = entity.physical_body_basis_q31() else {
        return Err(owned_error(EntityWeaponBlock::Runtime(
            "physical body matrix",
        )));
    };
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut || {
            u32::from(frame.world_fx.next_shared_retail_random_u16())
        })
    else {
        return Err(owned_error(EntityWeaponBlock::Runtime("scheduler prefix")));
    };
    let entity = manager.entity_mut(id).unwrap();
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let owned_error = |block| EntityWeaponTickError::Owned {
        block,
        committed_prefix: true,
    };
    let CommonSchedulerPrefixFlow::Continue {
        callback_elapsed_us: dt,
    } = prefix.flow
    else {
        return Ok(EntityWeaponTickOutcome::Waiting);
    };
    let mut ended = false;
    let mut trail_attempts = 0;
    if flags & 0x20000 != 0 {
        let RetailRuntimeValue::Known(mass) = common_scheduler_callback_mass(
            metadata.mass_raw,
            entity.collision.animation_offset_at_0xb2,
        ) else {
            return Err(owned_error(EntityWeaponBlock::Runtime("callback mass")));
        };
        entity.mass_raw = mass;
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            // Fresh slot read: an earlier terminal may have cleared every slot.
            let Some(task_id) = manager
                .entity_mut(id)
                .unwrap()
                .actor_tasks
                .task_in_slot(slot)
            else {
                continue;
            };
            if owner.task_ids[slot as usize] != Some(task_id) {
                return Err(owned_error(EntityWeaponBlock::TaskChanged));
            }
            let visit = ActorTaskVisit { slot, task_id };
            let (position, mut velocity, runtime, propulsion) = {
                let entity = manager.entity_mut(id).unwrap();
                (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.native_entity_weapon_runtime.unwrap(),
                    entity.sub_a_propulsion_runtime,
                )
            };
            let mut search_context = runtime.search_context;
            let mut acquisition = None;
            let mut trail = false;
            let mut terminal_after_unwind = false;
            let mut terminal_in_callback = false;
            // Every admitted callback must unwind even when a later owned
            // validation fails. Its elapsed/RNG prefix remains committed.
            let callback_result = (|| {
                let entity = manager.entity_mut(id).unwrap();
                let task_result = entity
                    .actor_tasks
                    .begin_exact_visit_with(visit, |task| {
                        match task {
                            ActorTaskRuntime::BoulderRolling(state) => {
                                let result = step_grenade_primary(
                                    state,
                                    GrenadePrimaryStepRequest {
                                        position_raw: position,
                                        velocity_raw: velocity,
                                        body_basis: basis,
                                        model_extent_raw: extent,
                                        elapsed_micros: dt,
                                        detailed: flags & 0x0200_0000 != 0,
                                        owner_transition_suppressed: flags & 0x1000 != 0,
                                    },
                                )
                                .map_err(|_| EntityWeaponBlock::Runtime("rolling model extent"))?;
                                velocity = result.velocity_raw;
                                basis = result.body_basis;
                                terminal_after_unwind = result.owner_transition.is_some();
                            }
                            ActorTaskRuntime::RocketFlight(state) => {
                                state.before_callback(dt);
                                let RetailRuntimeValue::Known(Some(propulsion)) = propulsion else {
                                    return Err(EntityWeaponBlock::Runtime("SubA runtime"));
                                };
                                let RetailRuntimeValue::Known(target_speed_raw) =
                                    propulsion.target_speed_raw()
                                else {
                                    return Err(EntityWeaponBlock::Runtime("SubA target"));
                                };
                                let (
                                    RetailRuntimeValue::Known(Some(a)),
                                    RetailRuntimeValue::Known(Some(b)),
                                    RetailRuntimeValue::Known(Some(c)),
                                ) = (
                                    metadata.sub_a_propulsion_descriptor,
                                    metadata.sub_b_lateral_descriptor,
                                    metadata.sub_c_lift_descriptor,
                                )
                                else {
                                    return Err(EntityWeaponBlock::Metadata);
                                };
                                let result = apply_rocket_flight_forces_raw(RocketFlightRequest {
                                    position_raw: position,
                                    velocity_raw: velocity,
                                    body_basis: basis,
                                    sub_a: a,
                                    sub_b: b,
                                    sub_c: c.into(),
                                    target_speed_raw,
                                    direction_multiplier: propulsion.direction_multiplier(),
                                    drive_scale_percent: propulsion.drive_scale_percent(),
                                    surface: sample_sub_c_world_surface(
                                        frame
                                            .resources
                                            .level_terrain()
                                            .ok_or(EntityWeaponBlock::Terrain)?,
                                        [position[0], position[2]],
                                        None,
                                    ),
                                    elapsed_micros: dt,
                                });
                                velocity = result.velocity_raw;
                                // SubC owns its penetration correction before the later common motion.
                                // Stored after the task-state loan ends below.
                                acquisition = Some(NativeTaskEffect::Position(result.position_raw));
                                terminal_after_unwind =
                                    state.timeout_after_unwind() && flags & 0x1000 == 0;
                            }
                            ActorTaskRuntime::GuardLocationAcquisition(state) => {
                                let context =
                                    search_context.as_mut().ok_or(EntityWeaponBlock::Graph)?;
                                let prefix = state.before_callback(
                                    context,
                                    u32::from(frame.world_fx.next_shared_retail_random_u16()),
                                );
                                acquisition = Some(NativeTaskEffect::Acquire(prefix));
                            }
                            ActorTaskRuntime::RocketTrail => {
                                trail = true;
                            }
                            _ => return Err(EntityWeaponBlock::TaskChanged),
                        }
                        Ok(())
                    })
                    .ok_or(owned_error(EntityWeaponBlock::TaskChanged))?;
                task_result.map_err(owned_error)?;
                entity.set_velocity_raw(velocity);
                entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
                entity
                    .native_entity_weapon_runtime
                    .as_mut()
                    .unwrap()
                    .search_context = search_context;
                if let Some(NativeTaskEffect::Position(position)) = acquisition {
                    entity.set_position_raw(position);
                }
                if let Some(NativeTaskEffect::Acquire(
                    GuardLocationAcquisitionCallbackPrefix::Acquire { search_context, .. },
                )) = acquisition
                {
                    let candidates = manager
                        .retail_live_order_ids()
                        .map(|candidate_id| {
                            let entity = manager
                                .iter_all()
                                .find(|entity| entity.id == candidate_id)
                                .unwrap();
                            GuardLocationEntityRef {
                                id: entity.id,
                                entity_type: entity.entity_type,
                                position_raw: entity.position_raw(),
                                state_flags_raw: entity.collision.state_flags_at_0x08,
                                capability_flags: RetailRuntimeValue::Known(
                                    entity.capability_flags,
                                ),
                                attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
                            }
                        })
                        .collect::<Vec<_>>();
                    let source = candidates
                        .iter()
                        .find(|candidate| candidate.id == id)
                        .unwrap();
                    let selection =
                        select_guard_location_candidate(GuardLocationCandidateRequest {
                            owner: *source,
                            candidates_in_intrusive_order: &candidates,
                            search_context,
                        })
                        .map_err(|_| {
                            owned_error(EntityWeaponBlock::Runtime("acquisition candidate state"))
                        })?;
                    terminal_in_callback =
                        matches!(selection, GuardLocationCandidateSelection::Selected(_))
                            && flags & 0x1000 == 0;
                }
                if trail && flags & 0x0200_0000 != 0 {
                    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
                    let collision =
                        TerrainCollisionContext::from_current_level_cache(frame.resources)
                            .ok_or(owned_error(EntityWeaponBlock::Terrain))?;
                    let outcome = emit_rocket_trail(
                        RocketTrailRequest {
                            callback_reason_raw: 0,
                            position_raw: entity.position_raw(),
                            velocity_raw: entity.velocity_raw(),
                            sea_level_raw: collision.terrain.sea_level_raw(),
                            owner: ParticleOwnerAtBirth {
                                entity_id: id,
                                entity_type: 42,
                            },
                            source_remote_bit: flags & 0x8000_0000 != 0,
                            elapsed_micros: dt,
                        },
                        &mut WorldFxRocketTrailHost {
                            fx: frame.world_fx,
                            environment: ParticleEnvironment::Terrain(collision),
                            retail_tick: frame.retail_tick,
                        },
                    );
                    trail_attempts += outcome.attempted_particles;
                    manager
                        .entity_mut(id)
                        .unwrap()
                        .native_entity_weapon_runtime
                        .as_mut()
                        .unwrap()
                        .model_effect_bits_at_0x84 |= outcome.model_effect_bits_to_set;
                }
                if terminal_in_callback {
                    terminal(manager, frame.resources, frame.world_fx, id)
                        .map_err(EntityWeaponTickError::Terminal)?;
                }
                Ok(())
            })();
            // A synchronous terminal clears the executing inner state; the
            // shared owner reclaims its outer wrapper only at this unwind.
            let survived = manager
                .entity_mut(id)
                .is_some_and(|entity| entity.actor_tasks.finish_exact_visit(visit));
            callback_result?;
            ended = terminal_in_callback;
            //401120 clears in_callback before tag9C00/+04 or timeout/+00.
            // A callback that retired its wrapper has no such suffix.
            if terminal_after_unwind && survived {
                terminal(manager, frame.resources, frame.world_fx, id)
                    .map_err(EntityWeaponTickError::Terminal)?;
                ended = true;
            }
            if ended {
                break;
            }
        }
        let entity = manager
            .entity_mut(id)
            .ok_or(owned_error(EntityWeaponBlock::Allocation))?;
        let mut velocity = entity.velocity_raw();
        let mut rotation = entity.rotation_heading_pitch_roll_raw();
        let flags = metadata
            .initializer
            .as_ref()
            .unwrap()
            .initializer_state_flags_raw;
        // DCA0/E870 rebuild13F70 after tasks and before E100 when the
        // effective0x4000 mask is clear. Both authored weapon styles allow it.
        if flags & 0x4000 == 0 {
            entity.apply_d720_euler_body_basis();
            let RetailRuntimeValue::Known(rebuilt) = entity.physical_body_basis_q31() else {
                unreachable!("13F70 just published the matrix")
            };
            basis = rebuilt;
        }
        apply_common_gravity_and_underwater_raw(
            &mut velocity,
            dt,
            CommonUnderwaterFrame {
                effective_environment_flags: flags,
                water_response_enabled: false,
                position_y_raw: entity.position_raw()[1],
                solid_or_sea_y_raw: 0,
                self_mass_raw: mass,
                attached_cargo_mass: 0,
            },
        );
        apply_common_wind_drag_raw(
            &mut velocity,
            &mut rotation,
            wind,
            CommonWindDragFrame {
                terrain: frame
                    .resources
                    .level_terrain()
                    .ok_or(owned_error(EntityWeaponBlock::Terrain))?,
                position_raw: entity.position_raw(),
                basis,
                callback_mass_raw: NonZeroU16::new(mass).unwrap(),
                elapsed_micros: dt,
            },
        );
        entity.set_velocity_raw(velocity);
        entity.set_rotation_heading_pitch_roll_raw(rotation);
    }
    let entity = manager
        .entity_mut(id)
        .ok_or(owned_error(EntityWeaponBlock::Allocation))?;
    commit_common_scheduler_post_callback(&mut entity.collision);
    let RetailRuntimeValue::Known(motion_flags) = entity
        .collision
        .state_flags_at_0x08
        .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
    else {
        return Err(owned_error(EntityWeaponBlock::Runtime(
            "master motion flags",
        )));
    };
    let motion = plan_common_master_motion(
        entity.position_raw(),
        entity.velocity_raw(),
        motion_flags,
        dt,
    );
    commit_common_master_motion(entity, motion);
    Ok(if ended {
        EntityWeaponTickOutcome::Terminal
    } else {
        EntityWeaponTickOutcome::Advanced {
            callback_elapsed_micros: dt,
            trail_attempts,
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityWeaponTickError<E> {
    Owned {
        block: EntityWeaponBlock,
        committed_prefix: bool,
    },
    Terminal(E),
}
impl<E> EntityWeaponTickError<E> {
    pub fn prefix_committed(&self) -> bool {
        match self {
            Self::Owned {
                committed_prefix, ..
            } => *committed_prefix,
            Self::Terminal(_) => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum NativeTaskEffect {
    Position([i16; 3]),
    Acquire(GuardLocationAcquisitionCallbackPrefix),
}

#[cfg(test)]
mod tests;
