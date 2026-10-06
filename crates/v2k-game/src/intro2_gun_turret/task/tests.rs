use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    intro2_gun_turret::native::tests::{fixture, generic, publish},
};

fn state(manager: &EntityManager, id: u32) -> GunTurretTaskState {
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let Some(ActorTaskRuntime::Intro2GunTurret(state)) =
        entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!()
    };
    let task = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    assert!(!entity.actor_tasks.wrapper_flags(task).unwrap().in_callback);
    *state
}
fn state_mut(manager: &mut EntityManager, id: u32) -> &mut GunTurretTaskState {
    let entity = manager.entity_mut(id).unwrap();
    let task = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    let Some(ActorTaskRuntime::Intro2GunTurret(state)) = entity.actor_tasks.task_state_mut(task)
    else {
        panic!()
    };
    state
}
fn isolate(manager: &mut EntityManager, id: u32) -> [u32; 2] {
    let ids: Vec<_> = manager.retail_live_order_ids().collect();
    let targets: Vec<_> = ids.iter().copied().filter(|v| *v != id).take(2).collect();
    for candidate in ids {
        let entity = manager.entity_mut(candidate).unwrap();
        entity.capability_flags = 0;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(u32::MAX, 0x8000);
        entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        entity.set_velocity_raw([0; 3]);
        entity.set_position_raw([0, 1000, 1000]);
    }
    let owner = manager.entity_mut(id).unwrap();
    owner.capability_flags = 0x44;
    owner.set_position_raw([0, 1000, 0]);
    owner.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    });
    [targets[0], targets[1]]
}

#[test]
fn turret_constructor_and_selection_priority_tables_preserve_signed_words() {
    let state = GunTurretTaskState::from_408df0(0x44, [i16::MIN, -123], false);
    assert_eq!(state.angles_raw(), [32768, -123]);
    assert_eq!(state.target_handle(), 0);
    assert_eq!(state.selection_remaining_us(), 0);
    assert!(!state.tracking_disabled());
    assert_eq!(state.lost_target_remaining_us(), 6_000_000);
    assert!(!state.normal_uses_infected_priorities);
    assert!(GunTurretTaskState::from_408df0(0x48, [0; 2], false).normal_uses_infected_priorities);
}

#[v2k_test_support::retail_test]
fn turret_live_selection_preserves_order_priority_relation_and_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let [player, enemy] = isolate(&mut manager, id);
    manager.entity_mut(player).unwrap().capability_flags = 1;
    manager.entity_mut(enemy).unwrap().capability_flags = 8;
    let mut task = state(&manager, id);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    expected.next_shared_retail_random_u16(); //unconditional500000 timer
    let priority_word = expected.next_shared_retail_random_u16(); //enemy score>1FFFF
    acquire(&mut manager, id, &mut task, &mut fx).unwrap();
    assert_eq!(task.target_handle(), enemy);
    assert!(!task.tracking_disabled());
    assert_eq!(
        task.selection_remaining_us(),
        (i32::from(priority_word) + 0x2625a) * 16
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    //A recent relation eliminates the otherwise highest-priority candidate;
    //the player remains selected but its low normal score disables firing.
    let enemy_entity = manager.entity_mut(enemy).unwrap();
    enemy_entity.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(id));
    enemy_entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
    acquire(&mut manager, id, &mut task, &mut fx).unwrap();
    assert_eq!(task.target_handle(), player);
    assert!(task.tracking_disabled());
    manager.entity_mut(player).unwrap().capability_flags = 0;
    acquire(&mut manager, id, &mut task, &mut fx).unwrap();
    assert_eq!(
        task.target_handle(),
        player,
        "no candidate retains the old handle"
    );
}

