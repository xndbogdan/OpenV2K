//! Class-14 `0xC6` remaining-people HUD: formatter 2 and queue-tick duration.

use v2k_game::gameplay_notifications::{
    GameplayNotifications, TextTypewriterCadence, TYPE9_SESSION_ZERO_CLASS14_TEXT_ID,
};
use v2k_game::world_complete_results::{
    fun_0042dd10_selector3_rescued, fun_0042e210_capability_census, Fun0042e210EntitySample,
};

const PEOPLE_LEFT: &str = "\t<*,3000, 4,30, 2>People left: %d";

#[test]
fn people_left_formats_selector3_before_type_on() {
    let mut notifications = GameplayNotifications::new();
    let mut cadence = TextTypewriterCadence::default();
    notifications.queue_type9_session_zero_class14(100);
    notifications.set_people_left(19);

    let typing = notifications.presentation(102, &mut cadence, |id| {
        (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
    });
    assert_eq!(typing.lines.len(), 1);
    assert_eq!(
        typing.lines[0].string_id,
        TYPE9_SESSION_ZERO_CLASS14_TEXT_ID
    );
    assert_eq!(typing.lines[0].text, "P");
    assert_eq!(typing.lines[0].x_percent, 25);
    assert_eq!(typing.lines[0].baseline_percent, 78);

    notifications.set_people_left(6);
    let complete = notifications.presentation(250, &mut cadence, |id| {
        (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
    });
    assert_eq!(complete.lines[0].text, "People left: 6");
}

#[test]
fn people_left_duration_uses_queue_tick_not_level_epoch() {
    let mut notifications = GameplayNotifications::new();
    let mut cadence = TextTypewriterCadence::default();
    notifications.queue_type9_session_zero_class14(0);
    notifications.set_people_left(4);
    let expired = notifications.presentation(151, &mut cadence, |id| {
        (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
    });
    assert!(expired.lines.is_empty());

    notifications.queue_type9_session_zero_class14(150);
    let visible = notifications.presentation(152, &mut cadence, |id| {
        (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
    });
    assert_eq!(visible.lines[0].text, "P");
}

#[test]
fn people_left_count_is_live_natives_after_one_dying_peasant() {
    let live = v2k_game::entity_collision_state::RetailStateWord::exact(0x0800_0004);
    let dying = v2k_game::entity_collision_state::RetailStateWord::exact(
        0x0800_0004 | v2k_game::entity_collision_state::DYING_STATE_BIT,
    );
    let census = fun_0042e210_capability_census([
        Fun0042e210EntitySample {
            capability: 0x1804,
            state: live,
            sub_m_native_count_raw: None,
        },
        Fun0042e210EntitySample {
            capability: 0x1804,
            state: dying,
            sub_m_native_count_raw: None,
        },
        Fun0042e210EntitySample {
            capability: 0x0400,
            state: live,
            sub_m_native_count_raw: Some(2),
        },
    ]);
    assert_eq!(
        fun_0042dd10_selector3_rescued(census.cap_0x400, census.cap_0x800),
        4
    );
}
