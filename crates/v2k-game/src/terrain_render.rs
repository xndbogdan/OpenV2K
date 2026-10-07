//! Per-level terrain transition sprite upload.

use std::collections::HashMap;
use std::sync::Arc;
use v2k_formats::palette::{BRIGHTEST_SHADE, COLORS_PER_SHADE, SHADE_LEVELS};

use v2k_render::terrain_tiles::{
    build_lookup, scan_dimensions, NativeSpriteRef, NativeTerrainMaterials, TerrainFrames,
    CANONICAL_FRAME_COUNT, INFECTION_BASE_OFFSET, INFECTION_FRAME_COUNT,
};
use v2k_render::{NativePalette, NativeSprite, NativeTexels, Renderer};

use crate::resource_cache::ResourceCache;

/// The first eight Section-6 dwords are the render context's primary colour
/// table. Terrain shade byte 0..7 indexes them directly; their high byte is
/// the true 0..31 row in a Section-3 palette.
pub fn primary_shade_rows(cache: &ResourceCache) -> Option<[u8; 8]> {
    let entries = cache.fog_gradient()?.get(..8)?;
    let mut rows = [0_u8; 8];
    for (row, entry) in rows.iter_mut().zip(entries) {
        if usize::from(entry.shade_level) >= SHADE_LEVELS {
            return None;
        }
        *row = entry.shade_level;
    }
    Some(rows)
}

