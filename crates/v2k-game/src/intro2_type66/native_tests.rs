use super::*;
use crate::{
    actor_task_owner::{ActorTaskVisit, ActorTaskWrapperFlags},
    entity::{EntityConstructionResources, EntityManager},
    entity_behavior::BehaviorChoiceListSource,
    session::GameSession,
};

pub(in crate::intro2_type66) fn fixture() -> Option<(GameSession, Vec<EntityTypeRuntimeMetadata>)> {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect();
    Some((session, metadata))
}

pub(in crate::intro2_type66) fn generic(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
) -> EntityManager {
    EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    )
}

pub(in crate::intro2_type66) fn publish(
    manager: &mut EntityManager,
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
) -> u32 {
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    publish_working_factory(
        manager.entity_mut(id).unwrap(),
        allocation,
        &metadata[66],
        &session.cache.level_desc().unwrap().entities[spawn],
        session.cache.terrain().unwrap(),
        &mut || 0x4567,
    )
    .unwrap();
    id
}

#[v2k_test_support::retail_test]
fn native_type66_post_load_health_is_separate_from_factory_construction() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    for spawn in INTRO2_TYPE66_SPAWN_INDICES {
        let id = publish(&mut manager, &session, &metadata, spawn);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(99_999)
        );
        let before_collision = entity.collision.clone();
        let before_factory = entity.base_factory_runtime;
        let before_context = entity.current_behavior_context;
        let before_tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
            (
                entity.actor_tasks.task_in_slot(slot),
                entity.actor_task_state(slot).copied(),
            )
        });
        apply_intro2_type66_post_load_health(entity).unwrap();
        let mut expected_collision = before_collision;
        expected_collision.health_raw = RetailRuntimeValue::Known(1);
        assert_eq!(entity.collision, expected_collision);
        assert_eq!(entity.base_factory_runtime, before_factory);
        assert_eq!(entity.current_behavior_context, before_context);
        assert_eq!(
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| (
                entity.actor_tasks.task_in_slot(slot),
                entity.actor_task_state(slot).copied(),
            )),
            before_tasks
        );
        let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
            panic!()
        };
        assert_eq!(factory.live_owner.unwrap().maximum_health_raw, 99_999);
        assert_eq!(factory.production.unwrap().cached_health_raw, 99_999);
        assert_eq!(
            apply_intro2_type66_post_load_health(entity),
            Err(Intro2Type66Error::ComponentStorage)
        );
        assert_eq!(
            entity.collision, expected_collision,
            "replay cannot reset later state"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type66_post_load_health_rejects_non_native_or_started_factory_atomically() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for invalid in 0..3 {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, 36);
        let entity = manager.entity_mut(id).unwrap();
        match invalid {
            0 => entity.intro2_type66_runtime = None,
            1 => {
                let primary = entity
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .unwrap();
                let Some(ActorTaskRuntime::WorkingFactory(task)) =
                    entity.actor_tasks.task_state_mut(primary)
                else {
                    panic!()
                };
                task.accumulate_elapsed_prefix(20_000);
            }
            2 => entity.collision.health_raw = RetailRuntimeValue::Known(777),
            _ => unreachable!(),
        }
        let before = (
            entity.collision.clone(),
            entity.base_factory_runtime,
            entity.current_behavior_context,
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
        );
        assert!(apply_intro2_type66_post_load_health(entity).is_err());
        assert_eq!(
            (
                entity.collision.clone(),
                entity.base_factory_runtime,
                entity.current_behavior_context,
                entity.actor_task_state(ActorTaskSlot::Primary).copied()
            ),
            before
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type66_both_births_consume_one_selector_and_retain_distinct_factory_templates() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for (spawn, position, models, capacity, config_flags, repair_rate) in [
        (
            36,
            [-28672, -256, -32768],
            [364, 365, 364, 365],
            5,
            0x2a,
            333,
        ),
        (51, [20992, -320, -5888], [210, 225, 210, 225], 0, 0, 1666),
    ] {
        for selector in [0, 0xffff] {
            let mut manager = generic(&session, &metadata);
            let allocation = manager
                .main_base_abort_actor_observation(spawn as u32 + 1)
                .unwrap()
                .lease;
            let entity = manager.entity_mut(spawn as u32 + 1).unwrap();
            let initial_basis = entity.physical_body_basis_q31();
            let mut draws = 0;
            let publication = publish_working_factory(
                entity,
                allocation,
                &metadata[66],
                &session.cache.level_desc().unwrap().entities[spawn],
                session.cache.terrain().unwrap(),
                &mut || {
                    draws += 1;
                    selector
                },
            )
            .unwrap();
            assert_eq!(draws, 1, "singleton Always still consumes AC60's word");
            assert_eq!(publication.selector_word, selector);
            assert_eq!(publication.selection.program.class_id, 39);
            assert_eq!(publication.selection.choice_index, 0);
            assert_eq!(entity.position_raw(), position);
            assert_eq!(entity.model_slots, models.map(Some));
            assert_eq!(entity.physical_body_basis_q31(), initial_basis);
            assert!(intro2_type66_allocation_authenticates(entity));
            assert!(crate::opening::intro2_uses_live_actor_pose(entity));
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0x25027)
            );
            // D3C0 disables integration but preserves the callback; no setup
            // substitutes a gameplay or captured exact state word.
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x50000),
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x20000),
                RetailRuntimeValue::Known(0x20000)
            );
            let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
                panic!()
            };
            let production = factory.production.unwrap();
            assert_eq!(production.scientist_capacity_raw, capacity);
            assert_eq!(production.current_scientists_raw, 0);
            assert_eq!(production.repair_rate_raw, repair_rate);
            assert_eq!(production.production_threshold_micros_raw, 0);
            assert_eq!(factory.progressive_death.config_flags_at_0x18, config_flags);
            assert_eq!(factory.progressive_death.elapsed_micros_raw, 0);
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!()
            };
            assert_eq!(context.active_style().style_address(), 0x004c9558);
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(None)
            );
            let Some(ActorTaskRuntime::WorkingFactory(task)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!()
            };
            assert_eq!(task.elapsed_ms(), 0);
            assert!(!task.under_attack_notification_latched());
            assert_eq!(
                task.owner_allocation_identity(),
                factory.live_owner.unwrap().allocation_identity
            );
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type66_invalid_admission_consumes_no_selector_or_constructor_prefix() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for invalid in 0..4 {
        let mut manager = generic(&session, &metadata);
        let allocation = manager.main_base_abort_actor_observation(37).unwrap().lease;
        let entity = manager.entity_mut(37).unwrap();
        let mut spawn = session.cache.level_desc().unwrap().entities[36].clone();
        let mut changed_metadata = metadata[66].clone();
        match invalid {
            0 => spawn.config.as_mut().unwrap()[0x18] ^= 0x20,
            1 => changed_metadata.status_component_descriptor = RetailRuntimeValue::Known(None),
            2 => entity.collision.health_raw = RetailRuntimeValue::Known(1),
            3 => spawn.rotation[0] = 0,
            _ => unreachable!(),
        }
        let before = (
            entity.position_raw(),
            entity.collision.clone(),
            entity.base_factory_runtime,
            entity.current_behavior_context,
        );
        let mut draws = 0;
        assert!(publish_working_factory(
            entity,
            allocation,
            &changed_metadata,
            &spawn,
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0
            }
        )
        .is_err());
        assert_eq!(draws, 0);
        assert_eq!(
            (
                entity.position_raw(),
                entity.collision.clone(),
                entity.base_factory_runtime,
                entity.current_behavior_context
            ),
            before
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert!(!intro2_type66_allocation_authenticates(entity));
    }
}

