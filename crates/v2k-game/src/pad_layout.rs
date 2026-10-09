//! Controller layouts: what a standard game controller does on each screen.
//!
//! - **PC original** is retail PC behaviour. A pad is a WinMM joystick that
//!   only the craft reader consults (`retail_input`): button 1 fires, button 2
//!   thrusts, the left stick steers. No screen binding set has a joystick
//!   table, so menus and the intro ignore it.
//! - **Console original** evaluates the PlayStation release's own binding
//!   sets (PAL `SLES_005.45`): Controller Config 0, Relative 1, the console's
//!   default, on every screen the port has. The PC executable still carries
//!   the same tables in its pad slots. `docs/re/CONTROLS.md` records both.
//! - **Remastered** is the port's own layout, with the same record evaluator.
//!
//! Records follow FGDK's digital binding rule: a record is active while all
//! of its "down" inputs are held and none of its "up" inputs are. A held
//! action lasts while any of its records is active; any other action happens
//! when a record becomes active.

use sdl2::keyboard::Keycode;
use v2k_render::{ControllerLayout, ControllerSample, PadButton, PadSnapshot};

use crate::chase_camera::ChaseCameraParameters;
use crate::menu_engine::MenuInput;
use crate::retail_input::{JoystickSample, JOYSTICK_AXIS_CENTRE};

/// FGDK console pad inputs: PlayStation pad bit numbers plus one. The pad
/// buttons are named by position, so Cross is the bottom face button of any
/// pad (Xbox A), Circle the right one (B), Square the left one (X) and
/// Triangle the top one (Y).
pub mod input {
    pub const SELECT: u8 = 1;
    pub const L3: u8 = 2;
    pub const R3: u8 = 3;
    pub const START: u8 = 4;
    pub const UP: u8 = 5;
    pub const RIGHT: u8 = 6;
    pub const DOWN: u8 = 7;
    pub const LEFT: u8 = 8;
    pub const L2: u8 = 9;
    pub const R2: u8 = 10;
    pub const L1: u8 = 11;
    pub const R1: u8 = 12;
    pub const TRIANGLE: u8 = 13;
    pub const CIRCLE: u8 = 14;
    pub const CROSS: u8 = 15;
    pub const SQUARE: u8 = 16;
    /// Port-only inputs for the Remastered menus: the left stick pushed past
    /// half deflection.
    pub const STICK_UP: u8 = 17;
    pub const STICK_RIGHT: u8 = 18;
    pub const STICK_DOWN: u8 = 19;
    pub const STICK_LEFT: u8 = 20;
}

use input::*;

/// The FGDK input number of a positional pad button.
pub const fn input_of(button: PadButton) -> u8 {
    match button {
        PadButton::Back => SELECT,
        PadButton::LeftStick => L3,
        PadButton::RightStick => R3,
        PadButton::Start => START,
        PadButton::DPadUp => UP,
        PadButton::DPadRight => RIGHT,
        PadButton::DPadDown => DOWN,
        PadButton::DPadLeft => LEFT,
        PadButton::LeftTrigger => L2,
        PadButton::RightTrigger => R2,
        PadButton::LeftShoulder => L1,
        PadButton::RightShoulder => R1,
        PadButton::North => TRIANGLE,
        PadButton::East => CIRCLE,
        PadButton::South => CROSS,
        PadButton::West => SQUARE,
        PadButton::LeftStickUp => STICK_UP,
        PadButton::LeftStickRight => STICK_RIGHT,
        PadButton::LeftStickDown => STICK_DOWN,
        PadButton::LeftStickLeft => STICK_LEFT,
    }
}

