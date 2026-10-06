use super::*;
use crate::{
    actor_detailed_sound::ActorDetailedSoundPolicy,
    actor_task_owner::PreparedActorTask,
    entity_behavior::{behavior_program, BehaviorChoiceListSource},
    entity_collision_state::RetailStateWord,
    entity_emitters::AuthoredRadialEmission,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    infection_evolution::{InfectionEvolutionEffects, InfectionTailSound},
    sub_n_runtime::{sub_n_runtime_from_constructor, SubNConstructorTerrain},
};
use std::collections::VecDeque;
use v2k_formats::{
    anim_frames::{ModelSlotPattern, TerrainObjectDescriptor, TerrainObjectTable},
    levels::EntityAnimation,
    terrain::{TerrainCell, TerrainGrid, GRID_SIZE},
};

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Radial(u32),
    Sound([i16; 3]),
}

struct Effects {
    words: VecDeque<u16>,
    events: Vec<Event>,
}

impl InfectionEvolutionEffects for Effects {
    fn next_shared_random_u16(&mut self) -> u16 {
        self.words
            .pop_front()
            .expect("unexpected fallback RNG draw")
    }

    fn queue_infection_tail_sound(&mut self, _: InfectionTailSound) {
        panic!("unresolved census must skip infection mutation");
    }
}

impl AuthoredHiveComponentEffects for Effects {
    fn emit_authored_radial(&mut self, emission: AuthoredRadialEmission) {
        self.events.push(Event::Radial(emission.source_id));
        // The production class5 allocation consumes three words here.
        for _ in 0..3 {
            self.next_shared_random_u16();
        }
    }

    fn queue_hive_detailed_sound(&mut self, sound_id: u16, position_raw: [i16; 3]) {
        assert_eq!(sound_id, 77);
        self.events.push(Event::Sound(position_raw));
    }
}

fn hive(id: u32, dying: bool, terrain: &TerrainGrid, objects: &TerrainObjectTable) -> Entity {
    let program = behavior_program(46).unwrap();
    let mut header = [0; 0x18];
    header[..4].copy_from_slice(&10_000_u32.to_le_bytes());
    header[4..8].copy_from_slice(&5_000_u32.to_le_bytes());
    let mut emitter = AuthoredRadialEmitterController::from_authored_spawn(
        program,
        Some([0; 10]),
        Some(&EntityAnimation {
            header,
            frames: Vec::new(),
        }),
        Some([1, 0]),
    )
    .unwrap();
    if dying {
        emitter.enter_dying_slot0();
        emitter.arm_wreck_suction();
        emitter.advance_wreck_timer(6_000_000);
    }
    let mut entity = Entity::unresolved_port_entity(id, EntityKind::Unknown(67), 67);
    entity.collision.state_flags_at_0x08 =
        RetailStateWord::exact(0x8800 | if dying { 0x4000 } else { 0 });
    entity.collision.health_raw = RetailRuntimeValue::Known(if dying { 0 } else { 2_000 });
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            program,
            0,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            program.initial_style,
        )
        .unwrap(),
    ));
    entity.actor_tasks.replace_prepared(
        ActorTaskSlot::Primary,
        PreparedActorTask::new(ActorTaskRuntime::HiveRadial(if dying {
            crate::hive_death::HiveRadialTaskState::dying()
        } else {
            crate::hive_death::HiveRadialTaskState::live()
        })),
    );
    entity.sub_n_runtime = sub_n_runtime_from_constructor(
        RetailRuntimeValue::Known(true),
        Some([0; 10]),
        true,
        [0; 3],
        Some(SubNConstructorTerrain {
            terrain,
            terrain_objects: objects,
        }),
    );
    entity.authored_radial_emitter = Some(emitter);
    // Distinct current cue positions also prove the later allocation ran.
    // The retained constructor anchor is deliberately still [128,0,128].
    entity.set_motion_raw([id as i16 * 100, 0, 0], [0; 3]);
    entity
}

