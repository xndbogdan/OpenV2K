use v2k_game::{
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    hive_controller::AuthoredHiveComponentFrame,
};

use v2k_formats::levels::EntityAnimation;
use v2k_game::actor_task_dispatcher::ActorTaskRuntime;
use v2k_game::actor_task_owner::ActorTaskSlot;
use v2k_game::entity::{
    EntityConstructionResources, EntityManager, MainBaseAbortAlternateCleanupAdvance,
    MainBaseAbortAlternateCleanupOutcome,
};
use v2k_game::entity_behavior::behavior_program;
use v2k_game::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use v2k_game::entity_emitters::{
    fun_00425ea0_sub_k_word, fun_004260f0_sub_k_word, AuthoredHiveComponentEffects,
    AuthoredRadialEmission, AuthoredRadialEmitterController, ComponentUpdateMode,
    HIVE_CONTROLLER_DEAD, HIVE_DEATH_SUCTION_DELAY_US, HIVE_SUB_K_DYING_OUTPUT_CAP,
};
use v2k_game::hive_death::HiveRadialTaskState;
use v2k_game::infection_evolution::{InfectionEvolutionEffects, InfectionTailSound};
use v2k_game::main_base_abort::{
    MainBaseAbortAction, MainBaseAbortActorRoute, MainBaseAbortMachine, MainBaseAbortPhase,
    MainBaseAbortPoll, MainBaseAbortResume, MainBaseAbortTransactionId,
    MainBaseAbortWorldControlLease,
};
use v2k_game::main_base_abort_world_effects::{
    snapshot_main_base_abort_terrain_geometry, MainBaseAbortTerrainAttributePublication,
    MainBaseAbortWorldEffects,
};
use v2k_game::session::GameSession;
use v2k_game::static_damage::StaticDamageScheduler;
use v2k_game::world_complete_results::FUN_0042E210_CAPABILITY_0X800;

#[derive(Default)]
struct RecordingEffects {
    emissions: Vec<AuthoredRadialEmission>,
    sounds: Vec<InfectionTailSound>,
    detailed_sounds: Vec<(u16, [i16; 3])>,
}

impl InfectionEvolutionEffects for RecordingEffects {
    fn next_shared_random_u16(&mut self) -> u16 {
        0
    }

    fn queue_infection_tail_sound(&mut self, sound: InfectionTailSound) {
        self.sounds.push(sound);
    }
}

impl AuthoredHiveComponentEffects for RecordingEffects {
    fn queue_hive_detailed_sound(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        self.detailed_sounds.push((sound_id, position_raw));
    }

    fn emit_authored_radial(&mut self, emission: AuthoredRadialEmission) {
        self.emissions.push(emission);
    }
}

#[test]
fn live_sub_k_word_matches_fun_00425ea0() {
    assert_eq!(fun_00425ea0_sub_k_word(0), 0x1000);
    assert_eq!(fun_00425ea0_sub_k_word(0x4000), 0x1fff);
}

#[test]
fn dying_sub_k_word_caps_at_0xd000() {
    assert_eq!(fun_004260f0_sub_k_word(0x1000, 0x100), 0x1001);
    assert_eq!(
        fun_004260f0_sub_k_word(HIVE_SUB_K_DYING_OUTPUT_CAP - 1, 0x200),
        HIVE_SUB_K_DYING_OUTPUT_CAP
    );
    assert_eq!(
        fun_004260f0_sub_k_word(HIVE_SUB_K_DYING_OUTPUT_CAP, 0x80_0000),
        HIVE_SUB_K_DYING_OUTPUT_CAP,
        "the completed ramp must not wrap back below the cap"
    );
    assert_eq!(
        fun_004260f0_sub_k_word(HIVE_SUB_K_DYING_OUTPUT_CAP + 1, 0x100),
        HIVE_SUB_K_DYING_OUTPUT_CAP + 1,
        "the native guard preserves any already-above-cap word"
    );
}

