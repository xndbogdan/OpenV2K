use super::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Burst(u32),
    Clear(u32),
    Source(u32),
    Count,
    Random(u16),
    Construct(SplitChildRequest),
    Dispose(u32),
    Deferred(u32),
}

struct Host {
    events: Vec<Event>,
    source: SplitSourceOwnership,
    launch_source: SplitLaunchSource,
    launch_after_constructor: Option<SplitLaunchSource>,
    burst_return: u32,
    native_count: i32,
    random: VecDeque<u16>,
    attempts: VecDeque<Result<SplitChildAttempt<u32>, &'static str>>,
    constructor_draws: usize,
    disposal_block: bool,
    deferred_block: bool,
    linked_children: Vec<SplitChildRequest>,
}

impl Host {
    /// `native_count` is the exact native `468D00` result.
    fn local(entity_type: u32, native_count: i32) -> Self {
        Self {
            events: vec![],
            source: SplitSourceOwnership::Local(SplitSource {
                entity_type,
                requested_entity_handle_raw: 0x1234_5678,
            }),
            launch_source: SplitLaunchSource {
                position_raw: [10, 20, 30],
                heading_raw: 0x4000,
                objective: true,
            },
            launch_after_constructor: None,
            burst_return: 0,
            native_count,
            random: VecDeque::new(),
            attempts: VecDeque::new(),
            constructor_draws: 0,
            disposal_block: false,
            deferred_block: false,
            linked_children: vec![],
        }
    }
}

impl SplitAndExplodeHost for Host {
    type Block = &'static str;
    type ConstructorError = u32;

    fn emit_split_burst(&mut self, source_id: u32) -> Result<u32, Self::Block> {
        self.events.push(Event::Burst(source_id));
        Ok(self.burst_return)
    }

    fn clear_source_tasks(&mut self, source_id: u32) -> Result<(), Self::Block> {
        self.events.push(Event::Clear(source_id));
        Ok(())
    }

    fn source_after_task_clear(
        &mut self,
        source_id: u32,
    ) -> Result<SplitSourceOwnership, Self::Block> {
        self.events.push(Event::Source(source_id));
        Ok(self.source)
    }

    fn native_entity_list_count_raw(&mut self) -> Result<i32, Self::Block> {
        self.events.push(Event::Count);
        Ok(self.native_count)
    }

    fn next_shared_random_u16(&mut self) -> u16 {
        let word = self.random.pop_front().unwrap_or(0x8000);
        self.events.push(Event::Random(word));
        word
    }

    fn current_launch_source(&mut self, _source_id: u32) -> Result<SplitLaunchSource, Self::Block> {
        Ok(self.launch_source)
    }

    fn construct_split_child(
        &mut self,
        request: SplitChildRequest,
    ) -> Result<SplitChildAttempt<Self::ConstructorError>, Self::Block> {
        self.events.push(Event::Construct(request));
        for _ in 0..self.constructor_draws {
            self.next_shared_random_u16();
        }
        let attempt = self
            .attempts
            .pop_front()
            .unwrap_or(Ok(SplitChildAttempt::Created));
        if matches!(attempt, Ok(SplitChildAttempt::Created)) {
            self.linked_children.push(request);
            self.native_count = self.native_count.wrapping_add(1);
        }
        if let Some(launch) = self.launch_after_constructor.take() {
            self.launch_source = launch;
        }
        attempt
    }

    fn dispose_constructor_error(
        &mut self,
        error: Self::ConstructorError,
    ) -> Result<(), Self::Block> {
        self.events.push(Event::Dispose(error));
        if self.disposal_block {
            Err("disposal prefix")
        } else {
            Ok(())
        }
    }

    fn mark_source_deferred_destroy(&mut self, source_id: u32) -> Result<(), Self::Block> {
        self.events.push(Event::Deferred(source_id));
        if self.deferred_block {
            Err("deferred custody")
        } else {
            Ok(())
        }
    }
}

#[test]
fn class18_switch_preserves_all_recovered_families_and_default() {
    for (parent, child, count) in [
        (27, 3, 2),
        (31, 105, 8),
        (34, 4, 2),
        (40, 56, 8),
        (125, 125, 2),
    ] {
        let mut host = Host::local(parent, 20);
        let mut execution = SplitAndExplodeExecution::new(7);
        assert_eq!(
            execution.execute(&mut host),
            Ok(SplitAndExplodeCompletion::ChildrenComplete)
        );
        assert_eq!(execution.progress().created_children, count);
        assert_eq!(execution.progress().launch_rng_words, 9 * count);
        assert!(host
            .linked_children
            .iter()
            .all(|request| request.entity_type == child));
        assert_eq!(
            &host.events[..4],
            &[
                Event::Burst(7),
                Event::Clear(7),
                Event::Source(7),
                Event::Count
            ]
        );
        assert_eq!(host.events.last(), Some(&Event::Deferred(7)));
    }
}

