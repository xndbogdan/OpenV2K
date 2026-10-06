use super::*;
use crate::{
    actor_task_dispatcher::ActorTaskRuntime,
    actor_task_owner::ActorTaskSlot,
    entity_collision_state::RetailStateWord,
    gkl_common_mover::GklCommonMoverBlock,
    intro2_type10::{
        native::tests::{fixture, generic, publish},
        tick_intro2_type10, Intro2Type10Frame, Intro2Type10Outcome, Intro2Type10Owner,
    },
    sub_g_runtime::{SubG06070RuntimeState, Type13SubGFrameBlock},
    world_fx::WorldFx,
};

fn mover_frame<'a>(
    metadata: &'a EntityTypeRuntimeMetadata,
    terrain: &'a TerrainGrid,
) -> MoverFrame<'a> {
    MoverFrame {
        metadata,
        terrain,
        active_model_extent_raw: 660,
        dispatch_mode: CommonMoverDispatchMode::Normal,
        elapsed_micros: 20_000,
        global_elapsed_micros: 20_000,
        retail_tick: 1,
    }
}

fn tracked(position_raw: [i16; 3]) -> RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>> {
    RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
        state_flags: RetailStateWord::exact(0x8000),
        position_raw,
        velocity_raw: [0; 3],
    }))
}

#[v2k_test_support::retail_test]
fn native_type10_mover_basis_failure_keeps_prelude_l_target_and_g_direction_only() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &metadata, 55);
    let entity = manager.entity_mut(id).unwrap();
    let before_runtime = entity.intro2_type10_runtime.unwrap();
    let before_position = entity.position_raw();
    let before_angles = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        unreachable!()
    };
    sub_g.apply_direction_reverse_write(true);
    let mut expected_g = *sub_g;
    expected_g.apply_direction_reverse_write(false);
    let mut target = WanderNearPrivateState::tracked_entity(before_position, 999);
    target.direction = -1;
    target.reversal_timer_ms = 1;
    let target_position = [
        before_position[0].wrapping_add(500),
        1800,
        before_position[2].wrapping_add(1500),
    ];
    let mut fx = WorldFx::new();
    let mut unchanged_rng = fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        run(
            entity,
            mover_frame(&metadata[10], session.cache.level_terrain().unwrap()),
            &mut target,
            tracked(target_position),
            &mut fx
        ),
        Err(Intro2Type10Block::Mover(
            GklCommonMoverBlock::BodyBasisUnavailable
        ))
    );
    // 01430 tests direction/timer before expiry. This visit clears reverse,
    // but does not refresh the tracked position until the following visit.
    assert_eq!(target.target_position_raw, before_position);
    assert_eq!(target.direction, 1);
    assert_eq!(target.reversal_timer_ms, 0);
    let runtime = entity.intro2_type10_runtime.unwrap();
    assert_eq!(
        runtime,
        Intro2Type10Runtime {
            sub_l_target_raw: before_position,
            ..before_runtime
        }
    );
    assert_eq!(
        entity.sub_g_06070_runtime,
        RetailRuntimeValue::Known(Some(expected_g))
    );
    assert_eq!(
        entity.physical_body_basis_q31,
        RetailRuntimeValue::Unresolved
    );
    assert_eq!(entity.position_raw(), before_position);
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), before_angles);
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        unchanged_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_mover_g_failure_keeps_sub_d_and_post_writes_without_output_dispatch() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &metadata, 55);
    let entity = manager.entity_mut(id).unwrap();
    let before = entity.intro2_type10_runtime.unwrap();
    let basis = entity.physical_body_basis_q31;
    let position = entity.position_raw();
    let angles = entity.rotation_heading_pitch_roll_raw();
    let target_position = [
        position[0].wrapping_add(500),
        1800,
        position[2].wrapping_add(1500),
    ];
    let mut target = WanderNearPrivateState::tracked_entity(position, 999);
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()));
    let mut fx = WorldFx::new();
    assert_eq!(
        run(
            entity,
            mover_frame(&metadata[10], session.cache.level_terrain().unwrap()),
            &mut target,
            tracked(target_position),
            &mut fx
        ),
        Err(Intro2Type10Block::Mover(GklCommonMoverBlock::SubG(
            Type13SubGFrameBlock::RuntimeUnavailable
        )))
    );
    let runtime = entity.intro2_type10_runtime.unwrap();
    assert_ne!(runtime.sub_d_runtime.last_yaw_step_raw, 0);
    assert_eq!(
        runtime
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        0x23
    );
    assert_eq!(runtime.sub_l_target_raw, target_position);
    assert_ne!(runtime.sub_k_smoothed_raw, before.sub_k_smoothed_raw);
    assert_ne!(runtime.sub_l_exact_raw, before.sub_l_exact_raw);
    assert_eq!(runtime.sub_e_runtime, before.sub_e_runtime);
    assert_eq!(runtime.sub_k_output_raw, before.sub_k_output_raw);
    assert_eq!(runtime.sub_l_output_raw, before.sub_l_output_raw);
    assert_eq!(entity.physical_body_basis_q31, basis);
    let step = runtime.sub_d_runtime.last_yaw_step_raw;
    assert_eq!(
        entity.rotation_heading_pitch_roll_raw(),
        [
            angles[0].wrapping_sub(step),
            angles[1],
            angles[2].wrapping_sub(step)
        ]
    );
    assert_eq!(
        entity.sub_g_06070_runtime,
        RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()))
    );
    assert!(fx.take_positional_sounds().is_empty());
}

