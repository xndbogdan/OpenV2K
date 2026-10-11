use super::*;
use crate::actor_task_owner::ActorTaskSlot;

#[derive(Clone, Copy)]
enum Storage {
    Scheduler,
    Pending,
    Retained,
}

#[v2k_test_support::retail_test]
fn corpse_attachment_retires_native_owner_in_each_actual_storage() {
    use crate::intro2_type17::capture::{kill_capture_contact, CaptureContext};
    use crate::main_base_type9_abort::MainBaseType9ResultScreenState;
    use crate::native_type122::construction_tests::native_fixture_with_player;

    for storage in [Storage::Scheduler, Storage::Pending, Storage::Retained] {
        for (world, kind) in [
            (25, 8),
            (19, 79),
            (24, 90),
            (40, 91),
            (42, 116),
            (17, 78),
            (24, 86),
            (16, 95),
            (49, 123),
        ] {
            let (_, mut manager, mut fx) = native_fixture_with_player(world);
            manager.cleanup_pending_actor_deferred_destroys();
            let id = manager
                .iter_all()
                .find(|e| e.entity_type == kind)
                .unwrap()
                .id;
            let parent = manager.iter_all().find(|e| e.id != id && e.active
                && matches!(&e.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(rows))
                    if rows.is_empty() && rows.capacity() > 0)).unwrap().id;
            let mut scheduler = SpecializedActorTaskScheduler::new();
            scheduler.adopt_intro2_type8(&mut manager);
            scheduler.adopt_native_type86(&mut manager);
            scheduler.adopt_native_type123(&mut manager);
            let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
            kill_capture_contact(
                &mut manager,
                id,
                &mut CaptureContext {
                    resources: None,
                    tasks: &mut scheduler,
                    world_fx: &mut fx,
                    notifications: &mut notifications,
                    retail_tick: 5000,
                    result_screen: MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .unwrap();
            let before_ids = scheduler
                .owners
                .iter()
                .map(SpecializedActorTaskOwner::entity_id)
                .collect::<Vec<_>>();
            assert!(before_ids.contains(&id));
            let mut oracle = fx.fork_for_main_base_abort_transaction();
            let mut pending = Vec::new();
            let mut retained = Vec::new();
            match storage {
                Storage::Scheduler => {}
                Storage::Pending => pending = std::mem::take(&mut scheduler.owners),
                Storage::Retained => retained = std::mem::take(&mut scheduler.owners),
            }
            let mut cursor = Intro2RadialCursorCustody {
                pending: &mut pending,
                retained: &mut retained,
                remaining_live_ids: &[],
            };
            let tasks: &mut dyn CaptureTaskCustody = match storage {
                Storage::Scheduler => &mut scheduler,
                Storage::Pending | Storage::Retained => &mut cursor,
            };
            assert!(tasks.capture_child_mutation_ready(&manager, id));
            tasks
                .mutate_capture_child(
                    &mut manager,
                    CaptureChildFrame {
                        child: id,
                        operation: CaptureChildOperation::Attach { captor: parent },
                        world_fx: &mut fx,
                        notifications: &mut notifications,
                        retail_tick: 5001,
                        result_screen: MainBaseType9ResultScreenState::NotShown,
                    },
                    &mut |manager| {
                        let RetailRuntimeValue::Known(Some(rows)) =
                            &mut manager.entity_mut(parent).unwrap().sub_j_attachment_runtime
                        else {
                            panic!()
                        };
                        rows.append(id).unwrap();
                        Ok(())
                    },
                )
                .unwrap();
            assert!(!tasks.capture_child_mutation_ready(&manager, id));
            let owners = match storage {
                Storage::Scheduler => &scheduler.owners,
                Storage::Pending => &pending,
                Storage::Retained => &retained,
            };
            assert_eq!(
                owners
                    .iter()
                    .map(SpecializedActorTaskOwner::entity_id)
                    .collect::<Vec<_>>(),
                before_ids
                    .into_iter()
                    .filter(|owner| *owner != id)
                    .collect::<Vec<_>>()
            );
            let entity = manager.entity_mut(id).unwrap();
            assert!(entity.active);
            assert_eq!(entity.attached_to, Some(parent));
            for slot in ActorTaskSlot::IN_RETAIL_TICK_ORDER {
                assert!(entity.actor_tasks.task_in_slot(slot).is_none());
            }
            assert_eq!(manager.pending_actor_deferred_destroy_ids(), [id]);
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );
            assert!(manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&id));
            assert!(!manager.iter_all().any(|e| e.id == id));
        }
    }
}

