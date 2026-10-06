//! RGB555 palette decoder for V2000 shade palettes.
//!
//! Section 3 palette blocks: 512 × `u16` RGB555, shade-major layout.
//! `palette[shade_level * 16 + pixel_value]` is one RGB555 colour:
//! 32 shade levels × 16 pixel-value slots = 512 entries.
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::read_u16;
use v2k_core::Result;

/// Palette block size: 512 entries x 2 bytes = 1024 bytes.
pub const PALETTE_BLOCK_SIZE: usize = 1024;

/// Number of shade levels per palette block.
pub const SHADE_LEVELS: usize = 32;

/// Number of pixel value slots per shade level.
pub const COLORS_PER_SHADE: usize = 16;

/// Brightest authored shade row.
pub const BRIGHTEST_SHADE: usize = SHADE_LEVELS - 1;

/// A decoded shade palette with 16 indexed colours (RGBA8) at 32 brightnesses.
#[derive(Debug, Clone)]
pub struct ShadePalette {
    /// 32 shade levels, each with 16 RGBA colours.
    /// shades[shade_level][pixel_value] = [R, G, B, A].
    pub shades: Vec<Vec<[u8; 4]>>,
}

impl ShadePalette {
    /// Get the color for a pixel value at a given shade level.
    pub fn color(&self, shade_level: usize, pixel_value: u8) -> [u8; 4] {
        let shade = shade_level.min(SHADE_LEVELS - 1);
        let pv = pixel_value as usize;
        if pv < self.shades[shade].len() {
            self.shades[shade][pv]
        } else {
            [0, 0, 0, 0]
        }
    }

    /// Get the brightest shade (level 31) colour for a pixel value.
    pub fn bright_color(&self, pixel_value: u8) -> [u8; 4] {
        self.color(BRIGHTEST_SHADE, pixel_value)
    }
}

/// Decode a 16-bit RGB555 value to RGBA8.
/// Format: bits 14:10 = R, 9:5 = G, 4:0 = B.
pub fn decode_rgb555(value: u16) -> [u8; 4] {
    // Retail expands each channel into the high five bits of an 8-bit
    // component and leaves the low three bits clear (`FUN_004a84a0` masks
    // the result with 0xf8). Do not normalize 31 to 255: the authored
    // brightest component is 248, and scaling it made every palette colour
    // subtly brighter than the original presentation.
    let r = (((value >> 10) & 0x1f) as u8) << 3;
    let g = (((value >> 5) & 0x1f) as u8) << 3;
    let b = ((value & 0x1f) as u8) << 3;
    [r, g, b, 255]
}

/// Parse a palette block from entry_data at the given offset.
/// Returns a [`ShadePalette`] with all 32 shade levels decoded.
pub fn parse_palette(entry_data: &[u8], pal_offset: usize) -> Result<ShadePalette> {
    let mut shades = Vec::with_capacity(SHADE_LEVELS);

    if pal_offset == 0 || pal_offset + PALETTE_BLOCK_SIZE > entry_data.len() {
        // No palette data — return grayscale fallback
        for _shade in 0..SHADE_LEVELS {
            let mut colors = vec![[0u8; 4]; COLORS_PER_SHADE];
            for (i, color) in colors.iter_mut().enumerate().take(16) {
                let v = (i as u8) * 17;
                *color = [v, v, v, 255];
            }
            shades.push(colors);
        }
        return Ok(ShadePalette { shades });
    }

    for shade in 0..SHADE_LEVELS {
        let base = pal_offset + shade * COLORS_PER_SHADE * 2;
        let mut colors = vec![[0u8; 4]; COLORS_PER_SHADE];
        for (px, color) in colors.iter_mut().enumerate() {
            let pos = base + px * 2;
            if pos + 2 <= entry_data.len() {
                let val = read_u16(entry_data, pos)?;
                *color = decode_rgb555(val);
            }
        }
        shades.push(colors);
    }

    Ok(ShadePalette { shades })
}

/// Build a simple flat palette for a given shade level (0–31).
/// Returns 256 RGBA entries indexed by pixel value.
pub fn flat_palette(palette: &ShadePalette, shade_level: usize) -> Vec<[u8; 4]> {
    let shade = shade_level.min(SHADE_LEVELS - 1);
    let mut colors = vec![[0u8; 4]; 256];
    for (i, c) in palette.shades[shade].iter().enumerate() {
        if i < 256 {
            colors[i] = *c;
        }
    }
    colors
}

