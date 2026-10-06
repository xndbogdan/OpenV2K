//! Existing-parent12DA0 membership for a real person killed while attached.
use super::*;
use crate::{
    entity_relation_release::relation_attach_state_word_after,
    native_actor_attachment::NativeActorAttachOutcome,
    native_type122::construction_tests::native_fixture_with_player,
    native_type123::{self, Type123Frame, Type123Outcome, Type123Owner},
    session::GameSession,
};

#[derive(Clone, Copy)]
enum Owner {
    Person(Type86Owner),
    Type123(Type123Owner),
}

impl Owner {
    fn tick(
        self,
        session: &GameSession,
        manager: &mut EntityManager,
        fx: &mut WorldFx,
    ) -> Result<Self, (String, bool)> {
        match self {
            Self::Person(owner) => {
                let result = tick_type86(
                    manager,
                    owner,
                    Type86Frame {
                        resources: &session.cache,
                        world_fx: fx,
                        elapsed_micros: 20_000,
                        global_elapsed_micros: 20_000,
                        retail_tick: 5000,
                    },
                );
                match result.outcome {
                    Type86Outcome::Advanced {
                        terminal: false, ..
                    } => Ok(Self::Person(result.retained_owner.unwrap())),
                    Type86Outcome::Blocked {
                        reason,
                        prefix_committed,
                        ..
                    } => Err((format!("{reason:?}"), prefix_committed)),
                    other => panic!("unexpected person outcome {other:?}"),
                }
            }
            Self::Type123(owner) => {
                let result = native_type123::tick_type123(
                    manager,
                    owner,
                    Type123Frame {
                        resources: &session.cache,
                        world_fx: fx,
                        elapsed_micros: 20_000,
                        global_elapsed_micros: 20_000,
                        retail_tick: 5000,
                    },
                );
                match result.outcome {
                    Type123Outcome::Advanced {
                        terminal: false, ..
                    } => Ok(Self::Type123(result.retained_owner.unwrap())),
                    Type123Outcome::Blocked {
                        reason,
                        prefix_committed,
                        ..
                    } => Err((format!("{reason:?}"), prefix_committed)),
                    other => panic!("unexpected Type123 outcome {other:?}"),
                }
            }
        }
    }
}

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn fixture(level: u32, kind: u32) -> (GameSession, EntityManager, WorldFx, u32, u32, Owner) {
    let (session, mut manager, mut fx) = native_fixture_with_player(level);
    manager.cleanup_pending_actor_deferred_destroys();
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == kind)
        .unwrap()
        .id;
    let parent = manager.player().unwrap().id;
    // These are the production18440/16700 prefix and the actual type-owned
    // callback, using the player's constructor-owned Sub-J storage.
    let attach_person = (kind != 123).then(|| cargo::prepare_attach(&manager, id, parent).unwrap());
    let attach123 =
        (kind == 123).then(|| native_type123::cargo::prepare_attach(&manager, id, parent).unwrap());
    let RetailRuntimeValue::Known(Some(rows)) =
        &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
    else {
        panic!("native player Sub-J")
    };
    rows.append(id).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08 =
        relation_attach_state_word_after(entity.collision.state_flags_at_0x08);
    entity.attached_to = Some(parent);
    let result = if let Some(plan) = attach_person {
        cargo::commit_attach(&mut manager, id, plan, &mut fx)
    } else {
        native_type123::cargo::commit_attach(&mut manager, id, attach123.unwrap(), &mut fx)
    };
    assert_eq!(result, NativeActorAttachOutcome::RetainedGraph);
    let owner = if kind == 123 {
        Owner::Type123(
            native_type123::impact::run_native_type123_standard_death(&mut manager, id, &mut fx)
                .unwrap()
                .publication
                .unwrap(),
        )
    } else {
        Owner::Person(
            impact::run_native_type86_standard_death(&mut manager, id, &mut fx)
                .unwrap()
                .publication
                .unwrap(),
        )
    };
    assert_eq!(actor(&manager, id).attached_to, Some(parent));
    let entity = manager.entity_mut(id).unwrap();
    // First isolate the relation prefix through12DA0's real callback gate;
    // the next continuing visit re-enables the complete Class14 callback.
    entity.collision.state_flags_at_0x08.overwrite(
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
        SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    );
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    (session, manager, fx, id, parent, owner)
}

