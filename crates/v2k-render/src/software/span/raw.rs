//! Raw-texel span fillers (table `0x004D5900`).
//!
//! Raw texels are display-format words. These fillers walk left to right
//! from the left edge with every step rounded toward zero. The raw table's
//! half-additive *and* additive rows (16/17/20/21/28/29/32/33) use fillers
//! that never read the destination: they write the texel at half intensity.

use super::super::fixed::{slope_q31, span_reciprocal};
use super::{pixel_x, raw_offset, step_textured, Span};

/// Raw span routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filler {
    /// `00474FB0` rows 4/5: copy texels.
    Plain,
    /// `004762E0` rows 8/9: copy non-zero texels.
    KeyedPlain,
    /// `00476D90` rows 16/17/28/29: texel at half intensity, opaque.
    Halved,
    /// `00477420` rows 20/21/32/33: keyed [`Self::Halved`].
    KeyedHalved,
}

impl Filler {
    pub(super) fn from_retail(address: u32) -> Option<Self> {
        Some(match address {
            0x0047_4FB0 => Self::Plain,
            0x0047_62E0 => Self::KeyedPlain,
            0x0047_6D90 => Self::Halved,
            0x0047_7420 => Self::KeyedHalved,
            _ => return None,
        })
    }

    pub(super) fn fill(self, span: Span<'_, '_, '_>) {
        let keyed = matches!(self, Self::KeyedPlain | Self::KeyedHalved);
        let halved = matches!(self, Self::Halved | Self::KeyedHalved);
        let start = pixel_x(span.left[0]);
        let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
        let reciprocal = span_reciprocal(width as u32);
        let du = slope_q31(span.right[1].wrapping_sub(span.left[1]), reciprocal);
        let dv = slope_q31(span.right[2].wrapping_sub(span.left[2]), reciprocal);
        if width > 0 {
            let material = span.context.material(span.state);
            let half_mask = span.context.halving_mask() & 0xFFFF;
            let mut u = span.left[1];
            let mut v = span.left[2];
            let mut x = span.row() + start as usize;
            for _ in 0..width {
                let texel = material.texel_word(raw_offset(u, v));
                if !(keyed && texel == 0) {
                    span.target.pixels[x] = if halved {
                        ((u32::from(texel) >> 1) & half_mask) as u16
                    } else {
                        texel
                    };
                }
                x += 1;
                u = u.wrapping_add(du);
                v = v.wrapping_add(dv);
            }
        }
        step_textured(span.left);
        step_textured(span.right);
    }
}
