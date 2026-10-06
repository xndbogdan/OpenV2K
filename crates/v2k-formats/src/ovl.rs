//! OVL container parser for V2000's 15-section "abcd"-delimited format.
//!
//! Each section: [4 bytes "abcd"] [uint32 value] [data until next marker or EOF]
//!
//! This Rust parser is canonical; the earlier Python parsers are retired.

use v2k_core::{Result, V2kError};

const MAGIC: &[u8; 4] = b"abcd";
pub const NUM_SECTIONS: usize = 15;

/// A parsed OVL file containing 15 sections.
#[derive(Debug)]
pub struct OvlFile {
    pub sections: Vec<OvlSection>,
}

/// A single section from an OVL file.
#[derive(Debug)]
pub struct OvlSection {
    /// Section index (0..14).
    pub index: usize,
    /// Byte offset of the "abcd" marker in the file.
    pub marker_offset: usize,
    /// The uint32 value field after the "abcd" magic.
    /// Meaning varies: sections 0-2 use it as data size,
    /// section 3 stores 0x001AE000 (texture buffer size),
    /// others use 0 for empty or as an alloc hint.
    pub header_value: u32,
    /// Raw section data (from marker+8 to next marker or EOF).
    pub data: Vec<u8>,
}

impl OvlFile {
    /// Parse a complete OVL file from raw bytes.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let markers = find_markers(data);

        if markers.len() != NUM_SECTIONS {
            return Err(V2kError::OvlFormat(format!(
                "expected {} 'abcd' markers, found {}",
                NUM_SECTIONS,
                markers.len()
            )));
        }

        let mut sections = Vec::with_capacity(NUM_SECTIONS);

        for (i, &marker_off) in markers.iter().enumerate() {
            // Read uint32 value after "abcd"
            let val_off = marker_off + 4;
            if val_off + 4 > data.len() {
                return Err(V2kError::truncated(val_off, 4, data.len() - val_off));
            }
            let header_value = u32::from_le_bytes([
                data[val_off],
                data[val_off + 1],
                data[val_off + 2],
                data[val_off + 3],
            ]);

            // Data spans from marker+8 to next marker (or EOF)
            let data_start = marker_off + 8;
            let data_end = if i + 1 < markers.len() {
                markers[i + 1]
            } else {
                data.len()
            };

            let section_data = if data_start <= data_end {
                data[data_start..data_end].to_vec()
            } else {
                Vec::new()
            };

            sections.push(OvlSection {
                index: i,
                marker_offset: marker_off,
                header_value,
                data: section_data,
            });
        }

        Ok(OvlFile { sections })
    }

    /// Get a section by index. Returns None if out of range.
    pub fn section(&self, index: usize) -> Option<&OvlSection> {
        self.sections.get(index)
    }

    /// Check if a section has non-empty data.
    pub fn has_data(&self, index: usize) -> bool {
        self.sections.get(index).is_some_and(|s| !s.data.is_empty())
    }
}

/// Find all positions of the "abcd" magic marker in data.
fn find_markers(data: &[u8]) -> Vec<usize> {
    let mut markers = Vec::new();
    let mut pos = 0;
    while pos + 4 <= data.len() {
        if &data[pos..pos + 4] == MAGIC {
            markers.push(pos);
            pos += 4;
        } else {
            pos += 1;
        }
    }
    markers
}

/// Read a little-endian u16 from a byte slice at the given offset.
pub fn read_u16(data: &[u8], offset: usize) -> Result<u16> {
    if offset + 2 > data.len() {
        return Err(V2kError::truncated(
            offset,
            2,
            data.len().saturating_sub(offset),
        ));
    }
    Ok(u16::from_le_bytes([data[offset], data[offset + 1]]))
}

/// Read a little-endian u32 from a byte slice at the given offset.
pub fn read_u32(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(V2kError::truncated(
            offset,
            4,
            data.len().saturating_sub(offset),
        ));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty_ovl() -> Vec<u8> {
        // 15 sections, each with "abcd" + 4-byte value + no data
        let mut buf = Vec::new();
        for _ in 0..NUM_SECTIONS {
            buf.extend_from_slice(b"abcd");
            buf.extend_from_slice(&0u32.to_le_bytes());
        }
        buf
    }

    #[test]
    fn parse_empty_ovl() {
        let data = make_empty_ovl();
        let ovl = OvlFile::parse(&data).unwrap();
        assert_eq!(ovl.sections.len(), NUM_SECTIONS);
        for (i, sec) in ovl.sections.iter().enumerate() {
            assert_eq!(sec.index, i);
            assert_eq!(sec.header_value, 0);
            assert!(sec.data.is_empty());
        }
    }

    #[test]
    fn parse_with_data() {
        let mut data = Vec::new();
        // Section 0: 4 bytes of data
        data.extend_from_slice(b"abcd");
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        // Sections 1-14: empty
        for _ in 1..NUM_SECTIONS {
            data.extend_from_slice(b"abcd");
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        let ovl = OvlFile::parse(&data).unwrap();
        assert_eq!(ovl.sections[0].data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(ovl.sections[0].header_value, 4);
    }

    #[test]
    fn wrong_marker_count() {
        let mut data = Vec::new();
        for _ in 0..10 {
            data.extend_from_slice(b"abcd");
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        assert!(OvlFile::parse(&data).is_err());
    }
}
