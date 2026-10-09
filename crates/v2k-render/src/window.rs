use std::cell::Cell;

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::joystick::Joystick;
use sdl2::keyboard::Keycode;
use sdl2::mouse::{MouseButton, MouseWheelDirection};
use sdl2::Sdl;
use sdl2::VideoSubsystem;
use sdl2::{EventPump, GameControllerSubsystem, JoystickSubsystem};

/// Input events consumed by the game loop.
pub enum GameEvent {
    Quit,
    Resize(u32, u32),
    KeyDown(Keycode),
    KeyUp(Keycode),
    /// Relative motion, reported only while the pointer is captured.
    MouseMotion {
        xrel: i32,
        yrel: i32,
    },
    MouseButtonDown(MouseButton),
    MouseButtonUp(MouseButton),
    /// Wheel notches, positive away from the player whatever the system's
    /// scrolling direction.
    MouseWheel {
        y: i32,
    },
    FocusLost,
    FocusGained,
    AuxiliaryWindowClick {
        window_id: u32,
        x: i32,
        y: i32,
    },
    WindowClosed(u32),
}

/// How the game window treats the system pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerMode {
    /// Relative motion for craft control. SDL hides the pointer and keeps it
    /// inside the window.
    Captured,
    /// Free but invisible: V2000 hides the pointer over a full-screen game,
    /// menus included.
    Hidden,
    /// Free and visible.
    Visible,
}

impl PointerMode {
    /// Capture during focused gameplay unless a diagnostics window needs the
    /// pointer. Otherwise hide it over a focused full-screen game, as V2000
    /// does, and show it everywhere else.
    pub fn for_frame(
        gameplay: bool,
        focused: bool,
        diagnostics_open: bool,
        fullscreen: bool,
    ) -> Self {
        if !focused || diagnostics_open {
            Self::Visible
        } else if gameplay {
            Self::Captured
        } else if fullscreen {
            Self::Hidden
        } else {
            Self::Visible
        }
    }
}

/// A standard game-controller pad in SDL's layout: sticks in
/// `-32768..=32767` (negative left and up), triggers in `0..=32767`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PadSnapshot {
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
    pub left_trigger: i16,
    pub right_trigger: i16,
    /// South (A), east (B), west (X), north (Y), left shoulder, right
    /// shoulder, back, start, left stick, right stick.
    pub buttons: [bool; 10],
}

/// One frame's state of the first attached controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerSample {
    /// A device SDL recognises as a standard game controller.
    Pad(PadSnapshot),
    /// Any other joystick: its first six axes and first 32 buttons, in the
    /// device's own order (bit `n` is button `n + 1`).
    Joystick { axes: [i16; 6], buttons: u32 },
}

const PAD_BUTTONS: [Button; 10] = [
    Button::A,
    Button::B,
    Button::X,
    Button::Y,
    Button::LeftShoulder,
    Button::RightShoulder,
    Button::Back,
    Button::Start,
    Button::LeftStick,
    Button::RightStick,
];

enum OpenController {
    Pad(GameController),
    Joystick(Joystick),
}

impl OpenController {
    fn instance_id(&self) -> u32 {
        match self {
            Self::Pad(pad) => pad.instance_id(),
            Self::Joystick(joystick) => joystick.instance_id(),
        }
    }

    fn sample(&self) -> ControllerSample {
        match self {
            Self::Pad(pad) => ControllerSample::Pad(PadSnapshot {
                left_x: pad.axis(Axis::LeftX),
                left_y: pad.axis(Axis::LeftY),
                right_x: pad.axis(Axis::RightX),
                right_y: pad.axis(Axis::RightY),
                left_trigger: pad.axis(Axis::TriggerLeft),
                right_trigger: pad.axis(Axis::TriggerRight),
                buttons: PAD_BUTTONS.map(|button| pad.button(button)),
            }),
            Self::Joystick(joystick) => {
                let axes = std::array::from_fn(|index| joystick.axis(index as u32).unwrap_or(0));
                let buttons = (0..joystick.num_buttons().min(32))
                    .filter(|&index| joystick.button(index).unwrap_or(false))
                    .fold(0, |mask, index| mask | 1 << index);
                ControllerSample::Joystick { axes, buttons }
            }
        }
    }
}

