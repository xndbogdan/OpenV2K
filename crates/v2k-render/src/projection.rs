//! Render-context projection effects recovered from the retail software
//! projector.
//!
//! V2000 does not distort the completed framebuffer underwater. When the
//! camera is strictly below the Section-10 sea plane, `FUN_00433FA0` installs
//! four alternate vertex projectors. They retain the ordinary view transform,
//! perspective depth and fog, but quantize each projected vertex to integer
//! top-origin pixels and add a tiny table-driven X/Y displacement before clip
//! flags are computed.

use v2k_formats::fixed_math::retail_sine_q15;

use crate::camera::Camera;

/// Per-scene policy for the projected-vertex stage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProjectionEffect {
    /// Ordinary projection callbacks.
    #[default]
    None,
    /// Retail underwater callbacks, driven by the wrapping 50 Hz engine tick.
    RetailUnderwater { tick: i32 },
}

/// Integer top-origin point produced by the shared retail screen stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectedPoint {
    pub x: i32,
    pub y: i32,
}

/// Why this scene intentionally retains the existing floating projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionCompatibilityReason {
    #[default]
    Intrinsic,
    Free,
    LogicalViewportAdapter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionAuthorityMissing {
    NativeLens,
    NativeView,
}

/// Lens custody is independent of a model's endpoint custody. A source lens
/// does not make an intrinsic model producer native; a deliberate Free or
/// logical resize mode may retain compatibility despite native endpoint data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneProjectionAuthority {
    Compatibility(ProjectionCompatibilityReason),
    Native(NativeScreenProjection),
    Missing(ProjectionAuthorityMissing),
}

impl Default for SceneProjectionAuthority {
    fn default() -> Self {
        Self::Compatibility(ProjectionCompatibilityReason::Intrinsic)
    }
}

/// Authenticated source display globals consumed by 46CD90/46CEB0 and
/// 46CCF0. Logical resizing is a separate adapter; callers must not recover
/// these integer values by rounding a GL projection matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeScreenProjection {
    focal_pixels: [i32; 2],
    centre_pixels: [i32; 2],
    viewport_pixels: [i32; 2],
    pub effect: ProjectionEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeProjectedEndpoint {
    pub screen: [i16; 2],
    pub depth_raw: i32,
    pub clip: u8,
}

impl NativeScreenProjection {
    pub fn new(
        focal_pixels: [i32; 2],
        centre_pixels: [i32; 2],
        viewport_pixels: [i32; 2],
        effect: ProjectionEffect,
    ) -> Option<Self> {
        (focal_pixels
            .into_iter()
            .chain(viewport_pixels)
            .all(|value| value > 0))
        .then_some(Self {
            focal_pixels,
            centre_pixels,
            viewport_pixels,
            effect,
        })
    }

    pub fn viewport_pixels(self) -> [i32; 2] {
        self.viewport_pixels
    }

    /// Source signed DIV truncates the perspective quotient before adding
    /// centre X or subtracting from centre Y. The extreme-input branch divides
    /// first, then multiplies; ordinary multiplication wraps as x86 IMUL does.
    pub fn project(self, view_raw: [i32; 3]) -> NativeProjectedEndpoint {
        let [x, y, depth_raw] = view_raw;
        if depth_raw < 0x40 {
            return NativeProjectedEndpoint {
                screen: [0; 2],
                depth_raw,
                clip: 0x40,
            };
        }
        let x = self.centre_pixels[0].wrapping_add(native_perspective_quotient(
            x,
            self.focal_pixels[0],
            depth_raw,
        ));
        let y = self.centre_pixels[1].wrapping_sub(native_perspective_quotient(
            y,
            self.focal_pixels[1],
            depth_raw,
        ));
        let full = retail_projected_point(x, y, self.effect);
        NativeProjectedEndpoint {
            screen: [full.x as i16, full.y as i16],
            depth_raw,
            clip: viewport_clip(
                full.x,
                full.y,
                self.viewport_pixels.map(|value| value as u32),
            ),
        }
    }

    /// Exact 4594C0 width helper; the unsigned-looking decompile retains
    /// signed SAR/comparisons and does not use abs for this width cap.
    pub(crate) fn half_width(self, value: i32, depth_raw: i32) -> (i16, i16) {
        if depth_raw < 0x40 {
            return (0, 0);
        }
        let mut x = native_perspective_quotient(value, self.focal_pixels[0], depth_raw);
        let mut y = native_perspective_quotient(value, self.focal_pixels[1], depth_raw);
        let mut control = x | y;
        while control > 0x1fff {
            control >>= 1;
            x >>= 1;
            y >>= 1;
        }
        (x as i16, y as i16)
    }
}

