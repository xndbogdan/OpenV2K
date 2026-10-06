//! Shared submission of resolved camera-facing quads. Model traversal and
//! particle admission keep their own owners; only geometry/material data meet
//! here, at the near AA20 / far AA40 indexed sprite filler boundary.

use super::*;

pub(super) struct SpriteQuad {
    pub corners: [[f32; 3]; 4],
    pub material: FaceMaterial,
    pub opacity: f32,
    pub fog: SpriteFog,
    pub fog_color: [f32; 3],
}

/// A67C60/B68520 use the owning node's constructor table, then repeat the
/// projected centre byte over the quad. A zero centre byte in a far node
/// still selects F060's row-28 accumulator. B rejects terminal fade.
pub(super) fn model_billboard_fog(
    center_depth: f32,
    pass: PaletteFogPass,
    fade: ResolvedModelDepthFade,
) -> Option<SpriteFog> {
    // Keep projection/near clipping at its existing owner. Retail's 0x40
    // projector guard needs a coordinate domain independent of fog enablement;
    // inferring it from a disabled fade would confuse frontend and world units.
    if !center_depth.is_finite() {
        return None;
    }
    if pass == PaletteFogPass::Near {
        return Some(SpriteFog::Near);
    }
    let (depth, near, far) = match fade.fixed_point_domain {
        Some(domain) => (
            (f64::from(center_depth) * f64::from(FRONTEND_MODEL_UNITS_PER_VIEW)).round() as i64,
            i64::from(domain.near_raw),
            i64::from(domain.far_raw),
        ),
        None => (
            (f64::from(center_depth) * 256.0).round() as i64,
            (f64::from(fade.near) * 256.0).round() as i64,
            (f64::from(fade.far) * 256.0).round() as i64,
        ),
    };
    let fade_byte = retail_fixed_fade_byte(depth, near, far);
    (fade_byte != 255).then_some(SpriteFog::Far { fade_byte })
}

pub(super) fn camera_facing_corners(
    center: [f32; 3],
    half_extent: [f32; 2],
    angle: f32,
    basis: [[f32; 3]; 3],
) -> [[f32; 3]; 4] {
    let (sin, cos) = angle.sin_cos();
    let axis_x: [f32; 3] = std::array::from_fn(|i| basis[0][i] * cos + basis[1][i] * sin);
    let axis_y: [f32; 3] = std::array::from_fn(|i| basis[1][i] * cos - basis[0][i] * sin);
    [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|sign| {
        std::array::from_fn(|i| {
            center[i] + axis_x[i] * half_extent[0] * sign[0] + axis_y[i] * half_extent[1] * sign[1]
        })
    })
}

