//! The per-frame primitive queue: `FUN_00494860..FUN_00494B60` with the push
//! helpers `FUN_00459D10`, `FUN_0045B220` and `FUN_0045B280`.
//!
//! Producers do not draw. They allocate a record in one bump arena, link it
//! into the current list and fill its payload; at the end of the frame the
//! lists are sorted and drained through the fill slots. The port keeps the
//! arena as bytes with the retail layout, so record contents, links, keys
//! and overflow behave exactly as in the original:
//!
//! - the root header (24 bytes) holds the current scope's mode (1 sorted,
//!   0 FIFO), the root list head, the current tail (address of the last
//!   `next` field), the current scope header, the limit and the cursor;
//! - a FIFO record is `{next, callback, payload}` and a sorted record
//!   `{key, next, callback, payload}`; a list links `next` fields, so the
//!   callback always sits at `node + 4` and the payload at `node + 8`;
//! - a group record's payload is a child list header `{saved mode, head,
//!   saved tail, saved scope}`, and its callback sorts and drains
//!   (`FUN_00494930`) or just drains (`FUN_00494A50`) the child list;
//! - sorting orders a list by descending signed key, equal keys by
//!   ascending record address, and rewrites its links in place;
//! - draining stops at the first callback that returns non-zero.
//!
//! Arena addresses are offsets from [`ARENA_BASE`]; only their order is
//! observable (equal-key ties), and that matches allocation order.

use super::slots::FillSlot;

/// Address the arena is modelled at. Any non-zero base gives the same
/// behaviour; zero is reserved for null links.
const ARENA_BASE: u32 = 0x0100_0000;
/// Bytes of the root header (`FUN_004948C0` starts the cursor after it).
const ROOT_HEADER: u32 = 0x18;
/// `FUN_00494860` keeps the last twelve arena bytes free: a record header is
/// written before the limit check.
const RESERVED_TAIL: u32 = 12;

/// `FUN_0044F540`: the world viewport's arena request.
pub const WORLD_ARENA_BYTES: usize = 0x19000;

/// Group callbacks stored in group records.
const SORT_AND_DISPATCH: u32 = 0x0049_4930;
const DISPATCH: u32 = 0x0049_4A50;

/// Error records the queue functions return or raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// `FUN_00494860`: fewer than 0x80 arena bytes (`0x004C5508`).
    ArenaTooSmall,
    /// `FUN_00459D10` / `FUN_0045B220` raise `0x004C50E8` / `0x004C50D8`
    /// through `FUN_00471150` when a record would pass the limit. The record
    /// is not linked and the cursor does not move; retail unwinds to the
    /// nearest error handler instead of flushing.
    PrimitiveOverflow,
    /// `FUN_00494AB0` / `FUN_00494B60` return `0x004C54C8` (sorted scope) or
    /// `0x004C54D8` (FIFO scope) when the group record does not fit. The
    /// current scope is unchanged.
    GroupOverflow,
    /// `FUN_00494A80` at the root returns `0x004C5510`.
    EndAtRoot,
}

/// What a record's callback word names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueCallback {
    /// A fill-slot thunk (`0x0047A720..0x0047AB00`).
    Fill(FillSlot),
    /// A group whose child list is sorted before it is drained.
    SortedGroup,
    /// A group whose child list is drained in order.
    FifoGroup,
    /// A word no queue writer stores (for example a FIFO record read as a
    /// sorted one).
    Other(u32),
}

/// The retail thunk that calls `slot` through the device fill table.
pub fn thunk_address(slot: FillSlot) -> u32 {
    let offset = u32::from(slot.offset());
    match offset {
        0x1020..=0x1064 => 0x0047_A720 + (offset - 0x1020) * 8,
        0x1080..=0x10AC => 0x0047_A960 + (offset - 0x1080) * 8,
        0x10C0 => 0x0047_AAE0,
        0x10C4 => 0x0047_AB00,
        _ => unreachable!("fill slot +{offset:04X} has no queue thunk"),
    }
}

fn callback_kind(word: u32) -> QueueCallback {
    match word {
        SORT_AND_DISPATCH => QueueCallback::SortedGroup,
        DISPATCH => QueueCallback::FifoGroup,
        _ => FillSlot::ALL
            .iter()
            .copied()
            .find(|&slot| thunk_address(slot) == word)
            .map_or(QueueCallback::Other(word), QueueCallback::Fill),
    }
}

/// One primitive queue arena with its root header at the start.
#[derive(Debug, Clone)]
pub struct PrimitiveQueue {
    arena: Vec<u8>,
}

