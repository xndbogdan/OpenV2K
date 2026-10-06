use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::entity::{
    AuthoredWorldConstruction, EntityConstructionResources, EntityManager,
    MainBaseConversionDestroyQueueOutcome,
};
use v2k_game::entity_collision_state::{
    EntityTypeRuntimeMetadata, PairComponentContactPolicy, RetailRuntimeValue,
};
use v2k_game::factory_arrival_production::LevelOneFactoryArrivalFrame;
use v2k_game::factory_delivery::FactoryDeliveryPairSuffix;
use v2k_game::factory_pair_live::{
    resolve_factory_pair_arrivals, FactoryPairPassResult, FactoryPairRequest,
};
use v2k_game::factory_pair_suffix::{
    FactoryPairCandidateBehavior, FactoryPairCandidateComponents, FactoryPairPhysical,
    FactoryPairSubjectComponents,
};
use v2k_game::factory_production::FactoryProductionPhase;
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::main_base_conversion::MainBaseReplacementSpawn;
use v2k_game::main_base_conversion_runtime::{
    apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
};
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler;
use v2k_game::world_fx::WorldFx;

const FACTORY_SPAWN_INDEX: usize = 23;
const MAIN_BASE_SPAWN_INDEX: usize = 6;
const FACTORY_POSITION_RAW: [i16; 3] = [0x5700, -0x0300, 0x3A00];

#[v2k_test_support::retail_test]
fn main_load_order_keeps_native_spawn23_factory_with_its_native_scheduler() {
    use v2k_game::factory_arrival_production::{
        LevelOneFactoryArrivalAdoptionError, LevelOneFactoryArrivalOwner,
    };
    use v2k_game::specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    };
    use v2k_game::static_damage::StaticDamageScheduler;

    for (level, factory_count) in [(13, 1), (25, 13)] {
        let (mut session, mut entities, mut fx) = native_world(level);
        let factory = entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == 66 && entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX)
            })
            .unwrap();
        assert!(matches!(
            LevelOneFactoryArrivalOwner::adopt_published(factory),
            Err(LevelOneFactoryArrivalAdoptionError::NativeType66Owner { .. })
        ));
        let mut scheduler = SpecializedActorTaskScheduler::new();
        // Match main.rs's load order: selected Type9, then the legacy arrival
        // adapter, then native Type66. Identity must not depend on call order.
        scheduler
            .adopt_fresh_level1_type9_selected(&mut entities)
            .unwrap();
        assert_eq!(scheduler.adopt_level_one_factory_arrival(&entities), 0);
        assert_eq!(scheduler.adopt_intro2_type66(&entities), factory_count);
        // Playing repeats legacy adoption before every task pass.
        assert_eq!(scheduler.adopt_level_one_factory_arrival(&entities), 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let unadopted_scientist_id = entities
            .iter_all()
            .find(|entity| entity.entity_type == 8)
            .map(|entity| entity.id);
        if let Some(scientist_id) = unadopted_scientist_id {
            let factory_id = entities
                .iter_all()
                .find(|entity| entity.entity_type == 66)
                .unwrap()
                .id;
            let collision_before = entities.entity_mut(scientist_id).unwrap().collision.clone();
            assert!(
                scheduler
                    .apply_factory_pair_arrival(
                        &mut entities,
                        &mut fx,
                        &mut notifications,
                        factory_id,
                        scientist_id,
                        0
                    )
                    .is_none(),
                "a factory owner alone cannot consume an unadopted native worker"
            );
            assert_eq!(
                entities.entity_mut(scientist_id).unwrap().collision,
                collision_before
            );
        }
        let pass = scheduler.tick(
            &mut entities,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 100_000,
                global_elapsed_micros: 100_000,
                retail_tick: 5,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "level{level}: {:?}", pass.block);
        assert_eq!(
            pass.outcomes
                .iter()
                .filter(|outcome| matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::Intro2Type66(_)
                ))
                .count(),
            factory_count
        );
        assert!(pass.outcomes.iter().all(|outcome| !matches!(
            outcome,
            SpecializedActorTaskProductionOutcome::LevelOneFactoryArrival(_)
        )));
    }
}