impl GlRenderer {
    pub(super) fn submit_sprite_quads(
        &mut self,
        quads: &[SpriteQuad],
        depth_policy: ModelDepthPolicy,
    ) {
        let group_depth = match depth_policy {
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
        if quads.is_empty() {
            return;
        }
        unsafe {
            // Preserve scene fog, including its explicit-coordinate source on
            // shaderless drivers. No sprite may leave local planes behind.
            let painter = depth_policy != ModelDepthPolicy::Geometry;
            let depth_state = painter.then(|| PainterDepthState::capture());
            crate::gl::PushAttrib(crate::gl::FOG_BIT);
            crate::gl::Disable(crate::gl::FOG);
            if depth_policy == ModelDepthPolicy::Painter {
                crate::gl::Disable(crate::gl::DEPTH_TEST);
            } else {
                crate::gl::Enable(crate::gl::DEPTH_TEST);
                crate::gl::DepthFunc(crate::gl::LEQUAL);
                if let Some(depth) = group_depth {
                    crate::gl::DepthRange(depth, depth);
                }
            }
            crate::gl::Disable(crate::gl::CULL_FACE);
            if let Some(program) = &self.model_program {
                crate::gl::UseProgram(program.id);
                crate::gl::Uniform1i(program.index_sampler, 0);
                crate::gl::Uniform1i(program.palette_sampler, 1);
                crate::gl::Uniform1f(program.gouraud_solid, 0.0);
                crate::gl::Uniform1f(program.use_explicit_fog_coord, 1.0);
            }
            for quad in quads {
                let material = quad.material;
                let (pass, fade_byte) = match quad.fog {
                    SpriteFog::Near => (PaletteFogPass::Near, 0),
                    SpriteFog::Far { fade_byte } => (PaletteFogPass::Far, fade_byte),
                };
                let fade_amount = f32::from(fade_byte) / 255.0;
                let indexed = material
                    .texture
                    .and_then(|id| self.indexed_model_textures.get(&id.0));
                let blend = material.blend;
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
                crate::gl::DepthMask(if billboard_writes_depth(blend) {
                    crate::gl::TRUE
                } else {
                    crate::gl::FALSE
                });
                let mut color = material.unlit_color();
                for (channel, emissive) in color.iter_mut().zip(material.emissive) {
                    *channel = (*channel + emissive).min(1.0);
                }
                // Indexed textures use the authored palette below. Direct-RGB
                // textures and shaderless drivers keep an explicit RGB fallback;
                // additive source attenuation must never add the fog colour.
                let palette_shader = indexed.is_some() && self.model_program.is_some();
                let rgb_fog = !palette_shader && blend != WorldSpriteBlend::Additive;
                if !palette_shader && blend == WorldSpriteBlend::Additive {
                    color = color.map(|value| value * (1.0 - fade_amount));
                }
                let alpha;
                if let Some(program) = &self.model_program {
                    crate::gl::Uniform1f(program.opacity, quad.opacity);
                    crate::gl::Uniform1f(
                        program.palette_fog,
                        if palette_shader {
                            pass.shader_enabled()
                        } else {
                            0.0
                        },
                    );
                    crate::gl::Uniform1f(program.blend_mode, model_blend_uniform(blend));
                    crate::gl::Uniform1f(
                        program.fog_enabled,
                        if palette_shader || rgb_fog { 1.0 } else { 0.0 },
                    );
                    crate::gl::Uniform3f(
                        program.fog_color,
                        quad.fog_color[0],
                        quad.fog_color[1],
                        quad.fog_color[2],
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
                    } else {
                        crate::gl::ActiveTexture(crate::gl::TEXTURE1);
                        crate::gl::Disable(crate::gl::TEXTURE_2D);
                        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
                        crate::gl::Uniform1f(program.mask_zero, 0.0);
                        crate::gl::Uniform1f(
                            program.mode,
                            if material.texture.is_some() { 2.0 } else { 0.0 },
                        );
                        if let Some(texture) = material.texture {
                            crate::gl::Enable(crate::gl::TEXTURE_2D);
                            crate::gl::BindTexture(crate::gl::TEXTURE_2D, texture.0);
                        } else {
                            crate::gl::Disable(crate::gl::TEXTURE_2D);
                        }
                    }
                    alpha = palette_fog_shade(
                        ModelFaceShading::Flat,
                        material.flat_shade_row,
                        pass,
                        fade_byte,
                    ) / 31.0;
                } else {
                    if let Some(program) = &self.compatibility_program {
                        activate_compatibility_program(
                            program,
                            material.texture.is_some(),
                            rgb_fog,
                            0.0,
                            1.0,
                            quad.fog_color,
                        );
                        crate::gl::Uniform1f(program.use_explicit_fog_coord, 1.0);
                    } else {
                        crate::gl::Fogi(crate::gl::FOG_COORD_SRC, crate::gl::FOG_COORD as i32);
                        crate::gl::Fogi(crate::gl::FOG_MODE, crate::gl::LINEAR as i32);
                        crate::gl::Fogf(crate::gl::FOG_START, 0.0);
                        crate::gl::Fogf(crate::gl::FOG_END, 1.0);
                        let fog_color =
                            [quad.fog_color[0], quad.fog_color[1], quad.fog_color[2], 1.0];
                        crate::gl::Fogfv(crate::gl::FOG_COLOR, fog_color.as_ptr());
                        if rgb_fog {
                            crate::gl::Enable(crate::gl::FOG);
                        } else {
                            crate::gl::Disable(crate::gl::FOG);
                        }
                    }
                    if let Some(texture) = material.texture {
                        let row = if pass == PaletteFogPass::Far {
                            28
                        } else {
                            material.flat_shade_row
                        };
                        let texture = fixed_function_model_texture(
                            texture,
                            indexed,
                            ModelFaceShading::Flat,
                            row,
                        );
                        crate::gl::Enable(crate::gl::TEXTURE_2D);
                        crate::gl::BindTexture(crate::gl::TEXTURE_2D, texture.0);
                        crate::gl::Enable(crate::gl::ALPHA_TEST);
                        crate::gl::AlphaFunc(
                            crate::gl::GREATER,
                            if blend == WorldSpriteBlend::HalfAdditive {
                                0.0
                            } else {
                                0.5
                            },
                        );
                    } else {
                        crate::gl::Disable(crate::gl::TEXTURE_2D);
                        crate::gl::Disable(crate::gl::ALPHA_TEST);
                    }
                    alpha = quad.opacity;
                }
                crate::gl::FogCoordf(fade_amount);
                crate::gl::Color4f(color[0], color[1], color[2], alpha);
                crate::gl::Begin(crate::gl::QUADS);
                for (point, uv) in
                    quad.corners
                        .iter()
                        .zip([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
                {
                    crate::gl::TexCoord2f(uv[0], uv[1]);
                    crate::gl::Vertex3f(point[0], point[1], point[2]);
                }
                crate::gl::End();
            }
            crate::gl::UseProgram(0);
            crate::gl::ActiveTexture(crate::gl::TEXTURE1);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::ActiveTexture(crate::gl::TEXTURE0);
            crate::gl::Disable(crate::gl::TEXTURE_2D);
            crate::gl::Disable(crate::gl::ALPHA_TEST);
            crate::gl::Disable(crate::gl::BLEND);
            crate::gl::Color4f(1.0, 1.0, 1.0, 1.0);
            crate::gl::FogCoordf(0.0);
            crate::gl::DepthMask(crate::gl::TRUE);
            crate::gl::DepthFunc(crate::gl::LEQUAL);
            crate::gl::Enable(crate::gl::CULL_FACE);
            crate::gl::PopAttrib();
            if let Some(state) = depth_state {
                state.restore();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billboard_table_selection_is_independent_of_its_center_byte() {
        let fade =
            resolve_model_depth_fade(ModelDepthFade::InheritWorldFog, true, 13.0, 21.0, [0.0; 3]);
        assert_eq!(
            model_billboard_fog(12.0, PaletteFogPass::Far, fade),
            Some(SpriteFog::Far { fade_byte: 0 })
        );
        assert_eq!(
            model_billboard_fog(17.0, PaletteFogPass::Far, fade),
            Some(SpriteFog::Far { fade_byte: 128 })
        );
        assert_eq!(model_billboard_fog(21.0, PaletteFogPass::Far, fade), None);
        assert_eq!(
            model_billboard_fog(21.0, PaletteFogPass::Near, fade),
            Some(SpriteFog::Near)
        );
    }

    #[test]
    fn center_fade_retains_reciprocal_truncation_and_frontend_units() {
        let fade = resolve_model_depth_fade(
            ModelDepthFade::Linear {
                near: 12.0,
                far: 24.0,
                color: [0.0; 3],
            },
            false,
            0.0,
            0.0,
            [0.0; 3],
        );
        assert_eq!(
            model_billboard_fog(18.0, PaletteFogPass::Far, fade),
            Some(SpriteFog::Far { fade_byte: 127 })
        );
        let frontend = resolve_model_depth_fade(
            ModelDepthFade::FrontendFixedPointLinear {
                near_raw: 0xc00,
                far_raw: 0x1800,
                color: [0.0; 3],
            },
            false,
            0.0,
            0.0,
            [0.0; 3],
        );
        assert_eq!(
            model_billboard_fog(46.08, PaletteFogPass::Far, frontend),
            Some(SpriteFog::Far { fade_byte: 127 })
        );
    }
}
