use std::cell::Cell;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::EventPump;
use sdl2::Sdl;
use sdl2::VideoSubsystem;

/// Input events consumed by the game loop.
pub enum GameEvent {
    Quit,
    Resize(u32, u32),
    KeyDown(Keycode),
    KeyUp(Keycode),
    MouseMotion { xrel: i32, yrel: i32 },
    MouseButtonDown(MouseButton),
    MouseButtonUp(MouseButton),
    FocusLost,
    AuxiliaryWindowClick { window_id: u32, x: i32, y: i32 },
    WindowClosed(u32),
}

/// SDL2 lifecycle and event manager.
///
/// Owns the SDL context and event pump. The actual window is owned by
/// the renderer (GL takes ownership, software consumes via into_canvas).
pub struct GameWindow {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    event_pump: EventPump,
    mouse_captured: bool,
    primary_window_id: Cell<Option<u32>>,
}

impl GameWindow {
    /// Initialize SDL2. Call this before creating a window or renderer.
    pub fn new() -> Result<Self, String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let event_pump = sdl.event_pump()?;

        Ok(Self {
            sdl,
            video,
            event_pump,
            mouse_captured: false,
            primary_window_id: Cell::new(None),
        })
    }

    /// Create an SDL2 window suitable for the OpenGL backend.
    pub fn create_gl_window(
        &self,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<sdl2::video::Window, String> {
        let gl_attr = self.video.gl_attr();
        gl_attr.set_context_profile(sdl2::video::GLProfile::Compatibility);
        gl_attr.set_context_version(2, 1);
        gl_attr.set_double_buffer(true);

        let mut window = self
            .video
            .window(title, width, height)
            .position_centered()
            .resizable()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;
        apply_app_window_icon(&mut window);
        self.primary_window_id.set(Some(window.id()));
        Ok(window)
    }

    /// Create an SDL2 window suitable for the software backend.
    pub fn create_sw_window(
        &self,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<sdl2::video::Window, String> {
        let mut window = self
            .video
            .window(title, width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
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
                    self.sdl.mouse().set_relative_mouse_mode(false);
                    self.mouse_captured = false;
                    events.push(GameEvent::FocusLost);
                }
                Event::KeyDown {
                    window_id,
                    keycode: Some(kc),
                    repeat: false,
                    ..
                } if self.primary_window_id.get() == Some(window_id) || kc == Keycode::F12 => {
                    if kc == Keycode::Escape && self.mouse_captured {
                        self.sdl.mouse().set_relative_mouse_mode(false);
                        self.mouse_captured = false;
                    }
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
                    if !self.mouse_captured {
                        self.sdl.mouse().set_relative_mouse_mode(true);
                        self.mouse_captured = true;
                    }
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
                _ => {}
            }
        }
        events
    }

    /// Release relative-mode capture so auxiliary diagnostic windows can use
    /// the normal OS cursor. Clicking the game window captures it again.
    pub fn release_mouse_capture(&mut self) {
        self.sdl.mouse().set_relative_mouse_mode(false);
        self.mouse_captured = false;
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
