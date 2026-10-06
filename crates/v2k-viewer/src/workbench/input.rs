//! Clock-driven workbench input state that does not depend on SDL repeat events.

use std::time::{Duration, Instant};

const IDLE_EVENT_WAIT: Duration = Duration::from_millis(250);

/// Delay before a held navigation key starts advancing through the catalog.
const NAVIGATION_REPEAT_DELAY: Duration = Duration::from_millis(300);

/// Catalog navigation cadence after the initial hold delay.
const NAVIGATION_REPEAT_INTERVAL: Duration = Duration::from_millis(65);

/// Prevent a long render or debugger pause from causing an unbounded burst.
const MAX_REPEAT_STEPS_PER_TICK: usize = 4;

/// Keeps pasted catalog positions inside the fixed-width toolbar field.
const MAX_CATALOG_POSITION_DIGITS: usize = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextInputMode {
    Search,
    CatalogPosition,
}

/// Owns mutually-exclusive toolbar text focus and the transient catalog
/// position buffer. Search text itself remains persistent in the workbench
/// state, while every position edit deliberately starts with a blank buffer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TextInputState {
    active: Option<TextInputMode>,
    catalog_position: String,
}

impl TextInputState {
    pub fn activate_search(&mut self) {
        self.active = Some(TextInputMode::Search);
        self.catalog_position.clear();
    }

    pub fn activate_catalog_position(&mut self) {
        self.active = Some(TextInputMode::CatalogPosition);
        self.catalog_position.clear();
    }

