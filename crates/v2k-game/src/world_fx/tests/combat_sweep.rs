use super::*;
use crate::damage::{
    CLASS49_TURRET_BOLT_DAMAGE_PACKET, CLASS56_TURRET_BOLT_DAMAGE_PACKET,
    CLASS68_STATIC_ROUTE_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET, TURRET_BOLT_DAMAGE_PACKET,
    TYPE_47_PROJECTILE_DAMAGE_PACKET,
};
use crate::primary_weapon::fire_profile_from_descriptor;
use crate::weapon_inventory::{PowerUpPayload, WeaponInventory};

fn plasma_fire_event(selector: u8, origin_y: f32) -> PrimaryFireEvent {
    let mut inventory = WeaponInventory::new();
    inventory.acquire_weapon(PowerUpPayload::from_runtime_word(
        0x100 | u32::from(selector),
    ));
    let profile = fire_profile_from_descriptor(inventory.selected_descriptor()).unwrap();
    PrimaryWeapon::new().update_with_profile(
        Duration::ZERO,
        PrimaryTriggerInput {
            source_a: true,
            source_b: false,
        },
        primary_geometry(origin_y, [0.0; 3]),
        profile,
        PrimaryShotBudget::Limited(1),
    )[0]
}

#[test]
fn player_plasma_fire_selects_each_410b0_water_pair_before_first_update() {
    for (selector, airborne, submerged) in [(0x0c, 49, 80), (0x0d, 56, 82), (0x0e, 55, 81)] {
        for (origin_y, water, expected) in [
            (-1.0, None, airborne),
            (-1.0, Some(0.0), submerged),
            (0.0, Some(0.0), submerged),
            (1.0 / 256.0, Some(0.0), airborne),
        ] {
            let mut fx = WorldFx::new();
            fx.queue_primary_fire_batch(&[plasma_fire_event(selector, origin_y)], water);
            fx.process_pending();
            let particles = fx.test_particles_in_virgin_birth_order();
            assert_eq!(particles.len(), 1, "plasma has no auxiliary muzzle command");
            let particle = particles[0];
            assert_eq!(particle.source_class, expected, "selector {selector:#x}");
            assert_eq!(particle.owner_id, Some(7));
            assert_eq!(particle.source_entity_type_at_birth, Some(46));
            assert_eq!(particle.position, [4.0, origin_y, 5.0]);
            assert_eq!(
                particle.current_sprite_id(),
                particle_frame(expected, 0).unwrap().sprite_id
            );
        }
    }
}

#[test]
fn green_player_fire_sweeps_air_and_water_and_delivers_its_own_packet() {
    for (origin_y, water, expected_class) in [(2.0, None, 56), (-1.0, Some(0.0), 82)] {
        let mut fx = WorldFx::new();
        fx.queue_primary_fire_batch(&[plasma_fire_event(0x0d, origin_y)], water);
        fx.process_pending();
        let pool = solid_sphere_pool(100);
        let entities = [collision_entity(9, [4.0, origin_y, 5.0], 80)];
        let request = match water {
            None => ParticleUpdateRequest::dry(0, 0),
            Some(sea) => ParticleUpdateRequest::flat_water(0, 0, sea),
        };
        let outcome = fx.update(request.with_callbacks(ParticleCallbackContext {
            owner_motions: &[],
            collision: Some(ParticleCollisionContext {
                entities: &entities,
                model_pool: &pool,
            }),
        }));
        assert_eq!(outcome.entity_impacts.len(), 1);
        let impact = outcome.entity_impacts[0];
        assert_eq!(impact.source_particle_class, expected_class);
        assert_eq!(impact.target_entity_id, 9);
        assert_eq!(
            impact.damage_delivery_record().unwrap().raw_dwords(),
            [2, 3, 4000, 1500, 46, 7]
        );
        assert!(!fx
            .particles
            .slots
            .iter()
            .flatten()
            .any(|p| matches!(p.source_class, 56 | 82)));
    }
}

