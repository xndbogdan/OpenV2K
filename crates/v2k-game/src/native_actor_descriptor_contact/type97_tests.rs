//! World46's actual Type122/Type97 bodies through the late oriented pair walk.

use super::*;
use crate::{
    actor_task_owner::ActorTaskVisit,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_gun_turret::{
        authored_tests::fixture, tick_intro2_gun_turret, Intro2GunTurretFrame,
        Intro2GunTurretOutcome, Intro2GunTurretOwner,
    },
    intro2_type17::pair::tests::Fixture,
    native_actor_capture::pair::{
        resolve_native_captor_active_contacts_with_playing, CaptureFeedbackPolicy,
        NativeCaptorPairBehaviorResult, NativeCaptorPairBlock, NativeCaptorPairOutcome,
        NativeCaptorPairStage,
    },
    native_type122::Type122Owner,
    player_active_contact::{
        active_pair_body_from_entity, classify_oriented_active_pair_contact,
        OrientedActivePairContactRequest,
    },
    player_hull::PlayerHull,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    static_damage::StaticDamageScheduler,
};

fn ready() -> Fixture {
    let (mut session, mut entities, mut fx) = fixture(46);
    // Exercise the late pair pass after the load's gate-helper cleanup.
    entities.cleanup_pending_actor_deferred_destroys();
    let spider = entities
        .iter_all()
        .find(|e| e.entity_type == 122)
        .unwrap()
        .id;
    // The reported counterpart is authored spawn18, independent of earlier
    // player/helper allocations and their resulting runtime handles.
    let turret = entities
        .iter_all()
        .find(|e| e.entity_type == 97 && e.authored_spawn_index == Some(18))
        .unwrap()
        .id;
    let mut notifications = GameplayNotifications::new();
    let owner = Intro2GunTurretOwner::adopt(&entities, turret).unwrap();
    entities
        .entity_mut(turret)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(0x68000, 0x68000);
    let tick = tick_intro2_gun_turret(
        &mut entities,
        owner,
        Intro2GunTurretFrame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            notifications: &mut notifications,
            notification_phase: GameplayNotificationPhase::Playing,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(tick.outcome, Intro2GunTurretOutcome::Advanced { .. }),
        "{:?}",
        tick.outcome
    );
    let mut scheduler = SpecializedActorTaskScheduler::new();
    scheduler.register_intro2_gun_turret(tick.retained_owner.unwrap());
    scheduler.register_type122(Type122Owner::adopt(&entities, spider).unwrap());
    // Only motion and presentation admission are controlled. Both constructor
    // receipts, authored models, Class29 graph and Type122 task remain native.
    let center = [
        32720,
        session.cache.terrain().unwrap().sea_level_raw() + 6000,
        16000,
    ];
    for id in [spider, turret] {
        let entity = entities.entity_mut(id).unwrap();
        entity.set_motion_raw(center, [0; 3]);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000, 0x8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
    let order = entities.retail_live_order_ids().collect::<Vec<_>>();
    let (subject, candidate) =
        if order.iter().position(|&id| id == spider) < order.iter().position(|&id| id == turret) {
            (spider, turret)
        } else {
            (turret, spider)
        };
    let mut f = Fixture {
        session,
        entities,
        scheduler,
        fx,
        notifications,
        static_damage: StaticDamageScheduler::new(),
        spider,
        child: turret,
        subject,
        candidate,
        tick: 2,
    };
    f.place_oriented_contact();
    f
}

fn physical_plan(f: &Fixture) -> crate::active_pair::ActivePairPhysicalPlan {
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
    .expect("real model274/model173 oriented contact");
    crate::active_pair::plan_active_pair_response_and_damage_cap(
        active_pair_body_from_entity(subject, &f.session.cache),
        active_pair_body_from_entity(candidate, &f.session.cache),
        contact,
    )
    .unwrap()
}

fn snapshot(f: &Fixture) -> String {
    let actors = [f.spider, f.child].map(|id| {
        let e = actor(&f.entities, id).unwrap();
        (
            e.position_raw(),
            e.velocity_raw(),
            e.collision.clone(),
            format!("{:?}", e.actor_tasks),
            e.physical_body_basis_q31(),
        )
    });
    format!("{:?}", (actors, &f.fx, &f.notifications))
}

#[v2k_test_support::retail_test]
fn world46_type97_null_class29_contact_completes_the_native_type122_pair() {
    let mut f = ready();
    let plan = physical_plan(&f);
    assert_eq!(plan.capped_pair_damage_raw, 0);
    assert!(
        plan.subject.position_raw != actor(&f.entities, f.subject).unwrap().position_raw()
            || plan.candidate.position_raw
                != actor(&f.entities, f.candidate).unwrap().position_raw(),
        "the null callbacks must still reach a physical writer"
    );
    let turret = actor(&f.entities, f.child).unwrap();
    let task = turret.actor_task_state(ActorTaskSlot::Tertiary).copied();
    assert!(matches!(task, Some(ActorTaskRuntime::Intro2GunTurret(_))));
    let health =
        [f.subject, f.candidate].map(|id| actor(&f.entities, id).unwrap().collision.health_raw);
    let result = f.pair(CaptureFeedbackPolicy::Gameplay);
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("{result:?}")
    };
    assert_eq!(visits.len(), 1);
    let visit = &visits[0];
    assert!(!visit.physical_suppressed);
    assert!(visit.stages.contains(&NativeCaptorPairStage::Behavior {
        owner: f.child,
        style: 0x004c8230,
        result: NativeCaptorPairBehaviorResult::Null,
    }));
    assert!(visit.stages.contains(&NativeCaptorPairStage::Component {
        owner: f.child,
        slot: ActorTaskSlot::Tertiary,
        result: NativeDescriptorContactOutcome::Null,
    }));
    assert_eq!(
        visit.stages.last(),
        Some(&NativeCaptorPairStage::Physical {
            capped_damage_raw: 0
        })
    );
    for (index, body) in [&plan.subject, &plan.candidate].into_iter().enumerate() {
        let entity = actor(&f.entities, body.id).unwrap();
        assert_eq!(
            (entity.position_raw(), entity.velocity_raw()),
            (body.position_raw, body.velocity_raw)
        );
        assert_eq!(entity.collision.health_raw, health[index]);
    }
    assert_eq!(
        actor(&f.entities, f.child)
            .unwrap()
            .actor_task_state(ActorTaskSlot::Tertiary)
            .copied(),
        task
    );
    assert!(f
        .scheduler
        .prepare_native_actor_mutation(&f.entities, f.child));
}

