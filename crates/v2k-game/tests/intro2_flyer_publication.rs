use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue, RetailStateWord},
    entity_view_detail::RetailViewDetailContext,
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_flyers_live::{
        tick_intro2_flyer_scheduler_owner, Intro2FlyerFrame, Intro2FlyerPrimaryVisitResult,
        Intro2FlyerSchedulerOwner, Intro2FlyerSchedulerProductionOutcome,
        INTRO2_TYPE15_ENTITY_TYPE, INTRO2_TYPE15_MODEL_ID, INTRO2_TYPE15_SPAWN_INDEX,
        INTRO2_TYPE87_ENTITY_TYPE, INTRO2_TYPE87_MODEL_ID, INTRO2_TYPE87_SPAWN_INDEX,
    },
    intro2_projectiles::drain_intro2_projectiles,
    opening::intro2_uses_live_actor_pose,
    search_attack::SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{ParticleEnvironment, WorldFx},
};

#[v2k_test_support::retail_test]
fn supported_constructors_follow_authored_order_and_publish_native_null_callbacks() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    let mut draw = 0u32;
    let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        v2k_game::entity::Intro2BirthSelection::default(),
        &mut || {
            draw += 1;
            draw << 8
        },
    )
    .unwrap();
    let type13 = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(0))
        .unwrap();
    let RetailRuntimeValue::Known(payloads) = metadata[13].common_mover_gkl_payloads else {
        panic!()
    };
    let descriptor = payloads.sub_g.unwrap();
    let base = i32::from(i16::from_le_bytes([descriptor[12], descriptor[13]]));
    let RetailRuntimeValue::Known(Some(sub_g)) = type13.sub_g_06070_runtime else {
        panic!()
    };
    assert_eq!(
        sub_g.randomized_target_raw_at_0x38(),
        RetailRuntimeValue::Known(base + 3),
        "spawn zero consumes its 1B8C0, selector, and ACD0 word before later actors"
    );
    assert!(matches!(
        type13.initial_behavior,
        RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 5
    ));
    for entity in manager.iter_all() {
        let callbacks = entity.collision.pair_callbacks;
        if !matches!(entity.entity_type, 0x2e | 0x49 | 0x6d | 0x6e) {
            assert_eq!(
                callbacks.damage_modifier_address,
                RetailRuntimeValue::Known(None)
            );
            assert_eq!(
                callbacks.damage_modifier_identity_context_empty,
                RetailRuntimeValue::Known(true)
            );
        }
        assert_eq!(
            callbacks.type_hit_callback_address,
            RetailRuntimeValue::Known(None)
        );
    }
}

