//! Section 11 sound-effect entry decoder.
//!
//! 20-byte entries dispatched by type field:
//! - type=0: unused/padding
//! - type=1: raw PCM sample data
//! - type=5: parametric alias with 16.16 fixed-point frequency and volume
//!   parameters
//!
//! Audio format: 22050 Hz mono 16-bit signed PCM.
//!
//! Use `blob_as_wav()` to encode type=1 blobs as playable WAV files.
//! Owns the section data to avoid lifetime complexity.
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::read_u32;
use v2k_core::{Result, V2kError};

/// Entry size in bytes.
const ENTRY_SIZE: usize = 0x14; // 20 bytes

/// Entry type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    /// Unused/padding entry.
    Unused,
    /// Raw PCM audio data.
    DataBlob,
    /// Parametric alias: references a GLOBAL pool id and applies frequency
    /// variance, frequency multiplication, and volume multiplication.
    Alias,
}

/// A Section 11 entry.
#[derive(Debug, Clone)]
pub struct AnimSoundEntry {
    /// Entry index in the table.
    pub index: usize,
    /// Entry type (0, 1, or 5).
    pub entry_type: EntryType,
    /// Raw type value.
    pub raw_type: u32,
    /// For type=1: resolved data offset within section data.
    /// For type=5: GLOBAL sound-pool target id (lower 16 bits on disk).
    pub offset_or_index: u32,
    /// For type=1: blob size in bytes.
    /// For type=5: frequency variance (16.16 fixed-point).
    pub size_or_scale: u32,
    /// For type=5: frequency multiplier (16.16 fixed-point).
    pub param1: u32,
    /// For type=5: volume multiplier (16.16 fixed-point).
    pub param2: u32,
}

impl AnimSoundEntry {
    /// Type-5 frequency variance as a scalar (16.16 fixed → f64).
    pub fn frequency_variance_f64(&self) -> f64 {
        self.size_or_scale as f64 / 65536.0
    }

    /// Type-5 frequency multiplier as a scalar.
    pub fn frequency_multiplier_f64(&self) -> f64 {
        self.param1 as f64 / 65536.0
    }

    /// Type-5 volume multiplier as a scalar.
    pub fn volume_multiplier_f64(&self) -> f64 {
        self.param2 as f64 / 65536.0
    }
}

/// One slot in a global sound pool assembled by concatenating Section-11
/// tables in retail load order.
#[derive(Debug, Clone, Copy)]
pub struct SoundPoolSlot<'a> {
    pub global_id: usize,
    pub table_index: usize,
    pub local_id: usize,
    pub table: &'a AnimSoundTable,
    pub entry: &'a AnimSoundEntry,
}

/// Exact parameters contributed by one type-5 hop in an alias chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundAliasHop {
    pub global_id: usize,
    pub target_global_id: usize,
    pub frequency_variance_16_16: u32,
    pub frequency_multiplier_16_16: u32,
    pub volume_multiplier_16_16: u32,
}

/// A playable global sound resolved to its terminal PCM slot.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSound {
    pub requested_global_id: usize,
    /// Requested id, every alias target, and the terminal PCM id.
    pub resolution_chain: Vec<usize>,
    pub pcm_global_id: usize,
    pub alias_hops: Vec<SoundAliasHop>,
    pub frequency_multiplier: f64,
    pub volume_multiplier: f64,
}

/// Why a global sound slot could not resolve to PCM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundResolutionError {
    MissingSlot { global_id: usize, chain: Vec<usize> },
    UnusedSlot { global_id: usize, chain: Vec<usize> },
    AliasCycle { global_id: usize, chain: Vec<usize> },
}

impl std::fmt::Display for SoundResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSlot { global_id, .. } => {
                write!(f, "alias targets missing global sound {global_id}")
            }
            Self::UnusedSlot { global_id, .. } => {
                write!(f, "global sound {global_id} is unused")
            }
            Self::AliasCycle { global_id, .. } => {
                write!(f, "alias cycle returns to global sound {global_id}")
            }
        }
    }
}

impl std::error::Error for SoundResolutionError {}

/// Global sound-pool view. Tables must be supplied in the same order in which
/// the retail loader appends them to `DAT_004FE64C` (V2000: system level 2,
/// then system level 3).
#[derive(Debug)]
pub struct SoundPool<'a> {
    slots: Vec<SoundPoolSlot<'a>>,
}

