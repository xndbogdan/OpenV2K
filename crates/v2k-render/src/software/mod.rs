//! Faithful CPU reimplementation of V2000's 16-bpp software raster.
//!
//! Retail draws every queued primitive through the Graph2D fill table that
//! `FUN_00480D10` installs for a 16-bpp software surface: a slot handler
//! decodes the primitive packet into raster vertices, the polygon scan
//! converter `FUN_00472B20` walks and clips its edges, and a span filler
//! writes each scanline into the locked RGB565 surface. This module mirrors
//! that pipeline with the same integer arithmetic, state and quirks, so its
//! output can be compared byte for byte with the original. The contract and
//! its evidence are in `docs/re/SOFTWARE_RASTER.md`.

mod attr;
mod fixed;
mod fog;
mod format;
mod material;
mod rows;
mod scan;
mod slots;
mod span;
mod state;

pub use fog::FogRamp;
pub use format::PixelFormat;
pub use material::{
    flags as material_flags, MaterialId, MaterialSource, MaterialView, ATLAS_STRIDE,
};
pub use rows::{SpanRow, TexelTable};
pub use slots::FillSlot;
pub use state::ClipRect;

use span::SpanTarget;
use state::RasterState;

/// An RGB565 surface with the retail pitch convention (bytes per row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Surface565 {
    pub width: u32,
    pub height: u32,
    pub pitch: usize,
    pub pixels: Vec<u16>,
}

impl Surface565 {
    pub fn new(width: u32, height: u32) -> Self {
        Self::with_pitch(width, height, width as usize * 2)
    }

    pub fn with_pitch(width: u32, height: u32, pitch: usize) -> Self {
        assert!(pitch >= width as usize * 2 && pitch % 2 == 0);
        Self {
            width,
            height,
            pitch,
            pixels: vec![0; pitch / 2 * height as usize],
        }
    }

    pub fn row(&self, y: u32) -> &[u16] {
        let start = y as usize * (self.pitch / 2);
        &self.pixels[start..start + self.width as usize]
    }
}

/// The software raster's persistent state: the Graph2D block plus the
/// process globals its routines keep between primitives.
#[derive(Debug, Clone)]
pub struct SoftwareRaster {
    state: RasterState,
    fog: FogRamp,
    format: PixelFormat,
    /// ESP at a slot handler's first instruction. Only its effect on the
    /// scan converter's edge-record addresses is observable (the dither seed
    /// of shaded fillers); see [`state::ScanFrame`].
    handler_esp: u32,
}

impl SoftwareRaster {
    pub fn new(clip: ClipRect, format: PixelFormat, handler_esp: u32) -> Self {
        Self {
            state: RasterState::new(clip),
            fog: FogRamp::default(),
            format,
            handler_esp,
        }
    }

    pub fn clip(&self) -> ClipRect {
        self.state.clip
    }

    pub fn set_clip(&mut self, clip: ClipRect) {
        self.state.clip = clip;
    }

    pub fn fog(&self) -> &FogRamp {
        &self.fog
    }

    pub fn fog_mut(&mut self) -> &mut FogRamp {
        &mut self.fog
    }

    /// Graph2D `+0x10C8` after the last draw.
    pub fn dither_word(&self) -> u16 {
        self.state.dither
    }

    /// Draw one primitive packet through `slot`, as the queue thunk would.
    pub fn draw(
        &mut self,
        slot: FillSlot,
        packet: &[u8],
        surface: &mut Surface565,
        materials: &dyn MaterialSource,
    ) {
        let mut target = SpanTarget {
            pixels: &mut surface.pixels,
            pitch: surface.pitch,
        };
        self.dispatch(slot, packet, &mut target, materials);
    }
}
