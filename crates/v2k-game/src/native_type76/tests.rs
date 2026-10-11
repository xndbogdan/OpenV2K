//! Ordinary Type76/Type77 rows on the shared ground host.

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

/// Every authored world and its Type76-family count (6 + 8 rows).
const WORLDS: [(u32, usize); 5] = [(26, 3), (31, 4), (37, 2), (39, 3), (40, 2)];

fn fixture(level: u32) -> (GameSession, EntityManager, WorldFx) {
    crate::native_type122::construction_tests::native_fixture_with_player(level)
}

fn ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| Type76Row::from_entity_type(entity.entity_type).is_some())
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
    tasks.adopt_type28(manager);
    tasks.adopt_type76_family(manager);
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
            let blocked_on_family = match reason {
                crate::native_actor_capture::pair::NativeCaptorPairBlock::UnresolvedBehavior {
                    entity_type,
                    ..
                } => Type76Row::from_entity_type(*entity_type).is_some(),
                // A parked captor (the held drowning chain) fails its pairs.
                crate::native_actor_capture::pair::NativeCaptorPairBlock::Runtime(
                    "completed pair owner",
                ) => !held_captors,
                _ => true,
            };
            let subject = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(
                !blocked_on_family,
                "tick{t} subject{id} type{} attached {:?} relation {:?}: {pairs:?}",
                subject.entity_type,
                subject.attached_to,
                subject.native_capture_relation.is_some()
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type76_family_births_publish_every_authored_row() {
    let mut births = [0; 2];
    for (level, count) in WORLDS {
        let (session, manager, _) = fixture(level);
        let ids = ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            assert!(
                manager_allocation_authenticates(&manager, id),
                "world{level}"
            );
            let row = type76_row(entity(&manager, id)).unwrap();
            assert!(
                row.living_classes().contains(&class(&manager, id)),
                "world{level}: class{}",
                class(&manager, id)
            );
            assert!(
                Type76FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
            let runtime = entity(&manager, id).native_type76_runtime.unwrap();
            assert_eq!(
                runtime.sub_e_runtime.projectile_method,
                row.emitter().projectile_method
            );
            births[usize::from(row == Type76Row::Type77)] += 1;
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type76_family(&manager), count, "world{level}");
        assert!(session.cache.level_desc().is_some());
    }
    assert_eq!(births, [6, 8]);
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
fn type76_family_cohorts_run_ten_seconds_with_late_static_contact() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type76_family(&manager), ids.len());
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
                if let SpecializedActorTaskProductionOutcome::NativeType76Family(outcome) = outcome
                {
                    assert!(
                        !matches!(outcome, Type76Outcome::Blocked { .. }),
                        "world{level} tick{t}: {outcome:?}"
                    );
                }
            }
            for &id in &ids {
                let outcome = contact::resolve_type76_family_static_contact(
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
                    !matches!(outcome, contact::Type76ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
                classes.insert(class(&manager, id));
            }
        }
        assert!(classes
            .iter()
            .all(|class| matches!(class, 4 | 5 | 7 | 26 | 33)));
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
        assert!(tasks.adopt_type76_family(&manager) > 0);
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
        let SharedActorImpactOutcome::Type76Family(impact::Type76ImpactOutcome::Applied(applied)) =
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
        // The corpse falls under the class12 world (effective 28 or 428).
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

/// A minute of every Type76-family world with its pair walks: no owner or pair
/// blocks, and the classes and captures each actor reaches.
#[v2k_test_support::retail_test]
fn type76_family_worlds_run_a_minute_with_pairs() {
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = adopt_world(&mut manager);
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut classes = std::collections::BTreeSet::new();
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
                if let SpecializedActorTaskProductionOutcome::NativeType76Family(
                    Type76Outcome::Blocked { .. },
                ) = outcome
                {
                    panic!("world{level} tick{t}: {outcome:?}");
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
                false,
            );
            for &id in &ids {
                if !manager
                    .iter_all()
                    .any(|entity| entity.id == id && entity.active)
                {
                    continue;
                }
                classes.insert(class(&manager, id));
            }
        }
        println!("world{level} classes {classes:?}");
    }
}

/// The nearest cell centre whose seabed, and its eight neighbours', lies
/// below `threshold`: a body resting there stays in E370's deep band.
fn deep_seabed(
    terrain: &v2k_formats::terrain::TerrainGrid,
    near: [i16; 3],
    threshold: i16,
) -> Option<[i16; 3]> {
    let centre = |cx: i32, cz: i32| {
        (
            ((cx & 0xff) << 8 | 0x80) as u16 as i16,
            ((cz & 0xff) << 8 | 0x80) as u16 as i16,
        )
    };
    let deep = |cx: i32, cz: i32| {
        let (x, z) = centre(cx, cz);
        terrain.bilinear_height_raw(x, z) < threshold
    };
    let home = [
        i32::from(near[0] as u16 >> 8),
        i32::from(near[2] as u16 >> 8),
    ];
    (0..256 * 256)
        .map(|index| (index >> 8, index & 0xff))
        .filter(|&(cx, cz)| (-1..=1).all(|dx| (-1..=1).all(|dz| deep(cx + dx, cz + dz))))
        .min_by_key(|&(cx, cz)| {
            let wrap = |d: i32| d.rem_euclid(256).min(256 - d.rem_euclid(256));
            wrap(cx - home[0]).pow(2) + wrap(cz - home[1]).pow(2)
        })
        .map(|(cx, cz)| {
            let (x, z) = centre(cx, cz);
            [x, terrain.bilinear_height_raw(x, z), z]
        })
}

