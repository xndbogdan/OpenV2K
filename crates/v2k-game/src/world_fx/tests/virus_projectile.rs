use super::*;
use crate::world_fx::virus_projectile::uses_virus_projectile_callbacks;

fn context(terrain: &TerrainGrid, selector: u8) -> TerrainCollisionContext<'_> {
    TerrainCollisionContext {
        terrain,
        terrain_objects: None,
        ground_response_selectors: [selector; 8],
        water_response_selectors: [selector; 8],
    }
}

fn emitter() -> ParticleEmitterContext {
    ParticleEmitterContext {
        current_owner: Some(ParticleOwnerAtBirth {
            entity_id: 901,
            entity_type: 68,
        }),
    }
}

fn projectile(
    fx: &mut WorldFx,
    position: [i16; 3],
    velocity: [i16; 3],
    environment: ParticleEnvironment<'_>,
) -> usize {
    fx.materialize_descriptor_particle_request(
        DescriptorParticleRequest {
            source_class: 50,
            position_raw: position,
            velocity_raw: velocity,
            owner: Some(ParticleOwnerAtBirth {
                entity_id: 115,
                entity_type: 115,
            }),
            suppresses_impact_damage: false,
        },
        environment,
        0,
    )
    .unwrap()
}

fn carriers(fx: &WorldFx) -> Vec<WorldParticle> {
    fx.test_particles_in_virgin_birth_order()
        .into_iter()
        .filter(|particle| particle.source_class == 5)
        .collect()
}

fn assert_carriers(fx: &WorldFx, position: [f32; 3], count: usize, horizontal_shift: u32) {
    let children = carriers(fx);
    assert_eq!(children.len(), count);
    for (index, child) in children.iter().enumerate() {
        let direction = RETAIL_DIRECTION_TABLE_RAW[index + 1];
        assert_eq!(child.position, position);
        assert_eq!(
            child.velocity.map(world_velocity_component_to_raw),
            [
                i32::from(direction[0]) >> horizontal_shift,
                (i32::from(direction[1]) >> 2).abs(),
                i32::from(direction[2]) >> horizontal_shift,
            ]
        );
        assert_eq!(
            child.owner_id,
            Some(901),
            "current global owner, not flower parent"
        );
        assert_eq!(child.source_entity_type_at_birth, Some(68));
        assert!(!child.suppresses_impact_damage);
    }
}

#[test]
fn class50_descriptor_owns_carrier_callbacks_and_exact_word_motion_gravity() {
    assert_eq!(
        (0..PARTICLE_DESCRIPTORS.len() as u8)
            .filter(|&class| uses_virus_projectile_callbacks(class))
            .collect::<Vec<_>>(),
        [50]
    );
    for (environment, y, dt, expected_position, expected_vertical) in [
        (
            ParticleEnvironment::Dry,
            1000,
            20_000,
            [i16::MIN + 146, 993, -10],
            -362,
        ),
        (
            ParticleEnvironment::FlatWater { sea_level: 0.0 },
            -1000,
            20_000,
            [i16::MIN + 146, -1007, -10],
            -347,
        ),
        (ParticleEnvironment::Dry, 1000, 31, [32760, 1000, 0], -333),
    ] {
        let mut fx = WorldFx::new();
        let slot = projectile(&mut fx, [32760, y, 0], [8123, -333, -511], environment);
        let rng = fx.rng_state;
        fx.update(ParticleUpdateRequest::new(dt, 0, environment));
        let after = fx.particles.slots[slot].unwrap();
        assert_eq!(world_position_to_raw(after.position), expected_position);
        assert_eq!(
            after.velocity.map(world_velocity_component_to_raw),
            [8123, expected_vertical, -511]
        );
        assert_eq!(fx.rng_state, rng);
        assert_eq!(fx.direction_cursor, 0);
    }
}

