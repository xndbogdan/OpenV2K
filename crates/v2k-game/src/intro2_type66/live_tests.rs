use super::*;
use crate::{
    entity::BaseFactoryRuntimeState,
    intro2_type66::{
        native::tests::{fixture, generic, publish},
        INITIAL_HEALTH_RAW,
    },
    static_damage::StaticDamageScheduler,
};

fn base(manager: &EntityManager, id: u32) -> BaseFactoryRuntimeState {
    let RetailRuntimeValue::Known(Some(base)) = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .base_factory_runtime
    else {
        panic!("native Sub-M")
    };
    base
}

fn enabled(manager: &mut EntityManager, id: u32, detailed: bool) {
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
}

fn step(
    manager: &mut EntityManager,
    owner: Intro2Type66Owner,
    cache: &mut ResourceCache,
    fx: &mut WorldFx,
    dt: u32,
    now: u32,
) -> Intro2Type66Tick {
    let world_style_raw = cache.level_desc().unwrap().world_style;
    tick_intro2_type66_owner(
        manager,
        owner,
        Intro2Type66Frame {
            resources: cache,
            world_fx: fx,
            notifications: &mut GameplayNotifications::new(),
            static_damage: &mut StaticDamageScheduler::new(),
            elapsed_micros: dt,
            retail_tick: now,
            world_style_raw,
            main_base_abort_active: false,
        },
    )
}

#[v2k_test_support::retail_test]
fn ordinary_autonomous_factory_publishes_real_pickup_and_waits_for_collection() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    session.load_level_by_id(18, 1).unwrap();
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 5);
    enabled(&mut manager, id, true);
    let mut fx = WorldFx::new();
    let mut owner = Intro2Type66Owner::adopt(&manager, id).unwrap();
    let before_count = manager.iter_all().count();
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut fx,
        125_000,
        1000,
    );
    assert!(
        matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    owner = tick.retained_owner.unwrap();
    //12DA0 caps each callback delta at125000us even when random waits are
    // disabled. The authored250000us threshold therefore needs two visits.
    let half = base(&manager, id).production.unwrap();
    assert_eq!(
        half.phase,
        crate::factory_production::FactoryProductionPhase::Producing
    );
    assert_eq!(half.production_progress_micros_raw, 125_000);
    assert_eq!(manager.iter_all().count(), before_count);
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut fx,
        125_000,
        1006,
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type66Outcome::Advanced {
                callback_elapsed_micros: 125_000,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    owner = tick.retained_owner.unwrap();
    let production = base(&manager, id).production.unwrap();
    assert_eq!(
        production.phase,
        crate::factory_production::FactoryProductionPhase::Delivering
    );
    assert_eq!(manager.iter_all().count(), before_count + 1);
    let product = manager
        .iter_all()
        .find(|entity| entity.id == production.spawned_pickup_handle)
        .unwrap();
    assert_eq!(product.entity_type, 61);
    assert_eq!(
        product.power_up_payload_packed,
        Some(production.output_payload_packed)
    );
    assert_eq!(
        product.collision.recent_relation_id_at_0x60,
        RetailRuntimeValue::Known(Some(id))
    );
    let pickup_id = product.id;
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut fx,
        125_000,
        1012,
    );
    assert!(
        matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    owner = tick.retained_owner.unwrap();
    assert_eq!(
        base(&manager, id).production.unwrap().phase,
        crate::factory_production::FactoryProductionPhase::Delivering
    );
    assert_eq!(
        base(&manager, id)
            .production
            .unwrap()
            .delivery_progress_micros_raw,
        125_000
    );
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut fx,
        125_000,
        1018,
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type66Outcome::Advanced {
                callback_elapsed_micros: 125_000,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    assert_eq!(
        base(&manager, id).production.unwrap().phase,
        crate::factory_production::FactoryProductionPhase::WaitingForPickup
    );
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut fx,
        250_000,
        1024,
    );
    assert!(
        matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    assert_eq!(
        base(&manager, id).production.unwrap().spawned_pickup_handle,
        pickup_id
    );
    assert_eq!(manager.iter_all().count(), before_count + 1);
}

#[v2k_test_support::retail_test]
fn ordinary_convex_factory_staged_death_reaches_the_authored_wreck() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    session.load_level_by_id(25, 1).unwrap();
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 0);
    assert_eq!(manager.entity_mut(id).unwrap().model_index, Some(195));
    enabled(&mut manager, id, true);
    let mut fx = WorldFx::new();
    let death =
        super::super::death::publish_intro2_type66_standard_death(&mut manager, id, &mut fx)
            .unwrap();
    let mut owner = death.owner.unwrap();
    let mut effect_count = 0;
    for frame in 0..26 {
        let tick = step(
            &mut manager,
            owner,
            &mut session.cache,
            &mut fx,
            125_000,
            1000 + frame * 6,
        );
        assert!(
            matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
            "frame {frame}: {:?}",
            tick.outcome
        );
        effect_count += tick.explosion_lights.len();
        owner = tick.retained_owner.unwrap();
    }
    assert!(
        effect_count > 0,
        "stage31 visits every sphere, including model195's convex children"
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.model_index, Some(225));
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Class0Timer(_))
    ));
    assert_eq!(base(&manager, id).progressive_death.elapsed_micros_raw, -1);
}

