use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{Entity, EntityConstructionResources, EntityManager, Intro2BirthSelection},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    intro2_type58::Intro2Type58Owner,
    opening::intro2_uses_live_actor_pose,
    session::GameSession,
    sub_h_external_frame::SubHSurfacePolicy,
};

fn fixture() -> (GameSession, Vec<EntityTypeRuntimeMetadata>) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(kind, model_slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    (session, metadata)
}

fn assert_no_actor_task_ownership(entity: &Entity) {
    for slot in [
        ActorTaskSlot::Primary,
        ActorTaskSlot::Secondary,
        ActorTaskSlot::Tertiary,
    ] {
        assert!(
            entity.actor_task_state(slot).is_none(),
            "Sub-H construction must not publish a task: type {} spawn {:?} slot {slot:?}",
            entity.entity_type,
            entity.authored_spawn_index,
        );
    }
}

#[v2k_test_support::retail_test]
fn native_intro2_constructs_authored_sub_h_and_keeps_task_admission_separate() {
    let (session, metadata) = fixture();
    let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();

    let mut observed = Vec::new();
    for entity in manager.iter_all() {
        let authored = &metadata[entity.entity_type as usize];
        let RetailRuntimeValue::Known(Some(descriptor)) = &authored.sub_h_external_frame_descriptor
        else {
            continue;
        };
        let RetailRuntimeValue::Known(Some(runtime)) = &entity.sub_h_external_frame_runtime else {
            panic!(
                "missing authored Sub-H for type {} spawn {:?}",
                entity.entity_type, entity.authored_spawn_index,
            );
        };
        let RetailRuntimeValue::Known(Some(c)) = authored.sub_c_lift_descriptor else {
            panic!("the Intro2 Sub-H cohort also authors Sub-C");
        };
        assert_eq!(runtime.records().len(), descriptor.records.len());
        assert_eq!(
            runtime.surface_policy(),
            if c.surface_mode_raw == 0 {
                SubHSurfacePolicy::Terrain
            } else {
                SubHSurfacePolicy::TerrainAndWater
            },
        );
        observed.push((
            entity.authored_spawn_index.unwrap(),
            entity.entity_type,
            runtime.records().len(),
            c.surface_mode_raw,
        ));
    }

    // Independent canonical Section-12/13 census, including actors whose task
    // initializers have not been admitted. Type94 alone sets H+0C through C+0C.
    assert_eq!(
        observed,
        [
            (4, 17, 8, 0),
            (5, 16, 8, 0),
            (6, 47, 6, 0),
            (7, 47, 6, 0),
            (8, 47, 6, 0),
            (10, 26, 6, 0),
            (20, 53, 6, 0),
            (21, 122, 6, 0),
            (25, 26, 6, 0),
            (26, 53, 6, 0),
            (30, 17, 8, 0),
            (38, 53, 6, 0),
            (40, 58, 6, 0),
            (41, 53, 6, 0),
            (42, 16, 8, 0),
            (43, 94, 6, 1),
            (45, 77, 6, 0),
        ],
    );
    for spawn in [21, 45] {
        let entity = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap();
        assert_no_actor_task_ownership(entity);
    }
    let type122 = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(21))
        .unwrap();
    assert!(
        !intro2_uses_live_actor_pose(type122),
        "Type122 still lacks a native owner"
    );
    let type77 = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(45))
        .unwrap();
    assert!(
        intro2_uses_live_actor_pose(type77),
        "dormant Type77 keeps the committed authored pose without a task owner"
    );
    for spawn in [5, 42] {
        let entity = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap();
        assert!(entity.intro2_type16_runtime.is_some());
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
        assert!(intro2_uses_live_actor_pose(entity));
    }
    let entity = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(40))
        .unwrap();
    assert!(entity.intro2_type58_runtime.is_some());
    assert_eq!(
        Intro2Type58Owner::adopt(&manager, entity.id)
            .unwrap()
            .entity_id(),
        entity.id,
        "Type58's separate native constructor authenticates its task ownership"
    );
    // The supplied selector word 1 chooses Follow33, independently of H.
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::SharedRetarget(_))
    ));
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_some());
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    assert!(intro2_uses_live_actor_pose(entity));
    let entity = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(43))
        .unwrap();
    assert!(entity.intro2_type94_runtime.is_some());
    assert_eq!(
        v2k_game::intro2_type94::Intro2Type94Owner::adopt(&manager, entity.id)
            .unwrap()
            .entity_id(),
        entity.id
    );
    assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_some());
    assert!(intro2_uses_live_actor_pose(entity));
}

#[v2k_test_support::retail_test]
fn generic_constructor_retains_authored_type94_water_policy_without_native_receipts() {
    let (session, metadata) = fixture();
    let manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
    );
    let entity = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(43))
        .unwrap();
    assert_eq!(entity.entity_type, 94);
    let RetailRuntimeValue::Known(Some(c)) = metadata[94].sub_c_lift_descriptor else {
        panic!("canonical Type94 Sub-C");
    };
    assert_eq!(c.surface_mode_raw, 1);
    let RetailRuntimeValue::Known(Some(runtime)) = &entity.sub_h_external_frame_runtime else {
        panic!("generic 09A80 must construct Type94 Sub-H");
    };
    assert_eq!(runtime.records().len(), 6);
    assert_eq!(runtime.surface_policy(), SubHSurfacePolicy::TerrainAndWater);
    assert_eq!(runtime.cursor(), 0);
    assert!(runtime.is_enabled());
    assert!(runtime
        .records()
        .iter()
        .all(|record| record.flags_raw == 0 && record.phase_raw == 0));
    assert_no_actor_task_ownership(entity);
}