#[test]
fn unresolved_objective_keeps_all_hive_radial_sound_wreck_and_sub_k_fallbacks() {
    let mut terrain = TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    terrain.cells[0].attribute = 1;
    let objects = TerrainObjectTable {
        records: vec![
            TerrainObjectDescriptor {
                model_ids: [0; 4],
                kind_index: 22,
                pattern: ModelSlotPattern::Static,
            };
            2
        ],
    };
    let live = hive(10, false, &terrain, &objects);
    let detailed_wreck = hive(11, true, &terrain, &objects);
    let coarse_wreck = hive(12, true, &terrain, &objects);
    let mut disabled_wreck = hive(13, true, &terrain, &objects);
    disabled_wreck
        .authored_radial_emitter
        .as_mut()
        .unwrap()
        .set_behavior_enabled(false);
    let mut unresolved = Entity::unresolved_port_entity(90, EntityKind::Unknown(122), 122);
    unresolved.capability_flags = 8;
    let mut player = Entity::unresolved_port_entity(1, EntityKind::Player, 46);
    player.capability_flags = 1;
    player.collision.state_flags_at_0x08 = RetailStateWord::exact(0x8000);
    player.set_motion_raw([228, 0, 128], [0; 3]);
    let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 68];
    metadata[67].initial_health_raw = Some(2_000);
    metadata[67].sub_n_payload = Some([0; 10]);
    // A controlled nonzero low-health cue makes DCA0 observable on wrecks;
    // this asserts shared dispatch, not a claim about authored Hive sounds.
    metadata[67].detailed_sound_policy = RetailRuntimeValue::Known(ActorDetailedSoundPolicy {
        full_health_raw: 2_000,
        healthy_sounds: [77, 0],
        low_health_sound: 77,
        periods_raw: [1_024; 2],
    });
    let mut manager = EntityManager::from_entities_with_type_metadata_for_test(
        vec![
            player,
            live,
            detailed_wreck,
            coarse_wreck,
            disabled_wreck,
            unresolved,
        ],
        metadata,
        false,
    );
    let mut effects = Effects {
        words: VecDeque::from([0; 5]),
        events: Vec::new(),
    };
    let mut notifications = GameplayNotifications::new();
    let mut tally = crate::world_complete_results::WorldCompleteTally::default();
    let result = manager.advance_authored_hive_components(
        AuthoredHiveComponentFrame {
            elapsed_us: 20_000,
            terrain: Some(&terrain),
            retail_tick: 1,
            notification_phase: GameplayNotificationPhase::Playing,
            notifications: &mut notifications,
            world_complete_tally: &mut tally,
        },
        |entity| {
            if entity.id == 12 {
                ComponentUpdateMode::Coarse
            } else {
                ComponentUpdateMode::Detailed
            }
        },
        &mut effects,
    );
    assert_eq!(
        result,
        Err(AuthoredHiveInfectionBlock::UnresolvedObjectiveState { entity_id: 90 })
    );
    assert_eq!(
        effects.events,
        [
            Event::Radial(10),
            Event::Sound([1_000, 0, 0]),
            Event::Sound([1_100, 0, 0])
        ]
    );
    assert!(effects.words.is_empty());
    let emitter = |id| {
        manager
            .entities
            .iter()
            .find(|entity| entity.id == id)
            .unwrap()
            .authored_radial_emitter
            .as_ref()
            .unwrap()
    };
    assert_eq!(emitter(10).accumulator_us(), 10_000);
    assert_eq!(emitter(10).infection_accumulator_us(), 0);
    assert_eq!(
        manager
            .entities
            .iter()
            .find(|entity| entity.id == 10)
            .unwrap()
            .collision
            .health_raw,
        RetailRuntimeValue::Known(2_000)
    );
    assert_eq!(emitter(11).suction_timer_us(), 20_000);
    assert_eq!(emitter(11).sub_k_output(), 78);
    assert_eq!(emitter(12).suction_timer_us(), 20_000);
    assert_eq!(
        emitter(12).sub_k_output(),
        0,
        "coarse dying wrapper skips its SubK write"
    );
    assert_eq!(
        emitter(13).suction_timer_us(),
        0,
        "disabled shared callback keeps its clock frozen"
    );
    assert_eq!(
        emitter(13).sub_k_output(),
        78,
        "the detailed wrapper's SubK suffix remains independent of its disabled shared callback"
    );
    assert_eq!(manager.player().unwrap().velocity_raw(), [-6, -56, 0]);
    assert_eq!(notifications.save_tail_seen_mask(), 0);
    assert_eq!(tally.ticks_0x2bc, 0);
}

