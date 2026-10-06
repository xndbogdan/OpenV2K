use super::*;
use std::collections::VecDeque;

const SUB_A: SubAPropulsionDescriptor = SubAPropulsionDescriptor {
    acceleration_raw: 4_000,
    overspeed_correction_raw: -4_000,
    target_speed_base_raw: 10_000,
};

#[derive(Debug, PartialEq, Eq)]
enum InitEvent {
    Range(i32),
    Speed(i32),
    Direction(i32),
    Prepare(RocketTaskPublication),
    Random(u16),
    Publish(RocketTaskPublication),
}

struct InitHost {
    events: Vec<InitEvent>,
    words: VecDeque<u16>,
    fail: Option<RocketTaskPublication>,
}

impl RocketClass22InitHost for InitHost {
    type PreparedTask = RocketTaskPublication;
    type Error = &'static str;

    fn write_search_range_raw(&mut self, value: i32) {
        self.events.push(InitEvent::Range(value));
    }
    fn write_sub_a_target_speed_raw(&mut self, value: i32) {
        self.events.push(InitEvent::Speed(value));
    }
    fn write_sub_a_direction_multiplier_raw(&mut self, value: i32) {
        self.events.push(InitEvent::Direction(value));
    }
    fn prepare_task(
        &mut self,
        request: RocketTaskPublication,
    ) -> Result<Self::PreparedTask, Self::Error> {
        self.events.push(InitEvent::Prepare(request));
        if self.fail == Some(request) {
            Err("allocation failed")
        } else {
            Ok(request)
        }
    }
    fn next_shared_retail_random_u16(&mut self) -> u16 {
        let word = self.words.pop_front().expect("unexpected RNG consumption");
        self.events.push(InitEvent::Random(word));
        word
    }
    fn publish_task(&mut self, request: RocketTaskPublication, task: Self::PreparedTask) {
        assert_eq!(request, task);
        self.events.push(InitEvent::Publish(request));
    }
}

#[test]
fn class22_prepares_resets_and_publishes_in_native_order() {
    use RocketTaskPublication::{Acquisition, Flight, Trail};
    let mut host = InitHost {
        events: vec![],
        words: [0x0100, 0xFF00].into(),
        fail: None,
    };
    let outcome = initialize_rocket_class22(
        RocketClass22InitRequest {
            sub_a: SUB_A,
            existing_target_speed_raw: 10_000,
        },
        &mut host,
    )
    .unwrap();
    assert_eq!(
        host.events,
        [
            InitEvent::Range(768),
            InitEvent::Speed(15_000),
            InitEvent::Prepare(Acquisition),
            InitEvent::Random(0x0100),
            InitEvent::Direction(1),
            InitEvent::Speed(10_003),
            InitEvent::Publish(Acquisition),
            InitEvent::Prepare(Trail),
            InitEvent::Publish(Trail),
            InitEvent::Prepare(Flight),
            InitEvent::Random(0xFF00),
            InitEvent::Direction(1),
            InitEvent::Speed(10_996),
            InitEvent::Publish(Flight),
        ]
    );
    assert!(outcome.acquisition_published && outcome.trail_published && outcome.flight_published);
    assert_eq!(outcome.target_speed_raw, 10_996);
    assert_eq!(outcome.acquisition_reset_word, Some(0x0100));
    assert_eq!(outcome.flight_reset_word, Some(0xFF00));
    assert_eq!(Acquisition.slot(), ActorTaskSlot::Secondary);
    assert_eq!(Trail.slot(), ActorTaskSlot::Tertiary);
    assert_eq!(Flight.slot(), ActorTaskSlot::Primary);
}