fn native_perspective_quotient(coordinate: i32, focal: i32, depth: i32) -> i32 {
    let divide_first = if coordinate < 0 {
        coordinate >> 12 < depth.wrapping_neg()
    } else {
        depth < coordinate >> 12
    };
    if divide_first {
        (coordinate / depth).wrapping_mul(focal)
    } else {
        coordinate.wrapping_mul(focal) / depth
    }
}

/// Stored coordinate words and clip byte produced after underwater wobble.
/// Retail narrows the screen words but computes clip flags from the retained
/// full-width registers, so both representations are intentional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderwaterProjection {
    pub screen: [i16; 2],
    pub full: ProjectedPoint,
    pub clip: u8,
}

/// Integer center projection consumed by the retail particle presentation
/// gate (`FUN_0043D410`).
///
/// `depth_raw` remains signed 8.8 so the caller can apply the routine's exact
/// `0x40` near threshold and render-context far word. `screen` is top-origin
/// in the renderer's logical viewport. A near/behind result guarantees only
/// clip bit `0x40`; retail leaves the two screen words undefined in that case,
/// so this port deliberately returns zeroes which callers must not inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParticleCenterProjection {
    pub screen: [i32; 2],
    pub depth_raw: i32,
    pub clip: u8,
}

/// Project one camera-relative world point through the port camera for the
/// policy-exact `FUN_0043D410` particle gate.
///
/// The gate and clip-byte policy are retail-exact. The view transform remains
/// the port's floating-point camera, so this is intentionally not advertised
/// as a bit-identical replacement for retail's Q31 matrix and raw focal words.
/// Keeping it beside the camera projection nevertheless makes the CPU gate and
/// the submitted GL geometry agree at the viewport boundary.
pub fn project_particle_center(
    camera: &Camera,
    viewport: [u32; 2],
    camera_relative_world: [f32; 3],
) -> ParticleCenterProjection {
    let view = camera.view_matrix();
    let view_x = view[0] * camera_relative_world[0]
        + view[4] * camera_relative_world[1]
        + view[8] * camera_relative_world[2]
        + view[12];
    let view_y = view[1] * camera_relative_world[0]
        + view[5] * camera_relative_world[1]
        + view[9] * camera_relative_world[2]
        + view[13];
    let depth = -(view[2] * camera_relative_world[0]
        + view[6] * camera_relative_world[1]
        + view[10] * camera_relative_world[2]
        + view[14]);
    let depth_raw = (depth * 256.0).round() as i32;
    if depth_raw < 0x40 || depth <= 0.0 {
        return ParticleCenterProjection {
            screen: [0, 0],
            depth_raw,
            clip: 0x40,
        };
    }

    let width = viewport[0] as f32;
    let height = viewport[1] as f32;
    let focal_y = height * 0.5 / (camera.fov * 0.5).tan();
    let focal_x = width * 0.5 / (camera.fov * 0.5).tan() / camera.aspect;
    // Camera::projection_matrix stores the offsets in the Z column. With
    // clip.w=-view.z this moves the top-origin optical center by (-x,+y).
    let center_x = width * 0.5 * (1.0 - camera.projection_offset[0]);
    let center_y = height * 0.5 * (1.0 + camera.projection_offset[1]);
    let x = (view_x * focal_x / depth + center_x).trunc() as i32;
    let y = (center_y - view_y * focal_y / depth).trunc() as i32;
    let (x, y) = stabilize_projected_point(x, y);

    ParticleCenterProjection {
        screen: [x, y],
        depth_raw,
        clip: viewport_clip(x, y, viewport),
    }
}

/// Apply the exact post-perspective arithmetic shared by `FUN_004361F0`,
/// `FUN_00436440`, `LAB_00435E70`, and `LAB_00436010`.
///
/// Inputs are the ordinary integer projected coordinates. Offscreen values
/// are repeatedly arithmetic-halved before the sine phase and clip flags are
/// evaluated. The returned coordinates therefore include both that retail
/// stabilization and the final `{-2,-1,0,1}` displacement.
pub fn retail_underwater_projected_point(x: i32, y: i32, tick: i32) -> ProjectedPoint {
    retail_projected_point(x, y, ProjectionEffect::RetailUnderwater { tick })
}

