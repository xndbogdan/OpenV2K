use std::collections::HashMap;

mod sprite_quads;
use sprite_quads::{camera_facing_corners, model_billboard_fog, SpriteQuad};

use sdl2::video::{GLContext, Window};

use v2k_formats::models::{
    ModelFaceCull, ModelFaceCullPlane, ModelFaceShading, ModelSlotClip, ModelSurfaceOrigin,
    ResolvedModelSlot,
};
use v2k_formats::system::{FogGradientEntry, PaletteEntry};
use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

use crate::camera::Camera;
use crate::config::ScalingMode;
use crate::model_fog::{model_fog_pass, palette_fog_shade, PaletteFogPass};
use crate::model_near::{face_admitted, vertex_admission, world_view_depth};
use crate::projection::{retail_underwater_offset_table, ProjectionEffect};
use crate::renderer::{
    CapturedFrame, ExternalFrameMode, FaceMaterial, FrameCaptureSource, IndexedModelTexture,
    ModelBillboardDraw, ModelDepthFade, ModelDepthPolicy, ModelDraw, ModelNearClip,
    ModelOverlayKind, ModelSurfaceResolution, OverlayDepthPolicy, RenderScene, RenderViewport,
    Renderer, SpriteFog, TextureId, ViewPinMode, WorldSprite, WorldSpriteBlend,
    WorldSurfaceProjection, FRONTEND_MODEL_UNITS_PER_VIEW, HALF_ADDITIVE_ALPHA,
};
use crate::terrain_footprint::{TerrainFootprint, TerrainScan};

/// Shared GLSL 1.20 implementation of the post-perspective stage used by the
/// four projectors installed by `FUN_00433FA0`. The sine/Q31 results are
/// supplied as an exact 64-entry integer table; the shader mirrors retail's
/// integer truncation, offscreen SAR loop, top-origin phase lookup and
/// clip-space reconstruction after the port's ordinary floating-point
/// perspective transform. Z/W are never changed.
const RETAIL_PROJECTION_GLSL: &str = r#"
uniform float underwater_projection_enabled;
uniform vec2 underwater_projection_offsets[64];
uniform vec2 logical_viewport_size;

float retail_trunc_zero(float value) {
    return value < 0.0 ? ceil(value) : floor(value);
}

void apply_retail_projection(inout vec4 clip) {
    if (underwater_projection_enabled < 0.5 || clip.w <= 0.0 || clip.z < -clip.w) {
        return;
    }

    float screen_x = retail_trunc_zero((clip.x / clip.w * 0.5 + 0.5) * logical_viewport_size.x);
    float screen_y = retail_trunc_zero((0.5 - clip.y / clip.w * 0.5) * logical_viewport_size.y);
    // Retail latches abs(x)|abs(y) before the loop and shifts that control
    // word independently. Recomputing from signed, odd SAR results would add
    // an extra iteration for values such as -16383 -> -8192.
    float control = max(abs(screen_x), abs(screen_y));
    for (int shift = 0; shift < 31; ++shift) {
        if (control < 8192.0) {
            break;
        }
        control = floor(control * 0.5);
        screen_x = floor(screen_x * 0.5);
        screen_y = floor(screen_y * 0.5);
    }

    float table_index = mod(screen_x + screen_y, 64.0);
    vec2 displacement = underwater_projection_offsets[int(floor(table_index))];
    screen_x += displacement.x;
    screen_y += displacement.y;
    clip.x = (screen_x * 2.0 / logical_viewport_size.x - 1.0) * clip.w;
    clip.y = (1.0 - screen_y * 2.0 / logical_viewport_size.y) * clip.w;
}
"#;

#[derive(Debug, Clone, Copy)]
struct ProjectionUniforms {
    enabled: i32,
    offsets: i32,
    viewport_size: i32,
}

struct ModelProgram {
    id: u32,
    index_sampler: i32,
    palette_sampler: i32,
    mode: i32,
    mask_zero: i32,
    opacity: i32,
    palette_fog: i32,
    gouraud_solid: i32,
    solid_tint: i32,
    solid_emissive: i32,
    blend_mode: i32,
    fog_enabled: i32,
    fog_near: i32,
    fog_far: i32,
    fog_color: i32,
    use_explicit_fog_coord: i32,
    projection: ProjectionUniforms,
}

