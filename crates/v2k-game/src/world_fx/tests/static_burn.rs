use super::*;

#[test]
fn primary_immediate_burn_changes_later_f800_model_and_target_in_same_traversal() {
    use crate::static_damage::KIND_4_STATIC_OBJECT;
    // With a surviving burned model, F800 must reread its burned state. A
    // burned model without collision must remove the later static hit entirely.
    for burned_radius in [200, 0] {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x10,
        };
        let objects = terrain_object_table(1, [0, 1, 0, 1], KIND_4_STATIC_OBJECT);
        let context = TerrainCollisionContext {
            terrain: &terrain,
            terrain_objects: Some(&objects),
            ground_response_selectors: [0; 8],
            water_response_selectors: [0; 8],
        };
        let mut pool = solid_sphere_pool(200);
        pool.0.push(solid_sphere_pool(burned_radius).0.remove(0));
        let mut fx = WorldFx::new();
        for owner in [22, 11] {
            let mut particle = descriptor_test_particle(
                PRIMARY_BULLET_PARTICLE_CLASS,
                [4.0, 2.0, 5.0],
                [1_000, 0, 0],
                Some(owner),
            );
            particle.source_entity_type_at_birth = Some(46);
            fx.particles.allocate(particle).unwrap();
        }
        let mut scheduler = StaticDamageScheduler::new();
        let mut hits = Vec::new();
        let outcome = fx.update_with_event_handlers(
            ParticleUpdateRequest::terrain(0, 0, context).with_callbacks(ParticleCallbackContext {
                owner_motions: &[],
                collision: Some(ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                }),
            }),
            |_, impact| panic!("unexpected entity impact {impact:?}"),
            |fx, event| {
                let impact = expect_static_event(event);
                let current = impact.current.expect("unsuppressed F800 target");
                let result = scheduler.submit_hit(
                    current.target,
                    impact.damage.unwrap().packet,
                    &mut || fx.next_shared_retail_random_u16(),
                );
                hits.push((impact, result));
                ParticleTerrainResponse::StaticDamage(result)
            },
        );
        assert_eq!(hits[0].0.source_owner_id, Some(11));
        assert_eq!(hits[0].0.model_id, 0);
        assert_eq!(
            hits[0].1,
            StaticDamageOutcome::ImmediateBurn {
                cell: [3, 4],
                severity_raw: 3000,
                sample: None,
            }
        );
        assert_eq!(
            outcome.terrain_type_mutations,
            [ParticleTerrainMutation::ImmediateBurn { cell: [3, 4] }]
        );
        assert_eq!(scheduler.active_program_count(), 0);
        if burned_radius == 0 {
            assert_eq!(
                hits.len(),
                1,
                "the later slot must see the burned model's empty collision"
            );
        } else {
            assert_eq!(hits.len(), 2);
            assert_eq!(hits[1].0.source_owner_id, Some(22));
            assert_eq!(hits[1].0.model_id, 1);
            assert_eq!(hits[1].0.terrain_type, 0x18);
            assert_eq!(hits[1].0.current.unwrap().target.state.terrain_type, 0x18);
            assert_eq!(
                hits[1].1,
                StaticDamageOutcome::BurnedIgnored {
                    severity_raw: 1500,
                    sample: None
                }
            );
        }
        assert_eq!(
            terrain.cell(3, 4).unwrap().terrain_type,
            0x10,
            "the borrowed host returns its exact commit journal"
        );
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn burn_overlay_composes_with_ordered_infection_and_does_not_touch_sibling_bits() {
    let cell = [3, 4];
    let writes = [
        ParticleTerrainMutation::Infection {
            cell,
            infected: true,
        },
        ParticleTerrainMutation::ImmediateBurn { cell },
        ParticleTerrainMutation::Infection {
            cell,
            infected: false,
        },
    ];
    assert_eq!(effective_terrain_type(0xe5, cell, &writes[..1]), 0xf5);
    assert_eq!(effective_terrain_type(0xe5, cell, &writes[..2]), 0xfd);
    assert_eq!(effective_terrain_type(0xe5, cell, &writes), 0xed);
    assert_eq!(effective_terrain_type(0xe5, [4, 3], &writes), 0xe5);
}