#[test]
fn class50_surface_emits_paced_full_horizontal_spread_without_snap_or_rng() {
    let terrain = flat_terrain(0, 0);
    let context = context(&terrain, 0);
    for full_rate in [false, true] {
        let mut fx = if full_rate {
            full_rate_world_fx()
        } else {
            WorldFx::new()
        };
        let slot = projectile(
            &mut fx,
            [1024, -32, 1280],
            [0; 3],
            ParticleEnvironment::Terrain(context),
        );
        let rng = fx.rng_state;
        let outcome = fx
            .update(ParticleUpdateRequest::terrain(0, 0, context).with_particle_emitter(emitter()));
        assert!(fx.particles.slots[slot].is_none());
        assert_carriers(&fx, [4.0, -0.125, 5.0], if full_rate { 8 } else { 1 }, 0);
        assert_eq!(fx.direction_cursor, if full_rate { 8 } else { 1 });
        assert_eq!(fx.rng_state, rng);
        assert_eq!(
            outcome,
            ParticleUpdateOutcome::default(),
            "children in earlier slots wait for next pass"
        );
        assert!(fx.take_positional_sounds().is_empty());
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn class50_selector_six_precedes_suppression_and_uses_signed_division() {
    let terrain = flat_terrain(0, 0);
    for selector in [0, 6] {
        for suppressed in [false, true] {
            let context = context(&terrain, selector);
            let mut fx = full_rate_world_fx();
            let slot = projectile(
                &mut fx,
                [1024, -32, 1280],
                [-7, -5, 7],
                ParticleEnvironment::Terrain(context),
            );
            fx.particles.slots[slot]
                .as_mut()
                .unwrap()
                .suppresses_impact_damage = suppressed;
            let rng = fx.rng_state;
            let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));
            if selector == 6 {
                let retained = fx.particles.slots[slot].unwrap();
                assert_eq!(
                    retained.velocity.map(world_velocity_component_to_raw),
                    [-1, -2, 1]
                );
                assert_eq!(retained.position, [4.0, -0.125, 5.0]);
                assert_eq!(fx.direction_cursor, 0);
                assert!(carriers(&fx).is_empty());
            } else {
                assert!(fx.particles.slots[slot].is_none());
                assert_eq!(carriers(&fx).len(), if suppressed { 0 } else { 8 });
                assert_eq!(fx.direction_cursor, if suppressed { 0 } else { 8 });
                for child in carriers(&fx) {
                    assert_eq!(
                        child.owner_id, None,
                        "default current global is unresolved sentinel"
                    );
                    assert_eq!(child.source_entity_type_at_birth, Some(0));
                }
            }
            assert_eq!(outcome, ParticleUpdateOutcome::default());
            assert_eq!(fx.rng_state, rng);
            assert!(fx.take_positional_sounds().is_empty());
        }
    }
}

