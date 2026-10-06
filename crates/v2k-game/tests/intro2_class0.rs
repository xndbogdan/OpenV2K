//! Intro2's invisible Type52 anchors use the shared constructor and task owner.

use std::cell::RefCell;
use v2k_game::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity::{
        CapturedIntro2EntityConstructionError, Class0ActorError, Class0ActorOutcome,
        Class0ActorOwner, Entity, EntityConstructionResources, EntityManager, Intro2BirthSelection,
    },
    entity_behavior::BehaviorChoiceListSource,
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, FULLY_ABOVE_SURFACE_STATE_BIT,
        SURFACE_STATE_MASK,
    },
    gameplay_notifications::GameplayNotifications,
    intro2_commands::Intro2Commands,
    intro2_type13_live::Intro2Type13BirthSelection,
    intro2_type26_defecate_virus::Intro2Type26BirthSelection,
    intro2_type47_live::Intro2Type47BirthSelection,
    opening::{intro2_uses_live_actor_pose, intro_actor_visible},
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

const FLAGS: [(usize, [i16; 3]); 12] = [
    (14, [-24832, -152, -28160]),
    (22, [-19712, 8, 2304]),
    (23, [-15616, 72, 5632]),
    (27, [-16640, 200, 5632]),
    (28, [-19712, 328, 6144]),
    (29, [-14080, -344, 4096]),
    (32, [-29184, 8, -32256]),
    (37, [-29184, 456, -29696]),
    (39, [-29184, -280, 30720]),
    (47, [-27648, 72, -32256]),
    (48, [-30208, 8, -32256]),
    (57, [21504, -248, -8192]),
];

fn session() -> GameSession {
    let data = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&data).unwrap();
    session.load_auxiliary_ovl(3, 1).unwrap();
    session.load_level_by_id(13, 1).unwrap();
    let ordinary =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(52).unwrap());
    session.load_level_by_id(50, 1).unwrap();
    assert_eq!(
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(52).unwrap()),
        ordinary,
        "Intro2 uses the same canonical Type52 descriptor as ordinary worlds"
    );
    session
}

fn metadata(session: &GameSession) -> Vec<EntityTypeRuntimeMetadata> {
    session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(id, _)| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        })
        .collect()
}

fn actor(manager: &EntityManager, index: usize) -> &Entity {
    manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(index))
        .unwrap()
}

fn retained_selection() -> Intro2BirthSelection {
    Intro2BirthSelection {
        type13: Intro2Type13BirthSelection::CapturedClass7,
        type47: Intro2Type47BirthSelection::CapturedGuard,
        type26: Intro2Type26BirthSelection::CapturedDefecate,
    }
}

#[derive(Debug, Clone, Copy)]
enum Entry {
    Native,
    CapturedWeighted,
    CapturedRetained,
}