#[v2k_test_support::retail_test]
fn retained_legacy_factory_owner_rejects_native_publication_before_task_prefix() {
    use v2k_game::factory_arrival_production::{
        tick_level_one_factory_arrival_owner, LevelOneFactoryArrivalOwner,
        LevelOneFactoryArrivalProductionDrop, LevelOneFactoryArrivalProductionOutcome,
    };
    let (session, mut entities, mut fx) = native_world(25);
    let factory_id = entities
        .iter_all()
        .find(|entity| {
            entity.entity_type == 66 && entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX)
        })
        .unwrap()
        .id;
    let factory = entities.entity_mut(factory_id).unwrap();
    let native = factory.intro2_type66_runtime.take();
    let stale_owner = LevelOneFactoryArrivalOwner::adopt_published(factory).unwrap();
    factory.intro2_type66_runtime = native;
    let before = factory.actor_task_state(ActorTaskSlot::Primary).copied();
    let collision = factory.collision.clone();
    let mut notifications = GameplayNotifications::new();
    let result = tick_level_one_factory_arrival_owner(
        &mut entities,
        stale_owner,
        session.cache.terrain().unwrap(),
        &mut fx,
        &mut notifications,
        5,
        100_000,
        false,
    );
    assert_eq!(
        result.outcome,
        LevelOneFactoryArrivalProductionOutcome::Dropped {
            factory_id,
            reason: LevelOneFactoryArrivalProductionDrop::NativeType66Owner,
        }
    );
    assert!(result.retained_owner.is_none());
    let factory = entities.entity_mut(factory_id).unwrap();
    assert_eq!(
        factory.actor_task_state(ActorTaskSlot::Primary).copied(),
        before
    );
    assert_eq!(factory.collision, collision);
}

fn fresh_level_one() -> (GameSession, EntityManager) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("initialize V2000 retail corpus");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load Level-1 auxiliary overlay");
    let type_models = session.cache.global_entity_model_table();
    let type_metadata = type_models
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
    session
        .load_level_by_id(13, 1)
        .expect("load Level 1 from retail corpus");
    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session
            .cache
            .level_desc()
            .expect("loaded Level-1 descriptor"),
        &type_metadata,
        Some(session.cache.terrain().expect("loaded Level-1 terrain")),
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");
    (session, manager)
}

fn native_world(level_id: u32) -> (GameSession, EntityManager, WorldFx) {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, models)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *models,
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    let mut world_fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            logical_world_index: (level_id - 12) as i32,
            level: session.cache.level_desc().unwrap(),
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 0,
        },
        &mut world_fx,
    )
    .expect("native authored allocation and behavior publication");
    (session, manager, world_fx)
}

