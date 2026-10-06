//! Actual ordinary allocations and geometry at the production late-contact seam.
use super::*;
use crate::{
    active_pair::{plan_active_pair_response_and_damage_cap, ActivePairBody, ActivePairContact},
    intro2_contacts::Intro2ContactFrame,
    intro2_radial::Intro2RadialTaskCustody,
    intro2_type17::pair::tests::{ChildOrder, Fixture},
    native_actor_capture::pair::{
        actor, resolve_native_actor_active_contacts_at_entry, CaptureFeedbackPolicy,
        NativeActorPairRequest, NativeCaptorPairBehaviorResult, NativeCaptorPairBlock,
        NativeCaptorPairOutcome, NativeCaptorPairStage,
    },
    player_active_contact::{
        active_pair_body_from_entity, classify_oriented_active_pair_contact,
        OrientedActivePairContactRequest,
    },
};

fn entry(f: &Fixture, id: u32) -> ActivePairBody {
    active_pair_body_from_entity(actor(&f.entities, id).unwrap(), &f.session.cache)
}

fn contact(
    f: &Fixture,
    subject: u32,
    candidate: u32,
    admission: &ActivePairBody,
) -> Option<ActivePairContact> {
    classify_oriented_active_pair_contact(
        OrientedActivePairContactRequest {
            subject: actor(&f.entities, subject).unwrap(),
            candidate: actor(&f.entities, candidate).unwrap(),
            subject_entry: admission,
            retail_tick: f.tick,
        },
        &f.session.cache,
    )
    .unwrap()
}

fn place_contact(
    f: &mut Fixture,
    subject: u32,
    candidate: u32,
    admission: &ActivePairBody,
) -> ActivePairContact {
    let center = actor(&f.entities, subject).unwrap().position_raw();
    for dy in [-150_i16, -75, 0, 75, 150] {
        for dx in [80_i16, 160, 240, -80, -160, -240, 0] {
            for dz in [80_i16, 160, -80, -160, 0] {
                f.entities.entity_mut(candidate).unwrap().set_position_raw([
                    center[0].wrapping_add(dx),
                    center[1].wrapping_add(dy),
                    center[2].wrapping_add(dz),
                ]);
                if let Some(contact) = contact(f, subject, candidate, admission) {
                    return contact;
                }
            }
        }
    }
    panic!("actual oriented native bodies did not contact");
}

fn frame(f: &mut Fixture) -> Intro2ContactFrame<'_> {
    Intro2ContactFrame {
        entities: &mut f.entities,
        resources: &mut f.session.cache,
        world_fx: &mut f.fx,
        static_damage: &mut f.static_damage,
        notifications: &mut f.notifications,
        retail_tick: f.tick,
        actor_tasks: &mut f.scheduler,
    }
}

fn late(f: &mut Fixture, id: u32) -> OrdinaryType9LateContactOutcome {
    let metadata = f.entities.type_runtime_metadata(9).unwrap().clone();
    resolve_ordinary_type9_late_contact(&mut frame(f), id, &metadata, None)
}

fn static_effects_in_own_allocation(
    outcome: &OrdinaryType9StaticContactOutcome,
    manager: &EntityManager,
) -> (OrdinaryType9StaticContactOutcome, bool) {
    let mut effects = outcome.clone();
    let mut published = false;
    if let OrdinaryType9StaticContactOutcome::Applied(applied) = &mut effects {
        if let Some(lease) = applied
            .actor_damage
            .as_mut()
            .and_then(|damage| damage.death_publication.take())
        {
            published = true;
            assert_eq!(
                manager
                    .main_base_abort_actor_observation(lease.actor().entity_id)
                    .unwrap()
                    .lease,
                lease.actor(),
                "the death receipt belongs to this actual manager"
            );
            assert_eq!(
                manager
                    .iter_all()
                    .find(|e| e.id == lease.actor().entity_id)
                    .unwrap()
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary),
                Some(lease.task_id())
            );
        }
    }
    (effects, published)
}

fn peers() -> Fixture {
    peers_in_world(13)
}

