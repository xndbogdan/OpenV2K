use super::*;
use v2k_game::{
    common_mover::type9_attitude::Type9BodyBasis,
    entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
};

fn intro2_session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session.load_level_by_id(50, 1).expect("normal-tier Intro2");
    session
}

fn basis_from_post_task_angles(
    entity: &v2k_game::entity::Entity,
) -> RetailRuntimeValue<Type9BodyBasis> {
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll))
}

#[v2k_test_support::retail_test]
fn complete_pass_rebuilds_after_the_callback_and_feeds_later_sub_g_visits() {
    let session = intro2_session();
    let (metadata, mut live, mut owner) = captured_intro2_type13(&session);
    let (_, mut callback_only, callback_owner) = captured_intro2_type13(&session);
    let id = owner.entity_id();
    assert_eq!(id, callback_owner.entity_id());
    for manager in [&mut live, &mut callback_only] {
        let entity = manager.entity_mut(id).expect("captured Type-13");
        assert_eq!(
            entity.collision.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(8),
        );
        entity.collision.state_flags_at_0x08.overwrite(
            BODY_BASIS_REBUILT_STATE_BIT | TYPE13_C690_SUPPRESS_STATE_BIT,
            TYPE13_C690_SUPPRESS_STATE_BIT,
        );
        entity.set_motion_raw(INTRO2_TYPE13_POSITION_RAW, [0, -500, 1200]);
    }
    let original_basis = live.entity_mut(id).unwrap().physical_body_basis_q31();
    let mut world_fx = WorldFx::new();
    for pass in 0..15 {
        let callback = tick_intro2_type13_primary(
            callback_only.entity_mut(id).expect("callback-only control"),
            metadata.get(TYPE13_ENTITY_TYPE as usize),
            primary_frame(&session, 32_768),
            &mut || 1,
        )
        .expect("normal Primary admission");
        assert_eq!(callback.result, Intro2Type13PrimaryVisitResult::Continue);
        let tick = tick_intro2_type13_scheduler_owner_with_random(
            &mut live,
            owner,
            &mut world_fx,
            Some(primary_frame(&session, 32_768)),
            &mut |_| 1,
        );
        assert!(matches!(
            tick.outcome,
            Intro2Type13SchedulerProductionOutcome::B6c0Visit {
                primary: v2k_game::intro2_type13_live::Intro2Type13PrimaryVisit {
                    result: Intro2Type13PrimaryVisitResult::Continue,
                    ..
                },
                ..
            }
        ));
        owner = tick.retained_owner.expect("no-target B6C0 remains live");
        let entity = live.entity_mut(id).unwrap();
        assert_eq!(
            entity.physical_body_basis_q31(),
            basis_from_post_task_angles(entity)
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT),
        );
        let control = callback_only.entity_mut(id).unwrap();
        assert_eq!(control.physical_body_basis_q31(), original_basis);
        if pass == 0 {
            // Both first callbacks read the constructor matrix. The enclosing
            // traversal publishes its new matrix only after those writes.
            assert_eq!(entity.velocity_raw(), control.velocity_raw());
            assert_eq!(
                entity.rotation_heading_pitch_roll_raw(),
                control.rotation_heading_pitch_roll_raw()
            );
            assert_eq!(entity.sub_g_06070_runtime, control.sub_g_06070_runtime);
            assert_ne!(entity.physical_body_basis_q31(), original_basis);
        }
    }
    let entity = live.entity_mut(id).unwrap();
    let frozen = callback_only.entity_mut(id).unwrap();
    assert_ne!(
        entity.velocity_raw(),
        frozen.velocity_raw(),
        "later Sub-G force must consume the refreshed body axes"
    );
    assert_eq!(
        entity.position_raw(),
        INTRO2_TYPE13_POSITION_RAW,
        "master motion remains a separate owner"
    );
}

#[v2k_test_support::retail_test]
fn class5_complete_pass_publishes_its_post_task_basis_without_secondary() {
    let session = intro2_session();
    let (_, mut manager, owner) = captured_intro2_type13(&session);
    let owner = reselect_class5_from_b6c0(&session, &mut manager, owner);
    let id = owner.entity_id();
    let before = manager.entity_mut(id).unwrap().physical_body_basis_q31();
    let tick = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type13SchedulerProductionOutcome::Class5AimlessVisit { .. }
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.physical_body_basis_q31(),
        basis_from_post_task_angles(entity)
    );
    assert_ne!(entity.physical_body_basis_q31(), before);
    assert_eq!(entity.actor_task_state(ActorTaskSlot::Secondary), None);
}

#[v2k_test_support::retail_test]
fn unavailable_flags_or_unported_attitude_block_before_callback_or_random() {
    use v2k_game::intro2_type13_live::Intro2Type13PostTaskBasisBlock;
    let session = intro2_session();
    for (flags, reason) in [
        (
            RetailRuntimeValue::Unresolved,
            Intro2Type13PostTaskBasisBlock::EffectiveFlagsUnavailable,
        ),
        (
            RetailRuntimeValue::Known(0x18),
            Intro2Type13PostTaskBasisBlock::TerrainAttitudeUnsupported {
                effective_flags: 0x18,
            },
        ),
    ] {
        let (_, mut manager, owner) = captured_intro2_type13(&session);
        let id = owner.entity_id();
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.default_state_flags_at_0xc8 = flags;
        let primary_before = entity.actor_task_state(ActorTaskSlot::Primary).copied();
        let basis_before = entity.physical_body_basis_q31();
        let tick = tick_intro2_type13_scheduler_owner_with_random(
            &mut manager,
            owner,
            &mut WorldFx::new(),
            Some(primary_frame(&session, 20_000)),
            &mut |_| panic!("basis preflight must precede callback RNG"),
        );
        assert!(
            matches!(tick.outcome, Intro2Type13SchedulerProductionOutcome::PostTaskBasisBlocked { reason: blocked, .. } if blocked == reason)
        );
        assert!(tick.retained_owner.is_some());
        let entity = manager.entity_mut(id).unwrap();
        assert_eq!(
            entity.actor_task_state(ActorTaskSlot::Primary).copied(),
            primary_before
        );
        assert_eq!(entity.physical_body_basis_q31(), basis_before);
    }
}

#[v2k_test_support::retail_test]
fn explicit_matrix_owner_flag_preserves_the_existing_basis_and_rebuild_bit() {
    let session = intro2_session();
    let (_, mut manager, owner) = captured_intro2_type13(&session);
    let id = owner.entity_id();
    let entity = manager.entity_mut(id).unwrap();
    entity.collision.default_state_flags_at_0xc8 = RetailRuntimeValue::Known(0x4008);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, 0);
    let before = entity.physical_body_basis_q31();
    let tick = tick_intro2_type13_scheduler_owner_with_random(
        &mut manager,
        owner,
        &mut WorldFx::new(),
        Some(primary_frame(&session, 20_000)),
        &mut |_| 1,
    );
    assert!(matches!(
        tick.outcome,
        Intro2Type13SchedulerProductionOutcome::B6c0Visit { .. }
    ));
    let entity = manager.entity_mut(id).unwrap();
    assert_ne!(
        basis_from_post_task_angles(entity),
        before,
        "the callback changed angle words"
    );
    assert_eq!(entity.physical_body_basis_q31(), before);
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(BODY_BASIS_REBUILT_STATE_BIT),
        RetailRuntimeValue::Known(0)
    );
}
