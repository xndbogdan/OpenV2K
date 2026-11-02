use super::*;
use crate::{
    entity::{AuthoredWorldConstruction, EntityConstructionResources},
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
};

fn world(level_id: u32) -> (GameSession, EntityManager, WorldFx) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(level_id, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: level_id as i32 - 12,
            type_metadata: &metadata,
            player_arrival: None,
            retail_tick: 1000,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
            },
        },
        &mut fx,
    )
    .unwrap();
    (session, manager, fx)
}

fn type41s(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|e| e.entity_type == TYPE41_ENTITY_TYPE)
        .map(|e| e.id)
        .collect()
}

fn class(manager: &EntityManager, id: u32) -> u8 {
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("context")
    };
    match context.active_style() {
        ActiveBehaviorStyle::Audited(style) => style.class_id,
        other => panic!("{other:?}"),
    }
}

fn pass(
    session: &mut GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    scheduler: &mut SpecializedActorTaskScheduler,
    tick: u32,
) -> Vec<Intro2FlyerSchedulerProductionOutcome> {
    let output = scheduler.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage: &mut StaticDamageScheduler::new(),
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        &mut GameplayNotifications::new(),
    );
    assert!(output.block.is_none(), "{output:?}");
    output
        .outcomes
        .into_iter()
        .filter_map(|outcome| match outcome {
            SpecializedActorTaskProductionOutcome::Intro2FlyerScheduler(outcome) => Some(outcome),
            _ => None,
        })
        .collect()
}

fn assert_progress(outcomes: &[Intro2FlyerSchedulerProductionOutcome]) {
    for outcome in outcomes {
        assert!(
            matches!(
                outcome,
                Intro2FlyerSchedulerProductionOutcome::Type41Visit { .. }
                    | Intro2FlyerSchedulerProductionOutcome::SchedulerWaiting { .. }
                    | Intro2FlyerSchedulerProductionOutcome::CallbackDisabled { .. }
            ),
            "{outcome:?}"
        );
    }
}

#[v2k_test_support::retail_test]
fn type41s_are_born_on_the_flyer_owner_and_fly_in_every_world() {
    for (level, count) in [(22, 1), (33, 2), (34, 4)] {
        let (mut session, mut manager, mut fx) = world(level);
        let ids = type41s(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            let entity = manager.iter_all().find(|e| e.id == id).unwrap();
            assert!(flyer_manager_identity_authenticates(&manager, id));
            assert_eq!(entity.mass_raw, 50);
            // A newborn's +34 is zero: no Run Away at birth.
            assert!(matches!(class(&manager, id), 4 | 5 | 6), "world{level}");
            assert!(Intro2FlyerSchedulerOwner::adopt_published(entity).is_ok());
        }
        let before: Vec<_> = ids
            .iter()
            .map(|&id| {
                manager
                    .iter_all()
                    .find(|e| e.id == id)
                    .unwrap()
                    .position_raw()
            })
            .collect();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_flyers(&manager);
        // One is detailed throughout; coarse visits run the tasks too.
        manager
            .entity_mut(ids[0])
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x0200_0000, 0x0200_0000);
        let mut classes = std::collections::BTreeSet::new();
        for tick in 1001..2001 {
            let outcomes = pass(&mut session, &mut manager, &mut fx, &mut scheduler, tick);
            assert_eq!(outcomes.len(), count, "world{level} tick{tick}");
            assert_progress(&outcomes);
            for &id in &ids {
                classes.insert(class(&manager, id));
            }
        }
        assert!(classes.iter().all(|class| matches!(class, 4 | 5 | 6)));
        let moved = ids.iter().zip(&before).any(|(&id, before)| {
            manager
                .iter_all()
                .find(|e| e.id == id)
                .unwrap()
                .position_raw()
                != *before
        });
        assert!(moved, "world{level}");
    }
}

#[v2k_test_support::retail_test]
fn a_recent_hit_weighs_run_away_and_its_search_flees_a_nearby_type41() {
    let (mut session, mut manager, mut fx) = world(34);
    let ids = type41s(&manager);
    let (id, other) = (ids[0], ids[1]);
    let metadata = manager
        .type_runtime_metadata(TYPE41_ENTITY_TYPE)
        .cloned()
        .unwrap();
    // Within the 512 axis of a same-type body (filter word zero).
    let position = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .position_raw();
    manager.entity_mut(other).unwrap().set_position_raw([
        position[0].wrapping_add(200),
        position[1],
        position[2],
    ]);
    let entity = manager.intro2_flyer_entity_mut(id).unwrap();
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(1000);
    assert_eq!(under_attack(entity, 1001), Some(true));
    assert_eq!(under_attack(entity, 1250), Some(false));
    assert_eq!(under_attack(entity, 200), Some(false), "clock below 250");
    // Weights 1/3/1/4: the last word slot picks Run Away.
    reselect_flyer(entity, &metadata, 1001, &mut || 0xffff).unwrap();
    assert_eq!(class(&manager, id), 10);
    assert!(acquiring_graph_authenticates(
        manager.iter_all().find(|e| e.id == id).unwrap()
    ));
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_flyers(&manager);
    let mut fled = false;
    for tick in 1001..1101 {
        assert_progress(&pass(
            &mut session,
            &mut manager,
            &mut fx,
            &mut scheduler,
            tick,
        ));
        if manager
            .iter_all()
            .find(|e| e.id == id)
            .is_some_and(fleeing_graph_authenticates)
        {
            fled = true;
            break;
        }
    }
    assert!(fled, "C7D0 hands the same-type body to B3F0");
}

#[v2k_test_support::retail_test]
fn a_dead_type41_falls_through_class12_and_is_removed_on_a_coarse_visit() {
    let (mut session, mut manager, mut fx) = world(33);
    let id = type41s(&manager)[0];
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_flyers(&manager);
    pass(&mut session, &mut manager, &mut fx, &mut scheduler, 1001);
    let death = publish_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(death.returned_nonzero);
    scheduler.register_intro2_flyer(death.publication.unwrap());
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert!(dying_graph_authenticates(entity));
    assert_eq!(entity.velocity_raw()[1], 500);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    // A repeated 10C10 is the already-dying no-op.
    let again = publish_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(again.returned_nonzero && again.publication.is_none());
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0200_0000, 0x0200_0000);
    let roll = manager
        .iter_all()
        .find(|e| e.id == id)
        .unwrap()
        .rotation_heading_pitch_roll_raw();
    for tick in 1002..1052 {
        assert_progress(&pass(
            &mut session,
            &mut manager,
            &mut fx,
            &mut scheduler,
            tick,
        ));
    }
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert!(entity.active && dying_graph_authenticates(entity));
    assert_ne!(entity.rotation_heading_pitch_roll_raw(), roll, "flips over");
    assert!(
        entity.velocity_raw()[1] < 500,
        "gravity pulls the corpse down"
    );
    // E870's 04220 takes 4BE170: the terminal stages removal.
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0200_0000, 0);
    for tick in 1052..1152 {
        assert_progress(&pass(
            &mut session,
            &mut manager,
            &mut fx,
            &mut scheduler,
            tick,
        ));
        if manager.pending_actor_deferred_destroy_ids().contains(&id) {
            break;
        }
    }
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    assert!(finished_dying_authenticates(
        manager.iter_all().find(|e| e.id == id).unwrap()
    ));
}