/// What a pad record drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadAction {
    /// A menu engine input, as the keyboard's arrows, Enter and Escape give.
    Menu(MenuInput),
    /// Remastered pause menus: resume from the root, else go back.
    PauseResumeOrBack,
    /// Intro movie skip (`FUN_0042ADA0`).
    SkipMovie,
    /// In-play `FUN_00455D30`: skips the level opening; afterwards its
    /// continue has no port owner yet.
    Continue,
    /// In-play pause (`FUN_00455D60`), which also skips the level opening.
    Pause,
    /// Mission abort (`+0x28D`); not ported.
    Abort,
    OpenMap,
    CloseMap,
    /// The progress map's save, as S.
    Save,
    /// Both Space descriptors: throttle and the positive-thrust gate.
    Throttle,
    Reverse,
    Fire,
    PitchUp,
    PitchDown,
    TurnLeft,
    TurnRight,
    ToggleMode,
    NextWeapon,
    PreviousWeapon,
    Collect,
    Drop,
    /// Next target (E); not ported.
    NextTarget,
    /// Special weapon (`0x00444400`); not ported.
    SpecialWeapon,
    /// Debug spawn behind the cheat flags; not ported.
    DebugSpawn,
    /// Cheat code digits 1-9; not ported.
    CheatDigit(u8),
}

impl PadAction {
    /// Held descriptors, as opposed to callbacks.
    pub const fn is_held(self) -> bool {
        matches!(
            self,
            Self::Throttle
                | Self::Reverse
                | Self::Fire
                | Self::PitchUp
                | Self::PitchDown
                | Self::TurnLeft
                | Self::TurnRight
        )
    }

    /// The key bound to the same held descriptor in the PC craft set.
    pub const fn keyboard_equivalent(self) -> Option<Keycode> {
        Some(match self {
            Self::Throttle => Keycode::Space,
            Self::Reverse => Keycode::RShift,
            Self::Fire => Keycode::Return,
            Self::PitchUp => Keycode::Up,
            Self::PitchDown => Keycode::Down,
            Self::TurnLeft => Keycode::Left,
            Self::TurnRight => Keycode::Right,
            _ => return None,
        })
    }
}

/// One FGDK digital record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadRecord {
    pub action: PadAction,
    /// Inputs that must be held.
    pub down: &'static [u8],
    /// Inputs that must not be held.
    pub up: &'static [u8],
}

impl PadRecord {
    const fn new(action: PadAction, down: &'static [u8]) -> Self {
        Self {
            action,
            down,
            up: &[],
        }
    }

    const fn unless(action: PadAction, down: &'static [u8], up: &'static [u8]) -> Self {
        Self { action, down, up }
    }

    fn satisfied(&self, held: u32) -> bool {
        self.down.iter().all(|&input| held & 1 << input != 0)
            && self.up.iter().all(|&input| held & 1 << input == 0)
    }
}

/// The screens with their own binding sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadContext {
    /// Loading and transitions: nothing is bound.
    Inactive,
    FrontendMenu,
    PauseMenu,
    IntroMovie,
    /// The Intro2 level opening, before the craft set is installed.
    Opening,
    /// Flight: the in-play set beside the craft set.
    Play,
    /// The full-screen map.
    Map,
    /// The progress map after a world ("Press S to Save, Space to Continue").
    Progress,
}

use PadAction::*;

const fn menu(input: MenuInput, down: &'static [u8]) -> PadRecord {
    PadRecord::new(Menu(input), down)
}

/// Console menus, frontend and pause (`0x80010738`; PC copy `0x004C0818`).
const CONSOLE_MENU: &[PadRecord] = &[
    menu(MenuInput::Back, &[TRIANGLE]),
    menu(MenuInput::Select, &[CROSS]),
    menu(MenuInput::Up, &[UP]),
    menu(MenuInput::Down, &[DOWN]),
    menu(MenuInput::Left, &[LEFT]),
    menu(MenuInput::Right, &[RIGHT]),
];

/// Console intro movie (`0x80010A40`; PC copy `0x004C0B28`): every button
/// but the stick clicks, which the console driver masks off.
const CONSOLE_INTRO: &[PadRecord] = &[
    PadRecord::new(SkipMovie, &[START]),
    PadRecord::new(SkipMovie, &[SQUARE]),
    PadRecord::new(SkipMovie, &[CIRCLE]),
    PadRecord::new(SkipMovie, &[TRIANGLE]),
    PadRecord::new(SkipMovie, &[CROSS]),
    PadRecord::new(SkipMovie, &[LEFT]),
    PadRecord::new(SkipMovie, &[RIGHT]),
    PadRecord::new(SkipMovie, &[UP]),
    PadRecord::new(SkipMovie, &[DOWN]),
    PadRecord::new(SkipMovie, &[L1]),
    PadRecord::new(SkipMovie, &[L2]),
    PadRecord::new(SkipMovie, &[R1]),
    PadRecord::new(SkipMovie, &[R2]),
    PadRecord::new(SkipMovie, &[SELECT]),
];