#[v2k_test_support::retail_test]
fn native_pair_requests_select_each_actual_factory_and_reject_foreign_allocation_receipts() {
    for (level_id, expected_factories) in [(14, 1), (25, 13), (39, 2)] {
        let (session, mut entities, _) = native_world(level_id);
        let factories = entities
            .iter_all()
            .filter(|entity| entity.entity_type == 66)
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        assert_eq!(factories.len(), expected_factories);
        for &factory_id in &factories {
            let result = resolve_factory_pair_arrivals(
                &entities,
                &session.cache,
                FactoryPairRequest {
                    factory_id,
                    retail_tick: 0,
                },
            )
            .unwrap();
            let FactoryPairPassResult::Resolved(pass) = result else {
                panic!("actual native factory {factory_id} must be selected");
            };
            assert_eq!(pass.factory_id, factory_id);
            assert!(entities
                .iter_all()
                .find(|entity| entity.id == factory_id)
                .unwrap()
                .intro2_type66_runtime
                .is_some());
        }
        assert_eq!(
            resolve_factory_pair_arrivals(
                &entities,
                &session.cache,
                FactoryPairRequest {
                    factory_id: u32::MAX,
                    retail_tick: 0
                }
            )
            .unwrap(),
            FactoryPairPassResult::FactoryAbsent
        );

        let (_, foreign, _) = native_world(level_id);
        let factory_id = factories[0];
        let foreign_receipt = foreign
            .iter_all()
            .find(|entity| entity.id == factory_id)
            .unwrap()
            .intro2_type66_runtime;
        entities
            .entity_mut(factory_id)
            .unwrap()
            .intro2_type66_runtime = foreign_receipt;
        assert!(matches!(
            resolve_factory_pair_arrivals(
                &entities,
                &session.cache,
                FactoryPairRequest {
                    factory_id,
                    retail_tick: 0
                }
            ),
            Err(v2k_game::factory_pair_live::FactoryPairUnresolved::FactoryOwner(_))
        ));
    }
}