#[v2k_test_support::retail_test]
fn carried_person_class14_keeps_or_releases_existing_parent_then_advances_own_task() {
    for (level, kind) in [(17, 78), (24, 86), (16, 95), (49, 123)] {
        for compacted in [false, true] {
            let (session, mut manager, mut fx, id, parent, owner) = fixture(level, kind);
            if compacted {
                // Explicit fixture for18640's already-completed dying-row
                // compaction. It leaves the child's1000/backlink untouched.
                let RetailRuntimeValue::Known(Some(rows)) =
                    &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
                else {
                    panic!()
                };
                rows.commit_stable_compaction(Vec::new());
            }
            let before = actor(&manager, id);
            let task = before
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let context = before.current_behavior_context;
            let task_before = *before.actor_tasks.task_state(task).unwrap();
            let seed = before.type8_sub_d_frame_owner;
            let animation = before.actor_animation_runtime;
            let mut oracle = fx.fork_for_main_base_abort_transaction();
            let owner = owner
                .tick(&session, &mut manager, &mut fx)
                .unwrap_or_else(|error| panic!("type{kind} compacted={compacted}: {error:?}"));
            let after = actor(&manager, id);
            assert_eq!(after.attached_to, (!compacted).then_some(parent));
            assert_eq!(
                after.collision.state_flags_at_0x08.masked(0x1000),
                RetailRuntimeValue::Known(if compacted { 0 } else { 0x1000 })
            );
            assert_eq!(
                after.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(0x2f)
            );
            assert_eq!(after.current_behavior_context, context);
            assert_eq!(
                after.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                Some(task)
            );
            assert_eq!(*after.actor_tasks.task_state(task).unwrap(), task_before);
            assert_eq!(after.type8_sub_d_frame_owner, seed);
            assert_eq!(
                after.actor_animation_runtime, animation,
                "null Class14 release must not reset Sub-I"
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16(),
                "membership prefix owns no RNG"
            );
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(
                    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT,
                );
            owner
                .tick(&session, &mut manager, &mut fx)
                .unwrap_or_else(|error| panic!("type{kind} compacted={compacted}: {error:?}"));
            let after = actor(&manager, id);
            assert_eq!(
                after.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                Some(task)
            );
            assert_ne!(
                *after.actor_tasks.task_state(task).unwrap(),
                task_before,
                "Class14 wrapper must age"
            );
            assert_eq!(
                after
                    .type8_sub_d_frame_owner
                    .unwrap()
                    .classifier_cache()
                    .stagger_counter(),
                seed.unwrap()
                    .classifier_cache()
                    .stagger_counter()
                    .wrapping_add(1),
                "the full Class14 callback advances its existing Sub-D exactly once"
            );
            assert!(if kind == 123 {
                crate::native_type123::native_type123_allocation_authenticates(after)
            } else {
                crate::native_type86::native_type86_allocation_authenticates(after)
            });
            assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
            assert!(matches!(&actor(&manager,parent).sub_j_attachment_runtime,
                RetailRuntimeValue::Known(Some(rows)) if rows.contains(id) != compacted));
        }
    }
}

#[v2k_test_support::retail_test]
fn carried_person_class14_missing_parent_blocks_before_scheduler_task_or_rng_writes() {
    for (level, kind) in [(17, 78), (24, 86), (16, 95), (49, 123)] {
        let (session, mut manager, mut fx, id, _, owner) = fixture(level, kind);
        manager.entity_mut(id).unwrap().attached_to = Some(u32::MAX);
        let snapshot = |manager: &EntityManager| {
            let entity = actor(manager, id);
            (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.collision.state_flags_at_0x08,
                entity.collision.callback_scheduler_accumulator_us_at_0x6c,
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.current_behavior_context,
                entity.actor_animation_runtime,
                *entity
                    .actor_tasks
                    .state_in_slot(ActorTaskSlot::Primary)
                    .unwrap(),
            )
        };
        let before = snapshot(&manager);
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        let Err((reason, committed)) = owner.tick(&session, &mut manager, &mut fx) else {
            panic!("missing parent must remain explicit")
        };
        assert_eq!(reason, "Runtime(\"missing-parent180F0\")");
        assert!(!committed);
        assert_eq!(snapshot(&manager), before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
}
