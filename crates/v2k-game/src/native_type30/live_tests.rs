//! Ordinary native Type30 callback reachability and presentation-time Method20.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime as Task,
    actor_task_owner::ActorTaskSlot as Slot,
    common_mover::{component_dispatch::CommonMoverDispatchMode, type9_attitude::Type9BodyBasis},
    entity_scheduler::{
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    generic_projectile_emitter::GenericEmitterSpeedField,
    projectile_emitter::projectile_class_row,
    session::GameSession,
    world_fx::ParticleEnvironment,
};

fn live_fixture(class: u8) -> (GameSession, EntityManager, WorldFx, u32, u32) {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(17);
    let id = manager
        .iter_all()
        .find(|entity| allocation_authenticates(entity))
        .unwrap()
        .id;
    let player = manager.player().unwrap().id;
    let others: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id != id && entity.id != player)
        .map(|entity| entity.id)
        .collect();
    for other in others {
        manager.entity_mut(other).unwrap().active = false;
    }
    let target = manager.entity_mut(player).unwrap();
    target.set_motion_raw([0, 1000, 1000], [0; 3]);
    target.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_motion_raw([0, 1000, 0], [50, -80, 100]);
    entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
    entity.collision.state_flags_at_0x08.overwrite(
        u32::MAX,
        4 | 0x68000
            | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT
            | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    // Select an actual point in the process stream. C690 must consume the
    // selected word and run the real initializer; no context/task is stamped.
    let mut reached = false;
    for _ in 0..65536 {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        let word = u32::from(peek.next_shared_retail_random_u16());
        if tasks::select(true, &mut || word).unwrap().program.class_id == class {
            reached = true;
            break;
        }
        fx.next_shared_retail_random_u16();
    }
    assert!(reached, "actual stream must reach class{class}");
    shared::behavior::reselect::<profile::Type30Profile>(
        &mut manager,
        id,
        4794,
        &mut fx,
        None,
        shared::behavior::ReselectionEntry::Impact,
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        panic!("context");
    };
    assert_eq!(
        context.active_style().style_address(),
        match class {
            5 => 0x4c7930,
            7 => 0x4c7a50,
            26 => 0x4c7738,
            _ => unreachable!(),
        }
    );
    // This typed transient write follows birth. The outer callback must use
    // authored mass+B2, then consume B2 exactly once in its motion tail.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(17);
    (session, manager, fx, id, player)
}

#[v2k_test_support::retail_test]
fn native_type30_living_classes5_7_26_reach_own_kl_and_current_task_wrappers() {
    for class in [5, 7, 26] {
        let (mut session, mut manager, mut fx, id, player) = live_fixture(class);
        let entity = manager.entity_mut(id).unwrap();
        let runtime_before = entity.native_type30_runtime.as_ref().unwrap().clone();
        let primary_before = entity.actor_tasks.task_in_slot(Slot::Primary).unwrap();
        let owner = Type30Owner::adopt(&manager, id).unwrap();
        let tick = tick_type30(
            &mut manager,
            owner,
            Type30Frame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                retail_tick: 4794,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                Type30Outcome::Advanced {
                    callback_enabled: true,
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "class{class}: {:?}",
            tick.outcome
        );
        let entity = manager.entity_mut(id).unwrap();
        let runtime = entity.native_type30_runtime.as_ref().unwrap();
        assert_eq!(runtime.allocation, runtime_before.allocation);
        assert_ne!(
            runtime.kl_components.model_variables_raw(),
            runtime_before.kl_components.model_variables_raw(),
            "actual01430 must commit the allocation's bank"
        );
        assert_eq!(
            runtime.kl_components.model_variables_raw()[3],
            160,
            "K consumes VY-80 before C/A/B"
        );
        assert_eq!(entity.mass_raw, 117);
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let retained = tick.retained_owner.unwrap();
        assert_eq!(Type30Owner::adopt(&manager, id), Ok(retained));
        let entity = manager.entity_mut(id).unwrap();
        for slot in Slot::IN_RETAIL_TICK_ORDER {
            if let Some(task) = entity.actor_tasks.task_in_slot(slot) {
                assert!(!entity.actor_tasks.wrapper_flags(task).unwrap().in_callback);
            }
        }
        if class == 5 {
            assert_eq!(
                entity.actor_tasks.task_in_slot(Slot::Primary),
                Some(primary_before)
            );
            assert!(
                matches!(entity.actor_task_state(Slot::Primary),Some(Task::SharedRetarget(task)) if task.elapsed_ms()==20)
            );
        } else if class == 7 {
            assert!(entity.actor_tasks.wrapper_flags(primary_before).is_none());
            assert!(entity.actor_task_state(Slot::Secondary).is_none());
            assert!(
                matches!(entity.actor_task_state(Slot::Primary),Some(Task::ChaseTarget(task)) if task.target_id()==player)
            );
            assert!(
                matches!(entity.actor_task_state(Slot::Tertiary),Some(Task::AimAndFire(task)) if task.private_state().target_entity_id()==player)
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_type30_real_acquisition_method20_fifo_survives_class12_and_drains_once() {
    let (mut session, mut manager, mut fx, id, player) = live_fixture(7);
    let owner = Type30Owner::adopt(&manager, id).unwrap();
    let tick = tick_type30(
        &mut manager,
        owner,
        Type30Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 4794,
        },
    );
    assert!(
        matches!(tick.outcome, Type30Outcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    assert!(
        matches!(manager.entity_mut(id).unwrap().actor_task_state(Slot::Primary),Some(Task::ChaseTarget(task)) if task.target_id()==player)
    );
    let metadata = manager.type_runtime_metadata(30).unwrap().clone();
    // Age the actual Aim and Sub-E cadence until it emits. Every failed gate
    // remains a source-owned visit and consumes its real RNG prefix.
    for _ in 0..64 {
        if manager
            .entity_mut(id)
            .unwrap()
            .native_type30_aim_runtime
            .as_ref()
            .is_some_and(|queue| queue.queued_shot_count() > 0)
        {
            break;
        }
        aim::tick_type30_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            125_000,
            Some(&metadata),
        )
        .unwrap();
    }
    let queue = manager
        .entity_mut(id)
        .unwrap()
        .native_type30_aim_runtime
        .clone()
        .unwrap();
    assert!(
        queue.queued_shot_count() > 0,
        "actualMethod20 cadence must emit"
    );
    let row = projectile_class_row(20).unwrap();
    assert!(queue
        .transient_shots()
        .iter()
        .all(|shot| shot.source_handle == id
            && shot.owner_handle == id
            && shot.projectile_method == 20
            && shot.speed_field == GenericEmitterSpeedField::Explicit(row.speed_raw as i16)));
    crate::intro2_common_dying::publish_intro2_common_standard_death(&mut manager, id, &mut fx)
        .unwrap()
        .unwrap();
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .native_type30_aim_runtime
            .as_ref(),
        Some(&queue)
    );
    let before = fx.particle_count();
    let drained =
        aim::drain_type30_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4794).unwrap();
    assert_eq!(
        drained.materialized_particle_classes,
        vec![row.particle_class; queue.queued_shot_count()]
    );
    let particles = fx.test_particles_in_virgin_birth_order();
    assert!(particles[before..]
        .iter()
        .all(|particle| particle.owner_id == Some(id)
            && particle.source_entity_type_at_birth == Some(30)
            && particle.age_ticks == 0.0));
    assert!(
        aim::drain_type30_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4794)
            .unwrap()
            .materialized_particle_classes
            .is_empty()
    );
}
