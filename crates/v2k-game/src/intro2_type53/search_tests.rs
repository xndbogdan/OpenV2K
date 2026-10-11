use super::tests::{fixture, generic};
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    aim_and_fire::{AimAndFireInvalidTargetReason, AimAndFireTransitionReason},
    common_mover::component_dispatch::CommonMoverDispatchMode,
    entity::EntityManager,
    entity_collision_state::{RetailStateWord, DYING_STATE_BIT},
    search_attack::SearchAttackTarget,
    search_attack_live::{
        apply_search_attack_ade0_without_mover, apply_search_attack_c7d0_c6b0_without_mover,
    },
    session::GameSession,
    world_fx::WorldFx,
};

fn random_probe(fx: &WorldFx) -> [u16; 4] {
    // Test-only observation leaves the live effect owner and stream untouched.
    let mut snapshot = fx.fork_for_main_base_abort_transaction();
    std::array::from_fn(|_| snapshot.next_shared_retail_random_u16())
}

fn acquiring(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
) -> (EntityManager, u32, u32) {
    let mut manager = generic(session, metadata);
    let id = spawn as u32 + 1;
    let mut preceding = generic(session, metadata);
    let witness = preceding.entity_mut(1).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    witness.set_position_raw(entity.position_raw());
    witness.capability_flags = 1;
    witness.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    let mut words = [0, 0xffff, 0, 0].into_iter();
    assert_eq!(
        publish_intro2_type53(
            entity,
            &metadata[53],
            std::slice::from_ref(witness),
            session.cache.terrain().unwrap(),
            &mut || words.next().unwrap()
        )
        .unwrap()
        .selection
        .program
        .class_id,
        7
    );
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    let position = entity.position_raw();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 9)
        .unwrap()
        .id;
    let ids: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    for candidate in ids {
        manager.entity_mut(candidate).unwrap().capability_flags &= !0xc05;
    }
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags = 1;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(4);
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    target.set_position_raw([
        position[0].wrapping_add(700),
        position[1],
        position[2].wrapping_add(1100),
    ]);
    (manager, id, target_id)
}

fn pursuing(
    session: &GameSession,
    metadata: &[EntityTypeRuntimeMetadata],
    spawn: usize,
) -> (EntityManager, u32, u32) {
    let (mut manager, id, target_id) = acquiring(session, metadata, spawn);
    let mut fx = WorldFx::new();
    let entity = manager.entity_mut(id).unwrap();
    let before = random_probe(&fx);
    apply_search_attack_c7d0_c6b0_without_mover(
        entity,
        SearchAttackTarget {
            id: target_id,
            scaled_distance_squared_raw: 0,
        },
    )
    .unwrap();
    apply_search_attack_ade0_without_mover(entity, &metadata[53], target_id, &mut fx).unwrap();
    assert!(search::pursuing_graph_authenticates(entity));
    assert_eq!(random_probe(&fx), before);
    assert_eq!(fx.pending_event_count(), 0);
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x8000),
        RetailRuntimeValue::Known(0x8000)
    );
    let RetailRuntimeValue::Known(Some(a)) = entity.sub_a_propulsion_runtime else {
        panic!()
    };
    assert_eq!(a.target_speed_raw(), RetailRuntimeValue::Known(533));
    (manager, id, target_id)
}

