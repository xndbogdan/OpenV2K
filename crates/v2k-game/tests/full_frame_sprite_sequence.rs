//! Real-data checks for the shared session-`+0x295` presentation resources.

use v2k_game::full_frame_sprite_sequence::{
    decode_full_frame_sprite, FullFrameSpriteSequence, FULL_FRAME_SPRITE_IDS,
};
use v2k_game::session::GameSession;
use v2k_render::WorldSpriteBlend;

fn session_with_system_tier(variant: u32) -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, variant)
        .expect("retail fixture must load");
    session
}

#[v2k_test_support::retail_test]
fn low_and_high_system_tiers_supply_the_exact_flash_materials() {
    for variant in [0, 1] {
        let session = session_with_system_tier(variant);
        let mut sequence = FullFrameSpriteSequence::default();
        sequence.request();

        for (offset, expected_id) in FULL_FRAME_SPRITE_IDS.into_iter().enumerate() {
            let frame = sequence.current_frame().expect("active table entry");
            assert_eq!(usize::from(frame.table_index), offset + 1);
            assert_eq!(frame.global_sprite_id, expected_id);

            let decoded = decode_full_frame_sprite(&session.cache, frame)
                .unwrap_or_else(|| panic!("variant {variant} global sprite {expected_id}"));
            assert_eq!((decoded.width, decoded.height), (2, 2));
            assert_eq!(
                (decoded.render_flags, decoded.blend),
                if expected_id == 0x227 {
                    (0x04, WorldSpriteBlend::Masked)
                } else {
                    (0x14, WorldSpriteBlend::Additive)
                },
                "variant {variant} global sprite {expected_id}"
            );
            assert!(sequence.acknowledge_submitted(frame));
        }

        assert!(!sequence.is_active());
    }
}
