//! Level water shoreline frame set: resolves the 5 marching-squares shape
//! sprites from the global sprite pool and uploads them as one GPU texture.
//!
//! The engine computes the shoreline base once per level load
//! (`FUN_00433180`): `DAT_004db26c = terrain_sprite_base + 120 + 5`, where the
//! base is the Section 13 descriptor field +0x4C (a GLOBAL sprite-pool index) and
//! 120 is the constant count of D4-canonical 5-level corner configurations
//! (the interior-tile set `[base .. +119]` used by the opaque pass-1
//! surface, which the port does not draw yet). The five frames at
//! `base + 125 ..= +129` are the pass-2 shoreline shapes consumed by
//! the 16-entry table at 0x4CACC8 (see `v2k_render::water`).

use v2k_formats::palette::{BRIGHTEST_SHADE, COLORS_PER_SHADE, SHADE_LEVELS};
use v2k_render::terrain_tiles::scan_dimensions;
use v2k_render::water::{WaterFrames, SHORELINE_BASE_OFFSET};
use v2k_render::Renderer;

use crate::resource_cache::ResourceCache;
use crate::terrain_render::primary_shade_rows;

/// Build the water shoreline frame set for the currently loaded level.
///
/// Returns `None` (and the water pass falls back to flat color) when the
/// level is dry, the descriptor is missing, or the sprites are not resolvable
/// from any loaded layer. The caller owns the texture and must
/// `destroy_texture` it on level unload.
pub fn build_water_frames(
    cache: &ResourceCache,
    renderer: &mut dyn Renderer,
) -> Option<WaterFrames> {
    let terrain = cache.terrain()?;
    if !terrain.water_enabled() {
        return None;
    }
    let desc = cache.level_desc()?;
    let shade_rows = primary_shade_rows(cache)?;
    let (scan_columns, scan_rows) = scan_dimensions(desc.terrain_draw_depth);
    let base = desc
        .terrain_sprite_base
        .checked_add(SHORELINE_BASE_OFFSET)?;

    // Decode the 5 shape sprites at the brightest shade (per-vertex Gouraud
    // handles brightness at draw time, mirroring the engine's shade index).
    let mut sprites = Vec::with_capacity(5);
    let mut indexed = Vec::with_capacity(5);
    let mut palettes = Vec::with_capacity(5);
    let mut shade_scales = [[1.0f32; 8]; 5];
    for k in 0..5u32 {
        let gid = u16::try_from(base + k).ok()?;
        let (atlas, entry) = cache.global_sprite(gid)?;
        match atlas.decode_sprite(entry, BRIGHTEST_SHADE) {
            Ok(dec) => {
                indexed.push(atlas.decode_indices(entry).ok()?);
                let mut frame_palette = Vec::with_capacity(SHADE_LEVELS * COLORS_PER_SHADE * 4);
                for shade in 0..SHADE_LEVELS {
                    let row = atlas.palette_row(entry, shade).ok()?;
                    for index in 0..COLORS_PER_SHADE {
                        let color = row.get(index).copied().unwrap_or([0, 0, 0, 255]);
                        frame_palette.extend_from_slice(&color);
                    }
                }
                palettes.push(frame_palette);
                let bright_luma = sprite_luminance(&dec.rgba).max(1.0);
                for shade in 0..8 {
                    let shaded = atlas
                        .decode_sprite(entry, usize::from(shade_rows[shade]))
                        .ok()?;
                    shade_scales[k as usize][shade] =
                        (sprite_luminance(&shaded.rgba) / bright_luma).clamp(0.0, 1.5);
                }
                sprites.push(dec);
            }
            Err(e) => {
                eprintln!("water: failed to decode shoreline sprite {gid}: {e}");
                return None;
            }
        }
    }

    // Pack the frames side by side into one RGBA strip. Opaque texels get
    // alpha 128: the engine's water span function is `dst = texel + dst/2`,
    // which the renderer reproduces as premultiplied
    // `BlendFunc(ONE, ONE_MINUS_SRC_ALPHA)` with source alpha 0.5.
    let total_w: u32 = sprites.iter().map(|s| s.width as u32).sum();
    let max_h: u32 = sprites.iter().map(|s| s.height as u32).max()?;
    if total_w == 0 || max_h == 0 {
        return None;
    }
    let mut rgba = vec![0u8; (total_w * max_h * 4) as usize];
    let mut indices_rgba = vec![0u8; (total_w * max_h * 4) as usize];
    let mut frame_uvs = [[0.0f32; 4]; 5];
    let mut x_cursor = 0u32;
    for (k, s) in sprites.iter().enumerate() {
        let (w, h) = (s.width as u32, s.height as u32);
        for y in 0..h {
            for x in 0..w {
                let src = ((y * w + x) * 4) as usize;
                if !water_texel_covered(k, s.rgba[src + 3]) {
                    continue; // transparent = the dry part of the tile
                }
                let dst = ((y * total_w + x_cursor + x) * 4) as usize;
                let index = indexed[k].indices[(y * w + x) as usize];
                if s.rgba[src + 3] == 0 {
                    // The full-water tile is rasterized as a solid surface by
                    // the retail water span. Its decorative index-zero dots
                    // are palette texels, not cut-outs to the seabed.
                    let palette_offset =
                        (BRIGHTEST_SHADE * COLORS_PER_SHADE + usize::from(index)) * 4;
                    rgba[dst..dst + 3]
                        .copy_from_slice(&palettes[k][palette_offset..palette_offset + 3]);
                } else {
                    rgba[dst..dst + 3].copy_from_slice(&s.rgba[src..src + 3]);
                }
                rgba[dst + 3] = 128;
                indices_rgba[dst] = index;
                indices_rgba[dst + 3] = 255;
            }
        }
        // Half-texel inset keeps NEAREST sampling from bleeding across frames.
        frame_uvs[k] = [
            (x_cursor as f32 + 0.5) / total_w as f32,
            0.5 / max_h as f32,
            (x_cursor as f32 + w as f32 - 0.5) / total_w as f32,
            (h as f32 - 0.5) / max_h as f32,
        ];
        x_cursor += w;
    }

    let texture = renderer.create_texture(&rgba, total_w, max_h)?;
    let Some(index_texture) = renderer.create_texture_nearest(&indices_rgba, total_w, max_h) else {
        renderer.destroy_texture(texture);
        return None;
    };
    let palette_rgba: Vec<u8> = palettes.into_iter().flatten().collect();
    let Some(palette_texture) = renderer.create_texture_nearest(
        &palette_rgba,
        COLORS_PER_SHADE as u32,
        (5 * SHADE_LEVELS) as u32,
    ) else {
        renderer.destroy_texture(index_texture);
        renderer.destroy_texture(texture);
        return None;
    };
    Some(WaterFrames {
        texture,
        index_texture,
        palette_texture,
        frame_uvs,
        shade_scales,
        shade_rows,
        scan_columns,
        scan_rows,
    })
}

fn water_texel_covered(frame: usize, decoded_alpha: u8) -> bool {
    frame == 4 || decoded_alpha != 0
}

fn sprite_luminance(rgba: &[u8]) -> f32 {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| pixel[0] as f32 * 0.2126 + pixel[1] as f32 * 0.7152 + pixel[2] as f32 * 0.0722)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::water_texel_covered;

    #[test]
    fn full_water_keeps_index_zero_detail_opaque() {
        assert!(!water_texel_covered(0, 0));
        assert!(water_texel_covered(0, 255));
        assert!(water_texel_covered(4, 0));
    }
}