#[test]
fn native_list_cap_keeps_parent_and_pending_actors_under_native_count_policy() {
    for (native_count, permitted) in [(71, 8), (72, 8), (73, 7), (79, 1), (80, 0), (81, 0)] {
        let mut host = Host::local(40, native_count);
        let mut execution = SplitAndExplodeExecution::new(7);
        assert_eq!(
            execution.execute(&mut host),
            Ok(SplitAndExplodeCompletion::ChildrenComplete)
        );
        assert_eq!(execution.progress().permitted_children, permitted);
        assert_eq!(execution.progress().created_children, permitted);
        assert_eq!(execution.progress().launch_rng_words, permitted * 9);
        assert_eq!(
            execution.progress().native_entity_list_count_raw,
            Some(native_count)
        );
        assert_eq!(
            host.events
                .iter()
                .filter(|event| **event == Event::Count)
                .count(),
            1
        );
        assert_eq!(host.native_count, native_count + permitted as i32);
    }
}

#[test]
fn birth_words_wrap_and_unwritten_record_fields_stay_zero() {
    let source = SplitSource {
        entity_type: 40,
        requested_entity_handle_raw: 0x1234_5678,
    };
    let launch = SplitLaunchSource {
        position_raw: [i16::MIN, i16::MAX, 123],
        heading_raw: 1,
        objective: true,
    };
    let child = child_request(
        source,
        launch,
        56,
        [
            0,
            u16::MAX,
            0x8000,
            0,
            u16::MAX,
            u16::MAX,
            0,
            u16::MAX,
            0x8000,
        ],
    );
    assert_eq!(child.position_raw, [32256, -32258, 123]);
    assert_eq!(child.velocity_raw, [-1024, 1023, 1023]);
    assert_eq!(child.rotation_heading_pitch_roll_raw, [0xf801, 0x07ff, 0]);
    let mut expected = [0; SPLIT_BIRTH_RECORD_BYTES];
    expected[0..4].copy_from_slice(&0x1234_5678_u32.to_le_bytes());
    expected[8..12].copy_from_slice(&56_u32.to_le_bytes());
    expected[0x0c..0x12].copy_from_slice(&[0x00, 0x7e, 0xfe, 0x81, 0x7b, 0x00]);
    expected[0x14..0x18].copy_from_slice(&1_u32.to_le_bytes());
    expected[0x18..0x1e].copy_from_slice(&[0x00, 0xfc, 0xff, 0x03, 0xff, 0x03]);
    expected[0x24..0x2a].copy_from_slice(&[0x01, 0xf8, 0xff, 0x07, 0x00, 0x00]);
    assert_eq!(child.to_native_record(), expected);
    let nonobjective = child_request(
        source,
        SplitLaunchSource {
            objective: false,
            ..launch
        },
        56,
        [0; 9],
    );
    assert_eq!(&nonobjective.to_native_record()[0x14..0x18], &[0; 4]);
}

#[test]
fn later_child_reads_current_parent_words_after_nested_constructor() {
    let mut host = Host::local(27, 20);
    host.launch_after_constructor = Some(SplitLaunchSource {
        position_raw: [300, 400, 500],
        heading_raw: 0x8000,
        objective: false,
    });
    let mut execution = SplitAndExplodeExecution::new(7);
    execution.execute(&mut host).unwrap();
    assert_eq!(host.linked_children[0].position_raw, [10, 20, 30]);
    assert_eq!(
        host.linked_children[0].rotation_heading_pitch_roll_raw[0],
        0x4000
    );
    assert!(host.linked_children[0].objective);
    assert_eq!(host.linked_children[1].position_raw, [300, 400, 500]);
    assert_eq!(
        host.linked_children[1].rotation_heading_pitch_roll_raw[0],
        0x8000
    );
    assert!(!host.linked_children[1].objective);
    assert_eq!(
        host.linked_children[0].requested_entity_handle_raw,
        host.linked_children[1].requested_entity_handle_raw
    );
    assert_eq!(execution.progress().launch_rng_words, 18);
}

