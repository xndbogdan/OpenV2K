use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    intro2_gun_turret::native::tests::{fixture, generic, publish},
    particle_descriptors::particle_descriptor,
};

fn identity_body() -> Type9BodyBasis {
    Type9BodyBasis {
        lateral: [i32::MAX, 0, 0],
        up: [0, i32::MAX, 0],
        forward: [0, 0, i32::MAX],
    }
}

fn presented(fx: &mut WorldFx) -> Vec<crate::world_fx::WorldParticle> {
    fx.prepare_presentation([640, 480], 0x3000, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [320, 240],
            depth_raw: 1000,
            clip: 0,
        }
    })
    .particles()
    .map(|p| p.particle)
    .collect()
}

#[v2k_test_support::retail_test]
fn turret_muzzle_walk_keeps_selected_face_group_stamp_and_last_callback_order() {
    let Some((session, _)) = fixture() else {
        return;
    };
    let mut walk = MuzzleWalk {
        descriptor: crate::intro2_gun_turret::EMITTER,
        resources: &session.cache,
        body: identity_body(),
        position: [0; 3],
        muzzles: [None; 2],
        ancestors: Vec::new(),
    };
    let mut vars = AnimVars::default();
    vars.registers[3] = 1;
    walk.visit(164, [10., 20., 30.], IDENTITY, None, &vars)
        .unwrap();
    assert_eq!(walk.muzzles, [Some([9, 33, 300]), Some([9, 33, 300])]);
    // C6 explicitly resolves44 after the selector0 face. The trailing38
    // diagnostic has no normal callback effect; the second instance wins.
    walk.visit(164, [100., 200., 300.], IDENTITY, None, &vars)
        .unwrap();
    assert_eq!(walk.muzzles, [Some([99, 213, 570]), Some([99, 213, 570])]);
    vars.registers[3] = 0;
    walk.visit(164, [-100., 0., 0.], IDENTITY, None, &vars)
        .unwrap();
    assert_eq!(walk.muzzles, [Some([99, 213, 570]), Some([-100, 13, 270])]);
}

#[v2k_test_support::retail_test]
fn turret_canonical_both_barrels_follow_live_mounts_and_retained_body() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    for spawn in [53, 54] {
        let mut manager = generic(&session, &metadata);
        let id = publish(&mut manager, &session, &metadata, spawn);
        let entity = manager.entity_mut(id).unwrap();
        entity.set_position_raw([32_740, -500, -32_740]);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(identity_body());
        // Independent scalar oracle for authored162->163->164: both barrel
        // attachments are +/-88, the quarter-turn mounts move muzzleY14
        // inward, root/pillar heights are55+124, muzzleZ is271. Production
        // reads all these points and instance transforms from the actual pool.
        for (yaw, pitch) in [(0_i16, 0_i16), (4096, 6144), (-8192, -4096)] {
            entity
                .intro2_gun_turret_runtime
                .as_mut()
                .unwrap()
                .sub_l_output_raw = [yaw, pitch];
            let result = resolve_muzzles(entity, &session.cache, 37).unwrap();
            let (sy, cy) = (f64::from(yaw) * std::f64::consts::TAU / 65536.).sin_cos();
            let (sp, cp) = (f64::from(pitch) * std::f64::consts::TAU / 65536.).sin_cos();
            for (selector, x) in [(0, 74.), (1, -74.)] {
                let local = [
                    x * cy + 271. * cp * sy,
                    179. - 271. * sp,
                    -x * sy + 271. * cp * cy,
                ];
                let expected =
                    local.map(|v| ((v as i32 as i64 * i64::from(i32::MAX)) >> 31) as i16);
                let actual = result[selector].unwrap();
                for axis in 0..3 {
                    let offset = actual[axis].wrapping_sub(entity.position_raw()[axis]);
                    // The existing intrinsic mount API uses floating trig;
                    // one raw unit at a truncation boundary is explicit.
                    assert!(i32::from(offset).abs_diff(i32::from(expected[axis])) <= 1,
                        "spawn{spawn} yaw{yaw} pitch{pitch} selector{selector}: {actual:?} vs {expected:?}");
                }
            }
            assert_ne!(result[0], result[1]);
        }
        entity
            .intro2_gun_turret_runtime
            .as_mut()
            .unwrap()
            .sub_l_output_raw = [0; 2];
        let original = resolve_muzzles(entity, &session.cache, 0).unwrap();
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(Type9BodyBasis {
            lateral: [0, 0, i32::MIN],
            up: [0, i32::MAX, 0],
            forward: [i32::MAX, 0, 0],
        });
        let rotated = resolve_muzzles(entity, &session.cache, 0).unwrap();
        assert_ne!(rotated, original);
        assert_eq!(
            rotated[0].unwrap()[0].wrapping_sub(entity.position_raw()[0]),
            270
        );
        assert_eq!(
            rotated[0].unwrap()[2].wrapping_sub(entity.position_raw()[2]),
            -74
        );
    }
}

