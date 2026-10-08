//! Model-face material resolution for Section-8 geometry.
//!
//! The face opcode's 0x80 bit selects the `mat` operand namespace:
//!
//! - clear: a Section-7 palette colour;
//! - set: a global Section-3 sprite, texture-mapped over the primitive using
//!   the implicit UV layout retained by `v2k-formats::models`.
//!
//! The original executable passes the resolved sprite entry to its textured
//! triangle/quad rasterizers (for example 0x83 -> 0x47C600 and
//! 0xC3 -> 0x47CBE0). The sprite images therefore must not be collapsed to an
//! average colour: Klaus's bones and wing membranes are painted into them.

use std::cell::RefCell;
use std::collections::HashMap;

use v2k_formats::models::{face_material, Billboard};
use v2k_formats::sprites::SpriteTexelFormat;
use v2k_formats::system::PaletteEntry;
use v2k_render::{
    BillboardMaterial, FaceMaterial, IndexedModelTexture, Renderer, TextureId, WorldSpriteBlend,
};

use crate::resource_cache::ResourceCache;

/// Fallback colour for an unresolvable palette or sprite material.
const FALLBACK: [f32; 3] = [0.6, 0.6, 0.6];

/// Persistent GPU texture cache for sprite-backed model materials.
///
/// Global sprite ids are stable slots in the PRELOAD resource pool. Models
/// may be materialized every frame, so only the lightweight `FaceMaterial`
/// vector is rebuilt; decoded/uploaded sprite textures are reused.
#[derive(Default)]
pub struct ModelMaterialCache {
    sprite_textures: RefCell<HashMap<u16, TextureId>>,
    sprite_dimensions: RefCell<HashMap<u16, (u16, u16)>>,
}

