//! Fixed sprites keep indexed palettes so their near/far constructors can
//! select authored rows before lookup, including nonlinear shade ramps.

use v2k_formats::models::Billboard;
use v2k_formats::sprites::{PixelRect, SpriteAtlas, SpriteEntry};
use v2k_formats::{system::PaletteEntry, terrain::TerrainGrid};
use v2k_game::{level::LevelState, model_color::ModelMaterialCache, resource_cache::ResourceCache};
use v2k_render::{Camera, IndexedModelTexture, Renderer, TextureId, WorldSpriteBlend};

#[derive(Default)]
struct TextureUploads {
    rgba: Vec<Vec<u8>>,
    dimensions: Vec<(u32, u32)>,
    indexed: Vec<Vec<u8>>,
}

impl Renderer for TextureUploads {
    fn backend_name(&self) -> &str {
        "texture uploads"
    }
    fn clear(&mut self, _: f32, _: f32, _: f32) {}
    fn present(&mut self) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn set_camera(&mut self, _: &Camera) {}
    fn set_fog(&mut self, _: bool, _: f32, _: f32, _: [f32; 3]) {}
    fn draw_terrain(
        &mut self,
        _: &TerrainGrid,
        _: &[PaletteEntry],
        _: Option<&v2k_render::terrain_tiles::TerrainFrames>,
        _: Option<&v2k_render::terrain_light::TerrainLightWindow>,
        _: u32,
    ) {
    }
    fn draw_sprite(&mut self, _: &[u8], _: u32, _: u32, _: i32, _: i32) {}
    fn draw_material_sprite(
        &mut self,
        _: &[u8],
        _: u32,
        _: u32,
        _: i32,
        _: i32,
        _: WorldSpriteBlend,
    ) {
    }
    fn draw_fullscreen(&mut self, _: &[u8], _: u32, _: u32) {}
    fn draw_color_overlay(&mut self, _: f32, _: f32, _: f32, _: f32) {}
    fn viewport_size(&self) -> (u32, u32) {
        (640, 480)
    }
    fn create_texture(&mut self, rgba: &[u8], width: u32, height: u32) -> Option<TextureId> {
        self.rgba.push(rgba.to_vec());
        self.dimensions.push((width, height));
        Some(TextureId(self.rgba.len() as u32))
    }
    fn create_indexed_model_texture(
        &mut self,
        texture: IndexedModelTexture<'_>,
    ) -> Option<TextureId> {
        self.indexed.push(texture.palette_rgba.to_vec());
        self.create_texture(texture.fallback_rgba, texture.width, texture.height)
    }
}

fn palette_fixture() -> ResourceCache {
    let mut palette = vec![0; 4 + 1024];
    // Give texel 1 unmistakably different base, fixed, and fallback colours.
    for (row, color) in [
        (0, 0x7c00u16),
        (13, 0x7fe0),
        (27, 0x7c1f),
        (28, 0x03e0),
        (31, 0x001f),
    ] {
        let at = 4 + (row * 16 + 1) * 2;
        palette[at..at + 2].copy_from_slice(&color.to_le_bytes());
    }
    let entries = [0, 4, 0x14, 0x0c, 2, 6, 4]
        .into_iter()
        .enumerate()
        .map(|(index, flags)| SpriteEntry {
            entry_idx: index,
            index: index as u16,
            pal_size: flags,
            shade_count: if flags & 2 != 0 {
                0
            } else if index == 6 {
                32
            } else {
                16
            },
            tex_offset: if flags & 2 != 0 { 1 } else { 0 },
            pal_offset: if flags & 2 != 0 { 0 } else { 4 },
            flags: if flags & 2 != 0 {
                0x0001_0002
            } else {
                0x0001_0001
            },
        })
        .collect();
    ResourceCache::new(vec![LevelState {
        source_path: "fixed palette fixture".into(),
        system_level: Some(2),
        fixup_data: None,
        fixup_code: None,
        strings: vec![],
        sprites: Some(SpriteAtlas {
            // Indexed texel 1, followed by two raw RGB555 texels: red/black.
            atlas: vec![1, 0, 0x7c, 0, 0],
            width: 2048,
            height: 1,
            entries,
            rects: vec![
                PixelRect {
                    x_start: 0,
                    y_start: 0,
                    width: 1,
                    height: 1,
                    data_offset: 0,
                },
                PixelRect {
                    x_start: 1,
                    y_start: 0,
                    width: 4,
                    height: 1,
                    data_offset: 0,
                },
            ],
            entry_data: palette,
        }),
        params: None,
        display_modes: None,
        fog_gradient: None,
        color_palettes: None,
        models: None,
        anim_frames: None,
        terrain: None,
        anim_sound: None,
        collision: None,
        level: None,
        linkage: None,
    }])
}