fn peers_in_world(world: u32) -> Fixture {
    let mut f = Fixture::ready(world, None, ChildOrder::First);
    let ids: Vec<_> = f
        .entities
        .retail_live_order_ids()
        .filter(|&id| actor(&f.entities, id).unwrap().entity_type == 9)
        .collect();
    (f.subject, f.candidate) = (ids[0], ids[1]);
    let y = actor(&f.entities, f.spider).unwrap().position_raw()[1];
    f.entities
        .entity_mut(f.spider)
        .unwrap()
        .set_position_raw([0, y, 0]);
    for id in [f.subject, f.candidate] {
        let e = f.entities.entity_mut(id).unwrap();
        e.set_motion_raw([32720, y, 16000], [0; 3]);
        e.collision.state_flags_at_0x08.overwrite(0x8000, 0x8000);
        e.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
    let admission = entry(&f, f.subject);
    let (subject, candidate) = (f.subject, f.candidate);
    place_contact(&mut f, subject, candidate, &admission);
    f
}

#[v2k_test_support::retail_test]
fn native_peers_run_both_behavior_and_a900_orders_then_separate_once() {
    for (world, reverse_motion) in [(13, false), (13, true), (50, false), (50, true)] {
        let mut f = peers_in_world(world);
        let (subject, candidate) = (f.subject, f.candidate);
        if reverse_motion {
            let positions =
                [subject, candidate].map(|id| actor(&f.entities, id).unwrap().position_raw());
            f.entities
                .entity_mut(subject)
                .unwrap()
                .set_position_raw(positions[1]);
            f.entities
                .entity_mut(candidate)
                .unwrap()
                .set_position_raw(positions[0]);
        }
        let admission = entry(&f, subject);
        let contact = contact(&f, subject, candidate, &admission).expect("native peer geometry");
        let expected = plan_active_pair_response_and_damage_cap(
            entry(&f, subject),
            entry(&f, candidate),
            contact,
        )
        .unwrap();
        let before = [subject, candidate].map(|id| {
            let e = actor(&f.entities, id).unwrap();
            (
                e.physical_body_basis_q31(),
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| e.actor_task_state(slot).copied()),
            )
        });
        let fx = format!("{:?}", f.fx);
        let result = late(&mut f, subject);
        assert_eq!(
            result.static_contact,
            if world == 50 {
                OrdinaryType9StaticContactOutcome::Ineligible
            } else {
                OrdinaryType9StaticContactOutcome::Miss
            }
        );
        let OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { visits }) =
            result.pair_suffix
        else {
            panic!("{result:?}")
        };
        assert_eq!(visits.len(), 1);
        let visit = &visits[0];
        assert_eq!(visit.candidate_id, candidate);
        assert!(!visit.physical_suppressed);
        assert!(
            matches!(visit.stages[0], NativeCaptorPairStage::Behavior { owner, result: NativeCaptorPairBehaviorResult::Null, .. } if owner == subject)
        );
        assert!(
            matches!(visit.stages[1], NativeCaptorPairStage::Behavior { owner, result: NativeCaptorPairBehaviorResult::Null, .. } if owner == candidate)
        );
        for (index, (owner, slot)) in [subject, candidate]
            .into_iter()
            .flat_map(|id| ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| (id, slot)))
            .enumerate()
        {
            assert!(
                matches!(visit.stages[index + 2], NativeCaptorPairStage::Component { owner: actual, slot: actual_slot, .. } if actual == owner && actual_slot == slot)
            );
        }
        assert_eq!(
            visit.stages.last(),
            Some(&NativeCaptorPairStage::Physical {
                capped_damage_raw: expected.capped_pair_damage_raw
            })
        );
        assert_eq!(
            format!("{:?}", f.fx),
            fx,
            "Type9 null pair hooks and Sub-I02DA0 consume no RNG or cue"
        );
        for (index, body) in [&expected.subject, &expected.candidate]
            .into_iter()
            .enumerate()
        {
            let e = actor(&f.entities, body.id).unwrap();
            assert_eq!(e.position_raw(), body.position_raw);
            assert_eq!(e.velocity_raw(), body.velocity_raw);
            assert_eq!(e.physical_body_basis_q31(), before[index].0);
            assert_eq!(
                ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| e.actor_task_state(slot).copied()),
                before[index].1,
                "Sub-I descriptor contact preserves private targets and clocks"
            );
            assert!(f
                .scheduler
                .prepare_native_actor_mutation(&f.entities, body.id));
        }
        let result = late(&mut f, candidate);
        assert!(
            matches!(&result.pair_suffix, OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { visits }) if visits.is_empty()),
            "the later actor does not replay its predecessor pair"
        );
    }
}

