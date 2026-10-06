use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use v2k_formats::system::PaletteEntry;
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

use crate::camera::Camera;
use crate::config::ScalingMode;
use crate::renderer::{
    CapturedFrame, FrameCaptureSource, RenderViewport, Renderer, WorldSpriteBlend,
};

/// Software renderer stub.
///
/// Renders a top-down terrain heightmap view. Models and sprites are no-ops.
/// Enough to verify the fallback path works.
pub struct SoftwareRenderer {
    canvas: Canvas<Window>,
    output_width: u32,
    output_height: u32,
    reference_width: u32,
    reference_height: u32,
    width: u32,
    height: u32,
    scaling_mode: ScalingMode,
    ui_submission_policy: crate::ui_mapping::UiSubmissionPolicy,
    viewport: RenderViewport,
    framebuffer: Vec<u8>, // RGBA
}

impl SoftwareRenderer {
    pub fn new(window: Window, width: u32, height: u32) -> Result<Self, String> {
        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let framebuffer = vec![0u8; (width * height * 4) as usize];

        Ok(Self {
            canvas,
            output_width: width,
            output_height: height,
            reference_width: width,
            reference_height: height,
            width,
            height,
            scaling_mode: ScalingMode::Native,
            ui_submission_policy: Default::default(),
            viewport: RenderViewport::for_output(width, height, width, height, ScalingMode::Native),
            framebuffer,
        })
    }

    fn update_render_viewport(&mut self) {
        self.viewport = RenderViewport::for_output(
            self.output_width,
            self.output_height,
            self.reference_width,
            self.reference_height,
            self.scaling_mode,
        );
        self.width = self.viewport.logical_width;
        self.height = self.viewport.logical_height;
        self.framebuffer = vec![0u8; (self.width * self.height * 4) as usize];
    }
}

impl Renderer for SoftwareRenderer {
    fn backend_name(&self) -> &str {
        "Software"
    }

    fn clear(&mut self, r: f32, g: f32, b: f32) {
        let rb = (r * 255.0) as u8;
        let gb = (g * 255.0) as u8;
        let bb = (b * 255.0) as u8;
        for pixel in self.framebuffer.chunks_exact_mut(4) {
            pixel[0] = rb;
            pixel[1] = gb;
            pixel[2] = bb;
            pixel[3] = 255;
        }
    }

    fn present(&mut self) {
        let tc = self.canvas.texture_creator();
        let mut texture = tc
            .create_texture_streaming(PixelFormatEnum::RGBA32, self.width, self.height)
            .unwrap();
        let pitch = (self.width * 4) as usize;
        texture.update(None, &self.framebuffer, pitch).unwrap();
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        let dest = Rect::new(
            self.viewport.x,
            // SDL's destination rectangles use a top-left origin. The
            // viewport is centered, so the same inset works on either axis.
            self.viewport.y,
            self.viewport.physical_width,
            self.viewport.physical_height,
        );
        self.canvas.copy(&texture, None, Some(dest)).unwrap();
        self.canvas.present();
    }

    fn capture_frame(&mut self, _source: FrameCaptureSource) -> Option<CapturedFrame> {
        // This stub keeps one CPU framebuffer: present only blits it, with no
        // separate display color conversion or front-buffer pixel storage.
        // Both sources therefore copy the same current logical image.
        Some(CapturedFrame {
            rgba: self.framebuffer.clone(),
            width: self.width,
            height: self.height,
        })
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.output_width = width.max(1);
        self.output_height = height.max(1);
        self.update_render_viewport();
    }

    fn set_camera(&mut self, _camera: &Camera) {
        // No-op for top-down view
    }

    fn set_fog(&mut self, _enabled: bool, _near: f32, _far: f32, _color: [f32; 3]) {
        // No-op
    }

