use super::*;
use crate::cleansing_vehicle::tasks;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity_behavior::{select_initial_behavior, BehaviorContextRuntime},
    entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    infection_evolution::{InfectionCellWrite, INFECTION_TERRAIN_TYPE_BIT},
    world_fx::WorldFx,
};

fn publish(manager: &mut EntityManager, id: u32, class: u32, fx: &mut WorldFx) {
    let metadata = manager.type_runtime_metadata(49).unwrap().clone();
    let choices = [v2k_formats::collision::BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: class,
    }];
    let selection = select_initial_behavior(&choices, |_| 1, &mut || {
        u32::from(fx.next_shared_retail_random_u16())
    })
    .unwrap()
    .unwrap();
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
    tasks::publish_selection(
        manager.entity_mut(id).unwrap(),
        &metadata,
        selection,
        context,
        &mut || u32::from(fx.next_shared_retail_random_u16()),
    )
    .unwrap();
}

#[v2k_test_support::retail_test]
fn cleansing_live_detailed_moves_and_emits_cleanse_carriers() {
    let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    publish(&mut manager, id, 42, &mut fx);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        );
    let before = manager.entity_mut(id).unwrap().position_raw();
    let mut owner = CleansingVehicleOwner::adopt(&manager, id).unwrap();
    for retail_tick in 1001..1201 {
        let tick = tick_cleansing_vehicle(
            &mut manager,
            owner,
            CleansingVehicleFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick,
            },
        );
        assert!(
            matches!(tick.outcome, CleansingVehicleOutcome::Advanced { .. }),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
    }
    assert_ne!(manager.entity_mut(id).unwrap().position_raw(), before);
    assert!(fx.particle_count() > 0);
    assert!(owner.completed_mutation_boundary(&manager));
}

#[v2k_test_support::retail_test]
fn stationary_coarse_skips_timer_while_detailed_advances_it() {
    let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    publish(&mut manager, id, 68, &mut fx);
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
    let before = manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let mut owner = CleansingVehicleOwner::adopt(&manager, id).unwrap();
    for retail_tick in 1001..1031 {
        let tick = tick_cleansing_vehicle(
            &mut manager,
            owner,
            CleansingVehicleFrame {
                resources: &mut session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick,
            },
        );
        assert!(
            matches!(
                tick.outcome,
                CleansingVehicleOutcome::Advanced { .. } | CleansingVehicleOutcome::Waiting { .. }
            ),
            "{:?}",
            tick.outcome
        );
        owner = tick.retained_owner.unwrap();
    }
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied(),
        before
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
    let tick = tick_cleansing_vehicle(
        &mut manager,
        owner,
        CleansingVehicleFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 1031,
        },
    );
    assert!(
        matches!(tick.outcome, CleansingVehicleOutcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    assert!(
        matches!(manager.entity_mut(id).unwrap().actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::Class0Timer(timer)) if timer.elapsed_ms() > 0)
    );
}

