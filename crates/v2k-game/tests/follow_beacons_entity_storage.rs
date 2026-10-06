use v2k_formats::collision::CommonAxisDescriptor;
use v2k_game::{
    entity::EntityManager,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    session::GameSession,
};

const FOLLOW_BEACON_ENTITY_TYPE: u32 = 52;
const TYPE17_ENTITY_TYPE: u32 = 17;

#[v2k_test_support::retail_test]
fn first_world_retains_follow_priorities_and_type17_actor_axis_copy() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_metadata = session
        .cache
        .global_entity_model_table()
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
        .collect::<Vec<_>>();

    let manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        session.cache.terrain(),
    );

    let priorities = manager
        .iter_all()
        .filter(|entity| entity.entity_type == FOLLOW_BEACON_ENTITY_TYPE)
        .map(|entity| {
            assert_eq!(entity.power_up_payload_packed, None);
            entity
                .authored_follow_beacon_priority_raw
                .expect("authored type-52 priority")
        })
        .collect::<Vec<_>>();
    assert_eq!(priorities, [0, 8, 7, 6, 11, 11, 12, 10, 10, 5, 4]);

    let expected_actor_axis = RetailRuntimeValue::Known(CommonAxisDescriptor {
        strict_axis_limit_raw: 0x0A00,
        raw_word_at_0x04: 3,
    });
    let type17_entities = manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE17_ENTITY_TYPE)
        .collect::<Vec<_>>();
    assert_eq!(type17_entities.len(), 4);
    for entity in type17_entities {
        assert_eq!(entity.authored_follow_beacon_priority_raw, None);
        assert_eq!(entity.actor_common_axis_descriptor, expected_actor_axis);
    }

    assert!(manager.iter_all().all(|entity| {
        entity.entity_type == FOLLOW_BEACON_ENTITY_TYPE
            || entity.authored_follow_beacon_priority_raw.is_none()
    }));
}