impl PrimitiveQueue {
    /// `FUN_00494860` + `FUN_004948C0`: an arena of `bytes` (rounded down
    /// to a multiple of four) with an empty sorted root.
    pub fn new(bytes: usize) -> Result<Self, QueueError> {
        if bytes < 0x80 {
            return Err(QueueError::ArenaTooSmall);
        }
        let bytes = bytes & !3;
        let mut queue = Self {
            arena: vec![0; bytes],
        };
        let limit = ARENA_BASE + bytes as u32 - RESERVED_TAIL;
        queue.write(ARENA_BASE + 0x10, limit);
        queue.reset();
        Ok(queue)
    }

    /// `FUN_004948C0`: forget every record and reopen the sorted root.
    pub fn reset(&mut self) {
        let root = ARENA_BASE;
        self.write(root, 1);
        self.write(root + 8, root + 4);
        self.write(root + 0xC, root);
        self.write(root + 4, 0);
        self.write(root + 0x14, root + ROOT_HEADER);
    }

    /// Bytes allocated after the root header.
    pub fn used_bytes(&self) -> usize {
        (self.read(ARENA_BASE + 0x14) - ARENA_BASE - ROOT_HEADER) as usize
    }

    /// Whether the current scope sorts its records.
    pub fn sorted_scope(&self) -> bool {
        self.read(ARENA_BASE) != 0
    }

    /// Whether a group is still open.
    pub fn group_open(&self) -> bool {
        self.read(ARENA_BASE + 0xC) != ARENA_BASE
    }

    /// `FUN_00459D10`: append a FIFO record (no key) for `slot` and return
    /// its payload, `payload_bytes` rounded up to whole dwords. Like the
    /// retail arena, the payload still holds whatever an earlier frame left
    /// there; producers write every field their handler reads.
    pub fn push_fifo(
        &mut self,
        slot: FillSlot,
        payload_bytes: usize,
    ) -> Result<&mut [u8], QueueError> {
        let root = ARENA_BASE;
        let record = self.read(root + 0x14);
        self.write(record, 0);
        self.write(record + 4, thunk_address(slot));
        let end = record + words(payload_bytes) * 4 + 8;
        if end > self.read(root + 0x10) {
            return Err(QueueError::PrimitiveOverflow);
        }
        let tail = self.read(root + 8);
        self.write(root + 0x14, end);
        self.write(tail, record);
        self.write(root + 8, record);
        Ok(self.payload(record + 8, end))
    }

    /// `FUN_0045B220`: append a keyed record for `slot` and return its
    /// payload (see [`Self::push_fifo`]).
    pub fn push_sorted(
        &mut self,
        key: i32,
        slot: FillSlot,
        payload_bytes: usize,
    ) -> Result<&mut [u8], QueueError> {
        let root = ARENA_BASE;
        let record = self.read(root + 0x14);
        let node = record + 4;
        self.write(record, key as u32);
        self.write(record + 8, thunk_address(slot));
        self.write(node, 0);
        let end = record + words(payload_bytes) * 4 + 0xC;
        if end > self.read(root + 0x10) {
            return Err(QueueError::PrimitiveOverflow);
        }
        let tail = self.read(root + 8);
        self.write(root + 0x14, end);
        self.write(tail, node);
        self.write(root + 8, node);
        Ok(self.payload(record + 0xC, end))
    }

    /// `FUN_0045B280`: a keyed record in a sorted scope, a FIFO record (key
    /// dropped) otherwise.
    pub fn push(
        &mut self,
        key: i32,
        slot: FillSlot,
        payload_bytes: usize,
    ) -> Result<&mut [u8], QueueError> {
        if self.sorted_scope() {
            self.push_sorted(key, slot, payload_bytes)
        } else {
            self.push_fifo(slot, payload_bytes)
        }
    }

    /// `FUN_00494AB0`: open a group whose records are sorted when drained.
    /// `key` orders the group record itself in a sorted enclosing scope.
    pub fn begin_sorted_group(&mut self, key: i32) -> Result<(), QueueError> {
        self.begin_group(key, SORT_AND_DISPATCH, 1)
    }

    /// `FUN_00494B60`: open a group drained in submission order.
    pub fn begin_fifo_group(&mut self, key: i32) -> Result<(), QueueError> {
        self.begin_group(key, DISPATCH, 0)
    }

