//! The retail Graph2D raster state (`FUN_00480D10`'s 0x10CC-byte block).
//!
//! The scan converter keeps raw pointers into this block; here they are
//! [`VertexRef`] indices with the same identity semantics. The vertex pool and
//! edge lists persist between primitives exactly as the retail block does, so
//! a list slot the converter reads past its live length sees what the last
//! primitive left there.

use super::rows::{SpanRow, TexelTable};

/// One raster vertex: nine dwords at Graph2D `+0x24 + i * 0x24`.
pub(crate) type RasterVertex = [i32; 9];
/// Scan converter edge state: eighteen dwords on `FUN_00472B20`'s stack.
pub(crate) type Edge = [i32; 18];

/// Vertex dword 4: x in 16.16 fixed point.
pub(crate) const VX: usize = 4;
/// Vertex dword 5: integer scanline.
pub(crate) const VY: usize = 5;
/// Vertex dwords 6/7: texel u and v in 16.16.
pub(crate) const VU: usize = 6;
pub(crate) const VV: usize = 7;
/// Edge dword 16: copied into a vertex's y by routine 5; never written.
pub(crate) const EDGE_Y: usize = 0x10;
/// Edge dword 17: set by routine 1 when the edge is initialised.
pub(crate) const EDGE_VALID: usize = 0x11;

/// Graph2D `+0x24..+0xE34` holds 100 vertices.
pub(crate) const POOL_VERTICES: usize = 100;
/// Each of the four 0x78-byte lists holds ten `{from, to, flags}` entries.
pub(crate) const LIST_ENTRIES: usize = 10;
pub(crate) const LIST_WORDS: usize = LIST_ENTRIES * 3;

/// A vertex pointer in a list slot. Zero is the null pointer the zeroed
/// allocation starts with; `i + 1` is pool vertex `i`.
pub(crate) type VertexRef = u32;

pub(crate) const fn vertex_ref(index: usize) -> VertexRef {
    index as VertexRef + 1
}

/// Graph2D list bases, in words from `+0xE38`.
pub(crate) const LIST_CHAIN_A: usize = 0;
pub(crate) const LIST_CHAIN_B: usize = LIST_WORDS;
pub(crate) const LIST_CLIP_A: usize = 2 * LIST_WORDS;
pub(crate) const LIST_CLIP_B: usize = 3 * LIST_WORDS;

/// The four edge lists, contiguous as at Graph2D `+0xE38..+0x1018`.
pub(crate) type ListWords = [u32; 4 * LIST_WORDS];

/// Clip rectangle at Graph2D `+0xC`: inclusive left/top, exclusive
/// right/bottom, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRect {
    pub x0: i16,
    pub y0: i16,
    pub x1: i16,
    pub y1: i16,
}

/// Retail Graph2D fields the raster consumes, plus the stack addresses of
/// the scan converter's two edge records (see [`ScanFrame`]).
#[derive(Debug, Clone)]
pub(crate) struct RasterState {
    pub clip: ClipRect,
    /// `+0x14`: the bound material record.
    pub material: Option<super::material::MaterialId>,
    /// `+0x18`: per-primitive word (texel shade row byte at `+0x1B`, or the
    /// span constant some fillers add).
    pub word_18: u32,
    /// `+0x1C`: flat colour / packed word read by untextured fillers.
    pub word_1c: u32,
    /// `+0x20`: selected span row.
    pub row: SpanRow,
    /// `+0x24`: vertex pool.
    pub pool: [RasterVertex; POOL_VERTICES],
    /// `+0xE34`: live pool length.
    pub pool_count: i32,
    /// `+0xE38..+0x1018`: chains A/B and their x-split lists.
    pub lists: ListWords,
    /// `+0x1018`: table base, as written by the last handler.
    pub table: TexelTable,
    /// `+0x10C8`: shade-dither word.
    pub dither: u16,
}

impl RasterState {
    pub(crate) fn new(clip: ClipRect) -> Self {
        Self {
            clip,
            material: None,
            word_18: 0,
            word_1c: 0,
            row: SpanRow::new(TexelTable::Raw, 0),
            pool: [[0; 9]; POOL_VERTICES],
            pool_count: 0,
            lists: [0; 4 * LIST_WORDS],
            // FUN_00480D10 stores PTR_004D5C64 for the software table.
            table: TexelTable::Raw,
            dither: 0,
        }
    }

    pub(crate) fn vertex(&self, reference: VertexRef) -> &RasterVertex {
        &self.pool[pool_index(reference)]
    }

    pub(crate) fn vertex_mut(&mut self, reference: VertexRef) -> &mut RasterVertex {
        &mut self.pool[pool_index(reference)]
    }

    /// Allocate the next pool vertex (`pool[+0xE34]++`).
    pub(crate) fn allocate(&mut self) -> VertexRef {
        let index = self.pool_count as usize;
        assert!(
            index < POOL_VERTICES,
            "software raster vertex pool overflow"
        );
        self.pool_count += 1;
        vertex_ref(index)
    }
}

fn pool_index(reference: VertexRef) -> usize {
    assert!(
        reference != 0,
        "software raster followed a null vertex pointer"
    );
    reference as usize - 1
}

/// Addresses of `FUN_00472B20`'s two edge records. Several shaded fillers
/// seed their dither generator's high half with the left edge pointer's high
/// sixteen bits, so the retail stack location is an input to those pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanFrame {
    /// ESP after `FUN_00472B20`'s prologue for a top-level draw. Chain A's
    /// edge sits at `+0x90`, chain B's at `+0x48`.
    pub esp: u32,
}

impl ScanFrame {
    /// `FUN_004734E0` splits a non-monotonic quad and re-enters the scan
    /// converter 0x1A0 bytes deeper.
    pub(crate) fn nested(self) -> Self {
        Self {
            esp: self.esp.wrapping_sub(0x1A0),
        }
    }

    pub(crate) fn chain_a_edge(self) -> u32 {
        self.esp.wrapping_add(0x90)
    }

    pub(crate) fn chain_b_edge(self) -> u32 {
        self.esp.wrapping_add(0x48)
    }
}
