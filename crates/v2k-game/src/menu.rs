//! 3D elliptical carousel menu — layout math, sprite helpers, and menu resources.
//!
//! The carousel arranges N items around an ellipse. The selected item sits
//! at the front (bottom-center), largest and brightest. Items receding into
//! the background rise upward, shrink, and dim — matching the original V2000
//! menu perspective where distant items appear higher on screen.

use std::f32::consts::TAU;

use v2k_formats::sprites::SpriteAtlas;

use crate::level::LevelState;

// ── Menu resources (decoded OVL sprites) ──

/// A decoded RGBA icon ready for rendering.
pub struct DecodedIcon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Pre-decoded menu visual assets from OVL preload data.
///
/// Created once during init from the Level 5 menu-graphics OVL.
/// Contains the 7 trophy carousel icons, copyright banner, and logos.
pub struct MenuResources {
    /// 7 trophy sprites (102x128 RGBA) for carousel item icons.
    pub trophy_icons: Vec<DecodedIcon>,
    /// The 6 background flame/emblem billboard frames, indexed by
    /// (global sprite id − 1294): L5 entries 9–14 = the animated backdrop
    /// behind the prop ring (940 ms cycle; MENU_SYSTEM.md §Background).
    pub flame_frames: Vec<DecodedIcon>,
    /// Copyright banner sprite (variant 0: 155×14; variants 1–3: 309×28).
    pub copyright_banner: Option<DecodedIcon>,
    /// Frontier developer logo (variant 0: 54×33; variants 1–3: 110×68).
    pub frontier_logo: Option<DecodedIcon>,
    /// Publisher logo (variant 0: 46×22; variants 1–3: 115×56).
    pub publisher_logo: Option<DecodedIcon>,
    /// Index of the "screeno2" model in Level 5's ModelCollection.
    pub screeno2_model_idx: Option<usize>,
    /// On-screen upscale factor for the bottom logos. 1 for the high-res
    /// variant-1 atlas (blit native); 3 for the low-res variant-0 atlas (the
    /// old behaviour — enlarge the tiny sprite to keep the same footprint).
    /// Set by the caller from the graphics-detail tier.
    pub logo_scale: u32,
}

impl MenuResources {
    /// Decode menu sprites from the Level 5 menu-graphics OVL.
    ///
    /// Level 5 sprite layout:
    ///   0-4:  National flags (128x64-85)
    ///   5:    Frontier logo (54x33)
    ///   6:    Publisher logo (46x22)
    ///   7:    Copyright banner (155x14 in variant 0)
    ///   8-14: Trophy/chalice icons (102x128) — carousel item icons
    pub fn from_menu_ovl(menu_gfx: &LevelState) -> Self {
        let shade = v2k_formats::palette::BRIGHTEST_SHADE;
        let atlas = match &menu_gfx.sprites {
            Some(a) => a,
            None => return Self::empty(),
        };

        let trophy_icons = Self::decode_range(atlas, 8, 15, shade);
        let flame_frames = Self::decode_range(atlas, 9, 15, shade);
        let copyright_banner = Self::decode_one(atlas, 7, shade);
        let frontier_logo = Self::decode_one(atlas, 5, shade);
        let publisher_logo = Self::decode_one(atlas, 6, shade);

        // Find screeno2 model by name
        let screeno2_model_idx = menu_gfx.models.as_ref().and_then(|mc| {
            mc.all_entries
                .iter()
                .position(|e| e.name.as_deref() == Some("screeno2"))
        });

        Self {
            trophy_icons,
            flame_frames,
            copyright_banner,
            frontier_logo,
            publisher_logo,
            screeno2_model_idx,
            logo_scale: 1,
        }
    }

    pub fn empty() -> Self {
        Self {
            trophy_icons: Vec::new(),
            flame_frames: Vec::new(),
            copyright_banner: None,
            frontier_logo: None,
            publisher_logo: None,
            screeno2_model_idx: None,
            logo_scale: 1,
        }
    }

    fn decode_one(atlas: &SpriteAtlas, entry_idx: usize, shade: usize) -> Option<DecodedIcon> {
        let entry = atlas.entries.get(entry_idx)?;
        let decoded = atlas.decode_sprite(entry, shade).ok()?;
        Some(DecodedIcon {
            width: decoded.width as u32,
            height: decoded.height as u32,
            rgba: decoded.rgba,
        })
    }

    fn decode_range(
        atlas: &SpriteAtlas,
        start: usize,
        end: usize,
        shade: usize,
    ) -> Vec<DecodedIcon> {
        (start..end)
            .filter_map(|i| Self::decode_one(atlas, i, shade))
            .collect()
    }
}

/// Computed position/appearance for one carousel item in a single frame.
#[derive(Debug, Clone)]
pub struct CarouselSlot {
    /// Screen X position (center of icon).
    pub x: f32,
    /// Screen Y position (center of icon).
    pub y: f32,
    /// Scale factor: 1.0 at front, 0.35 at back.
    pub scale: f32,
    /// Alpha/brightness factor: 1.0 at front, 0.3 at back.
    pub alpha: f32,
    /// Depth order value (higher = closer to viewer). Sort ascending for painter's algorithm.
    pub z_order: f32,
    /// Index into the menu items array.
    pub item_index: usize,
}

