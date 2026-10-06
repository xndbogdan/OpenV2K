use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    defecate_virus::{DefecateVirusCallbackPlan, DEFECATE_VIRUS_WANDER_LIFETIME_MS},
    defecate_virus_owner::DEFECATE_VIRUS_TERRAIN_MODE,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_view_detail::{
        RetailViewDetail, RetailViewDetailContext, BROADER_DETAIL_STATE_BIT,
        VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT, VIEW_DETAIL_STATE_MASK,
    },
    infection_evolution::INFECTION_TERRAIN_TYPE_BIT,
    intro2_type26_defecate_virus::{
        apply_intro2_type26_tertiary_plan, intro2_type26_model_extent_raw_by_state,
        tick_intro2_type26_scheduler_owner, Intro2Type26PrimaryVisitResult,
        Intro2Type26SchedulerOwner, Intro2Type26SchedulerProductionOutcome,
        Intro2Type26TertiaryVisitResult, INTRO2_DEFECATE_VIRUS_BEHAVIOR_CLASS_ID,
        INTRO2_DEFECATE_VIRUS_CHOICE_INDEX, INTRO2_DEFECATE_VIRUS_MODEL_ID,
        INTRO2_DEFECATE_VIRUS_SPAWN_INDEX, INTRO2_DEFECATE_VIRUS_TERRAIN_LIFETIME_MS,
        INTRO2_TYPE26_BE00_0F00_SUB_D_SEED, INTRO2_TYPE26_SPAWN10_INDEX,
        INTRO2_TYPE26_SPAWN25_SUB_D_SEED, TYPE26_SUB_H_RECORD_COUNT,
    },
    opening::intro2_uses_live_actor_pose,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{ParticleUpdateRequest, TerrainCollisionContext, WorldFx},
};

fn spawn25_model_records(session: &GameSession) -> Option<&[[i16; 4]]> {
    session
        .cache
        .global_model(INTRO2_DEFECATE_VIRUS_MODEL_ID)
        .map(|model| model.records.as_slice())
}

