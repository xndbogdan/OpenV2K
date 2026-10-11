use super::*;
use crate::{
    actor_task_owner::ActorTaskVisit, entity_behavior::behavior_program,
    entity_collision_state::RetailStateWord, shared_fish::tests::fixture,
};

fn fixture_with_owner() -> (EntityManager, WorldFx, SpecializedActorTaskScheduler, u32) {
    let (_, mut manager, fx) = fixture(30);
    // Native loading may already have queued non-fish deferred removals.
    // Finish that ordinary sweep before isolating this fish's death callback.
    let fish_ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 22 | 24 | 124))
        .map(|entity| entity.id)
        .collect();
    let removed = manager.cleanup_pending_actor_deferred_destroys();
    assert!(removed.iter().all(|id| !fish_ids.contains(id)));
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 22)
        .unwrap()
        .id;
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.adopt_shared_fish(&manager);
    (manager, fx, scheduler, id)
}

#[v2k_test_support::retail_test]
fn quiet_death_stages_then_sweeps_without_motion_sound_or_rng_and_repeated_entry_is_noop() {
    let (mut manager, mut fx, scheduler, id) = fixture_with_owner();
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    let before = manager.entity_mut(id).unwrap();
    let motion = (
        before.position_raw(),
        before.velocity_raw(),
        before.rotation_heading_pitch_roll_raw(),
    );
    let variables = before.shared_fish_runtime.as_ref().unwrap().variables;
    let outcome = begin_shared_fish_standard_death(&mut manager, id, &mut fx, &scheduler).unwrap();
    assert!(outcome.returned_nonzero);
    assert_eq!(outcome.publication, Some(()));
    assert!(completed_shared_fish_death(&manager, id));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert_eq!(
        (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.rotation_heading_pitch_roll_raw()
        ),
        motion
    );
    assert_eq!(
        entity.shared_fish_runtime.as_ref().unwrap().variables,
        variables
    );
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x0016_4000),
        RetailRuntimeValue::Known(0x0010_4000)
    );
    let before = entity.collision.clone();
    // A second10C10 reaches its already-dying return before it needs a live task.
    let repeated = begin_shared_fish_standard_death(
        &mut manager,
        id,
        &mut fx,
        &SpecializedActorTaskScheduler::new(),
    )
    .unwrap();
    assert!(repeated.returned_nonzero);
    assert!(repeated.publication.is_none());
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    assert_eq!(manager.pending_actor_deferred_destroy_ids(), &[id]);
    assert!(fx.take_positional_sounds().is_empty());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
    assert_eq!(manager.cleanup_pending_actor_deferred_destroys(), vec![id]);
    assert!(!completed_shared_fish_death(&manager, id));
}

#[v2k_test_support::retail_test]
fn remote_precedes_unresolved_dying_and_current_task_state() {
    let (mut manager, mut fx, _, id) = fixture_with_owner();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08 =
        RetailStateWord::from_known_bits(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);
    entity.current_behavior_context = RetailRuntimeValue::Unresolved;
    let before = entity.collision.clone();
    let outcome = begin_shared_fish_standard_death(
        &mut manager,
        id,
        &mut fx,
        &SpecializedActorTaskScheduler::new(),
    )
    .unwrap();
    assert!(!outcome.returned_nonzero);
    assert!(outcome.publication.is_none());
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
}

#[v2k_test_support::retail_test]
fn absent_stale_executing_and_parked_owners_reject_before_death_writes() {
    for failure in 0..4 {
        let (mut manager, mut fx, mut scheduler, id) = fixture_with_owner();
        match failure {
            0 => scheduler = SpecializedActorTaskScheduler::new(),
            1 => {
                // Publish a valid replacement while deliberately retaining the
                // scheduler's earlier context/slot receipt.
                let metadata = manager.type_runtime_metadata(22).unwrap().clone();
                crate::shared_fish_tasks::reselect_after_task_result(
                    manager.entity_mut(id).unwrap(),
                    &metadata,
                    &mut || 0,
                )
                .unwrap();
            }
            2 => {
                let entity = manager.entity_mut(id).unwrap();
                let visit = ActorTaskVisit {
                    slot: ActorTaskSlot::Primary,
                    task_id: entity
                        .actor_tasks
                        .task_in_slot(ActorTaskSlot::Primary)
                        .unwrap(),
                };
                assert!(entity
                    .actor_tasks
                    .begin_exact_visit_with(visit, |_| ())
                    .is_some());
            }
            _ => {
                assert!(scheduler.park_native_contact_prefix(&manager, id));
            }
        }
        let before = manager.entity_mut(id).unwrap().collision.clone();
        let result = begin_shared_fish_standard_death(&mut manager, id, &mut fx, &scheduler);
        assert!(
            matches!(
                result,
                Err(SharedFishDeathBlock::Runtime(
                    "completed native allocation/task custody"
                ))
            ),
            "{result:?}"
        );
        assert_eq!(manager.entity_mut(id).unwrap().collision, before);
        assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
    }
}

#[v2k_test_support::retail_test]
fn all_four_living_styles_enter_class2_but_fabricated_class2_has_no_terminal_receipt() {
    for (class, variant) in [(5, 0), (6, 0), (13, 0), (13, 1)] {
        let (mut manager, mut fx, mut scheduler, id) = fixture_with_owner();
        let program = behavior_program(class).unwrap();
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!()
        };
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(
            context
                .reselect_named_type_default(
                    program,
                    variant.into(),
                    *audited_behavior_style(class, variant).unwrap(),
                )
                .unwrap(),
        ));
        scheduler.register_shared_fish(
            crate::shared_fish::SharedFishOwner::adopt(&manager, id).unwrap(),
        );
        assert!(
            begin_shared_fish_standard_death(&mut manager, id, &mut fx, &scheduler)
                .unwrap()
                .publication
                .is_some()
        );
        assert!(completed_shared_fish_death(&manager, id));
        manager
            .entity_mut(id)
            .unwrap()
            .shared_fish_runtime
            .as_mut()
            .unwrap()
            .quiet_death_context = None;
        assert!(!completed_shared_fish_death(&manager, id));
    }
}

#[v2k_test_support::retail_test]
fn type124_never_enters_the_class2_quiet_terminal() {
    let (mut manager, mut fx, scheduler, _) = fixture_with_owner();
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 124)
        .unwrap()
        .id;
    let before = manager.entity_mut(id).unwrap().collision.clone();
    assert!(matches!(
        begin_shared_fish_standard_death(&mut manager, id, &mut fx, &scheduler),
        Err(SharedFishDeathBlock::UnsupportedDeathProgram {
            entity_type: 124,
            alternate_behavior_class: 63
        })
    ));
    assert_eq!(manager.entity_mut(id).unwrap().collision, before);
    assert!(manager.pending_actor_deferred_destroy_ids().is_empty());
}