#[test]
fn combat_sweep_preserves_descriptor_packet_provenance_and_endpoint() {
    for (class, source_type, packet) in [
        (38, 10, DRAGON_FIREBALL_DAMAGE_PACKET),
        (49, 97, CLASS49_TURRET_BOLT_DAMAGE_PACKET),
        (80, 97, CLASS49_TURRET_BOLT_DAMAGE_PACKET),
        (55, 102, TURRET_BOLT_DAMAGE_PACKET),
        (81, 102, TURRET_BOLT_DAMAGE_PACKET),
        (56, 103, CLASS56_TURRET_BOLT_DAMAGE_PACKET),
        (82, 103, CLASS56_TURRET_BOLT_DAMAGE_PACKET),
    ] {
        let mut fx = WorldFx::new();
        let slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: class,
                    position_raw: [0, 1000, 0],
                    velocity_raw: [300, 400, 500],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 7,
                        entity_type: source_type,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        let mut particle = fx.particles.slots[slot].unwrap();
        let endpoint = particle.position;
        let pool = solid_sphere_pool(100);
        let models = [collision_entity(9, endpoint, 100)];
        let program = ProjectileModelSweepProgram::for_class(class).unwrap();
        let Some(BallisticSweepImpact::Entity(hit)) = detect_damage_projectile_sweep(
            program,
            slot,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &models,
                    model_pool: &pool,
                },
                terrain_context: None,
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        ) else {
            panic!("class{class} must use its F590 callback")
        };
        assert_eq!(hit.position_world, endpoint);
        assert_eq!(
            hit.impact_position_argument_va,
            retail_particle_impact_position_argument_va(slot)
        );
        assert_eq!(hit.target_entity_id, 9);
        assert_eq!(
            hit.damage_delivery_record(),
            Some(DamageDeliveryRecord {
                packet,
                source_entity_type_raw: u32::from(source_type),
                owner_handle: 7,
            })
        );

        particle.suppresses_impact_damage = true;
        let Some(BallisticSweepImpact::Entity(hit)) = detect_damage_projectile_sweep(
            program,
            slot,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &models,
                    model_pool: &pool,
                },
                terrain_context: None,
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        ) else {
            panic!("suppression must retain the collision/F610/deletion path")
        };
        assert!(hit.damage.is_none());
    }
    // These descriptors have separately owned surface or static callback policies.
    assert!(ProjectileModelSweepProgram::for_class(1).is_none());
    assert!(ProjectileModelSweepProgram::for_class(5).is_none());
    assert!(ProjectileModelSweepProgram::for_class(87).is_none());
}

#[test]
fn static_route_gate_covers_exactly_classes_52_68_85() {
    for class in 0..PARTICLE_DESCRIPTORS.len() as u8 {
        assert_eq!(
            particle_uses_static_route_entity_hit(class),
            matches!(class, 52 | 68 | 85),
            "class{class} static-route gate"
        );
    }
    // Shared packet rows stay in their own families: class 87 keeps
    // FUN_0043F590 and dragon class 38 keeps its F590 program.
    assert!(matches!(
        ProjectileModelSweepProgram::for_class(38),
        Some(ProjectileModelSweepProgram::DragonFireball)
    ));
    assert!(ProjectileModelSweepProgram::for_class(87).is_none());
    assert!(matches!(
        ProjectileModelSweepProgram::for_class(52),
        Some(ProjectileModelSweepProgram::StaticRoute {
            static_noop: true,
            ..
        })
    ));
    assert!(matches!(
        ProjectileModelSweepProgram::for_class(68),
        Some(ProjectileModelSweepProgram::StaticRoute {
            static_noop: true,
            ..
        })
    ));
    assert!(matches!(
        ProjectileModelSweepProgram::for_class(85),
        Some(ProjectileModelSweepProgram::StaticRoute {
            static_noop: false,
            ..
        })
    ));
}

#[test]
fn type97_bolt_static_sweep_retains_49_80_packet_and_birth_provenance() {
    for class in [49, 80] {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0; 4], KIND_9_STATIC_OBJECT);
        let center = raw_position_to_world(static_tile_center_raw(&terrain, 3, 4));
        let mut particle = descriptor_test_particle(class, center, [0; 3], Some(7));
        particle.source_entity_type_at_birth = Some(97);
        let pool = solid_sphere_pool(100);
        let program = ProjectileModelSweepProgram::for_class(class).unwrap();
        let Some(BallisticSweepImpact::StaticTile(hit)) = detect_damage_projectile_sweep(
            program,
            0,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                },
                terrain_context: Some(TerrainCollisionContext {
                    terrain: &terrain,
                    terrain_objects: Some(&objects),
                    ground_response_selectors: [0; 8],
                    water_response_selectors: [0; 8],
                }),
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        ) else {
            panic!("class{class} must deliver its F800 packet")
        };
        assert_eq!(hit.source_particle_class, class);
        assert_eq!(hit.source_owner_id, Some(7));
        assert_eq!(
            hit.damage,
            Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 3],
                    amounts_raw: [6000, 2000]
                },
                source_entity_type_at_birth: Some(97),
                source_owner_id: Some(7),
            })
        );
    }
}