fn hive_animation(words: [u32; 6]) -> EntityAnimation {
    let mut header = [0; 0x18];
    for (index, word) in words.into_iter().enumerate() {
        header[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    EntityAnimation {
        header,
        frames: Vec::new(),
    }
}

#[test]
fn type_67_sub_k_selector_one_publishes_into_anim_vars_dynamic() {
    let mut controller = AuthoredRadialEmitterController::from_authored_spawn(
        behavior_program(46).expect("Alien Hive behavior"),
        Some([0, 0, 0, 0, 0x80, 0x01, 0x90, 0x01, 0x80, 0x01]),
        Some(&hive_animation([50_000, 5_000, 5, 5, 0, 0x1234_5678])),
        Some([1, 0]),
    )
    .expect("authored emitter component");
    assert_eq!(controller.sub_k_word_selector(), 1);
    let mut vars = v2k_formats::models::AnimVars::default();
    controller.publish_sub_k_word(&mut vars);
    assert_eq!(vars.dynamic[1], 0);
    controller.advance_sub_k(32, ComponentUpdateMode::Detailed);
    controller.publish_sub_k_word(&mut vars);
    assert_eq!(vars.dynamic[1], i32::from(fun_00425ea0_sub_k_word(1)));
    assert!(vars.dynamic[0] == 0 && vars.dynamic[2..].iter().all(|&value| value == 0));
}

fn animation_words(animation: &EntityAnimation) -> [u32; 6] {
    std::array::from_fn(|index| {
        u32::from_le_bytes(
            animation.header[index * 4..index * 4 + 4]
                .try_into()
                .expect("animation dword"),
        )
    })
}

#[v2k_test_support::retail_test]
fn intro2_spawn_24_installs_and_runs_the_authored_hive_component() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 0).expect("load L3 aux");
    session.load_level_by_id(50, 0).expect("load Intro2");

    let type_models = session.cache.global_entity_model_table();
    let type_metadata: Vec<_> = type_models
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
        .collect();
    assert_eq!(
        match type_metadata[67].common_mover_gkl_payloads {
            RetailRuntimeValue::Known(payloads) => payloads.sub_k,
            _ => None,
        },
        Some([1, 0]),
        "type 67 authors Sub-K [1, 0] for FUN_00425EA0 / FUN_004260F0",
    );
    assert_eq!(
        type_metadata[67].sub_n_payload,
        Some([0, 0, 0, 0, 0x80, 0x01, 0x90, 0x01, 0x80, 0x01]),
    );

    let level = session.cache.level_desc().expect("Intro2 Section 13");
    let hive_spawn = &level.entities[24];
    assert_eq!(hive_spawn.index, 24);
    assert_eq!(hive_spawn.entity_type, 67);
    let animation = hive_spawn.animation.as_ref().expect("hive animation");
    let animation_words = animation_words(animation);
    assert_eq!(animation_words[..5], [50_000, 5_000, 5, 5, 0]);

    let mut manager = EntityManager::from_level_with_type_metadata(
        level,
        &type_metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    assert_eq!(
        manager
            .iter_all()
            .filter(|entity| entity.authored_radial_emitter.is_some())
            .map(|entity| entity.authored_spawn_index)
            .collect::<Vec<_>>(),
        vec![Some(24)],
        "Intro2 currently has exactly one retained authored emitter; if the corpus changes, operation-2 ownership must be classified per component",
    );
    let hive_id = {
        let hive = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(24))
            .expect("hive allocation");
        let emitter = hive
            .authored_radial_emitter
            .as_ref()
            .expect("retained hive component");
        assert_eq!(hive.position_raw(), [-17_920, -640, 32_512]);
        assert_eq!(emitter.interval_us(), 50_000);
        assert_eq!(emitter.attachment_raw(), [384, 400, 384]);
        assert_eq!(emitter.sub_k_word_selector(), 1);
        assert_eq!(emitter.sub_k_output(), 0);
        let spawn_vars = hive.presentation_anim_vars(0);
        assert_eq!(spawn_vars.dynamic[1], 0);
        assert!(spawn_vars.dynamic.iter().all(|&value| value == 0));
        assert_eq!(
            hive.actor_task_state(ActorTaskSlot::Primary),
            Some(&ActorTaskRuntime::HiveRadial(HiveRadialTaskState::live())),
            "FUN_00425760 FUN_00401020-publishes slot 0 with tick FUN_00425EA0",
        );
        assert_eq!(hive.actor_task_state(ActorTaskSlot::Secondary), None);
        assert_eq!(hive.actor_task_state(ActorTaskSlot::Tertiary), None);
        hive.id
    };

    manager.disable_authored_behavior_components();
    let mut effects = RecordingEffects::default();
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 100_000,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("Intro2 objective state and terrain resolve");
    assert!(effects.emissions.is_empty());

    manager.set_authored_behavior_components_enabled(24, true);
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 50_000,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("Intro2 objective state and terrain resolve");
    assert!(
        effects.emissions.is_empty(),
        "strict equality must not allocate"
    );
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 1,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("Intro2 objective state and terrain resolve");
    assert_eq!(effects.emissions.len(), 1);
    assert_eq!(effects.emissions[0].position_raw, [-17_536, -240, -32_640]);
    assert_eq!(effects.emissions[0].source_id, hive_id);
    assert_eq!(effects.emissions[0].particle_class, 5);
    let hive = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("hive allocation");
    let emitter = hive
        .authored_radial_emitter
        .as_ref()
        .expect("retained hive component");
    let vars = hive.presentation_anim_vars(0);
    assert_eq!(vars.dynamic[1], i32::from(emitter.sub_k_output()));
    assert_ne!(vars.dynamic[1], 0);
    assert!(vars.dynamic[0] == 0 && vars.dynamic[2..].iter().all(|&value| value == 0));
}

