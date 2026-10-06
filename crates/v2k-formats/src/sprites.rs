//! Sprite atlas decoder for V2000 OVL Section 3 + Section 8.
//!
//! Section 3 layout (after "abcd" marker):
//!   [4B tex_buf_size = 0x001AE000]  <- texture atlas buffer size
//!   [4B entry_data_size]            <- size of entry metadata + palette blocks
//!   [entry_data bytes]:
//!       N x 28-byte entries (sprite metadata)
//!       followed by 1024-byte palette blocks (RGB555 shade tables)
//!   [pixel loop]:
//!       0xFFFF-terminated rectangles placed into the texture atlas
//!       Each rect: [u16 x_start][u16 y_start][u16 x_end][u16 y_end][pixel_data]
//!
//! This Rust decoder is canonical; the earlier Python prototype is retired.

use crate::ovl::{read_u16, read_u32};
use crate::palette;
use v2k_core::{Result, V2kError};

/// Atlas row stride in bytes, shared by indexed and RGB555 rectangles.
pub const ATLAS_STRIDE: usize = 2048;

/// Sprite entry size in bytes (7 x uint32, but read as mixed u16/u32).
const ENTRY_SIZE: usize = 28;

/// Intrinsic source encoding selected by the sprite's render flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteTexelFormat {
    Indexed8,
    /// Little-endian RGB555 words; the unused high bit is not alpha.
    Rgb555,
}

/// A sprite entry from the OVL metadata.
#[derive(Debug, Clone)]
pub struct SpriteEntry {
    /// Sequential index within this OVL.
    pub entry_idx: usize,
    /// Sprite ID from the entry data.
    pub index: u16,
    /// Runtime render flags stored in the low byte (`0x01` zero
    /// keying, `0x02` RGB555 texels, `0x04` fixed shade row, `0x08`
    /// half-additive blend, `0x10` additive blend). The historical field name
    /// is retained for source compatibility with the extractor/viewer.
    pub pal_size: u16,
    /// Palette colour count/layout selector. Values 1–16 use the normal
    /// 32-shade × 16-colour ramp; larger values are flat direct palettes.
    pub shade_count: u16,
    /// Byte offset into the texture atlas.
    pub tex_offset: u32,
    /// Byte offset into entry_data pointing to the palette block.
    pub pal_offset: u32,
    /// Packed on-disk dimensions at offsets `+0x10/+0x12`. Kept as one raw
    /// word for compatibility; decoded dimensions come from the atlas rect.
    pub flags: u32,
}

impl SpriteEntry {
    pub const fn texel_format(&self) -> SpriteTexelFormat {
        if self.pal_size & 0x02 != 0 {
            SpriteTexelFormat::Rgb555
        } else {
            SpriteTexelFormat::Indexed8
        }
    }

    /// Whether retail selects the zero-keyed filler family. Indexed texels
    /// key index zero; RGB555 texels key zero after masking the unused bit 15.
    /// This policy comes solely from render-flag bit 0, independently of the
    /// sprite's palette/shade layout.
    pub const fn is_zero_keyed(&self) -> bool {
        self.pal_size & 0x01 != 0
    }
}

/// A pixel rectangle placed into the atlas.
#[derive(Debug, Clone)]
pub struct PixelRect {
    /// Horizontal byte coordinate in the atlas, not an RGB555 pixel index.
    pub x_start: u16,
    pub y_start: u16,
    /// Bytes per rectangle row; RGB555 output has half this many pixels.
    pub width: u16,
    pub height: u16,
    /// Offset into the raw OVL data where pixel bytes start.
    pub data_offset: usize,
}

/// A decoded sprite: RGBA pixels + metadata.
#[derive(Debug)]
pub struct DecodedSprite {
    pub entry: SpriteEntry,
    pub width: u16,
    pub height: u16,
    /// RGBA8 pixel data, row-major, width*height*4 bytes.
    pub rgba: Vec<u8>,
}

/// Raw indexed pixels for shader-driven palette lookup.
#[derive(Debug)]
pub struct DecodedIndexedSprite {
    pub width: u16,
    pub height: u16,
    pub indices: Vec<u8>,
}

