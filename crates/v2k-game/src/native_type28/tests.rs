//! Ordinary Type28 rows on the shared ground host.

use super::*;
use crate::{
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    session::GameSession,
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::WorldFx,
};

/// Every authored world and its Type28 count (11 rows).
const WORLDS: [(u32, usize); 5] = [(21, 1), (22, 1), (23, 2), (30, 5), (36, 2)];

fn fixture(level: u32) -> (GameSession, EntityManager, WorldFx) {
    crate::native_type122::construction_tests::native_fixture_with_player(level)
}

fn ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn class(manager: &EntityManager, id: u32) -> u8 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context
        .active_style()
        .audited()
        .map_or(0, |style| style.class_id)
}

fn style(manager: &EntityManager, id: u32) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context.active_style().style_address()
}

fn context_target(manager: &EntityManager, id: u32) -> Option<u32> {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        return None;
    };
    match context.target_handle_at_0x08() {
        RetailRuntimeValue::Known(target) => target,
        RetailRuntimeValue::Unresolved => None,
    }
}

fn activate(manager: &mut EntityManager, ids: &[u32]) {
    for &id in ids {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
}

/// Production's ordinary adoption set: every native owner `main` adopts.
fn adopt_world(manager: &mut EntityManager) -> SpecializedActorTaskScheduler {
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_intro2_type53(manager);
    tasks.adopt_type122(manager);
    tasks.adopt_type18(manager);
    tasks.adopt_type28(manager);
    tasks.adopt_type30(manager);
    tasks.adopt_type40(manager);
    tasks.adopt_type56(manager);
    tasks.adopt_type43(manager);
    tasks.adopt_type38_family(manager);
    tasks.adopt_shared_fish(manager);
    tasks.adopt_cleansing_vehicle(manager);
    tasks.adopt_intro2_type16(manager);
    tasks.adopt_intro2_type58(manager);
    tasks.adopt_intro2_type94(manager);
    tasks.adopt_intro2_type66(manager);
    tasks.adopt_class0_actors(manager);
    tasks.adopt_main_base(manager);
    tasks.adopt_intro2_type10(manager);
    tasks.adopt_intro2_type57(manager);
    tasks.adopt_intro2_gun_turret(manager);
    tasks.adopt_intro2_type17(manager);
    tasks.adopt_intro2_type8(manager);
    tasks.adopt_intro2_type9(manager);
    tasks.adopt_native_type123(manager);
    tasks.adopt_native_type86(manager);
    tasks
}

/// 11AD0's pair walk over every subject in live order, as Playing runs it:
/// a pair is visited from whichever body comes first.
#[allow(clippy::too_many_arguments)]
fn walk_pairs(
    manager: &mut EntityManager,
    session: &mut GameSession,
    fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    tasks: &mut SpecializedActorTaskScheduler,
    t: u32,
    held_captors: bool,
) {
    let live: Vec<u32> = manager.retail_live_order_ids().collect();
    for id in live {
        if !manager
            .iter_all()
            .any(|entity| entity.id == id && entity.active)
        {
            continue;
        }
        let pairs = crate::native_actor_capture::pair::resolve_native_captor_active_contacts(
            &mut Intro2ContactFrame {
                entities: manager,
                resources: &mut session.cache,
                world_fx: fx,
                static_damage,
                notifications,
                retail_tick: t,
                actor_tasks: tasks,
            },
            id,
            crate::native_actor_capture::pair::CaptureFeedbackPolicy::Gameplay,
        );
        if let crate::native_actor_capture::pair::NativeCaptorPairOutcome::Blocked {
            reason, ..
        } = &pairs
        {
            let blocked_on_type28 = match reason {
                crate::native_actor_capture::pair::NativeCaptorPairBlock::UnresolvedBehavior {
                    entity_type,
                    ..
                } => *entity_type == ENTITY_TYPE,
                // A parked captor (the held drowning chain) fails its pairs.
                crate::native_actor_capture::pair::NativeCaptorPairBlock::Runtime(
                    "completed pair owner",
                ) => !held_captors,
                _ => true,
            };
            let subject = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(
                !blocked_on_type28,
                "tick{t} subject{id} type{} attached {:?} relation {:?}: {pairs:?}",
                subject.entity_type,
                subject.attached_to,
                subject.native_capture_relation.is_some()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type28_births_publish_every_authored_row() {
    let mut total = 0;
    for (level, count) in WORLDS {
        let (session, manager, _) = fixture(level);
        let ids = ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            assert!(
                manager_allocation_authenticates(&manager, id),
                "world{level}"
            );
            assert!(
                matches!(class(&manager, id), 9 | 10 | 26 | 33),
                "world{level}: class{}",
                class(&manager, id)
            );
            assert!(Type28Owner::adopt(&manager, id).is_ok(), "world{level}");
            assert!(entity(&manager, id).native_type28_runtime.is_some());
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type28(&manager), count, "world{level}");
        assert!(session.cache.level_desc().is_some());
        total += count;
    }
    assert_eq!(total, 11);
}

#[allow(clippy::too_many_arguments)]
fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut GameSession,
    fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    t: u32,
) -> crate::specialized_actor_task_production::SpecializedActorTaskProductionPass {
    tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: t * 20_000,
            retail_tick: t,
            main_base_abort_active: false,
        },
        notifications,
    )
}

/// Ten seconds of every cohort under the real scheduler and the late static
/// walk: no owner blocks and every survivor keeps a living graph.
#[v2k_test_support::retail_test]
fn type28_cohorts_run_ten_seconds_with_late_static_contact() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type28(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut classes = std::collections::BTreeSet::new();
        for t in 1..=500u32 {
            let pass = tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            assert!(
                pass.block.is_none(),
                "world{level} tick{t}: {:?}",
                pass.block
            );
            for outcome in &pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::NativeType28(outcome) = outcome {
                    assert!(
                        !matches!(outcome, Type28Outcome::Blocked { .. }),
                        "world{level} tick{t}: {outcome:?}"
                    );
                }
            }
            for &id in &ids {
                let outcome = contact::resolve_type28_static_contact(
                    &mut Intro2ContactFrame {
                        entities: &mut manager,
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        static_damage: &mut static_damage,
                        notifications: &mut notifications,
                        retail_tick: t,
                        actor_tasks: &mut tasks,
                    },
                    id,
                );
                assert!(
                    !matches!(outcome, contact::Type28ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
                classes.insert(class(&manager, id));
            }
        }
        assert!(classes
            .iter()
            .all(|class| matches!(class, 4 | 7 | 9 | 26 | 33)));
    }
}

fn lethal_hit(id: u32) -> crate::world_fx::ParticleEntityImpact {
    crate::world_fx::ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(crate::world_fx::BallisticDamageRequest {
            packet: crate::damage::DamagePacket {
                channels: [1, 0],
                amounts_raw: [1_000_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

/// A lethal primary hit takes alternate class12 through Type122's captor
/// death: the Class12 owner replaces the living one.
#[v2k_test_support::retail_test]
fn a_lethal_hit_runs_class12() {
    use crate::shared_actor_impact::{
        apply_shared_actor_particle_hit, SharedActorImpactFrame, SharedActorImpactOutcome,
    };
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let id = ids(&manager)[0];
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_type28(&manager) > 0);
        let mut notifications = GameplayNotifications::new();
        let outcome = apply_shared_actor_particle_hit(
            SharedActorImpactFrame {
                resources: &session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut tasks,
                notifications: &mut notifications,
                retail_tick: 5000,
            },
            lethal_hit(id),
        )
        .unwrap();
        let SharedActorImpactOutcome::Type28(impact::Type28ImpactOutcome::Applied(applied)) =
            outcome
        else {
            panic!("world{level}: {outcome:?}")
        };
        assert!(applied.death_publication.is_some(), "world{level}");
        assert_eq!(
            entity(&manager, id).collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert!(
            crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(&manager, id).is_ok(),
            "world{level}"
        );
        assert_eq!(
            tasks.family_for(id),
            Some(crate::specialized_actor_task_production::SpecializedActorTaskFamily::Intro2CommonDying)
        );
        // The corpse falls under the class12 world (effective 428).
        let mut static_damage = StaticDamageScheduler::new();
        for t in 5001..=5250 {
            let pass = tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            assert!(
                pass.block.is_none(),
                "world{level} tick{t}: {:?}",
                pass.block
            );
        }
    }
}

/// A minute of every Type28 world with its pair walks: no owner or pair
/// blocks, and the classes and captures each actor reaches.
#[v2k_test_support::retail_test]
fn type28_worlds_run_a_minute_with_pairs() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = adopt_world(&mut manager);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut classes = std::collections::BTreeSet::new();
        let mut captures = 0;
        let mut drowned = std::collections::BTreeSet::new();
        let mut held = std::collections::BTreeSet::new();
        for t in 1..=3000u32 {
            let pass = tick(
                &mut tasks,
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                t,
            );
            assert!(
                pass.block.is_none(),
                "world{level} tick{t}: {:?}",
                pass.block
            );
            for outcome in &pass.outcomes {
                match outcome {
                    // Held: a carried four-choice person drowning runs 16750's
                    // CE90 release, which its E370 does not own yet.
                    SpecializedActorTaskProductionOutcome::NativeType86(
                        crate::native_type86::Type86Outcome::Blocked {
                            entity_id,
                            reason:
                                crate::native_type86::Type86Block::Surface(
                                    crate::intro2_common_dying::Intro2CommonDyingBlock::UnsupportedReleaseHook(
                                        crate::entity_behavior::ReleaseCallbackPolicy::EnablePairAndReselect,
                                    ),
                                ),
                            ..
                        },
                    ) => {
                        drowned.insert(*entity_id);
                    }
                    SpecializedActorTaskProductionOutcome::NativeType28(
                        Type28Outcome::Blocked { reason, .. },
                    ) => {
                        // ...after which its captor's release fails closed.
                        assert!(
                            !drowned.is_empty()
                                && matches!(
                                    reason,
                                    Type28Block::Capture(block)
                                        if block.reason == "capture four-choice actor pending callback"
                                ),
                            "world{level} tick{t}: {outcome:?}"
                        );
                        held.insert(outcome.entity_id());
                    }
                    SpecializedActorTaskProductionOutcome::NativeType18(
                        crate::native_type18::Type18Outcome::Blocked { .. },
                    ) => panic!("world{level} tick{t}: {outcome:?}"),
                    _ => {}
                }
            }
            walk_pairs(
                &mut manager,
                &mut session,
                &mut fx,
                &mut static_damage,
                &mut notifications,
                &mut tasks,
                t,
                !held.is_empty(),
            );
            for &id in &ids {
                if !manager
                    .iter_all()
                    .any(|entity| entity.id == id && entity.active)
                {
                    continue;
                }
                if entity(&manager, id).native_capture_relation.is_some() {
                    captures += 1;
                }
                classes.insert(class(&manager, id));
            }
        }
        println!(
            "world{level} classes {classes:?} captured-ticks {captures} drowned {drowned:?} held captors {held:?}"
        );
    }
}

/// With a person beside it, PeopleNearby weighs Capture9 against Follow3.
/// Once Type28 pursues that person, a body contact runs C910: the person
/// attaches to Type28's J row and is carried without a block.
#[v2k_test_support::retail_test]
fn a_type28_captures_its_pursued_person() {
    let (mut session, mut manager, mut fx) = fixture(21);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
    let person = manager
        .iter_all()
        .find(|entity| {
            entity.capability_flags & 0x0c00 != 0 && entity.native_type86_runtime.is_some()
        })
        .map(|entity| entity.id)
        .expect("world 21 authors native Type86 people");
    let [x, y, z] = entity(&manager, ids[0]).position_raw();
    manager.entity_mut(person).unwrap().set_position_raw([
        x.wrapping_add(0x180),
        y,
        z.wrapping_add(0x180),
    ]);
    let mut tasks = adopt_world(&mut manager);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut captured_at = None;
    for t in 1..=3000u32 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::NativeType28(outcome) = outcome {
                assert!(
                    !matches!(outcome, Type28Outcome::Blocked { .. }),
                    "tick{t}: {outcome:?}"
                );
            }
        }
        // Pursuit 4C8038 of this person: bring the bodies into contact.
        if captured_at.is_none()
            && style(&manager, ids[0]) == 0x004c_8038
            && context_target(&manager, ids[0]) == Some(person)
        {
            let [x, y, z] = entity(&manager, ids[0]).position_raw();
            manager
                .entity_mut(person)
                .unwrap()
                .set_position_raw([x.wrapping_add(0x20), y, z]);
        }
        walk_pairs(
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            &mut tasks,
            t,
            false,
        );
        if captured_at.is_none() && entity(&manager, ids[0]).native_capture_relation.is_some() {
            captured_at = Some(t);
            assert_eq!(entity(&manager, person).attached_to, Some(ids[0]));
        }
    }
    assert!(
        captured_at.is_some(),
        "the pursued person was never attached"
    );
}
