//! Packed fog ramp and colour masks (`FUN_0047CA20`, globals `0x004FBDC0..`).
//!
//! Fogged handlers pass their fog colour to `FUN_0047CA20`, which rebuilds
//! sixteen packed 12-bit colours `colour * k / 15` and the masks the blending
//! fillers use for saturating packed adds. The rebuild is skipped while the
//! colour equals the cached key at `0x004FBE28`. Process memory starts zeroed,
//! so until a fogged primitive supplies a non-zero colour the ramp *and the
//! masks* stay zero; additive fillers read the same masks.

use super::format::PixelFormat;

/// Process-global state written by `FUN_0047CA20`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FogRamp {
    /// `0x004FBE28`: colour the tables were last built for.
    pub key: u32,
    /// `0x004FBDC8..=0x004FBE0C`: ramp entries -1..=16. Entries -1 and 16
    /// duplicate 0 and 15 so dithered lookups may overshoot by one.
    pub ramp: [u32; 18],
    /// `0x004FBDC0`/`0x004FBDC4`: red carry bit and 4-bit field mask.
    pub red: [u32; 2],
    /// `0x004FBE14`/`0x004FBE18`: green carry bit and field mask.
    pub green: [u32; 2],
    /// `0x004FBE20`/`0x004FBE1C`: blue carry bit and field mask.
    pub blue: [u32; 2],
    /// `0x004FBE10`: the three 4-bit field masks.
    pub mask: u32,
    /// `0x004FBE24`: carry bits shifted down by four (lowest bit of each
    /// 4-bit field).
    pub carry: u32,
}

impl Default for FogRamp {
    fn default() -> Self {
        Self {
            key: 0,
            ramp: [0; 18],
            red: [0; 2],
            green: [0; 2],
            blue: [0; 2],
            mask: 0,
            carry: 0,
        }
    }
}

impl FogRamp {
    /// Ramp entry `index` (-1..=16), as `(&0x004FBDCC)[index]`.
    #[inline]
    pub(crate) fn entry(&self, index: i32) -> u32 {
        let slot = index + 1;
        match usize::try_from(slot)
            .ok()
            .and_then(|slot| self.ramp.get(slot))
        {
            Some(&value) => value,
            None => panic!("fog ramp index {index} outside the retail table"),
        }
    }

    /// `FUN_0047CA20`: rebuild the ramp for `colour` unless it is cached.
    pub fn select(&mut self, format: PixelFormat, colour: u32) {
        if self.key == colour {
            return;
        }
        let red_shift = format.red_shift as i32;
        let green_shift = format.green_shift as i32;
        let blue_shift = format.blue_shift as i32;
        let red_step = (colour >> shift(red_shift + 1)) & 0xF8;
        let green_step = (colour >> shift(green_shift + 1)) & 0xF8;
        let blue_step = colour.wrapping_shl(shift(blue_shift - 1)) & 0xF8;
        let mut red = 0i32;
        let mut green = 0i32;
        let mut blue = 0i32;
        for slot in 1..=16 {
            let r = ((red / 15) as u32 & 0xF0).wrapping_shl(shift(red_shift + 1));
            let g = ((green / 15) as u32 & 0xF0).wrapping_shl(shift(green_shift + 1));
            let b = ((blue / 15) & 0xF0) >> shift(blue_shift - 1);
            self.ramp[slot] = g | r | b as u32;
            blue += blue_step as i32;
            green += green_step as i32;
            red += red_step as i32;
        }
        self.ramp[0] = self.ramp[1];
        self.ramp[17] = self.ramp[16];
        self.key = colour;
        self.red = [
            0x100u32.wrapping_shl(shift(red_shift + 1)),
            0xF0u32.wrapping_shl(shift(red_shift + 1)),
        ];
        self.green = [
            0x100u32.wrapping_shl(shift(green_shift + 1)),
            0xF0u32.wrapping_shl(shift(green_shift + 1)),
        ];
        self.blue = [
            0x100u32 >> shift(blue_shift - 1),
            0xF0u32 >> shift(blue_shift - 1),
        ];
        self.mask = self.blue[1] | self.green[1] | self.red[1];
        self.carry = ((self.blue[0] | self.green[0] | self.red[0]) as i32 >> 4) as u32;
    }
}

/// x86 shift counts use the low five bits.
#[inline]
fn shift(count: i32) -> u32 {
    (count & 31) as u32
}
