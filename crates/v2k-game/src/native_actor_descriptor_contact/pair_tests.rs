use super::*;
use crate::{
    intro2_type17::pair::{
        tests::{ChildOrder, Fixture},
        CaptureFeedbackPolicy, Type17PairOutcome, Type17PairStage,
    },
    native_actor_capture::pair::actor as pair_actor,
    player_active_contact::{
        active_pair_body_from_entity, classify_oriented_active_pair_contact,
        OrientedActivePairContactRequest,
    },
};

#[v2k_test_support::retail_test]
fn independent_type58_subject_completes_type58_counterpart_pair() {
    // World14's four ordinary Type58s carry native construction receipts.
    // With the spider parked, the earlier Type58's own 11AD0 pass must visit
    // the later Type58 through the current descriptor callbacks, without a
    // captor participant and with no UnresolvedBehavior before physical
    // writes. Unresolved families stay fail-closed at their existing
    // boundaries and are not whitelisted here.
    let mut f = Fixture::ready(14, Some(28), ChildOrder::First);
    f.scheduler.adopt_intro2_type58(&f.entities);
    let mut t58: Vec<u32> = f
        .entities
        .iter_all()
        .filter(|e| {
            e.entity_type == 58
                && crate::intro2_type58::type58_manager_allocation_authenticates(&f.entities, e.id)
        })
        .map(|e| e.id)
        .collect();
    t58.sort();
    assert!(t58.len() >= 2, "world14 authors four Type58s");
    let y = f.entities.entity_mut(f.spider).unwrap().position_raw()[1];
    f.entities
        .entity_mut(f.spider)
        .unwrap()
        .set_position_raw([0, y, 0]);
    f.entities
        .entity_mut(f.child)
        .unwrap()
        .set_position_raw([100, y, 100]);
    let (a, b) = (t58[0], t58[1]);
    let center = f.entities.entity_mut(a).unwrap().position_raw();
    let mut found = false;
    'outer: for dy in [-150_i16, -75, 0, 75, 150] {
        for dx in [80_i16, 160, 240, -80, -160, -240, 0] {
            for dz in [80_i16, 160, -80, -160, 0] {
                f.entities.entity_mut(b).unwrap().set_position_raw([
                    center[0].wrapping_add(dx),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz),
                ]);
                for id in [a, b] {
                    let e = f.entities.entity_mut(id).unwrap();
                    e.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
                    e.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
                    e.set_motion_raw(e.position_raw(), [0; 3]);
                }
                let order: Vec<u32> = f.entities.retail_live_order_ids().collect();
                let (subject, candidate) = if order.iter().position(|&id| id == a)
                    < order.iter().position(|&id| id == b)
                {
                    (a, b)
                } else {
                    (b, a)
                };
                let s = pair_actor(&f.entities, subject).unwrap();
                let c = pair_actor(&f.entities, candidate).unwrap();
                let contact = classify_oriented_active_pair_contact(
                    OrientedActivePairContactRequest {
                        subject: s,
                        candidate: c,
                        subject_entry: &active_pair_body_from_entity(s, &f.session.cache),
                        retail_tick: f.tick,
                    },
                    &f.session.cache,
                )
                .unwrap();
                if contact.is_some() {
                    f.subject = subject;
                    f.candidate = candidate;
                    found = true;
                    break 'outer;
                }
            }
        }
    }
    assert!(found, "two ordinary Type58s did not contact in scan");
    let subject = pair_actor(&f.entities, f.subject).unwrap();
    let candidate = pair_actor(&f.entities, f.candidate).unwrap();
    assert_eq!(subject.entity_type, 58);
    assert_eq!(candidate.entity_type, 58);
    let contact = classify_oriented_active_pair_contact(
        OrientedActivePairContactRequest {
            subject,
            candidate,
            subject_entry: &active_pair_body_from_entity(subject, &f.session.cache),
            retail_tick: f.tick,
        },
        &f.session.cache,
    )
    .unwrap()
    .expect("actual oriented Type58 contact");
    let plan = crate::active_pair::plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(subject, &f.session.cache),
        active_pair_body_from_entity(candidate, &f.session.cache),
        contact,
    )
    .unwrap();
    assert_eq!(plan.capped_pair_damage_raw, 0);
    let before_health = [subject.collision.health_raw, candidate.collision.health_raw];
    let result = f.pair(CaptureFeedbackPolicy::Cinematic);
    let Type17PairOutcome::Resolved { visits } = result else {
        panic!("{result:?}");
    };
    assert_eq!(visits.len(), 1, "isolated independent Type58 pair");
    let visit = &visits[0];
    assert_eq!(visit.candidate_id, f.candidate);
    assert!(!visit.physical_suppressed);
    // Both Class58 descriptor hooks run their current callbacks: no
    // UnresolvedBehavior or UnsupportedBehavior before the physical write.
    let behaviors: Vec<_> = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            Type17PairStage::Behavior { owner, result, .. } => Some((*owner, *result)),
            _ => None,
        })
        .collect();
    assert_eq!(
        behaviors,
        [
            (
                f.subject,
                crate::native_actor_capture::pair::NativeCaptorPairBehaviorResult::Null
            ),
            (
                f.candidate,
                crate::native_actor_capture::pair::NativeCaptorPairBehaviorResult::Null
            ),
        ]
    );
    assert_eq!(
        visit.stages.last(),
        Some(&Type17PairStage::Physical {
            capped_damage_raw: 0
        })
    );
    for (index, body) in [&plan.subject, &plan.candidate].into_iter().enumerate() {
        let entity = pair_actor(&f.entities, body.id).unwrap();
        assert_eq!(
            (entity.position_raw(), entity.velocity_raw()),
            (body.position_raw, body.velocity_raw)
        );
        assert_eq!(entity.collision.health_raw, before_health[index]);
        assert_eq!(entity.attached_to, None);
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, body.id));
    }
}

