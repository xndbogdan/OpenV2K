//! Span fillers: routine 0 of each span row.
//!
//! A filler draws one scanline between the left (smaller-x) and right edge
//! records, then advances both edges by one line. Fillers differ in pixel
//! direction (indexed fillers write right-to-left through `push`), in which
//! interpolant slopes are rounded toward zero, in which shifts are signed,
//! and in blending. Each implementation names the retail routine it
//! reproduces; see `docs/re/SOFTWARE_RASTER.md`.

mod flat;
mod indexed;
mod raw;

use super::fog::FogRamp;
use super::format::PixelFormat;
use super::material::{MaterialSource, MaterialView};
use super::state::{Edge, RasterState};

/// The locked surface rows a filler writes.
pub(crate) struct SpanTarget<'a> {
    pub pixels: &'a mut [u16],
    /// Surface pitch in bytes (DirectDraw device `+0x1C`).
    pub pitch: usize,
    /// Surface size (DirectDraw device `+0x00/+0x04`).
    pub width: u32,
    pub height: u32,
}

impl SpanTarget<'_> {
    #[inline]
    fn row_start(&self, line: i32) -> usize {
        line as usize * (self.pitch / 2)
    }
}

/// Inputs fillers read besides the Graph2D block.
pub(crate) struct SpanContext<'a> {
    pub materials: &'a dyn MaterialSource,
    pub format: PixelFormat,
    pub fog: &'a FogRamp,
}

impl SpanContext<'_> {
    fn material(&self, state: &RasterState) -> MaterialView<'_> {
        let id = state
            .material
            .expect("textured span filler without a bound material");
        self.materials.material(id)
    }

    /// Per-channel saturating add of two packed 12-bit colours, using the
    /// masks `FUN_0047CA20` last built.
    #[inline]
    fn saturating_add(&self, a: u32, b: u32) -> u16 {
        let sum = (a & self.fog.mask).wrapping_add(b & self.fog.mask);
        saturate(sum, self.fog)
    }
}

/// `(sum & mask) | ((sum >> 4) & carry) * 15`: a 4-bit field that overflowed
/// into the bit above it becomes all ones.
#[inline]
fn saturate(sum: u32, fog: &FogRamp) -> u16 {
    ((sum & fog.mask) | ((sum >> 4) & fog.carry).wrapping_mul(15)) as u16
}

/// x in pixels: `shr reg, 16` of a 16.16 edge x.
#[inline]
fn pixel_x(x: i32) -> u32 {
    (x as u32) >> 16
}

/// Texel byte offset of 16.16 `u`/`v` in a 2048-byte-stride atlas, with the
/// unsigned shifts most fillers use.
#[inline]
fn indexed_offset(u: i32, v: i32) -> u32 {
    (((v as u32) >> 5) & 0xFFFF_F800).wrapping_add((u as u32) >> 16)
}

/// The same offset with arithmetic shifts (`sar`), as the fogged indexed
/// fillers compute it.
#[inline]
fn indexed_offset_signed(u: i32, v: i32) -> u32 {
    ((v >> 5) as u32 & 0xFFFF_F800).wrapping_add((u >> 16) as u32)
}

/// Texel word offset of 16.16 `u`/`v` in a 1024-word-stride atlas.
#[inline]
fn raw_offset(u: i32, v: i32) -> u32 {
    (((v as u32) >> 6) & 0x03FF_FC00).wrapping_add((u as u32) >> 16)
}

/// One-line advance of a textured edge (`[0]+=[3]`, `[1]+=[4]`, `[2]+=[5]`).
#[inline]
fn step_textured(edge: &mut Edge) {
    edge[0] = edge[0].wrapping_add(edge[3]);
    edge[1] = edge[1].wrapping_add(edge[4]);
    edge[2] = edge[2].wrapping_add(edge[5]);
}

/// One-line advance of a shaded textured edge: x, shade, u, v.
#[inline]
fn step_textured_shade(edge: &mut Edge) {
    edge[0] = edge[0].wrapping_add(edge[7]);
    edge[6] = edge[6].wrapping_add(edge[13]);
    edge[1] = edge[1].wrapping_add(edge[8]);
    edge[2] = edge[2].wrapping_add(edge[9]);
}

/// One-line advance of a shaded, faded textured edge: x, shade, u, v, fade.
#[inline]
fn step_textured_shade_fade(edge: &mut Edge) {
    edge[0] = edge[0].wrapping_add(edge[8]);
    edge[6] = edge[6].wrapping_add(edge[14]);
    edge[1] = edge[1].wrapping_add(edge[9]);
    edge[2] = edge[2].wrapping_add(edge[10]);
    edge[7] = edge[7].wrapping_add(edge[15]);
}

/// Everything a filler needs for one scanline.
pub(crate) struct Span<'s, 't, 'c> {
    pub state: &'s mut RasterState,
    pub target: &'s mut SpanTarget<'t>,
    pub context: &'s SpanContext<'c>,
    pub line: i32,
    pub left: &'s mut Edge,
    pub right: &'s mut Edge,
    /// Address of the left edge record; some fillers seed their dither
    /// register's high half with it.
    pub left_address: u32,
}

impl Span<'_, '_, '_> {
    fn row(&self) -> usize {
        self.target.row_start(self.line)
    }
}

/// The retail span routine at a row's first slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpanFiller {
    Solid,
    Flat(flat::Filler),
    Indexed(indexed::Filler),
    Raw(raw::Filler),
}

impl SpanFiller {
    /// The port of the retail routine at `address`. `None` for the fillers
    /// of rows no fill slot binds (3, 15, 24..27), which are not ported.
    pub(crate) fn from_retail(address: u32) -> Option<Self> {
        if address == 0x0047_3830 {
            return Some(Self::Solid);
        }
        flat::Filler::from_retail(address)
            .map(Self::Flat)
            .or_else(|| indexed::Filler::from_retail(address).map(Self::Indexed))
            .or_else(|| raw::Filler::from_retail(address).map(Self::Raw))
    }

    pub(crate) fn fill(self, span: Span<'_, '_, '_>) {
        match self {
            Self::Solid => solid(span),
            Self::Flat(filler) => filler.fill(span),
            Self::Indexed(filler) => filler.fill(span),
            Self::Raw(filler) => filler.fill(span),
        }
    }
}

/// `00473830`: flat colour from Graph2D `+0x1C`, left to right.
fn solid(span: Span<'_, '_, '_>) {
    let start = pixel_x(span.left[0]);
    let width = pixel_x(span.right[0]).wrapping_sub(start) as i32;
    if width > 0 {
        let colour = span.state.word_1c as u16;
        let row = span.row() + start as usize;
        span.target.pixels[row..row + width as usize].fill(colour);
    }
    span.left[0] = span.left[0].wrapping_add(span.left[1]);
    span.right[0] = span.right[0].wrapping_add(span.right[1]);
}
