//! Descriptor-owned motion and surface tails for 38/49/55/56/80/81/82 in FUN_00440120.
//! Model sweeps precede these tails; a queued 28720 program runs in the next
//! static-program pass, while allocations and sounds happen in this callback.

use super::*;

/// FUN_00441B70 -> FUN_00428720's fixed 4C98F8 program (time 0, opcode 11,
/// argument 1). This is a ground-program submission, not a damage packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatGroundProgramRequest {
    pub position_raw: [i16; 3],
}

/// Actual descriptor selected by 410B0 before the 40A60 allocation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatProjectileBirth {
    pub slot: usize,
    pub particle_class: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CombatProjectileProgram {
    DragonFireball,
    TurretBolt(TurretBoltPair),
    SubmergedTurretBolt(TurretBoltPair),
}

///4410B0,441C90 and43F030 preserve each authored bolt descriptor pair.
///Their callbacks are shared, but their color and damage packets are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TurretBoltPair {
    Class49,
    Class55,
    Class56,
}

impl TurretBoltPair {
    const fn airborne(self) -> u8 {
        match self {
            Self::Class49 => 49,
            Self::Class55 => 55,
            Self::Class56 => 56,
        }
    }

    pub(super) const fn submerged(self) -> u8 {
        match self {
            Self::Class49 => 80,
            Self::Class55 => 81,
            Self::Class56 => 82,
        }
    }
}

impl CombatProjectileProgram {
    pub(super) fn for_class(class: u8) -> Option<Self> {
        let descriptor = particle_descriptor(class)?;
        let (program, update, surface, cleanup) = match class {
            38 => (Self::DragonFireball, 0x0043_F350, 0x0044_1B70, 0),
            49 => (
                Self::TurretBolt(TurretBoltPair::Class49),
                0x0042_E8E0,
                0x0044_1C90,
                0,
            ),
            55 => (
                Self::TurretBolt(TurretBoltPair::Class55),
                0x0042_E8E0,
                0x0044_1C90,
                0,
            ),
            56 => (
                Self::TurretBolt(TurretBoltPair::Class56),
                0x0042_E8E0,
                0x0044_1C90,
                0,
            ),
            80 => (
                Self::SubmergedTurretBolt(TurretBoltPair::Class49),
                0x0043_F030,
                0x0044_1C90,
                0x0044_2140,
            ),
            81 => (
                Self::SubmergedTurretBolt(TurretBoltPair::Class55),
                0x0043_F030,
                0x0044_1C90,
                0x0044_2140,
            ),
            82 => (
                Self::SubmergedTurretBolt(TurretBoltPair::Class56),
                0x0043_F030,
                0x0044_1C90,
                0x0044_2140,
            ),
            _ => return None,
        };
        (descriptor.raw_byte(0x0e) == 3
            && descriptor.raw_u32(0x14) == update
            && descriptor.raw_u32(0x18) == surface
            && descriptor.raw_u32(0x2c) == cleanup)
            .then_some(program)
    }
}