#[v2k_test_support::retail_test]
fn captured_intro2_route_publishes_only_the_authenticated_type26_receipt() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

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
    assert_eq!(
        type_metadata[26].terrain_contact_task_lifetime_ms,
        RetailRuntimeValue::Known(INTRO2_DEFECATE_VIRUS_TERRAIN_LIFETIME_MS)
    );

    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let direct = EntityManager::from_level_with_type_metadata(level, &type_metadata, resources);
    for (spawn_index, expected_angles) in [
        (INTRO2_TYPE26_SPAWN10_INDEX, [-0x8000i16, 0, 0]),
        (INTRO2_DEFECATE_VIRUS_SPAWN_INDEX, [0x4000i16, 0, 0]),
    ] {
        let direct_entity = direct
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn_index))
            .unwrap_or_else(|| panic!("Intro2 authored spawn {spawn_index}"));
        assert_eq!(direct_entity.entity_type, 26);
        assert_eq!(direct_entity.heading_raw(), expected_angles[0] as u16);
        assert_eq!(
            direct_entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                expected_angles[0],
                expected_angles[1],
                expected_angles[2],
            ))
        );
        assert_eq!(
            direct_entity.initial_behavior,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            direct_entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(direct_entity.actor_task_state(slot), None);
        }
    }

    let captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection { type26: v2k_game::intro2_type26_defecate_virus::Intro2Type26BirthSelection::CapturedDefecate, ..Default::default() },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    for (spawn_index, expected_seed, expected_angles) in [
        (
            INTRO2_TYPE26_SPAWN10_INDEX,
            INTRO2_TYPE26_BE00_0F00_SUB_D_SEED,
            [-0x8000i16, 0, 0],
        ),
        (
            INTRO2_DEFECATE_VIRUS_SPAWN_INDEX,
            INTRO2_TYPE26_SPAWN25_SUB_D_SEED,
            [0x4000i16, 0, 0],
        ),
    ] {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn_index))
            .unwrap_or_else(|| panic!("captured Intro2 authored spawn {spawn_index}"));

        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!("captured route must publish the accepted weighted selection for spawn {spawn_index}")
        };
        assert_eq!(selection.choice_index, INTRO2_DEFECATE_VIRUS_CHOICE_INDEX);
        assert_eq!(
            selection.program.class_id,
            INTRO2_DEFECATE_VIRUS_BEHAVIOR_CLASS_ID
        );
        assert_eq!(selection.program.initial_style.frame_address, 0x004C_7E88);

        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!(
                "captured route must publish the accepted behavior context for spawn {spawn_index}"
            )
        };
        assert_eq!(context.active_style().style_address(), 0x004C_7E88);

        let Some(ActorTaskRuntime::DefecateVirusWander(wander)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("Primary must own the duration-2000 Wander task for spawn {spawn_index}")
        };
        assert_eq!(DEFECATE_VIRUS_WANDER_LIFETIME_MS, 2_000);
        assert_eq!(wander.elapsed_ms(), 0);
        assert_eq!(
            wander.private_state().target_position_raw,
            entity.position_raw()
        );
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
        let Some(ActorTaskRuntime::DefecateVirusTerrain(terrain)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("Tertiary must own the mode-5 terrain task for spawn {spawn_index}")
        };
        assert_eq!(DEFECATE_VIRUS_TERRAIN_MODE, 5);
        assert_eq!(
            terrain.lifetime_ms(),
            u32::from(INTRO2_DEFECATE_VIRUS_TERRAIN_LIFETIME_MS)
        );
        assert_eq!(terrain.elapsed_ms(), 0);
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                expected_angles[0],
                expected_angles[1],
                expected_angles[2],
            ))
        );
        let owner = entity
            .intro2_type26_sub_d_frame_owner
            .unwrap_or_else(|| panic!("spawn {spawn_index} must retain the TTD first-query reset"));
        assert!(owner.classifier_cache().can_classify());
        assert_eq!(owner.classifier_cache().stagger_counter(), expected_seed);
        assert!(intro2_uses_live_actor_pose(entity));
    }

    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type26(&captured), 2);
    let mut empty = SpecializedActorTaskScheduler::new();
    assert_eq!(empty.adopt_intro2_type26(&direct), 0);
}