#[test]
fn class22_allocation_errors_retain_exact_constructor_prefix() {
    use RocketTaskPublication::{Acquisition, Flight, Trail};
    for (failed, expected_words, acquired, trailed) in [
        (Acquisition, 0, false, false),
        (Trail, 1, true, false),
        (Flight, 1, true, true),
    ] {
        let mut host = InitHost {
            events: vec![],
            words: [0x0100, 0xFF00].into(),
            fail: Some(failed),
        };
        let error = initialize_rocket_class22(
            RocketClass22InitRequest {
                sub_a: SUB_A,
                existing_target_speed_raw: -3,
            },
            &mut host,
        )
        .unwrap_err();
        assert_eq!(error.failed_preparation, failed);
        assert_eq!(error.error, "allocation failed");
        assert_eq!(host.events[1], InitEvent::Speed(-4));
        assert_eq!(host.events.last(), Some(&InitEvent::Prepare(failed)));
        assert_eq!(2 - host.words.len(), expected_words);
        assert_eq!(error.committed_prefix.acquisition_published, acquired);
        assert_eq!(error.committed_prefix.trail_published, trailed);
        assert!(!error.committed_prefix.flight_published);
        assert_eq!(
            error.committed_prefix.target_speed_raw,
            if acquired { 10_003 } else { -4 }
        );
    }
}

#[test]
fn primary_elapsed_truncates_each_visit_and_expires_strictly_after_2000() {
    let mut state = RocketFlightTaskState::new();
    for _ in 0..3 {
        state.before_callback(999);
    }
    assert_eq!(state.elapsed_ms(), 0);
    for _ in 0..50 {
        state.before_callback(40_000);
    }
    assert_eq!(state.elapsed_ms(), 2_000);
    assert!(!state.timeout_after_unwind());
    state.before_callback(1_000);
    assert!(state.timeout_after_unwind());
    state.elapsed_ms = u32::MAX - 5;
    state.before_callback(10_000);
    assert_eq!(state.elapsed_ms(), 4);
    assert!(!state.timeout_after_unwind());
}

#[test]
fn primary_forces_retain_native_lift_correction_and_fixed_point_narrowing() {
    let axis = 0x7FFF_0000;
    let outcome = apply_rocket_flight_forces_raw(RocketFlightRequest {
        position_raw: [1, 0, 2],
        velocity_raw: [123, -100, 0],
        body_basis: Type9BodyBasis {
            lateral: [axis, 0, 0],
            up: [0, axis, 0],
            forward: [0, 0, axis],
        },
        sub_a: SUB_A,
        sub_b: SubBLateralDescriptor {
            projection_threshold_rate_raw: 100_000,
            correction_rate_raw: 100_000,
        },
        sub_c: HoverLiftConfig {
            base_clearance_raw: 75,
            lift_range_raw: 75,
            strength_raw: 0x30_0000,
            near_boost_range_raw: 100,
            damping_range_raw: 200,
            use_wave_surface: false,
            offset_sample: false,
        },
        target_speed_raw: 10_000,
        direction_multiplier: 1,
        drive_scale_percent: 100,
        surface: SubCSurfaceSample::Terrain { terrain_y_raw: 0 },
        elapsed_micros: 40_000,
    });
    assert_eq!(outcome.position_raw, [1, 75, 2]);
    assert_eq!(outcome.velocity_raw, [2, 15, 151]);
    assert_eq!(outcome.lift.position_correction_raw, 75);
    assert_eq!(outcome.lift.lift_impulse_raw, 116);
    // This phase neither applies gravity nor integrates the final position.
}

#[derive(Debug, PartialEq, Eq)]
enum TrailEvent {
    Allocation(DescriptorParticleRequest),
    Random(u16),
}

#[derive(Default)]
struct TrailHost {
    events: Vec<TrailEvent>,
    words: VecDeque<u16>,
    accept_allocations: bool,
}

impl RocketTrailHost for TrailHost {
    fn allocate_trail_particle(&mut self, request: DescriptorParticleRequest) -> Option<usize> {
        self.events.push(TrailEvent::Allocation(request));
        self.accept_allocations.then_some(7)
    }
    fn next_shared_retail_random_u16(&mut self) -> u16 {
        let word = self.words.pop_front().expect("unexpected RNG consumption");
        self.events.push(TrailEvent::Random(word));
        word
    }
}

