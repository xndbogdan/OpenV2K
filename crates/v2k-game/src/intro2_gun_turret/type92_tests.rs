use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    damage::DamagePacket,
    entity::world_position_raw,
    entity_collision_state::RetailRuntimeValue,
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_type47_live::world::native_intro2_fixture,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    type60_exploding_ring::Type60ConstructionProvenance,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, ParticleEnvironment, WorldFx},
};
use native::tests::{fixture, generic, publish};
use v2k_formats::models::ModelMaterializationContext;

#[v2k_test_support::retail_test]
fn both_native_gun_turrets_publish_the_global_clock_independently_of_aim() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    for (spawn, entity_type) in [(52, 92), (53, 102)] {
        let id = publish(&mut manager, &session, &metadata, spawn);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.entity_type, entity_type);
        let runtime = entity.intro2_gun_turret_runtime.as_mut().unwrap();
        runtime.sub_l_output_raw = [1234, 5678];
        runtime.sub_e_joint_word_raw = 9012;
        let retained = entity.intro2_gun_turret_runtime;

        // 40D320 returns the global word even when no aim update occurs.
        // Include both the plasma's 64-tick cycle and the callback's u16 wrap.
        for (tick, clock_word) in [
            (0, 0),
            (16, 16),
            (32, 32),
            (48, 48),
            (64, 64),
            (65_535, 65_535),
            (65_536, 0),
            (65_552, 16),
        ] {
            let vars = entity.presentation_anim_vars(tick);
            assert_eq!(
                vars.dynamic[0], clock_word,
                "type {entity_type}, tick {tick}"
            );
            assert_eq!(&vars.dynamic[1..4], &[1234, 5678, 9012]);
        }
        assert_eq!(entity.intro2_gun_turret_runtime, retained);
    }
}

#[v2k_test_support::retail_test]
fn type92_constructor_requires_its_own_profile_and_two_unshared_output_words() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(52))
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.entity_type, 92);
    assert_eq!(entity.model_slots, [Some(171); 4]);
    let before = entity.position_raw();
    assert_eq!(
        publish_intro2_gun_turret(
            entity,
            &metadata[102],
            session.cache.terrain().unwrap(),
            &mut || panic!("foreign metadata must fail before the selector")
        ),
        Err(Intro2GunTurretError::Metadata)
    );
    assert_eq!(entity.position_raw(), before);
    assert!(entity.intro2_gun_turret_runtime.is_none());
    let mut draws = 0;
    let publication = publish_intro2_gun_turret(
        entity,
        &metadata[92],
        session.cache.terrain().unwrap(),
        &mut || {
            draws += 1;
            0x8123
        },
    )
    .unwrap();
    assert_eq!(draws, 1);
    assert_eq!(publication.selection.program.class_id, 29);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(5000));
    assert_eq!(entity.capability_flags, 0x1044);
    assert_eq!(
        entity.actor_common_axis_descriptor,
        RetailRuntimeValue::Known(CommonAxisDescriptor {
            strict_axis_limit_raw: 5120,
            raw_word_at_0x04: 0x100b,
        })
    );
    let runtime = entity.intro2_gun_turret_runtime.unwrap();
    assert_eq!(runtime.profile, Intro2GunTurretProfile::Type92);
    assert_eq!(runtime.sub_e_runtime.joint_bindings, [None; 2]);
    assert_eq!(runtime.sub_l_output_raw, [0; 2]);
    assert_eq!(runtime.sub_e_joint_word_raw, 0);
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::Intro2GunTurret(_))
    ));
    assert!(intro2_gun_turret_allocation_authenticates(entity));
    entity.intro2_gun_turret_runtime.as_mut().unwrap().profile = Intro2GunTurretProfile::Type102;
    assert!(!intro2_gun_turret_allocation_authenticates(entity));
    entity.intro2_gun_turret_runtime = Some(runtime);
    assert_eq!(
        session.cache.global_entity_type(92).unwrap().raw_header[0x110],
        2
    );
}

