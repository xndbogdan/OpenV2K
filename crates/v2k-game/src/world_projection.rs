//! Authored world projection from system level 2's six Section-0 scalars.
//!
//! `FUN_00453570` copies the focal lengths, viewport bounds and centre into
//! `0x004FEED0..0x004FEEE4`. Its session `+0x26C` focal multiplier starts at
//! `0x10000` (`FUN_0044FA30`, store at `0x0044FB0E`), so ordinary gameplay
//! uses the authored lengths unchanged. The frontend display reader
//! `FUN_0042B870` installs the same six words; Intro2 `FUN_0042AF00`
//! publishes the current context block before model traversal.
//! This scene adapter leaves the generic viewer camera and menu lens alone.

use crate::system_layout::SystemLayoutSource;
use v2k_render::projection::{
    NativeScreenProjection, ProjectionAuthorityMissing, ProjectionCompatibilityReason,
    ProjectionEffect, SceneProjectionAuthority,
};
use v2k_render::Camera;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldProjection {
    focal_pixels: [i32; 2],
    viewport_pixels: [i32; 2],
    centre_pixels: [i32; 2],
}

impl WorldProjection {
    /// Read the selected system-2 tier, including PRELOAD's authored low tier
    /// when it is the active fallback. Other systems' Section-0 data cannot
    /// substitute for this display block.
    pub fn from_cache(cache: &impl SystemLayoutSource) -> Option<Self> {
        let mut words = [0; 6];
        for (index, word) in words.iter_mut().enumerate() {
            *word = cache.system_data_value(2, index)? as i32;
        }
        Self::from_words(words)
    }

    fn from_words(words: [i32; 6]) -> Option<Self> {
        if words[..4].iter().any(|&value| value <= 0) {
            return None;
        }
        Some(Self {
            focal_pixels: [words[0], words[1]],
            viewport_pixels: [words[2], words[3]],
            centre_pixels: [words[4], words[5]],
        })
    }

    /// The six original system-2 words, with no GL conversion or resize.
    pub fn authored_words(self) -> [i32; 6] {
        [
            self.focal_pixels[0],
            self.focal_pixels[1],
            self.viewport_pixels[0],
            self.viewport_pixels[1],
            self.centre_pixels[0],
            self.centre_pixels[1],
        ]
    }

    /// Install the authored lens in the renderer's logical viewport, retaining
    /// the camera's pose, handedness and clipping planes. Native widescreen
    /// extends the horizontal view at the same vertical scale; 4:3/Stretched
    /// supply their 4:3 logical viewport before the backend's final mapping.
    pub fn apply_to(self, camera: &mut Camera, viewport: (u32, u32)) {
        let aspect = viewport.0.max(1) as f32 / viewport.1.max(1) as f32;
        let [focal_x, focal_y] = self.focal_pixels.map(|value| value as f32);
        let [width, height] = self.viewport_pixels.map(|value| value as f32);
        let [centre_x, centre_y] = self.centre_pixels.map(|value| value as f32);

        camera.fov = 2.0 * (height / (2.0 * focal_y)).atan();
        camera.aspect = aspect * focal_y / focal_x;
        // Projection offsets occupy the matrix's eye-Z column (visible Z is
        // negative), hence the opposite X/Y signs for top-origin pixels.
        camera.projection_offset = [
            (width - 2.0 * centre_x) / (height * aspect),
            2.0 * centre_y / height - 1.0,
        ];
    }
}

