use std::{cell::RefCell, collections::HashMap};

use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Lookup(u32),
    Construct(HiveBirthRequest),
    Source(u32, u32),
    Geometry(HiveEjectionRequest),
    Random(u16),
    Height(i16, i16),
    Ejection(u32, HiveChildEjection),
    Restore(u32),
}

struct Host {
    events: RefCell<Vec<Event>>,
    states: HashMap<u32, u32>,
    lookup_block: Option<u32>,
    attempts: VecDeque<Result<HiveBirthAttempt<&'static str>, &'static str>>,
    random: VecDeque<u16>,
    geometry: Result<HiveEjectionGeometry, &'static str>,
    terrain: TerrainGrid,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            states: HashMap::new(),
            lookup_block: None,
            attempts: VecDeque::new(),
            random: VecDeque::new(),
            geometry: Ok(HiveEjectionGeometry {
                anchor_raw: [1_000, 999, 2_000],
                model_extent_raw: 200,
            }),
            terrain: TerrainGrid {
                header: [0; 5],
                cells: vec![
                    TerrainCell {
                        height: 0,
                        attribute: 0,
                        terrain_type: 0,
                    };
                    GRID_SIZE * GRID_SIZE
                ],
            },
        }
    }
}

impl Host {
    fn record(&self, event: Event) {
        self.events.borrow_mut().push(event);
    }

    fn events(&self) -> Vec<Event> {
        self.events.borrow().clone()
    }

    fn with_births(handles: impl IntoIterator<Item = u32>) -> Self {
        let handles: Vec<_> = handles.into_iter().collect();
        Self {
            attempts: handles
                .iter()
                .copied()
                .map(HiveBirthAttempt::Created)
                .map(Ok)
                .collect(),
            random: vec![1, 2, 3, 4].repeat(handles.len()).into(),
            ..Self::default()
        }
    }
}

impl HiveBirthHost for Host {
    type Block = &'static str;

    fn child_state_flags(&mut self, child_handle: u32) -> Result<Option<u32>, Self::Block> {
        self.record(Event::Lookup(child_handle));
        if self.lookup_block == Some(child_handle) {
            Err("unowned child state")
        } else {
            Ok(self.states.get(&child_handle).copied())
        }
    }

    fn construct_hive_child(
        &mut self,
        request: HiveBirthRequest,
    ) -> Result<HiveBirthAttempt<Self::Block>, Self::Block> {
        self.record(Event::Construct(request));
        let attempt = self.attempts.pop_front().expect("expected birth attempt");
        if let Ok(HiveBirthAttempt::Created(child)) = attempt {
            self.states.insert(child, 0x8000);
        }
        if matches!(attempt, Ok(HiveBirthAttempt::CommittedPrefixBlock(_))) {
            // Simulate a reached constructor-owned RNG prefix. It must not be
            // rolled back or repeated when the affected row is revisited.
            self.next_shared_random_u16();
        }
        attempt
    }

    fn publish_hive_child_source(&mut self, child_handle: u32, source_id: u32) {
        self.record(Event::Source(child_handle, source_id));
    }

    fn prepare_hive_ejection(
        &mut self,
        request: HiveEjectionRequest,
    ) -> Result<HiveEjectionGeometry, Self::Block> {
        self.record(Event::Geometry(request));
        self.geometry
    }

    fn sample_hive_ejection_terrain_height_raw(&self, x_raw: i16, z_raw: i16) -> i16 {
        self.record(Event::Height(x_raw, z_raw));
        self.terrain.bilinear_height_raw(x_raw, z_raw)
    }

    fn publish_hive_child_ejection(&mut self, child_handle: u32, ejection: HiveChildEjection) {
        self.record(Event::Ejection(child_handle, ejection));
        *self.states.get_mut(&child_handle).unwrap() &= !0x8000;
    }

    fn restore_hive_child_pairs(&mut self, child_handle: u32) {
        self.record(Event::Restore(child_handle));
        *self.states.get_mut(&child_handle).unwrap() |= 0x8000;
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        let random = self.random.pop_front().expect("expected shared RNG draw");
        self.record(Event::Random(random));
        random
    }
}

