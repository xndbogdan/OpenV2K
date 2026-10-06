//! Physical-slot cursor and short-lived world borrows for FUN_00440120.
//! Every admitted external impact consumes its parent. Its owned exit releases
//! terrain/model references before synchronous callbacks mutate the same world.

use super::combat_projectiles::{
    integrate_particle_words, CombatProjectileProgram, CombatSurfaceOutcome,
};
use super::virus_projectile::{apply_virus_projectile_gravity, uses_virus_projectile_callbacks};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleTraversalTiming {
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Clone, Copy)]
pub struct ParticleTraversalContext<'a> {
    pub environment: ParticleEnvironment<'a>,
    pub callbacks: ParticleCallbackContext<'a>,
    pub particle_emitter: ParticleEmitterContext,
}

/// Terrain-side effects delivered in physical-particle order. Ground programs
/// share the same mutable scheduler as static hits, but do not run a hit filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParticleTerrainEvent {
    StaticImpact(ParticleStaticImpact),
    GroundProgram(CombatGroundProgramRequest),
}

/// The borrowed world host distinguishes a scheduler submission from an F800
/// result whose immediate terrain write must precede the next physical slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleTerrainResponse {
    Unhandled,
    GroundProgramAccepted,
    StaticDamage(crate::static_damage::StaticDamageOutcome),
}

/// Whether the live world has taken custody of the ordered terrain writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleTerrainPublication {
    /// Keep the writes in the collision overlay for a borrowing caller to apply.
    Deferred,
    /// Later contexts now contain these writes; never overlay or replay them.
    Committed,
}

/// Context is borrowed for one physical slot, then released before an impact.
/// A host may mutate terrain, models and actor allocations synchronously; the
/// next slot observes its current context. It must not snapshot active slots.
pub trait ParticleTraversalHost {
    fn context(&self) -> ParticleTraversalContext<'_>;
    /// Called after releasing the slot's terrain borrow, before an external
    /// callback or the next slot. Committed writes remain in the outcome as
    /// history, but leave the overlay so a later callback can supersede them.
    fn publish_terrain_mutations(
        &mut self,
        _mutations: &[ParticleTerrainMutation],
    ) -> ParticleTerrainPublication {
        ParticleTerrainPublication::Deferred
    }
    fn entity_impact(&mut self, world_fx: &mut WorldFx, impact: ParticleEntityImpact);
    /// Real B2 write and full owner snapshot at 425D0's callback boundary.
    /// Collision-only projections cannot authorize mutable entity state.
    fn begin_attached_update(
        &mut self,
        _world_fx: &mut WorldFx,
        _request: AttachedParticleOwnerRequest,
    ) -> AttachedParticleOwnerLookup {
        AttachedParticleOwnerLookup::Blocked(AttachedParticleUpdateBlock::MutableOwnerUnavailable)
    }
    fn attached_damage(
        &mut self,
        _world_fx: &mut WorldFx,
        _request: AttachedParticleDamageRequest,
    ) -> Result<i32, crate::attached_particle_damage::AttachedParticleDamageBlock> {
        Err(crate::attached_particle_damage::AttachedParticleDamageBlock::MutableOwnerUnavailable)
    }
    fn attached_cascade_owner(
        &mut self,
        _world_fx: &mut WorldFx,
        _request: AttachedParticleCascadeOwnerRequest,
    ) -> Result<AttachedParticleCascadeOwner, AttachedParticleUpdateBlock> {
        Err(AttachedParticleUpdateBlock::CachedOwnerAllocationUnavailable)
    }
    /// 442950's post-hit allocation lookup, retaining dying/ineligible actors
    /// until unlink. Its pose/capability projection authorizes an offset, not
    /// the later 425D0 B2 writer or checked-damage admission.
    /// Borrowed fixture hosts can explicitly supply their collision projection.
    fn attachment_owner(
        &self,
        id: u32,
    ) -> crate::entity_collision_state::RetailRuntimeValue<Option<ParticleAttachmentOwner>> {
        use crate::entity_collision_state::RetailRuntimeValue;
        let Some(collision) = self.context().callbacks.collision else {
            return RetailRuntimeValue::Unresolved;
        };
        RetailRuntimeValue::Known(
            collision
                .entities
                .iter()
                .find(|owner| owner.entity_id == id)
                .map(ParticleAttachmentOwner::from_collision_model),
        )
    }
    /// Mutable hosts apply the write directly and return None. A host borrowing
    /// terrain for the traversal returns the exact write for the shared overlay.
    fn static_impact(
        &mut self,
        world_fx: &mut WorldFx,
        impact: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation>;
    /// Returns true when the host took custody of the 28720 submission.
    /// Observational/borrowed hosts leave it in the update's explicit
    /// unhandled list; it is never silently discarded or replayed.
    fn ground_program_request(
        &mut self,
        _world_fx: &mut WorldFx,
        _request: CombatGroundProgramRequest,
    ) -> bool {
        false
    }
}

enum ParticleSlotExit {
    Done,
    AttachedUpdate,
    Entity {
        impact: ParticleEntityImpact,
        sound90: bool,
    },
    Static(ParticleStaticImpact),
    GroundProgram(CombatGroundProgramRequest),
}

struct BorrowedEventHost<'a, EntityHandler, TerrainHandler> {
    request: ParticleUpdateRequest<'a>,
    collision_entities: Option<Cow<'a, [EntityCollisionModel]>>,
    on_entity_impact: EntityHandler,
    on_terrain_event: TerrainHandler,
}

