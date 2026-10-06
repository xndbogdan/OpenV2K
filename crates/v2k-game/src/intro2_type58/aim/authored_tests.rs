//! Ordinary Type58 Search/Aim and presentation drain retain the native
//! allocation across task replacement and reject foreign manager generations.

use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    common_mover::type9_attitude::Type9BodyBasis,
    entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::RetailRuntimeValue,
    generic_projectile_emitter::GenericEmitterSpeedField,
    intro2_projectiles::{drain_intro2_projectiles, Intro2ShotDrainError},
    intro2_type53::authored_tests::native_fixture,
    intro2_type58::{behavior, native, search},
};

fn prepare_search(
    manager: &mut EntityManager,
    fx: &mut WorldFx,
    id: u32,
) -> (u32, EntityTypeRuntimeMetadata) {
    let metadata = manager.type_runtime_metadata(58).unwrap().clone();
    let candidates: Vec<_> = manager.iter_all().map(|entity| entity.id).collect();
    let target_id = manager
        .iter_all()
        .find(|entity| entity.active && entity.entity_type != 58)
        .unwrap()
        .id;
    for candidate in candidates {
        // Controlled player-mask search population, while retaining each
        // actor's actual manager allocation and native component receipt.
        let candidate = manager.entity_mut(candidate).unwrap();
        candidate.capability_flags &= !0xc03;
        if candidate.entity_type != 58 {
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
    let RetailRuntimeValue::Known(Some(birth_selection)) = entity.initial_behavior else {
        panic!("native birth selection");
    };
    let selection = BehaviorSelection {
        program: behavior_program(7).unwrap(),
        choice_index: 2,
        ..birth_selection
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
    assert!(native::publish_initial_style(
        entity,
        &metadata,
        selection,
        context,
        &mut || u32::from(fx.next_shared_retail_random_u16()),
    ));
    assert!(search::acquire(manager, id, 20_000, fx).unwrap());
    let entity = manager.entity_mut(id).unwrap();
    assert!(search::pursuing_graph_authenticates(entity));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == target_id
    ));
    (target_id, metadata)
}

fn queue_shot(manager: &mut EntityManager, fx: &mut WorldFx, id: u32) -> usize {
    let (_, metadata) = prepare_search(manager, fx, id);
    // Select a real stream entry accepted by 400000 / dt125000 without
    // replacing the process RNG or writing directly into the private FIFO.
    loop {
        let mut peek = fx.fork_for_main_base_abort_transaction();
        if u32::from(peek.next_shared_retail_random_u16()) % (400_000 / 125_000) == 0 {
            break;
        }
        fx.next_shared_retail_random_u16();
    }
    let outcome = tick_intro2_type58_aim(
        CommonMoverDispatchMode::Normal,
        manager,
        fx,
        id,
        125_000,
        Some(&metadata),
    )
    .unwrap();
    assert!(outcome.queued_shots_added > 0, "{outcome:?}");
    let queue = manager
        .entity_mut(id)
        .unwrap()
        .intro2_type58_aim_runtime
        .as_ref()
        .unwrap();
    assert!(queue.transient_shots().iter().all(|shot| {
        shot.source_handle == id
            && shot.owner_handle == id
            && shot.projectile_method == 20
            && shot.emitter_selector == 0
            && shot.speed_field == GenericEmitterSpeedField::Explicit(3000)
            && !shot.auxiliary
    }));
    queue.queued_shot_count()
}

fn aim_age(manager: &mut EntityManager, id: u32) -> u32 {
    let entity = manager.entity_mut(id).unwrap();
    let Some(ActorTaskRuntime::AimAndFire(task)) = entity.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("live Aim wrapper");
    };
    task.elapsed_ms()
}