/// Resolve the 120 D4-canonical opaque terrain sprites beginning at Section
/// 13 +0x4C and upload them as a compact atlas. Per-frame shade scales are
/// measured from the sprites' own palette rows rather than invented globally.
/// Returns `None` when any frame is unavailable, preserving the renderer's
/// diagnostic fallback.
pub fn build_terrain_frames(
    cache: &ResourceCache,
    renderer: &mut dyn Renderer,
) -> Option<TerrainFrames> {
    let descriptor = cache.level_desc()?;
    let shade_rows = primary_shade_rows(cache)?;
    let base = descriptor.terrain_sprite_base;
    let (scan_columns, scan_rows) = scan_dimensions(descriptor.terrain_draw_depth);
    let mut sprites = Vec::with_capacity(CANONICAL_FRAME_COUNT);
    let mut indexed = Vec::with_capacity(CANONICAL_FRAME_COUNT);
    let mut palettes = Vec::with_capacity(CANONICAL_FRAME_COUNT);
    let mut shade_scales = Vec::with_capacity(CANONICAL_FRAME_COUNT);
    for offset in 0..CANONICAL_FRAME_COUNT as u32 {
        let gid = u16::try_from(base.checked_add(offset)?).ok()?;
        let (atlas, entry) = cache.global_sprite(gid)?;
        match atlas.decode_sprite_opaque(entry, BRIGHTEST_SHADE) {
            Ok(sprite) => {
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
                let bright_luma = sprite_luminance(&sprite.rgba).max(1.0);
                let mut scales = [1.0f32; 8];
                for (shade, scale) in scales.iter_mut().enumerate() {
                    let shaded = atlas
                        .decode_sprite_opaque(entry, usize::from(shade_rows[shade]))
                        .ok()?;
                    *scale = (sprite_luminance(&shaded.rgba) / bright_luma).clamp(0.0, 1.5);
                }
                sprites.push(sprite);
                shade_scales.push(scales);
            }
            Err(error) => {
                eprintln!("terrain: failed to decode transition sprite {gid}: {error}");
                return None;
            }
        }
    }

    // `FUN_00430430` overlays one of these five marching-square shapes when
    // terrain byte 2 carries infection bit 0x10. Their authored flag 0x04
    // fixes the palette at row 28; frames 0..3 additionally key index zero,
    // while the all-four-corners frame is deliberately opaque.
    let mut infection_sprites = Vec::with_capacity(INFECTION_FRAME_COUNT);
    for offset in 0..INFECTION_FRAME_COUNT as u32 {
        let gid = u16::try_from(base.checked_add(INFECTION_BASE_OFFSET + offset)?).ok()?;
        let (atlas, entry) = cache.global_sprite(gid)?;
        let sprite = atlas.decode_sprite(entry, 28).ok()?;
        infection_sprites.push(sprite);
    }
    let (infection_rgba, infection_width, infection_height, infection_frame_uvs) =
        pack_infection_frames(&infection_sprites)?;

    let cell_w = sprites.iter().map(|s| s.width as u32).max()?;
    let cell_h = sprites.iter().map(|s| s.height as u32).max()?;
    if cell_w == 0 || cell_h == 0 {
        return None;
    }
    const COLUMNS: u32 = 12;
    let rows = (CANONICAL_FRAME_COUNT as u32 + COLUMNS - 1) / COLUMNS;
    let atlas_w = cell_w * COLUMNS;
    let atlas_h = cell_h * rows;
    let mut rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut indices_rgba = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut frame_uvs = vec![[0.0; 4]; CANONICAL_FRAME_COUNT];

    for (frame, sprite) in sprites.iter().enumerate() {
        let w = sprite.width as u32;
        let h = sprite.height as u32;
        let x0 = frame as u32 % COLUMNS * cell_w;
        let y0 = frame as u32 / COLUMNS * cell_h;
        for y in 0..h {
            let src = (y * w * 4) as usize;
            let dst = (((y0 + y) * atlas_w + x0) * 4) as usize;
            rgba[dst..dst + (w * 4) as usize]
                .copy_from_slice(&sprite.rgba[src..src + (w * 4) as usize]);
            for x in 0..w {
                let index = indexed[frame].indices[(y * w + x) as usize];
                let indexed_dst = (((y0 + y) * atlas_w + x0 + x) * 4) as usize;
                indices_rgba[indexed_dst] = index;
                indices_rgba[indexed_dst + 3] = 255;
            }
        }
        frame_uvs[frame] = [
            (x0 as f32 + 0.5) / atlas_w as f32,
            (y0 as f32 + 0.5) / atlas_h as f32,
            (x0 as f32 + w as f32 - 0.5) / atlas_w as f32,
            (y0 as f32 + h as f32 - 0.5) / atlas_h as f32,
        ];
    }

    let texture = renderer.create_texture(&rgba, atlas_w, atlas_h)?;
    let Some(index_texture) = renderer.create_texture_nearest(&indices_rgba, atlas_w, atlas_h)
    else {
        renderer.destroy_texture(texture);
        return None;
    };
    let palette_rgba: Vec<u8> = palettes.into_iter().flatten().collect();
    let Some(palette_texture) = renderer.create_texture_nearest(
        &palette_rgba,
        COLORS_PER_SHADE as u32,
        (CANONICAL_FRAME_COUNT * SHADE_LEVELS) as u32,
    ) else {
        renderer.destroy_texture(index_texture);
        renderer.destroy_texture(texture);
        return None;
    };
    let Some(infection_texture) =
        renderer.create_texture_nearest(&infection_rgba, infection_width, infection_height)
    else {
        renderer.destroy_texture(palette_texture);
        renderer.destroy_texture(index_texture);
        renderer.destroy_texture(texture);
        return None;
    };
    let native = native_terrain_materials(cache, renderer, base);
    Some(TerrainFrames {
        texture,
        index_texture,
        palette_texture,
        infection_texture,
        infection_frame_uvs,
        frame_uvs,
        lookup: build_lookup(),
        shade_scales,
        shade_rows,
        scan_columns,
        scan_rows,
        native,
    })
}

