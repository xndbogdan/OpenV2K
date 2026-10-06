use super::*;
use v2k_game::{
    aim_and_fire::AimAndFireInvalidTargetReason,
    common_mover::component_dispatch::CommonMoverDispatchMode,
    intro2_type13_aim::{drain_intro2_type13_shots, Type13AimRuntime},
};

fn session() -> GameSession {
    let mut session = GameSession::init(&v2k_test_support::retail_dir()).expect("retail session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session.load_level_by_id(50, 1).expect("Intro2");
    session
}

fn point_ahead(manager: &EntityManager, id: u32) -> [i16; 3] {
    let source = manager.iter_all().find(|e| e.id == id).unwrap();
    let RetailRuntimeValue::Known(basis) = source.physical_body_basis_q31() else {
        panic!("source retains the incoming physical matrix");
    };
    // A small lateral offset avoids the quantized collinear dot-product edge.
    std::array::from_fn(|axis| {
        source.position_raw()[axis].wrapping_add(
            ((i64::from(basis.forward[axis]) * 2048 + i64::from(basis.lateral[axis]) * 512) >> 31)
                as i16,
        )
    })
}

fn prepare_gun_target(session: &GameSession) -> (EntityManager, u32, u32) {
    let (_, mut manager, owner) = captured_intro2_type13(session);
    let id = owner.entity_id();
    let target_id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(54))
        .expect("authored Intro2 gun")
        .id;
    let target_position = point_ahead(&manager, id);
    let target = manager.entity_mut(target_id).unwrap();
    assert_eq!(target.entity_type, 102);
    assert_ne!(target.capability_flags & 0xC05, 0);
    target.active = true;
    target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
    target.set_motion_raw(target_position, [0; 3]);

    // Exercise the real selector and ADE0 handoff, including the newly
    // installed Tertiary's same-pass Aim. No Type102 task owner is required
    // to read an existing target allocation's state, XYZ and velocity.
    let first = tick_intro2_type13_scheduler_owner(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(session, 20_000)),
    );
    assert!(
        matches!(
            first.outcome,
            Intro2Type13SchedulerProductionOutcome::B6c0Visit {
                acquisition: SearchAttackLiveAcquisitionOutcome::Applied { .. },
                aim: Some(Ok(_)),
                ..
            }
        ),
        "Type102 acquisition must complete its same-pass Aim: {:?}",
        first.outcome,
    );
    assert!(first.retained_owner.is_some());
    let source = manager.entity_mut(id).unwrap();
    assert!(matches!(source.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == target_id));
    assert!(matches!(source.actor_task_state(ActorTaskSlot::Tertiary),
        Some(ActorTaskRuntime::AimAndFire(task)) if task.elapsed_ms() == 20));
    source.intro2_type13_aim_runtime = Some(
        Type13AimRuntime::from_type13_descriptor(TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR)
            .unwrap(),
    );
    let target_position = point_ahead(&manager, id);
    manager
        .entity_mut(target_id)
        .unwrap()
        .set_motion_raw(target_position, [0; 3]);
    (manager, id, target_id)
}

fn aim_age(manager: &EntityManager, id: u32) -> u32 {
    let source = manager.iter_all().find(|e| e.id == id).unwrap();
    let Some(ActorTaskRuntime::AimAndFire(task)) = source.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("retained Aim wrapper");
    };
    task.elapsed_ms()
}

#[v2k_test_support::retail_test]
fn type102_acquisition_fires_and_drains_type13_method10() {
    let session = session();
    let metadata = intro2_type_metadata(&session);
    let (mut manager, id, target_id) = prepare_gun_target(&session);
    let before = aim_age(&manager, id);
    let mut fx = WorldFx::new();
    for _ in 0..8 {
        fx.advance_frame_pacing(20_000);
    }
    let result = tick_intro2_type13_aim(
        CommonMoverDispatchMode::Normal,
        &mut manager,
        &mut fx,
        id,
        600_000,
        metadata.get(13),
    )
    .unwrap();
    assert_eq!(aim_age(&manager, id), before + 600);
    assert!(result.queued_shots_added > 0);
    let target = manager.entity_mut(target_id).unwrap();
    assert_eq!(target.entity_type, 102);
    let source = manager.entity_mut(id).unwrap();
    let runtime = source.intro2_type13_aim_runtime.as_ref().unwrap();
    assert_eq!(runtime.emitter_runtime().cadence_raw, 600_000);
    assert!(runtime
        .transient_shots()
        .iter()
        .all(|shot| shot.projectile_method == 10
            && shot.speed_field == GenericEmitterSpeedField::Explicit(2400)));
    fx.process_pending();
    assert!(fx
        .take_positional_sounds()
        .iter()
        .any(|sound| sound.sound_id == 75));
    let drained =
        drain_intro2_type13_shots(&mut manager, &mut fx, id, ParticleEnvironment::Dry, 4028)
            .unwrap();
    assert_eq!(drained.consumed_requests, result.queued_shots_added);
    assert_eq!(
        drained.materialized_particle_classes,
        vec![38; result.queued_shots_added]
    );
}

