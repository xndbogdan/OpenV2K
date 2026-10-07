//! Software backend: the retail 16-bpp raster drawing into an RGB565 surface.
//!
//! The surface has the logical size of the current [`RenderViewport`] and is
//! drawn by [`crate::software`]'s byte-exact fill slots. Queued primitives
//! are drained through the retail primitive queue before any immediate 2-D
//! draw, capture or presentation, as the original flushes its world queue
//! before the overlays that call fill slots directly.
//!
//! Draw calls whose inputs the port only holds as RGBA images are adapted
//! at this boundary: an RGBA image becomes a raw-texel material (exact for
//! colours that came from RGB565 palette words). 3-D producers are not
//! ported yet, so [`Renderer::supports_models`] stays false and callers keep
//! their 2-D fallbacks; terrain, water and world sprites draw nothing.

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use v2k_formats::system::PaletteEntry;
use v2k_formats::terrain::TerrainGrid;

use crate::camera::Camera;
use crate::config::ScalingMode;
use crate::projection::{NativeViewportWords, ProjectionEffect, SceneProjectionAuthority};
use crate::renderer::{
    CapturedFrame, FrameCaptureSource, RenderScene, RenderViewport, Renderer, TextureId,
    WorldModelFog, WorldSpriteBlend,
};
use crate::software::store::{rgb565, rgba8, MaterialStore, OwnedMaterial};
use crate::software::terrain::{
    draw_ground, ground_lead, GroundMaterial, GroundProjection, GroundScene,
};
use crate::software::{
    material_flags, ClipRect, FillSlot, MaterialId, NoWordImages, PixelFormat, PrimitiveQueue,
    SoftwareRaster, Surface565, WORLD_ARENA_BYTES,
};
use crate::sw_model::{
    queue_model_billboards, queue_model_body, queue_world_sprites, DerivedMaterials,
    ModelMaterials, ModelScene,
};
use crate::terrain_tiles::InfectionTerrainAnimation;

/// ESP at a fill-slot handler's first instruction while the world queue
/// drains. Only its high sixteen bits reach pixels (the `00478510` dither
/// seed); the shipped game's value has not been measured, so this assumes
/// the usual Win32 main-thread stack page.
const HANDLER_ESP: u32 = 0x0019_F000;

pub struct SoftwareRenderer {
    canvas: Canvas<Window>,
    output_width: u32,
    output_height: u32,
    reference_width: u32,
    reference_height: u32,
    scaling_mode: ScalingMode,
    ui_submission_policy: crate::ui_mapping::UiSubmissionPolicy,
    viewport: RenderViewport,
    surface: Surface565,
    raster: SoftwareRaster,
    queue: PrimitiveQueue,
    /// Bound materials; persistent textures use their [`TextureId`] value.
    materials: MaterialStore,
    /// Records queued since the last drain.
    queued: bool,
    /// Optional 2-D clip in surface pixels (`+0x102C` packets).
    sprite_clip: Option<ClipRect>,
    /// `V2K_SOFTWARE_FRAME_DUMP`: directory receiving every presented frame
    /// as a 24-bit BMP, for headless inspection and frame comparison.
    frame_dump: Option<FrameDump>,
    scene: RenderScene,
    authority: SceneProjectionAuthority,
    effect: ProjectionEffect,
    world_fog: Option<WorldModelFog>,
    native_viewport: Option<NativeViewportWords>,
    /// `FUN_00433530`'s process state, advanced only by terrain draws.
    infection: InfectionTerrainAnimation,
    /// Ground queue errors are reported once.
    ground_error_reported: bool,
    camera_position: [f32; 3],
    camera_basis: [[f32; 3]; 3],
    camera_lens: Option<FloatLens>,
    /// Sprite materials re-flagged per face (blend, row 28).
    derived: DerivedMaterials,
    model_error_reported: bool,
}

/// The floating camera's lens, for scenes without a native one.
#[derive(Debug, Clone, Copy)]
struct FloatLens {
    fov: f32,
    offset: [f32; 2],
}

struct FrameDump {
    directory: std::path::PathBuf,
    next: u32,
}

