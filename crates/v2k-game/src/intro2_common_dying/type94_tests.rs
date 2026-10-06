use super::*;
use crate::entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
use crate::intro2_type47_live::world::native_intro2_fixture;

#[v2k_test_support::retail_test]
fn native_type94_zero_surface_selectors_decay_in_living_and_dying_without_lifecycle_rng() {
    use crate::world_fx::ParticleEnvironment;
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(43))
        .unwrap()
        .id;
    let metadata = manager.type_runtime_metadata(94).unwrap().clone();
    let RetailRuntimeValue::Known(effects) = metadata.common_world_effects else {
        panic!()
    };
    assert_eq!(effects.surface_selectors, [0, 0]);
    assert_eq!(effects.surface_lifetime_ms, 0);
    let terrain = session.cache.level_terrain().unwrap();
    let mut fx = WorldFx::new();
    let mut death = None;
    for dying in [false, true] {
        for (dt, expected) in [(999, 20), (1000, 19), (125000, 0)] {
            let entity = manager.entity_mut(id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, if dying { DYING_STATE_BIT } else { 0 });
            entity.set_position_raw([0, i16::MIN, 0]);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(20);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
            if dying {
                world::run_surface(
                    entity,
                    &metadata,
                    terrain,
                    592,
                    dt,
                    99,
                    ParticleEnvironment::Dry,
                    &mut fx,
                )
                .unwrap();
            } else {
                run_living_actor_surface(
                    &mut manager,
                    id,
                    Intro2ActorSurfaceFrame {
                        metadata: &metadata,
                        terrain,
                        active_model_extent_raw: 592,
                        elapsed_micros: dt,
                        retail_tick: 99,
                        particle_environment: ParticleEnvironment::Dry,
                    },
                    &mut fx,
                    &mut death,
                )
                .unwrap();
            }
            assert_eq!(
                manager
                    .entity_mut(id)
                    .unwrap()
                    .surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(expected)
            );
            assert!(death.is_none());
            assert_eq!(fx.pending_event_count(), 0);
            assert_eq!(fx.particle_count(), 0);
        }
    }
    assert_eq!(fx.next_shared_retail_random_u16(), 38);
}

#[v2k_test_support::retail_test]
fn intro2_class12_type94_preserves_native_components_mass_and_sound_through_detailed_tick() {
    for spawn in [43] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(spawn))
            .unwrap()
            .id;
        let metadata = manager.type_runtime_metadata(94).unwrap().clone();
        assert_eq!(metadata.mass_raw, 100);
        assert_eq!(metadata.model_slots, [272; 4]);
        assert_eq!(metadata.death_sound_id, RetailRuntimeValue::Known(Some(75)));
        assert_eq!(session.cache.global_model(272).unwrap().radius, 592);
        let entity = manager.entity_mut(id).unwrap();
        let native_before = entity.intro2_type94_runtime;
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
            RetailRuntimeValue::Known(400 + i32::from(constructor_word >> 8) * 400 / 0xA00)
        );
        let RetailRuntimeValue::Known(Some(sub_h)) = &entity.sub_h_external_frame_runtime else {
            panic!("native Sub-H custody")
        };
        assert_eq!(sub_h.records().len(), 6);
        assert!(!sub_h.is_enabled());
        assert_eq!(entity.intro2_type94_runtime, native_before);
        fx.process_pending();
        assert_eq!(
            fx.take_positional_sounds()
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            [75]
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
        assert_eq!(entity.mass_raw, 109, "12DA0 uses live Type94 mass plus B2");
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(entity.model_index, Some(272));
        assert_eq!(entity.model_slots, [Some(272); 4]);
        assert_eq!(
            entity.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.intro2_type94_runtime, native_before,
            "class12 retains the native allocation receipt"
        );
        // The native receipt contains the own Sub-D state and first-query owner;
        // equality above proves null-target class12 leaves both untouched.
        assert!(tick.retained_owner.is_some());
    }
}

#[v2k_test_support::retail_test]
fn intro2_class12_type94_retains_strict_detailed_timeout_and_coarse_transition() {
    for detailed in [true, false] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(43))
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