impl ModelMaterialCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop the uploaded texture ids after the renderer that owned them was
    /// replaced; the next draw uploads each sprite again.
    pub fn forget_textures(&self) {
        self.sprite_textures.borrow_mut().clear();
    }

    /// Resolve the cached geometry materials for one global model id.
    pub fn materials_for_model(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        model_id: usize,
    ) -> Vec<FaceMaterial> {
        let Some(model) = cache.global_model(model_id) else {
            return Vec::new();
        };
        self.materials_for(cache, renderer, &model.face_materials)
    }

    /// Resolve a live materialized face stream.
    pub fn materials_for(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        materials: &[u16],
    ) -> Vec<FaceMaterial> {
        self.materials_for_palette(
            cache,
            renderer,
            materials,
            cache.color_palettes().map(Vec::as_slice),
        )
    }

    /// Resolve a face stream against an explicit Section-7 palette.
    ///
    /// This is for diagnostics which deliberately keep mutually exclusive
    /// resource packs resident together. Runtime callers should use
    /// [`Self::materials_for`] so the active cache layer remains authoritative.
    pub fn materials_for_with_palette(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        materials: &[u16],
        palette: &[PaletteEntry],
    ) -> Vec<FaceMaterial> {
        self.materials_for_palette(cache, renderer, materials, Some(palette))
    }

    fn materials_for_palette(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        materials: &[u16],
        palette: Option<&[PaletteEntry]>,
    ) -> Vec<FaceMaterial> {
        materials
            .iter()
            .map(|&packed| {
                let (mat, is_sprite) = face_material(packed);
                if is_sprite {
                    let flags = self.sprite_render_flags(cache, mat).unwrap_or_default();
                    FaceMaterial {
                        color: [1.0, 1.0, 1.0],
                        palette_rgb555: None,
                        emissive: [0.0, 0.0, 0.0],
                        texture: self.sprite_texture(cache, renderer, mat),
                        blend: sprite_blend(flags),
                        flat_shade_row: sprite_flat_shade_row(flags),
                    }
                } else {
                    let palette_rgb555 = palette
                        .and_then(|p| p.get(mat as usize))
                        .map(|entry| entry.rgb555);
                    FaceMaterial {
                        color: if palette_rgb555.is_some() {
                            [1.0; 3]
                        } else {
                            FALLBACK
                        },
                        palette_rgb555,
                        emissive: [0.0, 0.0, 0.0],
                        texture: None,
                        blend: WorldSpriteBlend::Masked,
                        flat_shade_row: v2k_formats::palette::BRIGHTEST_SHADE as u8,
                    }
                }
            })
            .collect()
    }

    /// Resolve Section-8 billboard colour/texture plus the source sprite's
    /// authored aspect ratio. Flat-colour billboards are square.
    pub fn billboard_materials(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        billboards: &[Billboard],
    ) -> Vec<BillboardMaterial> {
        self.billboard_materials_for_palette(
            cache,
            renderer,
            billboards,
            cache.color_palettes().map(Vec::as_slice),
        )
    }

    /// Resolve billboards while using an explicit palette for flat-colour
    /// entries. Textured billboards still resolve their canonical global
    /// sprite ids through the shared cache.
    pub fn billboard_materials_with_palette(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        billboards: &[Billboard],
        palette: &[PaletteEntry],
    ) -> Vec<BillboardMaterial> {
        self.billboard_materials_for_palette(cache, renderer, billboards, Some(palette))
    }

    fn billboard_materials_for_palette(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        billboards: &[Billboard],
        palette: Option<&[PaletteEntry]>,
    ) -> Vec<BillboardMaterial> {
        billboards
            .iter()
            .map(|billboard| {
                let (face, blend) = if billboard.textured {
                    let flags = self
                        .sprite_render_flags(cache, billboard.id)
                        .unwrap_or_default();
                    let blend = sprite_blend(flags);
                    (
                        FaceMaterial {
                            color: [1.0; 3],
                            palette_rgb555: None,
                            emissive: [0.0; 3],
                            texture: self.sprite_texture(cache, renderer, billboard.id),
                            blend,
                            flat_shade_row: sprite_flat_shade_row(flags),
                        },
                        blend,
                    )
                } else {
                    let packed = billboard.id;
                    let face = self
                        .materials_for_palette(cache, renderer, &[packed], palette)
                        .into_iter()
                        .next()
                        .unwrap_or(FaceMaterial {
                            color: FALLBACK,
                            palette_rgb555: None,
                            emissive: [0.0; 3],
                            texture: None,
                            blend: WorldSpriteBlend::Masked,
                            flat_shade_row: v2k_formats::palette::BRIGHTEST_SHADE as u8,
                        });
                    (face, WorldSpriteBlend::Additive)
                };
                let face = if billboard.textured && face.texture.is_none() {
                    FaceMaterial {
                        color: FALLBACK,
                        palette_rgb555: None,
                        emissive: [0.0; 3],
                        texture: None,
                        blend: WorldSpriteBlend::Masked,
                        flat_shade_row: v2k_formats::palette::BRIGHTEST_SHADE as u8,
                    }
                } else {
                    face
                };
                let (width, height) = if billboard.textured {
                    self.sprite_dimensions(cache, billboard.id)
                        .unwrap_or((1, 1))
                } else {
                    (1, 1)
                };
                BillboardMaterial {
                    face,
                    width,
                    height,
                    blend,
                }
            })
            .collect()
    }

    /// Resolve and cache any global Section-3 sprite texture. World effects
    /// share this with model materials so the same global id is decoded and
    /// uploaded only once.
    pub fn sprite_texture(
        &self,
        cache: &ResourceCache,
        renderer: &mut dyn Renderer,
        sprite_id: u16,
    ) -> Option<TextureId> {
        if let Some(id) = self.sprite_textures.borrow().get(&sprite_id).copied() {
            return Some(id);
        }

        let (atlas, entry) = cache.global_sprite(sprite_id)?;
        // Raw RGB555 texels have no palette rows. Decode their intrinsic
        // colour and retain the direct RGBA upload; draw policy owns lighting.
        let decoded = atlas
            .decode_sprite(entry, v2k_formats::palette::BRIGHTEST_SHADE)
            .ok()?;
        // Ordinary indexed sprites keep every row for per-draw model lighting
        // and fixed-sprite fog; large flat palettes retain direct RGBA uploads.
        let texture = if entry.texel_format() == SpriteTexelFormat::Indexed8
            && entry.shade_count as usize <= v2k_formats::palette::COLORS_PER_SHADE
        {
            let indexed = atlas.decode_indices(entry).ok()?;
            let mut palette_rgba = Vec::with_capacity(
                v2k_formats::palette::SHADE_LEVELS * v2k_formats::palette::COLORS_PER_SHADE * 4,
            );
            for row in 0..v2k_formats::palette::SHADE_LEVELS {
                let colors = atlas.palette_row(entry, row).ok()?;
                for color in colors.iter().take(v2k_formats::palette::COLORS_PER_SHADE) {
                    palette_rgba.extend_from_slice(color);
                }
            }
            renderer.create_indexed_model_texture(IndexedModelTexture {
                fallback_rgba: &decoded.rgba,
                indices: &indexed.indices,
                palette_rgba: &palette_rgba,
                width: decoded.width as u32,
                height: decoded.height as u32,
                // The textured triangle/quad handlers select their zero-key
                // rasterizer family from sprite flag bit 0. Palette index
                // zero remains authored surface colour when that bit is
                // clear; shade-count is not a coverage policy.
                transparent_zero: entry.is_zero_keyed(),
            })?
        } else {
            renderer.create_texture(&decoded.rgba, decoded.width as u32, decoded.height as u32)?
        };
        self.sprite_textures.borrow_mut().insert(sprite_id, texture);
        Some(texture)
    }

    /// Low-byte Section-3 render flags used by the retail fixed-billboard
    /// fillers to select shade row and framebuffer blend mode.
    pub fn sprite_render_flags(&self, cache: &ResourceCache, sprite_id: u16) -> Option<u8> {
        cache
            .global_sprite(sprite_id)
            .map(|(_, entry)| entry.pal_size as u8)
    }

    /// Authored dimensions of a global sprite, used to retain its aspect when
    /// a backend-neutral world particle supplies only a scalar extent.
    pub fn sprite_dimensions(&self, cache: &ResourceCache, sprite_id: u16) -> Option<(u16, u16)> {
        if let Some(dimensions) = self.sprite_dimensions.borrow().get(&sprite_id).copied() {
            return Some(dimensions);
        }
        let (atlas, entry) = cache.global_sprite(sprite_id)?;
        let (width, height) = atlas.dimensions(entry).ok()?;
        let dimensions = (width.max(1), height.max(1));
        self.sprite_dimensions
            .borrow_mut()
            .insert(sprite_id, dimensions);
        Some(dimensions)
    }
}

