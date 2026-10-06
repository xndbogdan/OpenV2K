//! Direct `0xDF` hull warning: layout 14 centers, formatter 15 blinks.

use v2k_game::gameplay_notifications::{
    fun_00452270_case_e_hides_line, GameplayNotifications, TextTypewriterCadence, HULL_LOW_TEXT_ID,
};

const LOW_HULL: &str = "\t\t<*,3000,14, *,15>Your ship is critically damaged";

#[test]
fn authored_hull_low_centers_and_blinks_on_formatter_f() {
    assert!(fun_00452270_case_e_hides_line(100));
    assert!(!fun_00452270_case_e_hides_line(140));

    let mut notifications = GameplayNotifications::new();
    let mut cadence = TextTypewriterCadence::default();
    notifications.queue_hull_low(100);

    let hidden = notifications.presentation(100, &mut cadence, |id| {
        (id == HULL_LOW_TEXT_ID).then_some(LOW_HULL)
    });
    assert!(hidden.lines.is_empty());
    assert!(!hidden.play_typewriter_sound);

    let shown = notifications.presentation(140, &mut cadence, |id| {
        (id == HULL_LOW_TEXT_ID).then_some(LOW_HULL)
    });
    assert_eq!(shown.lines.len(), 1);
    assert_eq!(shown.lines[0].text, "Your ship is critically damaged");
    assert!(shown.lines[0].center_x);
    assert_eq!(shown.lines[0].baseline_percent, 78);
    assert!(!shown.play_typewriter_sound);
}
