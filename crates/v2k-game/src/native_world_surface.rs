//! Source world tf13 (4349C0) over an authenticated cached VIEW point.
use crate::native_model_frame::NativeWorldViewport;
use v2k_formats::models::ModelSlotClip;
use v2k_render::WorldSurfaceProjection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeWorldSurfacePoint {
    pub view_raw: [i32; 3],
    pub world_raw: [i32; 3],
    pub clip: ModelSlotClip,
}

fn mul_q31(value: i32, axis: i32) -> i32 {
    ((i64::from(value) * i64::from(axis)) >> 31) as i32
}

/// 4340B0 uses the same cached VIEW/inverse/forward custody with plain
/// Section10 bilinear height and no wave, directional correction or clip band.
pub fn project_native_world_alias(
    source_view_raw: [i32; 3],
    viewport: NativeWorldViewport,
    surface: WorldSurfaceProjection<'_>,
) -> NativeWorldSurfacePoint {
    let mut world = viewport.view_point_to_world(source_view_raw);
    world[1] = i32::from(
        surface
            .terrain()
            .bilinear_height_raw(world[0] as i16, world[2] as i16),
    );
    let relative = std::array::from_fn(|axis| world[axis].wrapping_sub(viewport.origin_raw[axis]));
    NativeWorldSurfacePoint {
        view_raw: viewport.world_vector_to_view(relative),
        world_raw: world,
        clip: ModelSlotClip::Clear,
    }
}