#[v2k_test_support::retail_test]
fn real_peer_physical_packets_publish_both_class14_owners_and_next_visit_runs_them() {
    let mut f = peers();
    let (subject, candidate) = (f.subject, f.candidate);
    let admission = entry(&f, subject);
    let contact = contact(&f, subject, candidate, &admission).unwrap();
    let RetailRuntimeValue::Known(profile) = actor(&f.entities, subject)
        .unwrap()
        .collision
        .damage_profile
    else {
        panic!()
    };
    let RetailRuntimeValue::Known(health) =
        actor(&f.entities, subject).unwrap().collision.health_raw
    else {
        panic!()
    };
    let mut expected = None;
    'stimulus: for speed in [2000_i16, 4000, 8000, 12000, 16000] {
        for axis in 0..3 {
            for (id, sign) in [(subject, 1_i16), (candidate, -1_i16)] {
                let mut velocity = [0; 3];
                velocity[axis] = speed * sign;
                f.entities
                    .entity_mut(id)
                    .unwrap()
                    .set_velocity_raw(velocity);
            }
            let plan = plan_active_pair_response_and_damage_cap(
                entry(&f, subject),
                entry(&f, candidate),
                contact,
            )
            .unwrap();
            if DamagePacket::collision(plan.capped_pair_damage_raw).filtered_raw(Some(&profile))
                >= health
            {
                expected = Some(plan);
                break 'stimulus;
            }
        }
    }
    let expected = expected.expect("actual14D30 cap reaches native health without filter edits");
    let stamps = [subject, candidate].map(|id| {
        actor(&f.entities, id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34
    });
    let result = late(&mut f, subject);
    let OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { visits }) =
        result.pair_suffix
    else {
        panic!("{result:?}")
    };
    assert_eq!(visits.len(), 1);
    assert_eq!(
        visits[0].stages.last(),
        Some(&NativeCaptorPairStage::Physical {
            capped_damage_raw: expected.capped_pair_damage_raw
        })
    );
    for (index, id) in [subject, candidate].into_iter().enumerate() {
        let e = actor(&f.entities, id).unwrap();
        assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            e.collision
                .state_flags_at_0x08
                .masked(crate::entity_collision_state::DYING_STATE_BIT),
            RetailRuntimeValue::Known(crate::entity_collision_state::DYING_STATE_BIT)
        );
        assert_eq!(
            e.collision.last_hit_presentation_tick_at_0x34, stamps[index],
            "15040 preserves the primary hit stamp"
        );
        assert!(
            matches!(e.current_behavior_context, RetailRuntimeValue::Known(Some(context)) if context.active_style().style_address() == 0x004c_70c0)
        );
        assert!(
            matches!(e.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.lifetime_ms() == 1000)
        );
        assert!(f.scheduler.prepare_native_actor_mutation(&f.entities, id));
    }
    let fx = format!("{:?}", f.fx);
    let result = late(&mut f, candidate);
    assert!(
        matches!(result.pair_suffix, OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { visits }) if visits.is_empty()),
        "fresh Class14 entry is ineligible"
    );
    assert_eq!(format!("{:?}", f.fx), fx);
    let pass = f.scheduler.tick(
        &mut f.entities,
        crate::specialized_actor_task_production::SpecializedActorTaskProductionFrame {
            world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut f.static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: 20_000,
            retail_tick: 341,
            notification_phase: crate::gameplay_notifications::GameplayNotificationPhase::Playing,
            main_base_abort_active: false,
        },
        &mut f.notifications,
    );
    assert!(pass.block.is_none(), "{pass:?}");
    assert!(!format!("{pass:?}").contains("Dropped"), "{pass:?}");
    for id in [subject, candidate] {
        assert!(
            f.scheduler.prepare_native_actor_mutation(&f.entities, id),
            "next Class14 visit retains its native allocation"
        );
        assert!(
            matches!(actor(&f.entities, id).unwrap().actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::SharedRetarget(task)) if task.elapsed_ms() == 20)
        );
    }
}