/// Decode a **direct-indexed** palette: a flat run of `n_colors` RGB555
/// entries starting at `pal_offset`, indexed straight by the pixel value.
///
/// Used by full-colour sprites whose `shade_count` exceeds the 16-shade
/// ramp (e.g. the menu flame billboard: 247 colours, pixel values 0-246).
/// These are NOT shade ramps — the pixel byte is the palette index. Returns
/// 256 RGBA entries (unused slots stay transparent black).
pub fn direct_palette(entry_data: &[u8], pal_offset: usize, n_colors: usize) -> Vec<[u8; 4]> {
    let mut colors = vec![[0u8; 4]; 256];
    let n = n_colors.min(256);
    for (i, slot) in colors.iter_mut().enumerate().take(n) {
        let pos = pal_offset + i * 2;
        if pos + 2 <= entry_data.len() {
            *slot = decode_rgb555(read_u16(entry_data, pos).unwrap_or(0));
        }
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_rgb555_black() {
        assert_eq!(decode_rgb555(0x0000), [0, 0, 0, 255]);
    }

    #[test]
    fn decode_rgb555_white() {
        assert_eq!(decode_rgb555(0x7FFF), [248, 248, 248, 255]);
    }

    #[test]
    fn decode_rgb555_red() {
        // Pure red: R=31, G=0, B=0 = 0b_0_11111_00000_00000 = 0x7C00
        assert_eq!(decode_rgb555(0x7C00), [248, 0, 0, 255]);
    }

    #[test]
    fn decode_rgb555_matches_retail_sky_blue() {
        // The matching retail frame presents packed 0x0214 as #0080a0.
        assert_eq!(decode_rgb555(0x0214), [0, 128, 160, 255]);
    }

    #[test]
    fn grayscale_fallback() {
        let pal = parse_palette(&[], 0).unwrap();
        assert_eq!(pal.shades.len(), SHADE_LEVELS);
        // Pixel 0 should be black
        assert_eq!(pal.color(BRIGHTEST_SHADE, 0), [0, 0, 0, 255]);
        // Pixel 15 should be white (15*17 = 255)
        assert_eq!(pal.color(BRIGHTEST_SHADE, 15), [255, 255, 255, 255]);
    }

    #[test]
    fn shade_palette_is_32_rows_by_16_colours() {
        let mut data = vec![0u8; PALETTE_BLOCK_SIZE + 2];
        let base = 2;
        let (row, px) = (1usize, 2usize);
        let row1_px2 = base + (row * COLORS_PER_SHADE + px) * 2;
        let old_transposed_row1_px2 = base + (row * 32 + px) * 2;
        data[row1_px2..row1_px2 + 2].copy_from_slice(&0x7c00u16.to_le_bytes());
        data[old_transposed_row1_px2..old_transposed_row1_px2 + 2]
            .copy_from_slice(&0x03e0u16.to_le_bytes());

        let palette = parse_palette(&data, base).unwrap();
        assert_eq!(palette.color(1, 2), [248, 0, 0, 255]);
        assert_eq!(palette.color(2, 2), [0, 248, 0, 255]);
    }

    #[test]
    fn direct_palette_indexes_by_pixel() {
        // Flat run of RGB555 colours (as full-colour sprites like the menu
        // flame use): index N reads the Nth entry directly, no shade math.
        let mut data = vec![0u8; 8];
        data[0..2].copy_from_slice(&0x0000u16.to_le_bytes()); // idx 0 = black
        data[2..4].copy_from_slice(&0x7C00u16.to_le_bytes()); // idx 1 = red
        data[4..6].copy_from_slice(&0x03E0u16.to_le_bytes()); // idx 2 = green
        data[6..8].copy_from_slice(&0x001Fu16.to_le_bytes()); // idx 3 = blue
        let pal = direct_palette(&data, 0, 4);
        assert_eq!(pal[1], [248, 0, 0, 255]);
        assert_eq!(pal[2], [0, 248, 0, 255]);
        assert_eq!(pal[3], [0, 0, 248, 255]);
        // Past the flat run stays transparent black.
        assert_eq!(pal[4], [0, 0, 0, 0]);
        assert_eq!(pal[200], [0, 0, 0, 0]);
    }
}
