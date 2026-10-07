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

    /// Graph2D `+0x08`: the bit just above each channel (`FUN_00480D10`).
    pub fn carry_bits(self) -> u32 {
        (0x100u32 << ((self.green_shift as u32 + 1) & 31))
            | (0x100u32 << ((self.red_shift as u32 + 1) & 31))
            | (0x100u32 >> ((self.blue_shift as u32).wrapping_sub(1) & 31))
    }

    /// Graph2D `+0x04`: every other bit of the low word.
    pub fn halving_mask(self) -> u32 {
        !self.carry_bits() & 0xFFFF
    }
}
