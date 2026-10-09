//! Positional game-controller buttons for the controller layouts.
//!
//! SDL names a standard pad's controls by position, so a PlayStation, Xbox,
//! Switch or Steam Deck pad all arrive in the same layout. The triggers and
//! the left stick's four directions are reported as buttons too: retail's
//! console pad had digital L2/R2, and the port's menus can be steered with
//! the stick.

use std::collections::HashMap;

use sdl2::controller::{Axis, Button};

/// One positional control of a standard game controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadButton {
    /// Bottom face button: Cross, A.
    South,
    /// Right face button: Circle, B.
    East,
    /// Left face button: Square, X.
    West,
    /// Top face button: Triangle, Y.
    North,
    /// Select, Back, View.
    Back,
    /// Start, Menu.
    Start,
    LeftStick,
    RightStick,
    /// L1, LB.
    LeftShoulder,
    /// R1, RB.
    RightShoulder,
    /// L2, LT, past `TRIGGER_PRESS`.
    LeftTrigger,
    /// R2, RT, past `TRIGGER_PRESS`.
    RightTrigger,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    /// The left stick pushed past `STICK_PRESS` in one direction.
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
}

/// A trigger counts as pressed past XInput's `XINPUT_GAMEPAD_TRIGGER_THRESHOLD`
/// (30 of 255), scaled to SDL's `0..=32767`.
pub const TRIGGER_PRESS: i16 = 3855;
/// It is released again below half that, so a trigger near the threshold
/// does not chatter.
pub const TRIGGER_RELEASE: i16 = TRIGGER_PRESS / 2;
/// A stick direction counts as pressed past half deflection and released
/// below three eighths.
pub const STICK_PRESS: i16 = 0x4000;
pub const STICK_RELEASE: i16 = 0x3000;
/// Analog motion this large makes its pad the active one even when it
/// presses nothing.
pub const ACTIVITY: i16 = 0x4000;

/// SDL's named button as a positional pad button. The Guide button belongs
/// to the system, and paddles, the touchpad and Misc1 have no V2000 role.
pub fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::A => PadButton::South,
        Button::B => PadButton::East,
        Button::X => PadButton::West,
        Button::Y => PadButton::North,
        Button::Back => PadButton::Back,
        Button::Start => PadButton::Start,
        Button::LeftStick => PadButton::LeftStick,
        Button::RightStick => PadButton::RightStick,
        Button::LeftShoulder => PadButton::LeftShoulder,
        Button::RightShoulder => PadButton::RightShoulder,
        Button::DPadUp => PadButton::DPadUp,
        Button::DPadDown => PadButton::DPadDown,
        Button::DPadLeft => PadButton::DPadLeft,
        Button::DPadRight => PadButton::DPadRight,
        _ => return None,
    })
}

/// Pressed state of the analog controls reported as buttons, for one pad.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnalogLatches {
    left_trigger: bool,
    right_trigger: bool,
    left_up: bool,
    left_down: bool,
    left_left: bool,
    left_right: bool,
}

impl AnalogLatches {
    /// Fold one axis value in and return the buttons it pressed or released.
    pub fn update(&mut self, axis: Axis, value: i16) -> Vec<(PadButton, bool)> {
        let mut changes = Vec::new();
        let mut latch = |state: &mut bool, button: PadButton, pressed: bool| {
            if *state != pressed {
                *state = pressed;
                changes.push((button, pressed));
            }
        };
        let trigger = |pressed, value| hysteresis(pressed, value, TRIGGER_PRESS, TRIGGER_RELEASE);
        let stick = |pressed, value| hysteresis(pressed, value, STICK_PRESS, STICK_RELEASE);
        // SDL's sticks are negative left and up.
        let negative = value.saturating_neg();
        match axis {
            Axis::TriggerLeft => {
                let pressed = trigger(self.left_trigger, value);
                latch(&mut self.left_trigger, PadButton::LeftTrigger, pressed);
            }
            Axis::TriggerRight => {
                let pressed = trigger(self.right_trigger, value);
                latch(&mut self.right_trigger, PadButton::RightTrigger, pressed);
            }
            Axis::LeftX => {
                let left = stick(self.left_left, negative);
                let right = stick(self.left_right, value);
                latch(&mut self.left_left, PadButton::LeftStickLeft, left);
                latch(&mut self.left_right, PadButton::LeftStickRight, right);
            }
            Axis::LeftY => {
                let up = stick(self.left_up, negative);
                let down = stick(self.left_down, value);
                latch(&mut self.left_up, PadButton::LeftStickUp, up);
                latch(&mut self.left_down, PadButton::LeftStickDown, down);
            }
            Axis::RightX | Axis::RightY => {}
        }
        changes
    }
}