#[v2k_test_support::retail_test]
fn carried_peers_are_excluded_in_either_seat_before_callbacks_or_rng() {
    for carry_subject in [false, true] {
        let mut f = peers();
        let (subject, candidate) = (f.subject, f.candidate);
        let carried = if carry_subject { subject } else { candidate };
        let allocation = f
            .scheduler
            .prepare_type9_cargo_attach(&f.entities, carried)
            .unwrap();
        let parent = f.entities.player().unwrap();
        let relation = crate::ordinary_type9_cargo::Type9RelationOwner {
            id: parent.id,
            capability_flags: parent.capability_flags,
            position_raw: parent.position_raw(),
        };
        let metadata = f.entities.type_runtime_metadata(9).unwrap().clone();
        let plan = crate::ordinary_type9_cargo::plan_attach(
            actor(&f.entities, carried).unwrap(),
            &metadata,
            relation,
        )
        .unwrap();
        assert!(f.scheduler.take_type9_for_cargo(&f.entities, allocation));
        let RetailRuntimeValue::Known(Some(rows)) =
            &mut f.entities.player_mut().unwrap().sub_j_attachment_runtime
        else {
            panic!()
        };
        rows.append(carried).unwrap();
        let e = f.entities.entity_mut(carried).unwrap();
        e.attached_to = Some(relation.id);
        e.collision.state_flags_at_0x08.overwrite(0x1000, 0x1000);
        let owner = crate::ordinary_type9_cargo::commit_attach(e, allocation, plan).unwrap();
        f.scheduler.register_type9_carried(owner).unwrap();
        assert!(f
            .scheduler
            .prepare_native_actor_mutation(&f.entities, carried));
        let snapshot = [subject, candidate].map(|id| {
            (
                entry(&f, id),
                format!("{:?}", actor(&f.entities, id).unwrap().actor_tasks),
            )
        });
        let fx = format!("{:?}", f.fx);
        let result = late(&mut f, subject);
        assert!(
            matches!(&result.pair_suffix, OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { visits }) if visits.is_empty()),
            "{result:?}"
        );
        assert_eq!(
            [subject, candidate].map(|id| (
                entry(&f, id),
                format!("{:?}", actor(&f.entities, id).unwrap().actor_tasks)
            )),
            snapshot
        );
        assert_eq!(format!("{:?}", f.fx), fx);
    }
}

#[v2k_test_support::retail_test]
fn changed_or_parked_peer_is_rejected_before_pair_rng_or_motion() {
    for foreign in [false, true] {
        let mut f = peers();
        let (subject, candidate) = (f.subject, f.candidate);
        if foreign {
            let other = peers();
            f.entities
                .entity_mut(candidate)
                .unwrap()
                .ordinary_type9_native_receipt = actor(&other.entities, candidate)
                .unwrap()
                .ordinary_type9_native_receipt;
        } else {
            assert!(f
                .scheduler
                .park_native_contact_prefix(&f.entities, candidate));
        }
        let before = [subject, candidate].map(|id| {
            let e = actor(&f.entities, id).unwrap();
            (
                e.position_raw(),
                e.velocity_raw(),
                format!("{:?}", e.actor_tasks),
            )
        });
        let fx = format!("{:?}", f.fx);
        let result = late(&mut f, subject);
        assert!(
            matches!(result.pair_suffix, OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Blocked { reason: NativeCaptorPairBlock::BodyCustody { entity_id }, committed_prefix: false, .. }) if entity_id == candidate),
            "{result:?}"
        );
        assert_eq!(format!("{:?}", f.fx), fx);
        assert_eq!(
            [subject, candidate].map(|id| {
                let e = actor(&f.entities, id).unwrap();
                (
                    e.position_raw(),
                    e.velocity_raw(),
                    format!("{:?}", e.actor_tasks),
                )
            }),
            before
        );
    }
}

