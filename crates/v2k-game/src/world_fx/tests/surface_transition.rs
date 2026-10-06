use super::*;
use crate::projectile_emitter::{
    projectile_surface_response, ProjectileSurfaceResponse,
    PROJECTILE_REPLACEMENT_GROUND_CALLBACK_VA,
};

const SOURCE_REPLACEMENTS: [(u8, u8); 5] = [(52, 74), (53, 74), (68, 76), (69, 76), (87, 88)];

fn request(source_class: u8, position_raw: [i16; 3]) -> DescriptorParticleRequest {
    DescriptorParticleRequest {
        source_class,
        position_raw,
        velocity_raw: [0, -8_000, 0],
        owner: Some(ParticleOwnerAtBirth {
            entity_id: 0x1234_0001,
            entity_type: 57,
        }),
        suppresses_impact_damage: false,
    }
}

#[test]
fn descriptor_family_and_retail_source_switch_precede_surface_selector() {
    let admitted = (0..PARTICLE_DESCRIPTORS.len())
        .filter(|class| uses_projectile_surface_callback(*class as u8))
        .collect::<Vec<_>>();
    assert_eq!(admitted, [52, 53, 68, 69, 87]);
    for (source, replacement) in SOURCE_REPLACEMENTS {
        for selector in [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12] {
            assert_eq!(
                projectile_surface_response(source, selector),
                ProjectileSurfaceResponse::ConvertToParticleClass(replacement),
                "source {source}, selector {selector}"
            );
        }
        assert_eq!(
            projectile_surface_response(source, 6),
            ProjectileSurfaceResponse::ConvertToParticleClass(73)
        );
        for selector in [7, 13, u8::MAX] {
            assert_eq!(
                projectile_surface_response(source, selector),
                ProjectileSurfaceResponse::Delete
            );
        }
    }
    for source in [0, 51, 54, 67, 70, 73, 74, 76, 86, 88, u8::MAX] {
        assert_eq!(
            projectile_surface_response(source, 6),
            ProjectileSurfaceResponse::Delete,
            "selector 6 cannot bypass F42420's source switch"
        );
    }
    for replacement in [73, 74, 76, 88] {
        let descriptor = particle_descriptor(replacement).unwrap();
        assert_eq!(descriptor.raw_byte(0x0e), 0);
        assert_eq!(
            descriptor.raw_u32(0x18),
            PROJECTILE_REPLACEMENT_GROUND_CALLBACK_VA
        );
        assert!(!uses_projectile_surface_callback(replacement));
    }
}