#[v2k_test_support::retail_test]
fn turret_fifo_drains_stamped_muzzles_after_reselection_with_method14_time_and_velocity() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 300_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        IMPACT_SUPPRESSION_STATE_BIT | PRESENTATION_DETAIL_STATE_BIT,
        PRESENTATION_DETAIL_STATE_BIT,
    );
    entity.set_position_raw([200, 1000, 300]);
    entity.set_velocity_raw([300, -200, 400]);
    entity
        .intro2_gun_turret_runtime
        .as_mut()
        .unwrap()
        .sub_l_output_raw = [2345, -3456];
    super::super::super::reselect_intro2_gun_turret(entity, &metadata[102], &mut || 0).unwrap();
    let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
    let emitter = entity.intro2_gun_turret_runtime;
    let shots: Vec<_> = entity
        .intro2_gun_turret_aim_runtime
        .as_ref()
        .unwrap()
        .transient_shots()
        .iter()
        .copied()
        .collect();
    let muzzles = resolve_muzzles(entity, &session.cache, 99).unwrap();
    let mut expected_fx = fx.fork_for_main_base_abort_transaction();
    for shot in &shots {
        let mut position = muzzles[usize::from(shot.emitter_selector)].unwrap();
        let scale = shot.time_offset_raw >> 5;
        for (word, speed) in position.iter_mut().zip([300, -200, 400]) {
            *word = word.wrapping_sub(((speed * scale) >> 15) as i16);
        }
        let advance = ((5000 * scale) >> 15) as i16;
        for (word, direction) in position.iter_mut().zip(shot.direction_raw) {
            *word = word.wrapping_add(((i64::from(direction) * i64::from(advance)) >> 31) as i16);
        }
        let boost = ((i64::from(shot.direction_raw[2]) * 400) >> 31) as i32;
        let velocity = shot
            .direction_raw
            .map(|d| ((i64::from(d) * i64::from(5000 + boost)) >> 31) as i16);
        assert_eq!(
            velocity[0], 0,
            "method14 does not add source lateral velocity"
        );
        expected_fx
            .materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 55,
                    position_raw: position,
                    velocity_raw: velocity,
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: id,
                        entity_type: 102,
                    }),
                    suppresses_impact_damage: false,
                },
                ParticleEnvironment::Dry,
                99,
            )
            .unwrap();
    }
    let outcome = drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        99,
    )
    .unwrap();
    assert_eq!(outcome.consumed_requests, 2);
    assert_eq!(outcome.materialized_particle_classes, vec![55; 2]);
    assert_eq!(presented(&mut fx), presented(&mut expected_fx));
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_fx.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
        task
    );
    assert_eq!(entity.intro2_gun_turret_runtime, emitter);
    assert_eq!(
        entity
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        0
    );
    assert_eq!(
        drain_intro2_gun_turret_shots(
            &mut manager,
            &mut fx,
            id,
            &session.cache,
            ParticleEnvironment::Dry,
            99
        )
        .unwrap()
        .consumed_requests,
        0
    );
}