impl SoftwareRenderer {
    pub fn new(window: Window, width: u32, height: u32) -> Result<Self, String> {
        // Present the logical surface with nearest-neighbour scaling.
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");
        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let viewport =
            RenderViewport::for_output(width, height, width, height, ScalingMode::Native);
        let surface = Surface565::new(viewport.logical_width, viewport.logical_height);
        let raster = SoftwareRaster::new(full_clip(&surface), PixelFormat::RGB565, HANDLER_ESP);
        Ok(Self {
            canvas,
            output_width: width,
            output_height: height,
            reference_width: width,
            reference_height: height,
            scaling_mode: ScalingMode::Native,
            ui_submission_policy: Default::default(),
            viewport,
            surface,
            raster,
            queue: PrimitiveQueue::new(WORLD_ARENA_BYTES).expect("world arena size"),
            materials: MaterialStore::default(),
            queued: false,
            sprite_clip: None,
            frame_dump: std::env::var_os("V2K_SOFTWARE_FRAME_DUMP").map(|directory| FrameDump {
                directory: directory.into(),
                next: 0,
            }),
            scene: RenderScene::Menu,
            authority: SceneProjectionAuthority::default(),
            effect: ProjectionEffect::None,
            world_fog: None,
            native_viewport: None,
            infection: InfectionTerrainAnimation::default(),
            ground_error_reported: false,
            camera_position: [0.0; 3],
            camera_basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            camera_lens: None,
            derived: DerivedMaterials::default(),
            model_error_reported: false,
        })
    }

    fn update_render_viewport(&mut self) {
        self.flush();
        self.viewport = RenderViewport::for_output(
            self.output_width,
            self.output_height,
            self.reference_width,
            self.reference_height,
            self.scaling_mode,
        );
        let (width, height) = (self.viewport.logical_width, self.viewport.logical_height);
        if (self.surface.width, self.surface.height) != (width, height) {
            self.surface = Surface565::new(width, height);
            self.sprite_clip = None;
            self.raster.set_clip(full_clip(&self.surface));
        }
    }

    /// Drain queued primitives into the surface (`FUN_004948F0`).
    fn flush(&mut self) {
        if !self.queued {
            return;
        }
        let Self {
            queue,
            raster,
            surface,
            materials,
            ..
        } = self;
        queue.flush(&mut |slot, payload| {
            raster.draw(slot, payload, surface, materials, &NoWordImages);
            0
        });
        queue.reset();
        self.queued = false;
    }

    /// The clip a direct 2-D draw uses.
    fn active_clip(&self) -> ClipRect {
        self.sprite_clip.unwrap_or_else(|| full_clip(&self.surface))
    }

    /// Run one fill slot immediately, as overlays that call a thunk directly
    /// do, with a temporary material.
    fn draw_now(&mut self, slot: FillSlot, packet: &[u8], material: Option<OwnedMaterial>) {
        self.flush();
        let id = material.map(|material| self.materials.insert(material));
        let mut packet = packet.to_vec();
        if let Some(id) = id {
            let at = material_offset(slot);
            packet[at..at + 4].copy_from_slice(&id.to_le_bytes());
        }
        self.raster.set_clip(self.active_clip());
        self.raster.draw(
            slot,
            &packet,
            &mut self.surface,
            &self.materials,
            &NoWordImages,
        );
        self.raster.set_clip(full_clip(&self.surface));
        if let Some(id) = id {
            self.materials.remove(id);
        }
    }

    /// An RGBA image as a material sprite at `(x, y)`.
    fn draw_rgba_sprite(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        blend: WorldSpriteBlend,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        match blend {
            // `+0x1030` (thunk `0047A7A0`) composes keyed and half-additive
            // materials but has no additive path.
            WorldSpriteBlend::Masked | WorldSpriteBlend::HalfAdditive => {
                let flags = if blend == WorldSpriteBlend::HalfAdditive {
                    material_flags::HALF_ADDITIVE
                } else {
                    0
                };
                let material = OwnedMaterial::raw_from_rgba(rgba, width, height, flags);
                let mut packet = [0u8; 8];
                packet[4..6].copy_from_slice(&(x as i16).to_le_bytes());
                packet[6..8].copy_from_slice(&(y as i16).to_le_bytes());
                self.draw_now(FillSlot::Sprite, &packet, Some(material));
            }
            // Additive screen materials go through a textured quad's
            // additive rows, mapped one texel per pixel.
            WorldSpriteBlend::Additive => {
                let material = overlay_material(rgba, width, height, material_flags::ADDITIVE);
                let corners = rect_corners(x, y, width as i32, height as i32);
                self.draw_textured_quad(corners, material);
            }
        }
    }

