//! Controlled contact phases on actual native allocations. Positions, velocity
//! and the recent-hit stamp select the test stimulus; task graphs and damage
//! profiles always come from the retail constructors and callbacks.

use super::{
    tests::{ChildOrder, Fixture},
    *,
};
use crate::{
    entity_behavior::BehaviorDescriptorIdentity,
    intro2_common_dying::publish_intro2_common_standard_death,
};

fn select_root(f: &mut Fixture, class_id: u8) {
    // 16490's real recent-hit evaluator makes RunAway available. Follow's
    // Always entry remains available with either stamp; no context is forged.
    f.entities
        .entity_mut(f.spider)
        .unwrap()
        .collision
        .last_hit_presentation_tick_at_0x34 =
        RetailRuntimeValue::Known(if class_id == 10 { f.tick } else { 0 });
    for _ in 0..256 {
        super::super::behavior::reselect(
            &mut f.entities,
            f.spider,
            f.tick,
            &mut f.fx,
            super::super::behavior::ReselectionEntry::TaskResult,
        )
        .unwrap();
        let entity = actor(&f.entities, f.spider).unwrap();
        if matches!(entity.current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
                if matches!(context.descriptor(), BehaviorDescriptorIdentity::Named(program)
                    if program.class_id == class_id))
        {
            f.scheduler
                .register_intro2_type17(Intro2Type17Owner::adopt(&f.entities, f.spider).unwrap());
            return;
        }
    }
    panic!("actual weighted selector did not reach class{class_id}");
}

fn replace_counterpart(f: &mut Fixture, kind: u32) {
    let old_child = f.child;
    let counterpart = f
        .entities
        .iter_all()
        .find(|entity| entity.entity_type == kind)
        .expect("authored counterpart in the selected canonical world")
        .id;
    let y = actor(&f.entities, old_child).unwrap().position_raw()[1];
    f.entities
        .entity_mut(old_child)
        .unwrap()
        .set_position_raw([0, y, 0]);
    f.child = counterpart;
    let entity = f.entities.entity_mut(counterpart).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x8000, 0x8000);
    entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    let order = f.entities.retail_live_order_ids().collect::<Vec<_>>();
    (f.subject, f.candidate) = if order.iter().position(|id| *id == f.spider)
        < order.iter().position(|id| *id == counterpart)
    {
        (f.spider, counterpart)
    } else {
        (counterpart, f.spider)
    };
    f.scheduler.adopt_intro2_type47_guards(&f.entities);
    f.place_oriented_contact();
}

fn physical_plan(f: &Fixture) -> crate::active_pair::ActivePairPhysicalPlan {
    let subject = actor(&f.entities, f.subject).unwrap();
    let candidate = actor(&f.entities, f.candidate).unwrap();
    let subject_entry = active_pair_body_from_entity(subject, &f.session.cache);
    let contact = classify_oriented_active_pair_contact(
        crate::player_active_contact::OrientedActivePairContactRequest {
            subject,
            candidate,
            subject_entry: &subject_entry,
            retail_tick: f.tick,
        },
        &f.session.cache,
    )
    .unwrap()
    .expect("actual oriented model contact");
    plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(subject, &f.session.cache),
        active_pair_body_from_entity(candidate, &f.session.cache),
        contact,
    )
    .unwrap()
}

fn set_zero_motion(f: &mut Fixture) {
    for id in [f.spider, f.child] {
        let entity = f.entities.entity_mut(id).unwrap();
        entity.set_motion_raw(entity.position_raw(), [0; 3]);
    }
}

fn set_lethal_motion(f: &mut Fixture) -> crate::active_pair::ActivePairPhysicalPlan {
    let target = actor(&f.entities, f.child).unwrap();
    let RetailRuntimeValue::Known(profile) = target.collision.damage_profile else {
        panic!()
    };
    let RetailRuntimeValue::Known(health) = target.collision.health_raw else {
        panic!()
    };
    assert_eq!(
        target.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(0)
    );
    // Find a bounded, nonoverflowing velocity stimulus through the real 14D30
    // ordered cap. This never writes health or replaces the authored filter.
    for speed in [2_000_i16, 4_000, 8_000, 12_000, 16_000] {
        for axis in 0..3 {
            for (id, sign) in [(f.spider, 1_i16), (f.child, -1_i16)] {
                let entity = f.entities.entity_mut(id).unwrap();
                let mut velocity = [0; 3];
                velocity[axis] = speed * sign;
                entity.set_motion_raw(entity.position_raw(), velocity);
            }
            let plan = physical_plan(f);
            if crate::damage::DamagePacket::collision(plan.capped_pair_damage_raw)
                .filtered_raw(Some(&profile))
                >= health
            {
                return plan;
            }
        }
    }
    panic!("canonical pair never reached the counterpart's lethal cap");
}