#[v2k_test_support::retail_test]
fn native_authored_scientist_contact_commits_only_its_target_factory_intake() {
    use v2k_game::factory_activation_live::{
        apply_prepared_factory_scientist_arrival, prepare_factory_scientist_arrival,
    };
    use v2k_game::intro2_type8::Intro2Type8Outcome;
    use v2k_game::specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    };
    use v2k_game::static_damage::StaticDamageScheduler;
    // World42's sole factory (spawn2) has authored capacity0, so its native
    // Type116 workers correctly have no intake destination. World43 retains
    // the same worker profile and has a real capacity5 factory at spawn22.
    for (level_id, worker_type, worker_count, model_id) in
        [(25, 8, 4, 559), (24, 90, 1, 890), (43, 116, 5, 1136)]
    {
        let (mut session, mut entities, mut world_fx) = native_world(level_id);
        let scientist_id = entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == worker_type && entity.authored_spawn_index.is_some()
            })
            .unwrap()
            .id;
        let initial_target = match entities
            .entity_mut(scientist_id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
        {
            Some(ActorTaskRuntime::GoToJob(state)) => state.target_id(),
            _ => None,
        };
        let factory = entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == 66
                    && initial_target.is_none_or(|target| target == entity.id)
                    && matches!(entity.base_factory_runtime, RetailRuntimeValue::Known(Some(base))
            if base.current_scientists < base.required_scientists)
            })
            .unwrap_or_else(|| panic!("world{level_id} worker{worker_type} target{initial_target:?}: no actual factory with an available worker slot"));
        let desired_factory_id = factory.id;
        let contact_position = factory.position_raw();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type8(&mut entities), worker_count);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut positioned_for_expiry = false;
        let mut selected_factory = None;
        let mut retail_tick = 0;
        for step in 1..=300 {
            let scientist = entities.entity_mut(scientist_id).unwrap();
            // Workers can legitimately precede all factories in the birth prefix
            // and start in Wander. Near its natural strict-five-second expiry,
            // arrange only contact position, retaining the real graph and timer.
            if let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
                scientist.actor_task_state(ActorTaskSlot::Primary)
            {
                if state.elapsed_ms() < 4_800 {
                    positioned_for_expiry = false;
                } else if !positioned_for_expiry {
                    scientist.position = contact_position.map(|word| f32::from(word) / 256.0);
                    positioned_for_expiry = true;
                }
            }
            retail_tick = step * 5;
            let pass = scheduler.tick(
                &mut entities,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut world_fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 100_000,
                    global_elapsed_micros: 100_000,
                    retail_tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "worker scheduler: {:?}", pass.block);
            for outcome in &pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::Intro2Type8(outcome) = outcome {
                    assert!(
                        !matches!(
                            outcome,
                            Intro2Type8Outcome::Blocked { .. }
                                | Intro2Type8Outcome::Dropped { .. }
                                | Intro2Type8Outcome::Pending { .. }
                        ),
                        "{outcome:?}"
                    );
                }
            }
            if let Some(ActorTaskRuntime::GoToJob(state)) = entities
                .entity_mut(scientist_id)
                .unwrap()
                .actor_task_state(ActorTaskSlot::Primary)
            {
                selected_factory = state.target_id();
                if selected_factory.is_some() {
                    break;
                }
            }
        }
        let factory_id =
            selected_factory.expect("the live worker root must select the nearby factory");
        assert_eq!(factory_id, desired_factory_id);
        let factory = entities
            .iter_all()
            .find(|entity| entity.id == factory_id)
            .unwrap();
        let position = factory.position_raw();
        let before = match factory.base_factory_runtime {
            RetailRuntimeValue::Known(Some(base)) => base,
            _ => panic!("native factory Sub-M"),
        };
        let other_staffing = entities
            .iter_all()
            .filter(|entity| entity.entity_type == 66 && entity.id != factory_id)
            .map(|entity| (entity.id, entity.base_factory_runtime))
            .collect::<Vec<_>>();
        entities.entity_mut(scientist_id).unwrap().position =
            position.map(|word| f32::from(word) / 256.0);
        let pass = resolve_factory_pair_arrivals(
            &entities,
            &session.cache,
            FactoryPairRequest {
                factory_id,
                retail_tick,
            },
        )
        .unwrap();
        let FactoryPairPassResult::Resolved(pass) = pass else {
            panic!("native factory present");
        };
        assert!(pass
            .contacts
            .iter()
            .any(|contact| contact.scientist_id == scientist_id));

        let scientist = entities.entity_mut(scientist_id).unwrap();
        assert_eq!(scientist.model_slots, [Some(model_id); 4]);
        let native_runtime = scientist.intro2_type8_runtime;
        let animation = scientist.actor_animation_runtime;
        let heading_before = scientist.heading_raw();
        let sub_a_before = scientist.sub_a_propulsion_runtime;
        let collision_before = scientist.collision.clone();
        let (_, foreign, _) = native_world(level_id);
        scientist.intro2_type8_runtime = foreign
            .iter_all()
            .find(|entity| entity.id == scientist_id)
            .unwrap()
            .intro2_type8_runtime;
        assert!(
            prepare_factory_scientist_arrival(&entities, factory_id, scientist_id).is_err(),
            "copied same-id receipt must not authenticate world{level_id}"
        );
        entities
            .entity_mut(scientist_id)
            .unwrap()
            .intro2_type8_runtime = native_runtime;

        // Native factory custody alone does not own the arriving worker task.
        let mut factory_only = SpecializedActorTaskScheduler::new();
        assert!(factory_only.adopt_intro2_type66(&entities) > 0);
        let effects_before = world_fx.pending_event_count();
        assert!(factory_only
            .apply_factory_pair_arrival(
                &mut entities,
                &mut world_fx,
                &mut notifications,
                factory_id,
                scientist_id,
                retail_tick,
            )
            .is_none());
        assert_eq!(world_fx.pending_event_count(), effects_before);
        assert_eq!(
            entities.entity_mut(scientist_id).unwrap().collision,
            collision_before
        );
        assert_eq!(
            entities
                .entity_mut(factory_id)
                .unwrap()
                .base_factory_runtime,
            RetailRuntimeValue::Known(Some(before))
        );
        assert!(entities.pending_factory_scientist_destroy_ids().is_empty());

        assert!(scheduler.adopt_intro2_type66(&entities) > 0);
        let stale = prepare_factory_scientist_arrival(&entities, factory_id, scientist_id).unwrap();
        let Some(LevelOneFactoryArrivalFrame::Applied {
            delivery: outcome, ..
        }) = scheduler.apply_factory_pair_arrival(
            &mut entities,
            &mut world_fx,
            &mut notifications,
            factory_id,
            scientist_id,
            retail_tick,
        )
        else {
            panic!("native worker{worker_type} intake must commit through actual task custody");
        };
        assert_eq!(outcome.factory_id, factory_id);
        assert_eq!(outcome.scientist_id, scientist_id);
        assert_eq!(
            outcome.factory_runtime.current_scientists_raw,
            before.production.unwrap().current_scientists_raw + 1
        );
        assert_eq!(
            outcome.visit_suffix.physical,
            FactoryPairPhysical::ClearedByA300
        );
        assert!(matches!(outcome.visit_suffix.candidate_components,
        FactoryPairCandidateComponents::Applied { heading_raw_before, heading_raw_after, .. }
        if heading_raw_before == heading_before && heading_raw_after == heading_before.wrapping_add(0x2000)));
        let scientist = entities.entity_mut(scientist_id).unwrap();
        assert_eq!(scientist.intro2_type8_runtime, native_runtime);
        assert_eq!(
            scientist.actor_animation_runtime, animation,
            "actual worker Sub-I is retained"
        );
        assert_eq!(
            scientist.sub_a_propulsion_runtime, sub_a_before,
            "Sub-I's descriptor-contact branch retains the current direction"
        );
        assert!(entities
            .pending_factory_scientist_destroy_ids()
            .contains(&scientist_id));
        assert!(apply_prepared_factory_scientist_arrival(
            &mut entities,
            &mut world_fx,
            &mut notifications,
            retail_tick,
            stale
        )
        .is_err());
        for (id, expected) in other_staffing {
            assert_eq!(
                entities
                    .iter_all()
                    .find(|entity| entity.id == id)
                    .unwrap()
                    .base_factory_runtime,
                expected
            );
        }
        assert!(
            v2k_game::intro2_type66::Intro2Type66Owner::adopt(&entities, factory_id).is_ok(),
            "intake retains the production owner's actual Primary and allocation"
        );
        assert_eq!(
            entities.cleanup_pending_factory_scientist_destroys(),
            [scientist_id]
        );
        assert!(entities.iter_all().all(|entity| entity.id != scientist_id));
    }
}

