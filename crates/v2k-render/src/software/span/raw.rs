//! Raw-texel span fillers (table `0x004D5900`).
//!
//! Raw texels are display-format words. These fillers walk left to right
//! from the left edge with every step rounded toward zero. The raw table's
//! half-additive *and* additive rows (16..23, 28..35) use fillers that never
//! read the destination: they write the texel at half intensity. The lit
//! rows add the interpolated vertex colour (edge dwords 3..5) to the texel
//! without saturation; the fogged rows then add a fog-ramp entry with the
//! ramp's per-channel saturation.

use super::super::fixed::{slope_q31, span_reciprocal};
use super::{pixel_x, raw_offset, step_textured, Span};
use crate::software::format::PixelFormat;

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
    /// `00475710` row 6: texel plus interpolated colour.
    Lit,
    /// `004763F0` row 10: keyed [`Self::Lit`].
    KeyedLit,
    /// `00476ED0` row 18 and `00477DD0` rows 30/31: [`Self::Lit`] at half
    /// intensity, opaque.
    HalvedLit,
    /// `00477560` row 22 and `00478040` rows 34/35: keyed
    /// [`Self::HalvedLit`].
    KeyedHalvedLit,
    /// `00476030` row 7: [`Self::Lit`] faded toward the fog colour.
    Fogged,
    /// `00476640` row 11: keyed [`Self::Fogged`].
    KeyedFogged,
    /// `00477140` row 19: [`Self::Fogged`] at half intensity, opaque.
    HalvedFogged,
    /// `004777D0` row 23: keyed [`Self::HalvedFogged`].
    KeyedHalvedFogged,
}

impl Filler {
    pub(super) fn from_retail(address: u32) -> Option<Self> {
        Some(match address {
            0x0047_4FB0 => Self::Plain,
            0x0047_62E0 => Self::KeyedPlain,
            0x0047_6D90 => Self::Halved,
            0x0047_7420 => Self::KeyedHalved,
            0x0047_5710 => Self::Lit,
            0x0047_63F0 => Self::KeyedLit,
            0x0047_6ED0 | 0x0047_7DD0 => Self::HalvedLit,
            0x0047_7560 | 0x0047_8040 => Self::KeyedHalvedLit,
            0x0047_6030 => Self::Fogged,
            0x0047_6640 => Self::KeyedFogged,
            0x0047_7140 => Self::HalvedFogged,
            0x0047_77D0 => Self::KeyedHalvedFogged,
            _ => return None,
        })
    }

    pub(super) fn fill(self, span: Span<'_, '_, '_>) {
        match self {
            Self::Plain | Self::KeyedPlain | Self::Halved | Self::KeyedHalved => plain(span, self),
            Self::Lit => lit(span, false, false),
            Self::KeyedLit => lit(span, true, false),
            Self::HalvedLit => lit(span, false, true),
            Self::KeyedHalvedLit => lit(span, true, true),
            Self::Fogged => fogged(span, false, false),
            Self::KeyedFogged => fogged(span, true, false),
            Self::HalvedFogged => fogged(span, false, true),
            Self::KeyedHalvedFogged => fogged(span, true, true),
        }
    }
}