/// Register the 120 canonical frames and five infection shapes as native
/// Section-3 records for backends that run the retail ground producer, with
/// the first eight Section-6 dwords. `None` when the backend keeps no
/// native materials or a record is not indexed.
fn native_terrain_materials(
    cache: &ResourceCache,
    renderer: &mut dyn Renderer,
    base: u32,
) -> Option<NativeTerrainMaterials> {
    let gradient = cache.fog_gradient()?;
    let shade_words: [u32; 8] = std::array::from_fn(|index| {
        gradient.get(index).map_or(0, |entry| {
            u32::from_le_bytes([entry.r, entry.g, entry.b, entry.shade_level])
        })
    });
    // One display-format palette block per Section-3 atlas, shared by its
    // records so palette overshoot reads the neighbouring palettes.
    let mut blocks: HashMap<*const v2k_formats::sprites::SpriteAtlas, Arc<[u16]>> = HashMap::new();
    let count = CANONICAL_FRAME_COUNT as u32 + INFECTION_FRAME_COUNT as u32;
    let mut sprites = Vec::with_capacity(count as usize);
    for offset in 0..count {
        let gid = u16::try_from(base.checked_add(offset)?).ok()?;
        let (atlas, entry) = cache.global_sprite(gid)?;
        if entry.pal_size & 0x02 != 0 {
            return None;
        }
        let indices = atlas.decode_indices(entry).ok()?.indices;
        let block = blocks
            .entry(atlas as *const _)
            .or_insert_with(|| Arc::from(atlas.display_palette_words()))
            .clone();
        // Record +0x10/+0x12, which the UV setup reads.
        let width = entry.flags as u16;
        let height = (entry.flags >> 16) as u16;
        let id = renderer.create_native_sprite(NativeSprite {
            flags: entry.pal_size & 0xFF,
            shade_count: entry.shade_count,
            width,
            height,
            texels: NativeTexels::Indexed {
                indices: &indices,
                palette: NativePalette {
                    block,
                    start: entry.pal_offset as usize / 2,
                },
            },
        })?;
        sprites.push(NativeSpriteRef { id, width, height });
    }
    debug_assert_eq!(INFECTION_BASE_OFFSET, CANONICAL_FRAME_COUNT as u32);
    Some(NativeTerrainMaterials {
        tile_base: base,
        sprites,
        shade_words,
    })
}

/// Pack the five fixed-shade infection shapes into one nearest-filtered strip.
/// Half-texel UV insets match the canonical terrain and shoreline atlases and
/// prevent keyed edge pixels from sampling a neighboring shape.
fn pack_infection_frames(
    sprites: &[v2k_formats::sprites::DecodedSprite],
) -> Option<(Vec<u8>, u32, u32, [[f32; 4]; INFECTION_FRAME_COUNT])> {
    if sprites.len() != INFECTION_FRAME_COUNT {
        return None;
    }
    let total_width = sprites.iter().map(|sprite| u32::from(sprite.width)).sum();
    let max_height = sprites
        .iter()
        .map(|sprite| u32::from(sprite.height))
        .max()?;
    if total_width == 0 || max_height == 0 {
        return None;
    }

    let mut rgba = vec![0; (total_width * max_height * 4) as usize];
    let mut frame_uvs = [[0.0; 4]; INFECTION_FRAME_COUNT];
    let mut x_cursor = 0;
    for (frame, sprite) in sprites.iter().enumerate() {
        let width = u32::from(sprite.width);
        let height = u32::from(sprite.height);
        for y in 0..height {
            let source = (y * width * 4) as usize;
            let destination = ((y * total_width + x_cursor) * 4) as usize;
            rgba[destination..destination + (width * 4) as usize]
                .copy_from_slice(&sprite.rgba[source..source + (width * 4) as usize]);
        }
        frame_uvs[frame] = [
            (x_cursor as f32 + 0.5) / total_width as f32,
            0.5 / max_height as f32,
            (x_cursor as f32 + width as f32 - 0.5) / total_width as f32,
            (height as f32 - 0.5) / max_height as f32,
        ];
        x_cursor += width;
    }
    Some((rgba, total_width, max_height, frame_uvs))
}

fn sprite_luminance(rgba: &[u8]) -> f32 {
    rgba.chunks_exact(4)
        .map(|pixel| pixel[0] as f32 * 0.2126 + pixel[1] as f32 * 0.7152 + pixel[2] as f32 * 0.0722)
        .sum()
}
