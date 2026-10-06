use super::*;
use crate::{
    entity::{BaseFactoryRuntimeState, Entity},
    entity_behavior::BehaviorContextRuntime,
    entity_collision_state::EntityCollisionRuntimeState,
};

fn fixture(spawn: usize) -> Option<(EntityManager, u32)> {
    let (session, metadata) = crate::intro2_type66::native::tests::fixture()?;
    let mut manager = crate::intro2_type66::native::tests::generic(&session, &metadata);
    let id = crate::intro2_type66::native::tests::publish(&mut manager, &session, &metadata, spawn);
    assert!(intro2_type66_allocation_authenticates(
        manager.iter_all().find(|e| e.id == id).unwrap()
    ));
    Some((manager, id))
}

#[v2k_test_support::retail_test]
fn ordinary_factory_terminal_death_consumes_its_own_complete_template() {
    let Some((mut session, metadata)) = crate::intro2_type66::native::tests::fixture() else {
        return;
    };
    for (level, spawn) in [(14, 39), (19, 8)] {
        session.load_level_by_id(level, 1).unwrap();
        let mut manager = crate::intro2_type66::native::tests::generic(&session, &metadata);
        let id =
            crate::intro2_type66::native::tests::publish(&mut manager, &session, &metadata, spawn);
        let mut expected: [u32; 14] = crate::factory_production::FactorySection13Config::decode(
            session.cache.level_desc().unwrap().entities[spawn]
                .config
                .as_ref()
                .unwrap(),
        )
        .raw_words()[6..20]
            .try_into()
            .unwrap();
        expected[10] = 66;
        expected[11] = id;
        let entity = manager.entity_mut(id).unwrap();
        let wreck_model = entity.model_slots[1];
        let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
            panic!()
        };
        // Isolate19750's terminal callback from the independently tested clock.
        factory.progressive_death.elapsed_micros_raw = -1;
        let mut fx = WorldFx::new();
        let result = finish_with_effects(&mut manager, id, &mut fx, |_, _, request| {
            assert_eq!(request.template_words, expected);
            Ok(Intro2Type66TerminalEffectsOutcome {
                presentation_requested: false,
                terrain_changed: false,
            })
        })
        .unwrap();
        assert!(result.returned_nonzero);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.model_index, wreck_model);
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::Class0Timer(_))
        ));
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    collision: EntityCollisionRuntimeState,
    factory: RetailRuntimeValue<Option<BaseFactoryRuntimeState>>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    tasks: [Option<ActorTaskRuntime>; 3],
    position: [i16; 3],
    euler: [i16; 3],
    basis: RetailRuntimeValue<crate::common_mover::type9_attitude::Type9BodyBasis>,
    model: Option<usize>,
}

fn snapshot(entity: &Entity) -> Snapshot {
    Snapshot {
        collision: entity.collision.clone(),
        factory: entity.base_factory_runtime,
        context: entity.current_behavior_context,
        tasks: ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|s| entity.actor_task_state(s).copied()),
        position: entity.position_raw(),
        euler: entity.rotation_heading_pitch_roll_raw(),
        basis: entity.physical_body_basis_q31(),
        model: entity.model_index,
    }
}

fn assert_death_sounds(fx: &mut WorldFx, position: [i16; 3], count: usize) {
    assert_eq!(fx.pending_event_count(), count);
    fx.process_pending();
    assert_eq!(
        fx.take_positional_sounds(),
        vec![
            crate::world_fx::PositionalSoundEvent::fixed(
                62,
                [
                    f32::from(position[0] as u16) / 256.0,
                    f32::from(position[1]) / 256.0,
                    f32::from(position[2] as u16) / 256.0,
                ],
            );
            count
        ]
    );
    assert_eq!(fx.particle_count(), 0);
}

