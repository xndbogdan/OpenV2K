//! Native Type122 actual scheduler cursor, component receipts and failure custody.

use super::{construction_tests::*, *};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

#[v2k_test_support::retail_test]
fn all_ten_type122_births_run_the_native_scheduler_with_own_d_e_h_j() {
    let (mut session, metadata) = fixture();
    let mut fx = WorldFx::new();
    let mut total = 0;
    let mut moved = false;
    for (level, notification_phase) in [
        (24, GameplayNotificationPhase::Playing),
        (42, GameplayNotificationPhase::Playing),
        (46, GameplayNotificationPhase::Playing),
        (49, GameplayNotificationPhase::Playing),
        (50, GameplayNotificationPhase::NonGameplay),
    ] {
        session.load_level_by_id(level, 1).unwrap();
        let mut manager = construct(&session, &metadata, level, &mut fx);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|entity| entity.entity_type == 122)
            .map(|entity| entity.id)
            .collect();
        let before: Vec<_> = ids
            .iter()
            .map(|&id| manager.entity_mut(id).unwrap().position_raw())
            .collect();
        let mut scheduler = SpecializedActorTaskScheduler::default();
        assert_eq!(scheduler.adopt_type122(&manager), ids.len());
        assert_eq!(scheduler.adopt_type122(&manager), 0);
        let mut notifications = GameplayNotifications::new();
        let mut static_damage = StaticDamageScheduler::new();
        for tick in 4794..4844 {
            let pass = scheduler.tick(
                &mut manager,
                SpecializedActorTaskProductionFrame {
                    world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                    hive_components: None,
                    notification_phase,
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
            assert!(pass.block.is_none(), "world{level} tick{tick}: {pass:?}");
            assert_eq!(
                pass.outcomes.len(),
                ids.len(),
                "every native allocation stays scheduled"
            );
            for outcome in pass.outcomes {
                assert!(
                    matches!(
                        outcome,
                        SpecializedActorTaskProductionOutcome::NativeType122(
                            Type122Outcome::Advanced { .. } | Type122Outcome::Waiting { .. }
                        )
                    ),
                    "world{level} tick{tick}: {outcome:?}"
                );
            }
        }
        for (&id, birth_position) in ids.iter().zip(before) {
            assert!(type122_manager_allocation_authenticates(&manager, id));
            assert!(Type122Owner::adopt(&manager, id).is_ok());
            let entity = manager.entity_mut(id).unwrap();
            moved |= entity.position_raw() != birth_position;
            assert_eq!(
                entity
                    .native_type122_runtime
                    .unwrap()
                    .sub_e_runtime
                    .projectile_method,
                24
            );
            assert_eq!(
                entity
                    .native_type122_runtime
                    .unwrap()
                    .sub_e_runtime
                    .sound_id,
                70
            );
            assert!(
                matches!(&entity.sub_h_external_frame_runtime,RetailRuntimeValue::Known(Some(h)) if h.records().len()==6)
            );
            assert!(
                matches!(&entity.sub_j_attachment_runtime,RetailRuntimeValue::Known(Some(j)) if j.authored_slot_count()==1)
            );
            assert!(entity.intro2_type53_runtime.is_none());
        }
        total += ids.len();
    }
    assert_eq!(total, 10);
    assert!(
        moved,
        "actual12DA0 component execution moves authored actors"
    );
}

#[v2k_test_support::retail_test]
fn type122_late_first_query_block_parks_exact_prefix_and_foreign_receipt_drops_before_writes() {
    let (session, mut manager, mut fx) = native_fixture(24);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    let runtime = entity.native_type122_runtime.as_mut().unwrap();
    let seed = runtime.sub_d_owner.classifier_cache().stagger_counter();
    let spawn = runtime.spawn_index;
    // Deliberately remove valid process custody to exercise an unknown first
    // query. Ordinary native histories above must never use this parked form.
    runtime.sub_d_owner = Type9SubDFrameOwner::pending_constructor_origin(seed);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    let owner = Type122Owner::adopt(&manager, id).unwrap();
    let mut tasks = SpecializedActorTaskScheduler::default();
    let mut notifications = GameplayNotifications::new();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    expected.next_shared_retail_random_u16();
    expected.next_shared_retail_random_u16();
    let tick = tick_type122(
        &mut manager,
        owner,
        Type122Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 4794,
            capture_tasks: &mut tasks,
            notifications: &mut notifications,
        },
    );
    assert_eq!(
        tick.outcome,
        Type122Outcome::Blocked {
            entity_id: id,
            reason: Type122Block::SubDFirstQuery {
                origin: crate::native_ground_actor::NativeGroundAllocationOrigin::Authored {
                    spawn_index: spawn
                },
                seed
            },
            prefix_committed: true
        }
    );
    let entity = manager.entity_mut(id).unwrap();
    let primary = entity.actor_task_state(ActorTaskSlot::Primary).copied();
    assert!(
        matches!(primary,Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms()==20)
    );
    let components = (
        entity.native_type122_runtime,
        entity.sub_a_propulsion_runtime,
        entity.sub_h_external_frame_runtime.clone(),
    );
    let tick = tick_type122(
        &mut manager,
        tick.retained_owner.unwrap(),
        Type122Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 4795,
            capture_tasks: &mut tasks,
            notifications: &mut notifications,
        },
    );
    assert_eq!(tick.outcome, Type122Outcome::Pending { entity_id: id });
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary).copied(),
        primary
    );
    assert_eq!(
        (
            entity.native_type122_runtime,
            entity.sub_a_propulsion_runtime,
            entity.sub_h_external_frame_runtime.clone()
        ),
        components
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );

    let (_, mut source, _) = native_fixture(24);
    let (_, mut foreign, _) = native_fixture(24);
    let owner = Type122Owner::adopt(&source, id).unwrap();
    std::mem::swap(
        source.entity_mut(id).unwrap(),
        foreign.entity_mut(id).unwrap(),
    );
    assert_eq!(
        Type122Owner::adopt(&foreign, id),
        Err(Type122Block::Allocation)
    );
    let entity = foreign.entity_mut(id).unwrap();
    let before = (
        entity.position_raw(),
        entity.native_type122_runtime,
        entity.actor_task_state(ActorTaskSlot::Primary).copied(),
    );
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let tick = tick_type122(
        &mut foreign,
        owner,
        Type122Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 4796,
            capture_tasks: &mut tasks,
            notifications: &mut notifications,
        },
    );
    assert_eq!(tick.outcome, Type122Outcome::Dropped { entity_id: id });
    assert!(tick.retained_owner.is_none());
    let entity = foreign.entity_mut(id).unwrap();
    assert_eq!(
        (
            entity.position_raw(),
            entity.native_type122_runtime,
            entity.actor_task_state(ActorTaskSlot::Primary).copied()
        ),
        before
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}