impl<EntityHandler, TerrainHandler> ParticleTraversalHost
    for BorrowedEventHost<'_, EntityHandler, TerrainHandler>
where
    EntityHandler: FnMut(&mut WorldFx, ParticleEntityImpact) -> ParticleCollisionCacheRefresh,
    TerrainHandler: FnMut(&mut WorldFx, ParticleTerrainEvent) -> ParticleTerrainResponse,
{
    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: self.request.environment,
            callbacks: ParticleCallbackContext {
                owner_motions: self.request.callbacks.owner_motions,
                collision: self.request.callbacks.collision.map(|collision| {
                    ParticleCollisionContext {
                        entities: self
                            .collision_entities
                            .as_deref()
                            .expect("collision adapter owns its projection"),
                        model_pool: collision.model_pool,
                    }
                }),
            },
            particle_emitter: self.request.particle_emitter,
        }
    }

    fn entity_impact(&mut self, world_fx: &mut WorldFx, impact: ParticleEntityImpact) {
        let refresh = (self.on_entity_impact)(world_fx, impact);
        apply_particle_collision_cache_refresh(&mut self.collision_entities, refresh);
    }

    fn static_impact(
        &mut self,
        world_fx: &mut WorldFx,
        impact: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        match (self.on_terrain_event)(world_fx, ParticleTerrainEvent::StaticImpact(impact)) {
            ParticleTerrainResponse::StaticDamage(
                crate::static_damage::StaticDamageOutcome::ImmediateBurn { cell, .. },
            ) => Some(ParticleTerrainMutation::ImmediateBurn { cell }),
            ParticleTerrainResponse::StaticDamage(
                crate::static_damage::StaticDamageOutcome::BurnedKind10Transition { .. },
            ) => unreachable!("the exact F800 packets cannot reach kind-10's immediate mutation"),
            _ => None,
        }
    }

    fn ground_program_request(
        &mut self,
        world_fx: &mut WorldFx,
        request: CombatGroundProgramRequest,
    ) -> bool {
        matches!(
            (self.on_terrain_event)(world_fx, ParticleTerrainEvent::GroundProgram(request)),
            ParticleTerrainResponse::GroundProgramAccepted
        )
    }
}

impl WorldFx {
    pub(super) fn update_with_borrowed_event_handlers(
        &mut self,
        request: ParticleUpdateRequest<'_>,
        on_entity_impact: impl FnMut(
            &mut WorldFx,
            ParticleEntityImpact,
        ) -> ParticleCollisionCacheRefresh,
        on_terrain_event: impl FnMut(&mut WorldFx, ParticleTerrainEvent) -> ParticleTerrainResponse,
    ) -> ParticleUpdateOutcome {
        let mut host = BorrowedEventHost {
            request,
            collision_entities: request
                .callbacks
                .collision
                .map(|collision| Cow::Borrowed(collision.entities)),
            on_entity_impact,
            on_terrain_event,
        };
        self.update_with_traversal_host(
            ParticleTraversalTiming {
                elapsed_micros: request.elapsed_micros,
                retail_tick: request.retail_tick,
            },
            &mut host,
        )
    }