fn assert_physical_visit(f: &Fixture, result: &Type17PairOutcome, cap: i32) {
    let Type17PairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    assert_eq!(visits.len(), 1, "isolated native pair: {result:?}");
    let visit = &visits[0];
    assert_eq!(visit.candidate_id, f.candidate);
    assert!(!visit.physical_suppressed);
    let behavior: Vec<_> = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            Type17PairStage::Behavior {
                owner,
                result: Type17PairBehaviorResult::Null,
                ..
            } => Some(*owner),
            _ => None,
        })
        .collect();
    assert_eq!(behavior, [f.subject, f.candidate]);
    assert_eq!(
        visit.stages.last(),
        Some(&Type17PairStage::Physical {
            capped_damage_raw: cap
        })
    );
    let component_owners: Vec<_> = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            Type17PairStage::Component {
                owner,
                slot: ActorTaskSlot::Primary,
                ..
            } => Some(*owner),
            _ => None,
        })
        .collect();
    assert_eq!(
        component_owners,
        [f.subject, f.candidate],
        "both A900 chains precede physical"
    );
}

#[v2k_test_support::retail_test]
fn native_follow_and_runaway_person_pairs_are_physical_in_both_intrusive_orders() {
    for class_id in [33, 10] {
        for order in [ChildOrder::BeforeSpider, ChildOrder::AfterSpider] {
            let mut f = Fixture::ready(50, Some(4), order);
            select_root(&mut f, class_id);
            set_zero_motion(&mut f);
            let expected = physical_plan(&f);
            assert_eq!(expected.capped_pair_damage_raw, 0);
            let before_health = [f.subject, f.candidate]
                .map(|id| actor(&f.entities, id).unwrap().collision.health_raw);
            let before_basis = [f.subject, f.candidate]
                .map(|id| actor(&f.entities, id).unwrap().physical_body_basis_q31());
            let before_primary = actor(&f.entities, f.spider)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary);
            let result = f.pair(CaptureFeedbackPolicy::Cinematic);
            assert_physical_visit(&f, &result, 0);
            for (index, body) in [&expected.subject, &expected.candidate]
                .into_iter()
                .enumerate()
            {
                let entity = actor(&f.entities, body.id).unwrap();
                assert_eq!(
                    (entity.position_raw(), entity.velocity_raw()),
                    (body.position_raw, body.velocity_raw)
                );
                assert_eq!(entity.collision.health_raw, before_health[index]);
                assert_eq!(
                    entity.physical_body_basis_q31(),
                    before_basis[index],
                    "02DA0 does not rebuild the incoming matrix"
                );
                assert_eq!(entity.attached_to, None);
            }
            assert_eq!(
                actor(&f.entities, f.spider)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                before_primary
            );
            assert!(f
                .scheduler
                .prepare_native_actor_mutation(&f.entities, f.spider));
        }
    }
}

#[v2k_test_support::retail_test]
fn native_follow_and_runaway_physical_damage_publish_real_person_class14() {
    for (class_id, order) in [
        (33, ChildOrder::BeforeSpider),
        (10, ChildOrder::AfterSpider),
    ] {
        let mut f = Fixture::ready(50, Some(4), order);
        select_root(&mut f, class_id);
        let plan = set_lethal_motion(&mut f);
        let stamp = actor(&f.entities, f.child)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34;
        let result = f.pair(CaptureFeedbackPolicy::Cinematic);
        assert_physical_visit(&f, &result, plan.capped_pair_damage_raw);
        let child = actor(&f.entities, f.child).unwrap();
        assert_eq!(child.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            child.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        assert!(matches!(child.current_behavior_context,
            RetailRuntimeValue::Known(Some(context))
                if matches!(context.descriptor(), BehaviorDescriptorIdentity::Named(program) if program.class_id == 14)));
        assert_eq!(
            child.collision.last_hit_presentation_tick_at_0x34, stamp,
            "15040 is not a primary projectile hit"
        );
        assert_eq!(child.attached_to, None);
    }
}

#[v2k_test_support::retail_test]
fn native_class12_spider_still_runs_person_pair_response() {
    let mut f = Fixture::ready(14, None, ChildOrder::First);
    select_root(&mut f, 33);
    let owner = publish_intro2_common_standard_death(&mut f.entities, f.spider, &mut f.fx)
        .unwrap()
        .expect("real native class12 constructor");
    f.scheduler.register_intro2_common_dying(owner);
    set_zero_motion(&mut f);
    let plan = physical_plan(&f);
    let result = f.pair(CaptureFeedbackPolicy::Gameplay);
    assert_physical_visit(&f, &result, 0);
    let spider = actor(&f.entities, f.spider).unwrap();
    assert!(matches!(
        spider.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::CommonDying(_))
    ));
    assert_eq!(
        spider.position_raw(),
        if plan.subject.id == f.spider {
            plan.subject.position_raw
        } else {
            plan.candidate.position_raw
        }
    );
    assert!(f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, f.spider));
}

