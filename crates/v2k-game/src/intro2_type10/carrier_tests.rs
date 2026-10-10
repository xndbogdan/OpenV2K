//! Ordinary Type80/126 power-up carriers on the Type10 owner, with class63.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    damage::DamagePacket,
    entity::EntityManager,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    intro2_contacts::Intro2ContactFrame,
    intro2_flyer_contacts::resolve_native_flyer_contacts,
    intro2_gun_turret::authored_tests::fixture,
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

/// Authored carriers: three Type80 in world 39, four Type126 in world 25.
const BIRTHS: [(u32, u32, Type10Profile, usize); 2] = [
    (39, 80, Type10Profile::Type80, 3),
    (25, 126, Type10Profile::Type126, 4),
];

fn carrier_ids(manager: &EntityManager, entity_type: u32) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == entity_type)
        .map(|entity| entity.id)
        .collect()
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
fn carriers_publish_their_rows_on_the_type10_owner() {
    for (level, entity_type, profile, count) in BIRTHS {
        let (session, manager, _) = fixture(level);
        let ids = carrier_ids(&manager, entity_type);
        assert_eq!(ids.len(), count, "world{level}");
        let level_desc = session.cache.level_desc().unwrap();
        for &id in &ids {
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(type10_manager_allocation_authenticates(&manager, id));
            assert_eq!(type10_profile(entity), Some(profile));
            assert_eq!(type10_auto_pilot_profile(entity), Some(profile));
            assert_eq!(profile.alternate_behavior_class(), 63);
            let runtime = entity.intro2_type10_runtime.unwrap();
            assert_eq!(
                runtime.ordinary_allocation.map(|lease| lease.entity_id),
                Some(id)
            );
            let spawn = &level_desc.entities[entity.authored_spawn_index.unwrap()];
            assert_eq!(
                entity.auto_pilot_payload_packed,
                Some(u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap()))
            );
            assert!(matches!(
                entity.initial_behavior,
                RetailRuntimeValue::Known(Some(selection)) if selection.program.class_id == 7
            ));
            assert!(Intro2Type10Owner::adopt(&manager, id).is_ok());
            assert!(matches!(
                crate::class49_death::source_profile(entity),
                Some(crate::class49_death::NativeExplosionSourceProfile::Type10Carrier(row))
                    if row == profile
            ));
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), count, "world{level}");
    }
}

/// Five seconds under the real scheduler and late contact walk: every carrier
/// hunts and flies without a block.
#[v2k_test_support::retail_test]
fn carrier_cohorts_fly_five_seconds_with_late_contacts() {
    for (level, entity_type, _, _) in BIRTHS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = carrier_ids(&manager, entity_type);
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
                    global_elapsed_micros: t * 20_000,
                    retail_tick: t,
                    main_base_abort_active: false,
                },
                &mut notifications,
            );
            assert!(
                pass.block.is_none(),
                "world{level} tick{t}: {:?}",
                pass.block
            );
            for outcome in &pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::Intro2Type10(outcome) = outcome {
                    assert!(
                        matches!(
                            outcome,
                            Intro2Type10Outcome::Waiting { .. }
                                | Intro2Type10Outcome::Advanced { .. }
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
            "world{level}: every carrier flies"
        );
    }
}

