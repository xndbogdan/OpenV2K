use v2k_formats::models::face_material;
use v2k_game::model_color::sprite_blend;
use v2k_game::player_shield::{
    PlayerShieldFrame, PLAYER_SHIELD_FIRST_SPRITE_ID, PLAYER_SHIELD_MODEL_ID,
};
use v2k_game::session::GameSession;
use v2k_render::WorldSpriteBlend;

#[v2k_test_support::retail_test]
fn retail_shield_model_uses_eight_authored_additive_flash_textures() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    for variant in 1..=3 {
        session.load_auxiliary_ovl(3, variant).unwrap();
        let model = session.cache.global_model(PLAYER_SHIELD_MODEL_ID).unwrap();
        assert_eq!(model.name.as_deref(), Some("shipaura"));
        assert_eq!(model.radius, 400);
        for level in 0..=7 {
            let frame = PlayerShieldFrame {
                spin_raw: 0,
                retail_tick: 99,
                damage_flash_level: level,
            };
            let materialized = model.materialize(&frame.animation_vars());
            assert_eq!(materialized.triangles.len(), 20);
            let sprite_id = PLAYER_SHIELD_FIRST_SPRITE_ID + u16::from(level);
            assert!(materialized
                .face_materials
                .iter()
                .all(|material| face_material(*material) == (sprite_id, true)));
            let (_, sprite) = session.cache.global_sprite(sprite_id).unwrap();
            assert_eq!(
                sprite_blend(sprite.pal_size as u8),
                WorldSpriteBlend::Additive
            );
        }
    }
}
