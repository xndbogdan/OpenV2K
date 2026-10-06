use super::*;
use crate::{
    damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET},
    entity_collision_state::{DYING_STATE_BIT, REMOTE_OWNED_STATE_BIT},
    entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    intro2_type47_live::world::native_intro2_fixture,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact, ParticleEnvironment},
};

fn id(manager: &EntityManager, spawn: usize) -> u32 {
    manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(spawn))
        .unwrap()
        .id
}
fn hit(id: u32, infected: bool, lethal: bool) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: if infected { 5 } else { 38 },
        impact_position_argument_va: if infected { 0x004d_cf48 } else { 0 },
        target_entity_id: id,
        position_world: [0.; 3],
        velocity_raw: [-2303, 297, -182],
        damage: Some(BallisticDamageRequest {
            packet: if infected {
                FUN_0043F780_DAMAGE_PACKET
            } else if lethal {
                DamagePacket {
                    channels: [2, 3],
                    amounts_raw: [500, 6000],
                }
            } else {
                DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [250, 0],
                }
            },
            source_entity_type_at_birth: Some(10),
            source_owner_id: Some(56),
        }),
    }
}

#[v2k_test_support::retail_test]
fn native_type8_births_keep_their_own_seeds_silent_animation_and_first_mass() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    for (spawn, seed, x, z) in [
        (12, 0x0c, 83, 227),
        (13, 0x0d, 80, 228),
        (15, 0x0e, 87, 232),
    ] {
        let entity = manager.entity_mut(id(&manager, spawn)).unwrap();
        assert!(intro2_type8_allocation_authenticates(entity));
        let receipt = entity.intro2_type8_runtime.unwrap();
        assert_eq!(receipt.sub_d_seed, seed);
        assert_eq!(
            [receipt.anchor_raw[0], receipt.anchor_raw[2]],
            [(x << 8) as i16, (z << 8) as i16]
        );
        assert_eq!(entity.model_slots, [Some(559); 4]);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert_eq!(animation.descriptor().capability_bit_3_sound_id, 0);
        assert_eq!(animation.descriptor().attention_stop_sound_id, 0);
        let owner = Intro2Type8Owner::take_birth(entity).unwrap();
        assert!(Intro2Type8Owner::take_birth(entity).is_none());
        assert_eq!(owner.entity_id(), entity.id);
        entity.authored_spawn_index = Some(2);
        assert!(
            !intro2_type8_allocation_authenticates(entity),
            "Type9 birth cannot authenticate this receipt"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type8_primary_and_infected_hits_replace_live_graph_without_fixed_actor_fallback() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert_eq!(scheduler.adopt_intro2_type8(&mut manager), 3);
    let id = id(&manager, 12);
    let mut fx = WorldFx::new();
    for (tick, infected) in [(20, false), (21, true), (22, true), (23, false)] {
        let entity = manager.entity_mut(id).unwrap();
        let before = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
        let old_stamp = entity.collision.last_hit_presentation_tick_at_0x34;
        let result = impact::apply_intro2_type8_particle_hit(
            &mut manager,
            &mut fx,
            &mut scheduler,
            hit(id, infected, false),
            tick,
        );
        assert!(
            matches!(result, impact::Intro2Type8ImpactOutcome::Applied(_)),
            "{result:?}"
        );
        let entity = manager.entity_mut(id).unwrap();
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            before
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            if infected {
                old_stamp
            } else {
                RetailRuntimeValue::Known(tick)
            }
        );
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(if tick < 23 { 1450 } else { 1400 })
        );
        assert!(scheduler.begin_intro2_type8_external_mutation(&manager, id));
    }
}

