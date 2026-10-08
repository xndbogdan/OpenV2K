//! The retail queue controls: each expectation below was observed by
//! executing the original `FUN_004948C0..FUN_00494B60`, `FUN_00459D10` and
//! `FUN_0045B220` instructions on the same operation sequence, with a fill
//! callback that records each record's payload identifier.

use super::*;

const SLOT: FillSlot = FillSlot::FlatTriangle;

/// Append a record carrying `id` the way the controls do (`FUN_0045B280`
/// semantics) and return its offset from the root.
fn append(queue: &mut PrimitiveQueue, key: i32, id: u32, payload_bytes: usize) -> usize {
    let offset = queue.used_bytes() + ROOT_HEADER as usize;
    let payload = queue
        .push(key, SLOT, payload_bytes)
        .expect("record fits the arena");
    payload[..4].copy_from_slice(&id.to_le_bytes());
    offset
}

/// Drain and return the result and the payload identifiers filled, stopping
/// with `stop.1` at identifier `stop.0`.
fn drain(queue: &mut PrimitiveQueue, stop: Option<(u32, u32)>) -> (u32, Vec<u32>) {
    let mut ids = Vec::new();
    let result = queue.flush(&mut |slot, payload| {
        assert_eq!(slot, SLOT);
        let id = u32::from_le_bytes(payload[..4].try_into().unwrap());
        ids.push(id);
        match stop {
            Some((at, result)) if at == id => result,
            _ => 0,
        }
    });
    (result, ids)
}

#[test]
fn signed_descending_keys_break_ties_by_arena_address() {
    let mut queue = PrimitiveQueue::new(2048).unwrap();
    let offsets: Vec<usize> = [
        (-1, 1),
        (7, 2),
        (7, 3),
        (i32::MIN, 4),
        (i32::MAX, 5),
        (0, 6),
    ]
    .into_iter()
    .map(|(key, id)| append(&mut queue, key, id, 4))
    .collect();
    assert_eq!(offsets, [24, 40, 56, 72, 88, 104]);
    let first = drain(&mut queue, None);
    assert_eq!(first, (0, vec![5, 2, 3, 6, 1, 4]));
    // Draining does not consume records.
    assert_eq!(drain(&mut queue, None), first);
}

fn nested(queue: &mut PrimitiveQueue) {
    append(queue, 5, 10, 4);
    queue.begin_fifo_group(10).unwrap();
    append(queue, 1, 11, 4);
    append(queue, 100, 12, 4);
    queue.begin_sorted_group(3).unwrap();
    append(queue, 1, 13, 4);
    append(queue, 9, 14, 4);
    queue.end_group().unwrap();
    append(queue, -5, 15, 4);
    queue.end_group().unwrap();
    append(queue, 9, 16, 4);
}

#[test]
fn nested_fifo_and_sorted_groups_drain_atomically() {
    let mut queue = PrimitiveQueue::new(2048).unwrap();
    nested(&mut queue);
    assert_eq!(
        drain(&mut queue, None),
        (0, vec![11, 12, 14, 13, 15, 16, 10])
    );
}

#[test]
fn first_nonzero_fill_result_stops_the_parent_lists() {
    let mut queue = PrimitiveQueue::new(2048).unwrap();
    nested(&mut queue);
    assert_eq!(drain(&mut queue, Some((12, 73))), (73, vec![11, 12]));
}

#[test]
fn flushing_the_root_closes_open_groups() {
    let mut queue = PrimitiveQueue::new(2048).unwrap();
    queue.begin_fifo_group(10).unwrap();
    append(&mut queue, 1, 17, 4);
    assert_eq!(drain(&mut queue, None), (0, vec![17]));
    assert!(!queue.group_open());
}

#[test]
fn primitive_overflow_leaves_the_cursor_and_does_not_flush() {
    let mut queue = PrimitiveQueue::new(128).unwrap();
    let offsets: Vec<usize> = (0..4)
        .map(|key| append(&mut queue, key, key as u32, 5))
        .collect();
    assert_eq!(offsets, [24, 44, 64, 84]);
    assert_eq!(queue.used_bytes() + 24, 104);
    assert_eq!(
        queue.push(5, SLOT, 5).unwrap_err(),
        QueueError::PrimitiveOverflow
    );
    assert_eq!(queue.used_bytes() + 24, 104);
}

#[test]
fn group_overflow_preserves_the_current_scope() {
    let mut queue = PrimitiveQueue::new(128).unwrap();
    for _ in 0..3 {
        queue.begin_fifo_group(0).unwrap();
        queue.end_group().unwrap();
    }
    assert_eq!(queue.used_bytes() + 24, 108);
    assert_eq!(
        queue.begin_sorted_group(0).unwrap_err(),
        QueueError::GroupOverflow
    );
    assert_eq!(queue.used_bytes() + 24, 108);
    assert!(!queue.group_open());
    assert_eq!(queue.end_group().unwrap_err(), QueueError::EndAtRoot);
}

#[test]
fn reset_discards_records() {
    let mut queue = PrimitiveQueue::new(WORLD_ARENA_BYTES).unwrap();
    append(&mut queue, 3, 1, 24);
    queue.reset();
    assert_eq!(queue.used_bytes(), 0);
    assert_eq!(drain(&mut queue, None), (0, vec![]));
}

#[test]
fn every_fill_slot_has_its_own_thunk() {
    let mut seen = std::collections::HashSet::new();
    for slot in FillSlot::ALL {
        let thunk = thunk_address(slot);
        assert!(seen.insert(thunk));
        assert_eq!(callback_kind(thunk), QueueCallback::Fill(slot));
    }
    assert_eq!(thunk_address(FillSlot::WordImage), 0x0047_A720);
    assert_eq!(thunk_address(FillSlot::FlatTriangle), 0x0047_A7E0);
    assert_eq!(thunk_address(FillSlot::FlatQuad), 0x0047_A960);
    assert_eq!(thunk_address(FillSlot::MappedShadedFogQuad), 0x0047_AB00);
}

#[test]
fn arenas_below_128_bytes_are_rejected() {
    assert_eq!(
        PrimitiveQueue::new(0x7F).unwrap_err(),
        QueueError::ArenaTooSmall
    );
}
