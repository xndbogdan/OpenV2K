//! Ordinary Type92/99/102 births share Type97's 104B0 -> D4A0/D190 E/L owner.

use super::authored_tests::fixture;
use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime, actor_task_owner::ActorTaskSlot,
    entity_collision_state::RetailRuntimeValue, world_fx::ParticleEnvironment,
};

const WORLDS: [(u32, u32); 12] = [
    (26, 92),
    (38, 92),
    (39, 92),
    (26, 99),
    (32, 99),
    (46, 99),
    (48, 99),
    (19, 102),
    (25, 102),
    (37, 102),
    (39, 102),
    (40, 102),
];

#[v2k_test_support::retail_test]
fn ordinary_type92_99_102_births_publish_their_own_native_class29_owner() {
    let mut births = 0;
    for (level, entity_type) in WORLDS {
        let (_, manager, _) = fixture(level);
        let profile = Intro2GunTurretProfile::for_native_authored(entity_type).unwrap();
        let actors: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == entity_type)
            .collect();
        assert!(!actors.is_empty(), "world{level} Type{entity_type}");
        for entity in actors {
            births += 1;
            let runtime = entity.intro2_gun_turret_runtime.unwrap();
            assert_eq!(runtime.profile, profile);
            assert!(matches!(
                runtime.origin,
                GunTurretConstructionOrigin::NativeOrdinary(_)
            ));
            assert!(is_native_ordinary_gun_turret(entity));
            assert!(Intro2GunTurretOwner::adopt(&manager, entity.id).is_ok());
            assert_eq!(
                entity.model_slots,
                [Some(profile.model()); 4],
                "world{level} Type{entity_type}"
            );
            assert_eq!(
                entity.collision.health_raw,
                RetailRuntimeValue::Known(profile.health())
            );
            assert!(matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::Intro2GunTurret(_))
            ));
            assert_eq!(
                runtime.sub_e_runtime.projectile_method,
                profile.emitter().projectile_method
            );
        }
    }
    assert_eq!(births, 3 + 10 + 9);
}

#[v2k_test_support::retail_test]
fn ordinary_type92_99_102_run_their_turret_owner_for_64_ticks() {
    for (level, entity_type) in WORLDS {
        let (mut session, mut manager, mut fx) = fixture(level);
        let ids: Vec<_> = manager
            .iter_all()
            .filter(|e| e.entity_type == entity_type)
            .map(|e| e.id)
            .collect();
        for id in ids {
            let basis = manager.entity_mut(id).unwrap().physical_body_basis_q31();
            let mut owner = Intro2GunTurretOwner::adopt(&manager, id).unwrap();
            for tick in 1..=64 {
                let result = tick_intro2_gun_turret(
                    &mut manager,
                    owner,
                    Intro2GunTurretFrame {
                        notification_phase:
                            crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                        notifications:
                            &mut crate::gameplay_notifications::GameplayNotifications::default(),
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        elapsed_micros: 20_000,
                        retail_tick: tick,
                    },
                );
                assert!(
                    matches!(
                        result.outcome,
                        Intro2GunTurretOutcome::Waiting { .. }
                            | Intro2GunTurretOutcome::Advanced { .. }
                    ),
                    "world{level} Type{entity_type} id{id} tick{tick}: {:?}",
                    result.outcome
                );
                owner = result.retained_owner.expect("ordinary native living owner");
            }
            assert_eq!(
                manager.entity_mut(id).unwrap().physical_body_basis_q31(),
                basis
            );
        }
    }
}

#[v2k_test_support::retail_test]
fn ordinary_type92_99_102_fire_their_own_emitter_method_and_drain_once() {
    for (level, entity_type) in [(26, 92), (26, 99), (19, 102)] {
        let mut notifications = crate::gameplay_notifications::GameplayNotifications::default();
        let (session, mut manager, mut fx) = fixture(level);
        let profile = Intro2GunTurretProfile::for_native_authored(entity_type).unwrap();
        let method = profile.emitter().projectile_method;
        let class = crate::projectile_emitter::projectile_class_row(method)
            .unwrap()
            .particle_class;
        let id = manager
            .iter_all()
            .find(|e| e.entity_type == entity_type)
            .unwrap()
            .id;
        let result = aim::tick_intro2_gun_turret_aim(
            &mut manager,
            id,
            Some([0, 0, 1024]),
            aim::Intro2GunTurretAimFrame {
                global_elapsed_micros: 400_000,
                world_fx: &mut fx,
                notifications: &mut notifications,
                notification_phase:
                    crate::gameplay_notifications::GameplayNotificationPhase::Playing,
                retail_tick: 75,
            },
        )
        .unwrap();
        assert!(result.queued_shots > 0, "world{level} Type{entity_type}");
        let entity = manager.entity_mut(id).unwrap();
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x0200_0000, 0x0200_0000);
        let queue = entity.intro2_gun_turret_aim_runtime.as_ref().unwrap();
        assert_eq!(queue.projectile_descriptor(), profile.emitter());
        for command in queue.transient_shots() {
            assert_eq!(u32::from(command.projectile_method), method);
        }
        let drained = aim::drain_intro2_gun_turret_shots(
            &mut manager,
            &mut fx,
            id,
            &session.cache,
            ParticleEnvironment::Dry,
            1,
        )
        .unwrap();
        assert_eq!(drained.consumed_requests, result.queued_shots);
        assert!(drained
            .materialized_particle_classes
            .iter()
            .all(|&materialized| materialized == class));
        let again = aim::drain_intro2_gun_turret_shots(
            &mut manager,
            &mut fx,
            id,
            &session.cache,
            ParticleEnvironment::Dry,
            2,
        )
        .unwrap();
        assert_eq!(again.consumed_requests, 0);
    }
}
