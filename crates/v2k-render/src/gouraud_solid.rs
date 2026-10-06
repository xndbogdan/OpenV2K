//! Solid 23/24 corner-colour setup, before screen-linear interpolation.
//!
//! A23/24 (47C150/47E930) and B23/24 (47C2F0/47EB40) feed the same
//! RGB span 474220. Their initial accumulators deliberately differ. Keep
//! these in channel units until raster time: packing the vertices first
//! loses native colour thresholds and carries within a polygon.

use crate::model_fog::PaletteFogPass;

/// Channel accumulators divided by 65536, including the near-path bias.
pub(crate) fn corner_channels(
    palette_rgb555: u16,
    shade_rgb: [u8; 3],
    pass: PaletteFogPass,
    fade: u8,
    fog_rgb: [f32; 3],
) -> [f32; 3] {
    if pass == PaletteFogPass::Near {
        return shade_rgb.map(|value| f32::from(value) + 0.5);
    }

    // B setup unpacks the relocated display words into five high colour
    // bits, even for RGB565 green. The sixth green bit is not part of this
    // corner setup. Both the base and fog colours use this extraction.
    let base = [
        ((palette_rgb555 >> 10) & 31) as i32 * 8,
        ((palette_rgb555 >> 5) & 31) as i32 * 8,
        (palette_rgb555 & 31) as i32 * 8,
    ];
    let fog = fog_rgb.map(|value| ((value.clamp(0.0, 1.0) * 255.0).round() as i32) & 0xf8);
    let fade = i32::from(fade) + i32::from(fade >> 7);
    std::array::from_fn(|axis| {
        let source = base[axis] + i32::from(shade_rgb[axis]);
        // 47C2F0: ((source << 8) + (fog-source)*fade) << 9.
        // Dividing by 65536 leaves an exact binary fraction /128.
        ((source << 8) + (fog[axis] - source) * fade) as f32 / 128.0
    })
}

/// RGB565 span packing after interpolation (474220, shifts 7/2/4).
/// This also supplies the no-shader fallback's corner colours; that fallback
/// cannot reproduce the fragment-stage quantization of the model shader.
pub(crate) fn packed_rgb(channels: [f32; 3]) -> [u8; 3] {
    let [r, g, b] = channels.map(|value| (value.floor() as u32) & 0xff0);
    let packed = ((r << 7) + (g << 2) + (b >> 4)) as u16;
    [
        ((packed >> 11) as u8) << 3,
        (((packed >> 5) & 63) as u8) << 2,
        ((packed & 31) as u8) << 3,
    ]
}

#[cfg(test)]
mod tests {
    use super::{corner_channels, packed_rgb};
    use crate::model_fog::PaletteFogPass::{Far, Near};

    #[test]
    fn near_ignores_base_and_packs_section6_channels() {
        for base in [0, 0x1234, 0x7fff] {
            let channels = corner_channels(base, [128, 96, 64], Near, 255, [1.0; 3]);
            assert_eq!(channels, [128.5, 96.5, 64.5]);
            assert_eq!(packed_rgb(channels), [64, 48, 32]);
        }
    }

    #[test]
    fn far_setup_retains_its_distinct_base_light_and_endpoint_rules() {
        let base = (3 << 10) | (5 << 5) | 7;
        let fog = [160.0 / 255.0, 96.0 / 255.0, 32.0 / 255.0];
        assert_eq!(
            corner_channels(base, [128, 96, 64], Far, 0, fog),
            [304.0, 272.0, 240.0]
        );
        assert_eq!(
            corner_channels(base, [128, 96, 64], Far, 127, fog),
            [311.9375, 232.3125, 152.6875]
        );
        assert_eq!(
            corner_channels(base, [128, 96, 64], Far, 128, fog),
            [312.0625, 231.6875, 151.3125]
        );
        assert_eq!(
            packed_rgb(corner_channels(base, [128, 96, 64], Far, 255, fog)),
            [160, 96, 32]
        );
        assert_eq!(
            packed_rgb(corner_channels(0, [128; 3], Far, 0, fog)),
            [128; 3]
        );
    }

    #[test]
    fn pack_after_interpolation_preserves_thresholds_and_channel_carries() {
        // Halfway between these native corner accumulators crosses a packed
        // colour threshold, which interpolating packed corner RGB misses.
        let a = [15.5, 31.5, 47.5];
        let b = [16.5, 32.5, 48.5];
        let midpoint = std::array::from_fn(|axis| (a[axis] + b[axis]) * 0.5);
        assert_eq!(packed_rgb(midpoint), [8, 16, 24]);
        assert_eq!(packed_rgb(a), [0, 8, 16]);
        assert_eq!(packed_rgb(b), [8, 16, 24]);
        // Native WORD addition lets an oversized blue channel carry into
        // green; independent normalized-channel clamping would lose it.
        assert_eq!(packed_rgb([0.0, 0.0, 512.0]), [0, 4, 0]);
        assert_eq!(packed_rgb([512.0, 0.0, 0.0]), [0, 0, 0]);
    }
}
