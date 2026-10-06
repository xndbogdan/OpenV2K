//! 4519BE post-load admission followed by the real Section-2 command stream.

use v2k_game::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::{
        DynamicRadialLiveRequest, Entity, EntityConstructionResources, EntityManager,
        Intro2BirthSelection,
    },
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    gameplay_notifications::GameplayNotifications,
    intro2_commands::Intro2Commands,
    intro2_type17::{
        tick_intro2_type17, Intro2Type17Frame, Intro2Type17Outcome, Intro2Type17Owner,
    },
    intro2_type57::{
        tick_intro2_type57, Intro2Type57Frame, Intro2Type57Outcome, Intro2Type57Owner,
    },
    opening::{
        intro2_type52_class0_allocation_authenticates, intro2_type67_hive_allocation_authenticates,
        intro2_type77_dormant_allocation_authenticates, intro2_uses_live_actor_pose,
        intro_actor_heading, intro_actor_pose, intro_actor_visible, INTRO2_DURATION_SECS,
    },
    radial_damage::RadialDamageTemplate,
    session::GameSession,
    world_fx::WorldFx,
};

const ACTIVATION: u32 = 0x68000;

fn fixture() -> (GameSession, EntityManager) {
    let path = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&path).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(50, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, slots)| {
            session
                .cache
                .global_entity_type(kind)
                .map(EntityTypeRuntimeMetadata::from_section12)
                .unwrap_or(EntityTypeRuntimeMetadata {
                    model_slots: *slots,
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    let manager = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        Intro2BirthSelection::default(),
        &mut || 1,
    )
    .unwrap();
    (session, manager)
}

fn actor(manager: &EntityManager, spawn: usize) -> &Entity {
    manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(spawn))
        .unwrap()
}

fn graph(entity: &Entity) -> String {
    format!(
        "{:?}",
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot))
    )
}

