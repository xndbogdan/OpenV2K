use super::*;
use crate::session::GameSession;

fn fixture(level_id: u32) -> (GameSession, Vec<EntityTypeRuntimeMetadata>) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").is_file(),
        "normal-tier corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    (session, metadata)
}

#[test]
fn cleanup_uses_wrapped_distance_and_strict_horizontal_and_height_bounds() {
    assert!(helpers_overlap([32760, 0, 0], [-32760, 0, 0], false));
    assert!(helpers_overlap([0; 3], [511, 0, 0], false));
    assert!(!helpers_overlap([0; 3], [512, 0, 0], false));
    assert!(helpers_overlap([0; 3], [1023, 0, 0], true));
    assert!(!helpers_overlap([0; 3], [1024, 0, 0], true));
    assert!(helpers_overlap([0; 3], [0, 7999, 0], true));
    assert!(!helpers_overlap([0; 3], [0, 8000, 0], true));
    assert!(!helpers_overlap([0; 3], [i16::MIN; 3], true));
}

#[v2k_test_support::retail_test]
fn moved_marker_chain_consumes_every_birth_and_retains_last_helper() {
    let (session, metadata) = fixture(30);
    let data = v2k_test_support::retail_dir();
    let bytes = std::fs::read(data.join("Overlay/1X30XX.OVL")).unwrap();
    let ovl = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
    let mut level = v2k_formats::levels::parse_level(&ovl.section(13).unwrap().data).unwrap();
    level.entities.clear();
    let mut terrain = session.cache.terrain().unwrap().clone();
    let marker = collect_authored_warp_markers(&terrain, session.cache.terrain_objects().unwrap())
        .unwrap()[0];
    for cell in &mut terrain.cells {
        cell.attribute = 0;
    }
    for x in 10..=12 {
        let cell = &mut terrain.cells[x * 256 + 20];
        cell.attribute = marker.attribute;
        cell.height = 0;
        cell.terrain_type = 8;
    }
    let mut fx = WorldFx::new();
    let mut expected_rng = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: &level,
            logical_world_index: 18,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: Some(&terrain),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut fx,
    )
    .unwrap();
    for _ in 0..3 {
        expected_rng.next_shared_retail_random_u16();
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
    assert_eq!(
        manager.next_common_body_ordinal(),
        RetailRuntimeValue::Known(4)
    );
    let gates = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 111)
        .collect::<Vec<_>>();
    assert_eq!(gates.len(), 3);
    for (index, gate) in gates.iter().enumerate() {
        assert_eq!(
            gate.position_raw(),
            [((10 + index) * 256 + 128) as i16, 512, 20 * 256 + 128]
        );
        assert_eq!(
            gate.construction_stamp_at_0xb4,
            RetailRuntimeValue::Known(18 * 1024 + index as u16 + 1)
        );
        assert_eq!(gate.attached_to, None);
        assert_eq!(
            manager.native_class0_actors[&gate.id].gate_self_relation,
            Some(gate.id)
        );
        assert!(Class0ActorOwner::adopt(&manager, gate.id).is_ok());
        assert_eq!(gate.presentation_anim_vars(1234).dynamic[1], 0);
    }
    let ids = gates.iter().map(|gate| gate.id).collect::<Vec<_>>();
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &ids[..2]);
    assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), ids[..2]);
    assert_eq!(
        manager
            .iter_all()
            .map(|entity| entity.id)
            .collect::<Vec<_>>(),
        vec![ids[2]]
    );
    assert_eq!(
        manager.next_common_body_ordinal(),
        RetailRuntimeValue::Known(4)
    );
}

#[v2k_test_support::retail_test]
fn ordinary_corpus_publishes_markers_before_authored_bodies_and_hive_cleanup() {
    let (mut session, _) = fixture(13);
    let mut fx = WorldFx::new();
    let mut worlds_with_helpers = 0;
    for level_id in 13..=48 {
        session.load_level_by_id(level_id, 1).unwrap();
        let metadata = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let terrain = session.cache.terrain().unwrap();
        let markers =
            collect_authored_warp_markers(terrain, session.cache.terrain_objects().unwrap())
                .unwrap();
        let level = session.cache.level_desc().unwrap();
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level,
                logical_world_index: (level_id - 12) as i32,
                type_metadata: &metadata,
                resources: EntityConstructionResources {
                    terrain: Some(terrain),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| {
                        session.cache.global_model(id).map(|model| model.radius)
                    }),
                },
                player_arrival: None,
                retail_tick: 0,
            }
            .with_exit_marker_bits(0x1f0),
            &mut fx,
        )
        .unwrap();
        let helpers = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 111)
            .collect::<Vec<_>>();
        assert_eq!(helpers.len(), markers.len(), "world{level_id}");
        worlds_with_helpers += usize::from(!helpers.is_empty());
        for (helper, marker) in helpers.iter().zip(&markers) {
            assert_eq!(helper.position_raw(), marker.position_raw);
            assert_eq!(helper.attached_to, None);
            assert_ne!(helper.capability_flags & 0x8000, 0);
            assert_eq!(
                helper.collision.state_flags_at_0x08.masked(0x800),
                RetailRuntimeValue::Known(if marker.terrain_type & 8 == 0 {
                    0
                } else {
                    0x800
                })
            );
        }
        if level_id == 13 {
            assert_eq!(helpers.len(), 2);
            assert_eq!(manager.pending_actor_deferred_destroy_ids().len(), 1);
            manager.cleanup_pending_actor_deferred_destroys();
            assert_eq!(
                manager
                    .iter_all()
                    .find(|entity| entity.entity_type == 111)
                    .unwrap()
                    .position_raw(),
                [-27264, -2080, -4224]
            );
        }
        assert_eq!(
            manager.next_common_body_ordinal(),
            RetailRuntimeValue::Known((1 + markers.len() + level.entities.len()) as u16)
        );
    }
    assert!(worlds_with_helpers > 2);
}