impl<'a> SoundPool<'a> {
    pub fn from_tables(tables: &[&'a AnimSoundTable]) -> Self {
        let mut slots = Vec::new();
        for (table_index, table) in tables.iter().enumerate() {
            for (local_id, entry) in table.entries.iter().enumerate() {
                slots.push(SoundPoolSlot {
                    global_id: slots.len(),
                    table_index,
                    local_id,
                    table,
                    entry,
                });
            }
        }
        Self { slots }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn slots(&self) -> &[SoundPoolSlot<'a>] {
        &self.slots
    }

    pub fn slot(&self, global_id: usize) -> Option<SoundPoolSlot<'a>> {
        self.slots.get(global_id).copied()
    }

    /// Resolve type-5 aliases exactly as the runtime does, retaining every
    /// hop's raw modulation parameters for playback or diagnostic export.
    pub fn resolve(
        &self,
        requested_global_id: usize,
    ) -> std::result::Result<ResolvedSound, SoundResolutionError> {
        let mut resolution_chain = Vec::new();
        let mut alias_hops = Vec::new();
        let mut visited = vec![false; self.slots.len()];
        let mut current = requested_global_id;
        let mut frequency_multiplier = 1.0;
        let mut volume_multiplier = 1.0;

        loop {
            let Some(slot) = self.slot(current) else {
                resolution_chain.push(current);
                return Err(SoundResolutionError::MissingSlot {
                    global_id: current,
                    chain: resolution_chain,
                });
            };
            if visited[current] {
                resolution_chain.push(current);
                return Err(SoundResolutionError::AliasCycle {
                    global_id: current,
                    chain: resolution_chain,
                });
            }
            visited[current] = true;
            resolution_chain.push(current);

            match slot.entry.entry_type {
                EntryType::DataBlob => {
                    return Ok(ResolvedSound {
                        requested_global_id,
                        resolution_chain,
                        pcm_global_id: current,
                        alias_hops,
                        frequency_multiplier,
                        volume_multiplier,
                    });
                }
                EntryType::Alias => {
                    let target = slot.entry.offset_or_index as usize;
                    alias_hops.push(SoundAliasHop {
                        global_id: current,
                        target_global_id: target,
                        frequency_variance_16_16: slot.entry.size_or_scale,
                        frequency_multiplier_16_16: slot.entry.param1,
                        volume_multiplier_16_16: slot.entry.param2,
                    });
                    frequency_multiplier *= slot.entry.frequency_multiplier_f64();
                    volume_multiplier *= slot.entry.volume_multiplier_f64();
                    current = target;
                }
                EntryType::Unused => {
                    return Err(SoundResolutionError::UnusedSlot {
                        global_id: current,
                        chain: resolution_chain,
                    });
                }
            }
        }
    }
}

/// Result of parsing Section 11.
#[derive(Debug)]
pub struct AnimSoundTable {
    pub entries: Vec<AnimSoundEntry>,
    /// Owned copy of the full section data (for blob access).
    data: Vec<u8>,
}

impl AnimSoundTable {
    /// Get the raw blob data for a type=1 entry.
    /// Returns None for non-blob entries or if the offset/size is out of range.
    pub fn blob_data(&self, entry: &AnimSoundEntry) -> Option<&[u8]> {
        if entry.entry_type != EntryType::DataBlob {
            return None;
        }
        let start = entry.offset_or_index as usize;
        let size = entry.size_or_scale as usize;
        if start + size <= self.data.len() {
            Some(&self.data[start..start + size])
        } else {
            None
        }
    }

    /// Count entries by type.
    pub fn type1_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::DataBlob)
            .count()
    }

    /// Count type=5 alias entries.
    pub fn type5_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::Alias)
            .count()
    }

    /// Encode a type=1 blob as a WAV file (22050 Hz mono 16-bit PCM).
    /// Returns None for non-blob entries or out-of-range data.
    pub fn blob_as_wav(&self, entry: &AnimSoundEntry) -> Option<Vec<u8>> {
        let pcm = self.blob_data(entry)?;
        Some(crate::wav::encode_wav(pcm, &crate::wav::V2K_AUDIO))
    }
}

