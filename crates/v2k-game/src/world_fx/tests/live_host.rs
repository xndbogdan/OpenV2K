use super::*;

struct MutableWorldHost {
    terrain: TerrainGrid,
    pool: TestModelPool,
    entities: Vec<EntityCollisionModel>,
    motions: Vec<ParticleOwnerMotion>,
    impacts: usize,
    parent_slot: usize,
    recycle_parent: bool,
    callback_sound: bool,
    publish_terrain_writes: bool,
    clear_infection_on_impact: Option<[u8; 2]>,
}

impl ParticleTraversalHost for MutableWorldHost {
    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: ParticleEnvironment::Terrain(TerrainCollisionContext {
                terrain: &self.terrain,
                terrain_objects: None,
                ground_response_selectors: [0; 8],
                water_response_selectors: [0; 8],
            }),
            callbacks: ParticleCallbackContext {
                owner_motions: &self.motions,
                collision: Some(ParticleCollisionContext {
                    entities: &self.entities,
                    model_pool: &self.pool,
                }),
            },
            particle_emitter: ParticleEmitterContext::default(),
        }
    }

    fn entity_impact(&mut self, fx: &mut WorldFx, impact: ParticleEntityImpact) {
        self.impacts += 1;
        if let Some([x, z]) = self.clear_infection_on_impact {
            let cell = &mut self.terrain.cells[usize::from(x) * 256 + usize::from(z)];
            assert_ne!(
                cell.terrain_type & INFECTION_TERRAIN_TYPE_BIT,
                0,
                "a later synchronous callback must see the earlier particle infection"
            );
            // Factory terminal terrain craters clear this bit synchronously.
            // The earlier particle journal must not replay across this write.
            cell.terrain_type &= !INFECTION_TERRAIN_TYPE_BIT;
        }
        assert!(fx.particles.slots[self.parent_slot].is_some());
        // F610's child is already allocated, but the source parent is not freed.
        assert!(fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|p| p.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS));
        // These are the same fields borrowed by context(), not a cloned world.
        self.terrain
            .cells
            .iter_mut()
            .for_each(|cell| cell.height = 64);
        self.entities.clear();
        self.motions[0].velocity = [7.0, 8.0, 9.0];
        if self.callback_sound {
            assert!(fx.ready_sounds.is_empty());
            fx.emit_fun_0044f450_hit_sound(62, impact.position_world);
        }
        if self.recycle_parent {
            assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
            // F610 has consumed the last virgin free slot. Free inserts at
            // the head, so the parent is now both head and tail; otherwise
            // allocation correctly prefers an older free slot instead.
            assert!(fx.particles.free(self.parent_slot));
            assert_eq!(
                fx.particles.allocate(descriptor_test_particle(
                    PRIMARY_ABOVE_WATER_IMPACT_CLASS,
                    [99.0; 3],
                    [0; 3],
                    None,
                )),
                Some(self.parent_slot)
            );
        }
    }

    fn static_impact(
        &mut self,
        _: &mut WorldFx,
        _: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        panic!("fixture has no static model");
    }

    fn publish_terrain_mutations(
        &mut self,
        mutations: &[ParticleTerrainMutation],
    ) -> ParticleTerrainPublication {
        if !self.publish_terrain_writes {
            return ParticleTerrainPublication::Deferred;
        }
        for &write in mutations {
            let [x, z] = write.cell();
            let cell = &mut self.terrain.cells[usize::from(x) * 256 + usize::from(z)];
            cell.terrain_type = write.apply(cell.terrain_type);
        }
        ParticleTerrainPublication::Committed
    }
}

fn host(parent_slot: usize) -> MutableWorldHost {
    let mut terrain = flat_terrain(-64, 0);
    terrain.header[0] = 0;
    MutableWorldHost {
        terrain,
        pool: solid_sphere_pool(100),
        entities: vec![collision_entity(9, [4.0, 2.0, 5.0], 100)],
        motions: vec![ParticleOwnerMotion {
            owner_id: 7,
            velocity: [1.0; 3],
        }],
        impacts: 0,
        parent_slot,
        recycle_parent: false,
        callback_sound: false,
        publish_terrain_writes: false,
        clear_infection_on_impact: None,
    }
}