#[v2k_test_support::retail_test]
fn native_type66_zero_threshold_templates_advance_both_modes_without_product_rng() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for spawn in [36, 51] {
        for detailed in [true, false] {
            let mut manager = generic(&session, &metadata);
            let id = publish(&mut manager, &session, &metadata, spawn);
            enabled(&mut manager, id, detailed);
            let entity = manager.entity_mut(id).unwrap();
            let position = entity.position_raw();
            let basis = entity.physical_body_basis_q31();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(17);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(250);
            let old = base(&manager, id);
            let mut fx = WorldFx::new();
            let mut expected = WorldFx::new();
            let owner = Intro2Type66Owner::adopt(&manager, id).unwrap();
            let tick = step(
                &mut manager,
                owner,
                &mut session.cache,
                &mut fx,
                125_000,
                1000,
            );
            assert_eq!(
                tick.outcome,
                Intro2Type66Outcome::Advanced {
                    entity_id: id,
                    callback_enabled: true,
                    callback_elapsed_micros: 125_000
                }
            );
            assert!(tick.retained_owner.is_some());
            assert!(tick.explosion_lights.is_empty());
            assert!(!tick.progressive_death_presentation_requested);
            let after = base(&manager, id);
            assert_eq!(after.production.unwrap().production_threshold_micros_raw, 0);
            assert_eq!(after.production.unwrap().spawned_pickup_handle, 0);
            assert_eq!(
                after.production.unwrap().remaining_stock_raw,
                old.production.unwrap().remaining_stock_raw
            );
            assert_eq!(
                after.live_owner.unwrap().state_version,
                old.live_owner.unwrap().state_version + 1
            );
            assert_eq!(
                after.live_owner.unwrap().next_transaction_id_raw,
                old.live_owner.unwrap().next_transaction_id_raw + 2
            );
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.position_raw(), position);
            assert_eq!(entity.physical_body_basis_q31(), basis);
            assert_eq!(entity.mass_raw, 1017);
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(125)
            );
            if !detailed {
                //12DA0 zero gate reloads +70 then+6C. Factory itself has no
                // production or effect draws on these authored zero thresholds.
                expected.next_shared_retail_random_u16();
                expected.next_shared_retail_random_u16();
            }
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type66_factory_repairs_but_unstaffed_hut_retains_damage() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for (spawn, repaired) in [(36, 0), (51, ((40_000_i32 >> 8) * 1666) >> 12)] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, spawn);
        enabled(&mut manager, id, true);
        manager.entity_mut(id).unwrap().collision.health_raw =
            RetailRuntimeValue::Known(INITIAL_HEALTH_RAW - 100);
        let owner = Intro2Type66Owner::adopt(&manager, id).unwrap();
        let tick = step(
            &mut manager,
            owner,
            &mut session.cache,
            &mut WorldFx::new(),
            40_000,
            1000,
        );
        assert!(
            matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
            "{tick_outcome:?}",
            tick_outcome = tick.outcome
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(INITIAL_HEALTH_RAW - 100 + repaired)
        );
        assert_eq!(
            base(&manager, id).live_owner.unwrap().animation_state_raw,
            if spawn == 36 { 0 } else { 5 }
        );
        assert_eq!(base(&manager, id).progressive_death.elapsed_micros_raw, 0);
    }
}

