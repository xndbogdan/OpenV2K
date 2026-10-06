//! Read-only evidence for the retail terrain infection state.
//!
//! `FUN_00433720` mutates bit `0x10` in the third byte of one wrapped
//! Section-10 terrain cell and keeps `DAT_004DC684` in sync.  This module
//! snapshots that complete mutable cell array behind an unchanged retail
//! tick/pointer/counter envelope.  Timelines can then retain compact hashes
//! and bounded exact cell diffs without serializing the 192 KiB grid on every
//! sample.

use serde::Serialize;
use serde_json::json;
use std::io::Write;

use crate::entity::TICK_50HZ;
use crate::process::{plausible_heap_pointer, u32_at, Process};

pub const TERRAIN_POOL_POINTER_ADDRESS: u32 = 0x004F_E648;
pub const CURRENT_TERRAIN_INDEX_ADDRESS: u32 = 0x004F_EC40;
pub const INFECTED_CELL_COUNT_ADDRESS: u32 = 0x004D_C684;
pub const TERRAIN_BIT_08_COUNT_ADDRESS: u32 = 0x004D_C67C;
pub const TERRAIN_CHANGE_CALLBACK_LIST_HEAD_ADDRESS: u32 = 0x004D_C5F8;

pub const TERRAIN_HEADER_BYTES: u32 = 0x14;
pub const TERRAIN_GRID_SIDE: usize = 0x100;
pub const TERRAIN_CELL_BYTES: usize = 3;
pub const TERRAIN_CELL_COUNT: usize = TERRAIN_GRID_SIDE * TERRAIN_GRID_SIDE;
pub const TERRAIN_GRID_BYTES: usize = TERRAIN_CELL_COUNT * TERRAIN_CELL_BYTES;
pub const INFECTION_BIT: u8 = 0x10;
pub const TERRAIN_BIT_08: u8 = 0x08;

pub const DEFAULT_REPORTED_CELL_DIFF_LIMIT: usize = 256;
pub const MAX_REPORTED_CELL_DIFF_LIMIT: usize = 4096;