#[v2k_test_support::retail_test]
fn captured_intro2_detailed_callback_uses_constructor_basis_and_section8_extent() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

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
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection { type26: v2k_game::intro2_type26_defecate_virus::Intro2Type26BirthSelection::CapturedDefecate, ..Default::default() },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let header_extent_raw = session
        .cache
        .global_model(INTRO2_DEFECATE_VIRUS_MODEL_ID)
        .map(|model| model.radius)
        .expect("Intro2 model 267 Section-8 header +0x08");
    assert_ne!(header_extent_raw, 0, "authored stag extent is nonzero");

    let extents = {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
            .expect("captured Intro2 authored spawn 25");
        let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
            panic!("generic FUN_0040D720 must have written constructor FUN_00413F70")
        };
        assert_eq!(basis, Type9BodyBasis::from_angle_words(0x4000, 0, 0));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BROADER_DETAIL_STATE_BIT),
            RetailRuntimeValue::Known(BROADER_DETAIL_STATE_BIT),
            "constructor view-detail bits admit Detailed FUN_00402850"
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("type-26 constructor must attach six-record Sub-H")
        };
        assert_eq!(sub_h.records().len(), TYPE26_SUB_H_RECORD_COUNT);
        intro2_type26_model_extent_raw_by_state(entity, |model_id| {
            session
                .cache
                .global_model(model_id)
                .map(|model| model.radius)
        })
    };
    assert_eq!(extents, [Some(header_extent_raw); 4]);

    let owner = Intro2Type26SchedulerOwner::adopt(&captured).expect("published spawn 25");
    let tick = tick_intro2_type26_scheduler_owner(
        &mut captured,
        owner,
        session.cache.terrain(),
        spawn25_model_records(&session),
        extents,
        19_500,
        &mut || 0,
    );
    let Intro2Type26SchedulerProductionOutcome::Primary {
        visit, tertiary, ..
    } = tick.outcome
    else {
        panic!("expected Primary+Tertiary visit, got {:?}", tick.outcome);
    };
    assert_eq!(visit.result, Intro2Type26PrimaryVisitResult::Continue);
    let Intro2Type26TertiaryVisitResult::Planned(DefecateVirusCallbackPlan::DetailedParticle(
        emission,
    )) = tertiary.result
    else {
        panic!(
            "expected DetailedParticle from constructor basis and header +0x08, got {:?}",
            tertiary.result
        );
    };

    let entity = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
        .expect("spawn 25 survives the visit");
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        panic!("constructor matrix must survive the Primary visit")
    };
    let extent_offset =
        |component: i32| ((i64::from(component) * i64::from(header_extent_raw)) >> 31) as i16;
    assert_eq!(
        emission.position_raw(),
        [
            entity.position_raw()[0].wrapping_sub(extent_offset(basis.forward[0])),
            entity.position_raw()[1]
                .wrapping_sub(extent_offset(basis.forward[1]))
                .wrapping_add(100),
            entity.position_raw()[2].wrapping_sub(extent_offset(basis.forward[2])),
        ]
    );
    assert_eq!(emission.scale_raw(), 0x0800);
    assert_eq!(emission.owner_entity_handle(), entity.id);
    assert_eq!(
        emission.particle_class(),
        v2k_game::defecate_virus::DEFECATE_VIRUS_PARTICLE_CLASS
    );

    let mut world_fx = WorldFx::new();
    let mut infection_writes = Vec::new();
    apply_intro2_type26_tertiary_plan(&mut world_fx, &mut infection_writes, &tertiary);
    assert!(infection_writes.is_empty());
    assert_eq!(
        world_fx.particle_count(),
        1,
        "cold FUN_00440DC0 promotes scale 0x0800 to one class-5 carrier"
    );
}

