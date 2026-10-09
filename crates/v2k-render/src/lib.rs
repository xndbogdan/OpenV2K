pub mod audio;
pub mod camera;
pub mod config;
pub mod edge_quads;
mod gl;
pub mod gl_backend;
mod gouraud_solid;
mod model_fog;
mod model_near;
pub mod music;
pub mod projection;
pub mod renderer;
pub mod software;
pub mod sound;
pub mod sw_backend;
mod sw_model;
mod sw_painter;
mod terrain_footprint;
pub mod terrain_light;
pub mod terrain_tiles;
pub mod ui_mapping;
pub mod water;
pub mod window;

pub use audio::AudioPlayer;
pub use camera::Camera;
pub use config::{
    ControllerLayout, Difficulty, DisplayModes, DisplayRequest, GameConfig, RendererChoice,
    ScalingMode, WindowMode, ORIGINAL_RESOLUTIONS,
};
pub use music::MusicPlayer;
pub use projection::{
    project_particle_center, retail_underwater_projected_point, retail_underwater_projection,
    ParticleCenterProjection, ProjectedPoint, ProjectionEffect, UnderwaterProjection,
};
pub use renderer::{
    mat3_mul, orientation_f32, orientation_from_ypr, BillboardMaterial, CapturedFrame,
    ExternalFrameMode, FaceMaterial, FrameCaptureSource, IndexedModelTexture, ModelBillboardDraw,
    ModelDepthFade, ModelDepthPolicy, ModelDraw, ModelMesh, ModelNearClip, ModelOverlayKind,
    ModelPainterNode, ModelTransform, NativeModelFogPass, NativePalette, NativeParticle,
    NativeParticleShadow, NativeSprite, NativeTexels, OverlayDepthPolicy, RenderBackend,
    RenderScene, Renderer, SpriteFog, TextureId, ViewPinMode, WorldModelFog, WorldSprite,
    WorldSpriteBlend, WorldSurfaceProjection, HALF_ADDITIVE_ALPHA,
};
pub use sound::{
    original_positional_mix, LoopingVoiceParams, PositionalMix, SoundListener, SoundManager,
    VoiceHandle,
};
pub use terrain_light::TerrainLightWindow;
pub use terrain_tiles::TerrainFrames;
pub use ui_mapping::{UiAnchor, UiMapping, UiMappingRequest, UiSubmissionPolicy};
pub use water::WaterFrames;
pub use window::{
    apply_app_window_icon, declare_dpi_awareness, ControllerSample, GameEvent, GameWindow,
    PadButton, PadSnapshot, PointerMode, WindowPlacement,
};

pub use gl_backend::{authored_face_plane_visible, GlRenderer};
pub use sw_backend::SoftwareRenderer;

/// Create a renderer for the given backend.
///
/// The `GameWindow` provides the SDL2 video subsystem; this function creates
/// the actual OS window and renderer together (since the software backend
/// consumes the window via `into_canvas`).
///
/// If `preferred` is OpenGL and creation fails, falls back to software
/// automatically when `auto_fallback` is true. `placement` reopens a replaced
/// renderer's window where the old one was; `None` centres it.
pub fn create_renderer(
    game_window: &GameWindow,
    title: &str,
    width: u32,
    height: u32,
    preferred: RenderBackend,
    auto_fallback: bool,
    placement: Option<WindowPlacement>,
) -> Result<(Box<dyn Renderer>, RenderBackend), String> {
    let (width, height) = placement.map_or((width, height), |placement| {
        placement.size_or(width, height)
    });
    match preferred {
        RenderBackend::OpenGL => {
            let window = game_window.create_gl_window(title, width, height, placement)?;
            match GlRenderer::new(window, width, height) {
                Ok(gl) => Ok((Box::new(gl), RenderBackend::OpenGL)),
                Err(e) => {
                    if auto_fallback {
                        eprintln!("OpenGL init failed ({}), falling back to software", e);
                        let sw_window =
                            game_window.create_sw_window(title, width, height, placement)?;
                        let sw = SoftwareRenderer::new(sw_window, width, height)?;
                        Ok((Box::new(sw), RenderBackend::Software))
                    } else {
                        Err(format!("OpenGL init failed: {}", e))
                    }
                }
            }
        }
        RenderBackend::Software => {
            let window = game_window.create_sw_window(title, width, height, placement)?;
            let sw = SoftwareRenderer::new(window, width, height)?;
            Ok((Box::new(sw), RenderBackend::Software))
        }
    }
}
