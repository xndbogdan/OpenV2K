use super::*;
use crate::intro2_type10::intro2_type10_allocation_authenticates;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    aim_and_fire::{AimAndFireFrameOutcome, AimAndFireTransitionReason},
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
    },
    entity_collision_state::RetailRuntimeValue,
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_type47_live::world::native_intro2_fixture,
    search_attack_live::apply_search_attack_ade0_without_mover,
    session::GameSession,
};

fn prepare(
    session: &GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    spawn: usize,
) -> (u32, u32, EntityTypeRuntimeMetadata) {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(spawn))
        .unwrap()
        .id;
    // Use a native spider rather than Type13's peasant-only target list.
    // Klaus is a separate camera object, not an EntityManager allocation.
    let target_id = manager.iter_all().find(|e| e.entity_type == 47).unwrap().id;
    let entity = manager.entity_mut(id).unwrap();
    let metadata = EntityTypeRuntimeMetadata::from_section12(
        session
            .cache
            .global_entity_type(entity.entity_type as usize)
            .unwrap(),
    );
    match entity.entity_type {
        10 => assert!(intro2_type10_allocation_authenticates(entity)),
        16 => assert!(crate::intro2_type16::intro2_type16_allocation_authenticates(entity)),
        _ => panic!("unexpected Aim fixture cohort"),
    }
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(7).unwrap(),
            1,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(target_id)),
            RetailRuntimeValue::Known(0),
            *audited_behavior_style(7, 1).unwrap(),
        )
        .unwrap(),
    ));
    entity.set_position_raw([0, 1000, 0]);
    entity.set_velocity_raw([500, 0, 100]);
    entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
    entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    apply_search_attack_ade0_without_mover(entity, &metadata, target_id, fx).unwrap();
    let target = manager.entity_mut(target_id).unwrap();
    target.set_position_raw([0, 1000, 1000]);
    target.set_velocity_raw([0; 3]);
    target.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    (id, target_id, metadata)
}

fn age(manager: &EntityManager, id: u32) -> u32 {
    let entity = manager.iter_all().find(|e| e.id == id).unwrap();
    let Some(ActorTaskRuntime::AimAndFire(task)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("Aim task")
    };
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Tertiary)
        .unwrap();
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(task_id)
            .unwrap()
            .in_callback
    );
    task.elapsed_ms()
}

fn warm(fx: &mut WorldFx) {
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
}

fn presented_particles(fx: &mut WorldFx) -> Vec<crate::world_fx::WorldParticle> {
    fx.prepare_presentation([640, 480], 0x3000, |_| {
        v2k_render::ParticleCenterProjection {
            screen: [320, 240],
            depth_raw: 1000,
            clip: 0,
        }
    })
    .particles()
    .map(|prepared| prepared.particle)
    .collect()
}

fn q31_product(left: i32, right: i32) -> i32 {
    ((i64::from(left) * i64::from(right)) >> 31) as i32
}

