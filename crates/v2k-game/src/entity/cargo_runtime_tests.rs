//! Live cargo callback custody, beyond the individual type-vtable adapters.
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    entity_behavior::ActiveBehaviorStyle,
    native_type122::construction_tests::native_fixture_with_player,
};

fn actor(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn completed(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &EntityManager,
    id: u32,
    kind: u32,
) -> bool {
    match kind {
        79 => tasks.begin_intro2_type8_external_mutation(manager, id),
        7 | 86 => tasks.begin_native_type86_external_mutation(manager, id),
        123 => tasks.begin_native_type123_external_mutation(manager, id),
        _ => unreachable!(),
    }
}

#[v2k_test_support::retail_test]
fn missing_native_type123_receipt_never_falls_back_to_unowned_relation_projection() {
    for attached in [false, true] {
        let (session, mut manager, mut fx) = native_fixture_with_player(49);
        manager.cleanup_pending_actor_deferred_destroys();
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 123)
            .unwrap()
            .id;
        let parent = manager
            .iter_all()
            .find(|entity| {
                entity.id != id
                    && entity.active
                    && matches!(&entity.sub_j_attachment_runtime,
                RetailRuntimeValue::Known(Some(rows)) if rows.is_empty() && rows.capacity() > 0)
            })
            .unwrap()
            .id;
        let mut tasks = SpecializedActorTaskScheduler::new();
        tasks.adopt_native_type123(&mut manager);
        let count = tasks.registered_len();
        let mut notifications = GameplayNotifications::new();
        if attached {
            let mut callbacks = LiveCargoCallbacks {
                scheduler: &mut tasks,
                world_fx: &mut fx,
                notifications: &mut notifications,
                retail_tick: 5000,
                terrain: session.cache.terrain(),
                blocked: Vec::new(),
            };
            let plan = callbacks.prepare_attach(&manager, id, parent).unwrap();
            callbacks.commit_attach(&mut manager, plan, |manager| {
                let RetailRuntimeValue::Known(Some(rows)) =
                    &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
                else {
                    panic!();
                };
                rows.append(id).unwrap();
            });
        }
        manager.entity_mut(id).unwrap().native_type123_runtime = None;
        let entity = actor(&manager, id);
        let before = (
            entity.current_behavior_context,
            entity.actor_animation_runtime,
            entity.collision.state_flags_at_0x08,
            entity.attached_to,
        );
        let pending = fx.pending_event_count();
        let mut oracle = fx.fork_for_main_base_abort_transaction();
        {
            let mut callbacks = LiveCargoCallbacks {
                scheduler: &mut tasks,
                world_fx: &mut fx,
                notifications: &mut notifications,
                retail_tick: 5000,
                terrain: session.cache.terrain(),
                blocked: Vec::new(),
            };
            if attached {
                assert!(callbacks
                    .prepare_release(&manager, id, parent, Type9CargoReleasePosition::Retained)
                    .is_none());
            } else {
                assert!(callbacks.prepare_attach(&manager, id, parent).is_none());
            }
            assert_eq!(callbacks.blocked.len(), 1);
        }
        let entity = actor(&manager, id);
        assert_eq!(
            (
                entity.current_behavior_context,
                entity.actor_animation_runtime,
                entity.collision.state_flags_at_0x08,
                entity.attached_to
            ),
            before
        );
        assert_eq!(tasks.registered_len(), count);
        assert_eq!(fx.pending_event_count(), pending);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
        assert!(matches!(&actor(&manager,parent).sub_j_attachment_runtime,
            RetailRuntimeValue::Known(Some(rows)) if rows.contains(id) == attached));
    }
}