#[v2k_test_support::retail_test]
fn first_insects_start_at_retail_sub_c_clearance_before_activation() {
    let (session, captured) = fixture();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect::<Vec<_>>();
    let mut fx = WorldFx::new();
    let native = EntityManager::from_native_intro2_frontend(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        4793,
        &mut fx,
    )
    .unwrap();
    // Births retained identically by20260712-203635-full-session and
    //20260722-022303-intro2-actor-ai.40D691 adds Sub-C before +90 is copied.
    for mut manager in [captured, native] {
        manager.disable_authored_behavior_components();
        for (spawn, expected) in [
            (4, [-29696, 11, -28672]),
            (5, [-27648, 491, -29184]),
            (20, [-28160, 242, -28928]),
            (38, [-28672, 178, -28928]),
        ] {
            assert_eq!(
                actor(&manager, spawn).position_raw(),
                expected,
                "spawn{spawn}"
            );
            assert_eq!(
                actor(&manager, spawn)
                    .collision
                    .state_flags_at_0x08
                    .masked(ACTIVATION),
                RetailRuntimeValue::Known(0)
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn captured_intro2_retains_explicit_type122_presentation_fallback() {
    let (_, manager) = fixture();
    let mut fallbacks = Vec::new();
    for entity in manager.iter_all() {
        let Some(spawn) = entity.authored_spawn_index else {
            continue;
        };
        let live = intro2_uses_live_actor_pose(entity);
        match entity.entity_type {
            52 => {
                assert!(intro2_type52_class0_allocation_authenticates(entity));
                assert!(live, "Type52 spawn {spawn} publishes class-0 live pose");
                assert!(!intro_actor_visible(spawn));
            }
            67 => {
                assert!(intro2_type67_hive_allocation_authenticates(entity));
                assert!(
                    live,
                    "Type67 hive spawn {spawn} follows the live emitter pose"
                );
                assert_eq!(spawn, 24);
            }
            115 => {
                assert!(entity.intro2_gun_turret_runtime.is_some());
                assert!(live, "Type115 spawn {spawn} keeps slot-2 virusedsunflower");
                assert_eq!(entity.model_index, Some(332));
                assert_eq!(spawn, 61);
            }
            122 => {
                assert!(!live, "captured Type122 has no native construction receipt");
                assert_eq!(spawn, 21);
                fallbacks.push(spawn);
            }
            _ => {
                assert!(
                    live,
                    "spawn {spawn} type {} authenticates a native owner and must not use intro_actor_pose",
                    entity.entity_type
                );
            }
        }
    }
    assert_eq!(
        fallbacks,
        [21],
        "the explicit captured entry keeps Type122 spawn21 as a fallback"
    );
}

#[v2k_test_support::retail_test]
fn native_type122_live_pose_waits_for_authored_activation_without_replacing_tasks() {
    let (session, _) = fixture();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect::<Vec<_>>();
    let mut fx = WorldFx::new();
    let mut manager = EntityManager::from_native_intro2_frontend(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&|id| session.cache.global_model(id).map(|model| model.radius)),
        },
        4793,
        &mut fx,
    )
    .unwrap();
    let before = actor(&manager, 21);
    assert!(intro2_uses_live_actor_pose(before));
    let (position, tasks, receipt) = (
        before.position_raw(),
        graph(before),
        before.native_type122_runtime,
    );
    manager.disable_authored_behavior_components();
    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    commands.present(&mut manager, 324);
    assert_eq!(
        actor(&manager, 21)
            .collision
            .state_flags_at_0x08
            .masked(ACTIVATION),
        RetailRuntimeValue::Known(0)
    );
    commands.present(&mut manager, 325);
    let after = actor(&manager, 21);
    assert_eq!(
        after.collision.state_flags_at_0x08.masked(ACTIVATION),
        RetailRuntimeValue::Known(ACTIVATION)
    );
    assert_eq!(
        (
            after.position_raw(),
            graph(after),
            after.native_type122_runtime
        ),
        (position, tasks, receipt)
    );
    assert!(intro2_uses_live_actor_pose(after));
}

#[v2k_test_support::retail_test]
fn post_load_disables_native_insects_and_meteors_without_replacing_birth_or_visibility() {
    let (_, mut manager) = fixture();
    assert_eq!(
        actor(&manager, 61).capability_flags,
        8,
        "the infected flower's Gun Turret constructor replaces authored capability0"
    );
    let before = manager
        .iter_all()
        .map(|entity| {
            (
                entity.id,
                entity.collision.state_flags_at_0x08,
                entity.position_raw(),
                entity.velocity_raw(),
                entity.physical_body_basis_q31(),
                entity.model_slots,
                entity.collision.active_model_slot(),
                entity.collision.health_raw,
                entity.current_behavior_context,
                graph(entity),
            )
        })
        .collect::<Vec<_>>();
    manager.disable_authored_behavior_components();

    // Canonical native capability-8 cohorts and the capability-zero meteors.
    for spawn in [
        0, 1, 4, 5, 6, 7, 8, 10, 20, 24, 25, 26, 30, 31, 33, 34, 35, 38, 40, 41, 42, 43, 44, 45,
        46, 55, 56, 61,
    ] {
        assert_eq!(
            actor(&manager, spawn)
                .collision
                .state_flags_at_0x08
                .masked(ACTIVATION),
            RetailRuntimeValue::Known(0),
            "spawn {spawn} must wait for its authored command"
        );
    }
    for (id, old_state, position, velocity, basis, models, slot, health, context, tasks) in before {
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let state = entity.collision.state_flags_at_0x08;
        assert_eq!(
            state.known_value_bits() & !ACTIVATION,
            old_state.known_value_bits() & !ACTIVATION
        );
        assert_eq!(
            state.known_mask() & !ACTIVATION,
            old_state.known_mask() & !ACTIVATION
        );
        assert_eq!(entity.position_raw(), position);
        assert_eq!(entity.velocity_raw(), velocity);
        assert_eq!(entity.physical_body_basis_q31(), basis);
        assert_eq!(entity.model_slots, models);
        assert_eq!(entity.collision.active_model_slot(), slot);
        assert_eq!(entity.collision.health_raw, health);
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(graph(entity), tasks);
    }
    for spawn in [2, 3, 12, 36, 51, 53, 54] {
        assert_ne!(
            actor(&manager, spawn)
                .collision
                .state_flags_at_0x08
                .masked(ACTIVATION),
            RetailRuntimeValue::Known(0),
            "resident/civilian/factory/turret spawn {spawn} is not blanket-disabled"
        );
    }
    assert!(!actor(&manager, 24)
        .authored_radial_emitter
        .as_ref()
        .unwrap()
        .behavior_enabled());
}

#[v2k_test_support::retail_test]
fn post_load_reads_instance_capability_and_type34_instead_of_a_scene_whitelist() {
    let (_, mut manager) = fixture();
    let peasant = actor(&manager, 2).id;
    let insect = actor(&manager, 4).id;
    let meteor = actor(&manager, 33).id;
    for id in [peasant, insect, meteor] {
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(ACTIVATION, ACTIVATION);
    }
    // 4519CC reads the live instance +64, not its type descriptor.
    manager.entity_mut(peasant).unwrap().capability_flags |= 8;
    manager.entity_mut(insect).unwrap().capability_flags &= !8;
    assert_eq!(manager.entity_mut(meteor).unwrap().capability_flags, 0);
    manager.disable_authored_behavior_components();
    assert_eq!(
        actor(&manager, 2)
            .collision
            .state_flags_at_0x08
            .masked(ACTIVATION),
        RetailRuntimeValue::Known(0)
    );
    assert_eq!(
        actor(&manager, 4)
            .collision
            .state_flags_at_0x08
            .masked(ACTIVATION),
        RetailRuntimeValue::Known(ACTIVATION)
    );
    assert_eq!(
        actor(&manager, 33)
            .collision
            .state_flags_at_0x08
            .masked(ACTIVATION),
        RetailRuntimeValue::Known(0)
    );
}

#[v2k_test_support::retail_test]
fn canonical_commands_release_native_tasks_after_the_meteors_without_a_visibility_spawn() {
    let (session, mut manager) = fixture();
    manager.disable_authored_behavior_components();
    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    // Each command writes during presentation; the next actor visit consumes it.
    for (tick, spawns) in [
        (25, &[33][..]),
        (50, &[34][..]),
        (75, &[35][..]),
        (150, &[31][..]),
        (275, &[4, 5, 20, 38][..]),
        (325, &[21, 46][..]),
        (750, &[7, 8, 24, 26, 30, 41, 42, 43, 44][..]),
        (1100, &[0, 1, 6, 10, 25, 40, 61][..]),
        (2500, &[55, 56][..]),
    ] {
        commands.present(&mut manager, tick - 1);
        for &spawn in spawns {
            assert_eq!(
                actor(&manager, spawn)
                    .collision
                    .state_flags_at_0x08
                    .masked(ACTIVATION),
                RetailRuntimeValue::Known(0),
                "spawn {spawn}, tick {}",
                tick - 1
            );
        }
        commands.present(&mut manager, tick);
        for &spawn in spawns {
            assert_eq!(
                actor(&manager, spawn)
                    .collision
                    .state_flags_at_0x08
                    .masked(ACTIVATION),
                RetailRuntimeValue::Known(ACTIVATION),
                "spawn {spawn}, tick {tick}"
            );
        }
    }
    assert!(actor(&manager, 24)
        .authored_radial_emitter
        .as_ref()
        .unwrap()
        .behavior_enabled());

    // Reconstruct a real native Type17 owner and run every pre-release visit.
    manager.disable_authored_behavior_components();
    let id = actor(&manager, 4).id;
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x0200_0000, 0x0200_0000);
    let initial_position = actor(&manager, 4).position_raw();
    let initial_graph = graph(actor(&manager, 4));
    let mut owner = Intro2Type17Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    for tick in 0..275 {
        let result = tick_intro2_type17(
            &mut manager,
            owner,
            Intro2Type17Frame {
                capture_tasks: &mut v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
                notifications: &mut v2k_game::gameplay_notifications::GameplayNotifications::new(),
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: tick,
            },
        );
        assert!(
            matches!(
                result.outcome,
                Intro2Type17Outcome::Advanced {
                    callback_enabled: false,
                    ..
                }
            ),
            "tick {tick}: {:?}",
            result.outcome
        );
        owner = result.retained_owner.unwrap();
    }
    assert_eq!(actor(&manager, 4).position_raw(), initial_position);
    assert_eq!(graph(actor(&manager, 4)), initial_graph);
    commands.present(&mut manager, 275);
    let result = tick_intro2_type17(
        &mut manager,
        owner,
        Intro2Type17Frame {
            capture_tasks: &mut v2k_game::specialized_actor_task_production::SpecializedActorTaskScheduler::default(),
            notifications: &mut v2k_game::gameplay_notifications::GameplayNotifications::new(),
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 276,
        },
    );
    assert!(
        matches!(
            result.outcome,
            Intro2Type17Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{:?}",
        result.outcome
    );
    assert!(result.retained_owner.is_some());
    assert_ne!(
        graph(actor(&manager, 4)),
        initial_graph,
        "the retained task starts only after operation 2"
    );
}

#[v2k_test_support::retail_test]
fn dormant_native_insect_ignores_real_meteor_radial_until_its_authored_enable() {
    let (session, mut manager) = fixture();
    manager.disable_authored_behavior_components();
    let id = actor(&manager, 5).id;
    let position = actor(&manager, 5).position_raw();
    let health = actor(&manager, 5).collision.health_raw;
    let velocity = actor(&manager, 5).velocity_raw();
    // Keep the real Type16 allocation at its authored pose, isolating the
    // target by clearing other recipients' admission rather than moving it.
    for other in manager
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        if other != id {
            manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    // The Type34 terminal template, with a controlled nearby impact origin.
    let template = RadialDamageTemplate {
        inner_radius_raw: 512,
        outer_radius_raw: 1024,
        impulse_raw: 2000,
        packet: DamagePacket {
            channels: [1, 3],
            amounts_raw: [4000, 4000],
        },
        trailing_raw: [34, 0],
    };
    let origin = [position[0].wrapping_sub(64), position[1], position[2]];
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let mut callbacks = |_: &EntityManager, _: u32| true;
    let report = manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
        origin_raw: origin,
        template,
        world_fx: &mut fx,
        retail_tick: 274,
        notifications: &mut notifications,
        callbacks: &mut callbacks,
    });
    assert!(report.completed(), "{report:?}");
    assert_eq!(report.accepted_targets, 0);
    assert!(report.completed_target_ids.is_empty());
    assert_eq!(actor(&manager, 5).collision.health_raw, health);
    assert_eq!(actor(&manager, 5).velocity_raw(), velocity);

    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    commands.present(&mut manager, 275);
    for other in manager
        .iter_all()
        .map(|entity| entity.id)
        .collect::<Vec<_>>()
    {
        if other != id {
            manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x8000, 0);
        }
    }
    // The same physical request now reaches the native checked-damage path.
    let mut callbacks = |_: &EntityManager, _: u32| true;
    let report = manager.apply_dynamic_radial_damage_live(DynamicRadialLiveRequest {
        origin_raw: origin,
        template,
        world_fx: &mut fx,
        retail_tick: 276,
        notifications: &mut notifications,
        callbacks: &mut callbacks,
    });
    assert!(report.completed(), "{report:?}");
    assert_eq!(report.accepted_targets, 1);
    assert_eq!(report.completed_target_ids, [id]);
    // Type16 filters the channel1/3 packet to 2000 + 8000. No buffer is
    // present at this birth, so its authored 12000 health becomes 2000.
    assert_eq!(health, RetailRuntimeValue::Known(12_000));
    assert_eq!(
        actor(&manager, 5).collision.health_raw,
        RetailRuntimeValue::Known(2000)
    );
    assert_eq!(
        actor(&manager, 5).collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(0)
    );
    assert_ne!(actor(&manager, 5).velocity_raw(), velocity);
}

