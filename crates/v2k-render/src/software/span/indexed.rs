//! Indexed-texel span fillers (table `0x004D55A0`).
//!
//! These fillers start at the right edge and `push` pixels leftward, so the
//! rightmost pixel takes the right edge's interpolants and the left edge's
//! values are never sampled exactly. Unless noted, the u step is not
//! rounded toward zero while every other step is. Shaded fillers dither the
//! palette row (and fog level) with a ×9 generator seeded from Graph2D
//! `+0x10C8`; each keeps the generator in a different register layout, which
//! changes the noise sequence, so every variant is reproduced separately.

use super::super::fixed::{mul_q31, slope_q31, span_reciprocal};
use super::super::material::{flags, MaterialView};
use super::{
    indexed_offset, indexed_offset_signed, pixel_x, saturate, step_textured, step_textured_shade,
    step_textured_shade_fade, Span,
};

/// Indexed span routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filler {
    /// `004782B0` row 4: palette row 0, or 28 with flag 0x04.
    Plain,
    /// `004783E0` row 5: palette row from Graph2D byte `+0x1B`.
    RowByte,
    /// `00478510` row 6: interpolated row, 32-bit dither register.
    Shaded,
    /// `004786A0` row 7: interpolated row and fog level.
    ShadedFog,
    /// `004788D0` row 8: keyed [`Self::Plain`].
    KeyedPlain,
    /// `00478A20` row 9: keyed [`Self::RowByte`].
    KeyedRowByte,
    /// `00478B60` row 10: keyed interpolated row, 16-bit dither register.
    KeyedShaded,
    /// `00478D00` row 11: keyed interpolated row and fog level.
    KeyedShadedFog,
    /// `00478F60` row 16: plain texel plus half the destination.
    HalfPlain,
    /// `004796D0` row 20: keyed [`Self::HalfPlain`].
    KeyedHalfPlain,
    /// `00479E00` row 28: plain texel saturating-added to the destination.
    AddPlain,
    /// `0047A270` row 32: keyed [`Self::AddPlain`].
    KeyedAddPlain,
}

impl Filler {
    pub(super) fn from_retail(address: u32) -> Option<Self> {
        Some(match address {
            0x0047_82B0 => Self::Plain,
            0x0047_83E0 => Self::RowByte,
            0x0047_8510 => Self::Shaded,
            0x0047_86A0 => Self::ShadedFog,
            0x0047_88D0 => Self::KeyedPlain,
            0x0047_8A20 => Self::KeyedRowByte,
            0x0047_8B60 => Self::KeyedShaded,
            0x0047_8D00 => Self::KeyedShadedFog,
            0x0047_8F60 => Self::HalfPlain,
            0x0047_96D0 => Self::KeyedHalfPlain,
            0x0047_9E00 => Self::AddPlain,
            0x0047_A270 => Self::KeyedAddPlain,
            _ => return None,
        })
    }

    pub(super) fn fill(self, span: Span<'_, '_, '_>) {
        match self {
            Self::Plain | Self::KeyedPlain | Self::RowByte | Self::KeyedRowByte => {
                unshaded(span, self, Blend::Replace)
            }
            Self::HalfPlain | Self::KeyedHalfPlain => unshaded(span, self, Blend::Half),
            Self::AddPlain | Self::KeyedAddPlain => unshaded(span, self, Blend::Add),
            Self::Shaded => shaded_wide_dither(span),
            Self::KeyedShaded => keyed_shaded(span),
            Self::ShadedFog => shaded_fog(span, false),
            Self::KeyedShadedFog => shaded_fog(span, true),
        }
    }