#[v2k_test_support::retail_test]
fn authored_ordinary_factory_construction_retains_every_canonical_template() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut total = 0;
    let mut autonomous = 0;
    for level in 13..=49 {
        session.load_level_by_id(level, 1).unwrap();
        let canonical = EntityTypeRuntimeMetadata::from_section12(
            session.cache.global_entity_type(66).unwrap(),
        );
        assert_eq!(
            canonical, metadata[66],
            "ordinary Type66 descriptor at {level}"
        );
        let mut manager = generic(&session, &metadata);
        for spawn in session
            .cache
            .level_desc()
            .unwrap()
            .entities
            .iter()
            .filter(|spawn| spawn.entity_type == 66)
        {
            let id = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn.index))
                .unwrap()
                .id;
            let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
            let entity = manager.entity_mut(id).unwrap();
            let models = entity.model_slots;
            let config = FactorySection13Config::decode(spawn.config.as_ref().unwrap());
            let mut draws = 0;
            let publication = publish_working_factory(
                entity,
                allocation,
                &metadata[66],
                spawn,
                session.cache.terrain().unwrap(),
                &mut || {
                    draws += 1;
                    0x1234
                },
            )
            .unwrap_or_else(|error| panic!("level{level}/spawn{}: {error:?}", spawn.index));
            assert_eq!(draws, 1);
            assert_eq!(publication.selection.program.class_id, 39);
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(INITIAL_HEALTH_RAW)
            );
            let runtime = entity.intro2_type66_runtime.unwrap();
            assert_eq!(runtime.model_slots, models);
            assert_eq!(runtime.config, config);
            assert_eq!(runtime.allocation, allocation);
            let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
                panic!()
            };
            assert_eq!(
                factory.production,
                Some(FactoryProductionRuntime::from_retail_template(
                    config,
                    INITIAL_HEALTH_RAW
                ))
            );
            assert!(super::super::Intro2Type66Owner::adopt(&manager, id).is_ok());
            total += 1;
            if config.scientist_capacity_raw() == 0 && config.production_threshold_micros_raw() > 0
            {
                autonomous += 1;
            }
        }
    }
    assert_eq!(total, 81);
    assert_eq!(autonomous, 20);
}