/// Console in-play set (`0x80013790`; PC copy `0x004C2AD8`).
const CONSOLE_IN_PLAY: &[PadRecord] = &[
    PadRecord::new(Continue, &[CROSS]),
    PadRecord::new(Continue, &[SQUARE]),
    PadRecord::new(Continue, &[CIRCLE]),
    PadRecord::new(Continue, &[TRIANGLE]),
    PadRecord::new(Abort, &[START, SELECT]),
    PadRecord::unless(Pause, &[START], &[SELECT]),
    PadRecord::unless(OpenMap, &[SELECT], &[START]),
    PadRecord::new(DebugSpawn, &[CROSS, R1, R1]),
    PadRecord::new(CheatDigit(1), &[R1, LEFT]),
    PadRecord::new(CheatDigit(2), &[R1, RIGHT]),
    PadRecord::new(CheatDigit(3), &[R1, TRIANGLE]),
    PadRecord::new(CheatDigit(4), &[R1, CIRCLE]),
    PadRecord::new(CheatDigit(5), &[R1, CROSS]),
    PadRecord::new(CheatDigit(6), &[R1, SQUARE]),
    PadRecord::new(CheatDigit(7), &[R1, R2]),
    PadRecord::new(CheatDigit(8), &[R1, L1]),
    PadRecord::new(CheatDigit(9), &[R1, L2]),
];

/// Console craft set, configuration 0 "Relative 1" (`0x80014A30`; PC copy
/// `0x004C4120`). The two Square records are the throttle and the
/// positive-thrust gate, which the port's Space binding drives together.
const CONSOLE_CRAFT: &[PadRecord] = &[
    PadRecord::unless(Reverse, &[CIRCLE], &[R1]),
    PadRecord::unless(Throttle, &[SQUARE], &[R1]),
    PadRecord::unless(Throttle, &[SQUARE], &[R1]),
    PadRecord::unless(Fire, &[CROSS], &[L1, L2]),
    PadRecord::new(SpecialWeapon, &[CROSS, L1, L2]),
    PadRecord::new(Drop, &[R2]),
    PadRecord::new(Collect, &[R1]),
    PadRecord::unless(ToggleMode, &[TRIANGLE], &[R1]),
    PadRecord::new(NextTarget, &[R1, R2]),
    PadRecord::unless(NextWeapon, &[L2], &[R1]),
    PadRecord::unless(PreviousWeapon, &[L1], &[R1]),
    PadRecord::new(PitchUp, &[UP]),
    PadRecord::new(PitchDown, &[DOWN]),
    PadRecord::new(TurnLeft, &[LEFT]),
    PadRecord::new(TurnRight, &[RIGHT]),
];

/// Console full-screen map (`0x80014EB0`; PC copy `0x004C4730`).
const CONSOLE_MAP: &[PadRecord] = &[
    PadRecord::new(CloseMap, &[CROSS]),
    PadRecord::unless(CloseMap, &[SELECT], &[START]),
];

/// Console briefing (`0x80013418`; PC copy `0x004C2580`), the port's
/// progress map.
const CONSOLE_PROGRESS: &[PadRecord] = &[
    PadRecord::new(Abort, &[START, SELECT]),
    PadRecord::new(Continue, &[START]),
    PadRecord::new(Continue, &[CROSS]),
    PadRecord::new(Save, &[TRIANGLE]),
];

/// Remastered directions: D-pad or left stick.
const fn directions(
    up: PadAction,
    down: PadAction,
    left: PadAction,
    right: PadAction,
) -> [PadRecord; 8] {
    [
        PadRecord::new(up, &[UP]),
        PadRecord::new(up, &[STICK_UP]),
        PadRecord::new(down, &[DOWN]),
        PadRecord::new(down, &[STICK_DOWN]),
        PadRecord::new(left, &[LEFT]),
        PadRecord::new(left, &[STICK_LEFT]),
        PadRecord::new(right, &[RIGHT]),
        PadRecord::new(right, &[STICK_RIGHT]),
    ]
}