/// Attached pads and joysticks, opened as SDL reports them. This stands in for
/// V2000's WinMM probe (`FUN_004AC110`, up to 16 joysticks) and its 20 ms poll
/// (`FUN_004AC2A0`); the game samples the first device once per frame.
struct Controllers {
    pads: GameControllerSubsystem,
    joysticks: JoystickSubsystem,
    open: Vec<OpenController>,
}

impl Controllers {
    fn new(sdl: &Sdl) -> Result<Self, String> {
        Ok(Self {
            pads: sdl.game_controller()?,
            joysticks: sdl.joystick()?,
            open: Vec::new(),
        })
    }

    /// Open device `index` once; SDL announces every device present at
    /// start-up the same way as a later hot-plug.
    fn attach(&mut self, index: u32) {
        let device = if self.pads.is_game_controller(index) {
            self.pads
                .open(index)
                .map(OpenController::Pad)
                .map_err(|e| e.to_string())
        } else {
            self.joysticks
                .open(index)
                .map(OpenController::Joystick)
                .map_err(|e| e.to_string())
        };
        match device {
            Ok(device) => {
                if self
                    .open
                    .iter()
                    .all(|open| open.instance_id() != device.instance_id())
                {
                    self.open.push(device);
                }
            }
            Err(error) => eprintln!("Controller {index} unavailable: {error}"),
        }
    }

    fn detach(&mut self, instance_id: u32) {
        self.open.retain(|open| open.instance_id() != instance_id);
    }
}

/// SDL2 lifecycle and event manager.
///
/// Owns the SDL context and event pump. The actual window is owned by
/// the renderer (GL takes ownership, software consumes via into_canvas).
/// Where a replacement game window opens, so a renderer switch does not
/// move the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPlacement {
    /// A windowed game keeps its position and size.
    Windowed {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
    /// A full-screen game reopens on the same display.
    FullScreenOn { display: i32 },
}

impl WindowPlacement {
    /// Placement of an existing game window.
    pub fn of(window: &sdl2::video::Window) -> Option<Self> {
        if window.fullscreen_state() != sdl2::video::FullscreenType::Off {
            return window
                .display_index()
                .ok()
                .map(|display| Self::FullScreenOn { display });
        }
        let (x, y) = window.position();
        let (width, height) = window.size();
        Some(Self::Windowed {
            x,
            y,
            width,
            height,
        })
    }

    /// The window size to create: a windowed game's own size, else the
    /// requested one (a full-screen game is sized by its display later).
    pub fn size_or(self, width: u32, height: u32) -> (u32, u32) {
        match self {
            Self::Windowed { width, height, .. } => (width, height),
            Self::FullScreenOn { .. } => (width, height),
        }
    }
}

pub struct GameWindow {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    event_pump: EventPump,
    mouse_captured: bool,
    /// The mode last applied; `None` forces the next request through.
    pointer_mode: Option<PointerMode>,
    controllers: Option<Controllers>,
    primary_window_id: Cell<Option<u32>>,
}

impl GameWindow {
    /// Initialize SDL2. Call this before creating a window or renderer.
    pub fn new() -> Result<Self, String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        // Controllers are optional: the keyboard and mouse still play.
        let controllers = Controllers::new(&sdl)
            .map_err(|error| eprintln!("Controller support unavailable: {error}"))
            .ok();
        let event_pump = sdl.event_pump()?;