/// 40120 narrows each signed product before adding it to a signed word.
/// Fractional displacement is discarded on every visit, including tiny
/// frames; a floating accumulator changes both collision and aim timing.
pub(super) fn integrate_particle_words(particle: &mut WorldParticle, elapsed_micros: u32) {
    let position = world_position_to_raw(particle.position);
    let velocity = particle
        .velocity
        .map(|v| world_velocity_component_to_raw(v) as i16);
    let scale = (elapsed_micros >> 5) as i32;
    particle.position = raw_position_to_world(std::array::from_fn(|axis| {
        position[axis].wrapping_add((i32::from(velocity[axis]) * scale >> 15) as i16)
    }));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CombatSurfaceOutcome {
    Continue,
    Consume,
    GroundProgram(CombatGroundProgramRequest),
}

fn flat_sea_raw(environment: ParticleEnvironment<'_>) -> Option<i16> {
    match environment {
        ParticleEnvironment::Dry => None,
        ParticleEnvironment::FlatWater { sea_level } => {
            Some(world_position_to_raw([0.0, sea_level, 0.0])[1])
        }
        // 410B0 and 41770 compare the authored sea word even when waves
        // are disabled. This is distinct from collision classification.
        ParticleEnvironment::Terrain(context) => Some(context.terrain.sea_level_raw()),
    }
}

impl WorldFx {
    /// The proven 4E770 particle rows for methods10/12/13/14 call410B0 before
    ///40A60. Other 410B0 input classes have distinct policies and are not
    /// admitted through this combat-projectile constructor.
    pub fn materialize_combat_projectile_410b0(
        &mut self,
        mut request: DescriptorParticleRequest,
        environment: ParticleEnvironment<'_>,
        retail_tick: u32,
    ) -> Option<CombatProjectileBirth> {
        if !matches!(request.source_class, 38 | 49 | 55 | 56) {
            return None;
        }
        if flat_sea_raw(environment).is_some_and(|sea| request.position_raw[1] <= sea) {
            match request.source_class {
                38 => {
                    request.source_class = TYPE13_PROJECTILE_UNDERWATER_PARTICLE_CLASS;
                    request.position_raw[1] = request.position_raw[1].wrapping_sub(100);
                }
                //4410F3 class0x31 selects0x50;441111 class0x37 selects0x51;
                //441102 class0x38 selects0x52, retaining green's own packet.
                49 => request.source_class = TurretBoltPair::Class49.submerged(),
                55 => request.source_class = TurretBoltPair::Class55.submerged(),
                56 => request.source_class = TurretBoltPair::Class56.submerged(),
                _ => unreachable!("admitted constructor class"),
            }
        }
        self.materialize_descriptor_particle_request(request, environment, retail_tick)
            .map(|slot| CombatProjectileBirth {
                slot,
                particle_class: request.source_class,
            })
    }

    pub(super) fn update_combat_projectile(
        &mut self,
        program: CombatProjectileProgram,
        slot: usize,
        particle: &mut WorldParticle,
        elapsed_micros: u32,
        birth: ParticleBirthContext<'_>,
    ) {
        match program {
            CombatProjectileProgram::DragonFireball => {
                if particle.water_state == 0 {
                    return;
                }
                // 40120 writes 4CB500 once per pass; F350 consumes its two
                // different shifts and executes its loop at least once.
                let budget = 5_000_000_i32 - 64 * self.frame_pacing.count_scale_q16 as i32;
                let step_micros = budget >> 5;
                let motion_scale = budget >> 10;
                let gravity = ((i64::from(step_micros) * 0x30_0000) >> 31) as i16;
                let mut remaining = elapsed_micros as i32;
                let mut trail_position = world_position_to_raw(particle.position);
                let mut velocity = particle
                    .velocity
                    .map(|v| world_velocity_component_to_raw(v) as i16);
                loop {
                    // 410B0 rejects class40 at/below flat sea; the attempt
                    // still precedes gravity and all three random draws.
                    self.emit_combat_410b0_child(
                        40,
                        trail_position,
                        particle.suppresses_impact_damage,
                        birth,
                    );
                    remaining -= step_micros;
                    velocity[1] = velocity[1].wrapping_sub(gravity);
                    particle.velocity = raw_velocity_to_world(velocity.map(i32::from));
                    self.particles.slots[slot] = Some(*particle);
                    for axis in 0..3 {
                        let jitter = (retail_random_u16(&mut self.rng_state) >> 10) as i16 - 32;
                        let motion = (i32::from(velocity[axis]) * motion_scale >> 15) as i16;
                        trail_position[axis] = trail_position[axis]
                            .wrapping_add(jitter)
                            .wrapping_add(motion);
                    }
                    if remaining <= 0 {
                        break;
                    }
                }
            }
            CombatProjectileProgram::TurretBolt(_) => (), // 42E8E0
            CombatProjectileProgram::SubmergedTurretBolt(pair) => {
                // F030 first restores the airborne descriptor when the
                // sphere is no longer fully submerged, resetting age to 0.
                if particle.water_state != 0 {
                    //43F11A restores80→49;43F0C2 restores81→55;
                    //43F067 restores82→56.
                    self.relink_combat_projectile(slot, particle, pair.airborne());
                }
                if particle.age_ticks > 5.0 {
                    particle.pending_destruction = true;
                }
                // Flag80 is consumed only by the next 40120 precheck.
            }
        }
    }

    fn relink_combat_projectile(&mut self, slot: usize, particle: &mut WorldParticle, class: u8) {
        // F030/41C90 share 42420's list operation but retain all velocity
        // words. 42420 itself has a different, zero-velocity contract.
        let velocity = particle.velocity;
        let replaced = self
            .particles
            .transition_class_in_place(slot, particle, class);
        assert!(
            replaced,
            "authenticated combat descriptor has a valid replacement"
        );
        particle.velocity = velocity;
        self.particles.slots[slot] = Some(*particle);
    }

    /// 442B40 calls the current descriptor cleanup before unlinking the
    /// physical record. Allocator recycling and bulk reset do not call it.
    pub(super) fn free_combat_particle(
        &mut self,
        slot: usize,
        birth: ParticleBirthContext<'_>,
    ) -> bool {
        if let Some(particle) = self.particles.slots[slot] {
            if matches!(
                CombatProjectileProgram::for_class(particle.source_class),
                Some(CombatProjectileProgram::SubmergedTurretBolt(_))
            ) {
                let velocity = particle
                    .velocity
                    .map(|v| world_velocity_component_to_raw(v) as i16);
                let squared = velocity.into_iter().fold(0_i32, |sum, v| {
                    sum.wrapping_add(i32::from(v) * i32::from(v))
                });
                let length = v2k_formats::fixed_math::retail_integer_sqrt(squared) as i16;
                let normalized = if length == 0 {
                    [0x1000_i16, 0, 0]
                } else {
                    velocity.map(|v| ((i32::from(v) << 12) / i32::from(length)) as i16)
                };
                let step = normalized.map(|v| (25 * i32::from(v) >> 12) as i16);
                let mut position = world_position_to_raw(particle.position);
                for _ in 0..9 {
                    for axis in 0..3 {
                        position[axis] = position[axis].wrapping_add(step[axis]);
                    }
                    self.emit_combat_410b0_child(
                        42,
                        position,
                        particle.suppresses_impact_damage,
                        birth,
                    );
                }
            }
        }
        self.particles.free(slot)
    }

    fn emit_combat_410b0_child(
        &mut self,
        class: u8,
        position: [i16; 3],
        suppressed: bool,
        birth: ParticleBirthContext<'_>,
    ) {
        let below_sea = flat_sea_raw(birth.environment).is_some_and(|sea| position[1] <= sea);
        // These are the only two 410B0 inputs used by F350 and 42140.
        let admitted = match class {
            40 => !below_sea,
            42 => below_sea,
            _ => unreachable!("combat child is an authenticated 410B0 input"),
        };
        if admitted {
            self.materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: class,
                    position_raw: position,
                    velocity_raw: [0; 3],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 0,
                        entity_type: 0,
                    }),
                    suppresses_impact_damage: suppressed,
                },
                birth.environment,
                birth.retail_tick,
            );
        }
    }

    pub(super) fn dispatch_combat_projectile_surface(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        context: TerrainCollisionContext<'_>,
        birth: ParticleBirthContext<'_>,
    ) -> CombatSurfaceOutcome {
        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface) =
                displaced_water_surface(context, particle.position, birth.retail_tick)
            {
                let material = nearest_material_code(context.terrain, particle.position);
                let selector = context.water_response_selectors[usize::from(material)];
                let result = self.apply_combat_projectile_surface(
                    slot,
                    particle,
                    selector,
                    world_position_to_raw([0.0, surface, 0.0])[1],
                    birth,
                );
                if result != CombatSurfaceOutcome::Continue {
                    return result;
                }
            }
        }
        let position = world_position_to_raw(particle.position);
        let cell_floor = i32::from(coarse_terrain_height_raw(
            context.terrain,
            position[0],
            position[2],
        ));
        let radius = i32::from(particle.collision_radius_raw as i16);
        let bottom = i32::from(position[1]) - radius;
        let touches = if i32::from(position[1]) + radius + 0x180 < cell_floor {
            true
        } else if cell_floor < bottom - 0x180 {
            false
        } else {
            bottom
                <= i32::from(bilinear_terrain_height_raw(
                    context.terrain,
                    position[0],
                    position[2],
                ))
        };
        if touches {
            let material = nearest_material_code(context.terrain, particle.position);
            return self.apply_combat_projectile_surface(
                slot,
                particle,
                context.ground_response_selectors[usize::from(material)],
                world_position_to_raw([
                    0.0,
                    coarse_particle_ground_response_y(context.terrain, particle.position),
                    0.0,
                ])[1],
                birth,
            );
        }
        CombatSurfaceOutcome::Continue
    }

    fn apply_combat_projectile_surface(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        selector: u8,
        surface_y_raw: i16,
        birth: ParticleBirthContext<'_>,
    ) -> CombatSurfaceOutcome {
        match CombatProjectileProgram::for_class(particle.source_class)
            .expect("combat surface descriptor")
        {
            CombatProjectileProgram::DragonFireball => match selector {
                7 => CombatSurfaceOutcome::Consume,
                6 => {
                    let mut position = world_position_to_raw(particle.position);
                    position[1] = surface_y_raw;
                    let below =
                        flat_sea_raw(birth.environment).is_some_and(|sea| surface_y_raw <= sea);
                    self.materialize_descriptor_particle_request(
                        DescriptorParticleRequest {
                            source_class: if below { 43 } else { 75 },
                            position_raw: position,
                            velocity_raw: [0, 200, 0],
                            owner: Some(ParticleOwnerAtBirth {
                                entity_id: 0,
                                entity_type: 0,
                            }),
                            suppresses_impact_damage: particle.suppresses_impact_damage,
                        },
                        birth.environment,
                        birth.retail_tick,
                    );
                    CombatSurfaceOutcome::Consume
                }
                _ => {
                    let mut position = world_position_to_raw(particle.position);
                    position[1] = surface_y_raw.wrapping_add(particle.collision_radius_raw as i16);
                    particle.position = raw_position_to_world(position);
                    let vertical = world_velocity_component_to_raw(particle.velocity[1]) as i16;
                    if vertical < 0 {
                        particle.velocity[1] =
                            raw_velocity_to_world([0, 1 - i32::from(vertical) / 4, 0])[1];
                    }
                    self.particles.slots[slot] = Some(*particle);
                    CombatSurfaceOutcome::GroundProgram(CombatGroundProgramRequest {
                        position_raw: position,
                    })
                }
            },
            CombatProjectileProgram::TurretBolt(pair)
            | CombatProjectileProgram::SubmergedTurretBolt(pair) => {
                if selector == 6 {
                    if particle.source_class == pair.airborne() {
                        //441D90 selects80 for49;441D38 selects81 for55;
                        //441CDD selects82 for56.
                        self.relink_combat_projectile(slot, particle, pair.submerged());
                    }
                    // 441DE1 is shared by every selector-6 path, including
                    // an already submerged descriptor with no relink.
                    particle.age_ticks = 0.0;
                    self.particles.slots[slot] = Some(*particle);
                    return CombatSurfaceOutcome::Continue;
                }
                const RESPONSE_CLASSES: [u8; 13] = [7, 8, 9, 10, 7, 11, 0, 0, 59, 10, 71, 70, 72];
                if let Some(&class) = RESPONSE_CLASSES.get(usize::from(selector)) {
                    let mut position = world_position_to_raw(particle.position);
                    position[1] = surface_y_raw;
                    let count =
                        self.frame_pacing.scaled_count_min_one(0x2000 >> 10).max(0) as usize;
                    for _ in 0..count {
                        self.direction_cursor =
                            (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
                        let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
                        // A null class still advances the 40DC0 cursor. It
                        // cannot allocate a live descriptor/priority record.
                        if class != 0 {
                            self.materialize_descriptor_particle_request(
                                DescriptorParticleRequest {
                                    source_class: class,
                                    position_raw: position,
                                    velocity_raw: [
                                        direction[0] >> 1,
                                        (direction[1] >> 2).abs(),
                                        direction[2] >> 1,
                                    ],
                                    owner: Some(ParticleOwnerAtBirth {
                                        entity_id: 0,
                                        entity_type: 0,
                                    }),
                                    suppresses_impact_damage: particle.suppresses_impact_damage,
                                },
                                birth.environment,
                                birth.retail_tick,
                            );
                        }
                    }
                    let frequency_q16 =
                        0xE000 + u32::from(retail_random_u16(&mut self.rng_state) >> 2);
                    self.ready_sounds.push(PositionalSoundEvent {
                        sound_id: 83,
                        position: particle.position,
                        frequency_q16,
                    });
                }
                CombatSurfaceOutcome::Consume
            }
        }
    }
}

#[cfg(test)]
mod tests;
