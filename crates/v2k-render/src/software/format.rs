//! DirectDraw surface channel parameters as the software raster uses them.

/// Channel placement words at DirectDraw device `+0x20/+0x24/+0x28`.
///
/// Fillers extract an 8-bit channel as `(pixel >> (red+1)) & 0xF8`,
/// `(pixel >> (green+1)) & 0xF8` and `(pixel << (blue-1)) & 0xF8`, and pack
/// it back with the opposite shifts. Packing therefore keeps five bits per
/// channel: computed RGB565 pixels always have green's lowest bit clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelFormat {
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

impl PixelFormat {
    /// The retail 16-bpp surface: R 0xF800, G 0x07E0, B 0x001F.
    pub const RGB565: Self = Self {
        red_shift: 7,
        green_shift: 2,
        blue_shift: 4,
    };

    /// Mask for a pixel already shifted right by one: clears the bits that
    /// green's and red's lowest bits land in, `!(0x80 << (green+1) |
    /// 0x80 >> (blue-1))` (RGB565: `0xFBEF`). Every halving filler and the
    /// sprite blit build it this way rather than masking before the shift.
    pub fn halved_pixel_mask(self) -> u16 {
        !((0x80u32 << ((u32::from(self.green_shift) + 1) & 31))
            | (0x80u32 >> (u32::from(self.blue_shift).wrapping_sub(1) & 31))) as u16
    }
}
