use super::*;

#[test]
fn handles_name_their_slot_entry_and_generation() {
    let mut table = HandleTable::new(9, 256);
    let first = table.allocate(3, 100).unwrap();
    // The free list hands out the last entry first, generation 1.
    assert_eq!(first, (3 << 26) | (255 << 16) | 1);
    assert_eq!(slot_of(first), 3);
    assert_eq!(table.lookup(first), Some(100));
    let second = table.allocate(3, 101).unwrap();
    assert_eq!(second, (3 << 26) | (254 << 16) | 1);
    // A freed entry goes to the tail, behind the fresh ones.
    table.free(first);
    assert_eq!(table.lookup(first), None);
    let third = table.allocate(3, 102).unwrap();
    assert_eq!(third >> 16 & 0x3FF, 253);
}

#[test]
fn a_reused_entry_advances_its_generation() {
    let mut table = HandleTable::new(2, 8);
    let handles: Vec<u32> = (0..7)
        .map(|body| table.allocate(1, body).unwrap())
        .collect();
    table.free(handles[2]);
    // The block is full but the freed entry; the next allocation takes it.
    let again = table.allocate(1, 50).unwrap();
    assert_eq!(again >> 16, handles[2] >> 16);
    assert_eq!(again as u16, 2);
    assert_eq!(table.lookup(handles[2]), None);
    assert_eq!(table.lookup(again), Some(50));
}

#[test]
fn a_full_block_doubles_and_keeps_its_handles() {
    let mut table = HandleTable::new(2, 8);
    let handles: Vec<u32> = (0..7)
        .map(|body| table.allocate(1, body).unwrap())
        .collect();
    let grown = table.allocate(1, 7).unwrap();
    assert_eq!(grown >> 16 & 0x3FF, 15);
    for (body, handle) in handles.iter().enumerate() {
        assert_eq!(table.lookup(*handle), Some(body as u32));
    }
}

/// A peer's handle registers at its own entry, which leaves the free list.
#[test]
fn a_peer_handle_registers_where_the_peer_put_it() {
    let mut table = HandleTable::new(9, 256);
    let remote = (2 << 26) | (255 << 16) | 4;
    assert!(table.register(remote, 9));
    assert_eq!(table.lookup(remote), Some(9));
    assert!(!table.register(remote, 10), "already registered");
    // Our own allocation in slot 2 would skip the taken entry.
    let next = table.allocate(2, 11).unwrap();
    assert_eq!(next >> 16 & 0x3FF, 254);
    assert_eq!(table.slot_handles(2), vec![(next, 11), (remote, 9)]);
}