#[test]
fn committed_particle_infection_precedes_callback_clear_and_later_reinfection() {
    for reinfect_after_callback in [false, true] {
        let mut fx = WorldFx::new();
        let cell = [4, 5];
        let carrier =
            descriptor_test_particle(ALIEN_HIVE_PARTICLE_CLASS, [4.0, 0.0, 5.0], [0; 3], None);
        // Virgin allocation descends physical slots. The optional last carrier
        // must be allocated first to run after the intervening entity callback.
        let later_slot = reinfect_after_callback.then(|| fx.particles.allocate(carrier).unwrap());
        let parent_slot = fx
            .particles
            .allocate(descriptor_test_particle(
                PRIMARY_BULLET_PARTICLE_CLASS,
                [4.0, 2.0, 5.0],
                [1000, 0, 0],
                Some(7),
            ))
            .unwrap();
        let first_slot = fx.particles.allocate(carrier).unwrap();
        assert!(first_slot < parent_slot);
        assert!(later_slot.is_none_or(|slot| parent_slot < slot));
        let mut host = host(parent_slot);
        host.terrain = flat_terrain(0, 0);
        host.publish_terrain_writes = true;
        host.clear_infection_on_impact = Some(cell);

        let outcome = fx.update_with_traversal_host(
            ParticleTraversalTiming {
                elapsed_micros: 0,
                retail_tick: 1,
            },
            &mut host,
        );

        assert_eq!(host.impacts, 1);
        let write = ParticleTerrainMutation::Infection {
            cell,
            infected: true,
        };
        assert_eq!(
            outcome.terrain_type_mutations,
            if reinfect_after_callback {
                vec![write, write]
            } else {
                vec![write]
            },
            "committed writes remain telemetry, but must leave the pending detection overlay"
        );
        assert_eq!(
            host.terrain.cells[4 * 256 + 5].terrain_type & INFECTION_TERRAIN_TYPE_BIT != 0,
            reinfect_after_callback,
            "the final state follows physical-slot order rather than replaying old writes"
        );
        assert!(fx.particles.slots[first_slot].is_none());
        if let Some(later_slot) = later_slot {
            assert!(fx.particles.slots[later_slot].is_none());
        }
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn particle_live_host_later_slots_observe_synchronous_terrain_and_owner_writes() {
    let mut fx = WorldFx::new();
    let watched = descriptor_test_particle(
        PRIMARY_MUZZLE_PARTICLE_CLASS,
        [20.0, -4.0, 20.0],
        [0; 3],
        Some(7),
    );
    let watched_slot = fx.particles.allocate(watched).unwrap();
    let parent = descriptor_test_particle(
        PRIMARY_BULLET_PARTICLE_CLASS,
        [4.0, 2.0, 5.0],
        [1000, 0, 0],
        Some(7),
    );
    let parent_slot = fx.particles.allocate(parent).unwrap();
    assert!(parent_slot < watched_slot);
    let mut host = host(parent_slot);
    assert_eq!(
        classify_particle_water(
            host.context().environment.terrain_context(),
            Some(0.0),
            watched.position,
            f32::from(watched.collision_radius_raw) / 256.0,
            1
        ),
        0
    );
    let outcome = fx.update_with_traversal_host(
        ParticleTraversalTiming {
            elapsed_micros: 0,
            retail_tick: 1,
        },
        &mut host,
    );
    assert_eq!(host.impacts, 1);
    assert_eq!(outcome.entity_impacts.len(), 1);
    assert!(fx.particles.slots[parent_slot].is_none());
    let watched = fx.particles.slots[watched_slot].unwrap();
    assert_eq!(watched.position, [20.0, -4.0, 20.0]);
    assert_eq!(watched.velocity, [7.0, 8.0, 9.0]);
    assert_eq!(
        watched.water_state, 2,
        "the freshly raised tile has no water surface"
    );
    fx.particles.assert_valid_topology();
}

#[test]
fn particle_live_host_f6e0_sound_follows_callback_and_outer_free_removes_replacement() {
    let mut fx = WorldFx::new();
    let parent =
        descriptor_test_particle(FUN_0043F6E0_CLASS_4, [4.0, 2.0, 5.0], [1000, 0, 0], Some(7));
    let parent_slot = fx.particles.allocate(parent).unwrap();
    let filler = descriptor_test_particle(
        PRIMARY_BULLET_PARTICLE_CLASS,
        [20.0, 20.0, 20.0],
        [1000, 0, 0],
        None,
    );
    // Leave one free physical slot for F610's visual, which is materialized
    // before the host callback. These unrelated bullets survive their visits.
    fx.particles.test_fill_to(MAX_WORLD_PARTICLES - 1, filler);
    let mut host = host(parent_slot);
    host.recycle_parent = true;
    host.callback_sound = true;
    let outcome = fx.update_with_traversal_host(
        ParticleTraversalTiming {
            elapsed_micros: 0,
            retail_tick: 1,
        },
        &mut host,
    );
    assert_eq!(outcome.entity_impacts.len(), 1);
    assert_eq!(host.impacts, 1);
    assert!(fx.particles.slots[parent_slot].is_none());
    assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES - 1);
    assert_eq!(
        fx.ready_sounds
            .iter()
            .map(|s| s.sound_id)
            .collect::<Vec<_>>(),
        [62, 90]
    );
    let mut rng = 0;
    for sound in &fx.ready_sounds {
        assert_eq!(
            sound.frequency_q16,
            0x1_0000 + u32::from(retail_random_u16(&mut rng) >> 3)
        );
    }
    assert_eq!(fx.rng_state, rng);
    fx.particles.assert_valid_topology();
}