#[v2k_test_support::retail_test]
fn both_flyers_acquire_fire_and_drain_their_authored_projectiles() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    for (spawn_index, method, particle_class, sound_id) in [(44, 30, 87, 70), (46, 20, 52, 92)] {
        let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            v2k_game::entity::Intro2BirthSelection::default(),
            &mut || 0,
        )
        .unwrap();
        let source_id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn_index))
            .unwrap()
            .id;
        let target_id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 9)
            .unwrap()
            .id;
        let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
        for id in ids {
            if id != source_id {
                manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08 = RetailStateWord::exact(0);
            }
        }
        let mut owner =
            Intro2FlyerSchedulerOwner::adopt_published(manager.entity_mut(source_id).unwrap())
                .unwrap();
        let mut world_fx = WorldFx::default();
        let mut total_shots = 0;
        let mut emitter_sounds = 0;
        let mut same_pass_aim_seen = false;
        for tick_index in 0..300 {
            let source = manager.entity_mut(source_id).unwrap();
            let position = source.position_raw();
            let RetailRuntimeValue::Known(basis) = source.physical_body_basis_q31() else {
                panic!()
            };
            RetailViewDetailContext::from_raw(position.map(i32::from), 0, (52, 30))
                .publish(position, &mut source.collision.state_flags_at_0x08);
            let cadence_before = source
                .intro2_flyer_aim_runtime
                .as_ref()
                .unwrap()
                .emitter_runtime()
                .cadence_raw;
            let target = manager.entity_mut(target_id).unwrap();
            target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
            target.capability_flags = 1;
            target.set_motion_raw(
                std::array::from_fn(|axis| {
                    position[axis]
                        .wrapping_add(((i64::from(basis.forward[axis]) * 512) >> 31) as i16)
                }),
                [0; 3],
            );
            let tick = tick_intro2_flyer_scheduler_owner(
                &mut manager,
                owner,
                Intro2FlyerFrame {
                    resources: &session.cache,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: tick_index,
                },
                &mut world_fx,
            );
            let aim = match &tick.outcome {
                Intro2FlyerSchedulerProductionOutcome::B6c0Visit { aim, .. } => {
                    same_pass_aim_seen |= aim.is_some();
                    aim.as_ref()
                }
                Intro2FlyerSchedulerProductionOutcome::PursuingVisit { aim, .. } => aim.as_ref(),
                other => panic!("flyer {spawn_index} frame {tick_index}: {other:?}"),
            };
            assert!(
                aim.is_some(),
                "the admitted target is visited in Tertiary this same pass"
            );
            owner = tick.retained_owner.unwrap();
            let source = manager.entity_mut(source_id).unwrap();
            let runtime = source.intro2_flyer_aim_runtime.as_ref().unwrap();
            let descriptor = runtime.projectile_descriptor();
            assert_eq!(descriptor.projectile_method, method);
            assert_eq!(descriptor.random_interval_us, 600_000);
            assert_eq!(u32::from(descriptor.sound_id), sound_id);
            let requests = runtime.transient_shots().to_vec();
            let expected_cadence = if cadence_before < 1 {
                -20_000
            } else {
                cadence_before - 20_000
            } + requests.len() as i32 * 600_000;
            assert_eq!(runtime.emitter_runtime().cadence_raw, expected_cadence);
            if requests.is_empty() {
                continue;
            }
            assert!(requests
                .iter()
                .all(|request| request.projectile_method == method));
            // Drain deliberately re-reads source velocity. Make that boundary
            // observable without altering the already-recorded shot direction.
            let inherited = [123i16, -57, 91];
            source.set_motion_raw(source.position_raw(), inherited);
            let expected_velocities: Vec<[i16; 3]> = requests
                .iter()
                .map(|request| {
                    let GenericEmitterSpeedField::Explicit(speed) = request.speed_field else {
                        panic!()
                    };
                    let speed = if speed == 0 {
                        v2k_game::projectile_emitter::projectile_class_row(method)
                            .unwrap()
                            .speed_raw as i16
                    } else {
                        speed
                    };
                    std::array::from_fn(|axis| {
                        inherited[axis].wrapping_add(
                            ((i64::from(request.direction_raw[axis]) * i64::from(speed)) >> 31)
                                as i16,
                        )
                    })
                })
                .collect();
            let prior_count = world_fx.particle_count();
            let drains = drain_intro2_projectiles(
                &mut manager,
                &mut world_fx,
                &session.cache,
                ParticleEnvironment::Dry,
                tick_index,
            );
            assert_eq!(drains.len(), 1);
            assert_eq!(drains[0].0, source_id);
            assert_eq!(*drains[0].1.as_ref().unwrap(), requests.len());
            assert_eq!(world_fx.particle_count(), prior_count + requests.len());
            assert_eq!(
                manager
                    .entity_mut(source_id)
                    .unwrap()
                    .intro2_flyer_aim_runtime
                    .as_ref()
                    .unwrap()
                    .queued_shot_count(),
                0
            );
            let presentation = world_fx.prepare_presentation([640, 480], 100_000, |_| {
                v2k_render::ParticleCenterProjection {
                    screen: [320, 240],
                    depth_raw: 1000,
                    clip: 0,
                }
            });
            let particles: Vec<_> = presentation
                .particles()
                .map(|prepared| &prepared.particle)
                .collect();
            for mut velocity in expected_velocities {
                velocity[1] = velocity[1].wrapping_add(
                    v2k_game::particle_descriptors::particle_descriptor(particle_class)
                        .unwrap()
                        .spawn_velocity_y_bias_raw(),
                );
                assert!(
                    particles
                        .iter()
                        .any(|particle| particle.source_class == particle_class
                            && particle.owner_id == Some(source_id)
                            && particle.velocity.map(|value| (value
                                * 256.0
                                * (1_048_576.0 / 1_000_000.0))
                                .round()
                                as i16)
                                == velocity),
                    "method {method} particle retains source velocity at drain"
                );
            }
            total_shots += requests.len();
            world_fx.process_pending();
            emitter_sounds += world_fx
                .take_positional_sounds()
                .into_iter()
                .filter(|sound| sound.sound_id == sound_id as usize)
                .count();
        }
        assert!(same_pass_aim_seen);
        assert!(total_shots > 0, "method {method} never emitted");
        assert!(
            emitter_sounds > 0,
            "method {method} sound was not published"
        );
    }
}

