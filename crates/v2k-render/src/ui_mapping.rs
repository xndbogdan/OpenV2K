//! Map an authored 2-D canvas without changing its intrinsic asset metrics.

use crate::config::ScalingMode;

/// A gameplay group's relationship to the drawable, independent of its art tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAnchor {
    BottomLeft,
    BottomRight,
    TopLeft,
}

/// Presentation choice supplied explicitly by the display configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum UiSubmissionPolicy {
    /// High Native grows complete UI compositions with the readability scale,
    /// bounded by the authored canvas fit. Original presets keep native pixels.
    NativeCanvas,
    /// Gameplay keeps the original pixel metrics through 1024x768, then grows
    /// toward the 800x600 HUD density with independently anchored groups.
    NativeHud { anchor: UiAnchor },
    /// Low and 4:3/Stretched retain the uniformly fitted authored canvas.
    #[default]
    FitAuthoredCanvas,
}

impl UiSubmissionPolicy {
    /// High Native gameplay has a distinct readability policy from frontend
    /// menus. Low and fitted presentation keep their existing canvas mapping.
    pub const fn for_gameplay_hud(self, anchor: UiAnchor) -> Self {
        match self {
            Self::NativeCanvas | Self::NativeHud { .. } => Self::NativeHud { anchor },
            Self::FitAuthoredCanvas => Self::FitAuthoredCanvas,
        }
    }

