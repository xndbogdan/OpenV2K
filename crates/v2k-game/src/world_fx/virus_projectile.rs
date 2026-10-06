//! Class50's descriptor-owned 441EA0/441FA0/442070 carrier callbacks.
//! These contacts emit class5; the carrier's later callback owns infection.

use super::*;

pub(super) fn uses_virus_projectile_callbacks(class: u8) -> bool {
    particle_descriptor(class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 3
            && descriptor.raw_u32(0x14) == 0x0043_F260
            && descriptor.raw_u32(0x18) == 0x0044_1EA0
            && descriptor.raw_u32(0x1c) == 0x0044_1FA0
            && descriptor.raw_u32(0x24) == 0x0044_2070
            && descriptor.raw_u32(0x2c) == 0
    })
}

pub(super) fn apply_virus_projectile_gravity(particle: &mut WorldParticle, elapsed_micros: u32) {
    // 40120 publishes DAT_004DCEE0 before traversal; F260 subtracts its
    // signed low word, or its arithmetic half when water classification is 0.
    let gravity = ((i64::from(elapsed_micros) * 0x30_0000) >> 31) as i16;
    let gravity = if particle.water_state == 0 {
        gravity >> 1
    } else {
        gravity
    };
    let vertical =
        (world_velocity_component_to_raw(particle.velocity[1]) as i16).wrapping_sub(gravity);
    particle.velocity[1] = raw_velocity_to_world([0, i32::from(vertical), 0])[1];
}

#[derive(Clone, Copy)]
enum CarrierContact {
    Surface,
    Model,
}

impl WorldFx {
    /// Runs mode3's entity, static, water-transition, then terrain sequence.
    /// Returns true only when a contact has consumed the physical parent.
    pub(super) fn dispatch_virus_projectile_contacts(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        request: ParticleTraversalContext<'_>,
        retail_tick: u32,
        terrain_type_mutations: &[ParticleTerrainMutation],
    ) -> bool {
        let birth = ParticleBirthContext {
            environment: request.environment,
            retail_tick,
        };
        if let Some(collision) = request.callbacks.collision {
            if retail_swept_model_hit(
                particle.step_start_position,
                particle.position,
                particle.collision_radius_raw,
                particle.owner_id,
                particle.age_ticks,
                collision.entities,
                collision.model_pool,
            )
            .is_some()
            {
                self.emit_virus_projectile_carriers(
                    particle,
                    CarrierContact::Model,
                    request.particle_emitter,
                    birth,
                );
                self.free_combat_particle(slot, birth);
                return true;
            }
            if let Some(context) = request.environment.terrain_context() {
                if let Some(objects) = context.terrain_objects {
                    if let Some(hit) = retail_swept_static_tile_hit(
                        particle.step_start_position,
                        particle.position,
                        particle.collision_radius_raw,
                        context.terrain,
                        objects,
                        retail_tick,
                        collision.model_pool,
                        terrain_type_mutations,
                    ) {
                        // FF10 publishes the refined probe into +8/+A/+C
                        // before 442070 reads that current particle endpoint.
                        particle.position = hit.position_world;
                        self.particles.slots[slot] = Some(*particle);
                        self.emit_virus_projectile_carriers(
                            particle,
                            CarrierContact::Model,
                            request.particle_emitter,
                            birth,
                        );
                        // 442070 frees inside its callback; FF10's second
                        // 442B40 sees the already-free record. No static hit
                        // packet or terrain writer runs on this parent path.
                        self.free_combat_particle(slot, birth);
                        return true;
                    }
                }
            }
        }
        if let Some(context) = request.environment.terrain_context() {
            let material = nearest_material_code(context.terrain, particle.position);
            if particle.step_start_water_state == 2
                && particle.water_state < 2
                && displaced_water_surface(context, particle.position, retail_tick).is_some()
                && self.apply_virus_projectile_surface(
                    particle,
                    context.water_response_selectors[usize::from(material)],
                    request.particle_emitter,
                    birth,
                )
            {
                self.free_combat_particle(slot, birth);
                return true;
            }

            // 40120's coarse rejection precedes the signed bilinear test.
            let position = world_position_to_raw(particle.position);
            let floor = i32::from(coarse_terrain_height_raw(
                context.terrain,
                position[0],
                position[2],
            ));
            let radius = i32::from(particle.collision_radius_raw as i16);
            let bottom = i32::from(position[1]) - radius;
            let touches = if i32::from(position[1]) + radius + 0x180 < floor {
                true
            } else if floor < bottom - 0x180 {
                false
            } else {
                bottom
                    <= i32::from(bilinear_terrain_height_raw(
                        context.terrain,
                        position[0],
                        position[2],
                    ))
            };
            if touches
                && self.apply_virus_projectile_surface(
                    particle,
                    context.ground_response_selectors[usize::from(material)],
                    request.particle_emitter,
                    birth,
                )
            {
                self.free_combat_particle(slot, birth);
                return true;
            }
        }
        // Selector6 allocates nothing, so only the retained parent may be
        // written here. A consumed callback must never restore a stale copy
        // over an allocator-recycled physical slot.
        self.particles.slots[slot] = Some(*particle);
        false
    }

    fn apply_virus_projectile_surface(
        &mut self,
        particle: &mut WorldParticle,
        selector: u8,
        emitter: ParticleEmitterContext,
        birth: ParticleBirthContext<'_>,
    ) -> bool {
        if selector == 6 {
            let velocity = particle
                .velocity
                .map(|v| world_velocity_component_to_raw(v) as i16);
            particle.velocity = raw_velocity_to_world([
                i32::from(velocity[0]) / 4,
                i32::from(velocity[1]) / 2,
                i32::from(velocity[2]) / 4,
            ]);
            return false;
        }
        self.emit_virus_projectile_carriers(particle, CarrierContact::Surface, emitter, birth);
        true
    }

    fn emit_virus_projectile_carriers(
        &mut self,
        particle: &WorldParticle,
        contact: CarrierContact,
        emitter: ParticleEmitterContext,
        birth: ParticleBirthContext<'_>,
    ) {
        if particle.suppresses_impact_damage {
            return;
        }
        let scaled = (self.frame_pacing.count_scale_q16 as i32) >> 13;
        let attempts = if scaled == 0 { 1 } else { scaled };
        for _ in 0..attempts {
            self.direction_cursor = (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
            let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
            let horizontal_shift = match contact {
                CarrierContact::Surface => 0,
                CarrierContact::Model => 1,
            };
            self.materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: ALIEN_HIVE_PARTICLE_CLASS,
                    position_raw: world_position_to_raw(particle.position),
                    velocity_raw: [
                        direction[0] >> horizontal_shift,
                        (direction[1] >> 2).abs(),
                        direction[2] >> horizontal_shift,
                    ],
                    owner: emitter.current_owner,
                    suppresses_impact_damage: particle.suppresses_impact_damage,
                },
                birth.environment,
                birth.retail_tick,
            );
        }
    }
}