fn hysteresis(pressed: bool, value: i16, press: i16, release: i16) -> bool {
    if pressed {
        value >= release
    } else {
        value >= press
    }
}

/// Whether motion on `axis` is deliberate enough to choose this pad.
pub fn is_activity(axis: Axis, value: i16) -> bool {
    match axis {
        Axis::TriggerLeft | Axis::TriggerRight => value >= TRIGGER_PRESS,
        _ => value.unsigned_abs() >= ACTIVITY as u16,
    }
}

/// Which open device the game reads, and the pad buttons it has reported
/// pressed. The device used last becomes active; the buttons still held on
/// the previous one are released first, so every press has its release.
#[derive(Debug, Default)]
pub struct Routing {
    active: Option<u32>,
    held: Vec<PadButton>,
    latches: HashMap<u32, AnalogLatches>,
}

/// One routed change: a pad button of the active device.
pub type Change = (PadButton, bool);

impl Routing {
    pub const fn active(&self) -> Option<u32> {
        self.active
    }

    fn activate(&mut self, device: u32, out: &mut Vec<Change>) {
        if self.active != Some(device) {
            self.release_all(out);
            self.active = Some(device);
        }
    }

    fn report(&mut self, button: PadButton, pressed: bool, out: &mut Vec<Change>) {
        let held = self.held.contains(&button);
        if pressed && !held {
            self.held.push(button);
            out.push((button, true));
        } else if !pressed && held {
            self.held.retain(|&other| other != button);
            out.push((button, false));
        }
    }

    /// Release every held button, for example when the window loses focus.
    pub fn release_all(&mut self, out: &mut Vec<Change>) {
        for button in std::mem::take(&mut self.held) {
            out.push((button, false));
        }
    }

    /// A pad's button. A press makes its pad active; a release counts only
    /// for the active pad.
    pub fn button(&mut self, device: u32, button: PadButton, pressed: bool, out: &mut Vec<Change>) {
        if pressed {
            self.activate(device, out);
        }
        if self.active == Some(device) {
            self.report(button, pressed, out);
        }
    }

    /// A pad's axis: triggers and the left stick's directions are buttons,
    /// and deliberate motion of any axis makes the pad active.
    pub fn axis(&mut self, device: u32, axis: Axis, value: i16, out: &mut Vec<Change>) {
        let changes = self.latches.entry(device).or_default().update(axis, value);
        if is_activity(axis, value) {
            self.activate(device, out);
        }
        if self.active == Some(device) {
            for (button, pressed) in changes {
                self.report(button, pressed, out);
            }
        }
    }

    /// A plain joystick was used: it becomes active, with no pad buttons.
    pub fn joystick_used(&mut self, device: u32, out: &mut Vec<Change>) {
        self.activate(device, out);
    }

