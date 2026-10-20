//! Type31 virus slugs, their class18 split and the Type105 children.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
};

/// Type31's authored rows: two in world 16, one in world 31.
const WORLDS: [u32; 2] = [16, 31];

fn ids(manager: &EntityManager, row: Type31Row) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == row.entity_type())
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn style(manager: &EntityManager, id: u32) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context.active_style().style_address()
}

fn activate(manager: &mut EntityManager, ids: &[u32]) {
    for &id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
}

#[allow(clippy::too_many_arguments)]
fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
    fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    t: u32,
) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
    tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: t * 20_000,
            retail_tick: t,
            main_base_abort_active: false,
        },
        notifications,
    )
}

#[v2k_test_support::retail_test]
fn both_rows_authenticate_their_canonical_metadata() {
    let (session, _) = crate::native_type122::construction_tests::fixture();
    for row in [Type31Row::Type31, Type31Row::Type105] {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(row.entity_type() as usize)
                .unwrap(),
        );
        authenticate_metadata(row, &metadata).unwrap();
    }
}

#[v2k_test_support::retail_test]
fn authored_type31_births_publish_defecate_or_aimless() {
    let mut births = 0;
    for level in WORLDS {
        let (_, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        for id in ids(&manager, Type31Row::Type31) {
            births += 1;
            let entity = entity(&manager, id);
            assert!(manager_allocation_authenticates(&manager, id));
            assert_eq!(type31_row(entity), Some(Type31Row::Type31));
            let runtime = entity.native_type31_runtime.unwrap();
            assert_eq!(runtime.sub_e_runtime, emitter_constructor());
            assert!(runtime.split_terminal.is_none() && runtime.birth.is_none());
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0x439)
            );
            let RetailRuntimeValue::Known(Some(h)) = &entity.sub_h_external_frame_runtime else {
                panic!("Type31 H");
            };
            assert_eq!(h.records().len(), 14);
            // UnderAttack is false at birth: Defecate (1) or Aimless (2).
            assert!(
                matches!(style(&manager, id), 0x004c_7e88 | 0x004c_7930),
                "world{level}: {:#x}",
                style(&manager, id)
            );
            assert!(Type31FamilyOwner::adopt(&manager, id).is_ok());
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(
            tasks.adopt_type31_family(&manager),
            ids(&manager, Type31Row::Type31).len(),
            "world{level}"
        );
    }
    assert_eq!(births, 3);
}

/// Ten seconds under the real scheduler and the late static walk.
#[v2k_test_support::retail_test]
fn type31_cohorts_run_ten_seconds_with_late_static_contact() {
    for level in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = ids(&manager, Type31Row::Type31);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type31_family(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let start = ids
            .iter()
            .map(|&id| entity(&manager, id).position_raw())
            .collect::<Vec<_>>();
        for t in 1..=500u32 {
            let pass = tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            assert!(
                pass.block.is_none(),
                "world{level} tick{t}: {:?}",
                pass.block
            );
            for outcome in &pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::NativeType31Family(outcome) = outcome
                {
                    assert!(
                        !matches!(
                            outcome,
                            Type31Outcome::Blocked { .. } | Type31Outcome::Pending { .. }
                        ),
                        "world{level} tick{t}: {outcome:?}"
                    );
                }
            }
            for &id in &ids {
                let outcome = contact::resolve_type31_family_static_contact(
                    &mut Intro2ContactFrame {
                        entities: &mut manager,
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        static_damage: &mut static_damage,
                        notifications: &mut notifications,
                        retail_tick: t,
                        actor_tasks: &mut tasks,
                    },
                    id,
                );
                assert!(
                    !matches!(outcome, contact::Type31ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        for (&id, start) in ids.iter().zip(start) {
            assert!(
                Type31FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
            assert_ne!(
                entity(&manager, id).position_raw(),
                start,
                "world{level} moves"
            );
        }
    }
}

/// Class18 commits eight real Type105 allocations inline; each child is
/// owned at once and publishes Defecate, its only weighted choice at birth.
#[v2k_test_support::retail_test]
fn class18_splits_type31_into_eight_owned_type105s() {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(31);
    let id = ids(&manager, Type31Row::Type31)[0];
    let before: Vec<_> = manager.iter_all().map(|e| e.id).collect();
    let mut tasks = SpecializedActorTaskScheduler::default();
    tasks.adopt_type31_family(&manager);
    let result = death::begin_type31_standard_death(
        &mut manager,
        id,
        death::Type31Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4794,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    assert!(result.returned_nonzero);
    tasks.register_native_ground_terminal(result.publication.unwrap());
    assert!(death::finished_terminal_authenticates(&manager, id));
    assert!(Type31FamilyOwner::adopt(&manager, id).is_err());
    let split = entity(&manager, id)
        .native_type31_runtime
        .unwrap()
        .split_terminal
        .unwrap();
    assert_eq!(
        split.status,
        death::Type31SplitStatus::Completed(
            crate::split_and_explode::SplitAndExplodeCompletion::ChildrenComplete
        )
    );
    assert_eq!(
        (
            split.progress.attempted_children,
            split.progress.created_children
        ),
        (8, 8)
    );
    let children: Vec<_> = manager
        .iter_all()
        .filter(|e| !before.contains(&e.id))
        .map(|e| e.id)
        .collect();
    assert_eq!(children.len(), 8);
    assert_eq!(
        tasks.adopt_type31_family(&manager),
        0,
        "every child already has synchronous task custody"
    );
    for &child in &children {
        let e = entity(&manager, child);
        assert_eq!(type31_row(e), Some(Type31Row::Type105));
        assert_eq!(
            e.sub_h_external_frame_runtime,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(style(&manager, child), 0x004c_7e88);
        assert!(matches!(
            e.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::DefecateVirusWander(_))
        ));
        assert_eq!(
            e.velocity_raw(),
            e.native_type31_runtime.unwrap().birth.unwrap().velocity_raw
        );
    }
    // A second visit returns the completed terminal without replaying it.
    let linked = manager.iter_all().count();
    let repeat = death::begin_type31_standard_death(
        &mut manager,
        id,
        death::Type31Class18Frame {
            resources: &session.cache,
            world_fx: &mut fx,
            retail_tick: 4795,
            tasks: &mut tasks,
        },
    )
    .unwrap();
    assert!(repeat.returned_nonzero && repeat.publication.is_none());
    assert_eq!(manager.iter_all().count(), linked);

    // A child's lethal entry is class2's quiet terminal.
    let child = children[0];
    let quiet = death::begin_type105_standard_death(&mut manager, child).unwrap();
    assert!(quiet.returned_nonzero && quiet.publication.is_some());
    assert!(death::finished_terminal_authenticates(&manager, child));
    assert!(manager
        .pending_actor_deferred_destroy_ids()
        .contains(&child));
    assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .all(|slot| entity(&manager, child)
            .actor_tasks
            .task_in_slot(slot)
            .is_none()));
}
