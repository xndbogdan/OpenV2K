use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntimeFamily,
    actor_task_owner::ActorTaskSlot,
    entity::EntityManager,
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    ordinary_type47_live::{
        admit_fresh_level1_ordinary_type47, FreshLevel1OrdinaryType47SpawnFacts,
        FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY,
        FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
        FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR,
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D,
        FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
    },
    session::GameSession,
    world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn first_world_retains_only_the_exact_three_type47_component_runtimes() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");

    let type_models = session.cache.global_entity_model_table();
    let type_metadata: Vec<_> = type_models
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

    let type47 = &type_metadata[47];
    assert_eq!(
        type47.projectile_emitter_descriptor,
        RetailRuntimeValue::Known(Some(FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR))
    );
    assert_eq!(
        type47.sub_d_steering_descriptor,
        RetailRuntimeValue::Known(Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D))
    );
    assert!(matches!(
        type47.common_mover_topology,
        RetailRuntimeValue::Known(topology)
            if topology == FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY
    ));
    assert_eq!(
        type_metadata[17].projectile_emitter_descriptor,
        RetailRuntimeValue::Known(None),
        "the non-shooting sibling capture must not acquire type 47's Sub-E"
    );
    assert_eq!(
        EntityTypeRuntimeMetadata::default().projectile_emitter_descriptor,
        RetailRuntimeValue::Unresolved,
        "compatibility metadata must fail closed"
    );

    let level = session.cache.level_desc().expect("Level-1 Section 13");
    let type47_spawns = level
        .entities
        .iter()
        .filter(|spawn| spawn.entity_type == 47)
        .collect::<Vec<_>>();
    assert_eq!(
        type47_spawns
            .iter()
            .map(|spawn| spawn.index)
            .collect::<Vec<_>>(),
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
    );
    for spawn in type47_spawns {
        let active_model = if spawn.model_overrides[0] != 0 {
            Some(spawn.model_overrides[0] as usize)
        } else {
            Some(usize::from(type47.model_slots[0]))
        };
        assert_eq!(active_model, Some(FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID));
        let runtime = admit_fresh_level1_ordinary_type47(
            FreshLevel1OrdinaryType47SpawnFacts {
                retail_first_world: true,
                authored_spawn_index: spawn.index,
                entity_type: spawn.entity_type,
                active_model,
            },
            Some(type47),
        )
        .expect("exact captured spawn")
        .aim_and_fire_runtime();
        assert_eq!(
            runtime.emitter_runtime(),
            FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME
        );
    }

    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        level,
        &type_metadata,
        session.cache.terrain(),
        0,
        &mut world_fx,
    )
    .expect("fresh type-17 birth publication");
    let retained = manager
        .iter_all()
        .filter_map(|entity| {
            entity
                .ordinary_type47_aim_and_fire_runtime
                .as_ref()
                .map(|runtime| (entity.authored_spawn_index, entity.entity_type, runtime))
        })
        .collect::<Vec<_>>();
    assert_eq!(retained.len(), 3);
    assert_eq!(
        retained
            .iter()
            .map(|(index, _, _)| index.expect("authored spawn"))
            .collect::<Vec<_>>(),
        FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
    );
    assert!(retained.iter().all(|(index, entity_type, runtime)| {
        *entity_type == 47
            && runtime.projectile_descriptor() == FRESH_LEVEL1_ORDINARY_TYPE47_PROJECTILE_DESCRIPTOR
            && runtime.component_topology() == FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY
            && runtime.emitter_runtime() == FRESH_LEVEL1_ORDINARY_TYPE47_EMITTER_RUNTIME
            && runtime.queued_shot_count() == 0
            && Some(runtime.sub_d_stagger_seed())
                == index.and_then(|spawn| {
                    FRESH_LEVEL1_ORDINARY_TYPE47_SPAWN_INDICES
                        .iter()
                        .position(|&expected| expected == spawn)
                        .map(|ordinal| FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D_SEEDS[ordinal])
                })
    }));
    let type47_sub_h = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 47)
        .map(|entity| &entity.sub_h_external_frame_runtime)
        .collect::<Vec<_>>();
    assert_eq!(type47_sub_h.len(), 3);
    assert!(type47_sub_h.iter().all(|runtime| {
        matches!(
            runtime,
            RetailRuntimeValue::Known(Some(runtime))
                if runtime.is_enabled()
                    && runtime.records().len()
                        == FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT
        )
    }));
    assert_eq!(manager.fresh_level1_type47_initial_productions().len(), 3);
    let type47_births = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 47)
        .collect::<Vec<_>>();
    assert_eq!(type47_births.len(), 3);
    for entity in type47_births {
        assert!(matches!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(selection))
                if selection.program.class_id == 32 && selection.choice_index == 0
        ));
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Primary)
                .map(|task| task.family()),
            Some(ActorTaskRuntimeFamily::OrdinaryType9Wander)
        );
        assert_eq!(
            entity
                .actor_task_state(ActorTaskSlot::Secondary)
                .map(|task| task.family()),
            Some(ActorTaskRuntimeFamily::GuardLocationAcquisition)
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Unresolved
        );
    }

    let generic = EntityManager::from_level_with_type_metadata(
        level,
        &type_metadata,
        session.cache.terrain(),
    );
    assert!(
        generic
            .iter_all()
            .all(|entity| entity.ordinary_type47_aim_and_fire_runtime.is_none()),
        "direct/replay/load construction must not inherit fresh-New-Game provenance"
    );
    assert!(generic
        .iter_all()
        .filter(|entity| entity.entity_type == 47)
        .all(|entity| matches!(&entity.sub_h_external_frame_runtime,
            RetailRuntimeValue::Known(Some(runtime))
                if runtime.records().len() == 6
                    && runtime.surface_policy() == v2k_game::sub_h_external_frame::SubHSurfacePolicy::Terrain)),
        "shared component construction does not inherit task/birth provenance");
}
