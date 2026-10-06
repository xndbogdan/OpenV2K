use super::*;
use v2k_game::common_mover::component_dispatch::CommonMoverDispatchMode;

#[v2k_test_support::retail_test]
fn restricted_aim_ages_and_expires_without_target_or_emitter_inputs() {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail session");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("normal system tier");
    session.load_level_by_id(50, 1).expect("normal-tier Intro2");
    let (mut manager, owner) = captured_intro2_type13_pursuing(&session);
    let source = manager.entity_mut(owner.entity_id()).expect("Type-13");
    let Some(ActorTaskRuntime::AimAndFire(aim)) = source.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("pursuing graph has Aim");
    };
    let target_id = aim.private_state().target_entity_id();
    let elapsed_before = aim.elapsed_ms();
    let private_before = aim.private_state();
    let basis_before = source.physical_body_basis_q31();

    // Retain the authentic published graph while withholding the emitter and
    // making the live target state unknown. The nonzero 2300 mode reads neither.
    source.intro2_type13_aim_runtime = None;
    manager
        .entity_mut(target_id)
        .expect("target")
        .collision
        .state_flags_at_0x08 = RetailStateWord::unknown();

    let mut world_fx = WorldFx::new();
    let tick = tick_intro2_type13_aim(
        CommonMoverDispatchMode::Restricted,
        &mut manager,
        &mut world_fx,
        owner.entity_id(),
        123_999,
        None,
    )
    .expect("coarse mode needs no target or Sub-E inputs");
    assert_eq!(tick.queued_shots_added, 0);
    assert_eq!(tick.resolution.prefix.elapsed_ms, elapsed_before + 123);
    assert_eq!(tick.resolution.outcome, AimAndFireFrameOutcome::Continue);

    let expired = tick_intro2_type13_aim(
        CommonMoverDispatchMode::Restricted,
        &mut manager,
        &mut world_fx,
        owner.entity_id(),
        5_001_000,
        None,
    )
    .expect("coarse mode still resolves lifetime expiry after wrapper unwind");
    assert_eq!(expired.queued_shots_added, 0);
    assert_eq!(
        expired.resolution.outcome,
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired,
        },
    );
    let source = manager
        .entity_mut(owner.entity_id())
        .expect("retained Type-13");
    let Some(ActorTaskRuntime::AimAndFire(aim)) = source.actor_task_state(ActorTaskSlot::Tertiary)
    else {
        panic!("coarse callback retains the same Aim task");
    };
    assert_eq!(aim.private_state(), private_before);
    assert_eq!(aim.elapsed_ms(), elapsed_before + 123 + 5_001);
    assert_eq!(source.physical_body_basis_q31(), basis_before);
    assert!(source.intro2_type13_aim_runtime.is_none());
    assert!(world_fx.take_positional_sounds().is_empty());
    assert_eq!(
        world_fx.next_shared_retail_random_u16(),
        WorldFx::new().next_shared_retail_random_u16(),
        "coarse Aim consumes no shared RNG",
    );
}