/// Compute the layout for all carousel items at the current rotation angle.
///
/// - `n`: number of items
/// - `current_angle`: current rotation angle in radians (0 = first item at front)
/// - `cx`, `cy`: center of the carousel on screen
/// - `rx`: horizontal ellipse radius (wide)
/// - `ry`: vertical ellipse radius (shallow, for perspective)
pub fn compute_carousel_layout(
    n: usize,
    current_angle: f32,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
) -> Vec<CarouselSlot> {
    if n == 0 {
        return vec![];
    }

    let step = TAU / n as f32;
    let mut slots: Vec<CarouselSlot> = (0..n)
        .map(|i| {
            let angle = current_angle + i as f32 * step;
            let x = cx + rx * angle.sin();
            // +cos = front (below center), -cos = back (above center)
            // Back items rise upward like the original V2000 menu
            let y = cy + ry * angle.cos();

            // cos(angle): +1 at front (angle=0), -1 at back (angle=pi)
            let depth = (angle.cos() + 1.0) / 2.0; // 0..1, 1=front
            let scale = 0.35 + 0.65 * depth;
            let alpha = 0.3 + 0.7 * depth;

            CarouselSlot {
                x,
                y,
                scale,
                alpha,
                z_order: angle.cos(), // -1..+1
                item_index: i,
            }
        })
        .collect();

    // Sort by z_order ascending so back items draw first (painter's algorithm)
    slots.sort_by(|a, b| a.z_order.partial_cmp(&b.z_order).unwrap());
    slots
}

/// Nearest-neighbor scale of an RGBA buffer.
///
/// Returns a new buffer of size `dst_w * dst_h * 4`.
pub fn scale_rgba(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return vec![0u8; (dst_w * dst_h * 4) as usize];
    }

    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];
    for dy in 0..dst_h {
        let sy = (dy * src_h / dst_h).min(src_h - 1);
        for dx in 0..dst_w {
            let sx = (dx * src_w / dst_w).min(src_w - 1);
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((dy * dst_w + dx) * 4) as usize;
            if si + 3 < src.len() && di + 3 < dst.len() {
                dst[di] = src[si];
                dst[di + 1] = src[si + 1];
                dst[di + 2] = src[si + 2];
                dst[di + 3] = src[si + 3];
            }
        }
    }
    dst
}

/// Apply brightness and alpha modulation to an RGBA buffer.
///
/// `brightness`: 0.0 = black, 1.0 = original color.
/// `alpha`: multiplied onto existing alpha channel.
pub fn tint_rgba(rgba: &[u8], brightness: f32, alpha: f32) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for pixel in out.chunks_exact_mut(4) {
        pixel[0] = (pixel[0] as f32 * brightness).min(255.0) as u8;
        pixel[1] = (pixel[1] as f32 * brightness).min(255.0) as u8;
        pixel[2] = (pixel[2] as f32 * brightness).min(255.0) as u8;
        pixel[3] = (pixel[3] as f32 * alpha).min(255.0) as u8;
    }
    out
}

/// Generate a solid-colored placeholder icon (RGBA).
///
/// Returns `(rgba, width, height)`.
pub fn generate_placeholder_icon(r: u8, g: u8, b: u8, w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
        pixel[3] = 255;
    }
    (rgba, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carousel_layout_front_item_largest() {
        let slots = compute_carousel_layout(7, 0.0, 320.0, 240.0, 200.0, 40.0);
        assert_eq!(slots.len(), 7);

        // Item 0 should be at front (highest z_order, scale ≈ 1.0)
        let front = slots.iter().find(|s| s.item_index == 0).unwrap();
        assert!(
            front.scale > 0.95,
            "Front item scale should be ~1.0, got {}",
            front.scale
        );
        assert!(
            front.alpha > 0.95,
            "Front item alpha should be ~1.0, got {}",
            front.alpha
        );

        // Back item (index n/2 ≈ 3 or 4) should be smallest
        let back = slots
            .iter()
            .min_by(|a, b| a.z_order.partial_cmp(&b.z_order).unwrap())
            .unwrap();
        assert!(
            back.scale < 0.5,
            "Back item scale should be < 0.5, got {}",
            back.scale
        );
    }

    #[test]
    fn carousel_sorted_back_to_front() {
        let slots = compute_carousel_layout(5, 0.5, 320.0, 240.0, 200.0, 40.0);
        for w in slots.windows(2) {
            assert!(
                w[0].z_order <= w[1].z_order,
                "Slots must be sorted back-to-front"
            );
        }
    }

    #[test]
    fn scale_rgba_identity() {
        let src = vec![255, 0, 0, 255, 0, 255, 0, 255]; // 2x1 red, green
        let dst = scale_rgba(&src, 2, 1, 2, 1);
        assert_eq!(dst, src);
    }

    #[test]
    fn tint_rgba_dims() {
        let src = vec![200, 100, 50, 255];
        let out = tint_rgba(&src, 0.5, 0.5);
        assert_eq!(out[0], 100);
        assert_eq!(out[1], 50);
        assert_eq!(out[2], 25);
        assert_eq!(out[3], 127);
    }

    #[test]
    fn placeholder_icon_correct_size() {
        let (rgba, w, h) = generate_placeholder_icon(255, 0, 0, 64, 64);
        assert_eq!(w, 64);
        assert_eq!(h, 64);
        assert_eq!(rgba.len(), 64 * 64 * 4);
        // Check first pixel is red
        assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
    }
}