#[v2k_test_support::retail_test]
fn delivered_child_task_is_consumed_before_its_deferred_body_sweep() {
    use crate::intro2_type17::capture::{
        execute_capture_delivery, kill_capture_contact, CaptureContext,
    };
    for storage in [Storage::Scheduler, Storage::Pending, Storage::Retained] {
        for corpse in [false, true] {
            let Some(mut f) = crate::intro2_type17::capture::tests::fixture() else {
                return;
            };
            if corpse {
                kill_capture_contact(
                    &mut f.manager,
                    f.child,
                    &mut CaptureContext {
                        resources: None,
                        tasks: &mut f.scheduler,
                        world_fx: &mut f.fx,
                        notifications: &mut f.notifications,
                        retail_tick: 301,
                        result_screen:
                            crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                        hive_dying: Default::default(),
                    },
                )
                .unwrap();
            }
            let destination = f
                .manager
                .iter_all()
                .find(|entity| entity.capability_flags & 0x10 != 0)
                .unwrap()
                .id;
            let health = f.manager.entity_mut(f.child).unwrap().collision.health_raw;
            let before_ids = f
                .scheduler
                .owners
                .iter()
                .map(SpecializedActorTaskOwner::entity_id)
                .collect::<Vec<_>>();
            let mut pending = Vec::new();
            let mut retained = Vec::new();
            match storage {
                Storage::Scheduler => {}
                Storage::Pending => pending = std::mem::take(&mut f.scheduler.owners),
                Storage::Retained => retained = std::mem::take(&mut f.scheduler.owners),
            }
            let mut cursor = Intro2RadialCursorCustody {
                pending: &mut pending,
                retained: &mut retained,
                remaining_live_ids: &[],
            };
            let tasks: &mut dyn CaptureTaskCustody = match storage {
                Storage::Scheduler => &mut f.scheduler,
                Storage::Pending | Storage::Retained => &mut cursor,
            };
            execute_capture_delivery(
                &mut f.manager,
                f.parent,
                destination,
                &mut CaptureContext {
                    resources: None,
                    tasks,
                    world_fx: &mut f.fx,
                    notifications: &mut f.notifications,
                    retail_tick: 302,
                    result_screen:
                        crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
                    hive_dying: Default::default(),
                },
            )
            .unwrap();
            let owners = match storage {
                Storage::Scheduler => &mut f.scheduler.owners,
                Storage::Pending => &mut pending,
                Storage::Retained => &mut retained,
            };
            assert_eq!(
                owners
                    .iter()
                    .filter(|owner| !owner.is_delivered_type9_contact())
                    .map(SpecializedActorTaskOwner::entity_id)
                    .collect::<Vec<_>>(),
                before_ids
                    .into_iter()
                    .filter(|id| *id != f.child)
                    .collect::<Vec<_>>(),
                "443D10 retires only the delivered child's ordinary task visit"
            );
            let child = f.manager.entity_mut(f.child).unwrap();
            assert_eq!(child.attached_to, None);
            assert_eq!(child.collision.health_raw, health);
            assert_eq!(
                child
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_scheduler::COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT),
                RetailRuntimeValue::Known(0)
            );
            assert!(f
                .manager
                .pending_actor_deferred_destroy_ids()
                .contains(&f.child));
            let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
            assert!(
                ready(owners, &f.manager, f.child),
                "completed16750 tasks retain exact contact custody until14990"
            );
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                expected_fx.next_shared_retail_random_u16()
            );
            assert!(f
                .manager
                .cleanup_pending_actor_deferred_destroys()
                .contains(&f.child));
            assert!(!f.manager.iter_all().any(|entity| entity.id == f.child));
        }
    }
}

#[v2k_test_support::retail_test]
fn child_type8_committed_death_error_parks_in_each_actual_owner_storage() {
    for storage in [Storage::Scheduler, Storage::Pending, Storage::Retained] {
        let Some((session, mut manager, _)) =
            crate::intro2_type47_live::world::native_intro2_fixture()
        else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.entity_type == 8)
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::default();
        scheduler.adopt_intro2_type8(&mut manager);
        let mut pending = Vec::new();
        let mut retained = Vec::new();
        match storage {
            Storage::Scheduler => {}
            Storage::Pending => pending = std::mem::take(&mut scheduler.owners),
            Storage::Retained => retained = std::mem::take(&mut scheduler.owners),
        }
        manager.entity_mut(id).unwrap().sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
        let before_primary = manager
            .entity_mut(id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let mut fx = WorldFx::new();
        let mut control = fx.fork_for_main_base_abort_transaction();
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
        let mut prefix_calls = 0;
        for attempt in 0..2 {
            let frame = CaptureChildFrame {
                child: id,
                operation: CaptureChildOperation::StandardDeath,
                world_fx: &mut fx,
                notifications: &mut notifications,
                retail_tick: 301,
                result_screen:
                    crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
            };
            let mut prefix = |_: &mut EntityManager| {
                prefix_calls += 1;
                Ok(())
            };
            let result = match storage {
                Storage::Scheduler => {
                    scheduler.mutate_capture_child(&mut manager, frame, &mut prefix)
                }
                Storage::Pending | Storage::Retained => Intro2RadialCursorCustody {
                    pending: &mut pending,
                    retained: &mut retained,
                    remaining_live_ids: &[],
                }
                .mutate_capture_child(&mut manager, frame, &mut prefix),
            };
            let error = result.unwrap_err();
            assert_eq!(error.committed_prefix, attempt == 0);
            let entity = manager.entity_mut(id).unwrap();
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(crate::entity_collision_state::DYING_STATE_BIT),
                RetailRuntimeValue::Known(crate::entity_collision_state::DYING_STATE_BIT)
            );
            assert_eq!(
                entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                before_primary,
                "failed C3A0 has not replaced the original Primary"
            );
            assert!(
                matches!(entity.current_behavior_context, RetailRuntimeValue::Known(Some(context))
                if context.active_style().audited().unwrap().class_id == 14)
            );
        }
        assert_eq!(
            prefix_calls, 1,
            "retry cannot replay the caller or child prefix"
        );
        let owners = match storage {
            Storage::Scheduler => &mut scheduler.owners,
            Storage::Pending => &mut pending,
            Storage::Retained => &mut retained,
        };
        let index = owners
            .iter()
            .position(|owner| owner.entity_id() == id)
            .unwrap();
        let SpecializedActorTaskOwner::Intro2Type8(owner) = owners.remove(index) else {
            panic!()
        };
        let tick = crate::intro2_type8::tick_intro2_type8(
            &mut manager,
            owner,
            crate::intro2_type8::Intro2Type8Frame {
                resources: &session.cache,
                world_fx: &mut fx,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 302,
            },
        );
        assert_eq!(
            tick.outcome,
            crate::intro2_type8::Intro2Type8Outcome::Pending { entity_id: id }
        );
        assert!(tick.retained_owner.is_some());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
    }
}

