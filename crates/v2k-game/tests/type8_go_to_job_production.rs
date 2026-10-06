use std::path::PathBuf;

use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;

use v2k_game::common_mover::type9_attitude::Type9BodyBasis;
use v2k_game::entity::{EntityManager, MainBaseConversionDestroyQueueOutcome};
use v2k_game::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT,
};
use v2k_game::main_base_conversion::MainBaseReplacementSpawn;
use v2k_game::main_base_conversion_runtime::{
    apply_prepared_first_world_main_base_replacement, prepare_first_world_main_base_replacement,
};
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler;
use v2k_game::type8_go_to_job_production::{
    tick_type8_go_to_job_scheduler_owner, Type8GoToJobMoverBlock, Type8GoToJobSchedulerOwner,
    Type8GoToJobSchedulerProductionOutcome,
};

use v2k_game::world_fx::WorldFx;

#[path = "type8_go_to_job_production/root_transitions.rs"]
// These tests exercise the separately supported direct task adapter's exact
// unwind/retry transactions. The live movement test below uses native custody.
mod type8_root_transitions;

const FACTORY_SPAWN_INDEX: usize = 23;
const MAIN_BASE_SPAWN_INDEX: usize = 6;
const FACTORY_POSITION_RAW: [i16; 3] = [0x5700, -0x0300, 0x3A00];

fn type_metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
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

fn data_dir() -> PathBuf {
    let dir = v2k_test_support::retail_dir();
    assert!(
        dir.join("PRELOAD.DAT").exists(),
        "Type8 production tests require PRELOAD.DAT in the selected retail directory"
    );
    dir
}

fn fresh_level_one() -> (GameSession, EntityManager) {
    let dir = data_dir();
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

fn spawn_factory_bound_scientist_at(
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

fn adopt_native_workers(
    entities: &mut EntityManager,
    selected_id: u32,
) -> SpecializedActorTaskScheduler {
    use v2k_game::entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    };
    let ids = entities
        .iter_all()
        .filter(|entity| entity.intro2_type8_runtime.is_some())
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    for id in &ids {
        let entity = entities.entity_mut(*id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                | if *id == selected_id {
                    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                } else {
                    0
                },
        );
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    }
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(
        scheduler.adopt_live_type8_go_to_job(entities),
        0,
        "native births cannot also enter legacy custody"
    );
    assert_eq!(scheduler.adopt_intro2_type8(entities), ids.len());
    assert_eq!(
        scheduler.adopt_intro2_type8(entities),
        0,
        "birth receipt is consumed once"
    );
    scheduler
}

fn native_step(
    session: &mut GameSession,
    entities: &mut EntityManager,
    scheduler: &mut SpecializedActorTaskScheduler,
    fx: &mut WorldFx,
    scientist_id: u32,
    elapsed_micros: u32,
) -> v2k_game::intro2_type8::Intro2Type8Outcome {
    use v2k_game::specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    };
    let pass = scheduler.tick(
        entities,
        SpecializedActorTaskProductionFrame {
            world:
                v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut v2k_game::static_damage::StaticDamageScheduler::new(),
            elapsed_micros,
            global_elapsed_micros: elapsed_micros,
            retail_tick: 1000,
            main_base_abort_active: false,
        },
        &mut v2k_game::gameplay_notifications::GameplayNotifications::new(),
    );
    assert!(pass.block.is_none(), "{:?}", pass.block);
    pass.outcomes
        .into_iter()
        .find_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::Intro2Type8(outcome)
                if outcome.entity_id() == scientist_id =>
            {
                Some(outcome)
            }
            _ => None,
        })
        .expect("selected native worker has one scheduler result")
}

#[v2k_test_support::retail_test]
fn cargo_converted_type8_visits_go_to_job_through_native_shared_abdi() {
    use v2k_game::intro2_type8::Intro2Type8Outcome;
    let (mut session, mut entities) = fresh_level_one();
    let factory_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(FACTORY_SPAWN_INDEX))
        .unwrap()
        .id;
    let main_base_id = entities
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(MAIN_BASE_SPAWN_INDEX))
        .unwrap()
        .id;
    let mut fx = WorldFx::new();
    let scientist_id = spawn_factory_bound_scientist_at(
        &session,
        &mut entities,
        &mut fx,
        factory_id,
        main_base_id,
        FACTORY_POSITION_RAW,
    );
    let birth_count = entities
        .iter_all()
        .filter(|entity| entity.intro2_type8_runtime.is_some())
        .count();
    let scientist = entities.entity_mut(scientist_id).unwrap();
    let position_before = scientist.position_raw();
    assert_eq!(
        scientist
            .type8_sub_d_frame_owner
            .unwrap()
            .classifier_cache()
            .stagger_counter(),
        fx.next_sub_d_allocation_seed().wrapping_sub(1),
        "the last conversion owns its actual process Sub-D allocation"
    );
    assert!(scientist.type8_sub_d_runtime.is_some());
    assert!(scientist.intro2_type8_runtime.is_some());
    assert!(matches!(
        scientist.physical_body_basis_q31(),
        RetailRuntimeValue::Known(_)
    ));
    let mut expected = WorldFx::new();
    for _ in 0..birth_count * 3 {
        expected.next_shared_retail_random_u16();
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16(),
        "each dynamic birth consumes20450, AC60, then06070; Sub-D allocation consumes no RNG"
    );
    let mut scheduler = adopt_native_workers(&mut entities, scientist_id);
    let mut moved = false;
    for _ in 0..41 {
        let outcome = native_step(
            &mut session,
            &mut entities,
            &mut scheduler,
            &mut fx,
            scientist_id,
            20_000,
        );
        assert!(
            matches!(outcome, Intro2Type8Outcome::Advanced { .. }),
            "{outcome:?}"
        );
        let scientist = entities.entity_mut(scientist_id).unwrap();
        let [heading, pitch, roll] = scientist.rotation_heading_pitch_roll_raw();
        assert_eq!(
            scientist.physical_body_basis_q31(),
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll)),
            "E870 rebuilds13F70 from the live Euler words"
        );
        assert_eq!(
            scientist
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        if scientist.position_raw() != position_before {
            moved = true;
            break;
        }
    }
    assert!(
        moved,
        "native12DA0 must integrate the worker's shared ABDI movement"
    );
}

#[v2k_test_support::retail_test]
fn authored_level_one_does_not_invent_dynamic_worker_custody() {
    let (_session, mut entities) = fresh_level_one();
    assert!(entities.iter_all().all(|entity| entity.entity_type != 8));
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_live_type8_go_to_job(&entities), 0);
    assert_eq!(scheduler.adopt_intro2_type8(&mut entities), 0);
}