    fn draw_textured_quad(&mut self, corners: [(i32, i32); 4], material: OwnedMaterial) {
        let mut packet = [0u8; 0x14];
        for (index, (x, y)) in corners.into_iter().enumerate() {
            packet[4 * index..4 * index + 2].copy_from_slice(&(x as i16).to_le_bytes());
            packet[4 * index + 2..4 * index + 4].copy_from_slice(&(y as i16).to_le_bytes());
        }
        self.draw_now(FillSlot::TexturedQuad, &packet, Some(material));
    }

    fn surface_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((self.surface.width * self.surface.height * 4) as usize);
        for y in 0..self.surface.height {
            for &pixel in self.surface.row(y) {
                rgba.extend_from_slice(&rgba8(pixel));
            }
        }
        rgba
    }
}

/// Sprite materials for model faces, with the face's flags folded in.
struct BackendMaterials<'a> {
    store: &'a mut MaterialStore,
    derived: &'a mut DerivedMaterials,
}

impl ModelMaterials for BackendMaterials<'_> {
    fn face_material(&mut self, texture: u32, flags: u16) -> Option<MaterialId> {
        if let Some(&id) = self.derived.by_flags.get(&(texture, flags)) {
            return Some(id);
        }
        let base = self.store.get(texture)?;
        let id = if flags == 0 {
            texture
        } else {
            let mut material = base.clone();
            material.flags |= flags;
            self.store.insert(material)
        };
        self.derived.by_flags.insert((texture, flags), id);
        Some(id)
    }
}

impl SoftwareRenderer {
    fn model_scene(&self) -> Option<ModelScene> {
        Some(ModelScene {
            camera_position: self.camera_position,
            camera_basis: self.camera_basis,
            native: match self.authority {
                SceneProjectionAuthority::Native(_) => self.native_viewport,
                _ => None,
            },
            lens: self.model_lens()?,
            world_fog: self.world_model_fog(),
        })
    }

    /// The lens model bodies project with: the scene's native lens, else one
    /// derived from the floating camera over the logical surface.
    fn model_lens(&self) -> Option<GroundProjection> {
        let identity = [[0; 3]; 3];
        let wet_clock = match self.effect {
            ProjectionEffect::RetailUnderwater { tick } => Some(tick as u32),
            ProjectionEffect::None => None,
        };
        if let SceneProjectionAuthority::Native(lens) = self.authority {
            let viewport = lens.viewport_pixels();
            return Some(GroundProjection {
                axes_q31: identity,
                translation: [0; 3],
                focal: lens.focal_pixels(),
                bounds: viewport.map(|value| value as u32),
                centre: lens.centre_pixels(),
                fade: [0, i32::MAX, i32::MAX],
                wet_clock,
            });
        }
        let camera = self.camera_lens?;
        let (width, height) = (self.surface.width as f32, self.surface.height as f32);
        let focal = height * 0.5 / (camera.fov * 0.5).tan();
        Some(GroundProjection {
            axes_q31: identity,
            translation: [0; 3],
            focal: [focal.round() as i32; 2],
            bounds: [self.surface.width, self.surface.height],
            centre: [
                (width * 0.5 * (1.0 - camera.offset[0])) as i32,
                (height * 0.5 * (1.0 + camera.offset[1])) as i32,
            ],
            fade: [0, i32::MAX, i32::MAX],
            wet_clock,
        })
    }
}