        Ok(Self {
            sdl,
            video,
            event_pump,
            mouse_captured: false,
            pointer_mode: None,
            controllers,
            primary_window_id: Cell::new(None),
        })
    }

    /// Create an SDL2 window suitable for the OpenGL backend.
    pub fn create_gl_window(
        &self,
        title: &str,
        width: u32,
        height: u32,
        placement: Option<WindowPlacement>,
    ) -> Result<sdl2::video::Window, String> {
        let gl_attr = self.video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Compatibility);
        gl_attr.set_context_version(2, 1);
        gl_attr.set_double_buffer(true);
        self.build_window(title, width, height, placement, true)
    }

    /// Create an SDL2 window suitable for the software backend.
    pub fn create_sw_window(
        &self,
        title: &str,
        width: u32,
        height: u32,
        placement: Option<WindowPlacement>,
    ) -> Result<sdl2::video::Window, String> {
        self.build_window(title, width, height, placement, false)
    }

    fn build_window(
        &self,
        title: &str,
        width: u32,
        height: u32,
        placement: Option<WindowPlacement>,
        opengl: bool,
    ) -> Result<sdl2::video::Window, String> {
        let (width, height) = placement.map_or((width, height), |placement| {
            placement.size_or(width, height)
        });
        let mut builder = self.video.window(title, width, height);
        match placement {
            Some(WindowPlacement::Windowed { x, y, .. }) => {
                builder.position(x, y);
            }
            Some(WindowPlacement::FullScreenOn { display }) => {
                match self.video.display_bounds(display) {
                    Ok(bounds) => {
                        builder.position(
                            bounds.x() + (bounds.width() as i32 - width as i32) / 2,
                            bounds.y() + (bounds.height() as i32 - height as i32) / 2,
                        );
                    }
                    Err(_) => {
                        builder.position_centered();
                    }
                }
            }
            None => {
                builder.position_centered();
            }
        }
        builder.resizable();
        if opengl {
            builder.opengl();
        }
        let mut window = builder.build().map_err(|e| e.to_string())?;
        apply_app_window_icon(&mut window);
        self.primary_window_id.set(Some(window.id()));
        Ok(window)
    }

    /// Poll all pending SDL2 events, returning game-level events.
    pub fn poll_events(&mut self) -> Vec<GameEvent> {
        // Collect raw SDL events first to avoid borrow conflicts
        let sdl_events: Vec<Event> = self.event_pump.poll_iter().collect();

        let mut events = Vec::new();
        for event in sdl_events {
            match event {
                Event::Quit { .. } => events.push(GameEvent::Quit),
                Event::Window {
                    window_id,
                    win_event: sdl2::event::WindowEvent::Resized(w, h),
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    events.push(GameEvent::Resize(w as u32, h as u32));
                }
                Event::Window {
                    window_id,
                    win_event: sdl2::event::WindowEvent::Close,
                    ..
                } => events.push(GameEvent::WindowClosed(window_id)),
                Event::Window {
                    window_id,
                    win_event: sdl2::event::WindowEvent::FocusLost,
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    // Free the pointer at once; the next policy keeps it free
                    // until focus returns.
                    self.release_mouse_capture();
                    events.push(GameEvent::FocusLost);
                }
                Event::Window {
                    window_id,
                    win_event: sdl2::event::WindowEvent::FocusGained,
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    events.push(GameEvent::FocusGained);
                }
                Event::KeyDown {
                    window_id,
                    keycode: Some(kc),
                    repeat: false,
                    ..
                } if self.primary_window_id.get() == Some(window_id) || kc == Keycode::F12 => {
                    events.push(GameEvent::KeyDown(kc));
                }
                Event::KeyUp {
                    window_id,
                    keycode: Some(kc),
                    repeat: false,
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    events.push(GameEvent::KeyUp(kc));
                }
                Event::MouseMotion {
                    window_id,
                    xrel,
                    yrel,
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    if self.mouse_captured {
                        events.push(GameEvent::MouseMotion { xrel, yrel });
                    }
                }
                Event::MouseButtonDown {
                    window_id,
                    mouse_btn,
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    events.push(GameEvent::MouseButtonDown(mouse_btn));
                }
                Event::MouseButtonDown {
                    window_id, x, y, ..
                } => events.push(GameEvent::AuxiliaryWindowClick { window_id, x, y }),
                Event::MouseButtonUp {
                    window_id,
                    mouse_btn,
                    ..
                } if self.primary_window_id.get() == Some(window_id) => {
                    events.push(GameEvent::MouseButtonUp(mouse_btn));
                }
                Event::MouseWheel {
                    window_id,
                    y,
                    direction,
                    ..
                } if self.primary_window_id.get() == Some(window_id) && y != 0 => {
                    let y = if direction == MouseWheelDirection::Flipped {
                        y.saturating_neg()
                    } else {
                        y
                    };
                    events.push(GameEvent::MouseWheel { y });
                }
                Event::JoyDeviceAdded { which, .. } => {
                    if let Some(controllers) = self.controllers.as_mut() {
                        controllers.attach(which);
                    }
                }
                Event::JoyDeviceRemoved { which, .. } => {
                    if let Some(controllers) = self.controllers.as_mut() {
                        controllers.detach(which);
                    }
                }
                _ => {}
            }
        }
        events
    }

    /// Whether the game window has keyboard focus.
    pub fn has_focus(&self) -> bool {
        let focused = self.sdl.keyboard().focused_window_id();
        focused.is_some() && focused == self.primary_window_id.get()
    }

    /// Apply a pointer mode. Repeating the current mode costs nothing.
    pub fn set_pointer_mode(&mut self, mode: PointerMode) {
        if self.pointer_mode == Some(mode) {
            return;
        }
        let mouse = self.sdl.mouse();
        let captured = mode == PointerMode::Captured;
        mouse.set_relative_mouse_mode(captured);
        mouse.show_cursor(mode == PointerMode::Visible);
        self.mouse_captured = captured;
        self.pointer_mode = Some(mode);
    }

    /// Free and show the pointer now, for example before another window
    /// takes over. The next [`Self::set_pointer_mode`] applies in full.
    pub fn release_mouse_capture(&mut self) {
        let mouse = self.sdl.mouse();
        mouse.set_relative_mouse_mode(false);
        mouse.show_cursor(true);
        self.mouse_captured = false;
        self.pointer_mode = None;
    }

    /// State of the first attached controller after the latest poll.
    pub fn controller_sample(&self) -> Option<ControllerSample> {
        let controllers = self.controllers.as_ref()?;
        controllers.open.first().map(OpenController::sample)
    }
}

