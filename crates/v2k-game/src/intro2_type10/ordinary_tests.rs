//! Ordinary-world Type5 births share the Type10 owner through their own profile.

use super::*;
use crate::{entity::EntityManager, intro2_gun_turret::authored_tests::fixture};

/// Authored Type5 births per ordinary world: all model1122, param1.
const BIRTHS: [(u32, usize); 3] = [(42, 2), (46, 2), (47, 2)];

fn type5_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == 5)
        .map(|entity| entity.id)
        .collect()
}

#[v2k_test_support::retail_test]
fn ordinary_type5_births_publish_the_type10_owner() {
    let mut births = 0;
    for (level, count) in BIRTHS {
        let (_, manager, _) = fixture(level);
        let ids = type5_ids(&manager);
        assert_eq!(ids.len(), count, "world{level}");
        for &id in &ids {
            births += 1;
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(type10_manager_allocation_authenticates(&manager, id));
            assert_eq!(type10_profile(entity), Some(Type10Profile::Type5));
            let runtime = entity.intro2_type10_runtime.unwrap();
            assert_eq!(
                runtime.ordinary_allocation.map(|lease| lease.entity_id),
                Some(id)
            );
            assert!(matches!(
                entity.initial_behavior,
                RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 7
            ));
            assert_eq!(
                entity.collision.pair_callbacks.type_hit_callback_address,
                RetailRuntimeValue::Known(None)
            );
            assert!(
                Intro2Type10Owner::adopt(&manager, id).is_ok(),
                "world{level}"
            );
        }
        let mut tasks =
            crate::specialized_actor_task_production::SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), count, "world{level}");
    }
    assert_eq!(births, 6);
}

use crate::{
    damage::DamagePacket,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_flyer_contacts::{
        resolve_native_flyer_contacts, resolve_native_flyer_contacts_with_playing,
    },
    native_actor_capture::pair::PlayingPlayerContact,
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

fn tick(
    tasks: &mut SpecializedActorTaskScheduler,
    manager: &mut EntityManager,
    session: &mut crate::session::GameSession,
    fx: &mut crate::world_fx::WorldFx,
    static_damage: &mut StaticDamageScheduler,
    notifications: &mut GameplayNotifications,
    tick: u32,
) -> Vec<SpecializedActorTaskProductionOutcome> {
    manager.advance_environment_frame(fx, 20_000);
    let pass = tasks.tick(
        manager,
        SpecializedActorTaskProductionFrame {
            world: SpecializedActorTaskWorld::Cinematic,
            hive_components: None,
            notification_phase: GameplayNotificationPhase::Playing,
            resources: &mut session.cache,
            world_fx: fx,
            static_damage,
            elapsed_micros: 20_000,
            global_elapsed_micros: tick * 20_000,
            retail_tick: tick,
            main_base_abort_active: false,
        },
        notifications,
    );
    assert!(pass.block.is_none(), "tick{tick}: {:?}", pass.block);
    pass.outcomes
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

#[v2k_test_support::retail_test]
fn ordinary_type5_cohorts_fly_five_seconds_with_late_contacts() {
    for (level, _) in BIRTHS {
        run_cohort(level, None);
    }
}

/// Effective-8 E100 calls EC60 with the current wind: world47's mode-2 gusts
/// above the sea change the flight relative to a forced calm run.
#[v2k_test_support::retail_test]
fn type5_flight_takes_ec60_wind_in_world47() {
    assert_ne!(run_cohort(47, None), run_cohort(47, Some(0)));
}

fn run_cohort(level: u32, wind_mode: Option<u32>) -> Vec<[i16; 3]> {
    let (mut session, mut manager, mut fx) = fixture(level);
    if let Some(mode) = wind_mode {
        manager.set_runtime_wind_mode_for_test(mode);
    }
    let ids = type5_ids(&manager);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let position = |manager: &EntityManager, id| {
        manager
            .iter_all()
            .find(|entity| entity.id == id)
            .unwrap()
            .position_raw()
    };
    let start: Vec<_> = ids.iter().map(|&id| position(&manager, id)).collect();
    for t in 1..=250u32 {
        for outcome in tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        ) {
            if let SpecializedActorTaskProductionOutcome::Intro2Type10(outcome) = &outcome {
                assert!(
                    matches!(
                        outcome,
                        Intro2Type10Outcome::Waiting { .. } | Intro2Type10Outcome::Advanced { .. }
                    ),
                    "world{level} tick{t}: {outcome:?}"
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
                    retail_tick: t,
                    actor_tasks: &mut tasks,
                },
                id,
            );
            assert!(
                !outcome.blocks_later_contacts(),
                "world{level} tick{t} id{id}: {outcome:?}"
            );
        }
    }
    let end: Vec<_> = ids.iter().map(|&id| position(&manager, id)).collect();
    assert!(
        start.iter().zip(&end).all(|(start, end)| start != end),
        "world{level}: every Type5 flies"
    );
    end
}

/// A lethal Playing hit publishes class11 Tumble. The falling body's own
/// contact then finishes C750 through Playing's radial with the lent player.
#[v2k_test_support::retail_test]
fn a_type5_killed_in_playing_tumbles_and_explodes_with_the_player_hull() {
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(46);
    let ids = type5_ids(&manager);
    activate(&mut manager, &ids);
    let id = ids[0];
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
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
        ParticleEntityImpact {
            source_particle_class: 16,
            impact_position_argument_va: 0,
            target_entity_id: id,
            position_world: [0.0; 3],
            velocity_raw: [0, 0, 8192],
            damage: Some(BallisticDamageRequest {
                packet: DamagePacket {
                    channels: [2, 0],
                    amounts_raw: [60_000, 0],
                },
                source_entity_type_at_birth: Some(34),
                source_owner_id: Some(35),
            }),
        },
    );
    assert!(
        matches!(
            &outcome,
            Some(SharedActorImpactOutcome::Type10(
                crate::intro2_type10::impact::Intro2Type10ImpactOutcome::Applied(applied)
            )) if applied.death_publication.is_some()
        ),
        "{outcome:?}"
    );
    let mut finalized = None;
    for t in 601..=2100u32 {
        tick(
            &mut tasks,
            &mut manager,
            &mut session,
            &mut fx,
            &mut static_damage,
            &mut notifications,
            t,
        );
        let outcome = resolve_native_flyer_contacts_with_playing(
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
            Some(PlayingPlayerContact {
                hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
            }),
        );
        assert!(!outcome.blocks_later_contacts(), "tick{t}: {outcome:?}");
        if manager.pending_actor_deferred_destroy_ids().contains(&id) {
            finalized = Some((t, outcome));
            break;
        }
    }
    let (_, outcome) = finalized.expect("the falling Type5 reaches terrain or water");
    if let Some(crate::intro2_type10::contact::Intro2Type10ContactOutcome::Applied(report)) =
        &outcome.tumble
    {
        if let Some(terminal) = &report.terminal {
            assert!(terminal.finalized);
            assert!(matches!(
                terminal.radial,
                crate::class49_terminal::Class49RadialReport::Playing { .. }
            ));
        }
    }
}
