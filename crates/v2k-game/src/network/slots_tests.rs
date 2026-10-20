use super::*;

fn packed(table: &SlotTable) -> Vec<u8> {
    let mut room = [0u8; 64];
    let size = SlotRecord(table).pack(&mut room).unwrap();
    room[..size].to_vec()
}

/// The authority takes slot 1, places a newcomer and sends the table at
/// once; the newcomer learns its slot from it.
#[test]
fn the_authority_assigns_slots_and_the_guest_learns_its_own() {
    let mut host = SlotTable::start(0x1111, true);
    let mut guest = SlotTable::start(0x2222, false);
    assert!(!host.timer());
    assert_eq!(host.local, 1);
    assert_eq!(host.ids[1], 0x1111);

    // The guest, without a slot, sends its id every visit.
    assert!(guest.timer());
    let ask = packed(&guest);
    assert_eq!(ask, [1, 0, 0, 0, 0x22, 0x22, 0, 0]);
    assert_eq!(host.apply(&ask), 8);
    assert_eq!(host.ids[2], 0x2222);
    assert!(host.timer(), "a newcomer sends the table at once");

    let table = packed(&host);
    assert_eq!(table.len(), 40);
    assert_eq!(guest.apply(&table), 40);
    assert_eq!(guest.local, 2);
    assert_eq!(guest.ids[1], 0x1111);
    assert!(!guest.timer(), "a placed guest stays quiet");
}

#[test]
fn the_table_goes_out_every_twelfth_visit() {
    let mut host = SlotTable::start(7, true);
    let sends: Vec<bool> = (0..24).map(|_| host.timer()).collect();
    assert_eq!(sends.iter().filter(|sent| **sent).count(), 2);
    assert!(sends[11] && sends[23]);
}

#[test]
fn a_destroyed_player_frees_its_slot() {
    let mut host = SlotTable::start(7, true);
    host.timer();
    host.apply(&[1, 0, 0, 0, 9, 0, 0, 0]);
    assert_eq!(host.slot_of(9), Some(2));
    host.remove(9);
    assert_eq!(host.slot_of(9), None);
}