/// Write `surface` as a bottom-up 24-bit BMP.
fn write_bmp(path: &std::path::Path, surface: &Surface565) -> std::io::Result<()> {
    let (width, height) = (surface.width as usize, surface.height as usize);
    let row_bytes = (width * 3 + 3) & !3;
    let image_bytes = row_bytes * height;
    let mut bytes = Vec::with_capacity(54 + image_bytes);
    bytes.extend_from_slice(b"BM");
    bytes.extend_from_slice(&((54 + image_bytes) as u32).to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&54u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(width as i32).to_le_bytes());
    bytes.extend_from_slice(&(height as i32).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&24u16.to_le_bytes());
    bytes.extend_from_slice(&[0; 24]);
    for y in (0..height).rev() {
        let start = bytes.len();
        for &pixel in surface.row(y as u32) {
            let [r, g, b, _] = rgba8(pixel);
            bytes.extend_from_slice(&[b, g, r]);
        }
        bytes.resize(start + row_bytes, 0);
    }
    std::fs::write(path, bytes)
}

fn full_clip(surface: &Surface565) -> ClipRect {
    ClipRect {
        x0: 0,
        y0: 0,
        x1: surface.width as i16,
        y1: surface.height as i16,
    }
}

/// A composited overlay image as a textured-quad material: indexed (as
/// retail's overlay sprites are) when its colours fit a palette, else raw.
fn overlay_material(rgba: &[u8], width: u32, height: u32, flags: u16) -> OwnedMaterial {
    OwnedMaterial::indexed_from_rgba(rgba, width, height, flags)
        .unwrap_or_else(|| OwnedMaterial::raw_from_rgba(rgba, width, height, flags))
}

fn rect_corners(x: i32, y: i32, width: i32, height: i32) -> [(i32, i32); 4] {
    [
        (x, y),
        (x + width, y),
        (x + width, y + height),
        (x, y + height),
    ]
}

/// Packet offset of the material dword the backend fills for `slot`.
fn material_offset(slot: FillSlot) -> usize {
    match slot {
        FillSlot::Sprite => 0,
        FillSlot::TexturedQuad => 0x10,
        other => unreachable!("{other:?} carries no backend material"),
    }
}

/// RGB565 of a 0..1 colour.
fn colour565(r: f32, g: f32, b: f32) -> u16 {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0) as u8;
    rgb565(byte(r), byte(g), byte(b))
}

impl Renderer for SoftwareRenderer {
    fn backend_name(&self) -> &str {
        "Software"
    }

    fn begin_scene(&mut self, scene: RenderScene) {
        self.flush();
        self.scene = scene;
        self.authority = SceneProjectionAuthority::default();
        self.effect = ProjectionEffect::None;
        self.sprite_clip = None;
        self.raster.set_clip(full_clip(&self.surface));
    }

    fn set_projection_effect(&mut self, effect: ProjectionEffect) {
        self.effect = effect;
    }

    fn set_scene_projection_authority(&mut self, authority: SceneProjectionAuthority) {
        self.authority = authority;
    }

    fn scene_projection_authority(&self) -> SceneProjectionAuthority {
        self.authority
    }

    fn set_native_world_viewport(&mut self, viewport: Option<NativeViewportWords>) {
        self.native_viewport = viewport;
    }

    fn set_world_model_fog(&mut self, fog: Option<WorldModelFog>) {
        self.world_fog = fog;
    }

    fn world_model_fog(&self) -> Option<WorldModelFog> {
        (self.scene == RenderScene::World)
            .then_some(self.world_fog)
            .flatten()
    }

    fn world_fog_planes(&self) -> Option<[f32; 2]> {
        self.world_model_fog().map(|fog| {
            [
                fog.planes.near_raw as f32 / 256.0,
                fog.planes.far_raw as f32 / 256.0,
            ]
        })
    }

    fn clear(&mut self, r: f32, g: f32, b: f32) {
        self.flush();
        self.surface.pixels.fill(colour565(r, g, b));
    }

