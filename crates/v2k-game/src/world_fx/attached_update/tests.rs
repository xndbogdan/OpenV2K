use super::*;
use crate::common_mover::type9_attitude::Type9BodyBasis;

struct Host {
    owner: Option<AttachedParticleOwnerState>,
    mass: u16,
    sea: Option<f32>,
    terrain: Option<v2k_formats::terrain::TerrainGrid>,
    emitter: Option<ParticleOwnerAtBirth>,
    requests: Vec<AttachedParticleOwnerRequest>,
}

impl Host {
    fn new(rotated: bool) -> Self {
        Self {
            owner: Some(AttachedParticleOwnerState {
                follow: ParticleAttachmentOwner {
                    entity_id: 7,
                    position_raw: [1000, 2000, 3000],
                    state_flags_at_0x08: Some(if rotated {
                        ATTACHED_FOLLOW_ROTATED_STATE_BIT
                    } else {
                        0
                    }),
                    capability_flags_at_0x64: 0,
                    basis: ParticleAttachmentBasis::Native(Type9BodyBasis {
                        lateral: [i32::MAX, 0, 0],
                        up: [0, i32::MAX, 0],
                        forward: [0, 0, i32::MAX],
                    }),
                },
                pre_health_buffer_raw: RetailRuntimeValue::Known(0),
                checked_damage_enabled: RetailRuntimeValue::Known(false),
                velocity_raw: [-701, -203, 901],
                allocation_owner: ParticleOwnerAtBirth {
                    entity_id: 7,
                    entity_type: 9,
                },
                cached_allocation: None,
            }),
            mass: 0,
            sea: None,
            terrain: None,
            emitter: Some(ParticleOwnerAtBirth {
                entity_id: 55,
                entity_type: 57,
            }),
            requests: vec![],
        }
    }
}

impl ParticleTraversalHost for Host {
    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: if let Some(terrain) = &self.terrain {
                ParticleEnvironment::Terrain(TerrainCollisionContext {
                    terrain,
                    terrain_objects: None,
                    ground_response_selectors: [0; 8],
                    water_response_selectors: [0; 8],
                })
            } else {
                self.sea.map_or(ParticleEnvironment::Dry, |sea_level| {
                    ParticleEnvironment::FlatWater { sea_level }
                })
            },
            callbacks: ParticleCallbackContext::default(),
            particle_emitter: ParticleEmitterContext {
                current_owner: self.emitter,
            },
        }
    }
    fn entity_impact(&mut self, _: &mut WorldFx, _: ParticleEntityImpact) {
        panic!("mode0 has no entity contact");
    }
    fn static_impact(
        &mut self,
        _: &mut WorldFx,
        _: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        panic!("mode0 has no static contact");
    }
    fn begin_attached_update(
        &mut self,
        _: &mut WorldFx,
        request: AttachedParticleOwnerRequest,
    ) -> AttachedParticleOwnerLookup {
        self.requests.push(request);
        self.owner
            .map_or(AttachedParticleOwnerLookup::Missing, |owner| {
                assert_eq!(request.owner_handle, owner.follow.entity_id);
                self.mass = self.mass.wrapping_add(request.mass_increment_raw());
                AttachedParticleOwnerLookup::Updated(owner)
            })
    }
}

fn attached(fx: &mut WorldFx, class: u8, age: u8) -> usize {
    let slot = fx
        .emit_attached_static_particle_raw(AttachedStaticEmission {
            target_handle: 7,
            position_world: [1.0, 2.0, 3.0],
            offset_raw: [256, -512, 768],
            particle_class: class,
        })
        .unwrap()
        .slot;
    fx.particles.slots[slot].as_mut().unwrap().age_ticks = f32::from(age);
    slot
}

fn update(fx: &mut WorldFx, host: &mut Host, tick: u32, micros: u32) -> ParticleUpdateOutcome {
    fx.update_with_traversal_host(
        ParticleTraversalTiming {
            retail_tick: tick,
            elapsed_micros: micros,
        },
        host,
    )
}

