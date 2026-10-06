//! C090/D030 geometry resolves each selected tier's authored resource metadata.

use v2k_game::menu_billboard::{MenuBillboardLayout, MenuBillboardPose, MenuBillboardRect};
use v2k_game::session::GameSession;

#[v2k_test_support::retail_test]
fn retail_menu_tiers_retain_reference_origin_and_selected_frame_size() {
    let dir = v2k_test_support::retail_dir();
    let expected = [
        ([320, 240], 256, [109, 55, 102, 129]),
        ([640, 480], 512, [218, 110, 206, 258]),
        ([800, 600], 640, [273, 121, 258, 323]),
        ([1024, 768], 820, [349, 138, 330, 413]),
    ];
    for (tier, (framebuffer, focal_y, rect)) in expected.into_iter().enumerate() {
        let mut session = GameSession::init(&dir).expect("PRELOAD");
        for level in [2, 3, 5] {
            session
                .load_auxiliary_ovl(level, tier as u32)
                .expect("selected menu tier");
        }
        let layout = MenuBillboardLayout::from_cache(&session.cache).expect("billboard layout");
        assert_eq!(layout.framebuffer, framebuffer);
        assert_eq!(layout.focal_y, focal_y);
        let (atlas, frame) = session
            .cache
            .global_sprite(1294)
            .expect("first emblem frame");
        let frame_size = [frame.flags as u16, (frame.flags >> 16) as u16];
        assert_eq!(
            atlas.dimensions(frame).unwrap(),
            (frame_size[0], frame_size[1])
        );
        let pose = MenuBillboardPose {
            zoom_raw: 0xffff,
            height_raw: 0,
        };
        assert_eq!(
            layout.rect(pose, frame_size),
            Some(MenuBillboardRect {
                x: rect[0],
                y: rect[1],
                width: rect[2] as u32,
                height: rect[3] as u32,
            }),
            "tier {tier}"
        );
    }
}

#[v2k_test_support::retail_test]
fn preload_without_reference_sprite_does_not_invent_billboard_geometry() {
    let dir = v2k_test_support::retail_dir();
    let session = GameSession::init(&dir).expect("PRELOAD");
    assert!(MenuBillboardLayout::from_cache(&session.cache).is_none());
}