const MENU_DIRECTIONS: [PadRecord; 8] = directions(
    Menu(MenuInput::Up),
    Menu(MenuInput::Down),
    Menu(MenuInput::Left),
    Menu(MenuInput::Right),
);

/// Remastered frontend menus: A or Menu select, B goes back.
const REMASTERED_FRONTEND_MENU: &[PadRecord] = &[
    MENU_DIRECTIONS[0],
    MENU_DIRECTIONS[1],
    MENU_DIRECTIONS[2],
    MENU_DIRECTIONS[3],
    MENU_DIRECTIONS[4],
    MENU_DIRECTIONS[5],
    MENU_DIRECTIONS[6],
    MENU_DIRECTIONS[7],
    menu(MenuInput::Select, &[CROSS]),
    menu(MenuInput::Select, &[START]),
    menu(MenuInput::Back, &[CIRCLE]),
];

/// Remastered pause menus: A selects; B and Menu resume from the root and
/// go back from deeper screens.
const REMASTERED_PAUSE_MENU: &[PadRecord] = &[
    MENU_DIRECTIONS[0],
    MENU_DIRECTIONS[1],
    MENU_DIRECTIONS[2],
    MENU_DIRECTIONS[3],
    MENU_DIRECTIONS[4],
    MENU_DIRECTIONS[5],
    MENU_DIRECTIONS[6],
    MENU_DIRECTIONS[7],
    menu(MenuInput::Select, &[CROSS]),
    PadRecord::new(PauseResumeOrBack, &[CIRCLE]),
    PadRecord::new(PauseResumeOrBack, &[START]),
];

/// Remastered intro movie: any button.
const REMASTERED_INTRO: &[PadRecord] = &[
    PadRecord::new(SkipMovie, &[CROSS]),
    PadRecord::new(SkipMovie, &[CIRCLE]),
    PadRecord::new(SkipMovie, &[SQUARE]),
    PadRecord::new(SkipMovie, &[TRIANGLE]),
    PadRecord::new(SkipMovie, &[START]),
    PadRecord::new(SkipMovie, &[SELECT]),
    PadRecord::new(SkipMovie, &[L1]),
    PadRecord::new(SkipMovie, &[R1]),
    PadRecord::new(SkipMovie, &[L2]),
    PadRecord::new(SkipMovie, &[R2]),
    PadRecord::new(SkipMovie, &[L3]),
    PadRecord::new(SkipMovie, &[R3]),
    PadRecord::new(SkipMovie, &[UP]),
    PadRecord::new(SkipMovie, &[DOWN]),
    PadRecord::new(SkipMovie, &[LEFT]),
    PadRecord::new(SkipMovie, &[RIGHT]),
];

/// Remastered level opening: a face button or Menu skips it.
const REMASTERED_OPENING: &[PadRecord] = &[
    PadRecord::new(Continue, &[CROSS]),
    PadRecord::new(Continue, &[CIRCLE]),
    PadRecord::new(Continue, &[SQUARE]),
    PadRecord::new(Continue, &[TRIANGLE]),
    PadRecord::new(Pause, &[START]),
];

/// Remastered flight. Both index fingers hold the two continuous actions,
/// as the PC mouse's buttons do (left thrusts, right fires); the thumbs
/// steer; the face buttons and bumpers take the discrete ones. The special
/// weapon needs both bumpers, a deliberate chord as on both retail versions.
const REMASTERED_PLAY: &[PadRecord] = &[
    PadRecord::new(Pause, &[START]),
    PadRecord::new(OpenMap, &[SELECT]),
    PadRecord::new(Throttle, &[L2]),
    PadRecord::new(Fire, &[R2]),
    PadRecord::new(Reverse, &[CIRCLE]),
    PadRecord::new(ToggleMode, &[CROSS]),
    PadRecord::new(Collect, &[SQUARE]),
    PadRecord::new(Drop, &[TRIANGLE]),
    PadRecord::unless(PreviousWeapon, &[L1], &[R1]),
    PadRecord::unless(NextWeapon, &[R1], &[L1]),
    PadRecord::new(SpecialWeapon, &[L1, R1]),
    PadRecord::new(NextTarget, &[R3]),
    PadRecord::new(PitchUp, &[UP]),
    PadRecord::new(PitchDown, &[DOWN]),
    PadRecord::new(TurnLeft, &[LEFT]),
    PadRecord::new(TurnRight, &[RIGHT]),
];