#[v2k_test_support::retail_test]
fn turret_current_target_distance_penalty_can_select_a_farther_new_target() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let [old, new] = isolate(&mut manager, id);
    manager.entity_mut(old).unwrap().capability_flags = 8;
    manager.entity_mut(new).unwrap().capability_flags = 8;
    manager
        .entity_mut(new)
        .unwrap()
        .set_position_raw([0, 1000, 1400]);
    let mut task = state(&manager, id);
    task.target_handle = old;
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    for _ in 0..3 {
        expected.next_shared_retail_random_u16();
    }
    acquire(&mut manager, id, &mut task, &mut fx).unwrap();
    //The old1000 range becomes1500; the new1400 range wins. Each newly
    //best hostile consumes its own high-priority timer word.
    assert_eq!(task.target_handle(), new);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn turret_attached_gate_still_zeros_velocity_but_skips_private_aim_and_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 54);
    isolate(&mut manager, id);
    let original = state(&manager, id);
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x1000, 0x1000);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity.set_velocity_raw([111, 222, 333]);
    let mut fx = WorldFx::new();
    assert_eq!(
        tick_intro2_gun_turret_task(
            &mut manager,
            id,
            TurretTaskFrame {
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                elapsed_micros: 40_000,
                global_elapsed_micros: 40_000,
                retail_tick: 75,
            },
            &mut fx
        ),
        Ok(TurretTaskOutcome::Continue)
    );
    let mut expected = original;
    expected.elapsed_ms = 40;
    assert_eq!(state(&manager, id), expected);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.velocity_raw(), [0; 3]);
    assert!(entity.intro2_gun_turret_aim_runtime.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn turret_countdown_equality_retargets_after_unwind_without_aim_or_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 54);
    isolate(&mut manager, id);
    let task = state_mut(&mut manager, id);
    task.selection_remaining_us = 40_000;
    task.tracking_disabled = true;
    task.lost_target_remaining_us = 40_000;
    manager
        .entity_mut(id)
        .unwrap()
        .set_velocity_raw([100, -200, 300]);
    let mut fx = WorldFx::new();
    assert_eq!(
        tick_intro2_gun_turret_task(
            &mut manager,
            id,
            TurretTaskFrame {
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                elapsed_micros: 40_000,
                global_elapsed_micros: 14_000,
                retail_tick: 75
            },
            &mut fx
        ),
        Ok(TurretTaskOutcome::RetargetRequired)
    );
    let task = state(&manager, id);
    assert_eq!(task.elapsed_ms(), 40);
    assert_eq!(task.selection_remaining_us(), 0);
    assert_eq!(task.lost_target_remaining_us(), 40_000);
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    assert_eq!(entity.velocity_raw(), [0; 3]);
    assert!(entity.intro2_gun_turret_aim_runtime.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn turret_missing_target_resets_target_age_and_advances_global_cadence_without_basis() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    isolate(&mut manager, id);
    let task = state_mut(&mut manager, id);
    task.selection_remaining_us = 100_000;
    task.target_handle = u32::MAX;
    task.target_age_us = 8000;
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let mut fx = WorldFx::new();
    assert_eq!(
        tick_intro2_gun_turret_task(
            &mut manager,
            id,
            TurretTaskFrame {
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                elapsed_micros: 40_000,
                global_elapsed_micros: 14_000,
                retail_tick: 75
            },
            &mut fx
        ),
        Ok(TurretTaskOutcome::Continue)
    );
    let task = state(&manager, id);
    assert_eq!(task.target_handle(), 0);
    assert_eq!(task.target_age_us(), 0);
    assert_eq!(task.selection_remaining_us(), 0);
    assert!(task.tracking_disabled());
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -14_000
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn turret_late_basis_block_retains_selection_velocity_and_exact_wrapper_age() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let [enemy, _] = isolate(&mut manager, id);
    manager.entity_mut(enemy).unwrap().capability_flags = 8;
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    manager.entity_mut(id).unwrap().set_velocity_raw([1, 2, 3]);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    expected.next_shared_retail_random_u16();
    expected.next_shared_retail_random_u16();
    assert_eq!(
        tick_intro2_gun_turret_task(
            &mut manager,
            id,
            TurretTaskFrame {
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                elapsed_micros: 40_000,
                global_elapsed_micros: 40_000,
                retail_tick: 75
            },
            &mut fx
        ),
        Err(TurretTaskBlock::Runtime("turret body basis"))
    );
    assert_eq!(state(&manager, id).target_handle(), enemy);
    assert_eq!(state(&manager, id).elapsed_ms(), 40);
    assert_eq!(manager.entity_mut(id).unwrap().velocity_raw(), [0; 3]);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn turret_aligned_target_runs_native_firing_and_publishes_l_after_callback() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 54);
    let [enemy, _] = isolate(&mut manager, id);
    manager.entity_mut(enemy).unwrap().capability_flags = 8;
    let mut fx = WorldFx::new();
    assert_eq!(
        tick_intro2_gun_turret_task(
            &mut manager,
            id,
            TurretTaskFrame {
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                elapsed_micros: 125_000,
                global_elapsed_micros: 300_000,
                retail_tick: 75
            },
            &mut fx
        ),
        Ok(TurretTaskOutcome::Continue)
    );
    let task = state(&manager, id);
    assert_eq!(task.target_handle(), enemy);
    assert_eq!(task.target_age_us(), 125_000);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.intro2_gun_turret_runtime.unwrap().sub_l_output_raw,
        [(task.yaw_raw as i16).wrapping_neg(), task.pitch_raw as i16]
    );
    assert_eq!(
        entity
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        2
    );
}