#[v2k_test_support::retail_test]
fn native_type66_fast_and_ordinary_death_retain_dormant_next_primary() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for (spawn, frames_to_terminal) in [(36, 2), (51, 26)] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, spawn);
        enabled(&mut manager, id, true);
        let mut fx = WorldFx::new();
        session
            .cache
            .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
            .unwrap();
        let first =
            super::super::death::publish_intro2_type66_standard_death(&mut manager, id, &mut fx)
                .unwrap();
        assert!(!first.returned_nonzero);
        let mut owner = first.owner.unwrap();
        let old_primary = owner.primary;
        let allocation = base(&manager, id).live_owner.unwrap().allocation_identity;
        for index in 0..frames_to_terminal {
            let tick = step(
                &mut manager,
                owner,
                &mut session.cache,
                &mut fx,
                125_000,
                1000 + index * 6,
            );
            assert!(
                matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
                "spawn={spawn} frame={index}: {:?}",
                tick.outcome
            );
            owner = tick
                .retained_owner
                .expect("native allocation retains class39 or class0");
            if index + 1 < frames_to_terminal {
                assert!(base(&manager, id).progressive_death.elapsed_micros_raw > 0);
            } else {
                assert!(tick.progressive_death_presentation_requested);
                assert_eq!(tick.terrain_changed, spawn == 36);
            }
        }
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity.collision.active_model_slot(),
            RetailRuntimeValue::Known(1)
        );
        assert_eq!(
            entity.actor_tasks.wrapper_flags(old_primary),
            None,
            "terminal DB80 retires and unwinds the executing wrapper"
        );
        assert!(
            matches!(entity.actor_tasks.task_state(owner.primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0),
            "new Primary waits for the next pass"
        );
        assert_eq!(
            base(&manager, id).live_owner.unwrap().allocation_identity,
            allocation
        );
        assert_eq!(base(&manager, id).progressive_death.elapsed_micros_raw, -1);
        assert_eq!(
            base(&manager, id)
                .production
                .unwrap()
                .scientist_capacity_raw,
            0
        );
        let mut expected = fx.fork_for_main_base_abort_transaction();
        let tick = step(
            &mut manager,
            owner,
            &mut session.cache,
            &mut fx,
            40_000,
            2000,
        );
        assert!(matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }));
        assert!(!tick.progressive_death_presentation_requested);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16(),
            "dormant tick does not rerun stages or selector"
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type66_late_effect_basis_failure_keeps_elapsed_and_does_not_replay() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 51);
    enabled(&mut manager, id, true);
    let mut fx = WorldFx::new();
    let owner =
        super::super::death::publish_intro2_type66_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .owner
            .unwrap();
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut fx,
        125_000,
        1000,
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type66Outcome::Blocked {
                reason: Intro2Type66Block::Runtime("staged effect basis"),
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let retained = tick.retained_owner.unwrap();
    assert!(retained.has_pending_prefix());
    let before = base(&manager, id);
    assert_eq!(before.progressive_death.elapsed_micros_raw, 125_001);
    assert!(
        matches!(manager.entity_mut(id).unwrap().actor_tasks.task_state(owner.primary), Some(ActorTaskRuntime::WorkingFactory(task)) if task.elapsed_ms() == 125)
    );
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let retry = step(
        &mut manager,
        retained,
        &mut session.cache,
        &mut fx,
        125_000,
        1006,
    );
    assert_eq!(
        retry.outcome,
        Intro2Type66Outcome::Pending { entity_id: id }
    );
    assert_eq!(base(&manager, id), before);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type66_stale_task_receipt_cannot_mutate_replacement() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 51);
    enabled(&mut manager, id, true);
    let stale = Intro2Type66Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let replacement =
        super::super::death::publish_intro2_type66_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .owner
            .unwrap();
    let before = base(&manager, id);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let tick = step(
        &mut manager,
        stale,
        &mut session.cache,
        &mut fx,
        125_000,
        1000,
    );
    assert_eq!(tick.outcome, Intro2Type66Outcome::Dropped { entity_id: id });
    assert_eq!(base(&manager, id), before);
    assert!(replacement.authenticates(&manager));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type66_disabled_callback_clears_b2_without_advancing_factory() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 51);
    enabled(&mut manager, id, true);
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, 0);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
    let owner = Intro2Type66Owner::adopt(&manager, id).unwrap();
    let before = base(&manager, id);
    let tick = step(
        &mut manager,
        owner,
        &mut session.cache,
        &mut WorldFx::new(),
        40_000,
        1000,
    );
    assert_eq!(
        tick.outcome,
        Intro2Type66Outcome::Advanced {
            entity_id: id,
            callback_enabled: false,
            callback_elapsed_micros: 40_000
        }
    );
    assert_eq!(base(&manager, id), before);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(0)
    );
    assert!(
        matches!(entity.actor_tasks.task_state(owner.primary), Some(ActorTaskRuntime::WorkingFactory(task)) if task.elapsed_ms() == 0)
    );
}

#[v2k_test_support::retail_test]
fn native_type66_dormant_timeout_is_strict_and_reselects_after_unwind_without_rng() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 51);
    enabled(&mut manager, id, true);
    let entity = manager.entity_mut(id).unwrap();
    // Controlled terminal suffix fixture: death tests own19750's prefix.
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::DYING_STATE_BIT,
        crate::entity_collision_state::DYING_STATE_BIT,
    );
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    let mut owner = super::super::death::reselect_intro2_type66_dormant(&mut manager, id).unwrap();
    let original = owner.primary;
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    for index in 0..72 {
        let tick = step(
            &mut manager,
            owner,
            &mut session.cache,
            &mut fx,
            125_000,
            1000 + index * 6,
        );
        assert!(
            matches!(tick.outcome, Intro2Type66Outcome::Advanced { .. }),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
        assert_eq!(owner.primary, original);
    }
    assert!(
        matches!(manager.entity_mut(id).unwrap().actor_tasks.task_state(original), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 9000)
    );
    let tick = step(&mut manager, owner, &mut session.cache, &mut fx, 1000, 1500);
    let owner = tick.retained_owner.unwrap();
    assert_ne!(owner.primary, original);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.actor_tasks.wrapper_flags(original), None);
    assert!(
        matches!(entity.actor_tasks.task_state(owner.primary), Some(ActorTaskRuntime::Class0Timer(task)) if task.elapsed_ms() == 0)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
