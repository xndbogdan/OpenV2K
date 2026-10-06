//! Every ordinary authored fish uses the same constructor and live owner.

use v2k_game::{
    actor_task_owner::ActorTaskSlot,
    entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    gameplay_notifications::GameplayNotifications,
    session::GameSession,
    shared_fish::SharedFishOutcome,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

// Discover the neutral swimming population from its authored component and
// behavior records. A production type list must not also define test coverage:
// that hid Type23 and the first world's Type62 zebrafsh from the old census.
fn is_authored_neutral_swimmer(metadata: &EntityTypeRuntimeMetadata) -> bool {
    matches!(
        metadata.sub_f_swimming_descriptor,
        RetailRuntimeValue::Known(Some(_))
    ) && metadata.initializer.as_ref().is_some_and(|initializer| {
        initializer.initializer_state_flags_raw & 0x2000 != 0
            && !initializer.behavior_choices.is_empty()
            && initializer.behavior_choices.iter().all(|choice| {
                choice.weight_rule_id == 1 && matches!(choice.behavior_class_id, 5 | 6 | 13)
            })
    })
}

#[v2k_test_support::retail_test]
fn every_ordinary_world_publishes_and_runs_its_native_fish() {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    let mut fx = WorldFx::new();
    let mut census = Vec::new();
    let mut failures = Vec::new();
    for world in 13..=49 {
        session.load_level_by_id(world, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        let rows: Vec<_> = session
            .cache
            .global_entity_model_table()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let count = level
            .entities
            .iter()
            .filter(|entity| is_authored_neutral_swimmer(&rows[entity.entity_type as usize]))
            .count();
        if count == 0 {
            continue;
        }
        let mut manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level,
                logical_world_index: world as i32 - 12,
                type_metadata: &rows,
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                player_arrival: None,
                retail_tick: 1000,
            },
            &mut fx,
        )
        .unwrap_or_else(|e| panic!("world{world}: {e:?}"));
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_shared_fish(&manager), count, "world{world}");
        let mut damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for tick in 1001..=1010 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            if pass.block.is_some() {
                failures.push(format!("world{world}: {pass:?}"));
                break;
            }
            assert_eq!(pass.outcomes.len(), count);
            let mut blocked = false;
            for outcome in pass.outcomes {
                if !matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::SharedFish(
                        SharedFishOutcome::Advanced { .. } | SharedFishOutcome::Waiting { .. }
                    )
                ) {
                    failures.push(format!("world{world}: {outcome:?}"));
                    blocked = true;
                    break;
                }
            }
            if blocked {
                break;
            }
        }
        census.push((world, count));
        fx.clear();
    }
    assert_eq!(
        census,
        [
            (13, 3),
            (14, 3),
            (18, 4),
            (22, 11),
            (23, 43),
            (30, 20),
            (34, 19),
            (36, 11)
        ]
    );
    eprintln!("native fish world/count census: {census:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[v2k_test_support::retail_test]
fn level_one_zebrafish_swim_animate_and_resume_after_coarse_parking() {
    let data = v2k_test_support::retail_dir();
    assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let rows: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level: session.cache.level_desc().unwrap(),
            logical_world_index: 1,
            type_metadata: &rows,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: None,
            retail_tick: 1000,
        },
        &mut fx,
    )
    .unwrap();
    let fish: Vec<_> = manager
        .iter_all()
        .filter(|entity| is_authored_neutral_swimmer(&rows[entity.entity_type as usize]))
        .map(|entity| (entity.id, entity.authored_spawn_index.unwrap()))
        .collect();
    assert_eq!(fish.len(), 3);
    assert_eq!(
        fish.iter().map(|(_, spawn)| *spawn).collect::<Vec<_>>(),
        [4, 25, 26]
    );
    let sea = session.cache.terrain().unwrap().sea_level_raw();
    assert_eq!(sea, -847);
    for &(id, spawn_index) in &fish {
        let entity = manager.entity_mut(id).unwrap();
        let authored = session.cache.level_desc().unwrap().entities[spawn_index].position_raw();
        assert_eq!(entity.entity_type, 62);
        assert_eq!(entity.model_index, Some(38));
        // Model38 derives fin-morph register1 from callback0. Sub-F's
        // changing bank alone cannot animate it while that clock is zero.
        assert_eq!(entity.presentation_anim_vars(0x1_2345).dynamic[0], 0x2345);
        let model = session.cache.global_model(38).unwrap();
        assert_ne!(
            model
                .materialize(&entity.presentation_anim_vars(0))
                .vertices,
            model
                .materialize(&entity.presentation_anim_vars(16))
                .vertices,
            "spawn{spawn_index} fin geometry must consume the retail clock"
        );
        assert_eq!(
            entity.position_raw(),
            authored,
            "birth must preserve authored Y"
        );
        assert_eq!(
            entity.position_raw()[1] > sea,
            matches!(spawn_index, 4 | 26)
        );
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
    }
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_shared_fish(&manager), 3);
    let mut damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut retail_tick = 1000;
    let mut advance = |manager: &mut EntityManager, frames: usize| {
        for _ in 0..frames {
            retail_tick += 1;
            let pass = scheduler.tick(
                manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "{pass:?}");
            assert_eq!(pass.outcomes.len(), 3);
            assert!(
                pass.outcomes.iter().all(|outcome| matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::SharedFish(
                        SharedFishOutcome::Advanced { .. } | SharedFishOutcome::Waiting { .. }
                    )
                )),
                "{pass:?}"
            );
        }
    };
    let snapshot = |manager: &EntityManager| {
        fish.iter()
            .map(|&(id, _)| {
                let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
                (
                    entity.position_raw(),
                    entity.velocity_raw(),
                    entity.rotation_heading_pitch_roll_raw(),
                    entity.presentation_anim_vars(1000).dynamic,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                )
            })
            .collect::<Vec<_>>()
    };
    // A coarse callback must retain the above-water authored positions. The
    // first detailed Sub-F invocation, not construction, owns the sea cap.
    let authored = snapshot(&manager);
    advance(&mut manager, 30);
    assert_eq!(snapshot(&manager), authored);
    for &(id, _) in &fish {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
    }
    advance(&mut manager, 1);
    for &(id, spawn_index) in &fish {
        if matches!(spawn_index, 4 | 26) {
            assert!(
                manager.entity_mut(id).unwrap().position_raw()[1] <= sea - 100,
                "spawn{spawn_index} must enter Sub-F's underwater cap"
            );
        }
    }
    advance(&mut manager, 300);
    let swimming = snapshot(&manager);
    for (index, (_, spawn)) in fish.iter().enumerate() {
        assert_ne!(
            swimming[index].0, authored[index].0,
            "spawn{spawn} did not swim"
        );
        assert_ne!(
            swimming[index].3, authored[index].3,
            "spawn{spawn} did not animate"
        );
    }
    for &(id, _) in &fish {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
    }
    advance(&mut manager, 30);
    assert_eq!(
        snapshot(&manager),
        swimming,
        "coarse fish must retain pose, animation and task time"
    );
    for &(id, _) in &fish {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
    }
    advance(&mut manager, 100);
    let resumed = snapshot(&manager);
    for (index, (_, spawn)) in fish.iter().enumerate() {
        assert_ne!(
            resumed[index].0, swimming[index].0,
            "spawn{spawn} failed to resume motion"
        );
        assert_ne!(
            resumed[index].3, swimming[index].3,
            "spawn{spawn} failed to resume animation"
        );
    }
}
