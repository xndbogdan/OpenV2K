use super::*;
use v2k_formats::terrain::{TerrainCell, GRID_SIZE};

fn birth(environment: ParticleEnvironment<'_>) -> ParticleBirthContext<'_> {
    ParticleBirthContext {
        environment,
        retail_tick: 0,
    }
}

fn particle(
    fx: &mut WorldFx,
    class: u8,
    position: [i16; 3],
    velocity: [i16; 3],
    environment: ParticleEnvironment<'_>,
) -> usize {
    fx.materialize_descriptor_particle_request(
        DescriptorParticleRequest {
            source_class: class,
            position_raw: position,
            velocity_raw: velocity,
            owner: Some(ParticleOwnerAtBirth {
                entity_id: 77,
                entity_type: 10,
            }),
            suppresses_impact_damage: false,
        },
        environment,
        0,
    )
    .unwrap()
}

fn terrain(height: i8) -> TerrainGrid {
    TerrainGrid {
        header: [i32::MIN, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: height as u8,
                attribute: 0,
                terrain_type: 0
            };
            GRID_SIZE * GRID_SIZE
        ],
    }
}

fn context(terrain: &TerrainGrid) -> TerrainCollisionContext<'_> {
    TerrainCollisionContext {
        terrain,
        terrain_objects: None,
        ground_response_selectors: [0; 8],
        water_response_selectors: [6; 8],
    }
}

