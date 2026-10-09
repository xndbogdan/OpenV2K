//! Making an SDL window what a [`DisplayRequest`] asks for, in physical
//! pixels, or leaving it as it was.

use sdl2::pixels::PixelFormatEnum;
use sdl2::video::{DisplayMode, FullscreenType, Window, WindowPos};
use sdl2::VideoSubsystem;

use crate::config::{DisplayModes, DisplayRequest, WindowMode};

/// What `display` offers: SDL's mode list collapsed to sizes (formats and
/// refresh rates are SDL's choice when it switches), and the desktop mode.
pub fn display_modes(video: &VideoSubsystem, display: i32) -> DisplayModes {
    let size =
        |mode: DisplayMode| (mode.w > 0 && mode.h > 0).then_some((mode.w as u32, mode.h as u32));
    let desktop = video.desktop_display_mode(display).ok().and_then(size);
    let count = video.num_display_modes(display).unwrap_or(0);
    let reported =
        (0..count).filter_map(|index| video.display_mode(display, index).ok().and_then(size));
    DisplayModes::new(desktop, reported)
}

/// Apply `request` to `window`. A failure restores the previous window and
/// display mode, as `FUN_0044E0E0` restarts the previous display when a new
/// one fails.
///
/// - A window takes the client size and stays where it was, moved inside the
///   display's work area as `FUN_004A8D90` does.
/// - Full Screen switches the window's display to exactly the requested
///   mode, which must be one the display reports; SDL would otherwise
///   pick the closest. SDL restores the desktop mode when the window leaves
///   full screen, minimizes on losing focus, or closes, and sets the mode
///   again when the window is restored, where `FUN_00494720` restarts the
///   display.
/// - Borderless covers the desktop without a mode change.
pub fn apply_display(window: &mut Window, request: DisplayRequest) -> Result<(), String> {
    let previous = Previous::capture(window)?;
    let Err(error) = configure(window, request) else {
        return Ok(());
    };
    match previous.restore(window) {
        Ok(()) => Err(error),
        Err(rollback) => Err(format!(
            "{error}; restoring the previous display failed: {rollback}"
        )),
    }
}

fn configure(window: &mut Window, request: DisplayRequest) -> Result<(), String> {
    let (width, height) = request.size;
    match request.mode {
        WindowMode::Window => {
            window.set_fullscreen(FullscreenType::Off)?;
            window
                .set_size(width, height)
                .map_err(|error| error.to_string())?;
            keep_inside_work_area(window)?;
            check_size(window, request.size)
        }
        WindowMode::FullScreen => {
            let display = window.display_index()?;
            let video = window.subsystem().clone();
            if !display_modes(&video, display)
                .reported()
                .contains(&request.size)
            {
                return Err(format!(
                    "display {display} does not report a {width}x{height} mode"
                ));
            }
            // Start from a window, as `FUN_0044E0E0` releases the old display
            // first. A full-screen SDL window given a new mode keeps its old
            // native size.
            if window.fullscreen_state() != FullscreenType::Off {
                window.set_fullscreen(FullscreenType::Off)?;
            }
            set_window_display_mode(
                window,
                DisplayMode::new(PixelFormatEnum::Unknown, width as i32, height as i32, 0),
            )?;
            check_mode(window.display_mode()?, request.size)?;
            window.set_fullscreen(FullscreenType::True)?;
            // A hidden or minimized window only records the request.
            if exclusive_mode_is_active(window) {
                check_mode(video.current_display_mode(display)?, request.size)?;
            }
            check_size(window, request.size)
        }
        WindowMode::Borderless => {
            if window.fullscreen_state() == FullscreenType::True {
                window.set_fullscreen(FullscreenType::Off)?;
            }
            window.set_fullscreen(FullscreenType::Desktop)?;
            let display = window.display_index()?;
            let desktop = window.subsystem().desktop_display_mode(display)?;
            check_size(window, (desktop.w as u32, desktop.h as u32))
        }
    }
}

