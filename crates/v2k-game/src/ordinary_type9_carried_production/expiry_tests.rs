use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_surface::{plan_actor_surface_bubble, ActorSurfaceBubbleFrame},
    ordinary_type9_cargo::{commit_release, prepare_release},
    ordinary_type9_standard_death::OrdinaryType9StandardDeathEntry,
    specialized_actor_task_production::{
        SpecializedActorTaskFamily, SpecializedActorTaskProductionFrame,
        SpecializedActorTaskProductionOutcome, SpecializedActorTaskScheduler,
    },
};

#[v2k_test_support::retail_test]
fn carried_expiry_runs_release_constructor_then_class14_before_fresh_surface_rng() {
    let mut temporary_classes = std::collections::BTreeSet::new();
    for (detailed, extra_ms, player_near, death_cue_unresolved) in [
        (false, 0, false, false),
        (false, 1, false, false),
        (true, 0, false, false),
        (true, 1, false, false),
        (true, 0, true, false),
        (true, 1, true, false),
        (true, 0, true, true),
        (true, 1, true, true),
    ] {
        let (session, mut manager, owner) = super::tests::carried_fixture();
        let id = owner.entity_id();
        let actor = owner.actor;
        let dt = 20_000 + extra_ms * 1_000;
        let metadata = manager.type_runtime_metadata(9).unwrap().clone();
        let terrain = session.cache.level_terrain().unwrap();
        let extent = session
            .cache
            .global_model(LEVEL_ONE_TYPE9_MODEL_ID)
            .unwrap()
            .radius;
        let initial_position = [
            1_000,
            terrain.sea_level_raw() - (extent >> 2) as i16 - 300,
            3_000,
        ];
        let angles = [0x4000, 0x700, -0x1800];
        if player_near {
            manager
                .player_mut()
                .unwrap()
                .set_position_raw(initial_position);
        }
        let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
        if death_cue_unresolved {
            entity.collision.death_sound_id = RetailRuntimeValue::Unresolved;
        }
        entity.collision.state_flags_at_0x08.overwrite(
            ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT
                | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                | DYING_STATE_BIT
                | COMMON_MASTER_MOTION_REQUIRED_STATE_MASK,
            if detailed {
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
            } else {
                0
            },
        );
        entity.set_motion_raw(initial_position, [1_000, 400, -700]);
        entity.set_rotation_heading_pitch_roll_raw(angles);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(123);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4_980);
        let none_task = owner.visit.task_id;
        let mut oracle_manager = manager.fork_for_main_base_abort_transaction();
        let oracle_owner = owner.fork_for_main_base_abort_transaction();
        let mut effects = WorldFx::new();
        let mut oracle_fx = WorldFx::new();
        let mut oracle_notifications = GameplayNotifications::new();
        let mut actual_notifications = GameplayNotifications::new();

        // Independently compose the already-proven primitive owners in
        // retail order. This checks the production adapter's cadence and
        // custody against real CE90 and 10C10, rather than a canned seed.
        let entity = oracle_manager
            .ordinary_type9_selected_entity_mut(id)
            .unwrap();
        let RetailRuntimeValue::Known(prefix) =
            plan_common_scheduler_prefix(&entity.collision, dt, &mut || {
                u32::from(oracle_fx.next_shared_retail_random_u16())
            })
        else {
            panic!("native scheduler");
        };
        assert!(
            matches!(prefix.flow, CommonSchedulerPrefixFlow::Continue { callback_elapsed_us } if callback_elapsed_us == dt)
        );
        commit_common_scheduler_prefix(&mut entity.collision, prefix);
        entity.mass_raw = 133;
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            panic!("native Sub-I");
        };
        animation.advance(dt, angles[0] as u16, animation.linked_handle().is_some());
        let basis = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        let mut velocity = entity.velocity_raw();
        apply_type9_carried_environment_raw(
            &mut velocity,
            dt,
            133,
            0,
            manager.intro2_type13_environment().1,
        );
        entity.set_velocity_raw(velocity);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(5_000 + extra_ms);
        let expiry =
            CarriedSurfaceExpiry::prepare(&oracle_manager, &oracle_owner, &metadata).unwrap();
        let request = expiry.request(&metadata);
        let entity = oracle_manager
            .ordinary_type9_selected_entity_mut(id)
            .unwrap();
        let prepared = prepare_release(entity, &oracle_owner, &request).unwrap();
        let mut sounds = Vec::new();
        let _publication = commit_release(
            entity,
            oracle_owner,
            prepared,
            request,
            || u32::from(oracle_fx.next_shared_retail_random_u16()),
            |text| {
                oracle_notifications
                    .queue_attract_attention_resource_text(text, 41)
                    .unwrap();
            },
            |sound| sounds.push(sound),
        )
        .unwrap();
        let living_task = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(
            living_task, none_task,
            "CE90 constructs its own actual living graph"
        );
        assert!(
            matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().audited().is_some_and(|style| matches!(style.class_id, 6 | 10 | 45 | 54)))
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            unreachable!();
        };
        let temporary_class = context.active_style().audited().unwrap().class_id;
        temporary_classes.insert(temporary_class);
        for sound in sounds {
            oracle_fx.queue_fixed_positional_sound_raw(sound.global_sound_id, sound.position_raw);
        }
        oracle_manager
            .publish_ordinary_type9_standard_death(
                id,
                OrdinaryType9StandardDeathEntry::GenericDeath,
                MainBaseType9ResultScreenState::NotShown,
                &mut oracle_fx,
                41,
                Some(&mut oracle_notifications),
            )
            .unwrap();
        let entity = oracle_manager
            .ordinary_type9_selected_entity_mut(id)
            .unwrap();
        let class14_task = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert_ne!(class14_task, living_task);
        if extra_ms == 0 {
            let bubble = plan_actor_surface_bubble(
                0,
                ActorSurfaceBubbleFrame {
                    entity_id: id,
                    entity_type: 9,
                    state_flags: entity.collision.state_flags_at_0x08.known_value_bits(),
                    position_raw: entity.position_raw(),
                    emission_axis_q31: basis.forward,
                    active_model_extent_raw: extent,
                },
                &mut || u32::from(oracle_fx.next_shared_retail_random_u16()),
            );
            if let Some(bubble) = bubble {
                oracle_fx.materialize_actor_surface_bubble_request(
                    bubble,
                    ParticleEnvironment::Terrain(
                        TerrainCollisionContext::from_current_level_cache(&session.cache).unwrap(),
                    ),
                    41,
                );
            }
        }
        // DYING4000 suppresses the sound draw after either exact expiry
        // or overshoot. Master motion instead rereads the released state.
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        commit_common_scheduler_post_callback(&mut entity.collision);
        let RetailRuntimeValue::Known(master) = entity
            .collision
            .state_flags_at_0x08
            .masked(COMMON_MASTER_MOTION_REQUIRED_STATE_MASK)
        else {
            panic!("master input");
        };
        let motion =
            plan_common_master_motion(entity.position_raw(), entity.velocity_raw(), master, dt);
        commit_common_master_motion(entity, motion);

        let mut scheduler_draws = 0;
        let tick = tick_type9_carried_owner(
            &mut manager,
            owner,
            Type9CarriedProductionFrame {
                resources: &session.cache,
                world_fx: &mut effects,
                notifications: &mut actual_notifications,
                elapsed_micros: dt,
                retail_tick: 41,
            },
            &mut |fx| {
                scheduler_draws += 1;
                u32::from(fx.next_shared_retail_random_u16())
            },
        );
        assert_eq!(scheduler_draws, if detailed { 0 } else { 2 });
        assert_eq!(
            tick.outcome,
            Type9CarriedProductionOutcome::Class14Published {
                entity_id: id,
                callback_elapsed_micros: dt
            }
        );
        assert!(tick.retained_owner.is_none());
        let lease = tick.class14_task_lease.expect("new native class14 custody");
        assert_eq!(lease.actor(), actor);
        let actual = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let expected = oracle_manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap();
        assert_eq!(actual.collision, expected.collision);
        assert_eq!(actual.position_raw(), expected.position_raw());
        assert_eq!(actual.velocity_raw(), expected.velocity_raw());
        assert_ne!(
            actual.velocity_raw()[1],
            0,
            "latched AD omits ground snap even after class14 publication"
        );
        assert_eq!(
            actual.physical_body_basis_q31,
            RetailRuntimeValue::Known(basis)
        );
        assert_eq!(
            actual.actor_animation_runtime,
            expected.actor_animation_runtime
        );
        assert_eq!(
            actual.sub_a_propulsion_runtime,
            expected.sub_a_propulsion_runtime
        );
        assert_eq!(
            actual.surface_lifetime_timer_ms_at_0x48,
            RetailRuntimeValue::Known(5_000 + extra_ms)
        );
        assert_eq!(actual.attached_to, None);
        assert_eq!(
            actual.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(class14_task)
        );
        assert!(
            matches!(actual.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 1_000 && task.elapsed_ms() == 0)
        );
        assert_eq!(effects.particle_count(), oracle_fx.particle_count());
        assert_eq!(
            *effects.entity_construction_state().0,
            *oracle_fx.entity_construction_state().0
        );
        effects.process_pending();
        oracle_fx.process_pending();
        let actual_sounds = effects.take_positional_sounds();
        let expected_sounds = oracle_fx.take_positional_sounds();
        assert_eq!(
            actual_sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>(),
            expected_sounds
                .iter()
                .map(|sound| sound.sound_id)
                .collect::<Vec<_>>()
        );
        assert!(!actual_sounds.iter().any(|sound| sound.sound_id == 106));
        assert!(actual_sounds.iter().any(|sound| sound.sound_id == 35));
        assert_eq!(
            actual_notifications.save_tail_seen_mask(),
            oracle_notifications.save_tail_seen_mask()
        );
        if temporary_class == 45 {
            assert_ne!(
                actual_notifications.save_tail_seen_mask() & (1 << 0x10),
                0,
                "the short-lived Attract initializer still emits its actual resource event"
            );
        }
    }
    assert!(
        temporary_classes.contains(&6) && temporary_classes.contains(&45),
        "exercise real Wander and Attract constructors before death: {temporary_classes:?}"
    );
}

