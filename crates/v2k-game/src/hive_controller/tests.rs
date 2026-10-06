use super::*;

#[test]
fn wreck_retry_inner_and_outer_gates_are_strict_in_original_wrapped_raw_coordinates() {
    for (position, expected) in [
        ([0, -129, 0], true),
        ([255, -129, 0], true),
        ([256, -129, 0], false),
        ([181, -129, 181], true),
        ([182, -129, 182], false),
        ([0, -128, 0], false),
        ([0, -127, 0], false),
        ([0, -7_999, 0], true),
        ([0, -8_000, 0], false),
        ([0, i16::MIN, 0], false),
    ] {
        assert_eq!(
            hive_wreck_retry_requested([0; 3], position, true),
            expected,
            "{position:?}"
        );
        assert!(!hive_wreck_retry_requested([0; 3], position, false));
    }
    assert!(hive_wreck_retry_requested(
        [32_760, 32_760, -32_760],
        [-32_768, 32_631, 32_767],
        true,
    ));
}
use crate::{entity::EntityKind, entity_collision_state::RetailStateWord};

fn step(
    state: &mut u32,
    timer: &mut i32,
    health: &mut i32,
    authored: i32,
    hostile: bool,
    elapsed: u32,
) -> Option<HiveHealthTransition> {
    advance_live_health(
        state,
        HiveLiveHealth {
            health_raw: health,
            authored_health_raw: authored,
            grace_timer_us: timer,
        },
        hostile,
        elapsed,
    )
}

#[test]
fn signed_timer_preserves_hostile_oscillation_and_negative_abort_prefix() {
    let (mut state, mut timer, mut health) = (1, 0, 735);
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 735, true, 20_000),
        Some(HiveHealthTransition::Locked)
    );
    assert_eq!((timer, health), (20_000, HIVE_LOCKED_HEALTH_RAW));
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 735, true, 20_000),
        None
    );
    assert_eq!(timer, 0);
    timer = -3_000_000;
    step(&mut state, &mut timer, &mut health, 735, true, 20_000);
    assert_eq!(
        timer, -2_980_000,
        "1CF90's exact +50 word survives and advances while negative"
    );
    step(&mut state, &mut timer, &mut health, 735, false, 4_980_000);
    assert_eq!((state, timer, health), (2, 2_000_000, 735));
}

#[test]
fn exact_threshold_uses_authored_health_and_does_not_rearm_vulnerable_hive() {
    let (mut state, mut timer, mut health) = (1, 0, HIVE_LOCKED_HEALTH_RAW);
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 4_321, false, 1_999_999),
        None
    );
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 4_321, false, 1),
        Some(HiveHealthTransition::Unlocked)
    );
    assert_eq!((state, timer, health), (2, 2_000_000, 4_321));
    health = 200;
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 4_321, true, 20_000),
        None
    );
    assert_eq!((state, timer, health), (2, 0, 200));
    health = 5_000;
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 4_321, true, 20_000),
        Some(HiveHealthTransition::Unlocked)
    );
    assert_eq!(
        health, 4_321,
        "state2 only clamps excess health, never heals ordinary damage"
    );
}

#[test]
fn large_delta_and_signed_wrapping_match_the_literal_native_branch() {
    let (mut state, mut timer, mut health) = (1, 0, 999);
    assert_eq!(
        step(&mut state, &mut timer, &mut health, 999, true, 2_000_000),
        Some(HiveHealthTransition::Unlocked)
    );
    assert_eq!(state, 2, "no second hostile check after the source ADD");
    state = 1;
    timer = 1_000_000;
    health = HIVE_LOCKED_HEALTH_RAW;
    step(
        &mut state,
        &mut timer,
        &mut health,
        999,
        false,
        i32::MAX as u32,
    );
    assert_eq!(timer, i32::MIN + 999_999);
    assert_eq!(state, 1);
}

#[test]
fn objective_predicate_has_no_model_cohort_or_family_whitelist() {
    let mut actor = Entity::unresolved_port_entity(7, EntityKind::Unknown(122), 122);
    actor.capability_flags = OBJECTIVE_CAPABILITY_BIT;
    actor.collision.state_flags_at_0x08 = RetailStateWord::exact(OBJECTIVE_STATE_BIT);
    assert_eq!(objective_hostile_present([&actor]), Ok(true));
    actor.collision.state_flags_at_0x08.overwrite(
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
        ACTIVE_MODEL_SLOT_LOW_STATE_BIT,
    );
    assert_eq!(objective_hostile_present([&actor]), Ok(false));
    actor.collision.state_flags_at_0x08 = RetailStateWord::unknown();
    assert_eq!(
        objective_hostile_present([&actor]),
        Err(ObjectivePredicateBlock::UnresolvedObjectiveState { entity_id: 7 })
    );
    actor.capability_flags = 0;
    assert_eq!(objective_hostile_present([&actor]), Ok(false));
}

#[test]
fn lock_hint_uses_callback_detail_and_phase_not_a_magic_previous_health() {
    for (phase, mode, expected) in [
        (
            GameplayNotificationPhase::Playing,
            ComponentUpdateMode::Detailed,
            true,
        ),
        (
            GameplayNotificationPhase::Playing,
            ComponentUpdateMode::Coarse,
            false,
        ),
        (
            GameplayNotificationPhase::NonGameplay,
            ComponentUpdateMode::Detailed,
            false,
        ),
    ] {
        let mut notifications = GameplayNotifications::new();
        let mut frame = AuthoredHiveComponentFrame {
            elapsed_us: 20_000,
            terrain: None,
            retail_tick: 50,
            notification_phase: phase,
            notifications: &mut notifications,
            world_complete_tally: &mut WorldCompleteTally::default(),
        };
        notify_health_transition(HiveHealthTransition::Locked, mode, &mut frame);
        assert_eq!(
            notifications.save_tail_seen_mask() & (1 << 10) != 0,
            expected
        );
    }
}

#[test]
fn unlock_arms_completion_timestamp_once_even_outside_gameplay() {
    let mut notifications = GameplayNotifications::new();
    let mut tally = WorldCompleteTally::default();
    let mut frame = AuthoredHiveComponentFrame {
        elapsed_us: 0,
        terrain: None,
        retail_tick: 20,
        notification_phase: GameplayNotificationPhase::NonGameplay,
        notifications: &mut notifications,
        world_complete_tally: &mut tally,
    };
    notify_health_transition(
        HiveHealthTransition::Unlocked,
        ComponentUpdateMode::Coarse,
        &mut frame,
    );
    assert_eq!(frame.world_complete_tally.ticks_0x2bc, -1);
    frame.world_complete_tally.ticks_0x2bc = 400;
    notify_health_transition(
        HiveHealthTransition::Unlocked,
        ComponentUpdateMode::Coarse,
        &mut frame,
    );
    assert_eq!(frame.world_complete_tally.ticks_0x2bc, 400);
    assert_eq!(notifications.save_tail_seen_mask(), 0);
}
