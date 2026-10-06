use super::*;
use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::intro2_gun_turret::native::tests::{fixture, generic, publish};

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
    owner: Intro2GunTurretOwner,
    cache: &mut ResourceCache,
    fx: &mut WorldFx,
    dt: u32,
) -> Intro2GunTurretTick {
    tick_intro2_gun_turret(
        manager,
        owner,
        Intro2GunTurretFrame {
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            resources: cache,
            world_fx: fx,
            elapsed_micros: dt,
            retail_tick: 1000,
        },
    )
}

#[v2k_test_support::retail_test]
fn native_type102_both_callback_modes_keep_fixed_body_and_ground_tail() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    for spawn in [53, 54] {
        for detailed in [true, false] {
            let mut manager = generic(&session, &metadata);
            let id = publish(&mut manager, &session, &metadata, spawn);
            enabled(&mut manager, id, detailed);
            let entity = manager.entity_mut(id).unwrap();
            let position = entity.position_raw();
            let basis = entity.physical_body_basis_q31();
            entity.set_position_raw([position[0], position[1] + 100, position[2]]);
            entity.set_velocity_raw([250, -300, 800]);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(17);
            entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(250);
            let owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
            let tick = step(
                &mut manager,
                owner,
                &mut session.cache,
                &mut WorldFx::new(),
                125_000,
            );
            assert_eq!(
                tick.outcome,
                Intro2GunTurretOutcome::Advanced {
                    entity_id: id,
                    callback_enabled: true,
                    callback_elapsed_micros: 125_000,
                }
            );
            assert!(tick.retained_owner.is_some());
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.position_raw(), position);
            assert_eq!(entity.velocity_raw(), [0; 3]);
            assert_eq!(entity.physical_body_basis_q31(), basis);
            assert_eq!(entity.mass_raw, 117);
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(
                entity.surface_lifetime_timer_ms_at_0x48,
                RetailRuntimeValue::Known(125)
            );
            let Some(ActorTaskRuntime::Intro2GunTurret(task)) =
                entity.actor_task_state(ActorTaskSlot::Tertiary)
            else {
                panic!("turret task");
            };
            assert_eq!(task.elapsed_ms(), 125);
        }
    }
}

#[v2k_test_support::retail_test]
fn flower_idle_coarse_callback_skips_clock_then_detailed_callback_advances_it() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 115)
        .unwrap()
        .id;
    super::super::publish_intro2_gun_turret(
        manager.entity_mut(id).unwrap(),
        &metadata[115],
        session.cache.terrain().unwrap(),
        &mut || 0xffff,
    )
    .unwrap();
    let mut owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    enabled(&mut manager, id, false);
    let coarse = step(&mut manager, owner, &mut session.cache, &mut fx, 125_000);
    assert!(matches!(
        coarse.outcome,
        Intro2GunTurretOutcome::Advanced { .. }
    ));
    owner = coarse.retained_owner.unwrap();
    assert!(
        matches!(manager.entity_mut(id).unwrap().actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(timer)) if timer.elapsed_ms() == 0)
    );
    enabled(&mut manager, id, true);
    let detailed = step(&mut manager, owner, &mut session.cache, &mut fx, 125_000);
    assert!(matches!(
        detailed.outcome,
        Intro2GunTurretOutcome::Advanced { .. }
    ));
    assert!(detailed.retained_owner.is_some());
    let entity = manager.entity_mut(id).unwrap();
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::Class0Timer(timer)) if timer.elapsed_ms() == 125)
    );
    assert_eq!(entity.position_raw(), [-16384, 128, 2816]);
}

#[v2k_test_support::retail_test]
fn native_type102_failed_tail_retains_prefix_without_repeating_task_or_random_stream() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    enabled(&mut manager, id, true);
    manager
        .entity_mut(id)
        .unwrap()
        .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Unresolved;
    let owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let first = step(&mut manager, owner, &mut session.cache, &mut fx, 40_000);
    assert!(
        matches!(
            first.outcome,
            Intro2GunTurretOutcome::Blocked {
                prefix_committed: true,
                reason: Intro2GunTurretBlock::Runtime("surface timer"),
                ..
            }
        ),
        "{:?}",
        first.outcome
    );
    let entity = manager.entity_mut(id).unwrap();
    let state = entity.intro2_gun_turret_runtime;
    let Some(ActorTaskRuntime::Intro2GunTurret(task)) =
        entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("task");
    };
    let age = task.elapsed_ms();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let second = step(
        &mut manager,
        first.retained_owner.unwrap(),
        &mut session.cache,
        &mut fx,
        40_000,
    );
    assert_eq!(
        second.outcome,
        Intro2GunTurretOutcome::Pending { entity_id: id }
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.intro2_gun_turret_runtime, state);
    let Some(ActorTaskRuntime::Intro2GunTurret(task)) =
        entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("task");
    };
    assert_eq!(task.elapsed_ms(), age);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