#[test]
fn constructor_draws_interleave_between_complete_nine_word_prefixes() {
    let mut host = Host::local(27, 20);
    host.random = (0..20).map(|word| word * 64).collect();
    host.constructor_draws = 1;
    let mut execution = SplitAndExplodeExecution::new(7);
    execution.execute(&mut host).unwrap();
    assert_eq!(host.events[13], Event::Construct(host.linked_children[0]));
    assert_eq!(host.events[14], Event::Random(9 * 64));
    assert_eq!(host.linked_children[0].position_raw, [-502, -491, -480]);
    assert_eq!(host.linked_children[1].position_raw, [-492, -481, -470]);
    assert_eq!(execution.progress().launch_rng_words, 18);
    assert_eq!(
        host.events
            .iter()
            .filter(|event| matches!(event, Event::Random(_)))
            .count(),
        20
    );
}

#[test]
fn native_birth_error_disposes_then_stops_and_marks_parent() {
    let mut host = Host::local(40, 20);
    host.attempts = [
        Ok(SplitChildAttempt::Created),
        Ok(SplitChildAttempt::NativeError(99)),
    ]
    .into();
    let mut execution = SplitAndExplodeExecution::new(7);
    assert_eq!(
        execution.execute(&mut host),
        Ok(SplitAndExplodeCompletion::NativeConstructorError)
    );
    assert_eq!(execution.progress().attempted_children, 2);
    assert_eq!(execution.progress().created_children, 1);
    assert_eq!(execution.progress().launch_rng_words, 18);
    assert!(execution.progress().constructor_error_disposed);
    assert_eq!(
        &host.events[host.events.len() - 2..],
        &[Event::Dispose(99), Event::Deferred(7)]
    );
    let events = host.events.clone();
    assert_eq!(
        execution.execute(&mut host),
        Err(SplitAndExplodeBlock::AlreadyVisited)
    );
    assert_eq!(host.events, events);
}

#[test]
fn host_constructor_block_retains_prior_children_and_rng_without_replay() {
    let mut host = Host::local(40, 20);
    host.attempts = [
        Ok(SplitChildAttempt::Created),
        Err("native task publication"),
    ]
    .into();
    host.constructor_draws = 1;
    let mut execution = SplitAndExplodeExecution::new(7);
    assert_eq!(
        execution.execute(&mut host),
        Err(SplitAndExplodeBlock::Host {
            phase: SplitAndExplodePhase::ChildConstruction,
            reason: "native task publication",
        })
    );
    assert_eq!(host.linked_children.len(), 1);
    assert_eq!(execution.progress().attempted_children, 2);
    assert_eq!(execution.progress().launch_rng_words, 18);
    assert!(!execution.progress().source_deferred_destroyed);
    let events = host.events.clone();
    assert_eq!(
        execution.execute(&mut host),
        Err(SplitAndExplodeBlock::AlreadyVisited)
    );
    assert_eq!(host.events, events);
}

#[test]
fn burst_nonzero_skips_entire_suffix_but_remote_still_clears_and_marks() {
    let mut host = Host::local(40, 20);
    host.burst_return = 42;
    let mut execution = SplitAndExplodeExecution::new(7);
    assert_eq!(
        execution.execute(&mut host),
        Ok(SplitAndExplodeCompletion::BurstReturned(42))
    );
    assert_eq!(host.events, [Event::Burst(7)]);
    assert_eq!(execution.progress().burst_return_raw, Some(42));
    let mut host = Host::local(40, 20);
    host.source = SplitSourceOwnership::Remote;
    let mut execution = SplitAndExplodeExecution::new(7);
    assert_eq!(
        execution.execute(&mut host),
        Ok(SplitAndExplodeCompletion::Remote)
    );
    assert_eq!(
        host.events,
        [
            Event::Burst(7),
            Event::Clear(7),
            Event::Source(7),
            Event::Deferred(7)
        ]
    );
    assert_eq!(execution.progress().launch_rng_words, 0);
    assert!(execution.progress().source_deferred_destroyed);
}

#[test]
fn disposal_or_terminal_block_retains_completed_prefix_and_never_repeats_it() {
    for disposal in [true, false] {
        let mut host = Host::local(40, 20);
        host.attempts = [Ok(SplitChildAttempt::NativeError(99))].into();
        host.disposal_block = disposal;
        host.deferred_block = !disposal;
        let mut execution = SplitAndExplodeExecution::new(7);
        assert!(
            matches!(execution.execute(&mut host), Err(SplitAndExplodeBlock::Host { phase, .. })
            if phase == if disposal { SplitAndExplodePhase::ErrorDisposal } else { SplitAndExplodePhase::DeferredDestroy })
        );
        assert_eq!(execution.progress().launch_rng_words, 9);
        assert_eq!(execution.progress().constructor_error_disposed, !disposal);
        assert!(!execution.progress().source_deferred_destroyed);
        let events = host.events.clone();
        assert_eq!(
            execution.execute(&mut host),
            Err(SplitAndExplodeBlock::AlreadyVisited)
        );
        assert_eq!(host.events, events);
    }
}