#[v2k_test_support::retail_test]
fn type92_detailed_and_coarse_callbacks_preserve_the_fixed_body() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for detailed in [true, false] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, 52);
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.state_flags_at_0x08.overwrite(
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
                | if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
        );
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        let position = entity.position_raw();
        let basis = entity.physical_body_basis_q31();
        entity.set_position_raw([position[0], position[1] + 100, position[2]]);
        entity.set_velocity_raw([250, -300, 800]);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(250);
        let owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
        let result = tick_intro2_gun_turret(
            &mut manager,
            owner,
            Intro2GunTurretFrame {
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                resources: &mut session.cache,
                world_fx: &mut WorldFx::new(),
                elapsed_micros: 125_000,
                retail_tick: 1000,
            },
        );
        assert_eq!(
            result.outcome,
            Intro2GunTurretOutcome::Advanced {
                entity_id: id,
                callback_enabled: true,
                callback_elapsed_micros: 125_000,
            }
        );
        assert!(result.retained_owner.is_some());
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.position_raw(), position);
        assert_eq!(entity.velocity_raw(), [0; 3]);
        assert_eq!(entity.physical_body_basis_q31(), basis);
        assert_eq!(
            entity.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(125)
        );
        assert!(
            matches!(entity.actor_task_state(ActorTaskSlot::Tertiary), Some(ActorTaskRuntime::Intro2GunTurret(task)) if task.elapsed_ms() == 125)
        );
    }
}

#[v2k_test_support::retail_test]
fn type92_model171_single_muzzle_queues_and_drains_without_alternation_or_joint_binding() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 52);
    let entity = manager.entity_mut(id).unwrap();
    let origin = [200, 1000, 300];
    entity.set_motion_raw(origin, [0; 3]);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    });
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8200_0000, 0x0200_0000);
    let mut fx = WorldFx::new();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    expected_rng.next_shared_retail_random_u16();
    let result = aim::tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 400_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    assert_eq!(result.queued_shots, 2);
    assert_eq!(result.rejected_appends, 0);
    let entity = manager.entity_mut(id).unwrap();
    let runtime = entity.intro2_gun_turret_runtime.unwrap();
    assert_eq!(runtime.sub_e_runtime.cadence_raw, 400_000);
    assert_eq!(runtime.sub_e_runtime.emitter_selector, 0);
    assert_eq!(runtime.sub_e_runtime.joint_bindings, [None; 2]);
    assert_eq!(runtime.sub_e_joint_word_raw, 0);
    let queue = entity.intro2_gun_turret_aim_runtime.as_ref().unwrap();
    let descriptor = queue.projectile_descriptor();
    assert_eq!(descriptor.raw_word_at_0x12, 18);
    assert_eq!(descriptor.alternate_emitter_raw, 0);
    assert_eq!(descriptor.variable_bindings, [0; 4]);
    for (index, shot) in queue.transient_shots().iter().enumerate() {
        assert_eq!(shot.emitter_selector, 0);
        assert_eq!(shot.time_offset_raw, index as i32 * 400_000);
        assert_eq!(shot.speed_field, GenericEmitterSpeedField::Explicit(4000));
    }
    let direction = queue.transient_shots()[1].direction_raw;
    // Independently resolve the authored single-barrel hierarchy. With zero
    // L outputs the mount is identity and adds its raw [0,90,0] attachment.
    let vars = entity.presentation_anim_vars(99);
    let root = session.cache.global_model(171).unwrap().materialize(&vars);
    assert_eq!(root.instances.len(), 1);
    let mount = &root.instances[0];
    assert_eq!(mount.model_id, 172);
    assert_eq!(mount.attach_slot, 12);
    assert_eq!(mount.attach_pos, Some([0., 90., 0.]));
    assert_eq!(
        mount.orientation,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
    );
    let local_muzzle = session
        .cache
        .global_model(172)
        .unwrap()
        .resolve_slot_with_context(18, ModelMaterializationContext::intrinsic(&vars, None))
        .expect("the actual Type92 barrel owns muzzle slot18")
        .position_raw;
    let expected_muzzle: [i16; 3] = std::array::from_fn(|axis| {
        let raw = (local_muzzle[axis] + mount.attach_pos.unwrap()[axis]) as i32;
        origin[axis].wrapping_add(((i64::from(raw) * i64::from(i32::MAX)) >> 31) as i16)
    });
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
    let drained = aim::drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        99,
    )
    .unwrap();
    assert_eq!(drained.consumed_requests, 2);
    assert_eq!(drained.materialized_particle_classes, vec![55; 2]);
    assert_eq!(fx.particle_count(), 2);
    let particles: Vec<_> = fx
        .prepare_presentation([640, 480], 0x3000, |_| {
            v2k_render::ParticleCenterProjection {
                screen: [320, 240],
                depth_raw: 1000,
                clip: 0,
            }
        })
        .particles()
        .copied()
        .collect();
    // D300 presents ten copied-record streak samples per physical class55.
    assert_eq!(particles.len(), 20);
    assert!(particles
        .iter()
        .all(|p| p.particle.source_entity_type_at_birth == Some(92)
            && p.particle.owner_id == Some(id)));
    assert_ne!(expected_muzzle, origin);
    // 11400 advances the second command by ((400000 >> 5) * 4000) >> 15
    // =1525 along its retained Q31 direction, with no source-velocity rewind.
    let expected_second = std::array::from_fn(|axis| {
        expected_muzzle[axis].wrapping_add(((i64::from(direction[axis]) * 1525) >> 31) as i16)
    });
    // 40A60 allocates virgin slots199 then198 and prepends each to its
    // priority list. Presentation therefore completes the second command's
    // streak before the first command's; it does not iterate the birth FIFO.
    assert!(particles[..10].iter().all(|p| p.slot == 198));
    assert!(particles[10..].iter().all(|p| p.slot == 199));
    assert_eq!(
        world_position_raw(particles[0].particle.position),
        expected_second
    );
    assert_eq!(
        world_position_raw(particles[10].particle.position),
        expected_muzzle
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.intro2_gun_turret_runtime, Some(runtime));
    assert_eq!(
        entity
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        0
    );
}

