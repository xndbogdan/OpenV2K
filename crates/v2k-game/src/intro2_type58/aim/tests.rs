use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    entity_collision_state::RetailRuntimeValue,
    ordinary_type47_shot_math::{evaluate_type47_aim_error, evaluate_type47_forward_half_space},
};
use crate::{
    entity::EntityManager,
    entity_behavior::{
        audited_behavior_style, behavior_program, BehaviorChoiceListSource, BehaviorContextRuntime,
    },
    intro2_type47_live::world::native_intro2_fixture,
    search_attack_live::apply_search_attack_ade0_without_mover,
    session::GameSession,
};

fn prepare(
    session: &GameSession,
    manager: &mut EntityManager,
    fx: &mut WorldFx,
) -> (u32, u32, EntityTypeRuntimeMetadata) {
    let id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(40))
        .unwrap()
        .id;
    let target_id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(3))
        .unwrap()
        .id;
    let metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(58).unwrap());
    let entity = manager.entity_mut(id).unwrap();
    assert!(intro2_type58_allocation_authenticates(entity));
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
    entity.set_velocity_raw([100, 0, -200]);
    // Retail13F70 heading0 points +X; quarter-turn4000 points toward +Z.
    entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    let basis = Type9BodyBasis::from_angle_words(0x4000, 0, 0);
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
    assert!(
        evaluate_type47_aim_error([0, 1000, 0], basis.lateral, basis.forward, [0, 1000, 1000])
            .unsigned_abs()
            < 16_000
    );
    assert!(evaluate_type47_forward_half_space(
        [0, 1000, 0],
        basis.forward,
        [0, 1000, 1000]
    ));
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
        panic!()
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

fn select_successful_gate(fx: &mut WorldFx) {
    loop {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        if u32::from(peek.next_shared_retail_random_u16()) % (400_000 / 125_000) == 0 {
            break;
        }
        fx.next_shared_retail_random_u16();
    }
}

#[v2k_test_support::retail_test]
fn native_type58_aim_restricted_only_ages_without_metadata_basis_target_or_rng() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, target, _) = prepare(&session, &mut manager, &mut fx);
    let entity = manager.entity_mut(id).unwrap();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity
        .intro2_type58_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = 123_456;
    manager
        .entity_mut(target)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(u32::MAX);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let result = tick_intro2_type58_aim(
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
            .intro2_type58_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        123_456
    );
    assert!(entity.intro2_type58_aim_runtime.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type58_aim_cadence_and_fifo_survive_root_replacement_and_drain_once() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx);
    // Controlled fixture selects a passing interval400000 / dt125000 gate.
    select_successful_gate(&mut fx);
    let result = tick_intro2_type58_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        125_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(result.queued_shots_added > 0, "{result:?}");
    // 24650 queues its one runtime+20 cue after the successful append loop.
    // WorldFx's host handoff materializes queued events before audio drains;
    // the later11400 projectile drain does not own this sound.
    fx.process_pending();
    let sounds = fx.take_positional_sounds();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].sound_id, 70);
    assert_eq!(sounds[0].frequency_q16, 0x1_0000);
    assert_eq!(sounds[0].position, manager.entity_mut(id).unwrap().position);
    let entity = manager.entity_mut(id).unwrap();
    let cadence = entity
        .intro2_type58_runtime
        .unwrap()
        .sub_e_runtime
        .cadence_raw;
    assert!(cadence > 0);
    let queue = entity.intro2_type58_aim_runtime.as_ref().unwrap();
    assert!(queue.transient_shots().iter().all(|shot| {
        shot.source_handle == id
            && shot.owner_handle == id
            && shot.projectile_method == 20
            && shot.emitter_selector == 0
            && shot.speed_field
                == crate::generic_projectile_emitter::GenericEmitterSpeedField::Explicit(3000)
            && !shot.auxiliary
    }));
    let count = queue.queued_shot_count();
    // Actual10C10/C850 class12 replacement clears Aim but leaves the native
    // allocation's Sub-E cadence and already queued11400 commands intact.
    let death =
        crate::intro2_common_dying::publish_intro2_common_standard_death(&mut manager, id, &mut fx)
            .unwrap()
            .expect("native common12 publication");
    assert_eq!(death.entity_id(), id);
    assert!(manager
        .entity_mut(id)
        .unwrap()
        .actor_task_state(ActorTaskSlot::Tertiary)
        .is_none());
    let result =
        drain_intro2_type58_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 20).unwrap();
    assert_eq!(result.consumed_requests, count);
    assert_eq!(result.materialized_particle_classes, vec![52; count]);
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_type58_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        cadence
    );
    assert_eq!(
        drain_intro2_type58_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 20)
            .unwrap()
            .consumed_requests,
        0
    );
}

#[v2k_test_support::retail_test]
fn native_type58_aim_late_basis_block_keeps_wrapper_age_and_native_cadence_prefix() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx);
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    select_successful_gate(&mut fx);
    assert_eq!(
        tick_intro2_type58_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            125_000,
            Some(&metadata)
        ),
        Err(Intro2Type58AimError::BodyBasisUnresolved)
    );
    assert_eq!(age(&manager, id), 125);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity
            .intro2_type58_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -125_000
    );
    assert_eq!(
        entity
            .intro2_type58_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        0
    );
}

#[v2k_test_support::retail_test]
fn native_type58_aim_rejected_cadence_gate_precedes_basis_and_preserves_one_rng_prefix() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx);
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let mut expected = fx.fork_for_main_base_abort_transaction();
    let gate = expected.next_shared_retail_random_u16();
    assert_ne!(u32::from(gate) % (400_000 / 20_000), 0);
    let result = tick_intro2_type58_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        20_000,
        Some(&metadata),
    )
    .unwrap();
    assert_eq!(result.queued_shots_added, 0);
    assert_eq!(age(&manager, id), 20);
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .intro2_type58_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -20_000
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type58_rejects_type16_fifo_and_foreign_descriptor_without_changing_cadence() {
    let Some((session, mut manager, _)) = native_intro2_fixture() else {
        return;
    };
    let mut fx = WorldFx::new();
    let (id, _, metadata) = prepare(&session, &mut manager, &mut fx);
    let other = manager
        .iter_all()
        .find(|entity| entity.authored_spawn_index == Some(5))
        .unwrap()
        .id;
    let other_metadata = manager.type_runtime_metadata(16).unwrap().clone();
    crate::intro2_type16::aim::ensure_intro2_type16_aim_runtime(
        manager.entity_mut(other).unwrap(),
        Some(&other_metadata),
    )
    .unwrap();
    let foreign = manager
        .entity_mut(other)
        .unwrap()
        .intro2_type16_aim_runtime
        .clone()
        .unwrap();
    let entity = manager.entity_mut(id).unwrap();
    let before = entity.intro2_type58_runtime;
    entity.intro2_type58_aim_runtime = Some(foreign);
    assert_eq!(
        ensure_intro2_type58_aim_runtime(entity, Some(&metadata)),
        Err(Intro2Type58AimError::RuntimeContractMismatch)
    );
    assert_eq!(entity.intro2_type58_runtime, before);
    entity.intro2_type58_aim_runtime = None;
    assert_eq!(
        ensure_intro2_type58_aim_runtime(entity, Some(&other_metadata)),
        Err(Intro2Type58AimError::DescriptorMismatch)
    );
    assert!(entity.intro2_type58_aim_runtime.is_none());
    assert_eq!(entity.intro2_type58_runtime, before);
}