/// Remastered full-screen map: View, B or A close it.
const REMASTERED_MAP: &[PadRecord] = &[
    PadRecord::new(CloseMap, &[SELECT]),
    PadRecord::new(CloseMap, &[CIRCLE]),
    PadRecord::new(CloseMap, &[CROSS]),
];

/// Remastered progress map: A or Menu continue, Y saves.
const REMASTERED_PROGRESS: &[PadRecord] = &[
    PadRecord::new(Continue, &[CROSS]),
    PadRecord::new(Continue, &[START]),
    PadRecord::new(Save, &[TRIANGLE]),
];

/// The binding sets a layout installs for a screen. Play installs two, the
/// in-play set and the craft set, as retail does.
pub fn binding_sets(
    layout: ControllerLayout,
    context: PadContext,
) -> &'static [&'static [PadRecord]] {
    match (layout, context) {
        (ControllerLayout::PcOriginal, _) | (_, PadContext::Inactive) => &[],
        (ControllerLayout::ConsoleOriginal, context) => match context {
            PadContext::FrontendMenu | PadContext::PauseMenu => &[CONSOLE_MENU],
            PadContext::IntroMovie => &[CONSOLE_INTRO],
            PadContext::Opening => &[CONSOLE_IN_PLAY],
            PadContext::Play => &[CONSOLE_IN_PLAY, CONSOLE_CRAFT],
            PadContext::Map => &[CONSOLE_MAP],
            PadContext::Progress => &[CONSOLE_PROGRESS],
            PadContext::Inactive => &[],
        },
        (ControllerLayout::Remastered, context) => match context {
            PadContext::FrontendMenu => &[REMASTERED_FRONTEND_MENU],
            PadContext::PauseMenu => &[REMASTERED_PAUSE_MENU],
            PadContext::IntroMovie => &[REMASTERED_INTRO],
            PadContext::Opening => &[REMASTERED_OPENING],
            PadContext::Play => &[REMASTERED_PLAY],
            PadContext::Map => &[REMASTERED_MAP],
            PadContext::Progress => &[REMASTERED_PROGRESS],
            PadContext::Inactive => &[],
        },
    }
}

/// The active layout's binding sets for the current screen, and which of
/// their records are active.
#[derive(Debug, Clone)]
pub struct PadBindings {
    layout: ControllerLayout,
    context: PadContext,
    /// Bit `n` is input `n`.
    held: u32,
    active: Vec<bool>,
}

impl PadBindings {
    pub fn new(layout: ControllerLayout) -> Self {
        Self {
            layout,
            context: PadContext::Inactive,
            held: 0,
            active: Vec::new(),
        }
    }

    pub const fn layout(&self) -> ControllerLayout {
        self.layout
    }

