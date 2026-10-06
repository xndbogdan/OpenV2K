//! Regression coverage for complemented Section-8 billboard size operands.

use v2k_formats::models::AnimVars;
use v2k_game::model_tree::linked_model_bounds;
use v2k_game::session::GameSession;

const BIGFUEL_MODEL_ID: usize = 141;
const BIGFUEL_GLOW_SPRITE_ID: u16 = 617;
const GAMEPLAY_MODEL_SCALE: f32 = 100.0 / 256.0;

#[v2k_test_support::retail_test]
fn bigfuel_tick_animation_decodes_complemented_glow_size() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("initialize retail resource cache");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load high-resolution common resources");
    session
        .load_level_by_id(15, 1)
        .expect("load high-resolution Castle world");

    let model = session
        .cache
        .global_model(BIGFUEL_MODEL_ID)
        .expect("global model 141");
    assert_eq!(model.name.as_deref(), Some("bigfuel"));

    // Original466870 DD with operands[1,0x400E,0x00C1] returns
    // word0xFFF6 at tick2292 (clock<<9=0xE800). The following1D
    // subtracts decoded-48, yielding38 for each authored617 billboard.
    for (tick, expected_size) in [(0, 48), (1_784, 41), (2_292, 38), (5_012, 62)] {
        let mut vars = AnimVars::default();
        vars.dynamic[0] = tick;
        let materialized = model.materialize(&vars);
        let glows: Vec<_> = materialized
            .billboards
            .iter()
            .map(|billboard| (billboard.id, billboard.size, billboard.textured))
            .collect();
        assert_eq!(
            glows,
            vec![
                (BIGFUEL_GLOW_SPRITE_ID, expected_size, true),
                (BIGFUEL_GLOW_SPRITE_ID, expected_size, true),
            ],
            "tick {tick} must not wrap the glow extent through u16"
        );
    }

    let mut vars = AnimVars::default();
    vars.dynamic[0] = 1_784;
    let bounds = linked_model_bounds(
        &session.cache,
        BIGFUEL_MODEL_ID,
        GAMEPLAY_MODEL_SCALE,
        8,
        &vars,
    )
    .expect("bigfuel model bounds");
    assert!(
        bounds.radius < 2.0,
        "decoded glow extent must not inflate bigfuel bounds: {bounds:?}"
    );
}