#[derive(Debug, Clone, Copy)]
struct IndexedModelTextures {
    index: TextureId,
    palette: TextureId,
    /// Fixed-function copies for the palette-base and flag-0x04 flat fills,
    /// in MODEL_FLAT_SHADE_ROWS order. Lit faces keep the public brightest-row
    /// fallback for its Section-6 RGB modulation.
    flat_rows: [Option<TextureId>; 2],
    transparent_zero: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResolvedModelDepthFade {
    enabled: bool,
    near: f32,
    far: f32,
    color: [f32; 3],
    fixed_point_domain: Option<FixedPointFadeDomain>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FixedPointFadeDomain {
    near_raw: i32,
    far_raw: i32,
}

impl ResolvedModelDepthFade {
    /// Quantize one projected 8.8 endpoint depth for the retail B/fogged edge
    /// constructor. When the caller retained the original integer plane
    /// domain, this reproduces `FUN_00470A10`'s reciprocal-first arithmetic;
    /// otherwise the generic view-space policy uses its continuous ratio.
    fn byte_at_raw_depth(self, depth_raw: i32) -> u8 {
        if !self.enabled {
            return 0;
        }
        if let Some(domain) = self.fixed_point_domain {
            let source_depth = ((f64::from(depth_raw) / 256.0)
                * f64::from(FRONTEND_MODEL_UNITS_PER_VIEW))
            .round() as i64;
            return retail_fixed_fade_byte(
                source_depth,
                i64::from(domain.near_raw),
                i64::from(domain.far_raw),
            );
        }
        let depth = depth_raw as f32 / 256.0;
        if self.near >= self.far {
            return if depth >= self.near { u8::MAX } else { 0 };
        }
        if depth < self.near {
            return 0;
        }
        if depth >= self.far {
            return u8::MAX;
        }
        ((((depth - self.near) / (self.far - self.near)) * 256.0).floor() as i32)
            .clamp(0, i32::from(u8::MAX)) as u8
    }
}

fn retail_fixed_fade_byte(depth_raw: i64, near_raw: i64, far_raw: i64) -> u8 {
    if near_raw >= far_raw {
        return if depth_raw >= near_raw { u8::MAX } else { 0 };
    }
    if depth_raw < near_raw {
        return 0;
    }
    if depth_raw >= far_raw {
        return u8::MAX;
    }
    let reciprocal = 0x0100_0000_i64 / (far_raw - near_raw);
    (((depth_raw - near_raw) * reciprocal) >> 16).clamp(0, i64::from(u8::MAX)) as u8
}

fn edge_endpoint_fade_bytes(
    depth_fade: ResolvedModelDepthFade,
    endpoint_depths_raw: [i32; 2],
) -> [u8; 2] {
    endpoint_depths_raw.map(|depth| depth_fade.byte_at_raw_depth(depth))
}

fn edge_quad_fade_bytes(
    depth_fade: ResolvedModelDepthFade,
    endpoint_depths_raw: [i32; 2],
) -> Option<[u8; 4]> {
    let [start, end] = edge_endpoint_fade_bytes(depth_fade, endpoint_depths_raw);
    ([start, end] != [u8::MAX; 2]).then_some([start, start, end, end])
}

fn selected_model_fog_pass(policy: ModelDepthFade) -> Option<PaletteFogPass> {
    match policy {
        ModelDepthFade::WorldRaw {
            pass: crate::renderer::NativeModelFogPass::Near,
            ..
        } => Some(PaletteFogPass::Near),
        ModelDepthFade::WorldRaw {
            pass: crate::renderer::NativeModelFogPass::Fog,
            ..
        } => Some(PaletteFogPass::Far),
        _ => None,
    }
}

fn resolve_model_depth_fade(
    policy: ModelDepthFade,
    world_enabled: bool,
    world_near: f32,
    world_far: f32,
    world_color: [f32; 3],
) -> ResolvedModelDepthFade {
    match policy {
        ModelDepthFade::InheritWorldFog => ResolvedModelDepthFade {
            enabled: world_enabled,
            near: world_near,
            far: world_far,
            color: world_color,
            fixed_point_domain: None,
        },
        ModelDepthFade::Disabled => ResolvedModelDepthFade {
            enabled: false,
            near: world_near,
            far: world_far,
            color: world_color,
            fixed_point_domain: None,
        },
        ModelDepthFade::Linear { near, far, color } => ResolvedModelDepthFade {
            enabled: true,
            near,
            far,
            color,
            fixed_point_domain: None,
        },
        ModelDepthFade::WorldRaw { fog, pass } => ResolvedModelDepthFade {
            enabled: pass == crate::renderer::NativeModelFogPass::Fog,
            near: fog.planes.near_raw as f32 / 256.0,
            far: fog.planes.far_raw as f32 / 256.0,
            color: fog.color,
            // Parent-plane table selection does not own active projector
            // reciprocal/cache fog state. Preserve the existing GL fade lane.
            fixed_point_domain: None,
        },
        ModelDepthFade::FrontendFixedPointLinear {
            near_raw,
            far_raw,
            color,
        } => ResolvedModelDepthFade {
            enabled: true,
            near: near_raw as f32 / FRONTEND_MODEL_UNITS_PER_VIEW,
            far: far_raw as f32 / FRONTEND_MODEL_UNITS_PER_VIEW,
            color,
            fixed_point_domain: Some(FixedPointFadeDomain { near_raw, far_raw }),
        },
    }
}

fn fixed_function_fog_far(near: f32, far: f32) -> f32 {
    if near >= far {
        near + f32::EPSILON * near.abs().max(1.0)
    } else {
        far
    }
}

/// 2-D overlays normally use conventional source-alpha compositing, while
/// authored Section-3 HUD layers select one of retail's sprite span families.
/// Keeping the modes explicit prevents a material flag from silently changing
/// every menu/text sprite in the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlaySpriteBlend {
    ConventionalAlpha,
    Retail(WorldSpriteBlend),
}

fn overlay_sprite_filter(scaling_mode: ScalingMode, blend: OverlaySpriteBlend) -> i32 {
    match blend {
        // Section-3 material sprites carry authored keyed masks. The HUD path
        // already materializes them in its selected layout; interpolating the
        // upload again can erode opaque shell pixels along neighbouring layer
        // boundaries.
        OverlaySpriteBlend::Retail(_) => crate::gl::NEAREST as i32,
        OverlaySpriteBlend::ConventionalAlpha => {
            if scaling_mode == ScalingMode::Native {
                crate::gl::NEAREST as i32
            } else {
                // Classic modes emulate the filtered enlargement of the
                // complete 320×240/640×480 framebuffer on a modern output.
                crate::gl::LINEAR as i32
            }
        }
    }
}

struct TerrainProgram {
    id: u32,
    index_sampler: i32,
    palette_sampler: i32,
    fog_enabled: i32,
    fog_near: i32,
    fog_far: i32,
    fog_color: i32,
    projection: ProjectionUniforms,
}

struct WaterProgram {
    id: u32,
    index_sampler: i32,
    palette_sampler: i32,
    fog_enabled: i32,
    fog_near: i32,
    fog_far: i32,
    fog_color: i32,
    projection: ProjectionUniforms,
}

struct CompatibilityProgram {
    id: u32,
    texture_sampler: i32,
    texture_enabled: i32,
    fog_enabled: i32,
    fog_near: i32,
    fog_far: i32,
    fog_color: i32,
    use_explicit_fog_coord: i32,
    projection: ProjectionUniforms,
}

unsafe fn projection_uniforms(program: u32) -> ProjectionUniforms {
    let uniform = |name: &[u8]| crate::gl::GetUniformLocation(program, name.as_ptr() as *const _);
    ProjectionUniforms {
        enabled: uniform(b"underwater_projection_enabled\0"),
        offsets: uniform(b"underwater_projection_offsets[0]\0"),
        viewport_size: uniform(b"logical_viewport_size\0"),
    }
}

unsafe fn apply_projection_uniforms(
    program: u32,
    uniforms: ProjectionUniforms,
    offsets: Option<&[[f32; 2]; 64]>,
    viewport: [u32; 2],
) {
    crate::gl::UseProgram(program);
    crate::gl::Uniform2f(
        uniforms.viewport_size,
        viewport[0].max(1) as f32,
        viewport[1].max(1) as f32,
    );
    match offsets {
        None => {
            crate::gl::Uniform1f(uniforms.enabled, 0.0);
        }
        Some(offsets) => {
            crate::gl::Uniform2fv(uniforms.offsets, 64, offsets.as_ptr().cast());
            crate::gl::Uniform1f(uniforms.enabled, 1.0);
        }
    }
}

unsafe fn activate_compatibility_program(
    program: &CompatibilityProgram,
    texture_enabled: bool,
    fog_enabled: bool,
    fog_near: f32,
    fog_far: f32,
    fog_color: [f32; 3],
) {
    crate::gl::UseProgram(program.id);
    crate::gl::Uniform1i(program.texture_sampler, 0);
    crate::gl::Uniform1f(
        program.texture_enabled,
        if texture_enabled { 1.0 } else { 0.0 },
    );
    crate::gl::Uniform1f(program.fog_enabled, if fog_enabled { 1.0 } else { 0.0 });
    crate::gl::Uniform1f(program.fog_near, fog_near);
    crate::gl::Uniform1f(program.fog_far, fog_far);
    crate::gl::Uniform3f(program.fog_color, fog_color[0], fog_color[1], fog_color[2]);
    crate::gl::Uniform1f(program.use_explicit_fog_coord, 0.0);
}

unsafe fn compile_shader(kind: u32, source: &str) -> Result<u32, String> {
    let shader = crate::gl::CreateShader(kind);
    let source = std::ffi::CString::new(source).map_err(|e| e.to_string())?;
    let ptr = source.as_ptr();
    crate::gl::ShaderSource(shader, 1, &ptr, std::ptr::null());
    crate::gl::CompileShader(shader);
    let mut ok = 0;
    crate::gl::GetShaderiv(shader, crate::gl::COMPILE_STATUS, &mut ok);
    if ok == 0 {
        let mut len = 0;
        crate::gl::GetShaderiv(shader, crate::gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len.max(1) as usize];
        crate::gl::GetShaderInfoLog(
            shader,
            len,
            std::ptr::null_mut(),
            log.as_mut_ptr() as *mut _,
        );
        crate::gl::DeleteShader(shader);
        return Err(String::from_utf8_lossy(&log)
            .trim_end_matches('\0')
            .to_string());
    }
    Ok(shader)
}

unsafe fn create_model_program() -> Result<ModelProgram, String> {
    const VERTEX_HEAD: &str = r#"#version 120
varying vec3 affine_uvq;
varying vec4 affine_colorq;
varying vec2 affine_shadeq;
varying vec2 affine_fogq;
uniform float fog_enabled;
uniform float fog_near;
uniform float fog_far;
uniform float use_explicit_fog_coord;
"#;
    const VERTEX_BODY: &str = r#"
float depth_fade_amount(float view_depth) {
    float range = fog_far - fog_near;
    if (range <= 0.0) {
        return step(fog_near, view_depth);
    }
    return clamp((view_depth - fog_near) / range, 0.0, 1.0);
}
void main() {
    vec4 eye = gl_ModelViewMatrix * gl_Vertex;
    gl_Position = gl_ProjectionMatrix * eye;
    apply_retail_projection(gl_Position);
    float w = gl_Position.w;
    affine_uvq = vec3(gl_MultiTexCoord0.xy * w, w);
    affine_colorq = vec4(gl_Color.rgb * w, w);
    affine_shadeq = vec2(gl_Color.a * w, w);
    float fog_amount = fog_enabled * (
        use_explicit_fog_coord > 0.5
            ? clamp(gl_FogCoord, 0.0, 1.0)
            : depth_fade_amount(abs(eye.z))
    );
    affine_fogq = vec2(fog_amount * w, w);
}
"#;
    const FRAGMENT: &str = r#"#version 120
uniform sampler2D index_tex;
uniform sampler2D palette_tex;
uniform float mode;
uniform float mask_zero;
uniform float opacity;
uniform float palette_fog;
uniform float gouraud_solid;
uniform vec3 solid_tint;
uniform vec3 solid_emissive;
uniform float blend_mode;
uniform vec3 fog_color;
varying vec3 affine_uvq;
varying vec4 affine_colorq;
varying vec2 affine_shadeq;
varying vec2 affine_fogq;
void main() {
    vec3 tint = affine_colorq.xyz / affine_colorq.w;
    if (gouraud_solid > 0.5) {
        // A/B23/24 interpolate unpacked channel accumulators, then 474220
        // packs them with RGB565 shifts 7/2/4 and WORD addition. gl_Color
        // transports channel/1024, keeping its components in [0,1].
        vec3 channels = floor(mod(floor(tint * 1024.0), 4096.0) / 16.0) * 16.0;
        // `packed` is a reserved GLSL word; strict compilers (Mesa) reject it.
        float rgb565 = mod(channels.r * 128.0 + channels.g * 4.0 + channels.b / 16.0, 65536.0);
        vec3 rgb = vec3(
            floor(rgb565 / 2048.0) * 8.0,
            floor(mod(rgb565, 2048.0) / 32.0) * 4.0,
            mod(rgb565, 32.0) * 8.0
        ) / 255.0;
        // The far initializer already composed base, light and fog. Applying
        // the ordinary RGB fog mix here would fog these solids twice.
        gl_FragColor = vec4(min(rgb * solid_tint + solid_emissive, vec3(1.0)), opacity);
        return;
    }
    vec3 source;
    if (mode < 0.5) {
        source = tint;
    } else {
        vec2 uv = affine_uvq.xy / affine_uvq.z;
        vec4 texel = texture2D(index_tex, uv);
        if (mode < 1.5) {
            float index = floor(texel.r * 255.0 + 0.5);
            if (mask_zero > 0.5 && index < 0.5) discard;
            float shade = clamp((affine_shadeq.x / affine_shadeq.y) * 31.0, 0.0, 31.0);
            // FUN_00478510 starts at +0.5 and adds signed pseudo-noise before
            // truncation. Select adjacent authored rows without temporal
            // flicker, matching the indexed terrain/water shaders.
            float row_dither = fract(sin(dot(floor(gl_FragCoord.xy), vec2(12.9898, 78.233))) * 43758.5453);
            float shade_row = min(floor(shade + row_dither), 31.0);
            source = texture2D(
                palette_tex,
                vec2((index + 0.5) / 16.0, (shade_row + 0.5) / 32.0)
            ).rgb * tint;
        } else {
            if (texel.a < 0.5) discard;
            source = texel.rgb * tint;
        }
    }
    float fog_amount = clamp(affine_fogq.x / affine_fogq.y, 0.0, 1.0);
    if (palette_fog > 0.5) {
        // 47C7C0/47F060/47F480 fade the shade accumulator BEFORE the
        // authored palette lookup. 479B90 keeps destination/2 and saturates
        // doubled source + fog before halving; 47A540 never adds fog colour.
        // RGB float arithmetic approximates the native packed-channel masks
        // and 16-bin fog LUT, retaining their compositing order.
        vec3 fog_term = fog_color * fog_amount;
        if (blend_mode > 1.5) {
            source = min(source * 2.0 + fog_term, vec3(1.0)) * 0.5;
        } else if (blend_mode < 0.5) {
            source = min(source + fog_term, vec3(1.0));
        }
    } else {
        // Direct RGB textures/solids retain their existing fallback. The
        // indexed palette-fog equations do not establish their fillers.
        source = mix(source, fog_color, fog_amount);
    }
    gl_FragColor = vec4(source, opacity);
}
"#;
    let vertex_source = [VERTEX_HEAD, RETAIL_PROJECTION_GLSL, VERTEX_BODY].concat();
    let vertex = compile_shader(crate::gl::VERTEX_SHADER, &vertex_source)?;
    let fragment = match compile_shader(crate::gl::FRAGMENT_SHADER, FRAGMENT) {
        Ok(shader) => shader,
        Err(error) => {
            crate::gl::DeleteShader(vertex);
            return Err(error);
        }
    };
    let id = crate::gl::CreateProgram();
    crate::gl::AttachShader(id, vertex);
    crate::gl::AttachShader(id, fragment);
    crate::gl::LinkProgram(id);
    crate::gl::DeleteShader(vertex);
    crate::gl::DeleteShader(fragment);
    let mut ok = 0;
    crate::gl::GetProgramiv(id, crate::gl::LINK_STATUS, &mut ok);
    if ok == 0 {
        let mut len = 0;
        crate::gl::GetProgramiv(id, crate::gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len.max(1) as usize];
        crate::gl::GetProgramInfoLog(id, len, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
        crate::gl::DeleteProgram(id);
        return Err(String::from_utf8_lossy(&log)
            .trim_end_matches('\0')
            .to_string());
    }
    let uniform = |name: &[u8]| crate::gl::GetUniformLocation(id, name.as_ptr() as *const _);
    Ok(ModelProgram {
        id,
        index_sampler: uniform(b"index_tex\0"),
        palette_sampler: uniform(b"palette_tex\0"),
        mode: uniform(b"mode\0"),
        mask_zero: uniform(b"mask_zero\0"),
        opacity: uniform(b"opacity\0"),
        palette_fog: uniform(b"palette_fog\0"),
        gouraud_solid: uniform(b"gouraud_solid\0"),
        solid_tint: uniform(b"solid_tint\0"),
        solid_emissive: uniform(b"solid_emissive\0"),
        blend_mode: uniform(b"blend_mode\0"),
        fog_enabled: uniform(b"fog_enabled\0"),
        fog_near: uniform(b"fog_near\0"),
        fog_far: uniform(b"fog_far\0"),
        fog_color: uniform(b"fog_color\0"),
        use_explicit_fog_coord: uniform(b"use_explicit_fog_coord\0"),
        projection: projection_uniforms(id),
    })
}

unsafe fn create_terrain_program() -> Result<TerrainProgram, String> {
    const VERTEX_HEAD: &str = r#"#version 120
varying vec3 affine_uvq;
varying vec2 affine_shadeq;
varying vec2 affine_fogq;
varying float frame_id;
uniform float fog_enabled;
uniform float fog_near;
uniform float fog_far;
"#;
    const VERTEX_BODY: &str = r#"
void main() {
    vec4 eye = gl_ModelViewMatrix * gl_Vertex;
    gl_Position = gl_ProjectionMatrix * eye;
    apply_retail_projection(gl_Position);
    float w = gl_Position.w;
    affine_uvq = vec3(gl_MultiTexCoord0.xy * w, w);
    affine_shadeq = vec2(gl_Color.r * w, w);
    float view_depth = abs(eye.z);
    float fog_amount = fog_enabled * clamp(
        (view_depth - fog_near) / max(fog_far - fog_near, 0.0001),
        0.0,
        1.0
    );
    // Retail computes one 0..255 fog byte per projected vertex, then the
    // software filler interpolates that byte affinely across the polygon.
    affine_fogq = vec2(fog_amount * w, w);
    frame_id = gl_Color.g;
}
"#;
    const FRAGMENT: &str = r#"#version 120
uniform sampler2D index_tex;
uniform sampler2D palette_tex;
uniform float fog_enabled;
uniform float fog_near;
uniform float fog_far;
uniform vec3 fog_color;
varying vec3 affine_uvq;
varying vec2 affine_shadeq;
varying vec2 affine_fogq;
varying float frame_id;
void main() {
    vec2 uv = affine_uvq.xy / affine_uvq.z;
    float shade = clamp((affine_shadeq.x / affine_shadeq.y) * 31.0, 0.0, 31.0);
    float frame = floor(frame_id * 119.0 + 0.5);
    float index = floor(texture2D(index_tex, uv).r * 255.0 + 0.5);
    // FUN_00478510 starts each interpolated row at +0.5 and adds signed
    // pseudo-noise in [-0.5,+0.5) before truncation. This stable screen-space
    // hash is the GL equivalent: unbiased stochastic selection between the
    // two adjacent authored shade rows without temporal flicker.
    float row_dither = fract(sin(dot(floor(gl_FragCoord.xy), vec2(12.9898, 78.233))) * 43758.5453);
    float shade_row = min(floor(shade + row_dither), 31.0);
    float px = (index + 0.5) / 16.0;
    vec4 color = texture2D(palette_tex, vec2(px, (frame * 32.0 + shade_row + 0.5) / 3840.0));
    float fog_amount = clamp(affine_fogq.x / affine_fogq.y, 0.0, 1.0);
    color.rgb = mix(color.rgb, fog_color, fog_amount);
    gl_FragColor = vec4(color.rgb, 1.0);
}
"#;
    let vertex_source = [VERTEX_HEAD, RETAIL_PROJECTION_GLSL, VERTEX_BODY].concat();
    let vertex = compile_shader(crate::gl::VERTEX_SHADER, &vertex_source)?;
    let fragment = match compile_shader(crate::gl::FRAGMENT_SHADER, FRAGMENT) {
        Ok(shader) => shader,
        Err(error) => {
            crate::gl::DeleteShader(vertex);
            return Err(error);
        }
    };
    let id = crate::gl::CreateProgram();
    crate::gl::AttachShader(id, vertex);
    crate::gl::AttachShader(id, fragment);
    crate::gl::LinkProgram(id);
    crate::gl::DeleteShader(vertex);
    crate::gl::DeleteShader(fragment);
    let mut ok = 0;
    crate::gl::GetProgramiv(id, crate::gl::LINK_STATUS, &mut ok);
    if ok == 0 {
        let mut len = 0;
        crate::gl::GetProgramiv(id, crate::gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len.max(1) as usize];
        crate::gl::GetProgramInfoLog(id, len, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
        crate::gl::DeleteProgram(id);
        return Err(String::from_utf8_lossy(&log)
            .trim_end_matches('\0')
            .to_string());
    }
    let uniform = |name: &[u8]| crate::gl::GetUniformLocation(id, name.as_ptr() as *const _);
    Ok(TerrainProgram {
        id,
        index_sampler: uniform(b"index_tex\0"),
        palette_sampler: uniform(b"palette_tex\0"),
        fog_enabled: uniform(b"fog_enabled\0"),
        fog_near: uniform(b"fog_near\0"),
        fog_far: uniform(b"fog_far\0"),
        fog_color: uniform(b"fog_color\0"),
        projection: projection_uniforms(id),
    })
}

unsafe fn create_water_program() -> Result<WaterProgram, String> {
    const VERTEX_HEAD: &str = r#"#version 120
varying vec3 affine_uvq;
varying vec2 affine_shadeq;
varying vec2 affine_fogq;
varying float frame_id;
uniform float fog_enabled;
uniform float fog_near;
uniform float fog_far;
"#;
    const VERTEX_BODY: &str = r#"
void main() {
    vec4 eye = gl_ModelViewMatrix * gl_Vertex;
    gl_Position = gl_ProjectionMatrix * eye;
    apply_retail_projection(gl_Position);
    float w = gl_Position.w;
    affine_uvq = vec3(gl_MultiTexCoord0.xy * w, w);
    affine_shadeq = vec2(gl_Color.r * w, w);
    float view_depth = abs(eye.z);
    float fog_amount = fog_enabled * clamp(
        (view_depth - fog_near) / max(fog_far - fog_near, 0.0001),
        0.0,
        1.0
    );
    affine_fogq = vec2(fog_amount * w, w);
    frame_id = gl_Color.g;
}
"#;
    const FRAGMENT: &str = r#"#version 120
uniform sampler2D index_tex;
uniform sampler2D palette_tex;
uniform float fog_enabled;
uniform float fog_near;
uniform float fog_far;
uniform vec3 fog_color;
varying vec3 affine_uvq;
varying vec2 affine_shadeq;
varying vec2 affine_fogq;
varying float frame_id;
void main() {
    vec2 uv = affine_uvq.xy / affine_uvq.z;
    vec4 indexed_texel = texture2D(index_tex, uv);
    if (indexed_texel.a < 0.5) discard;
    float index = floor(indexed_texel.r * 255.0 + 0.5);
    float shade = clamp((affine_shadeq.x / affine_shadeq.y) * 31.0, 0.0, 31.0);
    float row_dither = fract(sin(dot(floor(gl_FragCoord.xy), vec2(12.9898, 78.233))) * 43758.5453);
    float shade_row = min(floor(shade + row_dither), 31.0);
    float frame = floor(frame_id * 4.0 + 0.5);
    vec3 source = texture2D(
        palette_tex,
        vec2((index + 0.5) / 16.0, (frame * 32.0 + shade_row + 0.5) / 160.0)
    ).rgb;
    float fog_amount = clamp(affine_fogq.x / affine_fogq.y, 0.0, 1.0);
    // Retail water contributes a full palette texel plus half the destination.
    // At the terminal plane the premultiplied source must therefore become
    // half the fog colour, so compositing over already-fogged terrain remains
    // exactly the fog colour instead of brightening it twice.
    source = mix(source, fog_color * 0.5, fog_amount);
    gl_FragColor = vec4(source, 0.5);
}
"#;
    let vertex_source = [VERTEX_HEAD, RETAIL_PROJECTION_GLSL, VERTEX_BODY].concat();
    let vertex = compile_shader(crate::gl::VERTEX_SHADER, &vertex_source)?;
    let fragment = match compile_shader(crate::gl::FRAGMENT_SHADER, FRAGMENT) {
        Ok(shader) => shader,
        Err(error) => {
            crate::gl::DeleteShader(vertex);
            return Err(error);
        }
    };
    let id = crate::gl::CreateProgram();
    crate::gl::AttachShader(id, vertex);
    crate::gl::AttachShader(id, fragment);
    crate::gl::LinkProgram(id);
    crate::gl::DeleteShader(vertex);
    crate::gl::DeleteShader(fragment);
    let mut ok = 0;
    crate::gl::GetProgramiv(id, crate::gl::LINK_STATUS, &mut ok);
    if ok == 0 {
        let mut len = 0;
        crate::gl::GetProgramiv(id, crate::gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len.max(1) as usize];
        crate::gl::GetProgramInfoLog(id, len, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
        crate::gl::DeleteProgram(id);
        return Err(String::from_utf8_lossy(&log)
            .trim_end_matches('\0')
            .to_string());
    }
    let uniform = |name: &[u8]| crate::gl::GetUniformLocation(id, name.as_ptr() as *const _);
    Ok(WaterProgram {
        id,
        index_sampler: uniform(b"index_tex\0"),
        palette_sampler: uniform(b"palette_tex\0"),
        fog_enabled: uniform(b"fog_enabled\0"),
        fog_near: uniform(b"fog_near\0"),
        fog_far: uniform(b"fog_far\0"),
        fog_color: uniform(b"fog_color\0"),
        projection: projection_uniforms(id),
    })
}

/// Compatibility program for fixed-function 3-D submissions. Model and world
/// billboard quads use this while underwater so their projected corners pass
/// through the same post-perspective integer stage as opaque geometry. It also
/// provides the RGBA fallback when an indexed terrain/model program is not
/// available on the current GL driver.
unsafe fn create_compatibility_program() -> Result<CompatibilityProgram, String> {
    const VERTEX_HEAD: &str = r#"#version 120
varying vec4 compatibility_color;
varying vec2 compatibility_uv;
varying float compatibility_fog_amount;
uniform float fog_enabled;
uniform float fog_near;
uniform float fog_far;
uniform float use_explicit_fog_coord;
"#;
    const VERTEX_BODY: &str = r#"
float depth_fade_amount(float view_depth) {
    float range = fog_far - fog_near;
    if (range <= 0.0) {
        return step(fog_near, view_depth);
    }
    return clamp((view_depth - fog_near) / range, 0.0, 1.0);
}
void main() {
    vec4 eye = gl_ModelViewMatrix * gl_Vertex;
    gl_Position = gl_ProjectionMatrix * eye;
    apply_retail_projection(gl_Position);
    compatibility_color = gl_Color;
    compatibility_uv = gl_MultiTexCoord0.xy;
    compatibility_fog_amount = fog_enabled * (
        use_explicit_fog_coord > 0.5
            ? clamp(gl_FogCoord, 0.0, 1.0)
            : depth_fade_amount(abs(eye.z))
    );
}
"#;
    const FRAGMENT: &str = r#"#version 120
uniform sampler2D diffuse_tex;
uniform float texture_enabled;
uniform vec3 fog_color;
varying vec4 compatibility_color;
varying vec2 compatibility_uv;
varying float compatibility_fog_amount;
void main() {
    vec4 source = compatibility_color;
    if (texture_enabled > 0.5) {
        source *= texture2D(diffuse_tex, compatibility_uv);
    }
    source.rgb = mix(source.rgb, fog_color, clamp(compatibility_fog_amount, 0.0, 1.0));
    gl_FragColor = source;
}
"#;

    let vertex_source = [VERTEX_HEAD, RETAIL_PROJECTION_GLSL, VERTEX_BODY].concat();
    let vertex = compile_shader(crate::gl::VERTEX_SHADER, &vertex_source)?;
    let fragment = match compile_shader(crate::gl::FRAGMENT_SHADER, FRAGMENT) {
        Ok(shader) => shader,
        Err(error) => {
            crate::gl::DeleteShader(vertex);
            return Err(error);
        }
    };
    let id = crate::gl::CreateProgram();
    crate::gl::AttachShader(id, vertex);
    crate::gl::AttachShader(id, fragment);
    crate::gl::LinkProgram(id);
    crate::gl::DeleteShader(vertex);
    crate::gl::DeleteShader(fragment);
    let mut ok = 0;
    crate::gl::GetProgramiv(id, crate::gl::LINK_STATUS, &mut ok);
    if ok == 0 {
        let mut len = 0;
        crate::gl::GetProgramiv(id, crate::gl::INFO_LOG_LENGTH, &mut len);
        let mut log = vec![0u8; len.max(1) as usize];
        crate::gl::GetProgramInfoLog(id, len, std::ptr::null_mut(), log.as_mut_ptr() as *mut _);
        crate::gl::DeleteProgram(id);
        return Err(String::from_utf8_lossy(&log)
            .trim_end_matches('\0')
            .to_string());
    }
    let uniform = |name: &[u8]| crate::gl::GetUniformLocation(id, name.as_ptr() as *const _);
    Ok(CompatibilityProgram {
        id,
        texture_sampler: uniform(b"diffuse_tex\0"),
        texture_enabled: uniform(b"texture_enabled\0"),
        fog_enabled: uniform(b"fog_enabled\0"),
        fog_near: uniform(b"fog_near\0"),
        fog_far: uniform(b"fog_far\0"),
        fog_color: uniform(b"fog_color\0"),
        use_explicit_fog_coord: uniform(b"use_explicit_fog_coord\0"),
        projection: projection_uniforms(id),
    })
}

fn terrain_type_signature(terrain: &TerrainGrid) -> u64 {
    // The live game mutates Section-10 cell byte 2 without replacing the
    // TerrainGrid. Hash only that byte: heights have their own authored grid
    // lifetime, while all runtime terrain presentation flags live here.
    terrain.cells.iter().fold(0xcbf29ce484222325, |hash, cell| {
        (hash ^ u64::from(cell.terrain_type)).wrapping_mul(0x100000001b3)
    })
}

fn infection_corner_mask(corners: [u8; 4]) -> u8 {
    corners
        .into_iter()
        .enumerate()
        .fold(0, |mask, (corner, terrain_type)| {
            mask | (u8::from(terrain_type & 0x10 != 0) << corner)
        })
}

fn infection_quad_vertices(
    mask: u8,
    frame_uvs: &[[f32; 4]; crate::terrain_tiles::INFECTION_FRAME_COUNT],
    positions: [[f32; 3]; 4],
) -> Option<(u8, [[f32; 5]; 6])> {
    if mask == 0 {
        return None;
    }
    let entry = *crate::water::SHORE_TABLE.get(usize::from(mask))?;
    let rect = *frame_uvs.get(usize::from(entry.frame))?;
    let corner_uvs: [[f32; 2]; 4] = std::array::from_fn(|corner| {
        let (u, v) = crate::water::SLOT_UV[usize::from(entry.slot[corner])];
        [
            rect[0] + (rect[2] - rect[0]) * u,
            rect[1] + (rect[3] - rect[1]) * v,
        ]
    });
    let vertices = [0usize, 1, 3, 1, 2, 3].map(|corner| {
        [
            positions[corner][0],
            positions[corner][1],
            positions[corner][2],
            corner_uvs[corner][0],
            corner_uvs[corner][1],
        ]
    });
    Some((entry.frame, vertices))
}

fn scene_uses_world_fog(scene: RenderScene, configured: bool) -> bool {
    scene == RenderScene::World && configured
}

fn flip_rgba_rows(rgba: &mut [u8], width: u32, height: u32) {
    let row_len = width as usize * 4;
    if row_len == 0 || rgba.len() != row_len.saturating_mul(height as usize) {
        return;
    }
    for top in 0..height as usize / 2 {
        let bottom = height as usize - 1 - top;
        let (head, tail) = rgba.split_at_mut(bottom * row_len);
        head[top * row_len..(top + 1) * row_len].swap_with_slice(&mut tail[..row_len]);
    }
}

/// OpenGL NDC depth for an eye-space point at `z = -view_depth` under the
/// same perspective matrix used by [`Camera::projection_matrix`].
fn perspective_ndc_depth(view_depth: f32, near: f32, far: f32) -> f32 {
    (far + near) / (far - near) - (2.0 * far * near) / ((far - near) * view_depth)
}

/// A painter key changes queue order, never projection or face clipping.
/// Constant glDepthRange endpoints record that order after rasterization;
/// signed keys beyond the camera interval saturate at its depth boundaries.
fn painter_group_window_depth(view_depth_raw: i32, near: f32, far: f32) -> Option<f64> {
    if !near.is_finite() || !far.is_finite() || near <= 0.0 || far <= near {
        return None;
    }
    let depth = view_depth_raw as f32 / 100.0;
    if depth <= near {
        return Some(0.0);
    }
    if depth >= far {
        return Some(1.0);
    }
    Some(f64::from(
        ((perspective_ndc_depth(depth, near, far) + 1.0) * 0.5).clamp(0.0, 1.0),
    ))
}

/// Preserve ordered draw depth state without the driver attribute stack.
/// Some compatibility drivers narrow DEPTH_RANGE in PushAttrib/PopAttrib;
/// retain the queried doubles and restore only these depth fields explicitly.
struct PainterDepthState {
    test_enabled: bool,
    write_mask: u8,
    function: i32,
    range: [f64; 2],
}

impl PainterDepthState {
    unsafe fn capture() -> Self {
        let mut state = Self {
            test_enabled: crate::gl::IsEnabled(crate::gl::DEPTH_TEST) != 0,
            write_mask: crate::gl::TRUE,
            function: crate::gl::LEQUAL as i32,
            range: [0.0, 1.0],
        };
        crate::gl::GetBooleanv(crate::gl::DEPTH_WRITEMASK, &mut state.write_mask);
        crate::gl::GetIntegerv(crate::gl::DEPTH_FUNC, &mut state.function);
        crate::gl::GetDoublev(crate::gl::DEPTH_RANGE, state.range.as_mut_ptr());
        state
    }

    unsafe fn restore(self) {
        crate::gl::DepthRange(self.range[0], self.range[1]);
        crate::gl::DepthFunc(self.function as u32);
        crate::gl::DepthMask(self.write_mask);
        if self.test_enabled {
            crate::gl::Enable(crate::gl::DEPTH_TEST);
        } else {
            crate::gl::Disable(crate::gl::DEPTH_TEST);
        }
    }
}

/// Compute face normal from triangle vertices via cross product.
/// Fallback for when parsed normal data is unavailable.
fn compute_face_normal(vertices: &[[f64; 3]], tri: &[u16; 3]) -> [f32; 3] {
    let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
    if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
        return [0.0, 1.0, 0.0];
    }
    let v0 = &vertices[i0];
    let v1 = &vertices[i1];
    let v2 = &vertices[i2];

    let ax = (v1[0] - v0[0]) as f32;
    let ay = (v1[1] - v0[1]) as f32;
    let az = (v1[2] - v0[2]) as f32;
    let bx = (v2[0] - v0[0]) as f32;
    let by = (v2[1] - v0[1]) as f32;
    let bz = (v2[2] - v0[2]) as f32;

    let nx = ay * bz - az * by;
    let ny = az * bx - ax * bz;
    let nz = ax * by - ay * bx;
    let mag = (nx * nx + ny * ny + nz * nz).sqrt();
    if mag > 0.0 {
        [nx / mag, ny / mag, nz / mag]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Convert a unit model-face normal to the recovered Section-6 table slot.
///
/// `FUN_0046D3F0` dots the active signed integer light vector with an authored
/// signed-16 unit normal, shifts the result right by 19, then masks to four
/// bits. Local contexts use `(73,73,-73)`, the world its reduced Section-10
/// direction and frontend Klaus `(-100,50,-50)`. Their direction *and
/// magnitude* therefore belong to the render submission. The parser exposes
/// normalized float normals, so `32767 / 2^19` restores the equivalent
/// fixed-point bin scale. Callers pass VIEW-space normals
/// ([`view_light_normal`]).
const RETAIL_MODEL_NORMAL_TO_SHADE_BIN: f32 = 32_767.0 / 524_288.0;

fn retail_model_shade_index(
    normal: [f32; 3],
    light_direction_raw: [i32; 3],
    shade_shift: i32,
) -> usize {
    let dot_raw = normal[0] * light_direction_raw[0] as f32
        + normal[1] * light_direction_raw[1] as f32
        + normal[2] * light_direction_raw[2] as f32;
    let signed_bin = (dot_raw * RETAIL_MODEL_NORMAL_TO_SHADE_BIN).floor() as i32;
    retail_model_shade_slot(signed_bin, shade_shift)
}

/// FUN_004136C0/FUN_004138F0 shift the complete signed -8..7 lookup,
/// clamping at its two endpoints before applying the original 4-bit slot
/// layout. Slots 8..15 all resolve to Section-6 entry zero.
fn retail_model_shade_slot(signed_bin: i32, shade_shift: i32) -> usize {
    let slot = ((signed_bin + shade_shift).clamp(-8, 7) & 0x0f) as usize;
    if slot < 8 {
        slot
    } else {
        0
    }
}

/// `FUN_0046D3F0` in its own integers: the raw pool normal dotted with the
/// node's model-space light in wrapping 32-bit arithmetic, and the 4-bit
/// table slot of that dot shifted right by 19.
pub(crate) fn retail_model_shade_raw(
    table: Option<&[FogGradientEntry]>,
    normal_raw: [i32; 3],
    light_model_raw: [i32; 3],
    shade_shift: i32,
) -> Option<&FogGradientEntry> {
    let dot = light_model_raw[2]
        .wrapping_mul(normal_raw[2])
        .wrapping_add(light_model_raw[1].wrapping_mul(normal_raw[1]))
        .wrapping_add(light_model_raw[0].wrapping_mul(normal_raw[0]));
    let signed_bin = (((dot >> 19) & 0x0F) ^ 0x08) - 0x08;
    table?.get(retail_model_shade_slot(signed_bin, shade_shift))
}

pub(crate) fn retail_model_shade(
    table: Option<&[FogGradientEntry]>,
    normal: [f32; 3],
    light_direction_raw: [i32; 3],
    shade_shift: i32,
) -> Option<&FogGradientEntry> {
    table?.get(retail_model_shade_index(
        normal,
        light_direction_raw,
        shade_shift,
    ))
}

/// 43/44/C3/C4 carry one face-normal light value. The Gouraud families
/// instead carry distinct corner references; 03/04/83/84 carry no light.
pub(crate) fn model_lighting_normals(
    shading: ModelFaceShading,
    face_normal: [f32; 3],
    corner_normals: [[f32; 3]; 3],
) -> Option<[[f32; 3]; 3]> {
    match shading {
        ModelFaceShading::Flat => None,
        ModelFaceShading::FlatLit => Some([face_normal; 3]),
        ModelFaceShading::Gouraud => Some(corner_normals),
    }
}

/// The VIEW-space normal `FUN_0046D3F0` effectively dots with the light.
///
/// `FUN_00466160` brings the context's light into model space through the
/// node's VIEW axes (camera rows times model basis), so the light is fixed to
/// the camera. `basis` holds the camera's GL rows (x right, y up, z toward
/// the viewer); retail VIEW Z looks forward.
pub(crate) fn view_light_normal(
    basis: [[f32; 3]; 3],
    orientation: [[f32; 3]; 3],
    normal: [f32; 3],
) -> [f32; 3] {
    let world = transform_model_light_normal(orientation, normal);
    [
        dot3(basis[0], world),
        dot3(basis[1], world),
        -dot3(basis[2], world),
    ]
}

pub(crate) fn transform_model_light_normal(
    orientation: [[f32; 3]; 3],
    normal: [f32; 3],
) -> [f32; 3] {
    let v: [f32; 3] = std::array::from_fn(|axis| dot3(orientation[axis], normal));
    let magnitude = dot3(v, v).sqrt();
    if magnitude > 0.0 {
        v.map(|component| component / magnitude)
    } else {
        // Normal refs 0/1 are a real zero-direction light lookup. Inventing
        // an upward normal changes their Section-6 slot from zero to four.
        [0.0; 3]
    }
}

/// Solid 43/44 fills add a packed light value to the relocated Section-7
/// word (FUN_00473C00), rather than multiplying its RGB. Use the RGB565
/// display layout observed in the accepted DirectDraw framebuffer trace.
fn retail_flat_lit_rgb565(palette_rgb555: u16, shade_rgb: [u8; 3]) -> [u8; 3] {
    // FUN_004AA2E0 moves the five source bits into the display's high bits;
    // green's new low bit starts clear. FUN_004A8920 installs filler shifts
    // 7/2/4 for RGB565, so a light byte of 128 contributes 64, not 128.
    let base = ((palette_rgb555 & 0x7fe0) << 1) | (palette_rgb555 & 0x001f);
    let [r, g, b] = shade_rgb.map(|component| u16::from(component & 0xf0));
    let light = (r << 7) | (g << 2) | (b >> 4);
    let packed = base.wrapping_add(light);
    [
        ((packed >> 11) as u8) << 3,
        (((packed >> 5) & 0x3f) as u8) << 2,
        ((packed & 0x1f) as u8) << 3,
    ]
}

fn flat_lit_palette_color(material: &FaceMaterial, shade: &FogGradientEntry) -> Option<[f32; 3]> {
    let rgb = retail_flat_lit_rgb565(material.palette_rgb555?, [shade.r, shade.g, shade.b]);
    Some(std::array::from_fn(|axis| {
        rgb[axis] as f32 / 255.0 * material.color[axis]
    }))
}

/// Callback-resolved model vertices, shared by every backend: raw-local
/// points (`None` when a callback rejects the vertex), the world points the
/// projector sees, and near-plane admission.
pub(crate) struct ResolvedModelVertices {
    pub(crate) local: Vec<Option<[f32; 3]>>,
    pub(crate) world: Vec<Option<[f32; 3]>>,
    pub(crate) admitted: Vec<bool>,
}

/// Resolve every vertex of `draw` through its type callbacks, external
/// frame and surface policy, then admit it against the near plane.
pub(crate) fn resolve_model_vertices(
    draw: &ModelDraw<'_>,
    camera_position: [f32; 3],
    camera_basis: [[f32; 3]; 3],
) -> ResolvedModelVertices {
    let mesh = draw.mesh;
    let vertices = mesh.vertices;
    let vertex_type_flags = mesh.vertex_type_flags;
    let orientation = draw.transform.orientation;
    let position = draw.transform.position;
    let extra_scale = draw.transform.scale;
    let view_pin_mode = draw.view_pin;
    let world_surface = draw.world_surface;
    let external_frame_mode = draw.external_frame;
    let scale = extra_scale / 100.0_f32;
    let vertex_for = |idx: usize| -> Option<[f32; 3]> {
        if mesh
            .vertex_clip
            .get(idx)
            .is_some_and(|clip| *clip != ModelSlotClip::Clear)
        {
            return None;
        }
        let v = vertices.get(idx)?;
        let type_flag = vertex_type_flags.get(idx).copied().unwrap_or_default();
        let mut raw = [v[0] as f32, v[1] as f32, v[2] as f32];
        if type_flag == 14 {
            raw = apply_external_frame_operands(
                raw,
                external_frame_mode,
                orientation,
                position,
                extra_scale,
            );
        }
        if view_pin_mode == ViewPinMode::Raw
            || !matches!(type_flag, 13 | 12)
            || draw.surface_resolution == crate::renderer::ModelSurfaceResolution::ContextResolved
        {
            return Some(raw);
        }
        if type_flag == 13 {
            match view_pin_mode {
                ViewPinMode::Disabled => return None,
                ViewPinMode::CameraFacing => {
                    return Some(retail_view_pin_vertex(
                        raw,
                        orientation,
                        position,
                        extra_scale,
                        camera_position,
                        camera_basis,
                    ));
                }
                ViewPinMode::WorldSurface => {
                    return world_surface.and_then(|surface| {
                        let resolved = retail_world_surface_vertex(
                            raw,
                            orientation,
                            position,
                            extra_scale,
                            surface,
                        );
                        (resolved.clip == ModelSlotClip::Clear)
                            .then(|| resolved.position_raw.map(|value| value as f32))
                    });
                }
                ViewPinMode::Raw => return Some(raw),
            }
        }
        // tf 12: pure alias except in world contexts, where FUN_00433FA0's
        // swapped handlers ground Y onto the sampled terrain height.
        match view_pin_mode {
            ViewPinMode::WorldSurface => world_surface.map(|surface| {
                retail_world_surface_alias_vertex(
                    raw,
                    orientation,
                    position,
                    extra_scale,
                    surface.terrain,
                )
            }),
            _ => Some(raw),
        }
    };

    // The retail face handler computes every corner callback before applying
    // the combined clip-byte lookup. Resolve once per submitted model so an
    // inclusive sea-band rejection drops the complete face and both colour
    // passes observe the same animated surface sample.
    let resolved_vertices = (0..vertices.len()).map(vertex_for).collect::<Vec<_>>();
    // The projector sees callback-resolved world points. Type-14 selector
    // outputs bypass O in retail; use the same explicit world conversion as
    // edge submission instead of testing intrinsic raw-local coordinates.
    let world_vertices = (0..resolved_vertices.len())
        .map(|index| {
            if let Some(v2k_formats::models::ModelVertexProjection::WorldPoint(world)) =
                mesh.vertex_projection.get(index)
            {
                return resolved_vertices[index].map(|_| world.map(|component| component as f32));
            }
            edge_endpoint_world(
                index,
                vertices,
                vertex_type_flags,
                &resolved_vertices,
                external_frame_mode,
                orientation,
                position,
                scale,
            )
        })
        .collect::<Vec<_>>();
    let vertex_admitted = vertex_admission(
        draw.near_clip,
        &world_vertices,
        mesh.vertex_projection,
        camera_position,
        camera_basis[2],
    );
    ResolvedModelVertices {
        local: resolved_vertices,
        world: world_vertices,
        admitted: vertex_admitted,
    }
}

/// Shared model-mesh draw path: lit, authored-plane-culled triangles at
/// `position`, oriented by the row-major 3×3 `orientation` (applied as
/// `world = M · v`) and uniformly scaled by `extra_scale`. This backend's historical base
/// conversion is raw/100 for the reconstructed menu scene; gameplay callers
/// pass 100/256 because the original entity renderer shares the 8.8 world
/// coordinate domain.
/// All static and materialized model bodies enter through this one path.
///
/// # Safety
/// Must be called with a current GL context inside a frame.
#[allow(clippy::too_many_arguments)]
unsafe fn draw_tris_gl(
    draw: ModelDraw<'_>,
    camera_position: [f32; 3],
    camera_basis: [[f32; 3]; 3],
    program: Option<&ModelProgram>,
    compatibility_program: Option<&CompatibilityProgram>,
    indexed_textures: &HashMap<u32, IndexedModelTextures>,
    fog_near: f32,
    fog_far: f32,
    fog_color: [f32; 3],
    overlay_depth_policy: OverlayDepthPolicy,
    edge_intrinsics: Option<crate::edge_quads::ScreenIntrinsics>,
) {
    let mesh = draw.mesh;
    let vertices = mesh.vertices;
    let vertex_type_flags = mesh.vertex_type_flags;
    let triangles = mesh.triangles;
    let normals = mesh.normals;
    let face_cull = mesh.face_cull;
    let face_materials = mesh.materials;
    let face_uvs = mesh.face_uvs;
    let face_corner_normals = mesh.face_corner_normals;
    let face_shading = mesh.face_shading;
    let shade_table = mesh.shade_table;
    let light_direction_raw = mesh.light_direction_raw;
    let shade_shift = mesh.shade_shift;
    let orientation = draw.transform.orientation;
    let position = draw.transform.position;
    let extra_scale = draw.transform.scale;
    let view_pin_mode = draw.view_pin;
    if vertices.is_empty() {
        return;
    }

    let fog_was_enabled = crate::gl::IsEnabled(crate::gl::FOG) != 0;
    let mut depth_fade = resolve_model_depth_fade(
        draw.depth_fade,
        fog_was_enabled,
        fog_near,
        fog_far,
        fog_color,
    );
    let origin_relative = std::array::from_fn(|axis| position[axis] - camera_position[axis]);
    let origin_depth = -dot3(origin_relative, camera_basis[2]);
    let radius = f32::from(mesh.radius_raw) * extra_scale.abs() / 100.0;
    let node_fog_pass = selected_model_fog_pass(draw.depth_fade).unwrap_or_else(|| {
        model_fog_pass(
            origin_depth,
            radius,
            depth_fade
                .enabled
                .then_some([depth_fade.near, depth_fade.far]),
        )
    });
    depth_fade.enabled = node_fog_pass == PaletteFogPass::Far;

    // V2000 culls each face with its authored anchor/normal plane, NOT by
    // winding. The artist data is ~50/50 wound, so winding-based GL culling
    // would drop ~half of every model. Keep GL culling off and apply the exact
    // plane test immediately before each triangle submission below.
    crate::gl::Disable(crate::gl::CULL_FACE);
    crate::gl::Disable(crate::gl::POLYGON_OFFSET_FILL);

    if let Some(program) = program {
        crate::gl::Disable(crate::gl::FOG);
        crate::gl::Disable(crate::gl::ALPHA_TEST);
        crate::gl::UseProgram(program.id);
        crate::gl::Uniform1f(program.gouraud_solid, 0.0);
        crate::gl::Uniform1i(program.index_sampler, 0);
        crate::gl::Uniform1i(program.palette_sampler, 1);
        crate::gl::Uniform1f(
            program.fog_enabled,
            if depth_fade.enabled { 1.0 } else { 0.0 },
        );
        crate::gl::Uniform1f(program.fog_near, depth_fade.near);
        crate::gl::Uniform1f(program.fog_far, depth_fade.far);
        crate::gl::Uniform3f(
            program.fog_color,
            depth_fade.color[0],
            depth_fade.color[1],
            depth_fade.color[2],
        );
        crate::gl::Uniform1f(program.use_explicit_fog_coord, 0.0);
    } else if let Some(program) = compatibility_program {
        crate::gl::Disable(crate::gl::FOG);
        crate::gl::UseProgram(program.id);
        crate::gl::Uniform1i(program.texture_sampler, 0);
        crate::gl::Uniform1f(
            program.fog_enabled,
            if depth_fade.enabled { 1.0 } else { 0.0 },
        );
        crate::gl::Uniform1f(program.fog_near, depth_fade.near);
        crate::gl::Uniform1f(program.fog_far, depth_fade.far);
        crate::gl::Uniform3f(
            program.fog_color,
            depth_fade.color[0],
            depth_fade.color[1],
            depth_fade.color[2],
        );
        crate::gl::Uniform1f(program.use_explicit_fog_coord, 0.0);
    } else if depth_fade.enabled {
        crate::gl::Fogi(crate::gl::FOG_MODE, crate::gl::LINEAR as i32);
        crate::gl::Fogf(crate::gl::FOG_START, depth_fade.near);
        crate::gl::Fogf(
            crate::gl::FOG_END,
            fixed_function_fog_far(depth_fade.near, depth_fade.far),
        );
        let color = [
            depth_fade.color[0],
            depth_fade.color[1],
            depth_fade.color[2],
            1.0,
        ];
        crate::gl::Fogfv(crate::gl::FOG_COLOR, color.as_ptr());
        crate::gl::Enable(crate::gl::FOG);
    } else {
        crate::gl::Disable(crate::gl::FOG);
    }

    // Callback points are already world coordinates (46ECF0/4349C0).
    // Retain the camera matrix for a mixed face so its world corners never
    // undergo the body's non-orthonormal inverse/forward round trip.
    let mut world_modelview = [0.0_f32; 16];
    crate::gl::GetFloatv(crate::gl::MODELVIEW_MATRIX, world_modelview.as_mut_ptr());
    crate::gl::PushMatrix();
    crate::gl::Translatef(position[0], position[1], position[2]);
    // Column-major 4×4 from the row-major 3×3 (GL element (r,c) = m[r][c]).
    let m = orientation;
    let glm: [f32; 16] = [
        m[0][0], m[1][0], m[2][0], 0.0, // column 0
        m[0][1], m[1][1], m[2][1], 0.0, // column 1
        m[0][2], m[1][2], m[2][2], 0.0, // column 2
        0.0, 0.0, 0.0, 1.0,
    ];
    crate::gl::MultMatrixf(glm.as_ptr());

    let scale = extra_scale / 100.0_f32;

    let surface_vertices = match draw.surface_resolution {
        ModelSurfaceResolution::Intrinsic => SurfaceVertexSource::Intrinsic(vertex_type_flags),
        ModelSurfaceResolution::ContextResolved => {
            SurfaceVertexSource::Resolved(mesh.vertex_surface_origin)
        }
    };

    let ResolvedModelVertices {
        local: resolved_vertices,
        world: world_vertices,
        admitted: vertex_admitted,
    } = resolve_model_vertices(&draw, camera_position, camera_basis);
    let vertex_fade_bytes = world_vertices
        .iter()
        .map(|vertex| {
            vertex.map_or(0, |world| {
                let relative = std::array::from_fn(|axis| world[axis] - camera_position[axis]);
                let depth_raw = (-dot3(relative, camera_basis[2]) * 256.0) as i32;
                depth_fade.byte_at_raw_depth(depth_raw)
            })
        })
        .collect::<Vec<_>>();

    for (tri_idx, tri) in triangles.iter().enumerate() {
        // Surface-band rejection belongs to the complete authored primitive,
        // including a quad's fourth corner under the diagnostic camera policy.
        let original = mesh.face_vertices.get(tri_idx);
        if !face_admitted(original, tri, &vertex_admitted) {
            continue;
        }
        if face_cull.get(tri_idx).is_some_and(|mode| {
            matches!(mode, ModelFaceCull::Plane(plane) if
            !authored_face_plane_visible(
                *plane,
                orientation,
                position,
                extra_scale,
                camera_position,
            ))
        }) {
            continue;
        }
        // Disabled/default contexts omit a complete mixed face rather than
        // submitting an incomplete GL triangle. World contexts retain every
        // authored face and let the callback's clip byte reject the face.
        if !view_pin_triangle_visible(view_pin_mode, tri, vertex_type_flags) {
            continue;
        }
        if tri.iter().any(|&index| {
            resolved_vertices
                .get(index as usize)
                .and_then(|vertex| *vertex)
                .is_none()
        }) {
            continue;
        }
        let material = face_materials.get(tri_idx);
        let textured = material.and_then(|material| material.texture);
        let indexed = textured.and_then(|texture| indexed_textures.get(&texture.0));
        let corner_fades = tri.map(|index| vertex_fade_bytes[index as usize]);
        let palette_pass = if indexed.is_some() {
            node_fog_pass
        } else {
            PaletteFogPass::Near
        };
        // CameraFacing retains its legacy mixed-support underlay. WorldSurface
        // does not invent a type-based depth-write policy: FUN_0045A4E0 runs
        // the callback and then submits the original face opcode/material.
        let view_pin_underlay =
            view_pin_triangle_is_color_underlay(view_pin_mode, tri, vertex_type_flags);
        // A projected type-13 corner can land exactly on terrain/water already
        // present in the depth buffer. The renderer's world baseline is
        // LEQUAL; re-establish it here because an earlier model billboard or
        // emissive pass may have left the compatibility state at LESS.
        if draw.overlay == ModelOverlayKind::TerrainSurface
            || view_pin_triangle_needs_coplanar_depth(view_pin_mode, tri, surface_vertices)
        {
            crate::gl::DepthFunc(crate::gl::LEQUAL);
        }
        // A face whose vertices all come from world tf13, including tf11
        // imports, is a complete terrain/water decal (player
        // shadow, plant footprint, and similar authored surface art). Retail
        // rasterizes it after the ground primitive through the painter queue.
        // In GL the callback's exact bilinear endpoint and the terrain's two
        // hardware triangles can differ between the four cell corners, so
        // LEQUAL alone leaves alternating fragments below the depth surface.
        // A raster-depth bias preserves the recovered world coordinates and
        // ordinary object occlusion while giving the later decal the same
        // ordering as retail. Mixed type-13 faces are stems/supports and must
        // retain their ordinary depth slope. The Targetter terrain crosshair
        // is a flat ordinary mesh on the recovered hit, so it opts in through
        // `ModelOverlayKind::TerrainSurface` rather than invented type-13
        // vertices.
        let world_surface_decal = draw.depth_policy == ModelDepthPolicy::Geometry
            && triangle_uses_surface_overlay_depth(
                draw.overlay,
                view_pin_mode,
                tri,
                surface_vertices,
            );
        if world_surface_decal {
            if overlay_depth_policy.disables_decal_depth_test() {
                // Diagnosis only: the decal paints unconditionally, so a
                // missing fragment under this policy rules out depth as the
                // cause. Honest depth values are still written.
                crate::gl::Disable(crate::gl::DEPTH_TEST);
            } else if let Some((factor, units)) = overlay_depth_policy.overlay_fill_offset() {
                crate::gl::Enable(crate::gl::POLYGON_OFFSET_FILL);
                crate::gl::PolygonOffset(factor, units);
            }
        }

        // Per-face normal: parsed pool normal if present, else derived.
        let normal = if tri_idx < normals.len() {
            normals[tri_idx]
        } else {
            compute_face_normal(vertices, tri)
        };
        let blend = material
            .map(|material| material.blend)
            .unwrap_or(WorldSpriteBlend::Masked);
        let opacity = model_face_opacity(blend);
        let shading = face_shading
            .get(tri_idx)
            .copied()
            .unwrap_or(ModelFaceShading::Flat);
        // Per-face material colour (palette colour or white texture tint).
        let base = material
            .map(FaceMaterial::unlit_color)
            .unwrap_or([1.0, 1.0, 1.0]);
        let emissive = material
            .map(|material| material.emissive)
            .unwrap_or([0.0, 0.0, 0.0]);

        let shades = model_lighting_normals(
            shading,
            normal,
            face_corner_normals
                .get(tri_idx)
                .copied()
                .unwrap_or([normal; 3]),
        )
        .map(|normals| {
            normals.map(|normal| {
                retail_model_shade(
                    shade_table,
                    view_light_normal(camera_basis, orientation, normal),
                    light_direction_raw,
                    shade_shift,
                )
            })
        })
        .unwrap_or([None; 3]);
        let gouraud_channels = if shading == ModelFaceShading::Gouraud && textured.is_none() {
            material.and_then(|material| {
                let palette = material.palette_rgb555?;
                let [Some(a), Some(b), Some(c)] = shades else {
                    return None;
                };
                Some(std::array::from_fn::<_, 3, _>(|corner| {
                    let shade = [a, b, c][corner];
                    crate::gouraud_solid::corner_channels(
                        palette,
                        [shade.r, shade.g, shade.b],
                        node_fog_pass,
                        corner_fades[corner],
                        depth_fade.color,
                    )
                }))
            })
        } else {
            None
        };

        match blend {
            WorldSpriteBlend::Masked => crate::gl::Disable(crate::gl::BLEND),
            WorldSpriteBlend::Additive => {
                crate::gl::Enable(crate::gl::BLEND);
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE);
            }
            WorldSpriteBlend::HalfAdditive => {
                crate::gl::Enable(crate::gl::BLEND);
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE_MINUS_SRC_ALPHA);
            }
        }
        crate::gl::DepthMask(if model_face_writes_depth(blend, view_pin_underlay) {
            crate::gl::TRUE
        } else {
            crate::gl::FALSE
        });
        if let Some(program) = program {
            crate::gl::Uniform1f(program.opacity, opacity);
            crate::gl::Uniform1f(
                program.gouraud_solid,
                if gouraud_channels.is_some() { 1.0 } else { 0.0 },
            );
            if let Some(material) = material.filter(|_| gouraud_channels.is_some()) {
                crate::gl::Uniform3f(
                    program.solid_tint,
                    material.color[0],
                    material.color[1],
                    material.color[2],
                );
                crate::gl::Uniform3f(
                    program.solid_emissive,
                    emissive[0],
                    emissive[1],
                    emissive[2],
                );
            }
            crate::gl::Uniform1f(program.palette_fog, palette_pass.shader_enabled());
            crate::gl::Uniform1f(program.blend_mode, model_blend_uniform(blend));
            crate::gl::Uniform1f(
                program.use_explicit_fog_coord,
                if indexed.is_some() { 1.0 } else { 0.0 },
            );
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            if let Some(indexed) = indexed {
                crate::gl::Uniform1f(program.mode, 1.0);
                crate::gl::Uniform1f(
                    program.mask_zero,
                    if indexed.transparent_zero { 1.0 } else { 0.0 },
                );
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, indexed.index.0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, indexed.palette.0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
            } else if let Some(texture) = textured {
                crate::gl::Uniform1f(program.mode, 2.0);
                crate::gl::Uniform1f(program.mask_zero, 0.0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, texture.0);
            } else {
                crate::gl::Uniform1f(program.mode, 0.0);
                crate::gl::Uniform1f(program.mask_zero, 0.0);
            }
        } else if let Some(texture) = textured {
            if let Some(program) = compatibility_program {
                crate::gl::Uniform1f(program.texture_enabled, 1.0);
            }
            let texture = fixed_function_model_texture(
                texture,
                indexed,
                shading,
                material
                    .map(|material| material.flat_shade_row)
                    .unwrap_or(31),
            );
            crate::gl::Enable(crate::gl::TEXTURE_2D);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, texture.0);
            // RGBA fallbacks already encode the source record's bit-0 zero-key
            // policy in their alpha channel. Alpha testing prevents keyed
            // texels from writing depth; bit-0-clear index zero remains opaque.
            crate::gl::Enable(crate::gl::ALPHA_TEST);
            crate::gl::AlphaFunc(crate::gl::GREATER, 0.5);
        } else {
            if let Some(program) = compatibility_program {
                crate::gl::Uniform1f(program.texture_enabled, 0.0);
            }
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
        }

        if program.is_none() {
            // Without the model shader, use packed corner RGB. This retains
            // colour composition but fixed GL interpolates already packed
            // values; only the shader performs native fragment quantization.
            let enabled = depth_fade.enabled && gouraud_channels.is_none();
            if let Some(program) = compatibility_program {
                crate::gl::Uniform1f(program.fog_enabled, if enabled { 1.0 } else { 0.0 });
            } else if enabled {
                crate::gl::Enable(crate::gl::FOG);
            } else {
                crate::gl::Disable(crate::gl::FOG);
            }
        }

        let world_face = tri.iter().any(|&index| {
            matches!(
                mesh.vertex_projection.get(index as usize),
                Some(v2k_formats::models::ModelVertexProjection::WorldPoint(_))
            )
        });
        if world_face {
            crate::gl::PushMatrix();
            crate::gl::LoadMatrixf(world_modelview.as_ptr());
        }
        crate::gl::Begin(crate::gl::TRIANGLES);
        for (corner, &idx) in tri.iter().enumerate() {
            let shade = shades[corner];
            let shade_rgb = shade
                .map(|entry| {
                    [
                        entry.r as f32 / 128.0,
                        entry.g as f32 / 128.0,
                        entry.b as f32 / 128.0,
                    ]
                })
                .unwrap_or([1.0; 3]);
            let shade_row = model_face_palette_row(
                shading,
                shade.map(|entry| entry.shade_level),
                material
                    .map(|material| material.flat_shade_row)
                    .unwrap_or(31),
            );
            let shade_row =
                palette_fog_shade(shading, shade_row, palette_pass, corner_fades[corner]) / 31.0;
            // Indexed textures consume the exact Section-6 palette row in
            // the shader. Uniform solids use the packed palette-plus-light
            // filler. Gouraud solids carry unpacked RGB to their own span;
            // direct-RGB texture fallbacks retain their existing modulation.
            // The fixed fallback can only modulate its brightest-row texture,
            // so use that same authored RGB value rather than inventing a
            // second lighting curve.
            let apply_rgb_shade = textured.is_none() || (program.is_none() && indexed.is_some());
            let uniform_solid = if shading == ModelFaceShading::FlatLit && textured.is_none() {
                material
                    .zip(shade)
                    .and_then(|(material, shade)| flat_lit_palette_color(material, shade))
            } else {
                None
            };
            let lit = if let Some(color) = uniform_solid {
                color
            } else if apply_rgb_shade {
                [
                    base[0] * shade_rgb[0],
                    base[1] * shade_rgb[1],
                    base[2] * shade_rgb[2],
                ]
            } else {
                base
            };
            let add = if textured.is_some() {
                [0.0; 3]
            } else {
                emissive
            };
            crate::gl::FogCoordf(f32::from(corner_fades[corner]) / 255.0);
            let color = if let Some(channels) = gouraud_channels {
                if program.is_some() {
                    channels[corner].map(|value| value / 1024.0)
                } else {
                    let rgb = crate::gouraud_solid::packed_rgb(channels[corner]);
                    let tint = material
                        .expect("Gouraud setup has a palette material")
                        .color;
                    std::array::from_fn(|axis| {
                        (f32::from(rgb[axis]) / 255.0 * tint[axis] + emissive[axis]).min(1.0)
                    })
                }
            } else {
                std::array::from_fn(|axis| (lit[axis] + add[axis]).min(1.0))
            };
            crate::gl::Color4f(
                color[0],
                color[1],
                color[2],
                if program.is_some() {
                    shade_row
                } else {
                    opacity
                },
            );
            if textured.is_some() {
                let uv = face_uvs
                    .get(tri_idx)
                    .map(|uvs| uvs[corner])
                    .unwrap_or([0.0, 0.0]);
                crate::gl::TexCoord2f(uv[0], uv[1]);
            }
            let v = model_face_vertex_world_or_local(
                world_face,
                idx as usize,
                &resolved_vertices,
                &world_vertices,
                scale,
            );
            crate::gl::Vertex3f(v[0], v[1], v[2]);
        }
        crate::gl::End();
        crate::gl::DepthMask(crate::gl::TRUE);
        crate::gl::Disable(crate::gl::BLEND);

        // A multiplicative texture tint cannot illuminate dark painted
        // texels. Add a second, alpha-tested pass for coloured light spill;
        // GL_MODULATE preserves the sprite artwork while additive blending
        // raises its illuminated channels. Reusing the texture also preserves
        // palette-index-zero holes in wings, ribs and teeth.
        if textured.is_some() && emissive.iter().any(|&channel| channel > 0.0) {
            if let Some(program) = program {
                crate::gl::Uniform1f(
                    program.blend_mode,
                    model_blend_uniform(WorldSpriteBlend::Additive),
                );
            }
            crate::gl::Enable(crate::gl::BLEND);
            crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE);
            crate::gl::DepthFunc(crate::gl::LEQUAL);
            crate::gl::DepthMask(crate::gl::FALSE);
            crate::gl::Begin(crate::gl::TRIANGLES);
            for (corner, &idx) in tri.iter().enumerate() {
                let interpolated = shades[corner].map(|entry| entry.shade_level);
                let shade_row = model_face_palette_row(
                    shading,
                    interpolated,
                    material
                        .map(|material| material.flat_shade_row)
                        .unwrap_or(31),
                );
                let shade_row =
                    palette_fog_shade(shading, shade_row, palette_pass, corner_fades[corner])
                        / 31.0;
                crate::gl::FogCoordf(f32::from(corner_fades[corner]) / 255.0);
                crate::gl::Color4f(
                    emissive[0],
                    emissive[1],
                    emissive[2],
                    if program.is_some() { shade_row } else { 1.0 },
                );
                let uv = face_uvs
                    .get(tri_idx)
                    .map(|uvs| uvs[corner])
                    .unwrap_or([0.0, 0.0]);
                crate::gl::TexCoord2f(uv[0], uv[1]);
                let v = model_face_vertex_world_or_local(
                    world_face,
                    idx as usize,
                    &resolved_vertices,
                    &world_vertices,
                    scale,
                );
                crate::gl::Vertex3f(v[0], v[1], v[2]);
            }
            crate::gl::End();
            crate::gl::DepthMask(crate::gl::TRUE);
            crate::gl::DepthFunc(crate::gl::LEQUAL);
            crate::gl::Disable(crate::gl::BLEND);
        }
        if world_face {
            crate::gl::PopMatrix();
        }
        if world_surface_decal {
            if overlay_depth_policy.disables_decal_depth_test() {
                crate::gl::Enable(crate::gl::DEPTH_TEST);
            } else {
                crate::gl::Disable(crate::gl::POLYGON_OFFSET_FILL);
            }
        }
    }

    // Ribbons and hairlines below use their own material/fog paths.
    if let Some(program) = program {
        crate::gl::Uniform1f(program.gouraud_solid, 0.0);
    } else if let Some(program) = compatibility_program {
        crate::gl::Uniform1f(
            program.fog_enabled,
            if depth_fade.enabled { 1.0 } else { 0.0 },
        );
    } else if depth_fade.enabled {
        crate::gl::Enable(crate::gl::FOG);
    } else {
        crate::gl::Disable(crate::gl::FOG);
    }

    // Opcode 0x02/0x22 are recovered as `ModelEdge` data. Retail's near pass
    // `FUN_00459000` rasterizes 0x22 through screen-space `FUN_0047AA20` with
    // sprite +0x10/+0x12 width and `FUN_004594C0` perspective size; the
    // recovered tapered-quad selection runs in edge_quads and the survivors
    // fill here via the B/fogged `FUN_00459550` -> `FUN_0047F060` route:
    // implicit UVs, Section-3 blend flags, flat shade row, and separately
    // interpolated endpoint fade bytes. Palette-style 0x02 hairlines use the
    // same LUT on two endpoints (`FUN_00458c60`) and fill as pixel-ortho lines.
    // Unresolved sprite widths stay unsubmitted.
    //
    // The selector's contract is world-space endpoints (it projects against
    // `camera_position`). `FUN_0046ECF0` bypasses the model transform after
    // the type-14 callback, so Sub-H worlds go to the gate directly. Inverting
    // those points into raw-local and back through `O` is not retail: a
    // slightly non-orthonormal 13F70 basis lands the foot next to the eye,
    // the near-table overflows, and newant legs vanish while their type-13
    // 0x22 ribbons still plant as ground blobs. Type-0 hips still take
    // `position + O · (raw · scale)`.
    let submissions =
        edge_intrinsics.map_or_else(crate::edge_quads::EdgeSelection::default, |intr| {
            // The edge core keeps its established 8.8 width/window-Z units;
            // the scene's separate raw near guard only affects admission.
            let world_vertices = world_vertices
                .iter()
                .zip(&vertex_admitted)
                .map(|(point, &admitted)| if admitted { *point } else { None })
                .collect::<Vec<_>>();
            let projection = match mesh.edge_projection {
                crate::renderer::ModelEdgeProjection::Compatibility => {
                    crate::edge_quads::ModelEdgeProjectionInput::Compatibility {
                        authority: draw.projection_authority,
                        vertices: &world_vertices,
                        cam_pos: camera_position,
                        basis_rows: camera_basis,
                    }
                }
                crate::renderer::ModelEdgeProjection::CommandSnapshots(snapshots) => {
                    crate::edge_quads::ModelEdgeProjectionInput::CommandSnapshots {
                        snapshots,
                        authority: draw.projection_authority,
                        compatibility_vertices: &world_vertices,
                        cam_pos: camera_position,
                        basis_rows: camera_basis,
                    }
                }
            };
            let (selection, stats) =
                crate::edge_quads::select_model_edges(crate::edge_quads::ModelEdgeRequest {
                    edges: mesh.edges,
                    edge_widths: mesh.edge_widths,
                    edge_materials: mesh.edge_materials,
                    intr: &intr,
                    projection,
                });
            report_native_edge_boundaries(stats);
            selection
        });
    // FUN_0047EF10 / FUN_00458c60 paint constructor pixels. Submit those in
    // a HUD-style pixel ortho so the fill cannot leave screen space, with
    // window Z matched to the 3D perspective pass.
    if let Some(intr) = edge_intrinsics {
        let draw_quads = LIVE_MODEL_EDGE_QUAD_FILL && !submissions.quads.is_empty();
        let draw_lines = LIVE_MODEL_EDGE_LINE_FILL && !submissions.lines.is_empty();
        if draw_quads || draw_lines {
            begin_edge_fill_screen_space(&intr, depth_fade, program, compatibility_program);
            if draw_quads {
                submit_edge_quads_gl(
                    &submissions.quads,
                    &intr,
                    depth_fade,
                    program,
                    compatibility_program,
                    indexed_textures,
                );
            }
            if draw_lines {
                submit_edge_lines_gl(
                    &submissions.lines,
                    &intr,
                    depth_fade,
                    program,
                    compatibility_program,
                );
            }
            end_edge_fill_screen_space(program, compatibility_program, intr.projection_effect);
        }
    }

    crate::gl::Disable(crate::gl::ALPHA_TEST);
    crate::gl::Disable(crate::gl::TEXTURE_2D);
    crate::gl::Disable(crate::gl::POLYGON_OFFSET_FILL);
    crate::gl::DepthFunc(crate::gl::LEQUAL);

    if program.is_some() || compatibility_program.is_some() {
        crate::gl::UseProgram(0);
        crate::gl::ActiveTexture(crate::gl::TEXTURE1);
        crate::gl::BindTexture(crate::gl::TEXTURE_2D, 0);
        crate::gl::Disable(crate::gl::TEXTURE_2D);
        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
        crate::gl::BindTexture(crate::gl::TEXTURE_2D, 0);
    }
    crate::gl::Fogi(crate::gl::FOG_MODE, crate::gl::LINEAR as i32);
    crate::gl::Fogf(crate::gl::FOG_START, fog_near);
    crate::gl::Fogf(crate::gl::FOG_END, fog_far);
    let scene_fog_color = [fog_color[0], fog_color[1], fog_color[2], 1.0];
    crate::gl::Fogfv(crate::gl::FOG_COLOR, scene_fog_color.as_ptr());
    if fog_was_enabled {
        crate::gl::Enable(crate::gl::FOG);
    } else {
        crate::gl::Disable(crate::gl::FOG);
    }

    crate::gl::PopMatrix();
}

fn report_native_edge_boundaries(stats: crate::edge_quads::EdgeGateStats) {
    use std::sync::atomic::{AtomicU8, Ordering};
    static REPORTED: AtomicU8 = AtomicU8::new(0);
    for (bit, count, boundary) in [
        (
            1,
            stats.native_endpoint_missing,
            "missing command-time VIEW endpoint",
        ),
        (
            2,
            stats.native_projection_missing,
            "unowned native screen globals",
        ),
        (
            4,
            stats.native_viewport_mismatch,
            "unowned native-to-resized viewport mapping",
        ),
        (
            8,
            stats.snapshot_count_mismatch,
            "misaligned command-time endpoint receipts",
        ),
    ] {
        if count != 0 && REPORTED.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
            eprintln!(
                "native model edge presentation blocked: {boundary} ({count} edges in this draw)"
            );
        }
    }
}