#[v2k_test_support::retail_test]
fn flyer_world_retains_custody_through_root_expiry_acquisition_and_chase_expiry() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = intro2_type_metadata(&session);
    let mut manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        v2k_game::entity::Intro2BirthSelection::default(),
        &mut || 0,
    )
    .unwrap();
    let source_id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(44))
        .unwrap()
        .id;
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    // First isolate an empty eligible target set to exercise strict 500-ms
    // root expiry; later introduce one real type-9 allocation in range.
    for id in ids {
        if id != source_id {
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08 = RetailStateWord::exact(0);
        }
    }
    let source = manager.entity_mut(source_id).unwrap();
    assert_eq!(
        source.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    source.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(321);
    let mut owner = Intro2FlyerSchedulerOwner::adopt_published(source).unwrap();
    let mut world_fx = WorldFx::default();
    let mut roots_reselected = 0;
    let mut pursuing_visits = 0;
    let mut chase_reselected = 0;
    for tick_index in 0..300u32 {
        let source = manager.entity_mut(source_id).unwrap();
        let position = source.position_raw();
        RetailViewDetailContext::from_raw(position.map(i32::from), 0, (52, 30))
            .publish(position, &mut source.collision.state_flags_at_0x08);
        if tick_index >= 27 {
            let target = manager.entity_mut(target_id).unwrap();
            target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
            target.capability_flags = 1;
            target.set_motion_raw(
                [position[0], position[1], position[2].wrapping_add(1000)],
                [0; 3],
            );
        }
        let result = tick_intro2_flyer_scheduler_owner(
            &mut manager,
            owner,
            Intro2FlyerFrame {
                resources: &session.cache,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: tick_index,
            },
            &mut world_fx,
        );
        match &result.outcome {
            Intro2FlyerSchedulerProductionOutcome::B6c0Visit {
                reselected,
                primary,
                ..
            } => {
                assert_eq!(primary.result, Intro2FlyerPrimaryVisitResult::Continue);
                roots_reselected += usize::from(*reselected);
            }
            Intro2FlyerSchedulerProductionOutcome::PursuingVisit { reselected, .. } => {
                pursuing_visits += 1;
                chase_reselected += usize::from(*reselected);
            }
            other => panic!("flyer frame {tick_index} stopped: {other:?}"),
        }
        owner = result
            .retained_owner
            .expect("live acquiring/pursuing custody survives");
        let source = manager.entity_mut(source_id).unwrap();
        assert_eq!(
            source.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(source.mass_raw, if tick_index == 0 { 421 } else { 100 });
        if tick_index == 24 {
            let Some(ActorTaskRuntime::SharedRetarget(task)) =
                source.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("the initial root remains live")
            };
            assert_eq!(task.elapsed_ms(), 500, "500ms is not expired");
        }
        if tick_index == 25 {
            let Some(ActorTaskRuntime::SharedRetarget(task)) =
                source.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("C690 publishes the new root")
            };
            assert_eq!(task.elapsed_ms(), 0, "520ms reselects the root");
        }
        if tick_index == 27 {
            let Some(ActorTaskRuntime::ChaseTarget(task)) =
                source.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("same-pass ADE0 publishes the acquired Chase");
            };
            assert_eq!(
                task.elapsed_ms(),
                0,
                "newborn Primary does not tick twice in the acquisition pass"
            );
            assert!(matches!(
                source.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AimAndFire(_))
            ));
        }
    }
    assert!(roots_reselected >= 1);
    assert!(pursuing_visits > 250);
    assert!(
        chase_reselected >= 1,
        "strict 5000ms Chase expiry returns through C690/ADE0"
    );
}

fn intro2_type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
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
        .collect()
}