#[v2k_test_support::retail_test]
fn world46_type97_pair_rejects_foreign_and_unfinished_custody_before_physical_writes() {
    for foreign in [true, false] {
        let mut f = ready();
        assert_eq!(physical_plan(&f).capped_pair_damage_raw, 0);
        if foreign {
            let (_, other, _) = fixture(46);
            let owner = Intro2GunTurretOwner::adopt(&other, f.child).unwrap();
            assert_ne!(
                f.entities
                    .main_base_abort_actor_observation(f.child)
                    .unwrap()
                    .lease,
                other
                    .main_base_abort_actor_observation(f.child)
                    .unwrap()
                    .lease,
            );
            f.scheduler.register_intro2_gun_turret(owner);
        } else {
            let tasks = &mut f.entities.entity_mut(f.child).unwrap().actor_tasks;
            let visit = ActorTaskVisit {
                slot: ActorTaskSlot::Tertiary,
                task_id: tasks.task_in_slot(ActorTaskSlot::Tertiary).unwrap(),
            };
            assert_eq!(tasks.begin_exact_visit_with(visit, |_| ()), Some(()));
        }
        let before = snapshot(&f);
        assert!(matches!(f.pair(CaptureFeedbackPolicy::Gameplay),
            NativeCaptorPairOutcome::Blocked {
                reason: NativeCaptorPairBlock::BodyCustody { entity_id },
                committed_prefix: false, ref visits,
            } if entity_id == f.child && visits.is_empty()));
        assert_eq!(snapshot(&f), before, "foreign={foreign}");
    }
}

