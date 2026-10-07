//! Indexed-texel span fillers (table `0x004D55A0`).
//!
//! These fillers start at the right edge and write leftward, so the
//! rightmost pixel takes the right edge's interpolants and the left edge's
//! values are never sampled exactly. The u step is not rounded toward zero;
//! every other step is. Shaded fillers dither the palette row (and fog
//! level) with generators seeded from Graph2D `+0x10C8`, and each keeps its
//! generator differently, so every variant is reproduced on its own.

use super::super::fixed::{mul_q31, slope_q31, span_reciprocal};
use super::super::material::{flags, MaterialView};
use super::{
    indexed_offset, indexed_offset_signed, pixel_x, saturate, step_textured, step_textured_shade,
    step_textured_shade_fade, Span,
};

/// Indexed span routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filler {
    /// One palette row for the span: `004782B0` (row 4), `004783E0` (5),
    /// `004788D0` (8), `00478A20` (9), `00478F60` (16), `004790C0` (17),
    /// `004796D0` (20), `00479840` (21), `00479E00` (28), `00479F60` (29),
    /// `0047A270` (32), `0047A3E0` (33).
    Unshaded {
        row: RowSource,
        keyed: bool,
        blend: Blend,
    },
    /// `00478510` row 6: interpolated row, 32-bit dither register.
    Shaded,
    /// `00478B60` row 10: keyed interpolated row, 16-bit dither register.
    KeyedShaded,
    /// `00479220` row 18: half-additive interpolated row, one texel per
    /// aligned pixel pair.
    HalfShadedPairs,
    /// `004799B0` row 22: keyed half-additive interpolated row.
    KeyedHalfShaded,
    /// `0047A0B0` rows 30/31 and `0047A540` rows 34/35: additive
    /// interpolated row with the `0x43FD` generator.
    AddShaded { keyed: bool },
    /// `004786A0` (7), `00478D00` (11), `00479490` (19), `00479B90` (23):
    /// interpolated row and fog level.
    ShadedFog { keyed: bool, blend: Blend },
}

/// Where an unshaded filler takes its palette row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowSource {
    /// Row 0, or row 28 with material flag 0x04.
    Material,
    /// Graph2D byte `+0x1B`.
    RowByte,
}

/// Composition with the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blend {
    Replace,
    /// Unshaded: texel plus `(destination >> 1) & mask`, a 16-bit add.
    /// Fogged: `(fogged(2 * texel) + (destination & mask)) >> 1`.
    Half,
    /// Per-channel saturating add of the 12-bit masked colours.
    Add,
}

impl Filler {
    pub(super) fn from_retail(address: u32) -> Option<Self> {
        use Blend::*;
        use RowSource::*;
        let unshaded = |row, keyed, blend| Self::Unshaded { row, keyed, blend };
        Some(match address {
            0x0047_82B0 => unshaded(Material, false, Replace),
            0x0047_83E0 => unshaded(RowByte, false, Replace),
            0x0047_88D0 => unshaded(Material, true, Replace),
            0x0047_8A20 => unshaded(RowByte, true, Replace),
            0x0047_8F60 => unshaded(Material, false, Half),
            0x0047_90C0 => unshaded(RowByte, false, Half),
            0x0047_96D0 => unshaded(Material, true, Half),
            0x0047_9840 => unshaded(RowByte, true, Half),
            0x0047_9E00 => unshaded(Material, false, Add),
            0x0047_9F60 => unshaded(RowByte, false, Add),
            0x0047_A270 => unshaded(Material, true, Add),
            0x0047_A3E0 => unshaded(RowByte, true, Add),
            0x0047_8510 => Self::Shaded,
            0x0047_8B60 => Self::KeyedShaded,
            0x0047_9220 => Self::HalfShadedPairs,
            0x0047_99B0 => Self::KeyedHalfShaded,
            0x0047_A0B0 => Self::AddShaded { keyed: false },
            0x0047_A540 => Self::AddShaded { keyed: true },
            0x0047_86A0 => Self::ShadedFog {
                keyed: false,
                blend: Replace,
            },
            0x0047_8D00 => Self::ShadedFog {
                keyed: true,
                blend: Replace,
            },
            0x0047_9490 => Self::ShadedFog {
                keyed: false,
                blend: Half,
            },
            0x0047_9B90 => Self::ShadedFog {
                keyed: true,
                blend: Half,
            },
            _ => return None,
        })
    }