#[v2k_test_support::retail_test]
fn native_and_replay_entries_publish_all_flags_before_the_first_actor_pass() {
    let mut session = session();
    let rows = metadata(&session);
    let descriptor_allocations = session
        .cache
        .level_desc()
        .unwrap()
        .entities
        .iter()
        .filter(|spawn| {
            matches!(
                rows[spawn.entity_type as usize].sub_d_steering_descriptor,
                RetailRuntimeValue::Known(Some(_))
            )
        })
        .count() as u8;
    let mut previous_leases = None;
    for entry in [
        Entry::Native,
        Entry::CapturedWeighted,
        Entry::CapturedRetained,
    ] {
        let mut fx = WorldFx::new();
        let extent = |id| session.cache.global_model(id).map(|model| model.radius);
        let resources = EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&extent),
        };
        let level = session.cache.level_desc().unwrap();
        let mut manager = match entry {
            Entry::Native => {
                EntityManager::from_native_intro2_frontend(level, &rows, resources, 0, &mut fx)
            }
            Entry::CapturedWeighted | Entry::CapturedRetained => {
                EntityManager::from_captured_intro2_frontend_with_type_metadata(
                    level,
                    &rows,
                    resources,
                    match entry {
                        Entry::CapturedWeighted => Intro2BirthSelection::default(),
                        Entry::CapturedRetained => retained_selection(),
                        Entry::Native => unreachable!(),
                    },
                    &mut || 0,
                )
            }
        }
        .unwrap_or_else(|error| panic!("{entry:?}: {error:?}"));
        assert_eq!(
            manager.next_common_body_ordinal(),
            match entry {
                Entry::Native => RetailRuntimeValue::Known((level.entities.len() + 1) as u16),
                _ => RetailRuntimeValue::Unresolved,
            }
        );
        for (ordinal, entity) in manager.iter_all().enumerate() {
            assert_eq!(
                entity.construction_stamp_at_0xb4,
                match entry {
                    Entry::Native => RetailRuntimeValue::Known(0x9801 + ordinal as u16),
                    _ => RetailRuntimeValue::Unresolved,
                },
                "{entry:?} common body{ordinal}"
            );
        }
        assert_eq!(
            fx.next_sub_d_allocation_seed(),
            if matches!(entry, Entry::Native) {
                descriptor_allocations
            } else {
                0
            },
            "flags add no descriptor allocation or captured seed policy"
        );
        assert_eq!(
            manager
                .iter_all()
                .filter(|entity| entity.entity_type == 52)
                .count(),
            12
        );
        let mut owners = Vec::new();
        for (index, position) in FLAGS {
            let spawn = &level.entities[index];
            let entity = actor(&manager, index);
            assert_eq!(spawn.entity_type, 52);
            assert_eq!(spawn.param, 1);
            assert_eq!(spawn.rotation, [0; 3]);
            assert_eq!(entity.rotation_heading_pitch_roll_raw(), [0; 3]);
            assert_eq!(entity.position_raw(), position, "{entry:?} flag{index}");
            assert_eq!(entity.model_slots, [Some(145); 4]);
            assert_eq!(entity.model_index, Some(145));
            assert_eq!(
                entity.collision.active_model_slot(),
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1));
            assert_eq!(entity.mass_raw, 1);
            assert_eq!(entity.capability_flags, 0x100);
            assert!(
                intro2_uses_live_actor_pose(entity),
                "{entry:?} flag{index} is a native class-0 owner"
            );
            let state = entity.collision.state_flags_at_0x08;
            assert_eq!(
                state.masked(0x0100_0000),
                RetailRuntimeValue::Known(0x0100_0000)
            );
            assert_eq!(state.masked(0x20800), RetailRuntimeValue::Known(0));
            assert_eq!(
                state.masked(SURFACE_STATE_MASK),
                RetailRuntimeValue::Known(FULLY_ABOVE_SURFACE_STATE_BIT)
            );
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("{entry:?} flag{index} has no published context");
            };
            assert_eq!(context.active_style().style_address(), 0x004c7468);
            assert_eq!(
                context.choice_list_source(),
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            );
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(None)
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(0)
            );
            assert!(
                matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
            );
            assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
            assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
            assert!(!intro_actor_visible(index));
            let owner = Class0ActorOwner::adopt(&manager, entity.id).unwrap();
            assert_eq!(
                owner.allocation(),
                manager
                    .main_base_abort_actor_observation(entity.id)
                    .unwrap()
                    .lease
            );
            owners.push(owner);
        }
        let leases: Vec<_> = owners.iter().map(|owner| owner.allocation()).collect();
        if let Some(previous) = previous_leases.replace(leases.clone()) {
            assert_ne!(
                previous, leases,
                "a fresh manager cannot borrow another birth's custody"
            );
        }

        // Match the production load boundary: activation masking and owner
        // adoption precede the first Intro2 simulation, even under Klaus.
        manager.disable_authored_behavior_components();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_class0_actors(&manager), 12);
        assert_eq!(scheduler.adopt_class0_actors(&manager), 0);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        for retail_tick in 0..3 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: v2k_game::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        v2k_game::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    elapsed_micros: 20_000,
                    global_elapsed_micros: 20_000,
                    retail_tick,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(pass.block.is_none(), "{entry:?}: {:?}", pass.block);
            assert_eq!(pass.outcomes.len(), 12);
            for outcome in pass.outcomes {
                assert!(
                    matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::Class0Actor(
                            Class0ActorOutcome::Waiting { .. }
                        ) | SpecializedActorTaskProductionOutcome::Class0Actor(
                            Class0ActorOutcome::Advanced {
                                callback_enabled: false,
                                ..
                            }
                        )
                    ),
                    "{entry:?}: {outcome:?}"
                );
            }
            for ((index, position), owner) in FLAGS.into_iter().zip(&owners) {
                let entity = actor(&manager, index);
                assert_eq!(entity.position_raw(), position);
                assert_eq!(Class0ActorOwner::adopt(&manager, entity.id), Ok(*owner));
                assert!(
                    matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
                );
            }
            assert_eq!(scheduler.adopt_class0_actors(&manager), 0);
        }
        let mut commands = Intro2Commands::from_cache(&session.cache).unwrap();
        assert_eq!(commands.records()[0].spawn_index, 32);
        commands.present(&mut manager, 0);
        assert_eq!(
            commands.camera_subject(&manager).unwrap().position_raw,
            [-29184, 8, -32256]
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstructorEvent {
    Random,
    FlagHeader,
}

fn recorded_constructor(
    session: &GameSession,
    rows: &[EntityTypeRuntimeMetadata],
    fail_header: Option<usize>,
) -> (
    Result<EntityManager, CapturedIntro2EntityConstructionError>,
    Vec<ConstructorEvent>,
) {
    let events = RefCell::new(Vec::new());
    let headers = std::cell::Cell::new(0);
    let extent = |id| {
        if id == 145 {
            let index = headers.get();
            headers.set(index + 1);
            events.borrow_mut().push(ConstructorEvent::FlagHeader);
            if fail_header == Some(index) {
                return None;
            }
        }
        session.cache.global_model(id).map(|model| model.radius)
    };
    let result = EntityManager::from_captured_intro2_frontend_with_type_metadata(
        session.cache.level_desc().unwrap(),
        rows,
        EntityConstructionResources {
            terrain: session.cache.terrain(),
            terrain_objects: session.cache.terrain_objects(),
            model_extent_raw: Some(&extent),
        },
        Intro2BirthSelection::default(),
        &mut || {
            events.borrow_mut().push(ConstructorEvent::Random);
            0
        },
    );
    (result, events.into_inner())
}

#[v2k_test_support::retail_test]
fn missing_flag_model_header_stops_before_its_selector_and_retains_the_reached_rng_prefix() {
    let session = session();
    let rows = metadata(&session);
    let (complete, successful_events) = recorded_constructor(&session, &rows, None);
    complete.unwrap();
    let headers: Vec<_> = successful_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| (*event == ConstructorEvent::FlagHeader).then_some(index))
        .collect();
    assert_eq!(headers.len(), 12);
    for &index in &headers {
        assert_eq!(
            successful_events[index + 1],
            ConstructorEvent::Random,
            "each successful flag performs its own singleton selector after header lookup"
        );
    }
    // These authored flag pairs are adjacent allocations, so no other actor
    // constructor can disguise a missing or duplicated singleton draw.
    for first in [1, 3, 4, 9] {
        assert_eq!(FLAGS[first].0 + 1, FLAGS[first + 1].0);
        assert_eq!(
            headers[first + 1],
            headers[first] + 2,
            "adjacent flags must perform exactly one selector each"
        );
    }
    assert!(
        headers[0] > 0,
        "the first flag follows real preceding actor constructors"
    );
    for failed_flag in [0, 1, 11] {
        let (failed, actual_prefix) = recorded_constructor(&session, &rows, Some(failed_flag));
        assert_eq!(
            failed.err(),
            Some(CapturedIntro2EntityConstructionError::Class0Publication(
                Class0ActorError::Runtime("constructor model header08")
            ))
        );
        assert_eq!(actual_prefix, successful_events[..=headers[failed_flag]],
            "flag{failed_flag} must fail before its selector without replaying or erasing earlier births");
    }
}