    fn draw_terrain(
        &mut self,
        terrain: &TerrainGrid,
        colors: &[PaletteEntry],
        _frames: Option<&crate::terrain_tiles::TerrainFrames>,
        _lights: Option<&crate::terrain_light::TerrainLightWindow>,
        _elapsed_micros: u32,
    ) {
        let scale_x = self.width as f32 / GRID_SIZE as f32;
        let scale_y = self.height as f32 / GRID_SIZE as f32;
        let (h_min, h_max) = terrain.height_range();
        let h_range = (h_max - h_min).max(1) as f32;

        for z in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                let cell = terrain.cell(x, z).unwrap();
                let brightness = (cell.height - h_min) as f32 / h_range;

                let (br, bg, bb) = if (cell.terrain_type as usize) < colors.len() {
                    let c = &colors[cell.terrain_type as usize];
                    (c.r, c.g, c.b)
                } else {
                    (100, 150, 80)
                };

                let r = (br as f32 * brightness) as u8;
                let g = (bg as f32 * brightness) as u8;
                let b = (bb as f32 * brightness) as u8;

                let px_start = (x as f32 * scale_x) as u32;
                let py_start = (z as f32 * scale_y) as u32;
                let px_end = ((x + 1) as f32 * scale_x) as u32;
                let py_end = ((z + 1) as f32 * scale_y) as u32;

                for py in py_start..py_end.min(self.height) {
                    for px in px_start..px_end.min(self.width) {
                        let offset = ((py * self.width + px) * 4) as usize;
                        if offset + 3 < self.framebuffer.len() {
                            self.framebuffer[offset] = r;
                            self.framebuffer[offset + 1] = g;
                            self.framebuffer[offset + 2] = b;
                            self.framebuffer[offset + 3] = 255;
                        }
                    }
                }
            }
        }
    }

    fn supports_models(&self) -> bool {
        false
    }

    fn draw_sprite(&mut self, _rgba: &[u8], _width: u32, _height: u32, _x: i32, _y: i32) {
        // No-op stub
    }

    fn draw_material_sprite(
        &mut self,
        _rgba: &[u8],
        _width: u32,
        _height: u32,
        _x: i32,
        _y: i32,
        _blend: WorldSpriteBlend,
    ) {
        // Models and sprites are intentionally unsupported by this diagnostic
        // fallback; keep that limitation explicit at the material API boundary.
    }

    fn draw_fullscreen(&mut self, rgba: &[u8], src_w: u32, src_h: u32) {
        if rgba.is_empty() || src_w == 0 || src_h == 0 {
            return;
        }
        // Nearest-neighbor scale to framebuffer
        for y in 0..self.height {
            let src_y = (y * src_h / self.height).min(src_h - 1);
            for x in 0..self.width {
                let src_x = (x * src_w / self.width).min(src_w - 1);
                let src_off = ((src_y * src_w + src_x) * 4) as usize;
                let dst_off = ((y * self.width + x) * 4) as usize;
                if src_off + 3 < rgba.len() && dst_off + 3 < self.framebuffer.len() {
                    self.framebuffer[dst_off..dst_off + 4]
                        .copy_from_slice(&rgba[src_off..src_off + 4]);
                }
            }
        }
    }

    fn draw_color_overlay(&mut self, r: f32, g: f32, b: f32, a: f32) {
        if a < 0.001 {
            return;
        }
        let sr = (r * 255.0) as u32;
        let sg = (g * 255.0) as u32;
        let sb = (b * 255.0) as u32;
        let sa = (a * 255.0).min(255.0) as u32;
        let inv_a = 255 - sa;
        for pixel in self.framebuffer.chunks_exact_mut(4) {
            pixel[0] = ((pixel[0] as u32 * inv_a + sr * sa) / 255) as u8;
            pixel[1] = ((pixel[1] as u32 * inv_a + sg * sa) / 255) as u8;
            pixel[2] = ((pixel[2] as u32 * inv_a + sb * sa) / 255) as u8;
        }
    }

    fn viewport_size(&self) -> (u32, u32) {
        (self.width, self.height)
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
