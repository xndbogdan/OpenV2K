//! Owned Section-3 materials for a software backend.
//!
//! Retail binds pointers into the loaded resource block; the port keeps each
//! bound record's texels and palette in an [`OwnedMaterial`] and hands the
//! raster a [`MaterialView`] of it. Indexed materials keep their authored
//! bytes and palette words. Images that only exist as RGBA in the port are
//! stored as raw display-format texels, which the raw span rows and the
//! sprite blit draw exactly like the palette words they came from.

use std::sync::Arc;

use super::material::{flags, MaterialId, MaterialSource, MaterialView, ATLAS_STRIDE};
use crate::renderer::{NativeSprite, NativeTexels};

/// Texel words and palette of one bound material.
#[derive(Debug, Clone)]
pub struct OwnedMaterial {
    pub flags: u16,
    pub shade_count: u16,
    pub width: u16,
    pub height: u16,
    /// Rows of [`ATLAS_STRIDE`] bytes starting at the texel origin.
    pub atlas: Vec<u8>,
    /// Display-format palette block shared with the record's neighbours
    /// (empty for raw materials), and where this record's palette starts.
    pub palette: Arc<[u16]>,
    pub palette_start: usize,
}

/// RGB565 of an RGBA8 pixel, keeping the top bits of each channel.
#[inline]
pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3)
}

/// RGBA8 of an RGB565 pixel with the usual bit replication.
#[inline]
pub fn rgba8(pixel: u16) -> [u8; 4] {
    let r = (pixel >> 11) as u8;
    let g = ((pixel >> 5) & 0x3F) as u8;
    let b = (pixel & 0x1F) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        0xFF,
    ]
}

impl OwnedMaterial {
    /// A raw material from RGBA8 pixels. Fully transparent pixels become the
    /// zero key; opaque pixels that would encode as zero become `0x0001`
    /// so a keyed draw keeps them (retail keys the texel index, not its
    /// colour). Images wider than 1024 pixels are cropped to the atlas row.
    pub fn raw_from_rgba(rgba: &[u8], width: u32, height: u32, extra_flags: u16) -> Self {
        let columns = (width as usize).min(ATLAS_STRIDE / 2);
        let rows = height as usize;
        let mut atlas = vec![0u8; ATLAS_STRIDE * rows];
        let mut keyed = false;
        for y in 0..rows {
            for x in 0..columns {
                let at = (y * width as usize + x) * 4;
                let Some(pixel) = rgba.get(at..at + 4) else {
                    continue;
                };
                let word = if pixel[3] == 0 {
                    keyed = true;
                    0
                } else {
                    rgb565(pixel[0], pixel[1], pixel[2]).max(1)
                };
                let byte = y * ATLAS_STRIDE + x * 2;
                atlas[byte..byte + 2].copy_from_slice(&word.to_le_bytes());
            }
        }
        let mut material_flags = flags::RAW | extra_flags;
        if keyed {
            material_flags |= flags::KEYED;
        }
        Self {
            flags: material_flags,
            shade_count: 0,
            width: columns as u16,
            height: rows as u16,
            atlas,
            palette: Arc::from([]),
            palette_start: 0,
        }
    }

    /// An indexed material from texel indices and display-format palette
    /// words (`rows × 16` entries, row 0 first).
    pub fn indexed(
        indices: &[u8],
        width: u32,
        height: u32,
        palette: Vec<u16>,
        material_flags: u16,
    ) -> Self {
        let columns = (width as usize).min(ATLAS_STRIDE);
        let rows = height as usize;
        let mut atlas = vec![0u8; ATLAS_STRIDE * rows];
        for y in 0..rows {
            let source = &indices[y * width as usize..][..columns];
            atlas[y * ATLAS_STRIDE..][..columns].copy_from_slice(source);
        }
        Self {
            flags: material_flags & !flags::RAW,
            shade_count: (palette.len() / 16) as u16,
            width: columns as u16,
            height: rows as u16,
            atlas,
            palette: Arc::from(palette),
            palette_start: 0,
        }
    }

