//! How the game window occupies its monitor, and the resolutions it offers.
//!
//! Every size here is in physical pixels. Retail's Display row is the
//! `+0x1C` Full Screen word; Borderless and sizes other than the original
//! display records are port choices.

use serde::{Deserialize, Serialize};

/// The original 4:3 sizes: Section-5 display records 1..3 (High detail).
/// Record 0, 320x240, is the Low detail tier and keeps its own option.
pub const ORIGINAL_RESOLUTIONS: [(u32, u32); 3] = [(640, 480), (800, 600), (1024, 768)];

/// The Display row's values.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WindowMode {
    /// Retail value 0: a window whose client area is the resolution
    /// (`FUN_004A8D90`, `SetCooperativeLevel(DDSCL_NORMAL)`).
    #[default]
    Window,
    /// Retail value 1: the display switches to the resolution
    /// (`FUN_004A8D90`, `SetCooperativeLevel(0x51)` then
    /// `SetDisplayMode(width, height, 16, 0, 0)`).
    FullScreen,
    /// Port: a borderless window covering the desktop without a mode change.
    /// The resolution only picks the original layout.
    Borderless,
}

impl WindowMode {
    /// Display row order; values 0 and 1 are retail's.
    pub const ALL: [Self; 3] = [Self::Window, Self::FullScreen, Self::Borderless];

    pub const fn index(self) -> u32 {
        match self {
            Self::Window => 0,
            Self::FullScreen => 1,
            Self::Borderless => 2,
        }
    }

    pub const fn from_index(index: u32) -> Self {
        match index {
            1 => Self::FullScreen,
            2 => Self::Borderless,
            _ => Self::Window,
        }
    }

    /// Retail strings 6 and 7 name the first two values.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Window => "In a Window",
            Self::FullScreen => "Full Screen",
            Self::Borderless => "Borderless",
        }
    }

    /// Alt+Enter (`FUN_0042D5D0`) stores `Full Screen == 0`.
    pub const fn alt_enter(self) -> Self {
        match self {
            Self::Window => Self::FullScreen,
            Self::FullScreen | Self::Borderless => Self::Window,
        }
    }

    /// Whether the game covers its whole display.
    pub const fn covers_display(self) -> bool {
        !matches!(self, Self::Window)
    }

    /// The startup search increments the word and wraps (`FUN_0042D340`).
    const fn next(self) -> Self {
        Self::from_index((self.index() + 1) % Self::ALL.len() as u32)
    }
}

/// The largest original resolution that fits `size`. The originals are 4:3,
/// so this is also the largest that fits the size's 4:3 area. A size that
/// holds none of them takes the smallest.
pub fn original_layout(size: (u32, u32)) -> (u32, u32) {
    ORIGINAL_RESOLUTIONS
        .iter()
        .rev()
        .copied()
        .find(|&(width, height)| width <= size.0 && height <= size.1)
        .unwrap_or(ORIGINAL_RESOLUTIONS[0])
}

/// Whether an original High tier fits `size`.
pub fn holds_an_original(size: (u32, u32)) -> bool {
    let (width, height) = ORIGINAL_RESOLUTIONS[0];
    size.0 >= width && size.1 >= height
}

/// What the game window should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayRequest {
    pub mode: WindowMode,
    /// Client size of a window, or the display mode of Full Screen.
    /// Borderless covers the desktop whatever this is.
    pub size: (u32, u32),
}

/// What a monitor offers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayModes {
    /// The desktop's current size, when known.
    pub desktop: Option<(u32, u32)>,
    /// Every distinct size the monitor reports, ascending.
    reported: Vec<(u32, u32)>,
}

impl DisplayModes {
    pub fn new(
        desktop: Option<(u32, u32)>,
        reported: impl IntoIterator<Item = (u32, u32)>,
    ) -> Self {
        let mut reported: Vec<_> = reported.into_iter().collect();
        reported.sort_unstable();
        reported.dedup();
        Self { desktop, reported }
    }

    pub fn reported(&self) -> &[(u32, u32)] {
        &self.reported
    }