fn trail_request() -> RocketTrailRequest {
    RocketTrailRequest {
        callback_reason_raw: 0,
        position_raw: [0, 1, 0],
        velocity_raw: [0, 0, 701],
        sea_level_raw: 0,
        owner: ParticleOwnerAtBirth {
            entity_id: 12,
            entity_type: 42,
        },
        source_remote_bit: false,
        elapsed_micros: 40_000,
    }
}

#[test]
fn trail_threshold_and_signed_wrapping_norm_are_native() {
    for request in [
        RocketTrailRequest {
            callback_reason_raw: 1,
            ..trail_request()
        },
        RocketTrailRequest {
            velocity_raw: [700, 0, 0],
            ..trail_request()
        },
        RocketTrailRequest {
            velocity_raw: [i16::MIN; 3],
            ..trail_request()
        },
    ] {
        let mut host = TrailHost::default();
        assert_eq!(
            emit_rocket_trail(request, &mut host),
            RocketTrailOutcome::default()
        );
        assert!(host.events.is_empty());
    }
    let mut host = TrailHost {
        words: [0; 6].into(),
        ..TrailHost::default()
    };
    let outcome = emit_rocket_trail(
        RocketTrailRequest {
            velocity_raw: [700, 1, 0],
            ..trail_request()
        },
        &mut host,
    );
    assert_eq!(outcome.attempted_particles, 2);
}

#[test]
fn trail_attempts_allocation_before_three_words_even_when_pool_is_full() {
    for (elapsed_micros, expected) in [(0, 1), (30_000, 1), (30_001, 2), (125_000, 5)] {
        let mut host = TrailHost {
            words: vec![0x8000; expected * 3].into(),
            ..TrailHost::default()
        };
        let outcome = emit_rocket_trail(
            RocketTrailRequest {
                elapsed_micros,
                ..trail_request()
            },
            &mut host,
        );
        assert_eq!(outcome.attempted_particles, expected as u32);
        assert_eq!(outcome.allocated_particles, 0);
        assert_eq!(outcome.model_effect_bits_to_set, 0x10);
        assert!(host.words.is_empty());
        for events in host.events.chunks_exact(4) {
            assert!(matches!(events[0], TrailEvent::Allocation(_)));
            assert_eq!(
                &events[1..],
                &[
                    TrailEvent::Random(0x8000),
                    TrailEvent::Random(0x8000),
                    TrailEvent::Random(0x8000)
                ]
            );
        }
    }
}

#[test]
fn trail_preserves_source_owner_remote_flag_wrap_and_per_attempt_sea_class() {
    let mut host = TrailHost {
        words: [0; 6].into(),
        accept_allocations: true,
        ..TrailHost::default()
    };
    let outcome = emit_rocket_trail(
        RocketTrailRequest {
            position_raw: [i16::MIN, 0, i16::MAX],
            velocity_raw: [0, 1_000, 0],
            source_remote_bit: true,
            elapsed_micros: 60_000,
            ..trail_request()
        },
        &mut host,
    );
    assert_eq!(outcome.allocated_particles, 2);
    let allocations: Vec<_> = host
        .events
        .iter()
        .filter_map(|event| match event {
            TrailEvent::Allocation(request) => Some(*request),
            _ => None,
        })
        .collect();
    assert_eq!(allocations[0].source_class, 42);
    assert_eq!(allocations[0].position_raw, [i16::MIN, 0, i16::MAX]);
    assert_eq!(allocations[1].source_class, 31);
    assert_eq!(allocations[1].position_raw, [32_752, 12, 32_751]);
    for request in allocations {
        assert_eq!(request.owner, Some(trail_request().owner));
        assert!(request.suppresses_impact_damage);
        assert_eq!(request.velocity_raw, [0; 3]);
    }
    assert!(host.words.is_empty());
}