fn seed_for(fx: &mut WorldFx, predicate: impl Fn(u16) -> bool) {
    for seed in 1..10000 {
        let mut state = seed;
        if predicate(retail_random_u16(&mut state)) {
            fx.rng_state = seed;
            return;
        }
    }
    panic!("controlled RNG fixture has no seed");
}

#[test]
fn missing_owner_marks_until_next_visit_without_mass_or_rng() {
    let mut fx = WorldFx::new();
    let slot = attached(&mut fx, 84, 5);
    let mut host = Host::new(false);
    host.owner = None;
    let rng = fx.rng_state;
    let result = update(&mut fx, &mut host, 20, 0);
    assert!(result.attached_updates.is_empty());
    assert!(result.blocked_attached_updates.is_empty());
    assert!(fx.particles.slots[slot].unwrap().pending_destruction);
    assert_eq!(fx.particle_count(), 1);
    assert_eq!((host.mass, fx.rng_state), (0, rng));
    update(&mut fx, &mut host, 20, 0);
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(host.requests.len(), 1);
}

#[test]
fn all_three_descriptor_packets_follow_and_increment_wrapping_mass_after_age() {
    for (class, packet) in [(83, 0x004c_c0f0), (84, 0x004c_c108), (86, 0x004c_c048)] {
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, class, 5);
        fx.last_retail_tick = Some(20);
        let mut host = Host::new(false);
        host.mass = u16::MAX - 10;
        let rng = fx.rng_state;
        let result = update(&mut fx, &mut host, 21, 0);
        let [observed] = result.attached_updates.as_slice() else {
            panic!("{result:?}")
        };
        assert_eq!(host.requests[0].age_byte, 6);
        assert_eq!(host.mass, 51);
        assert_eq!(observed.mass_increment_raw, 62);
        assert_eq!(
            observed.damage_request,
            AttachedParticleDamageRequest {
                target_handle: 7,
                packet_va: packet,
                source_entity_type_raw: 0xffff_fffb,
                current_emitter: host.emitter,
                ratio_numerator: 1,
                ratio_denominator: 255,
            }
        );
        assert_eq!(
            fx.particles.slots[slot].unwrap().position,
            raw_position_to_world([1000, 2000, 3000])
        );
        assert_eq!(
            fx.rng_state, rng,
            "direct follow cannot consume the rotated RNG"
        );
    }
}

#[test]
fn rotated_q31_follow_consumes_rng_even_at_zero_delta_and_under_suppression() {
    let mut fx = WorldFx::new();
    let slot = attached(&mut fx, 84, 5);
    fx.particles.slots[slot]
        .as_mut()
        .unwrap()
        .suppresses_impact_damage = true;
    let mut host = Host::new(true);
    let mut expected = fx.rng_state;
    retail_random_u16(&mut expected);
    let result = update(&mut fx, &mut host, 20, 0);
    assert!(result.blocked_attached_updates.is_empty());
    assert_eq!(fx.rng_state, expected);
    assert_eq!(fx.particle_count(), 1);
    assert_eq!(
        world_position_to_raw(fx.particles.slots[slot].unwrap().position),
        [1255, 1488, 3767]
    );
}

#[test]
fn rotated_effect_gate_compares_elapsed_microseconds_as_a_signed_dword() {
    let mut fx = WorldFx::new();
    let slot = attached(&mut fx, 84, 5);
    fx.particles.slots[slot]
        .as_mut()
        .unwrap()
        .suppresses_impact_damage = true;
    let mut host = Host::new(true);
    let mut expected = fx.rng_state;
    retail_random_u16(&mut expected);
    // The public traversal caps elapsed time. Exercise 425D0's signed ABI
    // directly so that cap cannot hide a wrong comparison at this boundary.
    let mut result = ParticleUpdateOutcome::default();
    fx.update_attached_particle_slot(
        slot,
        ParticleTraversalTiming {
            retail_tick: 20,
            elapsed_micros: i32::MIN as u32,
        },
        0,
        &mut host,
        &mut result,
    );
    assert!(result.blocked_attached_updates.is_empty());
    assert_eq!(fx.rng_state, expected);
    assert_eq!(fx.particle_count(), 1);
}