    pub fn cancel(&mut self) {
        self.active = None;
        self.catalog_position.clear();
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn is_search_active(&self) -> bool {
        self.active == Some(TextInputMode::Search)
    }

    pub fn is_catalog_position_active(&self) -> bool {
        self.active == Some(TextInputMode::CatalogPosition)
    }

    pub fn push_catalog_position_text(&mut self, text: &str) {
        if self.is_catalog_position_active() {
            let remaining = MAX_CATALOG_POSITION_DIGITS.saturating_sub(self.catalog_position.len());
            self.catalog_position
                .extend(text.chars().filter(char::is_ascii_digit).take(remaining));
        }
    }

    pub fn backspace_catalog_position(&mut self) {
        if self.is_catalog_position_active() {
            self.catalog_position.pop();
        }
    }

    pub fn catalog_position(&self) -> &str {
        &self.catalog_position
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CatalogPositionError {
    Empty,
    Invalid,
    OutOfRange,
}

/// Parses the one-based position shown to users into a zero-based catalog
/// index. Deliberately reject non-ASCII numeric characters: SDL text input can
/// deliver Unicode digits, but the toolbar font and catalog notation are ASCII.
pub(super) fn parse_catalog_position(
    input: &str,
    item_count: usize,
) -> Result<usize, CatalogPositionError> {
    if input.is_empty() {
        return Err(CatalogPositionError::Empty);
    }
    if !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CatalogPositionError::Invalid);
    }
    let one_based = input
        .parse::<usize>()
        .map_err(|_| CatalogPositionError::OutOfRange)?;
    if !(1..=item_count).contains(&one_based) {
        return Err(CatalogPositionError::OutOfRange);
    }
    Ok(one_based - 1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NavigationDirection {
    Previous,
    Next,
}

impl NavigationDirection {
    const fn opposite(self) -> Self {
        match self {
            Self::Previous => Self::Next,
            Self::Next => Self::Previous,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NavigationSteps {
    pub direction: NavigationDirection,
    pub count: usize,
}

/// Tracks held previous/next keys using a clock-driven repeat schedule.
///
/// `press` reports whether the caller should perform the immediate first step.
/// Subsequent steps come from `take_due`, independent of SDL/OS key-repeat
/// settings and render frame count.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct NavigationRepeat {
    previous_down: bool,
    next_down: bool,
    active: Option<NavigationDirection>,
    next_repeat_at: Option<Instant>,
}

impl NavigationRepeat {
    pub fn press(&mut self, direction: NavigationDirection, now: Instant) -> bool {
        let already_down = self.is_down(direction);
        self.set_down(direction, true);
        if already_down {
            return false;
        }

        self.active = Some(direction);
        self.next_repeat_at = Some(now + NAVIGATION_REPEAT_DELAY);
        true
    }

    pub fn release(&mut self, direction: NavigationDirection, now: Instant) {
        self.set_down(direction, false);
        if self.active != Some(direction) {
            return;
        }

        let resumed = direction.opposite();
        if self.is_down(resumed) {
            self.active = Some(resumed);
            self.next_repeat_at = Some(now + NAVIGATION_REPEAT_DELAY);
        } else {
            self.active = None;
            self.next_repeat_at = None;
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn take_due(&mut self, now: Instant) -> Option<NavigationSteps> {
        let direction = self.active?;
        let next_repeat_at = self.next_repeat_at?;
        if now < next_repeat_at {
            return None;
        }

        let elapsed_intervals =
            now.duration_since(next_repeat_at).as_nanos() / NAVIGATION_REPEAT_INTERVAL.as_nanos();
        let requested = 1usize.saturating_add(
            usize::try_from(elapsed_intervals).unwrap_or(MAX_REPEAT_STEPS_PER_TICK),
        );
        let count = requested.min(MAX_REPEAT_STEPS_PER_TICK);

        self.next_repeat_at = if requested > MAX_REPEAT_STEPS_PER_TICK {
            Some(now + NAVIGATION_REPEAT_INTERVAL)
        } else {
            Some(next_repeat_at + NAVIGATION_REPEAT_INTERVAL * count as u32)
        };

        Some(NavigationSteps { direction, count })
    }

    pub fn wait_timeout_ms(&self, now: Instant) -> u32 {
        let duration = self
            .next_repeat_at
            .map(|deadline| deadline.saturating_duration_since(now))
            .unwrap_or(IDLE_EVENT_WAIT)
            .min(IDLE_EVENT_WAIT);
        let milliseconds =
            duration.as_millis() + u128::from(duration.subsec_nanos() % 1_000_000 != 0);
        milliseconds.min(u32::MAX as u128) as u32
    }

    fn is_down(&self, direction: NavigationDirection) -> bool {
        match direction {
            NavigationDirection::Previous => self.previous_down,
            NavigationDirection::Next => self.next_down,
        }
    }

    fn set_down(&mut self, direction: NavigationDirection, down: bool) {
        match direction {
            NavigationDirection::Previous => self.previous_down = down,
            NavigationDirection::Next => self.next_down = down,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_press_steps_immediately_then_waits_for_delay() {
        let start = Instant::now();
        let mut input = NavigationRepeat::default();

        assert!(input.press(NavigationDirection::Next, start));
        assert!(!input.press(NavigationDirection::Next, start));
        assert_eq!(
            input.take_due(start + NAVIGATION_REPEAT_DELAY - Duration::from_millis(1)),
            None
        );
        assert_eq!(
            input.take_due(start + NAVIGATION_REPEAT_DELAY),
            Some(NavigationSteps {
                direction: NavigationDirection::Next,
                count: 1,
            })
        );
    }

    #[test]
    fn repeat_count_is_elapsed_time_based_instead_of_frame_based() {
        let start = Instant::now();
        let final_tick = start + NAVIGATION_REPEAT_DELAY + NAVIGATION_REPEAT_INTERVAL * 3;
        let mut sampled = NavigationRepeat::default();
        let mut batched = NavigationRepeat::default();
        sampled.press(NavigationDirection::Previous, start);
        batched.press(NavigationDirection::Previous, start);

        let sampled_count = (0..=3)
            .map(|interval| {
                sampled
                    .take_due(
                        start + NAVIGATION_REPEAT_DELAY + NAVIGATION_REPEAT_INTERVAL * interval,
                    )
                    .map_or(0, |steps| steps.count)
            })
            .sum::<usize>();
        let batched_steps = batched.take_due(final_tick).unwrap();

        assert_eq!(sampled_count, 4);
        assert_eq!(batched_steps.count, sampled_count);
        assert_eq!(batched_steps.direction, NavigationDirection::Previous);
    }

    #[test]
    fn release_and_focus_style_clear_cancel_repeat() {
        let start = Instant::now();
        let mut input = NavigationRepeat::default();
        input.press(NavigationDirection::Previous, start);
        input.release(
            NavigationDirection::Previous,
            start + Duration::from_millis(20),
        );
        assert_eq!(input.take_due(start + NAVIGATION_REPEAT_DELAY * 2), None);

        input.press(NavigationDirection::Next, start);
        input.clear();
        assert_eq!(input.take_due(start + NAVIGATION_REPEAT_DELAY * 2), None);
    }

    #[test]
    fn long_hitch_is_capped_and_repeat_schedule_resynchronizes() {
        let start = Instant::now();
        let mut input = NavigationRepeat::default();
        input.press(NavigationDirection::Next, start);
        let hitch_end = start + NAVIGATION_REPEAT_DELAY + NAVIGATION_REPEAT_INTERVAL * 100;

        assert_eq!(
            input.take_due(hitch_end),
            Some(NavigationSteps {
                direction: NavigationDirection::Next,
                count: MAX_REPEAT_STEPS_PER_TICK,
            })
        );
        assert_eq!(input.take_due(hitch_end), None);
        assert_eq!(
            input.take_due(hitch_end + NAVIGATION_REPEAT_INTERVAL),
            Some(NavigationSteps {
                direction: NavigationDirection::Next,
                count: 1,
            })
        );
    }

    #[test]
    fn most_recent_direction_resumes_the_other_held_key_after_release() {
        let start = Instant::now();
        let mut input = NavigationRepeat::default();
        assert!(input.press(NavigationDirection::Previous, start));
        assert!(input.press(NavigationDirection::Next, start + Duration::from_millis(10)));
        input.release(NavigationDirection::Next, start + Duration::from_millis(20));

        assert_eq!(
            input.take_due(start + Duration::from_millis(20) + NAVIGATION_REPEAT_DELAY),
            Some(NavigationSteps {
                direction: NavigationDirection::Previous,
                count: 1,
            })
        );
    }

    #[test]
    fn toolbar_text_modes_are_mutually_exclusive_and_position_starts_blank() {
        let mut input = TextInputState::default();
        input.activate_catalog_position();
        input.push_catalog_position_text("5000");
        assert!(input.is_catalog_position_active());
        assert_eq!(input.catalog_position(), "5000");

        input.activate_search();
        assert!(input.is_search_active());
        assert!(!input.is_catalog_position_active());
        assert_eq!(input.catalog_position(), "");

        input.activate_catalog_position();
        assert!(input.is_catalog_position_active());
        assert_eq!(input.catalog_position(), "");
        input.cancel();
        assert!(!input.is_active());
    }

    #[test]
    fn catalog_position_buffer_accepts_ascii_digits_only_and_supports_backspace() {
        let mut input = TextInputState::default();
        input.activate_catalog_position();
        input.push_catalog_position_text("12x３ 456789012345");
        assert_eq!(input.catalog_position(), "124567890");
        input.backspace_catalog_position();
        assert_eq!(input.catalog_position(), "12456789");
    }

    #[test]
    fn catalog_positions_are_one_based_and_range_checked() {
        assert_eq!(parse_catalog_position("1", 760), Ok(0));
        assert_eq!(parse_catalog_position("760", 760), Ok(759));
        assert_eq!(
            parse_catalog_position("", 760),
            Err(CatalogPositionError::Empty)
        );
        assert_eq!(
            parse_catalog_position("0", 760),
            Err(CatalogPositionError::OutOfRange)
        );
        assert_eq!(
            parse_catalog_position("761", 760),
            Err(CatalogPositionError::OutOfRange)
        );
        assert_eq!(
            parse_catalog_position("12x", 760),
            Err(CatalogPositionError::Invalid)
        );
        assert_eq!(
            parse_catalog_position("１", 760),
            Err(CatalogPositionError::Invalid)
        );
        assert_eq!(
            parse_catalog_position("999999999999999999999999999999", 760),
            Err(CatalogPositionError::OutOfRange)
        );
    }
}