#[v2k_test_support::retail_test]
fn turret_failed_model_and_malformed_fifo_preserve_requests_rng_and_task() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 300_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        IMPACT_SUPPRESSION_STATE_BIT | PRESENTATION_DETAIL_STATE_BIT,
        PRESENTATION_DETAIL_STATE_BIT,
    );
    let queue = entity.intro2_gun_turret_aim_runtime.clone();
    let task = entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    entity.model_index = Some(usize::MAX);
    for _ in 0..2 {
        assert_eq!(
            drain_intro2_gun_turret_shots(
                &mut manager,
                &mut fx,
                id,
                &session.cache,
                ParticleEnvironment::Dry,
                0
            ),
            Err(Intro2GunTurretShotDrainError::ModelUnavailable {
                model_id: usize::MAX
            })
        );
    }
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.intro2_gun_turret_aim_runtime, queue);
    assert_eq!(
        entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
        task
    );
    entity.model_index = Some(162);
    entity.intro2_gun_turret_aim_runtime.as_mut().unwrap().shots[1].projectile_method = 30;
    assert_eq!(
        drain_intro2_gun_turret_shots(
            &mut manager,
            &mut fx,
            id,
            &session.cache,
            ParticleEnvironment::Dry,
            0
        ),
        Err(Intro2GunTurretShotDrainError::MalformedRequest { index: 1 })
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        2
    );
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn turret_rejected_particle_birth_consumes_both_commands_without_rng() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    assert_ne!(particle_descriptor(55).unwrap().flags() & 2, 0);
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 54);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 300_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    manager
        .entity_mut(id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(
            IMPACT_SUPPRESSION_STATE_BIT | PRESENTATION_DETAIL_STATE_BIT,
            IMPACT_SUPPRESSION_STATE_BIT,
        );
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let outcome = drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        0,
    )
    .unwrap();
    assert_eq!(outcome.consumed_requests, 2);
    assert!(outcome.materialized_particle_classes.is_empty());
    assert_eq!(fx.particle_count(), 0);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_gun_turret_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        0
    );
}

#[v2k_test_support::retail_test]
fn turret_offscreen_marker_one_uses_actor_position_without_model_or_basis_read() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &session, &metadata, 53);
    let mut fx = WorldFx::new();
    tick_intro2_gun_turret_aim(
        &mut manager,
        id,
        Some([0, 0, 1000]),
        crate::intro2_gun_turret::aim::Intro2GunTurretAimFrame {
            global_elapsed_micros: 300_000,
            world_fx: &mut fx,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::default(),
            notification_phase:
                crate::gameplay_notifications::GameplayNotificationPhase::NonGameplay,
            retail_tick: 75,
        },
    )
    .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.state_flags_at_0x08.overwrite(
        IMPACT_SUPPRESSION_STATE_BIT | PRESENTATION_DETAIL_STATE_BIT,
        0,
    );
    entity.set_position_raw([123, 234, 345]);
    entity.set_velocity_raw([0; 3]);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity.model_index = None;
    let outcome = drain_intro2_gun_turret_shots(
        &mut manager,
        &mut fx,
        id,
        &session.cache,
        ParticleEnvironment::Dry,
        0,
    )
    .unwrap();
    assert_eq!(outcome.consumed_requests, 2);
    let particles = presented(&mut fx);
    assert!(particles
        .iter()
        .any(|p| p.position == [123., 234., 345.].map(|v| v / 256.)));
}

#[v2k_test_support::retail_test]
fn native_type97_all_eight_model173_hierarchies_stamp_their_own_muzzle() {
    let mut count = 0;
    for level in [31, 42, 46, 47] {
        let (session, manager, _) = crate::intro2_gun_turret::authored_tests::fixture(level);
        for entity in manager.iter_all().filter(|entity| entity.entity_type == 97) {
            assert_eq!(entity.model_index, Some(173));
            let muzzles = resolve_muzzles(entity, &session.cache, 75).unwrap();
            let muzzle = muzzles[0].expect("model173/174 callback stamps own E muzzle18");
            assert_ne!(
                muzzle,
                entity.position_raw(),
                "detailed stamp is not the actor centre"
            );
            assert_eq!(muzzles[1], None, "Type97 has no alternate emitter");
            count += 1;
        }
    }
    assert_eq!(count, 8);
}