/// Post-perspective arithmetic shared by ordinary 46CD90/46CEB0 and the
/// underwater projectors. The signed abs(X)|abs(Y) control word is latched
/// once; SAR halves both screen coordinates before any low-word stores or
/// primitive construction. Underwater displacement follows that one cap.
/// The incoming camera/lens ownership is independent of this screen stage.
pub fn retail_projected_point(x: i32, y: i32, effect: ProjectionEffect) -> ProjectedPoint {
    let (mut x, mut y) = stabilize_projected_point(x, y);
    if let ProjectionEffect::RetailUnderwater { tick } = effect {
        let (dx, dy) = retail_underwater_offset(tick, x.wrapping_add(y));
        x = x.wrapping_add(dx);
        y = y.wrapping_add(dy);
    }
    ProjectedPoint { x, y }
}

/// Apply the post-perspective effect, low-word stores, and retail clip tests
/// for a top-origin viewport.
pub fn retail_underwater_projection(
    x: i32,
    y: i32,
    tick: i32,
    viewport: [u32; 2],
) -> UnderwaterProjection {
    let full = retail_underwater_projected_point(x, y, tick);
    UnderwaterProjection {
        screen: [full.x as i16, full.y as i16],
        full,
        clip: viewport_clip(full.x, full.y, viewport),
    }
}

fn viewport_clip(x: i32, y: i32, viewport: [u32; 2]) -> u8 {
    let x_clip = if x < 0 {
        0x01
    } else if x >= viewport[0] as i32 {
        0x04
    } else {
        0x02
    };
    let y_clip = if y < 0 {
        0x08
    } else if y >= viewport[1] as i32 {
        0x20
    } else {
        0x10
    };
    x_clip | y_clip
}

/// Build the 64-entry displacement table used by the GL vertex stage for one
/// retail tick. The phase includes `2 * (screen_x + screen_y)`, so only the
/// sum modulo 64 remains after the wrapping 16-bit angle conversion.
pub(crate) fn retail_underwater_offset_table(tick: i32) -> [[f32; 2]; 64] {
    std::array::from_fn(|sum| {
        let (x, y) = retail_underwater_offset(tick, sum as i32);
        [x as f32, y as f32]
    })
}

fn stabilize_projected_point(mut x: i32, mut y: i32) -> (i32, i32) {
    let abs_x = if x < 0 { x.wrapping_neg() } else { x };
    let abs_y = if y < 0 { y.wrapping_neg() } else { y };
    let mut control = abs_x | abs_y;
    while control >= 0x2000 {
        control >>= 1;
        x >>= 1;
        y >>= 1;
    }
    (x, y)
}

fn retail_underwater_offset(tick: i32, screen_sum: i32) -> (i32, i32) {
    let phase_word = tick
        .wrapping_add(screen_sum.wrapping_mul(2))
        .wrapping_mul(0x200) as u32;
    (
        repeated_sine_shift(phase_word.wrapping_add(0x4000)),
        repeated_sine_shift(phase_word),
    )
}

fn repeated_sine_shift(angle: u32) -> i32 {
    retail_sine_q15(angle).wrapping_mul(0x1_0001) >> 30
}

#[cfg(test)]
mod tests {
    use super::{
        project_particle_center, retail_underwater_offset, retail_underwater_offset_table,
        retail_underwater_projected_point, retail_underwater_projection, stabilize_projected_point,
        ParticleCenterProjection, ProjectedPoint, UnderwaterProjection,
    };
    use crate::camera::Camera;

    #[test]
    fn cardinal_phases_preserve_retail_signed_shift_asymmetry() {
        assert_eq!(retail_underwater_offset(0, 0), (1, 0));
        assert_eq!(retail_underwater_offset(0, 16), (0, 1));
        assert_eq!(retail_underwater_offset(0, 32), (-2, 0));
        assert_eq!(retail_underwater_offset(0, 48), (0, -2));
    }