#[v2k_test_support::retail_test]
fn native_type10_aim_restricted_ages_without_target_basis_emitter_or_rng_reads() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, target, _) = prepare(&session, &mut manager, &mut fx, 55);
    let entity = manager.entity_mut(id).unwrap();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity
        .intro2_type10_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = 12345;
    manager
        .entity_mut(target)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(u32::MAX);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let result = tick_intro2_type10_aim(
        CommonMoverDispatchMode::Restricted,
        &mut manager,
        &mut fx,
        id,
        20_000,
        None,
    )
    .unwrap();
    assert_eq!(result.queued_shots_added, 0);
    assert_eq!(age(&manager, id), 20);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity
            .intro2_type10_runtime
            .as_ref()
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        12345
    );
    assert!(entity.intro2_type10_aim_runtime.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_aim_both_births_use_override_cadence_sound_and_method10_drain() {
    for spawn in [55, 56] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let mut fx = WorldFx::new();
        warm(&mut fx);
        let (id, _, metadata) = prepare(&session, &mut manager, &mut fx, spawn);
        let mut expected_rng = fx.fork_for_main_base_abort_transaction();
        // 300000 / dt600000 clamps the gate divisor to one. One gate word
        // then two spread words; the catch-up loop itself draws no more RNG.
        for _ in 0..3 {
            expected_rng.next_shared_retail_random_u16();
        }
        let result = tick_intro2_type10_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            600_000,
            Some(&metadata),
        )
        .unwrap();
        assert_eq!(result.queued_shots_added, 3);
        assert_eq!(age(&manager, id), 600);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!((sounds[0].sound_id, sounds[0].frequency_q16), (81, 0x10000));
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity
                .intro2_type10_runtime
                .as_ref()
                .unwrap()
                .sub_e_runtime
                .cadence_raw,
            300_000
        );
        let shots = entity
            .intro2_type10_aim_runtime
            .as_ref()
            .unwrap()
            .transient_shots()
            .to_vec();
        assert_eq!(
            shots
                .iter()
                .map(|shot| shot.time_offset_raw)
                .collect::<Vec<_>>(),
            vec![0, 300_000, 600_000]
        );
        assert!(shots.iter().all(|shot| shot.source_handle == id
            && shot.owner_handle == id
            && shot.projectile_method == 10
            && shot.emitter_selector == 0
            && shot.speed_field == GenericEmitterSpeedField::Explicit(1500)
            && !shot.auxiliary));
        // Presentation drain uses the latest source pose/velocity even after
        // its behavior graph is replaced. Method10 must not add lateral X=500.
        entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
        entity.set_position_raw([200, 1000, 300]);
        let direction = shots[0].direction_raw;
        let dot = direction
            .into_iter()
            .zip([500i32, 0, 100])
            .fold(0i32, |sum, (d, v)| sum.wrapping_add(q31_product(d, v)));
        let speed = 1500 + dot.max(0);
        let mut velocity_raw = direction.map(|d| i32::from(q31_product(d, speed) as i16));
        velocity_raw[1] += i32::from(
            crate::particle_descriptors::particle_descriptor(38)
                .unwrap()
                .spawn_velocity_y_bias_raw(),
        );
        let result =
            drain_intro2_type10_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 30)
                .unwrap();
        assert_eq!(result.consumed_requests, 3);
        assert_eq!(result.materialized_particle_classes, vec![38; 3]);
        let particles = presented_particles(&mut fx);
        assert!(particles
            .iter()
            .all(|particle| particle.source_class == 38 && particle.owner_id == Some(id)));
        let first = particles
            .iter()
            .find(|particle| particle.position == [200.0 / 256.0, 1000.0 / 256.0, 300.0 / 256.0])
            .unwrap();
        assert_eq!(
            first.velocity,
            velocity_raw.map(|v| v as f32 * (1_000_000.0 / 1_048_576.0) / 256.0)
        );
        assert_eq!(
            drain_intro2_type10_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 30)
                .unwrap()
                .consumed_requests,
            0
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_method10_sea_boundary_substitutes46_and_suppression_consumes_fifo() {
    for (y, suppressed, expected_class) in [
        (1, false, Some(38)),
        (0, false, Some(46)),
        (-1, false, Some(46)),
        (1, true, None),
    ] {
        let Some((session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let mut fx = WorldFx::new();
        warm(&mut fx);
        let (id, _, metadata) = prepare(&session, &mut manager, &mut fx, 55);
        let result = tick_intro2_type10_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            300_000,
            Some(&metadata),
        )
        .unwrap();
        assert_eq!(result.queued_shots_added, 2);
        let entity = manager.entity_mut(id).unwrap();
        entity.set_position_raw([0, y, 0]);
        // The first record has zero time offset, so its exact sea-boundary
        // classification is independent of the later rewind/advance record.
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(0x8000_0000, if suppressed { 0x8000_0000 } else { 0 });
        let result = drain_intro2_type10_shots(
            &mut manager,
            &mut fx,
            id,
            ParticleEnvironment::FlatWater { sea_level: 0.0 },
            30,
        )
        .unwrap();
        assert_eq!(result.consumed_requests, 2);
        match expected_class {
            Some(class) => {
                assert_eq!(result.materialized_particle_classes[0], class);
                // 410B0 subtracts 100 before 40A60; class46's +0x28 bias adds
                // those units back, so the live centre stays on the muzzle Y.
                assert!(presented_particles(&mut fx)
                    .iter()
                    .any(|particle| particle.source_class == class
                        && particle.position == [0.0, f32::from(y) / 256.0, 0.0]));
            }
            None => assert!(result.materialized_particle_classes.is_empty()),
        }
        assert_eq!(
            drain_intro2_type10_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 30)
                .unwrap()
                .consumed_requests,
            0
        );
    }
}

