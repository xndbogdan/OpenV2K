//! The Autobot's Sub-E: `FUN_00424650` called straight from its visit, and
//! `FUN_00411400`'s launch of the commands it queued.
//!
//! The visit passes itself as the source and its owner (the player) as the
//! owner: each command's `+24` owner word is the player, so `FUN_00414870`
//! launches the shot owned by the player with the Autobot's type byte. The
//! descriptor's `+17` adds a method-15 auxiliary command after each shot.

use super::*;
use crate::generic_projectile_emitter::{
    GenericEmitterAction, GenericEmitterCompletion, GenericEmitterEntitySnapshot,
    GenericEmitterPoll, GenericEmitterRequest, GenericEmitterResume, GenericEmitterTransactionId,
    GenericProjectileEmitterMachine, GENERIC_PROJECTILE_AUXILIARY_METHOD,
};
use crate::ordinary_type47_shot_math::{
    drain_intro2_flyer_transient_request, evaluate_type47_aim_error,
    evaluate_type47_forward_half_space, resolve_ballistic_target_launch,
};
use crate::projectile_emitter::projectile_class_row;
use crate::world_fx::{DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth};

const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;

/// One allocation identity per handle: the emitter refreshes only the
/// source it cached at entry, and no visit outlives its allocation.
fn identity(handle: u32) -> u64 {
    u64::from(handle) | 1 << 32
}

fn snapshot(entity: &Entity) -> GenericEmitterEntitySnapshot {
    GenericEmitterEntitySnapshot {
        handle: entity.id,
        allocation_identity: identity(entity.id),
        state_flags_at_0x08: entity.collision.state_flags_at_0x08.known_value_bits(),
        position_raw: entity.position_raw(),
        velocity_raw: entity.velocity_raw(),
        orientation_raw: entity.rotation_heading_pitch_roll_raw(),
    }
}