    fn keyed(self) -> bool {
        matches!(
            self,
            Self::KeyedPlain
                | Self::KeyedRowByte
                | Self::KeyedShaded
                | Self::KeyedShadedFog
                | Self::KeyedHalfPlain
                | Self::KeyedAddPlain
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Blend {
    Replace,
    /// Texel plus `(destination >> 1) & mask`, a plain 16-bit add.
    Half,
    /// Per-channel saturating add of the 12-bit masked colours.
    Add,
}

/// Palette entry offset of the fixed row an unshaded filler uses.
fn fixed_row(filler: Filler, material: &MaterialView<'_>, row_byte: u8) -> usize {
    match filler {
        Filler::RowByte | Filler::KeyedRowByte => usize::from(row_byte) * 16,
        _ if material.flags & flags::ROW_28 != 0 => 0x380 / 2,
        _ => 0,
    }
}

/// Rows 4/5/8/9/16/20/28/32 (and their half/add/key variants): one palette
/// row for the whole span.
fn unshaded(span: Span<'_, '_, '_>, filler: Filler, blend: Blend) {
    let end = pixel_x(span.right[0]);
    let width = end.wrapping_sub(pixel_x(span.left[0]));
    if width != 0 {
        let reciprocal = span_reciprocal(width);
        let du = mul_q31(span.left[1].wrapping_sub(span.right[1]), reciprocal);
        let dv = slope_q31(span.left[2].wrapping_sub(span.right[2]), reciprocal);
        let material = span.context.material(span.state);
        let row = fixed_row(filler, &material, (span.state.word_18 >> 24) as u8);
        let half_mask = span.context.halving_mask() & 0x7FFF;
        let keyed = filler.keyed();
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span.row() + end as usize;
        for _ in 0..width {
            let texel = material.texel_byte(indexed_offset(u, v));
            v = v.wrapping_add(dv);
            u = u.wrapping_add(du);
            x -= 1;
            if keyed && texel == 0 {
                continue;
            }
            let source = material.palette_word(row, u32::from(texel));
            let pixel = &mut span.target.pixels[x];
            *pixel = match blend {
                Blend::Replace => source,
                Blend::Half => source.wrapping_add(((u32::from(*pixel) >> 1) & half_mask) as u16),
                Blend::Add => span
                    .context
                    .saturating_add(u32::from(source), u32::from(*pixel)),
            };
        }
    }
    step_textured(span.left);
    step_textured(span.right);
}

/// `00478510`: the generator is the whole 32-bit register, seeded with the
/// left edge pointer's high half above the stored low word; the row noise
/// is the register's high half.
fn shaded_wide_dither(span: Span<'_, '_, '_>) {
    let end = pixel_x(span.right[0]);
    let width = end.wrapping_sub(pixel_x(span.left[0]));
    if width != 0 {
        let reciprocal = span_reciprocal(width);
        let dshade = slope_q31(span.left[6].wrapping_sub(span.right[6]), reciprocal);
        let du = mul_q31(span.left[1].wrapping_sub(span.right[1]), reciprocal);
        let dv = slope_q31(span.left[2].wrapping_sub(span.right[2]), reciprocal);
        let material = span.context.material(span.state);
        let mut register = (span.left_address & 0xFFFF_0000) | u32::from(span.state.dither);
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span.row() + end as usize;
        for _ in 0..width {
            register = register.wrapping_mul(9);
            let texel = material.texel_byte(indexed_offset(u, v));
            v = v.wrapping_add(dv);
            let row = (((register as i32) >> 16).wrapping_add(shade) >> 16) << 4;
            u = u.wrapping_add(du);
            shade = shade.wrapping_add(dshade);
            x -= 1;
            span.target.pixels[x] =
                material.palette_word(0, (row as u32).wrapping_add(u32::from(texel)));
        }
        span.state.dither = register as u16;
    }
    step_textured_shade(span.left);
    step_textured_shade(span.right);
}

/// `00478B60`: a 16-bit generator in the register's low half; the row
/// noise is the new 16-bit state, advanced once per pixel, skipped or not.
fn keyed_shaded(span: Span<'_, '_, '_>) {
    let end = pixel_x(span.right[0]);
    let width = end.wrapping_sub(pixel_x(span.left[0]));
    if width != 0 {
        let reciprocal = span_reciprocal(width);
        let dshade = slope_q31(span.left[6].wrapping_sub(span.right[6]), reciprocal);
        let du = mul_q31(span.left[1].wrapping_sub(span.right[1]), reciprocal);
        let dv = slope_q31(span.left[2].wrapping_sub(span.right[2]), reciprocal);
        let material = span.context.material(span.state);
        let mut state = span.state.dither;
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span.row() + end as usize;
        for _ in 0..width {
            state = state.wrapping_mul(9);
            let texel = material.texel_byte(indexed_offset(u, v));
            v = v.wrapping_add(dv);
            let row = (i32::from(state as i16).wrapping_add(shade) >> 16) << 4;
            u = u.wrapping_add(du);
            shade = shade.wrapping_add(dshade);
            x -= 1;
            if texel != 0 {
                span.target.pixels[x] =
                    material.palette_word(0, (row as u32).wrapping_add(u32::from(texel)));
            }
        }
        span.state.dither = state;
    }
    step_textured_shade(span.left);
    step_textured_shade(span.right);
}

/// `004786A0` / `00478D00`: the generator sits in the register's high half
/// (the low half counts pixels). One step before the span, then per pixel
/// the current state dithers the row and the next state the fog level; a
/// drawn pixel steps the generator twice, a keyed-out one once. The fog
/// colour is added from the `FUN_0047CA20` ramp with per-channel
/// saturation. u/v use arithmetic shifts.
fn shaded_fog(span: Span<'_, '_, '_>, keyed: bool) {
    let end = pixel_x(span.right[0]);
    let width = end.wrapping_sub(pixel_x(span.left[0]));
    if width != 0 {
        let reciprocal = span_reciprocal(width);
        let dshade = slope_q31(span.left[6].wrapping_sub(span.right[6]), reciprocal);
        let dfade = slope_q31(span.left[7].wrapping_sub(span.right[7]), reciprocal);
        let du = mul_q31(span.left[1].wrapping_sub(span.right[1]), reciprocal);
        let dv = slope_q31(span.left[2].wrapping_sub(span.right[2]), reciprocal);
        let material = span.context.material(span.state);
        let fog = span.context.fog;
        let mut state = span.state.dither.wrapping_mul(9);
        let mut shade = span.right[6];
        let mut fade = span.right[7];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span.row() + end as usize;
        for remaining in (1..=width).rev() {
            let row_noise = i32::from(state as i16);
            let offset = indexed_offset_signed(u, v);
            u = u.wrapping_add(du);
            v = v.wrapping_add(dv);
            state = state.wrapping_mul(9);
            // `sar` of {state:16, remaining:16} by 12.
            let fog_noise = ((u32::from(state) << 16 | remaining) as i32) >> 12;
            let row = (row_noise.wrapping_add(shade) >> 12) & 0xFF0;
            let texel = material.texel_byte(offset);
            x -= 1;
            if keyed && texel == 0 {
                fade = fade.wrapping_add(dfade);
                shade = shade.wrapping_add(dshade);
                continue;
            }
            let level = fog_noise.wrapping_add(fade) >> 20;
            fade = fade.wrapping_add(dfade);
            shade = shade.wrapping_add(dshade);
            let source =
                u32::from(material.palette_word(0, u32::from(texel).wrapping_add(row as u32)));
            let sum = (source & fog.mask).wrapping_add(fog.entry(level));
            span.target.pixels[x] = saturate(sum, fog);
            state = state.wrapping_mul(9);
        }
        span.state.dither = state;
    }
    step_textured_shade_fade(span.left);
    step_textured_shade_fade(span.right);
}