    /// Screen-edge branding grows independently of the bounded frontend canvas
    /// above the original resolutions. Low Native and fitted presentation keep
    /// their existing enlargement paths.
    pub fn authored_pixel_scale(
        self,
        scaling: ScalingMode,
        output_size: [u32; 2],
        reference_height: u32,
    ) -> f32 {
        match (self, scaling) {
            (Self::FitAuthoredCanvas, ScalingMode::Native) => {
                output_size[1] as f32 / reference_height.max(1) as f32
            }
            (_, ScalingMode::Native) => {
                let effective_height = (output_size[1] as f32).min(output_size[0] as f32 * 0.75);
                if effective_height <= 768.0 {
                    1.0
                } else {
                    native_hud_comfort_scale(effective_height)
                }
            }
            _ => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiMappingRequest {
    /// Logical renderer pixels; physical viewport enlargement is separate.
    pub viewport: [u32; 2],
    /// Selected System2/3 authored layout dimensions, independent of glyph size.
    pub authored_canvas: [u32; 2],
    pub policy: UiSubmissionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiMapping {
    pub scale: f32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl UiMapping {
    pub fn new(request: UiMappingRequest) -> Self {
        let [width, height] = request.viewport.map(|value| value as f32);
        let [canvas_width, canvas_height] =
            request.authored_canvas.map(|value| value.max(1) as f32);
        let fit = (width / canvas_width).min(height / canvas_height);
        // Native extends horizontal coverage rather than zooming the HUD on
        // ultrawide displays. Narrow/portrait outputs also limit growth.
        let effective_height = height.min(width * 0.75);
        let scale = match request.policy {
            UiSubmissionPolicy::NativeCanvas => {
                fit.min(native_hud_comfort_scale(effective_height.max(768.0)))
            }
            UiSubmissionPolicy::NativeHud { .. } if effective_height <= 768.0 => fit.min(1.0),
            UiSubmissionPolicy::NativeHud { .. } => native_hud_comfort_scale(effective_height),
            UiSubmissionPolicy::FitAuthoredCanvas => fit,
        };
        let [anchor_x, anchor_y] = match request.policy {
            // Each original preset maps its matching canvas exactly. Native
            // fullscreen anchors each group to the drawable even at 720p;
            // only its actual pixels must fit, not the unused canvas.
            UiSubmissionPolicy::NativeHud { anchor } => match anchor {
                UiAnchor::BottomLeft => [0.0, 1.0],
                UiAnchor::BottomRight => [1.0, 1.0],
                UiAnchor::TopLeft => [0.0, 0.0],
            },
            _ => [0.5, 0.5],
        };
        let pixel_offset = |value: f32| match request.policy {
            // Round corner translations so fractional pixel scales do not
            // accumulate an extra pixel of error at the opposite screen edge.
            UiSubmissionPolicy::NativeHud { .. } => value.round() as i32,
            _ => value as i32,
        };
        Self {
            scale,
            offset_x: pixel_offset((width - canvas_width * scale) * anchor_x),
            offset_y: pixel_offset((height - canvas_height * scale) * anchor_y),
        }
    }
}

/// Port-owned readability extension. The original 96-pixel globe occupies
/// 16% of an 800x600 display. Ramp continuously from the compact 1024x768
/// tier to that density at 1080p, avoiding a size jump just above 768 pixels.
fn native_hud_comfort_scale(effective_height: f32) -> f32 {
    const ORIGINAL_CEILING_HEIGHT: f32 = 768.0;
    const COMFORT_REFERENCE_HEIGHT: f32 = 600.0;
    const FULL_COMFORT_HEIGHT: f32 = 1080.0;
    if effective_height < FULL_COMFORT_HEIGHT {
        let progress = (effective_height - ORIGINAL_CEILING_HEIGHT)
            / (FULL_COMFORT_HEIGHT - ORIGINAL_CEILING_HEIGHT);
        1.0 + progress * (FULL_COMFORT_HEIGHT / COMFORT_REFERENCE_HEIGHT - 1.0)
    } else {
        effective_height / COMFORT_REFERENCE_HEIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_native_preserves_original_canvases_and_bounds_readability_growth() {
        for (canvas, hd_scale, hd_margins, uhd_scale, uhd_margins) in [
            ([640, 480], 1.8, [384, 108], 3.6, [768, 216]),
            ([800, 600], 1.8, [240, 0], 3.6, [480, 0]),
            ([1024, 768], 1.40625, [240, 0], 2.8125, [480, 0]),
        ] {
            for (viewport, scale, margins) in [
                (canvas, 1.0, [0, 0]),
                ([1920, 1080], hd_scale, hd_margins),
                ([3840, 2160], uhd_scale, uhd_margins),
                (canvas, 1.0, [0, 0]),
            ] {
                assert_eq!(
                    UiMapping::new(UiMappingRequest {
                        viewport,
                        authored_canvas: canvas,
                        policy: UiSubmissionPolicy::NativeCanvas,
                    }),
                    UiMapping {
                        scale,
                        offset_x: margins[0],
                        offset_y: margins[1],
                    }
                );
            }
        }
    }

    #[test]
    fn explicit_fit_keeps_low_enlargement_and_classic_logical_canvas() {
        let low = UiMapping::new(UiMappingRequest {
            viewport: [1280, 720],
            authored_canvas: [320, 240],
            policy: UiSubmissionPolicy::FitAuthoredCanvas,
        });
        assert_eq!(
            low,
            UiMapping {
                scale: 3.0,
                offset_x: 160,
                offset_y: 0
            }
        );
        for canvas in [[320, 240], [640, 480], [800, 600], [1024, 768]] {
            assert_eq!(
                UiMapping::new(UiMappingRequest {
                    viewport: canvas,
                    authored_canvas: canvas,
                    policy: UiSubmissionPolicy::FitAuthoredCanvas,
                }),
                UiMapping {
                    scale: 1.0,
                    offset_x: 0,
                    offset_y: 0
                }
            );
        }
    }

    #[test]
    fn policy_is_independent_of_canvas_numeric_identity() {
        let request = UiMappingRequest {
            viewport: [1280, 960],
            authored_canvas: [640, 480],
            policy: UiSubmissionPolicy::NativeCanvas,
        };
        let native_scale = UiMapping::new(request).scale;
        assert!(native_scale > 1.0 && native_scale < 2.0);
        assert_eq!(
            UiMapping::new(UiMappingRequest {
                policy: UiSubmissionPolicy::FitAuthoredCanvas,
                ..request
            })
            .scale,
            2.0
        );
        assert_eq!(
            UiMapping::new(UiMappingRequest {
                viewport: [320, 240],
                ..request
            })
            .scale,
            0.5
        );
        assert_eq!(
            UiMapping::new(UiMappingRequest {
                viewport: [0, 0],
                ..request
            })
            .scale,
            0.0
        );
    }

    #[test]
    fn native_hud_keeps_every_original_output_and_down_fitted_canvas() {
        for canvas in [[640, 480], [800, 600], [1024, 768]] {
            for viewport in [canvas, [320, 240]] {
                let request = UiMappingRequest {
                    viewport,
                    authored_canvas: canvas,
                    policy: UiSubmissionPolicy::NativeCanvas,
                };
                for anchor in [
                    UiAnchor::BottomLeft,
                    UiAnchor::BottomRight,
                    UiAnchor::TopLeft,
                ] {
                    assert_eq!(
                        UiMapping::new(UiMappingRequest {
                            policy: request.policy.for_gameplay_hud(anchor),
                            ..request
                        }),
                        UiMapping::new(request),
                    );
                }
            }
        }
    }

    #[test]
    fn native_720p_uses_screen_edges_without_enlarging_the_original_pixels() {
        let request = UiMappingRequest {
            viewport: [1280, 720],
            authored_canvas: [800, 600],
            policy: UiSubmissionPolicy::NativeHud {
                anchor: UiAnchor::BottomLeft,
            },
        };
        assert_eq!(
            UiMapping::new(request),
            UiMapping {
                scale: 1.0,
                offset_x: 0,
                offset_y: 120
            }
        );
        assert_eq!(
            UiMapping::new(UiMappingRequest {
                policy: UiSubmissionPolicy::NativeHud {
                    anchor: UiAnchor::BottomRight
                },
                ..request
            }),
            UiMapping {
                scale: 1.0,
                offset_x: 480,
                offset_y: 120
            },
        );
    }

    #[test]
    fn native_hud_comfort_density_follows_height_and_screen_edges() {
        for canvas in [[640, 480], [800, 600], [1024, 768]] {
            for (viewport, expected_scale) in [
                ([1920, 1080], 1.8),
                ([2560, 1440], 2.4),
                ([3840, 2160], 3.6),
                ([3440, 1440], 2.4),
                ([2160, 3840], 2.7),
            ] {
                for (anchor, factors) in [
                    (UiAnchor::BottomLeft, [0.0, 1.0]),
                    (UiAnchor::BottomRight, [1.0, 1.0]),
                    (UiAnchor::TopLeft, [0.0, 0.0]),
                ] {
                    let mapping = UiMapping::new(UiMappingRequest {
                        viewport,
                        authored_canvas: canvas,
                        policy: UiSubmissionPolicy::NativeHud { anchor },
                    });
                    assert_eq!(mapping.scale, expected_scale);
                    // A point inset from the selected edge retains that inset
                    // times the pixel scale, without a centred 4:3 safe zone.
                    for axis in 0..2 {
                        let authored_point = canvas[axis] as f32 * factors[axis];
                        let offset = [mapping.offset_x, mapping.offset_y][axis];
                        let mapped_point = offset as f32 + authored_point * mapping.scale;
                        assert!((mapped_point - viewport[axis] as f32 * factors[axis]).abs() < 1.0);
                    }
                }
            }
        }
    }

    #[test]
    fn native_hud_growth_is_continuous_and_monotonic_at_both_ramp_boundaries() {
        let mut previous = 1.0;
        for height in 768..=2160 {
            let mapping = UiMapping::new(UiMappingRequest {
                viewport: [3840, height],
                authored_canvas: [1024, 768],
                policy: UiSubmissionPolicy::NativeHud {
                    anchor: UiAnchor::BottomLeft,
                },
            });
            assert!(mapping.scale >= previous);
            assert!(mapping.scale - previous < 0.003);
            previous = mapping.scale;
        }
    }

    #[test]
    fn gameplay_policy_preserves_fitted_low_and_bounds_frontend_canvas_growth() {
        assert_eq!(
            UiSubmissionPolicy::FitAuthoredCanvas.for_gameplay_hud(UiAnchor::BottomLeft),
            UiSubmissionPolicy::FitAuthoredCanvas,
        );
        assert_eq!(
            UiSubmissionPolicy::NativeHud {
                anchor: UiAnchor::BottomLeft
            }
            .for_gameplay_hud(UiAnchor::BottomRight),
            UiSubmissionPolicy::NativeHud {
                anchor: UiAnchor::BottomRight
            },
        );
        let request = UiMappingRequest {
            viewport: [3840, 2160],
            authored_canvas: [1024, 768],
            policy: UiSubmissionPolicy::NativeCanvas,
        };
        assert_eq!(UiMapping::new(request).scale, 2.8125);
        assert_eq!(
            UiMapping::new(UiMappingRequest {
                policy: request.policy.for_gameplay_hud(UiAnchor::BottomLeft),
                ..request
            })
            .scale,
            3.6
        );
    }

    #[test]
    fn branding_native_preserves_original_pixels_and_grows_continuously() {
        for (output_size, reference_height) in [
            ([640, 480], 480),
            ([800, 600], 600),
            ([1024, 768], 768),
            ([1280, 720], 600),
            ([320, 240], 768),
        ] {
            assert_eq!(
                UiSubmissionPolicy::NativeCanvas.authored_pixel_scale(
                    ScalingMode::Native,
                    output_size,
                    reference_height,
                ),
                1.0,
            );
        }
        for reference_height in [480, 600, 768] {
            for (output_size, expected) in [
                ([1920, 1080], 1.8),
                ([2560, 1440], 2.4),
                ([3840, 2160], 3.6),
            ] {
                let scale = UiSubmissionPolicy::NativeCanvas.authored_pixel_scale(
                    ScalingMode::Native,
                    output_size,
                    reference_height,
                );
                assert!((scale - expected).abs() < 1e-6);
            }
        }
        let scale_at = |height| {
            UiSubmissionPolicy::NativeCanvas.authored_pixel_scale(
                ScalingMode::Native,
                [1920, height],
                768,
            )
        };
        for boundary in [768, 1080] {
            let before = scale_at(boundary - 1);
            let at = scale_at(boundary);
            let after = scale_at(boundary + 1);
            assert!(before <= at && at <= after);
            assert!(after - before < 0.006);
        }
    }

    #[test]
    fn branding_native_growth_uses_the_short_axis() {
        for (output_size, expected) in [
            ([1920, 1080], 1.8),
            ([3840, 1080], 1.8),
            ([1440, 2160], 1.8),
            ([768, 2160], 1.0),
            ([1080, 1920], 1.0 + 42.0 / 312.0 * 0.8),
        ] {
            for policy in [
                UiSubmissionPolicy::NativeCanvas,
                UiSubmissionPolicy::NativeHud {
                    anchor: UiAnchor::BottomRight,
                },
            ] {
                let scale = policy.authored_pixel_scale(ScalingMode::Native, output_size, 768);
                assert!((scale - expected).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn branding_preserves_explicit_low_and_fitted_decisions() {
        assert_eq!(
            UiSubmissionPolicy::FitAuthoredCanvas.authored_pixel_scale(
                ScalingMode::Native,
                [1280, 720],
                240
            ),
            3.0
        );
        assert_eq!(
            UiSubmissionPolicy::FitAuthoredCanvas.authored_pixel_scale(
                ScalingMode::Native,
                [1080, 1920],
                240
            ),
            8.0
        );
        for scaling in [ScalingMode::FourThree, ScalingMode::Stretched] {
            for policy in [
                UiSubmissionPolicy::NativeCanvas,
                UiSubmissionPolicy::NativeHud {
                    anchor: UiAnchor::BottomLeft,
                },
                UiSubmissionPolicy::FitAuthoredCanvas,
            ] {
                assert_eq!(policy.authored_pixel_scale(scaling, [3840, 2160], 480), 1.0);
            }
        }
    }
}