#[v2k_test_support::retail_test]
fn captured_intro2_detailed_class5_carrier_writes_ordinary_surface_infection() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

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
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection { type26: v2k_game::intro2_type26_defecate_virus::Intro2Type26BirthSelection::CapturedDefecate, ..Default::default() },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let extents = {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
            .expect("captured Intro2 authored spawn 25");
        intro2_type26_model_extent_raw_by_state(entity, |model_id| {
            session
                .cache
                .global_model(model_id)
                .map(|model| model.radius)
        })
    };

    let owner = Intro2Type26SchedulerOwner::adopt(&captured).expect("published spawn 25");
    let tick = tick_intro2_type26_scheduler_owner(
        &mut captured,
        owner,
        session.cache.terrain(),
        spawn25_model_records(&session),
        extents,
        19_500,
        &mut || 0,
    );
    let Intro2Type26SchedulerProductionOutcome::Primary { tertiary, .. } = tick.outcome else {
        panic!("expected Primary+Tertiary visit, got {:?}", tick.outcome);
    };
    assert!(
        matches!(
            tertiary.result,
            Intro2Type26TertiaryVisitResult::Planned(DefecateVirusCallbackPlan::DetailedParticle(
                _
            ))
        ),
        "captured Detailed FUN_00402850 must emit a class-5 carrier, got {:?}",
        tertiary.result
    );

    let mut world_fx = WorldFx::new();
    let mut coarse_writes = Vec::new();
    apply_intro2_type26_tertiary_plan(&mut world_fx, &mut coarse_writes, &tertiary);
    assert!(coarse_writes.is_empty());
    assert_eq!(world_fx.particle_count(), 1);

    // FUN_0043E180 ordinary-surface contact deletes the carrier before the
    // descriptor's 200-tick lifetime. Stay under that bound so expiry cannot
    // masquerade as a surface hit.
    const SURFACE_TICKS: u32 = 180;
    const FRAME_MICROS: u32 = 20_000;
    let mut surface_writes = Vec::new();
    for retail_tick in 1..=SURFACE_TICKS {
        let frame_writes = {
            let context = TerrainCollisionContext::from_current_level_cache(&session.cache)
                .expect("Intro2 Section 10/13 collision context");
            world_fx
                .update(ParticleUpdateRequest::terrain(
                    FRAME_MICROS,
                    retail_tick,
                    context,
                ))
                .terrain_type_mutations
        };
        if !frame_writes.is_empty() {
            let changed = session
                .cache
                .apply_particle_terrain_mutations(&frame_writes)
                .expect("live Intro2 terrain");
            assert_eq!(
                changed,
                frame_writes.len(),
                "ordinary-surface writes must change bit 0x10: {frame_writes:?}"
            );
            surface_writes.extend(frame_writes);
        }
        if world_fx.particle_count() == 0 {
            break;
        }
    }

    assert_eq!(
        world_fx.particle_count(),
        0,
        "ordinary-surface FUN_0043E180 must remove the captured class-5 carrier"
    );
    assert!(
        !surface_writes.is_empty(),
        "ordinary-surface FUN_0043E180 must emit at least one infection write"
    );
    assert!(
        surface_writes.iter().all(|write| matches!(
            write,
            v2k_game::world_fx::ParticleTerrainMutation::Infection { infected: true, .. }
        )),
        "class-5 ordinary surface can only set bit 0x10, got {surface_writes:?}"
    );

    let terrain = session.cache.terrain().expect("Intro2 terrain");
    for write in &surface_writes {
        let v2k_game::world_fx::ParticleTerrainMutation::Infection {
            cell: written_cell, ..
        } = write
        else {
            panic!("class5 cannot burn a static object")
        };
        let cell = terrain
            .cell(usize::from(written_cell[0]), usize::from(written_cell[1]))
            .expect("written cell stays in the 256x256 grid");
        assert_ne!(
            cell.terrain_type & INFECTION_TERRAIN_TYPE_BIT,
            0,
            "applied FUN_00433720 write must leave cell {:?} infected",
            written_cell
        );
    }
}