#[v2k_test_support::retail_test]
fn native_type66_death_revives_both_births_without_reconstructing_factory_or_pose() {
    for spawn in [36, 51] {
        let Some((mut manager, id)) = fixture(spawn) else {
            return;
        };
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(-231);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(19);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(37);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(712);
        // The interrupted production callback may have private history. Only
        // the replaced Primary's age/latch is reset by257C0, not Sub-M.
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.task_state_mut(primary)
        else {
            panic!()
        };
        task.accumulate_elapsed_prefix(1_250_000);
        task.sample_under_attack(true, false);
        let before = snapshot(entity);
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let expected_word = oracle.next_shared_retail_random_u16();
        let outcome = publish_intro2_type66_standard_death(&mut manager, id, &mut fx).unwrap();
        assert!(!outcome.returned_nonzero);
        assert!(outcome.owner.is_some());
        assert_eq!(outcome.selector_word, Some(expected_word));
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert_death_sounds(&mut fx, before.position, 1);
        let entity = manager.entity_mut(id).unwrap();
        let after = snapshot(entity);
        let RetailRuntimeValue::Known(Some(mut expected_factory)) = before.factory else {
            panic!()
        };
        expected_factory.progressive_death.elapsed_micros_raw = 1;
        assert_eq!(
            after.factory,
            RetailRuntimeValue::Known(Some(expected_factory))
        );
        assert_eq!(after.position, before.position);
        assert_eq!(after.euler, before.euler);
        assert_eq!(after.basis, before.basis);
        assert_eq!(after.model, before.model);
        assert_eq!(after.context, before.context);
        let mut expected_collision = before.collision;
        expected_collision.health_raw = RetailRuntimeValue::Known(PROGRESSION_REVIVE_HEALTH_RAW);
        assert_eq!(after.collision, expected_collision);
        let replacement = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(replacement, primary);
        assert_eq!(entity.actor_tasks.wrapper_flags(primary), None);
        let Some(ActorTaskRuntime::WorkingFactory(task)) =
            entity.actor_tasks.task_state(replacement)
        else {
            panic!()
        };
        assert_eq!(task.elapsed_ms(), 0);
        assert!(!task.under_attack_notification_latched());
    }
}

#[v2k_test_support::retail_test]
fn native_type66_repeated_death_keeps_progressive_clock_and_draws_each_selector() {
    let Some((mut manager, id)) = fixture(36) else {
        return;
    };
    let mut fx = WorldFx::new();
    publish_intro2_type66_standard_death(&mut manager, id, &mut fx).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
        panic!()
    };
    factory.progressive_death.elapsed_micros_raw = 123_457;
    let previous = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let mut oracle = WorldFx::new();
    let _first_selector = oracle.next_shared_retail_random_u16();
    let expected = oracle.next_shared_retail_random_u16();
    let result = publish_intro2_type66_standard_death(&mut manager, id, &mut fx).unwrap();
    assert_eq!(result.selector_word, Some(expected));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(factory)) = entity.base_factory_runtime else {
        panic!()
    };
    assert_eq!(factory.progressive_death.elapsed_micros_raw, 123_457);
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        Some(previous)
    );
    assert_death_sounds(&mut fx, entity.position_raw(), 2);
}