#[v2k_test_support::retail_test]
fn production_static_prefix_precedes_capture_and_lethal_static_retains_entry_admission() {
    for lethal in [false, true] {
        let mut actual = Fixture::ready(13, None, ChildOrder::BeforeSpider);
        let mut expected = Fixture::ready(13, None, ChildOrder::BeforeSpider);
        let (subject, candidate) = (actual.child, actual.spider);
        assert_eq!(actual.subject, subject);
        for f in [&mut actual, &mut expected] {
            let e = f.entities.entity_mut(subject).unwrap();
            e.set_motion_raw(
                [-24961, 3, -26881],
                if lethal {
                    [-30_000, 0, 0]
                } else {
                    [-256, 0, 0]
                },
            );
            if lethal {
                e.collision.health_raw = RetailRuntimeValue::Known(1);
            }
        }
        let admission = entry(&expected, subject);
        let metadata = expected.entities.type_runtime_metadata(9).unwrap().clone();
        let static_contact = {
            let frame = frame(&mut expected);
            resolve_ordinary_type9_static_contact(
                frame.entities,
                subject,
                &metadata,
                OrdinaryType9StaticContactFrame {
                    resources: frame.resources,
                    static_damage: frame.static_damage,
                    world_fx: frame.world_fx,
                    scheduler: frame.actor_tasks,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                },
            )
        };
        assert!(
            matches!(
                static_contact,
                OrdinaryType9StaticContactOutcome::Applied(_)
            ),
            "{static_contact:?}"
        );
        // Separation can quantize to the same plane at some placements.
        // Select an actual oriented probe which distinguishes the
        // orders, rather than claiming every broad overlap does so.
        let center = actor(&expected.entities, subject).unwrap().position_raw();
        let before_admission = entry(&actual, subject);
        let mut distinguished = false;
        'placements: for dy in [-150_i16, -75, 0, 75, 150] {
            for dx in -240_i16..=240 {
                for dz in [-160_i16, -80, 0, 80, 160] {
                    let position = [
                        center[0].wrapping_add(dx),
                        center[1].wrapping_add(dy),
                        center[2].wrapping_add(dz),
                    ];
                    for f in [&mut actual, &mut expected] {
                        f.entities
                            .entity_mut(candidate)
                            .unwrap()
                            .set_position_raw(position);
                    }
                    if let Some(after) = contact(&expected, subject, candidate, &admission) {
                        if contact(&actual, subject, candidate, &before_admission) != Some(after) {
                            distinguished = true;
                            break 'placements;
                        }
                    }
                }
            }
        }
        assert!(
            distinguished,
            "actual fence response must change a real oriented pair probe: {static_contact:?}"
        );
        let pairs = resolve_native_actor_active_contacts_at_entry(
            &mut frame(&mut expected),
            NativeActorPairRequest {
                id: subject,
                feedback: CaptureFeedbackPolicy::Gameplay,
                playing_player: None,
                subject_entry: Some(&admission),
            },
        );
        let result = late(&mut actual, subject);
        assert_eq!(
            static_effects_in_own_allocation(&result.static_contact, &actual.entities),
            static_effects_in_own_allocation(&static_contact, &expected.entities)
        );
        assert_eq!(result.pair_suffix, OrdinaryType9PairSuffix::Visited(pairs));
        for id in [subject, candidate] {
            let a = actor(&actual.entities, id).unwrap();
            let b = actor(&expected.entities, id).unwrap();
            assert_eq!(
                (
                    a.position_raw(),
                    a.velocity_raw(),
                    a.current_behavior_context
                ),
                (
                    b.position_raw(),
                    b.velocity_raw(),
                    b.current_behavior_context
                )
            );
            assert_eq!(
                format!("{:?}", a.actor_tasks),
                format!("{:?}", b.actor_tasks)
            );
            assert!(actual
                .scheduler
                .prepare_native_actor_mutation(&actual.entities, id));
        }
        assert_eq!(format!("{:?}", actual.fx), format!("{:?}", expected.fx));
        if lethal {
            let e = actor(&actual.entities, subject).unwrap();
            assert_eq!(e.collision.health_raw, RetailRuntimeValue::Known(0));
            assert_eq!(
                e.collision.state_flags_at_0x08.masked(0x8000),
                RetailRuntimeValue::Known(0)
            );
            let OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { visits }) =
                result.pair_suffix
            else {
                panic!()
            };
            assert_eq!(
                visits.len(),
                1,
                "cached entry still admits the same-pass corpse"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn static_block_stops_only_this_subject_without_replaying_committed_prefix() {
    let mut f = peers();
    let (subject, candidate) = (f.subject, f.candidate);
    f.entities
        .entity_mut(subject)
        .unwrap()
        .set_motion_raw([-24961, 3, -26881], [-30_000, 0, 0]);
    f.entities.entity_mut(subject).unwrap().collision.health_raw = RetailRuntimeValue::Unresolved;
    let candidate_before = (
        actor(&f.entities, candidate).unwrap().position_raw(),
        format!("{:?}", actor(&f.entities, candidate).unwrap().actor_tasks),
    );
    let result = late(&mut f, subject);
    assert!(
        matches!(
            result.static_contact,
            OrdinaryType9StaticContactOutcome::Blocked {
                committed_prefix: true,
                ..
            }
        ),
        "{result:?}"
    );
    assert_eq!(result.pair_suffix, OrdinaryType9PairSuffix::StaticBlocked);
    assert_eq!(
        (
            actor(&f.entities, candidate).unwrap().position_raw(),
            format!("{:?}", actor(&f.entities, candidate).unwrap().actor_tasks)
        ),
        candidate_before
    );
    let snapshot = (
        entry(&f, subject),
        format!("{:?}", f.fx),
        format!("{:?}", actor(&f.entities, subject).unwrap().actor_tasks),
    );
    let _ = late(&mut f, subject);
    assert_eq!(
        (
            entry(&f, subject),
            format!("{:?}", f.fx),
            format!("{:?}", actor(&f.entities, subject).unwrap().actor_tasks)
        ),
        snapshot
    );
    assert!(
        matches!(
            late(&mut f, candidate).pair_suffix,
            OrdinaryType9PairSuffix::Visited(NativeCaptorPairOutcome::Resolved { .. })
        ),
        "next actor still progresses"
    );
}
