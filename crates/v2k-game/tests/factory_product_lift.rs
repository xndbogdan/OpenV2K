//! Level-1 factory product lift-platform site against the real overlay corpus.

use std::path::PathBuf;

use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::factory_product_lift::base_factory_product_marker_world_with_vars;
use v2k_game::session::GameSession;

fn data_dir() -> PathBuf {
    let dir = v2k_test_support::retail_dir();
    assert!(dir.join("PRELOAD.DAT").is_file(), "retail corpus required");
    dir
}

#[v2k_test_support::retail_test]
fn level_one_lifter_fork_rises_away_from_factory_origin() {
    let dir = data_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("load L3 aux");
    let table = session.cache.global_entity_model_table();
    let type_metadata: Vec<_> = table
        .iter()
        .copied()
        .enumerate()
        .map(|(entity_type, model_slots)| {
            session
                .cache
                .global_entity_type(entity_type)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..EntityTypeRuntimeMetadata::default()
                })
        })
        .collect();
    session.load_level_by_id(13, 1).expect("load level 13");
    let level = session.cache.level_desc().expect("Section 13");
    let terrain = session.cache.terrain().expect("first-world terrain");
    let mut manager = v2k_game::entity::EntityManager::from_level_with_type_metadata(
        level,
        &type_metadata,
        Some(terrain),
    );
    let (factory_position, rest, lifted) = {
        let factory = manager
            .iter()
            .find(|entity| entity.entity_type == 66)
            .expect("Working Factory");
        assert_eq!(factory.position, [87.0, -3.0, 58.0]);
        assert_eq!(factory.model_index, Some(227));
        let rest_vars = factory.presentation_anim_vars(0);
        let rest = base_factory_product_marker_world_with_vars(&session.cache, factory, &rest_vars)
            .expect("lifterarmfork at rest");
        let mut lifted_vars = rest_vars;
        lifted_vars.dynamic[1] = 0xFFFF;
        let lifted =
            base_factory_product_marker_world_with_vars(&session.cache, factory, &lifted_vars)
                .expect("lifterarmfork raised");
        (factory.position, rest, lifted)
    };
    let origin_delta =
        |world: [f32; 3]| (world[0] - factory_position[0]).hypot(world[2] - factory_position[2]);
    assert!(
        origin_delta(rest) > 1.0,
        "rest fork {rest:?} must leave the building origin {factory_position:?}"
    );
    assert!(
        lifted[1] - rest[1] > 0.5,
        "raised fork {lifted:?} must sit above rest {rest:?}"
    );
    assert!(
        (lifted[0] - rest[0]).abs() + (lifted[2] - rest[2]).abs() > 0.05
            || lifted[1] - rest[1] > 0.5,
        "channel 1 must move the fork, rest={rest:?} lifted={lifted:?}"
    );

    let authored_before: Vec<_> = manager
        .iter()
        .filter(|entity| entity.entity_type == 61)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    assert_eq!(authored_before.len(), 3);
    manager.follow_base_factory_products_to_marker(&session.cache, 0);
    let authored_after: Vec<_> = manager
        .iter()
        .filter(|entity| entity.entity_type == 61)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    assert_eq!(
        authored_after, authored_before,
        "world type-61 pickups are not tracked by factory Sub-M +88 and must stay put"
    );
}
