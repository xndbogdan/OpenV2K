use super::*;
use crate::entity_behavior::{
    audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
};
use crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
use crate::intro2_type47_live::world::native_intro2_fixture;

#[v2k_test_support::retail_test]
fn intro2_class12_type16_preserves_native_components_mass_and_sound_through_detailed_tick() {
    for spawn in [5, 42] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let metadata = manager.type_runtime_metadata(16).unwrap().clone();
        assert_eq!(metadata.mass_raw, 100);
        assert_eq!(metadata.model_slots, [257; 4]);
        assert_eq!(metadata.death_sound_id, RetailRuntimeValue::Known(Some(86)));
        assert_eq!(session.cache.global_model(257).unwrap().radius, 315);
        let entity = manager.entity_mut(id).unwrap();
        let native_before = entity.intro2_type16_runtime;
        assert!(
            native_before.is_some(),
            "fixture must publish the native birth"
        );
        entity.set_motion_raw([0, 10_000, 0], [50, -80, 100]);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(9);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x68000);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(321);
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        let constructor_word = control.next_shared_retail_random_u16();
        let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.velocity_raw(), [50, 500, 100]);
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0x439)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(9)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(321)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("native Sub-A custody")
        };
        assert_eq!(
            sub_a.target_speed_raw(),
            RetailRuntimeValue::Known(200 + i32::from(constructor_word >> 8) * 200 / 0xA00)
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("native Sub-H custody")
        };
        assert_eq!(sub_h.records().len(), 8);
        assert!(!sub_h.is_enabled());
        assert_eq!(entity.intro2_type16_runtime, native_before);
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [86]
        );
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        assert_eq!(
            publish_intro2_common_standard_death(&mut manager, id, &mut fx).unwrap(),
            None
        );
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
        let tick = tick_intro2_common_dying(
            &mut manager,
            owner,
            Intro2CommonDyingFrame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 1,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Intro2CommonDyingOutcome::Advanced {
                    detailed: true,
                    terminal: false,
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "{:?}",
            tick.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.mass_raw, 109, "12DA0 uses live Type16 mass plus B2");
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.model_index, Some(257));
        assert_eq!(entity.model_slots, [Some(257); 4]);
        let RetailRuntimeValue::Known(Some(sub_j)) = &entity.sub_j_attachment_runtime else {
            panic!("authored Sub-J must remain present")
        };
        assert!(sub_j.is_empty());
        assert_eq!(sub_j.policy_raw_at_0x0c(), 0);
        assert_eq!(
            entity.intro2_type16_runtime, native_before,
            "class12 retains the native allocation receipt"
        );
        // The native receipt contains the own Sub-D state and first-query owner;
        // equality above proves null-target class12 leaves both untouched.
        assert!(tick.retained_owner.is_some());
    }
}

#[v2k_test_support::retail_test]
fn intro2_class12_type16_retains_strict_detailed_timeout_and_coarse_transition() {
    for detailed in [true, false] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(42))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        entity.set_motion_raw([0, 10_000, 0], [0; 3]);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x68000);
        let mut fx = WorldFx::new();
        let owner = publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            if detailed {
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            } else {
                0
            },
        );
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(250_000);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c =
            RetailRuntimeValue::Known(if detailed { 0 } else { 125_001 });
        if detailed {
            let ActorTaskRuntime::CommonDying(task) = entity
                .actor_tasks
                .task_state_mut(owner.visit.task_id)
                .unwrap()
            else {
                panic!()
            };
            task.before_callback(9_000_000);
        }
        let tick = tick_intro2_common_dying(
            &mut manager,
            owner,
            Intro2CommonDyingFrame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 0,
                retail_tick: 0,
            },
        );
        assert!(
            matches!(tick.outcome, Intro2CommonDyingOutcome::Advanced {terminal, ..}
            if terminal == !detailed),
            "{:?}",
            tick.outcome
        );
        if detailed {
            let tick = tick_intro2_common_dying(
                &mut manager,
                tick.retained_owner.unwrap(),
                Intro2CommonDyingFrame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 1_000,
                    retail_tick: 1,
                },
            );
            assert!(
                matches!(
                    tick.outcome,
                    Intro2CommonDyingOutcome::Advanced {
                        detailed: true,
                        terminal: true,
                        ..
                    }
                ),
                "{:?}",
                tick.outcome
            );
            assert!(tick.retained_owner.is_none());
        }
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
        assert!(manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .is_none());
    }
}

#[v2k_test_support::retail_test]
fn intro2_class12_type16_preserves_capture_nested_death_boundary_without_mutation() {
    for variant in 2..=5 {
        let Some((_session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(5))
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x68000);
        // Controlled style tests the exact D040 boundary while retaining the
        // real allocation/components. It does not fabricate attached task state.
        let context = BehaviorContextRuntime::named_audited(
            behavior_program(9).unwrap(),
            u32::from(variant),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            *audited_behavior_style(9, variant).unwrap(),
        )
        .unwrap();
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
        let before = (
            entity.collision.health_raw,
            entity.collision.state_flags_at_0x08,
            entity.velocity_raw(),
            entity.current_behavior_context,
            ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| entity.actor_task_state(slot).copied()),
        );
        let mut fx = WorldFx::new();
        assert!(matches!(
            publish_intro2_common_standard_death(&mut manager, id, &mut fx),
            Err(Intro2CommonDyingBlock::UnsupportedDeathHook(
                DeathCallbackPolicy::CapturePeopleCleanup
            ))
        ));
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            (
                entity.collision.health_raw,
                entity.collision.state_flags_at_0x08,
                entity.velocity_raw(),
                entity.current_behavior_context,
                ActorTaskSlot::IN_RETAIL_TICK_ORDER
                    .map(|slot| entity.actor_task_state(slot).copied())
            ),
            before
        );
        assert_eq!(fx.next_shared_retail_random_u16(), 38);
        assert_eq!(fx.pending_event_count(), 0);
    }
}