#[v2k_test_support::retail_test]
fn level_1_hive_uses_its_slower_authored_gameplay_cadence() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("load L3 aux");
    session.load_level_by_id(13, 1).expect("load Level 1");

    let type_models = session.cache.global_entity_model_table();
    let type_metadata: Vec<_> = type_models
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
        .collect();
    for entity_type in [46_usize, 51, 88] {
        let RetailRuntimeValue::Known(topology) = type_metadata[entity_type].common_mover_topology
        else {
            panic!("Level-1 type {entity_type} topology must be decoded")
        };
        assert!(!topology.sub_n, "Level-1 type {entity_type} has no Sub-N");
        assert_eq!(type_metadata[entity_type].sub_n_payload, None);
    }

    let level = session.cache.level_desc().expect("Level 1 Section 13");
    let hive_spawn = level
        .entities
        .iter()
        .find(|spawn| spawn.entity_type == 67)
        .expect("Level 1 type-67 hive");
    assert_eq!(hive_spawn.index, 24);
    let animation = hive_spawn.animation.as_ref().expect("hive animation");
    assert_eq!(
        animation_words(animation)[..5],
        [80_000, 5_000, 5, 5, 1],
        "Level 1 authors its gameplay plume cadence and terrain-virus controller words",
    );

    let mut manager = EntityManager::from_level_with_type_metadata(
        level,
        &type_metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let hive = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(hive_spawn.index))
        .expect("Level 1 hive allocation");
    let hive_id = hive.id;
    let emitter = hive
        .authored_radial_emitter
        .as_ref()
        .expect("retained hive component");
    assert_eq!(hive.position_raw(), [-17_920, -256, -32_768]);
    assert_eq!(emitter.interval_us(), 80_000);
    assert_eq!(emitter.attachment_raw(), [384, 400, 384]);
    assert_eq!(emitter.sub_k_word_selector(), 1);
    assert_eq!(hive.presentation_anim_vars(0).dynamic[1], 0);
    let load_census = manager.fun_0042e210_capability_census();
    assert_eq!(
        load_census.cap_0x800, 6,
        "FUN_0042E570 counts Level-1 type-9 capability 0x800"
    );
    assert_eq!(
        manager
            .iter_all()
            .filter(|entity| entity.entity_type == 9
                && entity.capability_flags & FUN_0042E210_CAPABILITY_0X800 != 0)
            .count(),
        6
    );
    assert_eq!(
        hive.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::HiveRadial(HiveRadialTaskState::live())),
        "FUN_00425760 FUN_00401020-publishes slot 0 with tick FUN_00425EA0",
    );
    assert_eq!(hive.actor_task_state(ActorTaskSlot::Secondary), None);
    assert_eq!(hive.actor_task_state(ActorTaskSlot::Tertiary), None);
    let RetailRuntimeValue::Known(Some(sub_n_runtime)) = hive.sub_n_runtime else {
        panic!("Level-1 hive must retain its loader-owned Sub-N allocation")
    };
    assert_eq!(
        sub_n_runtime.terrain_patch_present(),
        RetailRuntimeValue::Known(true)
    );
    assert_eq!(sub_n_runtime.cleanup_timer_us(), 0);
    assert_eq!(
        sub_n_runtime.anchor_raw(),
        RetailRuntimeValue::Known([-17_536, 0, -32_384]),
        "FUN_0041BC20 runs before later terrain-Y normalization"
    );
    let terrain_geometry = snapshot_main_base_abort_terrain_geometry(&session.cache)
        .expect("complete current-level Section 10 geometry");
    let terrain = session.cache.terrain().expect("Level-1 Section 10");
    let terrain_objects = session.cache.terrain_objects().expect("Level-1 Section 9");
    let footprint = [
        [186_u8, 128_u8],
        [186, 129],
        [186, 130],
        [187, 128],
        [187, 129],
        [187, 130],
        [188, 128],
        [188, 129],
        [188, 130],
    ];
    let attributes =
        footprint.map(|[x, z]| terrain.cells[usize::from(x) * 256 + usize::from(z)].attribute);
    assert_eq!(attributes, [0, 0, 0, 0, 255, 0, 0, 0, 0]);
    assert_eq!(terrain_objects.records[255].kind_index, 22);

    let hive_actor = manager
        .main_base_abort_actor_observation(hive_id)
        .expect("live Level-1 hive actor");
    let mut machine = MainBaseAbortMachine::start(
        MainBaseAbortTransactionId::new(1).unwrap(),
        RetailRuntimeValue::Known(Some(MainBaseAbortWorldControlLease {
            allocation_identity: 1,
        })),
        RetailRuntimeValue::Known(Some(hive_actor)),
    );
    let issued = match machine.poll() {
        MainBaseAbortPoll::Action(issued) => issued,
        other => panic!("expected spawn-24 alternate action, got {other:?}"),
    };
    let MainBaseAbortAction::ProcessActor { actor, route, .. } = issued.action else {
        panic!("expected hive actor action")
    };
    assert_eq!(
        route,
        MainBaseAbortActorRoute::AlternateCleanup { entity_id: hive_id }
    );
    let mut static_damage = StaticDamageScheduler::new();
    let (cleanup_advance, cleanup_report) = {
        let mut cleanup_effects = MainBaseAbortWorldEffects::new(
            &terrain_geometry,
            &mut session.cache,
            &mut static_damage,
        )
        .expect("complete shared Main Base world-effects custody");
        let advance = manager
            .apply_main_base_abort_alternate_cleanup(actor.lease, &mut cleanup_effects)
            .expect("corpus-authenticated Hive cleanup");
        (advance, cleanup_effects.finish())
    };
    let MainBaseAbortAlternateCleanupAdvance::Advanced {
        outcome:
            MainBaseAbortAlternateCleanupOutcome::Applied {
                entity_id,
                runtime,
                terrain_publications,
            },
        next_actor,
    } = cleanup_advance
    else {
        panic!("spawn 24 remains linked through callback return")
    };
    assert_eq!(entity_id, hive_id);
    assert_eq!(terrain_publications, 9);
    assert_eq!(
        runtime.cleanup_timer_us(),
        v2k_game::sub_n_runtime::MAIN_BASE_ABORT_SUB_N_TIMER_US
    );
    assert_eq!(
        cleanup_report
            .terrain_attribute_publications()
            .iter()
            .map(|publication| (publication.cell, publication.old_attribute))
            .collect::<Vec<_>>(),
        footprint.into_iter().zip(attributes).collect::<Vec<_>>()
    );
    assert_eq!(
        cleanup_report.terrain_attribute_publications()[4],
        MainBaseAbortTerrainAttributePublication {
            cell: [187, 129],
            old_attribute: 255,
        }
    );
    assert!(cleanup_report.static_radial_outcomes().is_empty());
    assert!(cleanup_report.terrain_dirty());
    assert_eq!(
        session
            .cache
            .level()
            .unwrap()
            .terrain
            .as_ref()
            .unwrap()
            .cell(187, 129)
            .unwrap()
            .attribute,
        0,
        "the production-shaped adapter must mutate the authoritative live terrain"
    );
    assert_eq!(
        next_actor.map(|next| (next.lease.entity_id, next.entity_type)),
        Some((hive_id + 1, 62))
    );
    assert_eq!(
        manager
            .iter_all()
            .find(|entity| entity.id == hive_id + 1)
            .and_then(|entity| entity.authored_spawn_index),
        Some(25)
    );
    machine
        .resume(
            issued.receipt,
            MainBaseAbortResume::ActorProcessed {
                phase: MainBaseAbortPhase::ActorSweep,
                actor: actor.lease,
                successor: RetailRuntimeValue::Known(next_actor),
            },
        )
        .unwrap();
    assert!(matches!(
        machine.poll(),
        MainBaseAbortPoll::Action(issued)
            if matches!(
                issued.action,
                MainBaseAbortAction::ProcessActor {
                    actor,
                    route: MainBaseAbortActorRoute::OrdinaryDeath { entity_id },
                    ..
                } if actor.lease.entity_id == hive_id + 1
                    && actor.entity_type == 62
                    && entity_id == hive_id + 1
            )
    ));

    let mut effects = RecordingEffects::default();
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 80_000,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("Level-1 objective state and terrain resolve");
    assert!(
        effects.emissions.is_empty(),
        "strict equality must not allocate"
    );
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 1,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("Level-1 objective state and terrain resolve");
    assert_eq!(effects.emissions.len(), 1);
    assert_eq!(effects.emissions[0].position_raw, [-17_536, 144, -32_384]);
    assert_eq!(effects.emissions[0].source_id, hive_id);
    assert_eq!(effects.emissions[0].particle_class, 5);
    let hive = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("Level 1 hive allocation");
    let emitter = hive
        .authored_radial_emitter
        .as_ref()
        .expect("retained hive component");
    let vars = hive.presentation_anim_vars(0);
    assert_eq!(vars.dynamic[1], i32::from(emitter.sub_k_output()));
    assert_ne!(vars.dynamic[1], 0);
    assert!(vars.dynamic[0] == 0 && vars.dynamic[2..].iter().all(|&value| value == 0));
}

