//! Normal-tier Main Base conversion through the real oriented contact walker.

use v2k_game::entity::{EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::main_base_conversion::{
    MainBaseConversionActionOutcome, MainBaseConversionUnresolved,
};
use v2k_game::main_base_conversion_live::{
    resolve_first_world_main_base_conversions, FirstWorldMainBaseConversionPass,
    FirstWorldMainBaseConversionUnresolved, MainBaseConversionFrame,
};
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler,
};
use v2k_game::world_fx::WorldFx;

fn fresh_world() -> (
    GameSession,
    EntityManager,
    WorldFx,
    GameplayNotifications,
    SpecializedActorTaskScheduler,
) {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").exists(),
        "canonical retail corpus required"
    );
    let mut session = GameSession::init(&data).expect("load PRELOAD");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal-tier resources");
    session.load_level_by_id(13, 1).expect("first world");
    let metadata: Vec<_> = session
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
                    ..Default::default()
                })
        })
        .collect();
    let mut effects = WorldFx::new();
    let mut entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
        0,
        &mut effects,
    )
    .expect("fresh first-world publication");
    let mut notifications = GameplayNotifications::new();
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
    assert_eq!(
        scheduler
            .adopt_fresh_level1_type9_selected(&mut entities)
            .unwrap(),
        6
    );
    // Establish the first scheduler flags and live body basis through the
    // production mover before exercising the ordinary active-pair phase.
    for retail_tick in 0..3 {
        scheduler.tick(
            &mut entities,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut effects,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
    }
    (session, entities, effects, notifications, scheduler)
}

#[v2k_test_support::retail_test]
fn distant_unknown_callbacks_do_not_block_a_later_peasant_conversion() {
    let (session, mut entities, mut effects, mut notifications, mut scheduler) = fresh_world();
    let base = entities
        .iter_all()
        .find(|e| e.entity_type == 6 && e.authored_spawn_index == Some(6))
        .unwrap();
    let base_id = base.id;
    let contact_position = base.position;
    let peasants: Vec<_> = entities
        .iter_all()
        .filter(|e| e.entity_type == 9)
        .map(|e| e.id)
        .collect();
    let distant_id = peasants[0];
    let contact_id = peasants[1];
    let distant_position = entities.entity_mut(distant_id).unwrap().position;
    assert!((distant_position[0] - contact_position[0]).abs() > 100.0);
    let distant = entities.entity_mut(distant_id).unwrap();
    distant.current_behavior_context = RetailRuntimeValue::Unresolved;
    distant.collision.pair_callbacks.component_contact = RetailRuntimeValue::Unresolved;
    // This is a contact fixture, not a movement shortcut in production. Keep
    // the authored models and the scheduler-established orientation/state.
    entities.entity_mut(contact_id).unwrap().position = contact_position;

    let result = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut scheduler,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut effects,
        notifications: &mut notifications,
        retail_tick: 3,
        world_style: RetailRuntimeValue::Known(session.cache.level_desc().unwrap().world_style),
    })
    .expect("distant callbacks are not visited by retail's pair dispatcher");
    let FirstWorldMainBaseConversionPass::Resolved(pass) = result else {
        panic!("Main Base must exist")
    };
    let distant_visit = pass
        .visits
        .iter()
        .find(|v| v.entity_id == distant_id)
        .unwrap();
    assert!(distant_visit.action_outcomes.is_empty());
    assert!(distant_visit.suffix_outcome.is_none());
    let visit = pass
        .visits
        .iter()
        .find(|v| v.entity_id == contact_id)
        .unwrap();
    let [MainBaseConversionActionOutcome::Applied, MainBaseConversionActionOutcome::Applied, MainBaseConversionActionOutcome::ReplacementSpawned {
        replacement_entity_id,
    }] = visit.action_outcomes.as_slice()
    else {
        panic!("one ordered conversion required: {visit:?}")
    };
    let scientist = entities
        .iter_all()
        .find(|e| e.id == *replacement_entity_id)
        .unwrap();
    assert_eq!(scientist.entity_type, 8);
    assert_eq!(
        scientist.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(base_id))
    );
    assert_eq!(scientist.attached_to, None, "the scientist is free to walk");
    assert_eq!(
        entities.pending_main_base_conversion_destroy_ids(),
        &[contact_id]
    );
    assert!(visit.suffix_outcome.is_some());
    assert!(
        pass.visits
            .iter()
            .any(|v| v.entity_id == *replacement_entity_id),
        "saved-next traversal must visit the tail append"
    );
}

