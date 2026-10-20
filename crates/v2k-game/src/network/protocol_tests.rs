use super::*;

fn packed(record: &GameRecord) -> Vec<u8> {
    let mut room = [0xEEu8; 300];
    let size = record.pack(&mut room).unwrap();
    room[..size].to_vec()
}

fn round_trip(record: GameRecord) -> Vec<u8> {
    let bytes = packed(&record);
    let (parsed, size) = GameRecord::parse(record.kind_of(), &bytes).unwrap();
    assert_eq!(parsed, record);
    assert!(size <= bytes.len());
    bytes
}

#[test]
fn the_stamp_carries_session_counter_world_and_slots() {
    let stamp = Stamp {
        session: 0x01020304,
        counter: 7,
        world: 13,
        slot: 2,
        host_slot: 0,
    };
    let bytes = stamp.pack();
    assert_eq!(bytes, [4, 3, 2, 1, 7, 0, 0, 0, 13, 0, 2, 0]);
    assert_eq!(Stamp::parse(&bytes), Some(stamp));
}

/// `FUN_00417DB0` puts the damage kind plus 6 below the sender's slot.
#[test]
fn damage_carries_the_kind_and_the_senders_slot() {
    let bytes = round_trip(GameRecord::Damage {
        handle: 0x0C00_FF01,
        amount: 1800,
        kind_slot: (1 + 6) | (3 << 8),
    });
    assert_eq!(bytes, [1, 0xFF, 0, 0x0C, 0x08, 0x07, 7, 3]);
}

#[test]
fn scores_names_and_lines_keep_retail_sizes() {
    let mut table = [[0u16; 3]; 8];
    table[0] = [2, 1, 45];
    let scores = round_trip(GameRecord::Scores {
        limit_winner: 100,
        table,
    });
    assert_eq!(scores.len(), 52);
    assert_eq!(&scores[4..10], &[2, 0, 1, 0, 45, 0]);
    let name = round_trip(GameRecord::Name {
        initials: *b"BOB\0\0\0",
        name: *b"Bob\0\0\0\0\0\0\0\0\0\0\0\0\0",
    });
    assert_eq!(name.len(), 32);
    let line = round_trip(GameRecord::Line(b"Bob blew up".to_vec()));
    assert_eq!(line[0], 11);
    assert_eq!(GameRecord::parse(21, &line).unwrap().1, 12);
    let long = packed(&GameRecord::Line(vec![b'x'; 300]));
    assert_eq!(long.len(), 251);
}

/// Kinds 22..=25 select their optional words by flags.
#[test]
fn a_sound_takes_the_kind_its_optional_words_name() {
    for (rate, gain, kind, size) in [
        (None, None, 22, 4),
        (Some(0x12000), None, 23, 8),
        (None, Some(0x8000), 24, 8),
        (Some(1), Some(2), 25, 12),
    ] {
        let record = GameRecord::Sound {
            word: 0x30_20_10_3E,
            rate,
            gain,
        };
        assert_eq!(record.kind_of(), kind);
        assert_eq!(round_trip(record).len(), size);
    }
}

#[test]
fn a_body_record_carries_the_fields_its_flags_name() {
    let full = BodySync {
        handle: (2 << 26) | (255 << 16) | 1,
        resend: 3,
        flags: 0x1 | 0x2 | 0x8 | 0x10 | 0x20 | 0x80 | 0x100 | 0x200 | 0x400 | 0x1000,
        extras: 7,
        position: [100, -200, 300],
        entity_type: 51,
        velocity: [1, 2, 3],
        euler: [0; 3],
        basis: [4096, 0, 0, 0, 0, -4096],
        capability: 9,
        owner: 11,
        words: [1, 2, 3, 4],
        sequence: 5,
        shield: 40000,
        payload: 0x1B0,
        model_variables: vec![0x10, 0x20, 0x30],
        parent: 77,
    };
    let bytes = round_trip(GameRecord::Body(full.clone()));
    assert_eq!(
        bytes.len(),
        10 + 2 + 6 + 4 + 6 + 12 + 4 + 4 + 8 + 2 + 4 + 4 + 8 + 4
    );
    let minimal = BodySync {
        handle: 1 << 26 | 1 << 16 | 1,
        flags: 0x20,
        position: [1, 2, 3],
        ..BodySync::default()
    };
    assert_eq!(round_trip(GameRecord::Body(minimal)).len(), 16);
}

#[test]
fn lists_and_reports_round_trip() {
    let list = round_trip(GameRecord::LiveList(vec![5, 6, 7]));
    assert_eq!(list.len(), 16);
    assert_eq!(
        round_trip(GameRecord::KillReport {
            victim: 2,
            attacker: 1
        }),
        [2, 1]
    );
    round_trip(GameRecord::Impact {
        handle: 9,
        deltas: [1, -1, 2, -2, 3, -3],
    });
    round_trip(GameRecord::Release {
        child: 4,
        parent: 5,
    });
    round_trip(GameRecord::Attach([1, 2, 3]));
}