#[v2k_test_support::retail_test]
fn type92_own_damage_threshold_reaches_class49_then_the_real_type60_lifetime() {
    let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(52))
        .unwrap()
        .id;
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    for other in ids {
        if other != id {
            manager
                .entity_mut(other)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, 0);
        }
    }
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.intro2_gun_turret_runtime.unwrap().profile,
        Intro2GunTurretProfile::Type92
    );
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x0c06_8000);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    entity.set_position_raw([100, 10_000, -300]);
    let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_intro2_gun_turret(Intro2GunTurretOwner::adopt(&manager, id).unwrap());
    let mut fx = WorldFx::new();
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::default();
    for (tick, amount, filtered, health) in [
        (91, 1800, 0, 5000),
        (92, 1801, 1, 4999),
        (93, 6799, 4999, 0),
    ] {
        let result = impact::apply_intro2_gun_turret_particle_hit(
            impact::Intro2GunTurretImpactFrame {
                world: impact::GunTurretImpactWorld::Cinematic,
                entities: &mut manager,
                resources: &mut session.cache,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                retail_tick: tick,
            },
            ParticleEntityImpact {
                source_particle_class: 16,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.; 3],
                velocity_raw: [0, 0, 8192],
                damage: Some(BallisticDamageRequest {
                    packet: DamagePacket {
                        channels: [2, 0],
                        amounts_raw: [amount, 0],
                    },
                    source_entity_type_at_birth: Some(34),
                    source_owner_id: Some(35),
                }),
            },
        );
        let impact::Intro2GunTurretImpactOutcome::Applied(checked) = result else {
            panic!("{result:?}")
        };
        assert_eq!(checked.filtered_damage_raw, filtered);
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(health)
        );
        if health > 0 {
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
                task
            );
        }
    }
    let source = manager.entity_mut(id).unwrap();
    assert!(
        matches!(source.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c71e0)
    );
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| source.actor_task_state(slot).is_none()));
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    let rings: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 60)
        .collect();
    assert_eq!(rings.len(), 1);
    let ring = rings[0];
    assert_eq!(
        ring.type60_construction_provenance(),
        Some(Type60ConstructionProvenance::Class49ExplosionTail)
    );
    assert_eq!(ring.model_slots, [Some(243); 4]);
    let ring_id = ring.id;
    assert!(scheduler.family_for(ring_id).is_some());
    let pass = scheduler.tick(
        &mut manager,
        SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            elapsed_micros: 158 << 12,
            global_elapsed_micros: 158 << 12,
            retail_tick: 94,
            main_base_abort_active: false,
        },
        &mut notifications,
    );
    assert_eq!(pass.block, None);
    assert!(fx.exploding_rings().is_empty());
    assert!(manager
        .pending_actor_deferred_destroy_ids()
        .contains(&ring_id));
    assert!(scheduler.family_for(ring_id).is_none());
}
