//! Section-3 sprite records as the software raster reads them.
//!
//! Retail binds a pointer to the 28-byte record at Graph2D `+0x14`; fillers
//! read its flags (`+0x04`), atlas pointer (`+0x08`, 2048-byte rows),
//! display-format palette pointer (`+0x0C`) and pixel size (`+0x10/+0x12`).
//! Texel and palette reads are not bounded by the sprite's own rectangle:
//! interpolation overshoot samples neighbouring atlas bytes and palette
//! entries, so a view exposes the whole atlas page and palette block from
//! the record's origin onward.

/// Row stride of every Section-3 atlas page, in bytes.
pub const ATLAS_STRIDE: usize = 2048;

/// Caller-assigned identity of a bound material.
pub type MaterialId = u32;

/// Render flag bits at record `+0x04`.
pub mod flags {
    /// Texel value zero is transparent.
    pub const KEYED: u16 = 0x01;
    /// Texels are raw display-format words, not palette indices.
    pub const RAW: u16 = 0x02;
    /// Unshaded indexed draws use palette row 28 (`+0x380` bytes).
    pub const ROW_28: u16 = 0x04;
    /// Half-additive composition.
    pub const HALF_ADDITIVE: u16 = 0x08;
    /// Additive composition.
    pub const ADDITIVE: u16 = 0x10;
}

/// One bound sprite record.
#[derive(Debug, Clone, Copy)]
pub struct MaterialView<'a> {
    pub flags: u16,
    pub shade_count: u16,
    pub width: u16,
    pub height: u16,
    /// Atlas bytes; the record's texel pointer is `atlas[origin]`.
    pub atlas: &'a [u8],
    pub origin: usize,
    /// Display-format palette words starting at the record's palette
    /// pointer, followed by whatever the retail block holds after it.
    pub palette: &'a [u16],
}

impl MaterialView<'_> {
    /// Indexed texel byte at `offset` bytes from the record's texel pointer.
    /// A read outside the supplied atlas returns zero: retail would read
    /// unrelated heap bytes there.
    #[inline]
    pub(crate) fn texel_byte(&self, offset: u32) -> u8 {
        self.origin
            .checked_add(offset as usize)
            .and_then(|index| self.atlas.get(index))
            .copied()
            .unwrap_or(0)
    }

    /// Raw texel word at `offset` words from the record's texel pointer.
    #[inline]
    pub(crate) fn texel_word(&self, offset: u32) -> u16 {
        let byte = self.origin.checked_add((offset as usize).wrapping_mul(2));
        match byte {
            Some(index) if index + 1 < self.atlas.len() => {
                u16::from_le_bytes([self.atlas[index], self.atlas[index + 1]])
            }
            _ => 0,
        }
    }

    /// Palette word `index` entries from `base_entries`.
    #[inline]
    pub(crate) fn palette_word(&self, base_entries: usize, index: u32) -> u16 {
        base_entries
            .checked_add(index as usize)
            .and_then(|entry| self.palette.get(entry))
            .copied()
            .unwrap_or(0)
    }
}

/// Resolves the material identities carried by packets.
pub trait MaterialSource {
    fn material(&self, id: MaterialId) -> MaterialView<'_>;
}

impl MaterialSource for [MaterialView<'_>] {
    fn material(&self, id: MaterialId) -> MaterialView<'_> {
        self[id as usize]
    }
}

impl MaterialSource for Vec<MaterialView<'_>> {
    fn material(&self, id: MaterialId) -> MaterialView<'_> {
        self[id as usize]
    }
}
