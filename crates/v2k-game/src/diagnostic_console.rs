//! Optional developer console, created only on an explicit toggle.
//! An inherited terminal belongs to the invoking shell and is never hidden.

#[cfg(windows)]
pub(super) use windows::DiagnosticConsole;

#[cfg(windows)]
mod windows {
    use std::{ffi::c_void, io};

    const SW_HIDE: i32 = 0;
    const SW_SHOWNOACTIVATE: i32 = 4;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleWindow() -> *mut c_void;
        fn AllocConsole() -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(window: *mut c_void, command: i32) -> i32;
        fn IsWindowVisible(window: *mut c_void) -> i32;
    }

    pub(crate) struct DiagnosticConsole {
        window: *mut c_void,
        owned: bool,
    }

    impl DiagnosticConsole {
        pub(crate) fn new() -> Self {
            // SAFETY: This query takes no pointers. An existing console belongs
            // to the invoking terminal; normal GUI startup creates none.
            let window = unsafe { GetConsoleWindow() };
            Self {
                window,
                owned: false,
            }
        }

        pub(crate) fn toggle(&mut self) -> io::Result<bool> {
            if self.window.is_null() {
                // A detached launcher may supply no console at all. Create
                // one only when requested; ordinary startup stays window-free.
                // SAFETY: AllocConsole takes no pointers and belongs to this
                // process. Existing consoles are never detached or freed.
                if unsafe { AllocConsole() } == 0 {
                    return Err(io::Error::last_os_error());
                }
                self.window = unsafe { GetConsoleWindow() };
                if self.window.is_null() {
                    return Err(io::Error::other("allocated console has no window"));
                }
                self.owned = true;
                // Allocation already shows the new console: the first key
                // press opens it rather than immediately hiding it again.
                return Ok(true);
            }
            if !self.owned {
                return Ok(false);
            }
            // SAFETY: This process remains attached for the object's lifetime.
            // Showing without activation leaves the game receiving the next
            // toggle key, while its mouse capture is released by the caller.
            let visible = unsafe { IsWindowVisible(self.window) } != 0;
            unsafe {
                ShowWindow(
                    self.window,
                    if visible { SW_HIDE } else { SW_SHOWNOACTIVATE },
                );
            }
            Ok(!visible)
        }
    }
}

#[cfg(not(windows))]
pub(super) struct DiagnosticConsole;

#[cfg(not(windows))]
impl DiagnosticConsole {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn toggle(&mut self) -> std::io::Result<bool> {
        // The invoking terminal owns stdout/stderr on other platforms.
        Ok(false)
    }
}