/// Live `0x22` GL fill. Construction, `DAT_004c5268`, and `FUN_0047EF10`
/// triangulation are recovered; the fill paints constructor pixels in a
/// HUD-style ortho.
const LIVE_MODEL_EDGE_QUAD_FILL: bool = true;

/// Live `0x02` palette hairlines. `FUN_00458c60` is a 2D screen-space filler;
/// 3D view-space `GL_LINES` produced full-width cables.
const LIVE_MODEL_EDGE_LINE_FILL: bool = true;

unsafe fn begin_edge_fill_screen_space(
    intr: &crate::edge_quads::ScreenIntrinsics,
    depth_fade: ResolvedModelDepthFade,
    program: Option<&ModelProgram>,
    compatibility_program: Option<&CompatibilityProgram>,
) {
    // Source endpoints have already passed the integer projector. Native
    // screen-space fillers do not project/wobble their stored corners again.
    if let Some(program) = program {
        crate::gl::Uniform1f(program.projection.enabled, 0.0);
    } else if let Some(program) = compatibility_program {
        crate::gl::Uniform1f(program.projection.enabled, 0.0);
    }
    crate::gl::MatrixMode(crate::gl::PROJECTION);
    crate::gl::PushMatrix();
    crate::gl::LoadIdentity();
    crate::gl::Ortho(
        0.0,
        f64::from(intr.width_px.max(1)),
        f64::from(intr.height_px.max(1)),
        0.0,
        -1.0,
        1.0,
    );
    crate::gl::MatrixMode(crate::gl::MODELVIEW);
    crate::gl::PushMatrix();
    crate::gl::LoadIdentity();
    crate::gl::Disable(crate::gl::CULL_FACE);
    // The model's ordinary fog derives from eye Z, but these vertices are
    // already projected screen pixels. Retail's B path carries one quantized
    // fade byte at each source endpoint, so transport that byte explicitly.
    crate::gl::Fogi(
        crate::gl::FOG_COORDINATE_SOURCE,
        crate::gl::FOG_COORDINATE as i32,
    );
    if let Some(program) = program {
        crate::gl::Disable(crate::gl::FOG);
        crate::gl::Uniform1f(
            program.fog_enabled,
            if depth_fade.enabled { 1.0 } else { 0.0 },
        );
        crate::gl::Uniform1f(program.use_explicit_fog_coord, 1.0);
    } else if let Some(program) = compatibility_program {
        crate::gl::Disable(crate::gl::FOG);
        crate::gl::Uniform1f(
            program.fog_enabled,
            if depth_fade.enabled { 1.0 } else { 0.0 },
        );
        crate::gl::Uniform1f(program.use_explicit_fog_coord, 1.0);
    } else {
        crate::gl::Fogi(crate::gl::FOG_MODE, crate::gl::LINEAR as i32);
        crate::gl::Fogf(crate::gl::FOG_START, 0.0);
        crate::gl::Fogf(crate::gl::FOG_END, 1.0);
        let color = [
            depth_fade.color[0],
            depth_fade.color[1],
            depth_fade.color[2],
            1.0,
        ];
        crate::gl::Fogfv(crate::gl::FOG_COLOR, color.as_ptr());
        if depth_fade.enabled {
            crate::gl::Enable(crate::gl::FOG);
        } else {
            crate::gl::Disable(crate::gl::FOG);
        }
    }
}

/// Select the same corner coordinates for both face colour passes. A mixed
/// callback face is emitted under the saved camera matrix; ordinary faces
/// retain the body matrix and local scaling.
fn model_face_vertex_world_or_local(
    world_face: bool,
    index: usize,
    local: &[Option<[f32; 3]>],
    world: &[Option<[f32; 3]>],
    scale: f32,
) -> [f32; 3] {
    if world_face {
        world[index].expect("an admitted world face has all corner endpoints")
    } else {
        local[index]
            .expect("an admitted local face has all corner endpoints")
            .map(|value| value * scale)
    }
}

unsafe fn end_edge_fill_screen_space(
    program: Option<&ModelProgram>,
    compatibility_program: Option<&CompatibilityProgram>,
    projection_effect: ProjectionEffect,
) {
    let enabled = match projection_effect {
        ProjectionEffect::None => 0.0,
        ProjectionEffect::RetailUnderwater { .. } => 1.0,
    };
    if let Some(program) = program {
        crate::gl::Uniform1f(program.use_explicit_fog_coord, 0.0);
        crate::gl::Uniform1f(program.projection.enabled, enabled);
    } else if let Some(program) = compatibility_program {
        crate::gl::Uniform1f(program.use_explicit_fog_coord, 0.0);
        crate::gl::Uniform1f(program.projection.enabled, enabled);
    }
    crate::gl::Fogi(
        crate::gl::FOG_COORDINATE_SOURCE,
        crate::gl::FRAGMENT_DEPTH as i32,
    );
    crate::gl::MatrixMode(crate::gl::MODELVIEW);
    crate::gl::PopMatrix();
    crate::gl::MatrixMode(crate::gl::PROJECTION);
    crate::gl::PopMatrix();
    crate::gl::MatrixMode(crate::gl::MODELVIEW);
}

/// `FUN_0047EF10` fill for recovered `0x22` quads: same Section-3 blend /
/// depth-write / indexed-or-rgba bind as model faces, flat shade row, implicit
/// UVs, triangles A-C-D and A-D-B. Corners are constructor screen pixels with
/// window Z matched to the 3D pass. Caller has already installed pixel ortho.
unsafe fn submit_edge_quads_gl(
    submissions: &[crate::edge_quads::EdgeQuadSubmission],
    intr: &crate::edge_quads::ScreenIntrinsics,
    depth_fade: ResolvedModelDepthFade,
    program: Option<&ModelProgram>,
    compatibility_program: Option<&CompatibilityProgram>,
    indexed_textures: &HashMap<u32, IndexedModelTextures>,
) {
    if submissions.is_empty() {
        return;
    }
    crate::gl::DepthFunc(crate::gl::LEQUAL);
    for quad in submissions {
        let Some(fade_bytes) = edge_quad_fade_bytes(depth_fade, quad.endpoint_depths_raw) else {
            // FUN_00459550 rejects a B-pass edge only when both endpoint
            // bytes have reached the terminal 0xff value.
            continue;
        };
        let z = intr.ortho_fill_z(quad.mid_depth_raw);
        let material = quad.material;
        let blend = material.blend;
        let opacity = model_face_opacity(blend);
        match blend {
            WorldSpriteBlend::Masked => crate::gl::Disable(crate::gl::BLEND),
            WorldSpriteBlend::Additive => {
                crate::gl::Enable(crate::gl::BLEND);
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE);
            }
            WorldSpriteBlend::HalfAdditive => {
                crate::gl::Enable(crate::gl::BLEND);
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE_MINUS_SRC_ALPHA);
            }
        }
        crate::gl::DepthMask(if model_face_writes_depth(blend, false) {
            crate::gl::TRUE
        } else {
            crate::gl::FALSE
        });
        let textured = material.texture;
        let indexed = textured.and_then(|texture| indexed_textures.get(&texture.0));
        let palette_pass = if indexed.is_some() {
            if depth_fade.enabled {
                PaletteFogPass::Far
            } else {
                PaletteFogPass::Near
            }
        } else {
            PaletteFogPass::Near
        };
        if let Some(program) = program {
            crate::gl::Uniform1f(program.opacity, opacity);
            crate::gl::Uniform1f(program.palette_fog, palette_pass.shader_enabled());
            crate::gl::Uniform1f(program.blend_mode, model_blend_uniform(blend));
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            if let Some(indexed) = indexed {
                crate::gl::Uniform1f(program.mode, 1.0);
                crate::gl::Uniform1f(
                    program.mask_zero,
                    if indexed.transparent_zero { 1.0 } else { 0.0 },
                );
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, indexed.index.0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, indexed.palette.0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
            } else if let Some(texture) = textured {
                crate::gl::Uniform1f(program.mode, 2.0);
                crate::gl::Uniform1f(program.mask_zero, 0.0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, texture.0);
            } else {
                crate::gl::Uniform1f(program.mode, 0.0);
                crate::gl::Uniform1f(program.mask_zero, 0.0);
            }
        } else if let Some(texture) = textured {
            if let Some(program) = compatibility_program {
                crate::gl::Uniform1f(program.texture_enabled, 1.0);
            }
            let texture = fixed_function_model_texture(
                texture,
                indexed,
                ModelFaceShading::Flat,
                material.flat_shade_row,
            );
            crate::gl::Enable(crate::gl::TEXTURE_2D);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, texture.0);
            crate::gl::Enable(crate::gl::ALPHA_TEST);
            crate::gl::AlphaFunc(crate::gl::GREATER, 0.5);
        } else {
            if let Some(program) = compatibility_program {
                crate::gl::Uniform1f(program.texture_enabled, 0.0);
            }
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
        }

        let color = material.unlit_color();
        crate::gl::Begin(crate::gl::TRIANGLES);
        for tri in crate::edge_quads::EDGE_QUAD_TRIANGLES {
            for corner_index in tri {
                let uv = crate::edge_quads::EDGE_QUAD_UVS[corner_index];
                crate::gl::FogCoordf(f32::from(fade_bytes[corner_index]) / 255.0);
                let alpha = if program.is_some() {
                    palette_fog_shade(
                        ModelFaceShading::Flat,
                        material.flat_shade_row,
                        palette_pass,
                        fade_bytes[corner_index],
                    ) / 31.0
                } else {
                    opacity
                };
                crate::gl::Color4f(color[0], color[1], color[2], alpha);
                if textured.is_some() {
                    crate::gl::TexCoord2f(uv[0], uv[1]);
                }
                let [sx, sy] = quad.screen_corners[corner_index];
                crate::gl::Vertex3f(sx, sy, z);
            }
        }
        crate::gl::End();
        crate::gl::DepthMask(crate::gl::TRUE);
        crate::gl::Disable(crate::gl::BLEND);
    }
}