#[test]
fn rotated_4417e0_uses_flat_sea_equality_current_emitter_and_descriptor_birth_biases() {
    for (owner_y, child_class) in [(1, 31), (0, 42), (-1, 42)] {
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, 84, 5);
        fx.particles.slots[slot]
            .as_mut()
            .unwrap()
            .attached_offset_raw = [0; 3];
        fx.particles.slots[slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = true;
        seed_for(&mut fx, |word| {
            u32::from(word) * 2 < RETAIL_FRAME_DELTA_MAX_US
        });
        let mut expected = fx.rng_state;
        retail_random_u16(&mut expected);
        let mut host = Host::new(true);
        host.owner.as_mut().unwrap().follow.position_raw[1] = owner_y;
        host.sea = Some(0.0);
        let result = update(&mut fx, &mut host, 20, RETAIL_FRAME_DELTA_MAX_US);
        assert!(result.blocked_attached_updates.is_empty());
        let children: Vec<_> = fx
            .particles
            .slots
            .iter()
            .flatten()
            .filter(|p| p.source_class == child_class)
            .collect();
        assert_eq!(children.len(), 1);
        let child = children[0];
        assert_eq!(child.owner_id, Some(55));
        assert_eq!(child.source_entity_type_at_birth, Some(57));
        assert!(child.suppresses_impact_damage);
        assert_eq!(
            world_position_to_raw(child.position)[1],
            owner_y.wrapping_add(
                particle_descriptor(child_class)
                    .unwrap()
                    .spawn_position_y_bias_raw()
            )
        );
        assert_eq!(
            world_velocity_component_to_raw(child.velocity[1]) as i16,
            particle_descriptor(child_class)
                .unwrap()
                .spawn_velocity_y_bias_raw()
        );
        assert_eq!(fx.rng_state, expected);
    }
}

#[test]
fn positive_pre_health_buffer_clamps_age_despite_disabled_damage_and_expires_next_visit() {
    let mut fx = WorldFx::new();
    let slot = attached(&mut fx, 84, 5);
    fx.last_retail_tick = Some(20);
    let mut host = Host::new(false);
    host.owner.as_mut().unwrap().pre_health_buffer_raw = RetailRuntimeValue::Known(100);
    let rng = fx.rng_state;
    let result = update(&mut fx, &mut host, 21, 0);
    assert_eq!(
        result.attached_updates[0].damage_request.ratio_numerator,
        249
    );
    assert_eq!(result.attached_updates[0].age_byte_after_damage, 255);
    assert_eq!(fx.particles.slots[slot].unwrap().age_ticks, 255.0);
    assert_eq!(fx.rng_state, rng);
    update(&mut fx, &mut host, 22, 0);
    assert!(fx.particles.slots[slot].is_none());
    assert_eq!(host.requests.len(), 1);
}

#[test]
fn cascade_matrix_uses_full_tick_crossing_target_provenance_and_signed_inherited_velocity() {
    for (class, child_class) in [(83, 53), (84, 69), (86, 39)] {
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, class, 15);
        fx.last_retail_tick = Some(20);
        seed_for(&mut fx, |word| word & 1 != 0);
        let mut expected = fx.rng_state;
        retail_random_u16(&mut expected);
        let mut host = Host::new(false);
        let result = update(&mut fx, &mut host, 21, 0);
        assert!(result.blocked_attached_updates.is_empty());
        assert_eq!(fx.particles.slots[slot].unwrap().age_ticks, 16.0);
        let child = fx
            .particles
            .slots
            .iter()
            .flatten()
            .find(|p| p.source_class == child_class)
            .unwrap();
        assert_eq!(
            (child.owner_id, child.source_entity_type_at_birth),
            (Some(7), Some(9))
        );
        assert_eq!(
            child.age_ticks, 0.0,
            "the earlier physical child waits until the next traversal"
        );
        assert_eq!(
            child
                .velocity
                .map(|v| world_velocity_component_to_raw(v) as i16),
            [-350, if child_class == 39 { -203 } else { -253 }, 450]
        );
        assert_eq!(fx.rng_state, expected);
    }
}

