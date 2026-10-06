//! Main-model scan regressions at the canonical first-world buildings.

use v2k_game::entity_view_detail::{RetailViewDetail, RetailViewDetailContext};
use v2k_game::session::GameSession;
use v2k_render::Camera;

struct Buildings {
    positions_raw: [[i16; 3]; 2],
    scan: (u32, u32),
}

fn first_world_buildings() -> Buildings {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("present preload must parse");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session
        .load_level_by_id(13, 1)
        .expect("first-world overlay");
    let level = session.cache.level_desc().expect("level descriptor");
    let positions_raw = [(6, 6), (23, 66)].map(|(index, entity_type)| {
        let spawn = level
            .entities
            .iter()
            .find(|spawn| spawn.index == index)
            .unwrap();
        assert_eq!(spawn.entity_type, entity_type);
        spawn.position_raw()
    });
    Buildings {
        positions_raw,
        scan: v2k_render::terrain_tiles::scan_dimensions(level.terrain_draw_depth),
    }
}

#[v2k_test_support::retail_test]
fn diagonal_buildings_survive_the_world_axis_window_at_different_camera_heights() {
    let buildings = first_world_buildings();
    let dx = buildings.scan.0 as f32 / 2.0 - 1.0;
    let dz = buildings.scan.1 as f32 - 1.0;
    let horizontal_distance = dx.hypot(dz);
    assert!(horizontal_distance > buildings.scan.1 as f32);

    for building in buildings.positions_raw {
        let position = building.map(|value| f32::from(value) / 256.0);
        for side in [-1.0, 1.0] {
            for height in [0.0, 12.0] {
                let mut camera = Camera::new(16.0 / 9.0);
                camera.position = [
                    position[0] - side * dx,
                    position[1] + height,
                    position[2] - dz,
                ];
                camera.yaw = (side * dx).atan2(-dz);
                camera.pitch = (-height).atan2(horizontal_distance);
                camera.left_handed = true;
                let context = RetailViewDetailContext::from_world(
                    camera.position,
                    camera.view_matrix()[9],
                    buildings.scan,
                );
                assert_eq!(context.classify(building), RetailViewDetail::Full);

                // The removed camera-yaw rectangle used this normalized XZ
                // distance as its forward coordinate and rejected the whole
                // building, despite dx/dz both passing 411400's world bounds.
                let forward = camera.forward();
                let along =
                    (side * dx * forward[0] + dz * forward[2]) / forward[0].hypot(forward[2]);
                assert!(along > buildings.scan.1 as f32);
            }
        }
    }
}

#[v2k_test_support::retail_test]
fn building_scan_boundaries_are_inclusive_without_model_radius_expansion() {
    let buildings = first_world_buildings();
    let half_width_raw = (buildings.scan.0 as i32 / 2) * 256;
    let depth_raw = buildings.scan.1 as i32 * 256;
    for building in buildings.positions_raw {
        for (dx, dz, expected) in [
            (half_width_raw, depth_raw, RetailViewDetail::Full),
            (-half_width_raw, depth_raw, RetailViewDetail::Full),
            (half_width_raw + 1, depth_raw, RetailViewDetail::Broader),
            (-half_width_raw - 1, depth_raw, RetailViewDetail::Broader),
            (0, depth_raw + 1, RetailViewDetail::Broader),
            (0, depth_raw + 4 * 256, RetailViewDetail::Coarse),
        ] {
            let camera = [
                i32::from(building[0]) - dx,
                i32::from(building[1]),
                i32::from(building[2]) - dz,
            ];
            assert_eq!(
                RetailViewDetailContext::from_raw(camera, 0, buildings.scan).classify(building),
                expected,
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn elevated_building_rear_plane_and_torus_images_share_the_same_classification() {
    let buildings = first_world_buildings();
    for building in buildings.positions_raw {
        // 4114B0..CD: dy=-1280, true-up Z=0.5 gives rear plane -640.
        // There is no invented -rows lower bound and equality is admitted.
        let camera = [
            i32::from(building[0]),
            i32::from(building[1]) + 1280,
            i32::from(building[2]) + 640,
        ];
        for image in [-65536, 0, 65536] {
            let wrapped_camera = [camera[0] + image, camera[1], camera[2] - image];
            assert_eq!(
                RetailViewDetailContext::from_raw(wrapped_camera, 0x4000_0000, buildings.scan)
                    .classify(building),
                RetailViewDetail::Full,
            );
            let beyond = [wrapped_camera[0], wrapped_camera[1], wrapped_camera[2] + 1];
            assert_eq!(
                RetailViewDetailContext::from_raw(beyond, 0x4000_0000, buildings.scan)
                    .classify(building),
                RetailViewDetail::Broader,
            );
        }
        assert_eq!(
            RetailViewDetailContext::from_raw(
                [camera[0], i32::from(building[1]), camera[2]],
                0x4000_0000,
                buildings.scan
            )
            .classify(building),
            RetailViewDetail::Broader,
            "a level camera does not retain the elevated rear allowance",
        );
    }
}