#[v2k_test_support::retail_test]
fn native_type66_death_noops_precede_sound_and_factory_callback_reads() {
    for (mask, returned_nonzero) in [
        (REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT, false),
        (DYING_STATE_BIT, true),
    ] {
        let Some((mut manager, id)) = fixture(51) else {
            return;
        };
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT | DYING_STATE_BIT, mask);
        entity.collision.death_sound_id = RetailRuntimeValue::Unresolved;
        entity.current_behavior_context = RetailRuntimeValue::Unresolved;
        let before = snapshot(entity);
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let outcome = publish_intro2_type66_standard_death(&mut manager, id, &mut fx).unwrap();
        assert_eq!(outcome.returned_nonzero, returned_nonzero);
        assert!(outcome.owner.is_none());
        assert_eq!(snapshot(manager.entity_mut(id).unwrap()), before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(fx.pending_event_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn native_type66_death_rejects_wrong_profile_and_terminal_entry_before_prefix() {
    for terminal in [false, true] {
        let Some((mut manager, id)) = fixture(36) else {
            return;
        };
        if terminal {
            let entity = manager.entity_mut(id).unwrap();
            let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
                panic!()
            };
            factory.progressive_death.elapsed_micros_raw = -1;
        } else {
            manager
                .type_runtime_metadata_mut_for_test(66)
                .unwrap()
                .death_sound_id = RetailRuntimeValue::Known(Some(1));
        }
        let before = snapshot(manager.entity_mut(id).unwrap());
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let error = publish_intro2_type66_standard_death(&mut manager, id, &mut fx).unwrap_err();
        assert!(!error.committed_prefix);
        assert_eq!(snapshot(manager.entity_mut(id).unwrap()), before);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type66_terminal_template_precedes_status_and_replaces_executing_factory() {
    use crate::actor_task_owner::{ActorTaskVisit, ActorTaskWrapperFlags};
    for (spawn, flags, wreck) in [(36, 0x2a, 365), (51, 0, 225)] {
        let Some((mut manager, id)) = fixture(spawn) else {
            return;
        };
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
            panic!()
        };
        factory.progressive_death.elapsed_micros_raw = -1;
        // Preserve live staff until after37390, regardless of how it was
        // accumulated before this death. Initializer+BC does not reconstruct M.
        factory.production.as_mut().unwrap().current_scientists_raw = 2;
        factory.current_scientists = 2;
        let before_factory = *factory;
        let position = entity.position_raw();
        let old_task = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let visit = ActorTaskVisit {
            slot: ActorTaskSlot::Primary,
            task_id: old_task,
        };
        assert!(entity
            .actor_tasks
            .begin_exact_visit_with(visit, |_| ())
            .is_some());
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        let outcome = finish_with_effects(&mut manager, id, &mut fx, |manager, fx, request| {
            assert_eq!(request.entity_id, id);
            assert_eq!(request.position_raw, position);
            assert_eq!(request.template_words[0], flags);
            assert_eq!(request.template_words[10..12], [66, id]);
            assert_eq!(request.template_words[12..], if spawn == 36 { [1280, 16] } else { [0, 0] });
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.base_factory_runtime, RetailRuntimeValue::Known(Some(before_factory)));
            assert_eq!(entity.model_index, Some(wreck));
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert!(matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c_9558));
            assert_death_sounds(fx, position, 1);
            Ok(Intro2Type66TerminalEffectsOutcome {
                presentation_requested: flags & 2 != 0,
                terrain_changed: false,
            })
        }).unwrap();
        assert!(outcome.returned_nonzero);
        assert_eq!(outcome.presentation_requested, flags & 2 != 0);
        assert_eq!(outcome.selector_word, None);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(after)) = entity.base_factory_runtime else {
            panic!()
        };
        assert_eq!(after.current_scientists, 0);
        assert_eq!(after.required_scientists, 0);
        assert_eq!(after.progressive_death, before_factory.progressive_death);
        let before_owner = before_factory.live_owner.unwrap();
        let after_owner = after.live_owner.unwrap();
        assert_eq!(after_owner.state_version, before_owner.state_version);
        assert_eq!(
            after_owner.next_transaction_id_raw,
            before_owner.next_transaction_id_raw
        );
        assert_eq!(
            entity.actor_tasks.wrapper_flags(old_task),
            Some(ActorTaskWrapperFlags {
                alive: false,
                in_callback: true
            })
        );
        assert!(!entity.actor_tasks.finish_exact_visit(visit));
        assert_eq!(entity.actor_tasks.wrapper_flags(old_task), None);
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
        );
        assert!(outcome.owner.is_some());
        assert!(
            matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c_7468)
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type66_terminal_effect_failure_keeps_generic_prefix_and_old_factory_task() {
    let Some((mut manager, id)) = fixture(36) else {
        return;
    };
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(factory)) = &mut entity.base_factory_runtime else {
        panic!()
    };
    factory.progressive_death.elapsed_micros_raw = -1;
    let before = snapshot(entity);
    let old_task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    let mut fx = WorldFx::new();
    let mut oracle = WorldFx::new();
    let error = finish_with_effects(&mut manager, id, &mut fx, |_, _, _| {
        Err(block("crater dependency", false))
    })
    .unwrap_err();
    assert_eq!(
        error,
        Intro2Type66DeathBlock {
            phase: "crater dependency",
            committed_prefix: true,
            presentation_requested: true,
            terrain_changed: false,
        }
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.model_index, Some(365));
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(entity.base_factory_runtime, before.factory);
    assert_eq!(entity.current_behavior_context, before.context);
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        old_task
    );
    assert_death_sounds(&mut fx, before.position, 1);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        oracle.next_shared_retail_random_u16()
    );
}
