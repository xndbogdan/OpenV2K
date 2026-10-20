//! The nine-slot player table ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Player slots").
//!
//! `004F73F0` holds the local DirectPlay id in entry 0 and the players of
//! slots 1..=8 after it. The slot authority (the host that opened the
//! session) gives every player a slot; the others learn theirs from its
//! table, which travels as the session channel's only record.

use super::framing::Record;

/// Slots 1..=8, with entry 0 the local id.
pub const SLOTS: usize = 9;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotTable {
    /// `004F73F0`.
    pub ids: [u32; SLOTS],
    /// `004F7378`: the local slot, 0 while unknown.
    pub local: u8,
    /// `004F737C`: set whenever the local slot may have changed.
    pub changed: bool,
    /// `004F73D4`: this machine assigns the slots.
    pub authority: bool,
    /// `004F73DC`: a newcomer asked for the table to go out now.
    pub broadcast_requests: u32,
    /// `004F7424`: timer visits since the table last went out.
    pub visits: u32,
}

impl SlotTable {
    /// `FUN_0046BED0`'s start: the table cleared with the local id first.
    pub fn start(local_id: u32, authority: bool) -> Self {
        let mut table = Self {
            authority,
            changed: true,
            ..Self::default()
        };
        table.ids[0] = local_id;
        table
    }

    /// `FUN_0046BD80`, once a second: the authority places itself, then
    /// every twelfth visit (or at once on request) the table goes out; a
    /// player without a slot sends its id every visit. Returns whether to
    /// send the session record.
    pub fn timer(&mut self) -> bool {
        if !self.authority {
            if self.local == 0 || self.broadcast_requests != 0 {
                self.visits = 0;
                self.broadcast_requests = 0;
                return true;
            }
            return false;
        }
        if self.local == 0 {
            if let Some(slot) = (1..SLOTS).find(|&slot| self.ids[slot] == self.ids[0]) {
                self.local = slot as u8;
                self.changed = true;
            } else if let Some(slot) = (1..SLOTS).find(|&slot| self.ids[slot] == 0) {
                self.ids[slot] = self.ids[0];
                self.local = slot as u8;
                self.changed = true;
            }
        }
        if self.broadcast_requests == 0 && self.visits < 11 {
            self.visits += 1;
            return false;
        }
        self.visits = 0;
        self.broadcast_requests = 0;
        true
    }

    /// `FUN_0046BB10`: a peer's record. The authority places a newcomer in
    /// the first free slot and asks for the table to go out; the others take
    /// the authority's table and find their own slot in it. Returns the
    /// record's size.
    pub fn apply(&mut self, record: &[u8]) -> usize {
        let Some(count) = word(record, 0) else {
            return record.len();
        };
        let size = count as usize * 4 + 4;
        if !self.authority {
            if count > 1 {
                for index in 1..(count as usize).min(SLOTS) {
                    let Some(id) = word(record, 4 + index * 4) else {
                        break;
                    };
                    self.ids[index] = id;
                    if id == self.ids[0] && index as u8 != self.local {
                        // A slot that moved is forgotten first, then retaken.
                        self.local = if self.local != 0 { 0 } else { index as u8 };
                        self.changed = true;
                    }
                }
            }
        } else if let Some(id) = word(record, 4) {
            if !(1..SLOTS).any(|slot| self.ids[slot] == id) {
                if let Some(slot) = (1..SLOTS).find(|&slot| self.ids[slot] == 0) {
                    self.ids[slot] = id;
                    self.broadcast_requests += 1;
                }
            }
        }
        size
    }

    /// `FUN_0046C300`'s destroyed player frees its slot.
    pub fn remove(&mut self, id: u32) {
        if let Some(slot) = (1..SLOTS).find(|&slot| self.ids[slot] == id) {
            self.ids[slot] = 0;
        }
    }

    /// The slot holding `id`, if any.
    pub fn slot_of(&self, id: u32) -> Option<u8> {
        (1..SLOTS)
            .find(|&slot| self.ids[slot] == id)
            .map(|slot| slot as u8)
    }
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// `FUN_0046BC10`: the authority's whole table, or the local id alone.
pub struct SlotRecord<'a>(pub &'a SlotTable);

impl Record for SlotRecord<'_> {
    fn kind(&self) -> u8 {
        0
    }
    /// The descriptor's size word at `0x4D49D4`.
    fn size_word(&self) -> usize {
        8
    }
    fn pack(&self, room: &mut [u8]) -> Option<usize> {
        let count = if self.0.authority { SLOTS } else { 1 };
        let size = count * 4 + 4;
        let target = room.get_mut(..size)?;
        target[..4].copy_from_slice(&(count as u32).to_le_bytes());
        for (index, id) in self.0.ids[..count].iter().enumerate() {
            target[4 + index * 4..8 + index * 4].copy_from_slice(&id.to_le_bytes());
        }
        Some(size)
    }
}

#[cfg(test)]
#[path = "slots_tests.rs"]
mod tests;