#[v2k_test_support::retail_test]
fn cleared_native_type95_receipt_cannot_fall_back_to_a_type9_conversion() {
    use v2k_game::main_base_conversion::MainBaseConversionUnresolved;
    use v2k_game::main_base_conversion_live::{
        resolve_first_world_main_base_conversions, FirstWorldMainBaseConversionUnresolved,
        MainBaseConversionFrame,
    };
    use v2k_game::main_base_conversion_runtime::MainBaseReplacementPreflightError;

    let (session, mut entities, mut fx) = native_world(16);
    let base = entities
        .iter_all()
        .find(|entity| entity.entity_type == 6)
        .unwrap();
    let base_id = base.id;
    let base_position = base.position;
    let person_id = entities
        .iter_all()
        .find(|entity| entity.entity_type == 95)
        .unwrap()
        .id;
    let world_style = session.cache.level_desc().unwrap().world_style;
    entities
        .entity_mut(person_id)
        .unwrap()
        .native_type86_runtime
        .take()
        .expect("the authored Type95 must begin with its own native receipt");
    let person_position_raw = entities.entity_mut(person_id).unwrap().position_raw();
    assert_eq!(world_style, 2, "the real output selector chooses Type91");
    assert!(matches!(
        prepare_first_world_main_base_replacement(
            &entities,
            session.cache.terrain(),
            MainBaseReplacementSpawn {
                source_entity_id: person_id,
                main_base_entity_id: base_id,
                replacement_type: 91,
                position_raw: person_position_raw,
            },
            0,
        ),
        Err(
            MainBaseReplacementPreflightError::SourceTypeMismatch { source_id, entity_type: 95 }
        ) if source_id == person_id
    ));
    // Keep all other eligible people far from this specific native Base.
    let people: Vec<_> = entities
        .iter_all()
        .filter(|entity| entity.capability_flags & 0x800 != 0)
        .map(|entity| entity.id)
        .collect();
    for id in people {
        entities.entity_mut(id).unwrap().position =
            [base_position[0] + 64.0, base_position[1], base_position[2]];
    }
    let mut notifications = GameplayNotifications::new();
    let mut actor_tasks = SpecializedActorTaskScheduler::new();
    let distant = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut actor_tasks,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 0,
        world_style: RetailRuntimeValue::Known(world_style),
    });
    assert!(
        distant.is_ok(),
        "an unreached family cannot stop the scan: {distant:?}"
    );
    entities.entity_mut(person_id).unwrap().position = base_position;
    let collision = entities.entity_mut(person_id).unwrap().collision.clone();
    let task = entities
        .entity_mut(person_id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let effects_before = fx.pending_event_count();
    let notifications_before = format!("{notifications:?}");
    let count_before = entities.iter_all().count();
    let failure = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut actor_tasks,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut fx,
        notifications: &mut notifications,
        retail_tick: 0,
        world_style: RetailRuntimeValue::Known(world_style),
    })
    .expect_err("a cleared native receipt cannot authorize actual Type95 contact");
    assert!(
        matches!(failure.unresolved, MainBaseConversionUnresolved::Host {
        candidate_id, source: FirstWorldMainBaseConversionUnresolved::SourceAllocationUnavailable {
            entity_id,
        }, ..
    } if candidate_id == person_id && entity_id == person_id)
    );
    assert!(entities
        .pending_main_base_conversion_destroy_ids()
        .is_empty());
    assert_eq!(entities.iter_all().count(), count_before);
    assert_eq!(entities.entity_mut(person_id).unwrap().collision, collision);
    assert_eq!(
        entities
            .entity_mut(person_id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied(),
        task
    );
    assert_eq!(fx.pending_event_count(), effects_before);
    assert_eq!(format!("{notifications:?}"), notifications_before);
}

