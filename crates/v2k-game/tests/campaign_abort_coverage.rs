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
    // World 18 completes: its Type73 bugs die quietly through class2.
    // World 19 completes: its Type55 guard takes class11 Tumble.
    // Type14 flyers explode through their class1 terminal, Type15 wasps die
    // quietly through class2, and Type16 and Type94 enter class12.
    // World 20 completes: its Type29 mobbers take class11 Tumble.
    // World 21 completes: native Type18/28 take Type122's captor class12
    // abort death, and Flood's Type89s leave class38 and its Type74 Remote
    // Beamer class59 for their class1 explosions.
    // World 22 completes: its Type41 flyer falls through class12 on the
    // flyer owner, and its Type21 swimmers take the fish owner's class12.
    // World 23 completes: its Native22/23/24 fish take the class2 quiet
    // terminal, its Type25 sharks and Type37 class12, its Type20 class1 and
    // its Type117 class63.
    // World 24 completes: its Type29 mobbers take class11 Tumble.
    // World 25 completes: its Type82 factory shares Type66's class39 owner.
    // World 26 completes: its Type73/109 bugs die quietly and its Type121
    // mobile turrets take class49, as does world 40's.
    // Type27 boulders split into Type3s (class18), which explode (class1);
    // world 27 completes with its Type127 carriers' class63.
    // Native class29 turrets now complete; their finished class49 terminals
    // admit the later power-up radial, as 14AE0 does. World 29's Type55
    // guards take class11 Tumble, and it completes.
    // Worlds 30 and 39 complete: their Type83 radars take class49.
    // World 31 completes: its Type107/Type121 mobile turrets take class49.
    // World 32 completes: its Type55 guards take class11 Tumble, its Type82
    // factory class39 and its Type32s the ground host's terminal deaths.
    // World 33 completes: its Type20 swimmer explodes through class1 and its
    // Type41 flyers fall through class12.
    // World 34 completes: its Type25 shark and Type21 swimmers take class12,
    // its Type117 carrier class63 and its Type41 flyers class12.
    // World 35 completes: its Type12s explode through class1.
    // World 36 completes: its Type74 Remote Beamers explode through class1.
    // World 37 completes: its Type106 dies quietly through class2.
    // World 38 completes: its Type82 factory takes class39 too.
    // Type125 factories share Type66's class39 owner; Type50 dies into
    // class38's Change Sea Level and the Type118s into class65 Geysers.
    // World 40 completes: its Type121 mobile turrets take class49.
    // World 41 completes: its Type35 launchers republish their class7
    // alternate on the dying body.
    // World 42 completes: Type108 flyers die quietly, the Type83 radar
    // through class49, Alien1's Type35s republish Search and its Type36
    // flare generators republish class16.
    // World 43 completes: its Type33 fireectos take class12.
    // World 46 completes: ordinary Type13 and the emitter-only Type43 take
    // their native class1 abort deaths, Type5 its class11 Tumble publication
    // and Type35 Search.
    // World 47 completes: its Type110 Queen takes class67 again, dying.
];

/// Worlds whose first unported actor is reached by an earlier native actor's
/// death radial: (world, the dying actor's type, the unported target's type).
const RADIAL_BLOCKED: &[(u32, u32, u32)] = &[];

