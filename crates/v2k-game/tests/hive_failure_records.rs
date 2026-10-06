//! Static Section-13 evidence for the Level-1 Hive's failed-world birth lane.
//!
//! This checks authored bytes through the canonical parser. It does not execute
//! the unowned dynamic Hive constructor, ejection, or live-list continuation.

use v2k_game::session::GameSession;

#[v2k_test_support::retail_test]
fn level_one_hive_authors_only_three_failed_world_type15_births_in_every_tier() {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).expect("retail PRELOAD parses");

    // Variant zero is inspected only as a static, nonvisual corpus control.
    // No runtime presentation or actor construction uses that tier here.
    for variant in 0..=3 {
        session
            .load_level_by_id(13, variant)
            .expect("original Level-1 tier parses");
        let level = session.cache.level_desc().expect("Level-1 Section 13");
        let hives: Vec<_> = level
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 67)
            .collect();
        assert_eq!(hives.len(), 1, "variant {variant}: one authored Hive");
        assert_eq!(hives[0].index, 24);
        let animation = hives[0].animation.as_ref().expect("Hive Sub-N program");
        let header: [u32; 6] = std::array::from_fn(|index| {
            u32::from_le_bytes(
                animation.header[index * 4..index * 4 + 4]
                    .try_into()
                    .expect("raw header dword"),
            )
        });
        assert_eq!(header[..5], [80_000, 5_000, 5, 5, 1]);
        assert_eq!(animation.frames.len(), 1, "no normal-world birth row");
        let row: [i32; 7] = std::array::from_fn(|index| {
            i32::from_le_bytes(
                animation.frames[0][index * 4..index * 4 + 4]
                    .try_into()
                    .expect("raw 0x1C birth-record dword"),
            )
        });
        assert_eq!(
            row,
            [15, 0, 2_000_000, 1_000_000, 3, 3, 3],
            "variant {variant}: Type15, initial timer, delay, jitter, total cap, live cap, flags"
        );
    }
}
