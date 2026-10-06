use super::*;
use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};
use v2k_render::ParticleCenterProjection;

const VIEWPORT: [u32; 2] = [320, 240];
const FAR: i32 = 0x1800;

fn center(depth_raw: i32, clip: u8) -> ParticleCenterProjection {
    ParticleCenterProjection {
        screen: [160, 120],
        depth_raw,
        clip,
    }
}

fn birth(fx: &mut WorldFx, class: u8, position: [i16; 3], velocity: [i16; 3]) -> usize {
    fx.materialize_descriptor_particle_request(
        DescriptorParticleRequest {
            source_class: class,
            position_raw: position,
            velocity_raw: velocity,
            owner: Some(ParticleOwnerAtBirth {
                entity_id: 54,
                entity_type: 102,
            }),
            suppresses_impact_damage: false,
        },
        ParticleEnvironment::Dry,
        17,
    )
    .expect("authored descriptor birth in an empty pool")
}

#[test]
fn d300_normalization_uses_signed_words_and_the_actual_zero_speed_axis() {
    // Independent EXE arithmetic controls: division truncates toward zero,
    // the later shift floors negative products, and AX is signed before IDIV.
    assert_eq!(streak_step_raw([0; 3]), [25, 0, 0]);
    assert_eq!(streak_step_raw([3, 4, 0]), [14, 19, 0]);
    assert_eq!(streak_step_raw([-3, -4, 0]), [-15, -20, 0]);
    assert_eq!(streak_step_raw([30001, 30002, 0]), [-25, -50, 0]);
    assert_eq!(streak_step_raw([i16::MIN, 1, 0]), [0, -25, 0]);
    // The three-square sum wraps negative; 457730 returns zero in this case.
    assert_eq!(streak_step_raw([i16::MAX; 3]), [25, 0, 0]);
}

