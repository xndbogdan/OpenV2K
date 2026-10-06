//! Real-data check: overlay-13 Section-13 Type-9 Go-To-Job/Wander must leave spawn.

use std::path::PathBuf;

use v2k_game::entity::{EntityConstructionResources, EntityManager};
use v2k_game::entity_collision_state::EntityTypeRuntimeMetadata;
use v2k_game::game_state::LoadingPurpose;
use v2k_game::gameplay_notifications::GameplayNotifications;
use v2k_game::opening::FIRST_WORLD_LEVEL_ID;
use v2k_game::save::SavedPlayerState;
use v2k_game::session::GameSession;
use v2k_game::specialized_actor_task_production::{
    SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
    SpecializedActorTaskScheduler,
};

fn data_dir() -> PathBuf {
    let dir = v2k_test_support::retail_dir();
    assert!(
        dir.join("PRELOAD.DAT").is_file(),
        "canonical retail corpus is required: {}",
        dir.display()
    );
    dir
}

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

fn load_level1(session: &mut GameSession) {
    session
        .load_auxiliary_ovl(3, 1)
        .expect("load normal-tier first-world resources");
    session
        .load_level_by_id(13, 1)
        .expect("load normal-tier Level 1");
}

fn dummy_saved_player() -> SavedPlayerState {
    SavedPlayerState {
        position_raw: [0; 3],
        velocity_raw: [0; 3],
        heading_raw: 0,
        pitch_raw: 0,
        roll_raw: 0,
        health_raw: 0,
    }
}

fn tick_selected_type9(
    manager: &mut EntityManager,
    session: &mut GameSession,
    world_fx: &mut v2k_game::world_fx::WorldFx,
    frames: u32,
) -> (usize, Option<String>) {
    let mut scheduler = SpecializedActorTaskScheduler::new();
    let adopted = scheduler
        .adopt_fresh_level1_type9_selected(manager)
        .expect("selected Type-9 adoption");

    let mut notifications = GameplayNotifications::new();
    let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
    let mut first_block = None;
    for frame in 0..frames {
        let pass = scheduler.tick(
            manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: frame,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        if first_block.is_none() {
            first_block = pass.outcomes.into_iter().find_map(|outcome| match outcome {
                SpecializedActorTaskProductionOutcome::OrdinaryType9GoToJob(
                    v2k_game::ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome::Blocked {
                        entity_id,
                        reason,
                    },
                ) => Some(format!("go-to-job {entity_id}: {reason:?}")),
                SpecializedActorTaskProductionOutcome::OrdinaryType9Wander(
                    v2k_game::ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome::Blocked {
                        entity_id,
                        reason,
                    },
                ) => Some(format!("wander {entity_id}: {reason:?}")),
                _ => None,
            });
        }
    }
    (adopted, first_block)
}

#[test]
fn ordinary_section13_rebuilds_use_shared_publication() {
    assert!(LoadingPurpose::Normal.uses_authored_world_publication(FIRST_WORLD_LEVEL_ID));
    assert!(LoadingPurpose::CampaignWarp {
        source_level_id: 30,
        arrival_position_raw: [0; 3],
        arrival_heading_raw: 0,
    }
    .uses_authored_world_publication(FIRST_WORLD_LEVEL_ID));
    assert!(LoadingPurpose::Normal.uses_authored_world_publication(30));
    assert!(LoadingPurpose::CampaignWarp {
        source_level_id: FIRST_WORLD_LEVEL_ID,
        arrival_position_raw: [0; 3],
        arrival_heading_raw: 0,
    }
    .uses_authored_world_publication(14));
    let mut payload = [0; 0x248];
    payload[0x20..0x24].copy_from_slice(&1_u32.to_le_bytes());
    payload[0x3c..0x40].copy_from_slice(&40_000_i32.to_le_bytes());
    payload[0x40..0x44].copy_from_slice(&1_u32.to_le_bytes());
    payload[0x140..0x144].copy_from_slice(&46_u32.to_le_bytes());
    let restore =
        v2k_game::save::NativeSaveRestore::decode(&v2k_game::save::NativeCompatibilityPreview {
            logical_level_id: 1,
            state_payload: payload,
        })
        .unwrap();
    assert!(LoadingPurpose::NativeSave {
        restore: Box::new(restore),
    }
    .uses_authored_world_publication(FIRST_WORLD_LEVEL_ID));
    assert!(!LoadingPurpose::PortableCompatibilityPreview {
        player: dummy_saved_player(),
    }
    .uses_authored_world_publication(FIRST_WORLD_LEVEL_ID));
}

#[v2k_test_support::retail_test]
fn generic_level1_type9_has_no_selected_scheduler_owner() {
    let dir = data_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    load_level1(&mut session);

    let type_metadata = type_metadata(&session);
    let resources =
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects());
    let mut manager = EntityManager::from_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        resources,
    );

    let mut scheduler = SpecializedActorTaskScheduler::new();
    let adopted = scheduler
        .adopt_fresh_level1_type9_selected(&mut manager)
        .expect("generic Type-9 adoption is empty rather than rejected");
    assert_eq!(
        adopted, 0,
        "portable generic construction must fail closed for selected Type-9 publication"
    );
}