/// Result of parsing Section 3 sprite data.
#[derive(Debug)]
pub struct SpriteAtlas {
    /// The raw atlas bytes, containing indexed and/or RGB555 rectangles.
    pub atlas: Vec<u8>,
    /// Atlas row width in bytes (always 2048).
    pub width: usize,
    /// Atlas height (derived from buffer size).
    pub height: usize,
    /// Sprite entries (metadata).
    pub entries: Vec<SpriteEntry>,
    /// Pixel rectangles used to build the atlas.
    pub rects: Vec<PixelRect>,
    /// Raw entry data (contains palette blocks).
    pub entry_data: Vec<u8>,
}

impl SpriteAtlas {
    fn rect_for(&self, entry: &SpriteEntry) -> Result<&PixelRect> {
        self.rects
            .iter()
            .find(|r| {
                let atlas_off = r.y_start as usize * ATLAS_STRIDE + r.x_start as usize;
                atlas_off == entry.tex_offset as usize
            })
            .ok_or_else(|| {
                V2kError::section(3, format!("no rect for tex_offset {}", entry.tex_offset))
            })
    }

    /// Pixel dimensions for the entry's intrinsic texel encoding, without
    /// allocating decoded pixels or requiring a palette.
    pub fn dimensions(&self, entry: &SpriteEntry) -> Result<(u16, u16)> {
        self.rect_dimensions(entry, self.rect_for(entry)?)
    }

    fn rect_dimensions(&self, entry: &SpriteEntry, rect: &PixelRect) -> Result<(u16, u16)> {
        if entry.texel_format() == SpriteTexelFormat::Indexed8 {
            return Ok((rect.width, rect.height));
        }
        if rect.width == 0 || rect.width % 2 != 0 || rect.height == 0 {
            return Err(V2kError::section(
                3,
                "RGB555 sprite requires a nonempty rectangle with an even byte width",
            ));
        }
        let row_end = usize::from(rect.x_start) + usize::from(rect.width);
        if row_end > ATLAS_STRIDE {
            return Err(V2kError::section(3, "RGB555 sprite exceeds atlas byte row"));
        }
        let final_row = usize::from(rect.y_start) + usize::from(rect.height) - 1;
        if final_row * ATLAS_STRIDE + row_end > self.atlas.len() {
            return Err(V2kError::section(
                3,
                "RGB555 sprite exceeds atlas byte buffer",
            ));
        }
        Ok((rect.width / 2, rect.height))
    }

    /// Extract the original 8-bit texels without applying transparency or a
    /// palette. Terrain's software filler performs this lookup at raster time.
    pub fn decode_indices(&self, entry: &SpriteEntry) -> Result<DecodedIndexedSprite> {
        if entry.texel_format() != SpriteTexelFormat::Indexed8 {
            return Err(V2kError::section(3, "RGB555 sprite has no palette indices"));
        }
        let rect = self.rect_for(entry)?;
        let (w, h) = (rect.width as usize, rect.height as usize);
        let mut indices = Vec::with_capacity(w * h);
        for y in 0..h {
            let row = rect.y_start as usize + y;
            for x in 0..w {
                let col = rect.x_start as usize + x;
                indices.push(*self.atlas.get(row * ATLAS_STRIDE + col).unwrap_or(&0));
            }
        }
        Ok(DecodedIndexedSprite {
            width: rect.width,
            height: rect.height,
            indices,
        })
    }

    /// Return the palette row used for a shade input. Normal sprites expose
    /// the engine's 32×16 ramp; direct-indexed sprites return their flat table.
    /// RGB555 texels have no palette and are rejected.
    pub fn palette_row(&self, entry: &SpriteEntry, shade_level: usize) -> Result<Vec<[u8; 4]>> {
        if entry.texel_format() != SpriteTexelFormat::Indexed8 {
            return Err(V2kError::section(3, "RGB555 sprite has no palette row"));
        }
        if entry.shade_count as usize > palette::COLORS_PER_SHADE {
            Ok(palette::direct_palette(
                &self.entry_data,
                entry.pal_offset as usize,
                entry.shade_count as usize,
            ))
        } else {
            let palette = palette::parse_palette(&self.entry_data, entry.pal_offset as usize)?;
            Ok(palette::flat_palette(&palette, shade_level))
        }
    }

