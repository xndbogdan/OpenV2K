//! The full-frame flash's textured record (`FUN_00453410`): a two-by-two
//! row-28 material stretched over the whole surface. Past 640 pixels the
//! span reciprocal overruns and the texel steps leave the record; the
//! native record's index 0 then still shows the palette's colour, as
//! retail's 800x600 and 1024x768 frames do.

use std::sync::Arc;

use v2k_render::software::store::{MaterialStore, OwnedMaterial};
use v2k_render::software::{
    material_flags, ClipRect, FillSlot, NoWordImages, PixelFormat, SoftwareRaster, Surface565,
};
use v2k_render::{NativePalette, NativeSprite, NativeTexels};

const GREY: u16 = 0xB596;

fn flash(width: u32, height: u32) -> Surface565 {
    let mut palette = vec![0u16; 0x1D0];
    palette[0x1C0] = GREY;
    let material = OwnedMaterial::from_native(&NativeSprite {
        flags: material_flags::ROW_28,
        shade_count: 1,
        width: 2,
        height: 2,
        texels: NativeTexels::Indexed {
            indices: &[0; 4],
            palette: NativePalette {
                block: Arc::from(palette),
                start: 0,
            },
        },
    });
    let mut store = MaterialStore::default();
    let id = store.insert(material);
    let (w, h) = (width as i16, height as i16);
    let mut packet = [0u8; 0x14];
    for (index, (x, y)) in [(0, 0), (w, 0), (w, h), (0, h)].into_iter().enumerate() {
        packet[4 * index..4 * index + 2].copy_from_slice(&x.to_le_bytes());
        packet[4 * index + 2..4 * index + 4].copy_from_slice(&y.to_le_bytes());
    }
    packet[0x10..].copy_from_slice(&id.to_le_bytes());
    let mut surface = Surface565::new(width, height);
    let clip = ClipRect {
        x0: 0,
        y0: 0,
        x1: w,
        y1: h,
    };
    let mut raster = SoftwareRaster::new(clip, PixelFormat::RGB565, 0x0019_FDEC);
    raster.draw(
        FillSlot::TexturedQuad,
        &packet,
        &mut surface,
        &store,
        &NoWordImages,
    );
    surface
}

#[test]
fn the_flash_record_covers_every_original_tier() {
    for (width, height) in [(320, 240), (640, 480), (800, 600), (1024, 768)] {
        let surface = flash(width, height);
        assert!(
            surface.pixels.iter().all(|&pixel| pixel == GREY),
            "{width}x{height}"
        );
    }
}