    pub(super) fn fill(self, span: Span<'_, '_, '_>) {
        match self {
            Self::Unshaded { row, keyed, blend } => unshaded(span, row, keyed, blend),
            Self::Shaded => shaded_wide_dither(span),
            Self::KeyedShaded => keyed_shaded(span),
            Self::HalfShadedPairs => half_shaded_pairs(span),
            Self::KeyedHalfShaded => keyed_half_shaded(span),
            Self::AddShaded { keyed } => add_shaded(span, keyed),
            Self::ShadedFog { keyed, blend } => shaded_fog(span, keyed, blend),
        }
    }
}

/// Interpolants every right-to-left indexed filler sets up: the span
/// reciprocal and the u (unrounded) and v (rounded) steps toward the left
/// edge.
struct Walk {
    width: u32,
    reciprocal: i32,
    du: i32,
    dv: i32,
}

fn walk(span: &Span<'_, '_, '_>) -> Walk {
    let width = pixel_x(span.right[0]).wrapping_sub(pixel_x(span.left[0]));
    let reciprocal = span_reciprocal(width);
    Walk {
        width,
        reciprocal,
        du: mul_q31(span.left[1].wrapping_sub(span.right[1]), reciprocal),
        dv: slope_q31(span.left[2].wrapping_sub(span.right[2]), reciprocal),
    }
}

/// Word index of the pixel just right of the span.
fn span_end(span: &Span<'_, '_, '_>) -> usize {
    span.row() + pixel_x(span.right[0]) as usize
}

/// Palette entry offset of the fixed row an unshaded filler uses.
fn fixed_row(row: RowSource, material: &MaterialView<'_>, row_byte: u8) -> usize {
    match row {
        RowSource::RowByte => usize::from(row_byte) * 16,
        RowSource::Material if material.flags & flags::ROW_28 != 0 => 0x380 / 2,
        RowSource::Material => 0,
    }
}