/// Parse Section 11 sound-effect entries.
///
/// Scans 20-byte entries from the start of data, stopping when an invalid
/// type is encountered (not 0, 1, or 5).
pub fn parse_anim_sound(data: &[u8]) -> Result<AnimSoundTable> {
    if data.is_empty() {
        return Err(V2kError::section(11, "section 11 is empty"));
    }

    let max_entries = data.len() / ENTRY_SIZE;
    let mut entries = Vec::new();

    for i in 0..max_entries.min(500) {
        let off = i * ENTRY_SIZE;
        if off + ENTRY_SIZE > data.len() {
            break;
        }

        let raw_type = read_u32(data, off)?;
        let raw_val = read_u32(data, off + 4)?;
        let field_08 = read_u32(data, off + 8)?;
        let field_0c = read_u32(data, off + 12)?;
        let field_10 = read_u32(data, off + 16)?;

        let entry_type = match raw_type {
            0 => EntryType::Unused,
            1 => EntryType::DataBlob,
            5 => EntryType::Alias,
            _ => break, // End of entry table
        };

        let (offset_or_index, size_or_scale) = match entry_type {
            EntryType::DataBlob => {
                // raw_val is a relative offset within the section data
                (raw_val, field_08)
            }
            EntryType::Alias => {
                // raw_val lower 16 bits = cross-reference index
                (raw_val & 0xFFFF, field_08)
            }
            EntryType::Unused => (raw_val, field_08),
        };

        entries.push(AnimSoundEntry {
            index: i,
            entry_type,
            raw_type,
            offset_or_index,
            size_or_scale,
            param1: field_0c,
            param2: field_10,
        });
    }

    Ok(AnimSoundTable {
        entries,
        data: data.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_type1_entry() {
        // Entry 0 at offset 0: type=1, blob at offset 80, size 20
        // Entry 1 at offset 20: type=99 (invalid, stops scan)
        // Blob data at offset 80
        let mut data = vec![0u8; 120];
        // Entry 0: type = 1
        data[0..4].copy_from_slice(&1u32.to_le_bytes());
        // offset = 80 (blob starts after both entry slots)
        data[4..8].copy_from_slice(&80u32.to_le_bytes());
        // size = 20
        data[8..12].copy_from_slice(&20u32.to_le_bytes());
        // Entry 1: invalid type to end scan
        data[ENTRY_SIZE..ENTRY_SIZE + 4].copy_from_slice(&99u32.to_le_bytes());
        // Fill blob at offset 80 with 0xAB
        data[80..100].fill(0xAB);

        let table = parse_anim_sound(&data).unwrap();
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].entry_type, EntryType::DataBlob);
        let blob = table.blob_data(&table.entries[0]).unwrap();
        assert_eq!(blob.len(), 20);
        assert!(blob.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn parse_type5_entry() {
        let mut data = vec![0u8; ENTRY_SIZE * 2];
        // type = 5
        data[0..4].copy_from_slice(&5u32.to_le_bytes());
        // xref_index = 3
        data[4..8].copy_from_slice(&3u32.to_le_bytes());
        // scale = 0x10000 (1.0 in 16.16)
        data[8..12].copy_from_slice(&0x10000u32.to_le_bytes());
        // Invalid type to end scan
        data[ENTRY_SIZE..ENTRY_SIZE + 4].copy_from_slice(&99u32.to_le_bytes());

        let table = parse_anim_sound(&data).unwrap();
        assert_eq!(table.entries.len(), 1);
        assert_eq!(table.entries[0].entry_type, EntryType::Alias);
        assert_eq!(table.entries[0].offset_or_index, 3);
        assert!((table.entries[0].frequency_variance_f64() - 1.0).abs() < 0.001);
    }

    fn table(entries: &[(u32, u32, u32, u32, u32)]) -> AnimSoundTable {
        let mut data = vec![0u8; (entries.len() + 1) * ENTRY_SIZE];
        for (index, fields) in entries.iter().enumerate() {
            let offset = index * ENTRY_SIZE;
            for (field, value) in [fields.0, fields.1, fields.2, fields.3, fields.4]
                .into_iter()
                .enumerate()
            {
                data[offset + field * 4..offset + field * 4 + 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        let end = entries.len() * ENTRY_SIZE;
        data[end..end + 4].copy_from_slice(&99u32.to_le_bytes());
        parse_anim_sound(&data).unwrap()
    }

    #[test]
    fn global_pool_resolves_alias_chain_and_volume_field() {
        let first = table(&[(1, 60, 0, 0, 0)]);
        let second = table(&[(5, 0, 0x3333, 0xB333, 0x10000), (5, 1, 0, 0x20000, 0x8000)]);
        let pool = SoundPool::from_tables(&[&first, &second]);

        let resolved = pool.resolve(2).unwrap();
        assert_eq!(resolved.resolution_chain, vec![2, 1, 0]);
        assert_eq!(resolved.pcm_global_id, 0);
        assert_eq!(resolved.alias_hops.len(), 2);
        assert_eq!(resolved.alias_hops[0].volume_multiplier_16_16, 0x8000);
        assert!((resolved.frequency_multiplier - 1.4).abs() < 0.0001);
        assert!((resolved.volume_multiplier - 0.5).abs() < 0.0001);
    }

    #[test]
    fn global_pool_reports_cycles_and_dangling_targets() {
        let cycles = table(&[(5, 1, 0, 0x10000, 0x10000), (5, 0, 0, 0x10000, 0x10000)]);
        let pool = SoundPool::from_tables(&[&cycles]);
        assert_eq!(
            pool.resolve(0),
            Err(SoundResolutionError::AliasCycle {
                global_id: 0,
                chain: vec![0, 1, 0],
            })
        );

        let dangling = table(&[(5, 9, 0, 0x10000, 0x10000)]);
        let pool = SoundPool::from_tables(&[&dangling]);
        assert_eq!(
            pool.resolve(0),
            Err(SoundResolutionError::MissingSlot {
                global_id: 9,
                chain: vec![0, 9],
            })
        );
    }

    #[test]
    fn empty_data() {
        assert!(parse_anim_sound(&[]).is_err());
    }
}