    /// A device went away.
    pub fn removed(&mut self, device: u32, out: &mut Vec<Change>) {
        if self.active == Some(device) {
            self.release_all(out);
            self.active = None;
        }
        self.latches.remove(&device);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_press_at_the_xinput_threshold_and_release_with_hysteresis() {
        let mut latches = AnalogLatches::default();
        assert!(latches
            .update(Axis::TriggerRight, TRIGGER_PRESS - 1)
            .is_empty());
        assert_eq!(
            latches.update(Axis::TriggerRight, TRIGGER_PRESS),
            vec![(PadButton::RightTrigger, true)]
        );
        // Still pressed between the two thresholds.
        assert!(latches
            .update(Axis::TriggerRight, TRIGGER_RELEASE)
            .is_empty());
        assert_eq!(
            latches.update(Axis::TriggerRight, TRIGGER_RELEASE - 1),
            vec![(PadButton::RightTrigger, false)]
        );
        assert!(latches.update(Axis::TriggerLeft, 0).is_empty());
    }

    #[test]
    fn stick_directions_follow_sdl_signs() {
        let mut latches = AnalogLatches::default();
        assert_eq!(
            latches.update(Axis::LeftY, i16::MIN),
            vec![(PadButton::LeftStickUp, true)]
        );
        assert_eq!(
            latches.update(Axis::LeftY, i16::MAX),
            vec![
                (PadButton::LeftStickUp, false),
                (PadButton::LeftStickDown, true)
            ]
        );
        assert_eq!(
            latches.update(Axis::LeftX, -STICK_PRESS),
            vec![(PadButton::LeftStickLeft, true)]
        );
        assert!(latches.update(Axis::LeftX, -STICK_RELEASE).is_empty());
        assert_eq!(
            latches.update(Axis::LeftX, 0),
            vec![(PadButton::LeftStickLeft, false)]
        );
        // The right stick is analog only.
        assert!(latches.update(Axis::RightX, i16::MAX).is_empty());
    }

    #[test]
    fn the_pad_used_last_takes_over_and_releases_the_previous_ones_buttons() {
        let mut routing = Routing::default();
        let mut out = Vec::new();
        routing.button(1, PadButton::South, true, &mut out);
        routing.axis(1, Axis::TriggerRight, i16::MAX, &mut out);
        assert_eq!(
            out,
            vec![(PadButton::South, true), (PadButton::RightTrigger, true)]
        );
        assert_eq!(routing.active(), Some(1));

        // A second pad's small stick drift does not take over.
        out.clear();
        routing.axis(2, Axis::LeftX, ACTIVITY - 1, &mut out);
        assert!(out.is_empty());
        // Its button does, after the first pad's buttons are released.
        routing.button(2, PadButton::East, true, &mut out);
        assert_eq!(
            out,
            vec![
                (PadButton::South, false),
                (PadButton::RightTrigger, false),
                (PadButton::East, true)
            ]
        );
        // The first pad's late releases no longer count.
        out.clear();
        routing.button(1, PadButton::South, false, &mut out);
        assert!(out.is_empty());

        // Deliberate stick motion takes over too, and reports its direction.
        routing.axis(1, Axis::LeftY, i16::MIN, &mut out);
        assert_eq!(
            out,
            vec![(PadButton::East, false), (PadButton::LeftStickUp, true)]
        );
    }

    #[test]
    fn removal_focus_loss_and_joysticks_release_held_buttons() {
        let mut routing = Routing::default();
        let mut out = Vec::new();
        routing.button(7, PadButton::Start, true, &mut out);
        out.clear();
        routing.release_all(&mut out);
        assert_eq!(out, vec![(PadButton::Start, false)]);

        out.clear();
        routing.button(7, PadButton::North, true, &mut out);
        routing.joystick_used(9, &mut out);
        assert_eq!(
            out,
            vec![(PadButton::North, true), (PadButton::North, false)]
        );
        assert_eq!(routing.active(), Some(9));

        out.clear();
        routing.button(7, PadButton::West, true, &mut out);
        routing.removed(7, &mut out);
        assert_eq!(out, vec![(PadButton::West, true), (PadButton::West, false)]);
        assert_eq!(routing.active(), None);
        // Removing another device changes nothing.
        out.clear();
        routing.removed(3, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn system_and_extra_buttons_are_not_pad_buttons() {
        assert_eq!(pad_button(Button::A), Some(PadButton::South));
        assert_eq!(pad_button(Button::Y), Some(PadButton::North));
        assert_eq!(pad_button(Button::Guide), None);
        assert_eq!(pad_button(Button::Paddle1), None);
        assert!(is_activity(Axis::RightY, -ACTIVITY));
        assert!(!is_activity(Axis::RightY, ACTIVITY - 1));
        assert!(is_activity(Axis::TriggerLeft, TRIGGER_PRESS));
    }
}