/// Every source VIEW component first goes through the inverse viewport, and
/// the final callback writes a freshly forward-projected VIEW cache. X/Z stay
/// dwords except for toroidal terrain sampling; surface heights are signed
/// shorts. Sea-band rejection still writes that reprojected cached position.
pub fn project_native_world_surface(
    source_view_raw: [i32; 3],
    viewport: NativeWorldViewport,
    surface: WorldSurfaceProjection<'_>,
) -> NativeWorldSurfacePoint {
    let world = viewport.view_point_to_world(source_view_raw);
    let mut relative =
        std::array::from_fn(|axis| world[axis].wrapping_sub(viewport.origin_raw[axis]));
    let sample = |relative: [i32; 3]| {
        let x = viewport.origin_raw[0].wrapping_add(relative[0]) as i16;
        let z = viewport.origin_raw[2].wrapping_add(relative[2]) as i16;
        let terrain = surface.terrain().bilinear_height_raw(x, z);
        (x, z, terrain)
    };
    let (x, z, terrain) = sample(relative);
    let sea = surface.terrain().sea_level_raw();
    let world_y = viewport.origin_raw[1].wrapping_add(relative[1]);
    let above = i32::from(sea) < world_y.wrapping_sub(0x96);
    let use_wave = above || terrain > sea;
    let clip;
    if !use_wave && i32::from(sea) <= world_y.wrapping_add(0x96) {
        clip = ModelSlotClip::SurfaceBand;
    } else {
        let initial = if use_wave {
            surface.wave_surface_raw(x, z, terrain)
        } else {
            terrain
        };
        let delta = viewport.origin_raw[1]
            .wrapping_sub(i32::from(initial))
            .wrapping_add(relative[1]);
        let [slope_x, slope_z] = surface.slopes_q31();
        relative[0] = relative[0].wrapping_add(mul_q31(delta, slope_x));
        relative[2] = relative[2].wrapping_add(mul_q31(delta, slope_z));
        let (x, z, terrain) = sample(relative);
        let height = if use_wave {
            surface.wave_surface_raw(x, z, terrain)
        } else {
            terrain
        };
        relative[1] = i32::from(height).wrapping_sub(viewport.origin_raw[1]);
        clip = ModelSlotClip::Clear;
    }
    NativeWorldSurfacePoint {
        view_raw: viewport.world_vector_to_view(relative),
        world_raw: std::array::from_fn(|axis| {
            viewport.origin_raw[axis].wrapping_add(relative[axis])
        }),
        clip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::{TerrainCell, TerrainGrid, GRID_SIZE};

    #[test]
    fn shadow_surface_matches_original_machine_wet_wave_and_inclusive_band_controls() {
        let terrain = TerrainGrid {
            header: [0, -4, 4, 8, 0],
            cells: vec![
                TerrainCell {
                    height: (-10_i8) as u8,
                    attribute: 0,
                    terrain_type: 0
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let viewport = NativeWorldViewport {
            origin_raw: [0; 3],
            axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            identity: true,
        };
        let surface = WorldSurfaceProjection::new(&terrain, 1000);
        assert_eq!(surface.slopes_q31(), [536870912, -1073741824]);
        // Literal VIEW words and clip80/clipC0 come from interpreted original
        // 4349C0 -> 45860/45920 instructions in the verified PE. The source
        // tf14 cache is warm; this fixture does not substitute a live D360.
        for (y, enabled, expected_view, expected_clip) in [
            (-300, false, [105, -320, 190], ModelSlotClip::Clear),
            (-300, true, [105, -320, 190], ModelSlotClip::Clear),
            (-151, false, [142, -320, 115], ModelSlotClip::Clear),
            (-151, true, [142, -320, 115], ModelSlotClip::Clear),
            (-150, false, [100, -150, 200], ModelSlotClip::SurfaceBand),
            (-150, true, [100, -150, 200], ModelSlotClip::SurfaceBand),
            (0, false, [100, 0, 200], ModelSlotClip::SurfaceBand),
            (0, true, [100, 0, 200], ModelSlotClip::SurfaceBand),
            (150, false, [100, 150, 200], ModelSlotClip::SurfaceBand),
            (150, true, [100, 150, 200], ModelSlotClip::SurfaceBand),
            (151, false, [137, 0, 124], ModelSlotClip::Clear),
            (151, true, [139, -11, 121], ModelSlotClip::Clear),
            (300, false, [175, 0, 50], ModelSlotClip::Clear),
            (300, true, [176, -12, 47], ModelSlotClip::Clear),
        ] {
            let projected = project_native_world_surface(
                [100, y, 200],
                viewport,
                surface.with_waves_enabled(enabled),
            );
            assert_eq!(projected.view_raw, expected_view, "y={y}, waves={enabled}");
            assert_eq!(
                projected.world_raw, expected_view,
                "identity viewport world cache"
            );
            assert_eq!(projected.clip, expected_clip, "y={y}, waves={enabled}");
        }
    }

    #[v2k_test_support::retail_test]
    fn actual_world13_terrain_and_nonidentity_view_shadow_match_original_machine() {
        let data = v2k_test_support::retail_dir();
        assert!(
            data.join("PRELOAD.DAT").is_file(),
            "retail shadow corpus is required"
        );
        let mut session = crate::session::GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(13, 1).unwrap();
        let terrain = session.cache.terrain().unwrap();
        assert_eq!(terrain.header, [-216832, -73, 73, -73, 115866688]);
        let viewport = NativeWorldViewport {
            origin_raw: [17324, 2336, 31398],
            axes_q31: [
                [2147345377, 0, 13827079],
                [-4394708, 2035509248, 682498122],
                [-13168014, -682542069, 2035555527],
            ],
            identity: false,
        };
        let surface = WorldSurfaceProjection::new(terrain, 1000).with_waves_enabled(true);
        assert_eq!(surface.slopes_q31(), [536870912; 2]);
        // Accepted viewport plus actual full Section10 bytes; controlled
        // signed callback outputs. These are arithmetic controls, not matched
        // actor/camera timestamps or a captured live D360 return.
        for (source_words, expected_cache, expected_shadow) in [
            (
                [-18688, -462, -32760],
                [29530, -2277, 2013],
                [29537, -2313, 2033],
            ),
            (
                [-18688, -462, 32512],
                [29529, -2360, 1762],
                [29538, -2401, 1784],
            ),
            (
                [-32768, 32767, 32767],
                [15451, 29247, -8470],
                [23888, 434, 9972],
            ),
        ] {
            let image = viewport.actor_world_image(source_words);
            let relative =
                std::array::from_fn(|axis| image[axis].wrapping_sub(viewport.origin_raw[axis]));
            let cached = viewport.world_vector_to_view(relative);
            assert_eq!(cached, expected_cache, "40D350/6ECF0 cache prefix");
            let projected = project_native_world_surface(cached, viewport, surface);
            assert_eq!(
                projected.view_raw, expected_shadow,
                "4349C0 with actual terrain"
            );
            assert_eq!(projected.clip, ModelSlotClip::Clear);
        }
    }
}