#[test]
fn pre_follow_underwater_mark_does_not_skip_cascade_at_followed_position() {
    let mut fx = WorldFx::new();
    let slot = attached(&mut fx, 84, 15);
    fx.particles.slots[slot].as_mut().unwrap().position[1] = -2.0;
    fx.last_retail_tick = Some(20);
    seed_for(&mut fx, |word| word & 1 != 0);
    let mut host = Host::new(false);
    host.sea = Some(0.0);
    let result = update(&mut fx, &mut host, 21, 0);
    assert_eq!(result.attached_updates.len(), 1);
    let parent = fx.particles.slots[slot].unwrap();
    assert!(parent.pending_destruction);
    assert_eq!(parent.position, raw_position_to_world([1000, 2000, 3000]));
    assert!(fx
        .particles
        .slots
        .iter()
        .flatten()
        .any(|p| p.source_class == 69 && p.position == parent.position));
    update(&mut fx, &mut host, 21, 0);
    assert!(fx.particles.slots[slot].is_none());
}

#[test]
fn enabled_and_unresolved_damage_retire_with_committed_prefix_and_no_replay() {
    for (admission, reason) in [
        (
            RetailRuntimeValue::Known(true),
            AttachedParticleUpdateBlock::Damage(Box::new(
                crate::attached_particle_damage::AttachedParticleDamageBlock::MutableOwnerUnavailable)),
        ),
        (
            RetailRuntimeValue::Unresolved,
            AttachedParticleUpdateBlock::CheckedDamageAdmissionUnavailable,
        ),
    ] {
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, 84, 5);
        fx.last_retail_tick = Some(20);
        let mut host = Host::new(false);
        host.owner.as_mut().unwrap().checked_damage_enabled = admission;
        let result = update(&mut fx, &mut host, 21, 0);
        assert!(result.attached_updates.is_empty());
        let [diagnostic] = result.blocked_attached_updates.as_slice() else {
            panic!("{result:?}")
        };
        assert_eq!(diagnostic.reason, reason);
        assert!(diagnostic.committed_prefix);
        assert_eq!(diagnostic.damage_request.unwrap().ratio_numerator, 1);
        assert_eq!(host.mass, 62);
        assert!(fx.particles.slots[slot].is_none());
        update(&mut fx, &mut host, 22, 0);
        assert_eq!(host.requests.len(), 1);
        assert_eq!(host.mass, 62);
    }
}

#[test]
fn cascade_cannot_recycle_priority6_parent_and_consumes_rng_on_allocation_failure() {
    for parent_age in [0, 16] {
        let mut fx = WorldFx::new();
        let parent = attached(&mut fx, 84, parent_age);
        for _ in 1..MAX_WORLD_PARTICLES {
            attached(&mut fx, 84, 0);
        }
        seed_for(&mut fx, |word| word & 1 != 0);
        let mut expected = fx.rng_state;
        retail_random_u16(&mut expected);
        let mut host = Host::new(false);
        let mut result = ParticleUpdateOutcome::default();
        fx.update_attached_particle_slot(
            parent,
            ParticleTraversalTiming {
                retail_tick: 21,
                elapsed_micros: 0,
            },
            16,
            &mut host,
            &mut result,
        );
        assert!(result.blocked_attached_updates.is_empty());
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
        let current = fx.particles.slots[parent].unwrap();
        assert_eq!(current.source_class, 84);
        assert_eq!(current.age_ticks, f32::from(parent_age));
        assert_eq!(
            fx.rng_state, expected,
            "a rejected allocation still consumed the cascade gate word"
        );
    }
}