#[v2k_test_support::retail_test]
fn native_type10_direct_zero_mass_keeps_g_animation_prefix_without_output_dispatch() {
    let Some((session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &metadata, 55);
    let entity = manager.entity_mut(id).unwrap();
    // Exercise the detached B210 error boundary explicitly. The outer12DA0
    // scheduler clamps a wrapped zero to one, so B2 cannot reach this error.
    entity.mass_raw = 0;
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        unreachable!()
    };
    // Reach the real powered wing phase at ordinary mass. Type10's idle
    // force is zero, so a newborn zero-mass G visit would correctly skip B210.
    let mut powered_g = *sub_g;
    let position = entity.position_raw();
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31 else {
        unreachable!()
    };
    let mut reached = false;
    for _ in 0..256 {
        let outcome = crate::sub_g_runtime::plan_type13_sub_g_frame(
            crate::sub_g_runtime::Type13SubGFrameRequest {
                descriptor: &SUB_G,
                runtime: powered_g,
                reverse_write: None,
                position_raw: position,
                velocity_raw: [0; 3],
                pitch_raw: 0,
                roll_raw: 0,
                retained_body_basis: basis,
                active_model_extent_raw: 660,
                self_mass_raw: 100,
                attached_cargo_mass: 0,
                capability_flags: 8,
                terrain: session.cache.level_terrain().unwrap(),
                retail_tick: 1,
                elapsed_micros: 20_000,
            },
        )
        .unwrap();
        if outcome.runtime.source_raw_at_0x20() != RetailRuntimeValue::Known(0) {
            // Keep the incoming state: the tested callback must enter this
            // powered window itself, rather than advancing beyond it.
            reached = true;
            break;
        }
        powered_g = outcome.runtime;
    }
    assert!(reached, "normal G phase reaches its powered window");
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(powered_g));
    let RetailRuntimeValue::Known(Some(sub_g)) = &mut entity.sub_g_06070_runtime else {
        unreachable!()
    };
    sub_g.apply_collision_pulse_1b9d0();
    let before_g = *sub_g;
    let before = entity.intro2_type10_runtime.unwrap();
    let target_position = [
        position[0].wrapping_add(500),
        1800,
        position[2].wrapping_add(1500),
    ];
    let mut target = WanderNearPrivateState::tracked_entity(position, 999);
    let mut fx = WorldFx::new();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    assert_eq!(
        run(
            entity,
            mover_frame(&metadata[10], session.cache.level_terrain().unwrap()),
            &mut target,
            tracked(target_position),
            &mut fx,
        ),
        Err(Intro2Type10Block::Mover(GklCommonMoverBlock::SubG(
            Type13SubGFrameBlock::ZeroTotalMass,
        )))
    );
    let runtime = entity.intro2_type10_runtime.unwrap();
    assert_eq!(runtime.sub_l_target_raw, target_position);
    assert_eq!(
        runtime
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        0x23
    );
    assert_eq!(runtime.sub_k_output_raw, before.sub_k_output_raw);
    assert_eq!(runtime.sub_l_output_raw, before.sub_l_output_raw);
    let RetailRuntimeValue::Known(Some(sub_g)) = entity.sub_g_06070_runtime else {
        unreachable!()
    };
    assert_eq!(
        sub_g.animation_outputs_raw().map(|words| words[6]),
        RetailRuntimeValue::Known(0x3000)
    );
    assert_ne!(
        sub_g.decaying_output_raw_at_0x30(),
        before_g.decaying_output_raw_at_0x30()
    );
    assert_ne!(sub_g.source_raw_at_0x20(), RetailRuntimeValue::Known(0));
    assert_eq!(
        entity.physical_body_basis_q31,
        RetailRuntimeValue::Known(basis)
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
}

