//! Campaign-wide loss transition coverage, with exact actor-walk gaps explicit.

use std::num::NonZeroU64;
use v2k_game::{
    actor_task_owner::ActorTaskSlot,
    campaign_transition::RETAIL_CONTROLLER_DEFAULT_ARRIVAL,
    entity::{
        AuthoredPlayerArrival, AuthoredWorldConstruction, EntityConstructionResources,
        EntityManager,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    main_base_abort::{MainBaseAbortControllerStorage, MainBaseAbortTransactionId},
    main_base_abort_production::{
        execute_campaign_abort, MainBaseAbortActorCallbackBlock, MainBaseAbortFallbackRequest,
        MainBaseAbortGameplayContext, MainBaseAbortProductionDiagnostic,
    },
    player_hull::PlayerHull,
    power_up_contact::PlayerCampaignProgress,
    session::GameSession,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

fn adopt_authored_tasks(manager: &mut EntityManager) -> SpecializedActorTaskScheduler {
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_fresh_level1_type9_selected(manager).unwrap();
    tasks.adopt_fresh_level1_type17_follow_beacons(manager);
    tasks.adopt_fresh_level1_type47_scheduler(manager).unwrap();
    tasks.adopt_intro2_type13_search_attack(manager);
    tasks.adopt_intro2_type26(manager);
    tasks.adopt_intro2_type47_guards(manager);
    tasks.adopt_intro2_flyers(manager);
    tasks.adopt_intro2_type53(manager);
    tasks.adopt_type122(manager);
    tasks.adopt_type18(manager);
    tasks.adopt_type28(manager);
    tasks.adopt_type76_family(manager);
    tasks.adopt_type30(manager);
    tasks.adopt_rolling_boulders(manager);
    tasks.adopt_type40(manager);
    tasks.adopt_type56(manager);
    tasks.adopt_type43(manager);
    tasks.adopt_type38_family(manager);
    tasks.adopt_type12_family(manager);
    tasks.adopt_type31_family(manager);
    tasks.adopt_shared_fish(manager);
    tasks.adopt_cleansing_vehicle(manager);
    tasks.adopt_intro2_type16(manager);
    tasks.adopt_intro2_type58(manager);
    tasks.adopt_intro2_type94(manager);
    tasks.adopt_intro2_type66(manager);
    tasks.adopt_class0_actors(manager);
    tasks.adopt_portable_radars(manager);
    tasks.adopt_main_base(manager);
    tasks.adopt_intro2_type10(manager);
    tasks.adopt_intro2_type57(manager);
    tasks.adopt_intro2_gun_turret(manager);
    tasks.adopt_intro2_type17(manager);
    tasks.adopt_intro2_type8(manager);
    tasks.adopt_intro2_type9(manager);
    tasks.adopt_native_type123(manager);
    tasks.adopt_native_type86(manager);
    tasks.adopt_intro2_meteors(manager);
    tasks
}

fn construct_world(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    world: u32,
    fx: &mut WorldFx,
) -> EntityManager {
    let level = session.cache.level_desc().unwrap();
    let arrival = RETAIL_CONTROLLER_DEFAULT_ARRIVAL;
    EntityManager::from_authored_world(
        AuthoredWorldConstruction {
            level,
            logical_world_index: world as i32 - 12,
            type_metadata: &metadata,
            resources: EntityConstructionResources {
                terrain: session.cache.terrain(),
                terrain_objects: session.cache.terrain_objects(),
                model_extent_raw: Some(&|id| {
                    session.cache.global_model(id).map(|model| model.radius)
                }),
            },
            player_arrival: Some(AuthoredPlayerArrival {
                position_raw: arrival.position_raw,
                heading_raw: arrival.heading_raw,
            }),
            retail_tick: 4793,
        },
        fx,
    )
    .unwrap()
}

// First unsupported callback in the actual authored list, not a whitelist
// permitting arbitrary failure. Closing one requires updating this evidence
// matrix; every earlier actor must continue to execute or roll back correctly.
const FIRST_UNSUPPORTED: &[(u32, u32)] = &[
    // World 16 completes: its Type31s split into Type105s (class18), which
    // die quietly, and its Type50 dies into class38's Change Sea Level.
    // Type40 splits into Type56s (class18), which die quietly; Type30 takes
    // class12 and Type72 Change Sea Level. Worlds 17 and 45 complete.
    (18, 73),
    // World 19 completes: its Type55 guard takes class11 Tumble.
    // Type14 flyers explode through their class1 terminal, Type15 wasps die
    // quietly through class2, and Type16 and Type94 enter class12.
    (20, 29),
    // Native Type18/28 take Type122's captor class12 abort death; Flood's
    // Type89s leave class38 for their class1 explosion.
    (21, 74),
    (22, 41),
    // Native22/23/24 fish execute the class2 quiet terminal and the Type25
    // sharks class12; Reef's next unsupported callback is its Type37 actor.
    (23, 37),
    (24, 29),
    // World 25 completes: its Type82 factory shares Type66's class39 owner.
    (26, 109),
    // Type27 boulders split into Type3s (class18), which explode (class1);
    // world 27 completes with its Type127 carriers' class63.
    // Native class29 turrets now complete; their finished class49 terminals
    // admit the later power-up radial, as 14AE0 does. World 29's Type55
    // guards take class11 Tumble, and it completes.
    // Worlds 30 and 39 complete: their Type83 radars take class49.
    (31, 107),
    // World 32 completes: its Type55 guards take class11 Tumble, its Type82
    // factory class39 and its Type32s the ground host's terminal deaths.
    (33, 20),
    // The Type25 shark takes class12; next is the Type117 carrier.
    (34, 117),
    // World 35 completes: its Type12s explode through class1.
    (36, 74),
    // World 37 completes: its Type106 dies quietly through class2.
    // World 38 completes: its Type82 factory takes class39 too.
    // Type125 factories share Type66's class39 owner; Type50 dies into
    // class38's Change Sea Level and the Type118s into class65 Geysers.
    (40, 121),
    // World 41 completes: its Type35 launchers republish their class7
    // alternate on the dying body.
    // Type108 flyers die quietly and the Type83 radar through class49;
    // Alien1's Type35s republish Search and its next is a Type36.
    (42, 36),
    (43, 33),
    // Ordinary Type13 and the emitter-only Type43 take their native class1
    // abort deaths, Type5 its class11 Tumble publication and Type35 Search.
    (46, 36),
    (47, 110),
];

fn actor_snapshot(manager: &EntityManager) -> Vec<String> {
    manager
        .iter_all()
        .map(|entity| {
            format!(
                "{:?}",
                (
                    entity.id,
                    entity.entity_type,
                    entity.position_raw(),
                    entity.collision.clone(),
                    entity.current_behavior_context,
                    entity.base_factory_runtime,
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .map(|slot| entity.actor_task_state(slot).copied()),
                )
            )
        })
        .collect()
}

fn terrain_snapshot(session: &GameSession) -> Vec<[u8; 3]> {
    session
        .cache
        .terrain()
        .unwrap()
        .cells
        .iter()
        .map(|cell| [cell.height, cell.attribute, cell.terrain_type])
        .collect()
}

#[v2k_test_support::retail_test]
fn every_authored_casualty_world_has_a_loss_transition_and_explicit_actor_cleanup_coverage() {
    let data = v2k_test_support::retail_dir();
    assert!(
        data.join("PRELOAD.DAT").exists(),
        "retail campaign corpus required"
    );
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    let mut exact_worlds = Vec::new();
    let mut bounded_worlds = Vec::new();
    let mut casualty_worlds = Vec::new();
    let mut non_campaign_scenes = Vec::new();
    for world in 13..=50 {
        session.load_level_by_id(world, 1).unwrap();
        let level = session.cache.level_desc().unwrap();
        let casualty_records = level
            .campaign_records
            .iter()
            .filter(|record| record.flags() & 0x84 == 0x84)
            .count();
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some((world - 12) as usize));
        if progress.current_control_slot().is_none() {
            assert_eq!(casualty_records, 0, "non-campaign scene {world}");
            non_campaign_scenes.push(world);
            continue;
        }
        if casualty_records != 0 {
            assert_eq!(casualty_records, 1);
            casualty_worlds.push(world);
        }
        let mut fx = WorldFx::new();
        let mut manager = construct_world(&session, &metadata, world, &mut fx);
        let mut tasks = adopt_authored_tasks(&mut manager);
        let count_before = manager.iter_all().count();
        let (mut controller, _) = MainBaseAbortControllerStorage::from_loaded_level(
            NonZeroU64::new(u64::from(world)).unwrap(),
            level,
            false,
            &mut progress,
        )
        .unwrap();
        let controller_before = controller;
        let actors_before = actor_snapshot(&manager);
        let terrain_before = terrain_snapshot(&session);
        let mut notifications = GameplayNotifications::new();
        let result = execute_campaign_abort(
            MainBaseAbortTransactionId::new(u64::from(world)).unwrap(),
            None,
            &mut controller,
            &mut manager,
            &mut session.cache,
            &mut StaticDamageScheduler::new(),
            &mut PlayerHull::default(),
            &mut fx,
            &mut tasks,
            MainBaseAbortGameplayContext {
                extra_lives: v2k_game::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut notifications,
                retail_tick: 0x1dcb,
            },
        );
        match result {
            Ok(report) => {
                assert!(
                    !FIRST_UNSUPPORTED.iter().any(|(id, _)| *id == world),
                    "world{world} exact coverage advanced; update the evidence matrix"
                );
                assert!(report.processed_actors.len() >= count_before);
                assert!(report.completion.full_frame_submitted);
                exact_worlds.push(world);
            }
            Err(failure) => {
                let MainBaseAbortProductionDiagnostic::ActorCallbackBlocked {
                    actor,
                    error: MainBaseAbortActorCallbackBlock::UnsupportedOrdinaryType { entity_type },
                } = failure.diagnostic.clone()
                else {
                    panic!("world{world}: unexpected failure {:?}", failure.diagnostic);
                };
                // Compare the complete observed matrix after every world has
                // exercised its actor/terrain rollback and bounded fallback.
                // Stopping at the first changed row hid later coverage gaps.
                assert_eq!(controller, controller_before);
                assert_eq!(
                    actor_snapshot(&manager),
                    actors_before,
                    "world{world}: atomic actor rollback"
                );
                assert_eq!(
                    terrain_snapshot(&session),
                    terrain_before,
                    "world{world}: atomic terrain rollback"
                );
                assert_eq!(notifications, GameplayNotifications::new());
                assert!(failure.progress.processed_actors.is_empty());
                assert!(failure.progress.world_effects.is_none());
                assert!(failure.unscheduled_owner.is_none());
                let diagnostic = failure.diagnostic.clone();
                let unsupported_before = manager
                    .iter_all()
                    .find(|entity| entity.id == actor.lease.entity_id)
                    .unwrap()
                    .collision
                    .clone();
                let report = failure.apply_bounded_world_fallback(MainBaseAbortFallbackRequest {
                    world_control_lease: controller.world_control_lease(),
                    controller: &mut controller,
                    entities: &mut manager,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                });
                assert_eq!(report.diagnostic, diagnostic);
                assert!(
                    report.terminal_actor.is_none(),
                    "casualty does not invent a terminal Main Base"
                );
                // Independent control executes the former inline body from an
                // identically constructed world. This compares factory/audio,
                // terrain and exact shared-RNG suffix without exposing a
                // general WorldFx clone or test-only production accessors.
                let mut control_session = GameSession::init(&data).unwrap();
                control_session.load_auxiliary_ovl(3, 1).unwrap();
                control_session.load_level_by_id(world, 1).unwrap();
                let mut control_fx = WorldFx::new();
                let mut control_manager =
                    construct_world(&control_session, &metadata, world, &mut control_fx);
                let factories = control_manager.arm_live_factories_for_main_base_abort();
                for started in &factories {
                    if let Some(sound_id) = started.death_sound_id {
                        control_fx.queue_fixed_positional_sound_raw(
                            sound_id,
                            started.target_position_raw,
                        );
                    }
                }
                let terrain = control_session
                    .cache
                    .apply_main_base_abort_terrain_transform(|| {
                        control_fx.next_shared_retail_random_u16()
                    });
                assert_eq!(report.factories, factories);
                assert_eq!(report.terrain, terrain);
                assert_eq!(
                    fx.take_positional_sounds(),
                    control_fx.take_positional_sounds()
                );
                assert_eq!(
                    fx.next_shared_retail_random_u16(),
                    control_fx.next_shared_retail_random_u16(),
                    "world{world}: rollback plus fallback RNG order"
                );
                assert_eq!(
                    manager
                        .iter_all()
                        .find(|entity| entity.id == actor.lease.entity_id)
                        .unwrap()
                        .collision,
                    unsupported_before,
                    "fallback must not fabricate a missing death"
                );
                assert_eq!(
                    report.frame_request.unwrap(),
                    controller.submitted_frame_request()
                );
                bounded_worlds.push((world, entity_type));
            }
        }
        assert_eq!(
            controller.player_state(),
            RetailRuntimeValue::Known(5),
            "world{world}"
        );
        assert!(controller.abort_frame_submitted(), "world{world}");
    }
    assert_eq!(
        exact_worlds,
        [13, 14, 15, 16, 17, 19, 25, 27, 28, 29, 30, 32, 35, 37, 38, 39, 41, 44, 45, 48]
    );
    assert_eq!(
        bounded_worlds, FIRST_UNSUPPORTED,
        "first incomplete actor matrix changed"
    );
    assert_eq!(
        casualty_worlds,
        [13, 14, 15, 16, 17, 18, 19, 21, 24, 26, 31, 32, 34, 35, 36, 37, 39, 40, 42, 43, 46]
    );
    assert_eq!(non_campaign_scenes, [49, 50]);
}

#[v2k_test_support::retail_test]
fn bounded_fallback_keeps_terrain_and_factory_prefix_when_handoff_lease_is_stale() {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    // A one-factory world whose abort still stops early (world 21, at Type74).
    session.load_level_by_id(21, 1).unwrap();
    let metadata: Vec<_> = session
        .cache
        .global_entity_model_table()
        .iter()
        .copied()
        .enumerate()
        .map(|(id, model_slots)| {
            session
                .cache
                .global_entity_type(id)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots,
                    ..Default::default()
                })
        })
        .collect();
    let mut fx = WorldFx::new();
    let mut manager = construct_world(&session, &metadata, 21, &mut fx);
    let mut tasks = adopt_authored_tasks(&mut manager);
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some(9));
    let (mut controller, _) = MainBaseAbortControllerStorage::from_loaded_level(
        NonZeroU64::new(21).unwrap(),
        session.cache.level_desc().unwrap(),
        false,
        &mut progress,
    )
    .unwrap();
    for stale in [true, false] {
        let before = controller;
        let failure = execute_campaign_abort(
            MainBaseAbortTransactionId::new(21).unwrap(),
            None,
            &mut controller,
            &mut manager,
            &mut session.cache,
            &mut StaticDamageScheduler::new(),
            &mut PlayerHull::default(),
            &mut fx,
            &mut tasks,
            MainBaseAbortGameplayContext {
                extra_lives: v2k_game::entity_collision_state::RetailRuntimeValue::Unresolved,
                notifications: &mut GameplayNotifications::new(),
                retail_tick: 0x1dcb,
            },
        )
        .unwrap_err();
        let mut lease = controller.world_control_lease();
        if stale {
            lease.allocation_identity += 1;
        }
        let report = failure.apply_bounded_world_fallback(MainBaseAbortFallbackRequest {
            world_control_lease: lease,
            controller: &mut controller,
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
        });
        if stale {
            assert!(report.frame_request.is_err());
            assert_eq!(controller, before);
            assert_eq!(report.factories.len(), 1);
            assert!(report.terrain.is_some());
        } else {
            assert_eq!(
                report.frame_request.unwrap(),
                controller.submitted_frame_request()
            );
            assert!(
                report.factories.is_empty(),
                "known factory prefix already committed"
            );
            assert!(
                report.terrain.is_none(),
                "already-applied terrain still permits the handoff"
            );
            assert_eq!(controller.player_state(), RetailRuntimeValue::Known(5));
        }
    }
}