/// E370 drowns either row after 2000 ms below the sea: its lifecycle runs the
/// same class12 as a lethal hit, through the ground host's surface step.
/// Only world 40 has terrain under its sea, and its rows are Type77.
#[v2k_test_support::retail_test]
fn a_submerged_row_drowns_into_class12() {
    let mut wet = Vec::new();
    for (level, _) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let terrain = session.cache.level_terrain().unwrap().clone();
        // Bilinear terrain never dips below its lowest corner. World 31's sea
        // lies under all of its terrain; 26/37/39 park it at the dry marker.
        let lowest = terrain
            .cells
            .iter()
            .map(|cell| i16::from(cell.height as i8) << 5)
            .min()
            .unwrap();
        let submersible = terrain.water_enabled() && lowest < terrain.sea_level_raw();
        assert_eq!(submersible, level == 40, "world{level}");
        if !submersible {
            continue;
        }
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        for row in [Type76Row::Type76, Type76Row::Type77] {
            let Some(&id) = ids
                .iter()
                .find(|&&id| type76_row(entity(&manager, id)) == Some(row))
            else {
                continue;
            };
            let radius = session
                .cache
                .global_model(entity(&manager, id).model_index.unwrap())
                .unwrap()
                .radius;
            // E370 is deep strictly below sea - extent/4.
            let threshold = terrain.sea_level_raw() - (radius >> 2) as i16;
            let seabed = deep_seabed(&terrain, entity(&manager, id).position_raw(), threshold)
                .unwrap_or_else(|| panic!("world{level}: no deep seabed"));
            let mut tasks = SpecializedActorTaskScheduler::new();
            assert_eq!(tasks.adopt_type76_family(&manager), ids.len());
            let mut static_damage = StaticDamageScheduler::new();
            let mut notifications = GameplayNotifications::new();
            let mut drowned = None;
            for t in 1..=300u32 {
                // Rest the body on that seabed: C+0c is terrain-only, so no
                // lift carries it out of the deep band.
                manager
                    .entity_mut(id)
                    .unwrap()
                    .set_motion_raw(seabed, [0, 0, 0]);
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
                    if let SpecializedActorTaskProductionOutcome::NativeType76Family(outcome) =
                        outcome
                    {
                        assert!(
                            !matches!(outcome, Type76Outcome::Blocked { .. }),
                            "world{level} tick{t}: {outcome:?}"
                        );
                    }
                }
                if crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(&manager, id).is_ok() {
                    drowned = Some(t);
                    break;
                }
            }
            // 2000 ms of 20-ms visits: the lifecycle fires on the 100th.
            let tick = drowned.unwrap_or_else(|| panic!("world{level} {row:?}: no drowning"));
            assert_eq!(tick, 100, "world{level} {row:?}");
            assert_eq!(
                tasks.family_for(id),
                Some(crate::specialized_actor_task_production::SpecializedActorTaskFamily::Intro2CommonDying)
            );
            wet.push((level, row));
        }
    }
    assert_eq!(wet, [(40, Type76Row::Type77)]);
}

/// A hit makes UnderAttack weigh Search (10 or 20) and the row's emitter
/// fires at a nearby player through the native FIFO.
#[v2k_test_support::retail_test]
fn a_hit_row_fires_at_a_nearby_player() {
    use crate::shared_actor_impact::{apply_shared_actor_particle_hit, SharedActorImpactFrame};
    for (level, row) in [(31, Type76Row::Type76), (39, Type76Row::Type77)] {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = ids(&manager);
        activate(&mut manager, &ids);
        let id = *ids
            .iter()
            .find(|&&id| type76_row(entity(&manager, id)) == Some(row))
            .unwrap();
        let [x, y, z] = entity(&manager, id).position_raw();
        manager.player_mut().unwrap().set_position_raw([
            x.wrapping_add(0x300),
            y.wrapping_add(0x200),
            z,
        ]);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type76_family(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut fired = 0;
        for t in 1..=3000u32 {
            // A small nonlethal hit every second keeps 16490's window open.
            if t % 50 == 1 {
                let mut hit = lethal_hit(id);
                hit.damage.as_mut().unwrap().packet.amounts_raw = [4500, 0];
                let _ = apply_shared_actor_particle_hit(
                    SharedActorImpactFrame {
                        resources: &session.cache,
                        entities: &mut manager,
                        world_fx: &mut fx,
                        scheduler: &mut tasks,
                        notifications: &mut notifications,
                        retail_tick: t,
                    },
                    hit,
                );
                manager.entity_mut(id).unwrap().collision.health_raw =
                    RetailRuntimeValue::Known(row.health());
            }
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
            let queued = entity(&manager, id)
                .native_type76_aim_runtime
                .as_ref()
                .map_or(0, |runtime| runtime.queued_shot_count());
            if queued > 0 {
                let outcome = aim::drain_type76_family_shots(
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
                break;
            }
        }
        assert!(fired > 0, "world{level}: {row:?} never fired");
    }
}
