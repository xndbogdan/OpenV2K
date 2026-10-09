use std::collections::HashSet;

use super::*;
use crate::retail_input::{joystick_pitch_raw, joystick_turn_raw};

fn bindings(layout: ControllerLayout, context: PadContext) -> PadBindings {
    let mut bindings = PadBindings::new(layout);
    bindings.set_context(context);
    bindings
}

fn press(bindings: &mut PadBindings, button: PadButton) -> Vec<PadAction> {
    let mut started = Vec::new();
    bindings.button(button, true, &mut started);
    started
}

fn release(bindings: &mut PadBindings, button: PadButton) -> Vec<PadAction> {
    let mut started = Vec::new();
    bindings.button(button, false, &mut started);
    started
}

#[test]
fn console_r1_is_both_collect_and_a_shift() {
    let mut pad = bindings(ControllerLayout::ConsoleOriginal, PadContext::Play);
    // The in-play set sits beside the craft set: a face button is also its
    // continue, and R1 with another button types a cheat digit.
    assert_eq!(press(&mut pad, PadButton::West), vec![Continue]);
    assert!(pad.is_held(Throttle));
    // R1 alone collects, and its shift takes Square's throttle away.
    assert_eq!(
        press(&mut pad, PadButton::RightShoulder),
        vec![CheatDigit(6), Collect]
    );
    assert!(!pad.is_held(Throttle));
    // R1+R2: R2 drops on its own record and selects the next target with R1.
    assert_eq!(
        press(&mut pad, PadButton::RightTrigger),
        vec![CheatDigit(7), Drop, NextTarget]
    );
    assert!(release(&mut pad, PadButton::RightShoulder).is_empty());
    assert!(pad.is_held(Throttle));
}

#[test]
fn console_special_weapon_is_the_cross_l1_l2_chord() {
    let mut pad = bindings(ControllerLayout::ConsoleOriginal, PadContext::Play);
    assert_eq!(press(&mut pad, PadButton::South), vec![Continue]);
    assert!(pad.is_held(Fire));
    // Each shoulder still acts on its own first, as on the console.
    assert_eq!(
        press(&mut pad, PadButton::LeftShoulder),
        vec![PreviousWeapon]
    );
    assert!(!pad.is_held(Fire));
    assert_eq!(
        press(&mut pad, PadButton::LeftTrigger),
        vec![SpecialWeapon, NextWeapon]
    );
}

#[test]
fn console_start_pauses_unless_select_is_held() {
    let mut pad = bindings(ControllerLayout::ConsoleOriginal, PadContext::Play);
    assert_eq!(press(&mut pad, PadButton::Start), vec![Pause]);
    assert_eq!(press(&mut pad, PadButton::Back), vec![Abort]);
    release(&mut pad, PadButton::Start);
    release(&mut pad, PadButton::Back);
    assert_eq!(press(&mut pad, PadButton::Back), vec![OpenMap]);
    assert_eq!(press(&mut pad, PadButton::Start), vec![Abort]);
}

#[test]
fn console_menus_and_intro_follow_the_console_sets() {
    let mut menu = bindings(ControllerLayout::ConsoleOriginal, PadContext::FrontendMenu);
    assert_eq!(
        press(&mut menu, PadButton::South),
        vec![Menu(MenuInput::Select)]
    );
    assert_eq!(
        press(&mut menu, PadButton::North),
        vec![Menu(MenuInput::Back)]
    );
    assert!(press(&mut menu, PadButton::East).is_empty());
    assert!(press(&mut menu, PadButton::LeftStickUp).is_empty());

    let mut intro = bindings(ControllerLayout::ConsoleOriginal, PadContext::IntroMovie);
    assert_eq!(press(&mut intro, PadButton::RightTrigger), vec![SkipMovie]);
    // The console driver masks the stick clicks.
    assert!(press(&mut intro, PadButton::LeftStick).is_empty());
    for set in [
        CONSOLE_MENU,
        CONSOLE_INTRO,
        CONSOLE_IN_PLAY,
        CONSOLE_CRAFT,
        CONSOLE_MAP,
        CONSOLE_PROGRESS,
    ] {
        for record in set {
            for input in record.down.iter().chain(record.up) {
                assert!(
                    (1..=16).contains(input) && ![L3, R3].contains(input),
                    "{record:?}"
                );
            }
        }
    }
}

#[test]
fn a_button_held_through_a_screen_change_does_not_act_there() {
    let mut pad = bindings(ControllerLayout::Remastered, PadContext::FrontendMenu);
    assert_eq!(
        press(&mut pad, PadButton::South),
        vec![Menu(MenuInput::Select)]
    );
    pad.set_context(PadContext::Play);
    // A toggles Hover/VTOL in flight, but this press began in the menu.
    assert!(release(&mut pad, PadButton::South).is_empty());
    assert_eq!(press(&mut pad, PadButton::South), vec![ToggleMode]);

    // Held descriptors carry straight over.
    let mut pad = bindings(ControllerLayout::Remastered, PadContext::Map);
    press(&mut pad, PadButton::RightTrigger);
    pad.set_context(PadContext::Play);
    assert!(pad.is_held(Fire));
}