#[v2k_test_support::retail_test]
fn contacting_unknown_behavior_still_stops_before_conversion() {
    let (session, mut entities, mut effects, mut notifications, mut scheduler) = fresh_world();
    let contact_position = entities
        .iter_all()
        .find(|e| e.entity_type == 6 && e.authored_spawn_index == Some(6))
        .unwrap()
        .position;
    let peasant_id = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
    let peasant = entities.entity_mut(peasant_id).unwrap();
    peasant.position = contact_position;
    peasant.current_behavior_context = RetailRuntimeValue::Unresolved;
    let scientist_count = entities.iter_all().filter(|e| e.entity_type == 8).count();
    let failure = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut scheduler,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut effects,
        notifications: &mut notifications,
        retail_tick: 3,
        world_style: RetailRuntimeValue::Known(session.cache.level_desc().unwrap().world_style),
    })
    .expect_err("unknown reached callbacks cannot be skipped");
    assert!(matches!(failure.unresolved,
        MainBaseConversionUnresolved::Host {
            candidate_id,
            source: FirstWorldMainBaseConversionUnresolved::CandidateBehaviorUnavailable { entity_id }, ..
        } if candidate_id == peasant_id && entity_id == peasant_id
    ));
    assert!(entities
        .pending_main_base_conversion_destroy_ids()
        .is_empty());
    assert_eq!(
        entities.iter_all().filter(|e| e.entity_type == 8).count(),
        scientist_count
    );
}

#[v2k_test_support::retail_test]
fn unsupported_world_style_rejects_without_falling_back_to_a_first_world_scientist() {
    use v2k_game::main_base_conversion::MainBaseConversionPhase;
    use v2k_game::main_base_conversion_runtime::MainBaseReplacementPreflightError;

    let (world_style, replacement_type) = (0, 0);
    let (session, mut entities, mut effects, mut notifications, mut scheduler) = fresh_world();
    let contact_position = entities
        .iter_all()
        .find(|e| e.entity_type == 6)
        .unwrap()
        .position;
    let peasant_id = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
    entities.entity_mut(peasant_id).unwrap().position = contact_position;
    let entities_before: Vec<_> = entities
        .iter_all()
        .map(|e| (e.id, e.entity_type, e.position, e.collision.health_raw))
        .collect();
    let effects_before = format!("{effects:?}");
    let notifications_before = format!("{notifications:?}");
    // Hold the actual first-world pair geometry constant while changing
    // only the explicit Section-13 input read by 42EB70.
    let failure = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut scheduler,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut effects,
        notifications: &mut notifications,
        retail_tick: 3,
        world_style: RetailRuntimeValue::Known(world_style),
    })
    .expect_err("a distinct output constructor must own this path");
    assert!(
        matches!(failure.unresolved, MainBaseConversionUnresolved::Host {
            candidate_id,
            phase: MainBaseConversionPhase::ReplacementPreflight,
            source: FirstWorldMainBaseConversionUnresolved::Replacement(
                MainBaseReplacementPreflightError::UnsupportedReplacementType { replacement_type: actual }
            ),
        } if candidate_id == peasant_id && actual == replacement_type),
        "style {world_style}: {failure:?}"
    );
    assert!(entities
        .pending_main_base_conversion_destroy_ids()
        .is_empty());
    assert_eq!(
        entities
            .iter_all()
            .map(|e| (e.id, e.entity_type, e.position, e.collision.health_raw))
            .collect::<Vec<_>>(),
        entities_before
    );
    assert_eq!(format!("{effects:?}"), effects_before);
    assert_eq!(format!("{notifications:?}"), notifications_before);
}