#[v2k_test_support::retail_test]
fn fresh_level1_type9_go_to_job_or_wander_leaves_spawn() {
    let dir = data_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    load_level1(&mut session);

    let type_metadata = type_metadata(&session);
    let resources =
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects());
    let mut world_fx = v2k_game::world_fx::WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        resources,
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    let start: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| {
            (
                entity.id,
                entity.authored_spawn_index,
                entity.position_raw(),
            )
        })
        .collect();
    assert!(
        !start.is_empty(),
        "fresh Level 1 must publish ordinary Type-9 villagers"
    );

    let (adopted, first_block) = tick_selected_type9(&mut manager, &mut session, &mut world_fx, 90);
    assert!(
        adopted > 0,
        "fresh Level 1 must adopt at least one Type-9 scheduler owner"
    );

    let moved = start.iter().any(|(id, _, before)| {
        manager
            .iter_all()
            .find(|entity| entity.id == *id)
            .is_some_and(|entity| {
                let after = entity.position_raw();
                [after[0], after[2]] != [before[0], before[2]]
            })
    });
    assert!(
        moved,
        "no Type-9 left spawn after 90 frames (adopted {adopted}); first block: {first_block:?}; start={start:?}"
    );
}

#[v2k_test_support::retail_test]
fn fresh_windmill_peasant_moves_across_initial_rng_states_without_player_contact() {
    use v2k_game::entity_view_detail::RetailViewDetailContext;
    use v2k_game::ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionOutcome as Attract;
    use v2k_game::ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome as GoToJob;
    use v2k_game::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome as RunAway;
    use v2k_game::ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome as Wander;
    use SpecializedActorTaskProductionOutcome as Outcome;

    let mut session = GameSession::init(&data_dir()).unwrap();
    load_level1(&mut session);
    let metadata = type_metadata(&session);
    let scan_dimensions = v2k_render::terrain_tiles::scan_dimensions(
        session.cache.level_desc().unwrap().terrain_draw_depth,
    );

    // The cold RNG cursor happened to wait on the first visit. These ten
    // offsets instead froze spawn 15 on frame zero at 20ms: Continue read an
    // unresolved +B2, then retained CallbackFailurePending indefinitely.
    let formerly_stuck_offsets = [3, 11, 13, 18, 29, 32, 39, 40, 42, 63];
    for elapsed_micros in [20_000_u32, 16_667, 6_944] {
        for rng_offset in 0..64 {
            if elapsed_micros != 20_000 && !formerly_stuck_offsets.contains(&rng_offset) {
                continue;
            }
            session.load_level_by_id(13, 1).unwrap();
            let mut world_fx = v2k_game::world_fx::WorldFx::new();
            for _ in 0..rng_offset {
                world_fx.next_shared_retail_random_u16();
            }
            let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
                session.cache.level_desc().unwrap(),
                &metadata,
                EntityConstructionResources::new(
                    session.cache.terrain(),
                    session.cache.terrain_objects(),
                ),
                0,
                &mut world_fx,
            )
            .unwrap();
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(
                scheduler
                    .adopt_fresh_level1_type9_selected(&mut manager)
                    .unwrap(),
                6
            );
            // Include the other live owners so the shared RNG ordering matches
            // gameplay. The player remains at spawn throughout the run.
            scheduler.adopt_fresh_level1_type17_follow_beacons(&manager);
            scheduler.adopt_level_one_factory_arrival(&manager);
            scheduler
                .adopt_fresh_level1_type47_scheduler(&mut manager)
                .unwrap();
            let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
            let mut notifications = GameplayNotifications::new();
            notifications
                .drain_fresh_level1_type9_attract_attention_receipts(&mut manager)
                .unwrap();
            let windmill_peasant = manager
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(15))
                .unwrap();
            assert_eq!(windmill_peasant.entity_type, 9);
            let peasant_id = windmill_peasant.id;
            let mut previous_position = windmill_peasant.position_raw();
            let mut late_xz_steps = 0_u64;

            for frame in 0..(30_000_000 / elapsed_micros) {
                scheduler.adopt_live_type8_go_to_job(&manager);
                scheduler.adopt_level_one_factory_arrival(&manager);
                let claims = scheduler.actor_animation_claims().collect::<Vec<_>>();
                let pass = scheduler.tick(
                    &mut manager,
                    SpecializedActorTaskProductionFrame {
                        world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                        hive_components: None,
                        notification_phase:
                            v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                        resources: &mut session.cache,
                        world_fx: &mut world_fx,
                        static_damage: &mut static_damage,
                        elapsed_micros,
                        global_elapsed_micros: elapsed_micros,
                        retail_tick: ((u64::from(frame) * u64::from(elapsed_micros)) / 20_000)
                            as u32,
                        main_base_abort_active: false,
                    },
                    &mut notifications,
                );
                for outcome in pass.outcomes {
                    let failed = match &outcome {
                        Outcome::OrdinaryType9Wander(value) => {
                            matches!(value, Wander::Blocked { .. } | Wander::Dropped { .. })
                        }
                        Outcome::OrdinaryType9GoToJob(value) => {
                            matches!(value, GoToJob::Blocked { .. } | GoToJob::Dropped { .. })
                        }
                        Outcome::OrdinaryType9RunAway(value) => {
                            matches!(value, RunAway::Blocked { .. } | RunAway::Dropped { .. })
                        }
                        Outcome::OrdinaryType9AttractAttention(value) => {
                            matches!(value, Attract::Blocked { .. } | Attract::Dropped { .. })
                        }
                        _ => false,
                    };
                    assert!(
                        !failed,
                        "RNG offset={rng_offset}, dt={elapsed_micros}, frame={frame}: {outcome:?}"
                    );
                }
                manager.advance_unclaimed_actor_animations(elapsed_micros, &claims);
                let player = manager.player().unwrap().position;
                scheduler.publish_presented_view_detail(
                    &mut manager,
                    RetailViewDetailContext::from_world(
                        [player[0], player[1] + 8.0, player[2] - 8.0],
                        0.7,
                        scan_dimensions,
                    ),
                );
                let position = manager
                    .iter_all()
                    .find(|entity| entity.id == peasant_id)
                    .unwrap()
                    .position_raw();
                if frame * elapsed_micros >= 20_000_000 {
                    late_xz_steps += u64::from(
                        position[0]
                            .wrapping_sub(previous_position[0])
                            .unsigned_abs(),
                    ) + u64::from(
                        position[2]
                            .wrapping_sub(previous_position[2])
                            .unsigned_abs(),
                    );
                }
                previous_position = position;
            }
            assert!(
                late_xz_steps > 100,
                "RNG offset={rng_offset}, dt={elapsed_micros}: windmill peasant stopped moving ({late_xz_steps})"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn fresh_level1_peasants_keep_moving_after_repeated_root_reselection() {
    use std::collections::BTreeMap;
    use v2k_game::ordinary_type9_attract_attention_production::OrdinaryType9AttractAttentionProductionOutcome as Attract;
    use v2k_game::ordinary_type9_go_to_job_production::OrdinaryType9GoToJobProductionOutcome as GoToJob;
    use v2k_game::ordinary_type9_run_away_production::OrdinaryType9RunAwayProductionOutcome as RunAway;
    use v2k_game::ordinary_type9_wander_production::OrdinaryType9WanderProductionOutcome as Wander;
    use SpecializedActorTaskProductionOutcome as Outcome;

    for elapsed_micros in [20_000_u32, 16_667, 6_944] {
        let mut session = GameSession::init(&data_dir()).unwrap();
        load_level1(&mut session);
        let metadata = type_metadata(&session);
        let resources = EntityConstructionResources::new(
            session.cache.terrain(),
            session.cache.terrain_objects(),
        );
        let mut world_fx = v2k_game::world_fx::WorldFx::new();
        let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
            session.cache.level_desc().unwrap(),
            &metadata,
            resources,
            0,
            &mut world_fx,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(
            scheduler
                .adopt_fresh_level1_type9_selected(&mut manager)
                .unwrap(),
            6
        );
        // Match the ordinary main pass's other families and shared RNG order.
        let mut static_damage = v2k_game::static_damage::StaticDamageScheduler::new();
        scheduler.adopt_fresh_level1_type17_follow_beacons(&manager);
        scheduler.adopt_level_one_factory_arrival(&manager);
        scheduler
            .adopt_fresh_level1_type47_scheduler(&mut manager)
            .unwrap();
        let mut notifications = GameplayNotifications::new();
        let mut positions = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9)
            .map(|entity| (entity.id, entity.position_raw()))
            .collect::<BTreeMap<_, _>>();
        let mut late_xz_steps = positions
            .keys()
            .map(|id| (*id, 0_u64))
            .collect::<BTreeMap<_, _>>();
        let mut root_publications = positions
            .keys()
            .map(|id| (*id, 0_u32))
            .collect::<BTreeMap<_, _>>();
        let mut cross_family_published = false;
        for frame in 0..(30_000_000 / elapsed_micros) {
            scheduler.adopt_live_type8_go_to_job(&manager);
            scheduler.adopt_level_one_factory_arrival(&manager);
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut world_fx,
                    static_damage: &mut static_damage,
                    elapsed_micros,
                    global_elapsed_micros: elapsed_micros,
                    retail_tick: ((u64::from(frame) * u64::from(elapsed_micros)) / 20_000) as u32,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            for outcome in pass.outcomes {
                let (failed, published, cross_family) = match &outcome {
                    Outcome::OrdinaryType9Wander(value) => (
                        matches!(value, Wander::Blocked { .. } | Wander::Dropped { .. }),
                        matches!(
                            value,
                            Wander::RootWanderPublished { .. }
                                | Wander::RootGoToJobPublished { .. }
                                | Wander::RootRunAwayPublished { .. }
                                | Wander::RootAttractAttentionPublished { .. }
                        ),
                        matches!(
                            value,
                            Wander::RootRunAwayPublished { .. }
                                | Wander::RootGoToJobPublished { .. }
                        ),
                    ),
                    Outcome::OrdinaryType9RunAway(value) => (
                        matches!(value, RunAway::Blocked { .. } | RunAway::Dropped { .. }),
                        matches!(
                            value,
                            RunAway::RootWanderPublished { .. }
                                | RunAway::RootRunAwayPublished { .. }
                        ),
                        matches!(value, RunAway::RootWanderPublished { .. }),
                    ),
                    Outcome::OrdinaryType9GoToJob(value) => (
                        matches!(value, GoToJob::Blocked { .. } | GoToJob::Dropped { .. }),
                        matches!(
                            value,
                            GoToJob::RootWanderPublished { .. }
                                | GoToJob::RootGoToJobPublished { .. }
                                | GoToJob::RootRunAwayPublished { .. }
                                | GoToJob::RootAttractAttentionPublished { .. }
                        ),
                        false,
                    ),
                    Outcome::OrdinaryType9AttractAttention(value) => (
                        matches!(value, Attract::Blocked { .. } | Attract::Dropped { .. }),
                        matches!(
                            value,
                            Attract::RootWanderPublished { .. }
                                | Attract::RootGoToJobPublished { .. }
                                | Attract::RootRunAwayPublished { .. }
                                | Attract::RootAttractAttentionPublished { .. }
                        ),
                        false,
                    ),
                    _ => continue,
                };
                assert!(!failed, "dt={elapsed_micros}, frame={frame}: {outcome:?}");
                if published {
                    *root_publications.get_mut(&outcome.entity_id()).unwrap() += 1;
                }
                cross_family_published |= cross_family;
            }
            for entity in manager.iter_all().filter(|entity| entity.entity_type == 9) {
                let after = entity.position_raw();
                let before = positions.insert(entity.id, after).unwrap();
                if u64::from(frame) * u64::from(elapsed_micros) >= 20_000_000 {
                    *late_xz_steps.get_mut(&entity.id).unwrap() +=
                        u64::from(after[0].wrapping_sub(before[0]).unsigned_abs())
                            + u64::from(after[2].wrapping_sub(before[2]).unsigned_abs());
                }
            }
        }
        assert!(
            cross_family_published,
            "dt={elapsed_micros}: exercise a cross-family root publication"
        );
        for (id, steps) in late_xz_steps {
            assert!(steps > 100, "dt={elapsed_micros}, peasant={id}: no sustained XZ motion after 20 seconds ({steps})");
            assert!(
                root_publications[&id] >= 2,
                "dt={elapsed_micros}, peasant={id}: did not exercise repeated root replacement"
            );
        }
        assert_eq!(
            scheduler.registered_len(),
            11,
            "dt={elapsed_micros}: no selected owner may disappear after replacement"
        );

        // Preserving arbitrary root context words is not permission to adopt
        // an unrelated same-class context mutation between actor visits.
        use v2k_game::entity_behavior::{BehaviorContextRuntime, BehaviorDescriptorIdentity};
        use v2k_game::entity_collision_state::RetailRuntimeValue;
        let id = *positions.keys().next().unwrap();
        let entity = manager.entity_mut(id).unwrap();
        let before = entity.position_raw();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("live root context")
        };
        let BehaviorDescriptorIdentity::Named(program) = context.descriptor() else {
            panic!("audited root descriptor")
        };
        let RetailRuntimeValue::Known(auxiliary) = context.auxiliary_word_at_0x0c() else {
            panic!("known root auxiliary word")
        };
        entity.current_behavior_context =
            RetailRuntimeValue::Known(BehaviorContextRuntime::named_audited(
                program,
                context.style_table_index_raw_at_0x10(),
                context.choice_list_source(),
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(auxiliary.wrapping_add(1)),
                context.active_style().audited().unwrap(),
            ));
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    v2k_game::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut world_fx,
                static_damage: &mut static_damage,
                elapsed_micros,
                global_elapsed_micros: elapsed_micros,
                retail_tick: 1500,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(
            pass.outcomes.iter().any(|outcome| outcome.entity_id() == id
                && matches!(
                    outcome,
                    Outcome::OrdinaryType9Wander(Wander::Dropped { .. })
                        | Outcome::OrdinaryType9RunAway(RunAway::Dropped { .. })
                        | Outcome::OrdinaryType9GoToJob(GoToJob::Dropped { .. })
                )),
            "dt={elapsed_micros}: unrelated context must invalidate current task custody"
        );
        assert_eq!(
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .position_raw(),
            before
        );
    }
}

