//! Ordinary Type38 ground shooters and Type129 carriers on the shared host.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    class49_death::{source_profile, NativeExplosionPolicy, NativeExplosionSourceProfile},
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

/// Every authored row: Type129 in world 41, Type38 in worlds 42 and 43.
const WORLDS: [(u32, Type38Row); 3] = [
    (41, Type38Row::Type129),
    (42, Type38Row::Type38),
    (43, Type38Row::Type38),
];

fn ids(manager: &EntityManager, row: Type38Row) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == row.entity_type())
        .map(|entity| entity.id)
        .collect()
}

fn entity(manager: &EntityManager, id: u32) -> &Entity {
    manager.iter_all().find(|entity| entity.id == id).unwrap()
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

fn style(manager: &EntityManager, id: u32) -> u32 {
    let RetailRuntimeValue::Known(Some(context)) = entity(manager, id).current_behavior_context
    else {
        panic!("context");
    };
    context.active_style().style_address()
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

fn assert_no_block(
    pass: &crate::specialized_actor_task_production::SpecializedActorTaskProductionPass,
    context: &str,
) {
    assert!(pass.block.is_none(), "{context}: {:?}", pass.block);
    for outcome in &pass.outcomes {
        if let SpecializedActorTaskProductionOutcome::NativeType38Family(outcome) = outcome {
            assert!(
                !matches!(
                    outcome,
                    Type38Outcome::Blocked { .. } | Type38Outcome::Pending { .. }
                ),
                "{context}: {outcome:?}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn type38_family_births_publish_their_rows_on_the_ground_host() {
    let mut births = [0; 2];
    for (level, row) in WORLDS {
        let (session, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = ids(&manager, row);
        assert!(!ids.is_empty(), "world{level}");
        let level_desc = session.cache.level_desc().unwrap();
        for &id in &ids {
            births[usize::from(row == Type38Row::Type129)] += 1;
            let entity = entity(&manager, id);
            assert!(manager_allocation_authenticates(&manager, id));
            assert_eq!(type38_row(entity), Some(row));
            let runtime = entity.native_type38_runtime.as_ref().unwrap();
            // 24E30 copies each row's emitter sound: Type129 is silent.
            assert_eq!(runtime.sub_e_runtime.projectile_method, 10);
            assert_eq!(
                runtime.sub_e_runtime.sound_id,
                u32::from(row.emitter().sound_id)
            );
            assert!(matches!(style(&manager, id), 0x004c_7930 | 0x004c_7a50));
            assert!(Type38FamilyOwner::adopt(&manager, id).is_ok());
            let profile = source_profile(entity).unwrap();
            assert_eq!(profile, NativeExplosionSourceProfile::Type38Family(row));
            assert_eq!(
                profile.policy(),
                match row {
                    Type38Row::Type38 => NativeExplosionPolicy::Class1,
                    Type38Row::Type129 => NativeExplosionPolicy::Class63,
                }
            );
            if row == Type38Row::Type129 {
                let spawn = &level_desc.entities[entity.authored_spawn_index.unwrap()];
                assert_eq!(
                    entity.auto_pilot_payload_packed,
                    Some(u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap()))
                );
            }
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(
            tasks.adopt_type38_family(&manager),
            ids.len(),
            "world{level}"
        );
    }
    assert_eq!(births, [10, 25]);
}

/// Ten seconds under the real scheduler and the late static walk.
#[v2k_test_support::retail_test]
fn type38_family_cohorts_run_ten_seconds_with_late_static_contact() {
    for (level, row) in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = ids(&manager, row);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type38_family(&manager), ids.len());
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
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
            assert_no_block(&pass, &format!("world{level} tick{t}"));
            for &id in &ids {
                let outcome = contact::resolve_type38_family_static_contact(
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
                    None,
                );
                assert!(
                    !matches!(outcome, contact::Type38ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        for &id in &ids {
            assert!(
                Type38FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
    }
}

/// A player inside the 3840 axis: PlayerNearby makes class7 five times as
/// likely, and method10 shots leave through the native FIFO.
#[v2k_test_support::retail_test]
fn a_nearby_player_draws_type38_fire() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(42);
    let ids = ids(&manager, Type38Row::Type38);
    activate(&mut manager, &ids);
    let [x, y, z] = entity(&manager, ids[0]).position_raw();
    manager.player_mut().unwrap().set_position_raw([
        x.wrapping_add(0x300),
        y.wrapping_add(0x200),
        z,
    ]);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type38_family(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut fired = 0;
    for t in 1..=1500u32 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_no_block(&pass, &format!("tick{t}"));
        for &id in &ids {
            let queued = entity(&manager, id)
                .native_type38_aim_runtime
                .as_ref()
                .map_or(0, |runtime| runtime.queued_shot_count());
            if queued == 0 {
                continue;
            }
            let outcome = aim::drain_type38_family_shots(
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
    assert!(fired > 0, "no Type38 fired within 30 s of a nearby player");
}

fn lethal_hit(id: u32) -> ParticleEntityImpact {
    ParticleEntityImpact {
        source_particle_class: 16,
        impact_position_argument_va: 0,
        target_entity_id: id,
        position_world: [0.0; 3],
        velocity_raw: [0, 0, 8192],
        damage: Some(BallisticDamageRequest {
            packet: DamagePacket {
                channels: [2, 0],
                amounts_raw: [1_000_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

/// Lethal Playing hits run each row's alternate on the shared terminal:
/// Type38's BAC0 clears every task; Type129's BC90 keeps them and drops the
/// Type61 authored in its +88 at the carrier.
#[v2k_test_support::retail_test]
fn lethal_playing_hits_run_class1_and_class63() {
    for (level, row) in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids = ids(&manager, row);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type38_family(&manager), ids.len());
        let id = ids[0];
        let payload = entity(&manager, id).auto_pilot_payload_packed;
        let position = entity(&manager, id).position_raw();
        let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut hull = PlayerHull::default();
        let outcome = apply_playing_actor_particle_hit(
            PlayingActorImpactFrame {
                extra_lives: RetailRuntimeValue::Known(3),
                resources: &mut session.cache,
                entities: &mut manager,
                world_fx: &mut fx,
                scheduler: &mut tasks,
                notifications: &mut notifications,
                static_damage: &mut static_damage,
                player_hull: &mut hull,
                retail_tick: 600,
            },
            lethal_hit(id),
        );
        assert!(
            matches!(
                &outcome,
                Some(SharedActorImpactOutcome::Type38Family(
                    impact::Type38ImpactOutcome::Applied(applied)
                )) if applied.death_publication.is_none()
            ),
            "world{level}: {outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        let primary = entity(&manager, id)
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        match row {
            Type38Row::Type38 => {
                assert_eq!(style(&manager, id), 0x004c_7150);
                assert!(primary.is_none(), "BAC0 runs A860");
            }
            Type38Row::Type129 => {
                assert_eq!(style(&manager, id), 0x004c_7198);
                assert!(primary.is_some(), "BC90 never calls A860");
                let drop = manager
                    .iter_all()
                    .find(|entity| entity.id > newest && entity.entity_type == 61)
                    .expect("Type61 drop");
                assert_eq!(drop.position_raw(), position);
                assert_eq!(drop.power_up_payload_packed, payload);
            }
        }
        // The corpse stays in the late static walk until 14990.
        let outcome = contact::resolve_type38_family_static_contact(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 601,
                actor_tasks: &mut tasks,
            },
            id,
            Some(crate::native_actor_capture::pair::PlayingPlayerContact {
                hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
            }),
        );
        assert!(
            !matches!(outcome, contact::Type38ContactOutcome::Blocked { .. }),
            "world{level}: {outcome:?}"
        );
    }
}

/// A nonlethal hit runs C690: AC60 republishes class5 or class7 and the
/// scheduler keeps the new graph's owner.
#[v2k_test_support::retail_test]
fn a_nonlethal_hit_reselects_the_type38_graph() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(42);
    let ids = ids(&manager, Type38Row::Type38);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_type38_family(&manager), ids.len());
    let id = ids[0];
    let mut hit = lethal_hit(id);
    hit.damage.as_mut().unwrap().packet.amounts_raw = [12_000, 0];
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = PlayerHull::default();
    let outcome = apply_playing_actor_particle_hit(
        PlayingActorImpactFrame {
            extra_lives: RetailRuntimeValue::Known(3),
            resources: &mut session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            scheduler: &mut tasks,
            notifications: &mut notifications,
            static_damage: &mut static_damage,
            player_hull: &mut hull,
            retail_tick: 600,
        },
        hit,
    );
    assert!(
        matches!(
            &outcome,
            Some(SharedActorImpactOutcome::Type38Family(
                impact::Type38ImpactOutcome::Applied(applied)
            )) if applied.death_publication.is_none()
        ),
        "{outcome:?}"
    );
    assert!(matches!(entity(&manager, id).collision.health_raw,
        RetailRuntimeValue::Known(health) if health > 0 && health < HEALTH));
    assert!(matches!(style(&manager, id), 0x004c_7930 | 0x004c_7a50));
    for t in 601..=700 {
        let pass = tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        assert_no_block(&pass, &format!("tick{t}"));
    }
}

#[test]
fn rows_differ_in_four_words() {
    assert_eq!(
        Type38Row::Type129.emitter(),
        ProjectileEmitterDescriptor {
            sound_id: 0,
            ..EMITTER
        }
    );
    assert_eq!(Type38Row::Type38.target_warning_sound(), Some(87));
    assert_eq!(Type38Row::Type129.target_warning_sound(), None);
    assert_eq!(Type38Row::Type38.axis().raw_word_at_0x04, 3109);
    assert_eq!(Type38Row::Type129.axis().raw_word_at_0x04, 3073);
    assert_eq!(Type38Row::Type38.alternate_behavior_class(), 1);
    assert_eq!(Type38Row::Type129.alternate_behavior_class(), 63);
}
