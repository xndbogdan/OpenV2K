use super::*;
use crate::intro2_gun_turret::native::tests::{fixture, generic, publish};

#[v2k_test_support::retail_test]
fn turret_manual_null_and_rejected_gate_only_advance_cadence() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        None,
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 125_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -125_000
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16()
    );
    //Find a deterministic odd next word; divisor2 must reject that draw.
    let mut reference = WorldFx::new();
    let mut fx = WorldFx::new();
    loop {
        let word = reference.next_shared_retail_random_u16();
        if word % 2 == 1 {
            break;
        }
        fx.next_shared_retail_random_u16();
    }
    let result = tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 125_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    assert_eq!(result.queued_shots, 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        reference.next_shared_retail_random_u16()
    );
    let runtime = manager
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_runtime
        .unwrap();
    assert_eq!(runtime.sub_e_runtime.cadence_raw, -125_000);
    assert_eq!(runtime.sub_e_joint_word_raw, 0);
    assert_eq!(runtime.sub_e_runtime.emitter_selector, 0);
}

#[v2k_test_support::retail_test]
fn turret_catchup_alternates_joints_queues_exact_records_and_one_sound() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let mut fx = WorldFx::new();
    let mut expected = WorldFx::new();
    expected.next_shared_retail_random_u16();
    let result = tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([100, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 300_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    assert_eq!(
        result,
        Intro2GunTurretAimTickOutcome {
            queued_shots: 2,
            rejected_appends: 0
        }
    );
    let entity = manager.entity_mut(id).unwrap();
    let runtime = entity.intro2_gun_turret_runtime.unwrap();
    assert_eq!(runtime.sub_e_runtime.cadence_raw, 300_000);
    assert_eq!(runtime.sub_e_runtime.emitter_selector, 0);
    assert_eq!(runtime.sub_e_joint_word_raw, u16::MAX);
    let queue = entity.intro2_gun_turret_aim_runtime.as_ref().unwrap();
    for (index, shot) in queue.transient_shots().iter().enumerate() {
        assert_eq!(shot.projectile_method, 14);
        assert_eq!(shot.source_handle, id);
        assert_eq!(shot.owner_handle, id);
        assert_eq!(shot.emitter_selector, index as u16);
        assert_eq!(shot.time_offset_raw, index as i32 * 300_000);
        assert_eq!(shot.speed_field, GenericEmitterSpeedField::Explicit(5000));
        assert_eq!(
            shot.direction_raw,
            normalize_retail_vector_q31([100, 0, 1000])
        );
        assert!(!shot.auxiliary);
    }
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    fx.process_pending();
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 78);
}

#[v2k_test_support::retail_test]
fn turret_queue_survives_native_reselection_without_reconstructing_e() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 54);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 300_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let queue = entity.intro2_gun_turret_aim_runtime.clone();
    let emitter = entity.intro2_gun_turret_runtime.unwrap();
    super::super::reselect_intro2_gun_turret(entity, &metadata[102], &mut || 0).unwrap();
    assert_eq!(entity.intro2_gun_turret_aim_runtime, queue);
    assert_eq!(entity.intro2_gun_turret_runtime, Some(emitter));
    let mut queue = entity.intro2_gun_turret_aim_runtime.take().unwrap();
    assert!(queue.authenticates(entity));
    assert_eq!(queue.pop_front().unwrap().emitter_selector, 0);
    assert_eq!(queue.pop_front().unwrap().emitter_selector, 1);
    assert!(queue.pop_front().is_none());
}

#[v2k_test_support::retail_test]
fn turret_foreign_queue_preflight_and_late_joint_error_preserve_distinct_prefixes() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let other = publish(&mut manager, &session, &metadata, 54);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        None,
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 40_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    manager
        .entity_mut(other)
        .unwrap()
        .intro2_gun_turret_aim_runtime = manager
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_aim_runtime
        .clone();
    assert_eq!(
        tick_intro2_gun_turret_aim(
            &mut manager,
            other,
            Some([0, 0, 1000]),
            crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
                global_elapsed_micros: 300_000,
                world_fx: &mut fx,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                retail_tick: 75
            }
        ),
        Err(Intro2GunTurretAimError::QueueCustody)
    );
    assert_eq!(
        manager
            .entity_mut(other)
            .unwrap()
            .intro2_gun_turret_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        0
    );
    let emitter = &mut manager
        .entity_mut(id)
        .unwrap()
        .intro2_gun_turret_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime;
    emitter.emitter_selector = 1;
    emitter.joint_bindings[0] = Some(99);
    assert_eq!(
        tick_intro2_gun_turret_aim(
            &mut manager,
            id,
            Some([0, 0, 1000]),
            crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
                global_elapsed_micros: 300_000,
                world_fx: &mut fx,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
                retail_tick: 75
            }
        ),
        Err(Intro2GunTurretAimError::Runtime("425160 joint binding"))
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity
            .intro2_gun_turret_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -300_000
    );
    assert_eq!(
        entity
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        0
    );
    let mut expected = WorldFx::new();
    expected.next_shared_retail_random_u16();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