#[v2k_test_support::retail_test]
fn peasant_campaign_return_type9_go_to_job_or_wander_leaves_spawn() {
    let dir = data_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    load_level1(&mut session);

    let type_metadata = type_metadata(&session);
    let resources =
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects());
    let mut world_fx = v2k_game::world_fx::WorldFx::new();
    let mut manager = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().expect("Level-1 Section 13"),
        &type_metadata,
        resources,
        0,
        &mut world_fx,
    )
    .expect("fresh Level-1 construction");

    manager.place_or_spawn_player_at_campaign_arrival(
        type_metadata.get(46),
        [0x4D00, 0, 0x3A00],
        0x4000,
        session.cache.terrain(),
    );

    let start: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 9)
        .map(|entity| (entity.id, entity.position_raw()))
        .collect();
    assert!(
        !start.is_empty(),
        "campaign-return Peasant must still publish ordinary Type-9 villagers"
    );

    let (adopted, first_block) = tick_selected_type9(&mut manager, &mut session, &mut world_fx, 90);
    assert!(
        adopted > 0,
        "campaign-return Peasant must adopt at least one Type-9 scheduler owner"
    );

    let moved = start.iter().any(|(id, before)| {
        manager
            .iter_all()
            .find(|entity| entity.id == *id)
            .is_some_and(|entity| entity.position_raw() != *before)
    });
    assert!(
        moved,
        "no Type-9 left spawn after campaign-return placement (adopted {adopted}); first block: {first_block:?}; start={start:?}"
    );
}
