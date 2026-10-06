//! Section 9 static terrain-object descriptor decoder.
//!
//! 256 × 12-byte records. A nonzero Section-10 terrain attribute selects one
//! record; terrain-type bits 3–4 select one of its four global Section-8 model
//! ids. The trailing dword selects a 44-byte executable object-kind record.
//!
//! Present in levels 6-11 (24 OVLs, 3072 bytes each).
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::read_u16;
use v2k_core::{Result, V2kError};

/// Expected record count per Section 9 table.
pub const NUM_RECORDS: usize = 256;

/// Size of one terrain-object descriptor.
const RECORD_SIZE: usize = 12;

/// Expected total section data size.
pub const EXPECTED_SIZE: usize = NUM_RECORDS * RECORD_SIZE; // 3072

/// Classification of the four authored model slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSlotPattern {
    /// All four states use one model: [A, A, A, A].
    Static,
    /// Two models alternate by state bit: [A, B, A, B].
    Paired,
    /// Four distinct states (or another non-static/non-paired layout).
    Varied,
}

/// One static terrain-object descriptor.
#[derive(Debug, Clone)]
pub struct TerrainObjectDescriptor {
    /// Four global Section-8 model ids selected by terrain-type bits 3–4.
    pub model_ids: [u16; 4],
    /// Index into the executable's 44-byte static-object kind table.
    pub kind_index: u32,
    /// Classified model-slot pattern, retained for diagnostics.
    pub pattern: ModelSlotPattern,
}

impl TerrainObjectDescriptor {
    /// Select the live model exactly as `FUN_00427410` and `FUN_0042F650` do.
    pub fn model_id_for(&self, terrain_type: u8) -> u16 {
        self.model_ids[usize::from((terrain_type >> 3) & 3)]
    }
}

/// Parsed Section-9 terrain-object table.
#[derive(Debug)]
pub struct TerrainObjectTable {
    pub records: Vec<TerrainObjectDescriptor>,
}

/// Backwards-compatible names for callers outside the active port. New code
/// should use the terrain-object terminology above.
pub type AnimFrameRecord = TerrainObjectDescriptor;
pub type AnimFrameTable = TerrainObjectTable;
pub type QuadPattern = ModelSlotPattern;

/// Classify the four model slots.
fn classify_quad(quad: &[u16; 4]) -> ModelSlotPattern {
    let [a, b, c, d] = *quad;
    if a == b && b == c && c == d {
        ModelSlotPattern::Static
    } else if a == c && b == d {
        ModelSlotPattern::Paired
    } else {
        ModelSlotPattern::Varied
    }
}

/// Parse the Section-9 terrain-object descriptor table.
/// Expects exactly 3072 bytes (256 × 12-byte records).
pub fn parse_terrain_objects(data: &[u8]) -> Result<TerrainObjectTable> {
    if data.len() < EXPECTED_SIZE {
        return Err(V2kError::section(
            9,
            format!(
                "section 9 too small: {} bytes, need {}",
                data.len(),
                EXPECTED_SIZE
            ),
        ));
    }

    let mut records = Vec::with_capacity(NUM_RECORDS);
    for i in 0..NUM_RECORDS {
        let off = i * RECORD_SIZE;
        let model_ids = [
            read_u16(data, off)?,
            read_u16(data, off + 2)?,
            read_u16(data, off + 4)?,
            read_u16(data, off + 6)?,
        ];
        let kind_index = u32::from_le_bytes(data[off + 8..off + 12].try_into().unwrap());
        let pattern = classify_quad(&model_ids);

        records.push(TerrainObjectDescriptor {
            model_ids,
            kind_index,
            pattern,
        });
    }

    Ok(TerrainObjectTable { records })
}

/// Legacy parser name retained for source compatibility.
pub fn parse_anim_frames(data: &[u8]) -> Result<AnimFrameTable> {
    parse_terrain_objects(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_static() {
        assert_eq!(classify_quad(&[5, 5, 5, 5]), ModelSlotPattern::Static);
    }

    #[test]
    fn classify_paired() {
        assert_eq!(classify_quad(&[1, 2, 1, 2]), ModelSlotPattern::Paired);
    }

    #[test]
    fn classify_varied() {
        assert_eq!(classify_quad(&[1, 2, 3, 4]), ModelSlotPattern::Varied);
    }

    #[test]
    fn parse_minimal() {
        // Build 256 records of zeros
        let data = vec![0u8; EXPECTED_SIZE];
        let table = parse_terrain_objects(&data).unwrap();
        assert_eq!(table.records.len(), NUM_RECORDS);
        assert_eq!(table.records[0].pattern, ModelSlotPattern::Static);
        assert_eq!(table.records[0].model_ids, [0, 0, 0, 0]);
        assert_eq!(table.records[0].kind_index, 0);
    }

    #[test]
    fn selects_model_from_terrain_state_bits() {
        let descriptor = TerrainObjectDescriptor {
            model_ids: [10, 11, 12, 13],
            kind_index: 7,
            pattern: ModelSlotPattern::Varied,
        };
        assert_eq!(descriptor.model_id_for(0x00), 10);
        assert_eq!(descriptor.model_id_for(0x08), 11);
        assert_eq!(descriptor.model_id_for(0x10), 12);
        assert_eq!(descriptor.model_id_for(0x18), 13);
        assert_eq!(descriptor.model_id_for(0xF8), 13);
    }

    #[test]
    fn too_small() {
        let data = vec![0u8; 100];
        assert!(parse_terrain_objects(&data).is_err());
    }
}