/// Authored OpenV2K V mark, exported alongside the executable's ICO resource.
const APP_ICON_PNG: &[u8] = include_bytes!("../assets/openv2k.png");

/// Install the authored OpenV2K V icon on an SDL window.
///
/// The EXE resource (Windows Explorer / taskbar) is embedded separately via
/// `windres`. This covers the live window itself on every backend.
pub fn apply_app_window_icon(window: &mut sdl2::video::Window) {
    let Some((width, height, mut rgba)) = app_icon_rgba() else {
        return;
    };
    let Ok(surface) = sdl2::surface::Surface::from_data(
        &mut rgba,
        width,
        height,
        width.saturating_mul(4),
        sdl2::pixels::PixelFormatEnum::RGBA32,
    ) else {
        return;
    };
    window.set_icon(surface);
}

fn app_icon_rgba() -> Option<(u32, u32, Vec<u8>)> {
    // This embedded asset is authored RGBA8. Preserve soft glow alpha; no
    // palette/mask conversion from the old retail ICO is needed.
    let decoder = png::Decoder::new(std::io::Cursor::new(APP_ICON_PNG));
    let mut reader = decoder.read_info().ok()?;
    let mut rgba = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut rgba).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    rgba.truncate(info.buffer_size());
    Some((info.width, info.height, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_is_captured_only_in_focused_gameplay() {
        use PointerMode::*;
        for fullscreen in [false, true] {
            assert_eq!(
                PointerMode::for_frame(true, true, false, fullscreen),
                Captured
            );
            // Losing focus or opening a diagnostics window frees it.
            assert_eq!(
                PointerMode::for_frame(true, false, false, fullscreen),
                Visible
            );
            assert_eq!(
                PointerMode::for_frame(true, true, true, fullscreen),
                Visible
            );
        }
        // Menus: V2000 hides the pointer only over a full-screen game.
        assert_eq!(PointerMode::for_frame(false, true, false, true), Hidden);
        assert_eq!(PointerMode::for_frame(false, true, false, false), Visible);
        assert_eq!(PointerMode::for_frame(false, false, false, true), Visible);
    }

    #[test]
    fn app_icon_keeps_authored_size_and_transparent_glow() {
        let (width, height, rgba) = app_icon_rgba().expect("decode OpenV2K icon");
        assert_eq!((width, height), (256, 256));
        assert_eq!(rgba.len(), 256 * 256 * 4);
        assert_eq!(rgba[3], 0);
        assert!(rgba
            .chunks_exact(4)
            .any(|pixel| pixel[3] > 0 && pixel[3] < 255));
        assert!(rgba
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 200 && pixel[1] > 100 && pixel[3] > 200));
    }
}