    /// Decode intrinsic sprite pixels to RGBA. Indexed texels use the palette
    /// at the given shade level; RGB555 source words ignore palette and shade.
    /// Runtime lighting and compositing remain the renderer's responsibility.
    /// Render-flag bit 0 selects the retail zero-key filler family; when it is
    /// clear, zero remains ordinary authored colour. The filler
    /// selection is independent of palette/shade layout, including one-colour
    /// sprites.
    pub fn decode_sprite(&self, entry: &SpriteEntry, shade_level: usize) -> Result<DecodedSprite> {
        self.decode_sprite_impl(entry, shade_level, entry.is_zero_keyed())
    }

    /// Decode a sprite for an opaque surface pass. Unlike ordinary sprites,
    /// indexed zero remains a visible palette colour and RGB555 zero remains
    /// opaque black. The original
    /// terrain filler is opaque and does not apply the masked-sprite zero
    /// test used by models, billboards, and shoreline overlays.
    pub fn decode_sprite_opaque(
        &self,
        entry: &SpriteEntry,
        shade_level: usize,
    ) -> Result<DecodedSprite> {
        self.decode_sprite_impl(entry, shade_level, false)
    }

    fn decode_sprite_impl(
        &self,
        entry: &SpriteEntry,
        shade_level: usize,
        transparent_zero: bool,
    ) -> Result<DecodedSprite> {
        let rect = self.rect_for(entry)?;
        let (width, height) = self.rect_dimensions(entry, rect)?;

        if entry.texel_format() == SpriteTexelFormat::Rgb555 {
            let mut rgba = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
            for y in 0..usize::from(height) {
                let row_start =
                    (usize::from(rect.y_start) + y) * ATLAS_STRIDE + usize::from(rect.x_start);
                let row = &self.atlas[row_start..row_start + usize::from(rect.width)];
                for bytes in row.chunks_exact(2) {
                    let word = u16::from_le_bytes([bytes[0], bytes[1]]);
                    // FUN_004AA2E0 relocates low-15 RGB555 before the keyed
                    // raw filler tests zero; 0x8000 therefore also keys out.
                    let color = if transparent_zero && word & 0x7fff == 0 {
                        [0; 4]
                    } else {
                        palette::decode_rgb555(word)
                    };
                    rgba.extend_from_slice(&color);
                }
            }
            return Ok(DecodedSprite {
                entry: entry.clone(),
                width,
                height,
                rgba,
            });
        }

        let w = rect.width as usize;
        let h = rect.height as usize;

        // Two palette layouts, distinguished by shade_count:
        //  - shade_count <= 16: a 32-shade × 16-slot ramp; pixel value picks
        //    the colour, `shade_level` picks the brightness (logos, terrain).
        //    Section-3 bit 0 independently decides whether pixel 0 is keyed.
        //  - shade_count  > 16: a flat run of `shade_count` colours indexed
        //    directly by the pixel byte (0..shade_count) — full-colour
        //    sprites like the menu flame billboard (247 colours). No shading.
        let colors = self.palette_row(entry, shade_level)?;

        // Render RGBA
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            let atlas_row = rect.y_start as usize + y;
            for x in 0..w {
                let atlas_col = rect.x_start as usize + x;
                let atlas_idx = atlas_row * ATLAS_STRIDE + atlas_col;
                let px = if atlas_idx < self.atlas.len() {
                    self.atlas[atlas_idx]
                } else {
                    0
                };

                let out_idx = (y * w + x) * 4;
                if transparent_zero && px == 0 {
                    // Transparent
                    rgba[out_idx] = 0;
                    rgba[out_idx + 1] = 0;
                    rgba[out_idx + 2] = 0;
                    rgba[out_idx + 3] = 0;
                } else {
                    let c = colors[px as usize];
                    rgba[out_idx] = c[0];
                    rgba[out_idx + 1] = c[1];
                    rgba[out_idx + 2] = c[2];
                    rgba[out_idx + 3] = c[3];
                }
            }
        }