    /// The Resolution row's entries for `mode`, ascending by width, then
    /// height. Sizes that hold no original tier are left out: the tier rule
    /// has nothing to draw them with.
    pub fn resolutions(&self, mode: WindowMode) -> Vec<(u32, u32)> {
        let mut sizes: Vec<_> = match mode {
            WindowMode::Borderless => return ORIGINAL_RESOLUTIONS.to_vec(),
            WindowMode::FullScreen => self.reported.clone(),
            WindowMode::Window => ORIGINAL_RESOLUTIONS
                .iter()
                .chain(&self.reported)
                .copied()
                .filter(|&size| self.fits_desktop(size))
                .collect(),
        };
        sizes.retain(|&size| holds_an_original(size));
        sizes.sort_unstable();
        sizes.dedup();
        sizes
    }

    fn fits_desktop(&self, size: (u32, u32)) -> bool {
        self.desktop
            .is_none_or(|(width, height)| size.0 <= width && size.1 <= height)
    }

    /// The entry `size` selects in `mode`'s row: itself when listed, the
    /// original layout in Borderless, else the next listed size, wrapping
    /// to the first, as the startup search's index does. `None` when the
    /// row is empty.
    pub fn resolve(&self, mode: WindowMode, size: (u32, u32)) -> Option<(u32, u32)> {
        if mode == WindowMode::Borderless {
            return Some(original_layout(size));
        }
        let sizes = self.resolutions(mode);
        sizes
            .iter()
            .copied()
            .find(|&listed| listed >= size)
            .or_else(|| sizes.first().copied())
    }

    /// Row index of the entry `size` selects in `mode`.
    pub fn selection(&self, mode: WindowMode, size: (u32, u32)) -> Option<usize> {
        let selected = self.resolve(mode, size)?;
        self.resolutions(mode)
            .iter()
            .position(|&listed| listed == selected)
    }