const MAX_TERRAIN_RESOURCES: u32 = 4096;
const CALLBACK_LIST_HEAD_BYTES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainResolutionEvidence {
    pub terrain_pool_pointer_address: u32,
    pub terrain_pool_pointer: u32,
    pub current_terrain_index_address: u32,
    pub current_terrain_index: u32,
    pub terrain_pointer_slot_address: u32,
    pub terrain_pointer: u32,
    pub terrain_grid_address: u32,
    pub terrain_grid_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainCounterEvidence {
    pub address: u32,
    pub value: u32,
    pub within_cell_count: bool,
    pub derived_percent: Option<u32>,
    pub counted_cells_in_grid: u32,
    pub matches_grid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainChangeCallbackListEvidence {
    pub address: u32,
    pub raw_hex: String,
    pub first_node_pointer: u32,
    pub first_node_pointer_plausible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainInfectionFrameEvidence {
    pub tick_before: u32,
    pub tick_after: u32,
    pub attempts: u8,
    pub observationally_stable: bool,
    pub resolution: TerrainResolutionEvidence,
    pub terrain_fnv1a64: String,
    pub infected_cells: TerrainCounterEvidence,
    pub terrain_bit_08_cells: TerrainCounterEvidence,
    pub change_callback_list: TerrainChangeCallbackListEvidence,
    pub stability_errors: Vec<String>,
    /// Cross-checks which do not make the duplicate-read envelope torn.
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainCellTripleDiff {
    pub cell_xz: [u8; 2],
    pub cell_index: u32,
    pub cell_address: u32,
    pub before: [u8; TERRAIN_CELL_BYTES],
    pub after: [u8; TERRAIN_CELL_BYTES],
    /// Bit zero corresponds to byte zero of the three-byte cell.
    pub changed_byte_mask: u8,
    pub infection_before: bool,
    pub infection_after: bool,
    pub terrain_bit_08_before: bool,
    pub terrain_bit_08_after: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TerrainCellDiffSummary {
    pub changed_cells: usize,
    pub infection_bit_changes: usize,
    pub terrain_bit_08_changes: usize,
    pub reported_limit: usize,
    pub reported_changes: Vec<TerrainCellTripleDiff>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerrainInfectionSample {
    pub frame: TerrainInfectionFrameEvidence,
    /// A new terrain resource starts a new comparison generation.
    pub baseline_generation: u64,
    pub baseline_reset: bool,
    pub baseline_fnv1a64: Option<String>,
    pub previous_fnv1a64: Option<String>,
    pub changes_from_previous: Option<TerrainCellDiffSummary>,
    pub changes_from_baseline: Option<TerrainCellDiffSummary>,
}

#[derive(Debug, Clone)]
struct TerrainFrame {
    evidence: TerrainInfectionFrameEvidence,
    grid: Vec<u8>,
}

/// Stateful comparison boundary for a generic timeline.
///
/// Only observationally stable frames advance `previous` or establish a new
/// baseline.  Rejected samples remain reportable but cannot manufacture a
/// mutation between two torn endpoints.
pub struct TerrainInfectionSampler {
    reported_diff_limit: usize,
    baseline_generation: u64,
    baseline: Option<TerrainFrame>,
    previous: Option<TerrainFrame>,
}

/// JSON-lines adapter used by the combined runtime timelines.
pub struct TerrainInfectionState {
    sampler: TerrainInfectionSampler,
}

impl TerrainInfectionState {
    pub fn new() -> Self {
        Self {
            sampler: TerrainInfectionSampler::with_default_limit(),
        }
    }

    pub fn sample(
        &mut self,
        process: &Process,
        writer: &mut impl Write,
        sample: u64,
        elapsed_ms: f64,
    ) -> Result<(), String> {
        let current = self.sampler.sample(process, 3)?;
        write_json_line(
            writer,
            &json!({
                "record_kind": "terrain_infection_sample",
                "sample": sample,
                "elapsed_ms": elapsed_ms,
                "current": current,
            }),
        )
    }
}

impl Default for TerrainInfectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl TerrainInfectionSampler {
    pub fn new(reported_diff_limit: usize) -> Result<Self, String> {
        if reported_diff_limit > MAX_REPORTED_CELL_DIFF_LIMIT {
            return Err(format!(
                "terrain cell diff limit {reported_diff_limit} exceeds the safety cap {MAX_REPORTED_CELL_DIFF_LIMIT}"
            ));
        }
        Ok(Self {
            reported_diff_limit,
            baseline_generation: 0,
            baseline: None,
            previous: None,
        })
    }

    pub fn with_default_limit() -> Self {
        Self::new(DEFAULT_REPORTED_CELL_DIFF_LIMIT)
            .expect("the built-in terrain diff limit is within its safety cap")
    }

    pub fn sample(
        &mut self,
        process: &Process,
        maximum_attempts: u8,
    ) -> Result<TerrainInfectionSample, String> {
        let frame = read_frame(process, maximum_attempts)?;
        if !frame.evidence.observationally_stable {
            return Ok(TerrainInfectionSample {
                frame: frame.evidence,
                baseline_generation: self.baseline_generation,
                baseline_reset: false,
                baseline_fnv1a64: self
                    .baseline
                    .as_ref()
                    .map(|baseline| baseline.evidence.terrain_fnv1a64.clone()),
                previous_fnv1a64: self
                    .previous
                    .as_ref()
                    .map(|previous| previous.evidence.terrain_fnv1a64.clone()),
                changes_from_previous: None,
                changes_from_baseline: None,
            });
        }

        let resource_changed = self
            .baseline
            .as_ref()
            .is_some_and(|baseline| baseline.evidence.resolution != frame.evidence.resolution);
        if resource_changed {
            self.baseline_generation = self.baseline_generation.wrapping_add(1);
            self.baseline = None;
            self.previous = None;
        }

        let baseline_reset = self.baseline.is_none();
        if baseline_reset {
            self.baseline = Some(frame.clone());
        }
        let changes_from_previous = self.previous.as_ref().map(|previous| {
            diff_cell_triples(
                &previous.grid,
                &frame.grid,
                frame.evidence.resolution.terrain_grid_address,
                self.reported_diff_limit,
            )
        });
        let changes_from_baseline = self.baseline.as_ref().map(|baseline| {
            diff_cell_triples(
                &baseline.grid,
                &frame.grid,
                frame.evidence.resolution.terrain_grid_address,
                self.reported_diff_limit,
            )
        });
        let sample = TerrainInfectionSample {
            frame: frame.evidence.clone(),
            baseline_generation: self.baseline_generation,
            baseline_reset,
            baseline_fnv1a64: self
                .baseline
                .as_ref()
                .map(|baseline| baseline.evidence.terrain_fnv1a64.clone()),
            previous_fnv1a64: self
                .previous
                .as_ref()
                .map(|previous| previous.evidence.terrain_fnv1a64.clone()),
            changes_from_previous,
            changes_from_baseline,
        };
        self.previous = Some(frame);
        Ok(sample)
    }
}

fn read_frame(process: &Process, maximum_attempts: u8) -> Result<TerrainFrame, String> {
    let maximum_attempts = maximum_attempts.max(1);
    let mut last_frame = None;
    let mut last_read_error = None;

    for attempt in 1..=maximum_attempts {
        match read_frame_attempt(process, attempt) {
            Ok(frame) if frame.evidence.observationally_stable => return Ok(frame),
            Ok(frame) => last_frame = Some(frame),
            Err(error) => last_read_error = Some(error),
        }
    }

    if let Some(frame) = last_frame {
        Ok(frame)
    } else {
        Err(last_read_error.unwrap_or_else(|| "terrain infection sample produced no read".into()))
    }
}

fn read_frame_attempt(process: &Process, attempt: u8) -> Result<TerrainFrame, String> {
    let tick_before = process.read_u32(TICK_50HZ)?;
    let resolution_before = read_current_terrain(process)?;
    let infected_before = process.read_u32(INFECTED_CELL_COUNT_ADDRESS as usize)?;
    let bit_08_before = process.read_u32(TERRAIN_BIT_08_COUNT_ADDRESS as usize)?;
    let callback_before = process.read_bytes(
        TERRAIN_CHANGE_CALLBACK_LIST_HEAD_ADDRESS as usize,
        CALLBACK_LIST_HEAD_BYTES,
    )?;
    let first_grid = process.read_bytes(
        resolution_before.terrain_grid_address as usize,
        TERRAIN_GRID_BYTES,
    )?;
    let grid = process.read_bytes(
        resolution_before.terrain_grid_address as usize,
        TERRAIN_GRID_BYTES,
    )?;
    let callback_after = process.read_bytes(
        TERRAIN_CHANGE_CALLBACK_LIST_HEAD_ADDRESS as usize,
        CALLBACK_LIST_HEAD_BYTES,
    )?;
    let infected_after = process.read_u32(INFECTED_CELL_COUNT_ADDRESS as usize)?;
    let bit_08_after = process.read_u32(TERRAIN_BIT_08_COUNT_ADDRESS as usize)?;
    let resolution_after = read_current_terrain(process)?;
    let tick_after = process.read_u32(TICK_50HZ)?;

    let mut stability_errors = Vec::new();
    if tick_before != tick_after {
        stability_errors.push("50-Hz tick changed across terrain reads".into());
    }
    if resolution_before != resolution_after {
        stability_errors.push("current Section-10 terrain resolution changed across reads".into());
    }
    if first_grid != grid {
        stability_errors.push("duplicate full terrain-grid reads differ".into());
    }
    if infected_before != infected_after {
        stability_errors.push("infection counter changed across terrain reads".into());
    }
    if bit_08_before != bit_08_after {
        stability_errors.push("terrain bit-0x08 counter changed across terrain reads".into());
    }
    if callback_before != callback_after {
        stability_errors.push("terrain-change callback-list head changed across reads".into());
    }
    let observationally_stable = stability_errors.is_empty();

    let counted_infected = count_cells_with_bit(&grid, INFECTION_BIT);
    let counted_bit_08 = count_cells_with_bit(&grid, TERRAIN_BIT_08);
    let infected_cells = counter_evidence(
        INFECTED_CELL_COUNT_ADDRESS,
        infected_after,
        counted_infected,
    );
    let terrain_bit_08_cells =
        counter_evidence(TERRAIN_BIT_08_COUNT_ADDRESS, bit_08_after, counted_bit_08);
    let mut validation_errors = Vec::new();
    if observationally_stable && !infected_cells.matches_grid {
        validation_errors.push(format!(
            "infection counter {} does not match {} terrain cells with bit 0x10",
            infected_cells.value, infected_cells.counted_cells_in_grid
        ));
    }
    if observationally_stable && !terrain_bit_08_cells.matches_grid {
        validation_errors.push(format!(
            "terrain bit-0x08 counter {} does not match {} matching cells",
            terrain_bit_08_cells.value, terrain_bit_08_cells.counted_cells_in_grid
        ));
    }

    let first_node_pointer = u32_at(&callback_after, 0);
    Ok(TerrainFrame {
        evidence: TerrainInfectionFrameEvidence {
            tick_before,
            tick_after,
            attempts: attempt,
            observationally_stable,
            resolution: resolution_after,
            terrain_fnv1a64: format!("{:016X}", fnv1a64(&grid)),
            infected_cells,
            terrain_bit_08_cells,
            change_callback_list: TerrainChangeCallbackListEvidence {
                address: TERRAIN_CHANGE_CALLBACK_LIST_HEAD_ADDRESS,
                raw_hex: encode_hex(&callback_after),
                first_node_pointer,
                first_node_pointer_plausible: first_node_pointer != 0
                    && (0x1_0000..=0x7FFF_FFFF).contains(&(first_node_pointer as usize)),
            },
            stability_errors,
            validation_errors,
        },
        grid,
    })
}

fn read_current_terrain(process: &Process) -> Result<TerrainResolutionEvidence, String> {
    let pool = process.read_u32(TERRAIN_POOL_POINTER_ADDRESS as usize)?;
    if !plausible_heap_pointer(pool as usize) {
        return Err(format!(
            "Section-10 terrain pool pointer {pool:08X} is implausible"
        ));
    }
    let index = process.read_u32(CURRENT_TERRAIN_INDEX_ADDRESS as usize)?;
    if index >= MAX_TERRAIN_RESOURCES {
        return Err(format!(
            "current terrain index {index} exceeds passive-read safety limit {MAX_TERRAIN_RESOURCES}"
        ));
    }
    let slot = pool
        .checked_add(
            index
                .checked_mul(4)
                .ok_or("terrain pointer-slot offset overflow")?,
        )
        .ok_or("terrain pointer-slot address overflow")?;
    let terrain = process.read_u32(slot as usize)?;
    if !plausible_heap_pointer(terrain as usize) {
        return Err(format!(
            "current Section-10 terrain pointer {terrain:08X} is implausible"
        ));
    }
    let grid = terrain
        .checked_add(TERRAIN_HEADER_BYTES)
        .ok_or("terrain grid address overflow")?;
    grid.checked_add(TERRAIN_GRID_BYTES as u32)
        .ok_or("terrain grid end address overflow")?;

    Ok(TerrainResolutionEvidence {
        terrain_pool_pointer_address: TERRAIN_POOL_POINTER_ADDRESS,
        terrain_pool_pointer: pool,
        current_terrain_index_address: CURRENT_TERRAIN_INDEX_ADDRESS,
        current_terrain_index: index,
        terrain_pointer_slot_address: slot,
        terrain_pointer: terrain,
        terrain_grid_address: grid,
        terrain_grid_bytes: TERRAIN_GRID_BYTES,
    })
}

fn counter_evidence(address: u32, value: u32, counted_cells: u32) -> TerrainCounterEvidence {
    let within_cell_count = value <= TERRAIN_CELL_COUNT as u32;
    TerrainCounterEvidence {
        address,
        value,
        within_cell_count,
        derived_percent: within_cell_count.then(|| terrain_state_percent(value)),
        counted_cells_in_grid: counted_cells,
        matches_grid: within_cell_count && value == counted_cells,
    }
}

/// Exact `FUN_004366F0`/`FUN_00436720` percentage transform for a valid
/// terrain-cell count.
pub fn terrain_state_percent(cell_count: u32) -> u32 {
    debug_assert!(cell_count <= TERRAIN_CELL_COUNT as u32);
    let denominator = 0x0010_0000u64 + u64::from(cell_count) * 99;
    100 - (0x0640_0000u64 / denominator) as u32
}

/// Retail wraps signed 8.8 X/Z through their unsigned high bytes.
#[cfg(test)]
fn wrapped_cell_xz(position_raw_8_8: [i16; 2]) -> [u8; 2] {
    position_raw_8_8.map(|component| ((component as u16) >> 8) as u8)
}

#[cfg(test)]
fn terrain_cell_index(cell_xz: [u8; 2]) -> usize {
    usize::from(cell_xz[0]) * TERRAIN_GRID_SIDE + usize::from(cell_xz[1])
}

fn cell_xz_from_index(index: usize) -> [u8; 2] {
    [
        (index / TERRAIN_GRID_SIDE) as u8,
        (index % TERRAIN_GRID_SIDE) as u8,
    ]
}

fn count_cells_with_bit(grid: &[u8], bit: u8) -> u32 {
    grid.chunks_exact(TERRAIN_CELL_BYTES)
        .filter(|cell| cell[2] & bit != 0)
        .count() as u32
}

fn diff_cell_triples(
    before: &[u8],
    after: &[u8],
    terrain_grid_address: u32,
    reported_limit: usize,
) -> TerrainCellDiffSummary {
    debug_assert_eq!(before.len(), TERRAIN_GRID_BYTES);
    debug_assert_eq!(after.len(), TERRAIN_GRID_BYTES);
    let mut summary = TerrainCellDiffSummary {
        reported_limit,
        ..TerrainCellDiffSummary::default()
    };

    for (index, (before, after)) in before
        .chunks_exact(TERRAIN_CELL_BYTES)
        .zip(after.chunks_exact(TERRAIN_CELL_BYTES))
        .enumerate()
    {
        if before == after {
            continue;
        }
        summary.changed_cells += 1;
        let infection_before = before[2] & INFECTION_BIT != 0;
        let infection_after = after[2] & INFECTION_BIT != 0;
        let terrain_bit_08_before = before[2] & TERRAIN_BIT_08 != 0;
        let terrain_bit_08_after = after[2] & TERRAIN_BIT_08 != 0;
        summary.infection_bit_changes += usize::from(infection_before != infection_after);
        summary.terrain_bit_08_changes +=
            usize::from(terrain_bit_08_before != terrain_bit_08_after);

        if summary.reported_changes.len() < reported_limit {
            let changed_byte_mask = (0..TERRAIN_CELL_BYTES).fold(0, |mask, byte| {
                mask | (u8::from(before[byte] != after[byte]) << byte)
            });
            summary.reported_changes.push(TerrainCellTripleDiff {
                cell_xz: cell_xz_from_index(index),
                cell_index: index as u32,
                cell_address: terrain_grid_address + (index * TERRAIN_CELL_BYTES) as u32,
                before: [before[0], before[1], before[2]],
                after: [after[0], after[1], after[2]],
                changed_byte_mask,
                infection_before,
                infection_after,
                terrain_bit_08_before,
                terrain_bit_08_after,
            });
        }
    }
    summary.truncated = summary.changed_cells > summary.reported_changes.len();
    summary
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

fn write_json_line(writer: &mut impl Write, value: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value).map_err(|error| error.to_string())?;
    writeln!(writer).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        diff_cell_triples, terrain_cell_index, terrain_state_percent, wrapped_cell_xz,
        INFECTION_BIT, TERRAIN_BIT_08, TERRAIN_CELL_BYTES, TERRAIN_GRID_BYTES,
    };

    #[test]
    fn signed_raw_coordinates_wrap_through_the_unsigned_high_byte() {
        assert_eq!(wrapped_cell_xz([0, 0x1234]), [0, 0x12]);
        assert_eq!(wrapped_cell_xz([-1, i16::MIN]), [0xFF, 0x80]);
        assert_eq!(terrain_cell_index([0xFF, 0xFF]), 0xFFFF);
    }

    #[test]
    fn percentage_transform_matches_the_retail_integer_formula() {
        assert_eq!(terrain_state_percent(0), 0);
        assert_eq!(terrain_state_percent(1), 1);
        assert_eq!(terrain_state_percent(0x1_0000), 87);
    }

    #[test]
    fn diffs_retain_exact_triples_and_cap_only_the_reported_sample() {
        let mut before = vec![0u8; TERRAIN_GRID_BYTES];
        let mut after = before.clone();
        let first = terrain_cell_index([1, 2]) * TERRAIN_CELL_BYTES;
        let second = terrain_cell_index([0xFF, 0]) * TERRAIN_CELL_BYTES;
        before[first..first + 3].copy_from_slice(&[4, 5, TERRAIN_BIT_08]);
        after[first..first + 3].copy_from_slice(&[4, 6, INFECTION_BIT]);
        after[second..second + 3].copy_from_slice(&[7, 8, INFECTION_BIT]);

        let summary = diff_cell_triples(&before, &after, 0x1000_0000, 1);
        assert_eq!(summary.changed_cells, 2);
        assert_eq!(summary.infection_bit_changes, 2);
        assert_eq!(summary.terrain_bit_08_changes, 1);
        assert!(summary.truncated);
        assert_eq!(summary.reported_changes.len(), 1);
        let change = &summary.reported_changes[0];
        assert_eq!(change.cell_xz, [1, 2]);
        assert_eq!(change.cell_index, 0x0102);
        assert_eq!(change.cell_address, 0x1000_0000 + 0x0102 * 3);
        assert_eq!(change.before, [4, 5, TERRAIN_BIT_08]);
        assert_eq!(change.after, [4, 6, INFECTION_BIT]);
        assert_eq!(change.changed_byte_mask, 0b110);
    }
}