#[v2k_test_support::retail_test]
fn native_factory_runtime_from_another_manager_cannot_authorize_adoption_or_death() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut first = generic(&session, &metadata);
    let mut second = generic(&session, &metadata);
    let id = publish(&mut first, &session, &metadata, 51);
    assert_eq!(publish(&mut second, &session, &metadata, 51), id);
    second.entity_mut(id).unwrap().intro2_type66_runtime =
        first.entity_mut(id).unwrap().intro2_type66_runtime;
    assert_eq!(
        super::super::Intro2Type66Owner::adopt(&second, id),
        Err(super::super::Intro2Type66Block::Allocation)
    );
    let before = second.entity_mut(id).unwrap().collision.clone();
    let mut fx = crate::world_fx::WorldFx::new();
    let result =
        super::super::death::publish_intro2_type66_standard_death(&mut second, id, &mut fx);
    assert!(
        matches!(result, Err(error) if !error.committed_prefix && error.phase == "native allocation")
    );
    assert_eq!(second.entity_mut(id).unwrap().collision, before);
    assert_eq!(fx.pending_event_count(), 0);
}

#[v2k_test_support::retail_test]
fn native_type66_reentry_preserves_factory_and_context_and_retires_executing_primary() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 36);
    let entity = manager.entity_mut(id).unwrap();
    let program = behavior_program(39).unwrap();
    let context = BehaviorContextRuntime::named_audited(
        program,
        0,
        RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
        RetailRuntimeValue::Known(Some(51)),
        RetailRuntimeValue::Known(0x12345678),
        program.initial_style,
    )
    .unwrap();
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known((-17_i16) as u16);
    let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
        panic!()
    };
    factory.progressive_death.elapsed_micros_raw = 12345;
    factory.live_owner.as_mut().unwrap().state_version = 7;
    let factory_before = *factory;
    let allocation = factory.live_owner.unwrap().allocation_identity;
    for slot in [ActorTaskSlot::Secondary, ActorTaskSlot::Tertiary] {
        entity.actor_tasks.replace_prepared(
            slot,
            PreparedActorTask::new(ActorTaskRuntime::WorkingFactory(
                WorkingFactoryTaskState::new(allocation),
            )),
        );
    }
    let old_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id: old_id,
    };
    assert!(entity
        .actor_tasks
        .begin_exact_visit_with(visit, |_| ())
        .is_some());
    let mut draws = 0;
    let publication = reselect_working_factory(entity, &mut || {
        draws += 1;
        0xdeadbeef
    })
    .unwrap();
    assert_eq!(draws, 1);
    assert_eq!(publication.selector_word, 0xdeadbeef);
    assert_eq!(
        entity.current_behavior_context,
        RetailRuntimeValue::Known(Some(context))
    );
    assert_eq!(
        entity.base_factory_runtime,
        RetailRuntimeValue::Known(Some(factory_before))
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known((-17_i16) as u16)
    );
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(old_id)
    );
    assert_eq!(
        entity.actor_tasks.wrapper_flags(old_id),
        Some(ActorTaskWrapperFlags {
            alive: false,
            in_callback: true
        })
    );
    assert!(!entity.actor_tasks.finish_exact_visit(visit));
    assert_eq!(entity.actor_tasks.wrapper_flags(old_id), None);
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    assert!(intro2_type66_allocation_authenticates(entity));
    assert!(crate::opening::intro2_uses_live_actor_pose(entity));
}