impl EntityManager {
    /// `FUN_00424650(self, owner, target, null, Sub-E, runtime, elapsed)`.
    pub(super) fn autobot_fire(
        &mut self,
        id: u32,
        owner: u32,
        target: u32,
        dt: u32,
        world_fx: &mut WorldFx,
    ) -> Result<(), Class0ActorError> {
        let components = self.autobot_components_mut(id)?;
        let transaction = components.next_transaction;
        components.next_transaction = transaction.wrapping_add(1).max(1);
        let mut machine = GenericProjectileEmitterMachine::start(
            GenericEmitterTransactionId::new(transaction).expect("nonzero transaction"),
            GenericEmitterRequest {
                source_handle: id,
                owner_handle: owner,
                target_handle: target,
                explicit_target_position_raw: None,
                descriptor: EMITTER,
                runtime: components.sub_e,
                elapsed_us: dt,
            },
        );
        loop {
            match machine.poll() {
                GenericEmitterPoll::Action(issued) => {
                    let response =
                        self.autobot_emitter_action(id, target, issued.action, world_fx)?;
                    machine
                        .resume(issued.receipt, response)
                        .map_err(|_| Class0ActorError::Runtime("Autobot emitter protocol"))?;
                }
                GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnZero) => return Ok(()),
                GenericEmitterPoll::Complete(GenericEmitterCompletion::ReturnNonZero(_)) => {
                    return Err(Class0ActorError::Runtime("Autobot shot append failed"));
                }
                GenericEmitterPoll::Awaiting(_) | GenericEmitterPoll::Blocked(_) => {
                    return Err(Class0ActorError::Runtime("Autobot emitter blocked"));
                }
            }
        }
    }

    fn autobot_emitter_action(
        &mut self,
        id: u32,
        target: u32,
        action: GenericEmitterAction,
        world_fx: &mut WorldFx,
    ) -> Result<GenericEmitterResume, Class0ActorError> {
        let phase = action.phase();
        let live = |manager: &Self, handle: u32| {
            manager
                .iter_all()
                .find(|entity| entity.id == handle && entity.active)
                .map(snapshot)
        };
        fn source(manager: &EntityManager, id: u32) -> Result<&Entity, Class0ActorError> {
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .ok_or(Class0ActorError::Allocation)
        }
        let mismatch = Class0ActorError::Runtime("Autobot emitter action");
        Ok(match action {
            GenericEmitterAction::LookupEntity { handle, .. }
                if handle == id || handle == target =>
            {
                GenericEmitterResume::EntityLookupResolved {
                    phase,
                    entity: live(self, handle),
                }
            }
            GenericEmitterAction::RefreshCachedSource { source_handle, .. }
                if source_handle == id =>
            {
                GenericEmitterResume::CachedSourceRefreshed {
                    phase,
                    entity: live(self, id),
                }
            }
            GenericEmitterAction::CommitInitialCadence { write, .. } => {
                let sub_e = &mut self.autobot_components_mut(id)?.sub_e;
                if sub_e.cadence_raw != write.before_raw {
                    return Err(mismatch);
                }
                sub_e.cadence_raw = write.after_raw;
                GenericEmitterResume::Acknowledged { phase }
            }
            GenericEmitterAction::CommitTargetCadence { write, .. } => {
                let sub_e = &mut self.autobot_components_mut(id)?.sub_e;
                if sub_e.cadence_raw != write.cadence_before_raw {
                    return Err(mismatch);
                }
                sub_e.cadence_raw = write.cadence_after_raw;
                GenericEmitterResume::Acknowledged { phase }
            }
            GenericEmitterAction::DrawSharedRandom { .. } => GenericEmitterResume::RandomDrawn {
                phase,
                value: u32::from(world_fx.next_shared_retail_random_u16()),
            },
            // FUN_0041E4D0 and FUN_0041E930 read the matrix the last update
            // built; the visit's own turn reaches it only after the visit.
            GenericEmitterAction::EvaluateAim {
                target_position_raw,
                ..
            } => {
                let source = source(self, id)?;
                let RetailRuntimeValue::Known(basis) = source.physical_body_basis_q31() else {
                    return Err(Class0ActorError::Runtime("Autobot matrix"));
                };
                GenericEmitterResume::AimEvaluated {
                    phase,
                    error_raw: i32::from(evaluate_type47_aim_error(
                        source.position_raw(),
                        basis.lateral,
                        basis.forward,
                        target_position_raw,
                    )),
                }
            }
            GenericEmitterAction::EvaluateForwardHalfSpace {
                target_position_raw,
                ..
            } => {
                let source = source(self, id)?;
                let RetailRuntimeValue::Known(basis) = source.physical_body_basis_q31() else {
                    return Err(Class0ActorError::Runtime("Autobot matrix"));
                };
                GenericEmitterResume::ForwardHalfSpaceEvaluated {
                    phase,
                    in_forward_half_space: evaluate_type47_forward_half_space(
                        source.position_raw(),
                        basis.forward,
                        target_position_raw,
                    ),
                }
            }
            GenericEmitterAction::ResolveTargetLaunch { request, .. } => {
                let speed = projectile_class_row(request.projectile_method)
                    .ok_or(mismatch)?
                    .speed_raw as i16;
                GenericEmitterResume::TargetLaunchResolved {
                    phase,
                    solution: resolve_ballistic_target_launch(request, speed)
                        .map_err(|_| Class0ActorError::Runtime("Autobot shot launch"))?,
                }
            }
            GenericEmitterAction::AppendTransient { request, .. } => {
                self.autobot_components_mut(id)?.shots.push(request);
                GenericEmitterResume::AppendReturned {
                    phase,
                    result_raw: 0,
                }
            }
            GenericEmitterAction::PlayPositionalSound {
                sound_id,
                source_position_raw: Some(position),
                ..
            } => {
                world_fx.queue_networked_positional_sound_raw(sound_id as u16, position);
                GenericEmitterResume::Acknowledged { phase }
            }
            _ => return Err(mismatch),
        })
    }

    pub(crate) fn autobot_queued_shots(&self, id: u32) -> &[GenericEmitterAppendRequest] {
        self.native_class0_actors
            .get(&id)
            .and_then(|runtime| runtime.autobot.as_ref())
            .map_or(&[], |components| &components.shots)
    }

    /// `FUN_00411400`'s launches (`FUN_00414870`, `FUN_0044E770`): each
    /// command from its selector's muzzle stamp, else the centre. Method 1's
    /// row `{0, 4000, 1, 0}` adds the Autobot's velocity; the method-15
    /// auxiliary is that velocity alone, its class 32 dropped at or below the
    /// sea (`FUN_004410B0`).
    pub(crate) fn drain_autobot_shots(
        &mut self,
        id: u32,
        world_fx: &mut WorldFx,
        environment: ParticleEnvironment<'_>,
        retail_tick: u32,
    ) -> Result<usize, Class0ActorError> {
        let shots = std::mem::take(&mut self.autobot_components_mut(id)?.shots);
        let entity = self
            .iter_all()
            .find(|entity| entity.id == id)
            .ok_or(Class0ActorError::Allocation)?;
        let RetailRuntimeValue::Known(suppression) = entity
            .collision
            .state_flags_at_0x08
            .masked(IMPACT_SUPPRESSION_STATE_BIT)
        else {
            return Err(Class0ActorError::Runtime("Autobot shot suppression bit"));
        };
        let velocity = entity.velocity_raw();
        let muzzles = entity.emitter_muzzles;
        let centre = entity.position_raw();
        for request in &shots {
            let origin = muzzles[usize::from(request.emitter_selector)].unwrap_or(centre);
            let particle = DescriptorParticleRequest {
                source_class: 0,
                position_raw: origin,
                velocity_raw: velocity,
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: request.owner_handle,
                    entity_type: ENTITY_TYPE as u8,
                }),
                suppresses_impact_damage: suppression != 0,
            };
            if request.auxiliary {
                let class = projectile_class_row(GENERIC_PROJECTILE_AUXILIARY_METHOD)
                    .expect("method 15 is an authored row")
                    .particle_class;
                world_fx.materialize_combat_projectile_410b0(
                    DescriptorParticleRequest {
                        source_class: class,
                        ..particle
                    },
                    environment,
                    retail_tick,
                );
            } else {
                let solution = drain_intro2_flyer_transient_request(*request, origin, velocity)
                    .map_err(|_| Class0ActorError::Runtime("Autobot shot drain"))?;
                world_fx.materialize_descriptor_particle_request(
                    DescriptorParticleRequest {
                        source_class: projectile_class_row(request.projectile_method)
                            .expect("method 1 is an authored row")
                            .particle_class,
                        position_raw: solution.position_raw,
                        velocity_raw: solution.velocity_raw,
                        ..particle
                    },
                    environment,
                    retail_tick,
                );
            }
        }
        Ok(shots.len())
    }
}