#[v2k_test_support::retail_test]
fn type77_spawn45_stays_at_authored_pose_and_never_receives_operation_2() {
    let (session, mut manager) = fixture();
    manager.disable_authored_behavior_components();
    let insect = actor(&manager, 45);
    assert_eq!(insect.entity_type, 77);
    assert!(intro2_type77_dormant_allocation_authenticates(insect));
    assert!(intro2_uses_live_actor_pose(insect));
    assert_eq!(
        insect.collision.state_flags_at_0x08.masked(ACTIVATION),
        RetailRuntimeValue::Known(0)
    );
    let authored = insect.position_raw();
    let base = insect.position;
    for elapsed in [0.0, 1.0, 5.5, 6.0, 22.0, INTRO2_DURATION_SECS] {
        assert_eq!(intro_actor_pose(45, base, elapsed).position, base);
        assert_eq!(intro_actor_heading(45), 0.0);
    }
    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    commands.present(&mut manager, 1100);
    commands.present(&mut manager, 2500);
    let insect = actor(&manager, 45);
    assert_eq!(
        insect.collision.state_flags_at_0x08.masked(ACTIVATION),
        RetailRuntimeValue::Known(0),
        "Type77 spawn 45 has no enable record"
    );
    assert_eq!(insect.position_raw(), authored);
}