#[test]
fn requested_effect_requires_static_sea_and_water_disabled_terrain_still_uses_its_header() {
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
    for terrain_available in [false, true] {
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, 84, 5);
        fx.particles.slots[slot]
            .as_mut()
            .unwrap()
            .attached_offset_raw = [0; 3];
        seed_for(&mut fx, |word| {
            u32::from(word) * 2 < RETAIL_FRAME_DELTA_MAX_US
        });
        let mut expected_rng = fx.rng_state;
        retail_random_u16(&mut expected_rng);
        let mut host = Host::new(true);
        host.owner.as_mut().unwrap().follow.position_raw[1] = -6144;
        if terrain_available {
            host.terrain = Some(TerrainGrid {
                header: [-6144 << 8, 0, 0, 0, 0],
                cells: vec![
                    TerrainCell {
                        height: 0,
                        attribute: 0,
                        terrain_type: 0
                    };
                    GRID_SIZE * GRID_SIZE
                ],
            });
            assert!(!host.terrain.as_ref().unwrap().water_enabled());
        }
        let result = update(&mut fx, &mut host, 20, RETAIL_FRAME_DELTA_MAX_US);
        assert_eq!(fx.rng_state, expected_rng);
        if terrain_available {
            assert!(result.blocked_attached_updates.is_empty());
            let parent = fx.particles.slots[slot].unwrap();
            assert!(
                !parent.pending_destruction,
                "water classification stays dry despite the authored sea comparison"
            );
            assert!(fx
                .particles
                .slots
                .iter()
                .flatten()
                .any(|child| child.source_class == 42));
        } else {
            let [diagnostic] = result.blocked_attached_updates.as_slice() else {
                panic!("{result:?}")
            };
            assert_eq!(
                diagnostic.reason,
                AttachedParticleUpdateBlock::StaticSeaPlaneUnavailable
            );
            assert!(diagnostic.committed_prefix);
            assert!(fx.particles.slots[slot].is_none());
            assert_eq!(host.mass, 62);
            assert_eq!(fx.particle_count(), 0);
        }
    }
}

#[test]
fn cascade_suppression_skips_rng_and_positive_owner_y_velocity_clamps_to_zero() {
    for suppressed in [false, true] {
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, 84, 15);
        fx.particles.slots[slot]
            .as_mut()
            .unwrap()
            .suppresses_impact_damage = suppressed;
        fx.last_retail_tick = Some(20);
        seed_for(&mut fx, |word| word & 1 != 0);
        let before_rng = fx.rng_state;
        let mut host = Host::new(false);
        host.owner.as_mut().unwrap().velocity_raw[1] = 203;
        let result = update(&mut fx, &mut host, 21, 0);
        assert_eq!(result.attached_updates.len(), 1);
        if suppressed {
            assert_eq!(fx.rng_state, before_rng);
            assert_eq!(fx.particle_count(), 1);
        } else {
            let child = fx
                .particles
                .slots
                .iter()
                .flatten()
                .find(|child| child.source_class == 69)
                .unwrap();
            assert_eq!(
                world_velocity_component_to_raw(child.velocity[1]) as i16,
                -50,
                "the argument is zero before class69's authored -50 birth bias"
            );
        }
    }
}

#[test]
fn borrowed_context_cannot_claim_mutable_b2_or_global_handle_authority() {
    let mut fx = WorldFx::new();
    let slot = attached(&mut fx, 84, 5);
    let rng = fx.rng_state;
    let result = fx.update(ParticleUpdateRequest::new(0, 20, ParticleEnvironment::Dry));
    let [diagnostic] = result.blocked_attached_updates.as_slice() else {
        panic!("{result:?}")
    };
    assert_eq!(
        diagnostic.reason,
        AttachedParticleUpdateBlock::MutableOwnerUnavailable
    );
    assert!(!diagnostic.committed_prefix);
    assert!(fx.particles.slots[slot].is_none());
    assert_eq!(fx.rng_state, rng);
}

