//! Real NULL-J C910 kill keeps the opposite callbacks and12760 suffix.

use super::{
    pair::{actor, *},
    *,
};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity_behavior::{BehaviorDescriptorIdentity, PairContactCallbackPolicy},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    player_active_contact::{active_pair_body_from_entity, classify_oriented_active_pair_contact},
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

#[v2k_test_support::retail_test]
fn genuine_type40_null_j_kills_native_person_then_runs_both_components_and_separation() {
    let (mut session, mut entities, mut fx) =
        crate::native_type122::construction_tests::native_fixture(19);
    let parent = entities
        .iter_all()
        .find(|e| e.entity_type == 40)
        .unwrap()
        .id;
    // Type79 is this world's actual native worker/person, with its own body,
    // selected graph and Class14 terminal. No Type8/Type9 receipt is copied.
    let child = entities
        .iter_all()
        .find(|e| e.entity_type == 79)
        .unwrap()
        .id;
    let center = [4096, 6000, 4096];
    let ids = entities.retail_live_order_ids().collect::<Vec<_>>();
    for &id in &ids {
        let e = entities.entity_mut(id).unwrap();
        e.set_motion_raw(
            if [parent, child].contains(&id) {
                center
            } else {
                [16000, 16000, 16000]
            },
            [0; 3],
        );
        if [parent, child].contains(&id) {
            e.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
            e.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        }
    }
    let (subject, candidate) =
        if ids.iter().position(|id| *id == parent) < ids.iter().position(|id| *id == child) {
            (parent, child)
        } else {
            (child, parent)
        };
    let mut contact = None;
    'search: for dy in [-150i16, -75, 0, 75, 150] {
        for dx in [-240i16, -160, -80, 0, 80, 160, 240] {
            for dz in [-240i16, -160, -80, 0, 80, 160, 240] {
                entities.entity_mut(child).unwrap().set_position_raw([
                    center[0].wrapping_add(dx),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz),
                ]);
                let subject_actor = actor(&entities, subject).unwrap();
                let entry = active_pair_body_from_entity(subject_actor, &session.cache);
                let found = classify_oriented_active_pair_contact(
                    crate::player_active_contact::OrientedActivePairContactRequest {
                        subject: subject_actor,
                        candidate: actor(&entities, candidate).unwrap(),
                        subject_entry: &entry,
                        retail_tick: 5000,
                    },
                    &session.cache,
                )
                .expect("canonical full Type40/person model program");
                if let Some(found) = found.filter(|found| found.penetration_raw >= 32) {
                    contact = Some(found);
                    break 'search;
                }
            }
        }
    }
    let contact = contact.expect("genuine Type40/person narrow solids overlap");
    for _ in 0..128 {
        crate::native_ground_actor::behavior::secondary::<
            crate::native_type40::profile::Type40Profile,
        >(&mut entities, parent, 20_000, 4999, &mut fx)
        .unwrap();
        if matches!(actor(&entities, parent).unwrap().current_behavior_context,
            RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c8038)
        {
            break;
        }
    }
    let parent_before = actor(&entities, parent).unwrap();
    assert_eq!(
        parent_before.sub_j_attachment_runtime,
        RetailRuntimeValue::Known(None)
    );
    assert!(parent_before.native_capture_relation.is_none());
    assert!(
        matches!(parent_before.current_behavior_context, RetailRuntimeValue::Known(Some(context))
        if context.active_style().style_address() == 0x004c8038
            && context.active_style().pair_contact_callback_policy() == PairContactCallbackPolicy::CapturePeople)
    );
    let root = parent_before.current_behavior_context;
    let parent_tasks = ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .map(|slot| parent_before.actor_tasks.task_in_slot(slot));
    let positions = [subject, candidate].map(|id| actor(&entities, id).unwrap().position_raw());
    assert!(actor(&entities, child).unwrap().capability_flags & 0xc00 != 0);
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_type40(&entities);
    tasks.adopt_intro2_type8(&mut entities);
    assert!(tasks.prepare_native_actor_mutation(&entities, parent));
    assert!(tasks.prepare_native_actor_mutation(&entities, child));

    // Independent child-terminal control accounts for its own source effects.
    // The actual pair may add only the reported native component-contact draws,
    // never a Type40 weighted root/transport selection or capture hint8.
    let mut control_entities = entities.fork_for_main_base_abort_transaction();
    let mut control_tasks = tasks.fork_for_main_base_abort_transaction();
    let mut control_fx = fx.fork_for_main_base_abort_transaction();
    let mut control_notifications = GameplayNotifications::new();
    kill_capture_contact(
        &mut control_entities,
        child,
        &mut CaptureContext {
            resources: None,
            tasks: &mut control_tasks,
            world_fx: &mut control_fx,
            notifications: &mut control_notifications,
            retail_tick: 5000,
            result_screen: crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
            hive_dying: Default::default(),
        },
    )
    .unwrap();
    let mut notifications = GameplayNotifications::new();
    let mut static_damage = StaticDamageScheduler::new();
    let outcome = resolve_native_captor_active_contacts(
        &mut Intro2ContactFrame {
            entities: &mut entities,
            resources: &mut session.cache,
            world_fx: &mut fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 5000,
            actor_tasks: &mut tasks,
        },
        subject,
        CaptureFeedbackPolicy::Gameplay,
    );
    let NativeCaptorPairOutcome::Resolved { visits } = outcome else {
        panic!("{outcome:?}")
    };
    assert_eq!(visits.len(), 1);
    let visit = &visits[0];
    assert_eq!(visit.candidate_id, candidate);
    assert_eq!(visit.contact, contact);
    assert!(!visit.physical_suppressed);
    let behavior_owners = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            NativeCaptorPairStage::Behavior { owner, result, .. } => {
                assert_eq!(*result, NativeCaptorPairBehaviorResult::Null);
                Some(*owner)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(behavior_owners, [subject, candidate]);
    let components = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            NativeCaptorPairStage::Component {
                owner,
                slot,
                result,
            } => Some((*owner, *slot, *result)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let expected_slots = [subject, candidate]
        .into_iter()
        .flat_map(|id| ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| (id, slot)))
        .collect::<Vec<_>>();
    assert_eq!(
        components
            .iter()
            .map(|(id, slot, _)| (*id, *slot))
            .collect::<Vec<_>>(),
        expected_slots
    );
    for (_, _, result) in components {
        if let NativeCaptorPairComponentResult::Applied { rng_draws } = result {
            for _ in 0..rng_draws {
                control_fx.next_shared_retail_random_u16();
            }
        }
    }
    assert!(matches!(
        visit.stages.last(),
        Some(NativeCaptorPairStage::Physical {
            capped_damage_raw: 0
        })
    ));
    assert_ne!(
        [subject, candidate].map(|id| actor(&entities, id).unwrap().position_raw()),
        positions,
        "12760 separates after the NULL C910 result"
    );
    let child_after = actor(&entities, child).unwrap();
    assert_eq!(
        child_after.collision.health_raw,
        RetailRuntimeValue::Known(0)
    );
    assert!(
        matches!(child_after.current_behavior_context, RetailRuntimeValue::Known(Some(context))
        if matches!(context.descriptor(), BehaviorDescriptorIdentity::Named(program) if program.class_id == 14))
    );
    assert!(tasks.prepare_native_actor_mutation(&entities, child));
    let parent_after = actor(&entities, parent).unwrap();
    assert_eq!(parent_after.current_behavior_context, root);
    assert_eq!(
        parent_after.sub_j_attachment_runtime,
        RetailRuntimeValue::Known(None)
    );
    assert!(parent_after.native_capture_relation.is_none());
    assert_eq!(
        ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| parent_after.actor_tasks.task_in_slot(slot)),
        parent_tasks
    );
    assert!(matches!(
        parent_after.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CapturePeoplePursuit(task)) if task.target_id() == Some(child)
    ));
    assert!(crate::native_type40::manager_allocation_authenticates(
        &entities, parent
    ));
    assert_eq!(notifications, control_notifications);
    assert_eq!(notifications.save_tail_seen_mask() & (1 << 8), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control_fx.next_shared_retail_random_u16()
    );
}
