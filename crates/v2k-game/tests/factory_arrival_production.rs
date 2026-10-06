use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::entity::{EntityManager, MainBaseConversionDestroyQueueOutcome};
use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::factory_arrival_production::{
    level_one_factory_under_attack_latched, stamp_level_one_factory_last_hit_tick,
    tick_level_one_factory_arrival_owner, LevelOneFactoryArrivalFrame, LevelOneFactoryArrivalOwner,
    LevelOneFactoryArrivalPairClaim, LevelOneFactoryArrivalProductionOutcome,
    LevelOneFactoryTaskVisit,
};
use v2k_game::factory_delivery::FactoryDeliveryPairSuffix;
use v2k_game::factory_pair_suffix::{FactoryPairCandidateBehavior, FactoryPairPhysical};
use v2k_game::factory_production::FactoryProductionPhase;
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::main_base_conversion::MainBaseReplacementSpawn;
use v2k_game::main_base_conversion_runtime::{
    apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
};
use v2k_game::main_base_type66_abort::WorkingFactoryNotificationOutcome;
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler;
use v2k_game::type17_impact_reselection::UNDER_ATTACK_WINDOW_TICKS;
use v2k_game::world_fx::WorldFx;

const FACTORY_SPAWN_INDEX: usize = 23;
const MAIN_BASE_SPAWN_INDEX: usize = 6;
const FACTORY_POSITION_RAW: [i16; 3] = [0x5700, -0x0300, 0x3A00];
const FRAME_MICROS: u32 = 20_000;

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

fn spawn_factory_bound_scientist(
    session: &GameSession,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    factory_id: u32,
    main_base_id: u32,
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
            position_raw: FACTORY_POSITION_RAW,
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
fn factory_arrival_idles_until_an_explicit_scientist_is_queued() {
    let (session, mut entities) = fresh_level_one();
    let factory = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory");
    let factory_id = factory.id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let owner = LevelOneFactoryArrivalOwner::adopt_published(factory).expect("spawn-23 factory");
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let idle = tick_level_one_factory_arrival_owner(
        &mut entities,
        owner,
        terrain,
        &mut world_fx,
        &mut notifications,
        0,
        FRAME_MICROS,
        false,
    );
    match idle.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            factory_id: id,
            task_visit,
            production,
            arrival,
        } => {
            assert_eq!(
                task_visit,
                LevelOneFactoryTaskVisit::WorkingFactory {
                    notification: WorkingFactoryNotificationOutcome::NoAttackAndLatchCleared,
                }
            );
            assert_eq!(id, factory_id);
            assert!(matches!(arrival, LevelOneFactoryArrivalFrame::None));
            let production = production.expect("0/2 factory takes the status tail");
            assert_eq!(
                production.production.phase,
                FactoryProductionPhase::Producing
            );
            assert_eq!(production.production.current_scientists_raw, 0);
            assert_eq!(production.production.production_progress_micros_raw, 0);
        }
        other => panic!("expected production tick without arrival, got {other:?}"),
    }

    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_level_one_factory_arrival(&entities), 1);
    assert_eq!(scheduler.adopt_level_one_factory_arrival(&entities), 0);

    let scientist_id = spawn_factory_bound_scientist(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
    );
    scheduler
        .queue_explicit_level_one_factory_arrival(factory_id, scientist_id)
        .expect("factory owner is in custody");

    let mut owner = idle.retained_owner.expect("idle retains the factory");
    owner.queue_explicit_scientist(scientist_id);
    let applied = tick_level_one_factory_arrival_owner(
        &mut entities,
        owner,
        terrain,
        &mut world_fx,
        &mut notifications,
        0,
        FRAME_MICROS,
        false,
    );
    match applied.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            arrival:
                LevelOneFactoryArrivalFrame::Applied {
                    scientist_id: applied_id,
                    pair_claim,
                    machine_pair_suffix,
                    visit_suffix,
                    ..
                },
            production,
            ..
        } => {
            assert_eq!(applied_id, scientist_id);
            assert_eq!(pair_claim, LevelOneFactoryArrivalPairClaim::SchedulerOwned);
            assert_eq!(machine_pair_suffix, FactoryDeliveryPairSuffix::Unclaimed);
            assert_eq!(
                visit_suffix.candidate_behavior,
                FactoryPairCandidateBehavior::SkippedNull
            );
            assert_eq!(visit_suffix.physical, FactoryPairPhysical::ClearedByA300);
            let production = production.expect("understaffed owner frame");
            assert_eq!(production.production.current_scientists_raw, 0);
            assert_eq!(production.production.production_progress_micros_raw, 0);
        }
        other => panic!("expected explicit arrival after the production tick, got {other:?}"),
    }
}