#[test]
fn source_specific_ground_replacements_keep_physical_address_provenance_and_mode_zero() {
    let terrain = flat_terrain(0, 0);
    let context = TerrainCollisionContext {
        terrain: &terrain,
        terrain_objects: None,
        ground_response_selectors: [0; 8],
        water_response_selectors: [0; 8],
    };
    for (source, replacement) in SOURCE_REPLACEMENTS {
        let mut fx = WorldFx::new();
        let existing = fx
            .particles
            .allocate(descriptor_test_particle(
                replacement,
                [20.0; 3],
                [0; 3],
                None,
            ))
            .unwrap();
        let spawn = request(source, [4 << 8, 128, 2 << 8]);
        let slot = fx
            .materialize_descriptor_particle_request(
                spawn,
                ParticleEnvironment::Terrain(context),
                0,
            )
            .unwrap();
        let random_before = fx.rng_state;
        let direction_before = fx.direction_cursor;
        fx.update(ParticleUpdateRequest::terrain(20_000, 1, context));

        let converted = fx.particles.slots[slot].unwrap();
        assert_eq!(converted.source_class, replacement, "source {source}");
        assert_eq!(converted.position, [4.0, 0.0, 2.0]);
        assert_eq!(converted.velocity, [0.0; 3]);
        assert_eq!(converted.age_ticks, 0.0);
        assert_eq!(converted.owner_id, spawn.owner.map(|owner| owner.entity_id));
        assert_eq!(converted.source_entity_type_at_birth, Some(57));
        assert_eq!(fx.particles.links[existing].next, Some(slot));
        assert_eq!(fx.particles.lists[2].tail, Some(slot));

        // Although E3D0 remains in its descriptor, collision mode0 skips it
        // on the next visit. Calling it here would incorrectly change to73.
        fx.update(ParticleUpdateRequest::terrain(20_000, 2, context));
        let later = fx.particles.slots[slot].unwrap();
        assert_eq!(later.source_class, replacement);
        assert_eq!(later.position, converted.position);
        assert_eq!(later.age_ticks, 1.0);
        assert_eq!(fx.rng_state, random_before);
        assert_eq!(fx.direction_cursor, direction_before);
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn all_source_water_replacements_continue_only_the_current_mode_three_ground_tail() {
    let mut terrain = flat_terrain(0, 0);
    terrain.header[0] = 256;
    let context = TerrainCollisionContext {
        terrain: &terrain,
        terrain_objects: None,
        ground_response_selectors: [0; 8],
        water_response_selectors: [0; 8],
    };
    for (source, _) in SOURCE_REPLACEMENTS {
        let mut fx = WorldFx::new();
        let slot = fx
            .materialize_descriptor_particle_request(
                request(source, [4 << 8, 128, 2 << 8]),
                ParticleEnvironment::Terrain(context),
                0,
            )
            .unwrap();
        fx.update(ParticleUpdateRequest::terrain(20_000, 1, context));
        let converted = fx.particles.slots[slot].unwrap();
        assert_eq!(converted.source_class, 73, "source {source}");
        assert_eq!(converted.position, [4.0, 0.0, 2.0]);
        assert_eq!(converted.velocity, [0.0; 3]);
        assert_eq!(converted.age_ticks, 0.0);
        assert_eq!(converted.owner_id, Some(0x1234_0001));
        assert_eq!(fx.particles.lists[2].tail, Some(slot));
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn water_replacement_ground_child_uses_current_emitter_and_allocation_precedes_parent_free() {
    let mut terrain = flat_terrain(0, 0);
    terrain.header[0] = 256;
    let context = TerrainCollisionContext {
        terrain: &terrain,
        terrain_objects: None,
        ground_response_selectors: [7; 8],
        water_response_selectors: [0; 8],
    };
    let emitter = ParticleOwnerAtBirth {
        entity_id: 0x0423_0007,
        entity_type: 68,
    };
    for source in [52, 68] {
        for full_pool in [false, true] {
            let mut fx = WorldFx::new();
            let slot = fx
                .materialize_descriptor_particle_request(
                    request(source, [4 << 8, 128, 2 << 8]),
                    ParticleEnvironment::Terrain(context),
                    0,
                )
                .unwrap();
            fx.particles.slots[slot]
                .as_mut()
                .unwrap()
                .suppresses_impact_damage = true;
            if full_pool {
                // All other slots have priority5. The newly converted
                // priority2 parent has age0 and cannot be recycled by the
                // priority2 child. Freeing it early would wrongly admit that child.
                for _ in 1..MAX_WORLD_PARTICLES {
                    fx.particles
                        .allocate(descriptor_test_particle(68, [20.0; 3], [0; 3], None))
                        .unwrap();
                }
            }
            let random_before = fx.rng_state;
            let direction_before = fx.direction_cursor;
            fx.update(
                ParticleUpdateRequest::terrain(20_000, 1, context).with_particle_emitter(
                    ParticleEmitterContext {
                        current_owner: Some(emitter),
                    },
                ),
            );
            assert!(fx.particles.slots[slot].is_none());
            let children = fx
                .particles
                .slots
                .iter()
                .flatten()
                .filter(|particle| matches!(particle.source_class, 43 | 75))
                .collect::<Vec<_>>();
            assert_eq!(children.len(), usize::from(!full_pool));
            if let Some(child) = children.first() {
                assert_eq!(child.owner_id, Some(emitter.entity_id));
                assert_eq!(child.source_entity_type_at_birth, Some(emitter.entity_type));
                assert!(child.suppresses_impact_damage);
                assert_eq!(child.position, [4.0, 0.0, 2.0]);
            }
            assert_eq!(fx.rng_state, random_before);
            assert_eq!(fx.direction_cursor, direction_before);
            fx.particles.assert_valid_topology();
        }
    }
}
