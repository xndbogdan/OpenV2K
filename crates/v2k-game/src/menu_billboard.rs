//! Authored menu billboard geometry from `FUN_0042C090` and `FUN_0042D030`.
//!
//! The reference sprite (global 420) positions the billboard; the selected
//! animation frame supplies its aspect ratio. All coordinates remain in the
//! selected system overlay's framebuffer until the presentation adapter maps
//! the resulting integer rectangle to the host viewport.

use crate::resource_cache::ResourceCache;

/// Live menu actor context words consumed by `FUN_0042C090`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuBillboardPose {
    pub zoom_raw: u32,
    pub height_raw: i16,
}

impl MenuBillboardPose {
    /// Signed 16.16 scale, including retail's wrapping 32-bit zoom shift.
    pub fn scale_raw(self) -> i32 {
        let zoom = (self.zoom_raw as i32).wrapping_shl(15);
        let height_factor = 0x20500 - 32 * i32::from(self.height_raw);
        ((i64::from(zoom) * i64::from(height_factor)) >> 31) as i32
    }
}

/// Selected-tier layout resources, independent of the current frame's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuBillboardLayout {
    pub framebuffer: [i32; 2],
    pub focal_y: i32,
    pub layout_y: i16,
    pub reference_size: [u16; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuBillboardRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl MenuBillboardLayout {
    /// Resolve the exact system-2 scalar/layout slots and global sprite 420
    /// metadata. Missing resources do not select a different display tier.
    pub fn from_cache(cache: &ResourceCache) -> Option<Self> {
        let (_, reference) = cache.global_sprite(420)?;
        let layout = Self {
            framebuffer: [
                cache.system_data_value(2, 2)? as i32,
                cache.system_data_value(2, 3)? as i32,
            ],
            focal_y: cache.system_data_value(2, 1)? as i32,
            layout_y: cache.system_layout_point(2, 1)?.1,
            reference_size: [reference.flags as u16, (reference.flags >> 16) as u16],
        };
        layout.has_dimensions().then_some(layout)
    }

    fn has_dimensions(self) -> bool {
        self.framebuffer.into_iter().all(|value| value > 0)
            && self.focal_y > 0
            && self.reference_size.into_iter().all(|value| value != 0)
    }

    /// Construct retail's signed-word corners, rejecting empty or inverted
    /// output. Viewport mapping and clipping belong to the presentation
    /// adapter, so even offscreen rectangles retain their complete extent.
    pub fn rect(self, pose: MenuBillboardPose, frame_size: [u16; 2]) -> Option<MenuBillboardRect> {
        if !self.has_dimensions() || frame_size.contains(&0) {
            return None;
        }

        let [framebuffer_w, framebuffer_h] = self.framebuffer;
        let [reference_w, reference_h] = self.reference_size.map(i32::from);
        let scale = pose.scale_raw();
        let reference_product = reference_w.wrapping_mul(scale);
        let half_reference = ((reference_product >> 16) - (reference_product >> 31)) >> 1;
        let mut left = ((framebuffer_w / 2) as i16).wrapping_sub(half_reference as i16);
        let vertical_divisor = if framebuffer_h < 480 { 24 } else { 11 };
        let mut top = self
            .layout_y
            .wrapping_add((-i32::from(pose.height_raw) / vertical_divisor) as i16);

        if framebuffer_h > 480 {
            let width_delta =
                (framebuffer_w.wrapping_mul(reference_w) / 640).wrapping_sub(reference_w);
            left = left.wrapping_sub((width_delta.wrapping_mul(scale) >> 17) as i16);
            let height_delta =
                (reference_h.wrapping_mul(framebuffer_h) / 480).wrapping_sub(reference_h);
            top = top.wrapping_sub((height_delta / 2) as i16);
        }

        // FUN_0042D030 narrows only the final corner additions. Its focal
        // quarter and scale product first retain x86 signed-dword wrapping.
        let product = (self.focal_y.wrapping_shl(6) >> 8).wrapping_mul(scale);
        let height = product >> 16;
        let width = i32::from(frame_size[0]).wrapping_mul(height) / i32::from(frame_size[1]);
        if width <= 0 || height <= 0 {
            return None;
        }
        let right = left.wrapping_add(width as i16);
        let bottom = top.wrapping_add(height as i16);
        let [left, top, right, bottom] = [left, top, right, bottom].map(i32::from);
        if right <= left || bottom <= top {
            return None;
        }
        Some(MenuBillboardRect {
            x: left,
            y: top,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIERS: [MenuBillboardLayout; 4] = [
        MenuBillboardLayout {
            framebuffer: [320, 240],
            focal_y: 256,
            layout_y: 55,
            reference_size: [51, 64],
        },
        MenuBillboardLayout {
            framebuffer: [640, 480],
            focal_y: 512,
            layout_y: 110,
            reference_size: [102, 129],
        },
        MenuBillboardLayout {
            framebuffer: [800, 600],
            focal_y: 640,
            layout_y: 137,
            reference_size: [102, 129],
        },
        MenuBillboardLayout {
            framebuffer: [1024, 768],
            focal_y: 820,
            layout_y: 176,
            reference_size: [102, 129],
        },
    ];

    fn pose(zoom_raw: u32, height_raw: i16) -> MenuBillboardPose {
        MenuBillboardPose {
            zoom_raw,
            height_raw,
        }
    }

    fn components(rect: MenuBillboardRect) -> [i32; 4] {
        [rect.x, rect.y, rect.width as i32, rect.height as i32]
    }

    #[test]
    fn authored_tiers_match_retail_at_idle_zoom_and_vertical_motion() {
        // Independent integer evaluations of the recovered callbacks, using
        // each tier's actual reference-420 and selected-1294 metadata.
        let cases = [
            (
                pose(65535, 0),
                132349,
                [
                    [109, 55, 102, 129],
                    [218, 110, 206, 258],
                    [273, 121, 258, 323],
                    [349, 138, 330, 413],
                ],
            ),
            (
                pose(65535, 1200),
                93950,
                [
                    [124, 5, 72, 91],
                    [247, 1, 146, 183],
                    [310, 12, 183, 229],
                    [396, 29, 234, 293],
                ],
            ),
            (
                pose(32767, 0),
                66173,
                [
                    [135, 55, 51, 64],
                    [269, 110, 103, 129],
                    [337, 121, 128, 161],
                    [431, 138, 164, 206],
                ],
            ),
            (
                pose(65535, -96),
                135421,
                [
                    [108, 59, 105, 132],
                    [215, 118, 210, 264],
                    [270, 129, 263, 330],
                    [344, 146, 338, 423],
                ],
            ),
        ];
        for (pose, scale, expected) in cases {
            assert_eq!(pose.scale_raw(), scale);
            for (tier, layout) in TIERS.into_iter().enumerate() {
                let frame = if tier == 0 { [102, 128] } else { [203, 254] };
                assert_eq!(
                    components(layout.rect(pose, frame).unwrap()),
                    expected[tier]
                );
            }
        }
    }

    #[test]
    fn signed_height_translation_truncates_toward_zero() {
        for (tier, divisor) in [(0, 24), (1, 11)] {
            let layout = TIERS[tier];
            for (height, delta) in [
                (divisor - 1, 0),
                (divisor, -1),
                (-divisor + 1, 0),
                (-divisor, 1),
            ] {
                let rect = layout.rect(pose(65535, height), [1, 1]).unwrap();
                assert_eq!(rect.y, i32::from(layout.layout_y) + delta);
            }
        }
    }

    #[test]
    fn height_480_changes_divisor_and_only_larger_heights_apply_corrections() {
        let mut layout = TIERS[1];
        layout.framebuffer[0] = 800;
        for (height, expected) in [
            (479, [298, 109, 204, 256]),
            (480, [298, 108, 204, 256]),
            (488, [273, 107, 204, 256]),
        ] {
            layout.framebuffer[1] = height;
            assert_eq!(
                components(layout.rect(pose(65535, 24), [203, 254]).unwrap()),
                expected,
            );
        }
    }

    #[test]
    fn reference_controls_position_while_selected_frame_controls_aspect() {
        let layout = TIERS[1];
        let wide = layout.rect(pose(65535, 0), [203, 254]).unwrap();
        let square = layout.rect(pose(65535, 0), [254, 254]).unwrap();
        assert_eq!(components(wide), [218, 110, 206, 258]);
        assert_eq!(components(square), [218, 110, 258, 258]);
        let other_reference = MenuBillboardLayout {
            reference_size: [103, 129],
            ..layout
        };
        assert_eq!(
            components(other_reference.rect(pose(65535, 0), [203, 254]).unwrap()),
            [216, 110, 206, 258],
        );
    }

    #[test]
    fn wrapped_negative_reference_product_uses_signed_truncating_half() {
        let layout = MenuBillboardLayout {
            framebuffer: [40000, 480],
            reference_size: [25000, 129],
            ..TIERS[1]
        };
        assert_eq!(
            components(layout.rect(pose(65535, 0), [203, 254]).unwrap()),
            [27524, 110, 206, 258],
        );
    }

    #[test]
    fn scale_preserves_raw_zoom_shift_and_signed_height() {
        assert_eq!(pose(65535, 0).scale_raw(), 132349);
        assert_eq!(pose(65536, 0).scale_raw(), -132352);
        assert_eq!(pose(u32::MAX, 0).scale_raw(), -3);
        assert_eq!(pose(131071, 0).scale_raw(), -3);
        assert_eq!(pose(196607, 0).scale_raw(), 132349);
        assert_eq!(pose(65535, i16::MIN).scale_raw(), 1180909);
        assert_eq!(pose(65535, i16::MAX).scale_raw(), -916179);
        for zoom in [0, 1, 65536, u32::MAX] {
            assert_eq!(TIERS[1].rect(pose(zoom, 0), [203, 254]), None);
        }
    }

    #[test]
    fn invalid_metadata_and_wrapped_corners_are_not_repaired() {
        let valid = TIERS[1];
        for layout in [
            MenuBillboardLayout {
                framebuffer: [0, 480],
                ..valid
            },
            MenuBillboardLayout {
                framebuffer: [640, -1],
                ..valid
            },
            MenuBillboardLayout {
                focal_y: 0,
                ..valid
            },
            MenuBillboardLayout {
                reference_size: [0, 129],
                ..valid
            },
            MenuBillboardLayout {
                reference_size: [102, 0],
                ..valid
            },
            MenuBillboardLayout {
                focal_y: i32::MAX,
                ..valid
            },
            MenuBillboardLayout {
                layout_y: 32760,
                ..valid
            },
        ] {
            assert_eq!(layout.rect(pose(65535, 0), [203, 254]), None);
        }
        for frame in [[0, 254], [203, 0], [130, 1]] {
            assert_eq!(valid.rect(pose(65535, 0), frame), None);
        }
        assert_eq!(
            MenuBillboardLayout::from_cache(&ResourceCache::new(Vec::new())),
            None
        );
    }

    #[test]
    fn offscreen_positions_preserve_authored_texture_extent() {
        let layout = MenuBillboardLayout {
            layout_y: -20,
            ..TIERS[1]
        };
        assert_eq!(
            components(layout.rect(pose(65535, 0), [203, 254]).unwrap()),
            [218, -20, 206, 258],
        );
        let hidden = MenuBillboardLayout {
            layout_y: -300,
            ..layout
        };
        assert_eq!(
            components(hidden.rect(pose(65535, 0), [203, 254]).unwrap()),
            [218, -300, 206, 258],
        );
    }
}