#[v2k_test_support::retail_test]
fn captured_intro2_coarse_callback_writes_direct_infection() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

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
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection { type26: v2k_game::intro2_type26_defecate_virus::Intro2Type26BirthSelection::CapturedDefecate, ..Default::default() },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let extents = {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
            .expect("captured Intro2 authored spawn 25");
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT),
            RetailRuntimeValue::Known(VIEW_DETAIL_CLASSIFICATION_ENABLED_STATE_BIT),
            "constructor 0x800 admits FUN_00411400"
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BROADER_DETAIL_STATE_BIT),
            RetailRuntimeValue::Known(BROADER_DETAIL_STATE_BIT),
            "constructor bits still admit Detailed until FUN_00411400"
        );
        intro2_type26_model_extent_raw_by_state(entity, |model_id| {
            session
                .cache
                .global_model(model_id)
                .map(|model| model.radius)
        })
    };

    let context = RetailViewDetailContext::from_raw([0, 0, 0], 0, (52, 30));
    {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
            .expect("spawn 25");
        assert_eq!(
            context.classify(entity.position_raw()),
            RetailViewDetail::Coarse
        );
    }

    let classified =
        v2k_game::intro2_type26_defecate_virus::publish_captured_intro2_type26_view_detail(
            &mut captured,
            context,
        )
        .expect("spawn 25 admits FUN_00411400");
    assert_eq!(
        classified,
        RetailRuntimeValue::Known(Some(RetailViewDetail::Coarse))
    );
    let entity = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
        .expect("spawn 25 after Coarse publish");
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(VIEW_DETAIL_STATE_MASK),
        RetailRuntimeValue::Known(0),
        "FUN_00411400 Coarse clears 0x06000000"
    );

    let owner = Intro2Type26SchedulerOwner::adopt(&captured).expect("published spawn 25");
    let tick = tick_intro2_type26_scheduler_owner(
        &mut captured,
        owner,
        session.cache.terrain(),
        spawn25_model_records(&session),
        extents,
        19_500,
        &mut || 0,
    );
    let Intro2Type26SchedulerProductionOutcome::Primary { tertiary, .. } = tick.outcome else {
        panic!("expected Primary+Tertiary visit, got {:?}", tick.outcome);
    };
    let Intro2Type26TertiaryVisitResult::Planned(DefecateVirusCallbackPlan::CoarseTerrainMutation(
        write,
    )) = tertiary.result
    else {
        panic!(
            "expected coarse FUN_00402850 mutation, got {:?}",
            tertiary.result
        );
    };
    assert!(write.infected, "mode 5 coarse FUN_00433720 sets bit 0x10");

    let position_raw = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
        .expect("spawn 25 survives the visit")
        .position_raw();
    assert_eq!(
        write.cell,
        [
            ((position_raw[0].wrapping_sub(0x0100) as u16) >> 8) as u8,
            ((position_raw[2].wrapping_sub(0x0100) as u16) >> 8) as u8,
        ],
        "coarse Z then X use (random16 >> 7) - 256"
    );

    let mut world_fx = WorldFx::new();
    let mut infection_writes = Vec::new();
    apply_intro2_type26_tertiary_plan(&mut world_fx, &mut infection_writes, &tertiary);
    assert_eq!(
        world_fx.particle_count(),
        0,
        "coarse mode creates no carrier"
    );
    assert_eq!(infection_writes, [write]);
    let changed = session
        .cache
        .apply_level_infection_writes(&infection_writes)
        .expect("live Intro2 terrain");
    assert_eq!(changed, 1);
    let terrain = session.cache.terrain().expect("Intro2 terrain");
    let cell = terrain
        .cell(usize::from(write.cell[0]), usize::from(write.cell[1]))
        .expect("written cell stays in the 256x256 grid");
    assert_ne!(cell.terrain_type & INFECTION_TERRAIN_TYPE_BIT, 0);
}

#[v2k_test_support::retail_test]
fn captured_intro2_type26_translates_after_master_motion() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

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
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection { type26: v2k_game::intro2_type26_defecate_virus::Intro2Type26BirthSelection::CapturedDefecate, ..Default::default() },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let extents = {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
            .expect("captured Intro2 authored spawn 25");
        assert!(intro2_uses_live_actor_pose(entity));
        intro2_type26_model_extent_raw_by_state(entity, |model_id| {
            session
                .cache
                .global_model(model_id)
                .map(|model| model.radius)
        })
    };
    let start = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
        .expect("spawn 25")
        .position_raw();
    let mut owner = Intro2Type26SchedulerOwner::adopt(&captured).expect("published spawn 25");
    for frame in 0..90 {
        let tick = tick_intro2_type26_scheduler_owner(
            &mut captured,
            owner,
            session.cache.terrain(),
            spawn25_model_records(&session),
            extents,
            19_500,
            &mut || u32::from(frame == 0),
        );
        match tick.outcome {
            Intro2Type26SchedulerProductionOutcome::Primary { visit, .. } => {
                assert!(
                    matches!(
                        visit.result,
                        Intro2Type26PrimaryVisitResult::Continue
                            | Intro2Type26PrimaryVisitResult::TaggedZeroMover
                    ),
                    "spawn 25 blocked on frame {frame}: {:?}",
                    visit.result
                );
            }
            other => panic!("expected Primary+Tertiary visit, got {other:?}"),
        }
        owner = tick.retained_owner.expect("spawn 25 owner stays live");
    }
    let entity = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_DEFECATE_VIRUS_SPAWN_INDEX))
        .expect("spawn 25 survives");
    let end = entity.position_raw();
    let vel = entity.velocity_raw();
    // Sub-C restates clearance and FUN_00412DA0 integrates that Y. Horizontal
    // walk still waits on Sub-A writing an XZ velocity; do not invent one.
    assert_ne!(
        start, end,
        "Intro2 Type-26 live pose must leave the constructor rest pose, start={start:?} end={end:?} vel={vel:?} flags={:?}",
        entity.collision.state_flags_at_0x08
    );
    let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
        panic!("Intro2 Type-26 must own live six-record Sub-H");
    };
    assert_eq!(sub_h.records().len(), TYPE26_SUB_H_RECORD_COUNT);
    assert!(
        sub_h.records().iter().all(|record| record.flags_raw & 7 == 0),
        "Mover-only visits cannot publish D360 flags without selected model presentation, flags={:08x?}",
        sub_h
            .records()
            .iter()
            .map(|record| record.flags_raw)
            .collect::<Vec<_>>()
    );
}