/// Contextual signed model-light shift used by world entity/static-object
/// draws in `FUN_004136C0` and `FUN_004138F0`.
///
/// The scrolling terrain-light byte is sampled at the entity cell, then the
/// exact `FUN_004336E0` 0..8 underwater step is subtracted. The renderer
/// applies the result to the signed -8..7 Section-6 model-light lookup.
pub fn contextual_model_shade_shift(
    light_delta: i8,
    height_world_raw: i16,
    darkness_start: i16,
    darkness_range: i16,
) -> i32 {
    i32::from(light_delta)
        - i32::from(v2k_render::terrain_tiles::underwater_darkness(
            height_world_raw,
            darkness_start,
            darkness_range,
        ))
}

/// Low-byte Section-3 flags select the same three textured-primitive material
/// families in `FUN_0047EF10`. That callback services ordinary Section-8
/// textured faces as well as the 0x78-family billboard queue. Tree foliage
/// such as sprite 1433 has flags 0x05 and is therefore masked, not additive.
/// Flag `0x08` selects retail's `source + destination / 2` span family; the
/// renderer names that recovered policy [`WorldSpriteBlend::HalfAdditive`].
pub fn sprite_blend(flags: u8) -> WorldSpriteBlend {
    if flags & 0x10 != 0 {
        WorldSpriteBlend::Additive
    } else if flags & 0x08 != 0 {
        WorldSpriteBlend::HalfAdditive
    } else {
        WorldSpriteBlend::Masked
    }
}

/// Flat textured fillers retain the sprite's palette base, adding `0x380`
/// bytes only for flag `0x04`: 28 rows × 16 colours × 2-byte RGB555 entries.
/// `FUN_004782B0`/`FUN_004788D0` implement the opaque/keyed model fills;
/// HUD's fixed blit `FUN_0047AD90` applies the same rule at `0047B272`.
/// `FUN_004ABB80` merely relocates the source palette pointer: there is no
/// implicit brightest-row bias. Direct palettes ignore the row at decode.
pub const fn sprite_flat_shade_row(flags: u8) -> u8 {
    if flags & 0x04 != 0 {
        28
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_flags_distinguish_foliage_glow_and_half_additive() {
        assert_eq!(sprite_blend(0x05), WorldSpriteBlend::Masked);
        assert_eq!(sprite_blend(0x15), WorldSpriteBlend::Additive);
        assert_eq!(sprite_blend(0x0d), WorldSpriteBlend::HalfAdditive);
    }

    #[test]
    fn sprite_flag_four_selects_flat_palette_row_twenty_eight() {
        assert_eq!(sprite_flat_shade_row(0x04), 28);
        assert_eq!(sprite_flat_shade_row(0x05), 28);
        assert_eq!(sprite_flat_shade_row(0x00), 0);
        assert_eq!(sprite_flat_shade_row(0x01), 0);
        assert_eq!(sprite_flat_shade_row(0x08), 0);
        assert_eq!(sprite_flat_shade_row(0x10), 0);
        assert_eq!(sprite_flat_shade_row(0x14), 28);
    }

    #[test]
    fn contextual_model_shade_subtracts_exact_underwater_step() {
        assert_eq!(contextual_model_shade_shift(3, -1024, -1024, 512), 3);
        assert_eq!(contextual_model_shade_shift(3, -1280, -1024, 512), -1);
        assert_eq!(contextual_model_shade_shift(3, -1536, -1024, 512), -5);
        assert_eq!(contextual_model_shade_shift(-7, -1537, -1024, 512), -15);
    }
}
