//! Ordinary Type18 rows on the shared ground host.

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

/// Every authored world and its Type18 count (19 rows).
const WORLDS: [(u32, usize); 8] = [
    (16, 2),
    (21, 1),
    (22, 1),
    (23, 2),
    (30, 3),
    (33, 4),
    (34, 3),
    (36, 3),
];

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

#[v2k_test_support::retail_test]
fn type18_births_publish_every_authored_row() {
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
                matches!(class(&manager, id), 4 | 7 | 9 | 26 | 33),
                "world{level}: class{}",
                class(&manager, id)
            );
            assert!(Type18Owner::adopt(&manager, id).is_ok(), "world{level}");
            let runtime = entity(&manager, id).native_type18_runtime.unwrap();
            assert_eq!(runtime.sub_e_runtime.projectile_method, 20);
            assert_eq!(runtime.sub_e_runtime.sound_id, 69);
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type18(&manager), count, "world{level}");
        assert!(session.cache.level_desc().is_some());
        total += count;
    }
    assert_eq!(total, 19);
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
fn type18_cohorts_run_ten_seconds_with_late_static_contact() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type18(&manager), ids.len());
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
                if let SpecializedActorTaskProductionOutcome::NativeType18(outcome) = outcome {
                    assert!(
                        !matches!(outcome, Type18Outcome::Blocked { .. }),
                        "world{level} tick{t}: {outcome:?}"
                    );
                }
            }
            for &id in &ids {
                let outcome = contact::resolve_type18_static_contact(
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
                    !matches!(outcome, contact::Type18ContactOutcome::Blocked { .. }),
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
        assert!(tasks.adopt_type18(&manager) > 0);
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
        let SharedActorImpactOutcome::Type18(impact::Type18ImpactOutcome::Applied(applied)) =
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
        // The corpse falls under the class12 world (effective 420, no drag).
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

/// A player inside the 4352 axis weighs Search20 against the rest: Type18
/// eventually takes class7, pursues, and its method20 shots leave the FIFO.
#[v2k_test_support::retail_test]
fn a_nearby_player_draws_type18_fire() {
    let (mut session, mut manager, mut fx) = fixture(16);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
    let [x, y, z] = entity(&manager, ids[0]).position_raw();
    manager.player_mut().unwrap().set_position_raw([
        x.wrapping_add(0x300),
        y.wrapping_add(0x200),
        z,
    ]);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type18(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut fired = 0;
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
        for &id in &ids {
            let queued = entity(&manager, id)
                .native_type18_aim_runtime
                .as_ref()
                .map_or(0, |runtime| runtime.queued_shot_count());
            if queued == 0 {
                continue;
            }
            let outcome = aim::drain_type18_shots(
                &mut manager,
                &mut fx,
                id,
                crate::world_fx::ParticleEnvironment::Terrain(
                    crate::world_fx::TerrainCollisionContext::from_current_level_cache(
                        &session.cache,
                    )
                    .unwrap(),
                ),
                t,
            )
            .unwrap();
            assert_eq!(outcome.consumed_requests, queued);
            fired += queued;
        }
        if fired > 0 {
            break;
        }
    }
    assert!(fired > 0, "no Type18 fired within 60 s of a nearby player");
}

/// World 21's Type18 weighs Capture8 when people are near: within a minute it
/// attaches a person and carries it without a blocked owner.
#[v2k_test_support::retail_test]
fn a_type18_captures_and_carries_a_person() {
    let (mut session, mut manager, mut fx) = fixture(21);
    let ids = ids(&manager);
    activate(&mut manager, &ids);
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
            if let SpecializedActorTaskProductionOutcome::NativeType18(outcome) = outcome {
                assert!(
                    !matches!(outcome, Type18Outcome::Blocked { .. }),
                    "tick{t}: {outcome:?}"
                );
            }
        }
        let pairs = crate::native_actor_capture::pair::resolve_native_captor_active_contacts(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: t,
                actor_tasks: &mut tasks,
            },
            ids[0],
            crate::native_actor_capture::pair::CaptureFeedbackPolicy::Gameplay,
        );
        assert!(
            !matches!(
                pairs,
                crate::native_actor_capture::pair::NativeCaptorPairOutcome::Blocked { .. }
            ),
            "tick{t}: {pairs:?}"
        );
        if captured_at.is_none() && entity(&manager, ids[0]).native_capture_relation.is_some() {
            captured_at = Some(t);
        }
    }
    // A Type86 person is attached within the first ten seconds.
    assert!(
        captured_at.is_some_and(|tick| tick < 500),
        "{captured_at:?}"
    );
}
