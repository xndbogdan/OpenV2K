//! Ordinary-world Type13 births share the Intro2 owner through their own receipt.

use super::*;
use crate::{
    damage::DamagePacket,
    entity::EntityManager,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_flyer_contacts::{resolve_native_flyer_contacts, Intro2FlyerContactOutcome},
    intro2_gun_turret::authored_tests::fixture,
    native_ground_actor::contact::NativeGroundContactOutcome,
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

/// Authored Type13 births per ordinary world: all model291, param1.
const BIRTHS: [(u32, usize); 8] = [
    (24, 1),
    (26, 3),
    (31, 2),
    (40, 3),
    (42, 2),
    (46, 4),
    (47, 2),
    (48, 3),
];

fn type13_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == TYPE13_ENTITY_TYPE)
        .map(|entity| entity.id)
        .collect()
}

#[v2k_test_support::retail_test]
fn ordinary_type13_births_publish_their_native_owner() {
    let mut births = 0;
    for (level, count) in BIRTHS {
        let (_, manager, _) = fixture(level);
        let ids = type13_ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            births += 1;
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(type13_manager_allocation_authenticates(&manager, id));
            assert_eq!(
                entity.native_type13_allocation.map(|lease| lease.entity_id),
                Some(id)
            );
            assert_eq!(
                authenticate_intro2_type13(entity).map(|admission| admission.spawn_index),
                Ok(entity.authored_spawn_index.unwrap())
            );
            assert!(matches!(
                entity.initial_behavior,
                RetailRuntimeValue::Known(Some(selection))
                    if matches!(selection.program.class_id, 5 | 7)
            ));
            assert!(entity.intro2_type13_common_mover_runtime.is_some());
            assert_eq!(
                entity.collision.pair_callbacks.type_hit_callback_address,
                RetailRuntimeValue::Known(None)
            );
            assert!(
                Intro2Type13WorldOwner::adopt_entity(&manager, id).is_ok(),
                "world{level}"
            );
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(
            tasks.adopt_intro2_type13_search_attack(&manager),
            count,
            "world{level}"
        );
        // A second pass adopts nothing: each allocation has one owner.
        assert_eq!(tasks.adopt_intro2_type13_search_attack(&manager), 0);
    }
    assert_eq!(births, 20);
}

#[v2k_test_support::retail_test]
fn a_receipt_never_borrows_the_intro2_spawn0_identity() {
    let (_, mut manager, _) = fixture(24);
    let id = type13_ids(&manager)[0];
    let foreign = manager
        .iter_all()
        .find(|entity| entity.id != id)
        .map(|entity| entity.id)
        .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let mut lease = entity.native_type13_allocation.unwrap();
    lease.entity_id = foreign;
    entity.native_type13_allocation = Some(lease);
    entity.authored_spawn_index = Some(INTRO2_TYPE13_SPAWN_INDEX);
    assert!(authenticate_intro2_type13(manager.entity_mut(id).unwrap()).is_err());
    assert!(!type13_manager_allocation_authenticates(&manager, id));
    assert!(Intro2Type13WorldOwner::adopt_entity(&manager, id).is_err());
}

#[v2k_test_support::retail_test]
fn ordinary_type13_cohorts_fly_five_seconds_with_late_contacts() {
    for (level, _) in BIRTHS {
        run_cohort(level, None);
    }
}

/// Retail EC60 reads the current wind for effective drag bit8. World46's
/// wind acts only below its sea (+80 = 0), so its above-sea cohort takes the
/// plain drag fallback exactly; world47's mode-2 gusts act above the sea.
#[v2k_test_support::retail_test]
fn windy_worlds_move_type13_through_ec60() {
    assert_eq!(run_cohort(46, None), run_cohort(46, Some(0)));
    assert_ne!(run_cohort(47, None), run_cohort(47, Some(0)));
}

fn run_cohort(level: u32, wind_mode: Option<u32>) -> Vec<[i16; 3]> {
    let (mut session, mut manager, mut fx) = fixture(level);
    if let Some(mode) = wind_mode {
        manager.set_runtime_wind_mode_for_test(mode);
    }
    let ids = type13_ids(&manager);
    // Near-player activation: detailed callbacks and contact eligibility.
    for &id in &ids {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
    }
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type13_search_attack(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    for tick in 1..=250u32 {
        manager.advance_environment_frame(&mut fx, 20_000);
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
        assert!(
            pass.block.is_none(),
            "world{level} tick{tick}: {:?}",
            pass.block
        );
        for outcome in &pass.outcomes {
            if let SpecializedActorTaskProductionOutcome::Intro2Type13SearchAttack(outcome) =
                outcome
            {
                assert!(
                    !matches!(
                        outcome,
                        Intro2Type13WorldOutcome::Blocked { .. }
                            | Intro2Type13WorldOutcome::Dropped { .. }
                            | Intro2Type13WorldOutcome::IncompletePending { .. }
                    ),
                    "world{level} tick{tick}: {outcome:?}"
                );
            }
        }
        for &id in &ids {
            let outcome = resolve_native_flyer_contacts(
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
                !matches!(outcome.surface, Intro2FlyerContactOutcome::Blocked { .. })
                    && !matches!(
                        outcome.static_contact,
                        Some(NativeGroundContactOutcome::Blocked { .. })
                    ),
                "world{level} tick{tick} id{id}: {outcome:?}"
            );
        }
    }
    ids.iter()
        .map(|&id| {
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .position_raw()
        })
        .collect()
}

#[v2k_test_support::retail_test]
fn a_playing_primary_hit_reaches_the_ordinary_type13_owner() {
    let (mut session, mut manager, mut fx) = fixture(26);
    let id = type13_ids(&manager)[0];
    let mut tasks = SpecializedActorTaskScheduler::new();
    tasks.adopt_intro2_type13_search_attack(&manager);
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
    let Some(SharedActorImpactOutcome::Type13(impact::Intro2Type13ImpactOutcome::Applied(applied))) =
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
