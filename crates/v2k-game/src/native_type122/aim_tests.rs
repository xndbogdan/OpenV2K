//! Actual Search ADE0 -> method24 Aim -> presentation FIFO for native ice ants.

use super::{aim::*, construction_tests::native_fixture, profile::Type122Profile, *};
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    common_mover::{component_dispatch::CommonMoverDispatchMode, type9_attitude::Type9BodyBasis},
    entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection},
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_projectiles::{drain_intro2_projectiles, Intro2ShotDrainError},
    native_ground_actor::{behavior, search},
    world_fx::{ParticleEnvironment, WorldFx},
};

pub(crate) fn prepare_search(
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    id: u32,
) -> (u32, EntityTypeRuntimeMetadata) {
    let metadata = manager.type_runtime_metadata(122).unwrap().clone();
    let candidates: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.active && entity.entity_type != 122)
        .unwrap()
        .id;
    for candidate in candidates {
        let candidate = manager.entity_mut(candidate).unwrap();
        candidate.capability_flags &= !0xc03;
        if candidate.entity_type != 122 {
            candidate
                .collision
                .state_flags_at_0x08
                .overwrite(u32::MAX, 4);
        }
    }
    let target = manager.entity_mut(target_id).unwrap();
    target.capability_flags |= 1;
    target.set_position_raw([0, 1000, 1000]);
    target.set_velocity_raw([0; 3]);
    target.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
    let entity = manager.entity_mut(id).unwrap();
    entity.set_position_raw([0, 1000, 0]);
    entity.set_velocity_raw([100, 0, -200]);
    entity.set_rotation_heading_pitch_roll_raw([0x4000, 0, 0]);
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0x4000, 0, 0));
    entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    let RetailRuntimeValue::Known(Some(birth)) = entity.initial_behavior else {
        panic!("native birth")
    };
    let selection = BehaviorSelection {
        program: behavior_program(7).unwrap(),
        choice_index: 3,
        ..birth
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
    assert!(publish_acquiring(
        entity,
        &metadata,
        selection,
        context,
        &mut || u32::from(fx.next_shared_retail_random_u16())
    ));
    behavior::secondary::<Type122Profile>(manager, id, 20_000, 4794, fx).unwrap();
    let entity = manager.entity_mut(id).unwrap();
    assert!(search::pursuing_graph_authenticates::<Type122Profile>(
        entity
    ));
    assert!(
        matches!(entity.actor_task_state(ActorTaskSlot::Primary),Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id()==target_id)
    );
    assert!(
        matches!(&entity.sub_j_attachment_runtime,RetailRuntimeValue::Known(Some(j)) if j.is_empty() && j.authored_slot_count()==1)
    );
    (target_id, metadata)
}

fn select_gate(fx: &mut WorldFx, elapsed: u32, passing: bool) {
    for _ in 0..65536 {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        if (u32::from(peek.next_shared_retail_random_u16()) % (400_000 / elapsed) == 0) == passing {
            return;
        }
        fx.next_shared_retail_random_u16();
    }
    panic!("real process stream must reach requested cadence gate")
}

fn aim_age(manager: &EntityManager, id: u32) -> u32 {
    let entity = manager.iter_all().find(|entity| entity.id == id).unwrap();
    let Some(ActorTaskRuntime::AimAndFire(task)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("real Aim")
    };
    task.elapsed_ms()
}