#[v2k_test_support::retail_test]
fn native_type10_late_basis_block_retains_age_cadence_and_rng_prefix() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx, 56);
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let mut expected = fx.fork_for_main_base_abort_transaction();
    expected.next_shared_retail_random_u16();
    assert_eq!(
        tick_intro2_type10_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            300_000,
            Some(&metadata)
        ),
        Err(Intro2Type10AimError::BodyBasisUnresolved)
    );
    assert_eq!(age(&manager, id), 300);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity
            .intro2_type10_runtime
            .as_ref()
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -300_000
    );
    assert_eq!(
        entity
            .intro2_type10_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        0
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_invalid_target_tag_precedes_emitter_and_timeout() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, target, metadata) = prepare(&session, &mut manager, &mut fx, 55);
    manager
        .entity_mut(target)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 0x4000);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let result = tick_intro2_type10_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        5_001_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(matches!(
        result.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::TaggedInvalidTarget { .. }
        }
    ));
    assert_eq!(age(&manager, id), 5001);
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_type10_runtime
            .as_ref()
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        0
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_profile_extension_preserves_method20_velocity_and_class52() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    warm(&mut fx);
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx, 5);
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    for _ in 0..3 {
        expected_rng.next_shared_retail_random_u16();
    }
    let result = crate::intro2_type16::aim::tick_intro2_type16_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        300_000,
        Some(&metadata),
    )
    .unwrap();
    assert_eq!(result.queued_shots_added, 2);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
    let entity = manager.entity_mut(id).unwrap();
    let shot = entity
        .intro2_type16_aim_runtime
        .as_ref()
        .unwrap()
        .transient_shots()[0];
    assert_eq!(shot.projectile_method, 20);
    assert_eq!(shot.speed_field, GenericEmitterSpeedField::Explicit(3000));
    let mut velocity = std::array::from_fn::<_, 3, _>(|axis| {
        [500i16, 0, 100][axis].wrapping_add(q31_product(shot.direction_raw[axis], 3000) as i16)
            as i32
    });
    velocity[1] += i32::from(
        crate::particle_descriptors::particle_descriptor(52)
            .unwrap()
            .spawn_velocity_y_bias_raw(),
    );
    let result = crate::intro2_type16::aim::drain_intro2_type16_shots(
        &mut manager,
        &mut fx,
        id,
        ParticleEnvironment::FlatWater {
            sea_level: 2000.0 / 256.0,
        },
        30,
    )
    .unwrap();
    assert_eq!(result.materialized_particle_classes, vec![52; 2]);
    let particles = presented_particles(&mut fx);
    let first = particles
        .iter()
        .find(|p| p.position == [0.0, 1000.0 / 256.0, 0.0])
        .unwrap();
    assert_eq!(first.source_class, 52);
    assert_eq!(
        first.velocity,
        velocity.map(|v| v as f32 * (1_000_000.0 / 1_048_576.0) / 256.0)
    );
}

#[v2k_test_support::retail_test]
fn native_type10_presentation_fifo_drains_in_live_source_order_across_profiles() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    warm(&mut fx);
    // Append the later dragon first. 11720 must still visit the earlier
    // Type16 allocation first, independent of the adapter and append order.
    let (dragon, _, dragon_metadata) = prepare(&session, &mut manager, &mut fx, 55);
    tick_intro2_type10_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        dragon,
        300_000,
        Some(&dragon_metadata),
    )
    .unwrap();
    let (trog, _, trog_metadata) = prepare(&session, &mut manager, &mut fx, 5);
    crate::intro2_type16::aim::tick_intro2_type16_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        trog,
        300_000,
        Some(&trog_metadata),
    )
    .unwrap();
    let result = crate::intro2_projectiles::drain_intro2_projectiles(
        &mut manager,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        30,
    );
    assert_eq!(
        result.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![trog, dragon]
    );
    assert!(result.iter().all(|(_, outcome)| matches!(outcome, Ok(2))));
    assert_eq!(fx.particle_count(), 4);
    assert!(crate::intro2_projectiles::drain_intro2_projectiles(
        &mut manager,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        30
    )
    .is_empty());
}
