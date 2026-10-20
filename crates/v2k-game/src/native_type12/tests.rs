//! Ordinary Type12 building-trashers and Type127 carriers on the shared host.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
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

/// Every authored row: Type127 in world 27, Type12 in worlds 31 and 35.
const WORLDS: [(u32, Type12Row); 3] = [
    (27, Type12Row::Type127),
    (31, Type12Row::Type12),
    (35, Type12Row::Type12),
];

fn ids(manager: &EntityManager, row: Type12Row) -> Vec<u32> {
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
        if let SpecializedActorTaskProductionOutcome::NativeType12Family(outcome) = outcome {
            assert!(
                !matches!(
                    outcome,
                    Type12Outcome::Blocked { .. } | Type12Outcome::Pending { .. }
                ),
                "{context}: {outcome:?}"
            );
        }
    }
}

/// The four living classes' installed styles.
const LIVING_STYLES: [u32; 5] = [
    0x004c_7b28, // Follow Beacons acquiring
    0x004c_7a50, // Search acquiring
    0x004c_7ff0, // Capture acquiring
    0x004c_76f0, // Trash Buildings
    0x004c_7b70, // Follow Beacons following
];

#[v2k_test_support::retail_test]
fn type12_family_births_publish_their_rows_on_the_ground_host() {
    let mut births = [0; 2];
    for (level, row) in WORLDS {
        let (session, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = ids(&manager, row);
        assert!(!ids.is_empty(), "world{level}");
        let level_desc = session.cache.level_desc().unwrap();
        for &id in &ids {
            births[usize::from(row == Type12Row::Type127)] += 1;
            let entity = entity(&manager, id);
            assert!(manager_allocation_authenticates(&manager, id));
            assert_eq!(type12_row(entity), Some(row));
            let runtime = entity.native_type12_runtime.as_ref().unwrap();
            assert_eq!(runtime.sub_e_runtime.projectile_method, 24);
            assert_eq!(runtime.sub_e_runtime.sound_id, 82);
            assert_eq!(
                runtime.kl_components.model_variables_raw().len(),
                MODEL_VARIABLES as usize
            );
            assert_eq!(
                entity.sub_j_attachment_runtime,
                RetailRuntimeValue::Known(None)
            );
            assert!(
                LIVING_STYLES.contains(&style(&manager, id)),
                "world{level} id{id}: {:#x}",
                style(&manager, id)
            );
            assert!(Type12FamilyOwner::adopt(&manager, id).is_ok());
            let profile = source_profile(entity).unwrap();
            assert_eq!(profile, NativeExplosionSourceProfile::Type12Family(row));
            assert_eq!(
                profile.policy(),
                match row {
                    Type12Row::Type12 => NativeExplosionPolicy::Class1,
                    Type12Row::Type127 => NativeExplosionPolicy::Class63,
                }
            );
            if row == Type12Row::Type127 {
                let spawn = &level_desc.entities[entity.authored_spawn_index.unwrap()];
                assert_eq!(
                    entity.auto_pilot_payload_packed,
                    Some(u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap()))
                );
            }
            // Class15's B7C0 hands the furniture task kind3.
            if let Some(ActorTaskRuntime::TrashFurniture(task)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            {
                assert_eq!(style(&manager, id), 0x004c_76f0);
                assert_eq!(task.kind_filter(), 3);
            }
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(
            tasks.adopt_type12_family(&manager),
            ids.len(),
            "world{level}"
        );
    }
    assert_eq!(births, [8, 25]);
}

/// Ten seconds under the real scheduler and the late static walk.
#[v2k_test_support::retail_test]
fn type12_family_cohorts_run_ten_seconds_with_late_static_contact() {
    for (level, row) in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::intro2_gun_turret::authored_tests::fixture(level);
        let ids = ids(&manager, row);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type12_family(&manager), ids.len());
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
                let outcome = contact::resolve_type12_family_static_contact(
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
                    !matches!(outcome, contact::Type12ContactOutcome::Blocked { .. }),
                    "world{level} tick{t} id{id}: {outcome:?}"
                );
            }
        }
        for &id in &ids {
            assert!(
                Type12FamilyOwner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
    }
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
/// Type12's BAC0 clears every task; Type127's BC90 keeps them and drops the
/// Type61 authored in its +88 at the carrier.
#[v2k_test_support::retail_test]
fn lethal_playing_hits_run_class1_and_class63() {
    for (level, row) in [(27, Type12Row::Type127), (31, Type12Row::Type12)] {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids = ids(&manager, row);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_type12_family(&manager), ids.len());
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
                Some(SharedActorImpactOutcome::Type12Family(
                    impact::Type12ImpactOutcome::Applied(applied)
                )) if applied.death_publication.is_none()
            ),
            "world{level}: {outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        match row {
            Type12Row::Type12 => {
                assert_eq!(style(&manager, id), 0x004c_7150);
                assert!(entity(&manager, id)
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .is_none());
            }
            Type12Row::Type127 => {
                assert_eq!(style(&manager, id), 0x004c_7198);
                let drop = manager
                    .iter_all()
                    .find(|entity| entity.id > newest && entity.entity_type == 61)
                    .expect("Type61 drop");
                assert_eq!(drop.position_raw(), position);
                assert_eq!(drop.power_up_payload_packed, payload);
            }
        }
        // The corpse stays in the late static walk until 14990.
        let outcome = contact::resolve_type12_family_static_contact(
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
            !matches!(outcome, contact::Type12ContactOutcome::Blocked { .. }),
            "world{level}: {outcome:?}"
        );
    }
}

/// Rule9 finds kind3 objects within the whole strict axis, and class15's
/// task scans for them with the same radius and kind.
#[v2k_test_support::retail_test]
fn trash_buildings_targets_kind3_objects() {
    for (level, row) in WORLDS {
        let (session, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        let terrain = session.cache.terrain().unwrap();
        let objects = session.cache.terrain_objects();
        for id in ids(&manager, row) {
            let entity = entity(&manager, id);
            let near =
                crate::native_ground_actor::behavior::buildings_nearby(entity, &session.cache)
                    .unwrap();
            let direct = crate::trash_furniture::find_furniture(
                terrain,
                objects,
                entity.position_raw(),
                row.axis().strict_axis_limit_raw,
                3,
            );
            assert_eq!(near, direct.is_some(), "world{level} id{id}");
            // A class15 birth holds the kind3 task whose scan found it.
            if style(&manager, id) == 0x004c_76f0 {
                assert!(near, "world{level} id{id}");
            }
        }
    }
}