    #[test]
    fn offscreen_stabilization_uses_arithmetic_halves_at_the_strict_boundary() {
        assert_eq!(
            stabilize_projected_point(0x1fff, -0x1fff),
            (0x1fff, -0x1fff)
        );
        assert_eq!(stabilize_projected_point(0x2000, 0), (0x1000, 0));
        assert_eq!(stabilize_projected_point(-0x2001, 1), (-0x1001, 0));
        // The control word is latched before signed coordinate halves. After
        // one shift it is 0x1fff even though SAR rounded the coordinate to the
        // still-boundary-valued -0x2000, so retail stops here.
        assert_eq!(stabilize_projected_point(-0x3fff, 0), (-0x2000, 0));
        assert_eq!(
            stabilize_projected_point(0x8000, -0x8000),
            (0x1000, -0x1000)
        );
        // Retail uses wrapping negation and then a signed comparison. The
        // pathological minimum therefore remains negative and bypasses the
        // loop rather than trapping or being treated as an unsigned absolute.
        assert_eq!(stabilize_projected_point(i32::MIN, 0), (i32::MIN, 0));
    }

    #[test]
    fn phase_wraps_in_the_retail_sixteen_bit_angle_domain() {
        assert_eq!(
            retail_underwater_offset(0, 0),
            retail_underwater_offset(128, 0)
        );
        assert_eq!(
            retail_underwater_offset(-1, 7),
            retail_underwater_offset(127, 7)
        );
        assert_eq!(
            retail_underwater_offset(i32::MAX, 23),
            retail_underwater_offset(i32::MIN + 127, 23)
        );
    }

    #[test]
    fn all_table_offsets_stay_inside_the_recovered_four_value_domain() {
        for tick in [-17, 0, 1, 63, 127, i32::MAX] {
            for [x, y] in retail_underwater_offset_table(tick) {
                assert!(matches!(x as i32, -2..=1));
                assert!(matches!(y as i32, -2..=1));
            }
        }
    }

    #[test]
    fn projected_point_adds_wobble_after_stabilization() {
        assert_eq!(
            retail_underwater_projected_point(0, 0, 0),
            ProjectedPoint { x: 1, y: 0 }
        );
        assert_eq!(
            retail_underwater_projected_point(0x2000, 0, 0),
            ProjectedPoint { x: 4097, y: 0 }
        );
    }

    #[test]
    fn wobble_precedes_full_width_clip_and_low_word_storage() {
        assert_eq!(
            retail_underwater_projection(319, 0, 2, [320, 240]),
            UnderwaterProjection {
                screen: [320, 0],
                full: ProjectedPoint { x: 320, y: 0 },
                clip: 0x14,
            }
        );
        assert_eq!(
            retail_underwater_projection(0, 239, 66, [320, 240]),
            UnderwaterProjection {
                screen: [0, 240],
                full: ProjectedPoint { x: 0, y: 240 },
                clip: 0x22,
            }
        );
        assert_eq!(
            retail_underwater_projection(i32::MIN, 0, 0, [320, 240]),
            UnderwaterProjection {
                screen: [1, 0],
                full: ProjectedPoint {
                    x: i32::MIN + 1,
                    y: 0,
                },
                clip: 0x11,
            }
        );
    }

    #[test]
    fn particle_center_projection_emits_retail_clip_bit_pairs() {
        let mut camera = Camera::new(320.0 / 240.0);
        camera.position = [0.0; 3];
        camera.yaw = 0.0;
        camera.pitch = 0.0;

        assert_eq!(
            project_particle_center(&camera, [320, 240], [0.0, 0.0, -1.0]),
            ParticleCenterProjection {
                screen: [160, 120],
                depth_raw: 256,
                clip: 0x12,
            }
        );
        assert_eq!(
            project_particle_center(&camera, [320, 240], [-2.0, 0.0, -1.0]).clip,
            0x11
        );
        assert_eq!(
            project_particle_center(&camera, [320, 240], [2.0, 0.0, -1.0]).clip,
            0x14
        );
        assert_eq!(
            project_particle_center(&camera, [320, 240], [0.0, 2.0, -1.0]).clip,
            0x0a
        );
        assert_eq!(
            project_particle_center(&camera, [320, 240], [0.0, -2.0, -1.0]).clip,
            0x22
        );
    }

    #[test]
    fn particle_center_projection_uses_the_retail_raw_near_threshold() {
        let mut camera = Camera::new(4.0 / 3.0);
        camera.position = [0.0; 3];
        camera.yaw = 0.0;
        camera.pitch = 0.0;

        let behind = project_particle_center(&camera, [320, 240], [0.0, 0.0, 1.0]);
        assert_eq!(behind.clip, 0x40);
        assert!(behind.depth_raw < 0x40);

        let too_near = project_particle_center(&camera, [320, 240], [0.0, 0.0, -0.24]);
        assert_eq!(too_near.clip, 0x40);
        let accepted = project_particle_center(&camera, [320, 240], [0.0, 0.0, -0.25]);
        assert_eq!(accepted.depth_raw, 0x40);
        assert_eq!(accepted.clip, 0x12);
    }
}