/// A lethal Playing hit runs class63 in place of Tumble: the carrier keeps
/// its Search tasks, Playing's radial completes, and BC90 appends the Type61
/// authored in its +88 at its position. The shared frame holds it instead.
#[v2k_test_support::retail_test]
fn a_lethal_playing_hit_drops_the_carrier_power_up() {
    for (level, entity_type, _, _) in BIRTHS {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids = carrier_ids(&manager, entity_type);
        activate(&mut manager, &ids);
        let id = ids[0];
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let payload = entity.auto_pilot_payload_packed.unwrap();
        let position = entity.position_raw();
        let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
        let mut static_damage = StaticDamageScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut hull = PlayerHull::default();
        let mut hit = |manager: &mut EntityManager,
                       session: &mut crate::session::GameSession,
                       fx: &mut crate::world_fx::WorldFx,
                       tasks: &mut SpecializedActorTaskScheduler| {
            apply_playing_actor_particle_hit(
                PlayingActorImpactFrame {
                    extra_lives: RetailRuntimeValue::Known(3),
                    resources: &mut session.cache,
                    entities: manager,
                    world_fx: fx,
                    scheduler: tasks,
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
            )
        };
        let outcome = hit(&mut manager, &mut session, &mut fx, &mut tasks);
        assert!(
            matches!(
                &outcome,
                Some(SharedActorImpactOutcome::Type10(
                    crate::intro2_type10::impact::Intro2Type10ImpactOutcome::Applied(applied)
                )) if applied.death_publication.is_none()
            ),
            "world{level}: {outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        // The hit's C690 prefix republished Primary before 15040; BC90 keeps
        // that graph because it never calls A860.
        assert!(
            entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .is_some(),
            "BC90 never calls A860"
        );
        assert!(!tasks.intro2_type10_completed_owner(&manager, id));
        let drop = manager
            .iter_all()
            .find(|entity| entity.id > newest && entity.entity_type == 61)
            .expect("Type61 drop");
        assert_eq!(drop.position_raw(), position);
        assert_eq!(drop.power_up_payload_packed, Some(payload));
        assert!(crate::native_type61::allocation_authenticates(
            &manager, drop.id
        ));
        // Before 14990 the corpse stays hittable: class63's +28 is null and
        // 10C10 is already dying, so a re-hit drops nothing more.
        let outcome = hit(&mut manager, &mut session, &mut fx, &mut tasks);
        assert!(
            matches!(
                &outcome,
                Some(SharedActorImpactOutcome::Type10(
                    crate::intro2_type10::impact::Intro2Type10ImpactOutcome::Applied(applied)
                )) if applied.death_publication.is_none()
            ),
            "world{level} re-hit: {outcome:?}"
        );
        assert_eq!(
            manager
                .iter_all()
                .filter(|entity| entity.id > newest && entity.entity_type == 61)
                .count(),
            1
        );
    }
}

#[v2k_test_support::retail_test]
fn the_shared_hit_frame_holds_a_lethal_carrier_hit() {
    let (session, mut manager, mut fx) = fixture(39);
    let ids = carrier_ids(&manager, 80);
    activate(&mut manager, &ids);
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert_eq!(tasks.adopt_intro2_type10(&manager), ids.len());
    let outcome = crate::intro2_type10::impact::apply_intro2_type10_particle_hit(
        &mut manager,
        &session.cache,
        &mut fx,
        &mut tasks,
        ParticleEntityImpact {
            source_particle_class: 16,
            impact_position_argument_va: 0,
            target_entity_id: ids[0],
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
        600,
    );
    let crate::intro2_type10::impact::Intro2Type10ImpactOutcome::Blocked {
        reason: crate::intro2_type10::impact::Intro2Type10ImpactBlock::Damage(error),
        committed_prefix: true,
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(
        error.reason,
        crate::live_actor_checked_damage::LiveActorDamageBlock::Death(
            crate::intro2_type10::death::Intro2Type10DeathBlock::AutoPilotCarrier
        )
    );
}

/// A lethal 11AD0 crash with the lent player runs class63 inside the contact
/// walk. The corpse keeps class63's null solid/water hooks and its Search
/// tasks until 14990, so a second walk before the sweep still admits it.
#[v2k_test_support::retail_test]
fn a_lethal_playing_crash_runs_class63_and_the_corpse_stays_contactable() {
    use crate::intro2_flyer_contacts::resolve_native_flyer_contacts_with_playing;
    use crate::native_actor_capture::pair::PlayingPlayerContact;
    let (mut session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(39);
    manager.cleanup_pending_actor_deferred_destroys();
    let id = carrier_ids(&manager, 80)[0];
    for cell in &mut session.cache.level_terrain_mut().unwrap().cells {
        cell.height = 0;
    }
    let payload = {
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0202_8000, 0x0202_8000);
        entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        let [x, _, z] = entity.position_raw();
        entity.set_position_raw([x, -50, z]);
        entity.set_velocity_raw([0, -12000, 0]);
        entity.collision.health_raw = RetailRuntimeValue::Known(1);
        entity.auto_pilot_payload_packed.unwrap()
    };
    let newest = manager.iter_all().map(|entity| entity.id).max().unwrap();
    let mut tasks = SpecializedActorTaskScheduler::new();
    assert!(tasks.adopt_intro2_type10(&manager) > 0);
    let mut static_damage = StaticDamageScheduler::new();
    let mut notifications = GameplayNotifications::new();
    let mut hull = PlayerHull::default();
    for walk in 0..2 {
        let outcome = resolve_native_flyer_contacts_with_playing(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 600 + walk,
                actor_tasks: &mut tasks,
            },
            id,
            Some(PlayingPlayerContact {
                hull: &mut hull,
                extra_lives: RetailRuntimeValue::Known(3),
            }),
        );
        assert!(!outcome.blocks_later_contacts(), "walk{walk}: {outcome:?}");
        if walk == 0 {
            assert!(
                matches!(
                    outcome.surface,
                    crate::intro2_flyer_contacts::Intro2FlyerContactOutcome::Applied {
                        solid_contact: true,
                        ..
                    }
                ),
                "{outcome:?}"
            );
        }
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
    }
    assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
    let drops: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.id > newest && entity.entity_type == 61)
        .collect();
    assert_eq!(drops.len(), 1, "one class63 drop");
    assert_eq!(drops[0].power_up_payload_packed, Some(payload));
}