fn unshaded(span: Span<'_, '_, '_>, row: RowSource, keyed: bool, blend: Blend) {
    let Walk { width, du, dv, .. } = walk(&span);
    if width != 0 {
        let material = span.context.material(span.state);
        let row = fixed_row(row, &material, (span.state.word_18 >> 24) as u8);
        let half_mask = u32::from(span.context.format.halved_pixel_mask() & 0x7FFF);
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span_end(&span);
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

/// Shade setup shared by the interpolated-row fillers.
fn shade_step(span: &Span<'_, '_, '_>, reciprocal: i32) -> i32 {
    slope_q31(span.left[6].wrapping_sub(span.right[6]), reciprocal)
}

/// `00478510`: the generator is the whole 32-bit register, seeded with the
/// left edge pointer's high half above the stored low word; the row noise
/// is the register's high half.
fn shaded_wide_dither(span: Span<'_, '_, '_>) {
    let Walk {
        width,
        reciprocal,
        du,
        dv,
    } = walk(&span);
    if width != 0 {
        let dshade = shade_step(&span, reciprocal);
        let material = span.context.material(span.state);
        let mut register = (span.left_address & 0xFFFF_0000) | u32::from(span.state.dither);
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span_end(&span);
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
/// noise is the new state, stepped once per pixel, drawn or not.
fn keyed_shaded(span: Span<'_, '_, '_>) {
    let Walk {
        width,
        reciprocal,
        du,
        dv,
    } = walk(&span);
    if width != 0 {
        let dshade = shade_step(&span, reciprocal);
        let material = span.context.material(span.state);
        let mut state = span.state.dither;
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span_end(&span);
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

/// Palette index of the half-additive shaded fillers: their "noise"
/// register still holds the texel column `u >> 16`, so the row is
/// `((u >> 16) + shade) >> 16`.
fn column_noise_index(u: i32, shade: i32, texel: u8) -> u32 {
    let row = ((((u as u32) >> 16) as i32).wrapping_add(shade) >> 16) << 4;
    (row as u32).wrapping_add(u32::from(texel))
}

/// `00479220`: aligned pixel pairs share one texel sampled at the right
/// pixel's coordinates, and the interpolants advance twice per pair. A
/// leading odd pixel and a trailing one are drawn singly; a two-pixel span
/// whose left pixel sits at an address of 2 mod 4 draws nothing. The
/// surface base is assumed dword aligned, as DirectDraw surfaces are.
fn half_shaded_pairs(span: Span<'_, '_, '_>) {
    let Walk {
        width,
        reciprocal,
        du,
        dv,
    } = walk(&span);
    if width != 0 {
        let dshade = shade_step(&span, reciprocal);
        let material = span.context.material(span.state);
        let mask = u32::from(span.context.format.halved_pixel_mask() & 0x7FFF);
        let halve = |pixel: u16| ((u32::from(pixel) >> 1) & mask) as u16;
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        // Byte addresses relative to the dword-aligned surface base.
        let end = span_end(&span) * 2;
        let stop = end - 2 * width as usize + 2;
        let aligned = (stop & !3) + 4;
        let mut esp = end;
        let mut sample = |steps: usize| {
            let texel = material.texel_byte(indexed_offset(u, v));
            let index = column_noise_index(u, shade, texel);
            for _ in 0..steps {
                v = v.wrapping_add(dv);
                u = u.wrapping_add(du);
                shade = shade.wrapping_add(dshade);
            }
            material.palette_word(0, index)
        };
        let pixels = &mut *span.target.pixels;
        if esp >= aligned {
            if esp & 2 != 0 {
                let source = sample(1);
                let at = esp / 2 - 1;
                pixels[at] = source.wrapping_add(halve(pixels[at]));
                esp -= 2;
            }
            loop {
                let source = sample(2);
                let right = esp / 2 - 1;
                pixels[right - 1] = source.wrapping_add(halve(pixels[right - 1]));
                pixels[right] = source.wrapping_add(halve(pixels[right]));
                let last = esp == aligned;
                esp -= 4;
                if last {
                    break;
                }
            }
        }
        if esp == stop {
            let source = sample(0);
            let at = esp / 2 - 1;
            pixels[at] = source.wrapping_add(halve(pixels[at]));
        }
    }
    step_textured_shade(span.left);
    step_textured_shade(span.right);
}

/// `004799B0`: per pixel, keyed. The generator register is overwritten by
/// the halved destination (`mov si, [esp-2]; shr esi, 1`), so the word
/// stored back to Graph2D `+0x10C8` is the last pixel's halved destination;
/// after a keyed-out last pixel the register's high half shifts into
/// bit 15.
fn keyed_half_shaded(span: Span<'_, '_, '_>) {
    let Walk {
        width,
        reciprocal,
        du,
        dv,
    } = walk(&span);
    if width != 0 {
        let dshade = shade_step(&span, reciprocal);
        let material = span.context.material(span.state);
        let mask = u32::from(span.context.format.halved_pixel_mask() & 0x7FFF);
        let mut register = (width << 16) | u32::from(span.state.dither);
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span_end(&span);
        for _ in 0..width {
            x -= 1;
            let destination = span.target.pixels[x];
            register = ((register & 0xFFFF_0000) | u32::from(destination)) >> 1;
            let texel = material.texel_byte(indexed_offset(u, v));
            v = v.wrapping_add(dv);
            let index = column_noise_index(u, shade, texel);
            u = u.wrapping_add(du);
            shade = shade.wrapping_add(dshade);
            if texel != 0 {
                register &= mask;
                let source = material.palette_word(0, index);
                span.target.pixels[x] = source.wrapping_add(register as u16);
            }
        }
        span.state.dither = register as u16;
    }
    step_textured_shade(span.left);
    step_textured_shade(span.right);
}

/// `0047A0B0` / `0047A540`: the stored dither word is multiplied by 0x43FD
/// (16-bit) for every drawn pixel and its signed value dithers the row; the
/// saturating sum is written unmasked. Texel rows use `(v >> 16) << 11`
/// with an arithmetic shift and columns `u >> 16` with a logical one.
fn add_shaded(span: Span<'_, '_, '_>, keyed: bool) {
    let Walk {
        width,
        reciprocal,
        du,
        dv,
    } = walk(&span);
    if width as i32 > 0 {
        let dshade = shade_step(&span, reciprocal);
        let material = span.context.material(span.state);
        let fog = span.context.fog;
        let mut shade = span.right[6];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span_end(&span);
        for _ in 0..width {
            x -= 1;
            let offset = ((v >> 16) << 11) as u32;
            let texel = material.texel_byte(offset.wrapping_add((u as u32) >> 16));
            if !(keyed && texel == 0) {
                span.state.dither = span.state.dither.wrapping_mul(0x43FD);
                let noise = i32::from(span.state.dither as i16);
                let row = (shade.wrapping_add(noise) >> 12) & !0xF;
                let source = u32::from(
                    material.palette_word(0, (row as u32).wrapping_add(u32::from(texel))),
                );
                let destination = u32::from(span.target.pixels[x]);
                let sum = (source & fog.mask).wrapping_add(destination & fog.mask);
                span.target.pixels[x] =
                    (sum | (((sum as i32 >> 4) as u32) & fog.carry).wrapping_mul(15)) as u16;
            }
            shade = shade.wrapping_add(dshade);
            v = v.wrapping_add(dv);
            u = u.wrapping_add(du);
        }
    }
    step_textured_shade(span.left);
    step_textured_shade(span.right);
}

/// `004786A0` / `00478D00` / `00479490` / `00479B90`: the generator sits in
/// the register's high half (the low half counts pixels). One step before
/// the span; per pixel the current state dithers the row and the next state
/// the fog level; a drawn pixel steps the generator twice, a keyed-out one
/// once. The fog colour is added from the `FUN_0047CA20` ramp with
/// per-channel saturation. u/v use arithmetic shifts.
fn shaded_fog(span: Span<'_, '_, '_>, keyed: bool, blend: Blend) {
    let Walk {
        width,
        reciprocal,
        du,
        dv,
    } = walk(&span);
    if width != 0 {
        let dshade = shade_step(&span, reciprocal);
        let dfade = slope_q31(span.left[7].wrapping_sub(span.right[7]), reciprocal);
        let material = span.context.material(span.state);
        let fog = span.context.fog;
        let mut state = span.state.dither.wrapping_mul(9);
        let mut shade = span.right[6];
        let mut fade = span.right[7];
        let mut u = span.right[1];
        let mut v = span.right[2];
        let mut x = span_end(&span);
        for remaining in (1..=width).rev() {
            let row_noise = i32::from(state as i16);
            let offset = indexed_offset_signed(u, v);
            u = u.wrapping_add(du);
            v = v.wrapping_add(dv);
            state = state.wrapping_mul(9);
            // `sar` of {state:16, remaining:16} by 12.
            let fog_noise = (((u32::from(state) << 16) | remaining) as i32) >> 12;
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
            let source = if blend == Blend::Half {
                source << 1
            } else {
                source
            };
            let fogged = saturate((source & fog.mask).wrapping_add(fog.entry(level)), fog);
            span.target.pixels[x] = if blend == Blend::Half {
                ((u32::from(fogged) + (u32::from(span.target.pixels[x]) & fog.mask)) >> 1) as u16
            } else {
                fogged
            };
            state = state.wrapping_mul(9);
        }
        span.state.dither = state;
    }
    step_textured_shade_fade(span.left);
    step_textured_shade_fade(span.right);
}