#[cfg(test)]
mod native_screen_tests {
    use super::*;
    fn projection() -> NativeScreenProjection {
        NativeScreenProjection::new([512; 2], [320, 240], [640, 480], ProjectionEffect::None)
            .unwrap()
    }
    #[test]
    fn native_division_precedes_centre_subtraction_for_control_and_warm_retail_slot() {
        assert_eq!(
            projection().project([140, 80, 768]),
            NativeProjectedEndpoint {
                screen: [413, 187],
                depth_raw: 768,
                clip: 0x12
            }
        );
        // Accepted tick952 warmed near-cache slot0, not a cold suffix.
        assert_eq!(
            projection().project([-1844, 100, 2848]),
            NativeProjectedEndpoint {
                screen: [-11, 223],
                depth_raw: 2848,
                clip: 0x11
            }
        );
        assert_eq!(projection().project([-140, -80, 768]).screen, [227, 293]);
    }
    #[test]
    fn native_near_cap_and_extreme_divide_first_keep_source_order() {
        assert_eq!(projection().project([0, 0, 63]).clip, 0x40);
        assert_eq!(projection().project([1024, 0, 64]).screen, [4256, 120]);
        assert_eq!(native_perspective_quotient(0x50000, 512, 64), 0x280000);
        assert_eq!(native_perspective_quotient(-0x50000, 512, 64), -0x280000);
        assert_eq!(projection().project([0x50001, 0, 64]).screen, [5120, 0]);
        assert_eq!(projection().half_width(4, 1280), (1, 1));
        assert_eq!(projection().half_width(-4, 1280), (-1, -1));
    }
    #[test]
    fn native_effect_uses_the_integer_point_once_and_invalid_lens_is_unowned() {
        let native = NativeScreenProjection::new(
            [512; 2],
            [320, 240],
            [640, 480],
            ProjectionEffect::RetailUnderwater { tick: 17 },
        )
        .unwrap();
        let expected = retail_projected_point(413, 187, native.effect);
        assert_eq!(
            native.project([140, 80, 768]).screen,
            [expected.x as i16, expected.y as i16]
        );
        assert!(NativeScreenProjection::new(
            [0, 512],
            [320, 240],
            [640, 480],
            ProjectionEffect::None
        )
        .is_none());
        assert!(NativeScreenProjection::new(
            [512; 2],
            [320, 240],
            [0, 480],
            ProjectionEffect::None
        )
        .is_none());
    }
}