#[v2k_test_support::retail_test]
fn type102_aim_reads_live_velocity_without_target_component_or_health_evidence() {
    let session = session();
    let metadata = intro2_type_metadata(&session);
    let mut directions = Vec::new();
    for velocity in [[0; 3], [700, -350, 900]] {
        let (mut manager, id, target_id) = prepare_gun_target(&session);
        let target = manager.entity_mut(target_id).unwrap();
        target.set_motion_raw(target.position_raw(), velocity);
        target.collision.health_raw = RetailRuntimeValue::Unresolved;
        target.model_index = None;
        // 02300 only needs a known nonzero state and a known-clear dying bit.
        target.collision.state_flags_at_0x08 = RetailStateWord::unknown();
        target.collision.state_flags_at_0x08.overwrite(0x4001, 1);
        let mut fx = WorldFx::new();
        let result = tick_intro2_type13_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            600_000,
            metadata.get(13),
        )
        .unwrap();
        assert!(result.queued_shots_added > 0);
        let source = manager.entity_mut(id).unwrap();
        directions.push(
            source
                .intro2_type13_aim_runtime
                .as_ref()
                .unwrap()
                .transient_shots()
                .iter()
                .map(|shot| shot.direction_raw)
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            manager.entity_mut(target_id).unwrap().velocity_raw(),
            velocity
        );
    }
    assert_eq!(directions[0].len(), directions[1].len());
    assert_ne!(
        directions[0], directions[1],
        "424892 must reread the gun's signed velocity for lead"
    );
}

#[v2k_test_support::retail_test]
fn type102_invalid_target_states_preserve_tag_and_unresolved_boundaries() {
    let session = session();
    let metadata = intro2_type_metadata(&session);
    for (active, state, reason) in [
        (
            false,
            RetailStateWord::unknown(),
            Some(AimAndFireInvalidTargetReason::Missing),
        ),
        (
            true,
            RetailStateWord::exact(0),
            Some(AimAndFireInvalidTargetReason::ZeroStateFlags),
        ),
        (
            true,
            RetailStateWord::exact(0x4000),
            Some(AimAndFireInvalidTargetReason::Dying),
        ),
        (true, RetailStateWord::unknown(), None),
    ] {
        let (mut manager, id, target_id) = prepare_gun_target(&session);
        let target = manager.entity_mut(target_id).unwrap();
        target.active = active;
        target.collision.state_flags_at_0x08 = state;
        let before = aim_age(&manager, id);
        let mut fx = WorldFx::new();
        let mut expected_rng = WorldFx::new();
        let result = tick_intro2_type13_aim(
            CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut fx,
            id,
            20_000,
            metadata.get(13),
        );
        if let Some(expected) = reason {
            assert!(matches!(result.unwrap().resolution.outcome,
                AimAndFireFrameOutcome::RequestOwnerTransition {
                    reason: AimAndFireTransitionReason::TaggedInvalidTarget { reason, .. },
                } if reason == expected));
        } else {
            assert_eq!(result, Err(Type13AimError::TargetStateUnresolved));
        }
        assert_eq!(aim_age(&manager, id), before + 20);
        let runtime = manager
            .entity_mut(id)
            .unwrap()
            .intro2_type13_aim_runtime
            .as_ref()
            .unwrap();
        assert_eq!(runtime.queued_shot_count(), 0);
        assert_eq!(runtime.emitter_runtime().cadence_raw, 0);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            expected_rng.next_shared_retail_random_u16()
        );
        fx.process_pending();
        assert!(fx.take_positional_sounds().is_empty());
    }
}