    fn present(&mut self) {
        self.flush();
        if let Some(dump) = &mut self.frame_dump {
            let path = dump.directory.join(format!("frame{:06}.bmp", dump.next));
            dump.next += 1;
            if let Err(error) = write_bmp(&path, &self.surface) {
                eprintln!("software frame dump {}: {error}", path.display());
            }
        }
        let (width, height) = (self.surface.width, self.surface.height);
        let creator = self.canvas.texture_creator();
        let Ok(mut texture) =
            creator.create_texture_streaming(PixelFormatEnum::RGB565, width, height)
        else {
            return;
        };
        let bytes: Vec<u8> = self
            .surface
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_le_bytes())
            .collect();
        if texture.update(None, &bytes, self.surface.pitch).is_err() {
            return;
        }
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        let dest = Rect::new(
            self.viewport.x,
            self.viewport.y,
            self.viewport.physical_width,
            self.viewport.physical_height,
        );
        let _ = self.canvas.copy(&texture, None, Some(dest));
        self.canvas.present();
    }

    fn capture_frame(&mut self, _source: FrameCaptureSource) -> Option<CapturedFrame> {
        // Presentation only scales the surface, so both sources are the
        // current RGB565 image.
        self.flush();
        Some(CapturedFrame {
            rgba: self.surface_rgba(),
            width: self.surface.width,
            height: self.surface.height,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.output_width = width.max(1);
        self.output_height = height.max(1);
        self.update_render_viewport();
    }

    fn set_camera(&mut self, camera: &Camera) {
        self.camera_position = camera.position;
        let view = camera.view_matrix();
        self.camera_basis = [
            [view[0], view[4], view[8]],
            [view[1], view[5], view[9]],
            [view[2], view[6], view[10]],
        ];
        self.camera_lens = Some(FloatLens {
            fov: camera.fov,
            offset: camera.projection_offset,
        });
    }

    fn set_fog(&mut self, _enabled: bool, _near: f32, _far: f32, _color: [f32; 3]) {}

    /// `FUN_0042F960`: queue the opaque ground through the retail scan. It
    /// needs the scene's native viewport, lens and native terrain sprites;
    /// without them nothing is drawn.
    fn draw_terrain(
        &mut self,
        terrain: &TerrainGrid,
        colors: &[PaletteEntry],
        frames: Option<&crate::terrain_tiles::TerrainFrames>,
        lights: Option<&crate::terrain_light::TerrainLightWindow>,
        elapsed_micros: u32,
    ) {
        self.infection.advance(elapsed_micros);
        let (Some(frames), Some(viewport), SceneProjectionAuthority::Native(lens)) =
            (frames, self.native_viewport, self.authority)
        else {
            return;
        };
        let Some(native) = frames.native.as_ref() else {
            return;
        };
        // 0x004FEEE8/EC/F0: the fade ramp between the world fog planes.
        let fade = match self.world_model_fog() {
            Some(fog) if fog.planes.far_raw > fog.planes.near_raw => [
                0x0100_0000 / (fog.planes.far_raw - fog.planes.near_raw),
                fog.planes.near_raw,
                fog.planes.far_raw,
            ],
            _ => [0, i32::MAX, i32::MAX],
        };
        let viewport_pixels = lens.viewport_pixels();
        let projection = GroundProjection {
            axes_q31: viewport.axes_q31,
            translation: [0; 3],
            focal: lens.focal_pixels(),
            bounds: viewport_pixels.map(|value| value as u32),
            centre: lens.centre_pixels(),
            fade,
            wet_clock: match self.effect {
                ProjectionEffect::RetailUnderwater { tick } => Some(tick as u32),
                ProjectionEffect::None => None,
            },
        };
        let empty_light = [0i8; 1024];
        let (light, light_origin) = lights.map_or((&empty_light, [0, 0]), |lights| lights.window());
        let motion = self.infection.frame();
        let display = |rgb555: u16| u32::from(((rgb555 & 0x7FE0) << 1) | (rgb555 & 0x1F));
        let fog_colour = self.world_fog.map_or(0, |fog| {
            let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0) as u8;
            u32::from(rgb565(
                byte(fog.color[0]),
                byte(fog.color[1]),
                byte(fog.color[2]),
            ))
        });
        let sprites = &native.sprites;
        let tile_base = native.tile_base;
        let sprite = |index: u32| {
            let local = index.wrapping_sub(tile_base) as usize;
            let record = sprites.get(local).or_else(|| sprites.first()).copied();
            record.map_or(
                GroundMaterial {
                    id: 0,
                    width: 1,
                    height: 1,
                },
                |record| GroundMaterial {
                    id: record.id,
                    width: record.width,
                    height: record.height,
                },
            )
        };
        let scene = GroundScene {
            grid: terrain,
            projection,
            eye: viewport.origin_raw.map(|value| value as i16),
            lead: ground_lead(viewport.axes_q31[2][1]),
            screen_height: viewport_pixels[1] as i16,
            shade_words: native.shade_words,
            fog_colour,
            cap_colour: colors.get(11).map_or(0, |entry| display(entry.rgb555)),
            rows: frames.scan_columns,
            points: frames.scan_rows.clamp(3, 30),
            light,
            light_origin,
            infection_offsets: motion.offsets(),
            infection_selectors: motion.selectors(),
            tiles: &frames.lookup,
            tile_base,
            infection_base: tile_base + crate::terrain_tiles::INFECTION_BASE_OFFSET,
            sprite: &sprite,
        };
        match draw_ground(&mut self.queue, &scene) {
            Ok(()) => {}
            Err(error) if !self.ground_error_reported => {
                self.ground_error_reported = true;
                eprintln!("software ground: {error:?}; the rest of the ground is not queued");
            }
            Err(_) => {}
        }
        self.queued = true;
    }

    fn supports_models(&self) -> bool {
        true
    }

    fn draw_world_sprites(&mut self, sprites: &[crate::renderer::WorldSprite]) {
        let Some(scene) = self.model_scene() else {
            return;
        };
        let mut materials = BackendMaterials {
            store: &mut self.materials,
            derived: &mut self.derived,
        };
        if let Err(error) = queue_world_sprites(&mut self.queue, sprites, &scene, &mut materials) {
            if !self.model_error_reported {
                self.model_error_reported = true;
                eprintln!("software sprites: {error:?}; later ones this frame are dropped");
            }
        }
        self.queued = true;
    }

    fn draw_model_billboards(&mut self, draw: crate::renderer::ModelBillboardDraw<'_>) {
        let Some(scene) = self.model_scene() else {
            return;
        };
        let mut materials = BackendMaterials {
            store: &mut self.materials,
            derived: &mut self.derived,
        };
        if let Err(error) = queue_model_billboards(&mut self.queue, &draw, &scene, &mut materials) {
            if !self.model_error_reported {
                self.model_error_reported = true;
                eprintln!("software billboards: {error:?}; later ones this frame are dropped");
            }
        }
        self.queued = true;
    }

    fn draw_model_body(&mut self, draw: crate::renderer::ModelDraw<'_>) {
        let Some(scene) = self.model_scene() else {
            return;
        };
        let mut materials = BackendMaterials {
            store: &mut self.materials,
            derived: &mut self.derived,
        };
        if let Err(error) = queue_model_body(&mut self.queue, &draw, &scene, &mut materials) {
            if !self.model_error_reported {
                self.model_error_reported = true;
                eprintln!("software models: {error:?}; later faces this frame are dropped");
            }
        }
        self.queued = true;
    }

    fn create_indexed_model_texture(
        &mut self,
        texture: crate::renderer::IndexedModelTexture<'_>,
    ) -> Option<TextureId> {
        let words: Vec<u16> = texture
            .palette_rgba
            .chunks_exact(4)
            .map(|pixel| rgb565(pixel[0], pixel[1], pixel[2]))
            .collect();
        let flags = if texture.transparent_zero {
            crate::software::material_flags::KEYED
        } else {
            0
        };
        let material =
            OwnedMaterial::indexed(texture.indices, texture.width, texture.height, words, flags);
        Some(TextureId(self.materials.insert(material)))
    }

    fn create_texture(&mut self, rgba: &[u8], width: u32, height: u32) -> Option<TextureId> {
        let id = self
            .materials
            .insert(OwnedMaterial::raw_from_rgba(rgba, width, height, 0));
        Some(TextureId(id))
    }

    fn create_native_sprite(&mut self, sprite: crate::renderer::NativeSprite<'_>) -> Option<u32> {
        Some(self.materials.insert(OwnedMaterial::from_native(&sprite)))
    }

    fn destroy_texture(&mut self, id: TextureId) {
        self.flush();
        self.materials.remove(id.0 as MaterialId);
    }

    fn set_sprite_clip(&mut self, rect: Option<(i32, i32, u32, u32)>) {
        self.flush();
        self.sprite_clip = rect.map(|(x, y, width, height)| {
            let surface = full_clip(&self.surface);
            ClipRect {
                x0: (x.max(0) as i16).min(surface.x1),
                y0: (y.max(0) as i16).min(surface.y1),
                x1: ((x + width as i32).clamp(0, i32::from(surface.x1))) as i16,
                y1: ((y + height as i32).clamp(0, i32::from(surface.y1))) as i16,
            }
        });
    }

    fn draw_sprite(&mut self, rgba: &[u8], width: u32, height: u32, x: i32, y: i32) {
        self.draw_rgba_sprite(rgba, width, height, x, y, WorldSpriteBlend::Masked);
    }

    fn draw_material_sprite(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        blend: WorldSpriteBlend,
    ) {
        self.draw_rgba_sprite(rgba, width, height, x, y, blend);
    }

    fn draw_material_sprite_quad(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        corners: [(i32, i32); 4],
        blend: WorldSpriteBlend,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        let flags = match blend {
            WorldSpriteBlend::Masked => 0,
            WorldSpriteBlend::HalfAdditive => material_flags::HALF_ADDITIVE,
            WorldSpriteBlend::Additive => material_flags::ADDITIVE,
        };
        let material = overlay_material(rgba, width, height, flags);
        self.draw_textured_quad(corners, material);
    }

    fn draw_fullscreen(&mut self, rgba: &[u8], src_w: u32, src_h: u32) {
        self.flush();
        if rgba.is_empty() || src_w == 0 || src_h == 0 {
            return;
        }
        let (width, height) = (self.surface.width, self.surface.height);
        let row_words = self.surface.pitch / 2;
        for y in 0..height {
            let src_y = (y * src_h / height).min(src_h - 1);
            for x in 0..width {
                let src_x = (x * src_w / width).min(src_w - 1);
                let at = ((src_y * src_w + src_x) * 4) as usize;
                if let Some(pixel) = rgba.get(at..at + 3) {
                    self.surface.pixels[y as usize * row_words + x as usize] =
                        rgb565(pixel[0], pixel[1], pixel[2]);
                }
            }
        }
    }

    fn draw_color_overlay(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.flush();
        if a < 0.001 {
            return;
        }
        // Port fade adapter: source-alpha blending in 8-bit channels.
        let alpha = (a.min(1.0) * 255.0) as u32;
        let inverse = 255 - alpha;
        let source = [r, g, b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u32);
        for pixel in &mut self.surface.pixels {
            let [pr, pg, pb, _] = rgba8(*pixel);
            let mix = |d: u8, s: u32| ((u32::from(d) * inverse + s * alpha) / 255) as u8;
            *pixel = rgb565(mix(pr, source[0]), mix(pg, source[1]), mix(pb, source[2]));
        }
    }

    fn viewport_size(&self) -> (u32, u32) {
        (self.surface.width, self.surface.height)
    }

    fn ui_submission_policy(&self) -> crate::ui_mapping::UiSubmissionPolicy {
        self.ui_submission_policy
    }

    fn set_ui_submission_policy(&mut self, policy: crate::ui_mapping::UiSubmissionPolicy) {
        self.ui_submission_policy = policy;
    }

    fn authored_pixel_scale(&self) -> f32 {
        self.ui_submission_policy.authored_pixel_scale(
            self.scaling_mode,
            [self.output_width, self.output_height],
            self.reference_height,
        )
    }

    fn set_fullscreen(&mut self, on: bool) {
        use sdl2::video::FullscreenType;
        let mode = if on {
            FullscreenType::Desktop
        } else {
            FullscreenType::Off
        };
        if let Err(e) = self.canvas.window_mut().set_fullscreen(mode) {
            eprintln!("set_fullscreen failed: {e}");
            return;
        }
        if let Ok((w, h)) = self.canvas.output_size() {
            self.resize(w, h);
        }
    }

    fn set_window_size(&mut self, width: u32, height: u32) {
        if let Err(e) = self.canvas.window_mut().set_size(width, height) {
            eprintln!("set window size failed: {e}");
            return;
        }
        if let Ok((w, h)) = self.canvas.output_size() {
            self.resize(w, h);
        }
    }

    fn set_scaling_mode(&mut self, mode: ScalingMode, reference_width: u32, reference_height: u32) {
        self.scaling_mode = mode;
        self.reference_width = reference_width.max(1);
        self.reference_height = reference_height.max(1);
        self.update_render_viewport();
    }
}