#[v2k_test_support::retail_test]
fn native_gunner_pair_preserves_zero_damage_and_publishes_lethal_class12() {
    for lethal in [false, true] {
        let mut f = Fixture::ready(14, None, ChildOrder::First);
        select_root(&mut f, 33);
        replace_counterpart(&mut f, 47);
        let before_health = actor(&f.entities, f.child).unwrap().collision.health_raw;
        let plan = if lethal {
            set_lethal_motion(&mut f)
        } else {
            set_zero_motion(&mut f);
            physical_plan(&f)
        };
        let result = f.pair(CaptureFeedbackPolicy::Gameplay);
        assert_physical_visit(&f, &result, plan.capped_pair_damage_raw);
        let gunner = actor(&f.entities, f.child).unwrap();
        if lethal {
            assert_eq!(gunner.collision.health_raw, RetailRuntimeValue::Known(0));
            assert!(matches!(
                gunner.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::CommonDying(_))
            ));
        } else {
            assert_eq!(gunner.collision.health_raw, before_health);
        }
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, f.child));
    }
}

#[v2k_test_support::retail_test]
fn powerup_rejects_spider_with_a300_and_suppresses_only_physical_suffix() {
    let mut f = Fixture::ready(13, None, ChildOrder::First);
    select_root(&mut f, 33);
    replace_counterpart(&mut f, 61);
    set_zero_motion(&mut f);
    let before = [f.subject, f.candidate].map(|id| {
        let entity = actor(&f.entities, id).unwrap();
        (
            entity.position_raw(),
            entity.velocity_raw(),
            entity.collision.health_raw,
        )
    });
    let result = f.pair(CaptureFeedbackPolicy::Gameplay);
    let Type17PairOutcome::Resolved { visits } = &result else {
        panic!("{result:?}")
    };
    assert_eq!(visits.len(), 1);
    let visit = &visits[0];
    assert!(visit.physical_suppressed);
    assert!(visit
        .stages
        .contains(&Type17PairStage::DispatchPowerUpUnsupportedRecipient));
    assert!(!visit
        .stages
        .iter()
        .any(|stage| matches!(stage, Type17PairStage::Physical { .. })));
    let behaviors: Vec<_> = visit
        .stages
        .iter()
        .filter_map(|stage| match stage {
            Type17PairStage::Behavior { owner, .. } => Some(*owner),
            _ => None,
        })
        .collect();
    assert_eq!(behaviors, [f.subject, f.candidate]);
    assert!(visit.stages.iter().any(|stage| matches!(stage,
        Type17PairStage::Component { owner, slot: ActorTaskSlot::Primary, .. } if *owner == f.spider)));
    for (index, id) in [f.subject, f.candidate].into_iter().enumerate() {
        let entity = actor(&f.entities, id).unwrap();
        assert_eq!(
            (
                entity.position_raw(),
                entity.velocity_raw(),
                entity.collision.health_raw
            ),
            before[index]
        );
    }
    assert_eq!(actor(&f.entities, f.child).unwrap().attached_to, None);
    assert!(!f
        .entities
        .pending_actor_deferred_destroy_ids()
        .contains(&f.child));
}

#[v2k_test_support::retail_test]
fn parked_native_gunner_class12_rejects_zero_damage_contact_before_mutation() {
    let mut f = Fixture::ready(14, None, ChildOrder::First);
    select_root(&mut f, 33);
    replace_counterpart(&mut f, 47);
    let gunner = f.child;
    let owner = publish_intro2_common_standard_death(&mut f.entities, gunner, &mut f.fx)
        .unwrap()
        .expect("actual native gunner class12 publication");
    f.scheduler.register_intro2_common_dying(owner);
    assert!(f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, gunner));
    f.place_oriented_contact();
    set_zero_motion(&mut f);
    assert_eq!(physical_plan(&f).capped_pair_damage_raw, 0);
    f.scheduler.park_native_type47_external_prefix(gunner);
    assert!(!f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, gunner));
    let before = f
        .entities
        .iter_all()
        .map(|entity| {
            (
                entity.id,
                entity.position_raw(),
                entity.velocity_raw(),
                entity.collision.health_raw,
                entity.rotation_heading_pitch_roll_raw(),
            )
        })
        .collect::<Vec<_>>();
    let mut expected_fx = f.fx.fork_for_main_base_abort_transaction();
    let result = f.pair(CaptureFeedbackPolicy::Gameplay);
    assert!(
        matches!(result,
        Type17PairOutcome::Blocked {
            reason: Type17PairBlock::BodyCustody { entity_id },
            committed_prefix: false,
            ref visits,
        } if entity_id == gunner && visits.is_empty()),
        "{result:?}"
    );
    let after = f
        .entities
        .iter_all()
        .map(|entity| {
            (
                entity.id,
                entity.position_raw(),
                entity.velocity_raw(),
                entity.collision.health_raw,
                entity.rotation_heading_pitch_roll_raw(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        after, before,
        "zero damage does not waive parked body custody"
    );
    assert_eq!(
        f.fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
    assert!(!f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, gunner));
}