fn retry_contact_manager() -> EntityManager {
    let mut terrain = TerrainGrid {
        header: [0; 5],
        cells: vec![
            TerrainCell {
                height: 0,
                attribute: 0,
                terrain_type: 0,
            };
            GRID_SIZE * GRID_SIZE
        ],
    };
    terrain.cells[0].attribute = 1;
    let objects = TerrainObjectTable {
        records: vec![
            TerrainObjectDescriptor {
                model_ids: [0; 4],
                kind_index: 22,
                pattern: ModelSlotPattern::Static,
            };
            2
        ],
    };
    let mut first = hive(10, true, &terrain, &objects);
    let mut second = hive(11, true, &terrain, &objects);
    for entity in [&mut first, &mut second] {
        entity
            .authored_radial_emitter
            .as_mut()
            .unwrap()
            .advance_wreck_timer(1);
    }
    let mut player = Entity::unresolved_port_entity(1, EntityKind::Player, 46);
    player.capability_flags = 1;
    player.collision.state_flags_at_0x08 = RetailStateWord::exact(0x8000);
    // Both constructor anchors are[128,0,128]; current Hive positions differ.
    player.set_motion_raw([228, -129, 128], [37, -2, -19]);
    EntityManager::from_entities_with_type_metadata_for_test(
        vec![player, first, second],
        Vec::new(),
        false,
    )
}

#[test]
fn failed_wreck_retry_inner_request_keeps_every_suction_and_all_allocation_custody() {
    let mut manager = retry_contact_manager();
    let before: Vec<_> = manager
        .entities
        .iter()
        .skip(1)
        .map(|entity| {
            (
                entity.id,
                entity.position_raw(),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                entity
                    .authored_radial_emitter
                    .as_ref()
                    .unwrap()
                    .suction_timer_us(),
            )
        })
        .collect();
    assert!(
        manager.apply_authored_hive_wreck_player_contacts(HiveWreckPlayerContactFrame {
            elapsed_us: 20_000,
            session_aborted: true,
        })
    );
    assert_eq!(manager.player().unwrap().position_raw(), [228, -129, 128]);
    assert_eq!(manager.player().unwrap().velocity_raw(), [7, -58, -19]);
    assert_eq!(manager.entities.len(), 3);
    let after: Vec<_> = manager
        .entities
        .iter()
        .skip(1)
        .map(|entity| {
            (
                entity.id,
                entity.position_raw(),
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                entity
                    .authored_radial_emitter
                    .as_ref()
                    .unwrap()
                    .suction_timer_us(),
            )
        })
        .collect();
    assert_eq!(
        after, before,
        "contact does not retick, clear or replace tasks"
    );
}

#[test]
fn wreck_retry_requires_actual_player_marker_clock_task_and_dead_body_gates() {
    for gate in [
        "normal",
        "player_capability",
        "player_state",
        "marker",
        "clock",
        "owner",
        "disabled",
        "live_body",
        "unresolved_body",
    ] {
        let mut manager = retry_contact_manager();
        manager.entities.truncate(2);
        match gate {
            "player_capability" => manager.entities[0].capability_flags = 0,
            "player_state" => {
                manager.entities[0].collision.state_flags_at_0x08 = RetailStateWord::exact(1)
            }
            "marker" => manager.entities[1].sub_n_runtime = RetailRuntimeValue::Known(None),
            "clock" => manager.entities[1]
                .authored_radial_emitter
                .as_mut()
                .unwrap()
                .arm_wreck_suction(),
            "owner" => manager.entities[1].authored_radial_emitter = None,
            "disabled" => manager.entities[1]
                .authored_radial_emitter
                .as_mut()
                .unwrap()
                .set_behavior_enabled(false),
            "live_body" => {
                manager.entities[1].collision.state_flags_at_0x08 = RetailStateWord::exact(0x8800)
            }
            "unresolved_body" => {
                manager.entities[1].collision.state_flags_at_0x08 = RetailStateWord::unknown()
            }
            "normal" => (),
            _ => unreachable!(),
        }
        assert!(
            !manager.apply_authored_hive_wreck_player_contacts(HiveWreckPlayerContactFrame {
                elapsed_us: 20_000,
                session_aborted: gate != "normal",
            }),
            "{gate}"
        );
        assert_eq!(manager.entities.len(), 2);
    }
    let mut manager = retry_contact_manager();
    manager.entities.truncate(2);
    manager.entities[1].collision.state_flags_at_0x08 = RetailStateWord::exact(0);
    assert!(
        manager.apply_authored_hive_wreck_player_contacts(HiveWreckPlayerContactFrame {
            elapsed_us: 20_000,
            session_aborted: true,
        }),
        "an allocated zero-state Hive also selects the source dead/null branch"
    );
    let mut manager = retry_contact_manager();
    manager.entities[0].collision.state_flags_at_0x08 = RetailStateWord::exact(0xC000);
    assert!(
        manager.apply_authored_hive_wreck_player_contacts(HiveWreckPlayerContactFrame {
            elapsed_us: 20_000,
            session_aborted: true,
        }),
        "the player predicate tests8000 and capability1; player dying4000 does not suppress it"
    );
}