    /// A record registered through [`crate::Renderer::create_native_sprite`].
    pub fn from_native(sprite: &NativeSprite<'_>) -> Self {
        let (width, height) = (usize::from(sprite.width), usize::from(sprite.height));
        let mut atlas = vec![0u8; ATLAS_STRIDE * height];
        let (palette, palette_start) = match &sprite.texels {
            NativeTexels::Indexed { indices, palette } => {
                let columns = width.min(ATLAS_STRIDE);
                for y in 0..height {
                    let row = indices.get(y * width..y * width + columns).unwrap_or(&[]);
                    atlas[y * ATLAS_STRIDE..][..row.len()].copy_from_slice(row);
                }
                (palette.block.clone(), palette.start)
            }
            NativeTexels::Raw { words } => {
                let columns = width.min(ATLAS_STRIDE / 2);
                for y in 0..height {
                    for x in 0..columns {
                        let word = words.get(y * width + x).copied().unwrap_or(0);
                        let at = y * ATLAS_STRIDE + 2 * x;
                        atlas[at..at + 2].copy_from_slice(&word.to_le_bytes());
                    }
                }
                (Arc::from([]), 0)
            }
        };
        Self {
            flags: sprite.flags,
            shade_count: sprite.shade_count,
            width: sprite.width,
            height: sprite.height,
            atlas,
            palette,
            palette_start,
        }
    }

    pub fn view(&self) -> MaterialView<'_> {
        MaterialView {
            flags: self.flags,
            shade_count: self.shade_count,
            width: self.width,
            height: self.height,
            atlas: &self.atlas,
            origin: 0,
            palette: self.palette.get(self.palette_start..).unwrap_or(&[]),
        }
    }
}

/// Materials addressed by [`MaterialId`], with reusable slots.
#[derive(Debug, Default)]
pub struct MaterialStore {
    slots: Vec<Option<OwnedMaterial>>,
    free: Vec<MaterialId>,
}

impl MaterialStore {
    pub fn insert(&mut self, material: OwnedMaterial) -> MaterialId {
        if let Some(id) = self.free.pop() {
            self.slots[id as usize] = Some(material);
            id
        } else {
            self.slots.push(Some(material));
            (self.slots.len() - 1) as MaterialId
        }
    }

    pub fn remove(&mut self, id: MaterialId) -> Option<OwnedMaterial> {
        let removed = self.slots.get_mut(id as usize)?.take();
        if removed.is_some() {
            self.free.push(id);
        }
        removed
    }

    pub fn get(&self, id: MaterialId) -> Option<&OwnedMaterial> {
        self.slots.get(id as usize)?.as_ref()
    }
}

impl MaterialSource for MaterialStore {
    fn material(&self, id: MaterialId) -> MaterialView<'_> {
        self.get(id)
            .unwrap_or_else(|| panic!("material {id} is not bound"))
            .view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_round_trips_through_bit_replication() {
        for pixel in [0x0000u16, 0xFFFF, 0xF800, 0x07E0, 0x001F, 0x1234, 0xABCD] {
            let [r, g, b, _] = rgba8(pixel);
            assert_eq!(rgb565(r, g, b), pixel);
        }
    }

    #[test]
    fn raw_materials_key_transparency_and_keep_opaque_black() {
        let rgba = [0, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0, 255];
        let material = OwnedMaterial::raw_from_rgba(&rgba, 3, 1, 0);
        assert_eq!(material.flags, flags::RAW | flags::KEYED);
        let view = material.view();
        assert_eq!(view.texel_word(0), 0);
        assert_eq!(view.texel_word(1), 1);
        assert_eq!(view.texel_word(2), 0xF800);
    }

    #[test]
    fn store_reuses_freed_slots() {
        let mut store = MaterialStore::default();
        let a = store.insert(OwnedMaterial::raw_from_rgba(&[0; 4], 1, 1, 0));
        let b = store.insert(OwnedMaterial::raw_from_rgba(&[0; 4], 1, 1, 0));
        assert_ne!(a, b);
        assert!(store.remove(a).is_some());
        assert_eq!(
            store.insert(OwnedMaterial::raw_from_rgba(&[0; 4], 1, 1, 0)),
            a
        );
    }
}