        Ok(DecodedSprite {
            entry: entry.clone(),
            width: rect.width,
            height: rect.height,
            rgba,
        })
    }

    /// Decode every sprite entry, returning one error that identifies every
    /// entry that could not be decoded.
    ///
    /// Bulk exporters should use this path so malformed or newly understood
    /// entries cannot disappear from generated assets without explanation.
    pub fn decode_all_strict(&self, shade_level: usize) -> Result<Vec<DecodedSprite>> {
        let mut decoded = Vec::with_capacity(self.entries.len());
        let mut failures = Vec::new();

        for entry in &self.entries {
            match self.decode_sprite(entry, shade_level) {
                Ok(sprite) => decoded.push(sprite),
                Err(error) => failures.push(format!(
                    "entry {} (sprite {}, tex_offset {}): {error}",
                    entry.entry_idx, entry.index, entry.tex_offset
                )),
            }
        }

        if failures.is_empty() {
            Ok(decoded)
        } else {
            Err(V2kError::section(
                3,
                format!(
                    "failed to decode {} of {} sprite entries:\n{}",
                    failures.len(),
                    self.entries.len(),
                    failures.join("\n")
                ),
            ))
        }
    }

    /// Best-effort decode for runtime consumers that can tolerate missing
    /// entries. Bulk exporters should use [`Self::decode_all_strict`] instead.
    pub fn decode_all(&self, shade_level: usize) -> Vec<DecodedSprite> {
        self.entries
            .iter()
            .filter_map(|e| self.decode_sprite(e, shade_level).ok())
            .collect()
    }
}

/// Parse Section 3 data from a complete OVL section (the raw data after "abcd" + value).
///
/// Note: The caller should pass the OvlSection for section 3.
/// The header_value is the tex_buf_size (usually 0x001AE000).
/// The section data starts with entry_data_size, then entry_data, then pixel rects.
pub fn parse_section3(header_value: u32, section_data: &[u8]) -> Result<SpriteAtlas> {
    let tex_buf_size = header_value as usize;
    if tex_buf_size == 0 {
        return Err(V2kError::section(3, "tex_buf_size is 0"));
    }

    if section_data.len() < 4 {
        return Err(V2kError::section(3, "section data too short"));
    }

    // Read entry_data_size
    let entry_data_size = read_u32(section_data, 0)? as usize;
    if entry_data_size == 0 {
        return Err(V2kError::section(3, "entry_data_size is 0"));
    }

    if 4 + entry_data_size > section_data.len() {
        return Err(V2kError::section(3, "entry_data exceeds section bounds"));
    }

    // Extract entry data
    let entry_data = section_data[4..4 + entry_data_size].to_vec();

    // Palette data follows the 28-byte entry table. Most atlases put the
    // table-end offset in the first entry, but system level 51 starts with 30
    // palette-less entries (`pal_offset == 0`) before three entries with local
    // palettes. Find the earliest plausible table boundary instead of treating
    // the whole metadata+palette block as entries when the first pointer is
    // null.
    if entry_data.len() < ENTRY_SIZE {
        return Err(V2kError::section(
            3,
            "entry data too small for even one entry",
        ));
    }
    let entry_count = infer_entry_count(&entry_data)?;

    // Parse entries
    let mut entries = Vec::with_capacity(entry_count);
    for i in 0..entry_count {
        let off = i * ENTRY_SIZE;
        if off + ENTRY_SIZE > entry_data.len() {
            break;
        }
        let index = read_u16(&entry_data, off)?;
        let _pad = read_u16(&entry_data, off + 2)?;
        let pal_size = read_u16(&entry_data, off + 4)?;
        let shade_count = read_u16(&entry_data, off + 6)?;
        let tex_offset = read_u32(&entry_data, off + 8)?;
        let pal_offset = read_u32(&entry_data, off + 12)?;
        let flags = read_u32(&entry_data, off + 16)?;

        entries.push(SpriteEntry {
            entry_idx: i,
            index,
            pal_size,
            shade_count,
            tex_offset,
            pal_offset,
            flags,
        });
    }

    // Parse pixel rectangles
    let pixel_loop_start = 4 + entry_data_size;
    let rects = parse_pixel_rects(section_data, pixel_loop_start)?;

    // Build atlas
    let mut atlas = vec![0u8; tex_buf_size];
    for rect in &rects {
        let w = rect.width as usize;
        for row_idx in 0..rect.height as usize {
            let y = rect.y_start as usize + row_idx;
            let src = rect.data_offset + row_idx * w;
            let dst = y * ATLAS_STRIDE + rect.x_start as usize;
            if src + w <= section_data.len() && dst + w <= atlas.len() {
                atlas[dst..dst + w].copy_from_slice(&section_data[src..src + w]);
            }
        }
    }

    let height = tex_buf_size / ATLAS_STRIDE;

    Ok(SpriteAtlas {
        atlas,
        width: ATLAS_STRIDE,
        height,
        entries,
        rects,
        entry_data,
    })
}