fn plain(span: Span<'_, '_, '_>, filler: Filler) {
    let keyed = matches!(filler, Filler::KeyedPlain | Filler::KeyedHalved);
    let halved = matches!(filler, Filler::Halved | Filler::KeyedHalved);
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    let reciprocal = span_reciprocal(width as u32);
    let du = slope_q31(span.right[1].wrapping_sub(span.left[1]), reciprocal);
    let dv = slope_q31(span.right[2].wrapping_sub(span.left[2]), reciprocal);
    if width > 0 {
        let material = span.context.material(span.state);
        let half_mask = span.context.format.halved_pixel_mask();
        let mut u = span.left[1];
        let mut v = span.left[2];
        let mut x = span.row() + start as usize;
        for _ in 0..width {
            let texel = material.texel_word(raw_offset(u, v));
            if !(keyed && texel == 0) {
                span.target.pixels[x] = if halved {
                    (texel >> 1) & half_mask
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

/// The interpolated vertex colour as the lit fillers add it to a texel:
/// `((b>>16)&0xFF0) >> blue + ((g>>16)&0xFF0) << green +
/// ((r>>16)&0xFF0) << red`.
#[inline]
fn light(format: PixelFormat, r: i32, g: i32, b: i32) -> u32 {
    let blue = (((b as u32) >> 16) & 0xFF0) >> (format.blue_shift & 31);
    let green = (((g as u32) >> 16) & 0xFF0) << (format.green_shift & 31);
    let red = (((r as u32) >> 16) & 0xFF0) << (format.red_shift & 31);
    blue.wrapping_add(green).wrapping_add(red)
}

/// `00475710` / `004763F0`: `texel + light`, 16-bit wrap. The halved rows
/// (`00476ED0`, `00477560`, `00477DD0`, `00478040`) shift the full 32-bit
/// sum right by one before masking. The shade dword (edge 6) is neither
/// read nor stepped.
fn lit(span: Span<'_, '_, '_>, keyed: bool, halved: bool) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    let reciprocal = span_reciprocal(width as u32);
    let slope =
        |field: usize| slope_q31(span.right[field].wrapping_sub(span.left[field]), reciprocal);
    let (dr, dg, db, du, dv) = (slope(3), slope(4), slope(5), slope(1), slope(2));
    if width > 0 {
        let material = span.context.material(span.state);
        let format = span.context.format;
        let half_mask = u32::from(format.halved_pixel_mask());
        let (mut r, mut g, mut b) = (span.left[3], span.left[4], span.left[5]);
        let (mut u, mut v) = (span.left[1], span.left[2]);
        let mut x = span.row() + start as usize;
        for _ in 0..width {
            let texel = material.texel_word(raw_offset(u, v));
            if !(keyed && texel == 0) {
                let sum = u32::from(texel).wrapping_add(light(format, r, g, b));
                span.target.pixels[x] = if halved {
                    ((sum >> 1) & half_mask) as u16
                } else {
                    sum as u16
                };
            }
            x += 1;
            r = r.wrapping_add(dr);
            g = g.wrapping_add(dg);
            b = b.wrapping_add(db);
            u = u.wrapping_add(du);
            v = v.wrapping_add(dv);
        }
    }
    for edge in [&mut *span.left, &mut *span.right] {
        edge[0] = edge[0].wrapping_add(edge[7]);
        edge[3] = edge[3].wrapping_add(edge[10]);
        edge[4] = edge[4].wrapping_add(edge[11]);
        edge[5] = edge[5].wrapping_add(edge[12]);
        edge[1] = edge[1].wrapping_add(edge[8]);
        edge[2] = edge[2].wrapping_add(edge[9]);
    }
}

/// `00476030` / `00476640` / `00477140` / `004777D0`: the lit sum, masked to
/// the ramp's 4-bit fields, plus the low word of ramp entry `level - 1`
/// (`level = fade >> 20`). Saturation tests each field's lowest bit
/// (`(sum >> 4) & (carry >> 4)`, times 15) rather than the carry: retail's
/// mask is shifted once more than the additive fillers'. The fade walks
/// from the *right* edge's value toward the left one's, stepping on every
/// pixel, drawn or not; u/v and the colour walk from the left edge. The
/// shade dword (edge 6) is neither read nor stepped.
fn fogged(span: Span<'_, '_, '_>, keyed: bool, halved: bool) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    let reciprocal = span_reciprocal(width as u32);
    let slope =
        |field: usize| slope_q31(span.right[field].wrapping_sub(span.left[field]), reciprocal);
    let (dr, dg, db, du, dv) = (slope(3), slope(4), slope(5), slope(1), slope(2));
    let dfade = slope_q31(span.left[7].wrapping_sub(span.right[7]), reciprocal);
    if width > 0 {
        let material = span.context.material(span.state);
        let format = span.context.format;
        let fog = span.context.fog;
        let half_mask = u32::from(format.halved_pixel_mask());
        let (mut r, mut g, mut b) = (span.left[3], span.left[4], span.left[5]);
        let (mut u, mut v) = (span.left[1], span.left[2]);
        let mut fade = span.right[7];
        let mut x = span.row() + start as usize;
        for _ in 0..width {
            let texel = material.texel_word(raw_offset(u, v));
            if !(keyed && texel == 0) {
                let sum = u32::from(texel).wrapping_add(light(format, r, g, b));
                let level = ((fade as u32) >> 20) as i32;
                let fogged = (sum & fog.mask).wrapping_add(fog.entry(level - 1) & 0xFFFF);
                let saturated = fogged | ((fogged >> 4) & (fog.carry >> 4)).wrapping_mul(15);
                span.target.pixels[x] = if halved {
                    ((saturated >> 1) & (fog.mask >> 1) & half_mask) as u16
                } else {
                    (saturated & fog.mask) as u16
                };
            }
            x += 1;
            r = r.wrapping_add(dr);
            g = g.wrapping_add(dg);
            b = b.wrapping_add(db);
            u = u.wrapping_add(du);
            v = v.wrapping_add(dv);
            fade = fade.wrapping_add(dfade);
        }
    }
    for edge in [&mut *span.left, &mut *span.right] {
        edge[0] = edge[0].wrapping_add(edge[8]);
        edge[3] = edge[3].wrapping_add(edge[11]);
        edge[5] = edge[5].wrapping_add(edge[13]);
        edge[4] = edge[4].wrapping_add(edge[12]);
        edge[1] = edge[1].wrapping_add(edge[9]);
        edge[2] = edge[2].wrapping_add(edge[10]);
        edge[7] = edge[7].wrapping_add(edge[15]);
    }
}