#[v2k_test_support::retail_test]
fn actual_type16_type53_type58_and_type94_components_complete_native_spider_pairs() {
    // Type53 has real allocations on both sides of spider spawn30. Type94
    // exists only at spawn43, so only that authored intrusive order is valid.
    for (world, kind, spawn, spider_first) in [
        (50, 16, Some(5), false),
        (50, 53, Some(20), false),
        (50, 53, Some(38), true),
        (50, 58, Some(40), true),
        (50, 94, Some(43), true),
        (14, 58, None, true),
    ] {
        let mut f = Fixture::ready(
            world,
            if world == 50 { Some(30) } else { Some(28) },
            ChildOrder::First,
        );
        f.scheduler.adopt_intro2_type16(&f.entities);
        f.scheduler.adopt_intro2_type53(&f.entities);
        f.scheduler.adopt_intro2_type58(&f.entities);
        f.scheduler.adopt_intro2_type94(&f.entities);
        let old_child = f.child;
        let id = f
            .entities
            .iter_all()
            .find(|entity| {
                entity.entity_type == kind
                    && spawn.is_none_or(|spawn| entity.authored_spawn_index == Some(spawn))
            })
            .unwrap()
            .id;
        let y = actor(&f.entities, old_child).unwrap().position_raw()[1];
        f.entities
            .entity_mut(old_child)
            .unwrap()
            .set_position_raw([0, y, 0]);
        f.child = id;
        let order = f.entities.retail_live_order_ids().collect::<Vec<_>>();
        let actual_spider_first = order.iter().position(|&id| id == f.spider)
            < order.iter().position(|&candidate| candidate == id);
        assert_eq!(actual_spider_first, spider_first);
        (f.subject, f.candidate) = if actual_spider_first {
            (f.spider, id)
        } else {
            (id, f.spider)
        };
        if world == 14 {
            // Medaeval's reported boundary was a real Class12 spider body
            // touching a living Type58 after the spider's lethal event.
            let corpse = crate::intro2_common_dying::publish_intro2_common_standard_death(
                &mut f.entities,
                f.spider,
                &mut f.fx,
            )
            .unwrap()
            .unwrap();
            f.scheduler.register_intro2_common_dying(corpse);
        }
        let entity = f.entities.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        f.place_oriented_contact();
        for body in [f.spider, id] {
            let entity = f.entities.entity_mut(body).unwrap();
            entity.set_motion_raw(entity.position_raw(), [0; 3]);
        }
        let subject = actor(&f.entities, f.subject).unwrap();
        let candidate = actor(&f.entities, f.candidate).unwrap();
        let contact = classify_oriented_active_pair_contact(
            OrientedActivePairContactRequest {
                subject,
                candidate,
                subject_entry: &active_pair_body_from_entity(subject, &f.session.cache),
                retail_tick: f.tick,
            },
            &f.session.cache,
        )
        .unwrap()
        .unwrap();
        let plan = crate::active_pair::plan_active_pair_response_and_damage_cap(
            active_pair_body_from_entity(subject, &f.session.cache),
            active_pair_body_from_entity(candidate, &f.session.cache),
            contact,
        )
        .unwrap();
        assert_eq!(plan.capped_pair_damage_raw, 0);
        let before_health = [subject.collision.health_raw, candidate.collision.health_raw];
        let before_h = actor(&f.entities, id)
            .unwrap()
            .sub_h_external_frame_runtime
            .clone();
        let result = f.pair(CaptureFeedbackPolicy::Cinematic);
        let Type17PairOutcome::Resolved { visits } = result else {
            panic!("world={world}, type={kind}, spawn={spawn:?}: {result:?}")
        };
        assert_eq!(visits.len(), 1);
        let visit = &visits[0];
        assert!(!visit.physical_suppressed);
        assert!(visit.stages.iter().any(|stage| match stage {
            Type17PairStage::Component {
                owner,
                slot: ActorTaskSlot::Primary,
                result,
            } if *owner == id => match result {
                NativeDescriptorContactOutcome::Behind
                | NativeDescriptorContactOutcome::Applied { .. } => true,
                NativeDescriptorContactOutcome::Null => kind == 58,
            },
            _ => false,
        }));
        assert_eq!(
            visit.stages.last(),
            Some(&Type17PairStage::Physical {
                capped_damage_raw: 0
            })
        );
        for (index, body) in [&plan.subject, &plan.candidate].into_iter().enumerate() {
            let entity = actor(&f.entities, body.id).unwrap();
            assert_eq!(
                (entity.position_raw(), entity.velocity_raw()),
                (body.position_raw, body.velocity_raw)
            );
            assert_eq!(entity.collision.health_raw, before_health[index]);
            assert_eq!(entity.attached_to, None);
            assert!(f
                .scheduler
                .prepare_native_actor_mutation(&f.entities, body.id));
        }
        assert_eq!(
            actor(&f.entities, id).unwrap().sub_h_external_frame_runtime,
            before_h,
            "01A20 does not advance the independent Sub-H joints"
        );
        assert!(f.scheduler.park_native_contact_prefix(&f.entities, id));
        assert!(!f.scheduler.prepare_native_actor_mutation(&f.entities, id));
    }
}