    fn records(&self) -> impl Iterator<Item = &'static PadRecord> {
        binding_sets(self.layout, self.context)
            .iter()
            .flat_map(|set| set.iter())
    }

    /// Install `context`'s binding sets. A record already satisfied by held
    /// buttons starts active without acting, so a button held through a
    /// screen change does not also act on the new screen.
    pub fn set_context(&mut self, context: PadContext) {
        if context != self.context {
            self.context = context;
            let held = self.held;
            self.active = self
                .records()
                .map(|record| record.satisfied(held))
                .collect();
        }
    }

    /// Fold one pad button change in, appending the actions it starts.
    pub fn button(&mut self, button: PadButton, pressed: bool, started: &mut Vec<PadAction>) {
        let bit = 1 << input_of(button);
        if pressed {
            self.held |= bit;
        } else {
            self.held &= !bit;
        }
        let held = self.held;
        let records: Vec<&PadRecord> = self.records().collect();
        for (record, active) in records.into_iter().zip(self.active.iter_mut()) {
            let now = record.satisfied(held);
            if now && !*active && !record.action.is_held() {
                started.push(record.action);
            }
            *active = now;
        }
    }

    /// Whether a held action is on.
    pub fn is_held(&self, action: PadAction) -> bool {
        self.records()
            .zip(&self.active)
            .any(|(record, &active)| active && record.action == action)
    }

    /// The craft keys whose held descriptors the pad holds now.
    pub fn held_keys(&self) -> impl Iterator<Item = Keycode> + '_ {
        self.records()
            .zip(&self.active)
            .filter(|(_, &active)| active)
            .filter_map(|(record, _)| record.action.keyboard_equivalent())
    }

    /// The craft reader's joystick sample. Plain joysticks always take the PC
    /// path. A pad does so in the PC original; otherwise its buttons go
    /// through the binding sets and only its left stick reaches the reader.
    pub fn joystick_sample(&self, sample: ControllerSample) -> JoystickSample {
        match (self.layout, sample) {
            (ControllerLayout::ConsoleOriginal, ControllerSample::Pad(pad)) => {
                console_stick_sample(pad)
            }
            (ControllerLayout::Remastered, ControllerSample::Pad(pad)) => JoystickSample {
                axes: [
                    winmm_position(pad.left_x),
                    winmm_position(pad.left_y),
                    JOYSTICK_AXIS_CENTRE,
                    JOYSTICK_AXIS_CENTRE,
                    JOYSTICK_AXIS_CENTRE,
                    JOYSTICK_AXIS_CENTRE,
                ],
                buttons: 0,
            },
            _ => JoystickSample::from_controller(sample),
        }
    }

    /// Chase-camera parameters from the right stick. The console's in-play
    /// set binds it; the PC never feeds these parameters.
    pub fn camera_parameters(&self, sample: Option<ControllerSample>) -> ChaseCameraParameters {
        match (self.layout, sample) {
            (ControllerLayout::PcOriginal, _)
            | (_, None | Some(ControllerSample::Joystick { .. })) => {
                ChaseCameraParameters::default()
            }
            (_, Some(ControllerSample::Pad(pad))) => console_camera_parameters(pad),
        }
    }
}

/// An SDL stick axis as the console analog pad's byte: 0 left or up, 0xFF
/// right or down, 0x80 at rest.
pub const fn console_stick_byte(value: i16) -> u8 {
    ((value as i32 + 0x8000) >> 8) as u8
}

fn winmm_position(value: i16) -> u16 {
    (i32::from(value) + 0x8000) as u16
}

/// The console driver (`0x800D07B4`) stores the left stick as axes 0 and 1.
/// The craft table scales `byte - 0x80` by 32 into the PC joystick's turn and
/// pitch channels, which is the PC joystick's `(word - 0x8000) / 8` for
/// `word = byte << 8`; the pad's buttons stay out of the joystick path.
fn console_stick_sample(pad: PadSnapshot) -> JoystickSample {
    JoystickSample {
        axes: [
            u16::from(console_stick_byte(pad.left_x)) << 8,
            u16::from(console_stick_byte(pad.left_y)) << 8,
            JOYSTICK_AXIS_CENTRE,
            JOYSTICK_AXIS_CENTRE,
            JOYSTICK_AXIS_CENTRE,
            JOYSTICK_AXIS_CENTRE,
        ],
        buttons: 0,
    }
}

/// The in-play analog table (console `0x80013748`; PC copy `0x004C2A90`)
/// scales the right stick's X (axis 2) by -8 and Y (axis 3) by +8 into the
/// camera's parameter-3 and parameter-4 channels.
fn console_camera_parameters(pad: PadSnapshot) -> ChaseCameraParameters {
    let centred = |value: i16| i32::from(console_stick_byte(value)) - 0x80;
    ChaseCameraParameters {
        swing: -8 * centred(pad.right_x),
        distance: 8 * centred(pad.right_y),
    }
}

#[cfg(test)]
mod tests;