#[v2k_test_support::retail_test]
fn level_1_dead_hive_stops_spit_and_arms_sub_n_suction() {
    let dir = v2k_test_support::retail_dir();

    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("load L3 aux");
    session.load_level_by_id(13, 1).expect("load Level 1");

    let type_models = session.cache.global_entity_model_table();
    let type_metadata: Vec<_> = type_models
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
        .collect();
    let level = session.cache.level_desc().expect("Level 1 Section 13");
    let mut manager = EntityManager::from_level_with_type_metadata(
        level,
        &type_metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
    );
    let hive_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .expect("Level 1 hive")
        .id;
    {
        let hive = manager
            .entity_mut(hive_id)
            .expect("Level 1 hive remains allocated");
        match hive.sub_n_runtime {
            RetailRuntimeValue::Known(Some(runtime)) => {
                assert_eq!(
                    runtime.terrain_patch_present(),
                    RetailRuntimeValue::Known(true)
                );
            }
            _ => panic!("Level-1 hive must retain Sub-N"),
        }
        let emitter = hive
            .authored_radial_emitter
            .as_mut()
            .expect("retained hive component");
        emitter.enter_dying_slot0();
        emitter.arm_wreck_suction();
        assert_eq!(emitter.controller_state(), HIVE_CONTROLLER_DEAD);
        assert_eq!(emitter.suction_timer_us(), HIVE_DEATH_SUCTION_DELAY_US);
        assert!(!emitter.wreck_suction_ready());
    }

    let mut effects = RecordingEffects::default();
    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 80_001,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("Level-1 objective state and terrain resolve");
    assert!(
        effects.emissions.is_empty(),
        "dead controller state 0 must stop class-5 spit"
    );

    manager
        .advance_authored_hive_components(
            AuthoredHiveComponentFrame {
                elapsed_us: 6_000_000,
                terrain: session.cache.terrain(),
                retail_tick: 0,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                notifications: &mut GameplayNotifications::new(),
                world_complete_tally:
                    &mut v2k_game::world_complete_results::WorldCompleteTally::default(),
            },
            |_| ComponentUpdateMode::Detailed,
            &mut effects,
        )
        .expect("dead wreck still ticks Sub-K on detailed updates");
    let hive = manager
        .iter_all()
        .find(|entity| entity.id == hive_id)
        .expect("Level 1 hive remains allocated");
    let emitter = hive
        .authored_radial_emitter
        .as_ref()
        .expect("retained hive component");
    let expected = fun_004260f0_sub_k_word(0, 80_001);
    let expected = fun_004260f0_sub_k_word(expected, 6_000_000);
    assert_eq!(emitter.sub_k_output(), expected);
    assert_eq!(
        hive.presentation_anim_vars(0).dynamic[1],
        i32::from(expected),
        "dying Sub-K must publish into the wreck's draw vars"
    );
    assert!(
        expected > 0x5000,
        "six detailed seconds must visibly open the mouth"
    );
    assert!(
        emitter.wreck_suction_ready(),
        "the -6_000_000 suction delay must elapse with the Sub-K ramp"
    );
}