#[v2k_test_support::retail_test]
fn ordinary_type58_shared_drain_preserves_manager_order_across_c690_and_class12() {
    let (session, mut manager, mut fx) = native_fixture(14);
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let ids: Vec<_> = manager
        .iter_all()
        .filter(|entity| entity.entity_type == 58)
        .take(2)
        .map(|entity| entity.id)
        .collect();
    assert_eq!(ids.len(), 2);
    let mut expected_counts = Vec::new();
    for &id in ids.iter().rev() {
        let count = queue_shot(&mut manager, &mut fx, id);
        expected_counts.push((id, count));
    }
    expected_counts.reverse();
    let components: Vec<_> = ids
        .iter()
        .map(|&id| {
            manager
                .entity_mut(id)
                .unwrap()
                .intro2_type58_runtime
                .unwrap()
        })
        .collect();
    let queues: Vec<_> = ids
        .iter()
        .map(|&id| {
            manager
                .entity_mut(id)
                .unwrap()
                .intro2_type58_aim_runtime
                .clone()
        })
        .collect();
    behavior::reselect(
        &mut manager,
        ids[0],
        &session.cache,
        &mut fx,
        behavior::ReselectionEntry::DirectCallback,
    )
    .unwrap();
    let dying = crate::intro2_common_dying::publish_intro2_common_standard_death(
        &mut manager,
        ids[1],
        &mut fx,
    )
    .unwrap()
    .expect("native class12 replacement");
    assert_eq!(dying.entity_id(), ids[1]);
    for (index, &id) in ids.iter().enumerate() {
        let entity = manager.entity_mut(id).unwrap();
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(entity.intro2_type58_runtime, Some(components[index]));
        assert_eq!(entity.intro2_type58_aim_runtime, queues[index]);
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
            .map(|(id, count)| (id, count.unwrap()))
            .collect::<Vec<_>>(),
        expected_counts
    );
    let expected_owners: Vec<_> = expected_counts
        .iter()
        .flat_map(|&(id, count)| std::iter::repeat_n(Some(id), count))
        .collect();
    let particles = fx.test_particles_in_virgin_birth_order();
    let born = &particles[before..];
    assert_eq!(born.len(), expected_owners.len());
    assert_eq!(
        born.iter()
            .map(|particle| particle.owner_id)
            .collect::<Vec<_>>(),
        expected_owners
    );
    assert!(born.iter().all(|particle| {
        particle.source_class == 52
            && particle.source_entity_type_at_birth == Some(58)
            && particle.age_ticks == 0.0
    }));
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    assert!(drain_intro2_projectiles(
        &mut manager,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        4794,
    )
    .is_empty());
    assert_eq!(fx.particle_count(), particles.len());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn ordinary_type58_restricted_aim_only_ages_its_authenticated_wrapper() {
    let (_, mut manager, mut fx) = native_fixture(14);
    let id = manager
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .id;
    let (target_id, _) = prepare_search(&mut manager, &mut fx, id);
    let entity = manager.entity_mut(id).unwrap();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    entity
        .intro2_type58_runtime
        .as_mut()
        .unwrap()
        .sub_e_runtime
        .cadence_raw = 123_456;
    let component = entity.intro2_type58_runtime;
    manager
        .entity_mut(target_id)
        .unwrap()
        .collision
        .state_flags_at_0x08
        .invalidate(u32::MAX);
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    let outcome = tick_intro2_type58_aim(
        CommonMoverDispatchMode::Restricted,
        &mut manager,
        &mut fx,
        id,
        20_000,
        None,
    )
    .unwrap();
    assert_eq!(outcome.queued_shots_added, 0);
    assert_eq!(aim_age(&mut manager, id), 20);
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.intro2_type58_runtime, component);
    assert!(entity.intro2_type58_aim_runtime.is_none());
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn ordinary_type58_foreign_manager_cannot_age_emit_or_drain_queued_commands() {
    let (_, mut source, mut source_fx) = native_fixture(14);
    let (session, mut foreign, mut fx) = native_fixture(14);
    let id = source
        .iter_all()
        .find(|entity| entity.entity_type == 58)
        .unwrap()
        .id;
    let queued = queue_shot(&mut source, &mut source_fx, id);
    let metadata = source.type_runtime_metadata(58).unwrap().clone();
    std::mem::swap(
        source.entity_mut(id).unwrap(),
        foreign.entity_mut(id).unwrap(),
    );
    assert!(intro2_type58_allocation_authenticates(
        foreign.entity_mut(id).unwrap()
    ));
    assert!(!type58_manager_allocation_authenticates(&foreign, id));
    let before_age = aim_age(&mut foreign, id);
    let entity = foreign.entity_mut(id).unwrap();
    let components = entity.intro2_type58_runtime;
    let queue = entity.intro2_type58_aim_runtime.clone();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    let before_particles = fx.particle_count();
    for mode in [
        CommonMoverDispatchMode::Normal,
        CommonMoverDispatchMode::Restricted,
    ] {
        assert_eq!(
            tick_intro2_type58_aim(
                mode,
                &mut foreign,
                &mut fx,
                id,
                125_000,
                if mode == CommonMoverDispatchMode::Normal {
                    Some(&metadata)
                } else {
                    None
                },
            ),
            Err(Intro2Type58AimError::GraphMismatch)
        );
    }
    assert_eq!(
        drain_intro2_type58_shots(&mut foreign, &mut fx, id, ParticleEnvironment::Dry, 4794,),
        Err(Intro2Type58ShotDrainError::RuntimeContractMismatch)
    );
    let drained = drain_intro2_projectiles(
        &mut foreign,
        &mut fx,
        &session.cache,
        ParticleEnvironment::Dry,
        4794,
    );
    assert!(
        matches!(drained.as_slice(), [(source, Err(Intro2ShotDrainError::Type58(Intro2Type58ShotDrainError::RuntimeContractMismatch)))] if *source == id)
    );
    assert_eq!(aim_age(&mut foreign, id), before_age);
    let entity = foreign.entity_mut(id).unwrap();
    assert_eq!(entity.intro2_type58_runtime, components);
    assert_eq!(entity.intro2_type58_aim_runtime, queue);
    assert_eq!(
        entity
            .intro2_type58_aim_runtime
            .as_ref()
            .unwrap()
            .queued_shot_count(),
        queued
    );
    assert_eq!(fx.particle_count(), before_particles);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}
