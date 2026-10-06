//! Classic framebuffer RGB565 presentation adapter.
//!
//! Retail's retained DirectDraw surface uses F800/07E0/001F masks. The port
//! still composites draws in RGBA8; quantizing the completed logical pixels
//! is a replaceable approximation of its packed per-draw blend/fog arithmetic.
//! Quantization precedes the single filtered enlargement, including readback.

#[derive(Debug)]
pub(super) struct ClassicColorProgram {
    pub id: u32,
    pub source_size: i32,
    pub source_sampler: i32,
}

impl ClassicColorProgram {
    pub unsafe fn create() -> Result<Self, String> {
        let vertex = super::compile_shader(
            crate::gl::VERTEX_SHADER,
            "#version 120\nvoid main() { gl_Position = gl_Vertex; gl_TexCoord[0] = gl_MultiTexCoord0; }",
        )?;
        let fragment = match super::compile_shader(crate::gl::FRAGMENT_SHADER, FRAGMENT) {
            Ok(fragment) => fragment,
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
            let mut log = vec![0_u8; len.max(1) as usize];
            crate::gl::GetProgramInfoLog(id, len, std::ptr::null_mut(), log.as_mut_ptr().cast());
            crate::gl::DeleteProgram(id);
            return Err(String::from_utf8_lossy(&log)
                .trim_end_matches('\0')
                .to_owned());
        }
        Ok(Self {
            id,
            source_size: crate::gl::GetUniformLocation(id, c"source_size".as_ptr()),
            source_sampler: crate::gl::GetUniformLocation(id, c"source_tex".as_ptr()),
        })
    }
}

const FRAGMENT: &str = r#"#version 120
uniform sampler2D source_tex;
uniform vec2 source_size;

vec3 rgb565_pixel(vec2 uv) {
    vec3 bytes = floor(texture2D(source_tex, uv).rgb * 255.0 + 0.5);
    vec3 steps = vec3(8.0, 4.0, 8.0);
    // Keep the high bits, as the retail RGB555/565 display-word decoders do.
    // Do not normalize the authored 248 maximum back up to 255.
    return floor(bytes / steps) * steps / 255.0;
}

void main() {
    vec2 pixel = gl_TexCoord[0].xy * source_size - 0.5;
    vec2 base = floor(pixel);
    vec2 fraction = fract(pixel);
    vec2 uv = (base + 0.5) / source_size;
    vec2 step_uv = 1.0 / source_size;
    // Point-read and quantize all four source pixels BEFORE bilinear scaling.
    vec3 a = rgb565_pixel(uv);
    vec3 b = rgb565_pixel(uv + vec2(step_uv.x, 0.0));
    vec3 c = rgb565_pixel(uv + vec2(0.0, step_uv.y));
    vec3 d = rgb565_pixel(uv + step_uv);
    gl_FragColor = vec4(mix(mix(a, b, fraction.x), mix(c, d, fraction.x), fraction.y), 1.0);
}
"#;

pub(super) fn quantize_readback(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] &= 0xf8;
        pixel[1] &= 0xfc;
        pixel[2] &= 0xf8;
    }
}

#[cfg(test)]
mod tests {
    use super::quantize_readback;

    #[test]
    fn display_words_preserve_authored_colors_and_six_bit_green() {
        let mut colors = [
            0, 128, 160, 255, 255, 255, 255, 255, 7, 7, 7, 123, 8, 4, 8, 0,
        ];
        quantize_readback(&mut colors);
        assert_eq!(
            colors,
            [0, 128, 160, 255, 248, 252, 248, 255, 0, 4, 0, 123, 8, 4, 8, 0]
        );
        let first = colors;
        quantize_readback(&mut colors);
        assert_eq!(colors, first, "pause-frame replay must be idempotent");
    }
}