/// `FUN_00458c60` / `LAB_0047a740` palette hairlines: two constructor screen
/// endpoints as pixel-ortho `GL_LINES`. Untextured; colour comes from
/// Section-7. View-space 3D lines projected the segment through the camera
/// frustum and produced full-width cables.
unsafe fn submit_edge_lines_gl(
    submissions: &[crate::edge_quads::EdgeLineSubmission],
    intr: &crate::edge_quads::ScreenIntrinsics,
    depth_fade: ResolvedModelDepthFade,
    program: Option<&ModelProgram>,
    compatibility_program: Option<&CompatibilityProgram>,
) {
    if submissions.is_empty() {
        return;
    }
    crate::gl::DepthFunc(crate::gl::LEQUAL);
    crate::gl::Disable(crate::gl::TEXTURE_2D);
    crate::gl::Disable(crate::gl::ALPHA_TEST);
    if let Some(program) = program {
        crate::gl::Uniform1f(program.mode, 0.0);
        crate::gl::Uniform1f(program.mask_zero, 0.0);
        crate::gl::Uniform1f(program.palette_fog, 0.0);
    } else if let Some(program) = compatibility_program {
        crate::gl::Uniform1f(program.texture_enabled, 0.0);
    }
    crate::gl::LineWidth(1.0);
    for line in submissions {
        let [start_fade, end_fade] =
            edge_endpoint_fade_bytes(depth_fade, [line.start_depth_raw, line.end_depth_raw]);
        let start_z = intr.ortho_fill_z(line.start_depth_raw);
        let end_z = intr.ortho_fill_z(line.end_depth_raw);
        let blend = line.material.blend;
        let opacity = model_face_opacity(blend);
        match blend {
            WorldSpriteBlend::Masked => crate::gl::Disable(crate::gl::BLEND),
            WorldSpriteBlend::Additive => {
                crate::gl::Enable(crate::gl::BLEND);
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE);
            }
            WorldSpriteBlend::HalfAdditive => {
                crate::gl::Enable(crate::gl::BLEND);
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE_MINUS_SRC_ALPHA);
            }
        }
        crate::gl::DepthMask(if model_face_writes_depth(blend, false) {
            crate::gl::TRUE
        } else {
            crate::gl::FALSE
        });
        let shade_row = f32::from(line.material.flat_shade_row.min(31)) / 31.0;
        let color = line.material.unlit_color();
        let alpha = if program.is_some() {
            shade_row
        } else {
            opacity
        };
        if let Some(program) = program {
            crate::gl::Uniform1f(program.opacity, opacity);
        }
        crate::gl::Color4f(color[0], color[1], color[2], alpha);
        crate::gl::Begin(crate::gl::LINES);
        crate::gl::FogCoordf(f32::from(start_fade) / 255.0);
        crate::gl::Vertex3f(line.start[0], line.start[1], start_z);
        crate::gl::FogCoordf(f32::from(end_fade) / 255.0);
        crate::gl::Vertex3f(line.end[0], line.end[1], end_z);
        crate::gl::End();
        crate::gl::DepthMask(crate::gl::TRUE);
        crate::gl::Disable(crate::gl::BLEND);
    }
}

/// OpenGL 2.1 fixed-function renderer.
pub struct GlRenderer {
    window: Window,
    _gl_context: GLContext,
    output_width: u32,
    output_height: u32,
    reference_width: u32,
    reference_height: u32,
    width: u32,
    height: u32,
    scaling_mode: ScalingMode,
    ui_submission_policy: crate::ui_mapping::UiSubmissionPolicy,
    viewport: RenderViewport,
    /// Exact selection envelope used to build the retained terrain mesh.
    terrain_cache_footprint: Option<TerrainFootprint>,
    terrain_light_revision: u64,
    /// FNV of the live Section-10 terrain type bytes. Terrain infection and
    /// damage mutate this grid in place, outside renderer ownership.
    terrain_type_signature: u64,
    /// Process-lifetime `FUN_00433530` globals. This advances only on terrain
    /// draw calls, so menus, pauses, and blocking loads freeze it like retail.
    terrain_infection_animation: crate::terrain_tiles::InfectionTerrainAnimation,
    /// Last light window submitted with terrain; water reads the same field.
    terrain_lights: Option<crate::terrain_light::TerrainLightWindow>,
    terrain_vertices: Vec<f32>, // interleaved: x,y,z, r,g,b, u,v
    terrain_vertex_count: usize,
    /// Fixed-row infection overlay stream: x,y,z,u,v. It shares the terrain
    /// geometry so the keyed marching-square shapes replace the base texels.
    terrain_infection_vertices: Vec<f32>,
    terrain_infection_vertex_count: usize,
    /// Reused scratch buffer for the per-frame wave-displaced water mesh
    /// (interleaved x,y,z, r,g,b). Rebuilt each frame in `draw_water`.
    water_vertices: Vec<f32>,
    /// Last camera pose (from `set_camera`), used to build the original-style
    /// bounded terrain window around the viewer.
    cam_pos: [f32; 3],
    cam_forward: [f32; 2],
    /// `FUN_0042F270`'s pitch-dependent terrain-row origin in signed 8.8.
    terrain_row_lead_raw: i32,
    /// Tangent of the horizontal half-FOV. The retail scan dimensions target
    /// a 4:3 framebuffer; widescreen needs conservative hidden overscan so a
    /// side edge cannot enter the viewport before the terminal fog plane.
    cam_horizontal_half_fov_tan: f32,
    /// View-space right/up/back rows, used by runtime type-13 vertex pins.
    cam_view_basis: [[f32; 3]; 3],
    /// Furthest visible distance (fog end, from `set_fog`), used by water.
    view_distance: f32,
    /// Retained camera projection terms for screen-space construction:
    /// `[m00, m11, offset_x_ndc, offset_y_ndc]` from `set_camera`.
    cam_projection_terms: [f32; 4],
    cam_near: f32,
    cam_far: f32,
    /// Controlled ground-overlay depth policy
    /// ([`OverlayDepthPolicy`]); default reproduces the established port
    /// behavior.
    overlay_depth_policy: OverlayDepthPolicy,
    /// Fog start (from `set_fog`); the water pass fades between `fog_near`
    /// and `view_distance` in place of GL fog.
    fog_near: f32,
    world_fog_enabled: bool,
    world_model_fog: Option<crate::renderer::WorldModelFog>,
    active_scene: RenderScene,
    world_fog_color: [f32; 3],
    indexed_model_textures: HashMap<u32, IndexedModelTextures>,
    projection_effect: ProjectionEffect,
    scene_projection_authority: crate::projection::SceneProjectionAuthority,
    model_program: Option<ModelProgram>,
    terrain_program: Option<TerrainProgram>,
    water_program: Option<WaterProgram>,
    compatibility_program: Option<CompatibilityProgram>,
}

impl GlRenderer {
    /// Create an OpenGL renderer from an SDL2 window (takes ownership).
    pub fn new(window: Window, width: u32, height: u32) -> Result<Self, String> {
        let gl_context = window.gl_create_context()?;
        window.gl_make_current(&gl_context)?;

        // Load GL function pointers
        crate::gl::load_with(|s| window.subsystem().gl_get_proc_address(s) as *const _);

        unsafe {
            crate::gl::Enable(crate::gl::DEPTH_TEST);
            crate::gl::DepthFunc(crate::gl::LEQUAL);
            crate::gl::Enable(crate::gl::CULL_FACE);
            crate::gl::CullFace(crate::gl::BACK);
            crate::gl::FrontFace(crate::gl::CCW);
            crate::gl::ShadeModel(crate::gl::SMOOTH);
            crate::gl::Viewport(0, 0, width as i32, height as i32);
        }

        // Enable vsync
        window
            .subsystem()
            .gl_set_swap_interval(sdl2::video::SwapInterval::VSync)
            .ok();

        let version = unsafe {
            let p = crate::gl::GetString(crate::gl::VERSION);
            if p.is_null() {
                "unknown".to_string()
            } else {
                std::ffi::CStr::from_ptr(p as *const _)
                    .to_string_lossy()
                    .to_string()
            }
        };
        eprintln!("OpenGL initialized: {}", version);

        let model_program = match unsafe { create_model_program() } {
            Ok(program) => {
                eprintln!("Indexed affine model shader active");
                Some(program)
            }
            Err(error) => {
                eprintln!("Model shader unavailable; using fixed-function fallback: {error}");
                None
            }
        };
        let terrain_program = match unsafe { create_terrain_program() } {
            Ok(program) => {
                eprintln!("Indexed affine terrain shader active");
                Some(program)
            }
            Err(error) => {
                eprintln!("Terrain shader unavailable; using RGBA fallback: {error}");
                None
            }
        };
        let water_program = match unsafe { create_water_program() } {
            Ok(program) => {
                eprintln!("Indexed affine water shader active");
                Some(program)
            }
            Err(error) => {
                eprintln!("Water shader unavailable; using RGBA fallback: {error}");
                None
            }
        };
        let compatibility_program = match unsafe { create_compatibility_program() } {
            Ok(program) => {
                eprintln!("Projected RGBA compatibility shader active");
                Some(program)
            }
            Err(error) => {
                eprintln!(
                    "Compatibility shader unavailable; fixed-function 3-D fallbacks cannot use underwater projection: {error}"
                );
                None
            }
        };
        Ok(Self {
            window,
            _gl_context: gl_context,
            output_width: width,
            output_height: height,
            reference_width: width,
            reference_height: height,
            width,
            height,
            scaling_mode: ScalingMode::Native,
            ui_submission_policy: Default::default(),
            viewport: RenderViewport::for_output(width, height, width, height, ScalingMode::Native),
            terrain_cache_footprint: None,
            terrain_light_revision: 0,
            terrain_type_signature: 0,
            terrain_infection_animation: Default::default(),
            terrain_lights: None,
            terrain_vertices: Vec::new(),
            terrain_vertex_count: 0,
            terrain_infection_vertices: Vec::new(),
            terrain_infection_vertex_count: 0,
            water_vertices: Vec::new(),
            cam_pos: [0.0, 0.0, 0.0],
            cam_forward: [0.0, -1.0],
            terrain_row_lead_raw: v2k_core::render_scan::DEFAULT_LEAD_RAW,
            cam_horizontal_half_fov_tan: 1.0,
            cam_view_basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            view_distance: 400.0,
            cam_projection_terms: [1.0, 1.0, 0.0, 0.0],
            cam_near: 0.1,
            cam_far: 500.0,
            overlay_depth_policy: OverlayDepthPolicy::default(),
            fog_near: 60.0,
            world_fog_enabled: false,
            world_model_fog: None,
            active_scene: RenderScene::World,
            world_fog_color: [0.0; 3],
            indexed_model_textures: HashMap::new(),
            projection_effect: ProjectionEffect::None,
            scene_projection_authority: crate::projection::SceneProjectionAuthority::default(),
            model_program,
            terrain_program,
            water_program,
            compatibility_program,
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
        self.bind_render_target();
    }

    fn bind_render_target(&self) {
        unsafe {
            crate::gl::Viewport(
                self.viewport.x,
                self.viewport.y,
                self.viewport.physical_width as i32,
                self.viewport.physical_height as i32,
            );
        }
    }

    fn configure_projection_effect(&mut self, effect: ProjectionEffect) {
        self.projection_effect = effect;
        let viewport = [self.width, self.height];
        let offsets = match effect {
            ProjectionEffect::None => None,
            ProjectionEffect::RetailUnderwater { tick } => {
                Some(retail_underwater_offset_table(tick))
            }
        };
        unsafe {
            if let Some(program) = &self.model_program {
                apply_projection_uniforms(
                    program.id,
                    program.projection,
                    offsets.as_ref(),
                    viewport,
                );
            }
            if let Some(program) = &self.terrain_program {
                apply_projection_uniforms(
                    program.id,
                    program.projection,
                    offsets.as_ref(),
                    viewport,
                );
            }
            if let Some(program) = &self.water_program {
                apply_projection_uniforms(
                    program.id,
                    program.projection,
                    offsets.as_ref(),
                    viewport,
                );
            }
            if let Some(program) = &self.compatibility_program {
                apply_projection_uniforms(
                    program.id,
                    program.projection,
                    offsets.as_ref(),
                    viewport,
                );
            }
            crate::gl::UseProgram(0);
        }
    }

    fn transient_sprite_filter(&self) -> i32 {
        if self.scaling_mode == ScalingMode::Native {
            crate::gl::NEAREST as i32
        } else {
            // Classic modes emulate the filtered enlargement of the complete
            // 320×240/640×480 framebuffer on a modern output.
            crate::gl::LINEAR as i32
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_overlay_sprite(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        blend: OverlaySpriteBlend,
    ) {
        let x0 = x as f32;
        let y0 = y as f32;
        let x1 = x0 + width as f32;
        let y1 = y0 + height as f32;
        self.draw_overlay_sprite_corners(
            rgba,
            width,
            height,
            [(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
            blend,
        );
    }

    fn draw_overlay_sprite_corners(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        corners: [(f32, f32); 4],
        blend: OverlaySpriteBlend,
    ) {
        if rgba.is_empty() || width == 0 || height == 0 {
            return;
        }

        let filter = overlay_sprite_filter(self.scaling_mode, blend);
        unsafe {
            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();
            crate::gl::Ortho(0.0, self.width as f64, self.height as f64, 0.0, -1.0, 1.0);
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();

            crate::gl::Disable(crate::gl::DEPTH_TEST);
            crate::gl::Disable(crate::gl::CULL_FACE);
            crate::gl::Enable(crate::gl::TEXTURE_2D);
            let opacity = match blend {
                OverlaySpriteBlend::ConventionalAlpha => {
                    crate::gl::Disable(crate::gl::ALPHA_TEST);
                    crate::gl::Enable(crate::gl::BLEND);
                    crate::gl::BlendFunc(crate::gl::SRC_ALPHA, crate::gl::ONE_MINUS_SRC_ALPHA);
                    1.0
                }
                OverlaySpriteBlend::Retail(WorldSpriteBlend::Masked) => {
                    crate::gl::Enable(crate::gl::ALPHA_TEST);
                    crate::gl::AlphaFunc(crate::gl::GREATER, 0.5);
                    crate::gl::Disable(crate::gl::BLEND);
                    1.0
                }
                OverlaySpriteBlend::Retail(WorldSpriteBlend::Additive) => {
                    crate::gl::Enable(crate::gl::ALPHA_TEST);
                    crate::gl::AlphaFunc(crate::gl::GREATER, 0.5);
                    crate::gl::Enable(crate::gl::BLEND);
                    crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE);
                    1.0
                }
                OverlaySpriteBlend::Retail(WorldSpriteBlend::HalfAdditive) => {
                    crate::gl::Enable(crate::gl::ALPHA_TEST);
                    crate::gl::AlphaFunc(crate::gl::GREATER, 0.0);
                    crate::gl::Enable(crate::gl::BLEND);
                    crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE_MINUS_SRC_ALPHA);
                    HALF_ADDITIVE_ALPHA
                }
            };

            let mut tex_id: u32 = 0;
            crate::gl::GenTextures(1, &mut tex_id);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, tex_id);
            crate::gl::TexParameteri(crate::gl::TEXTURE_2D, crate::gl::TEXTURE_MIN_FILTER, filter);
            crate::gl::TexParameteri(crate::gl::TEXTURE_2D, crate::gl::TEXTURE_MAG_FILTER, filter);
            crate::gl::TexParameteri(
                crate::gl::TEXTURE_2D,
                crate::gl::TEXTURE_WRAP_S,
                crate::gl::CLAMP_TO_EDGE as i32,
            );
            crate::gl::TexParameteri(
                crate::gl::TEXTURE_2D,
                crate::gl::TEXTURE_WRAP_T,
                crate::gl::CLAMP_TO_EDGE as i32,
            );
            crate::gl::TexImage2D(
                crate::gl::TEXTURE_2D,
                0,
                crate::gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                crate::gl::RGBA,
                crate::gl::UNSIGNED_BYTE,
                rgba.as_ptr() as *const _,
            );

            crate::gl::Color4f(1.0, 1.0, 1.0, opacity);
            crate::gl::Begin(crate::gl::QUADS);
            crate::gl::TexCoord2f(0.0, 0.0);
            crate::gl::Vertex2f(corners[0].0, corners[0].1);
            crate::gl::TexCoord2f(1.0, 0.0);
            crate::gl::Vertex2f(corners[1].0, corners[1].1);
            crate::gl::TexCoord2f(1.0, 1.0);
            crate::gl::Vertex2f(corners[2].0, corners[2].1);
            crate::gl::TexCoord2f(0.0, 1.0);
            crate::gl::Vertex2f(corners[3].0, corners[3].1);
            crate::gl::End();

            crate::gl::DeleteTextures(1, &tex_id);
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            crate::gl::Disable(crate::gl::BLEND);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::Color4f(1.0, 1.0, 1.0, 1.0);
            crate::gl::Enable(crate::gl::DEPTH_TEST);
            crate::gl::Enable(crate::gl::CULL_FACE);

            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PopMatrix();
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PopMatrix();
        }
    }

    fn upload_persistent_texture(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Option<TextureId> {
        if rgba.is_empty() || width == 0 || height == 0 {
            return None;
        }
        if rgba.len() < (width as usize) * (height as usize) * 4 {
            return None;
        }
        let mut tex_id = 0;
        unsafe {
            crate::gl::GenTextures(1, &mut tex_id);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, tex_id);
            crate::gl::TexParameteri(
                crate::gl::TEXTURE_2D,
                crate::gl::TEXTURE_MIN_FILTER,
                crate::gl::NEAREST as i32,
            );
            crate::gl::TexParameteri(
                crate::gl::TEXTURE_2D,
                crate::gl::TEXTURE_MAG_FILTER,
                crate::gl::NEAREST as i32,
            );
            crate::gl::TexParameteri(
                crate::gl::TEXTURE_2D,
                crate::gl::TEXTURE_WRAP_S,
                crate::gl::CLAMP_TO_EDGE as i32,
            );
            crate::gl::TexParameteri(
                crate::gl::TEXTURE_2D,
                crate::gl::TEXTURE_WRAP_T,
                crate::gl::CLAMP_TO_EDGE as i32,
            );
            crate::gl::TexImage2D(
                crate::gl::TEXTURE_2D,
                0,
                crate::gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                crate::gl::RGBA,
                crate::gl::UNSIGNED_BYTE,
                rgba.as_ptr() as *const _,
            );
        }
        Some(TextureId(tex_id))
    }

    fn delete_persistent_texture(&mut self, id: TextureId) {
        unsafe {
            crate::gl::DeleteTextures(1, &id.0);
        }
    }

    /// Snapshot the selection inputs shared by terrain and water. The exact
    /// derived footprint also owns cache validity; setters need no approximate
    /// cell, yaw, FOV or fog-row invalidation rules.
    fn terrain_scan(&self) -> TerrainScan {
        TerrainScan {
            camera_xz: [self.cam_pos[0], self.cam_pos[2]],
            forward: self.cam_forward,
            row_lead_raw: self.terrain_row_lead_raw,
            fog_far: self.world_fog_enabled.then_some(self.view_distance),
            horizontal_half_fov_tan: self.cam_horizontal_half_fov_tan,
        }
    }

    /// Build the camera-relative terrain window from the wrapped grid.
    ///
    /// `FUN_004330D0` caps the original scan at 52 columns × 30 rows. The
    /// opaque pass centers the wide axis on the camera and advances the other
    /// axis from the camera plus a small pitch lead. Rotating that footprint
    /// with the port camera also covers free-fly and reconstructed intro shots.
    fn build_terrain_cache(
        &mut self,
        terrain: &TerrainGrid,
        colors: &[PaletteEntry],
        frames: Option<&crate::terrain_tiles::TerrainFrames>,
        lights: Option<&crate::terrain_light::TerrainLightWindow>,
        infection_motion: &crate::terrain_tiles::InfectionMotionFrame,
        footprint: TerrainFootprint,
    ) {
        // Fractional movement can rebuild each frame. Reuse the bounded mesh
        // allocations just as water reuses its per-frame scratch stream.
        let mut verts = std::mem::take(&mut self.terrain_vertices);
        let mut infection_verts = std::mem::take(&mut self.terrain_infection_vertices);
        verts.clear();
        infection_verts.clear();
        let height_scale = v2k_formats::terrain::HEIGHT_SCALE;
        for [world_x, world_z] in footprint.cells() {
            // Wrap sample indices only. Geometry stays in the camera's
            // nearest unwrapped image of the torus.
            let x = world_x.rem_euclid(GRID_SIZE as i32) as usize;
            let z = world_z.rem_euclid(GRID_SIZE as i32) as usize;
            let xn = (x + 1) % GRID_SIZE;
            let zn = (z + 1) % GRID_SIZE;
            let c00 = terrain.cell(x, z).unwrap();
            let c10 = terrain.cell(x, zn).unwrap();
            let c01 = terrain.cell(xn, z).unwrap();
            let c11 = terrain.cell(xn, zn).unwrap();

            // Height byte is a signed i8 in the engine (relief −4096..+4064).
            let y00 = c00.height as i8 as f32 * height_scale;
            let y10 = c10.height as i8 as f32 * height_scale;
            let y01 = c01.height as i8 as f32 * height_scale;
            let y11 = c11.height as i8 as f32 * height_scale;
            let position_for = |cell_x: i32, cell_z: i32, y: f32, terrain_type: u8| -> [f32; 3] {
                let [offset_x, offset_z] = if terrain_type & 0x10 != 0 {
                    infection_motion.vertex_offset_raw(cell_x, cell_z)
                } else {
                    [0, 0]
                };
                [
                    cell_x as f32 + f32::from(offset_x) / 256.0,
                    y,
                    cell_z as f32 + f32::from(offset_z) / 256.0,
                ]
            };
            // `FUN_00430140` moves the ordinary terrain vertex before
            // both the base and infection passes are built. Sharing these
            // exact positions keeps adjoining cells watertight and makes
            // the blood-like overlay move with its underlying surface.
            let p00 = position_for(world_x, world_z, y00, c00.terrain_type);
            let p10 = position_for(world_x, world_z + 1, y10, c10.terrain_type);
            let p01 = position_for(world_x + 1, world_z, y01, c01.terrain_type);
            let p11 = position_for(world_x + 1, world_z + 1, y11, c11.terrain_type);

            let color_for = |cell: &v2k_formats::terrain::TerrainCell| -> [f32; 3] {
                // Diagnostic only: authentic levels use transition
                // sprites. Keep this fallback tied to the proven fields
                // and avoid the removed height/LOD material invention.
                let material = (cell.terrain_type & 7) as usize;
                let base = colors
                    .get(material)
                    .map(|c| [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0])
                    .unwrap_or([0.35, 0.5, 0.25]);
                let shade = ((cell.terrain_type >> 5) as f32 + 1.0) / 8.0;
                [base[0] * shade, base[1] * shade, base[2] * shade]
            };

            // The canonicalizer's base-5 digit order follows the quad
            // perimeter: (x,z), (x+1,z), (x+1,z+1), (x,z+1).
            let tile = frames.and_then(|frames| {
                frames.tile([
                    c00.terrain_type & 7,
                    c01.terrain_type & 7,
                    c11.terrain_type & 7,
                    c10.terrain_type & 7,
                ])
            });
            let (col00, col10, col01, col11) =
                if let (Some(frames), Some((tile, _))) = (frames, tile) {
                    let start = terrain.darkness_start_world_y();
                    let range = terrain.darkness_range();
                    let tint = |cell: &v2k_formats::terrain::TerrainCell, x, z| {
                        let shade = crate::terrain_tiles::vertex_shade(
                            cell.terrain_type,
                            cell.height,
                            lights.map(|lights| lights.sample(x, z)).unwrap_or(0),
                            start,
                            range,
                        );
                        if self.terrain_program.is_some() {
                            [
                                frames.palette_shade(shade) as f32 / 31.0,
                                tile.frame as f32 / 119.0,
                                0.0,
                            ]
                        } else {
                            [frames.shade_scale(tile.frame, shade); 3]
                        }
                    };
                    (
                        tint(c00, world_x, world_z),
                        tint(c10, world_x, world_z + 1),
                        tint(c01, world_x + 1, world_z),
                        tint(c11, world_x + 1, world_z + 1),
                    )
                } else {
                    (
                        color_for(c00),
                        color_for(c10),
                        color_for(c01),
                        color_for(c11),
                    )
                };
            let uv_for = |corner: usize| -> [f32; 2] {
                let Some((tile, rect)) = tile else {
                    return [0.0, 0.0];
                };
                let slot = tile.slots[corner] as usize;
                let (su, sv) = crate::water::SLOT_UV[slot];
                [
                    rect[0] + (rect[2] - rect[0]) * su,
                    rect[1] + (rect[3] - rect[1]) * sv,
                ]
            };
            let uv00 = uv_for(0);
            let uv01 = uv_for(1);
            let uv11 = uv_for(2);
            let uv10 = uv_for(3);

            // Triangle 1: (0,0), (1,0), (0,1)
            verts.extend_from_slice(&[
                p00[0], p00[1], p00[2], col00[0], col00[1], col00[2], uv00[0], uv00[1],
            ]);
            verts.extend_from_slice(&[
                p10[0], p10[1], p10[2], col10[0], col10[1], col10[2], uv10[0], uv10[1],
            ]);
            verts.extend_from_slice(&[
                p01[0], p01[1], p01[2], col01[0], col01[1], col01[2], uv01[0], uv01[1],
            ]);

            // Triangle 2: (1,0), (1,1), (0,1)
            verts.extend_from_slice(&[
                p10[0], p10[1], p10[2], col10[0], col10[1], col10[2], uv10[0], uv10[1],
            ]);
            verts.extend_from_slice(&[
                p11[0], p11[1], p11[2], col11[0], col11[1], col11[2], uv11[0], uv11[1],
            ]);
            verts.extend_from_slice(&[
                p01[0], p01[1], p01[2], col01[0], col01[1], col01[2], uv01[0], uv01[1],
            ]);

            // Terrain byte-2 bit 0x10 is the persistent infection mark.
            // FUN_00430430 orders its mask corners as (x,z), (x,z+1),
            // (x+1,z+1), (x+1,z), then uses DAT_004CACC8/CC — the exact
            // same marching-square shape/rotation table as shoreline.
            let infection_mask = infection_corner_mask([
                c00.terrain_type,
                c10.terrain_type,
                c11.terrain_type,
                c01.terrain_type,
            ]);
            if infection_mask != 0 {
                if let Some(frames) = frames {
                    // Marching-square order c0..c3 differs from the base
                    // terrain's canonical D4 order; map it explicitly.
                    let positions = [p00, p10, p11, p01];
                    if let Some((_, vertices)) = infection_quad_vertices(
                        infection_mask,
                        &frames.infection_frame_uvs,
                        positions,
                    ) {
                        infection_verts.extend(vertices.into_iter().flatten());
                    }
                }
            }
        }

        self.terrain_vertex_count = verts.len() / 8;
        self.terrain_vertices = verts;
        self.terrain_infection_vertex_count = infection_verts.len() / 5;
        self.terrain_infection_vertices = infection_verts;
        self.terrain_cache_footprint = Some(footprint);
        self.terrain_light_revision = lights.map(|lights| lights.revision()).unwrap_or(0);
        self.terrain_type_signature = terrain_type_signature(terrain);
    }

    /// Submit the already-built 9-float water vertex stream. Geometry
    /// generation and OpenGL state management are intentionally separate: the
    /// former reconstructs retail terrain rules, while this function owns the
    /// complete indexed/RGBA draw-state transaction.
    fn submit_water_vertices(&self, frames: Option<&crate::water::WaterFrames>) {
        let total_count = self.water_vertices.len() / 9;
        if total_count == 0 {
            return;
        }

        let verts = &self.water_vertices;
        let indexed_program_used = frames.is_some() && self.water_program.is_some();
        let compatibility_program = (self.projection_effect != ProjectionEffect::None
            && !indexed_program_used)
            .then_some(self.compatibility_program.as_ref())
            .flatten();
        unsafe {
            let fog_was_enabled = crate::gl::IsEnabled(crate::gl::FOG) != 0;
            crate::gl::Disable(crate::gl::FOG);
            crate::gl::Enable(crate::gl::BLEND);
            crate::gl::Disable(crate::gl::CULL_FACE);

            crate::gl::EnableClientState(crate::gl::VERTEX_ARRAY);
            crate::gl::EnableClientState(crate::gl::COLOR_ARRAY);
            let stride = 9 * std::mem::size_of::<f32>() as i32;
            let ptr = verts.as_ptr();
            crate::gl::VertexPointer(3, crate::gl::FLOAT, stride, ptr as *const _);
            crate::gl::ColorPointer(4, crate::gl::FLOAT, stride, ptr.add(3) as *const _);
            if let Some(program) = compatibility_program {
                activate_compatibility_program(
                    program,
                    frames.is_some(),
                    false,
                    self.fog_near,
                    self.view_distance,
                    self.world_fog_color,
                );
            }

            if let Some(frames) = frames {
                crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE_MINUS_SRC_ALPHA);
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                if let Some(program) = &self.water_program {
                    crate::gl::UseProgram(program.id);
                    crate::gl::Uniform1i(program.index_sampler, 0);
                    crate::gl::Uniform1i(program.palette_sampler, 1);
                    crate::gl::Uniform1f(
                        program.fog_enabled,
                        if self.world_fog_enabled { 1.0 } else { 0.0 },
                    );
                    crate::gl::Uniform1f(program.fog_near, self.fog_near);
                    crate::gl::Uniform1f(program.fog_far, self.view_distance);
                    crate::gl::Uniform3f(
                        program.fog_color,
                        self.world_fog_color[0],
                        self.world_fog_color[1],
                        self.world_fog_color[2],
                    );
                    crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.index_texture.0);
                    crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                    crate::gl::Enable(crate::gl::TEXTURE_2D);
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.palette_texture.0);
                    crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                } else {
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.texture.0);
                }
                crate::gl::EnableClientState(crate::gl::TEXTURE_COORD_ARRAY);
                crate::gl::TexCoordPointer(2, crate::gl::FLOAT, stride, ptr.add(7) as *const _);
                crate::gl::DrawArrays(crate::gl::TRIANGLES, 0, total_count as i32);
                crate::gl::DisableClientState(crate::gl::TEXTURE_COORD_ARRAY);
                if indexed_program_used {
                    crate::gl::UseProgram(0);
                    crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, 0);
                    crate::gl::Disable(crate::gl::TEXTURE_2D);
                    crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                } else if compatibility_program.is_some() {
                    crate::gl::UseProgram(0);
                }
                crate::gl::Disable(crate::gl::TEXTURE_2D);
            } else {
                crate::gl::BlendFunc(crate::gl::SRC_ALPHA, crate::gl::ONE_MINUS_SRC_ALPHA);
                crate::gl::DrawArrays(crate::gl::TRIANGLES, 0, total_count as i32);
                if compatibility_program.is_some() {
                    crate::gl::UseProgram(0);
                }
            }