#[v2k_test_support::retail_test]
fn captured_intro2_type26_spawn10_translates_after_master_motion() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

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
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection { type26: v2k_game::intro2_type26_defecate_virus::Intro2Type26BirthSelection::CapturedDefecate, ..Default::default() },
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    let extents = {
        let entity = captured
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE26_SPAWN10_INDEX))
            .expect("captured Intro2 authored spawn 10");
        assert!(intro2_uses_live_actor_pose(entity));
        intro2_type26_model_extent_raw_by_state(entity, |model_id| {
            session
                .cache
                .global_model(model_id)
                .map(|model| model.radius)
        })
    };
    let start = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE26_SPAWN10_INDEX))
        .expect("spawn 10")
        .position_raw();
    let mut owner = Intro2Type26SchedulerOwner::adopt_spawn(&captured, INTRO2_TYPE26_SPAWN10_INDEX)
        .expect("published spawn 10");
    for frame in 0..90 {
        let tick = tick_intro2_type26_scheduler_owner(
            &mut captured,
            owner,
            session.cache.terrain(),
            spawn25_model_records(&session),
            extents,
            19_500,
            &mut || u32::from(frame == 0),
        );
        match tick.outcome {
            Intro2Type26SchedulerProductionOutcome::Primary { visit, .. } => {
                assert!(
                    matches!(
                        visit.result,
                        Intro2Type26PrimaryVisitResult::Continue
                            | Intro2Type26PrimaryVisitResult::TaggedZeroMover
                    ),
                    "spawn 10 blocked on frame {frame}: {:?}",
                    visit.result
                );
            }
            other => panic!("expected Primary+Tertiary visit, got {other:?}"),
        }
        owner = tick.retained_owner.expect("spawn 10 owner stays live");
    }
    let entity = captured
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(INTRO2_TYPE26_SPAWN10_INDEX))
        .expect("spawn 10 survives");
    let end = entity.position_raw();
    let vel = entity.velocity_raw();
    assert_ne!(
        start, end,
        "Intro2 Type-26 spawn 10 live pose must leave the constructor rest pose, start={start:?} end={end:?} vel={vel:?} flags={:?}",
        entity.collision.state_flags_at_0x08
    );
    let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
        panic!("Intro2 Type-26 spawn 10 must own live six-record Sub-H");
    };
    assert_eq!(sub_h.records().len(), TYPE26_SUB_H_RECORD_COUNT);
    assert!(
        sub_h.records().iter().all(|record| record.flags_raw & 7 == 0),
        "Mover-only visits cannot publish D360 flags without selected model presentation, flags={:08x?}",
        sub_h
            .records()
            .iter()
            .map(|record| record.flags_raw)
            .collect::<Vec<_>>()
    );
}
