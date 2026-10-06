use super::*;
use v2k_game::intro2_type13_aim::Type13AimRuntime;

#[v2k_test_support::retail_test]
fn aim_and_forward_plane_use_retained_basis_before_the_outer_rebuild() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session.load_level_by_id(50, 1).expect("normal-tier Intro2");
    let metadata = intro2_type_metadata(&session);

    // Both directions disagree with the changed Euler heading. The forward
    // case needs both E4D0 and E930 to use the retained matrix to reach the
    // shot FIFO; the reverse case must remain rejected despite the new yaw.
    for target_direction in [1_i64, -1] {
        let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
        let id = owner.entity_id();
        let source = manager.entity_mut(id).expect("pursuing Type-13");
        let target_id = match source.actor_task_state(ActorTaskSlot::Primary) {
            Some(ActorTaskRuntime::ChaseTarget(chase)) => chase.target_id(),
            other => panic!("expected retained Chase, got {other:?}"),
        };
        let basis_before = source.physical_body_basis_q31();
        let RetailRuntimeValue::Known(basis) = basis_before else {
            panic!("constructor/outer owner retains the physical matrix");
        };
        let heading_before = source.heading_raw();
        source.heading += std::f32::consts::PI;
        assert_eq!(source.heading_raw(), heading_before.wrapping_add(0x8000));
        source.set_motion_raw([0; 3], [0; 3]);
        source.intro2_type13_aim_runtime = Some(
            Type13AimRuntime::from_type13_descriptor(TYPE13_SEARCH_ATTACK_PROJECTILE_DESCRIPTOR)
                .expect("audited Type-13 Sub-E"),
        );

        // Keep a small lateral offset: an exactly collinear target can make
        // retail's quantized Q31 normalization/dot wrap across the sign bit.
        let target_position = std::array::from_fn(|axis| {
            (((i64::from(basis.forward[axis]) * 2048 + i64::from(basis.lateral[axis]) * 512)
                * target_direction)
                >> 31) as i16
        });
        let target = manager
            .entity_mut(target_id)
            .expect("retained type-9 target");
        target.set_motion_raw(target_position, [0; 3]);
        target.active = true;
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);

        let mut world_fx = WorldFx::new();
        let tick = tick_intro2_type13_aim(
            v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode::Normal,
            &mut manager,
            &mut world_fx,
            id,
            600_000,
            metadata.get(TYPE13_ENTITY_TYPE as usize),
        )
        .expect("cached-basis Aim visit");
        assert!(matches!(
            tick.resolution.outcome,
            AimAndFireFrameOutcome::Continue,
        ));
        let source = manager.entity_mut(id).expect("surviving Type-13");
        assert_eq!(source.physical_body_basis_q31(), basis_before);
        let runtime = source.intro2_type13_aim_runtime.as_ref().unwrap();
        assert_eq!(tick.queued_shots_added, runtime.queued_shot_count());
        if target_direction == 1 {
            assert!(
                tick.queued_shots_added > 0,
                "both aim and forward-plane gates must use the cached forward direction",
            );
            assert!(runtime
                .transient_shots()
                .iter()
                .all(|shot| shot.projectile_method == 10));
        } else {
            assert_eq!(
                tick.queued_shots_added, 0,
                "a target behind the cached matrix stays rejected even after the angle words turn",
            );
        }
    }
}