#[v2k_test_support::retail_test]
fn explicit_diver_world_style_converts_one_peasant_to_a_native_four_choice_worker() {
    let (session, mut entities, mut effects, mut notifications, mut scheduler) = fresh_world();
    let base = entities.iter_all().find(|e| e.entity_type == 6).unwrap();
    let base_id = base.id;
    let contact_position = base.position;
    let peasant_id = entities.iter_all().find(|e| e.entity_type == 9).unwrap().id;
    entities.entity_mut(peasant_id).unwrap().position = contact_position;
    let before_ids = entities.retail_live_order_ids().collect::<Vec<_>>();
    let scientists_before = entities.iter_all().filter(|e| e.entity_type == 8).count();
    assert!(!entities.iter_all().any(|e| e.entity_type == 7));

    // Keep the real first-world source/Base contact and change only the
    // explicit Section13 selector. Style6 now owns the complete Type7 birth.
    let result = resolve_first_world_main_base_conversions(MainBaseConversionFrame {
        entities: &mut entities,
        actor_tasks: &mut scheduler,
        model_pool: &session.cache,
        terrain: session.cache.terrain(),
        world_fx: &mut effects,
        notifications: &mut notifications,
        retail_tick: 3,
        world_style: RetailRuntimeValue::Known(6),
    })
    .expect("style6 has its native diver constructor");
    let FirstWorldMainBaseConversionPass::Resolved(pass) = result else {
        panic!("Main Base must exist")
    };
    let visit = pass
        .visits
        .iter()
        .find(|v| v.entity_id == peasant_id)
        .unwrap();
    let [MainBaseConversionActionOutcome::Applied, MainBaseConversionActionOutcome::Applied, MainBaseConversionActionOutcome::ReplacementSpawned {
        replacement_entity_id,
    }] = visit.action_outcomes.as_slice()
    else {
        panic!("one ordered conversion required: {visit:?}")
    };
    let replacement_id = *replacement_entity_id;
    assert_eq!(
        pass.visits
            .iter()
            .flat_map(|v| &v.action_outcomes)
            .filter(|action| matches!(
                action,
                MainBaseConversionActionOutcome::ReplacementSpawned { .. }
            ))
            .count(),
        1
    );
    let diver = entities
        .iter_all()
        .find(|e| e.id == replacement_id)
        .unwrap();
    assert_eq!(diver.entity_type, 7);
    assert_eq!(diver.model_slots, [Some(1249); 4]);
    assert_eq!(diver.capability_flags, 0x1404);
    assert_eq!(diver.authored_spawn_index, None);
    assert!(diver.native_type86_runtime.is_some());
    assert!(diver.intro2_type8_runtime.is_none());
    assert_eq!(diver.attached_to, None);
    assert_eq!(
        diver.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(base_id))
    );
    assert_eq!(
        entities.iter_all().filter(|e| e.entity_type == 8).count(),
        scientists_before
    );
    assert_eq!(
        entities
            .retail_live_order_ids()
            .filter(|id| !before_ids.contains(id))
            .collect::<Vec<_>>(),
        [replacement_id]
    );
    assert!(visit.suffix_outcome.is_some());
    assert!(pass.visits.iter().any(|v| v.entity_id == replacement_id));
    assert_eq!(
        entities.pending_main_base_conversion_destroy_ids(),
        [peasant_id]
    );
    assert_ne!(notifications.save_tail_seen_mask() & (1 << 1), 0);
    assert!(
        entities
            .pending_fresh_level1_type9_resource_text_receipts()
            .is_empty(),
        "any BA40 receipt must be drained by the conversion host"
    );
    // Public adoption authenticates the actual manager allocation and consumes
    // the constructor graph once; presence of a receipt alone is insufficient.
    assert_eq!(scheduler.adopt_native_type86(&mut entities), 1);
    assert_eq!(scheduler.adopt_native_type86(&mut entities), 0);
    assert_eq!(
        entities.cleanup_pending_main_base_conversion_destroys(),
        [peasant_id]
    );
    assert!(!entities.iter_all().any(|e| e.id == peasant_id));
    assert!(entities.iter_all().any(|e| e.id == replacement_id));
}