fn set_turret_lethal_motion(f: &mut Fixture) -> crate::active_pair::ActivePairPhysicalPlan {
    // Precondition the turret with prior wear so a bounded pair impact can
    // finish it: retail turrets reach lethal pair delivery after surviving
    // earlier hits, not only from full health. This writes only health;
    // the damage profile, tasks and allocation remain authored.
    f.entities.entity_mut(f.child).unwrap().collision.health_raw = RetailRuntimeValue::Known(200);
    let target = actor(&f.entities, f.child).unwrap();
    let RetailRuntimeValue::Known(profile) = target.collision.damage_profile else {
        panic!("turret damage profile")
    };
    let RetailRuntimeValue::Known(health) = target.collision.health_raw else {
        panic!("turret health")
    };
    assert_eq!(health, 200);
    assert_eq!(
        target.collision.pre_health_damage_buffer_raw,
        RetailRuntimeValue::Known(0)
    );
    // Find a bounded, nonoverflowing velocity stimulus through the real 14D30
    // ordered cap. This never writes health or replaces the authored filter.
    for speed in [2_000_i16, 4_000, 8_000, 12_000, 16_000, 24_000, 30_000] {
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
    panic!("canonical Type122/Type97 pair never reached the turret's lethal cap");
}

fn pair_with_playing(f: &mut Fixture, hull: Option<&mut PlayerHull>) -> NativeCaptorPairOutcome {
    resolve_native_captor_active_contacts_with_playing(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut f.entities,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut f.static_damage,
            notifications: &mut f.notifications,
            retail_tick: f.tick,
            actor_tasks: &mut f.scheduler,
        },
        f.subject,
        CaptureFeedbackPolicy::Gameplay,
        hull.map(
            |hull| crate::native_actor_capture::pair::PlayingPlayerContact {
                hull,
                extra_lives: RetailRuntimeValue::Unresolved,
            },
        ),
    )
}

#[v2k_test_support::retail_test]
fn world46_type97_lethal_pair_finishes_playing_class49_with_ring_and_deferred_removal() {
    let mut f = ready();
    let plan = set_turret_lethal_motion(&mut f);
    assert!(plan.capped_pair_damage_raw > 0);
    let mut hull = PlayerHull::default();
    let result = pair_with_playing(&mut f, Some(&mut hull));
    let NativeCaptorPairOutcome::Resolved { visits } = result else {
        panic!("{result:?}");
    };
    assert_eq!(visits.len(), 1);
    let visit = &visits[0];
    assert!(!visit.physical_suppressed);
    assert_eq!(
        visit.stages.last(),
        Some(&NativeCaptorPairStage::Physical {
            capped_damage_raw: plan.capped_pair_damage_raw
        })
    );
    // The turret's lethal 15040 entry finishes the Playing Class49 terminal:
    // static/dynamic blast, Type60 ring and deferred-removal request.
    assert!(crate::class49_death::finished_terminal_hit_authenticates(
        &f.entities,
        f.child
    ));
    assert_eq!(
        f.entities.pending_actor_deferred_destroy_ids(),
        &[f.child],
        "turret deferred removal without borrowing campaign reconstruction"
    );
    assert_eq!(
        f.entities
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count(),
        1,
        "one Type60 ring from the pair-driven Class49 blast"
    );
    // The Tertiary Class29 graph is replaced by the Class49 continuation.
    let turret = actor(&f.entities, f.child).unwrap();
    assert!(matches!(
        turret.actor_task_state(ActorTaskSlot::Tertiary),
        None | Some(ActorTaskRuntime::CommonDying(_)) | Some(ActorTaskRuntime::Intro2GunTurret(_))
    ));
}

#[v2k_test_support::retail_test]
fn world46_type97_lethal_pair_without_playing_context_preserves_health_prefix() {
    let mut f = ready();
    let plan = set_turret_lethal_motion(&mut f);
    assert!(plan.capped_pair_damage_raw > 0);
    let result = pair_with_playing(&mut f, None);
    let NativeCaptorPairOutcome::Blocked {
        reason,
        committed_prefix,
        ..
    } = result
    else {
        panic!("{result:?}");
    };
    assert!(
        committed_prefix,
        "lethal health write commits before the boundary"
    );
    assert!(
        matches!(
            reason,
            NativeCaptorPairBlock::MissingPlayingContext { entity_id }
                if entity_id == f.child
        ),
        "{reason:?}"
    );
    // Health already reached zero or below; the Class49 blast, ring and
    // removal wait for the explicit Playing context instead of an invented hull.
    let RetailRuntimeValue::Known(health) =
        actor(&f.entities, f.child).unwrap().collision.health_raw
    else {
        panic!("turret health");
    };
    assert!(health <= 0, "lethal health prefix commits: {health}");
    assert!(!crate::class49_death::finished_terminal_hit_authenticates(
        &f.entities,
        f.child
    ));
    assert!(f.entities.pending_actor_deferred_destroy_ids().is_empty());
    assert_eq!(
        f.entities
            .iter_all()
            .filter(|entity| entity.entity_type == 60)
            .count(),
        0
    );
}
