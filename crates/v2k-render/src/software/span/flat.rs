//! Untextured span fillers reached by the fill slots (rows 1, 2, 12, 13, 14).
//!
//! Gouraud channels are 16.16 values whose integer part is twice an 8-bit
//! colour; `(c >> 16) & 0xFF0` keeps five bits for the channel position.
//! The tint rows build a colour from the high nibbles of Graph2D bytes
//! `+0x18..+0x1A` plus the word at `+0x1C`.

use super::super::fixed::{slope_q31, span_reciprocal};
use super::{pixel_x, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filler {
    /// `00473C00` row 1: flat tinted colour.
    Tinted,
    /// `00474220` row 2 and `00476A80` row 14 (identical routines): opaque
    /// Gouraud colour.
    Gouraud,
    /// `00476900` row 12: half the colour plus half the destination.
    HalfAverage,
    /// `00476990` row 13: half the tinted colour saturating-added to the
    /// destination. The routine keeps each pixel's unmasked sum in the
    /// register that held the colour, so from the second pixel on the
    /// previous sum is added instead: the span accumulates toward white.
    HalfTintedAdd,
}

impl Filler {
    pub(super) fn from_retail(address: u32) -> Option<Self> {
        Some(match address {
            0x0047_3C00 => Self::Tinted,
            0x0047_4220 | 0x0047_6A80 => Self::Gouraud,
            0x0047_6900 => Self::HalfAverage,
            0x0047_6990 => Self::HalfTintedAdd,
            _ => return None,
        })
    }

    pub(super) fn fill(self, span: Span<'_, '_, '_>) {
        match self {
            Self::Tinted => tinted(span),
            Self::Gouraud => gouraud(span),
            Self::HalfAverage => half_average(span),
            Self::HalfTintedAdd => half_tinted_add(span),
        }
    }
}

/// `(byte18 & 0xF0) << red + (byte19 & 0xF0) << green + (byte1A & 0xF0) >>
/// blue`, truncated to 16 bits, plus the word at `+0x1C`.
fn tint_colour(span: &Span<'_, '_, '_>) -> u32 {
    let format = span.context.format;
    let [red, green, blue, _] = span.state.word_18.to_le_bytes();
    let green = u32::from(green & 0xF0) << (format.green_shift & 31);
    // `shr bl, cl`: an 8-bit shift, zero once the count reaches eight.
    let blue = u32::from(
        (blue & 0xF0)
            .checked_shr(u32::from(format.blue_shift & 31))
            .unwrap_or(0),
    );
    let red = u32::from(red & 0xF0) << (format.red_shift & 31);
    (green.wrapping_add(blue).wrapping_add(red) & 0xFFFF).wrapping_add(span.state.word_1c & 0xFFFF)
}

fn step_flat(span: &mut Span<'_, '_, '_>) {
    span.left[0] = span.left[0].wrapping_add(span.left[1]);
    span.right[0] = span.right[0].wrapping_add(span.right[1]);
}

/// `00473C00`.
fn tinted(mut span: Span<'_, '_, '_>) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    if width > 0 {
        let colour = tint_colour(&span) as u16;
        let row = span.row() + start as usize;
        span.target.pixels[row..row + width as usize].fill(colour);
    }
    step_flat(&mut span);
}

/// `00474220` / `00476A80`: left to right from the left edge, all steps
/// rounded toward zero.
fn gouraud(span: Span<'_, '_, '_>) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    let reciprocal = span_reciprocal(width as u32);
    let dr = slope_q31(span.right[1].wrapping_sub(span.left[1]), reciprocal);
    let dg = slope_q31(span.right[2].wrapping_sub(span.left[2]), reciprocal);
    let db = slope_q31(span.right[3].wrapping_sub(span.left[3]), reciprocal);
    if width > 0 {
        let format = span.context.format;
        let (mut r, mut g, mut b) = (span.left[1], span.left[2], span.left[3]);
        let mut x = span.row() + start as usize;
        for _ in 0..width {
            let green = (((g as u32) >> 16) & 0xFF0) << (format.green_shift & 31);
            let red = (((r as u32) >> 16) & 0xFF0) << (format.red_shift & 31);
            let blue = (((b as u32) >> 16) & 0xFF0) >> (format.blue_shift & 31);
            span.target.pixels[x] = green.wrapping_add(red).wrapping_add(blue) as u16;
            x += 1;
            r = r.wrapping_add(dr);
            g = g.wrapping_add(dg);
            b = b.wrapping_add(db);
        }
    }
    for edge in [&mut *span.left, &mut *span.right] {
        edge[0] = edge[0].wrapping_add(edge[5]);
        edge[1] = edge[1].wrapping_add(edge[6]);
        edge[2] = edge[2].wrapping_add(edge[7]);
        edge[3] = edge[3].wrapping_add(edge[8]);
    }
}

/// `00476900`: `((dst >> 1) & m) + ((colour >> 1) & m)`, 16-bit.
fn half_average(mut span: Span<'_, '_, '_>) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    let mask = u32::from(span.context.format.halved_pixel_mask());
    let colour = ((span.state.word_1c & 0xFFFF) >> 1) & mask;
    if width > 0 {
        let row = span.row() + start as usize;
        for pixel in &mut span.target.pixels[row..row + width as usize] {
            *pixel = (((u32::from(*pixel) >> 1) & mask).wrapping_add(colour)) as u16;
        }
    }
    step_flat(&mut span);
}

/// `00476990`.
fn half_tinted_add(mut span: Span<'_, '_, '_>) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    let colour = (tint_colour(&span) >> 1) & u32::from(span.context.format.halved_pixel_mask());
    if width > 0 {
        let fog = span.context.fog;
        let row = span.row() + start as usize;
        let mut addend = colour;
        for pixel in &mut span.target.pixels[row..row + width as usize] {
            let sum = (addend & fog.mask).wrapping_add(u32::from(*pixel) & fog.mask);
            *pixel = (sum | ((sum >> 4) & fog.carry).wrapping_mul(15)) as u16;
            addend = sum;
        }
    }
    step_flat(&mut span);
}