#[v2k_test_support::retail_test]
fn native_type66_birth_cannot_be_replayed_and_identity_survives_destroyed_pose() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 36);
    let allocation = manager.main_base_abort_actor_observation(id).unwrap().lease;
    let entity = manager.entity_mut(id).unwrap();
    let mut draws = 0;
    assert_eq!(
        publish_working_factory(
            entity,
            allocation,
            &metadata[66],
            &session.cache.level_desc().unwrap().entities[36],
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                0
            }
        ),
        Err(Intro2Type66Error::AlreadyPublished)
    );
    assert_eq!(draws, 0);
    entity.set_position_raw([-28672, -896, -32768]);
    entity.model_index = Some(365);
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x4000, 0x4000);
    assert!(intro2_type66_allocation_authenticates(entity));
    assert!(
        crate::opening::intro2_uses_live_actor_pose(entity),
        "wreck presentation retains the live pose and crater basis"
    );
    let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
        panic!()
    };
    factory.live_owner.as_mut().unwrap().allocation_identity += 1;
    assert!(!intro2_type66_allocation_authenticates(entity));
}

#[v2k_test_support::retail_test]
fn native_type66_hut_crater_updates_real_terrain_and_grounded_pose_without_euler_reset() {
    use crate::terrain_crater::{
        apply_terrain_crater, crater_material_for_world_style, refresh_crater_grounded_actors,
        TerrainCraterRequest,
    };
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 36);
    let before_basis = manager.entity_mut(id).unwrap().physical_body_basis_q31();
    let before_angles = manager
        .entity_mut(id)
        .unwrap()
        .rotation_heading_pitch_roll_raw();
    // An ordinary peasant at the exact same point is still excluded by
    // EB20's fixed08000000 bit, despite its default ground-snap20 policy.
    let peasant = manager.entity_mut(3).unwrap();
    peasant.set_position_raw([-28672, 777, -32768]);
    assert_eq!(
        peasant.collision.state_flags_at_0x08.masked(0x08000000),
        RetailRuntimeValue::Known(0)
    );
    let peasant_before = (
        peasant.position_raw(),
        peasant.physical_body_basis_q31(),
        peasant.collision.state_flags_at_0x08,
    );
    let request = TerrainCraterRequest {
        position_raw: manager.entity_mut(id).unwrap().position_raw(),
        radius_raw: 1280,
        depth_height_units: 16,
        replacement_material: crater_material_for_world_style(
            session.cache.level_desc().unwrap().world_style,
        ),
    };
    let objects = v2k_formats::anim_frames::TerrainObjectTable {
        records: session.cache.terrain_objects().unwrap().records.clone(),
    };
    let report = apply_terrain_crater(
        session.cache.level_terrain_mut().unwrap(),
        &objects,
        &mut crate::static_damage::StaticDamageScheduler::default(),
        request,
    )
    .unwrap();
    assert!(report.terrain_changed);
    assert_eq!(report.radar_refresh_positions_raw.len(), 69);
    // This test exercises the no-RNG EB20 phase independently. The live
    // terminal bridge owns 4A890's intervening radar refresh/shared RNG.
    let refreshed = refresh_crater_grounded_actors(&mut manager, &session.cache, request).unwrap();
    assert_eq!(
        refreshed,
        vec![id],
        "the hut is the only canonical EB20-eligible actor in this radius"
    );
    let peasant = manager.entity_mut(3).unwrap();
    assert_eq!(
        (
            peasant.position_raw(),
            peasant.physical_body_basis_q31(),
            peasant.collision.state_flags_at_0x08
        ),
        peasant_before
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.position_raw(), [-28672, -896, -32768]);
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), before_angles);
    assert_ne!(entity.physical_body_basis_q31(), before_basis);
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!()
    };
    assert!(basis.lateral[1] > 0 && basis.forward[1] > 0);
    assert!(basis.up[0] < 0 && basis.up[2] < 0);
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x28),
        RetailRuntimeValue::Known(0x28)
    );
}
