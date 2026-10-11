//! Ordinary-world Type16 births share the Intro2 owner through their own receipt.

use super::*;
use crate::{
    damage::DamagePacket,
    entity::EntityManager,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_gun_turret::authored_tests::fixture,
    native_ground_actor::contact::{resolve_insect_static_contact, NativeGroundContactOutcome},
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

/// Authored Type16 births per ordinary world: all model257, param1.
const BIRTHS: [(u32, usize); 7] = [
    (18, 5),
    (20, 1),
    (21, 1),
    (24, 2),
    (31, 2),
    (35, 7),
    (37, 3),
];

fn type16_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == 16)
        .map(|entity| entity.id)
        .collect()
}

#[v2k_test_support::retail_test]
fn ordinary_type16_births_publish_their_native_owner() {
    let mut births = 0;
    for (level, count) in BIRTHS {
        let (_, manager, _) = fixture(level);
        let ids = type16_ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            births += 1;
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(type16_manager_allocation_authenticates(&manager, id));
            let runtime = entity.intro2_type16_runtime.unwrap();
            assert_eq!(
                runtime.ordinary_allocation.map(|lease| lease.entity_id),
                Some(id)
            );
            assert!(matches!(
                entity.initial_behavior,
                RetailRuntimeValue::Known(Some(selection))
                    if matches!(selection.program.class_id, 4 | 5 | 7 | 9)
            ));
            assert_eq!(
                entity.collision.pair_callbacks.type_hit_callback_address,
                RetailRuntimeValue::Known(None)
            );
            assert!(
                Intro2Type16Owner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type16(&manager), count, "world{level}");
    }
    assert_eq!(births, 21);
}

#[v2k_test_support::retail_test]
fn an_ordinary_type16_cohort_runs_five_seconds_with_late_static_contact() {
    let (mut session, mut manager, mut fx) = fixture(35);
    let ids = type16_ids(&manager);
    for &id in &ids {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type16(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    for tick in 1..=250u32 {
        let pass = tasks.tick(
            &mut manager,
            SpecializedActorTaskProductionFrame {
                world: SpecializedActorTaskWorld::Cinematic,
                hive_components: None,
                notification_phase: GameplayNotificationPhase::Playing,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                elapsed_micros: 20_000,
                global_elapsed_micros: tick * 20_000,
                retail_tick: tick,
                main_base_abort_active: false,
            },
            &mut notifications,
        );
        assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::Intro2Type16(outcome) = outcome {
                assert!(
                    !matches!(
                        outcome,
                        Intro2Type16Outcome::Blocked { .. } | Intro2Type16Outcome::Pending { .. }
                    ),
                    "tick{tick}: {outcome:?}"
                );
            }
        }
        for &id in &ids {
            let outcome = resolve_insect_static_contact(
                &mut Intro2ContactFrame {
                    entities: &mut manager,
                    resources: &mut session.cache,
                    world_fx: &mut fx,
                    static_damage: &mut static_damage,
                    notifications: &mut notifications,
                    retail_tick: tick,
                    actor_tasks: &mut tasks,
                },
                id,
            );
            assert!(
                !matches!(outcome, NativeGroundContactOutcome::Blocked { .. }),
                "tick{tick} id{id}: {outcome:?}"
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn a_playing_primary_hit_reaches_the_ordinary_type16_owner() {
    let (mut session, mut manager, mut fx) = fixture(35);
    let id = type16_ids(&manager)[0];
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_intro2_type16(&manager);
    let health = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .unwrap()
        .collision
        .health_raw;
    let outcome = apply_playing_actor_particle_hit(
        PlayingActorImpactFrame {
            extra_lives: RetailRuntimeValue::Unresolved,
            resources: &mut session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            scheduler: &mut tasks,
            notifications: &mut GameplayNotifications::new(),
            static_damage: &mut StaticDamageScheduler::new(),
            player_hull: &mut PlayerHull::default(),
            retail_tick: 600,
        },
        ParticleEntityImpact {
            source_particle_class: 16,
            impact_position_argument_va: 0,
            target_entity_id: id,
            position_world: [0.0; 3],
            velocity_raw: [0, 0, 8192],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [5000, 0],
                },
                source_entity_type_at_birth: Some(34),
                source_owner_id: Some(35),
            }),
        },
    );
    let Some(SharedActorImpactOutcome::Type16(impact::Intro2Type16ImpactOutcome::Applied(applied))) =
        outcome
    else {
        panic!("{outcome:?}")
    };
    assert!(applied.filtered_damage_raw > 0);
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    assert_ne!(entity.collision.health_raw, health);
    assert_eq!(
        entity.collision.last_hit_presentation_tick_at_0x34,
        RetailRuntimeValue::Known(600)
    );
}