/// Publish the source screen-stage inputs for an authenticated native view.
///
/// The scene supplies the same selected display block used by `apply_to`.
/// Only its exact authored logical size admits the raw retail lens; Free and
/// the named resize/widescreen adapter keep their existing compatibility path.
/// A claimed native scene with missing lens or view custody remains explicit.
pub fn scene_projection_authority(
    authored: Option<WorldProjection>,
    native_view_owned: bool,
    native_view_requested: bool,
    logical_viewport: (u32, u32),
    effect: ProjectionEffect,
) -> SceneProjectionAuthority {
    if !native_view_requested {
        return SceneProjectionAuthority::Compatibility(ProjectionCompatibilityReason::Free);
    }
    let Some(authored) = authored else {
        return SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeLens);
    };
    if authored.viewport_pixels.map(|value| value as u32)
        != [logical_viewport.0, logical_viewport.1]
    {
        return SceneProjectionAuthority::Compatibility(
            ProjectionCompatibilityReason::LogicalViewportAdapter,
        );
    }
    if !native_view_owned {
        return SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeView);
    }
    match NativeScreenProjection::new(
        authored.focal_pixels,
        authored.centre_pixels,
        authored.viewport_pixels,
        effect,
    ) {
        Some(projection) => SceneProjectionAuthority::Native(projection),
        None => SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeLens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_cache::ResourceCache;

    fn project(camera: &Camera, point: [f32; 3], viewport: (u32, u32)) -> [f32; 2] {
        let matrix = camera.projection_matrix();
        let depth = -point[2];
        let x = (matrix[0] * point[0] + matrix[8] * point[2]) / depth;
        let y = (matrix[5] * point[1] + matrix[9] * point[2]) / depth;
        [
            (x + 1.0) * viewport.0 as f32 * 0.5,
            (1.0 - y) * viewport.1 as f32 * 0.5,
        ]
    }

    fn assert_point(actual: [f32; 2], expected: [f32; 2]) {
        for axis in 0..2 {
            assert!(
                (actual[axis] - expected[axis]).abs() < 0.001,
                "{actual:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn normal_tier_matches_retail_focal_algebra_in_each_logical_viewport() {
        let projection = WorldProjection::from_words([512, 512, 640, 480, 320, 240]).unwrap();
        let mut camera = Camera::new(4.0 / 3.0);
        for (viewport, expected) in [
            ((640, 480), [371.2, 209.28]),
            ((1920, 1080), [1075.2, 470.88]),
            ((2560, 1080), [1395.2, 470.88]),
        ] {
            projection.apply_to(&mut camera, viewport);
            assert_point(project(&camera, [100.0, 60.0, -1000.0], viewport), expected);
        }
        assert!((camera.fov.to_degrees() - 50.229_67).abs() < 0.0001);
        assert_eq!(Camera::new(1.0).fov, 60.0_f32.to_radians());
    }

    #[test]
    fn unequal_focals_and_off_centre_pixels_survive_native_resize() {
        let projection = WorldProjection::from_words([600, 450, 640, 480, 300, 260]).unwrap();
        let mut camera = Camera::new(1.0);
        projection.apply_to(&mut camera, (1280, 720));
        assert_point(
            project(&camera, [0.0, 0.0, -1000.0], (1280, 720)),
            [610.0, 390.0],
        );
        assert_point(
            project(&camera, [100.0, 60.0, -1000.0], (1280, 720)),
            [700.0, 349.5],
        );
        projection.apply_to(&mut camera, (640, 480));
        assert_point(
            project(&camera, [100.0, 60.0, -1000.0], (640, 480)),
            [360.0, 233.0],
        );
    }

    #[test]
    fn applying_lens_preserves_chase_or_free_camera_pose_and_clip_planes() {
        let projection = WorldProjection::from_words([512, 512, 640, 480, 320, 240]).unwrap();
        let mut camera = Camera::new(1.0);
        camera.position = [75.0, -1.0, 52.0];
        camera.yaw = 0.7;
        camera.pitch = -0.2;
        camera.left_handed = true;
        camera.near = 0.2;
        camera.far = 75.0;
        let view = camera.view_matrix();
        projection.apply_to(&mut camera, (1920, 1080));
        assert_eq!(camera.view_matrix(), view);
        assert_eq!((camera.near, camera.far), (0.2, 75.0));
        projection.apply_to(&mut camera, (0, 0));
        assert!(camera
            .projection_matrix()
            .iter()
            .all(|value| value.is_finite()));
    }

    #[test]
    fn missing_or_invalid_display_block_does_not_fall_back_to_a_guessed_lens() {
        assert!(WorldProjection::from_cache(&ResourceCache::new(Vec::new())).is_none());
        for index in 0..4 {
            for invalid in [0, -1] {
                let mut words = [512, 512, 640, 480, 320, 240];
                words[index] = invalid;
                assert!(WorldProjection::from_words(words).is_none());
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn canonical_display_tiers_keep_authored_focal_rounding_and_pool_ownership() {
        let root = v2k_test_support::retail_dir().join("Overlay");
        assert!(
            root.join("1X2XX.OVL").is_file(),
            "retail display corpus required"
        );
        let mut cache = ResourceCache::new(Vec::new());
        for (tier, words) in [
            (0, [256, 256, 320, 240, 160, 120]),
            (1, [512, 512, 640, 480, 320, 240]),
            (2, [640, 640, 800, 600, 400, 300]),
            (3, [820, 820, 1024, 768, 512, 384]),
        ] {
            let mut layer =
                crate::loader::load_level(&root.join(format!("{tier}X2XX.OVL"))).unwrap();
            layer.system_level = Some(2);
            cache.add_auxiliary(layer);
            // A later pool has unrelated Section-0 values. Only the owning
            // system-2 layer may supply the lens, even when another is newer.
            let mut unrelated =
                crate::loader::load_level(&root.join(format!("{tier}X2XX.OVL"))).unwrap();
            unrelated.system_level = Some(3);
            unrelated.fixup_data.as_mut().unwrap().entries[..6].fill(1);
            cache.add_auxiliary(unrelated);
            let selected = WorldProjection::from_cache(&cache).unwrap();
            assert_eq!(Some(selected), WorldProjection::from_words(words));
            assert_eq!(selected.authored_words(), words);
            assert_eq!(
                scene_projection_authority(
                    Some(selected),
                    true,
                    true,
                    (words[2] as u32, words[3] as u32),
                    ProjectionEffect::None,
                ),
                SceneProjectionAuthority::Native(
                    NativeScreenProjection::new(
                        [words[0], words[1]],
                        [words[4], words[5]],
                        [words[2], words[3]],
                        ProjectionEffect::None,
                    )
                    .unwrap()
                ),
                "tier {tier} keeps its owning raw display words",
            );
        }
    }

    #[test]
    fn native_scene_preserves_unequal_focals_off_center_and_effect() {
        let authored = WorldProjection::from_words([600, 450, 640, 480, 300, 260]).unwrap();
        for effect in [
            ProjectionEffect::None,
            ProjectionEffect::RetailUnderwater { tick: 137 },
        ] {
            let SceneProjectionAuthority::Native(projection) =
                scene_projection_authority(Some(authored), true, true, (640, 480), effect)
            else {
                panic!("owned authored display must reach native screen stage");
            };
            assert_eq!(projection.viewport_pixels(), [640, 480]);
            assert_eq!(projection.effect, effect);
            if effect == ProjectionEffect::None {
                assert_eq!(projection.project([100, 60, 1000]).screen, [360, 233]);
            }
        }
        let outside_center = WorldProjection::from_words([512, 512, 640, 480, -7, 511]).unwrap();
        let SceneProjectionAuthority::Native(projection) = scene_projection_authority(
            Some(outside_center),
            true,
            true,
            (640, 480),
            ProjectionEffect::None,
        ) else {
            panic!("source centers are signed scalars, not viewport validation bounds");
        };
        assert_eq!(projection.project([0, 0, 1000]).screen, [-7, 511]);
    }

    #[test]
    fn changed_logical_viewport_stays_explicit_compatibility() {
        let authored = WorldProjection::from_words([512, 512, 640, 480, 320, 240]).unwrap();
        for logical in [(1280, 960), (1280, 720), (640, 481), (0, 0)] {
            for view_owned in [true, false] {
                assert_eq!(
                    scene_projection_authority(
                        Some(authored),
                        view_owned,
                        true,
                        logical,
                        ProjectionEffect::None,
                    ),
                    SceneProjectionAuthority::Compatibility(
                        ProjectionCompatibilityReason::LogicalViewportAdapter,
                    )
                );
            }
        }
    }

    #[test]
    fn free_scene_does_not_claim_missing_native_inputs() {
        assert_eq!(
            scene_projection_authority(None, false, false, (640, 480), ProjectionEffect::None,),
            SceneProjectionAuthority::Compatibility(ProjectionCompatibilityReason::Free)
        );
        let authored = WorldProjection::from_words([512, 512, 640, 480, 320, 240]).unwrap();
        assert_eq!(
            scene_projection_authority(
                Some(authored),
                true,
                false,
                (640, 480),
                ProjectionEffect::None,
            ),
            SceneProjectionAuthority::Compatibility(ProjectionCompatibilityReason::Free)
        );
    }

    #[test]
    fn claimed_native_scene_reports_distinct_missing_lens_and_view_owners() {
        assert_eq!(
            scene_projection_authority(None, true, true, (640, 480), ProjectionEffect::None,),
            SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeLens)
        );
        let authored = WorldProjection::from_words([512, 512, 640, 480, 320, 240]).unwrap();
        assert_eq!(
            scene_projection_authority(
                Some(authored),
                false,
                true,
                (640, 480),
                ProjectionEffect::None,
            ),
            SceneProjectionAuthority::Missing(ProjectionAuthorityMissing::NativeView)
        );
    }
}
