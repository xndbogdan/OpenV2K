//! Section 4 menu sprite-font decoder.
//!
//! 64-byte records with trailing glyph metrics (count × 8B) and global
//! sprite-id tables (count × 4B). The public type names are retained for
//! compatibility with the existing menu code, but these are not gameplay
//! parameters or entity stats.
//!
//! Present in level 2 only (4 OVLs, 2 records each).
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::{read_u16, read_u32};
use v2k_core::{Result, V2kError};

/// Size of one parameter record header.
const RECORD_SIZE: usize = 0x40; // 64 bytes

/// One glyph's signed metrics, stored as four raw u16 values.
///
/// `a`/`b` are pen advance and kerning in the font record's fixed-point
/// units (normally hundredths of a pixel). `c`/`d` are already whole-pixel
/// X/Y blit offsets; retail adds them only after dividing the pen position.
#[derive(Debug, Clone)]
pub struct ParamTuple {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub d: u16,
}

/// A parameter lookup table record from Section 4.
#[derive(Debug, Clone)]
pub struct ParamRecord {
    pub runtime_id: u32,
    /// Always 1.
    pub record_type: u32,
    /// Number of entries (128 or 256).
    pub count: u32,
    pub param_a: u32,
    pub param_b: u32,
    pub param_c: u32,
    pub param_d: u32,
    pub param_e: u32,
    pub param_f: u32,
    pub zero: u32,
    pub max_index: u32,
    pub unk_2c: u32,
    pub width: u32,
    pub height: u32,
    /// Offset to parameter tuples (relative to section start).
    pub palette_ptr: u32,
    /// Offset to index table (relative to section start).
    pub pixel_ptr: u32,
    /// Parsed parameter tuples.
    pub params: Vec<ParamTuple>,
    /// Parsed index table values.
    pub indices: Vec<u32>,
}

/// Result of parsing Section 4.
#[derive(Debug)]
pub struct ParamTable {
    pub records: Vec<ParamRecord>,
}

/// Parse Section 4 parameter lookup tables.
pub fn parse_params(data: &[u8]) -> Result<ParamTable> {
    if data.len() < RECORD_SIZE {
        return Err(V2kError::section(4, "section 4 data too short"));
    }

    // Infer record count from first record's palette_ptr
    let first_pal_ptr = read_u32(data, 0x38)? as usize;
    if first_pal_ptr == 0 || first_pal_ptr % RECORD_SIZE != 0 {
        return Err(V2kError::section(
            4,
            format!("invalid palette_ptr: 0x{:X}", first_pal_ptr),
        ));
    }

    let n_records = first_pal_ptr / RECORD_SIZE;
    if n_records == 0 || n_records * RECORD_SIZE > data.len() {
        return Err(V2kError::section(4, "record count exceeds section size"));
    }

    let mut records = Vec::with_capacity(n_records);
    for i in 0..n_records {
        let off = i * RECORD_SIZE;
        if off + RECORD_SIZE > data.len() {
            break;
        }

        let runtime_id = read_u32(data, off)?;
        let record_type = read_u32(data, off + 0x04)?;
        let count = read_u32(data, off + 0x08)?;
        let param_a = read_u32(data, off + 0x0C)?;
        let param_b = read_u32(data, off + 0x10)?;
        let param_c = read_u32(data, off + 0x14)?;
        let param_d = read_u32(data, off + 0x18)?;
        let param_e = read_u32(data, off + 0x1C)?;
        let param_f = read_u32(data, off + 0x20)?;
        let zero = read_u32(data, off + 0x24)?;
        let max_index = read_u32(data, off + 0x28)?;
        let unk_2c = read_u32(data, off + 0x2C)?;
        let width = read_u32(data, off + 0x30)?;
        let height = read_u32(data, off + 0x34)?;
        let palette_ptr = read_u32(data, off + 0x38)?;
        let pixel_ptr = read_u32(data, off + 0x3C)?;

        let count_usize = count as usize;

        // Parse parameter tuples: count × 8 bytes at palette_ptr
        let params_off = palette_ptr as usize;
        let mut params = Vec::with_capacity(count_usize);
        for j in 0..count_usize {
            let poff = params_off + j * 8;
            if poff + 8 > data.len() {
                break;
            }
            params.push(ParamTuple {
                a: read_u16(data, poff)?,
                b: read_u16(data, poff + 2)?,
                c: read_u16(data, poff + 4)?,
                d: read_u16(data, poff + 6)?,
            });
        }

        // Parse index table: count × 4 bytes at pixel_ptr
        let idx_off = pixel_ptr as usize;
        let mut indices = Vec::with_capacity(count_usize);
        for j in 0..count_usize {
            let ioff = idx_off + j * 4;
            if ioff + 4 > data.len() {
                break;
            }
            indices.push(read_u32(data, ioff)?);
        }

        records.push(ParamRecord {
            runtime_id,
            record_type,
            count,
            param_a,
            param_b,
            param_c,
            param_d,
            param_e,
            param_f,
            zero,
            max_index,
            unk_2c,
            width,
            height,
            palette_ptr,
            pixel_ptr,
            params,
            indices,
        });
    }

    Ok(ParamTable { records })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal() {
        // Build a minimal section with 1 record, count=1
        let mut data = vec![0u8; RECORD_SIZE + 8 + 4]; // header + 1 param tuple + 1 index
                                                       // count = 1
        data[0x08] = 1;
        // palette_ptr = 0x40 (= RECORD_SIZE, 1 record)
        data[0x38] = 0x40;
        // pixel_ptr = 0x48 (after param tuple)
        data[0x3C] = 0x48;
        // param tuple at 0x40: all zeros
        // index at 0x48: zero

        let table = parse_params(&data).unwrap();
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].count, 1);
        assert_eq!(table.records[0].params.len(), 1);
        assert_eq!(table.records[0].indices.len(), 1);
    }

    #[test]
    fn bad_palette_ptr() {
        let mut data = vec![0u8; RECORD_SIZE];
        // palette_ptr = 0 (invalid)
        assert!(parse_params(&data).is_err());

        // palette_ptr not aligned to RECORD_SIZE
        data[0x38] = 0x30;
        assert!(parse_params(&data).is_err());
    }
}
