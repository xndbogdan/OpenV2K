//! Type73/Type109 splitting bugs on the Type31 owner: Wander Near Location,
//! Run Away and Defecate Virus, the 416820 lethal split and class2 death.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    player_hull::PlayerHull,
    shared_actor_impact::{
        apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
    },
    specialized_actor_task_production::{
        SpecializedActorTaskProductionFrame, SpecializedActorTaskProductionOutcome,
        SpecializedActorTaskScheduler, SpecializedActorTaskWorld,
    },
    static_damage::StaticDamageScheduler,
    world_fx::{BallisticDamageRequest, ParticleEntityImpact},
};

/// Authored bugs: Type73 in worlds 18 and 26, Type109 in world 26.
const WORLDS: [(u32, usize, usize); 2] = [(18, 4, 0), (26, 2, 3)];

fn bugs(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| matches!(entity.entity_type, 73 | 109))
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
}

fn style(manager: &EntityManager, id: u32) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context.active_style().style_address()
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

#[allow(clippy::too_many_arguments)]
fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
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

fn hit(id: u32, channel: i32, amount: i32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        no_packet: false,
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [channel, 0],
                amounts_raw: [amount, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

struct World {
    session: crate::session::GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: SpecializedActorTaskScheduler,
    static_damage: StaticDamageScheduler,
    notifications: GameplayNotifications,
    hull: PlayerHull,
}

impl World {
    fn new(level: u32) -> Self {
        let (session, mut manager, fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        let ids = bugs(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type31_family(&manager), ids.len());
        Self {
            session,
            manager,
            fx,
            tasks,
            static_damage: StaticDamageScheduler::new(),
            notifications: GameplayNotifications::new(),
            hull: PlayerHull::default(),
        }
    }
    fn hit(&mut self, impact: ParticleEntityImpact) -> Option<SharedActorImpactOutcome> {
        apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                extra_lives: RetailRuntimeValue::Known(3),
                resources: &mut self.session.cache,
                entities: &mut self.manager,
                world_fx: &mut self.fx,
                scheduler: &mut self.tasks,
                notifications: &mut self.notifications,
                static_damage: &mut self.static_damage,
                player_hull: &mut self.hull,
                retail_tick: 600,
            },
            impact,
        )
    }
}

#[v2k_test_support::retail_test]
fn both_bug_rows_authenticate_their_canonical_metadata() {
    let (session, _) = crate::native_type122::construction_tests::fixture();
    for row in [Type31Row::Type73, Type31Row::Type109] {
        let metadata = EntityTypeRuntimeMetadata::from_section12(
            session
                .cache
                .global_entity_type(row.entity_type() as usize)
                .unwrap(),
        );
        authenticate_metadata(row, &metadata).unwrap();
    }
}

#[v2k_test_support::retail_test]
fn authored_bugs_publish_wander_or_defecate_with_the_split_modifier() {
    for (level, type73, type109) in WORLDS {
        let (_, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = bugs(&manager);
        let count = |entity_type| {
            ids.iter()
                .filter(|&&id| entity(&manager, id).entity_type == entity_type)
                .count()
        };
        assert_eq!((count(73), count(109)), (type73, type109), "world{level}");
        for &id in &ids {
            let entity = entity(&manager, id);
            let row = type31_row(entity).unwrap();
            let runtime = entity.native_type31_runtime.unwrap();
            assert!(runtime.spawn_index.is_some() && runtime.birth.is_none());
            assert_eq!(runtime.sub_e_runtime.is_some(), row == Type31Row::Type109);
            assert_eq!(
                entity.collision.pair_callbacks.damage_modifier_address,
                RetailRuntimeValue::Known(Some(0x0041_6820))
            );
            assert_eq!(
                entity.sub_h_external_frame_runtime,
                RetailRuntimeValue::Known(None)
            );
            assert_eq!(
                entity.collision.default_state_flags_at_0xc8,
                RetailRuntimeValue::Known(row.default_flags())
            );
            // UnderAttack is false at birth: Wander or Defecate.
            assert!(
                matches!(style(&manager, id), 0x004c_79c0 | 0x004c_7e88),
                "world{level}: {:#x}",
                style(&manager, id)
            );
            assert_eq!(runtime.anchor_raw, entity.position_raw());
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type31_family(&manager), ids.len());
    }
}

/// Ten seconds under the real scheduler and the late static walk.
#[v2k_test_support::retail_test]
fn bug_cohorts_run_ten_seconds_with_late_static_contact() {
    for (level, ..) in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = bugs(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type31_family(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let start: Vec<_> = ids
            .iter()
            .map(|&id| entity(&manager, id).position_raw())
            .collect();
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
                if let SpecializedActorTaskProductionOutcome::NativeType31Family(outcome) = outcome
                {
                    assert!(
                        !matches!(
                            outcome,
                            Type31Outcome::Blocked { .. } | Type31Outcome::Pending { .. }
                        ),
                        "world{level} tick{t}: {outcome:?}"
                    );
                }
            }
            for &id in &ids {
                let outcome = contact::resolve_type31_family_static_contact(
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
                    !matches!(outcome, contact::Type31ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        for (&id, start) in ids.iter().zip(start) {
            assert!(
                Type31FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
            assert_ne!(
                entity(&manager, id).position_raw(),
                start,
                "world{level} moves"
            );
        }
    }
}

/// An unaged body's 416820 returns 0: the hit only stamps `+34`.
#[v2k_test_support::retail_test]
fn an_unaged_bug_absorbs_a_lethal_hit() {
    let mut world = World::new(18);
    let id = bugs(&world.manager)[0];
    world
        .manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
    let before = world.manager.iter_all().count();
    let outcome = world.hit(hit(id, 2, 1_000_000));
    let Some(SharedActorImpactOutcome::Type31Family(
        crate::native_ground_actor::impact::NativeGroundImpactOutcome::Applied(applied),
    )) = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!(applied.filtered_damage_raw, 0);
    assert!(applied.death_publication.is_none());
    let bug = entity(&world.manager, id);
    assert_eq!(bug.collision.health_raw, RetailRuntimeValue::Known(1000));
    assert_eq!(world.manager.iter_all().count(), before);
}

/// An aged bug's lethal hit births two owned bugs of its own type, then
/// dies quietly through class2.
#[v2k_test_support::retail_test]
fn a_lethal_hit_splits_an_aged_bug_into_two_owned_newborns() {
    for (level, entity_type) in [(18, 73), (26, 109)] {
        let mut world = World::new(level);
        let id = bugs(&world.manager)
            .into_iter()
            .find(|&id| entity(&world.manager, id).entity_type == entity_type)
            .unwrap();
        world
            .manager
            .entity_mut(id)
            .unwrap()
            .collision
            .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(20_000);
        let before: Vec<_> = world.manager.iter_all().map(|e| e.id).collect();
        let outcome = world.hit(hit(id, 2, 1_000_000));
        let Some(SharedActorImpactOutcome::Type31Family(
            crate::native_ground_actor::impact::NativeGroundImpactOutcome::Applied(applied),
        )) = outcome
        else {
            panic!("world{level}: {outcome:?}");
        };
        assert_eq!(applied.filtered_damage_raw, 0, "416820 returns 0");
        assert!(applied.death_publication.is_some());
        assert!(death::finished_terminal_authenticates(&world.manager, id));
        let parent = entity(&world.manager, id);
        assert_eq!(parent.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            parent.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        let parent_position = parent.position_raw();
        let children: Vec<_> = world
            .manager
            .iter_all()
            .filter(|e| !before.contains(&e.id))
            .map(|e| e.id)
            .collect();
        assert_eq!(children.len(), 2, "world{level}");
        assert_eq!(world.tasks.adopt_type31_family(&world.manager), 0);
        for &child in &children {
            let e = entity(&world.manager, child);
            assert_eq!(e.entity_type, entity_type);
            let runtime = e.native_type31_runtime.unwrap();
            let birth = runtime.birth.unwrap();
            assert!(runtime.spawn_index.is_none());
            for axis in [0, 2] {
                assert!(
                    (i32::from(birth.position_raw[axis]) - i32::from(parent_position[axis])).abs()
                        <= 256
                );
            }
            assert_eq!(e.velocity_raw(), birth.velocity_raw);
            assert_eq!(
                e.collision.pair_callbacks.damage_modifier_address,
                RetailRuntimeValue::Known(Some(0x0041_6820))
            );
            assert!(matches!(
                style(&world.manager, child),
                0x004c_79c0 | 0x004c_7e88
            ));
            // A newborn's +68 is still zero: a second lethal hit is absorbed.
            assert_eq!(
                e.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(0)
            );
        }
        let child = children[0];
        let outcome = world.hit(hit(child, 2, 1_000_000));
        assert!(
            matches!(
                outcome,
                Some(SharedActorImpactOutcome::Type31Family(
                    crate::native_ground_actor::impact::NativeGroundImpactOutcome::Applied(_)
                ))
            ),
            "{outcome:?}"
        );
        assert_eq!(
            entity(&world.manager, child).collision.health_raw,
            RetailRuntimeValue::Known(1000)
        );
    }
}

/// A hit carried by channel 3 kills without births (particle 90).
#[v2k_test_support::retail_test]
fn a_channel3_lethal_hit_dies_without_births() {
    let mut world = World::new(18);
    let id = bugs(&world.manager)[0];
    world
        .manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(20_000);
    let before = world.manager.iter_all().count();
    let outcome = world.hit(hit(id, 3, 1_000_000));
    assert!(
        matches!(
            outcome,
            Some(SharedActorImpactOutcome::Type31Family(
                crate::native_ground_actor::impact::NativeGroundImpactOutcome::Applied(_)
            ))
        ),
        "{outcome:?}"
    );
    assert!(death::finished_terminal_authenticates(&world.manager, id));
    assert_eq!(world.manager.iter_all().count(), before);
}

/// A surviving hit stamps `+34`; the impact reselection then weighs
/// UnderAttack x100, and Run Away's H-less flee takes the player.
#[v2k_test_support::retail_test]
fn a_surviving_hit_makes_a_bug_run_away() {
    let mut world = World::new(18);
    let id = bugs(&world.manager)[0];
    world
        .manager
        .entity_mut(id)
        .unwrap()
        .collision
        .recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(20_000);
    let at = entity(&world.manager, id).position_raw();
    let player = world.manager.player().unwrap().id;
    world.manager.entity_mut(player).unwrap().set_position_raw([
        at[0].wrapping_add(0x200),
        at[1],
        at[2],
    ]);
    let outcome = world.hit(hit(id, 2, 1500));
    assert!(
        matches!(
            outcome,
            Some(SharedActorImpactOutcome::Type31Family(
                crate::native_ground_actor::impact::NativeGroundImpactOutcome::Applied(_)
            ))
        ),
        "{outcome:?}"
    );
    assert!(entity(&world.manager, id).collision.health_raw == RetailRuntimeValue::Known(500));
    let mut seen_flee = false;
    for t in 601..=700u32 {
        let pass = tick(
            &mut world.tasks,
            &mut world.manager,
            &mut world.session,
            &mut world.fx,
            &mut world.static_damage,
            &mut world.notifications,
            t,
        );
        assert!(pass.block.is_none(), "tick{t}: {:?}", pass.block);
        if style(&world.manager, id) == 0x004c_7660 {
            seen_flee = true;
            assert!(matches!(
                entity(&world.manager, id).actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::RunAway(_))
            ));
            break;
        }
    }
    assert!(seen_flee, "style {:#x}", style(&world.manager, id));
}

/// Lab receipt (world 18, `415040` called on an aged Type73 with a 4000-point
/// channel-2 packet from the player): retail's two `438080` records, and the
/// shared RNG word at the following `14E90`, from the word before the
/// first birth's nine draws.
#[v2k_test_support::retail_test]
fn a_lethal_split_matches_retail_births_and_rng() {
    use crate::live_actor_checked_damage::{
        apply_live_actor_checked_damage_with_split, LiveActorDamageEntry, LiveActorDamageFeedback,
        LiveActorDamageRequest, LiveActorDeathEntry,
    };
    let mut world = World::new(18);
    let id = bugs(&world.manager)
        .into_iter()
        .find(|&id| {
            let [x, _, z] = entity(&world.manager, id).position_raw();
            (x, z) == (14592, -6400)
        })
        .expect("retail handle 048F0001's spawn");
    let player = world.manager.player().unwrap().id;
    let bug = world.manager.entity_mut(id).unwrap();
    bug.set_position_raw([15047, -339, -5519]);
    bug.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(20_000);
    *world.fx.entity_construction_state().0 = 0x7254_074b;
    let before: Vec<_> = world.manager.iter_all().map(|e| e.id).collect();
    let World {
        session,
        manager,
        fx,
        tasks,
        notifications,
        ..
    } = &mut world;
    let outcome = apply_live_actor_checked_damage_with_split(
        manager,
        fx,
        LiveActorDamageRequest {
            entity_id: id,
            delivery: crate::damage::DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [4000, 0],
                },
                source_entity_type_raw: 46,
                owner_handle: player,
            },
            entry: LiveActorDamageEntry::Checked,
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: Some(LiveActorDamageFeedback {
                notifications,
                retail_tick: 1072,
            }),
        },
        true,
        |manager, fx, feedback, entry| {
            let LiveActorDeathEntry::LethalSplit(split) = entry else {
                panic!("lethal split");
            };
            let feedback = feedback.unwrap();
            split::publish_lethal_split(
                Type31Row::Type73,
                manager,
                id,
                &mut crate::native_ground_actor::NativeGroundDeathContext::Split {
                    resources: &session.cache,
                    fx,
                    tick: 1072,
                    tasks,
                },
                split,
                Some(&mut *feedback.notifications),
            )
        },
    )
    .unwrap();
    assert_eq!(outcome.filtered_damage_raw, 0);
    let births: Vec<_> = manager
        .iter_all()
        .filter(|e| !before.contains(&e.id))
        .map(|e| e.native_type31_runtime.unwrap().birth.unwrap())
        .collect();
    assert_eq!(births.len(), 2);
    assert_eq!(births[0].position_raw, [14824, -102, -5714]);
    assert_eq!(births[0].velocity_raw, [-20, 265, 499]);
    assert_eq!(births[1].position_raw, [14868, -328, -5392]);
    assert_eq!(births[1].velocity_raw, [-290, 310, -412]);
    assert_eq!(*fx.entity_construction_state().0, 0x87a6_95fa);
}