#[v2k_test_support::retail_test]
fn coarse_cleanse_clears_exact_wrapped_cell_without_model_or_basis() {
    let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    publish(&mut manager, id, 42, &mut fx);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw([i16::MAX, 0, i16::MIN]);
    entity.collision.state_flags_at_0x08.overwrite(0x1000, 0);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let z = i16::MIN
        .wrapping_add((expected.next_shared_retail_random_u16() >> 7).wrapping_sub(0x100) as i16);
    let x = i16::MAX
        .wrapping_add((expected.next_shared_retail_random_u16() >> 7).wrapping_sub(0x100) as i16);
    let cell = [(x as u16 >> 8) as u8, (z as u16 >> 8) as u8];
    session
        .cache
        .apply_level_infection_writes(&[InfectionCellWrite {
            cell,
            infected: true,
        }])
        .unwrap();
    super::terrain_task::tick(
        &mut manager,
        id,
        &mut session.cache,
        &mut fx,
        CommonMoverDispatchMode::Restricted,
        20_000,
    )
    .unwrap();
    assert_eq!(
        session
            .cache
            .level_terrain()
            .unwrap()
            .cell(cell[0] as usize, cell[1] as usize)
            .unwrap()
            .terrain_type
            & INFECTION_TERRAIN_TYPE_BIT,
        0
    );
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn cleansing_pair_callback_uses_native_custody_and_preserves_task_clock() {
    use crate::{
        gameplay_notifications::GameplayNotifications,
        native_actor_descriptor_contact::{
            resolve_native_actor_descriptor_contact, NativeDescriptorContactOutcome,
        },
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        static_damage::StaticDamageScheduler,
    };
    let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    let opposite = manager.iter_all().find(|e| e.id != id).unwrap().id;
    publish(&mut manager, id, 42, &mut fx);
    let entity = manager.entity_mut(id).unwrap();
    let position = entity.position_raw();
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let Some(ActorTaskRuntime::CleansingLandscape(task)) =
        entity.actor_tasks.task_state_mut(task_id)
    else {
        panic!()
    };
    task.elapsed_ms = 123;
    let before = *task;
    entity
        .cleansing_vehicle_runtime
        .as_mut()
        .unwrap()
        .sub_d_runtime
        .last_yaw_step_raw = 19;
    manager
        .entity_mut(opposite)
        .unwrap()
        .set_position_raw(position);
    let mut scheduler = SpecializedActorTaskScheduler::new();
    assert!(scheduler.adopt_cleansing_vehicle(&manager) > 0);
    let result = resolve_native_actor_descriptor_contact(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut manager,
            resources: &mut session.cache,
            world_fx: &mut fx,
            actor_tasks: &mut scheduler,
            static_damage: &mut StaticDamageScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            retail_tick: 1001,
        },
        id,
        opposite,
        ActorTaskSlot::Primary,
    )
    .unwrap();
    assert!(matches!(
        result,
        NativeDescriptorContactOutcome::Applied { rng_draws: 3 | 4 }
    ));
    let entity = manager.entity_mut(id).unwrap();
    let Some(ActorTaskRuntime::CleansingLandscape(task)) = entity.actor_tasks.task_state(task_id)
    else {
        panic!()
    };
    assert_eq!(task.elapsed_ms, before.elapsed_ms);
    assert_eq!(task.private.direction, -before.private.direction);
    assert_ne!(
        task.private.target_position_raw,
        before.private.target_position_raw
    );
    assert_eq!(
        entity
            .cleansing_vehicle_runtime
            .unwrap()
            .sub_d_runtime
            .last_yaw_step_raw,
        1953
    );
    assert!(scheduler.begin_cleansing_vehicle_external_mutation(&manager, id));
}

#[v2k_test_support::retail_test]
fn late_mover_block_retains_prefix_without_replaying_rng_or_task_age() {
    let (mut session, mut manager, mut fx) = super::super::tests::fixture(15);
    let id = manager.iter_all().find(|e| e.entity_type == 49).unwrap().id;
    publish(&mut manager, id, 42, &mut fx);
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let owner = CleansingVehicleOwner::adopt(&manager, id).unwrap();
    let tick = tick_cleansing_vehicle(
        &mut manager,
        owner,
        CleansingVehicleFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 1001,
        },
    );
    assert!(matches!(
        tick.outcome,
        CleansingVehicleOutcome::Blocked {
            prefix_committed: true,
            ..
        }
    ));
    let owner = tick.retained_owner.unwrap();
    assert!(owner.has_pending_prefix());
    assert!(!owner.completed_mutation_boundary(&manager));
    let after = manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Primary)
        .copied();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let tick = tick_cleansing_vehicle(
        &mut manager,
        owner,
        CleansingVehicleFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 1002,
        },
    );
    assert_eq!(
        tick.outcome,
        CleansingVehicleOutcome::Pending { entity_id: id }
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Primary)
            .copied(),
        after
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