#[test]
fn all_six_authored_d300_descriptors_draw_ten_copies_of_the_same_frame() {
    let family: Vec<_> = PARTICLE_DESCRIPTORS
        .iter()
        .enumerate()
        .filter(|(_, descriptor)| descriptor.raw_u32(0x30) == STREAK_DRAW_CALLBACK_VA)
        .map(|(class, _)| class as u8)
        .collect();
    assert_eq!(family, [49, 55, 56, 80, 81, 82]);
    let terrain = TerrainGrid {
        header: [i32::MIN, 0, 0, 0, 0],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    for class in family {
        let descriptor = particle_descriptor(class).unwrap();
        assert_eq!(descriptor.flags(), 6);
        assert_eq!(descriptor.size_jitter_divisor_raw(), 0);
        let authored_frame = particle_frame(class, 0).unwrap();
        assert_eq!(authored_frame.middle_raw, 0, "no invented streak lighting");
        let mut fx = WorldFx::new();
        let slot = birth(&mut fx, class, [1000, 512, -1000], [3, 4, 0]);
        fx.particles.slots[slot].as_mut().unwrap().age_ticks = 9.75;
        if class == 55 {
            // A legacy float remainder must not enter even the first copied
            // draw, and presentation must not erase it from the live record.
            fx.particles.slots[slot].as_mut().unwrap().position[0] += 0.001;
        }
        let original = fx.particles.slots[slot].unwrap();
        let rng = fx.rng_state;
        let mut projected = Vec::new();
        let frame = fx.prepare_presentation(VIEWPORT, FAR, |sample| {
            projected.push(world_position_to_raw(sample.position));
            center(0x100, 0)
        });
        let expected: Vec<_> = (0..10)
            .map(|index| [1000 + 14 * index, 512 + 19 * index, -1000])
            .collect();
        assert_eq!(projected, expected, "class {class}");
        assert_eq!(frame.particles().len(), 10, "class {class}");
        for (index, prepared) in frame.particles().enumerate() {
            let mut expected_copy = original;
            expected_copy.position = raw_position_to_world(expected[index]);
            assert_eq!(prepared.particle, expected_copy);
            assert_eq!(prepared.slot, slot);
            assert_eq!(prepared.particle.current_frame(), authored_frame);
            assert_eq!(prepared.effective_draw_scale_raw(), 0x600);
            assert!(frame.sprite_visible(prepared, [8, 8]));
            assert_eq!(prepared.particle.terrain_light_emission(&terrain), None);
        }
        assert_eq!(fx.particles.slots[slot], Some(original));
        assert_eq!(fx.particle_count(), 1);
        assert_eq!(fx.rng_state, rng);
    }
}

#[test]
fn d300_position_steps_wrap_all_three_signed_words_without_moving_live_state() {
    let mut fx = WorldFx::new();
    let slot = birth(&mut fx, 55, [32760, -32760, 32760], [5000, -5000, 5000]);
    let original = fx.particles.slots[slot].unwrap();
    let frame = fx.prepare_presentation(VIEWPORT, FAR, |_| center(0x100, 0));
    let positions: Vec<_> = frame
        .particles()
        .map(|prepared| world_position_to_raw(prepared.particle.position))
        .collect();
    assert_eq!(positions[0], [32760, -32760, 32760]);
    assert_eq!(positions[1], [-32762, 32761, -32762]);
    assert_eq!(positions[9], [-32650, 32641, -32650]);
    assert_eq!(fx.particles.slots[slot], Some(original));
}

#[test]
fn d300_hard_hidden_samples_do_not_stop_later_samples_or_trigger_class81_cleanup() {
    let mut fx = WorldFx::new();
    let slot = birth(&mut fx, 81, [0, 512, 0], [0; 3]);
    let original = fx.particles.slots[slot].unwrap();
    let rng = fx.rng_state;
    let mut visits = 0;
    let frame = fx.prepare_presentation(VIEWPORT, FAR, |_| {
        let index = visits;
        visits += 1;
        match index {
            0..=2 => center(-1, 0x40),
            // Centered beyond far reaches the ordinary post-light sprite gate.
            3..=5 => center(FAR, 0x12),
            _ => center(0x100, 0),
        }
    });
    assert_eq!(visits, 10);
    assert_eq!(frame.particles().len(), 7);
    assert_eq!(
        frame
            .particles()
            .map(|prepared| world_position_to_raw(prepared.particle.position)[0])
            .collect::<Vec<_>>(),
        [75, 100, 125, 150, 175, 200, 225]
    );
    assert_eq!(
        frame
            .particles()
            .map(|prepared| frame.sprite_visible(prepared, [8, 8]))
            .collect::<Vec<_>>(),
        [false, false, false, true, true, true, true]
    );
    assert_eq!(fx.particles.slots[slot], Some(original));
    assert_eq!(
        fx.particle_count(),
        1,
        "no destruction-time class42 children"
    );
    assert_eq!(fx.rng_state, rng);
    let hidden = fx.prepare_presentation(VIEWPORT, FAR, |_| center(-1, 0x40));
    assert_eq!(hidden.particles().len(), 0);
    assert_eq!(fx.particles.slots[slot], Some(original));
}

#[test]
fn streak_draws_are_contiguous_within_priority_and_newest_first_pool_order() {
    let mut fx = WorldFx::new();
    let older = birth(&mut fx, 55, [1000, 512, 0], [0; 3]);
    let newer = birth(&mut fx, 49, [2000, 512, 0], [0; 3]);
    let low_priority = birth(&mut fx, 15, [3000, 512, 0], [0; 3]);
    let frame = fx.prepare_presentation(VIEWPORT, FAR, |_| center(0x100, 0));
    let slots: Vec<_> = frame.particles().map(|prepared| prepared.slot).collect();
    assert_eq!(slots.len(), 21);
    assert_eq!(slots[0], low_priority);
    assert_eq!(slots[1..11], [newer; 10]);
    assert_eq!(slots[11..21], [older; 10]);
    assert_eq!(fx.particle_count(), 3);
}

#[test]
fn ordinary_d410_hard_cull_still_frees_only_its_real_slot() {
    let mut fx = WorldFx::new();
    let ordinary = birth(&mut fx, 15, [0, 512, 0], [0; 3]);
    let streak = birth(&mut fx, 55, [0, 512, 0], [0; 3]);
    let mut visits = Vec::new();
    let frame = fx.prepare_presentation(VIEWPORT, FAR, |particle| {
        visits.push(particle.source_class);
        center(-1, 0x40)
    });
    assert_eq!(visits[0], 15);
    assert_eq!(visits[1..], [55; 10]);
    assert_eq!(frame.particles().len(), 0);
    assert_eq!(fx.particles.slots[ordinary], None);
    assert!(fx.particles.slots[streak].is_some());
    assert_eq!(fx.particle_count(), 1);
    fx.particles.assert_valid_topology();
}

#[test]
fn all_six_authored_442240_descriptors_draw_the_native_live_and_backward_trail() {
    let family: Vec<_> = PARTICLE_DESCRIPTORS
        .iter()
        .enumerate()
        .filter(|(_, descriptor)| descriptor.raw_u32(0x30) == BACKWARD_STREAK_DRAW_CALLBACK_VA)
        .map(|(class, _)| class as u8)
        .collect();
    assert_eq!(family, [52, 53, 68, 69, 87, 89]);
    // These are independent44230B/442350/4423A8 source positions, not a
    // fixture assembled by calling the production sampling helper.
    let positions = [
        [1000, 512, -1000],
        [986, 493, -1000],
        [972, 474, -1000],
        [958, 455, -1000],
        [944, 436, -1000],
        [930, 417, -1000],
        [888, 360, -1000],
    ];
    for class in family {
        let descriptor = particle_descriptor(class).unwrap();
        let scale = descriptor.draw_scale_raw();
        let expected_scales = [
            scale,
            scale >> 1,
            scale >> 1,
            scale >> 2,
            scale >> 2,
            scale >> 3,
            scale >> 3,
        ];
        let mut fx = WorldFx::new();
        let slot = birth(&mut fx, class, positions[0], [3, 4, 0]);
        //53/69 have authored constructor velocity bias. This is a controlled
        //presentation case: set the current packed velocity after real birth,
        //matching the independent PE callback's actual input record.
        fx.particles.slots[slot].as_mut().unwrap().velocity = raw_velocity_to_world([3, 4, 0]);
        fx.particles.slots[slot].as_mut().unwrap().age_ticks = 9.75;
        let original = fx.particles.slots[slot].unwrap();
        let rng = fx.rng_state;
        let mut projected = Vec::new();
        let frame = fx.prepare_presentation(VIEWPORT, FAR, |sample| {
            projected.push((
                world_position_to_raw(sample.position),
                sample.draw_scale_raw,
            ));
            center(0x100, 0)
        });
        assert_eq!(
            projected,
            positions
                .into_iter()
                .zip(expected_scales)
                .collect::<Vec<_>>(),
            "class {class}"
        );
        assert_eq!(frame.particles().len(), 7, "class {class}");
        for (index, sample) in frame.particles().enumerate() {
            assert_eq!(sample.slot, slot);
            assert_eq!(sample.particle.current_frame(), original.current_frame());
            assert_eq!(sample.particle.age_ticks, 9.75);
            assert_eq!(sample.particle.velocity, original.velocity);
            assert!(frame.sprite_visible(sample, [8, 8]));
            if index == 0 {
                assert_eq!(
                    sample.draw_record_address,
                    ParticleDrawRecordAddress::LivePoolSlot
                );
                assert_eq!(
                    sample.native_effective_draw_scale_raw(),
                    Ok(effective_particle_draw_scale_raw(slot, &original))
                );
            } else {
                assert_eq!(
                    sample.draw_record_address,
                    ParticleDrawRecordAddress::Stack {
                        callback_va: BACKWARD_STREAK_DRAW_CALLBACK_VA,
                        record_address: None,
                    }
                );
                if descriptor.size_jitter_divisor_raw() == 0 {
                    assert_eq!(
                        sample.native_effective_draw_scale_raw(),
                        Ok(i32::from(expected_scales[index]))
                    );
                } else {
                    assert_eq!(
                        sample.native_effective_draw_scale_raw(),
                        Err(ParticleStackSizeJitterBoundary {
                            source_class: class,
                            callback_va: BACKWARD_STREAK_DRAW_CALLBACK_VA,
                            sample_scale_raw: i32::from(expected_scales[index]),
                            divisor_raw: descriptor.size_jitter_divisor_raw() as i8,
                        })
                    );
                    assert_eq!(
                        sample.effective_draw_scale_raw(),
                        i32::from(expected_scales[index]),
                        "explicit unadjusted bound"
                    );
                }
            }
        }
        assert_eq!(fx.particles.slots[slot], Some(original));
        assert_eq!(
            fx.particle_count(),
            1,
            "six copied records are not new allocations"
        );
        assert_eq!(fx.rng_state, rng);
        fx.particles.assert_valid_topology();
    }
}

#[test]
fn backward_streak_address_phase_is_supplied_explicitly_and_shared_by_all_copies() {
    let mut fx = WorldFx::new();
    let slot = birth(&mut fx, 87, [1000, 512, 0], [0; 3]);
    let original = fx.particles.slots[slot].unwrap();
    // Controlled oracle address, not an assertion about a retail run's ESP.
    let address = 0x0090_01a0;
    let frame = fx.prepare_presentation_with_stack_addresses(
        VIEWPORT,
        FAR,
        ParticlePresentationStackAddresses {
            backward_streak_record: Some(address),
            ..Default::default()
        },
        |_| center(0x100, 0),
    );
    let expected_positions = [1000, 975, 950, 925, 900, 875, 800];
    let expected_scales = [512, 256, 256, 128, 128, 64, 64];
    for (index, sample) in frame.particles().enumerate() {
        assert_eq!(
            world_position_to_raw(sample.particle.position)[0],
            expected_positions[index]
        );
        let expected = if index == 0 {
            effective_particle_draw_scale_raw(slot, &original)
        } else {
            expected_scales[index] + 13 * expected_scales[index] / 32
        };
        assert_eq!(sample.native_effective_draw_scale_raw(), Ok(expected));
    }
    assert_eq!(fx.particles.slots[slot], Some(original));
}

#[test]
fn backward_streak_signed_negative_steps_wrap_source_words_and_leave_live_motion_untouched() {
    let mut fx = WorldFx::new();
    let slot = birth(&mut fx, 87, [32760, 32760, 0], [-3, -4, 0]);
    fx.particles.slots[slot].as_mut().unwrap().position[0] += 0.001;
    let original = fx.particles.slots[slot].unwrap();
    let frame = fx.prepare_presentation(VIEWPORT, FAR, |_| center(0x100, 0));
    let positions: Vec<_> = frame
        .particles()
        .map(|sample| world_position_to_raw(sample.particle.position))
        .collect();
    assert_eq!(
        positions,
        [
            [32760, 32760, 0],
            [-32761, -32756, 0],
            [-32746, -32736, 0],
            [-32731, -32716, 0],
            [-32716, -32696, 0],
            [-32701, -32676, 0],
            [-32656, -32616, 0],
        ]
    );
    assert_eq!(
        frame.particles().next().unwrap().particle.position,
        raw_position_to_world(positions[0])
    );
    assert_eq!(fx.particles.slots[slot], Some(original));
}

#[test]
fn backward_streak_hidden_live_and_copied_samples_continue_without_removing_the_real_record() {
    for class in [52, 53, 68, 69, 87, 89] {
        let mut fx = WorldFx::new();
        let slot = birth(&mut fx, class, [0, 512, 0], [0; 3]);
        fx.particles.slots[slot].as_mut().unwrap().velocity = raw_velocity_to_world([0; 3]);
        let original = fx.particles.slots[slot].unwrap();
        assert_ne!(particle_descriptor(class).unwrap().flags() & 2, 0);
        let rng = fx.rng_state;
        let mut visits = 0;
        let frame = fx.prepare_presentation(VIEWPORT, FAR, |_| {
            let index = visits;
            visits += 1;
            match index {
                0 | 2 | 6 => center(-1, 0x40),
                3 => center(FAR, 0x12),
                _ => center(0x100, 0),
            }
        });
        assert_eq!(visits, 7);
        assert_eq!(
            frame
                .particles()
                .map(|sample| world_position_to_raw(sample.particle.position)[0])
                .collect::<Vec<_>>(),
            [-25, -75, -100, -125]
        );
        assert_eq!(
            frame
                .particles()
                .map(|sample| frame.sprite_visible(sample, [8, 8]))
                .collect::<Vec<_>>(),
            [true, false, true, true]
        );
        assert_eq!(fx.particles.slots[slot], Some(original));
        assert_eq!(fx.particle_count(), 1);
        assert_eq!(fx.rng_state, rng);
    }
}

#[test]
fn backward_streak_samples_finish_before_next_intrusive_particle_without_allocating_trail_children()
{
    let mut fx = WorldFx::new();
    let older = birth(&mut fx, 87, [1000, 512, 0], [0; 3]);
    let newer = birth(&mut fx, 52, [2000, 512, 0], [0; 3]);
    assert_eq!(
        particle_descriptor(87).unwrap().priority_raw(),
        particle_descriptor(52).unwrap().priority_raw()
    );
    let frame = fx.prepare_presentation(VIEWPORT, FAR, |_| center(0x100, 0));
    let slots: Vec<_> = frame.particles().map(|sample| sample.slot).collect();
    assert_eq!(
        slots,
        [newer; 7].into_iter().chain([older; 7]).collect::<Vec<_>>()
    );
    assert_eq!(fx.particle_count(), 2);
    assert_eq!(
        frame
            .particles()
            .map(|sample| sample.particle.owner_id)
            .collect::<Vec<_>>(),
        [Some(54); 14]
    );
    fx.particles.assert_valid_topology();
}
