use super::*;

/// F890 then 41850 for a class-54 mortar shell: sound 90, the 4566E0 radial
/// (template `0x004CD7F8` with the shell's source type and owner), sixteen
/// class-0x27 records at the full particle rate whatever `+0x1D` says, then
/// F890's sound 62.
#[test]
fn class54_static_hit_runs_its_radial_between_sound_90_and_its_debris() {
    for suppressed in [false, true] {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(54, [4.0, 2.0, 5.0], [1000, 0, 0], Some(7));
        parent.source_entity_type_at_birth = Some(100);
        parent.suppresses_impact_damage = suppressed;
        let mut fx = WorldFx::new();
        fx.frame_pacing.count_scale_q16 = 0x1_0000;
        let slot = fx.particles.allocate(parent).unwrap();
        let outcome = fx.update(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
        );
        // The borrowed host cannot run it, so the update lists it.
        let [radial] = outcome.unrun_world_radials.as_slice() else {
            panic!("{:?}", outcome.unrun_world_radials)
        };
        assert_eq!(
            radial.template,
            crate::radial_damage::RadialDamageTemplate {
                inner_radius_raw: 0xFF,
                outer_radius_raw: 0x3FF,
                impulse_raw: 1000,
                packet: crate::damage::DamagePacket {
                    channels: [3, 0],
                    amounts_raw: [8000, 0],
                },
                trailing_raw: [100, 7],
            }
        );
        assert!(fx.particles.slots[slot].map_or(true, |p| p.source_class != 54));
        let debris = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|p| matches!(p.source_class, 0x27 | 0x2E))
            .inspect(|p| assert_eq!(p.suppresses_impact_damage, suppressed))
            .count();
        assert_eq!(debris, 16);
        let sounds: Vec<_> = fx
            .ready_sounds
            .iter()
            .map(|sound| sound.sound_id)
            .filter(|&id| matches!(id, 62 | 90))
            .collect();
        assert_eq!(sounds, [90, 62]);
        fx.particles.assert_valid_topology();
    }
}