#[test]
fn remastered_flight_layout() {
    let mut pad = bindings(ControllerLayout::Remastered, PadContext::Play);
    press(&mut pad, PadButton::LeftTrigger);
    press(&mut pad, PadButton::RightTrigger);
    press(&mut pad, PadButton::East);
    press(&mut pad, PadButton::DPadLeft);
    assert_eq!(
        pad.held_keys().collect::<HashSet<_>>(),
        HashSet::from([
            Keycode::Space,
            Keycode::Return,
            Keycode::RShift,
            Keycode::Left
        ])
    );

    assert_eq!(press(&mut pad, PadButton::RightShoulder), vec![NextWeapon]);
    assert_eq!(
        press(&mut pad, PadButton::LeftShoulder),
        vec![SpecialWeapon]
    );
    release(&mut pad, PadButton::RightShoulder);
    release(&mut pad, PadButton::LeftShoulder);
    assert_eq!(
        press(&mut pad, PadButton::LeftShoulder),
        vec![PreviousWeapon]
    );
    assert_eq!(press(&mut pad, PadButton::West), vec![Collect]);
    assert_eq!(press(&mut pad, PadButton::North), vec![Drop]);
    assert_eq!(press(&mut pad, PadButton::Start), vec![Pause]);
    assert_eq!(press(&mut pad, PadButton::Back), vec![OpenMap]);
    // The left stick steers through the joystick sample, not as buttons.
    assert!(press(&mut pad, PadButton::LeftStickLeft).is_empty());
}

#[test]
fn remastered_menus_use_b_to_go_back_and_menu_to_resume() {
    let mut frontend = bindings(ControllerLayout::Remastered, PadContext::FrontendMenu);
    assert_eq!(
        press(&mut frontend, PadButton::LeftStickDown),
        vec![Menu(MenuInput::Down)]
    );
    assert_eq!(
        press(&mut frontend, PadButton::East),
        vec![Menu(MenuInput::Back)]
    );
    assert_eq!(
        press(&mut frontend, PadButton::Start),
        vec![Menu(MenuInput::Select)]
    );

    let mut pause = bindings(ControllerLayout::Remastered, PadContext::PauseMenu);
    assert_eq!(press(&mut pause, PadButton::East), vec![PauseResumeOrBack]);
    assert_eq!(press(&mut pause, PadButton::Start), vec![PauseResumeOrBack]);
    assert_eq!(
        press(&mut pause, PadButton::DPadUp),
        vec![Menu(MenuInput::Up)]
    );
}

#[test]
fn the_pc_original_binds_no_screen_and_keeps_the_joystick_path() {
    for context in [
        PadContext::FrontendMenu,
        PadContext::IntroMovie,
        PadContext::Play,
        PadContext::Progress,
    ] {
        let mut pad = bindings(ControllerLayout::PcOriginal, context);
        assert!(press(&mut pad, PadButton::South).is_empty());
        assert_eq!(pad.held_keys().count(), 0);
    }
    let pad = PadSnapshot {
        buttons: [
            true, false, false, false, false, false, false, false, false, false,
        ],
        right_x: i16::MAX,
        ..PadSnapshot::default()
    };
    let bindings = PadBindings::new(ControllerLayout::PcOriginal);
    assert!(bindings.joystick_sample(ControllerSample::Pad(pad)).fire());
    assert_eq!(
        bindings.camera_parameters(Some(ControllerSample::Pad(pad))),
        ChaseCameraParameters::default()
    );
}

#[test]
fn console_stick_has_the_analog_pads_eight_bit_resolution() {
    assert_eq!(console_stick_byte(i16::MIN), 0);
    assert_eq!(console_stick_byte(0), 0x80);
    assert_eq!(console_stick_byte(i16::MAX), 0xFF);
    let bindings = PadBindings::new(ControllerLayout::ConsoleOriginal);
    let sample = |left_x, left_y| {
        bindings.joystick_sample(ControllerSample::Pad(PadSnapshot {
            left_x,
            left_y,
            buttons: [true; 10],
            ..PadSnapshot::default()
        }))
    };
    // (0xFF - 0x80) * 32 = 4064 and (0 - 0x80) * 32 = -4096, less the
    // reader's 2000-word dead zone.
    assert_eq!(joystick_turn_raw(sample(i16::MAX, 0)), 2064);
    assert_eq!(joystick_turn_raw(sample(i16::MIN, 0)), -2096);
    assert_eq!(joystick_pitch_raw(sample(0, i16::MIN)), 2096);
    assert_eq!(joystick_turn_raw(sample(0, 0)), 0);
    // Its buttons go through the binding sets instead.
    assert!(!sample(0, 0).fire() && !sample(0, 0).thrust());
}

#[test]
fn right_stick_drives_the_chase_camera_parameters() {
    let pad = |right_x, right_y| {
        Some(ControllerSample::Pad(PadSnapshot {
            right_x,
            right_y,
            ..PadSnapshot::default()
        }))
    };
    for layout in [
        ControllerLayout::ConsoleOriginal,
        ControllerLayout::Remastered,
    ] {
        let bindings = PadBindings::new(layout);
        assert_eq!(
            bindings.camera_parameters(pad(i16::MAX, i16::MIN)),
            ChaseCameraParameters {
                swing: -1016,
                distance: -1024,
            }
        );
        assert_eq!(
            bindings.camera_parameters(pad(0, 0)),
            ChaseCameraParameters::default()
        );
        assert_eq!(
            bindings.camera_parameters(Some(ControllerSample::Joystick {
                axes: [i16::MAX; 6],
                buttons: 0,
            })),
            ChaseCameraParameters::default()
        );
    }
}
