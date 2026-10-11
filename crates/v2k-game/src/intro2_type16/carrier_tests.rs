//! Ordinary Type128 power-up carriers on the Type16 owner, with class63.

use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
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

/// Every authored Type128: worlds 25 and 28.
const WORLDS: [u32; 2] = [25, 28];

fn carrier_ids(manager: &EntityManager) -> Vec<u32> {
    manager
        .iter_all()
        .filter(|entity| entity.entity_type == 128)
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
                amounts_raw: [100_000, 0],
            },
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(35),
        }),
    }
}

#[v2k_test_support::retail_test]
fn type128_carriers_publish_their_row_on_the_type16_owner() {
    let mut births = 0;
    for level in WORLDS {
        let (session, mut manager, _) = fixture(level);
        let ids = carrier_ids(&manager);
        assert!(!ids.is_empty(), "world{level}");
        let level_desc = session.cache.level_desc().unwrap();
        for &id in &ids {
            births += 1;
            let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
            assert!(type16_manager_allocation_authenticates(&manager, id));
            assert_eq!(type16_row(entity), Some(Type16Row::Type128));
            assert_eq!(type16_auto_pilot_row(entity), Some(Type16Row::Type128));
            let runtime = entity.intro2_type16_runtime.unwrap();
            assert_eq!(
                runtime.ordinary_allocation.map(|lease| lease.entity_id),
                Some(id)
            );
            // 24E30 copies the row's silent emitter into Sub-E.
            assert_eq!(runtime.sub_e_runtime.sound_id, 0);
            let spawn = &level_desc.entities[entity.authored_spawn_index.unwrap()];
            assert_eq!(
                entity.auto_pilot_payload_packed,
                Some(u32::from_le_bytes(spawn.extra[8..12].try_into().unwrap()))
            );
            assert!(matches!(
                entity.initial_behavior,
                RetailRuntimeValue::Known(Some(selection))
                    if matches!(selection.program.class_id, 4 | 5 | 7 | 9)
            ));
            assert!(Intro2Type16Owner::adopt(&manager, id).is_ok());
            assert!(matches!(
                crate::class49_death::source_profile(entity),
                Some(
                    crate::class49_death::NativeExplosionSourceProfile::Type16Carrier(
                        Type16Row::Type128
                    )
                )
            ));
            let metadata = manager.type_runtime_metadata(128).cloned();
            assert_eq!(
                aim::ensure_intro2_type16_aim_runtime(
                    manager.entity_mut(id).unwrap(),
                    metadata.as_ref()
                ),
                Ok(())
            );
        }
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(
            tasks.adopt_intro2_type16(&manager) >= ids.len(),
            "world{level}"
        );
    }
    assert_eq!(births, 22);
}

/// Five seconds under the real scheduler and the late static walk.
#[v2k_test_support::retail_test]
fn type128_cohorts_run_five_seconds_with_late_static_contact() {
    for level in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids = carrier_ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_type16(&manager) >= ids.len());
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
            assert!(
                pass.block.is_none(),
                "world{level} tick{tick}: {:?}",
                pass.block
            );
            for outcome in &pass.outcomes {
                if let SpecializedActorTaskProductionOutcome::Intro2Type16(outcome) = outcome {
                    assert!(
                        !matches!(
                            outcome,
                            Intro2Type16Outcome::Blocked { .. }
                                | Intro2Type16Outcome::Pending { .. }
                        ),
                        "world{level} tick{tick}: {outcome:?}"
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
                    "world{level} tick{tick} id{id}: {outcome:?}"
                );
            }
        }
    }
}

/// A lethal Playing hit runs class63 in place of Flip Over And Die: the
/// carrier keeps its tasks, Playing's radial completes, and BC90 appends the
/// Type61 authored in its +88. Its corpse then still takes the static walk.
#[v2k_test_support::retail_test]
fn a_lethal_playing_hit_drops_the_type128_power_up() {
    for level in WORLDS {
        let (mut session, mut manager, mut fx) =
            crate::native_type122::construction_tests::native_fixture_with_player(level);
        manager.cleanup_pending_actor_deferred_destroys();
        let ids = carrier_ids(&manager);
        activate(&mut manager, &ids);
        let mut tasks = SpecializedActorTaskScheduler::new();
        assert!(tasks.adopt_intro2_type16(&manager) >= ids.len());
        // Pick a carrier whose living style has a null +2C (not carrying).
        let id = ids
            .iter()
            .copied()
            .find(|&id| {
                manager.iter_all().any(|entity| {
                    entity.id == id
                        && matches!(entity.current_behavior_context,
                            RetailRuntimeValue::Known(Some(context))
                                if !matches!(context.active_style().style_address(),
                                    0x004c_8080 | 0x004c_80c8 | 0x004c_8110 | 0x004c_8158))
                })
            })
            .expect("a non-carrying Type128");
        let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
        let payload = entity.auto_pilot_payload_packed.unwrap();
        let position = entity.position_raw();
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
                Some(SharedActorImpactOutcome::Type16(
                    crate::intro2_type16::impact::Intro2Type16ImpactOutcome::Applied(applied)
                )) if applied.death_publication.is_none()
            ),
            "world{level}: {outcome:?}"
        );
        assert!(crate::class49_death::finished_terminal_hit_authenticates(
            &manager, id
        ));
        assert!(manager.pending_actor_deferred_destroy_ids().contains(&id));
        assert!(
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .unwrap()
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .is_some(),
            "BC90 never calls A860"
        );
        let drop = manager
            .iter_all()
            .find(|entity| entity.id > newest && entity.entity_type == 61)
            .expect("Type61 drop");
        assert_eq!(drop.position_raw(), position);
        assert_eq!(drop.power_up_payload_packed, Some(payload));
        assert!(crate::native_type61::allocation_authenticates(
            &manager, drop.id
        ));
        // 11AD0 has no dying test: the corpse's retained Primary still takes
        // A8B0's static hook until 14990, without a custody block.
        let outcome = resolve_insect_static_contact(
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
        );
        assert!(
            !matches!(outcome, NativeGroundContactOutcome::Blocked { .. }),
            "world{level}: {outcome:?}"
        );
        // The ground solid/water kernel admits only the class12 corpse; the
        // class63 corpse stays ineligible there, as the living body is.
        let surface = crate::native_actor_surface_contact::resolve_native_actor_surface_contact(
            &mut Intro2ContactFrame {
                entities: &mut manager,
                resources: &mut session.cache,
                world_fx: &mut fx,
                static_damage: &mut static_damage,
                notifications: &mut notifications,
                retail_tick: 602,
                actor_tasks: &mut tasks,
            },
            id,
        );
        assert!(
            matches!(
                surface,
                crate::native_actor_surface_contact::NativeActorSurfaceContactOutcome::Ineligible
            ),
            "world{level}: {surface:?}"
        );
    }
}
