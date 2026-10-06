//! Corpus-backed coverage for the recovered `0x02`/`0x22` model-edge
//! submission path (see RENDER_PIPELINE.md "Model edge sprite quads").
//!
//! The loaded Level-1 pools carry a small but real set of sprite-style edges
//! (turret barrels, crab legs). Every one of them must resolve its sprite
//! record's packed `+0x10/+0x12` width pair through the resource cache, or the
//! renderer's `edge_widths` gate silently drops the stream.

use v2k_formats::models::ModelEdgeStyle;
use v2k_game::session::GameSession;

#[v2k_test_support::retail_test]
fn sprite_style_edges_resolve_their_record_width_pairs() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();

    let mut total_edges = 0usize;
    let mut sprite_edges = 0usize;
    let mut resolved_pairs = 0usize;
    let mut crabrotate_seen = false;

    for id in session.cache.global_model_ids() {
        let Some(model) = session.cache.global_model(id) else {
            continue;
        };
        if model.edges.is_empty() {
            continue;
        }
        if model.name.as_deref() == Some("crabrotate") {
            crabrotate_seen = true;
            assert!(
                model.edges.len() >= 6,
                "crabrotate should carry its leg cluster"
            );
        }
        for edge in &model.edges {
            total_edges += 1;
            let ModelEdgeStyle::Sprite { sprite_id, size: _ } = edge.style else {
                continue;
            };
            sprite_edges += 1;
            // The renderer resolves this exact pair through global_sprite.
            let (_, entry) = session
                .cache
                .global_sprite(sprite_id)
                .unwrap_or_else(|| panic!("edge sprite {sprite_id} must resolve"));
            let widths = (entry.flags as u16, (entry.flags >> 16) as u16);
            assert_ne!(
                widths,
                (0, 0),
                "model {id} ({}) edge sprite {sprite_id} resolved empty widths",
                model.name.clone().unwrap_or_default()
            );
            resolved_pairs += 1;
        }
    }

    assert!(total_edges > 0, "loaded pools should carry authored edges");
    assert_eq!(
        sprite_edges, resolved_pairs,
        "every sprite-style edge must resolve nonzero record widths"
    );
    assert!(
        crabrotate_seen,
        "crabrotate (the scanned leg cluster) must stay present in these pools"
    );
}
