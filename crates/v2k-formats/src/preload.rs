//! PRELOAD.DAT parser.
//!
//! File layout:
//!   - Bytes 0..3180: Resource count matrix (15 sections x 53 levels x uint32)
//!   - Bytes 3180..: Embedded OVL files (7 complete OVLs, each with 15 "abcd" sections)
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::{self, OvlFile};
use v2k_core::{Result, V2kError};

pub const NUM_SECTIONS: usize = 15;
pub const NUM_LEVELS: usize = 53;
/// Matrix size: 15 * 53 * 4 = 3180 bytes.
const MATRIX_SIZE: usize = NUM_SECTIONS * NUM_LEVELS * 4;

/// Parsed PRELOAD.DAT file.
#[derive(Debug)]
pub struct PreloadDat {
    /// Resource count matrix: grid[section][level] = count.
    pub grid: Vec<Vec<u32>>,
    /// Embedded OVL files (typically 7).
    pub embedded_ovls: Vec<EmbeddedOvl>,
}

/// An embedded OVL within PRELOAD.DAT.
#[derive(Debug)]
pub struct EmbeddedOvl {
    /// Index within the preload file (0-based).
    pub index: usize,
    /// Byte offset in the preload file where this OVL starts.
    pub file_offset: usize,
    /// Parsed OVL data.
    pub ovl: OvlFile,
}

impl PreloadDat {
    /// Parse a PRELOAD.DAT file from raw bytes.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < MATRIX_SIZE {
            return Err(V2kError::OvlFormat(format!(
                "PRELOAD.DAT too small: {} bytes, need at least {}",
                data.len(),
                MATRIX_SIZE
            )));
        }

        // Parse resource count matrix (section-major order)
        let mut grid = Vec::with_capacity(NUM_SECTIONS);
        for sec in 0..NUM_SECTIONS {
            let mut row = Vec::with_capacity(NUM_LEVELS);
            for lvl in 0..NUM_LEVELS {
                let offset = (sec * NUM_LEVELS + lvl) * 4;
                let val = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                row.push(val);
            }
            grid.push(row);
        }

        // Find embedded OVL files after the matrix
        let markers = find_markers(&data[MATRIX_SIZE..]);
        // Adjust offsets to be relative to data start
        let markers: Vec<usize> = markers.iter().map(|&m| m + MATRIX_SIZE).collect();

        let num_ovls = markers.len() / ovl::NUM_SECTIONS;
        let mut embedded_ovls = Vec::with_capacity(num_ovls);

        for oi in 0..num_ovls {
            let base_marker = oi * ovl::NUM_SECTIONS;
            let ovl_start = markers[base_marker];
            let ovl_end = if oi + 1 < num_ovls {
                markers[(oi + 1) * ovl::NUM_SECTIONS]
            } else {
                data.len()
            };

            let ovl_data = &data[ovl_start..ovl_end];
            match OvlFile::parse(ovl_data) {
                Ok(ovl) => {
                    embedded_ovls.push(EmbeddedOvl {
                        index: oi,
                        file_offset: ovl_start,
                        ovl,
                    });
                }
                Err(_) => {
                    // Skip malformed embedded OVLs
                    continue;
                }
            }
        }

        Ok(PreloadDat {
            grid,
            embedded_ovls,
        })
    }

    /// Get the resource count for a given section and level.
    pub fn count(&self, section: usize, level: usize) -> u32 {
        if section < NUM_SECTIONS && level < NUM_LEVELS {
            self.grid[section][level]
        } else {
            0
        }
    }

    /// Total non-zero entries in the matrix.
    pub fn non_zero_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|&&v| v > 0)
            .count()
    }
}

/// Find "abcd" markers in a byte slice.
fn find_markers(data: &[u8]) -> Vec<usize> {
    let mut markers = Vec::new();
    let mut pos = 0;
    while pos + 4 <= data.len() {
        if &data[pos..pos + 4] == b"abcd" {
            markers.push(pos);
            pos += 4;
        } else {
            pos += 1;
        }
    }
    markers
}