fn spawn_factory_bound_scientist(
    session: &GameSession,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    factory_id: u32,
    main_base_id: u32,
    position_raw: [i16; 3],
) -> u32 {
    let sources = entities
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    for source_id in sources {
        let request = MainBaseReplacementSpawn {
            source_entity_id: source_id,
            main_base_entity_id: main_base_id,
            replacement_type: 8,
            position_raw,
        };
        let Ok(plan) = prepare_first_world_main_base_replacement(
            entities,
            Some(session.cache.terrain().expect("Level-1 terrain")),
            request,
            0,
        ) else {
            continue;
        };
        assert_eq!(
            entities.queue_main_base_conversion_destroy(source_id),
            MainBaseConversionDestroyQueueOutcome::Queued { source_id }
        );
        let spawned = apply_prepared_first_world_main_base_replacement(
            entities,
            Some(session.cache.terrain().expect("Level-1 terrain")),
            world_fx,
            plan,
        );
        let replacement = entities
            .iter_all()
            .find(|entity| entity.id == spawned.replacement_id)
            .expect("published Main Base replacement");
        if matches!(
            replacement.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(state)) if state.target_id() == Some(factory_id)
        ) {
            return spawned.replacement_id;
        }
    }
    panic!("retail Level 1 supplies a factory-bound scientist replacement");
}