#[test]
fn class50_entity_contact_emits_half_horizontal_spread_at_integrated_endpoint_without_damage() {
    let pool = solid_sphere_pool(200);
    let entities = [collision_entity(9, [4.0, 2.0, 5.0], 200)];
    for suppressed in [false, true] {
        let mut fx = full_rate_world_fx();
        let slot = projectile(
            &mut fx,
            [1024, 512, 1280],
            [8000, 0, 0],
            ParticleEnvironment::Dry,
        );
        fx.particles.slots[slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = suppressed;
        let rng = fx.rng_state;
        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(20_000, 0)
                .with_particle_emitter(emitter())
                .with_callbacks(ParticleCallbackContext {
                    owner_motions: &[],
                    collision: Some(ParticleCollisionContext {
                        entities: &entities,
                        model_pool: &pool,
                    }),
                }),
            |_, _| panic!("441FA0 has no direct entity damage callback"),
            |_, _| panic!("fixture has no static model"),
        );
        assert!(fx.particles.slots[slot].is_none());
        assert_carriers(
            &fx,
            raw_position_to_world([1176, 512, 1280]),
            if suppressed { 0 } else { 8 },
            1,
        );
        assert_eq!(fx.rng_state, rng);
        assert_eq!(outcome, ParticleUpdateOutcome::default());
        assert!(fx.take_positional_sounds().is_empty());
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn class50_static_contact_uses_ff10_refined_position_without_static_damage() {
    let mut terrain = flat_terrain(16, 0);
    terrain_cell_mut(&mut terrain, 4, 4).attribute = 1;
    let objects = terrain_object_table(1, [0; 4], 29);
    let context = TerrainCollisionContext {
        terrain_objects: Some(&objects),
        ..context(&terrain, 0)
    };
    let pool = solid_sphere_pool(200);
    for suppressed in [false, true] {
        let mut fx = full_rate_world_fx();
        let slot = projectile(
            &mut fx,
            [1152, 512, 1152],
            [8000, 0, 0],
            ParticleEnvironment::Terrain(context),
        );
        fx.particles.slots[slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = suppressed;
        // Every phase-anchored FF10 probe visits occupied cell4/4. A path
        // straddling an empty neighboring cell can cancel its coarse hit.
        let endpoint = raw_position_to_world([1304, 512, 1152]);
        let hit = retail_swept_static_tile_hit(
            [4.5, 2.0, 4.5],
            endpoint,
            20,
            &terrain,
            &objects,
            0,
            &pool,
            &[],
        )
        .unwrap();
        assert_eq!(world_position_to_raw(hit.position_world), [1285, 512, 1152]);
        assert_ne!(hit.position_world, endpoint);
        let rng = fx.rng_state;
        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(20_000, 0, context)
                .with_particle_emitter(emitter())
                .with_callbacks(ParticleCallbackContext {
                    owner_motions: &[],
                    collision: Some(ParticleCollisionContext {
                        entities: &[],
                        model_pool: &pool,
                    }),
                }),
            |_, _| panic!("fixture has no entity"),
            |_, _| panic!("442070 has no direct static damage callback"),
        );
        assert!(fx.particles.slots[slot].is_none());
        assert_carriers(&fx, hit.position_world, if suppressed { 0 } else { 8 }, 1);
        assert_eq!(outcome, ParticleUpdateOutcome::default());
        assert_eq!(fx.rng_state, rng);
        assert!(fx.take_positional_sounds().is_empty());
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn class50_water_entry_damps_before_ground_and_initializes_child_water_at_birth() {
    let mut terrain = flat_terrain(-64, 0);
    terrain.header[0] = 0;
    // The authored wave terms are exactly zero here at tick0, so the
    // newborn sphere starts above the actual displaced surface.
    assert_eq!(wave_surface_raw(0, 0, 0, 0, -2048), 0);
    for selector in [0, 6] {
        let context = context(&terrain, selector);
        let mut fx = full_rate_world_fx();
        let slot = projectile(
            &mut fx,
            [0, 32, 0],
            [-7, -8000, 7],
            ParticleEnvironment::Terrain(context),
        );
        assert_eq!(fx.particles.slots[slot].unwrap().water_state, 2);
        let outcome = fx.update(
            ParticleUpdateRequest::terrain(20_000, 0, context).with_particle_emitter(emitter()),
        );
        if selector == 6 {
            let retained = fx.particles.slots[slot].unwrap();
            assert_eq!(retained.water_state, 0);
            assert_eq!(world_position_to_raw(retained.position), [-1, -121, 0]);
            assert_eq!(
                retained.velocity.map(world_velocity_component_to_raw),
                [-1, -4007, 1]
            );
            assert_eq!(fx.direction_cursor, 0);
        } else {
            assert!(fx.particles.slots[slot].is_none());
            assert_eq!(carriers(&fx).len(), 8);
            assert!(carriers(&fx)
                .iter()
                .all(|child| child.water_state_initialized && child.water_state == 0));
        }
        assert_eq!(outcome, ParticleUpdateOutcome::default());
    }
}

#[test]
fn class50_rejected_children_still_advance_each_direction_before_parent_is_freed() {
    let terrain = flat_terrain(0, 0);
    let context = context(&terrain, 0);
    let mut fx = full_rate_world_fx();
    let slot = projectile(
        &mut fx,
        [1024, 0, 1280],
        [0; 3],
        ParticleEnvironment::Terrain(context),
    );
    fx.particles.test_fill_to(
        MAX_WORLD_PARTICLES,
        descriptor_test_particle(50, [20.0; 3], [0; 3], None),
    );
    fx.direction_cursor = 98;
    let rng = fx.rng_state;
    let outcome = fx.update(ParticleUpdateRequest::terrain(0, 0, context));
    assert!(fx.particles.slots[slot].is_none());
    assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES - 1);
    assert!(
        carriers(&fx).is_empty(),
        "parent must remain occupied through all rejected attempts"
    );
    assert_eq!(fx.direction_cursor, 6);
    assert_eq!(fx.rng_state, rng);
    assert_eq!(outcome, ParticleUpdateOutcome::default());
    fx.particles.assert_valid_topology();
}

struct InfectionHost {
    terrain: TerrainGrid,
}

impl ParticleTraversalHost for InfectionHost {
    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: ParticleEnvironment::Terrain(context(&self.terrain, 0)),
            callbacks: ParticleCallbackContext::default(),
            particle_emitter: emitter(),
        }
    }
    fn publish_terrain_mutations(
        &mut self,
        mutations: &[ParticleTerrainMutation],
    ) -> ParticleTerrainPublication {
        for &write in mutations {
            let [x, z] = write.cell();
            let cell = terrain_cell_mut(&mut self.terrain, usize::from(x), usize::from(z));
            cell.terrain_type = write.apply(cell.terrain_type);
        }
        ParticleTerrainPublication::Committed
    }
    fn entity_impact(&mut self, _: &mut WorldFx, _: ParticleEntityImpact) {
        panic!("fixture has no entity")
    }
    fn static_impact(
        &mut self,
        _: &mut WorldFx,
        _: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        panic!("fixture has no static model")
    }
}

#[test]
fn class50_children_infect_live_terrain_in_physical_slot_order() {
    for children_in_later_slots in [false, true] {
        let mut fx = full_rate_world_fx();
        let mut host = InfectionHost {
            terrain: flat_terrain(0, 0),
        };
        if children_in_later_slots {
            fx.particles.test_fill_to(
                MAX_WORLD_PARTICLES - 1,
                descriptor_test_particle(50, [20.0; 3], [0; 3], None),
            );
        }
        let slot = projectile(&mut fx, [1024, 0, 1280], [0; 3], host.context().environment);
        if children_in_later_slots {
            assert_eq!(slot, 0);
            for later_slot in 1..=8 {
                assert!(fx.particles.free(later_slot));
            }
        } else {
            assert_eq!(slot, MAX_WORLD_PARTICLES - 1);
        }
        let rng = fx.rng_state;
        let timing = ParticleTraversalTiming {
            elapsed_micros: 0,
            retail_tick: 0,
        };
        let first = fx.update_with_traversal_host(timing, &mut host);
        assert!(fx.particles.slots[slot].is_none());
        assert_eq!(
            host.terrain.cells[4 * GRID_SIZE + 5].terrain_type & 0x10 != 0,
            children_in_later_slots
        );
        let writes = if children_in_later_slots {
            assert!(carriers(&fx).is_empty());
            first.terrain_type_mutations
        } else {
            assert!(first.terrain_type_mutations.is_empty());
            assert_eq!(carriers(&fx).len(), 8);
            fx.update_with_traversal_host(timing, &mut host)
                .terrain_type_mutations
        };
        assert!(!writes.is_empty());
        assert!(writes.iter().all(|write| *write
            == ParticleTerrainMutation::Infection {
                cell: [4, 5],
                infected: true
            }));
        assert_eq!(host.terrain.cells[4 * GRID_SIZE + 5].terrain_type, 0x10);
        assert!(carriers(&fx).is_empty());
        assert_eq!(fx.rng_state, rng);
        assert_eq!(fx.direction_cursor, 8);
        assert!(fx.take_positional_sounds().is_empty());
        fx.particles.assert_valid_topology();
    }
}
