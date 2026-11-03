//! Terrain between machines: kind 3's runs, kind 4's cells and kind 5's
//! requests ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Terrain and cells").

use std::cell::Cell;

use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

use super::framing::Record;
use crate::resource_cache::NetworkCellChange;

/// `FUN_00437730` stops once its record reaches 0x60 bytes.
pub const TERRAIN_RECORD_LIMIT: usize = 0x60;
/// `FUN_00437730` stops once its runs cover 4,096 cells.
pub const TERRAIN_RECORD_CELLS: u32 = 0x1000;

fn cell_at(terrain: &TerrainGrid, x: u32, z: u32) -> (u8, u8) {
    let cell = terrain.cells[x as usize * GRID_SIZE + z as usize];
    (cell.terrain_type, cell.attribute)
}

/// The next cell in `FUN_004377E0`'s and `FUN_004379C0`'s order: X, then Z,
/// each wrapping at 256.
pub fn advance(cursor: &mut [u32; 2]) {
    cursor[0] += 1;
    if cursor[0] == 0x100 {
        cursor[0] = 0;
        cursor[1] += 1;
        if cursor[1] == 0x100 {
            cursor[1] = 0;
        }
    }
}

/// `FUN_004377E0`: the run of at most 255 cells from `cursor`, which it
/// leaves on the first cell after the run, and the run's flags (1 infected,
/// 2 flag `0x08`, 4 attribute byte clear; 4 only with 2).
///
/// A later cell with neither an attribute byte nor flag `0x08` joins on its
/// infection bit alone. Any other must match flag `0x08` and then compares
/// "attribute byte set" (4 when set) against the first cell's "attribute
/// byte clear" (4 when clear): retail's inverted test, kept as is.
pub fn measure_run(terrain: &TerrainGrid, cursor: &mut [u32; 2]) -> (u8, u8) {
    let (first, attribute) = cell_at(terrain, cursor[0], cursor[1]);
    let infected = first & 0x10;
    let mut flags = u8::from(infected != 0);
    let mut burned = 0;
    let mut clear = 0;
    if first & 0x08 != 0 {
        burned = 0x08;
        flags |= 2;
        if attribute == 0 {
            clear = 4;
            flags |= 4;
        }
    }
    let mut count: u8 = 0;
    loop {
        count += 1;
        advance(cursor);
        let (next, attribute) = cell_at(terrain, cursor[0], cursor[1]);
        if next & 0x10 != infected {
            break;
        }
        if (attribute != 0 || next & 0x08 != 0)
            && (next & 0x08 != burned || (if attribute != 0 { 4 } else { 0 }) != clear)
        {
            break;
        }
        if count == 0xFF {
            break;
        }
    }
    (count, flags)
}

/// Kind 3 as `FUN_00437730` packs it into the room the message has left:
/// its size word, the sea level's high word as a dword, the start cell, then
/// one (count, flags) pair per run from the host's cursor `004DC9E0`, which
/// it advances.
pub struct TerrainRuns<'a> {
    pub terrain: &'a TerrainGrid,
    pub cursor: Cell<[u32; 2]>,
}

impl Record for TerrainRuns<'_> {
    fn kind(&self) -> u8 {
        3
    }

    fn size_word(&self) -> usize {
        16
    }

    fn pack(&self, room: &mut [u8]) -> Option<usize> {
        let mut cursor = self.cursor.get();
        let mut out = [0u8; TERRAIN_RECORD_LIMIT];
        let sea = i32::from(self.terrain.sea_level_raw());
        out[2..6].copy_from_slice(&sea.to_le_bytes());
        out[6] = cursor[0] as u8;
        out[7] = cursor[1] as u8;
        let mut size = 8;
        let mut left = room.len() as i64 - 8;
        let mut cells = 0u32;
        loop {
            if left < 2 || size >= TERRAIN_RECORD_LIMIT {
                break;
            }
            let (count, flags) = measure_run(self.terrain, &mut cursor);
            out[size] = count;
            out[size + 1] = flags;
            cells += u32::from(count);
            size += 2;
            left -= 2;
            if cells >= TERRAIN_RECORD_CELLS {
                break;
            }
        }
        out[..2].copy_from_slice(&(size as u16).to_le_bytes());
        // The record packs into the message; with less than 8 bytes of room
        // (never, after FUN_00469200's 16-byte check) retail would overrun.
        room.get_mut(..size)?.copy_from_slice(&out[..size]);
        self.cursor.set(cursor);
        Some(size)
    }
}

/// A received kind 3 (`FUN_00437910`): the sea level's high word, the start
/// cell and the runs. A record shorter than its header has no runs.
pub fn parse_runs(record: &[u8]) -> Option<(i32, [u32; 2], Vec<(u8, u8)>)> {
    let size = usize::from(u16::from_le_bytes([*record.first()?, *record.get(1)?]));
    let header = record.get(..8)?;
    let sea = i32::from_le_bytes([header[2], header[3], header[4], header[5]]);
    let start = [u32::from(header[6]), u32::from(header[7])];
    let runs = record
        .get(8..size.max(8))?
        .chunks_exact(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();
    Some((sea, start, runs))
}

/// The record `FUN_00438F60` sends for one notification: the host's kind 4
/// (`FUN_00437B00`: the cell, its flags and attribute byte) or a client's
/// kind 5 (the cell, an operation: 0/1 infect/clean, 2/3 burn/unburn, 4 the
/// attribute byte, which `FUN_00437C50` fills in; the other operations leave
/// the position word's top byte, which their apply never reads, sent as 0).
pub fn cell_change_record(change: NetworkCellChange, host: bool) -> super::protocol::GameRecord {
    use super::protocol::GameRecord;
    let [x, z] = change.cell;
    if host {
        return GameRecord::Cell([x, z, change.flags, change.attribute]);
    }
    let (operation, last) = match change.list {
        0 => (u8::from(change.value == 0), 0),
        1 => (if change.value != 0 { 2 } else { 3 }, 0),
        _ => (4, change.attribute),
    };
    GameRecord::CellRequest([x, z, operation, last])
}

#[cfg(test)]
#[path = "terrain_tests.rs"]
mod tests;