    /// `FUN_0042D340`'s startup order from the saved choice: every resolution
    /// of the saved window mode, starting at the saved one and wrapping, then
    /// the same for each following window mode. Retail then tries the other
    /// renderer and Bilinear value; the port's renderer falls back before
    /// its display is chosen, and Bilinear never affects the display.
    pub fn startup_order(&self, mode: WindowMode, size: (u32, u32)) -> Vec<DisplayRequest> {
        let mut order = Vec::new();
        let mut current = mode;
        for _ in WindowMode::ALL {
            let sizes = self.resolutions(current);
            let start = self.selection(current, size).unwrap_or(0);
            for &size in sizes[start..].iter().chain(&sizes[..start]) {
                order.push(DisplayRequest {
                    mode: current,
                    size,
                });
            }
            current = current.next();
        }
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3840x2160 monitor reporting a typical list, desktop at its largest.
    fn monitor() -> DisplayModes {
        DisplayModes::new(
            Some((3840, 2160)),
            [
                (3840, 2160),
                (640, 480),
                (720, 480),
                (800, 600),
                (1024, 768),
                (1280, 720),
                (1920, 1080),
                (2560, 1440),
                (1920, 1080),
            ],
        )
    }

    #[test]
    fn retail_values_keep_their_indices_and_alt_enter_stores_not_full_screen() {
        assert_eq!(WindowMode::Window.index(), 0);
        assert_eq!(WindowMode::FullScreen.index(), 1);
        assert_eq!(WindowMode::Borderless.index(), 2);
        for mode in WindowMode::ALL {
            assert_eq!(WindowMode::from_index(mode.index()), mode);
        }
        assert_eq!(WindowMode::Window.alt_enter(), WindowMode::FullScreen);
        assert_eq!(WindowMode::FullScreen.alt_enter(), WindowMode::Window);
        assert_eq!(WindowMode::Borderless.alt_enter(), WindowMode::Window);
        assert!(!WindowMode::Window.covers_display());
        assert!(WindowMode::FullScreen.covers_display());
        assert!(WindowMode::Borderless.covers_display());
    }

    #[test]
    fn original_layout_is_the_largest_original_inside_the_four_three_area() {
        for (size, layout) in [
            ((640, 480), (640, 480)),
            ((800, 600), (800, 600)),
            ((1024, 768), (1024, 768)),
            ((720, 480), (640, 480)),
            ((1280, 720), (800, 600)),
            ((1280, 1024), (1024, 768)),
            ((1920, 1080), (1024, 768)),
            ((3840, 2160), (1024, 768)),
            ((768, 1024), (640, 480)),
            ((320, 240), (640, 480)),
        ] {
            assert_eq!(original_layout(size), layout, "{size:?}");
        }
    }

    #[test]
    fn rows_follow_the_window_mode() {
        let modes = monitor();
        assert_eq!(
            modes.resolutions(WindowMode::Window),
            [
                (640, 480),
                (720, 480),
                (800, 600),
                (1024, 768),
                (1280, 720),
                (1920, 1080),
                (2560, 1440),
                (3840, 2160)
            ]
        );
        assert_eq!(
            modes.resolutions(WindowMode::FullScreen),
            modes.resolutions(WindowMode::Window)
        );
        assert_eq!(
            modes.resolutions(WindowMode::Borderless),
            ORIGINAL_RESOLUTIONS
        );

        // A window must fit the desktop; full screen offers reported modes
        // only, so an original the monitor lacks is not listed there.
        let small = DisplayModes::new(Some((1280, 720)), [(1280, 720), (1920, 1080), (640, 480)]);
        assert_eq!(
            small.resolutions(WindowMode::Window),
            [(640, 480), (800, 600), (1280, 720)]
        );
        assert_eq!(
            small.resolutions(WindowMode::FullScreen),
            [(640, 480), (1280, 720), (1920, 1080)]
        );
    }

    #[test]
    fn sizes_too_small_for_an_original_tier_are_not_offered() {
        let modes = DisplayModes::new(Some((1024, 768)), [(320, 200), (640, 400), (1024, 768)]);
        assert_eq!(modes.resolutions(WindowMode::FullScreen), [(1024, 768)]);
        assert_eq!(
            modes.resolutions(WindowMode::Window),
            [(640, 480), (800, 600), (1024, 768)]
        );
    }

    #[test]
    fn unknown_monitors_offer_originals_in_a_window_only() {
        let modes = DisplayModes::default();
        assert_eq!(modes.resolutions(WindowMode::Window), ORIGINAL_RESOLUTIONS);
        assert!(modes.resolutions(WindowMode::FullScreen).is_empty());
        assert_eq!(modes.resolve(WindowMode::FullScreen, (800, 600)), None);
        assert_eq!(modes.selection(WindowMode::FullScreen, (800, 600)), None);
    }

    #[test]
    fn an_unlisted_size_selects_the_next_entry_and_wraps() {
        let modes = DisplayModes::new(Some((1920, 1080)), [(1280, 720), (1920, 1080)]);
        let full = WindowMode::FullScreen;
        assert_eq!(modes.resolve(full, (1280, 720)), Some((1280, 720)));
        assert_eq!(modes.resolve(full, (1024, 768)), Some((1280, 720)));
        assert_eq!(modes.resolve(full, (1280, 1024)), Some((1920, 1080)));
        assert_eq!(modes.resolve(full, (2560, 1440)), Some((1280, 720)));
        assert_eq!(modes.selection(full, (1600, 900)), Some(1));
        let window = WindowMode::Window;
        assert_eq!(modes.resolve(window, (2560, 1440)), Some((640, 480)));
        assert_eq!(
            modes.resolve(WindowMode::Borderless, (1920, 1080)),
            Some((1024, 768))
        );
        assert_eq!(
            modes.selection(WindowMode::Borderless, (1280, 720)),
            Some(1)
        );
    }

    #[test]
    fn startup_order_cycles_resolutions_then_window_modes_like_retail() {
        let modes = DisplayModes::new(Some((1280, 720)), [(640, 480), (1280, 720)]);
        let order: Vec<_> = modes
            .startup_order(WindowMode::FullScreen, (1280, 720))
            .into_iter()
            .map(|request| (request.mode, request.size))
            .collect();
        use WindowMode::*;
        assert_eq!(
            order,
            [
                (FullScreen, (1280, 720)),
                (FullScreen, (640, 480)),
                (Borderless, (800, 600)),
                (Borderless, (1024, 768)),
                (Borderless, (640, 480)),
                (Window, (1280, 720)),
                (Window, (640, 480)),
                (Window, (800, 600)),
            ]
        );
        // A saved full-screen size the monitor lacks starts at the next one.
        let first = modes.startup_order(FullScreen, (1024, 768))[0];
        assert_eq!((first.mode, first.size), (FullScreen, (1280, 720)));
    }
}
