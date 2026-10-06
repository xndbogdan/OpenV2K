//! Section 14 sprite-dependency list / spatial-grid decoder.
//!
//! N × 28-byte records (7 × u32). Three fields at +0x10, +0x14, +0x18 are
//! relative pointers into trailing data that gets relocated at load time.
//!
//! Record count comes from PRELOAD.DAT. Present in 39 OVLs.
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::read_u32;
use v2k_core::{Result, V2kError};

/// Size of one linkage record.
pub(crate) const RECORD_SIZE: usize = 28;

/// A single Section-14 region descriptor.
#[derive(Debug, Clone)]
pub struct LinkageRecord {
    /// 1 = global sprite dependency list, 2 = spatial grid.
    pub record_type: u32,
    /// Number of layers (normally 1; 8 for layered grids).
    pub layer_count: u32,
    /// Number of entries in region 3.
    pub entry_count: u32,
    /// Size of one region-3 entry (4 for sprite dependency lists).
    pub entry_size: u32,
    /// Relative offset to data region 1 (relocated at load).
    pub region1_ptr: u32,
    /// Relative offset to data region 2 (relocated at load).
    pub region2_ptr: u32,
    /// Relative offset to data region 3 (relocated at load).
    pub region3_ptr: u32,
}

/// Result of parsing Section 14 linkage data.
#[derive(Debug)]
pub struct LinkageTable {
    pub records: Vec<LinkageRecord>,
    /// Raw section data (owned) for accessing trailing data via pointers.
    pub data: Vec<u8>,
}

/// Per-level global sprite dependency list from a type-1 Section-14 record.
#[derive(Debug, Clone)]
pub struct SpriteDependencies {
    pub sprite_ids: Vec<u32>,
}

impl LinkageTable {
    /// Get trailing data bytes after all records.
    pub fn trailing_data(&self) -> &[u8] {
        let record_end = self.records.len() * RECORD_SIZE;
        if record_end < self.data.len() {
            &self.data[record_end..]
        } else {
            &[]
        }
    }

    /// Extract the type-1 record's global Section-3 sprite ids.
    pub fn sprite_dependencies(&self) -> Option<SpriteDependencies> {
        let record = self.records.iter().find(|r| r.record_type == 1)?;
        if record.entry_size != 4 || record.entry_count > 10_000 {
            return None;
        }
        let start = record.region3_ptr as usize;
        let count = record.entry_count as usize;
        let end = start.checked_add(count.checked_mul(4)?)?;
        if end > self.data.len() {
            return None;
        }
        let mut sprite_ids = Vec::with_capacity(count);
        for i in 0..count {
            sprite_ids.push(read_u32(&self.data, start + i * 4).ok()?);
        }
        Some(SpriteDependencies { sprite_ids })
    }
}

/// Parse Section 14 linkage records.
///
/// `count` is the number of 28-byte records (from PRELOAD.DAT or inferred).
/// If `count` is 0, infers from data size.
pub fn parse_linkage(data: &[u8], count: usize) -> Result<LinkageTable> {
    if data.len() < RECORD_SIZE {
        return Err(V2kError::section(14, "section 14 data too short"));
    }

    let effective_count = if count > 0 {
        count
    } else {
        // Region pointers are relative to the section start, so the first
        // pointed-to byte is also the end of the descriptor array. This is
        // the loader's real boundary; treating all trailing region data as
        // more 28-byte records fabricated hundreds of bogus descriptors.
        let first_region = [16usize, 20, 24]
            .into_iter()
            .filter_map(|off| read_u32(data, off).ok())
            .filter(|&ptr| ptr != 0 && ptr as usize <= data.len())
            .min()
            .map(|ptr| ptr as usize);
        first_region
            .filter(|end| end % RECORD_SIZE == 0)
            .map(|end| end / RECORD_SIZE)
            .filter(|&n| n > 0)
            .unwrap_or(data.len() / RECORD_SIZE)
    };

    let mut records = Vec::with_capacity(effective_count);
    for i in 0..effective_count {
        let off = i * RECORD_SIZE;
        if off + RECORD_SIZE > data.len() {
            break;
        }
        records.push(LinkageRecord {
            record_type: read_u32(data, off)?,
            layer_count: read_u32(data, off + 4)?,
            entry_count: read_u32(data, off + 8)?,
            entry_size: read_u32(data, off + 12)?,
            region1_ptr: read_u32(data, off + 16)?,
            region2_ptr: read_u32(data, off + 20)?,
            region3_ptr: read_u32(data, off + 24)?,
        });
    }

    Ok(LinkageTable {
        records,
        data: data.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_record() {
        let mut data = vec![0u8; 28 + 16]; // 1 record + 16 bytes trailing
                                           // field_0 = 1
        data[0] = 1;
        // region1_ptr = 28 (pointing to trailing data start)
        data[16] = 28;
        let table = parse_linkage(&data, 1).unwrap();
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].record_type, 1);
        assert_eq!(table.records[0].region1_ptr, 28);
        assert_eq!(table.trailing_data().len(), 16);
    }

    #[test]
    fn too_short() {
        assert!(parse_linkage(&[0u8; 10], 1).is_err());
    }

    #[test]
    fn infer_count() {
        let data = vec![0u8; 56]; // exactly 2 records
        let table = parse_linkage(&data, 0).unwrap();
        assert_eq!(table.records.len(), 2);
    }

    #[test]
    fn sprite_dependencies_single_record() {
        // One descriptor followed by its three u32 sprite ids.
        let mut data = vec![0u8; 28 + 12];
        data[0] = 1; // record_type
        data[4] = 1; // layer_count
        data[8] = 3; // field_2 = 3
        data[12] = 4; // field_3 = 4
        data[16..20].copy_from_slice(&28u32.to_le_bytes());
        data[20..24].copy_from_slice(&28u32.to_le_bytes());
        data[24..28].copy_from_slice(&28u32.to_le_bytes());
        data[28..32].copy_from_slice(&100u32.to_le_bytes());
        data[32..36].copy_from_slice(&200u32.to_le_bytes());
        data[36..40].copy_from_slice(&300u32.to_le_bytes());

        let table = parse_linkage(&data, 0).unwrap();
        assert_eq!(table.records.len(), 1);
        let deps = table.sprite_dependencies().unwrap();
        assert_eq!(deps.sprite_ids, vec![100, 200, 300]);
    }

    #[test]
    fn sprite_dependencies_none_when_no_region() {
        let data = vec![0u8; 28]; // exactly 1 record, no trailing
        let table = parse_linkage(&data, 1).unwrap();
        assert!(table.sprite_dependencies().is_none());
    }
}