#[test]
fn sprites_and_models_share_indexed_uploads_with_authored_near_rows() {
    let cache = palette_fixture();
    let materials = ModelMaterialCache::new();
    let mut uploads = TextureUploads::default();
    let base = materials.sprite_texture(&cache, &mut uploads, 0).unwrap();
    let fixed = materials.sprite_texture(&cache, &mut uploads, 1).unwrap();
    assert_eq!(uploads.rgba[base.0 as usize - 1], [0, 0, 248, 255]);
    assert_eq!(uploads.rgba[fixed.0 as usize - 1], [0, 0, 248, 255]);
    assert_eq!(uploads.indexed.len(), 2);

    let faces = materials.materials_for(&cache, &mut uploads, &[0x8000, 0x8001]);
    assert_eq!(
        faces
            .iter()
            .map(|face| face.flat_shade_row)
            .collect::<Vec<_>>(),
        [0, 28]
    );
    assert_eq!(
        uploads.indexed.len(),
        2,
        "model draws retain all shade rows"
    );
    for (face, palette) in faces.iter().zip(&uploads.indexed) {
        let texture = face.texture.unwrap();
        assert_eq!(uploads.rgba[texture.0 as usize - 1], [0, 0, 248, 255]);
        assert_eq!(&palette[4..8], &[248, 0, 0, 255]);
        assert_eq!(
            &palette[(13 * 16 + 1) * 4..(13 * 16 + 2) * 4],
            &[248, 248, 0, 255],
            "fogged rows retain authored colors rather than RGB-scaled row 28"
        );
        assert_eq!(
            &palette[(27 * 16 + 1) * 4..(27 * 16 + 2) * 4],
            &[248, 0, 248, 255]
        );
        assert_eq!(
            &palette[(28 * 16 + 1) * 4..(28 * 16 + 2) * 4],
            &[0, 248, 0, 255]
        );
    }
    assert_eq!(
        materials.sprite_texture(&cache, &mut uploads, 0),
        Some(base)
    );
    assert_eq!(
        materials.sprite_texture(&cache, &mut uploads, 1),
        Some(fixed)
    );
    assert_eq!(
        uploads.rgba.len(),
        2,
        "fixed sprites and model faces reuse the same complete palettes"
    );
}

#[test]
fn billboards_keep_indexed_rows_for_every_blend_family() {
    let cache = palette_fixture();
    let materials = ModelMaterialCache::new();
    let mut uploads = TextureUploads::default();
    let billboards = [0, 1, 2, 3].map(|id| Billboard {
        vertex: 0,
        slot: 0,
        id,
        size: 10,
        angle: 0,
        textured: true,
    });
    let faces = materials.materials_for(&cache, &mut uploads, &[0x8000, 0x8001, 0x8002, 0x8003]);
    let resolved = materials.billboard_materials(&cache, &mut uploads, &billboards);
    for ((billboard, face), expected_blend) in resolved.iter().zip(&faces).zip([
        WorldSpriteBlend::Masked,
        WorldSpriteBlend::Masked,
        WorldSpriteBlend::Additive,
        WorldSpriteBlend::HalfAdditive,
    ]) {
        assert_eq!(billboard.face.texture, face.texture);
        assert_eq!(billboard.face.flat_shade_row, face.flat_shade_row);
        assert_eq!(billboard.blend, expected_blend);
        assert_eq!((billboard.width, billboard.height), (1, 1));
    }
    assert_eq!(resolved[0].face.flat_shade_row, 0);
    assert_eq!(resolved[1].face.flat_shade_row, 28);
    assert_eq!(uploads.indexed.len(), 4);
    assert_eq!(
        uploads.rgba.len(),
        4,
        "billboards must not upload fixed-row copies"
    );
}