#[v2k_test_support::retail_test]
fn staffed_factory_advances_phase0_on_the_scheduler_owner() {
    let (session, mut entities) = fresh_level_one();
    let factory = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory");
    let factory_id = factory.id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .expect("authored Level-1 Main Base")
        .id;
    let mut owner =
        LevelOneFactoryArrivalOwner::adopt_published(factory).expect("spawn-23 factory");
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let terrain = session.cache.terrain().expect("Level-1 terrain");

    let first = spawn_factory_bound_scientist(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
    );
    owner.queue_explicit_scientist(first);
    let after_first = tick_level_one_factory_arrival_owner(
        &mut entities,
        owner,
        terrain,
        &mut world_fx,
        &mut notifications,
        1,
        FRAME_MICROS,
        false,
    );
    match after_first.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            arrival: LevelOneFactoryArrivalFrame::Applied { .. },
            production,
            ..
        } => {
            let production = production.expect("first arrival keeps the status tail");
            assert_eq!(production.production.current_scientists_raw, 0);
            assert_eq!(production.production.production_progress_micros_raw, 0);
        }
        other => panic!("expected first explicit arrival, got {other:?}"),
    }

    let second = spawn_factory_bound_scientist(
        &session,
        &mut entities,
        &mut world_fx,
        factory_id,
        main_base_id,
    );
    let mut owner = after_first
        .retained_owner
        .expect("factory stays in custody");
    owner.queue_explicit_scientist(second);
    let after_second = tick_level_one_factory_arrival_owner(
        &mut entities,
        owner,
        terrain,
        &mut world_fx,
        &mut notifications,
        2,
        FRAME_MICROS,
        false,
    );
    match after_second.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            arrival: LevelOneFactoryArrivalFrame::Applied { .. },
            production,
            ..
        } => {
            let production = production.expect("second arrival still follows the task tick");
            assert_eq!(production.production.current_scientists_raw, 1);
            assert_eq!(production.production.production_progress_micros_raw, 0);
        }
        other => panic!("expected second explicit arrival, got {other:?}"),
    }

    let staffed = tick_level_one_factory_arrival_owner(
        &mut entities,
        after_second
            .retained_owner
            .expect("factory stays in custody"),
        terrain,
        &mut world_fx,
        &mut notifications,
        3,
        FRAME_MICROS,
        false,
    );
    match staffed.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            arrival: LevelOneFactoryArrivalFrame::None,
            production,
            ..
        } => {
            let production = production.expect("2/2 starts phase 0");
            assert_eq!(production.production.current_scientists_raw, 2);
            assert_eq!(
                production.production.production_progress_micros_raw,
                FRAME_MICROS as i32
            );
            assert_eq!(
                production.production.phase,
                FactoryProductionPhase::Producing
            );
        }
        other => panic!("expected staffed production tick, got {other:?}"),
    }
}

#[v2k_test_support::retail_test]
fn live_owner_emits_factory_under_attack_once_then_rearms() {
    let (session, mut entities) = fresh_level_one();
    let factory = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory");
    let factory_id = factory.id;
    let owner = LevelOneFactoryArrivalOwner::adopt_published(factory).expect("spawn-23 factory");
    assert!(stamp_level_one_factory_last_hit_tick(
        &mut entities,
        factory_id,
        UNDER_ATTACK_WINDOW_TICKS,
    ));
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let first = tick_level_one_factory_arrival_owner(
        &mut entities,
        owner,
        terrain,
        &mut world_fx,
        &mut notifications,
        UNDER_ATTACK_WINDOW_TICKS,
        FRAME_MICROS,
        false,
    );
    match first.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            task_visit:
                LevelOneFactoryTaskVisit::WorkingFactory {
                    notification: WorkingFactoryNotificationOutcome::NotificationRequested,
                },
            ..
        } => {}
        other => panic!("expected 0xD3 under-attack visit, got {other:?}"),
    }
    assert_eq!(
        level_one_factory_under_attack_latched(&entities, factory_id),
        Some(true)
    );

    let repeat = tick_level_one_factory_arrival_owner(
        &mut entities,
        first.retained_owner.expect("factory stays in custody"),
        terrain,
        &mut world_fx,
        &mut notifications,
        UNDER_ATTACK_WINDOW_TICKS,
        FRAME_MICROS,
        false,
    );
    match repeat.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            task_visit:
                LevelOneFactoryTaskVisit::WorkingFactory {
                    notification: WorkingFactoryNotificationOutcome::AlreadyLatched,
                },
            ..
        } => {}
        other => panic!("expected latch to suppress a second 0xD3, got {other:?}"),
    }

    assert!(stamp_level_one_factory_last_hit_tick(
        &mut entities,
        factory_id,
        0,
    ));
    let cleared = tick_level_one_factory_arrival_owner(
        &mut entities,
        repeat.retained_owner.expect("factory stays in custody"),
        terrain,
        &mut world_fx,
        &mut notifications,
        UNDER_ATTACK_WINDOW_TICKS * 2,
        FRAME_MICROS,
        false,
    );
    match cleared.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            task_visit:
                LevelOneFactoryTaskVisit::WorkingFactory {
                    notification: WorkingFactoryNotificationOutcome::NoAttackAndLatchCleared,
                },
            ..
        } => {}
        other => panic!("expected latch clear after the window, got {other:?}"),
    }
    assert_eq!(
        level_one_factory_under_attack_latched(&entities, factory_id),
        Some(false)
    );
}

#[v2k_test_support::retail_test]
fn live_owner_suppresses_under_attack_text_during_main_base_abort() {
    let (session, mut entities) = fresh_level_one();
    let factory = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .expect("authored Level-1 Working Factory");
    let factory_id = factory.id;
    let owner = LevelOneFactoryArrivalOwner::adopt_published(factory).expect("spawn-23 factory");
    assert!(stamp_level_one_factory_last_hit_tick(
        &mut entities,
        factory_id,
        UNDER_ATTACK_WINDOW_TICKS,
    ));
    let terrain = session.cache.terrain().expect("Level-1 terrain");
    let mut world_fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let aborted = tick_level_one_factory_arrival_owner(
        &mut entities,
        owner,
        terrain,
        &mut world_fx,
        &mut notifications,
        UNDER_ATTACK_WINDOW_TICKS,
        FRAME_MICROS,
        true,
    );
    match aborted.outcome {
        LevelOneFactoryArrivalProductionOutcome::Ticked {
            task_visit:
                LevelOneFactoryTaskVisit::WorkingFactory {
                    notification: WorkingFactoryNotificationOutcome::SuppressedByMainBaseAbort,
                },
            ..
        } => {}
        other => panic!("expected abort to skip 0xD3, got {other:?}"),
    }
    assert_eq!(
        level_one_factory_under_attack_latched(&entities, factory_id),
        Some(false)
    );
}
