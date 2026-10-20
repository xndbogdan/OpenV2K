//! Retail's message framing ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Messages and channels").
//!
//! A message is one DirectPlay `Send`: a four-byte header (its length, two
//! bytes; the channel; the sender's slot), the channel's header record, then
//! records. Records come in groups of four, each group led by a dword of
//! four kind bytes; every record and header part is rounded up to four
//! bytes. `FUN_00469110` starts a message, `FUN_00469200` appends a record
//! (sending the message first when the record's size word no longer fits),
//! `FUN_0046C050` sends it, and `FUN_004693A0` walks a received one.

/// `DAT_004D4900`: `FUN_00468FD0` clamps the provider's buffer to
/// 0x80..=0x400 bytes; the port's transport carries the largest.
pub const MESSAGE_SIZE: usize = 0x400;

/// Retail's two channels (records at `004F7340`, 0x1C bytes apart).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// The session channel: one record kind, the slot table, applied as it
    /// arrives (descriptor `0x4D49E8`).
    Session = 0,
    /// The game channel: the 12-byte stamp, then kinds 0..=25 of the table
    /// at `0x4CAE08`, queued for the frame's drain (descriptor `0x4CAFA8`).
    Game = 1,
}

/// A record kind's packer: writes into `room` and returns the bytes used,
/// or `None` when the record does not fit (retail's -1).
pub trait Record {
    fn kind(&self) -> u8;
    /// The table's size word: the room a message must have left before
    /// `FUN_00469200` tries to pack this kind into it.
    fn size_word(&self) -> usize;
    fn pack(&self, room: &mut [u8]) -> Option<usize>;
}

/// An outgoing message being filled (`004F7340`'s record).
pub struct Outgoing {
    channel: Channel,
    buffer: Vec<u8>,
    /// `+8`: bytes used, the header and every counted record.
    used: usize,
    /// `+C`: room left for records.
    room: usize,
    /// `+10`: where the current group's kind bytes go.
    kinds_at: usize,
    /// `+14`: where the next record goes.
    cursor: usize,
    /// `+18`: records in this message.
    count: usize,
}

const fn round4(size: usize) -> usize {
    (size + 3) & !3
}

impl Outgoing {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel,
            buffer: vec![0; MESSAGE_SIZE],
            used: 0,
            room: 0,
            kinds_at: 0,
            cursor: 0,
            count: 0,
        }
    }

    /// `FUN_00469110`: returns the pending message (when it holds a record)
    /// and starts a fresh one with the sender's slot and the channel's header
    /// record from `header` (the game channel's stamp).
    pub fn begin(&mut self, slot: u8, header: Option<&[u8]>) -> Option<Vec<u8>> {
        let pending = self.take();
        self.buffer.fill(0);
        self.buffer[2] = self.channel as u8;
        self.buffer[3] = slot;
        let mut used = 4;
        if let Some(header) = header {
            self.buffer[4..4 + header.len()].copy_from_slice(header);
            used += round4(header.len());
        }
        self.used = used;
        self.kinds_at = used;
        self.room = MESSAGE_SIZE - used - 4;
        self.cursor = used + 4;
        self.count = 0;
        pending
    }

    /// `FUN_0046C050`: the message to send, its length in its first two
    /// bytes, when it holds a record; afterwards nothing fits until the next
    /// `begin`.
    pub fn take(&mut self) -> Option<Vec<u8>> {
        let message = (self.count != 0 && self.used != 0).then(|| {
            let mut message = self.buffer[..self.used].to_vec();
            message[..2].copy_from_slice(&(self.used as u16).to_le_bytes());
            message
        });
        self.count = 0;
        self.room = 0;
        message
    }

    /// `FUN_00469200`'s append once the message has room: packs `record`,
    /// rounds it up and records its kind; `false` when it did not fit.
    pub fn append(&mut self, record: &dyn Record) -> bool {
        if self.room < record.size_word() {
            return false;
        }
        let end = self.cursor + self.room;
        let Some(size) = record.pack(&mut self.buffer[self.cursor..end]) else {
            return false;
        };
        if size == 0 {
            return true;
        }
        let size = round4(size).min(self.room);
        self.cursor += size;
        self.used += size;
        self.room -= size;
        self.buffer[self.kinds_at] = record.kind();
        self.kinds_at += 1;
        if self.count & 3 == 0 {
            self.used += 4;
        }
        self.count += 1;
        if self.count & 3 == 0 {
            if self.room > 7 {
                self.kinds_at = self.cursor;
                self.cursor += 4;
                self.room -= 4;
            } else {
                self.room = 0;
            }
        }
        true
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// A received message's records: each kind byte and the bytes from its
/// record to the message's end. `apply` returns the record's size, as the
/// retail apply routines do; `FUN_004693A0` stops at the end or at a kind
/// beyond the channel's table (`kinds`).
pub fn walk_records(
    message: &[u8],
    header_size: usize,
    kinds: u8,
    mut apply: impl FnMut(u8, &[u8]) -> usize,
) {
    let Some(length) = message.get(..2).map(|b| i16::from_le_bytes([b[0], b[1]])) else {
        return;
    };
    let total = usize::try_from(length).unwrap_or(0).min(message.len());
    let mut offset = 4 + round4(header_size);
    let mut kinds_at = 0;
    let mut index = 0;
    while offset < total {
        if index & 3 == 0 {
            kinds_at = offset;
            offset += 4;
            if offset > total {
                return;
            }
        }
        let kind = message[kinds_at + (index & 3)];
        if kind >= kinds {
            return;
        }
        let size = round4(apply(kind, &message[offset..total]));
        offset += size;
        index += 1;
        if size == 0 {
            return;
        }
    }
}

/// The sender's channel and slot from a received message's header.
pub fn header(message: &[u8]) -> Option<(u8, u8)> {
    (message.len() >= 4).then(|| (message[2], message[3]))
}

#[cfg(test)]
#[path = "framing_tests.rs"]
mod tests;