#[v2k_test_support::retail_test]
fn captured_intro2_route_publishes_flyer_graphs_and_live_pose() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("initialize retail-data session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier cumulative Section-12 records");
    session
        .load_level_by_id(50, 1)
        .expect("load normal-tier Intro2");

    let type_metadata = intro2_type_metadata(&session);
    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let model_extent = |id| session.cache.global_model(id).map(|model| model.radius);
    let resources = EntityConstructionResources {
        terrain: session.cache.terrain(),
        terrain_objects: session.cache.terrain_objects(),
        model_extent_raw: Some(&model_extent),
    };

    // 1. Direct level 50 construction keeps initial behavior and frame owner unresolved
    let direct = EntityManager::from_level_with_type_metadata(level, &type_metadata, resources);
    for (spawn_index, expected_type) in [
        (INTRO2_TYPE15_SPAWN_INDEX, INTRO2_TYPE15_ENTITY_TYPE),
        (INTRO2_TYPE87_SPAWN_INDEX, INTRO2_TYPE87_ENTITY_TYPE),
    ] {
        let entity = direct
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn_index))
            .expect("Intro2 authored flyer spawn");
        assert_eq!(entity.entity_type, expected_type);
        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!("direct construction resolves single-choice Section 12 behavior");
        };
        assert_eq!(selection.choice_index, 0);
        assert_eq!(selection.program.class_id, SEARCH_ATTACK_BEHAVIOR_CLASS_ID);
        assert_eq!(
            entity.current_behavior_context,
            RetailRuntimeValue::Unresolved
        );
        assert!(entity.intro2_flyer_frame_owner.is_none());
        assert!(!intro2_uses_live_actor_pose(entity));
        for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
            assert_eq!(entity.actor_task_state(slot), None);
        }
    }

    // 2. Captured frontend-to-Intro2 route publishes both flyers into Class 7
    let mut captured = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        level,
        &type_metadata,
        resources,
        v2k_game::entity::Intro2BirthSelection::default(),
        &mut || 0,
    )
    .expect("accepted frontend-to-Intro2 constructor receipt");

    for (spawn_index, expected_type, expected_model) in [
        (
            INTRO2_TYPE15_SPAWN_INDEX,
            INTRO2_TYPE15_ENTITY_TYPE,
            INTRO2_TYPE15_MODEL_ID,
        ),
        (
            INTRO2_TYPE87_SPAWN_INDEX,
            INTRO2_TYPE87_ENTITY_TYPE,
            INTRO2_TYPE87_MODEL_ID,
        ),
    ] {
        let entity = captured
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(spawn_index))
            .expect("captured flyer entity");
        assert_eq!(entity.entity_type, expected_type);
        assert_eq!(entity.model_index, Some(expected_model as usize));

        let RetailRuntimeValue::Known(Some(selection)) = entity.initial_behavior else {
            panic!("captured route must publish flyer selection");
        };
        assert_eq!(selection.choice_index, 0);
        assert_eq!(selection.program.class_id, SEARCH_ATTACK_BEHAVIOR_CLASS_ID);

        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        ));
        assert_eq!(entity.actor_task_state(ActorTaskSlot::Tertiary), None);

        let owner = entity
            .intro2_flyer_frame_owner
            .expect("captured flyer must retain live frame owner");
        assert_eq!(
            owner.birth_provenance.authored_spawn_index(),
            Some(spawn_index)
        );
        assert_eq!(owner.entity_type, expected_type);

        // Crucial requirement: admitted to live actor pose
        assert!(intro2_uses_live_actor_pose(entity));

        // Presentation anim vars: wing oscillation published to dynamic[1]
        let anim_vars = entity.presentation_anim_vars(0);
        assert_eq!(anim_vars.dynamic[1], owner.wing_var1);
    }

    // 3. Scheduler adoption
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_flyers(&captured), 2);
    let mut empty_scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(empty_scheduler.adopt_intro2_flyers(&direct), 0);

    // 4. Scheduler tick evaluates common mover and retains owner
    let owners = captured
        .iter_all()
        .filter_map(|entity| Intro2FlyerSchedulerOwner::adopt_published(entity).ok())
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 2);

    let mut world_fx = WorldFx::default();
    for owner in owners {
        let entity_id = owner.entity_id();
        let spawn_index = owner.birth_provenance().authored_spawn_index();
        let entity = captured.entity_mut(entity_id).unwrap();
        let view =
            RetailViewDetailContext::from_raw(entity.position_raw().map(i32::from), 0, (52, 30));
        view.publish(
            entity.position_raw(),
            &mut entity.collision.state_flags_at_0x08,
        );
        let tick = tick_intro2_flyer_scheduler_owner(
            &mut captured,
            owner,
            Intro2FlyerFrame {
                resources: &session.cache,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
        );

        assert!(tick.retained_owner.is_some());
        match tick.outcome {
            Intro2FlyerSchedulerProductionOutcome::B6c0Visit {
                entity_id: ticked_id,
                primary,
                ..
            } => {
                assert_eq!(ticked_id, entity_id);
                assert_eq!(primary.result, Intro2FlyerPrimaryVisitResult::Continue);
            }
            other => panic!("expected B6c0Visit outcome, got: {other:?}"),
        }

        let entity = captured
            .iter_all()
            .find(|e| e.id == entity_id)
            .expect("ticked entity");
        assert_eq!(entity.authored_spawn_index, spawn_index);
        assert!(matches!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(_)
        ));
        assert!(intro2_uses_live_actor_pose(entity));
    }
}