#[test]
fn combat_410b0_birth_selects_own_underwater_descriptor_at_sea_equality() {
    for (class, y, expected_class) in [
        (49, -1, 80),
        (49, 0, 80),
        (49, 1, 49),
        (55, -1, 81),
        (55, 0, 81),
        (55, 1, 55),
        (56, -1, 82),
        (56, 0, 82),
        (56, 1, 56),
        (38, 0, 46),
        (38, 1, 38),
    ] {
        let mut fx = WorldFx::new();
        let rng = fx.rng_state;
        let birth = fx
            .materialize_combat_projectile_410b0(
                DescriptorParticleRequest {
                    source_class: class,
                    position_raw: [100, y, 200],
                    velocity_raw: [1000, -100, 0],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 55,
                        entity_type: 102,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::FlatWater { sea_level: 0.0 },
                3,
            )
            .unwrap();
        assert_eq!(birth.particle_class, expected_class);
        let value = fx.particles.slots[birth.slot].unwrap();
        assert_eq!(
            world_position_to_raw(value.position),
            [100, y, 200],
            "class46 subtract100/add100 are separate constructors"
        );
        assert_eq!(value.owner_id, Some(55));
        assert_eq!(value.source_entity_type_at_birth, Some(102));
        assert_eq!(fx.rng_state, rng);
        if matches!(expected_class, 80 | 81 | 82) {
            assert_eq!(
                CombatProjectileProgram::for_class(value.source_class),
                Some(CombatProjectileProgram::SubmergedTurretBolt(
                    match expected_class {
                        80 => TurretBoltPair::Class49,
                        81 => TurretBoltPair::Class55,
                        82 => TurretBoltPair::Class56,
                        _ => unreachable!(),
                    }
                ))
            );
        }
    }
}

#[test]
fn combat_bolt_scope_keeps_distinct_57_89_out_of_constructor_and_callbacks() {
    for class in [57, 89] {
        let mut fx = WorldFx::new();
        assert!(fx
            .materialize_combat_projectile_410b0(
                DescriptorParticleRequest {
                    source_class: class,
                    position_raw: [0; 3],
                    velocity_raw: [0; 3],
                    owner: None,
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .is_none());
        assert!(CombatProjectileProgram::for_class(class).is_none());
        assert!(
            super::super::projectile_sweep::ProjectileModelSweepProgram::for_class(class).is_none()
        );
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(fx.rng_state, 0);
    }
}

#[test]
fn combat_motion_discards_each_fraction_and_wraps_signed_words() {
    for pair in [
        TurretBoltPair::Class49,
        TurretBoltPair::Class55,
        TurretBoltPair::Class56,
    ] {
        let mut fx = WorldFx::new();
        let slot = particle(
            &mut fx,
            pair.airborne(),
            [32760, -32760, 0],
            [32767, -32768, -1],
            ParticleEnvironment::Dry,
        );
        let mut value = fx.particles.slots[slot].unwrap();
        let original = value.position;
        integrate_particle_words(&mut value, 31);
        assert_eq!(value.position, original);
        integrate_particle_words(&mut value, 32);
        assert_eq!(world_position_to_raw(value.position), [32760, -32761, -1]);
        value.position = original;
        integrate_particle_words(&mut value, 125_000);
        assert_eq!(world_position_to_raw(value.position), [-28871, 28870, -1]);
        // Even repeated 32us visits never accumulate the positive fraction.
        value.position = raw_position_to_world([0; 3]);
        value.velocity = raw_velocity_to_world([1, 0, 0]);
        for _ in 0..1024 {
            integrate_particle_words(&mut value, 32);
        }
        assert_eq!(world_position_to_raw(value.position), [0; 3]);
    }
}

#[test]
fn fireball_f350_governor_loop_owns_gravity_and_three_words_even_at_zero_delta() {
    for (delta, iterations) in [(0, 1), (25_178, 1), (25_179, 2), (125_000, 5)] {
        let mut fx = WorldFx::new();
        fx.frame_pacing.count_scale_q16 = 65_536;
        let slot = particle(
            &mut fx,
            38,
            [1000, 4000, 2000],
            [100, 100, 300],
            ParticleEnvironment::Dry,
        );
        let mut value = fx.particles.slots[slot].unwrap();
        let mut expected_rng = fx.rng_state;
        for _ in 0..iterations * 3 {
            retail_random_u16(&mut expected_rng);
        }
        fx.update_combat_projectile(
            CombatProjectileProgram::DragonFireball,
            slot,
            &mut value,
            delta,
            birth(ParticleEnvironment::Dry),
        );
        assert_eq!(
            world_velocity_component_to_raw(value.velocity[1]),
            100 - 36 * iterations
        );
        assert_eq!(fx.rng_state, expected_rng);
        let trails: Vec<_> = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|p| p.source_class == 40)
            .collect();
        assert_eq!(trails.len(), iterations as usize);
        assert!(trails
            .iter()
            .all(|p| p.owner_id == Some(0) && p.source_entity_type_at_birth == Some(0)));
        assert_eq!(
            world_position_to_raw(trails.last().unwrap().position),
            [1000, 4000, 2000]
        );
    }
}

#[test]
fn fireball_f350_fully_submerged_skips_all_callback_work() {
    let mut fx = WorldFx::new();
    fx.frame_pacing.count_scale_q16 = 65_536;
    let environment = ParticleEnvironment::FlatWater { sea_level: 0.0 };
    let slot = particle(&mut fx, 38, [100, -1000, 200], [1, 2, 3], environment);
    let mut value = fx.particles.slots[slot].unwrap();
    assert_eq!(value.water_state, 0);
    let before = value.velocity;
    let rng = fx.rng_state;
    fx.update_combat_projectile(
        CombatProjectileProgram::DragonFireball,
        slot,
        &mut value,
        125_000,
        birth(environment),
    );
    assert_eq!(value.velocity, before);
    assert_eq!(fx.rng_state, rng);
    assert_eq!(fx.particle_count(), 1);
}

#[test]
fn fireball_bounce_preserves_positive_velocity_and_submits_exact_ground_program() {
    for (vy, expected) in [(-1001, 251), (-3, 1), (0, 0), (99, 99)] {
        let mut fx = WorldFx::new();
        let slot = particle(
            &mut fx,
            38,
            [-1, -20, 1024],
            [10, vy, 30],
            ParticleEnvironment::Dry,
        );
        let mut value = fx.particles.slots[slot].unwrap();
        let rng = fx.rng_state;
        let result = fx.apply_combat_projectile_surface(
            slot,
            &mut value,
            0,
            -100,
            birth(ParticleEnvironment::Dry),
        );
        assert_eq!(
            result,
            CombatSurfaceOutcome::GroundProgram(CombatGroundProgramRequest {
                position_raw: [-1, -60, 1024]
            })
        );
        assert_eq!(world_velocity_component_to_raw(value.velocity[1]), expected);
        assert_eq!(fx.rng_state, rng);
        assert_eq!(fx.particle_count(), 1);
    }
}

#[test]
fn fireball_water_41770_and_delete_are_distinct_from_bolt_spray() {
    for (surface, expected_class, expected_vy) in [(0, 43, 700), (1, 75, 280)] {
        let mut fx = WorldFx::new();
        let environment = ParticleEnvironment::FlatWater { sea_level: 0.0 };
        let slot = particle(&mut fx, 38, [256, 200, 512], [0; 3], environment);
        let mut value = fx.particles.slots[slot].unwrap();
        let rng = fx.rng_state;
        assert_eq!(
            fx.apply_combat_projectile_surface(slot, &mut value, 6, surface, birth(environment)),
            CombatSurfaceOutcome::Consume
        );
        let child = fx
            .particles
            .slots
            .iter()
            .flatten()
            .find(|p| p.source_class == expected_class)
            .unwrap();
        assert_eq!(world_position_to_raw(child.position), [256, surface, 512]);
        assert_eq!(
            world_velocity_component_to_raw(child.velocity[1]),
            expected_vy
        );
        assert_eq!(child.owner_id, Some(0));
        assert_eq!(fx.rng_state, rng);
        assert!(fx.ready_sounds.is_empty());
        let count = fx.particle_count();
        assert_eq!(
            fx.apply_combat_projectile_surface(slot, &mut value, 7, surface, birth(environment)),
            CombatSurfaceOutcome::Consume
        );
        assert_eq!(fx.particle_count(), count);
    }
}

#[test]
fn bolt_water_descriptor_relinks_without_resetting_motion_or_provenance() {
    for pair in [
        TurretBoltPair::Class49,
        TurretBoltPair::Class55,
        TurretBoltPair::Class56,
    ] {
        let mut fx = WorldFx::new();
        let slot = particle(
            &mut fx,
            pair.airborne(),
            [256, -1000, 512],
            [2000, -999, 500],
            ParticleEnvironment::FlatWater { sea_level: 0.0 },
        );
        let mut value = fx.particles.slots[slot].unwrap();
        value.age_ticks = 20.0;
        let position = value.position;
        let velocity = value.velocity;
        assert_eq!(
            fx.apply_combat_projectile_surface(
                slot,
                &mut value,
                6,
                0,
                birth(ParticleEnvironment::FlatWater { sea_level: 0.0 })
            ),
            CombatSurfaceOutcome::Continue
        );
        assert_eq!(
            (value.source_class, value.age_ticks),
            (pair.submerged(), 0.0)
        );
        assert_eq!(value.position, position);
        assert_eq!(value.velocity, velocity);
        assert_eq!(value.owner_id, Some(77));
        assert_eq!(value.source_entity_type_at_birth, Some(10));
        value.age_ticks = 4.0;
        assert_eq!(
            fx.apply_combat_projectile_surface(
                slot,
                &mut value,
                6,
                0,
                birth(ParticleEnvironment::FlatWater { sea_level: 0.0 })
            ),
            CombatSurfaceOutcome::Continue
        );
        assert_eq!(
            value.age_ticks, 0.0,
            "441DE1 resets even an existing submerged descriptor"
        );
        value.age_ticks = 4.0;
        value.water_state = 1;
        fx.update_combat_projectile(
            CombatProjectileProgram::SubmergedTurretBolt(pair),
            slot,
            &mut value,
            0,
            birth(ParticleEnvironment::FlatWater { sea_level: 0.0 }),
        );
        assert_eq!(
            (value.source_class, value.age_ticks),
            (pair.airborne(), 0.0)
        );
        assert_eq!(value.velocity, velocity);
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn submerged_bolt_marks_after_five_then_next_visit_cleans_up_nine_positions() {
    for class in [80, 81, 82] {
        let mut fx = WorldFx::new();
        let environment = ParticleEnvironment::FlatWater { sea_level: 0.0 };
        let slot = particle(
            &mut fx,
            class,
            [32700, -2000, 512],
            [1000, 0, 0],
            environment,
        );
        fx.last_retail_tick = Some(0);
        fx.update(ParticleUpdateRequest::flat_water(0, 5, 0.0));
        assert!(!fx.particles.slots[slot].unwrap().pending_destruction);
        fx.update(ParticleUpdateRequest::flat_water(0, 6, 0.0));
        assert!(fx.particles.slots[slot].unwrap().pending_destruction);
        assert_eq!(fx.particle_count(), 1, "F030 marking is not an inline free");
        let rng = fx.rng_state;
        fx.update(ParticleUpdateRequest::flat_water(0, 6, 0.0));
        assert!(fx.particles.slots[slot].is_none());
        let bubbles: Vec<_> = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|p| p.source_class == 42)
            .collect();
        assert_eq!(bubbles.len(), 9);
        for step in 1..=9 {
            assert!(bubbles.iter().any(|p| world_position_to_raw(p.position)
                == [32700_i16.wrapping_add(step * 25), -2000, 512]));
        }
        assert!(bubbles
            .iter()
            .all(|p| p.owner_id == Some(0) && p.source_entity_type_at_birth == Some(0)));
        assert_eq!(fx.rng_state, rng);
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn turret_ground_spray_consumes_eight_direction_entries_then_one_sound_word() {
    for class in [49, 55, 56, 80, 81, 82] {
        let mut fx = WorldFx::new();
        fx.frame_pacing.count_scale_q16 = 65_536;
        let slot = particle(
            &mut fx,
            class,
            [100, 99, 200],
            [300, -100, 0],
            ParticleEnvironment::Dry,
        );
        let mut value = fx.particles.slots[slot].unwrap();
        let mut rng = fx.rng_state;
        let rate = 0xE000 + u32::from(retail_random_u16(&mut rng) >> 2);
        let old_direction = fx.direction_cursor;
        assert_eq!(
            fx.apply_combat_projectile_surface(
                slot,
                &mut value,
                0,
                32,
                birth(ParticleEnvironment::Dry)
            ),
            CombatSurfaceOutcome::Consume
        );
        assert_eq!(fx.direction_cursor, (old_direction + 8) % 100);
        assert_eq!(fx.rng_state, rng);
        assert_eq!(fx.particle_count(), 9);
        assert_eq!(
            fx.ready_sounds,
            vec![PositionalSoundEvent {
                sound_id: 83,
                position: raw_position_to_world([100, 99, 200]),
                frequency_q16: rate
            }]
        );
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|p| p.source_class == 7)
            .all(|p| world_position_to_raw(p.position)[1] == 32 && p.owner_id == Some(0)));
    }
}

#[test]
fn borrowed_traversal_reports_unhandled_ground_program_after_committing_bounce() {
    let ground = terrain(0);
    let context = context(&ground);
    let mut fx = WorldFx::new();
    let slot = particle(
        &mut fx,
        38,
        [100, 30, 200],
        [0, -4, 0],
        ParticleEnvironment::Terrain(context),
    );
    let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));
    assert_eq!(
        outcome.unhandled_ground_programs,
        vec![CombatGroundProgramRequest {
            position_raw: [100, 40, 200]
        }]
    );
    assert_eq!(
        world_position_to_raw(fx.particles.slots[slot].unwrap().position),
        [100, 40, 200]
    );
}

struct GroundHost {
    terrain: TerrainGrid,
    submissions: Vec<CombatGroundProgramRequest>,
}
impl ParticleTraversalHost for GroundHost {
    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: ParticleEnvironment::Terrain(context(&self.terrain)),
            callbacks: ParticleCallbackContext::default(),
            particle_emitter: ParticleEmitterContext::default(),
        }
    }
    fn entity_impact(&mut self, _: &mut WorldFx, _: ParticleEntityImpact) {
        panic!("no models");
    }
    fn static_impact(
        &mut self,
        _: &mut WorldFx,
        _: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        panic!("no static models");
    }
    fn ground_program_request(
        &mut self,
        _: &mut WorldFx,
        request: CombatGroundProgramRequest,
    ) -> bool {
        self.submissions.push(request);
        // A synchronous world mutation must be visible to the next slot.
        self.terrain
            .cells
            .iter_mut()
            .for_each(|cell| cell.height = (-127_i8) as u8);
        true
    }
}

#[test]
fn ground_program_host_runs_between_physical_slots_without_replaying_submission() {
    let mut host = GroundHost {
        terrain: terrain(0),
        submissions: Vec::new(),
    };
    let mut fx = WorldFx::new();
    let environment = host.context().environment;
    let later = particle(&mut fx, 38, [100, 30, 200], [0; 3], environment);
    let earlier = particle(&mut fx, 38, [200, 30, 200], [0; 3], environment);
    assert!(earlier < later);
    let outcome = fx.update_with_traversal_host(
        ParticleTraversalTiming {
            elapsed_micros: 0,
            retail_tick: 0,
        },
        &mut host,
    );
    assert_eq!(
        host.submissions,
        vec![CombatGroundProgramRequest {
            position_raw: [200, 40, 200]
        }]
    );
    assert!(outcome.unhandled_ground_programs.is_empty());
    assert_eq!(
        world_position_to_raw(fx.particles.slots[later].unwrap().position)[1],
        30
    );
}