#[test]
fn static_route_sweep_emits_descriptor_packet_provenance_and_endpoint() {
    // Class68 is Type57 bat fire (entity type 57). Class52/85 firers are
    // unaudited; zero is scaffold provenance for the packet-identity check.
    for (class, source_type, packet) in [
        (52, 0, TYPE_47_PROJECTILE_DAMAGE_PACKET),
        (68, 57, CLASS68_STATIC_ROUTE_DAMAGE_PACKET),
        (85, 0, DRAGON_FIREBALL_DAMAGE_PACKET),
    ] {
        let mut fx = WorldFx::new();
        let slot = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: class,
                    position_raw: [0, 1000, 0],
                    velocity_raw: [300, 400, 500],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: 7,
                        entity_type: source_type,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        let mut particle = fx.particles.slots[slot].unwrap();
        let endpoint = particle.position;
        let pool = solid_sphere_pool(100);
        let models = [collision_entity(9, endpoint, 100)];
        let program = ProjectileModelSweepProgram::for_class(class).unwrap();
        let Some(BallisticSweepImpact::Entity(hit)) = detect_damage_projectile_sweep(
            program,
            slot,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &models,
                    model_pool: &pool,
                },
                terrain_context: None,
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        ) else {
            panic!("class{class} must use its 442950 callback")
        };
        assert_eq!(hit.position_world, endpoint);
        assert_eq!(
            hit.impact_position_argument_va,
            retail_particle_impact_position_argument_va(slot)
        );
        assert_eq!(hit.target_entity_id, 9);
        assert_eq!(
            hit.damage_delivery_record(),
            Some(DamageDeliveryRecord {
                packet,
                source_entity_type_raw: u32::from(source_type),
                owner_handle: 7,
            })
        );

        particle.suppresses_impact_damage = true;
        let Some(BallisticSweepImpact::Entity(hit)) = detect_damage_projectile_sweep(
            program,
            slot,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &models,
                    model_pool: &pool,
                },
                terrain_context: None,
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        ) else {
            panic!("suppression must retain the collision/F610/deletion path")
        };
        assert_eq!(
            hit.damage_delivery_record().unwrap().packet,
            packet,
            "442950 suppression gates attachment, not 11180 damage"
        );
    }
}

#[test]
fn static_route_orders_damage_before_f610_and_attachment_for_all_three_classes() {
    for (class, attached_class) in [(52, 83), (68, 84), (85, 86)] {
        for suppressed in [false, true] {
            let mut fx = WorldFx::new();
            let pool = solid_sphere_pool(100);
            let models = [collision_entity(9, [0.0, 4.0, 0.0], 100)];
            let slot = fx
                .materialize_descriptor_particle_request(
                    DescriptorParticleRequest {
                        source_class: class,
                        position_raw: [0, 1024, 0],
                        velocity_raw: [1, 0, 1],
                        owner: Some(ParticleOwnerAtBirth {
                            entity_id: 7,
                            entity_type: 57,
                        }),
                        suppresses_impact_damage: false,
                    },
                    ParticleEnvironment::Dry,
                    0,
                )
                .unwrap();
            // These descriptors reject suppressed births; a live flag change
            // still must retain 442950's unconditional 11180 delivery.
            fx.particles.slots[slot]
                .as_mut()
                .unwrap()
                .suppresses_impact_damage = suppressed;
            let mut called = 0;
            let outcome = fx.update_with_event_handlers(
                ParticleUpdateRequest::dry(0, 0).with_callbacks(ParticleCallbackContext {
                    owner_motions: &[],
                    collision: Some(ParticleCollisionContext {
                        entities: &models,
                        model_pool: &pool,
                    }),
                }),
                |fx, impact| {
                    called += 1;
                    assert!(impact.damage.is_some(), "suppression must preserve 11180");
                    assert_eq!(
                        fx.particle_count(),
                        1,
                        "F610 must follow synchronous damage"
                    );
                    let mut target = models[0].clone();
                    target.center_world = [1.0, 5.0, 1.0];
                    target.capability_flags_at_0x64 = 1;
                    ParticleCollisionCacheRefresh::Replace(vec![target])
                },
                |_, _| ParticleTerrainResponse::Unhandled,
            );
            assert_eq!(called, 1);
            assert_eq!(outcome.entity_impacts.len(), 1);
            let children = fx.test_particles_in_virgin_birth_order();
            assert_eq!(
                children
                    .iter()
                    .filter(|child| child.source_class == class)
                    .count(),
                0
            );
            let attached = children
                .iter()
                .find(|child| child.source_class == attached_class);
            assert_eq!(attached.is_some(), !suppressed);
            if let Some(attached) = attached {
                assert_eq!(attached.attached_offset_raw, [-192, 0, -192]);
                let burst = children
                    .iter()
                    .position(|child| child.source_class == PRIMARY_ABOVE_WATER_IMPACT_CLASS)
                    .unwrap();
                let attachment = children
                    .iter()
                    .position(|child| child.source_class == attached_class)
                    .unwrap();
                assert!(
                    burst < attachment,
                    "F610 must allocate before attached child"
                );
            }
        }
    }
}

