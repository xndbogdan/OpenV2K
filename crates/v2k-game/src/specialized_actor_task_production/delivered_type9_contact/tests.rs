use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::{ActorTaskId, ActorTaskSlot, ActorTaskWrapperFlags},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type17::capture::{execute_capture_delivery, CaptureContext},
    intro2_type17::pair::{
        tests::{ChildOrder, Fixture},
        CaptureFeedbackPolicy,
    },
    main_base_type9_abort::MainBaseType9ResultScreenState,
    native_actor_capture::pair::{
        resolve_native_captor_active_contacts, NativeCaptorPairBlock, NativeCaptorPairOutcome,
    },
};

fn tasks(
    manager: &EntityManager,
    id: u32,
) -> [(
    Option<ActorTaskId>,
    Option<ActorTaskRuntime>,
    Option<ActorTaskWrapperFlags>,
); 3] {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        let task = entity.actor_tasks.task_in_slot(slot);
        (
            task,
            entity.actor_task_state(slot).copied(),
            task.and_then(|task| entity.actor_tasks.wrapper_flags(task)),
        )
    })
}

fn delivered_fixture(world: u32) -> Fixture {
    let mut f = Fixture::ready(
        world,
        (world == 50).then_some(30),
        if world == 50 {
            ChildOrder::AuthoredSpawn(58)
        } else {
            ChildOrder::First
        },
    );
    let captured = f.pair(CaptureFeedbackPolicy::Cinematic);
    assert!(
        matches!(captured, NativeCaptorPairOutcome::Resolved { .. }),
        "{captured:?}"
    );
    assert_eq!(
        f.entities.entity_mut(f.child).unwrap().attached_to,
        Some(f.spider)
    );
    if world == 50 {
        assert_eq!(
            f.entities
                .entity_mut(f.spider)
                .unwrap()
                .authored_spawn_index,
            Some(30)
        );
        assert_eq!(
            f.entities.entity_mut(f.child).unwrap().authored_spawn_index,
            Some(58)
        );
        assert_eq!(
            f.entities.entity_mut(f.child).unwrap().model_index,
            Some(558)
        );
    }
    let destination = f
        .entities
        .iter_all()
        .find(|entity| entity.capability_flags & 0x10 != 0)
        .unwrap()
        .id;
    execute_capture_delivery(
        &mut f.entities,
        f.spider,
        destination,
        &mut CaptureContext {
            resources: None,
            tasks: &mut f.scheduler,
            world_fx: &mut f.fx,
            notifications: &mut f.notifications,
            retail_tick: f.tick,
            result_screen: MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    // The production D0B0 pair adapter adopts the captor after completedC690.
    // Direct callback entry in this fixture must retain that same new owner
    // before either the original-deletion or restored-contact branch runs.
    f.scheduler.register_intro2_type17(
        crate::intro2_type17::Intro2Type17Owner::adopt(&f.entities, f.spider).unwrap(),
    );
    assert_eq!(f.entities.entity_mut(f.child).unwrap().attached_to, None);
    assert_eq!(
        f.entities
            .entity_mut(f.child)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .masked(0x160000),
        RetailRuntimeValue::Known(0x100000)
    );
    assert_eq!(
        f.entities
            .pending_actor_deferred_destroy_ids()
            .iter()
            .filter(|id| **id == f.child)
            .count(),
        1
    );
    assert!(authenticated_retained_type9_contact(
        &f.scheduler.owners,
        &f.entities,
        f.child
    ));
    assert!(f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, f.child));
    f.place_oriented_contact();
    f
}

#[v2k_test_support::retail_test]
fn real_intro2_and_ordinary_deliveries_retain_contact_custody_before14990() {
    for world in [50, 14] {
        let mut f = delivered_fixture(world);
        // Reproduce the original owner deletion against the identical actual
        // allocation, task publication, intrusive direction and oriented body.
        let mut baseline_entities = f.entities.fork_for_main_base_abort_transaction();
        let mut baseline_scheduler = f.scheduler.fork_for_main_base_abort_transaction();
        let mut baseline_fx = f.fx.fork_for_main_base_abort_transaction();
        let mut baseline_notifications = GameplayNotifications::new();
        let mut baseline_damage = crate::static_damage::StaticDamageScheduler::new();
        baseline_scheduler
            .owners
            .retain(|owner| owner.entity_id() != f.child);
        let baseline = resolve_native_captor_active_contacts(
            &mut Intro2ContactFrame {
                entities: &mut baseline_entities,
                resources: &mut f.session.cache,
                world_fx: &mut baseline_fx,
                static_damage: &mut baseline_damage,
                notifications: &mut baseline_notifications,
                retail_tick: f.tick,
                actor_tasks: &mut baseline_scheduler,
            },
            f.subject,
            CaptureFeedbackPolicy::Cinematic,
        );
        assert!(
            matches!(baseline, NativeCaptorPairOutcome::Blocked {
            reason: NativeCaptorPairBlock::BodyCustody { entity_id }, ..
        } if entity_id == f.child),
            "world{world} original retirement: {baseline:?}"
        );
        let current = f.pair(CaptureFeedbackPolicy::Cinematic);
        let NativeCaptorPairOutcome::Resolved { visits } = current else {
            panic!("world{world} retained release contact: {current:?}");
        };
        assert!(visits.iter().any(|visit| visit.candidate_id == f.candidate));
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
        assert!(authenticated_retained_type9_contact(
            &f.scheduler.owners,
            &f.entities,
            f.child
        ));
        assert_eq!(
            f.entities
                .pending_actor_deferred_destroy_ids()
                .iter()
                .filter(|id| **id == f.child)
                .count(),
            1
        );
        assert!(f
            .entities
            .cleanup_pending_actor_deferred_destroys()
            .contains(&f.child));
        assert!(!f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
    }
}

#[v2k_test_support::retail_test]
fn delivered_publication_never_advances_an_ordinary_task_before_removal() {
    for world in [50, 14] {
        let mut f = delivered_fixture(world);
        f.scheduler
            .owners
            .retain(|owner| owner.entity_id() == f.child);
        let before = tasks(&f.entities, f.child);
        let position = f.entities.entity_mut(f.child).unwrap().position_raw();
        let context = f
            .entities
            .entity_mut(f.child)
            .unwrap()
            .current_behavior_context;
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let pass = f.scheduler.tick(
            &mut f.entities,
            SpecializedActorTaskProductionFrame {
                world:
                    crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                resources: &mut f.session.cache,
                world_fx: &mut f.fx,
                static_damage: &mut f.static_damage,
                elapsed_micros: 40_000,
                global_elapsed_micros: 40_000,
                retail_tick: f.tick + 1,
                notification_phase: GameplayNotificationPhase::NonGameplay,
                main_base_abort_active: false,
            },
            &mut f.notifications,
        );
        assert_eq!(pass.block, None);
        assert!(
            pass.outcomes.is_empty(),
            "contact-only receipt lends no actor visit"
        );
        assert_eq!(tasks(&f.entities, f.child), before);
        assert_eq!(
            f.entities.entity_mut(f.child).unwrap().position_raw(),
            position
        );
        assert_eq!(
            f.entities
                .entity_mut(f.child)
                .unwrap()
                .current_behavior_context,
            context
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
    }
}

#[v2k_test_support::retail_test]
fn delivered_contact_requires_its_queue_allocation_and_completed_graph() {
    let mut f = delivered_fixture(50);
    let before = tasks(&f.entities, f.child);
    let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
    f.entities
        .entity_mut(f.child)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x100000, 0);
    assert!(!f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, f.child));
    assert_eq!(tasks(&f.entities, f.child), before);
    f.entities
        .entity_mut(f.child)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x100000, 0x100000);
    let other = delivered_fixture(50);
    assert!(
        !f.scheduler
            .prepare_native_actor_mutation(&other.entities, f.child),
        "matching model/id never lends an old allocation lease"
    );
    let mut altered_entities = f.entities.fork_for_main_base_abort_transaction();
    let mut altered_scheduler = f.scheduler.fork_for_main_base_abort_transaction();
    altered_entities
        .entity_mut(f.child)
        .unwrap()
        .actor_tasks
        .clear_slot(ActorTaskSlot::Primary);
    assert!(
        !altered_scheduler.prepare_native_actor_mutation(&altered_entities, f.child),
        "pending allocation alone cannot replace the completed graph"
    );
    assert!(f.scheduler.park_native_contact_prefix(&f.entities, f.child));
    assert!(!f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, f.child));
    assert_eq!(tasks(&f.entities, f.child), before);
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn repeated_native_delivery_preserves_already_pending_state_and_one_receipt() {
    use crate::native_actor_capture::{
        attach_capture_child, execute_capture_root, CaptureRootCallback,
    };
    for world in [50, 14] {
        let mut f = delivered_fixture(world);
        // Enter the actual C910 callback with the empty genuine Sub-J row.
        // This invokes CD70/ADB0 on the owned released child; no carried
        // context or task is reconstructed by this phase fixture.
        attach_capture_child(
            &mut f.entities,
            f.spider,
            f.child,
            &mut CaptureContext {
                resources: None,
                tasks: &mut f.scheduler,
                world_fx: &mut f.fx,
                notifications: &mut f.notifications,
                retail_tick: f.tick,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap();
        assert_eq!(
            f.entities.entity_mut(f.child).unwrap().attached_to,
            Some(f.spider)
        );
        assert!(authenticated_retained_type9_contact(
            &f.scheduler.owners,
            &f.entities,
            f.child
        ));
        // Stress the independent callback-enable words on this authentic
        // pending graph. 16750/D440 and CE90 own their edits; a second10B70
        // must leave whatever those callbacks produce exactly intact.
        f.entities
            .entity_mut(f.child)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x60000, 0x60000);
        let mut expected_entities = f.entities.fork_for_main_base_abort_transaction();
        let mut expected_scheduler = f.scheduler.fork_for_main_base_abort_transaction();
        let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
        let mut expected_notifications = GameplayNotifications::new();
        // D040's cleanup owns the identical16750 callback without10B70.
        // Compare that native release with D0B0's repeated destroy callback.
        execute_capture_root(
            &mut expected_entities,
            f.spider,
            CaptureRootCallback::Cleanup,
            &mut CaptureContext {
                resources: None,
                tasks: &mut expected_scheduler,
                world_fx: &mut expected_fx,
                notifications: &mut expected_notifications,
                retail_tick: f.tick,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap();
        let destination = f
            .entities
            .iter_all()
            .find(|entity| entity.capability_flags & 0x10 != 0)
            .unwrap()
            .id;
        execute_capture_delivery(
            &mut f.entities,
            f.spider,
            destination,
            &mut CaptureContext {
                resources: None,
                tasks: &mut f.scheduler,
                world_fx: &mut f.fx,
                notifications: &mut f.notifications,
                retail_tick: f.tick,
                result_screen: MainBaseType9ResultScreenState::NotShown,
                hive_dying: Default::default(),
            },
        )
        .unwrap();
        let expected_state = expected_entities
            .entity_mut(f.child)
            .unwrap()
            .collision
            .state_flags_at_0x08;
        assert_eq!(
            expected_state.masked(0x160000),
            RetailRuntimeValue::Known(0x160000),
            "native release keeps independent callback words on the queued allocation"
        );
        assert_eq!(
            f.entities
                .entity_mut(f.child)
                .unwrap()
                .collision
                .state_flags_at_0x08,
            expected_state
        );
        assert_eq!(
            tasks(&f.entities, f.child),
            tasks(&expected_entities, f.child)
        );
        assert_eq!(
            f.entities
                .entity_mut(f.child)
                .unwrap()
                .current_behavior_context,
            expected_entities
                .entity_mut(f.child)
                .unwrap()
                .current_behavior_context
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected_fx.next_shared_retail_random_u16()
        );
        assert_eq!(
            f.entities
                .pending_actor_deferred_destroy_ids()
                .iter()
                .filter(|id| **id == f.child)
                .count(),
            1
        );
        let SpecializedActorTaskOwner::DeliveredType9Contact { retained, .. } = f
            .scheduler
            .owners
            .iter()
            .find(|owner| owner.entity_id() == f.child)
            .unwrap()
        else {
            panic!()
        };
        assert!(
            !retained.is_delivered_type9_contact(),
            "repeated callback retains one linear inner graph"
        );
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
        assert!(f
            .entities
            .cleanup_pending_actor_deferred_destroys()
            .contains(&f.child));
        assert!(!f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
    }
}