#[v2k_test_support::retail_test]
fn carried_expiry_invalid_death_cue_blocks_before_scheduler_release_and_rng() {
    for detailed in [false, true] {
        for invalid_cue in [None, Some(95)] {
            let (session, mut manager, owner) = super::tests::carried_fixture();
            let id = owner.entity_id();
            let extent = session
                .cache
                .global_model(LEVEL_ONE_TYPE9_MODEL_ID)
                .unwrap()
                .radius;
            let sea = session.cache.level_terrain().unwrap().sea_level_raw();
            let entity = manager.ordinary_type9_selected_entity_mut(id).unwrap();
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_SURFACE_OWNER_DISABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                if detailed {
                    SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT
                } else {
                    0
                },
            );
            let position = entity.position_raw();
            entity.set_motion_raw(
                [position[0], sea - (extent >> 2) as i16 - 1, position[2]],
                [1_000, 400, -700],
            );
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4_980);
            entity.collision.death_sound_id = RetailRuntimeValue::Known(invalid_cue);
            let before_collision = entity.collision.clone();
            let before_position = entity.position_raw();
            let before_velocity = entity.velocity_raw();
            let before_animation = entity.actor_animation_runtime;
            let before_basis = entity.physical_body_basis_q31;
            let before_sub_a = entity.sub_a_propulsion_runtime;
            let before_context = entity.current_behavior_context;
            let before_parent = entity.attached_to;
            let before_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
            let mut effects = WorldFx::new();
            let before_rng = *effects.entity_construction_state().0;
            let mut notifications = GameplayNotifications::new();
            let tick = tick_type9_carried_owner(
                &mut manager,
                owner,
                Type9CarriedProductionFrame {
                    resources: &session.cache,
                    world_fx: &mut effects,
                    notifications: &mut notifications,
                    elapsed_micros: 20_000,
                    retail_tick: 41,
                },
                &mut |_| panic!("invalid death cue must block before scheduler RNG"),
            );
            assert_eq!(
                tick.outcome,
                Type9CarriedProductionOutcome::Blocked {
                    entity_id: id,
                    reason: Type9CarriedProductionBlock::SurfaceLifecycleUnsupported,
                }
            );
            assert!(tick.class14_task_lease.is_none());
            let retained = tick.retained_owner.expect("no release consumed custody");
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(retained.authenticates(entity));
            assert_eq!(entity.collision, before_collision);
            assert_eq!(entity.position_raw(), before_position);
            assert_eq!(entity.velocity_raw(), before_velocity);
            assert_eq!(entity.actor_animation_runtime, before_animation);
            assert_eq!(entity.physical_body_basis_q31, before_basis);
            assert_eq!(entity.sub_a_propulsion_runtime, before_sub_a);
            assert_eq!(entity.current_behavior_context, before_context);
            assert_eq!(entity.attached_to, before_parent);
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                before_primary
            );
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(4_980)
            );
            assert_eq!(*effects.entity_construction_state().0, before_rng);
            assert_eq!(effects.particle_count(), 0);
            effects.process_pending();
            assert!(effects.take_positional_sounds().is_empty());
            assert_eq!(notifications.save_tail_seen_mask(), 0);
        }
    }
}