fn animation(records: &[[i32; 7]]) -> EntityAnimation {
    let mut header = [0; 0x18];
    header[16..20].copy_from_slice(&(records.len() as i32).to_le_bytes());
    header[20..24].copy_from_slice(&0x1234_5678_u32.to_le_bytes());
    EntityAnimation {
        header,
        frames: records
            .iter()
            .map(|words| {
                let mut raw = [0; 0x1C];
                for (index, word) in words.iter().enumerate() {
                    raw[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
                }
                raw
            })
            .collect(),
    }
}

fn runtime(records: &[[i32; 7]]) -> HiveBirthRuntime {
    HiveBirthRuntime::from_animation(&animation(records)).unwrap()
}

fn visit() -> HiveBirthVisit {
    HiveBirthVisit {
        controller_state: 1,
        session_aborted: true,
        elapsed_us: 20_000,
        source_id: 67,
        source_position_raw: [111, 222, 333],
    }
}

fn request(child_handle: u32) -> HiveEjectionRequest {
    HiveEjectionRequest {
        source_id: 67,
        child_handle,
    }
}

#[test]
fn decode_uses_signed_record_fields_and_header_count_not_pointer() {
    let authored = animation(&[[15, -4, -5, -6, -7, -8, 3]]);
    let runtime = HiveBirthRuntime::from_animation(&authored).unwrap();
    assert_eq!(
        runtime.rows()[0].record(),
        HiveBirthRecord {
            entity_type: 15,
            initial_timer_us: -4,
            delay_base_us: -5,
            jitter_operand: -6,
            total_production_cap: -7,
            retained_child_cap: -8,
            flags: 3,
        }
    );
    assert_eq!(runtime.rows()[0].timer_us(), -4);
    assert_eq!(runtime.rows()[0].produced_count(), 0);
    assert!(runtime.rows()[0].children().is_empty());
    let mut malformed = authored.clone();
    malformed.header[16..20].copy_from_slice(&2_i32.to_le_bytes());
    assert_eq!(
        HiveBirthRuntime::from_animation(&malformed),
        Err(HiveBirthDecodeError {
            authored_count: 2,
            decoded_count: 1,
        })
    );
    let mut empty = animation(&[]);
    empty.header[16..20].copy_from_slice(&(-1_i32).to_le_bytes());
    assert_eq!(
        HiveBirthRuntime::from_animation(&empty),
        Err(HiveBirthDecodeError {
            authored_count: -1,
            decoded_count: 0,
        })
    );
}

#[test]
fn only_state1_and_matching_abort_rows_advance() {
    let mut runtime = runtime(&[[15, 10, 100, 0, 3, 3, 3], [17, 10, 100, 0, 3, 3, 0]]);
    let mut host = Host::default();
    let snapshot = runtime.clone();
    for state in [0, 2] {
        assert!(runtime
            .advance_rows(
                HiveBirthVisit {
                    controller_state: state,
                    ..visit()
                },
                &mut host
            )
            .is_empty());
    }
    assert_eq!(runtime, snapshot);
    runtime.advance_rows(
        HiveBirthVisit {
            elapsed_us: 5,
            ..visit()
        },
        &mut host,
    );
    assert_eq!(runtime.rows[0].timer_us, 5);
    assert_eq!(runtime.rows[1].timer_us, 10);
    runtime.advance_rows(
        HiveBirthVisit {
            session_aborted: false,
            elapsed_us: 5,
            ..visit()
        },
        &mut host,
    );
    assert_eq!(runtime.rows[0].timer_us, 5);
    assert_eq!(runtime.rows[1].timer_us, 5);
    assert!(host.events().is_empty());
}

#[test]
fn successful_birth_owns_source_fifo_count_launch_then_delay_draw() {
    let mut runtime = runtime(&[[15, 0, 2_000_000, 1_000_000, 3, 3, 3]]);
    let mut host = Host::with_births([100]);
    assert!(runtime.advance_rows(visit(), &mut host).is_empty());
    assert_eq!(runtime.rows[0].produced_count, 1);
    assert_eq!(runtime.rows[0].children, VecDeque::from([100]));
    assert_eq!(runtime.rows[0].timer_us, 2_000_061);
    assert_eq!(host.states[&100] & 0x8000, 0);
    assert_eq!(
        host.events(),
        vec![
            Event::Construct(HiveBirthRequest {
                entity_type: 15,
                position_raw: [111, 222, 333],
                objective: true,
                source_id: 67,
            }),
            Event::Source(100, 67),
            Event::Geometry(request(100)),
            Event::Random(1),
            Event::Random(2),
            Event::Height(1_001, 2_300),
            Event::Random(3),
            Event::Ejection(
                100,
                HiveChildEjection {
                    position_raw: [1_001, 400, 2_300],
                    velocity_raw: [2, 850, 3],
                    rotation_raw: [0, 0, 0],
                }
            ),
            Event::Random(4),
        ]
    );
    assert_eq!(runtime.ejection_slots[0].timer_us, 500_000);
    assert!(runtime.advance_ejections(20_000, &mut host).is_empty());
    assert_eq!(runtime.ejection_slots[0].timer_us, 480_000);
}

#[test]
fn native_constructor_error_still_resets_delay_without_success_suffix() {
    let mut runtime = runtime(&[[15, 0, 10, -65_536, 3, 3, 3]]);
    let mut host = Host {
        attempts: VecDeque::from([Ok(HiveBirthAttempt::NativeError)]),
        random: VecDeque::from([4]),
        ..Host::default()
    };
    assert!(runtime.advance_rows(visit(), &mut host).is_empty());
    assert_eq!(runtime.rows[0].timer_us, 6);
    assert_eq!(runtime.rows[0].produced_count, 0);
    assert!(runtime.rows[0].children.is_empty());
    assert_eq!(host.events().len(), 2);
    assert_eq!(host.events()[1], Event::Random(4));
}

#[test]
fn unowned_constructor_freezes_only_its_row_and_later_rows_continue() {
    let mut runtime = runtime(&[[15, -1, 10, 0, 3, 3, 3], [17, -2, 20, 0, 3, 3, 2]]);
    let mut host = Host::with_births([101]);
    host.attempts.push_front(Err("unowned constructor"));
    assert_eq!(
        runtime.advance_rows(visit(), &mut host),
        vec![HiveBirthBlock::Row {
            row_index: 0,
            block: "unowned constructor"
        }]
    );
    assert_eq!(runtime.rows[0].timer_us, -1);
    assert_eq!(runtime.rows[0].produced_count, 0);
    assert_eq!(runtime.rows[1].timer_us, 20);
    assert_eq!(runtime.rows[1].produced_count, 1);
    assert!(!matches!(host.events()[1], Event::Random(_)));
    assert!(matches!(
        host.events()[1],
        Event::Construct(HiveBirthRequest {
            objective: false,
            ..
        })
    ));
    runtime.advance_ejections(20_000, &mut host);
    assert_eq!(runtime.ejection_slots[0].timer_us, 480_000);
}

#[test]
fn committed_constructor_prefix_parks_only_its_row_and_never_replays_rng() {
    let mut runtime = runtime(&[
        [15, -1, 2_000_000, 0, 3, 3, 3],
        [15, -1, 2_000_000, 0, 3, 3, 3],
    ]);
    let mut host = Host {
        attempts: VecDeque::from([
            Ok(HiveBirthAttempt::CommittedPrefixBlock(
                "retained constructor prefix",
            )),
            Ok(HiveBirthAttempt::Created(100)),
        ]),
        random: VecDeque::from([99, 1, 2, 3, 4]),
        ..Host::default()
    };
    assert_eq!(
        runtime.advance_rows(visit(), &mut host),
        vec![HiveBirthBlock::Row {
            row_index: 0,
            block: "retained constructor prefix",
        }]
    );
    assert!(runtime.rows()[0].constructor_parked());
    assert_eq!(runtime.rows()[0].timer_us(), -1);
    assert_eq!(runtime.rows()[0].produced_count(), 0);
    assert!(!runtime.rows()[1].constructor_parked());
    assert_eq!(runtime.rows()[1].produced_count(), 1);
    assert_eq!(host.events()[1], Event::Random(99));
    let before = host.events();
    assert!(runtime.advance_rows(visit(), &mut host).is_empty());
    assert_eq!(host.events(), before);
    assert!(runtime.advance_ejections(20_000, &mut host).is_empty());
    assert_eq!(runtime.ejection_slots()[0].timer_us, 480_000);
}

#[test]
fn strict_timer_equality_and_large_delta_have_no_catch_up_births() {
    let mut runtime = runtime(&[[15, 20_000, 0, 0, 0, 10, 3]]);
    let mut host = Host::with_births([100, 101]);
    runtime.advance_rows(visit(), &mut host);
    assert_eq!(runtime.rows[0].timer_us, 0);
    assert!(host.events().is_empty());
    runtime.advance_rows(
        HiveBirthVisit {
            elapsed_us: 1_000_000_000,
            ..visit()
        },
        &mut host,
    );
    assert_eq!(runtime.rows[0].produced_count, 1);
    runtime.advance_rows(
        HiveBirthVisit {
            elapsed_us: 1_000_000_000,
            ..visit()
        },
        &mut host,
    );
    assert_eq!(runtime.rows[0].produced_count, 2);
}

#[test]
fn fifo_prunes_all_missing_or_dying_nodes_only_when_at_live_cap() {
    let mut runtime = runtime(&[[15, -1, 10, 0, 3, 1, 3]]);
    runtime.rows[0].children = VecDeque::from([10, 11, 12]);
    let mut host = Host::default();
    host.states.insert(11, 0x4000);
    host.states.insert(12, 0);
    assert!(runtime.advance_rows(visit(), &mut host).is_empty());
    assert_eq!(runtime.rows[0].children, VecDeque::from([12]));
    assert_eq!(
        host.events(),
        vec![Event::Lookup(10), Event::Lookup(11), Event::Lookup(12)]
    );
    assert_eq!(runtime.rows[0].timer_us, -1);

    runtime.rows[0].record.retained_child_cap = 2;
    host.states.insert(12, 0x4000);
    host.attempts.push_back(Err("unowned constructor"));
    host.events.borrow_mut().clear();
    runtime.advance_rows(visit(), &mut host);
    assert!(matches!(host.events().as_slice(), [Event::Construct(_)]));
    assert_eq!(runtime.rows[0].children, VecDeque::from([12]));
}

#[test]
fn total_cap_precedes_pruning_and_nonpositive_child_caps_block_births() {
    let mut runtime = runtime(&[
        [15, -1, 10, 0, 1, 1, 3],
        [15, -1, 10, 0, 0, 0, 3],
        [15, -1, 10, 0, -1, -1, 3],
    ]);
    runtime.rows[0].produced_count = 1;
    runtime.rows[0].children.push_back(10);
    let mut host = Host::default();
    assert!(runtime.advance_rows(visit(), &mut host).is_empty());
    assert!(host.events().is_empty());
    assert_eq!(runtime.rows[0].children, VecDeque::from([10]));
    assert!(runtime.rows.iter().all(|row| row.timer_us == -1));
}

#[test]
fn child_state_block_keeps_prune_prefix_and_does_not_block_later_rows() {
    let mut runtime = runtime(&[[15, -1, 10, 0, 0, 1, 3], [15, 20_000, 10, 0, 0, 1, 3]]);
    runtime.rows[0].children = VecDeque::from([10, 11]);
    let mut host = Host {
        lookup_block: Some(11),
        ..Host::default()
    };
    assert_eq!(
        runtime.advance_rows(visit(), &mut host),
        vec![HiveBirthBlock::Row {
            row_index: 0,
            block: "unowned child state"
        }]
    );
    assert_eq!(runtime.rows[0].children, VecDeque::from([11]));
    assert_eq!(runtime.rows[0].timer_us, -1);
    assert_eq!(runtime.rows[1].timer_us, 0);
}

#[test]
fn signed_wrapping_timer_count_and_jitter_match_low_dword_pe() {
    let row =
        HiveBirthRecord::decode(animation(&[[15, 0, 2_000_000, 1_000_000, 3, 3, 3]]).frames[0]);
    let delays: Vec<_> = (0..=u16::MAX)
        .map(|word| row.delay_after_draw(word))
        .collect();
    assert!(delays
        .iter()
        .all(|delay| (1_967_232..=2_032_767).contains(delay)));
    assert_eq!(row.delay_after_draw(4), 2_000_061);
    assert_eq!(row.delay_after_draw(u16::MAX), 2_016_944);
    let mut runtime = runtime(&[[15, 0, i32::MAX, 65_536, 0, 3, 3]]);
    let mut host = Host::with_births([100]);
    runtime.rows[0].produced_count = i32::MAX;
    runtime.advance_rows(
        HiveBirthVisit {
            elapsed_us: u32::MAX,
            ..visit()
        },
        &mut host,
    );
    assert_eq!(
        runtime.rows[0].timer_us, 1,
        "signed -1 delta subtracts with wrapping arithmetic"
    );
    runtime.advance_rows(visit(), &mut host);
    assert_eq!(runtime.rows[0].produced_count, i32::MIN);
    assert_eq!(runtime.rows[0].timer_us, i32::MIN + 3);
}

#[test]
fn ejection_slots_use_allocation_presence_even_when_state_zero_or_dying() {
    let mut runtime = HiveBirthRuntime::default();
    runtime.ejection_slots = [
        HiveEjectionSlot {
            child_handle: Some(10),
            timer_us: 100,
        },
        HiveEjectionSlot {
            child_handle: Some(11),
            timer_us: 100,
        },
    ];
    let mut host = Host::default();
    host.states.extend([(10, 0), (11, 0x4000)]);
    assert_eq!(
        runtime.eject_child(request(10), &mut host),
        Ok(HiveEjectionOutcome::AlreadyRecorded)
    );
    assert!(host.events().is_empty());
    assert_eq!(
        runtime.eject_child(request(12), &mut host),
        Ok(HiveEjectionOutcome::SlotsOccupied)
    );
    assert_eq!(host.events(), vec![Event::Lookup(10), Event::Lookup(11)]);
    host.states.remove(&10);
    host.states.insert(12, 0x8000);
    host.random.extend([2, 3, 4]);
    assert_eq!(
        runtime.eject_child(request(12), &mut host),
        Ok(HiveEjectionOutcome::Launched { slot_index: 0 })
    );
    assert_eq!(runtime.ejection_slots[1].child_handle, Some(11));
}

#[test]
fn ejection_uses_retained_anchor_signed_bilinear_seam_and_exact_three_draws() {
    let mut runtime = HiveBirthRuntime::default();
    let mut host = Host::default();
    host.geometry = Ok(HiveEjectionGeometry {
        anchor_raw: [-128, 999, -256],
        model_extent_raw: 256,
    });
    host.states.insert(100, 0x8000);
    host.random.extend([2, 255, 256]);
    for (x, z, height) in [(255, 0, -8_i8), (0, 0, 8), (255, 1, 8), (0, 1, 16)] {
        host.terrain.cells[x * GRID_SIZE + z].height = height as u8;
    }
    runtime.eject_child(request(100), &mut host).unwrap();
    assert_eq!(
        host.events().last(),
        Some(&Event::Ejection(
            100,
            HiveChildEjection {
                position_raw: [-130, 546, 100],
                velocity_raw: [-255, 850, 0],
                rotation_raw: [0x8000, 0, 0],
            }
        ))
    );
    assert_eq!(
        host.events()
            .iter()
            .filter(|event| matches!(event, Event::Random(_)))
            .count(),
        3
    );
}

#[test]
fn ejection_right_launch_wraps_signed_position_words() {
    let mut runtime = HiveBirthRuntime::default();
    let mut host = Host::default();
    host.geometry = Ok(HiveEjectionGeometry {
        anchor_raw: [i16::MAX, 999, i16::MAX],
        model_extent_raw: 200,
    });
    host.states.insert(100, 0x8000);
    host.random.extend([199, 0x1234, 0x5678]);
    runtime.eject_child(request(100), &mut host).unwrap();
    assert_eq!(
        host.events().last(),
        Some(&Event::Ejection(
            100,
            HiveChildEjection {
                position_raw: [-32_570, 400, -32_469],
                velocity_raw: [52, 850, 120],
                rotation_raw: [0, 0, 0],
            }
        ))
    );
}

#[test]
fn blocked_ejection_has_no_launch_prefix_and_successful_birth_is_not_replayed() {
    let mut runtime = runtime(&[[15, 0, 2_000_000, 0, 3, 3, 3]]);
    let mut host = Host::with_births([100]);
    host.geometry = Err("unowned terrain");
    assert_eq!(
        runtime.advance_rows(visit(), &mut host),
        vec![HiveBirthBlock::Ejection {
            child_handle: 100,
            block: HiveEjectionBlock::Host("unowned terrain"),
        }]
    );
    assert_eq!(runtime.rows[0].produced_count, 1);
    assert_eq!(runtime.rows[0].timer_us, 2_000_000);
    assert_eq!(runtime.ejection_slots, [HiveEjectionSlot::default(); 2]);
    assert_eq!(host.states[&100], 0x8000);
    assert_eq!(host.events().last(), Some(&Event::Random(1)));
    let before = host.events();
    runtime.advance_rows(visit(), &mut host);
    assert_eq!(host.events(), before);
    host.geometry = Ok(HiveEjectionGeometry {
        anchor_raw: [0; 3],
        model_extent_raw: 0,
    });
    assert_eq!(
        runtime.eject_child(request(100), &mut host),
        Err(HiveEjectionBlock::ZeroModelExtent)
    );
    assert_eq!(runtime.ejection_slots, [HiveEjectionSlot::default(); 2]);
    assert_eq!(host.events().last(), Some(&Event::Geometry(request(100))));
}

#[test]
fn final_ejection_equality_missing_and_blocked_slots_preserve_source_progress() {
    let mut runtime = HiveBirthRuntime::default();
    runtime.ejection_slots = [
        HiveEjectionSlot {
            child_handle: Some(10),
            timer_us: 20_000,
        },
        HiveEjectionSlot {
            child_handle: Some(11),
            timer_us: 20_000,
        },
    ];
    let mut host = Host {
        lookup_block: Some(10),
        ..Host::default()
    };
    host.states.insert(10, 0x4000);
    host.states.insert(11, 0x4000);
    assert_eq!(
        runtime.advance_ejections(20_000, &mut host),
        vec![HiveBirthBlock::EjectionTimer {
            slot_index: 0,
            block: "unowned child state"
        }]
    );
    assert_eq!(runtime.ejection_slots[0].timer_us, 20_000);
    assert_eq!(runtime.ejection_slots[1], HiveEjectionSlot::default());
    assert_eq!(host.states[&11], 0xc000);
    host.lookup_block = None;
    host.states.remove(&10);
    runtime.advance_ejections(1, &mut host);
    assert_eq!(runtime.ejection_slots, [HiveEjectionSlot::default(); 2]);
    assert!(!host.events().contains(&Event::Restore(10)));
}

#[v2k_test_support::retail_test]
fn actual_level_one_animation_runs_abort_row_and_retains_three_success_cap() {
    let data = v2k_test_support::retail_dir();
    let mut session = crate::session::GameSession::init(&data).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let spawn = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 67)
        .unwrap();
    let mut runtime = HiveBirthRuntime::from_animation(spawn.animation.as_ref().unwrap()).unwrap();
    assert_eq!(
        runtime.rows()[0].record(),
        HiveBirthRecord {
            entity_type: 15,
            initial_timer_us: 0,
            delay_base_us: 2_000_000,
            jitter_operand: 1_000_000,
            total_production_cap: 3,
            retained_child_cap: 3,
            flags: 3,
        }
    );
    let mut host = Host::with_births([100, 101, 102]);
    runtime.advance_rows(
        HiveBirthVisit {
            session_aborted: false,
            ..visit()
        },
        &mut host,
    );
    assert!(host.events().is_empty());
    for child in [100, 101, 102] {
        assert!(runtime
            .advance_rows(
                HiveBirthVisit {
                    elapsed_us: 3_000_000,
                    ..visit()
                },
                &mut host
            )
            .is_empty());
        assert!(runtime.advance_ejections(3_000_000, &mut host).is_empty());
        assert_eq!(host.states[&child] & 0x8000, 0x8000);
        host.states.insert(child, 0x4000);
    }
    assert_eq!(runtime.rows()[0].produced_count(), 3);
    let before = host.events();
    runtime.advance_rows(
        HiveBirthVisit {
            elapsed_us: 3_000_000,
            ..visit()
        },
        &mut host,
    );
    assert_eq!(
        host.events(),
        before,
        "death does not refund lifetime production or prune after total cap"
    );
}