#[test]
fn static_route_static_contact_consumes_52_68_and_delivers_f800_for_85() {
    // Aim each probe exactly at the kind-9 object's authored center so the
    // small static-route radii overlap regardless of descriptor size.
    for class in [52u8, 68] {
        let mut terrain = flat_terrain(16, 0);
        *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
            height: 16,
            attribute: 1,
            terrain_type: 0x18,
        };
        let objects = terrain_object_table(1, [0; 4], KIND_9_STATIC_OBJECT);
        let center_world = raw_position_to_world(static_tile_center_raw(&terrain, 3, 4));
        let mut particle = descriptor_test_particle(class, center_world, [0; 3], Some(7));
        let pool = solid_sphere_pool(100);
        let program = ProjectileModelSweepProgram::for_class(class).unwrap();
        let impact = detect_damage_projectile_sweep(
            program,
            0,
            &mut particle,
            ProjectileSweepFrame {
                collision: ParticleCollisionContext {
                    entities: &[],
                    model_pool: &pool,
                },
                terrain_context: Some(TerrainCollisionContext {
                    terrain: &terrain,
                    terrain_objects: Some(&objects),
                    ground_response_selectors: [0; 8],
                    water_response_selectors: [0; 8],
                }),
                retail_tick: 0,
                terrain_type_mutations: &[],
            },
        );
        assert!(
            matches!(
                impact,
                Some(BallisticSweepImpact::ConsumedStaticNoOp { .. })
            ),
            "class{class} static contact must consume silently via 42E8E0 (got {impact:?})",
        );
    }

    // Class85 shares dragon class-38's packet row and FUN_0043F800 static slot.
    let mut terrain = flat_terrain(16, 0);
    *terrain_cell_mut(&mut terrain, 3, 4) = TerrainCell {
        height: 16,
        attribute: 1,
        terrain_type: 0x18,
    };
    let objects = terrain_object_table(1, [0; 4], KIND_9_STATIC_OBJECT);
    let center_world = raw_position_to_world(static_tile_center_raw(&terrain, 3, 4));
    let mut particle = descriptor_test_particle(85, center_world, [0; 3], Some(7));
    let pool = solid_sphere_pool(100);
    let program = ProjectileModelSweepProgram::for_class(85).unwrap();
    let Some(BallisticSweepImpact::StaticTile(static_hit)) = detect_damage_projectile_sweep(
        program,
        0,
        &mut particle,
        ProjectileSweepFrame {
            collision: ParticleCollisionContext {
                entities: &[],
                model_pool: &pool,
            },
            terrain_context: Some(TerrainCollisionContext {
                terrain: &terrain,
                terrain_objects: Some(&objects),
                ground_response_selectors: [0; 8],
                water_response_selectors: [0; 8],
            }),
            retail_tick: 0,
            terrain_type_mutations: &[],
        },
    ) else {
        panic!("class85 static contact must deliver F800");
    };
    assert_eq!(
        static_hit.damage.map(|request| request.packet),
        Some(DRAGON_FIREBALL_DAMAGE_PACKET)
    );
}

#[test]
fn descriptor_constructor_applies_signed_word_bias_before_classification() {
    let mut fx = WorldFx::new();
    let slot = fx
        .materialize_descriptor_particle_request(
            DescriptorParticleRequest {
                source_class: 46,
                position_raw: [123, i16::MAX - 20, -456],
                velocity_raw: [17, 32760, -19],
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: 7,
                    entity_type: 10,
                }),
                suppresses_impact_damage: false,
            },
            ParticleEnvironment::FlatWater { sea_level: 0.0 },
            0,
        )
        .unwrap();
    let particle = fx.particles.slots[slot].unwrap();
    let descriptor = particle_descriptor(46).unwrap();
    assert_eq!(descriptor.spawn_position_y_bias_raw(), 100);
    assert_eq!(
        world_position_to_raw(particle.position),
        [123, (i16::MAX - 20).wrapping_add(100), -456]
    );
    assert_eq!(
        particle.water_state, 0,
        "classify the wrapped, biased birth position"
    );
    assert_eq!(
        world_velocity_component_to_raw(particle.velocity[1]) as i16,
        32760_i16.wrapping_add(descriptor.spawn_velocity_y_bias_raw())
    );
    assert!(!particle.pending_destruction);
}
