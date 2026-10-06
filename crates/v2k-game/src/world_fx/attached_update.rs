//! FUN_004425D0's follow, direct scaled damage and post-callback cascade.
//! Cached EDI allocation custody and the current physical particle record are
//! distinct: callbacks can change both before the source resumes its suffix.

use super::*;
use crate::entity_collision_state::{RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT};

#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedParticleOwnerRequest {
    pub owner_handle: u32,
    /// Particle +1B after 40120's age increment, before any shield clamp.
    pub age_byte: u8,
}

impl AttachedParticleOwnerRequest {
    pub fn mass_increment_raw(self) -> u16 {
        (256 - u16::from(self.age_byte)) / 4
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttachedParticleOwnerState {
    pub follow: ParticleAttachmentOwner,
    /// 425D0 reads +50, the pre-health buffer, not health at +30.
    pub pre_health_buffer_raw: RetailRuntimeValue<i32>,
    pub checked_damage_enabled: RetailRuntimeValue<bool>,
    pub velocity_raw: [i16; 3],
    pub allocation_owner: ParticleOwnerAtBirth,
    /// Original EDI allocation, independent of subsequent handle lookups.
    pub cached_allocation: Option<crate::main_base_abort::MainBaseAbortActorLease>,
}

#[derive(Debug, Clone)]
pub enum AttachedParticleOwnerLookup {
    Missing,
    Updated(AttachedParticleOwnerState),
    Blocked(AttachedParticleUpdateBlock),
}

/// Live B2 writer shared by Intro2 and Playing's mutable entity boundary.
/// No projected pose or caller-supplied numeric sentinel can authorize this
/// write. A missing handle is the source's ordinary kill-mark branch.
pub fn begin_attached_particle_owner_update(
    entities: &mut crate::entity::EntityManager,
    request: AttachedParticleOwnerRequest,
) -> AttachedParticleOwnerLookup {
    let cached_allocation = entities
        .main_base_abort_actor_observation(request.owner_handle)
        .map(|observation| observation.lease);
    let Some(entity) = entities.entity_mut(request.owner_handle) else {
        return AttachedParticleOwnerLookup::Missing;
    };
    let RetailRuntimeValue::Known(mass) = entity.collision.animation_offset_at_0xb2 else {
        return AttachedParticleOwnerLookup::Blocked(AttachedParticleUpdateBlock::MassUnavailable);
    };
    entity.collision.animation_offset_at_0xb2 =
        RetailRuntimeValue::Known(mass.wrapping_add(request.mass_increment_raw()));
    AttachedParticleOwnerLookup::Updated(AttachedParticleOwnerState {
        follow: ParticleAttachmentOwner::from_entity(entity),
        pre_health_buffer_raw: entity.collision.pre_health_damage_buffer_raw,
        checked_damage_enabled: entity
            .collision
            .state_flags_at_0x08
            .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT)
            .map(|bits| bits != 0),
        velocity_raw: entity.velocity_raw(),
        allocation_owner: ParticleOwnerAtBirth {
            entity_id: entity.id,
            entity_type: entity.entity_type as u8,
        },
        cached_allocation,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedParticleCascadeOwner {
    pub velocity_raw: [i16; 3],
    pub emission_owner: Option<ParticleOwnerAtBirth>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedParticleCascadeOwnerRequest {
    pub cached_allocation: crate::main_base_abort::MainBaseAbortActorLease,
    ///44288E copies the current physical particle's+14, independent of EDI.
    pub emission_owner_handle: Option<u32>,
}

/// Source reads the original EDI record after15040, even when it is dying or
/// awaiting14990. A missing/rebound allocation cannot authorize frozen velocity.
pub fn sample_attached_particle_cached_owner(
    entities: &crate::entity::EntityManager,
    request: AttachedParticleCascadeOwnerRequest,
) -> Result<AttachedParticleCascadeOwner, AttachedParticleUpdateBlock> {
    let lease = request.cached_allocation;
    let observation = entities
        .main_base_abort_actor_observation(lease.entity_id)
        .ok_or(AttachedParticleUpdateBlock::CachedOwnerAllocationMissing)?;
    if observation.lease != lease {
        return Err(AttachedParticleUpdateBlock::CachedOwnerAllocationRebound);
    }
    let entity = entities
        .iter_all()
        .find(|entity| entity.id == lease.entity_id)
        .ok_or(AttachedParticleUpdateBlock::CachedOwnerAllocationMissing)?;
    Ok(AttachedParticleCascadeOwner {
        velocity_raw: entity.velocity_raw(),
        emission_owner: request
            .emission_owner_handle
            .map(|id| ParticleOwnerAtBirth {
                entity_id: id,
                //400A60's independent3A580 lookup yields type0 for a missing handle.
                entity_type: entities
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .map_or(0, |entity| entity.entity_type as u8),
            }),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachedParticleUpdateBlock {
    OwnerHandleUnavailable,
    MutableOwnerUnavailable,
    MassUnavailable,
    FollowStateUnavailable,
    BodyBasisUnavailable,
    StaticSeaPlaneUnavailable,
    PreHealthBufferUnavailable,
    CheckedDamageAdmissionUnavailable,
    Damage(Box<crate::attached_particle_damage::AttachedParticleDamageBlock>),
    PhysicalSlotFreedDuringDamage,
    CachedOwnerAllocationUnavailable,
    CachedOwnerAllocationMissing,
    CachedOwnerAllocationRebound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedParticleDamageRequest {
    pub target_handle: u32,
    pub packet_va: u32,
    pub source_entity_type_raw: u32,
    pub current_emitter: Option<ParticleOwnerAtBirth>,
    pub ratio_numerator: u16,
    pub ratio_denominator: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedParticleUpdateDiagnostic {
    pub physical_slot: usize,
    pub particle_class: u8,
    pub owner_handle: Option<u32>,
    pub reason: AttachedParticleUpdateBlock,
    /// B2/follow/effects may already have committed before the damage boundary.
    pub committed_prefix: bool,
    pub damage_request: Option<AttachedParticleDamageRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedParticleUpdateObservation {
    pub physical_slot: usize,
    pub particle_class: u8,
    pub mass_increment_raw: u16,
    pub damage_request: AttachedParticleDamageRequest,
    pub filtered_damage_raw: i32,
    pub age_byte_after_damage: u8,
}

#[derive(Clone, Copy)]
struct AttachedUpdateVisit {
    physical_slot: usize,
    particle_class: u8,
    owner_handle: Option<u32>,
    timing: ParticleTraversalTiming,
}

struct BlockedAttachedVisit {
    diagnostic: AttachedParticleUpdateDiagnostic,
    retail_tick: u32,
}

impl AttachedUpdateVisit {
    fn blocked(
        self,
        reason: AttachedParticleUpdateBlock,
        committed_prefix: bool,
        damage_request: Option<AttachedParticleDamageRequest>,
    ) -> BlockedAttachedVisit {
        BlockedAttachedVisit {
            diagnostic: AttachedParticleUpdateDiagnostic {
                physical_slot: self.physical_slot,
                particle_class: self.particle_class,
                owner_handle: self.owner_handle,
                reason,
                committed_prefix,
                damage_request,
            },
            retail_tick: self.timing.retail_tick,
        }
    }
}

impl WorldFx {
    pub(super) fn update_attached_particle_slot(
        &mut self,
        slot: usize,
        timing: ParticleTraversalTiming,
        elapsed_ticks: u32,
        host: &mut impl ParticleTraversalHost,
        outcome: &mut ParticleUpdateOutcome,
    ) {
        let Some(mut particle) = self.particles.slots[slot] else {
            return;
        };
        let source_class = particle.source_class;
        let visit = AttachedUpdateVisit {
            physical_slot: slot,
            particle_class: source_class,
            owner_handle: particle.attached_owner_handle,
            timing,
        };
        let descriptor = particle_descriptor(source_class).expect("attached descriptor");
        debug_assert!(particle_uses_attached_follow_update(source_class));
        debug_assert_eq!(descriptor.raw_byte(0x0e), 0);
        let Some(owner_handle) = particle.attached_owner_handle else {
            self.block_attached_update(
                visit.blocked(
                    AttachedParticleUpdateBlock::OwnerHandleUnavailable,
                    false,
                    None,
                ),
                host,
                outcome,
            );
            return;
        };
        let owner_request = AttachedParticleOwnerRequest {
            owner_handle,
            age_byte: particle.retail_age_byte(),
        };
        // Release the world borrow before the real entity+B2 write.
        let owner = match host.begin_attached_update(self, owner_request) {
            AttachedParticleOwnerLookup::Missing => {
                // 44290E sets bit80. Mode0 skips contacts, so 40120 does not
                // free this record until the next visit's pre-update gate.
                particle.pending_destruction = true;
                self.particles.slots[slot] = Some(particle);
                return;
            }
            AttachedParticleOwnerLookup::Blocked(reason) => {
                self.block_attached_update(visit.blocked(reason, false, None), host, outcome);
                return;
            }
            AttachedParticleOwnerLookup::Updated(owner) => owner,
        };
        let Some(flags) = owner.follow.state_flags_at_0x08 else {
            self.block_attached_update(
                visit.blocked(
                    AttachedParticleUpdateBlock::FollowStateUnavailable,
                    true,
                    None,
                ),
                host,
                outcome,
            );
            return;
        };
        let rotated = flags & ATTACHED_FOLLOW_ROTATED_STATE_BIT != 0;
        if rotated && matches!(owner.follow.basis, ParticleAttachmentBasis::Unresolved) {
            self.block_attached_update(
                visit.blocked(
                    AttachedParticleUpdateBlock::BodyBasisUnavailable,
                    true,
                    None,
                ),
                host,
                outcome,
            );
            return;
        }
        apply_attached_owner_follow(&mut particle, RetailRuntimeValue::Known(Some(owner.follow)));
        self.particles.slots[slot] = Some(particle);
        if rotated {
            // RNG runs even for a zero-microsecond visit, and only here.
            if i32::from(self.next_shared_retail_random_u16()) * 2 < timing.elapsed_micros as i32 {
                let context = host.context();
                let position_raw = world_position_to_raw(particle.position);
                // 4417E0 compares signed Y with the authored flat sea plane,
                // independent of waves, water enablement and classification.
                let at_or_below_sea = match context.environment {
                    ParticleEnvironment::Dry => {
                        self.block_attached_update(
                            visit.blocked(
                                AttachedParticleUpdateBlock::StaticSeaPlaneUnavailable,
                                true,
                                None,
                            ),
                            host,
                            outcome,
                        );
                        return;
                    }
                    ParticleEnvironment::FlatWater { sea_level } => {
                        position_raw[1] <= (sea_level * 256.0) as i16
                    }
                    ParticleEnvironment::Terrain(terrain) => {
                        position_raw[1] <= terrain.terrain.sea_level_raw()
                    }
                };
                self.materialize_descriptor_particle_request(
                    DescriptorParticleRequest {
                        source_class: if at_or_below_sea { 42 } else { 31 },
                        position_raw,
                        velocity_raw: [0; 3],
                        owner: context.particle_emitter.current_owner,
                        suppresses_impact_damage: particle.suppresses_impact_damage,
                    },
                    context.environment,
                    timing.retail_tick,
                );
            }
        }
        // 425D0 receives classification from BEFORE its follow write. The
        // mark does not short-circuit the remaining damage/cascade phases.
        let Some(current) = self.particles.slots[slot].as_mut() else {
            return;
        };
        if particle.water_state == 0 {
            current.pending_destruction = true;
        }
        let RetailRuntimeValue::Known(buffer) = owner.pre_health_buffer_raw else {
            self.block_attached_update(
                visit.blocked(
                    AttachedParticleUpdateBlock::PreHealthBufferUnavailable,
                    true,
                    None,
                ),
                host,
                outcome,
            );
            return;
        };
        let lifetime = u16::from(descriptor.raw_byte(0x0d));
        let damage_request = AttachedParticleDamageRequest {
            target_handle: current.attached_owner_handle.unwrap_or(owner_handle),
            packet_va: descriptor.raw_u32(0x20),
            source_entity_type_raw: (-5_i32) as u32,
            current_emitter: host.context().particle_emitter.current_owner,
            ratio_numerator: if buffer > 0 {
                lifetime.wrapping_sub(u16::from(current.retail_age_byte()))
            } else {
                elapsed_ticks as u16
            },
            ratio_denominator: lifetime,
        };
        let enabled = match owner.checked_damage_enabled {
            RetailRuntimeValue::Known(enabled) => enabled,
            RetailRuntimeValue::Unresolved => {
                self.block_attached_update(
                    visit.blocked(
                        AttachedParticleUpdateBlock::CheckedDamageAdmissionUnavailable,
                        true,
                        Some(damage_request),
                    ),
                    host,
                    outcome,
                );
                return;
            }
        };
        let filtered_damage_raw = if enabled {
            match host.attached_damage(self, damage_request) {
                Ok(filtered) => filtered,
                Err(reason) => {
                    self.block_attached_update(
                        visit.blocked(
                            AttachedParticleUpdateBlock::Damage(Box::new(reason)),
                            true,
                            Some(damage_request),
                        ),
                        host,
                        outcome,
                    );
                    return;
                }
            }
        } else {
            0
        };
        // 442855 writes to the original physical ESI slot using the descriptor
        // cached at callback entry. Never restore the prefollow particle copy.
        let Some(current) = self.particles.slots[slot].as_mut() else {
            self.block_attached_update(
                visit.blocked(
                    AttachedParticleUpdateBlock::PhysicalSlotFreedDuringDamage,
                    true,
                    Some(damage_request),
                ),
                host,
                outcome,
            );
            return;
        };
        if buffer > 0 {
            current.age_ticks = f32::from(lifetime);
        }
        let current = *current;
        outcome
            .attached_updates
            .push(AttachedParticleUpdateObservation {
                physical_slot: slot,
                particle_class: source_class,
                mass_increment_raw: owner_request.mass_increment_raw(),
                damage_request,
                filtered_damage_raw,
                age_byte_after_damage: current.retail_age_byte(),
            });
        // Source subtraction is a full wrapping dword, NOT an old age byte.
        let age = u32::from(current.retail_age_byte());
        if !current.suppresses_impact_damage
            && (age.wrapping_sub(elapsed_ticks) ^ age) & 0xffff_fff0 != 0
            && self.next_shared_retail_random_u16() & 1 != 0
        {
            let cascade_owner = if enabled {
                let Some(lease) = owner.cached_allocation else {
                    self.block_attached_update(
                        visit.blocked(
                            AttachedParticleUpdateBlock::CachedOwnerAllocationUnavailable,
                            true,
                            Some(damage_request),
                        ),
                        host,
                        outcome,
                    );
                    return;
                };
                match host.attached_cascade_owner(
                    self,
                    AttachedParticleCascadeOwnerRequest {
                        cached_allocation: lease,
                        emission_owner_handle: current.owner_id,
                    },
                ) {
                    Ok(owner) => owner,
                    Err(reason) => {
                        self.block_attached_update(
                            visit.blocked(reason, true, Some(damage_request)),
                            host,
                            outcome,
                        );
                        return;
                    }
                }
            } else {
                // No actor callback ran on the authenticated zero return.
                AttachedParticleCascadeOwner {
                    velocity_raw: owner.velocity_raw,
                    emission_owner: current.owner_id.map(|id| ParticleOwnerAtBirth {
                        entity_id: id,
                        entity_type: owner.allocation_owner.entity_type,
                    }),
                }
            };
            let context = host.context();
            self.materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: match current.source_class {
                        83 => 53,
                        84 => 69,
                        _ => 39,
                    },
                    position_raw: world_position_to_raw(current.position),
                    velocity_raw: [
                        cascade_owner.velocity_raw[0] / 2,
                        cascade_owner.velocity_raw[1].min(0),
                        cascade_owner.velocity_raw[2] / 2,
                    ],
                    owner: cascade_owner.emission_owner,
                    suppresses_impact_damage: current.suppresses_impact_damage,
                },
                context.environment,
                timing.retail_tick,
            );
            // The lower-priority child may replace another physical victim;
            // attached priority6 protects this parent. Do not rewrite a stale
            // record after the allocator changes the intrusive pool.
        }
    }

    fn block_attached_update(
        &mut self,
        blocked: BlockedAttachedVisit,
        host: &impl ParticleTraversalHost,
        outcome: &mut ParticleUpdateOutcome,
    ) {
        let diagnostic = blocked.diagnostic;
        eprintln!("FUN_004425D0 attached particle blocked and retired: {diagnostic:?}");
        outcome.blocked_attached_updates.push(diagnostic.clone());
        self.free_combat_particle(
            diagnostic.physical_slot,
            ParticleBirthContext {
                environment: host.context().environment,
                retail_tick: blocked.retail_tick,
            },
        );
    }
}