/// `FUN_004A8D90`'s windowed placement: keep the window's corner, then
/// shift its frame onto the work area's right and bottom edges, then its
/// left and top ones, so an oversized window keeps its title bar visible.
fn keep_inside_work_area(window: &mut Window) -> Result<(), String> {
    let display = window.display_index()?;
    let area = window.subsystem().display_usable_bounds(display)?;
    let (top, left, bottom, right) = window.border_size().unwrap_or((0, 0, 0, 0));
    let (x, y) = window.position();
    let (width, height) = window.size();
    let mut frame_left = x - i32::from(left);
    let mut frame_top = y - i32::from(top);
    let frame_right = x + width as i32 + i32::from(right);
    let frame_bottom = y + height as i32 + i32::from(bottom);
    if frame_right > area.right() {
        frame_left -= frame_right - area.right();
    }
    if frame_bottom > area.bottom() {
        frame_top -= frame_bottom - area.bottom();
    }
    frame_left = frame_left.max(area.left());
    frame_top = frame_top.max(area.top());
    window.set_position(
        WindowPos::Positioned(frame_left + i32::from(left)),
        WindowPos::Positioned(frame_top + i32::from(top)),
    );
    Ok(())
}

/// Both SDL's size and the native client area must be `expected`.
fn check_size(window: &Window, expected: (u32, u32)) -> Result<(), String> {
    let (mut width, mut height) = (0, 0);
    // SAFETY: a live SDL window on this thread; plain out-parameters.
    unsafe { sdl2::sys::SDL_GetWindowSizeInPixels(window.raw(), &mut width, &mut height) };
    for actual in [window.size(), (width.max(0) as u32, height.max(0) as u32)] {
        if actual != expected {
            return Err(format!(
                "SDL made the window {}x{}, not {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ));
        }
    }
    Ok(())
}

fn check_mode(mode: DisplayMode, expected: (u32, u32)) -> Result<(), String> {
    if (mode.w, mode.h) != (expected.0 as i32, expected.1 as i32) {
        return Err(format!(
            "SDL chose the {}x{} display mode, not {}x{}",
            mode.w, mode.h, expected.0, expected.1
        ));
    }
    Ok(())
}

fn exclusive_mode_is_active(window: &Window) -> bool {
    window.fullscreen_state() == FullscreenType::True
        && window.window_flags() & sdl2::sys::SDL_WindowFlags::SDL_WINDOW_SHOWN as u32 != 0
        && !window.is_minimized()
}

/// `Window::set_display_mode` in rust-sdl2 0.38 passes a pointer to a
/// temporary that ends inside its `match` arm, before SDL reads it. This
/// keeps the native mode alive across the call, which copies it.
fn set_window_display_mode(window: &mut Window, mode: DisplayMode) -> Result<(), String> {
    let native = mode.to_ll();
    // SAFETY: `window` owns a live SDL window on this thread and `native`
    // outlives the synchronous call.
    let result = unsafe { sdl2::sys::SDL_SetWindowDisplayMode(window.raw(), &native) };
    if result < 0 {
        Err(sdl2::get_error())
    } else {
        Ok(())
    }
}

/// The window as it was before a request.
struct Previous {
    fullscreen: FullscreenType,
    mode: Option<DisplayMode>,
    position: (i32, i32),
    size: (u32, u32),
}

impl Previous {
    fn capture(window: &Window) -> Result<Self, String> {
        Ok(Self {
            fullscreen: window.fullscreen_state(),
            // A window that never went full screen may have no mode yet.
            mode: window.display_mode().ok(),
            position: window.position(),
            size: window.size(),
        })
    }

    fn restore(&self, window: &mut Window) -> Result<(), String> {
        window.set_fullscreen(FullscreenType::Off)?;
        if let Some(mode) = self.mode {
            set_window_display_mode(window, mode)?;
        }
        window.set_position(
            WindowPos::Positioned(self.position.0),
            WindowPos::Positioned(self.position.1),
        );
        window
            .set_size(self.size.0, self.size.1)
            .map_err(|error| error.to_string())?;
        window.set_fullscreen(self.fullscreen)?;
        if window.fullscreen_state() != self.fullscreen || window.size() != self.size {
            return Err("SDL did not restore the previous window".into());
        }
        Ok(())
    }
}