#[v2k_test_support::retail_test]
fn intro2_type66_spawn51_keeps_authored_factory_slots_heading_and_grounded_y() {
    let (session, mut manager) = fixture();
    let factory = actor(&manager, 51);
    assert_eq!(factory.entity_type, 66);
    assert_eq!(
        factory.model_slots,
        [Some(210), Some(225), Some(210), Some(225)]
    );
    assert_eq!(factory.model_index, Some(210));
    assert_eq!(
        factory.rotation_heading_pitch_roll_raw(),
        [0x527D_u16 as i16, 0, 0]
    );
    let terrain = session.cache.terrain().unwrap();
    let [x, y, z] = factory.position_raw();
    assert_eq!([x, z], [20992, -5888]);
    assert_eq!(y, terrain.bilinear_height_raw(x, z));
    // Initializer 0x25027 retains bit 0x4000 on the default word. Live E870
    // therefore skips 13F70 so crater terrain alignment can stay authoritative;
    // constructor D720 still published the authored Euler basis.
    assert_eq!(
        factory.collision.default_state_flags_at_0xc8,
        RetailRuntimeValue::Known(0x25027)
    );
    assert_eq!(
        factory.physical_body_basis_q31(),
        RetailRuntimeValue::Known(
            v2k_game::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(
                0x527D_u16 as i16,
                0,
                0
            )
        )
    );
    assert!(intro2_uses_live_actor_pose(factory));
    manager.disable_authored_behavior_components();
    assert!(intro2_uses_live_actor_pose(actor(&manager, 51)));
}