#[v2k_test_support::retail_test]
fn native_hive_null_contact_still_completes_spider_pair_physical_response() {
    let mut f = Fixture::ready(50, Some(30), ChildOrder::First);
    let hive = f
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == 67)
        .unwrap()
        .id;
    let old_child = f.child;
    let y = actor(&f.entities, old_child).unwrap().position_raw()[1];
    f.entities
        .entity_mut(old_child)
        .unwrap()
        .set_position_raw([0, y, 0]);
    f.child = hive;
    let order = f.entities.retail_live_order_ids().collect::<Vec<_>>();
    assert!(
        order.iter().position(|&id| id == hive) < order.iter().position(|&id| id == f.spider),
        "the native Intro2 hive precedes spider spawn30"
    );
    (f.subject, f.candidate) = (hive, f.spider);
    let entity = f.entities.entity_mut(hive).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000, 0x8000);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    f.place_oriented_contact();
    for id in [hive, f.spider] {
        let entity = f.entities.entity_mut(id).unwrap();
        entity.set_motion_raw(entity.position_raw(), [0; 3]);
    }
    let subject = actor(&f.entities, hive).unwrap();
    let candidate = actor(&f.entities, f.spider).unwrap();
    let contact = classify_oriented_active_pair_contact(
        OrientedActivePairContactRequest {
            subject,
            candidate,
            subject_entry: &active_pair_body_from_entity(subject, &f.session.cache),
            retail_tick: f.tick,
        },
        &f.session.cache,
    )
    .unwrap()
    .unwrap();
    let plan = crate::active_pair::plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(subject, &f.session.cache),
        active_pair_body_from_entity(candidate, &f.session.cache),
        contact,
    )
    .unwrap();
    assert_eq!(plan.capped_pair_damage_raw, 0);
    let before_health = [subject.collision.health_raw, candidate.collision.health_raw];
    let result = f.pair(CaptureFeedbackPolicy::Cinematic);
    let Type17PairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    assert_eq!(visits.len(), 1);
    assert!(!visits[0].physical_suppressed);
    assert!(visits[0].stages.iter().any(|stage| matches!(stage,
        Type17PairStage::Component { owner, slot: ActorTaskSlot::Primary,
            result: NativeDescriptorContactOutcome::Null } if *owner == hive)));
    assert_eq!(
        visits[0].stages.last(),
        Some(&Type17PairStage::Physical {
            capped_damage_raw: 0
        })
    );
    for (index, body) in [&plan.subject, &plan.candidate].into_iter().enumerate() {
        let entity = actor(&f.entities, body.id).unwrap();
        assert_eq!(
            (entity.position_raw(), entity.velocity_raw()),
            (body.position_raw, body.velocity_raw)
        );
        assert_eq!(entity.collision.health_raw, before_health[index]);
        assert_eq!(entity.attached_to, None);
    }
    assert!(f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, f.spider));
}

#[v2k_test_support::retail_test]
fn all_existing_native_common_dying_families_share_completed_contact_custody() {
    let mut f = Fixture::ready(50, None, ChildOrder::First);
    for kind in [16, 26, 53, 58, 94] {
        let id = f
            .entities
            .iter_all()
            .find(|entity| entity.entity_type == kind)
            .unwrap()
            .id;
        let owner = crate::intro2_common_dying::publish_intro2_common_standard_death(
            &mut f.entities,
            id,
            &mut f.fx,
        )
        .unwrap()
        .expect("actual retained allocation's class12 publication");
        f.scheduler.register_intro2_common_dying(owner);
        assert!(
            f.scheduler.prepare_native_actor_mutation(&f.entities, id),
            "type={kind}"
        );
        let primary = actor(&f.entities, id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        assert!(matches!(
            actor(&f.entities, id)
                .unwrap()
                .actor_tasks
                .task_state(primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
        assert!(f.scheduler.park_native_contact_prefix(&f.entities, id));
        assert!(!f.scheduler.prepare_native_actor_mutation(&f.entities, id));
        f.scheduler.register_intro2_common_dying(owner);
        assert!(
            !f.scheduler.prepare_native_actor_mutation(&f.entities, id),
            "death readoption preserves the parked prefix"
        );
    }
}
