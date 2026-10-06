//! Actual Section13 far/width fields across all four retail display tiers.
use v2k_formats::{levels::LevelFogPlanes, ovl::OvlFile, sections};

#[v2k_test_support::retail_test]
fn retail_world_fog_uses_each_authored_width_across_display_tiers() {
    let root = v2k_test_support::retail_dir().join("Overlay");
    let widths = [
        8, 6, 10, 5, 7, 8, 6, 12, 5, 10, 8, 10, 2, 10, 2, 2, 29, 6, 5, 12, 12, 12, 4, 4, 5, 12, 8,
        7, 10, 7, 12, 1, 12, 11, 10, 4, 5, 12,
    ];
    let fars = [
        21, 22, 22, 29, 25, 24, 26, 20, 25, 20, 21, 21, 30, 25, 30, 30, 30, 23, 29, 18, 22, 25, 28,
        27, 28, 22, 25, 27, 30, 24, 27, 30, 24, 23, 24, 25, 29, 24,
    ];
    let mut checked = 0;
    for tier in 0..=3 {
        for (offset, (far, width)) in fars.into_iter().zip(widths).enumerate() {
            let id = offset + 13;
            let path = root.join(format!("{tier}X{id}XX.OVL"));
            assert!(
                path.is_file(),
                "required retail fog corpus missing: {}",
                path.display()
            );
            let ovl = OvlFile::parse(&std::fs::read(&path).unwrap()).unwrap();
            let descriptor = sections::parse_level(&ovl).unwrap();
            assert_eq!(descriptor.terrain_draw_depth, far, "tier{tier}/world{id}");
            assert_eq!(descriptor.fog_width_cells(), width, "tier{tier}/world{id}");
            assert_eq!(
                descriptor.fog_planes_raw(),
                LevelFogPlanes {
                    near_raw: ((far - width) * 256) as i32,
                    far_raw: (far * 256) as i32,
                },
                "tier{tier}/world{id}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 152);
}