            crate::gl::DisableClientState(crate::gl::COLOR_ARRAY);
            crate::gl::DisableClientState(crate::gl::VERTEX_ARRAY);
            crate::gl::Enable(crate::gl::CULL_FACE);
            crate::gl::Disable(crate::gl::BLEND);
            if fog_was_enabled {
                crate::gl::Enable(crate::gl::FOG);
            }
        }
    }
}

impl Drop for GlRenderer {
    fn drop(&mut self) {
        // A live renderer switch creates the replacement first, which may
        // leave another context current. Delete into this one.
        let _ = self.window.gl_make_current(&self._gl_context);
        if let Some(program) = self.model_program.take() {
            unsafe {
                crate::gl::DeleteProgram(program.id);
            }
        }
        if let Some(program) = self.terrain_program.take() {
            unsafe {
                crate::gl::DeleteProgram(program.id);
            }
        }
        if let Some(program) = self.water_program.take() {
            unsafe {
                crate::gl::DeleteProgram(program.id);
            }
        }
        if let Some(program) = self.compatibility_program.take() {
            unsafe {
                crate::gl::DeleteProgram(program.id);
            }
        }
    }
}

fn triangle_is_all_view_pin(triangle: &[u16; 3], vertex_type_flags: &[i16]) -> bool {
    triangle
        .iter()
        .all(|&index| vertex_type_flags.get(index as usize) == Some(&13))
}

fn triangle_has_view_pin(triangle: &[u16; 3], vertex_type_flags: &[i16]) -> bool {
    triangle
        .iter()
        .any(|&index| vertex_type_flags.get(index as usize) == Some(&13))
}

fn view_pin_triangle_visible(
    mode: ViewPinMode,
    triangle: &[u16; 3],
    vertex_type_flags: &[i16],
) -> bool {
    match mode {
        ViewPinMode::Raw => true,
        ViewPinMode::Disabled => !triangle_has_view_pin(triangle, vertex_type_flags),
        ViewPinMode::CameraFacing => !triangle_is_all_view_pin(triangle, vertex_type_flags),
        ViewPinMode::WorldSurface => true,
    }
}

/// GL compatibility policy for the legacy/default CameraFacing fallback.
///
/// This is not a categorical retail world-face rule: WorldSurface retains all
/// authored type-13 faces and projects them through the recovered callback.
/// Raw callers retain ordinary authored depth behavior. Only a visible support
/// is a colour underlay for later billboard attachments in the same model.
fn view_pin_triangle_is_color_underlay(
    mode: ViewPinMode,
    triangle: &[u16; 3],
    vertex_type_flags: &[i16],
) -> bool {
    match mode {
        ViewPinMode::CameraFacing => {
            triangle_has_view_pin(triangle, vertex_type_flags)
                && !triangle_is_all_view_pin(triangle, vertex_type_flags)
        }
        ViewPinMode::Disabled | ViewPinMode::Raw | ViewPinMode::WorldSurface => false,
    }
}

/// Preserve the existing depth adapter for the endpoints that actually came
/// from world tf13. Linked tf11 copies that provenance; arithmetic generators
/// clear it. Authored flags remain the authority for unresolved diagnostics.
#[derive(Clone, Copy)]
enum SurfaceVertexSource<'a> {
    Intrinsic(&'a [i16]),
    Resolved(&'a [ModelSurfaceOrigin]),
}

impl SurfaceVertexSource<'_> {
    fn is_projected(self, index: u16) -> bool {
        match self {
            Self::Intrinsic(flags) => flags.get(usize::from(index)) == Some(&13),
            Self::Resolved(origins) => {
                origins.get(usize::from(index)) == Some(&ModelSurfaceOrigin::ViewPin)
            }
        }
    }
}

fn view_pin_triangle_needs_coplanar_depth(
    mode: ViewPinMode,
    triangle: &[u16; 3],
    vertices: SurfaceVertexSource<'_>,
) -> bool {
    mode == ViewPinMode::WorldSurface && triangle.iter().any(|&index| vertices.is_projected(index))
}

fn view_pin_triangle_is_world_surface_decal(
    mode: ViewPinMode,
    triangle: &[u16; 3],
    vertices: SurfaceVertexSource<'_>,
) -> bool {
    mode == ViewPinMode::WorldSurface && triangle.iter().all(|&index| vertices.is_projected(index))
}

fn triangle_uses_surface_overlay_depth(
    overlay: ModelOverlayKind,
    mode: ViewPinMode,
    triangle: &[u16; 3],
    vertices: SurfaceVertexSource<'_>,
) -> bool {
    overlay == ModelOverlayKind::TerrainSurface
        || view_pin_triangle_is_world_surface_decal(mode, triangle, vertices)
}

/// `FUN_0046D3F0` authored-plane rejection in world coordinates.
///
/// Applying the model orientation to both the raw normal and anchor is
/// algebraically identical to retail's camera-relative model-space test. The
/// strict sign matters: a camera exactly on the plane rejects the face.
pub fn authored_face_plane_visible(
    plane: ModelFaceCullPlane,
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    model_scale: f32,
    camera: [f32; 3],
) -> bool {
    let orientation = orientation.map(|row| row.map(f64::from));
    let normal = plane.normal_raw.map(f64::from);
    let world_normal = [
        dot3_f64(orientation[0], normal),
        dot3_f64(orientation[1], normal),
        dot3_f64(orientation[2], normal),
    ];
    let scale = f64::from(model_scale) / 100.0;
    let anchor = plane.anchor_raw.map(|component| component * scale);
    let world_anchor = [
        f64::from(position[0]) + dot3_f64(orientation[0], anchor),
        f64::from(position[1]) + dot3_f64(orientation[1], anchor),
        f64::from(position[2]) + dot3_f64(orientation[2], anchor),
    ];
    let camera_relative_anchor = [
        world_anchor[0] - f64::from(camera[0]),
        world_anchor[1] - f64::from(camera[1]),
        world_anchor[2] - f64::from(camera[2]),
    ];
    dot3_f64(world_normal, camera_relative_anchor) < 0.0
}