    fn begin_group(&mut self, key: i32, callback: u32, mode: u32) -> Result<(), QueueError> {
        let root = ARENA_BASE;
        let record = self.read(root + 0x14);
        let header = if self.sorted_scope() {
            let node = record + 4;
            self.write(node, 0);
            self.write(record, key as u32);
            self.write(record + 8, callback);
            let end = record + 0x1C;
            if end > self.read(root + 0x10) {
                return Err(QueueError::GroupOverflow);
            }
            self.write(root + 0x14, end);
            let tail = self.read(root + 8);
            self.write(tail, node);
            self.write(root + 8, node);
            record + 0xC
        } else {
            self.write(record, 0);
            self.write(record + 4, callback);
            let end = record + 0x18;
            if end > self.read(root + 0x10) {
                return Err(QueueError::GroupOverflow);
            }
            self.write(root + 0x14, end);
            let tail = self.read(root + 8);
            self.write(tail, record);
            self.write(root + 8, record);
            record + 8
        };
        // The child header saves the enclosing scope.
        self.write(header + 4, 0);
        self.write(header, self.read(root));
        self.write(header + 8, self.read(root + 8));
        self.write(header + 0xC, self.read(root + 0xC));
        self.write(root + 8, header + 4);
        self.write(root, mode);
        self.write(root + 0xC, header);
        Ok(())
    }

    /// `FUN_00494A80`: close the innermost open group.
    pub fn end_group(&mut self) -> Result<(), QueueError> {
        let root = ARENA_BASE;
        let scope = self.read(root + 0xC);
        if scope == root {
            return Err(QueueError::EndAtRoot);
        }
        self.write(root, self.read(scope));
        self.write(root + 8, self.read(scope + 8));
        self.write(root + 0xC, self.read(scope + 0xC));
        Ok(())
    }

    /// `FUN_004948F0`: close every open group, then sort and drain the root
    /// (`FUN_00494930`). `fill` receives each primitive's slot and the arena
    /// from its payload onward (handlers read only their own packet) and
    /// returns the handler's result; the first non-zero result stops the
    /// drain and is returned. Records are not consumed: draining again
    /// replays the same order.
    pub fn flush(&mut self, fill: &mut dyn FnMut(FillSlot, &[u8]) -> u32) -> u32 {
        while self.group_open() {
            // Cannot fail while a group is open.
            self.end_group().expect("open group");
        }
        self.sort_and_dispatch(ARENA_BASE, fill)
    }

    /// `FUN_00494930`: sort the list whose head field is at `header + 4`,
    /// then drain it.
    fn sort_and_dispatch(
        &mut self,
        header: u32,
        fill: &mut dyn FnMut(FillSlot, &[u8]) -> u32,
    ) -> u32 {
        let mut nodes = Vec::new();
        let mut node = self.read(header + 4);
        while node != 0 {
            nodes.push(node);
            node = self.read(node);
        }
        // The retail bottom-up merge sort compares keys at `node - 4`, then
        // node addresses; that is a total order, so any sort agrees with it.
        nodes.sort_by(|&a, &b| {
            let (ka, kb) = (self.read(a - 4) as i32, self.read(b - 4) as i32);
            kb.cmp(&ka).then(a.cmp(&b))
        });
        let mut link = header + 4;
        for &node in &nodes {
            self.write(link, node);
            link = node;
        }
        if !nodes.is_empty() {
            self.write(link, 0);
        }
        self.dispatch(header, fill)
    }

    /// `FUN_00494A50`: call every record's callback in list order.
    fn dispatch(&mut self, header: u32, fill: &mut dyn FnMut(FillSlot, &[u8]) -> u32) -> u32 {
        let mut node = self.read(header + 4);
        while node != 0 {
            let payload = node + 8;
            let result = match callback_kind(self.read(node + 4)) {
                QueueCallback::Fill(slot) => {
                    fill(slot, &self.arena[(payload - ARENA_BASE) as usize..])
                }
                QueueCallback::SortedGroup => self.sort_and_dispatch(payload, fill),
                QueueCallback::FifoGroup => self.dispatch(payload, fill),
                QueueCallback::Other(word) => {
                    panic!("queue record at {node:08X} calls {word:08X}, which no writer stores")
                }
            };
            if result != 0 {
                return result;
            }
            node = self.read(node);
        }
        0
    }

    fn payload(&mut self, start: u32, end: u32) -> &mut [u8] {
        &mut self.arena[(start - ARENA_BASE) as usize..(end - ARENA_BASE) as usize]
    }

    fn read(&self, address: u32) -> u32 {
        let at = (address - ARENA_BASE) as usize;
        u32::from_le_bytes(self.arena[at..at + 4].try_into().unwrap())
    }

    fn write(&mut self, address: u32, value: u32) {
        let at = (address - ARENA_BASE) as usize;
        self.arena[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
}

/// Payload dwords: `(bytes + 3) >> 2`.
fn words(bytes: usize) -> u32 {
    ((bytes + 3) >> 2) as u32
}

#[cfg(test)]
mod tests;
