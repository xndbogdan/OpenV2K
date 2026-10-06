use super::*;

#[test]
fn f7c0_forwards_static_packet_and_live_arguments_without_f610_or_birth_provenance() {
    for suppressed in [false, true] {
        let mut parent = descriptor_test_particle(6, [4.0, 2.0, 5.0], [1_000, -100, 300], Some(7));
        parent.source_entity_type_at_birth = Some(46);
        parent.suppresses_impact_damage = suppressed;
        let mut fx = WorldFx::new();
        let slot = fx.particles.allocate(parent).unwrap();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, parent.position, 100)];
        let mut deliveries = Vec::new();
        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &entities,
                    model_pool: &pool,
                }),
            }),
            |_, impact| {
                deliveries.push(impact);
                ParticleCollisionCacheRefresh::Unchanged
            },
            |_, _| ParticleTerrainResponse::Unhandled,
        );
        assert_eq!(deliveries, outcome.entity_impacts);
        if suppressed {
            assert!(
                deliveries.is_empty(),
                "F7C0 skips11320 entirely when+1D bit0 is set"
            );
        } else {
            let [impact] = deliveries.as_slice() else {
                panic!("{deliveries:?}")
            };
            assert_eq!(
                impact.entity_hit_entry(),
                crate::damage::EntityHitEntry::Cured
            );
            assert_eq!(
                impact.damage_delivery_record(),
                Some(crate::damage::FUN_0043F7C0_DAMAGE_DELIVERY)
            );
            assert_eq!(
                impact.damage_delivery_record().unwrap().raw_dwords(),
                [2, 0, 1000, 0, 0, 0]
            );
            assert_eq!(
                impact.impact_position_argument_va,
                retail_particle_impact_position_argument_va(slot)
            );
            assert_eq!(impact.position_world, parent.position);
            assert_eq!(impact.velocity_raw, [1000, -100, 300]);
        }
        assert!(fx.particles.slots[slot].is_none());
        assert_eq!(
            fx.particle_count(),
            0,
            "F7C0 never emitsF610 or a capability suffix"
        );
        assert!(fx.ready_sounds.is_empty());
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn f950_clears_refined_static_tile_only_and_obeys_suppression_and_change_only_writes() {
    for (terrain_type, suppressed, expected_write) in [
        (0x18, false, true),
        (0x08, false, false),
        (0x18, true, false),
    ] {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type,
        };
        let objects = terrain_object_table(1, [0, 0, 0, 0], 29);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [5; 8],
            water_response_selectors: [7; 8],
        };
        let pool = solid_sphere_pool(200);
        let mut parent = descriptor_test_particle(6, [4.0, 2.0, 5.0], [1000, 0, 0], Some(7));
        parent.suppresses_impact_damage = suppressed;
        let mut fx = WorldFx::new();
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
        assert_eq!(
            outcome.terrain_type_mutations,
            if expected_write {
                vec![ParticleTerrainMutation::Infection {
                    cell: [3, 4],
                    infected: false,
                }]
            } else {
                vec![]
            }
        );
        assert!(outcome.entity_impacts.is_empty());
        assert!(outcome.primary_impacts.is_empty());
        assert!(outcome.ballistic_static_impacts.is_empty());
        assert!(fx.particles.slots[slot].is_none());
        assert_eq!(fx.particle_count(), 0);
        assert!(fx.ready_sounds.is_empty());
        fx.particles.assert_valid_topology();
    }
}