/// The innermost retained radial target whose callbacks are unresolved.
fn radial_unported_target(error: &str) -> Option<u32> {
    let marker = "Retained(DynamicRadialUnresolved { target_id: ";
    let start = error.rfind(marker)? + marker.len();
    let rest = &error[start..];
    rest[..rest.find(',')?].parse().ok()
}

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
                let MainBaseAbortProductionDiagnostic::ActorCallbackBlocked { actor, error } =
                    failure.diagnostic.clone()
                else {
                    panic!("world{world}: unexpected failure {:?}", failure.diagnostic);
                };
                let entity_type = match error {
                    MainBaseAbortActorCallbackBlock::UnsupportedOrdinaryType { entity_type } => {
                        entity_type
                    }
                    // A native actor's death radial reached an unported body.
                    error => {
                        let target =
                            radial_unported_target(&format!("{error:?}")).unwrap_or_else(|| {
                                panic!("world{world}: unexpected failure {error:?}")
                            });
                        let target_type = manager
                            .iter_all()
                            .find(|entity| entity.id == target)
                            .unwrap()
                            .entity_type;
                        assert!(
                            RADIAL_BLOCKED.contains(&(world, actor.entity_type, target_type)),
                            "world{world}: Type{} radial blocked at Type{target_type}",
                            actor.entity_type
                        );
                        target_type
                    }
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
        [
            13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34,
            35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48
        ]
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
    // A one-factory world. Every authored abort now completes, so a Type41
    // retyped to the unported Type65 stands in for an unsupported actor.
    const WORLD: u32 = 34;
    session.load_level_by_id(WORLD, 1).unwrap();
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
    let mut manager = construct_world(&session, &metadata, WORLD, &mut fx);
    let mut tasks = adopt_authored_tasks(&mut manager);
    let flyer = manager
        .iter_all()
        .find(|entity| entity.entity_type == 41)
        .unwrap()
        .id;
    manager.entity_mut(flyer).unwrap().entity_type = 65;
    let mut progress = PlayerCampaignProgress::new();
    progress.set_current_control_slot(Some((WORLD - 12) as usize));
    let (mut controller, _) = MainBaseAbortControllerStorage::from_loaded_level(
        NonZeroU64::new(u64::from(WORLD)).unwrap(),
        session.cache.level_desc().unwrap(),
        false,
        &mut progress,
    )
    .unwrap();
    for stale in [true, false] {
        let before = controller;
        let failure = execute_campaign_abort(
            MainBaseAbortTransactionId::new(u64::from(WORLD)).unwrap(),
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

/// The loss arrives mid-play: retail loses worlds 32 and 35 about 1,100
/// ticks after arrival. Every actor has run its own tasks by then, and an
/// owner its tick parked would refuse the abort's custody. Type40's chase
/// (Sub-D), Type32's Follow Beacons death styles and the Type125 factories'
/// understaffed countdown each did.
#[v2k_test_support::retail_test]
fn casualty_abort_completes_after_a_minute_of_actor_ticks() {
    use v2k_game::gameplay_notifications::GameplayNotificationPhase;
    use v2k_game::specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskWorld,
    };
    let data = v2k_test_support::retail_dir();
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
    for world in [32, 37, 40] {
        session.load_level_by_id(world, 1).unwrap();
        let mut fx = WorldFx::new();
        let mut manager = construct_world(&session, &metadata, world, &mut fx);
        let mut tasks = adopt_authored_tasks(&mut manager);
        // 44AFB0 builds the radar after authored construction, as the game does.
        session
            .cache
            .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
            .unwrap();
        let mut hull = PlayerHull::default();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for tick in 1..=1_100u32 {
            let pass = tasks.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: SpecializedActorTaskWorld::Playing {
                        player_hull: &mut hull,
                        extra_lives: RetailRuntimeValue::Known(0),
                    },
                    hive_components: None,
                    notification_phase: GameplayNotificationPhase::Playing,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick: tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            for outcome in &pass.outcomes {
                assert_eq!(outcome.stopped_actor(), None, "world{world} tick{tick}");
            }
        }
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some((world - 12) as usize));
        let (mut controller, _) = MainBaseAbortControllerStorage::from_loaded_level(
            NonZeroU64::new(u64::from(world)).unwrap(),
            session.cache.level_desc().unwrap(),
            false,
            &mut progress,
        )
        .unwrap();
        let result = execute_campaign_abort(
            MainBaseAbortTransactionId::new(u64::from(world)).unwrap(),
            None,
            &mut controller,
            &mut manager,
            &mut session.cache,
            &mut static_damage,
            &mut hull,
            &mut fx,
            &mut tasks,
            MainBaseAbortGameplayContext {
                extra_lives: RetailRuntimeValue::Known(0),
                notifications: &mut notifications,
                retail_tick: 1_101,
            },
        );
        let report = result.unwrap_or_else(|failure| {
            panic!(
                "world{world}: mid-play abort fell back: {:?}",
                failure.diagnostic
            )
        });
        assert!(report.completion.full_frame_submitted, "world{world}");
    }
}
