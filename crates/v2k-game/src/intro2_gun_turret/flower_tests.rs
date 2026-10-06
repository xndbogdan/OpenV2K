use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity_collision_state::RetailRuntimeValue,
    gameplay_notifications::{GameplayNotificationPhase, GameplayNotifications},
    world_fx::{ParticleEnvironment, WorldFx},
};
use native::tests::{fixture, generic};

#[v2k_test_support::retail_test]
fn authored_flower_selects_both_real_graphs_with_one_draw_and_its_own_components() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let profile = Intro2GunTurretProfile::Type115;
    native::authenticate_metadata(profile, &metadata[115]).unwrap();
    assert_eq!(profile.alternate_behavior_class(), 1);
    for (word, class) in [(0, 29), (0xffff, 0)] {
        let mut manager = generic(&session, &metadata);
        let id = manager
            .iter_all()
            .find(|e| e.entity_type == 115)
            .unwrap()
            .id;
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(entity.model_index, Some(332));
        let mut draws = 0;
        let publication = publish_intro2_gun_turret(
            entity,
            &metadata[115],
            session.cache.terrain().unwrap(),
            &mut || {
                draws += 1;
                word
            },
        )
        .unwrap();
        assert_eq!(draws, 1);
        assert_eq!(publication.selection.program.class_id, class);
        assert_eq!(entity.position_raw(), [-16384, 128, 2816]);
        assert_eq!(
            entity.model_slots,
            [Some(331), Some(144), Some(332), Some(144)]
        );
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(0x2000),
            RetailRuntimeValue::Known(0x2000)
        );
        let runtime = entity.intro2_gun_turret_runtime.unwrap();
        assert_eq!(runtime.sub_e_runtime.projectile_method, 16);
        assert_eq!(runtime.sub_e_runtime.joint_bindings, [None; 2]);
        if class == 29 {
            assert_eq!(entity.capability_flags, 8);
            assert_eq!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(&ActorTaskRuntime::Intro2GunTurret(
                    task::GunTurretTaskState::from_408df0(0, [0; 2], true)
                ))
            );
        } else {
            assert_eq!(entity.capability_flags, 0);
            assert!(task::idle_graph_authenticates(entity));
        }
        Intro2GunTurretOwner::adopt(&manager, id).unwrap();
    }
}

#[v2k_test_support::retail_test]
fn flower_wait_is_strict_and_reselection_preserves_component_cadence() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|e| e.entity_type == 115)
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    publish_intro2_gun_turret(
        entity,
        &metadata[115],
        session.cache.terrain().unwrap(),
        &mut || 0xffff,
    )
    .unwrap();
    let old_task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
    entity
        .intro2_gun_turret_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = 12345;
    assert_eq!(
        task::tick_flower_idle_task(entity, 9_000_999),
        Ok(task::TurretTaskOutcome::Continue)
    );
    assert_eq!(
        task::tick_flower_idle_task(entity, 999),
        Ok(task::TurretTaskOutcome::Continue)
    );
    assert_eq!(
        task::tick_flower_idle_task(entity, 1000),
        Ok(task::TurretTaskOutcome::RetargetRequired)
    );
    let result = native::reselect_intro2_gun_turret(entity, &metadata[115], &mut || 0).unwrap();
    assert_eq!(result.selection.program.class_id, 29);
    assert!(task::graph_authenticates(entity));
    assert_ne!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
        old_task
    );
    assert_eq!(entity.capability_flags, 8);
    assert_eq!(
        entity
            .intro2_gun_turret_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        12345
    );
    native::reselect_intro2_gun_turret(entity, &metadata[115], &mut || 0xffff).unwrap();
    assert!(task::idle_graph_authenticates(entity));
    assert_eq!(
        entity.capability_flags, 8,
        "C490 does not restore the original capability"
    );
}

#[v2k_test_support::retail_test]
fn flower_method16_queues_and_drains_real_class50_from_the_infected_model() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = manager
        .iter_all()
        .find(|e| e.entity_type == 115)
        .unwrap()
        .id;
    let entity = manager.entity_mut(id).unwrap();
    publish_intro2_gun_turret(
        entity,
        &metadata[115],
        session.cache.terrain().unwrap(),
        &mut || 0,
    )
    .unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x06000000, 0x06000000);
    let position = entity.position_raw();
    let mut fx = WorldFx::new();
    let mut notifications = GameplayNotifications::new();
    let outcome = aim::tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 1000, 2000]),
        aim::Intro2GunTurretAimFrame {
            world_fx: &mut fx,
            notifications: &mut notifications,
            notification_phase: GameplayNotificationPhase::NonGameplay,
            global_elapsed_micros: 400_000,
            retail_tick: 1173,
        },
    )
    .unwrap();
    assert_eq!(outcome.queued_shots, 2);
    let drained = aim::drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        1173,
    )
    .unwrap();
    assert_eq!(drained.materialized_particle_classes, [50, 50]);
    assert_eq!(drained.consumed_requests, 2);
    let shown = fx.prepare_presentation([640, 480], 0x3000, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [320, 240],
            depth_raw: 1000,
            clip: 0,
        }
    });
    let particles: Vec<_> = shown.particles().collect();
    assert_eq!(particles.len(), 2);
    assert!(particles.iter().all(|p| p.particle.source_class == 50));
    assert_ne!(
        particles[0].particle.position,
        position.map(|word| f32::from(word) / 256.0),
        "model332's slot26 muzzle must be visited"
    );
}
