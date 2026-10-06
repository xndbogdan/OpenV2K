use super::*;
use crate::retail_rng::retail_random_u16;
use crate::session::GameSession;
use crate::system_layout::HighSystemLayoutTier;

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(2, 1).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let mut rng = 0x1234_5678;
    session
        .cache
        .initialize_level_terrain_radar(&mut || retail_random_u16(&mut rng))
        .unwrap();
    session
}

#[v2k_test_support::retail_test]
fn authored_radar_refresh_preserves_map_lifecycle_raster_icons_and_evaluated_globe() {
    let mut session = session();
    let mut presenter = GameplayRadar::from_cache(&session.cache, 1).unwrap();
    let old_layout = presenter.layout();
    let terrain = session.cache.level_terrain_radar().unwrap();
    let before_coverage = terrain.packed_coverage().to_vec();
    let before_indices = terrain.indices().to_vec();
    let before_revision = terrain.revision();
    let coverage_address = terrain.packed_coverage().as_ptr();
    let indices_address = terrain.indices().as_ptr();
    let icons = presenter.map_icons.clone();
    let terrain_palette = presenter.palette.terrain;
    let marker_palette = presenter.palette.markers;
    let shade_grid = presenter.projection.shade_grid.clone();
    let mut frame = RadarHudFrame {
        image: project_globe(
            &presenter.projection,
            &presenter.palette,
            terrain.indices(),
            [0; 3],
        ),
        origin: old_layout.hud_origin,
        virtual_size: old_layout.virtual_size,
    };
    // An already accepted marker decision is represented in these cached pixels.
    frame.image.rgba[0..4].copy_from_slice(&[1, 2, 3, 255]);
    let original_frame = frame.clone();
    presenter.enter_fullscreen();
    for _ in 0..3 {
        presenter.advance_fullscreen(20_000);
    }
    assert!(presenter.fullscreen_image(terrain).is_some());
    assert_eq!(presenter.lifecycle.pass, 3);
    presenter.advance_fullscreen(20_000);
    presenter.advance_fullscreen(72_321);
    let lifecycle = presenter.lifecycle;
    let mut previous = old_layout;
    for tier in [
        HighSystemLayoutTier::High800,
        HighSystemLayoutTier::High1024,
        HighSystemLayoutTier::High640,
    ] {
        let stage = session.prepare_high_system_layout_refresh(tier).unwrap();
        let next = GameplayRadarLayout::from_cache(&stage, tier.variant()).unwrap();
        let prepared = presenter.prepare_layout_refresh(next).unwrap();
        frame.relayout(previous, next).unwrap();
        presenter.refresh_layout(prepared);
        assert!(
            presenter.fullscreen_raster.is_none(),
            "changed dimensions discard only derived fullscreen images"
        );
        assert_eq!(presenter.lifecycle, lifecycle);
        assert_eq!(presenter.layout(), next);
        assert_eq!(presenter.map_icons, icons);
        assert_eq!(presenter.palette.terrain, terrain_palette);
        assert_eq!(presenter.palette.markers, marker_palette);
        assert_eq!(presenter.projection.shade_grid, shade_grid);
        assert_eq!(
            frame.image, original_frame.image,
            "no globe projection or marker redraw"
        );
        let image = presenter.fullscreen_image(terrain).unwrap();
        assert_eq!(
            (image.width, image.height),
            (next.map_rect.width, next.map_rect.height)
        );
        let image_address = image.rgba.as_ptr();
        let same = presenter.prepare_layout_refresh(next).unwrap();
        presenter.refresh_layout(same);
        assert_eq!(
            presenter.fullscreen_image(terrain).unwrap().rgba.as_ptr(),
            image_address,
            "same-size refresh retains its derived raster allocation"
        );
        let current = session.cache.level_terrain_radar().unwrap();
        assert_eq!(current.packed_coverage(), before_coverage);
        assert_eq!(current.indices(), before_indices);
        assert_eq!(current.revision(), before_revision);
        assert_eq!(current.packed_coverage().as_ptr(), coverage_address);
        assert_eq!(current.indices().as_ptr(), indices_address);
        previous = next;
    }
    assert_eq!(frame, original_frame);
    assert_eq!(presenter.lifecycle, lifecycle);
    assert_eq!(
        session.cache.initialize_level_terrain_radar(&mut || panic!(
            "display refresh must retain the existing raster and its RNG decisions"
        )),
        Ok(0)
    );
}

#[v2k_test_support::retail_test]
fn incompatible_radar_layout_is_rejected_before_cached_frame_or_presenter_mutation() {
    let session = session();
    let presenter = GameplayRadar::from_cache(&session.cache, 1).unwrap();
    let layout = presenter.layout();
    let mut bad = layout;
    bad.globe_size[0] += 1;
    assert!(matches!(
        presenter.prepare_layout_refresh(bad),
        Err(GameplayRadarRelayoutError::GlobeGeometryChanged)
    ));
    assert_eq!(presenter.layout(), layout);
    let mut frame = RadarHudFrame {
        image: RadarImage {
            width: 96,
            height: 96,
            rgba: vec![0; 96 * 96 * 4],
        },
        origin: layout.hud_origin,
        virtual_size: layout.virtual_size,
    };
    let original = frame.clone();
    assert_eq!(
        frame.relayout(layout, bad),
        Err(GameplayRadarRelayoutError::GlobeGeometryChanged)
    );
    assert_eq!(frame, original);
    bad = layout;
    bad.map_rect.width = 1;
    assert!(matches!(
        presenter.prepare_layout_refresh(bad),
        Err(GameplayRadarRelayoutError::InvalidLayout)
    ));
    assert_eq!(
        frame.relayout(layout, bad),
        Err(GameplayRadarRelayoutError::InvalidLayout)
    );
    assert_eq!(frame, original);
    frame.origin[0] += 1;
    let malformed = frame.clone();
    assert_eq!(
        frame.relayout(layout, layout),
        Err(GameplayRadarRelayoutError::FrameLayoutMismatch)
    );
    assert_eq!(frame, malformed);
}