struct GroundYAlias;

impl v2k_formats::models::ModelVertexResolver for GroundYAlias {
    fn resolve_vertex_raw(
        &self,
        kind: v2k_formats::models::ModelVertexKind,
        source: v2k_formats::models::ResolvedModelSlot,
    ) -> Option<v2k_formats::models::ResolvedModelSlot> {
        let source_raw = source.position_raw;
        use v2k_formats::models::{ModelVertexKind, ResolvedModelSlot};
        Some(ResolvedModelSlot::clear(match kind {
            ModelVertexKind::Alias => [source_raw[0], 0.0, source_raw[2]],
            ModelVertexKind::ViewPin => source_raw,
        }))
    }
}

fn max_vertex_delta(closed: &[[f64; 3]], opened: &[[f64; 3]]) -> f64 {
    closed
        .iter()
        .zip(opened)
        .map(|(from, to)| {
            let dx = from[0] - to[0];
            let dy = from[1] - to[1];
            let dz = from[2] - to[2];
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .fold(0.0_f64, f64::max)
}

#[v2k_test_support::retail_test]
fn dead_hive_flaps_follow_sub_k_dynamic_word() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("init session");
    session.load_auxiliary_ovl(3, 1).expect("load L3 aux");
    session.load_level_by_id(13, 1).expect("load Level 1");

    let live = session.cache.global_model(341).expect("hive1xa");
    let live_body = session.cache.global_model(342).expect("hive1xa2");
    let dead = session.cache.global_model(343).expect("dhive1xa");
    assert_eq!(live.name.as_deref(), Some("hive1xa"));
    assert_eq!(dead.name.as_deref(), Some("dhive1xa"));
    assert!(
        live_body.records.iter().all(|record| record[0] != 8),
        "live hive1xa2 is a closed dome; Sub-K mouth morph is dying-only"
    );
    assert_eq!(
        live.materialize(&v2k_formats::models::AnimVars::default())
            .instances[0]
            .model_id,
        342
    );

    let dead_body = session.cache.global_model(344).expect("dhive1xa2");
    let body = dead_body.materialize(&v2k_formats::models::AnimVars::default());
    assert_eq!(body.instances.len(), 4);
    assert!(body.instances.iter().all(|inst| inst.model_id == 345));
    assert!(
        body.instances
            .iter()
            .all(|inst| inst.attach_pos == Some([0.0, 0.0, 0.0])),
        "four dh1flap petals share the wreck origin"
    );
    let mut open_vars = v2k_formats::models::AnimVars::default();
    open_vars.dynamic[1] = i32::from(HIVE_SUB_K_DYING_OUTPUT_CAP);
    let opened_body = dead_body.materialize(&open_vars);
    assert!(
        body.instances
            .iter()
            .zip(&opened_body.instances)
            .all(|(closed_inst, opened_inst)| {
                closed_inst.orientation == opened_inst.orientation
                    && closed_inst.attach_pos == opened_inst.attach_pos
                    && closed_inst.registers[..4] == opened_inst.registers[..4]
            }),
        "wreck flap instances are not themselves Sub-K animated"
    );
    assert_eq!(
        max_vertex_delta(&body.vertices, &opened_body.vertices),
        0.0,
        "dhive1xa2 dish is static; only dh1flap tf-8 verts follow Sub-K"
    );

    let flap = session.cache.global_model(345).expect("dh1flap");
    let closed = flap.materialize(&v2k_formats::models::AnimVars::default());
    let mut half_open = v2k_formats::models::AnimVars::default();
    half_open.dynamic[1] = 0x4000;
    let opened = flap.materialize(&half_open);
    let ramped = flap.materialize(&open_vars);
    let moved = closed
        .vertices
        .iter()
        .zip(&opened.vertices)
        .filter(|(from, to)| from != to)
        .count();
    assert_eq!(closed.billboards.len(), 0);
    assert_eq!(moved, 12, "Sub-K dynamic[1] morphs the wreck flaps open");
    assert!(
        flap.records
            .iter()
            .any(|record| record[0] == 8 && record[1] == 129),
        "tf-8 lerp operand 0x81 reads AnimVars.dynamic[1]"
    );
    let alias = GroundYAlias;
    let opened_alias =
        flap.materialize_with_context(v2k_formats::models::ModelMaterializationContext {
            vars: &open_vars,
            linked: None,
            vertex_resolver: Some(&alias),
            view_selection: v2k_formats::models::ModelViewSelection::IntrinsicAllBranches,
        });
    assert_eq!(
        max_vertex_delta(&ramped.vertices, &opened_alias.vertices),
        0.0,
        "world type-12 aliases of already-grounded Y=0 endpoints must not flatten the morph"
    );

    let closed_tree = hive_tree_vertices(&session, 343, 0);
    let opened_tree = hive_tree_vertices(&session, 343, 0x4000);
    let ramped_tree = hive_tree_vertices(&session, 343, i32::from(HIVE_SUB_K_DYING_OUTPUT_CAP));
    assert_ne!(
        closed_tree, opened_tree,
        "inherited child vars must still morph dh1flap in the wreck tree"
    );
    assert_ne!(opened_tree, ramped_tree);
    let max_delta = max_vertex_delta(&closed.vertices, &opened.vertices);
    assert!(
        max_delta > 50.0,
        "flap morph at dynamic[1]=0x4000 must be visible, max_delta={max_delta}"
    );
    assert!(
        max_vertex_delta(&closed.vertices, &ramped.vertices) > 500.0,
        "dying cap 0xD000 must fold the petals down to the dish rim"
    );
}

fn hive_tree_vertices(session: &GameSession, model_id: usize, dynamic1: i32) -> Vec<[i32; 3]> {
    fn walk(
        session: &GameSession,
        model_id: usize,
        vars: &v2k_formats::models::AnimVars,
        depth: u8,
        out: &mut Vec<[i32; 3]>,
    ) {
        let Some(model) = session.cache.global_model(model_id) else {
            return;
        };
        let materialized = model.materialize(vars);
        out.extend(materialized.vertices.iter().map(|v| {
            [
                v[0].round() as i32,
                v[1].round() as i32,
                v[2].round() as i32,
            ]
        }));
        if depth == 0 {
            return;
        }
        for instance in &materialized.instances {
            let mut child_vars = vars.clone();
            child_vars.registers.fill(0);
            child_vars.registers[..4].copy_from_slice(&instance.registers[..4]);
            walk(
                session,
                usize::from(instance.model_id),
                &child_vars,
                depth - 1,
                out,
            );
        }
    }
    let mut vars = v2k_formats::models::AnimVars::default();
    vars.dynamic[1] = dynamic1;
    let mut out = Vec::new();
    walk(session, model_id, &vars, 8, &mut out);
    out
}