#[test]
fn cascade_preempts_lower_priority_victim_and_all_children_leave_attached_parent_protected() {
    for (class, cascade, expected_priority) in [(83, 53, 4), (84, 69, 5), (86, 39, 4)] {
        assert_eq!(particle_descriptor(class).unwrap().priority_raw(), 6);
        assert_eq!(
            particle_descriptor(cascade).unwrap().priority_raw(),
            expected_priority
        );
        assert!(particle_descriptor(31).unwrap().priority_raw() < 6);
        assert!(particle_descriptor(42).unwrap().priority_raw() < 6);
        let mut fx = WorldFx::new();
        let victim = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 31,
                    position_raw: [1, 2, 3],
                    velocity_raw: [0; 3],
                    owner: None,
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        let younger_victim = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 31,
                    position_raw: [4, 5, 6],
                    velocity_raw: [0; 3],
                    owner: None,
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        let higher_priority_victim = fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 53,
                    position_raw: [7, 8, 9],
                    velocity_raw: [0; 3],
                    owner: None,
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                0,
            )
            .unwrap();
        fx.particles.slots[higher_priority_victim]
            .as_mut()
            .unwrap()
            .age_ticks = 16.0;
        let parent = attached(&mut fx, class, 16);
        for _ in 4..MAX_WORLD_PARTICLES {
            attached(&mut fx, 84, 0);
        }
        seed_for(&mut fx, |word| word & 1 != 0);
        let mut host = Host::new(false);
        let mut result = ParticleUpdateOutcome::default();
        fx.update_attached_particle_slot(
            parent,
            ParticleTraversalTiming {
                retail_tick: 21,
                elapsed_micros: 0,
            },
            16,
            &mut host,
            &mut result,
        );
        assert!(result.blocked_attached_updates.is_empty());
        assert_eq!(fx.particles.slots[parent].unwrap().source_class, class);
        assert_eq!(fx.particles.slots[parent].unwrap().age_ticks, 16.0);
        let child = fx.particles.slots[victim].unwrap();
        assert_eq!(child.source_class, cascade);
        assert_eq!(child.age_ticks, 0.0);
        assert_eq!(child.owner_id, Some(7));
        assert_eq!(
            fx.particles.slots[younger_victim].unwrap().source_class,
            31,
            "priority2's oldest tail is the victim before the younger slot"
        );
        assert_eq!(
            fx.particles.slots[higher_priority_victim]
                .unwrap()
                .source_class,
            53,
            "priority2 precedes an otherwise eligible priority4 victim"
        );
        assert_eq!(fx.particle_count(), MAX_WORLD_PARTICLES);
    }
}

#[test]
fn attached_descriptor_frozen_words_skip_integration_before_water_classification() {
    for class in [83, 84, 86] {
        assert_ne!(particle_descriptor(class).unwrap().raw_u16(0x04), 0);
        let mut fx = WorldFx::new();
        let slot = attached(&mut fx, class, 5);
        let parent = fx.particles.slots[slot].as_mut().unwrap();
        parent.position = [1.0, 1.0, 1.0];
        parent.attached_offset_raw = [0, i16::MIN, 0];
        // The retail words alias offsets and velocity. This explicit fixture
        // exposes 40120's frozen-bit gate instead of relying on the adapter's
        // detached zero-velocity representation to make integration harmless.
        parent.velocity = raw_velocity_to_world([0, i32::from(i16::MIN), 0]);
        let mut host = Host::new(false);
        host.sea = Some(0.0);
        let result = update(&mut fx, &mut host, 20, RETAIL_FRAME_DELTA_MAX_US);
        assert!(result.blocked_attached_updates.is_empty());
        let parent = fx.particles.slots[slot].unwrap();
        assert_eq!(parent.water_state, 2);
        assert!(!parent.pending_destruction);
        assert_eq!(parent.step_start_position, [1.0, 1.0, 1.0]);
        assert_eq!(parent.position, raw_position_to_world([1000, 2000, 3000]));
    }
}