#[cfg(test)]
mod accepted_model302_screen_tests {
    use super::*;
    #[test]
    fn all_eighty_warm_tick952_screen_rows_match_the_actual_native_projector() {
        // V200001 tick952 model302, accepted raw lens512/512/640/480/320/240.
        // Source VIEW/cache and original46CD90 independently agree for these
        // 80 warmed projected rows. No cold or VIEW-only screen is borrowed.
        let expected: [(u16, [i32; 3], [i16; 2], u8); 80] = [
            (0, [-1844, 100, 2848], [-11, 223], 17),
            (2, [-1865, 98, 2830], [-17, 223], 17),
            (3, [-1909, 122, 2882], [-19, 219], 17),
            (4, [-1906, 103, 2823], [-25, 222], 17),
            (6, [-1850, 124, 2851], [-12, 218], 17),
            (7, [-1874, 138, 2883], [-12, 216], 17),
            (8, [-1877, 122, 2829], [-19, 218], 17),
            (9, [-1901, 136, 2861], [-20, 216], 17),
            (10, [-1909, 111, 2825], [-25, 220], 17),
            (11, [-1917, 117, 2837], [-25, 219], 17),
            (12, [-1851, 88, 2866], [-10, 225], 17),
            (14, [-1884, 83, 2842], [-19, 226], 17),
            (16, [-1912, 98, 2830], [-25, 223], 17),
            (18, [-1900, 93, 2794], [-28, 223], 17),
            (20, [-1904, 123, 2795], [-28, 218], 17),
            (21, [-1938, 143, 2837], [-29, 215], 17),
            (22, [-1912, 73, 2811], [-28, 227], 17),
            (24, [-1921, 89, 2767], [-35, 224], 17),
            (26, [-1927, 127, 2769], [-36, 217], 17),
            (27, [-1965, 149, 2815], [-37, 213], 17),
            (28, [-1940, 61, 2792], [-35, 229], 17),
            (30, [-1975, 78, 2754], [-47, 226], 17),
            (31, [-2009, 98, 2796], [-47, 223], 17),
            (32, [-1980, 103, 2757], [-47, 221], 17),
            (33, [-1996, 113, 2779], [-47, 220], 17),
            (34, [-1976, 63, 2779], [-44, 229], 17),
            (36, [-1986, 70, 2768], [-47, 228], 17),
            (37, [-2002, 80, 2790], [-47, 226], 17),
            (38, [-1829, 115, 2882], [-4, 220], 17),
            (42, [-1831, 124, 2884], [-5, 218], 17),
            (43, [-1841, 130, 2898], [-5, 218], 17),
            (44, [-1836, 101, 2892], [-5, 223], 17),
            (46, [-1799, 112, 2890], [2, 221], 18),
            (48, [-1808, 135, 2887], [0, 217], 18),
            (49, [-1828, 147, 2913], [-1, 215], 17),
            (50, [-1819, 101, 2893], [-1, 223], 17),
            (52, [-1777, 136, 2905], [7, 217], 18),
            (53, [-1805, 152, 2941], [6, 214], 18),
            (54, [-1796, 135, 2893], [3, 217], 18),
            (55, [-1820, 149, 2925], [2, 214], 18),
            (58, [-1779, 104, 2932], [10, 222], 18),
            (59, [-1795, 114, 2954], [9, 221], 18),
            (60, [-1788, 98, 2923], [7, 223], 18),
            (78, [-1758, 13, 2931], [13, 238], 18),
            (79, [-1807, 43, 2994], [11, 233], 18),
            (80, [-1788, 7, 2894], [4, 239], 18),
            (81, [-1839, 35, 2956], [2, 234], 18),
            (82, [-1864, -3, 2833], [-16, 240], 17),
            (83, [-1914, 26, 2894], [-18, 236], 17),
            (84, [-1956, -28, 2767], [-41, 245], 17),
            (85, [-2008, 8, 2827], [-43, 239], 17),
            (86, [-1844, 100, 2848], [-11, 223], 17),
            (87, [-1886, 124, 2900], [-12, 219], 17),
            (100, [-1768, 68, 2826], [0, 228], 18),
            (102, [-1780, 58, 2849], [1, 230], 18),
            (104, [-1823, 68, 2773], [-16, 228], 17),
            (106, [-1878, 112, 2983], [-2, 221], 17),
            (108, [-1931, 130, 2970], [-12, 218], 17),
            (110, [-1968, 145, 2921], [-24, 215], 17),
            (112, [-1721, -26, 2835], [10, 244], 18),
            (114, [-1686, -10, 2875], [20, 241], 18),
            (116, [-1819, -29, 2779], [-15, 245], 17),
            (118, [-1866, 85, 3091], [11, 226], 18),
            (120, [-1966, 107, 3084], [-6, 223], 17),
            (122, [-2003, 65, 2966], [-25, 229], 17),
            (128, [-1733, 139, 2916], [16, 216], 18),
            (130, [-1789, 173, 2972], [12, 211], 18),
            (132, [-1819, 5, 2877], [-3, 240], 17),
            (133, [-1863, 30, 2928], [-5, 235], 17),
            (134, [-1839, 2, 2860], [-9, 240], 17),
            (135, [-1883, 27, 2913], [-10, 236], 17),
            (136, [-1726, -26, 2830], [8, 244], 18),
            (138, [-1688, -18, 2874], [20, 243], 18),
            (140, [-1823, -32, 2775], [-16, 245], 17),
            (142, [-1869, 81, 3089], [11, 227], 18),
            (144, [-1970, 105, 3081], [-7, 223], 17),
            (146, [-2006, 65, 2962], [-26, 229], 17),
            (148, [-1860, 99, 2835], [-15, 223], 17),
            (149, [-1902, 123, 2887], [-17, 219], 17),
            (152, [-1749, 102, 2956], [18, 223], 18),
        ];
        let projection =
            NativeScreenProjection::new([512; 2], [320, 240], [640, 480], ProjectionEffect::None)
                .unwrap();
        for (slot, view, screen, clip) in expected {
            let actual = projection.project(view);
            assert_eq!(
                actual.screen, screen,
                "native warmed slot{slot} VIEW{view:?}"
            );
            assert_eq!(actual.clip, clip, "native warmed slot{slot} clip");
        }
    }
}