#[v2k_test_support::retail_test]
fn native_search_handoff_runs_newborn_aim_once_and_chase_on_the_next_pass() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for spawn in INTRO2_TYPE53_SPAWN_INDICES {
        for restricted in [false, true] {
            let (mut manager, id, _) = acquiring(&session, &metadata, spawn);
            if restricted {
                manager
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x0200_0000, 0);
            }
            let mut owner = Intro2Type53Owner::adopt(&manager, id).unwrap();
            let mut fx = WorldFx::new();
            let mut first_task = None;
            let mut callback_visits = 0;
            let mut chase_age = 0;
            let mut aim_age = 0;
            for pass in 0..512 {
                let tick = tick_intro2_type53(
                    &mut manager,
                    owner,
                    Intro2Type53Frame {
                        resources: &session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 20_000,
                        retail_tick: 300 + pass,
                    },
                );
                let callback_dt = match tick.outcome {
                    Intro2Type53Outcome::Waiting { .. } => {
                        owner = tick.retained_owner.unwrap();
                        continue;
                    }
                    Intro2Type53Outcome::Advanced {
                        callback_enabled: true,
                        callback_elapsed_micros,
                        ..
                    } => callback_elapsed_micros,
                    other => panic!("spawn{spawn} restricted={restricted} pass{pass}: {other:?}"),
                };
                owner = tick.retained_owner.unwrap();
                if callback_visits != 0 {
                    chase_age += callback_dt / 1000;
                }
                aim_age += callback_dt / 1000;
                callback_visits += 1;
                let entity = manager.entity_mut(id).unwrap();
                assert!(search::pursuing_graph_authenticates(entity));
                let task_id = entity
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .unwrap();
                if let Some(first) = first_task {
                    assert_eq!(task_id, first);
                } else {
                    first_task = Some(task_id);
                }
                let Some(ActorTaskRuntime::ChaseTarget(chase)) =
                    entity.actor_task_state(ActorTaskSlot::Primary)
                else {
                    panic!()
                };
                assert_eq!(chase.elapsed_ms(), chase_age);
                let Some(ActorTaskRuntime::AimAndFire(aim)) =
                    entity.actor_task_state(ActorTaskSlot::Tertiary)
                else {
                    panic!()
                };
                assert_eq!(
                    aim.elapsed_ms(),
                    aim_age,
                    "newborn Aim must not tick inside acquisition too"
                );
                assert_eq!(aim.private_state().optional_sound_id(), None);
                if callback_visits == 3 {
                    break;
                }
            }
            assert_eq!(
                callback_visits, 3,
                "all native cohorts must resume through the actual scheduler"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn native_absent_emitter_aim_keeps_strict_lifetime_and_no_callback_effects() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for mode in [
        CommonMoverDispatchMode::Normal,
        CommonMoverDispatchMode::Restricted,
    ] {
        let (mut manager, id, _) = pursuing(&session, &metadata, 26);
        let mut fx = WorldFx::new();
        let before = random_probe(&fx);
        assert_eq!(
            aim::tick_tertiary(&mut manager, id, 5_000_999, mode, &mut fx),
            Ok(None)
        );
        assert_eq!(
            aim::tick_tertiary(&mut manager, id, 999, mode, &mut fx),
            Ok(None)
        );
        assert_eq!(
            aim::tick_tertiary(&mut manager, id, 1000, mode, &mut fx),
            Ok(Some(AimAndFireTransitionReason::LifetimeExpired))
        );
        assert_eq!(random_probe(&fx), before);
        assert_eq!(fx.pending_event_count(), 0);
    }
}

#[v2k_test_support::retail_test]
fn native_aim_checks_target_only_in_detailed_mode_and_unwinds_a_late_block() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let (mut manager, id, target_id) = pursuing(&session, &metadata, 20);
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08 = RetailStateWord::unknown();
    let mut fx = WorldFx::new();
    let before = random_probe(&fx);
    assert_eq!(
        aim::tick_tertiary(
            &mut manager,
            id,
            20_000,
            CommonMoverDispatchMode::Restricted,
            &mut fx
        ),
        Ok(None)
    );
    assert_eq!(
        aim::tick_tertiary(
            &mut manager,
            id,
            20_000,
            CommonMoverDispatchMode::Normal,
            &mut fx
        ),
        Err(Intro2Type53Block::Runtime("Aim target dying bit"))
    );
    let entity = manager.entity_mut(id).unwrap();
    let Some(ActorTaskRuntime::AimAndFire(task)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!()
    };
    assert_eq!(task.elapsed_ms(), 40);
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(task_id)
            .unwrap()
            .in_callback
    );
    assert_eq!(random_probe(&fx), before);
}

#[v2k_test_support::retail_test]
fn native_aim_invalid_target_precedes_expiry_and_uses_the_1000_owner_gate() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for suppressed in [false, true] {
        let (mut manager, id, target_id) = pursuing(&session, &metadata, 38);
        manager
            .entity_mut(target_id)
            .unwrap()
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(DYING_STATE_BIT);
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x1080, if suppressed { 0x1080 } else { 0x80 });
        let previous = entity.current_behavior_context;
        let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
        let mut fx = WorldFx::new();
        let before = random_probe(&fx);
        let result = aim::tick_tertiary(
            &mut manager,
            id,
            5_001_000,
            CommonMoverDispatchMode::Normal,
            &mut fx,
        )
        .unwrap();
        assert!(matches!(
            result,
            Some(AimAndFireTransitionReason::TaggedInvalidTarget {
                reason: AimAndFireInvalidTargetReason::Dying,
                ..
            })
        ));
        assert_eq!(random_probe(&fx), before);
        behavior::reselect(
            &mut manager,
            id,
            300,
            &mut fx,
            None,
            behavior::ReselectionEntry::TaskResult,
        )
        .unwrap();
        let entity = manager.entity_mut(id).unwrap();
        if suppressed {
            assert_eq!(entity.current_behavior_context, previous);
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
                task
            );
            assert_eq!(random_probe(&fx), before);
        } else {
            assert_ne!(entity.current_behavior_context, previous);
            assert_ne!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
                task
            );
            assert_ne!(random_probe(&fx), before);
        }
    }
}