#[v2k_test_support::retail_test]
fn native_type8_lethal_dragon_hit_and_later_hit_keep_one_class14_task() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_intro2_type8(&mut manager);
    let id = id(&manager, 12);
    let mut fx = WorldFx::new();
    let result = impact::apply_intro2_type8_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        hit(id, false, true),
        3246,
    );
    let impact::Intro2Type8ImpactOutcome::Applied(result) = result else {
        panic!("{result:?}")
    };
    assert_eq!(result.filtered_damage_raw, 12300);
    assert!(result.death_publication.is_some());
    let entity = manager.entity_mut(id).unwrap();
    let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(DYING_STATE_BIT | 0x8000),
        RetailRuntimeValue::Known(DYING_STATE_BIT)
    );
    let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
        panic!()
    };
    assert!(animation.special_mode());
    let result = impact::apply_intro2_type8_particle_hit(
        &mut manager,
        &mut fx,
        &mut scheduler,
        hit(id, false, true),
        3247,
    );
    assert!(
        matches!(result, impact::Intro2Type8ImpactOutcome::Applied(_)),
        "{result:?}"
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary),
        task
    );
    assert_eq!(
        fx.pending_event_count(),
        0,
        "Type8 does not inherit peasant death sound/C6"
    );
}

#[v2k_test_support::retail_test]
fn native_type8_standard_death_preserves_committed_radial_prefix_and_draws_once() {
    let Some((_session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = id(&manager, 12);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.health_raw = RetailRuntimeValue::Known(-6200);
    entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(71);
    entity.set_velocity_raw([37, 42, -51]);
    let mut fx = WorldFx::new();
    let mut control = WorldFx::new();
    let result = impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(result.returned_nonzero && result.publication.is_some());
    control.next_shared_retail_random_u16();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.velocity_raw(), [37, 42, -51]);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(71)
    );
    let again = impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx).unwrap();
    assert!(again.returned_nonzero && again.publication.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type8_class14_runs_both_modes_and_removes_only_after_strict_one_second() {
    for detailed in [false, true] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = id(&manager, 12);
        let mut fx = WorldFx::new();
        let mut owner = impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .publication
            .unwrap();
        let animation_at_death = manager.entity_mut(id).unwrap().actor_animation_runtime;
        for n in 1..=9 {
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
                RetailRuntimeValue::Known(125_001);
            let tick = tick_intro2_type8(
                &mut manager,
                owner,
                Intro2Type8Frame {
                    resources: &session.cache,
                    world_fx: &mut fx,
                    elapsed_micros: 0,
                    global_elapsed_micros: 20_000,
                    retail_tick: n,
                },
            );
            assert!(
                matches!(tick.outcome,Intro2Type8Outcome::Advanced{terminal,..}if terminal==(n==9)),
                "{n}: {:?}",
                tick.outcome
            );
            if n == 1 {
                let animation = manager.entity_mut(id).unwrap().actor_animation_runtime;
                if detailed {
                    let RetailRuntimeValue::Known(Some(a)) = animation else {
                        panic!()
                    };
                    assert_eq!(a.output(), 39);
                } else {
                    assert_eq!(animation, animation_at_death);
                }
            }
            if n == 9 {
                assert!(tick.retained_owner.is_none());
                break;
            }
            owner = tick.retained_owner.unwrap();
        }
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
        assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), vec![id]);
    }
}

#[v2k_test_support::retail_test]
fn native_type8_surface_lifecycle_publishes_class14_and_rejects_wrong_receipt() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = id(&manager, 12);
    let metadata = manager.type_runtime_metadata(8).unwrap().clone();
    let mut fx = WorldFx::new();
    let mut death = None;
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw([0, -10_000, 0]);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(REMOTE_OWNED_STATE_BIT | 0x2000_0000, 0);
    entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4999);
    let frame = || crate::intro2_common_dying::Intro2ActorSurfaceFrame {
        metadata: &metadata,
        terrain: session.cache.level_terrain().unwrap(),
        active_model_extent_raw: session.cache.global_model(559).unwrap().radius,
        elapsed_micros: 2000,
        retail_tick: 17,
        particle_environment: ParticleEnvironment::Dry,
    };
    crate::intro2_common_dying::run_actor_surface_with_death(
        &mut manager,
        id,
        frame(),
        &mut fx,
        &mut death,
        intro2_type8_allocation_authenticates,
        |m, id, fx| {
            impact::run_intro2_type8_standard_death(m, id, fx)
                .map(|r| r.publication)
                .map_err(|_| crate::intro2_common_dying::Intro2CommonDyingBlock::Runtime("death"))
        },
    )
    .unwrap();
    assert!(death.is_some());
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .surface_lifetime_timer_ms_at_0x48,
        RetailRuntimeValue::Known(5001)
    );
    manager.entity_mut(id).unwrap().intro2_type8_runtime = None;
    let before = manager
        .entity_mut(id)
        .unwrap()
        .surface_lifetime_timer_ms_at_0x48;
    let result = crate::intro2_common_dying::run_actor_surface_with_death(
        &mut manager,
        id,
        frame(),
        &mut fx,
        &mut death,
        intro2_type8_allocation_authenticates,
        |_, _, _| panic!("invalid receipt must not call death"),
    );
    assert_eq!(
        result,
        Err(crate::intro2_common_dying::Intro2CommonDyingBlock::UnauthenticatedAllocation)
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .surface_lifetime_timer_ms_at_0x48,
        before
    );
}