#[test]
fn raw_texels_keep_colour_and_pixel_aspect_without_palette_uploads() {
    let cache = palette_fixture();
    let materials = ModelMaterialCache::new();
    let mut uploads = TextureUploads::default();
    for id in [4, 5] {
        let texture = materials.sprite_texture(&cache, &mut uploads, id).unwrap();
        assert_eq!(
            uploads.rgba[texture.0 as usize - 1],
            [248, 0, 0, 255, 0, 0, 0, 255]
        );
        assert_eq!(uploads.dimensions[texture.0 as usize - 1], (2, 1));
        assert_eq!(materials.sprite_dimensions(&cache, id), Some((2, 1)));
        let resolved = materials.billboard_materials(
            &cache,
            &mut uploads,
            &[Billboard {
                vertex: 0,
                slot: 0,
                id,
                size: 10,
                angle: 0,
                textured: true,
            }],
        );
        assert_eq!((resolved[0].width, resolved[0].height), (2, 1));
        assert_eq!(resolved[0].face.texture, Some(texture));
        assert_eq!(materials.sprite_dimensions(&cache, id), Some((2, 1)));
    }
    assert_eq!(uploads.rgba.len(), 2, "billboards reuse the raw uploads");
    // A large flat palette is still indexed on disk but uploads direct RGBA.
    let flat = materials.sprite_texture(&cache, &mut uploads, 6).unwrap();
    assert_eq!(uploads.rgba[flat.0 as usize - 1], [248, 0, 0, 255]);
    assert!(uploads.indexed.is_empty());
    assert_eq!(materials.sprite_texture(&cache, &mut uploads, 999), None);
}

#[test]
fn solid_materials_retain_packed_palette_and_keep_scene_tint_separate() {
    use v2k_game::model_tree::{apply_model_scene_light, ModelSceneLight};

    let cache = palette_fixture();
    let materials = ModelMaterialCache::new();
    let mut uploads = TextureUploads::default();
    let palette = [PaletteEntry {
        rgb555: 0x1993,
        r: 48,
        g: 96,
        b: 152,
    }];
    let mut faces =
        materials.materials_for_with_palette(&cache, &mut uploads, &[0, 3, 0x8000], &palette);
    assert_eq!(faces[0].palette_rgb555, Some(0x1993));
    assert_eq!(faces[0].color, [1.0; 3]);
    assert_eq!(
        faces[0].unlit_color(),
        [48.0 / 255.0, 96.0 / 255.0, 152.0 / 255.0]
    );
    assert_eq!(faces[1].palette_rgb555, None);
    assert_eq!(faces[1].unlit_color(), [0.6; 3]);
    assert_eq!(faces[2].palette_rgb555, None);
    assert_eq!(faces[2].unlit_color(), [1.0; 3]);

    let light = ModelSceneLight {
        tint: [0.5, 0.25, 1.0],
        emissive: [0.1, 0.0, 0.2],
    };
    apply_model_scene_light(&mut faces, light);
    assert_eq!(faces[0].palette_rgb555, Some(0x1993));
    assert_eq!(faces[0].color, light.tint);
    assert_eq!(
        faces[0].unlit_color(),
        [24.0 / 255.0, 24.0 / 255.0, 152.0 / 255.0]
    );
    assert_eq!(faces[0].emissive, light.emissive);
}
