//! Retail model near/far table selection and indexed shade accumulators.
//!
//! Packed RGB565 carry masks and the 16-bin fog-colour LUT are still
//! approximated by the GL shader's RGB floats. Source palette selection,
//! primitive pass selection and constant half-additive opacity remain separate.

use v2k_formats::models::ModelFaceShading;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaletteFogPass {
    Near,
    Far,
}

impl PaletteFogPass {
    pub(crate) fn shader_enabled(self) -> f32 {
        match self {
            Self::Near => 0.0,
            Self::Far => 1.0,
        }
    }
}

/// FUN_00464E60 selects one constructor table for the complete node. Its
/// children repeat this decision with their own origin and authored radius.
/// Disabled planes include the model header's explicit 0x20 near-table rule.
/// Its separate far-radius whole-node rejection belongs at hierarchy traversal;
/// rejecting only the body here would still draw descendants and billboards.
pub(crate) fn model_fog_pass(
    origin_depth: f32,
    radius: f32,
    planes: Option<[f32; 2]>,
) -> PaletteFogPass {
    let Some([near, _far]) = planes else {
        return PaletteFogPass::Near;
    };
    if near <= origin_depth + radius {
        PaletteFogPass::Far
    } else {
        PaletteFogPass::Near
    }
}

/// Far unlit 47C7C0/47F060 always use shade 28, even when all corner fade
/// bytes are zero. 47F480/47F9E0 retain the face/corner Section-6 shade byte.
/// Their fixed accumulator is (((255-f)*s)+128)<<8. The shader already adds
/// [0,1] dither (the half-row bias plus signed noise), so return Q/65536-0.5.
pub(crate) fn palette_fog_shade(
    shading: ModelFaceShading,
    near_row: u8,
    pass: PaletteFogPass,
    fade_byte: u8,
) -> f32 {
    match pass {
        PaletteFogPass::Near => f32::from(near_row.min(31)),
        PaletteFogPass::Far => {
            let base = if shading == ModelFaceShading::Flat {
                28
            } else {
                near_row.min(31)
            };
            f32::from(u16::from(255 - fade_byte) * u16::from(base)) / 256.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_radius_selects_one_table_and_retains_boundary_equality() {
        let planes = Some([13.0, 21.0]);
        assert_eq!(model_fog_pass(10.0, 2.0, planes), PaletteFogPass::Near);
        assert_eq!(model_fog_pass(10.0, 3.0, planes), PaletteFogPass::Far);
        assert_eq!(model_fog_pass(22.0, 2.0, planes), PaletteFogPass::Far);
        assert_eq!(model_fog_pass(23.0, 2.0, planes), PaletteFogPass::Far);
        assert_eq!(model_fog_pass(100.0, 2.0, None), PaletteFogPass::Near);
    }

    #[test]
    fn far_unlit_uses_row_28_even_for_zero_fade_corners_and_near_row_zero() {
        assert_eq!(
            palette_fog_shade(ModelFaceShading::Flat, 0, PaletteFogPass::Near, 0),
            0.0
        );
        // 0x1C6400 is the exact unlit fixed accumulator before signed noise.
        let row = palette_fog_shade(ModelFaceShading::Flat, 0, PaletteFogPass::Far, 0);
        assert_eq!((row + 0.5) * 65536.0, 0x1c6400 as f32);
        assert_eq!(
            palette_fog_shade(ModelFaceShading::Flat, 28, PaletteFogPass::Far, 128),
            13.890625
        );
        assert_eq!(
            palette_fog_shade(ModelFaceShading::Flat, 28, PaletteFogPass::Far, 255),
            0.0
        );
        for shading in [ModelFaceShading::FlatLit, ModelFaceShading::Gouraud] {
            assert_eq!(
                palette_fog_shade(shading, 12, PaletteFogPass::Far, 128),
                5.953125
            );
            assert_eq!(
                palette_fog_shade(shading, 12, PaletteFogPass::Near, 128),
                12.0
            );
        }
    }
}
