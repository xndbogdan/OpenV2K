//! Foreground-scoped control correlation for the retail executable.
//!
//! This samples only explicitly listed controls and only while the attached
//! V2000 process owns the foreground window. The values come from the Win32
//! virtual-key high bit; they are correlation evidence, not a replacement for
//! the game's DirectInput buffer and not a system-wide input logger. Caps Lock
//! is sampled the same way as a guided-capture delimiter; retail also binds it
//! to Previous Weapon, so captures label the resulting inventory contamination.
//! F12 is a foreground-only delimiter for the Power Up/fire capture wrapper and
//! is not claimed to be a retail control.
//! Mouse position or cursor deltas are deliberately never sampled.

use serde::Serialize;
use std::ffi::c_void;

type WindowHandle = *mut c_void;

const VK_RBUTTON: i32 = 0x02;
const VK_TAB: i32 = 0x09;
const VK_RETURN: i32 = 0x0D;
const VK_LSHIFT: i32 = 0xA0;
const VK_LCONTROL: i32 = 0xA2;
const VK_RCONTROL: i32 = 0xA3;
const VK_CAPITAL: i32 = 0x14;
const VK_SPACE: i32 = 0x20;
const VK_LEFT: i32 = 0x25;
const VK_UP: i32 = 0x26;
const VK_RIGHT: i32 = 0x27;
const VK_DOWN: i32 = 0x28;
const VK_M: i32 = 0x4D;
const VK_B: i32 = 0x42;
const VK_C: i32 = 0x43;
const VK_D: i32 = 0x44;
const VK_E: i32 = 0x45;
const VK_V: i32 = 0x56;
const VK_S: i32 = 0x53;
const VK_X: i32 = 0x58;
const VK_RSHIFT: i32 = 0xA1;
const VK_F12: i32 = 0x7B;

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(virtual_key: i32) -> i16;
    fn GetForegroundWindow() -> WindowHandle;
    fn GetWindowThreadProcessId(window: WindowHandle, process_id: *mut u32) -> u32;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct FireInputState {
    pub enter: bool,
    pub right_mouse: bool,
    pub left_control: bool,
    pub right_control: bool,
    /// Physical E key; retail rotates a separate 12-byte selection list.
    pub key_e: bool,
    /// Physical B key; retail advances the weapon inventory.
    pub key_b: bool,
    /// Physical V key; retail moves backward through the weapon inventory.
    pub key_v: bool,
    /// Physical Left Shift key; another forward weapon binding.
    pub left_shift: bool,
    /// Protocol-only stop key for the Power Up/fire guided wrapper.
    pub capture_stop_f12: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct HoverInputState {
    pub space: bool,
    pub right_shift: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub s: bool,
    pub x: bool,
    pub tab: bool,
    /// Proven fullscreen-map key. Retail binds DIK_M through the parallel
    /// gameplay-UI input table rather than the craft control-vector table.
    pub m: bool,
    /// Physical Caps Lock state used to delimit retries. Retail simultaneously
    /// treats it as Previous Weapon, so it is never a behavior-neutral input.
    pub caps_lock: bool,
}

/// Foreground controls useful across actor, factory, pickup, and combat
/// protocols. This deliberately records only the small named set needed to
/// correlate a guided capture; it is not a general keyboard logger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ScenarioInputState {
    pub space: bool,
    pub right_shift: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub tab: bool,
    pub cargo_pickup_c: bool,
    pub cargo_drop_d: bool,
    pub fire_enter: bool,
    pub fire_right_mouse: bool,
    pub capture_retry_caps_lock: bool,
    pub capture_stop_f12: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FireControl {
    Enter,
    RightMouse,
    LeftControl,
    RightControl,
    KeyE,
    KeyB,
    KeyV,
    LeftShift,
    CaptureStopF12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HoverControl {
    Space,
    RightShift,
    Up,
    Down,
    Left,
    Right,
    S,
    X,
    Tab,
    M,
    CapsLock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioControl {
    Space,
    RightShift,
    Up,
    Down,
    Left,
    Right,
    Tab,
    CargoPickupC,
    CargoDropD,
    FireEnter,
    FireRightMouse,
    CaptureRetryCapsLock,
    CaptureStopF12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputEdge {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FireInputEdge {
    pub control: FireControl,
    pub edge: InputEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HoverInputEdge {
    pub control: HoverControl,
    pub edge: InputEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ScenarioInputEdge {
    pub control: ScenarioControl,
    pub edge: InputEdge,
}

impl FireInputState {
    pub fn edges_from(self, previous: Self) -> Vec<FireInputEdge> {
        let mut edges = Vec::with_capacity(9);
        push_fire_edge(&mut edges, FireControl::Enter, previous.enter, self.enter);
        push_fire_edge(
            &mut edges,
            FireControl::RightMouse,
            previous.right_mouse,
            self.right_mouse,
        );
        push_fire_edge(
            &mut edges,
            FireControl::LeftControl,
            previous.left_control,
            self.left_control,
        );
        push_fire_edge(
            &mut edges,
            FireControl::RightControl,
            previous.right_control,
            self.right_control,
        );
        push_fire_edge(&mut edges, FireControl::KeyE, previous.key_e, self.key_e);
        push_fire_edge(&mut edges, FireControl::KeyB, previous.key_b, self.key_b);
        push_fire_edge(&mut edges, FireControl::KeyV, previous.key_v, self.key_v);
        push_fire_edge(
            &mut edges,
            FireControl::LeftShift,
            previous.left_shift,
            self.left_shift,
        );
        push_fire_edge(
            &mut edges,
            FireControl::CaptureStopF12,
            previous.capture_stop_f12,
            self.capture_stop_f12,
        );
        edges
    }
}

impl HoverInputState {
    pub fn edges_from(self, previous: Self) -> Vec<HoverInputEdge> {
        let mut edges = Vec::with_capacity(11);
        push_hover_edge(&mut edges, HoverControl::Space, previous.space, self.space);
        push_hover_edge(
            &mut edges,
            HoverControl::RightShift,
            previous.right_shift,
            self.right_shift,
        );
        push_hover_edge(&mut edges, HoverControl::Up, previous.up, self.up);
        push_hover_edge(&mut edges, HoverControl::Down, previous.down, self.down);
        push_hover_edge(&mut edges, HoverControl::Left, previous.left, self.left);
        push_hover_edge(&mut edges, HoverControl::Right, previous.right, self.right);
        push_hover_edge(&mut edges, HoverControl::S, previous.s, self.s);
        push_hover_edge(&mut edges, HoverControl::X, previous.x, self.x);
        push_hover_edge(&mut edges, HoverControl::Tab, previous.tab, self.tab);
        push_hover_edge(&mut edges, HoverControl::M, previous.m, self.m);
        push_hover_edge(
            &mut edges,
            HoverControl::CapsLock,
            previous.caps_lock,
            self.caps_lock,
        );
        edges
    }
}

impl ScenarioInputState {
    pub fn edges_from(self, previous: Self) -> Vec<ScenarioInputEdge> {
        let mut edges = Vec::with_capacity(13);
        for (control, before, after) in [
            (ScenarioControl::Space, previous.space, self.space),
            (
                ScenarioControl::RightShift,
                previous.right_shift,
                self.right_shift,
            ),
            (ScenarioControl::Up, previous.up, self.up),
            (ScenarioControl::Down, previous.down, self.down),
            (ScenarioControl::Left, previous.left, self.left),
            (ScenarioControl::Right, previous.right, self.right),
            (ScenarioControl::Tab, previous.tab, self.tab),
            (
                ScenarioControl::CargoPickupC,
                previous.cargo_pickup_c,
                self.cargo_pickup_c,
            ),
            (
                ScenarioControl::CargoDropD,
                previous.cargo_drop_d,
                self.cargo_drop_d,
            ),
            (
                ScenarioControl::FireEnter,
                previous.fire_enter,
                self.fire_enter,
            ),
            (
                ScenarioControl::FireRightMouse,
                previous.fire_right_mouse,
                self.fire_right_mouse,
            ),
            (
                ScenarioControl::CaptureRetryCapsLock,
                previous.capture_retry_caps_lock,
                self.capture_retry_caps_lock,
            ),
            (
                ScenarioControl::CaptureStopF12,
                previous.capture_stop_f12,
                self.capture_stop_f12,
            ),
        ] {
            if before != after {
                edges.push(ScenarioInputEdge {
                    control,
                    edge: edge_for_state(after),
                });
            }
        }
        edges
    }
}

fn push_fire_edge(edges: &mut Vec<FireInputEdge>, control: FireControl, before: bool, after: bool) {
    if before != after {
        edges.push(FireInputEdge {
            control,
            edge: edge_for_state(after),
        });
    }
}

fn push_hover_edge(
    edges: &mut Vec<HoverInputEdge>,
    control: HoverControl,
    before: bool,
    after: bool,
) {
    if before != after {
        edges.push(HoverInputEdge {
            control,
            edge: edge_for_state(after),
        });
    }
}

fn edge_for_state(after: bool) -> InputEdge {
    if after {
        InputEdge::Pressed
    } else {
        InputEdge::Released
    }
}

/// Poll the established fire, weapon/target selection, and capture-stop
/// controls while V2000 owns focus.
pub fn poll_fire_foreground(process_id: u32) -> Option<FireInputState> {
    poll_when_foreground(process_id, || FireInputState {
        enter: key_is_down(VK_RETURN),
        right_mouse: key_is_down(VK_RBUTTON),
        left_control: key_is_down(VK_LCONTROL),
        right_control: key_is_down(VK_RCONTROL),
        key_e: key_is_down(VK_E),
        key_b: key_is_down(VK_B),
        key_v: key_is_down(VK_V),
        left_shift: key_is_down(VK_LSHIFT),
        capture_stop_f12: key_is_down(VK_F12),
    })
}

/// Poll the gameplay controls relevant to Hover movement, steering, gun pitch,
/// and mode changes while V2000 owns focus.
pub fn poll_hover_foreground(process_id: u32) -> Option<HoverInputState> {
    poll_when_foreground(process_id, || HoverInputState {
        space: key_is_down(VK_SPACE),
        right_shift: key_is_down(VK_RSHIFT),
        up: key_is_down(VK_UP),
        down: key_is_down(VK_DOWN),
        left: key_is_down(VK_LEFT),
        right: key_is_down(VK_RIGHT),
        s: key_is_down(VK_S),
        x: key_is_down(VK_X),
        tab: key_is_down(VK_TAB),
        m: key_is_down(VK_M),
        caps_lock: key_is_down(VK_CAPITAL),
    })
}

/// Poll the deliberately bounded cross-scenario control set while V2000 owns
/// focus. C/D and F12 are the only additions beyond established craft inputs.
pub fn poll_scenario_foreground(process_id: u32) -> Option<ScenarioInputState> {
    poll_when_foreground(process_id, || ScenarioInputState {
        space: key_is_down(VK_SPACE),
        right_shift: key_is_down(VK_RSHIFT),
        up: key_is_down(VK_UP),
        down: key_is_down(VK_DOWN),
        left: key_is_down(VK_LEFT),
        right: key_is_down(VK_RIGHT),
        tab: key_is_down(VK_TAB),
        cargo_pickup_c: key_is_down(VK_C),
        cargo_drop_d: key_is_down(VK_D),
        fire_enter: key_is_down(VK_RETURN),
        fire_right_mouse: key_is_down(VK_RBUTTON),
        capture_retry_caps_lock: key_is_down(VK_CAPITAL),
        capture_stop_f12: key_is_down(VK_F12),
    })
}

fn poll_when_foreground<T>(process_id: u32, sample: impl FnOnce() -> T) -> Option<T> {
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut foreground_process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(window, &mut foreground_process_id);
    }
    (foreground_process_id == process_id).then(sample)
}

fn key_is_down(virtual_key: i32) -> bool {
    unsafe { GetAsyncKeyState(virtual_key) as u16 & 0x8000 != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_only_changed_fire_controls() {
        let before = FireInputState::default();
        let after = FireInputState {
            enter: true,
            right_mouse: false,
            ..FireInputState::default()
        };
        let edges = after.edges_from(before);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].control, FireControl::Enter);
        assert_eq!(edges[0].edge, InputEdge::Pressed);

        let released = before.edges_from(after);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].edge, InputEdge::Released);
    }

    #[test]
    fn reports_extended_fire_edges_in_stable_control_order() {
        let before = FireInputState {
            right_mouse: true,
            right_control: true,
            key_b: true,
            left_shift: true,
            ..FireInputState::default()
        };
        let after = FireInputState {
            enter: true,
            left_control: true,
            key_e: true,
            key_v: true,
            capture_stop_f12: true,
            ..FireInputState::default()
        };

        assert_eq!(
            after.edges_from(before),
            vec![
                FireInputEdge {
                    control: FireControl::Enter,
                    edge: InputEdge::Pressed,
                },
                FireInputEdge {
                    control: FireControl::RightMouse,
                    edge: InputEdge::Released,
                },
                FireInputEdge {
                    control: FireControl::LeftControl,
                    edge: InputEdge::Pressed,
                },
                FireInputEdge {
                    control: FireControl::RightControl,
                    edge: InputEdge::Released,
                },
                FireInputEdge {
                    control: FireControl::KeyE,
                    edge: InputEdge::Pressed,
                },
                FireInputEdge {
                    control: FireControl::KeyB,
                    edge: InputEdge::Released,
                },
                FireInputEdge {
                    control: FireControl::KeyV,
                    edge: InputEdge::Pressed,
                },
                FireInputEdge {
                    control: FireControl::LeftShift,
                    edge: InputEdge::Released,
                },
                FireInputEdge {
                    control: FireControl::CaptureStopF12,
                    edge: InputEdge::Pressed,
                },
            ]
        );
    }

    #[test]
    fn reports_hover_edges_in_stable_control_order() {
        let before = HoverInputState {
            right_shift: true,
            x: true,
            ..HoverInputState::default()
        };
        let after = HoverInputState {
            space: true,
            left: true,
            tab: true,
            m: true,
            caps_lock: true,
            ..HoverInputState::default()
        };

        let edges = after.edges_from(before);
        assert_eq!(
            edges,
            vec![
                HoverInputEdge {
                    control: HoverControl::Space,
                    edge: InputEdge::Pressed,
                },
                HoverInputEdge {
                    control: HoverControl::RightShift,
                    edge: InputEdge::Released,
                },
                HoverInputEdge {
                    control: HoverControl::Left,
                    edge: InputEdge::Pressed,
                },
                HoverInputEdge {
                    control: HoverControl::X,
                    edge: InputEdge::Released,
                },
                HoverInputEdge {
                    control: HoverControl::Tab,
                    edge: InputEdge::Pressed,
                },
                HoverInputEdge {
                    control: HoverControl::M,
                    edge: InputEdge::Pressed,
                },
                HoverInputEdge {
                    control: HoverControl::CapsLock,
                    edge: InputEdge::Pressed,
                },
            ]
        );
    }

    #[test]
    fn reports_caps_lock_release_as_protocol_state() {
        let before = HoverInputState {
            caps_lock: true,
            ..HoverInputState::default()
        };

        assert_eq!(
            HoverInputState::default().edges_from(before),
            vec![HoverInputEdge {
                control: HoverControl::CapsLock,
                edge: InputEdge::Released,
            }]
        );
    }

    #[test]
    fn scenario_edges_include_cargo_and_protocol_controls_in_stable_order() {
        let before = ScenarioInputState {
            cargo_drop_d: true,
            ..ScenarioInputState::default()
        };
        let after = ScenarioInputState {
            cargo_pickup_c: true,
            fire_enter: true,
            capture_stop_f12: true,
            ..ScenarioInputState::default()
        };
        assert_eq!(
            after.edges_from(before),
            vec![
                ScenarioInputEdge {
                    control: ScenarioControl::CargoPickupC,
                    edge: InputEdge::Pressed,
                },
                ScenarioInputEdge {
                    control: ScenarioControl::CargoDropD,
                    edge: InputEdge::Released,
                },
                ScenarioInputEdge {
                    control: ScenarioControl::FireEnter,
                    edge: InputEdge::Pressed,
                },
                ScenarioInputEdge {
                    control: ScenarioControl::CaptureStopF12,
                    edge: InputEdge::Pressed,
                },
            ]
        );
    }
}