#[v2k_test_support::retail_test]
fn live_cargo_attach_transfers_living_graphs_and_retires_terminal_people_and_workers() {
    for (level, kind) in [(19, 79), (24, 86), (34, 7), (49, 123)] {
        for dying in [false, true] {
            let (session, mut manager, mut fx) = native_fixture_with_player(level);
            manager.cleanup_pending_actor_deferred_destroys();
            let id = manager
                .iter_all()
                .find(|entity| entity.entity_type == kind)
                .unwrap()
                .id;
            let parent = manager
                .iter_all()
                .find(|entity| {
                    entity.id != id
                        && entity.active
                        && matches!(&entity.sub_j_attachment_runtime,
                    RetailRuntimeValue::Known(Some(rows)) if rows.is_empty() && rows.capacity() > 0)
                })
                .unwrap()
                .id;
            let mut tasks = SpecializedActorTaskScheduler::new();
            match kind {
                79 => {
                    tasks.adopt_intro2_type8(&mut manager);
                }
                7 | 86 => {
                    tasks.adopt_native_type86(&mut manager);
                }
                123 => {
                    tasks.adopt_native_type123(&mut manager);
                }
                _ => unreachable!(),
            }
            if dying {
                match kind {
                    79 => {
                        let owner = crate::intro2_type8::impact::run_intro2_type8_standard_death(
                            &mut manager,
                            id,
                            &mut fx,
                        )
                        .unwrap()
                        .publication
                        .unwrap();
                        tasks.register_intro2_type8(owner);
                    }
                    7 | 86 => {
                        let owner = crate::native_type86::impact::run_native_type86_standard_death(
                            &mut manager,
                            id,
                            &mut fx,
                        )
                        .unwrap()
                        .publication
                        .unwrap();
                        tasks.register_native_type86(owner);
                    }
                    123 => {
                        let owner =
                            crate::native_type123::impact::run_native_type123_standard_death(
                                &mut manager,
                                id,
                                &mut fx,
                            )
                            .unwrap()
                            .publication
                            .unwrap();
                        tasks.register_native_type123(owner);
                    }
                    _ => unreachable!(),
                }
            }
            assert!(completed(&mut tasks, &manager, id, kind));
            let family = tasks.family_for(id).unwrap();
            let count = tasks.registered_len();
            let old_task = actor(&manager, id)
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            fx.process_pending();
            fx.take_positional_sounds();
            let mut oracle = fx.fork_for_main_base_abort_transaction();
            let mut notifications = GameplayNotifications::new();
            {
                let mut callbacks = LiveCargoCallbacks {
                    scheduler: &mut tasks,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                    retail_tick: 5000,
                    terrain: session.cache.terrain(),
                    blocked: Vec::new(),
                };
                let plan = callbacks
                    .prepare_attach(&manager, id, parent)
                    .unwrap_or_else(|| panic!("type{kind} dying={dying}: {:?}", callbacks.blocked));
                let mut prefix_calls = 0;
                callbacks.commit_attach(&mut manager, plan, |manager| {
                    prefix_calls += 1;
                    // The production caller must publish Sub-J before16700.
                    assert_eq!(actor(manager, id).attached_to, None);
                    let RetailRuntimeValue::Known(Some(rows)) =
                        &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
                    else {
                        panic!();
                    };
                    rows.append(id).unwrap();
                });
                assert_eq!(prefix_calls, 1);
                assert!(callbacks.blocked.is_empty());
            }
            assert_eq!(actor(&manager, id).attached_to, Some(parent));
            assert!(matches!(&actor(&manager,parent).sub_j_attachment_runtime,
                RetailRuntimeValue::Known(Some(rows)) if rows.contains(id)));
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
            if dying {
                assert_eq!(tasks.family_for(id), None);
                assert_eq!(tasks.registered_len(), count - 1);
                assert!(!completed(&mut tasks, &manager, id, kind));
                assert_eq!(
                    actor(&manager, id).actor_tasks.wrapper_flags(old_task),
                    None
                );
                for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                    assert_eq!(actor(&manager, id).actor_tasks.task_in_slot(slot), None);
                }
                let RetailRuntimeValue::Known(Some(context)) =
                    actor(&manager, id).current_behavior_context
                else {
                    panic!();
                };
                let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
                    panic!();
                };
                assert_eq!(style.frame_address, 0x004c7108);
                assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
                assert!(actor(&manager, id).active);
                assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), [id]);
                assert!(!manager.iter_all().any(|entity| entity.id == id));
            } else {
                assert_eq!(tasks.family_for(id), Some(family));
                assert_eq!(tasks.registered_len(), count);
                // The retained owner authenticates the new relation publication,
                // not merely the old family tag or the pre-attach task ID.
                assert!(completed(&mut tasks, &manager, id, kind));
                assert_ne!(
                    actor(&manager, id)
                        .actor_tasks
                        .task_in_slot(ActorTaskSlot::Primary),
                    Some(old_task)
                );
                assert!(matches!(
                    actor(&manager, id)
                        .actor_tasks
                        .state_in_slot(ActorTaskSlot::Primary),
                    Some(ActorTaskRuntime::None)
                ));
                assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
                assert!(manager.cleanup_pending_actor_deferred_destroys().is_empty());
                assert!(actor(&manager, id).active);
            }
        }
    }
}