fn queue_shot(manager: &mut EntityManager, fx: &mut WorldFx, id: u32) -> usize {
    let (_, metadata) = prepare_search(manager, fx, id);
    select_gate(fx, 125_000, true);
    let outcome = tick_type122_aim(
        CommonMoverDispatchMode::Normal,
        manager,
        fx,
        id,
        125_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(outcome.queued_shots_added > 0, "{outcome:?}");
    let entity = manager.entity_mut(id).unwrap();
    let queue = entity.native_type122_aim_runtime.as_ref().unwrap();
    assert!(queue
        .transient_shots()
        .iter()
        .all(|shot| shot.source_handle == id
            && shot.owner_handle == id
            && shot.projectile_method == 24
            && shot.emitter_selector == 0
            && shot.speed_field == GenericEmitterSpeedField::Explicit(2000)
            && !shot.auxiliary));
    assert!(
        entity
            .native_type122_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw
            > 0
    );
    queue.queued_shot_count()
}

#[v2k_test_support::retail_test]
fn method24_class68_fifo_and_sound70_survive_c690_and_class12_then_drain_in_manager_order() {
    let (session, mut manager, mut fx) = native_fixture(24);
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    fx.process_pending();
    fx.take_positional_sounds();
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 122)
        .take(2)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 2);
    let mut counts = Vec::new();
    // Queue in reverse allocation order;11400 still follows manager order.
    for &id in ids.iter().rev() {
        counts.push((id, queue_shot(&mut manager, &mut fx, id)));
        fx.process_pending();
        let sounds = fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!((sounds[0].sound_id, sounds[0].frequency_q16), (70, 0x10000));
        assert_eq!(sounds[0].position, manager.entity_mut(id).unwrap().position);
    }
    counts.reverse();
    let components: Vec<_> = ids
        .iter()
        .map(|&id| manager.entity_mut(id).unwrap().native_type122_runtime)
        .collect();
    let queues: Vec<_> = ids
        .iter()
        .map(|&id| {
            manager
                .entity_mut(id)
                .unwrap()
                .native_type122_aim_runtime
                .clone()
        })
        .collect();
    behavior::reselect::<Type122Profile>(
        &mut manager,
        ids[0],
        4794,
        &mut fx,
        behavior::ReselectionEntry::Impact,
    )
    .unwrap();
    let death = crate::intro2_common_dying::publish_intro2_common_standard_death(
        &mut manager,
        ids[1],
        &mut fx,
    )
    .unwrap()
    .expect("actual native Class12 replacement");
    assert_eq!(death.entity_id(), ids[1]);
    for (index, &id) in ids.iter().enumerate() {
        let entity = manager.entity_mut(id).unwrap();
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(entity.native_type122_runtime, components[index]);
        assert_eq!(entity.native_type122_aim_runtime, queues[index]);
    }
    let before = fx.particle_count();
    let drained = drain_intro2_projectiles(
        &mut manager,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        4794,
    );
    assert_eq!(
        drained
            .into_iter()
            .map(|(id, result)| (id, result.unwrap()))
            .collect::<Vec<_>>(),
        counts
    );
    let expected: Vec<_> = counts
        .iter()
        .flat_map(|&(id, count)| std::iter::repeat_n(Some(id), count))
        .collect();
    let particles = fx.test_particles_in_virgin_birth_order();
    let born = &particles[before..];
    assert_eq!(
        born.iter()
            .map(|particle| particle.owner_id)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(born.iter().all(|particle| particle.source_class == 68
        && particle.source_entity_type_at_birth == Some(122)
        && particle.age_ticks == 0.0));
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    assert!(drain_intro2_projectiles(
        &mut manager,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        4794
    )
    .is_empty());
    assert_eq!(fx.particle_count(), particles.len());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn type122_restricted_aim_ages_only_and_normal_cadence_rejection_consumes_exactly_one_word() {
    let (_, mut manager, mut fx) = native_fixture(24);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let (target, metadata) = prepare_search(&mut manager, &mut fx, id);
    let entity = manager.entity_mut(id).unwrap();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity
        .native_type122_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = 123456;
    let before = entity.native_type122_runtime;
    manager
        .entity_mut(target)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(u32::MAX);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        tick_type122_aim(
            CommonMoverDispatchMode::Restricted,
            &mut manager,
            &mut fx,
            id,
            20_000,
            None
        )
        .unwrap()
        .queued_shots_added,
        0
    );
    assert_eq!(aim_age(&manager, id), 20);
    assert_eq!(
        manager.entity_mut(id).unwrap().native_type122_runtime,
        before
    );
    assert!(manager
        .entity_mut(id)
        .unwrap()
        .native_type122_aim_runtime
        .is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );
    manager
        .entity_mut(target)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .overwrite(u32::MAX, 4);
    manager
        .entity_mut(id)
        .unwrap()
        .native_type122_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = 0;
    select_gate(&mut fx, 20_000, false);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    assert_ne!(u32::from(expected.next_shared_retail_random_u16()) % 20, 0);
    assert_eq!(
        tick_type122_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            20_000,
            Some(&metadata)
        )
        .unwrap()
        .queued_shots_added,
        0
    );
    assert_eq!(aim_age(&manager, id), 40);
    assert_eq!(
        manager
            .entity_mut(id)
            .unwrap()
            .native_type122_runtime
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
fn type122_late_basis_failure_keeps_exact_native_cadence_and_wrapper_prefix() {
    let (_, mut manager, mut fx) = native_fixture(24);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let (_, metadata) = prepare_search(&mut manager, &mut fx, id);
    manager.entity_mut(id).unwrap().physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    select_gate(&mut fx, 125_000, true);
    let mut expected = fx.fork_for_main_base_abort_transaction();
    assert_eq!(u32::from(expected.next_shared_retail_random_u16()) % 3, 0);
    assert_eq!(
        tick_type122_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            125_000,
            Some(&metadata)
        ),
        Err(Type122AimError::BodyBasisUnresolved)
    );
    assert_eq!(aim_age(&manager, id), 125);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity
            .native_type122_runtime
            .unwrap()
            .sub_e_runtime
            .cadence_raw,
        -125_000
    );
    assert_eq!(
        entity
            .native_type122_aim_runtime
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
fn type122_foreign_manager_or_other_method24_profile_cannot_emit_or_drain() {
    let (_, mut source, mut source_fx) = native_fixture(24);
    let (session, mut foreign, mut fx) = native_fixture(24);
    let id = source
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let queued = queue_shot(&mut source, &mut source_fx, id);
    let metadata = source.type_runtime_metadata(122).unwrap().clone();
    std::mem::swap(
        source.entity_mut(id).unwrap(),
        foreign.entity_mut(id).unwrap(),
    );
    assert!(type122_allocation_authenticates(
        foreign.entity_mut(id).unwrap()
    ));
    assert!(!type122_manager_allocation_authenticates(&foreign, id));
    let age = aim_age(&foreign, id);
    let entity = foreign.entity_mut(id).unwrap();
    let component = entity.native_type122_runtime;
    let queue = entity.native_type122_aim_runtime.clone();
    let before = fx.particle_count();
    let mut expected = fx.fork_for_main_base_abort_transaction();
    for mode in [
        CommonMoverDispatchMode::Normal,
        CommonMoverDispatchMode::Restricted,
    ] {
        assert_eq!(
            tick_type122_aim(mode, &mut foreign, &mut fx, id, 125_000, Some(&metadata)),
            Err(Type122AimError::GraphMismatch)
        );
    }
    assert_eq!(
        drain_type122_shots(&mut foreign, &mut fx, id, ParticleEnvironment::Dry, 4794),
        Err(Type122ShotDrainError::RuntimeContractMismatch)
    );
    let drained = drain_intro2_projectiles(
        &mut foreign,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        4794,
    );
    assert!(
        matches!(drained.as_slice(),[(source,Err(Intro2ShotDrainError::Type122(Type122ShotDrainError::RuntimeContractMismatch)))] if *source==id)
    );
    assert_eq!(aim_age(&foreign, id), age);
    let entity = foreign.entity_mut(id).unwrap();
    assert_eq!(entity.native_type122_runtime, component);
    assert_eq!(entity.native_type122_aim_runtime, queue);
    assert_eq!(
        entity
            .native_type122_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        queued
    );
    assert_eq!(fx.particle_count(), before);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected.next_shared_retail_random_u16()
    );

    // Type57 shares method24, but its own emitter/profile is not Type122 custody.
    let (session, mut native, _) = native_fixture(50);
    let ant = native
        .iter_all()
        .find(|entity| entity.entity_type == 122)
        .unwrap()
        .id;
    let other = native
        .iter_all()
        .find(|entity| entity.entity_type == 57)
        .unwrap()
        .id;
    let other_metadata =
        EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(57).unwrap());
    let entity = native.entity_mut(other).unwrap();
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(
        BehaviorContextRuntime::named_audited(
            behavior_program(7).unwrap(),
            1,
            RetailRuntimeValue::Known(
                crate::entity_behavior::BehaviorChoiceListSource::TypeDefault,
            ),
            RetailRuntimeValue::Known(Some(ant)),
            RetailRuntimeValue::Known(0),
            *crate::entity_behavior::audited_behavior_style(7, 1).unwrap(),
        )
        .unwrap(),
    ));
    entity.collision.state_flags_at_0x08.overwrite(u32::MAX, 4);
    crate::search_attack_live::apply_search_attack_ade0_without_mover(
        entity,
        &other_metadata,
        ant,
        &mut fx,
    )
    .unwrap();
    crate::intro2_type57::aim::tick_intro2_type57_aim(
        CommonMoverDispatchMode::Normal,
        &mut native,
        &mut fx,
        other,
        20_000,
        Some(&other_metadata),
    )
    .unwrap();
    let other_queue = native
        .entity_mut(other)
        .unwrap()
        .intro2_type57_aim_runtime
        .clone();
    let entity = native.entity_mut(ant).unwrap();
    let before = entity.native_type122_runtime;
    entity.native_type122_aim_runtime = other_queue;
    assert_eq!(
        ensure_native_type122_aim_runtime(entity, Some(&metadata)),
        Err(Type122AimError::RuntimeContractMismatch)
    );
    assert_eq!(entity.native_type122_runtime, before);
    entity.native_type122_aim_runtime = None;
    assert_eq!(
        ensure_native_type122_aim_runtime(entity, Some(&other_metadata)),
        Err(Type122AimError::DescriptorMismatch)
    );
    assert!(entity.native_type122_aim_runtime.is_none());
}
