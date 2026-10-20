//! Network handles ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Session tick, handles and queues").
//!
//! Every body has a handle: a 16-bit generation in the low word, its entry
//! in the high word's low ten bits, and the owning slot above them. Each
//! slot has a block of entries whose free list hands out the highest index
//! first; a peer's bodies are registered under the handles the peer chose.

/// One slot's block (0x18 bytes at `004DC9F4`).
#[derive(Clone, Debug)]
struct Block {
    /// `+8`: the free list's head, `+C` its tail.
    head: usize,
    tail: usize,
    /// `+14`: handle, body (0 free) and next free entry, 12 bytes each.
    entries: Vec<Entry>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Entry {
    handle: u32,
    body: Option<u32>,
    next: usize,
}

/// `FUN_0043A0C0` doubles a block up to this many entries.
pub const MAX_ENTRIES: usize = 0x400;

pub const fn slot_of(handle: u32) -> u8 {
    (handle >> 26) as u8
}

const fn index_of(handle: u32) -> usize {
    ((handle >> 16) & 0x3FF) as usize
}

#[derive(Clone, Debug)]
pub struct HandleTable {
    blocks: Vec<Block>,
}

impl HandleTable {
    /// `FUN_0043A6E0(blocks, entries)`: block 0 stays unused. A network
    /// session opens nine blocks of 256 (`FUN_00438880`), single play two.
    pub fn new(blocks: usize, entries: usize) -> Self {
        let mut table = Self {
            blocks: vec![Block::empty(); blocks],
        };
        for slot in 1..blocks {
            table.blocks[slot] = Block::fresh(slot as u8, entries);
        }
        table
    }

    /// `FUN_0043A290`: the next free entry of the slot's block, its
    /// generation advanced; a block without free entries grows first.
    pub fn allocate(&mut self, slot: u8, body: u32) -> Option<u32> {
        let size = self.blocks.get(usize::from(slot))?.entries.len();
        if slot == 0 {
            return None;
        }
        let head = self.blocks[usize::from(slot)].head;
        if head < 1 || head >= size {
            self.grow(slot, size + 1)?;
        }
        let block = &mut self.blocks[usize::from(slot)];
        let index = block.head;
        if index < 1 || index >= block.entries.len() {
            return None;
        }
        let entry = &mut block.entries[index];
        block.head = entry.next;
        entry.body = Some(body);
        let generation = (entry.handle as u16).wrapping_add(1);
        entry.handle = u32::from(generation) | (u32::from(slot) << 26) | ((index as u32) << 16);
        Some(entry.handle)
    }

    /// `FUN_0043A3C0`: a peer's body under the handle it chose, its entry
    /// taken out of the free list.
    pub fn register(&mut self, handle: u32, body: u32) -> bool {
        let slot = slot_of(handle);
        let index = index_of(handle);
        if slot == 0 || usize::from(slot) >= self.blocks.len() || index == 0 {
            return false;
        }
        if index >= self.blocks[usize::from(slot)].entries.len()
            && self.grow(slot, index + 1).is_none()
        {
            return false;
        }
        if self.lookup(handle).is_some() {
            return false;
        }
        let block = &mut self.blocks[usize::from(slot)];
        if index >= block.entries.len() {
            return false;
        }
        let mut previous = 0;
        if block.head == index {
            block.head = block.entries[index].next;
        } else {
            let mut at = block.head;
            while at != 0 && at != index {
                previous = at;
                at = block.entries[at].next;
            }
            if at == index && previous != 0 {
                block.entries[previous].next = block.entries[index].next;
            }
        }
        if block.tail == index {
            block.tail = previous;
        }
        block.entries[index].body = Some(body);
        block.entries[index].handle = handle;
        true
    }

    /// `FUN_0043A580`.
    pub fn lookup(&self, handle: u32) -> Option<u32> {
        let block = self.blocks.get(usize::from(slot_of(handle)))?;
        let index = index_of(handle);
        if slot_of(handle) == 0 || index == 0 {
            return None;
        }
        let entry = block.entries.get(index)?;
        (entry.handle == handle).then_some(entry.body).flatten()
    }

    /// `FUN_0043A600`: the entry returns to the tail of its free list.
    pub fn free(&mut self, handle: u32) {
        if self.lookup(handle).is_none() {
            return;
        }
        let index = index_of(handle);
        let block = &mut self.blocks[usize::from(slot_of(handle))];
        block.entries[index].body = None;
        block.entries[index].next = 0;
        if block.tail != 0 {
            block.entries[block.tail].next = index;
        }
        block.tail = index;
        if block.head == 0 {
            block.head = index;
        }
    }

    /// Every live handle in a slot's block, by entry (`FUN_00438CB0` and
    /// `FUN_00439010` walk entries 1.. in order).
    pub fn slot_handles(&self, slot: u8) -> Vec<(u32, u32)> {
        self.blocks
            .get(usize::from(slot))
            .map(|block| {
                block
                    .entries
                    .iter()
                    .skip(1)
                    .filter_map(|entry| entry.body.map(|body| (entry.handle, body)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `FUN_0043A0C0`: the block doubles until it holds `wanted` entries
    /// (at most 0x400); live entries keep their handles.
    fn grow(&mut self, slot: u8, wanted: usize) -> Option<()> {
        let old = self.blocks[usize::from(slot)].clone();
        let mut size = old.entries.len().max(1);
        while size < wanted {
            size *= 2;
        }
        let size = size.min(MAX_ENTRIES);
        if size <= old.entries.len() {
            return (wanted <= old.entries.len()).then_some(());
        }
        self.blocks[usize::from(slot)] = Block::fresh(slot, size);
        for (index, entry) in old.entries.iter().enumerate() {
            match entry.body {
                Some(body) => {
                    self.register(entry.handle, body);
                }
                None => self.blocks[usize::from(slot)].entries[index].handle = entry.handle,
            }
        }
        Some(())
    }
}

impl Block {
    fn empty() -> Self {
        Self {
            head: 0,
            tail: 0,
            entries: Vec::new(),
        }
    }

    /// `FUN_0043A1D0`: entry `i` names slot and index with generation 0 and
    /// links to `i - 1`; the list runs from the last entry down.
    fn fresh(slot: u8, size: usize) -> Self {
        let entries = (0..size)
            .map(|index| Entry {
                handle: (u32::from(slot) << 26) | ((index as u32) << 16),
                body: None,
                next: index.saturating_sub(1),
            })
            .collect();
        Self {
            head: size - 1,
            tail: 1,
            entries,
        }
    }
}

#[cfg(test)]
#[path = "handles_tests.rs"]
mod tests;
