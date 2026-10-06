use thiserror::Error;

#[derive(Debug, Error)]
pub enum V2kError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("OVL format error: {0}")]
    OvlFormat(String),

    #[error("Section {section} error: {message}")]
    Section { section: usize, message: String },

    #[error("Data truncated: need {need} bytes at offset {offset}, have {have}")]
    Truncated {
        offset: usize,
        need: usize,
        have: usize,
    },
}

impl V2kError {
    pub fn truncated(offset: usize, need: usize, have: usize) -> Self {
        Self::Truncated { offset, need, have }
    }

    pub fn section(section: usize, message: impl Into<String>) -> Self {
        Self::Section {
            section,
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, V2kError>;

/// Camera-relative world-scan policies shared by terrain, water, and static
/// object traversal.
pub mod render_scan {
    /// Ordinary terrain scans begin two cells in front of the camera.
    pub const DEFAULT_LEAD_RAW: i32 = 0x200;

    /// Recover the pitch-dependent row lead used by `FUN_0042F270` and
    /// `FUN_00431890`.
    ///
    /// `camera_forward_y` is the vertical component of the unit view
    /// direction. `FUN_0040F3A0` stores its normalized focus-minus-eye vector
    /// as the third camera-basis column, making camera word 12 exactly this Y
    /// component in signed Q31. Retail's signed 32-bit addition wraps: the
    /// scan origin changes below -0.6875 and at/above +0.3125. The upper
    /// boundary intentionally wraps to `i32::MIN`, producing retail's large
    /// discontinuous rearward overscan before the curve returns toward -6.75
    /// cells at the vertical endpoint. This keeps terrain, water, and objects
    /// beneath a craft-visible camera pose instead of exposing the clear
    /// colour.
    pub fn terrain_row_lead_raw(camera_forward_y: f32) -> i32 {
        let camera_forward_y = camera_forward_y.clamp(-1.0, 1.0);
        let component_q31 = if camera_forward_y <= -1.0 {
            i32::MIN
        } else if camera_forward_y >= 1.0 {
            i32::MAX
        } else {
            (f64::from(camera_forward_y) * 2_147_483_648.0).round() as i32
        };
        let shifted = component_q31.wrapping_add(0x5800_0000);
        if shifted < 0 {
            (((i64::from(shifted) * 0x1c00) >> 31) as i32).wrapping_add(DEFAULT_LEAD_RAW)
        } else {
            DEFAULT_LEAD_RAW
        }
    }

    /// Pitch-dependent row lead in the port's one-unit-per-cell domain.
    pub fn terrain_row_lead(camera_forward_y: f32) -> f32 {
        terrain_row_lead_raw(camera_forward_y) as f32 / 256.0
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ordinary_views_keep_the_two_cell_lead() {
            assert_eq!(terrain_row_lead_raw(-0.6875), 0x200);
            assert_eq!(terrain_row_lead_raw(0.0), 0x200);
            assert_eq!(terrain_row_lead_raw(0.25), 0x200);
        }

        #[test]
        fn steep_views_follow_both_exact_signed_q31_tails() {
            assert_eq!(terrain_row_lead_raw(-1.0), -0x6c0);
            assert_eq!(terrain_row_lead_raw(-0.75), 0x40);
            assert_eq!(terrain_row_lead_raw(0.3125), -0x1a00);
            assert_eq!(terrain_row_lead_raw(0.5), -0x14c0);
            assert_eq!(terrain_row_lead_raw(0.75), -0xdc0);
            assert_eq!(terrain_row_lead_raw(1.0), -0x6c1);
        }
    }
}

/// Toroidal world topology (the "round planet").
///
/// V2000's world wraps on the X/Z plane: fly off one edge and you arrive at
/// the opposite edge. The engine gets this for free by storing every position
/// component as a 16-bit `short` — the 65536-unit world (256 cells × 256
/// units/cell) overflows at exactly the wrap period, and every position delta
/// is a `short` subtraction, so its natural mod-65536 result is already the
/// shortest way around the torus. The vertical (Y) axis does NOT wrap.
///
/// The port uses `f32` with 1.0 unit per cell, so the wrap period is
/// [`WORLD_CELLS`] (256.0) rather than 65536, but the arithmetic is identical.
pub mod world {
    /// Horizontal world size in port units: 256 cells × 1.0 unit/cell.
    /// Positions wrap modulo this on X and Z.
    pub const WORLD_CELLS: f32 = 256.0;

    /// Half the world — the largest possible shortest-path delta magnitude.
    pub const HALF_WORLD: f32 = WORLD_CELLS * 0.5;

    /// Wrap a horizontal coordinate into `[0, WORLD_CELLS)`.
    ///
    /// Mirrors the engine's free 16-bit overflow on X/Z position stores.
    #[inline]
    pub fn wrap(v: f32) -> f32 {
        v.rem_euclid(WORLD_CELLS)
    }

    /// Shortest signed delta from `b` to `a` across the torus, in
    /// `[-HALF_WORLD, HALF_WORLD)`.
    ///
    /// This is the port equivalent of the engine's `(short)(a - b)`: distances,
    /// aim-lead, and camera-follow all use it so nothing ever takes the long way
    /// around when the seam is between the two points. At exactly half a
    /// world, signed-short subtraction selects the negative displacement in
    /// both directions. Unquantized floats retain this interval convention;
    /// this helper does not quantize them to native 8.8 coordinates.
    #[inline]
    pub fn delta(a: f32, b: f32) -> f32 {
        let d = (a - b).rem_euclid(WORLD_CELLS);
        if d >= HALF_WORLD {
            d - WORLD_CELLS
        } else {
            d
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn wrap_into_range() {
            assert_eq!(wrap(0.0), 0.0);
            assert_eq!(wrap(255.0), 255.0);
            assert!((wrap(256.0) - 0.0).abs() < 1e-4);
            assert!((wrap(257.5) - 1.5).abs() < 1e-4);
            assert!((wrap(-1.0) - 255.0).abs() < 1e-4);
            assert!((wrap(-256.5) - 255.5).abs() < 1e-4);
        }

        #[test]
        fn delta_shortest_path() {
            // Straightforward interior deltas.
            assert!((delta(10.0, 4.0) - 6.0).abs() < 1e-4);
            assert!((delta(4.0, 10.0) - -6.0).abs() < 1e-4);
            // Across the seam: 250 -> 5 is +11 (forward over the edge), not -245.
            assert!((delta(5.0, 250.0) - 11.0).abs() < 1e-4);
            assert!((delta(250.0, 5.0) - -11.0).abs() < 1e-4);
            // Antipodal stays bounded.
            assert!(delta(128.0, 0.0).abs() <= HALF_WORLD + 1e-4);
        }

        #[test]
        fn delta_matches_signed_word_subtraction() {
            // 4138F0 narrows the difference before selecting the world image.
            // Check every 8.8 displacement against WORD arithmetic, including
            // both antipodal directions and signed/unsigned position images.
            for origin in [i16::MIN, -120, 0, 32648, i16::MAX] {
                for displacement in 0..=u16::MAX {
                    let target = origin.wrapping_add(displacement as i16);
                    let expected = f32::from(target.wrapping_sub(origin)) / 256.0;
                    for (a, b) in [
                        (f32::from(target) / 256.0, f32::from(origin) / 256.0),
                        (
                            f32::from(target as u16) / 256.0,
                            f32::from(origin as u16) / 256.0,
                        ),
                    ] {
                        assert_eq!(delta(a, b), expected, "origin={origin} target={target}");
                    }
                }
            }
        }
    }
}
