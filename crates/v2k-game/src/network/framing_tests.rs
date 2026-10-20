use super::*;

struct Bytes(u8, Vec<u8>, usize);

impl Record for Bytes {
    fn kind(&self) -> u8 {
        self.0
    }
    fn size_word(&self) -> usize {
        self.2
    }
    fn pack(&self, room: &mut [u8]) -> Option<usize> {
        let target = room.get_mut(..self.1.len())?;
        target.copy_from_slice(&self.1);
        Some(self.1.len())
    }
}

fn walk(message: &[u8], header: usize) -> Vec<(u8, Vec<u8>)> {
    let mut seen = Vec::new();
    walk_records(message, header, 26, |kind, data| {
        let size = match kind {
            7 => 6,
            _ => 4,
        };
        seen.push((kind, data[..size].to_vec()));
        size
    });
    seen
}

/// The header carries the length, channel and slot; the stamp follows; each
/// group of four records starts with its kind bytes; records round up.
#[test]
fn records_group_their_kinds_and_round_to_four_bytes() {
    let mut out = Outgoing::new(Channel::Game);
    assert_eq!(out.begin(3, Some(&[0xAA; 12])), None);
    for (index, kind) in [9u8, 7, 8, 16, 21].into_iter().enumerate() {
        let data = if kind == 7 {
            vec![index as u8; 6]
        } else {
            vec![index as u8; 4]
        };
        assert!(out.append(&Bytes(kind, data, 4)));
    }
    let message = out.take().unwrap();
    // 4 + 12 stamp + 4 kinds + 4 + 8 + 4 + 4 + 4 kinds + 4.
    assert_eq!(message.len(), 48);
    assert_eq!(&message[..4], &[48, 0, 1, 3]);
    assert_eq!(&message[16..20], &[9, 7, 8, 16]);
    assert_eq!(&message[40..44], &[21, 0, 0, 0]);
    assert_eq!(
        walk(&message, 12),
        vec![
            (9, vec![0; 4]),
            (7, vec![1; 6]),
            (8, vec![2; 4]),
            (16, vec![3; 4]),
            (21, vec![4; 4]),
        ]
    );
}

#[test]
fn an_empty_message_is_not_sent_and_a_full_one_is_flushed_first() {
    let mut out = Outgoing::new(Channel::Session);
    out.begin(1, None);
    assert_eq!(out.take(), None);
    out.begin(1, None);
    let big = Bytes(0, vec![1; 600], 600);
    assert!(out.append(&big));
    // The second no longer fits: the caller sends and begins again.
    assert!(!out.append(&big));
    let sent = out.begin(1, None).unwrap();
    assert_eq!(sent.len(), 4 + 4 + 600);
    assert!(out.append(&big));
}

#[test]
fn a_kind_beyond_the_table_ends_the_walk() {
    let mut out = Outgoing::new(Channel::Game);
    out.begin(2, Some(&[0; 12]));
    out.append(&Bytes(4, vec![1; 4], 4));
    out.append(&Bytes(30, vec![2; 4], 4));
    out.append(&Bytes(4, vec![3; 4], 4));
    let message = out.take().unwrap();
    assert_eq!(walk(&message, 12), vec![(4, vec![1; 4])]);
}