fn infer_entry_count(entry_data: &[u8]) -> Result<usize> {
    let maximum_count = entry_data.len() / ENTRY_SIZE;
    let mut entry_count = maximum_count;

    for index in 0..maximum_count {
        let record_end = (index + 1) * ENTRY_SIZE;
        let pal_offset = read_u32(entry_data, index * ENTRY_SIZE + 12)? as usize;
        if pal_offset >= record_end
            && pal_offset <= entry_data.len()
            && pal_offset % ENTRY_SIZE == 0
        {
            entry_count = entry_count.min(pal_offset / ENTRY_SIZE);
        }
    }

    Ok(entry_count)
}

/// Parse 0xFFFF-terminated pixel rectangles.
fn parse_pixel_rects(data: &[u8], start: usize) -> Result<Vec<PixelRect>> {
    let mut rects = Vec::new();
    let mut pos = start;
    let end = data.len();

    while pos + 2 <= end {
        let x_start = read_u16(data, pos)?;
        pos += 2;

        if x_start == 0xFFFF {
            break;
        }

        if pos + 6 > end {
            break;
        }
        let y_start = read_u16(data, pos)?;
        let x_end = read_u16(data, pos + 2)?;
        let y_end = read_u16(data, pos + 4)?;
        pos += 6;

        let width = x_end.saturating_sub(x_start);
        let height = y_end.saturating_sub(y_start);

        if width == 0 || height == 0 || width as usize > ATLAS_STRIDE || height > 2048 {
            continue;
        }

        let pixel_count = width as usize * height as usize;
        if pos + pixel_count > end {
            break;
        }

        rects.push(PixelRect {
            x_start,
            y_start,
            width,
            height,
            data_offset: pos,
        });
        pos += pixel_count;
    }

    Ok(rects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_pixel_atlas(shade_count: u16, render_flags: u16) -> SpriteAtlas {
        let pal_offset = ENTRY_SIZE;
        let mut entry_data = vec![0u8; ENTRY_SIZE + palette::PALETTE_BLOCK_SIZE];
        let brightest_px0 = pal_offset + palette::BRIGHTEST_SHADE * palette::COLORS_PER_SHADE * 2;
        entry_data[brightest_px0..brightest_px0 + 2].copy_from_slice(&0x7fffu16.to_le_bytes());

        SpriteAtlas {
            atlas: vec![0],
            width: 1,
            height: 1,
            entries: vec![SpriteEntry {
                entry_idx: 0,
                index: 0,
                pal_size: render_flags,
                shade_count,
                tex_offset: 0,
                pal_offset: pal_offset as u32,
                flags: 0,
            }],
            rects: vec![PixelRect {
                x_start: 0,
                y_start: 0,
                width: 1,
                height: 1,
                data_offset: 0,
            }],
            entry_data,
        }
    }

    #[test]
    fn masked_single_colour_sprite_keeps_zero_transparent() {
        let atlas = single_pixel_atlas(1, 0x01);
        assert!(atlas.entries[0].is_zero_keyed());
        let decoded = atlas
            .decode_sprite(&atlas.entries[0], palette::BRIGHTEST_SHADE)
            .unwrap();
        assert_eq!(&decoded.rgba, &[0, 0, 0, 0]);
    }

    #[test]
    fn opaque_single_colour_sprite_keeps_palette_zero() {
        let atlas = single_pixel_atlas(1, 0x00);
        assert!(!atlas.entries[0].is_zero_keyed());
        let decoded = atlas
            .decode_sprite(&atlas.entries[0], palette::BRIGHTEST_SHADE)
            .unwrap();
        assert_eq!(&decoded.rgba, &[248, 248, 248, 255]);
    }

    #[test]
    fn masked_multi_colour_sprites_keep_zero_transparent() {
        let atlas = single_pixel_atlas(2, 0x01);
        let decoded = atlas
            .decode_sprite(&atlas.entries[0], palette::BRIGHTEST_SHADE)
            .unwrap();
        assert_eq!(&decoded.rgba, &[0, 0, 0, 0]);
    }

    #[test]
    fn opaque_multi_colour_sprites_keep_palette_zero() {
        let atlas = single_pixel_atlas(2, 0x00);
        let decoded = atlas
            .decode_sprite(&atlas.entries[0], palette::BRIGHTEST_SHADE)
            .unwrap();
        assert_eq!(&decoded.rgba, &[248, 248, 248, 255]);
    }

    #[test]
    fn opaque_surface_decode_keeps_palette_zero() {
        let atlas = single_pixel_atlas(2, 0x01);
        let decoded = atlas
            .decode_sprite_opaque(&atlas.entries[0], palette::BRIGHTEST_SHADE)
            .unwrap();
        assert_eq!(&decoded.rgba, &[248, 248, 248, 255]);
    }

    #[test]
    fn indexed_decode_preserves_zero_and_dimensions() {
        let atlas = single_pixel_atlas(2, 0x01);
        let decoded = atlas.decode_indices(&atlas.entries[0]).unwrap();
        assert_eq!((decoded.width, decoded.height), (1, 1));
        assert_eq!(decoded.indices, vec![0]);
        assert_eq!(
            atlas
                .palette_row(&atlas.entries[0], palette::BRIGHTEST_SHADE)
                .unwrap()[0],
            [248, 248, 248, 255]
        );
    }

    #[test]
    fn strict_bulk_decode_returns_every_entry_when_valid() {
        let atlas = single_pixel_atlas(2, 0x01);
        let decoded = atlas.decode_all_strict(palette::BRIGHTEST_SHADE).unwrap();
        assert_eq!(decoded.len(), atlas.entries.len());
    }

    #[test]
    fn strict_bulk_decode_reports_every_failed_entry() {
        let mut atlas = single_pixel_atlas(2, 0x01);
        let mut first_missing = atlas.entries[0].clone();
        first_missing.entry_idx = 1;
        first_missing.index = 101;
        first_missing.tex_offset = 100;
        let mut second_missing = atlas.entries[0].clone();
        second_missing.entry_idx = 2;
        second_missing.index = 202;
        second_missing.tex_offset = 200;
        atlas.entries.extend([first_missing, second_missing]);

        let error = atlas
            .decode_all_strict(palette::BRIGHTEST_SHADE)
            .unwrap_err()
            .to_string();

        assert!(error.contains("failed to decode 2 of 3 sprite entries"));
        assert!(error.contains("entry 1 (sprite 101, tex_offset 100)"));
        assert!(error.contains("entry 2 (sprite 202, tex_offset 200)"));
        assert_eq!(atlas.decode_all(palette::BRIGHTEST_SHADE).len(), 1);
    }

    #[test]
    fn entry_table_boundary_can_come_from_a_later_palette_pointer() {
        const ENTRY_COUNT: usize = 3;
        let table_bytes = ENTRY_COUNT * ENTRY_SIZE;
        let mut entry_data = vec![0u8; table_bytes + palette::PALETTE_BLOCK_SIZE];

        // The first two records deliberately have no local palette. The third
        // record identifies the start of the palette region, as in level 51.
        entry_data[2 * ENTRY_SIZE + 12..2 * ENTRY_SIZE + 16]
            .copy_from_slice(&(table_bytes as u32).to_le_bytes());

        let mut section_data = Vec::new();
        section_data.extend_from_slice(&(entry_data.len() as u32).to_le_bytes());
        section_data.extend_from_slice(&entry_data);
        section_data.extend_from_slice(&0xffffu16.to_le_bytes());

        let atlas = parse_section3(ATLAS_STRIDE as u32, &section_data).unwrap();
        assert_eq!(atlas.entries.len(), ENTRY_COUNT);
    }
}