#[v2k_test_support::retail_test]
fn native_type10_scheduler_g_failure_keeps_task_prefix_and_rng_without_replay() {
    let Some((mut session, metadata)) = fixture() else {
        return;
    };
    let mut manager = generic(&session, &metadata);
    let id = publish(&mut manager, &metadata, 55);
    let entity = manager.entity_mut(id).unwrap();
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(0x0202_0000, 0x0202_0000);
    entity.collision.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
    let wrapped_zero_b2 = 0u16.wrapping_sub(100);
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(wrapped_zero_b2);
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(SubG06070RuntimeState::pending()));
    let before = entity.intro2_type10_runtime.unwrap();
    let primary = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .unwrap();
    let owner = Intro2Type10Owner::adopt(&manager, id).unwrap();
    let mut fx = WorldFx::new();
    let mut expected_rng = fx.fork_for_main_base_abort_transaction();
    for _ in 0..2 {
        expected_rng.next_shared_retail_random_u16();
    }
    let tick = tick_intro2_type10(
        &mut manager,
        owner,
        Intro2Type10Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 20_000,
            retail_tick: 1,
        },
    );
    assert!(
        matches!(
            tick.outcome,
            Intro2Type10Outcome::Blocked {
                reason: Intro2Type10Block::Mover(GklCommonMoverBlock::SubG(
                    Type13SubGFrameBlock::RuntimeUnavailable
                )),
                prefix_committed: true,
                ..
            }
        ),
        "{:?}",
        tick.outcome
    );
    let owner = tick.retained_owner.unwrap();
    assert!(owner.has_pending_prefix());
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(
        entity.mass_raw, 1,
        "12DA0 clamps the wrapped zero before the callback"
    );
    assert_eq!(
        entity.collision.animation_offset_at_0xb2,
        RetailRuntimeValue::Known(wrapped_zero_b2),
        "the post-callback B2 clear has not been reached"
    );
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity.actor_task_state(ActorTaskSlot::Primary).copied()
    else {
        panic!("retained Primary")
    };
    assert_eq!(task.elapsed_ms(), 20);
    assert!(
        !entity
            .actor_tasks
            .wrapper_flags(primary)
            .unwrap()
            .in_callback
    );
    let runtime = entity.intro2_type10_runtime.unwrap();
    assert_eq!(
        runtime.sub_l_target_raw,
        task.private_state().target_position_raw
    );
    assert_eq!(
        runtime
            .sub_d_frame_owner
            .classifier_cache()
            .stagger_counter(),
        0x23
    );
    assert_eq!(runtime.sub_k_output_raw, before.sub_k_output_raw);
    assert_eq!(runtime.sub_l_output_raw, before.sub_l_output_raw);
    let frozen_g = entity.sub_g_06070_runtime;
    let frozen_angles = entity.rotation_heading_pitch_roll_raw();
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        expected_rng.next_shared_retail_random_u16()
    );
    let mut frozen_rng = fx.fork_for_main_base_abort_transaction();
    let again = tick_intro2_type10(
        &mut manager,
        owner,
        Intro2Type10Frame {
            resources: &mut session.cache,
            world_fx: &mut fx,
            elapsed_micros: 125_000,
            retail_tick: 8,
        },
    );
    assert!(matches!(again.outcome, Intro2Type10Outcome::Pending { .. }));
    let entity = manager.entity_mut(id).unwrap();
    assert_eq!(entity.sub_g_06070_runtime, frozen_g);
    assert_eq!(entity.intro2_type10_runtime, Some(runtime));
    assert_eq!(entity.rotation_heading_pitch_roll_raw(), frozen_angles);
    assert_eq!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(&ActorTaskRuntime::SharedRetarget(task))
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        frozen_rng.next_shared_retail_random_u16()
    );
}