    pub fn update_with_traversal_host(
        &mut self,
        timing: ParticleTraversalTiming,
        host: &mut impl ParticleTraversalHost,
    ) -> ParticleUpdateOutcome {
        let timing = ParticleTraversalTiming {
            elapsed_micros: timing.elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US),
            ..timing
        };
        let elapsed_ticks = self
            .last_retail_tick
            .map_or(0, |previous| timing.retail_tick.wrapping_sub(previous));
        self.last_retail_tick = Some(timing.retail_tick);
        let mut outcome = ParticleUpdateOutcome::default();
        let mut committed_terrain_mutations = Vec::new();
        // Do not snapshot active slots: callback allocations in a later slot
        // receive this same traversal, while earlier slots wait until next time.
        for slot in 0..MAX_WORLD_PARTICLES {
            let exit = self.advance_particle_slot(
                slot,
                timing,
                elapsed_ticks,
                host.context(),
                &mut outcome,
            );
            publish_terrain_mutations(host, &mut outcome, &mut committed_terrain_mutations);
            match exit {
                ParticleSlotExit::Done => (),
                ParticleSlotExit::AttachedUpdate => {
                    self.update_attached_particle_slot(
                        slot,
                        timing,
                        elapsed_ticks,
                        host,
                        &mut outcome,
                    );
                }
                ParticleSlotExit::Entity { impact, sound90 } => {
                    host.entity_impact(self, impact);
                    if particle_uses_static_route_entity_hit(impact.source_particle_class) {
                        // 442950: the damage callback runs before F610, then
                        // the current physical record chooses the attachment.
                        // Callback allocations may have recycled this address.
                        if let Some(current) = self.particles.slots[slot] {
                            let context = host.context();
                            self.materialize_common_hit_burst(
                                current.position,
                                current.suppresses_impact_damage,
                                ParticleBirthContext {
                                    environment: context.environment,
                                    retail_tick: timing.retail_tick,
                                },
                            );
                        }
                        if let Some(current) = self.particles.slots[slot] {
                            if !current.suppresses_impact_damage {
                                if let crate::entity_collision_state::RetailRuntimeValue::Known(
                                    Some(target),
                                ) = host.attachment_owner(impact.target_entity_id)
                                {
                                    let emission = AttachedStaticEmission {
                                        target_handle: target.entity_id,
                                        position_world: current.position,
                                        offset_raw: attached_impact_offset_raw(
                                            world_position_to_raw(current.position),
                                            target.position_raw,
                                            target.capability_flags_at_0x64,
                                        ),
                                        particle_class: match current.source_class {
                                            52 => 83,
                                            68 => 84,
                                            _ => 86,
                                        },
                                    };
                                    self.emit_attached_static_particle_raw(emission);
                                }
                            }
                        }
                    }
                    if sound90 {
                        self.emit_fun_0043f6e0_hit_sound(impact.position_world);
                    }
                    // A synchronous allocation may have recycled this address.
                    // Retail frees the record now occupying the physical slot.
                    self.free_combat_particle(
                        slot,
                        ParticleBirthContext {
                            environment: host.context().environment,
                            retail_tick: timing.retail_tick,
                        },
                    );
                }
                ParticleSlotExit::Static(impact) => {
                    if let Some(write) = host.static_impact(self, impact) {
                        outcome.terrain_type_mutations.push(write);
                    }
                    self.free_combat_particle(
                        slot,
                        ParticleBirthContext {
                            environment: host.context().environment,
                            retail_tick: timing.retail_tick,
                        },
                    );
                }
                ParticleSlotExit::GroundProgram(request) => {
                    if !host.ground_program_request(self, request) {
                        outcome.unhandled_ground_programs.push(request);
                    }
                }
            }
            publish_terrain_mutations(host, &mut outcome, &mut committed_terrain_mutations);
        }
        self.process_pending_with_birth_context(Some(ParticleBirthContext {
            environment: host.context().environment,
            retail_tick: timing.retail_tick,
        }));
        committed_terrain_mutations.append(&mut outcome.terrain_type_mutations);
        outcome.terrain_type_mutations = committed_terrain_mutations;
        outcome
    }

    fn advance_particle_slot(
        &mut self,
        slot: usize,
        timing: ParticleTraversalTiming,
        elapsed_ticks: u32,
        request: ParticleTraversalContext<'_>,
        outcome: &mut ParticleUpdateOutcome,
    ) -> ParticleSlotExit {
        let elapsed_micros = timing.elapsed_micros;
        let retail_tick = timing.retail_tick;
        let terrain_context = request.environment.terrain_context();
        let sea_level = request.environment.sea_level();
        let collision_model_pool = request
            .callbacks
            .collision
            .map(|collision| collision.model_pool);
        let collision_entities = request
            .callbacks
            .collision
            .map(|collision| collision.entities);
        let dt_secs = elapsed_micros as f32 / 1_000_000.0;
        let dt_micros = elapsed_micros as i32;
        let birth_context = ParticleBirthContext {
            environment: request.environment,
            retail_tick,
        };
        let Some(mut particle) = self.particles.slots[slot] else {
            return ParticleSlotExit::Done;
        };
        let next_age = particle.age_ticks + elapsed_ticks as f32;
        // `FUN_00440120` removes a record before integration only when
        // `new_age > lifetime`; equality still receives this update and
        // renders once. A zero-lifetime record is never materialized.
        if particle.pending_destruction
            || particle.lifetime_ticks == 0
            || next_age.max(0.0).floor() > f32::from(particle.lifetime_ticks)
        {
            self.free_combat_particle(slot, birth_context);
            return ParticleSlotExit::Done;
        }
        particle.age_ticks = next_age;
        particle.step_start_position = particle.position;
        let radius = f32::from(particle.collision_radius_raw) / 256.0;
        if !particle.water_state_initialized {
            initialize_particle_water_state(&mut particle, request.environment, retail_tick);
        }
        // FUN_00440120 consumes the classification stored by allocation or
        // the preceding traversal. It does not recompute the old state at
        // `retail_tick - elapsed_ticks`, which matters for moving waves and
        // for children allocated earlier in this same physical scan.
        particle.step_start_water_state = particle.water_state;
        let combat_program = CombatProjectileProgram::for_class(particle.source_class);
        let virus_projectile = uses_virus_projectile_callbacks(particle.source_class);
        let attached_update = particle_uses_attached_follow_update(particle.source_class);
        if attached_update {
            // 40D5C's nonzero descriptor model word sets particle+1D bit40;
            // 40120 at4401E4 skips generic integration. The stored +E/+10/+12
            // words are attachment offsets, not a velocity on this branch.
            debug_assert_ne!(
                particle_descriptor(particle.source_class)
                    .unwrap()
                    .raw_u16(0x04),
                0
            );
        } else if combat_program.is_some() || virus_projectile {
            integrate_particle_words(&mut particle, elapsed_micros);
        } else {
            particle.position[0] =
                v2k_core::world::wrap(particle.position[0] + particle.velocity[0] * dt_secs);
            particle.position[1] += particle.velocity[1] * dt_secs;
            particle.position[2] =
                v2k_core::world::wrap(particle.position[2] + particle.velocity[2] * dt_secs);
        }
        particle.water_state = classify_particle_water(
            terrain_context,
            sea_level,
            particle.position,
            radius,
            retail_tick,
        );
        // The generic integrator runs before the class callback. Above
        // sea, class 0x10 then applies full gravity with no damping.
        // Class 0x12 has no equivalent callback and keeps constant
        // velocity. The short particle lifetime cannot approach the
        // original i16 velocity-wrap boundary, so this smooth conversion
        // is equivalent for the meteor burst.
        let mut keep = true;
        let mut inline_trail = None;
        if let Some(program) = combat_program {
            self.particles.slots[slot] = Some(particle);
            self.update_combat_projectile(
                program,
                slot,
                &mut particle,
                elapsed_micros,
                birth_context,
            );
        } else if virus_projectile {
            apply_virus_projectile_gravity(&mut particle, elapsed_micros);
        } else if matches!(
            particle.source_class,
            PRIMARY_BULLET_PARTICLE_CLASS | RAPID_PRIMARY_PARTICLE_CLASS
        ) {
            let fully_underwater = particle.water_state == 0;
            if fully_underwater {
                let drag_factor =
                    (1.0 - PRIMARY_BULLET_UNDERWATER_DRAG_PER_SECOND * dt_secs).max(0.0);
                for component in &mut particle.velocity {
                    *component *= drag_factor;
                }
            }
            if particle.velocity.iter().all(|component| {
                component.abs() < PRIMARY_BULLET_MIN_SPEED_COMPONENT_WORLD_PER_SECOND
            }) {
                // Retail frees the record inside FUN_0043EF30, after which
                // the outer mode-3 branch nominally continues with that
                // now-free class-0 record. Treat the free as terminal here:
                // emulating a stale intrusive-list record would be unsafe,
                // and no capture supports a visible post-free collision.
                keep = false;
            }
            if keep {
                let gravity = if fully_underwater {
                    METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.5
                } else {
                    METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2
                };
                particle.velocity[1] -= gravity * dt_secs;
            }
        } else if matches!(
            particle.source_class,
            PRIMARY_ABOVE_WATER_IMPACT_CLASS
                | STATIC_PUFF_ABOVE_WATER_CLASS
                | STATIC_FLAME_ABOVE_WATER_CLASS
        ) {
            // `FUN_0043F450` advances the process-global MSVC RNG three
            // times per callback and perturbs raw Y, X, then Z velocity.
            // These increments are deliberately frame callbacks rather
            // than dt-scaled acceleration in the retail executable.
            let y_raw = i32::from(retail_random_u16(&mut self.rng_state) % 10) - 5;
            let x_raw = i32::from(retail_random_u16(&mut self.rng_state) & 7) - 4;
            let z_raw = i32::from(retail_random_u16(&mut self.rng_state) & 3) - 2;
            let delta = raw_velocity_to_world([x_raw, y_raw, z_raw]);
            for (component, increment) in particle.velocity.iter_mut().zip(delta) {
                *component += increment;
            }
        } else if matches!(
            particle.source_class,
            PRIMARY_UNDERWATER_IMPACT_CLASS
                | STATIC_FLAME_UNDERWATER_CLASS
                | PLAYER_SUBMERGED_DOWNWASH_PARTICLE_CLASS
        ) {
            let fully_underwater = particle.water_state == 0;
            if !fully_underwater {
                // Class 42's `FUN_0043EFD0` frees the bubble as soon as it
                // intersects or rises above the water surface.
                keep = false;
            }
            if keep {
                let rise_raw = dt_micros >> 10;
                particle.velocity[1] += raw_velocity_to_world([0, rise_raw, 0])[1];
                let horizontal_drag = 1.0 - (dt_micros >> 2) as f32 / 32768.0;
                particle.velocity[0] *= horizontal_drag;
                particle.velocity[2] *= horizontal_drag;
            }
        } else if uses_downwash_debris_update_callback(particle.source_class) {
            // FUN_004423C0 applies ordinary gravity, then keeps colored
            // downwash debris on the sampled terrain with an exact
            // one-eighth vertical bounce. FUN_00440120 passes
            // `(water_state == 0)` as callback arg2 and FUN_004423C0
            // returns immediately when that value is nonzero, so this
            // policy runs only while the record is not fully submerged.
            // It remains distinct from the descriptor-wide F260 gravity
            // family.
            if particle.water_state != 0 {
                particle.velocity[1] -= METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * dt_secs;
                if let Some(context) = terrain_context {
                    let position_raw = world_position_to_raw(particle.position);
                    let surface_raw = bilinear_terrain_height_raw(
                        context.terrain,
                        position_raw[0],
                        position_raw[2],
                    );
                    if position_raw[1] < surface_raw {
                        particle.position[1] = f32::from(surface_raw) / 256.0;
                        let vertical_raw = world_velocity_component_to_raw(particle.velocity[1]);
                        particle.velocity[1] =
                            raw_velocity_to_world([0, -(vertical_raw / 8), 0])[1];
                    }
                }
            }
        } else if uses_gravity_update_callback(particle.source_class) {
            // FUN_0043F260 applies ordinary gravity above water and half
            // gravity while the particle sphere is fully submerged. Its
            // descriptor slot is independent of collision mode: class 5,
            // for example, uses this update with its distinct mode-3
            // FUN_0043E180 collision callback rather than mode-1/E230.
            let gravity = if particle.water_state == 0 {
                METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * 0.5
            } else {
                METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2
            };
            particle.velocity[1] -= gravity * dt_secs;
        } else if attached_update {
            // Mode0 has no contact tail. End the geometry borrow before B2
            // mutation, follow, inline allocator calls and damage admission.
            self.particles.slots[slot] = Some(particle);
            return ParticleSlotExit::AttachedUpdate;
        } else if uses_ballistic_trail_update_callback(particle.source_class) {
            // FUN_0043ECD0 is shared by class 16's meteor scatter,
            // class 30's common-explosion scatter, and class 58. Every
            // descriptor using it attempts one trail after ordinary
            // integration even for a zero-delta traversal. The callback
            // compares the post-integration centre to the authored flat
            // sea plane: class 31 above it, class 42 at or below it.
            let trail_class =
                if sea_level.is_some_and(|sea_level| particle.position[1] <= sea_level) {
                    BALLISTIC_UNDERWATER_TRAIL_CLASS
                } else {
                    METEOR_TRAIL_CLASS
                };
            inline_trail = Some((particle.position, trail_class));
            particle.velocity[1] -= METEOR_SCATTER_GRAVITY_WORLD_PER_SEC2 * dt_secs;
        } else if particle.source_class == METEOR_TRAIL_CLASS {
            // `FUN_0044EC60` strength-3/mass-5 drag runs after generic
            // integration: scale=(dt_us*3)/(5*8), then each raw component
            // becomes `v - ((scale*v)>>15)`. This smooth equivalent keeps
            // that callback order. The >805696-us retrace/cascade special
            // remains intentionally outside this ordinary-frame path.
            let drag_factor = (1.0 - METEOR_TRAIL_DRAG_PER_SECOND * dt_secs).max(0.0);
            for component in &mut particle.velocity {
                *component *= drag_factor;
            }
        }

        // Class 32's FUN_00442920 looks up the source handle stored in the
        // particle, then copies the entity's current velocity. Integration
        // above deliberately consumed the preceding velocity first.
        if particle.source_class == PRIMARY_MUZZLE_PARTICLE_CLASS {
            if let Some(owner_id) = particle.owner_id {
                if let Some(owner) = request
                    .callbacks
                    .owner_motions
                    .iter()
                    .find(|owner| owner.owner_id == owner_id)
                {
                    particle.velocity = owner.velocity;
                }
            }
        }

        if !keep {
            self.particles.free(slot);
            return ParticleSlotExit::Done;
        }

        // Finish the parent write before invoking the allocator. This is
        // both borrow-safe and faithful: a recycled slot must never be
        // overwritten afterward by a stale local copy.
        self.particles.slots[slot] = Some(particle);

        if let Some((position, source_class)) = inline_trail.take() {
            // ECD0 attempts its selected trail from inside the update callback,
            // before FUN_00440120 enters the descriptor's collision-mode
            // dispatch. This order matters when the fixed pool is near
            // saturation and when both children land in later physical
            // slots during the same traversal.
            let mut trail = ballistic_trail_spawn(source_class);
            trail.suppresses_impact_damage = particle.suppresses_impact_damage;
            self.materialize_particle_spawn_with_birth_context(
                position,
                trail,
                Some(birth_context),
            );
        }

        if virus_projectile
            && self.dispatch_virus_projectile_contacts(
                slot,
                &mut particle,
                request,
                retail_tick,
                &outcome.terrain_type_mutations,
            )
        {
            return ParticleSlotExit::Done;
        }

        if is_supported_primary_projectile(particle.source_class) {
            if let (Some(entities), Some(model_pool)) = (collision_entities, collision_model_pool) {
                let primary = ParticleCollisionContext {
                    entities,
                    model_pool,
                };
                let dispatch = detect_primary_collision(
                    slot,
                    &mut particle,
                    primary,
                    terrain_context,
                    retail_tick,
                    &outcome.terrain_type_mutations,
                );
                self.particles.slots[slot] = Some(particle);
                for effect in dispatch.effects {
                    self.materialize_primary_impact_effect(
                        effect,
                        particle.suppresses_impact_damage,
                        birth_context,
                    );
                }
                for impact in dispatch.impacts {
                    if let PrimaryImpact::StaticTile(mut static_impact) = impact {
                        // F800 re-reads suppression after F610. Only the
                        // unsuppressed branch resolves the then-current
                        // Section-10 cell before entering FUN_00427950.
                        if static_impact.damage.is_some() {
                            static_impact.current = resolve_particle_static_damage_snapshot(
                                terrain_context,
                                collision_model_pool,
                                static_impact.cell,
                                &outcome.terrain_type_mutations,
                            );
                        }
                        outcome
                            .primary_impacts
                            .push(PrimaryImpact::StaticTile(static_impact));
                        return ParticleSlotExit::Static(static_impact);
                    } else {
                        outcome.primary_impacts.push(impact);
                    }
                }
                if let Some(entity_impact) = dispatch.entity_impact {
                    outcome.entity_impacts.push(entity_impact);
                    debug_assert!(!dispatch.keep_particle);
                    return ParticleSlotExit::Entity {
                        impact: entity_impact,
                        sound90: false,
                    };
                }
                if !dispatch.keep_particle {
                    // The callback may synchronously recycle this address.
                    // FUN_00440120 frees whatever record occupies the
                    // physical slot after the callback returns.
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
            }
        } else if is_f6e0_primary_projectile(particle.source_class) {
            if let Some(model_pool) = collision_model_pool {
                if let Some(entities) = collision_entities {
                    let primary = ParticleCollisionContext {
                        entities,
                        model_pool,
                    };
                    if let Some(entity_impact) = detect_f6e0_entity_hit(slot, &particle, primary) {
                        self.particles.slots[slot] = Some(particle);
                        self.materialize_primary_impact_effect(
                            PrimaryImpactEffect::HitBurst(entity_impact.position_world),
                            particle.suppresses_impact_damage,
                            birth_context,
                        );
                        outcome.entity_impacts.push(entity_impact);
                        return ParticleSlotExit::Entity {
                            impact: entity_impact,
                            sound90: true,
                        };
                    }
                }
                // FUN_0043FF10 writes the refined probe, then FUN_0043F890
                // runs F610. Class 4 then runs FUN_00441850's sound-90
                // prologue and FUN_00441A50. Class 54 runs that prologue
                // and FUN_004410B0 class 0x27. FUN_004566E0 stays
                // fail-closed. Sound 62 is the F890 tail. Class 77/78
                // skip 41850.
                if let Some(context) = terrain_context {
                    if let Some(terrain_objects) = context.terrain_objects {
                        if let Some(static_hit) = retail_swept_static_tile_hit(
                            particle.step_start_position,
                            particle.position,
                            particle.collision_radius_raw,
                            context.terrain,
                            terrain_objects,
                            retail_tick,
                            model_pool,
                            &outcome.terrain_type_mutations,
                        ) {
                            particle.position = static_hit.position_world;
                            let position = static_hit.position_world;
                            let source_class = particle.source_class;
                            let owner_id = particle.owner_id;
                            let source_type = particle.source_entity_type_at_birth;
                            let suppressed = particle.suppresses_impact_damage;
                            self.particles.slots[slot] = Some(particle);
                            self.materialize_primary_impact_effect(
                                PrimaryImpactEffect::HitBurst(position),
                                suppressed,
                                birth_context,
                            );
                            if source_class == FUN_0043F6E0_CLASS_4 {
                                self.emit_fun_0044f450_hit_sound(90, position);
                                self.emit_fun_00441a50(
                                    position,
                                    FUN_00441850_CLASS_4_DEBRIS_CLASS,
                                    FUN_00441A50_CLASS_4_COUNT_BASE,
                                    owner_id,
                                    source_type,
                                    suppressed,
                                    birth_context,
                                );
                                self.emit_fun_0044f450_hit_sound(62, position);
                            } else if source_class == FUN_0043F6E0_CLASS_54 {
                                self.emit_fun_0044f450_hit_sound(90, position);
                                self.emit_fun_00441850_class_54_410b0(
                                    position,
                                    owner_id,
                                    source_type,
                                    suppressed,
                                    birth_context,
                                );
                                self.emit_fun_0044f450_hit_sound(62, position);
                            }
                            self.particles.free(slot);
                            return ParticleSlotExit::Done;
                        }
                    }
                }
            }
        } else if is_f780_primary_projectile(particle.source_class)
            || particle_uses_fun_0043f7c0_entity_hit(particle.source_class)
        {
            // F780/F7C0 omitF610. Suppression skips11250/11320 but still
            // consumes the parent. Mode3 then triesFF10 withF920/F950
            // before the E180/E1A0 surface tail.
            if let Some(model_pool) = collision_model_pool {
                if let Some(entities) = collision_entities {
                    let primary = ParticleCollisionContext {
                        entities,
                        model_pool,
                    };
                    if let Some(entity_impact) =
                        detect_model_switch_particle_entity_hit(slot, &particle, primary)
                    {
                        self.particles.slots[slot] = Some(particle);
                        if entity_impact.damage.is_some() {
                            outcome.entity_impacts.push(entity_impact);
                            return ParticleSlotExit::Entity {
                                impact: entity_impact,
                                sound90: false,
                            };
                        }
                        self.particles.free(slot);
                        return ParticleSlotExit::Done;
                    }
                }
                if let Some(context) = terrain_context {
                    if let Some(terrain_objects) = context.terrain_objects {
                        if let Some(static_hit) = retail_swept_static_tile_hit(
                            particle.step_start_position,
                            particle.position,
                            particle.collision_radius_raw,
                            context.terrain,
                            terrain_objects,
                            retail_tick,
                            model_pool,
                            &outcome.terrain_type_mutations,
                        ) {
                            particle.position = static_hit.position_world;
                            let cell = static_hit.cell;
                            let suppressed = particle.suppresses_impact_damage;
                            self.particles.slots[slot] = Some(particle);
                            if !suppressed {
                                queue_static_model_switch_infection(
                                    cell,
                                    if particle_uses_fun_0043f7c0_entity_hit(particle.source_class)
                                    {
                                        TerrainContactMode::Cleanse
                                    } else {
                                        TerrainContactMode::Infect
                                    },
                                    context.terrain,
                                    &mut outcome.terrain_type_mutations,
                                );
                            }
                            self.particles.free(slot);
                            return ParticleSlotExit::Done;
                        }
                    }
                }
            }
        }

        if let Some(program) = ProjectileModelSweepProgram::for_class(particle.source_class) {
            if let (Some(entities), Some(model_pool)) = (collision_entities, collision_model_pool) {
                let collision = ParticleCollisionContext {
                    entities,
                    model_pool,
                };
                if let Some(impact) = detect_damage_projectile_sweep(
                    program,
                    slot,
                    &mut particle,
                    ProjectileSweepFrame {
                        collision,
                        terrain_context,
                        retail_tick,
                        terrain_type_mutations: &outcome.terrain_type_mutations,
                    },
                ) {
                    self.particles.slots[slot] = Some(particle);
                    match impact {
                        BallisticSweepImpact::Entity(entity_impact) => {
                            if !particle_uses_static_route_entity_hit(particle.source_class) {
                                self.materialize_common_hit_burst(
                                    entity_impact.position_world,
                                    particle.suppresses_impact_damage,
                                    birth_context,
                                );
                            }
                            outcome.entity_impacts.push(entity_impact);
                            return ParticleSlotExit::Entity {
                                impact: entity_impact,
                                sound90: false,
                            };
                        }
                        BallisticSweepImpact::StaticTile(mut static_impact) => {
                            self.materialize_common_hit_burst(
                                static_impact.position_world,
                                particle.suppresses_impact_damage,
                                birth_context,
                            );
                            if static_impact.damage.is_some() {
                                static_impact.current = resolve_particle_static_damage_snapshot(
                                    terrain_context,
                                    collision_model_pool,
                                    static_impact.cell,
                                    &outcome.terrain_type_mutations,
                                );
                            }
                            outcome.ballistic_static_impacts.push(static_impact);
                            return ParticleSlotExit::Static(static_impact);
                        }
                        BallisticSweepImpact::ConsumedStaticNoOp { .. } => {
                            // `FUN_0042E8E0`: the parent is already stored
                            // above; free it with no burst, damage, or program.
                            self.particles.free(slot);
                            return ParticleSlotExit::Done;
                        }
                    }
                }
            }
        }

        if uses_class_87_sweep_callbacks(particle.source_class) {
            if let (Some(entities), Some(model_pool)) = (collision_entities, collision_model_pool) {
                let collision = ParticleCollisionContext {
                    entities,
                    model_pool,
                };
                if let Some(impact) = detect_class_87_sweep(
                    slot,
                    &mut particle,
                    collision,
                    terrain_context,
                    retail_tick,
                    &outcome.terrain_type_mutations,
                ) {
                    self.particles.slots[slot] = Some(particle);
                    match impact {
                        Class87SweepImpact::Entity(entity_impact) => {
                            self.materialize_common_hit_burst(
                                entity_impact.position_world,
                                particle.suppresses_impact_damage,
                                birth_context,
                            );
                            outcome.entity_impacts.push(entity_impact);
                            return ParticleSlotExit::Entity {
                                impact: entity_impact,
                                sound90: false,
                            };
                        }
                        Class87SweepImpact::StaticTile(static_impact) => {
                            outcome.class_87_static_impacts.push(static_impact);
                        }
                    }
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
            }
        }

        if CombatProjectileProgram::for_class(particle.source_class).is_some() {
            if let Some(context) = terrain_context {
                let result = self.dispatch_combat_projectile_surface(
                    slot,
                    &mut particle,
                    context,
                    birth_context,
                );
                self.particles.slots[slot] = Some(particle);
                match result {
                    CombatSurfaceOutcome::Continue => (),
                    CombatSurfaceOutcome::Consume => {
                        self.free_combat_particle(slot, birth_context);
                        return ParticleSlotExit::Done;
                    }
                    CombatSurfaceOutcome::GroundProgram(request) => {
                        return ParticleSlotExit::GroundProgram(request)
                    }
                }
            }
        }

        if uses_ballistic_surface_callback(particle.source_class) {
            if let Some(context) = terrain_context {
                if !self.dispatch_ballistic_surface_collision(
                    slot,
                    &mut particle,
                    context,
                    birth_context,
                ) {
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
                self.particles.slots[slot] = Some(particle);
            }
        }

        if uses_projectile_surface_callback(particle.source_class) {
            if let Some(context) = terrain_context {
                if !self.dispatch_projectile_surface_collision(
                    slot,
                    &mut particle,
                    context,
                    birth_context,
                    request.particle_emitter,
                ) {
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
                self.particles.slots[slot] = Some(particle);
            }
        }

        if let Some(terrain_contact_mode) = terrain_contact_mode_for_particle(particle.source_class)
        {
            if let Some(context) = terrain_context {
                if !Self::dispatch_terrain_contact_surface_collision(
                    &mut particle,
                    terrain_contact_mode,
                    context,
                    retail_tick,
                    &mut outcome.terrain_type_mutations,
                ) {
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
                self.particles.slots[slot] = Some(particle);
            }
        }

        if uses_player_surface_probe_callback(particle.source_class) {
            if let Some(context) = terrain_context {
                if !self.dispatch_player_surface_probe_collision(&particle, context, birth_context)
                {
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
            }
        }

        if uses_mode_one_surface_callback(particle.source_class) {
            if let Some(context) = terrain_context {
                if !self.dispatch_mode_one_surface_collision(
                    slot,
                    &mut particle,
                    context,
                    birth_context,
                ) {
                    // E230 selector 6 returns one after its allocation
                    // attempt, and FUN_00440120 then frees this address.
                    // If same-priority recycling replaced the address, that
                    // replacement is deliberately what retail frees.
                    self.particles.free(slot);
                    return ParticleSlotExit::Done;
                }
                self.particles.slots[slot] = Some(particle);
            }
        }
        ParticleSlotExit::Done
    }
}

fn publish_terrain_mutations(
    host: &mut impl ParticleTraversalHost,
    outcome: &mut ParticleUpdateOutcome,
    committed: &mut Vec<ParticleTerrainMutation>,
) {
    if !outcome.terrain_type_mutations.is_empty()
        && host.publish_terrain_mutations(&outcome.terrain_type_mutations)
            == ParticleTerrainPublication::Committed
    {
        committed.append(&mut outcome.terrain_type_mutations);
    }
}