#[v2k_test_support::retail_test]
fn factory_pair_walker_requires_oriented_contact_not_distance() {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory")
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let mut world_fx = WorldFx::new();
    let scientist_id = spawn_factory_bound_scientist(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
        FACTORY_POSITION_RAW,
    );
    match resolve_factory_pair_arrivals(
        &entities,
        &session.cache,
        FactoryPairRequest {
            factory_id,
            retail_tick: 0,
        },
    ) {
        Ok(FactoryPairPassResult::Resolved(pass)) => {
            assert_eq!(pass.factory_id, factory_id);
            assert!(
                pass.contacts
                    .iter()
                    .any(|contact| contact.scientist_id == scientist_id),
                "an overlapping factory-bound scientist must classify as contact"
            );
            assert!(
                pass.contacts.iter().all(|contact| {
                    entities
                        .iter_all()
                        .find(|entity| entity.id == contact.scientist_id)
                        .is_some_and(|entity| entity.entity_type == 8)
                }),
                "pair intake must not treat nearby peasants or the factory as arrived"
            );
        }
        other => panic!("expected contact at the factory, got {other:?}"),
    }
}

#[v2k_test_support::retail_test]
fn factory_pair_contact_applies_scheduler_owned_arrival() {
    let (session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory")
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let scientist_id = spawn_factory_bound_scientist(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
        FACTORY_POSITION_RAW,
    );
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_level_one_factory_arrival(&entities), 1);

    let pass = match resolve_factory_pair_arrivals(
        &entities,
        &session.cache,
        FactoryPairRequest {
            factory_id,
            retail_tick: 0,
        },
    ) {
        Ok(FactoryPairPassResult::Resolved(pass)) => pass,
        other => panic!("expected factory contact, got {other:?}"),
    };
    assert!(pass
        .contacts
        .iter()
        .any(|contact| contact.scientist_id == scientist_id));
    scheduler
        .queue_explicit_level_one_factory_arrival(factory_id, scientist_id)
        .expect("factory owner is in custody");
    let scientist_before = entities
        .iter_all()
        .find(|entity| entity.id == scientist_id)
        .expect("live scientist");
    assert_eq!(
        scientist_before.collision.pair_callbacks.component_contact,
        RetailRuntimeValue::Known([
            PairComponentContactPolicy::DescriptorContact,
            PairComponentContactPolicy::None,
            PairComponentContactPolicy::None,
        ])
    );
    let heading_before = scientist_before.heading_raw();
    let velocity_before = scientist_before.velocity_raw();
    match scheduler.apply_pending_level_one_factory_arrival(
        &mut entities,
        &mut world_fx,
        &mut notifications,
        0,
    ) {
        Some(LevelOneFactoryArrivalFrame::Applied {
            scientist_id: applied,
            machine_pair_suffix,
            visit_suffix,
            ..
        }) => {
            assert_eq!(applied, scientist_id);
            assert_eq!(machine_pair_suffix, FactoryDeliveryPairSuffix::Unclaimed);
            assert_eq!(
                visit_suffix.candidate_behavior,
                FactoryPairCandidateBehavior::SkippedNull
            );
            assert_eq!(
                visit_suffix.subject_components,
                FactoryPairSubjectComponents::None
            );
            assert_eq!(visit_suffix.physical, FactoryPairPhysical::ClearedByA300);
            match visit_suffix.candidate_components {
                FactoryPairCandidateComponents::Applied {
                    heading_raw_before,
                    heading_raw_after,
                    ..
                } => {
                    assert_eq!(heading_raw_before, heading_before);
                    assert_eq!(heading_raw_after, heading_before.wrapping_add(0x2000));
                }
                FactoryPairCandidateComponents::Miss { .. } => {}
            }
        }
        other => panic!("expected scheduler-owned pair apply, got {other:?}"),
    }
    let scientist_after = entities
        .iter_all()
        .find(|entity| entity.id == scientist_id)
        .expect("queued scientist remains until deferred splice");
    assert_eq!(scientist_after.velocity_raw(), velocity_before);
    let factory = entities
        .iter_all()
        .find(|entity| entity.id == factory_id)
        .expect("factory remains live");
    let RetailRuntimeValue::Known(Some(state)) = factory.base_factory_runtime else {
        panic!("factory runtime");
    };
    let production = state.production.expect("production");
    assert_eq!(production.current_scientists_raw, 1);
    assert_eq!(production.phase, FactoryProductionPhase::Producing);
}