#[v2k_test_support::retail_test]
fn level1_materialiser_carried_expiry_hands_off_scheduler_and_retires_parent_row() {
    use crate::entity::{
        BeamCommand, BeamOutcome, CargoDropContext, LateTailMaterialiserFrame, PlayerCargoFrame,
    };
    for extra_ms in [0, 1] {
        let (mut session, mut manager, owner) = super::tests::carried_fixture();
        let id = owner.entity_id();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.register_type9_carried(owner).unwrap();
        let mut fx = WorldFx::new();
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = crate::static_damage::StaticDamageScheduler::new();
        // The same authored Level1 seabed used by NoCD03. This controlled
        // expiry extends beyond its observed 545ms carried window; no capture
        // value supplies a gameplay timer, task choice or runtime trajectory.
        manager
            .player_mut()
            .unwrap()
            .set_position_raw([24_613, -622, -25_925]);
        manager.queue_beam(BeamCommand::Drop);
        let mut proxy = None;
        for tick in 0..6 {
            let cargo = manager.update_player_cargo(PlayerCargoFrame {
                elapsed_micros: 20_000,
                drop_context: Some(CargoDropContext {
                    carrier_orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                    terrain: session.cache.terrain().unwrap(),
                }),
                retail_tick: tick,
                scheduler: &mut scheduler,
                world_fx: &mut fx,
                notifications: &mut notifications,
            });
            assert!(
                cargo.blocked.is_empty(),
                "real beam release: {:?}",
                cargo.blocked
            );
            if let Some(beam) = cargo.beam {
                let BeamOutcome::DropStarted { cargo_id, proxy_id } = beam else {
                    panic!("drop: {beam:?}");
                };
                assert_eq!(cargo_id, id);
                proxy = Some(proxy_id);
            }
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
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
            assert!(pass.block.is_none());
            assert!(pass.outcomes.iter().any(|outcome| matches!(outcome,
                SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(
                    Type9CarriedProductionOutcome::Continuing { entity_id, .. }
                    | Type9CarriedProductionOutcome::SchedulerWaiting { entity_id }
                ) if *entity_id == id)));
            let blocked = manager.update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 20_000,
                terrain: session.cache.terrain().unwrap(),
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut notifications,
                retail_tick: tick,
            });
            assert!(blocked.is_empty(), "native Type93 pose: {blocked:?}");
            if proxy.is_some() {
                break;
            }
        }
        let proxy_id = proxy.expect("native Type93 constructed by the beam");
        let child = manager.entity_mut(id).unwrap();
        assert_eq!(child.attached_to, Some(proxy_id));
        assert!(
            child.position_raw()[1] < session.cache.terrain().unwrap().sea_level_raw() - (165 >> 2)
        );
        child.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4_980 + extra_ms);
        child.collision.state_flags_at_0x08.overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
        child.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        let position_before = child.position_raw();
        let none = child
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let pass = scheduler.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 40,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none());
        assert!(pass.outcomes.iter().any(|outcome| matches!(outcome,
            SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(
                Type9CarriedProductionOutcome::Class14Published { entity_id, callback_elapsed_micros: 20_000 }
            ) if *entity_id == id)), "expiry outcome: {:?}", pass.outcomes);
        assert_eq!(
            scheduler.family_for(id),
            Some(SpecializedActorTaskFamily::OrdinaryType9Class14)
        );
        assert_eq!(scheduler.registered_len(), 1);
        let child = manager.entity_mut(id).unwrap();
        assert_eq!(child.attached_to, None);
        assert_eq!(
            child.position_raw(),
            position_before,
            "old AD visit still omits DF70"
        );
        assert_ne!(
            child.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(none)
        );
        assert!(
            matches!(child.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms() == 0)
        );
        assert!(
            matches!(&manager.entity_mut(proxy_id).unwrap().sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(rows)) if rows.ordered_entity_ids() == [id]),
            "16750 does not compact its old parent row"
        );
        let blocked = manager.update_late_tail_materialisers(LateTailMaterialiserFrame {
            elapsed_micros: 20_000,
            terrain: session.cache.terrain().unwrap(),
            world_fx: &mut fx,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            retail_tick: 40,
        });
        assert!(blocked.is_empty(), "dying row compaction: {blocked:?}");
        assert!(
            matches!(&manager.entity_mut(proxy_id).unwrap().sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(rows)) if rows.is_empty())
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().position_raw(),
            position_before,
            "retired row cannot copy its parent again"
        );

        let mut class14_visits = 0;
        for tick in 41..105 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase:
                        crate::gameplay_notifications::GameplayNotificationPhase::Playing,
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
            assert!(pass.block.is_none());
            for outcome in pass.outcomes {
                assert!(
                    !matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::OrdinaryType9Carried(_)
                    ),
                    "consumed None owner never returns"
                );
                assert!(
                    !format!("{outcome:?}").contains("Blocked"),
                    "class14 successor: {outcome:?}"
                );
                if matches!(
                    outcome,
                    SpecializedActorTaskProductionOutcome::OrdinaryType9Class14(_)
                ) {
                    class14_visits += 1;
                }
            }
            let blocked = manager.update_late_tail_materialisers(LateTailMaterialiserFrame {
                elapsed_micros: 20_000,
                terrain: session.cache.terrain().unwrap(),
                world_fx: &mut fx,
                scheduler: &mut scheduler,
                notifications: &mut notifications,
                retail_tick: tick,
            });
            assert!(
                blocked.is_empty(),
                "empty materialiser retirement: {blocked:?}"
            );
            manager.cleanup_pending_actor_deferred_destroys();
        }
        assert!(
            class14_visits > 40,
            "class14 must run its actual timed terminal path"
        );
        assert!(!manager
            .iter_all()
            .any(|entity| entity.id == id || entity.id == proxy_id));
        assert_eq!(scheduler.family_for(id), None);
    }
}