#[v2k_test_support::retail_test]
fn native_type8_blocked_class14_visit_keeps_elapsed_retarget_rng_and_never_replays() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let id = id(&manager, 12);
    let mut fx = WorldFx::new();
    let owner = impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx)
        .unwrap()
        .publication
        .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(None);
    let mut fx = WorldFx::new();
    let mut control = WorldFx::new();
    let tick = tick_intro2_type8(
        &mut manager,
        owner,
        Intro2Type8Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 40_000,
            global_elapsed_micros: 40_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type8Outcome::Blocked {
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        panic!()
    };
    assert_eq!(task.elapsed_ms(), 40);
    assert_ne!(
        task.private_state().target_position_raw,
        entity.position_raw()
    );
    let retained = *task;
    control.next_shared_retail_random_u16();
    control.next_shared_retail_random_u16();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
    let tick = tick_intro2_type8(
        &mut manager,
        tick.retained_owner.unwrap(),
        Intro2Type8Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 40_000,
            global_elapsed_micros: 40_000,
            retail_tick: 2,
        },
    );
    assert_eq!(tick.outcome, Intro2Type8Outcome::Pending { entity_id: id });
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .state_in_slot(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::SharedRetarget(retained))
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type8_missing_death_component_keeps_10c10_prefix_and_parks_partial_graph() {
    for missing_sub_a in [false, true] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = id(&manager, 12);
        let entity = manager.entity_mut(id).unwrap();
        let mut owner = Intro2Type8Owner::take_birth(entity).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(-6200);
        if missing_sub_a {
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
        } else {
            entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
        }
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        assert!(impact::run_intro2_type8_standard_death(&mut manager, id, &mut fx).is_err());
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!();
        };
        assert_eq!(context.active_style().audited().unwrap().class_id, 14);
        if missing_sub_a {
            let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
                panic!();
            };
            assert!(
                animation.special_mode(),
                "C3A0's Sub-I prefix precedes the Sub-A boundary"
            );
            assert_eq!(
                entity.collision.state_flags_at_0x08.masked(0x8000),
                RetailRuntimeValue::Known(0)
            );
        }
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
        owner.park_external_prefix();
        let tick = tick_intro2_type8(
            &mut manager,
            owner,
            Intro2Type8Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 40_000,
                global_elapsed_micros: 40_000,
                retail_tick: 2,
            },
        );
        assert_eq!(tick.outcome, Intro2Type8Outcome::Pending { entity_id: id });
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type8_owner_rejects_same_spawn_from_another_level_allocation() {
    let Some((_session, mut first, _)) = native_intro2_fixture() else {
        return;
    };
    let Some((session, mut second, _)) = native_intro2_fixture() else {
        return;
    };
    let id = id(&first, 12);
    let owner = Intro2Type8Owner::take_birth(first.entity_mut(id).unwrap()).unwrap();
    assert_eq!(
        id,
        second
            .iter_all()
            .find(|e| e.authored_spawn_index == Some(12))
            .unwrap()
            .id
    );
    assert!(!owner.completed_hit_boundary(&second));
    let mut fx = WorldFx::new();
    let tick = tick_intro2_type8(
        &mut second,
        owner,
        Intro2Type8Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 40_000,
            global_elapsed_micros: 40_000,
            retail_tick: 2,
        },
    );
    assert_eq!(tick.outcome, Intro2Type8Outcome::Dropped { entity_id: id });
    assert!(tick.retained_owner.is_none());
}
