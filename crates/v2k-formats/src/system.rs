//! System section decoders for V2000 OVL Sections 0, 1, 5, 6, 7.
//!
//! Section 0 & 1: Runtime pointer fixup tables (flat u32 arrays).
//! Section 5: Display mode configuration (4 × 20B records).
//! Section 6: Fog/shade gradient table (26 × 4B RGBA entries).
//! Section 7: RGB555 color palettes (N × 4B entries, decoded via `palette::decode_rgb555`).
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::read_u32;
use crate::palette;
use v2k_core::{Result, V2kError};

// ── Section 0 & 1: Pointer fixup tables ─────────────────────────────────────

/// A pointer fixup table (Sections 0 or 1).
#[derive(Debug, Clone)]
pub struct FixupTable {
    pub entries: Vec<u32>,
}

/// Parse a pointer fixup table from raw section data.
/// Each entry is a u32 that gets relocated at load time.
pub fn parse_fixup_table(data: &[u8]) -> FixupTable {
    let count = data.len() / 4;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * 4;
        if off + 4 <= data.len() {
            let val = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
            entries.push(val);
        }
    }
    FixupTable { entries }
}

// ── Section 5: Display mode configuration ───────────────────────────────────

const DISPLAY_MODE_SIZE: usize = 20;

/// A display mode record from Section 5.
#[derive(Debug, Clone)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub reserved: u32,
    pub param_1: u32,
    pub param_2: u32,
}

/// Parse display mode records from Section 5 data.
/// Each record is 20 bytes: width, height, reserved, param_1, param_2.
pub fn parse_display_modes(data: &[u8]) -> Result<Vec<DisplayMode>> {
    if data.len() < DISPLAY_MODE_SIZE {
        return Err(V2kError::section(5, "section 5 data too short"));
    }
    let count = data.len() / DISPLAY_MODE_SIZE;
    let mut modes = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * DISPLAY_MODE_SIZE;
        if off + DISPLAY_MODE_SIZE > data.len() {
            break;
        }
        modes.push(DisplayMode {
            width: read_u32(data, off)?,
            height: read_u32(data, off + 4)?,
            reserved: read_u32(data, off + 8)?,
            param_1: read_u32(data, off + 12)?,
            param_2: read_u32(data, off + 16)?,
        });
    }
    Ok(modes)
}

// ── Section 6: render shade lookup table ────────────────────────────────────

/// A packed render shade lookup entry from Section 6.
///
/// The legacy type name is retained for API compatibility. Terrain indexes
/// the first eight records with its 0..7 light result and consumes
/// `shade_level` as a direct 0..31 Section-3 palette row. Gameplay fog colour
/// comes from the authored sky palette instead.
#[derive(Debug, Clone)]
pub struct FogGradientEntry {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub shade_level: u8,
}

/// Parse render shade lookup entries from Section 6 data.
/// Each entry is 4 bytes: R, G, B, shade_level.
pub fn parse_fog_gradient(data: &[u8]) -> Result<Vec<FogGradientEntry>> {
    if data.len() < 4 {
        return Err(V2kError::section(6, "section 6 data too short"));
    }
    let count = data.len() / 4;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * 4;
        entries.push(FogGradientEntry {
            r: data[off],
            g: data[off + 1],
            b: data[off + 2],
            shade_level: data[off + 3],
        });
    }
    Ok(entries)
}

// ── Section 7: RGB555 color palettes ────────────────────────────────────────

/// A color palette entry from Section 7.
#[derive(Debug, Clone)]
pub struct PaletteEntry {
    /// Raw RGB555 value (lower 16 bits of the u32).
    pub rgb555: u16,
    /// Decoded 8-bit red.
    pub r: u8,
    /// Decoded 8-bit green.
    pub g: u8,
    /// Decoded 8-bit blue.
    pub b: u8,
}

/// Parse RGB555 color palette entries from Section 7 data.
/// Each entry is 4 bytes (u32), lower 16 bits = RGB555 color value.
pub fn parse_color_palettes(data: &[u8]) -> Result<Vec<PaletteEntry>> {
    if data.len() < 4 {
        return Err(V2kError::section(7, "section 7 data too short"));
    }
    let count = data.len() / 4;
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let val32 = read_u32(data, i * 4)?;
        let rgb555 = (val32 & 0xFFFF) as u16;
        let [r, g, b, _] = palette::decode_rgb555(rgb555);
        entries.push(PaletteEntry { rgb555, r, g, b });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixup_table_basic() {
        let data = [
            0x01, 0x00, 0x00, 0x00, // 1
            0xFF, 0x00, 0x00, 0x00, // 255
            0x00, 0x10, 0x00, 0x00, // 4096
        ];
        let table = parse_fixup_table(&data);
        assert_eq!(table.entries, vec![1, 255, 4096]);
    }

    #[test]
    fn fixup_table_empty() {
        let table = parse_fixup_table(&[]);
        assert!(table.entries.is_empty());
    }

    #[test]
    fn display_modes_basic() {
        let mut data = Vec::new();
        // 320x240
        data.extend_from_slice(&320u32.to_le_bytes());
        data.extend_from_slice(&240u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0x02000010u32.to_le_bytes());
        data.extend_from_slice(&0x00010101u32.to_le_bytes());
        let modes = parse_display_modes(&data).unwrap();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].width, 320);
        assert_eq!(modes[0].height, 240);
    }

    #[test]
    fn fog_gradient_basic() {
        let data = [128, 64, 32, 15, 255, 128, 0, 31];
        let entries = parse_fog_gradient(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].r, 128);
        assert_eq!(entries[0].shade_level, 15);
        assert_eq!(entries[1].shade_level, 31);
    }

    #[test]
    fn color_palette_basic() {
        // RGB555 white = 0x7FFF
        let mut data = Vec::new();
        data.extend_from_slice(&0x7FFFu32.to_le_bytes());
        // RGB555 black = 0x0000
        data.extend_from_slice(&0x0000u32.to_le_bytes());
        let entries = parse_color_palettes(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].rgb555, 0x7FFF);
        assert_eq!(entries[0].r, 248);
        assert_eq!(entries[0].g, 248);
        assert_eq!(entries[0].b, 248);
        assert_eq!(entries[1].rgb555, 0);
        assert_eq!(entries[1].r, 0);
    }
}