#[v2k_test_support::retail_test]
fn type57_spawn1_enables_at_22000ms_and_keeps_class7_graph() {
    let (mut session, mut manager) = fixture();
    manager.disable_authored_behavior_components();
    let bat = actor(&manager, 1);
    assert_eq!(bat.entity_type, 57);
    assert!(intro2_uses_live_actor_pose(bat));
    assert_eq!(
        bat.collision.state_flags_at_0x08.masked(ACTIVATION),
        RetailRuntimeValue::Known(0)
    );
    let authored = bat.position_raw();
    let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
    let id = actor(&manager, 1).id;
    let mut fx = WorldFx::new();
    let dormant_owner = Intro2Type57Owner::adopt(&manager, id).unwrap();
    let dormant = tick_intro2_type57(
        &mut manager,
        dormant_owner,
        Intro2Type57Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            dormant.outcome,
            Intro2Type57Outcome::Advanced {
                callback_enabled: false,
                ..
            }
        ),
        "{:?}",
        dormant.outcome
    );
    let dormant_position = actor(&manager, 1).position_raw();
    commands.present(&mut manager, 1099);
    assert_eq!(
        actor(&manager, 1)
            .collision
            .state_flags_at_0x08
            .masked(ACTIVATION),
        RetailRuntimeValue::Known(0)
    );
    commands.present(&mut manager, 1100);
    assert_eq!(
        actor(&manager, 1)
            .collision
            .state_flags_at_0x08
            .masked(ACTIVATION),
        RetailRuntimeValue::Known(ACTIVATION)
    );
    let owner = Intro2Type57Owner::adopt(&manager, id).unwrap();
    let tick = tick_intro2_type57(
        &mut manager,
        owner,
        Intro2Type57Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1100,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type57Outcome::Advanced {
                callback_enabled: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    assert_eq!(
        dormant_position, authored,
        "dormant visits must not translate before operation 2"
    );
    assert_ne!(
        actor(&manager, 1).position_raw(),
        authored,
        "after enable the class7 mover is live, not a frozen pose proxy"
    );
}