#[test]
fn living_release_resource_callback_precedes_cue_and_primary_rng() {
    use crate::attract_attention::{
        AttractAttentionResourceTextRequest, ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
        ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
    };
    use crate::entity_collision_state::RetailStateWord;
    use crate::ordinary_type9_go_to_job_initializer::OrdinaryType9GoToJobCandidateEvidence;
    use crate::ordinary_type9_root_reselection::OrdinaryType9RootEntityRef;
    let (link, metadata) =
        crate::ordinary_type9_initial_production::exact_wander_link_ready_fixture();
    let (mut entity, _, _) = link.into_parts();
    let position = entity.position_raw();
    let captor = Type9RelationOwner {
        id: 0x7000,
        capability_flags: 8,
        position_raw: position,
    };
    let plan = ordinary_type9_cargo::plan_attach(&entity, &metadata, captor).unwrap();
    let allocation = MainBaseAbortActorLease {
        entity_id: entity.id,
        allocation_identity: 1,
    };
    // Detached callback input: production attachment and manager custody are
    // covered by the actual captured-row tests, independently of this order.
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x1000, 0x1000);
    entity.attached_to = Some(captor.id);
    let carried = ordinary_type9_cargo::commit_attach(&mut entity, allocation, plan).unwrap();
    let candidates = [
        (entity.id, 9, entity.capability_flags),
        (0x7100, 17, 8),
        (0x7200, 46, 1),
        (0x7300, 99, 0x20),
    ]
    .map(|(id, entity_type, capability)| OrdinaryType9RootEntityRef {
        id,
        entity_type,
        position_raw: position,
        state_flags_raw: RetailStateWord::exact(1),
        capability_flags: RetailRuntimeValue::Known(capability),
        attached_entity_handle: RetailRuntimeValue::Known(None),
    });
    let jobs = candidates
        .iter()
        .map(|candidate| OrdinaryType9GoToJobCandidateEvidence {
            candidate_id: candidate.id,
            state_flags: candidate.state_flags_raw,
            capability_flags: candidate.capability_flags,
            capacity: RetailRuntimeValue::Known((candidate.id == 0x7300).then_some(
                crate::job_nearby::JobCapacityState {
                    current_jobs_raw: 0,
                    capacity_raw: 1,
                },
            )),
        })
        .collect::<Vec<_>>();
    let request = Type9CargoReleaseRequest {
        metadata: &metadata,
        candidates: &candidates,
        job_evidence: &jobs,
        position: Type9CargoReleasePosition::Retained,
    };
    let prepared = ordinary_type9_cargo::prepare_release(&entity, &carried, &request).unwrap();
    let events = std::cell::RefCell::new(Vec::new());
    let mut words = 0;
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let mut expected = notifications.clone();
    expected
        .queue_attract_attention_resource_text(
            AttractAttentionResourceTextRequest {
                event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
                global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
            },
            301,
        )
        .unwrap();
    let publication = ordinary_type9_cargo::commit_release(
        &mut entity,
        carried,
        prepared,
        request,
        || {
            events.borrow_mut().push("rng");
            words += 1;
            if words == 1 {
                3063
            } else {
                0x4040
            }
        },
        |text| {
            notifications
                .queue_attract_attention_resource_text(text, 301)
                .unwrap();
            events.borrow_mut().push("resource");
        },
        |_| events.borrow_mut().push("sound"),
    )
    .unwrap();
    assert!(publication.authenticates(&entity));
    assert_eq!(notifications, expected);
    let events = events.into_inner();
    let resource = events
        .iter()
        .position(|event| *event == "resource")
        .unwrap();
    assert_eq!(
        events[resource + 1..]
            .iter()
            .filter(|event| **event == "rng")
            .count(),
        2,
        "BA40 emits resource before both Cue and Primary constructor RNG draws"
    );
    assert_eq!(words, 4, "selector, Candidate, Cue, Primary");
}