fn dot3_f64(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

const RETAIL_VIEW_PIN_Y: f32 = -32767.0 / 256.0;

/// `FUN_0046EBD0` type-13 transform expressed around the backend's local-model
/// matrix. Retail preserves the source point's view X/Z and replaces view Y
/// with signed -32767 in the 8.8 world domain. The returned point is converted
/// back to raw local coordinates because `draw_tris_gl` subsequently reapplies
/// the model scale, orientation, and translation through OpenGL.
fn retail_view_pin_vertex(
    raw: [f32; 3],
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    model_scale: f32,
    camera: [f32; 3],
    view_basis: [[f32; 3]; 3],
) -> [f32; 3] {
    let scale = model_scale / 100.0;
    let local = [raw[0] * scale, raw[1] * scale, raw[2] * scale];
    let world = [
        position[0] + dot3(orientation[0], local),
        position[1] + dot3(orientation[1], local),
        position[2] + dot3(orientation[2], local),
    ];
    let camera_relative = [
        world[0] - camera[0],
        world[1] - camera[1],
        world[2] - camera[2],
    ];
    let view_x = dot3(camera_relative, view_basis[0]);
    let view_z = dot3(camera_relative, view_basis[2]);
    let pinned_world = [
        camera[0]
            + view_basis[0][0] * view_x
            + view_basis[1][0] * RETAIL_VIEW_PIN_Y
            + view_basis[2][0] * view_z,
        camera[1]
            + view_basis[0][1] * view_x
            + view_basis[1][1] * RETAIL_VIEW_PIN_Y
            + view_basis[2][1] * view_z,
        camera[2]
            + view_basis[0][2] * view_x
            + view_basis[1][2] * RETAIL_VIEW_PIN_Y
            + view_basis[2][2] * view_z,
    ];
    world_point_as_raw_local(pinned_world, orientation, position, model_scale)
}

/// Exact signed-word bilinear sampler used by `FUN_00445860`.
fn retail_terrain_height_raw(terrain: &TerrainGrid, x_raw: i16, z_raw: i16) -> i16 {
    let x_word = x_raw as u16;
    let z_word = z_raw as u16;
    let x0 = usize::from(x_word >> 8);
    let z0 = usize::from(z_word >> 8);
    let x1 = (x0 + 1) & 0xff;
    let z1 = (z0 + 1) & 0xff;
    let x_fraction = i32::from(x_word & 0xff);
    let z_fraction = i32::from(z_word & 0xff);
    let height = |x: usize, z: usize| {
        i32::from(terrain.cell(x, z).expect("complete 256x256 terrain").height as i8) << 5
    };

    let h00 = height(x0, z0);
    let h10 = height(x1, z0);
    let h01 = height(x0, z1);
    let h11 = height(x1, z1);
    let along_x0 = (((h10 - h00) * x_fraction) >> 8) + h00;
    let along_x1 = (((h11 - h01) * x_fraction) >> 8) + h01;
    ((((along_x1 - along_x0) * z_fraction) >> 8) + along_x0) as i16
}

fn retail_q31_mul(value: i32, multiplier: i32) -> i32 {
    ((i64::from(value) * i64::from(multiplier)) >> 31) as i32
}

/// World callback installed for type-13 records by `FUN_00433FA0`.
///
/// `FUN_004349C0`/`FUN_00435090`/`FUN_00435780` perform one surface sample,
/// one Section-10-direction X/Z correction, and one endpoint resample. The
/// sea +/-0x96 interval is an inclusive clip band; it is not a distance cap.
pub fn retail_world_surface_vertex(
    raw: [f32; 3],
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    model_scale: f32,
    projection: WorldSurfaceProjection<'_>,
) -> ResolvedModelSlot {
    let scale = model_scale / 100.0;
    let local = [raw[0] * scale, raw[1] * scale, raw[2] * scale];
    let world = [
        position[0] + dot3(orientation[0], local),
        position[1] + dot3(orientation[1], local),
        position[2] + dot3(orientation[2], local),
    ];
    let mut result = retail_world_surface_point(world, projection);
    result.position_raw = if result.clip == ModelSlotClip::SurfaceBand {
        raw.map(f64::from)
    } else {
        world_point_as_raw_local(
            result
                .world_point
                .unwrap()
                .map(|component| component as f32),
            orientation,
            position,
            model_scale,
        )
        .map(f64::from)
    };
    result
}

/// Project an owned external-frame endpoint directly, before any body-space
/// representation. 4349C0 first invokes its source callback (including 6ECF0).
pub fn retail_world_surface_point(
    world: [f32; 3],
    projection: WorldSurfaceProjection<'_>,
) -> ResolvedModelSlot {
    let mut world_raw = world.map(|component| (component * 256.0).round() as i32);
    let sample_x = world_raw[0] as i16;
    let sample_z = world_raw[2] as i16;
    let terrain_y = retail_terrain_height_raw(projection.terrain, sample_x, sample_z);
    let sea_y = projection.terrain.sea_level_raw();

    let use_wave_surface = world_raw[1] > i32::from(sea_y) + 0x96 || terrain_y > sea_y;
    let initial_surface = if use_wave_surface {
        projection.wave_surface_raw(sample_x, sample_z, terrain_y)
    } else if world_raw[1] >= i32::from(sea_y) - 0x96 {
        // 4349C0 sets clip40 without discarding the source coordinates. A
        // linked tf11 copies both; a later spatial generator may use XYZ and
        // compute a fresh clip result of its own.
        return ResolvedModelSlot {
            position_raw: world_raw.map(f64::from),
            world_point: Some(world_raw.map(|component| f64::from(component) / 256.0)),
            native_view_point: None,
            clip: ModelSlotClip::SurfaceBand,
            surface_origin: ModelSurfaceOrigin::ViewPin,
        };
    } else {
        terrain_y
    };

    let vertical_delta = world_raw[1] - i32::from(initial_surface);
    world_raw[0] =
        world_raw[0].wrapping_add(retail_q31_mul(vertical_delta, projection.slope_x_q31));
    world_raw[2] =
        world_raw[2].wrapping_add(retail_q31_mul(vertical_delta, projection.slope_z_q31));

    let endpoint_x = world_raw[0] as i16;
    let endpoint_z = world_raw[2] as i16;
    let endpoint_terrain = retail_terrain_height_raw(projection.terrain, endpoint_x, endpoint_z);
    world_raw[1] = i32::from(if use_wave_surface {
        projection.wave_surface_raw(endpoint_x, endpoint_z, endpoint_terrain)
    } else {
        endpoint_terrain
    });

    let endpoint_world = world_raw.map(|component| component as f32 / 256.0);
    ResolvedModelSlot {
        position_raw: world_raw.map(f64::from),
        world_point: Some(endpoint_world.map(f64::from)),
        native_view_point: None,
        clip: ModelSlotClip::Clear,
        surface_origin: ModelSurfaceOrigin::ViewPin,
    }
}

/// `FUN_004340B0`/`FUN_004343A0`/`FUN_004346B0` — the tf-12 entries that world
/// setup (`FUN_00433FA0`) installs next to the type-13 family. The swapped
/// handler copies slot a's position, rotates it into object-relative world
/// offsets, replaces Y with the exact `FUN_00445860` bilinear terrain sample at
/// that world X/Z relative to the object origin, and rotates back. Unlike the
/// type-13 family there is no slope correction, wave surface, or sea-band clip:
/// the sample is always plain terrain. This is what grounds tree-trunk bases
/// and shadow quads in world draws; default/menu contexts keep the pure alias.
pub fn retail_world_surface_alias_vertex(
    raw: [f32; 3],
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    model_scale: f32,
    terrain: &TerrainGrid,
) -> [f32; 3] {
    let scale = model_scale / 100.0;
    let local = [raw[0] * scale, raw[1] * scale, raw[2] * scale];
    let world = [
        position[0] + dot3(orientation[0], local),
        position[1] + dot3(orientation[1], local),
        position[2] + dot3(orientation[2], local),
    ];
    let sample_x = ((world[0] * 256.0).round() as i32) as i16;
    let sample_z = ((world[2] * 256.0).round() as i32) as i16;
    let terrain_y = retail_terrain_height_raw(terrain, sample_x, sample_z);
    let grounded_world = [world[0], i32::from(terrain_y) as f32 / 256.0, world[2]];
    world_point_as_raw_local(grounded_world, orientation, position, model_scale)
}

fn model_local_as_world(
    raw: [f32; 3],
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    scale: f32,
) -> [f32; 3] {
    [
        position[0]
            + (orientation[0][0] * raw[0]
                + orientation[0][1] * raw[1]
                + orientation[0][2] * raw[2])
                * scale,
        position[1]
            + (orientation[1][0] * raw[0]
                + orientation[1][1] * raw[1]
                + orientation[1][2] * raw[2])
                * scale,
        position[2]
            + (orientation[2][0] * raw[0]
                + orientation[2][1] * raw[1]
                + orientation[2][2] * raw[2])
                * scale,
    ]
}

fn type14_selector_world(
    authored: [f64; 3],
    external_frame_mode: ExternalFrameMode,
) -> Option<[f32; 3]> {
    match external_frame_mode {
        ExternalFrameMode::Raw => None,
        ExternalFrameMode::WorldPoint(world) => Some(world),
        ExternalFrameMode::SelectorWorldPoints(points) => {
            let selector = authored[0] as i32;
            (0..16)
                .contains(&selector)
                .then(|| points[selector as usize])
                .flatten()
        }
    }
}

/// World endpoint for one model-edge vertex. Type-14 uses the callback world
/// point when the live Sub-H/static frame supplied one; everything else keeps
/// the raw-local → world map the mesh already uses.
#[allow(clippy::too_many_arguments)]
pub fn edge_endpoint_world(
    index: usize,
    vertices: &[[f64; 3]],
    vertex_type_flags: &[i16],
    resolved_vertices: &[Option<[f32; 3]>],
    external_frame_mode: ExternalFrameMode,
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    scale: f32,
) -> Option<[f32; 3]> {
    if vertex_type_flags.get(index).copied() == Some(14) {
        if let Some(authored) = vertices.get(index) {
            if let Some(world) = type14_selector_world(*authored, external_frame_mode) {
                return Some(world);
            }
        }
    }
    resolved_vertices
        .get(index)
        .copied()
        .flatten()
        .map(|raw| model_local_as_world(raw, orientation, position, scale))
}

fn apply_external_frame_operands(
    raw: [f32; 3],
    external_frame_mode: ExternalFrameMode,
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    extra_scale: f32,
) -> [f32; 3] {
    match external_frame_mode {
        ExternalFrameMode::Raw => raw,
        ExternalFrameMode::WorldPoint(world) => {
            world_point_as_raw_local(world, orientation, position, extra_scale)
        }
        ExternalFrameMode::SelectorWorldPoints(points) => {
            let selector = raw[0] as i32;
            if (0..16).contains(&selector) {
                if let Some(world) = points[selector as usize] {
                    world_point_as_raw_local(world, orientation, position, extra_scale)
                } else {
                    raw
                }
            } else {
                raw
            }
        }
    }
}

/// Re-express a callback-produced world point in the backend's raw local
/// coordinate contract. `FUN_0046ECF0`/`FUN_0046EE70`/`FUN_0046F010` bypass
/// the ordinary model transform after invoking the type-14 callback; this
/// inverse makes the later OpenGL model matrix produce the same world point.
pub fn world_point_as_raw_local(
    world: [f32; 3],
    orientation: [[f32; 3]; 3],
    position: [f32; 3],
    model_scale: f32,
) -> [f32; 3] {
    let scale = model_scale / 100.0;
    let delta = [
        world[0] - position[0],
        world[1] - position[1],
        world[2] - position[2],
    ];
    [
        dot3(
            delta,
            [orientation[0][0], orientation[1][0], orientation[2][0]],
        ) / scale,
        dot3(
            delta,
            [orientation[0][1], orientation[1][1], orientation[2][1]],
        ) / scale,
        dot3(
            delta,
            [orientation[0][2], orientation[1][2], orientation[2][2]],
        ) / scale,
    ]
}

fn billboard_writes_depth(blend: WorldSpriteBlend) -> bool {
    blend == WorldSpriteBlend::Masked
}

fn model_blend_uniform(blend: WorldSpriteBlend) -> f32 {
    match blend {
        WorldSpriteBlend::Masked => 0.0,
        WorldSpriteBlend::Additive => 1.0,
        WorldSpriteBlend::HalfAdditive => 2.0,
    }
}

fn model_face_opacity(blend: WorldSpriteBlend) -> f32 {
    if blend == WorldSpriteBlend::HalfAdditive {
        HALF_ADDITIVE_ALPHA
    } else {
        1.0
    }
}

fn model_face_palette_row(
    shading: ModelFaceShading,
    interpolated: Option<u8>,
    flat_shade_row: u8,
) -> u8 {
    match shading {
        ModelFaceShading::FlatLit | ModelFaceShading::Gouraud => interpolated.unwrap_or(31).min(31),
        ModelFaceShading::Flat => flat_shade_row.min(31),
    }
}

const MODEL_FIXED_SHADE_ROW: u8 = 28;
const MODEL_FLAT_SHADE_ROWS: [u8; 2] = [0, MODEL_FIXED_SHADE_ROW];
const MODEL_PALETTE_ROWS: usize = 32;
const MODEL_PALETTE_COLORS: usize = 16;
const RGBA_CHANNELS: usize = 4;

/// Decode one indexed model shade row for the fixed-function fallback.
///
/// The public fallback supplied by the game is deliberately row 31 because
/// The lit fixed-function paths modulate that row with Section-6 RGB.
/// OpenGL's model shader performs the row lookup at draw time, but a driver
/// without that shader needs RGBA copies for default/flag-0x04 flat faces.
/// Keep the source entry's bit-0 zero-key policy while deriving each copy
/// from the same indices and
/// complete palette used by the shader.
fn indexed_model_rgba_at_row(
    indices: &[u8],
    palette_rgba: &[u8],
    pixel_count: usize,
    row: u8,
    transparent_zero: bool,
) -> Option<Vec<u8>> {
    let indices = indices.get(..pixel_count)?;
    let row = usize::from(row);
    if row >= MODEL_PALETTE_ROWS {
        return None;
    }
    let row_bytes = MODEL_PALETTE_COLORS * RGBA_CHANNELS;
    let row_start = row.checked_mul(row_bytes)?;
    let palette_row = palette_rgba.get(row_start..row_start.checked_add(row_bytes)?)?;

    let mut rgba = Vec::with_capacity(pixel_count * RGBA_CHANNELS);
    for &index in indices {
        if transparent_zero && index == 0 {
            rgba.extend_from_slice(&[0; RGBA_CHANNELS]);
            continue;
        }
        let color_start = usize::from(index).checked_mul(RGBA_CHANNELS)?;
        rgba.extend_from_slice(
            palette_row.get(color_start..color_start.checked_add(RGBA_CHANNELS)?)?,
        );
    }
    Some(rgba)
}

fn fixed_function_model_texture(
    fallback: TextureId,
    indexed: Option<&IndexedModelTextures>,
    shading: ModelFaceShading,
    flat_shade_row: u8,
) -> TextureId {
    if shading == ModelFaceShading::Flat {
        if let Some(slot) = MODEL_FLAT_SHADE_ROWS
            .iter()
            .position(|&row| row == flat_shade_row)
        {
            return indexed
                .and_then(|textures| textures.flat_rows[slot])
                .unwrap_or(fallback);
        }
    }
    fallback
}

fn model_face_writes_depth(blend: WorldSpriteBlend, view_pin_underlay: bool) -> bool {
    !view_pin_underlay && blend == WorldSpriteBlend::Masked
}

fn world_sprite_draw_order(sprites: &[WorldSprite]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..sprites.len()).collect();
    // Retail merges one particle queue by projected depth plus the signed
    // descriptor bias. `sort_by` is stable, so equal keys retain the original
    // priority/list traversal order.
    order.sort_by(|&a, &b| sprites[b].sort_key_raw.cmp(&sprites[a].sort_key_raw));
    order
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl Renderer for GlRenderer {
    fn backend_name(&self) -> &str {
        "OpenGL 2.1"
    }

    fn set_overlay_depth_policy(&mut self, policy: OverlayDepthPolicy) {
        self.overlay_depth_policy = policy;
    }

    fn begin_scene(&mut self, scene: RenderScene) {
        self.active_scene = scene;
        self.scene_projection_authority = crate::projection::SceneProjectionAuthority::default();
        // Retail rebuilds the renderer context at each high-level scene
        // boundary. Prevent an underwater world callback from leaking into a
        // later menu or dry-world submission.
        self.bind_render_target();
        self.configure_projection_effect(ProjectionEffect::None);
        unsafe {
            // Canonical fixed-function baseline. Individual draw calls may
            // override these settings, but no previous frame/mode survives.
            crate::gl::UseProgram(0);
            crate::gl::ActiveTexture(crate::gl::TEXTURE1);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, 0);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::ActiveTexture(crate::gl::TEXTURE0);
            crate::gl::Disable(crate::gl::BLEND);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            crate::gl::Disable(crate::gl::SCISSOR_TEST);
            crate::gl::Disable(crate::gl::LIGHTING);
            crate::gl::DisableClientState(crate::gl::VERTEX_ARRAY);
            crate::gl::DisableClientState(crate::gl::COLOR_ARRAY);
            crate::gl::DisableClientState(crate::gl::NORMAL_ARRAY);
            crate::gl::DisableClientState(crate::gl::TEXTURE_COORD_ARRAY);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, 0);
            crate::gl::BlendFunc(crate::gl::SRC_ALPHA, crate::gl::ONE_MINUS_SRC_ALPHA);
            crate::gl::Enable(crate::gl::DEPTH_TEST);
            crate::gl::DepthMask(crate::gl::TRUE);
            crate::gl::DepthFunc(crate::gl::LEQUAL);
            crate::gl::Enable(crate::gl::CULL_FACE);
            crate::gl::CullFace(crate::gl::BACK);
            crate::gl::FrontFace(crate::gl::CCW);
            crate::gl::Color4f(1.0, 1.0, 1.0, 1.0);

            if scene_uses_world_fog(scene, self.world_fog_enabled) {
                crate::gl::Enable(crate::gl::FOG);
                crate::gl::Fogi(crate::gl::FOG_MODE, crate::gl::LINEAR as i32);
                crate::gl::Fogf(crate::gl::FOG_START, self.fog_near);
                crate::gl::Fogf(crate::gl::FOG_END, self.view_distance);
                let color = [
                    self.world_fog_color[0],
                    self.world_fog_color[1],
                    self.world_fog_color[2],
                    1.0,
                ];
                crate::gl::Fogfv(crate::gl::FOG_COLOR, color.as_ptr());
            } else {
                crate::gl::Disable(crate::gl::FOG);
            }
        }
    }

    fn clear(&mut self, r: f32, g: f32, b: f32) {
        self.bind_render_target();
        unsafe {
            crate::gl::Disable(crate::gl::SCISSOR_TEST);
            let inset = self.viewport.physical_width != self.output_width
                || self.viewport.physical_height != self.output_height;
            if inset {
                crate::gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                crate::gl::Clear(crate::gl::COLOR_BUFFER_BIT | crate::gl::DEPTH_BUFFER_BIT);
                crate::gl::Enable(crate::gl::SCISSOR_TEST);
                crate::gl::Scissor(
                    self.viewport.x,
                    self.viewport.y,
                    self.viewport.physical_width as i32,
                    self.viewport.physical_height as i32,
                );
            }
            crate::gl::ClearColor(r, g, b, 1.0);
            crate::gl::Clear(crate::gl::COLOR_BUFFER_BIT | crate::gl::DEPTH_BUFFER_BIT);
            crate::gl::Disable(crate::gl::SCISSOR_TEST);
        }
    }

    fn present(&mut self) {
        self.window.gl_swap_window();
        self.bind_render_target();
    }

    fn capture_frame(&mut self, source: FrameCaptureSource) -> Option<CapturedFrame> {
        let width = self.viewport.physical_width;
        let height = self.viewport.physical_height;
        let byte_len = usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?
            .checked_mul(4)?;
        let mut rgba = vec![0; byte_len];

        unsafe {
            // Presented reads the completed swap; CurrentScene reads the
            // unfinished BACK target without making an intermediate frame
            // visible. Both exclude any 4:3 letterbox bars.
            crate::gl::ReadBuffer(match source {
                FrameCaptureSource::Presented => crate::gl::FRONT,
                FrameCaptureSource::CurrentScene => crate::gl::BACK,
            });
            crate::gl::PixelStorei(crate::gl::PACK_ALIGNMENT, 1);
            crate::gl::ReadPixels(
                self.viewport.x,
                self.viewport.y,
                width as i32,
                height as i32,
                crate::gl::RGBA,
                crate::gl::UNSIGNED_BYTE,
                rgba.as_mut_ptr().cast(),
            );
            crate::gl::ReadBuffer(crate::gl::BACK);
        }

        // OpenGL readback starts at the lower-left; fullscreen images in the
        // renderer use top-to-bottom rows.
        flip_rgba_rows(&mut rgba, width, height);
        Some(CapturedFrame {
            rgba,
            width,
            height,
        })
    }

    fn clear_depth(&mut self) {
        self.bind_render_target();
        unsafe {
            crate::gl::DepthMask(crate::gl::TRUE);
            crate::gl::Clear(crate::gl::DEPTH_BUFFER_BIT);
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.output_width = width.max(1);
        self.output_height = height.max(1);
        self.update_render_viewport();
    }

    fn set_camera(&mut self, camera: &Camera) {
        self.cam_horizontal_half_fov_tan = (camera.fov * 0.5).tan() * camera.aspect;
        self.cam_pos = camera.position;
        let forward = camera.forward();
        let horizontal_len = (forward[0] * forward[0] + forward[2] * forward[2]).sqrt();
        self.cam_forward = if horizontal_len > 1.0e-5 {
            [forward[0] / horizontal_len, forward[2] / horizontal_len]
        } else {
            [0.0, -1.0]
        };
        self.terrain_row_lead_raw = v2k_core::render_scan::terrain_row_lead_raw(forward[1]);
        let proj = camera.projection_matrix();
        let view = camera.view_matrix();
        self.cam_projection_terms = [
            proj[0],
            proj[5],
            camera.projection_offset[0],
            camera.projection_offset[1],
        ];
        self.cam_near = camera.near;
        self.cam_far = camera.far;
        self.cam_view_basis = [
            [view[0], view[4], view[8]],
            [view[1], view[5], view[9]],
            [view[2], view[6], view[10]],
        ];

        unsafe {
            // Reflecting view X reverses projected winding. Keep terrain and
            // other culled world geometry front-facing under the retail
            // left-handed live-world camera; ordinary menu cameras stay on
            // the canonical OpenGL convention.
            crate::gl::FrontFace(if camera.left_handed {
                crate::gl::CW
            } else {
                crate::gl::CCW
            });
            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::LoadMatrixf(proj.as_ptr());
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::LoadMatrixf(view.as_ptr());
        }
    }

    fn set_projection_effect(&mut self, effect: ProjectionEffect) {
        self.configure_projection_effect(effect);
    }

    fn set_scene_projection_authority(
        &mut self,
        authority: crate::projection::SceneProjectionAuthority,
    ) {
        self.scene_projection_authority = authority;
    }

    fn scene_projection_authority(&self) -> crate::projection::SceneProjectionAuthority {
        self.scene_projection_authority
    }

    fn set_fog(&mut self, enabled: bool, near: f32, far: f32, color: [f32; 3]) {
        self.world_model_fog = None;
        self.world_fog_enabled = enabled;
        self.world_fog_color = color;
        unsafe {
            if enabled {
                self.view_distance = far;
                self.fog_near = near;
                crate::gl::Enable(crate::gl::FOG);
                crate::gl::Fogi(crate::gl::FOG_MODE, crate::gl::LINEAR as i32);
                crate::gl::Fogf(crate::gl::FOG_START, near);
                crate::gl::Fogf(crate::gl::FOG_END, far);
                let fog_color = [color[0], color[1], color[2], 1.0];
                crate::gl::Fogfv(crate::gl::FOG_COLOR, fog_color.as_ptr());
            } else {
                crate::gl::Disable(crate::gl::FOG);
            }
        }
    }

    fn set_world_model_fog(&mut self, fog: Option<crate::renderer::WorldModelFog>) {
        if let Some(fog) = fog {
            self.set_fog(
                true,
                fog.planes.near_raw as f32 / 256.0,
                fog.planes.far_raw as f32 / 256.0,
                fog.color,
            );
        } else {
            self.set_fog(false, 0.0, 0.0, [0.0; 3]);
        }
        self.world_model_fog = fog;
    }

    fn world_model_fog(&self) -> Option<crate::renderer::WorldModelFog> {
        scene_uses_world_fog(self.active_scene, self.world_fog_enabled)
            .then_some(self.world_model_fog)
            .flatten()
    }

    fn retained_world_model_fog(&self) -> Option<crate::renderer::WorldModelFog> {
        self.world_model_fog
    }

    fn world_fog_planes(&self) -> Option<[f32; 2]> {
        scene_uses_world_fog(self.active_scene, self.world_fog_enabled)
            .then_some([self.fog_near, self.view_distance])
    }

    fn draw_terrain(
        &mut self,
        terrain: &TerrainGrid,
        colors: &[PaletteEntry],
        frames: Option<&crate::terrain_tiles::TerrainFrames>,
        lights: Option<&crate::terrain_light::TerrainLightWindow>,
        elapsed_micros: u32,
    ) {
        self.terrain_lights = lights.cloned();
        self.terrain_infection_animation.advance(elapsed_micros);
        let infection_motion = self.terrain_infection_animation.frame();
        let dimensions = frames.map_or([52, 30], |frames| [frames.scan_columns, frames.scan_rows]);
        let footprint = self.terrain_scan().footprint(dimensions);
        let light_revision = lights.map(|lights| lights.revision()).unwrap_or(0);
        let type_signature = terrain_type_signature(terrain);
        if self.terrain_cache_footprint != Some(footprint)
            || self.terrain_light_revision != light_revision
            || self.terrain_type_signature != type_signature
            || self.terrain_infection_vertex_count != 0
        {
            self.build_terrain_cache(
                terrain,
                colors,
                frames,
                lights,
                &infection_motion,
                footprint,
            );
        }

        if self.terrain_vertex_count == 0 {
            return;
        }

        let indexed_program_used = frames.is_some() && self.terrain_program.is_some();
        let compatibility_program = (self.projection_effect != ProjectionEffect::None
            && !indexed_program_used)
            .then_some(self.compatibility_program.as_ref())
            .flatten();
        let infection_compatibility_program = (self.projection_effect != ProjectionEffect::None)
            .then_some(self.compatibility_program.as_ref())
            .flatten();

        unsafe {
            let fog_was_enabled = crate::gl::IsEnabled(crate::gl::FOG) != 0;
            crate::gl::EnableClientState(crate::gl::VERTEX_ARRAY);
            crate::gl::EnableClientState(crate::gl::COLOR_ARRAY);
            if frames.is_none() {
                if let Some(program) = compatibility_program {
                    crate::gl::Disable(crate::gl::FOG);
                    activate_compatibility_program(
                        program,
                        false,
                        self.world_fog_enabled,
                        self.fog_near,
                        self.view_distance,
                        self.world_fog_color,
                    );
                }
            }
            if let Some(frames) = frames {
                crate::gl::Enable(crate::gl::TEXTURE_2D);
                if let Some(program) = &self.terrain_program {
                    crate::gl::UseProgram(program.id);
                    crate::gl::Uniform1i(program.index_sampler, 0);
                    crate::gl::Uniform1i(program.palette_sampler, 1);
                    crate::gl::Uniform1f(
                        program.fog_enabled,
                        if self.world_fog_enabled { 1.0 } else { 0.0 },
                    );
                    crate::gl::Uniform1f(program.fog_near, self.fog_near);
                    crate::gl::Uniform1f(program.fog_far, self.view_distance);
                    crate::gl::Uniform3f(
                        program.fog_color,
                        self.world_fog_color[0],
                        self.world_fog_color[1],
                        self.world_fog_color[2],
                    );
                    crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.index_texture.0);
                    crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                    crate::gl::Enable(crate::gl::TEXTURE_2D);
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.palette_texture.0);
                    crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                } else {
                    if let Some(program) = compatibility_program {
                        crate::gl::Disable(crate::gl::FOG);
                        activate_compatibility_program(
                            program,
                            true,
                            self.world_fog_enabled,
                            self.fog_near,
                            self.view_distance,
                            self.world_fog_color,
                        );
                    }
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.texture.0);
                }
                crate::gl::EnableClientState(crate::gl::TEXTURE_COORD_ARRAY);
            }

            let stride = 8 * std::mem::size_of::<f32>() as i32;
            let ptr = self.terrain_vertices.as_ptr();

            crate::gl::VertexPointer(3, crate::gl::FLOAT, stride, ptr as *const _);
            crate::gl::ColorPointer(3, crate::gl::FLOAT, stride, ptr.add(3) as *const _);
            if frames.is_some() {
                crate::gl::TexCoordPointer(2, crate::gl::FLOAT, stride, ptr.add(6) as *const _);
            }

            // Candidate fix for the surface-overlay depth gap: recede only the
            // opaque terrain fill so later-submitted, terrain-conformant
            // overlays win depth ties the way retail's stable painter orders
            // them. Foreground geometry and water are unoffset.
            if let Some((factor, units)) = self.overlay_depth_policy.terrain_fill_offset() {
                crate::gl::Enable(crate::gl::POLYGON_OFFSET_FILL);
                crate::gl::PolygonOffset(factor, units);
            }
            crate::gl::DrawArrays(crate::gl::TRIANGLES, 0, self.terrain_vertex_count as i32);
            if self.overlay_depth_policy.terrain_fill_offset().is_some() {
                crate::gl::PolygonOffset(0.0, 0.0);
                crate::gl::Disable(crate::gl::POLYGON_OFFSET_FILL);
            }

            if indexed_program_used {
                crate::gl::UseProgram(0);
                crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                crate::gl::BindTexture(crate::gl::TEXTURE_2D, 0);
                crate::gl::Disable(crate::gl::TEXTURE_2D);
                crate::gl::ActiveTexture(crate::gl::TEXTURE0);
            } else if compatibility_program.is_some() {
                crate::gl::UseProgram(0);
                if fog_was_enabled {
                    crate::gl::Enable(crate::gl::FOG);
                } else {
                    crate::gl::Disable(crate::gl::FOG);
                }
            }

            if self.terrain_infection_vertex_count != 0 {
                if let Some(frames) = frames {
                    // Retail emits this fixed-row-28 keyed pass immediately after
                    // the ordinary terrain quad. LEQUAL is the depth-buffer
                    // equivalent of that same-geometry software ordering.
                    crate::gl::DisableClientState(crate::gl::COLOR_ARRAY);
                    crate::gl::Enable(crate::gl::TEXTURE_2D);
                    crate::gl::BindTexture(crate::gl::TEXTURE_2D, frames.infection_texture.0);
                    crate::gl::Enable(crate::gl::ALPHA_TEST);
                    crate::gl::AlphaFunc(crate::gl::GREATER, 0.5);
                    crate::gl::DepthFunc(crate::gl::LEQUAL);
                    // The software renderer emits the infection shape directly
                    // after its base quad. Separate GL programs are not
                    // guaranteed depth-invariant for coplanar submissions, so
                    // pull this decal infinitesimally toward the eye and leave
                    // the terrain depth buffer authoritative for later models.
                    crate::gl::Enable(crate::gl::POLYGON_OFFSET_FILL);
                    crate::gl::PolygonOffset(-1.0, -1.0);
                    crate::gl::DepthMask(crate::gl::FALSE);
                    crate::gl::Color4f(1.0, 1.0, 1.0, 1.0);

                    if let Some(program) = infection_compatibility_program {
                        crate::gl::Disable(crate::gl::FOG);
                        activate_compatibility_program(
                            program,
                            true,
                            self.world_fog_enabled,
                            self.fog_near,
                            self.view_distance,
                            self.world_fog_color,
                        );
                    }

                    let infection_stride = 5 * std::mem::size_of::<f32>() as i32;
                    let infection_ptr = self.terrain_infection_vertices.as_ptr();
                    crate::gl::VertexPointer(
                        3,
                        crate::gl::FLOAT,
                        infection_stride,
                        infection_ptr as *const _,
                    );
                    crate::gl::TexCoordPointer(
                        2,
                        crate::gl::FLOAT,
                        infection_stride,
                        infection_ptr.add(3) as *const _,
                    );
                    crate::gl::DrawArrays(
                        crate::gl::TRIANGLES,
                        0,
                        self.terrain_infection_vertex_count as i32,
                    );
                    crate::gl::DepthMask(crate::gl::TRUE);
                    crate::gl::PolygonOffset(0.0, 0.0);
                    crate::gl::Disable(crate::gl::POLYGON_OFFSET_FILL);

                    if infection_compatibility_program.is_some() {
                        crate::gl::UseProgram(0);
                        if fog_was_enabled {
                            crate::gl::Enable(crate::gl::FOG);
                        } else {
                            crate::gl::Disable(crate::gl::FOG);
                        }
                    }
                    // `begin_scene` establishes LEQUAL for the world pass and
                    // shoreline/water submission inherits it. Restore that
                    // renderer baseline after the coplanar infection overlay.
                    crate::gl::DepthFunc(crate::gl::LEQUAL);
                    crate::gl::Disable(crate::gl::ALPHA_TEST);
                }
            }

            crate::gl::DisableClientState(crate::gl::TEXTURE_COORD_ARRAY);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::DisableClientState(crate::gl::COLOR_ARRAY);
            crate::gl::DisableClientState(crate::gl::VERTEX_ARRAY);
        }
    }

    fn draw_water(
        &mut self,
        terrain: &TerrainGrid,
        sea_level_y: f32,
        color: [f32; 3],
        retail_tick: i32,
        frames: Option<&crate::water::WaterFrames>,
    ) {
        // The engine's Zarch-heritage water pass (FUN_00431a40 chain): one quad
        // per terrain cell. FUN_00445920 leaves dry corners on the static
        // sea plane; only wet corners animate and clamp troughs to ground.
        // This is separate from the physics ride surface. Every quad's 4-bit
        // corner-submerged code selects a shoreline shape sprite + rotation
        // from the 16-entry table at 0x4CACC8 (see `crate::water`); fully dry
        // cells (code 0) are skipped.
        //
        // Blend is the engine's water span function `dst = texel + dst/2`,
        // i.e. premultiplied `(ONE, ONE_MINUS_SRC_ALPHA)` with texel alpha
        // 0.5 baked into the frame texture. Without a frame set we fall back
        // to the flat translucent color.
        //
        // Water consumes the same per-corner depth fade as opaque terrain.
        // The indexed path resolves the authored palette row first, then
        // approaches half the fog colour so the retail `texel + dst/2`
        // composite terminates at exactly the already-fogged background.
        //
        // `FUN_00431A60` walks the same `DAT_004CAB70/74` scan dimensions as
        // opaque terrain. There is no separate coarse far ocean, enlarged
        // radius, or wave-damping rim in the original.
        let sea16 = terrain.header[0] >> 8; // sea level, engine 16-bit units
        let cx = self.cam_pos[0];
        let cz = self.cam_pos[2];
        let fog_near = self.fog_near;
        let fog_far = self.view_distance;
        let terrain_lights = self.terrain_lights.clone();
        let dimensions = frames.map_or([52, 30], |frames| [frames.scan_columns, frames.scan_rows]);
        let footprint = self.terrain_scan().footprint(dimensions);
        let forward = self.cam_forward;

        // Per-lattice-point terrain byte (engine submerged test is on the raw
        // byte: terrain*32 < sea), ground height, and wave surface.
        let cell_h8 = |x: i32, z: i32| -> i8 {
            let xi = (x.rem_euclid(GRID_SIZE as i32)) as usize;
            let zi = (z.rem_euclid(GRID_SIZE as i32)) as usize;
            terrain.cell(xi, zi).unwrap().height as i8
        };
        let surface_y = |x: i32, z: i32| -> f32 {
            let terrain_y = cell_h8(x, z) as f32 * v2k_formats::terrain::HEIGHT_SCALE;
            crate::water::surface_y_at_cell(x, z, retail_tick, sea_level_y, terrain_y)
        };

        // Emit triangles. Vertex format: x,y,z, r,g,b,a, u,v (9 floats).
        // Vertex color carries either the exact palette row/frame ids for the
        // shader or a scalar brightness for the RGBA fallback.
        let textured = frames.is_some();
        let indexed_water = textured && self.water_program.is_some();
        let fog_enabled = self.world_fog_enabled;
        let fog_color = self.world_fog_color;
        let verts = &mut self.water_vertices;
        verts.clear();
        let fade_at = |x: f32, z: f32| -> f32 {
            if !fog_enabled {
                return 0.0;
            }
            let dx = x - cx;
            let dz = z - cz;
            let depth = dx * forward[0] + dz * forward[1];
            ((depth - fog_near) / (fog_far - fog_near).max(1.0)).clamp(0.0, 1.0)
        };
        let shade_at = |x: i32, z: i32| -> u8 {
            let to_world_y =
                |y: f32| (y * 256.0).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            let wave_y = to_world_y(surface_y(x, z));
            let previous_wave_y = to_world_y(surface_y(x, z - 1));
            let light = terrain_lights
                .as_ref()
                .map(|lights| lights.sample(x, z))
                .unwrap_or(0);
            crate::water::vertex_shade(wave_y, previous_wave_y, light)
        };
        let push = |verts: &mut Vec<f32>,
                    x: f32,
                    y: f32,
                    z: f32,
                    u: f32,
                    v: f32,
                    shade: f32,
                    frame: f32| {
            if indexed_water {
                verts.extend_from_slice(&[x, y, z, shade, frame, 0.0, 1.0, u, v]);
            } else if textured {
                let vis = 1.0 - fade_at(x, z);
                let b = shade * vis;
                verts.extend_from_slice(&[x, y, z, b, b, b, vis, u, v]);
            } else {
                let b = shade;
                let fade = fade_at(x, z);
                let source = [
                    color[0] * b * (1.0 - fade) + fog_color[0] * fade,
                    color[1] * b * (1.0 - fade) + fog_color[1] * fade,
                    color[2] * b * (1.0 - fade) + fog_color[2] * fade,
                ];
                verts.extend_from_slice(&[x, y, z, source[0], source[1], source[2], 0.55, u, v]);
            }
        };

        for [world_x, world_z] in footprint.cells() {
            // Corner order c0=(x,z) c1=(x,z+1) c2=(x+1,z+1) c3=(x+1,z);
            // code bits per crate::water (bit i = corner c_i submerged).
            let submerged = |x, z| (cell_h8(x, z) as i32) * 32 < sea16;
            let code = submerged(world_x, world_z) as usize
                | (submerged(world_x, world_z + 1) as usize) << 1
                | (submerged(world_x + 1, world_z + 1) as usize) << 2
                | (submerged(world_x + 1, world_z) as usize) << 3;
            if code == 0 {
                continue; // fully dry — engine skips these
            }
            let xa = world_x as f32;
            let xb = xa + 1.0;
            let za = world_z as f32;
            let zb = za + 1.0;
            let pos = [
                [xa, surface_y(world_x, world_z), za],
                [xa, surface_y(world_x, world_z + 1), zb],
                [xb, surface_y(world_x + 1, world_z + 1), zb],
                [xb, surface_y(world_x + 1, world_z), za],
            ];
            // FUN_004327C0 combines all four projected corner flags
            // before either GL triangle exists. Include animated Y and
            // camera pitch; the scan's horizontal window is not depth.
            if pos.iter().any(|&point| {
                !ModelNearClip::RetailWorld.accepts_view_depth(world_view_depth(
                    point,
                    self.cam_pos,
                    self.cam_view_basis[2],
                ))
            }) {
                continue;
            }
            let mut uv = [[0.0f32; 2]; 4];
            if let Some(f) = frames {
                let e = &crate::water::SHORE_TABLE[code];
                let rect = &f.frame_uvs[e.frame as usize];
                for (c, corner_uv) in uv.iter_mut().enumerate() {
                    let (su, sv) = crate::water::SLOT_UV[e.slot[c] as usize];
                    *corner_uv = [
                        rect[0] + (rect[2] - rect[0]) * su,
                        rect[1] + (rect[3] - rect[1]) * sv,
                    ];
                }
            }
            let shades = [
                shade_at(world_x, world_z),
                shade_at(world_x, world_z + 1),
                shade_at(world_x + 1, world_z + 1),
                shade_at(world_x + 1, world_z),
            ];
            let frame = crate::water::SHORE_TABLE[code].frame;
            let shade_values = shades.map(|shade| {
                frames.map_or((shade as f32 + 1.0) / 8.0, |frames| {
                    if indexed_water {
                        frames.palette_shade(shade) as f32 / 31.0
                    } else {
                        frames.shade_scale(frame, shade)
                    }
                })
            });
            let frame_value = frame as f32 / 4.0;
            // Two triangles split along the c0-c2 diagonal.
            for &c in &[0usize, 1, 2, 0, 2, 3] {
                push(
                    verts,
                    pos[c][0],
                    pos[c][1],
                    pos[c][2],
                    uv[c][0],
                    uv[c][1],
                    shade_values[c],
                    frame_value,
                );
            }
        }
        self.submit_water_vertices(frames);
    }

    fn invalidate_terrain_cache(&mut self) {
        self.terrain_cache_footprint = None;
        self.terrain_light_revision = 0;
        self.terrain_type_signature = 0;
        self.terrain_vertices.clear();
        self.terrain_vertex_count = 0;
        self.terrain_infection_vertices.clear();
        self.terrain_infection_vertex_count = 0;
    }

    fn draw_model_body(&mut self, draw: ModelDraw<'_>) {
        let group_depth = match draw.depth_policy {
            ModelDepthPolicy::PainterGroup { view_depth_raw } => {
                let Some(depth) =
                    painter_group_window_depth(view_depth_raw, self.cam_near, self.cam_far)
                else {
                    return;
                };
                Some(depth)
            }
            ModelDepthPolicy::Geometry | ModelDepthPolicy::Painter => None,
        };
        let needs_compatibility_program = self.projection_effect != ProjectionEffect::None
            || (self.model_program.is_none()
                && matches!(
                    draw.depth_fade,
                    ModelDepthFade::Linear { .. }
                        | ModelDepthFade::FrontendFixedPointLinear { .. }
                        | ModelDepthFade::WorldRaw { .. }
                ));
        let compatibility_program = needs_compatibility_program
            .then_some(self.compatibility_program.as_ref())
            .flatten();
        // Screen-space intrinsics for recovered 0x02/0x22 fills. Keep the
        // off-centre sign conversion in the projection authority shared by
        // the headless selector tests and this live backend.
        let mut edge_intrinsics = crate::edge_quads::ScreenIntrinsics::from_projection_terms(
            [self.viewport.logical_width, self.viewport.logical_height],
            self.cam_projection_terms,
            [self.cam_near, self.cam_far],
        );
        if let Some(intr) = &mut edge_intrinsics {
            intr.projection_effect = self.projection_effect;
        }
        unsafe {
            // C6/46 groups are painter-list entries, not geometric Z offsets.
            // Isolated painter draws omit depth entirely. An opaque enclosing
            // group instead records its one key while LEQUAL lets later group
            // members overwrite ties and retains nearer existing geometry.
            // Preserve the complete depth state for either ordered policy.
            let painter = draw.depth_policy != ModelDepthPolicy::Geometry;
            let depth_state = painter.then(|| PainterDepthState::capture());
            if painter {
                if let Some(depth) = group_depth {
                    crate::gl::Enable(crate::gl::DEPTH_TEST);
                    crate::gl::DepthFunc(crate::gl::LEQUAL);
                    crate::gl::DepthRange(depth, depth);
                } else {
                    crate::gl::Disable(crate::gl::DEPTH_TEST);
                }
            }
            draw_tris_gl(
                draw,
                self.cam_pos,
                self.cam_view_basis,
                self.model_program.as_ref(),
                compatibility_program,
                &self.indexed_model_textures,
                self.fog_near,
                self.view_distance,
                self.world_fog_color,
                self.overlay_depth_policy,
                edge_intrinsics,
            );
            if let Some(state) = depth_state {
                state.restore();
            }
        }
    }

    fn draw_model_billboards(&mut self, draw: ModelBillboardDraw<'_>) {
        let ModelBillboardDraw {
            vertices,
            vertex_projection,
            vertex_clip,
            vertex_view_raw: _,
            billboards,
            materials,
            transform,
            radius_raw,
            depth_fade,
            near_clip,
            depth_policy,
        } = draw;
        if vertices.is_empty() || billboards.is_empty() {
            return;
        }
        let raw_scale = transform.scale / 100.0;
        let world_vertices = vertices
            .iter()
            .enumerate()
            .map(|(index, anchor)| {
                if vertex_clip
                    .get(index)
                    .is_some_and(|clip| *clip != ModelSlotClip::Clear)
                {
                    return None;
                }
                let local = anchor.map(|value| value as f32 * raw_scale);
                Some(std::array::from_fn(|i| {
                    transform.position[i] + dot3(transform.orientation[i], local)
                }))
            })
            .collect::<Vec<_>>();
        let vertex_admitted = vertex_admission(
            near_clip,
            &world_vertices,
            vertex_projection,
            self.cam_pos,
            self.cam_view_basis[2],
        );
        let fade = resolve_model_depth_fade(
            depth_fade,
            self.world_fog_enabled,
            self.fog_near,
            self.view_distance,
            self.world_fog_color,
        );
        let relative = std::array::from_fn(|i| transform.position[i] - self.cam_pos[i]);
        let pass = selected_model_fog_pass(depth_fade).unwrap_or_else(|| {
            model_fog_pass(
                -dot3(relative, self.cam_view_basis[2]),
                f32::from(radius_raw) * raw_scale.abs(),
                fade.enabled.then_some([fade.near, fade.far]),
            )
        });
        // Keep the existing opaque-before-translucent attachment order. This
        // adapter does not replace the hierarchy's separate painter policy.
        let mut order: Vec<usize> = (0..billboards.len().min(materials.len())).collect();
        if depth_policy == ModelDepthPolicy::Geometry {
            order.sort_by_key(|&i| !billboard_writes_depth(materials[i].blend));
        }
        let mut quads = Vec::with_capacity(order.len());
        for index in order {
            let billboard = &billboards[index];
            let material = &materials[index];
            let Some(anchor) = vertices.get(billboard.vertex as usize) else {
                continue;
            };
            if billboard.size == 0 {
                continue;
            }
            let local = anchor.map(|value| value as f32 * raw_scale);
            let center = std::array::from_fn(|i| {
                transform.position[i] + dot3(transform.orientation[i], local)
            });
            if vertex_admitted.get(usize::from(billboard.vertex)) != Some(&true) {
                continue;
            }
            let relative = std::array::from_fn(|i| center[i] - self.cam_pos[i]);
            let fog = if billboard.textured {
                let Some(fog) =
                    model_billboard_fog(-dot3(relative, self.cam_view_basis[2]), pass, fade)
                else {
                    continue;
                };
                fog
            } else {
                // 68/E8 use separate A960/A980 solid fillers. Their current
                // additive presentation is outside the indexed sprite repair.
                SpriteFog::Near
            };
            let extent = billboard.size as f32 * 2.0 * raw_scale;
            let half_extent = if billboard.textured {
                let max_dim = material.width.max(material.height).max(1) as f32;
                let extent = extent * std::f32::consts::FRAC_1_SQRT_2;
                [
                    extent * material.width.max(1) as f32 / max_dim,
                    extent * material.height.max(1) as f32 / max_dim,
                ]
            } else {
                [extent; 2]
            };
            let mut face = material.face;
            face.blend = material.blend;
            quads.push(SpriteQuad {
                corners: camera_facing_corners(
                    center,
                    half_extent,
                    billboard.angle as f32 / 65536.0 * std::f32::consts::TAU,
                    self.cam_view_basis,
                ),
                material: face,
                opacity: model_face_opacity(material.blend),
                fog,
                fog_color: fade.color,
            });
        }
        self.submit_sprite_quads(&quads, depth_policy);
    }

    fn draw_world_sprites(&mut self, sprites: &[WorldSprite]) {
        let mut quads = Vec::with_capacity(sprites.len());
        for index in world_sprite_draw_order(sprites) {
            let sprite = &sprites[index];
            if !sprite.position.iter().all(|v| v.is_finite())
                || !sprite.size.iter().all(|v| v.is_finite())
                || !sprite.color.iter().all(|v| v.is_finite())
                || !sprite.rotation.is_finite()
                || sprite.size[0] <= 0.0
                || sprite.size[1] <= 0.0
            {
                continue;
            }
            quads.push(SpriteQuad {
                corners: camera_facing_corners(
                    sprite.position,
                    sprite.size.map(|v| v * 0.5),
                    sprite.rotation,
                    self.cam_view_basis,
                ),
                material: FaceMaterial {
                    color: [sprite.color[0], sprite.color[1], sprite.color[2]]
                        .map(|v| v.clamp(0.0, 1.0)),
                    palette_rgb555: None,
                    emissive: [0.0; 3],
                    texture: Some(sprite.texture),
                    blend: sprite.blend,
                    flat_shade_row: sprite.flat_shade_row,
                },
                opacity: sprite.color[3].clamp(0.0, 1.0),
                fog: sprite.fog,
                fog_color: self.world_fog_color,
            });
        }
        self.submit_sprite_quads(&quads, ModelDepthPolicy::Geometry);
    }
    fn create_texture(&mut self, rgba: &[u8], width: u32, height: u32) -> Option<TextureId> {
        self.upload_persistent_texture(rgba, width, height)
    }

    fn create_texture_nearest(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Option<TextureId> {
        self.upload_persistent_texture(rgba, width, height)
    }

    fn create_indexed_model_texture(
        &mut self,
        texture: IndexedModelTexture<'_>,
    ) -> Option<TextureId> {
        let pixel_count = texture.width as usize * texture.height as usize;
        if texture.indices.len() < pixel_count
            || texture.palette_rgba.len() < 32 * 16 * 4
            || texture.fallback_rgba.len() < pixel_count * 4
        {
            return None;
        }

        // FUN_004782B0 reads the palette base, or adds 0x380 for flag04.
        // Shaderless drivers need both fixed rows; lit faces retain their
        // brightest-row fallback and separate Section-6 RGB modulation.
        let flat_rgba = if self.model_program.is_none() {
            Some(
                MODEL_FLAT_SHADE_ROWS
                    .iter()
                    .map(|&row| {
                        indexed_model_rgba_at_row(
                            texture.indices,
                            texture.palette_rgba,
                            pixel_count,
                            row,
                            texture.transparent_zero,
                        )
                    })
                    .collect::<Option<Vec<_>>>()?,
            )
        } else {
            None
        };

        let fallback =
            self.upload_persistent_texture(texture.fallback_rgba, texture.width, texture.height)?;
        let mut flat_rows = [None; 2];
        if let Some(rows) = flat_rgba {
            for (slot, rgba) in rows.iter().enumerate() {
                let Some(row_texture) =
                    self.upload_persistent_texture(rgba, texture.width, texture.height)
                else {
                    for texture in flat_rows.into_iter().flatten() {
                        self.delete_persistent_texture(texture);
                    }
                    self.delete_persistent_texture(fallback);
                    return None;
                };
                flat_rows[slot] = Some(row_texture);
            }
        }
        let mut index_rgba = Vec::with_capacity(pixel_count * 4);
        for &index in texture.indices.iter().take(pixel_count) {
            index_rgba.extend_from_slice(&[index, 0, 0, 255]);
        }
        let Some(index) =
            self.upload_persistent_texture(&index_rgba, texture.width, texture.height)
        else {
            for row_texture in flat_rows.into_iter().flatten() {
                self.delete_persistent_texture(row_texture);
            }
            self.delete_persistent_texture(fallback);
            return None;
        };
        let Some(palette) = self.upload_persistent_texture(texture.palette_rgba, 16, 32) else {
            self.delete_persistent_texture(index);
            for row_texture in flat_rows.into_iter().flatten() {
                self.delete_persistent_texture(row_texture);
            }
            self.delete_persistent_texture(fallback);
            return None;
        };
        self.indexed_model_textures.insert(
            fallback.0,
            IndexedModelTextures {
                index,
                palette,
                flat_rows,
                transparent_zero: texture.transparent_zero,
            },
        );
        Some(fallback)
    }

    fn destroy_texture(&mut self, id: TextureId) {
        if let Some(indexed) = self.indexed_model_textures.remove(&id.0) {
            self.delete_persistent_texture(indexed.index);
            self.delete_persistent_texture(indexed.palette);
            for row_texture in indexed.flat_rows.into_iter().flatten() {
                self.delete_persistent_texture(row_texture);
            }
        }
        self.delete_persistent_texture(id);
    }

    fn set_sprite_clip(&mut self, rect: Option<(i32, i32, u32, u32)>) {
        unsafe {
            if let Some((x, y, width, height)) = rect {
                // Menu/layout coordinates start at the top-left; OpenGL's
                // scissor rectangle starts at the bottom-left and uses
                // physical pixels even when the logical frame is stretched.
                let x0 = x.clamp(0, self.width as i32);
                let y0 = y.clamp(0, self.height as i32);
                let x1 = (x.saturating_add(width as i32)).clamp(x0, self.width as i32);
                let y1 = (y.saturating_add(height as i32)).clamp(y0, self.height as i32);
                let sx = self.viewport.physical_width as f64 / self.width as f64;
                let sy = self.viewport.physical_height as f64 / self.height as f64;
                let (px0, py0, px1, py1) = (
                    self.viewport.x + (x0 as f64 * sx).floor() as i32,
                    self.viewport.y + self.viewport.physical_height as i32
                        - (y1 as f64 * sy).ceil() as i32,
                    self.viewport.x + (x1 as f64 * sx).ceil() as i32,
                    self.viewport.y + self.viewport.physical_height as i32
                        - (y0 as f64 * sy).floor() as i32,
                );
                crate::gl::Enable(crate::gl::SCISSOR_TEST);
                crate::gl::Scissor(px0, py0, px1 - px0, py1 - py0);
            } else {
                crate::gl::Disable(crate::gl::SCISSOR_TEST);
            }
        }
    }

    fn draw_sprite(&mut self, rgba: &[u8], width: u32, height: u32, x: i32, y: i32) {
        self.draw_overlay_sprite(
            rgba,
            width,
            height,
            x,
            y,
            OverlaySpriteBlend::ConventionalAlpha,
        );
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
        self.draw_overlay_sprite(rgba, width, height, x, y, OverlaySpriteBlend::Retail(blend));
    }

    fn draw_material_sprite_quad(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        corners: [(i32, i32); 4],
        blend: WorldSpriteBlend,
    ) {
        self.draw_overlay_sprite_corners(
            rgba,
            width,
            height,
            corners.map(|(x, y)| (x as f32, y as f32)),
            OverlaySpriteBlend::Retail(blend),
        );
    }

    fn draw_additive_sprite_at_depth(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        x: i32,
        y: i32,
        view_depth: f32,
        near: f32,
        far: f32,
    ) {
        if rgba.is_empty()
            || width == 0
            || height == 0
            || near <= 0.0
            || far <= near
            || view_depth <= 0.0
        {
            return;
        }

        // Perspective depth for eye-space z=-view_depth. The temporary ortho
        // matrix maps vertex z to -NDC z, so negate it when submitting the
        // screen-space quad.
        let depth = view_depth.clamp(near, far);
        let ndc_z = perspective_ndc_depth(depth, near, far);
        let ortho_z = -ndc_z;

        let filter = self.transient_sprite_filter();
        unsafe {
            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();
            crate::gl::Ortho(0.0, self.width as f64, self.height as f64, 0.0, -1.0, 1.0);
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();

            crate::gl::Enable(crate::gl::DEPTH_TEST);
            crate::gl::DepthFunc(crate::gl::LESS);
            crate::gl::DepthMask(crate::gl::FALSE);
            crate::gl::Disable(crate::gl::CULL_FACE);
            crate::gl::Enable(crate::gl::TEXTURE_2D);
            crate::gl::Enable(crate::gl::ALPHA_TEST);
            crate::gl::AlphaFunc(crate::gl::GREATER, 0.5);
            crate::gl::Enable(crate::gl::BLEND);
            // Retail's near additive sprite filler accumulates source and
            // destination; GL ONE/ONE implements that software blend here.
            crate::gl::BlendFunc(crate::gl::ONE, crate::gl::ONE);

            let mut tex_id: u32 = 0;
            crate::gl::GenTextures(1, &mut tex_id);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, tex_id);
            crate::gl::TexParameteri(crate::gl::TEXTURE_2D, crate::gl::TEXTURE_MIN_FILTER, filter);
            crate::gl::TexParameteri(crate::gl::TEXTURE_2D, crate::gl::TEXTURE_MAG_FILTER, filter);
            crate::gl::TexImage2D(
                crate::gl::TEXTURE_2D,
                0,
                crate::gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                crate::gl::RGBA,
                crate::gl::UNSIGNED_BYTE,
                rgba.as_ptr() as *const _,
            );

            let x0 = x as f32;
            let y0 = y as f32;
            let x1 = x0 + width as f32;
            let y1 = y0 + height as f32;
            crate::gl::Color4f(1.0, 1.0, 1.0, 1.0);
            crate::gl::Begin(crate::gl::QUADS);
            crate::gl::TexCoord2f(0.0, 0.0);
            crate::gl::Vertex3f(x0, y0, ortho_z);
            crate::gl::TexCoord2f(1.0, 0.0);
            crate::gl::Vertex3f(x1, y0, ortho_z);
            crate::gl::TexCoord2f(1.0, 1.0);
            crate::gl::Vertex3f(x1, y1, ortho_z);
            crate::gl::TexCoord2f(0.0, 1.0);
            crate::gl::Vertex3f(x0, y1, ortho_z);
            crate::gl::End();

            crate::gl::DeleteTextures(1, &tex_id);
            crate::gl::Disable(crate::gl::BLEND);
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::DepthMask(crate::gl::TRUE);
            crate::gl::Enable(crate::gl::CULL_FACE);

            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PopMatrix();
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PopMatrix();
        }
    }

    fn draw_fullscreen(&mut self, rgba: &[u8], width: u32, height: u32) {
        if rgba.is_empty() || width == 0 || height == 0 {
            return;
        }

        let filter = crate::gl::LINEAR as i32;
        unsafe {
            let fog_was_enabled = crate::gl::IsEnabled(crate::gl::FOG) != 0;

            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();
            crate::gl::Ortho(0.0, 1.0, 1.0, 0.0, -1.0, 1.0);
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();

            crate::gl::Disable(crate::gl::DEPTH_TEST);
            crate::gl::Disable(crate::gl::CULL_FACE);
            crate::gl::Disable(crate::gl::FOG);
            crate::gl::Enable(crate::gl::TEXTURE_2D);

            let mut tex_id: u32 = 0;
            crate::gl::GenTextures(1, &mut tex_id);
            crate::gl::BindTexture(crate::gl::TEXTURE_2D, tex_id);
            crate::gl::TexParameteri(crate::gl::TEXTURE_2D, crate::gl::TEXTURE_MIN_FILTER, filter);
            crate::gl::TexParameteri(crate::gl::TEXTURE_2D, crate::gl::TEXTURE_MAG_FILTER, filter);
            crate::gl::TexImage2D(
                crate::gl::TEXTURE_2D,
                0,
                crate::gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                crate::gl::RGBA,
                crate::gl::UNSIGNED_BYTE,
                rgba.as_ptr() as *const _,
            );

            crate::gl::Color4f(1.0, 1.0, 1.0, 1.0);
            crate::gl::Begin(crate::gl::QUADS);
            crate::gl::TexCoord2f(0.0, 0.0);
            crate::gl::Vertex2f(0.0, 0.0);
            crate::gl::TexCoord2f(1.0, 0.0);
            crate::gl::Vertex2f(1.0, 0.0);
            crate::gl::TexCoord2f(1.0, 1.0);
            crate::gl::Vertex2f(1.0, 1.0);
            crate::gl::TexCoord2f(0.0, 1.0);
            crate::gl::Vertex2f(0.0, 1.0);
            crate::gl::End();

            crate::gl::DeleteTextures(1, &tex_id);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::Enable(crate::gl::DEPTH_TEST);
            crate::gl::Enable(crate::gl::CULL_FACE);

            if fog_was_enabled {
                crate::gl::Enable(crate::gl::FOG);
            }

            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PopMatrix();
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PopMatrix();
        }
    }

    fn draw_color_overlay(&mut self, r: f32, g: f32, b: f32, a: f32) {
        if a < 0.001 {
            return;
        }
        unsafe {
            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();
            crate::gl::Ortho(0.0, 1.0, 1.0, 0.0, -1.0, 1.0);
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PushMatrix();
            crate::gl::LoadIdentity();

            crate::gl::Disable(crate::gl::DEPTH_TEST);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::Disable(crate::gl::CULL_FACE);
            crate::gl::Disable(crate::gl::FOG);
            crate::gl::Enable(crate::gl::BLEND);
            crate::gl::BlendFunc(crate::gl::SRC_ALPHA, crate::gl::ONE_MINUS_SRC_ALPHA);

            crate::gl::Color4f(r, g, b, a);
            crate::gl::Begin(crate::gl::QUADS);
            crate::gl::Vertex2f(0.0, 0.0);
            crate::gl::Vertex2f(1.0, 0.0);
            crate::gl::Vertex2f(1.0, 1.0);
            crate::gl::Vertex2f(0.0, 1.0);
            crate::gl::End();

            crate::gl::Disable(crate::gl::BLEND);
            crate::gl::Enable(crate::gl::DEPTH_TEST);
            crate::gl::Enable(crate::gl::CULL_FACE);

            crate::gl::MatrixMode(crate::gl::PROJECTION);
            crate::gl::PopMatrix();
            crate::gl::MatrixMode(crate::gl::MODELVIEW);
            crate::gl::PopMatrix();
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

    fn set_display(&mut self, request: crate::config::DisplayRequest) -> Result<(), String> {
        let result = crate::window::apply_display(&mut self.window, request);
        // Adopt the drawable either way: a restored window may still differ.
        let (w, h) = self.window.drawable_size();
        self.resize(w, h);
        result
    }

    fn display_index(&self) -> Option<i32> {
        self.window.display_index().ok()
    }

    fn window_placement(&self) -> Option<crate::WindowPlacement> {
        crate::WindowPlacement::of(&self.window)
    }

    fn set_scaling_mode(&mut self, mode: ScalingMode, reference_width: u32, reference_height: u32) {
        self.scaling_mode = mode;
        self.reference_width = reference_width.max(1);
        self.reference_height = reference_height.max(1);
        self.update_render_viewport();
    }
}

#[cfg(test)]
mod callback_face_coordinate_tests {
    use super::model_face_vertex_world_or_local;
    #[test]
    fn mixed_face_uses_world_endpoints_for_every_corner_and_both_passes() {
        // The deliberately unrelated storage coordinates cannot become the
        // owned foot, and ordinary corners of the same face use their already
        // transformed world coordinates under that face's camera matrix.
        let local = [Some([900.0, 800.0, 700.0]), Some([20.0, 30.0, 40.0])];
        let world = [Some([1.125, -2.5, 3.75]), Some([4.0, 5.0, 6.0])];
        for _colour_pass in 0..2 {
            assert_eq!(
                model_face_vertex_world_or_local(true, 0, &local, &world, 0.25),
                [1.125, -2.5, 3.75]
            );
            assert_eq!(
                model_face_vertex_world_or_local(true, 1, &local, &world, 0.25),
                [4.0, 5.0, 6.0]
            );
        }
        assert_eq!(
            model_face_vertex_world_or_local(false, 1, &local, &world, 0.25),
            [5.0, 7.5, 10.0]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_external_frame_operands, authored_face_plane_visible, billboard_writes_depth, dot3,
        edge_endpoint_fade_bytes, edge_endpoint_world, edge_quad_fade_bytes,
        fixed_function_fog_far, fixed_function_model_texture, flat_lit_palette_color,
        flip_rgba_rows, indexed_model_rgba_at_row, infection_corner_mask, infection_quad_vertices,
        model_face_opacity, model_face_palette_row, model_face_writes_depth,
        model_lighting_normals, overlay_sprite_filter, painter_group_window_depth,
        perspective_ndc_depth, resolve_model_depth_fade, retail_flat_lit_rgb565,
        retail_model_shade_index, retail_model_shade_raw, retail_view_pin_vertex,
        retail_world_surface_alias_vertex, retail_world_surface_vertex, scene_uses_world_fog,
        terrain_type_signature, transform_model_light_normal, triangle_uses_surface_overlay_depth,
        view_light_normal, view_pin_triangle_is_color_underlay,
        view_pin_triangle_is_world_surface_decal, view_pin_triangle_needs_coplanar_depth,
        view_pin_triangle_visible, world_point_as_raw_local, world_sprite_draw_order,
        IndexedModelTextures, OverlaySpriteBlend, SurfaceVertexSource, MODEL_FIXED_SHADE_ROW,
        RETAIL_VIEW_PIN_Y,
    };
    use crate::config::ScalingMode;
    use crate::renderer::{
        ExternalFrameMode, ModelDepthFade, ModelOverlayKind, RenderScene, TextureId, ViewPinMode,
        WorldSprite, WorldSpriteBlend, WorldSurfaceProjection, HALF_ADDITIVE_ALPHA,
        RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
    };
    use v2k_formats::models::{
        ModelFaceCullPlane, ModelFaceShading, ModelSlotClip, ModelSurfaceOrigin,
    };
    use v2k_formats::system::FogGradientEntry;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    #[test]
    fn menu_never_inherits_world_fog_and_world_restores_it() {
        assert!(!scene_uses_world_fog(RenderScene::Menu, true));
        assert!(!scene_uses_world_fog(RenderScene::Menu, false));
        assert!(scene_uses_world_fog(RenderScene::World, true));
        assert!(!scene_uses_world_fog(RenderScene::World, false));
    }

    #[test]
    fn model_depth_fade_resolves_without_mutating_scene_policy() {
        let world_color = [0.1, 0.2, 0.3];
        let inherited = resolve_model_depth_fade(
            ModelDepthFade::InheritWorldFog,
            true,
            12.0,
            48.0,
            world_color,
        );
        assert!(inherited.enabled);
        assert_eq!(inherited.near, 12.0);
        assert_eq!(inherited.far, 48.0);
        assert_eq!(inherited.color, world_color);

        let disabled =
            resolve_model_depth_fade(ModelDepthFade::Disabled, true, 12.0, 48.0, world_color);
        assert!(!disabled.enabled);

        let explicit = resolve_model_depth_fade(
            ModelDepthFade::Linear {
                near: 7.0,
                far: 7.0,
                color: [0.8, 0.7, 0.6],
            },
            false,
            12.0,
            48.0,
            world_color,
        );
        assert!(explicit.enabled);
        assert_eq!(explicit.near, 7.0);
        assert_eq!(explicit.far, 7.0);
        assert_eq!(explicit.color, [0.8, 0.7, 0.6]);

        let fixed = resolve_model_depth_fade(
            ModelDepthFade::FrontendFixedPointLinear {
                near_raw: 1276,
                far_raw: 2812,
                color: [0.9, 0.1, 0.0],
            },
            false,
            12.0,
            48.0,
            world_color,
        );
        assert_eq!(fixed.near, 12.76);
        assert_eq!(fixed.far, 28.12);
        assert_eq!(fixed.color, [0.9, 0.1, 0.0]);
        assert_eq!(
            fixed.byte_at_raw_depth((20.44_f32 * 256.0).round() as i32),
            127,
            "FUN_00470A10 truncates the reciprocal before the midpoint multiply"
        );
    }

    #[test]
    fn fixed_function_non_increasing_fog_planes_expand_to_a_positive_band() {
        let far = fixed_function_fog_far(7.0, 7.0);
        assert!(far > 7.0);
        assert!(fixed_function_fog_far(7.0, 6.0) > 7.0);
        assert_eq!(fixed_function_fog_far(7.0, 8.0), 8.0);
    }

    #[test]
    fn edge_fade_quantizes_retail_endpoint_bytes_and_corner_mapping() {
        let fade = resolve_model_depth_fade(
            ModelDepthFade::Linear {
                near: 10.0,
                far: 14.0,
                color: [1.0, 0.0, 0.0],
            },
            false,
            0.0,
            0.0,
            [0.0; 3],
        );
        assert_eq!(fade.byte_at_raw_depth(9 * 256), 0);
        assert_eq!(fade.byte_at_raw_depth(10 * 256), 0);
        assert_eq!(fade.byte_at_raw_depth(11 * 256), 64);
        assert_eq!(fade.byte_at_raw_depth(12 * 256), 128);
        assert_eq!(fade.byte_at_raw_depth(13 * 256), 192);
        assert_eq!(fade.byte_at_raw_depth(14 * 256), u8::MAX);

        assert_eq!(
            edge_quad_fade_bytes(fade, [10 * 256, 12 * 256]),
            Some([0, 0, 128, 128]),
            "constructor corners A/B inherit start and C/D inherit end"
        );
        assert_eq!(
            edge_endpoint_fade_bytes(fade, [14 * 256, 13 * 256]),
            [u8::MAX, 192],
            "one terminal endpoint must retain the lengthwise gradient"
        );
        assert_eq!(
            edge_quad_fade_bytes(fade, [14 * 256, 15 * 256]),
            None,
            "retail's B:22 path rejects a pair of terminal endpoint bytes"
        );
        assert_eq!(
            edge_endpoint_fade_bytes(fade, [14 * 256, 15 * 256]),
            [u8::MAX; 2],
            "B:02 palette hairlines retain terminal-color endpoints"
        );
    }

    #[test]
    fn edge_fade_preserves_disabled_and_equal_plane_policies() {
        let disabled = resolve_model_depth_fade(ModelDepthFade::Disabled, true, 1.0, 2.0, [0.2; 3]);
        assert_eq!(
            edge_endpoint_fade_bytes(disabled, [100 * 256, 200 * 256]),
            [0, 0]
        );

        let step = resolve_model_depth_fade(
            ModelDepthFade::Linear {
                near: 7.0,
                far: 7.0,
                color: [1.0, 0.0, 0.0],
            },
            false,
            0.0,
            0.0,
            [0.0; 3],
        );
        assert_eq!(step.byte_at_raw_depth(7 * 256 - 1), 0);
        assert_eq!(step.byte_at_raw_depth(7 * 256), u8::MAX);
    }

    #[test]
    fn live_terrain_type_mutation_invalidates_the_cached_mesh_signature() {
        let mut terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let before = terrain_type_signature(&terrain);
        terrain.cells[123].attribute = 0xff;
        assert_eq!(terrain_type_signature(&terrain), before);
        terrain.cells[123].terrain_type = 0x10;
        assert_ne!(terrain_type_signature(&terrain), before);
    }

    #[test]
    fn infection_mask_uses_retail_corner_bit_order_only() {
        assert_eq!(infection_corner_mask([0x10, 0, 0, 0]), 0b0001);
        assert_eq!(infection_corner_mask([0, 0x10, 0, 0]), 0b0010);
        assert_eq!(infection_corner_mask([0, 0, 0x10, 0]), 0b0100);
        assert_eq!(infection_corner_mask([0, 0, 0, 0x10]), 0b1000);
        assert_eq!(infection_corner_mask([0x10; 4]), 0b1111);
        assert_eq!(infection_corner_mask([0xef, 0x0f, 0xe0, 0x08]), 0);
    }

    #[test]
    fn infection_quad_maps_exact_frames_uvs_and_terrain_triangle_order() {
        const L: f32 = 0.125;
        const T: f32 = 0.25;
        const R: f32 = 0.75;
        const B: f32 = 0.875;
        let frame_uvs = [[L, T, R, B]; crate::terrain_tiles::INFECTION_FRAME_COUNT];
        let positions = [
            [0.0, 10.0, 0.0],
            [0.0, 20.0, 1.0],
            [1.0, 30.0, 1.0],
            [1.0, 40.0, 0.0],
        ];
        let triangle_corners = [0usize, 1, 3, 1, 2, 3];
        let one_corner_cases = [
            (0b0001, [[L, B], [R, B], [R, T], [L, T]]),
            (0b0010, [[L, T], [L, B], [R, B], [R, T]]),
            (0b0100, [[R, T], [R, B], [L, B], [L, T]]),
            (0b1000, [[L, T], [R, T], [R, B], [L, B]]),
        ];

        for (mask, expected_corner_uvs) in one_corner_cases {
            let (frame, vertices) = infection_quad_vertices(mask, &frame_uvs, positions).unwrap();
            assert_eq!(frame, 0, "mask {mask:04b}");
            for (vertex, corner) in vertices.iter().zip(triangle_corners) {
                assert_eq!(&vertex[..3], &positions[corner], "mask {mask:04b}");
                assert_eq!(
                    &vertex[3..],
                    &expected_corner_uvs[corner],
                    "mask {mask:04b}"
                );
            }
        }

        assert_eq!(
            infection_quad_vertices(0b0101, &frame_uvs, positions)
                .unwrap()
                .0,
            2
        );
        assert_eq!(
            infection_quad_vertices(0b1010, &frame_uvs, positions)
                .unwrap()
                .0,
            2
        );
        assert_eq!(
            infection_quad_vertices(0b1111, &frame_uvs, positions)
                .unwrap()
                .0,
            4
        );
        assert!(infection_quad_vertices(0, &frame_uvs, positions).is_none());
        assert!(infection_quad_vertices(0x10, &frame_uvs, positions).is_none());
    }

    #[test]
    fn retail_material_overlays_keep_nearest_sampling_in_every_scaling_mode() {
        for mode in [
            ScalingMode::Native,
            ScalingMode::FourThree,
            ScalingMode::Stretched,
        ] {
            for blend in [
                WorldSpriteBlend::Masked,
                WorldSpriteBlend::Additive,
                WorldSpriteBlend::HalfAdditive,
            ] {
                assert_eq!(
                    overlay_sprite_filter(mode, OverlaySpriteBlend::Retail(blend)),
                    crate::gl::NEAREST as i32
                );
            }
        }
    }

    #[test]
    fn conventional_overlay_filter_retains_output_scaling_policy() {
        assert_eq!(
            overlay_sprite_filter(ScalingMode::Native, OverlaySpriteBlend::ConventionalAlpha),
            crate::gl::NEAREST as i32
        );
        for mode in [ScalingMode::FourThree, ScalingMode::Stretched] {
            assert_eq!(
                overlay_sprite_filter(mode, OverlaySpriteBlend::ConventionalAlpha),
                crate::gl::LINEAR as i32
            );
        }
    }

    #[test]
    fn presented_frame_readback_is_flipped_to_top_down_rows() {
        let mut rgba = vec![
            1, 2, 3, 4, 5, 6, 7, 8, // OpenGL bottom row
            9, 10, 11, 12, 13, 14, 15, 16, // OpenGL top row
        ];
        flip_rgba_rows(&mut rgba, 2, 2);
        assert_eq!(
            rgba,
            vec![9, 10, 11, 12, 13, 14, 15, 16, 1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn retail_model_light_uses_the_recovered_sixteen_slot_table() {
        let inv_sqrt_3 = 0.577_350_26;
        assert_eq!(
            retail_model_shade_index(
                [inv_sqrt_3, inv_sqrt_3, -inv_sqrt_3],
                RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
                0,
            ),
            7
        );
        assert_eq!(
            retail_model_shade_index(
                [1.0, 0.0, 0.0],
                RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
                0,
            ),
            4
        );
        assert_eq!(
            retail_model_shade_index(
                [0.707_106_77, -0.707_106_77, 0.0],
                RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
                0,
            ),
            0
        );
        // Slots 8..15 all contain Section-6 id zero in both retail contexts.
        assert_eq!(
            retail_model_shade_index(
                [-inv_sqrt_3, -inv_sqrt_3, inv_sqrt_3],
                RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW,
                0,
            ),
            0
        );
    }

    #[test]
    fn integer_model_light_bins_the_raw_dot_as_46d3f0_does() {
        let table: Vec<FogGradientEntry> = (0..8)
            .map(|shade_level| FogGradientEntry {
                r: 0,
                g: 0,
                b: 0,
                shade_level,
            })
            .collect();
        let level = |normal, light, shift| {
            retail_model_shade_raw(Some(&table), normal, light, shift)
                .map(|entry| entry.shade_level)
        };
        let light = RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW;
        assert_eq!(level([32_767, 0, 0], light, 0), Some(4));
        assert_eq!(level([-32_767, 0, 0], light, 0), Some(0));
        assert_eq!(level([32_767, 0, 0], light, 2), Some(6));
        assert_eq!(level([0, 0, 0], light, 0), Some(0));
        // The slot is the dot's four bits, not a clamped bin.
        assert_eq!(level([32_767, 0, 0], [8_000, 0, 0], 0), Some(3));
    }

    #[test]
    fn frontend_klaus_uses_its_distinct_raw_light_vector() {
        let klaus = [-100, 50, -50];
        assert_eq!(retail_model_shade_index([1.0, 0.0, 0.0], klaus, 0), 0);
        assert_eq!(retail_model_shade_index([-1.0, 0.0, 0.0], klaus, 0), 6);
        assert_eq!(retail_model_shade_index([0.0, 1.0, 0.0], klaus, 0), 3);
        assert_eq!(retail_model_shade_index([0.0, 0.0, -1.0], klaus, 0), 3);
    }

    #[test]
    fn contextual_model_shade_shift_clamps_the_signed_16_slot_table() {
        let normal = [1.0, 0.0, 0.0];
        assert_eq!(
            retail_model_shade_index(normal, RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW, 2),
            6
        );
        assert_eq!(
            retail_model_shade_index(normal, RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW, 99),
            7
        );
        // A sufficiently dark shift enters a negative slot, whose authored
        // table entry is Section-6 id zero.
        assert_eq!(
            retail_model_shade_index(normal, RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW, -5),
            0
        );
        assert_eq!(
            retail_model_shade_index(normal, RETAIL_ORDINARY_MODEL_LIGHT_DIRECTION_RAW, -99),
            0
        );
    }

    #[test]
    fn perspective_depth_maps_camera_planes_to_ndc_edges() {
        let near = 0.05;
        let far = 200.0;
        assert!((perspective_ndc_depth(near, near, far) + 1.0).abs() < 1e-5);
        assert!((perspective_ndc_depth(far, near, far) - 1.0).abs() < 1e-5);
        assert!(perspective_ndc_depth(23.0, near, far) < perspective_ndc_depth(30.72, near, far));
        assert!(perspective_ndc_depth(30.72, near, far) < perspective_ndc_depth(47.0, near, far));
    }

    #[test]
    fn enclosing_group_depth_orders_front_logo_and_far_flying_props() {
        let window_depth = |raw| painter_group_window_depth(raw, 0.05, 200.0).unwrap();
        let klaus = window_depth(0x1400);
        let logo = window_depth(0xC00);
        let rear_ring = window_depth(4700);
        let flying_ring = window_depth(4700 + 0x7000 / 2);

        // The group key, not a nearer wing/cover vertex, owns inter-group
        // visibility. A flying prop can cross behind that whole group.
        assert!(logo < klaus);
        assert!(rear_ring < klaus);
        assert!(klaus < flying_ring);
        let expected = (perspective_ndc_depth(51.2, 0.05, 200.0) + 1.0) * 0.5;
        assert_eq!(klaus, f64::from(expected));
    }

    #[test]
    fn enclosing_group_depth_clamps_signed_keys_and_rejects_invalid_cameras() {
        for key in [i32::MIN, -1, 0, 5] {
            assert_eq!(painter_group_window_depth(key, 0.05, 200.0), Some(0.0));
        }
        for key in [20_000, i32::MAX] {
            assert_eq!(painter_group_window_depth(key, 0.05, 200.0), Some(1.0));
        }
        for (near, far) in [
            (0.0, 200.0),
            (-1.0, 200.0),
            (1.0, 1.0),
            (2.0, 1.0),
            (f32::NAN, 200.0),
            (0.05, f32::INFINITY),
        ] {
            assert_eq!(painter_group_window_depth(0x1400, near, far), None);
        }
    }

    #[test]
    fn premultiplied_water_terminal_composite_is_exact_fog_colour() {
        let fog = [0.12_f32, 0.46, 0.68];
        let source = fog.map(|channel| channel * 0.5);
        let destination = fog;
        let composite = source
            .iter()
            .zip(destination)
            .map(|(source, destination)| source + destination * 0.5)
            .collect::<Vec<_>>();
        for (actual, expected) in composite.into_iter().zip(fog) {
            assert!((actual - expected).abs() < 1.0e-6);
        }
    }

    #[test]
    fn view_pin_modes_apply_explicit_whole_face_policy() {
        let flags = [0, 13, 0, 0];
        let ordinary = [0, 2, 3];
        let mixed = [0, 1, 2];
        let all_pinned = [1, 1, 1];

        for triangle in [&ordinary, &mixed, &all_pinned] {
            assert!(view_pin_triangle_visible(
                ViewPinMode::Raw,
                triangle,
                &flags
            ));
        }
        assert!(view_pin_triangle_visible(
            ViewPinMode::Disabled,
            &ordinary,
            &flags
        ));
        assert!(!view_pin_triangle_visible(
            ViewPinMode::Disabled,
            &mixed,
            &flags
        ));
        assert!(!view_pin_triangle_visible(
            ViewPinMode::Disabled,
            &all_pinned,
            &flags
        ));
        assert!(view_pin_triangle_visible(
            ViewPinMode::CameraFacing,
            &ordinary,
            &flags
        ));
        assert!(view_pin_triangle_visible(
            ViewPinMode::CameraFacing,
            &mixed,
            &flags
        ));
        assert!(!view_pin_triangle_visible(
            ViewPinMode::CameraFacing,
            &all_pinned,
            &flags
        ));
        for triangle in [&ordinary, &mixed, &all_pinned] {
            assert!(view_pin_triangle_visible(
                ViewPinMode::WorldSurface,
                triangle,
                &flags
            ));
        }

        assert!(!view_pin_triangle_is_color_underlay(
            ViewPinMode::CameraFacing,
            &ordinary,
            &flags
        ));
        assert!(view_pin_triangle_is_color_underlay(
            ViewPinMode::CameraFacing,
            &mixed,
            &flags
        ));
        assert!(!view_pin_triangle_is_color_underlay(
            ViewPinMode::Raw,
            &mixed,
            &flags
        ));
        assert!(!view_pin_triangle_is_color_underlay(
            ViewPinMode::Disabled,
            &mixed,
            &flags
        ));
        assert!(!view_pin_triangle_is_color_underlay(
            ViewPinMode::WorldSurface,
            &mixed,
            &flags
        ));
        assert!(!view_pin_triangle_is_color_underlay(
            ViewPinMode::WorldSurface,
            &all_pinned,
            &flags
        ));
        assert!(!view_pin_triangle_needs_coplanar_depth(
            ViewPinMode::WorldSurface,
            &ordinary,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(view_pin_triangle_needs_coplanar_depth(
            ViewPinMode::WorldSurface,
            &mixed,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(view_pin_triangle_needs_coplanar_depth(
            ViewPinMode::WorldSurface,
            &all_pinned,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(!view_pin_triangle_needs_coplanar_depth(
            ViewPinMode::CameraFacing,
            &mixed,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(!view_pin_triangle_is_world_surface_decal(
            ViewPinMode::WorldSurface,
            &ordinary,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(!view_pin_triangle_is_world_surface_decal(
            ViewPinMode::WorldSurface,
            &mixed,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(view_pin_triangle_is_world_surface_decal(
            ViewPinMode::WorldSurface,
            &all_pinned,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(!view_pin_triangle_is_world_surface_decal(
            ViewPinMode::CameraFacing,
            &all_pinned,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(triangle_uses_surface_overlay_depth(
            ModelOverlayKind::TerrainSurface,
            ViewPinMode::WorldSurface,
            &ordinary,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(!triangle_uses_surface_overlay_depth(
            ModelOverlayKind::None,
            ViewPinMode::WorldSurface,
            &ordinary,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
        assert!(triangle_uses_surface_overlay_depth(
            ModelOverlayKind::None,
            ViewPinMode::WorldSurface,
            &all_pinned,
            SurfaceVertexSource::Intrinsic(&flags)
        ));
    }

    fn flat_world_surface_terrain(height: i8, sea_y_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_y_raw) << 8, -73, 73, -73, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    #[test]
    fn linked_surface_endpoints_keep_decal_depth_without_relabelling_authored_types() {
        use ModelSurfaceOrigin::{None, ViewPin};
        let triangle = [0, 1, 2];
        // These are the two tf11 imports and one local tf13 of a wing quad.
        // Intrinsic diagnostic geometry still has only one projected corner.
        let authored = SurfaceVertexSource::Intrinsic(&[11, 13, 11]);
        assert!(!view_pin_triangle_is_world_surface_decal(
            ViewPinMode::WorldSurface,
            &triangle,
            authored
        ));
        let resolved = SurfaceVertexSource::Resolved(&[ViewPin, ViewPin, ViewPin]);
        assert!(view_pin_triangle_is_world_surface_decal(
            ViewPinMode::WorldSurface,
            &triangle,
            resolved
        ));
        assert!(view_pin_triangle_needs_coplanar_depth(
            ViewPinMode::WorldSurface,
            &triangle,
            resolved
        ));
        assert!(!view_pin_triangle_is_world_surface_decal(
            ViewPinMode::CameraFacing,
            &triangle,
            resolved
        ));
        // An arithmetic dependent has its own new position, so it must retain
        // the ordinary mixed-support depth slope even when its sources project.
        let mixed = SurfaceVertexSource::Resolved(&[ViewPin, None, ViewPin]);
        assert!(!view_pin_triangle_is_world_surface_decal(
            ViewPinMode::WorldSurface,
            &triangle,
            mixed
        ));
        assert!(view_pin_triangle_needs_coplanar_depth(
            ViewPinMode::WorldSurface,
            &triangle,
            mixed
        ));
    }

    #[test]
    fn world_model_light_is_the_reduced_section10_direction_fixed_to_the_camera() {
        use crate::renderer::retail_world_model_light_direction_raw;
        // 42EA30 divides X and Z by four, rounding toward zero.
        let mut terrain = flat_world_surface_terrain(0, 0);
        assert_eq!(
            retail_world_model_light_direction_raw(&terrain),
            [-18, 73, -18]
        );
        terrain.header[1..4].copy_from_slice(&[75, -40, -3]);
        assert_eq!(
            retail_world_model_light_direction_raw(&terrain),
            [18, -40, 0]
        );

        // 466160 rotates the light through the node's VIEW axes, so one world
        // normal shades differently once the camera turns.
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let up = [0.0, 1.0, 0.0];
        let east = [1.0, 0.0, 0.0];
        assert_eq!(view_light_normal(identity, identity, up), [0.0, 1.0, 0.0]);
        // GL rows look down -Z; retail VIEW Z looks forward.
        assert_eq!(
            view_light_normal(identity, identity, [0.0, 0.0, 1.0])[2],
            -1.0
        );
        let light = [-18, 73, -18];
        let facing_camera = view_light_normal(identity, identity, [0.0, 0.0, 1.0]);
        let yawed = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let facing_after_yaw = view_light_normal(yawed, identity, east);
        assert_eq!(facing_after_yaw, facing_camera);
        assert_ne!(
            retail_model_shade_index(view_light_normal(identity, identity, east), light, 0),
            retail_model_shade_index(facing_after_yaw, light, 0)
        );
        assert_eq!(retail_model_shade_index(facing_camera, light, 0), 1);
    }

    #[test]
    fn world_surface_projection_uses_retail_section10_slopes() {
        let terrain = flat_world_surface_terrain(0, -0x1800);
        let projection = WorldSurfaceProjection::new(&terrain, 123);
        assert_eq!(projection.slope_x_q31, 0x2000_0000);
        assert_eq!(projection.slope_z_q31, 0x2000_0000);

        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let projected = retail_world_surface_vertex(
            [0.0; 3],
            identity,
            [0.0, 2.0, 0.0],
            100.0 / 256.0,
            projection,
        );

        // Source world Y is +512 raw. A +0.25 X/Z slope moves the endpoint
        // +128 raw before the second flat-terrain sample installs Y=0.
        assert_eq!(projected.position_raw, [128.0, -512.0, 128.0]);
        assert_eq!(projected.clip, ModelSlotClip::Clear);
        assert_eq!(projected.surface_origin, ModelSurfaceOrigin::ViewPin);
    }

    #[test]
    fn world_surface_projection_resamples_nonflat_endpoint_height() {
        let mut terrain = flat_world_surface_terrain(0, -0x1800);
        terrain.cells[GRID_SIZE].height = 1;
        terrain.cells[1].height = 2;
        terrain.cells[GRID_SIZE + 1].height = 3;
        let projection = WorldSurfaceProjection::new(&terrain, 0);
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        let projected = retail_world_surface_vertex(
            [0.0; 3],
            identity,
            [0.0, 2.0, 0.0],
            100.0 / 256.0,
            projection,
        );

        // The first sample at (0,0) is zero, so the recovered +0.25 slopes
        // move X/Z to half-cell coordinates (128,128). The endpoint bilinear
        // sample across raw heights 0,32,64,96 is 48; omitting the required
        // second sample would incorrectly leave local Y at -512.
        assert_eq!(projected.position_raw, [128.0, -464.0, 128.0]);
        assert_eq!(projected.clip, ModelSlotClip::Clear);
        assert_eq!(projected.surface_origin, ModelSurfaceOrigin::ViewPin);
    }

    #[test]
    fn world_surface_projection_rejects_the_inclusive_sea_band_only() {
        let terrain = flat_world_surface_terrain(-1, 0);
        let projection = WorldSurfaceProjection::new(&terrain, 0);
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let project_at_raw_y = |y_raw: i32| {
            retail_world_surface_vertex(
                [0.0; 3],
                identity,
                [0.0, y_raw as f32 / 256.0, 0.0],
                100.0 / 256.0,
                projection,
            )
        };

        for y_raw in [-0x96, 0, 0x96] {
            let clipped = project_at_raw_y(y_raw);
            assert_eq!(clipped.clip, ModelSlotClip::SurfaceBand);
            assert_eq!(
                clipped.position_raw, [0.0; 3],
                "clipping retains source XYZ"
            );
        }
        assert_eq!(project_at_raw_y(-0x97).clip, ModelSlotClip::Clear);
        assert_eq!(project_at_raw_y(0x97).clip, ModelSlotClip::Clear);
    }

    #[test]
    fn world_surface_alias_keeps_xz_and_installs_plain_terrain_y() {
        let terrain = flat_world_surface_terrain(2, -0x1800);
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        let grounded = retail_world_surface_alias_vertex(
            [0.0, 512.0, 0.0],
            identity,
            [0.0, 0.5, 0.0],
            100.0 / 256.0,
            &terrain,
        );

        // Height byte 2 samples to raw 64 (world 0.25). Relative to the object
        // origin at world Y=0.5 the grounded corner sits at raw -64 while X/Z
        // stay untouched. No wave surface or sea-band clip applies here.
        assert_eq!(grounded, [0.0, -64.0, 0.0]);
    }

    #[test]
    fn world_surface_alias_samples_each_corner_world_position_through_rotation() {
        let mut terrain = flat_world_surface_terrain(0, -0x1800);
        terrain.cells[GRID_SIZE].height = 8;
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        // Rotate local +X onto world -Z.
        let yawed = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];

        // Identity: the corner one cell +X samples the neighbouring height
        // byte 8 (raw 256) instead of its authored crown height (raw 512).
        assert_eq!(
            retail_world_surface_alias_vertex(
                [256.0, 512.0, 0.0],
                identity,
                [0.0; 3],
                100.0 / 256.0,
                &terrain,
            ),
            [256.0, 256.0, 0.0]
        );
        // Yawed: local +X becomes world -Z, so the sample reads cell (0,255)
        // and the grounded local Y is the plain origin-relative zero.
        assert_eq!(
            retail_world_surface_alias_vertex(
                [256.0, 512.0, 0.0],
                yawed,
                [0.0; 3],
                100.0 / 256.0,
                &terrain,
            ),
            [256.0, 0.0, 0.0]
        );
    }

    #[test]
    fn authored_face_plane_requires_a_strictly_negative_camera_dot() {
        let plane = ModelFaceCullPlane {
            anchor_raw: [0.0; 3],
            native_anchor_raw: None,
            normal_raw: [1, 0, 0],
        };
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert!(authored_face_plane_visible(
            plane,
            identity,
            [-1.0, 0.0, 0.0],
            100.0,
            [0.0; 3]
        ));
        assert!(!authored_face_plane_visible(
            plane, identity, [0.0; 3], 100.0, [0.0; 3]
        ));
        assert!(!authored_face_plane_visible(
            plane,
            identity,
            [1.0, 0.0, 0.0],
            100.0,
            [0.0; 3]
        ));
    }

    #[test]
    fn authored_face_plane_transforms_anchor_normal_scale_and_rotation_together() {
        let plane = ModelFaceCullPlane {
            anchor_raw: [100.0, 0.0, 0.0],
            native_anchor_raw: None,
            normal_raw: [1, 0, 0],
        };
        // Rotate local +X to world -Z. The backend scale contract divides by
        // 100, so scale 0.5 places raw X=100 at world Z=-0.5.
        let rotate_y = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert!(authored_face_plane_visible(
            plane,
            rotate_y,
            [2.0, 0.0, 0.0],
            0.5,
            [2.0, 0.0, -1.0]
        ));
        assert!(!authored_face_plane_visible(
            plane,
            rotate_y,
            [2.0, 0.0, 0.0],
            0.5,
            [2.0, 0.0, -0.25]
        ));
    }

    #[test]
    fn retail_view_pin_preserves_view_xz_and_sets_absolute_view_y() {
        let orientation = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let basis = [[1.0, 0.0, 0.0], [0.0, 0.8, 0.6], [0.0, -0.6, 0.8]];
        let position = [4.0, 7.0, -3.0];
        let camera = [1.0, 2.0, -8.0];
        let model_scale = 0.5;
        let raw = [300.0, 400.0, 500.0];
        let pinned = retail_view_pin_vertex(raw, orientation, position, model_scale, camera, basis);

        let to_world = |local_raw: [f32; 3]| {
            let scale = model_scale / 100.0;
            let local = local_raw.map(|component| component * scale);
            [
                position[0] + dot3(orientation[0], local),
                position[1] + dot3(orientation[1], local),
                position[2] + dot3(orientation[2], local),
            ]
        };
        let to_view = |world: [f32; 3]| {
            let relative = [
                world[0] - camera[0],
                world[1] - camera[1],
                world[2] - camera[2],
            ];
            [
                dot3(relative, basis[0]),
                dot3(relative, basis[1]),
                dot3(relative, basis[2]),
            ]
        };
        let source_view = to_view(to_world(raw));
        let pinned_view = to_view(to_world(pinned));

        assert!((pinned_view[0] - source_view[0]).abs() < 1.0e-4);
        assert!((pinned_view[1] - RETAIL_VIEW_PIN_Y).abs() < 1.0e-4);
        assert!((pinned_view[2] - source_view[2]).abs() < 1.0e-4);
    }

    #[test]
    fn type14_selector_world_points_reexpress_as_raw_local() {
        let orientation = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let position = [10.0, 2.0, 4.0];
        let mut points = [None; 16];
        points[8] = Some([11.0, 2.0, 5.0]);
        let local = apply_external_frame_operands(
            [8.0, 0.0, 0.0],
            ExternalFrameMode::SelectorWorldPoints(points),
            orientation,
            position,
            100.0 / 256.0,
        );
        let scale = (100.0 / 256.0) / 100.0;
        let reconstructed = [
            position[0] + local[0] * scale,
            position[1] + local[1] * scale,
            position[2] + local[2] * scale,
        ];
        assert!((reconstructed[0] - 11.0_f32).abs() < 1.0e-4);
        assert!((reconstructed[1] - 2.0_f32).abs() < 1.0e-4);
        assert!((reconstructed[2] - 5.0_f32).abs() < 1.0e-4);
    }

    #[test]
    fn type14_edge_endpoint_uses_callback_world_without_model_basis() {
        let orientation = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let position = [10.0, 2.0, 4.0];
        let mut points = [None; 16];
        points[6] = Some([11.0, 2.5, 5.0]);
        let vertices = [[6.0_f64, 0.0, 0.0]];
        let flags = [14_i16];
        let resolved = [Some([6.0_f32, 0.0, 0.0])];
        let world = edge_endpoint_world(
            0,
            &vertices,
            &flags,
            &resolved,
            ExternalFrameMode::SelectorWorldPoints(points),
            orientation,
            position,
            (100.0 / 256.0) / 100.0,
        )
        .expect("type-14 edge world");
        assert!((world[0] - 11.0).abs() < 1.0e-5);
        assert!((world[1] - 2.5).abs() < 1.0e-5);
        assert!((world[2] - 5.0).abs() < 1.0e-5);
    }

    #[test]
    fn external_frame_world_point_bypasses_model_translation_rotation_and_scale() {
        let orientation = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        let position = [4.0, 7.0, -3.0];
        let model_scale = 0.5;
        let expected_world = [5.0, 2.0, 7.0];
        let local_raw =
            world_point_as_raw_local(expected_world, orientation, position, model_scale);
        let local = local_raw.map(|component| component * model_scale / 100.0);
        let reconstructed = [
            position[0] + dot3(orientation[0], local),
            position[1] + dot3(orientation[1], local),
            position[2] + dot3(orientation[2], local),
        ];
        for (actual, expected) in reconstructed.into_iter().zip(expected_world) {
            assert!((actual - expected).abs() < 1.0e-5);
        }
    }

    #[test]
    fn only_masked_billboards_establish_depth() {
        assert!(billboard_writes_depth(WorldSpriteBlend::Masked));
        assert!(!billboard_writes_depth(WorldSpriteBlend::HalfAdditive));
        assert!(!billboard_writes_depth(WorldSpriteBlend::Additive));
    }

    #[test]
    fn model_face_blend_uses_exact_destination_half_and_underlay_depth_policy() {
        assert_eq!(HALF_ADDITIVE_ALPHA, 0.5);
        assert_eq!(model_face_opacity(WorldSpriteBlend::Masked), 1.0);
        assert_eq!(model_face_opacity(WorldSpriteBlend::Additive), 1.0);
        assert_eq!(
            model_face_opacity(WorldSpriteBlend::HalfAdditive),
            HALF_ADDITIVE_ALPHA
        );
        assert_eq!(
            1.0 - model_face_opacity(WorldSpriteBlend::HalfAdditive),
            0.5
        );

        assert!(model_face_writes_depth(WorldSpriteBlend::Masked, false));
        assert!(!model_face_writes_depth(WorldSpriteBlend::Masked, true));
        assert!(!model_face_writes_depth(
            WorldSpriteBlend::HalfAdditive,
            false
        ));
        assert!(!model_face_writes_depth(WorldSpriteBlend::Additive, false));
    }

    #[test]
    fn face_families_select_fixed_or_normal_derived_palette_rows() {
        assert_eq!(model_face_palette_row(ModelFaceShading::Flat, None, 28), 28);
        assert_eq!(
            model_face_palette_row(ModelFaceShading::Flat, Some(7), 31),
            31
        );
        assert_eq!(
            model_face_palette_row(ModelFaceShading::FlatLit, Some(7), 28),
            7
        );
        assert_eq!(
            model_face_palette_row(ModelFaceShading::Gouraud, Some(7), 28),
            7
        );
        assert_eq!(
            model_face_palette_row(ModelFaceShading::Gouraud, None, 28),
            31
        );
    }

    #[test]
    fn lit_flat_faces_keep_one_light_normal_and_zero_references_stay_zero() {
        let normal = [0.0, 0.0, 1.0];
        let corners = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert_eq!(
            model_lighting_normals(ModelFaceShading::Flat, normal, corners),
            None
        );
        assert_eq!(
            model_lighting_normals(ModelFaceShading::FlatLit, normal, corners),
            Some([normal; 3])
        );
        assert_eq!(
            model_lighting_normals(ModelFaceShading::Gouraud, normal, corners),
            Some(corners)
        );
        let quarter_y = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
        assert_eq!(
            transform_model_light_normal(quarter_y, normal),
            [1.0, 0.0, 0.0]
        );
        let zero = transform_model_light_normal(quarter_y, [0.0; 3]);
        assert_eq!(zero, [0.0; 3]);
        assert_eq!(retail_model_shade_index(zero, [73, 73, -73], 0), 0);
    }

    #[test]
    fn uniform_solid_light_adds_to_the_packed_palette_before_scene_tint() {
        // The gun/engine base is black: multiplication would leave it black.
        assert_eq!(retail_flat_lit_rgb565(0, [128; 3]), [64; 3]);
        assert_eq!(retail_flat_lit_rgb565(0x3000, [128; 3]), [160, 64, 64]);
        assert_eq!(retail_flat_lit_rgb565(0x3000, [15; 3]), [96, 0, 0]);
        // Preserve the filler's word arithmetic, including carries into the
        // sixth green bit; per-channel float saturation is not equivalent.
        assert_eq!(retail_flat_lit_rgb565(0x001f, [0, 0, 16]), [0, 4, 0]);
        assert_eq!(retail_flat_lit_rgb565(0x7fff, [128; 3]), [64, 60, 56]);

        let mut material = crate::renderer::FaceMaterial {
            color: [0.5, 1.0, 1.0],
            palette_rgb555: Some(0x3000),
            emissive: [0.0; 3],
            texture: None,
            blend: crate::renderer::WorldSpriteBlend::Masked,
            flat_shade_row: 0,
        };
        let shade = v2k_formats::system::FogGradientEntry {
            r: 128,
            g: 128,
            b: 128,
            shade_level: 31,
        };
        assert_eq!(material.unlit_color(), [48.0 / 255.0, 0.0, 0.0]);
        assert_eq!(
            flat_lit_palette_color(&material, &shade),
            Some([80.0 / 255.0, 64.0 / 255.0, 64.0 / 255.0])
        );
        material.palette_rgb555 = None;
        assert_eq!(material.unlit_color(), material.color);
        assert!(flat_lit_palette_color(&material, &shade).is_none());
    }

    #[test]
    fn fixed_model_row_decode_preserves_palette_and_zero_key_policy() {
        let mut palette = vec![0u8; 32 * 16 * 4];
        let color_offset = |row: usize, color: usize| (row * 16 + color) * 4;
        palette[color_offset(28, 0)..color_offset(28, 0) + 4].copy_from_slice(&[10, 20, 30, 255]);
        palette[color_offset(28, 1)..color_offset(28, 1) + 4].copy_from_slice(&[40, 50, 60, 254]);
        palette[color_offset(28, 15)..color_offset(28, 15) + 4].copy_from_slice(&[70, 80, 90, 253]);

        let indices = [0, 1, 15];
        assert_eq!(
            indexed_model_rgba_at_row(&indices, &palette, 3, 28, true).unwrap(),
            [0, 0, 0, 0, 40, 50, 60, 254, 70, 80, 90, 253]
        );
        assert_eq!(
            indexed_model_rgba_at_row(&indices, &palette, 3, 28, false).unwrap(),
            [10, 20, 30, 255, 40, 50, 60, 254, 70, 80, 90, 253]
        );
        assert!(indexed_model_rgba_at_row(&[16], &palette, 1, 28, false).is_none());
    }

    #[test]
    fn fixed_function_uses_authored_flat_rows_and_keeps_gouraud_fallback() {
        let fallback = TextureId(1);
        let indexed = IndexedModelTextures {
            index: TextureId(2),
            palette: TextureId(3),
            flat_rows: [Some(TextureId(5)), Some(TextureId(4))],
            transparent_zero: true,
        };
        assert_eq!(
            fixed_function_model_texture(
                fallback,
                Some(&indexed),
                ModelFaceShading::Flat,
                MODEL_FIXED_SHADE_ROW,
            ),
            TextureId(4)
        );
        assert_eq!(
            fixed_function_model_texture(fallback, Some(&indexed), ModelFaceShading::FlatLit, 28),
            fallback
        );
        assert_eq!(
            fixed_function_model_texture(fallback, Some(&indexed), ModelFaceShading::Flat, 0),
            TextureId(5)
        );
        assert_eq!(
            fixed_function_model_texture(fallback, Some(&indexed), ModelFaceShading::Flat, 31,),
            fallback
        );
        assert_eq!(
            fixed_function_model_texture(
                fallback,
                Some(&indexed),
                ModelFaceShading::Gouraud,
                MODEL_FIXED_SHADE_ROW,
            ),
            fallback
        );
        assert_eq!(
            fixed_function_model_texture(
                fallback,
                None,
                ModelFaceShading::Flat,
                MODEL_FIXED_SHADE_ROW,
            ),
            fallback
        );
    }

    #[test]
    fn world_sprites_use_stable_retail_painter_keys_across_materials() {
        let sprite = |z, sort_key_raw, blend| WorldSprite {
            texture: TextureId(1),
            position: [0.0, 0.0, z],
            size: [1.0; 2],
            rotation: 0.0,
            color: [1.0; 4],
            sort_key_raw,
            blend,
            flat_shade_row: 28,
            fog: crate::renderer::SpriteFog::Near,
            native: None,
        };
        let sprites = [
            sprite(100.0, 10, WorldSpriteBlend::HalfAdditive),
            sprite(1.0, 30, WorldSpriteBlend::Masked),
            sprite(2.0, 20, WorldSpriteBlend::Additive),
            sprite(200.0, 20, WorldSpriteBlend::Masked),
        ];
        assert_eq!(world_sprite_draw_order(&sprites), vec![1, 2, 3, 0]);
    }
    #[test]
    fn native_world_node_pass_bypasses_float_reselection_for_body_edges_and_billboards() {
        use crate::renderer::{NativeModelFogPass, WorldModelFog};
        let fog = WorldModelFog {
            planes: v2k_formats::levels::LevelFogPlanes {
                near_raw: 3072,
                far_raw: 6144,
            },
            color: [0.2, 0.3, 0.4],
        };
        for (pass, expected, enabled) in [
            (
                NativeModelFogPass::Near,
                crate::model_fog::PaletteFogPass::Near,
                false,
            ),
            (
                NativeModelFogPass::Fog,
                crate::model_fog::PaletteFogPass::Far,
                true,
            ),
        ] {
            let policy = ModelDepthFade::WorldRaw { fog, pass };
            assert_eq!(super::selected_model_fog_pass(policy), Some(expected));
            let resolved = resolve_model_depth_fade(policy, true, -1.0, 0.0, [0.9; 3]);
            assert_eq!(
                (
                    resolved.enabled,
                    resolved.near,
                    resolved.far,
                    resolved.color
                ),
                (enabled, 12.0, 24.0, fog.color)
            );
            assert_eq!(
                resolved.fixed_point_domain, None,
                "parent planes alone do not own active projector reciprocal/cache bytes"
            );
        }
        assert_eq!(
            super::selected_model_fog_pass(ModelDepthFade::InheritWorldFog),
            None
        );
    }
}
