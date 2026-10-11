//! Retail V2000's shared 50 Hz world clock.
//!
//! `FUN_00428B00` accumulates the frame delta in microseconds, clamps each
//! individual advance to 125 ms, and publishes the low 32 bits of the total
//! divided by 20 ms. Keeping the accumulated microseconds (rather than adding
//! a truncated tick delta per frame) preserves the sub-tick remainder.

/// Duration of one retail world tick in microseconds.
pub const RETAIL_TICK_MICROS: u64 = 20_000;

/// Maximum frame delta charged by one active retail engine update.
///
/// Vehicle, camera, HUD, particle motion, and the shared tick accumulator all
/// consume this one policy so their defensive clamps cannot drift apart.
pub const RETAIL_FRAME_DELTA_MAX_US: u32 = 125_000;

/// `FUN_00494070`'s minimum frame period (task `+0x20`): a frame runs only
/// once this much has passed, so retail tops out at 125 frames a second.
pub const RETAIL_MIN_FRAME_PERIOD_MICROS: u64 = 8_000;

/// `FUN_00494070`'s gate on `FUN_00495FF0`'s clock (`timeGetTime() * 1000`):
/// the whole-millisecond delta since the last frame, once it reaches the
/// minimum period. Earlier polls run no frame.
pub const fn retail_frame_delta_micros(last_ms: u64, now_ms: u64) -> Option<u64> {
    let delta_micros = now_ms.saturating_sub(last_ms) * 1_000;
    if delta_micros >= RETAIL_MIN_FRAME_PERIOD_MICROS {
        Some(delta_micros)
    } else {
        None
    }
}

/// Retail's frame pump on a whole-millisecond clock. Every delta is a whole
/// number of milliseconds and at least the 8-ms minimum period, as
/// `FUN_00494070` produces from `timeGetTime`. The port sleeps until the
/// next frame is due instead of polling.
#[derive(Debug)]
pub struct RetailFramePump {
    origin: std::time::Instant,
    last_ms: u64,
}

impl RetailFramePump {
    pub fn new() -> Self {
        Self {
            origin: std::time::Instant::now(),
            last_ms: 0,
        }
    }

    fn now_ms(&self) -> u64 {
        self.origin.elapsed().as_millis() as u64
    }

    /// Wait until the next frame is due, start it and return its delta.
    pub fn wait_for_frame(&mut self) -> u64 {
        loop {
            let now_ms = self.now_ms();
            if let Some(delta_micros) = retail_frame_delta_micros(self.last_ms, now_ms) {
                self.last_ms = now_ms;
                return delta_micros;
            }
            let due = self.origin
                + std::time::Duration::from_micros(
                    self.last_ms * 1_000 + RETAIL_MIN_FRAME_PERIOD_MICROS,
                );
            std::thread::sleep(due.saturating_duration_since(std::time::Instant::now()));
        }
    }

    /// Restart the period after a blocking load, so the load's duration
    /// never becomes a frame's delta.
    pub fn restart(&mut self) {
        self.last_ms = self.now_ms();
    }
}

impl Default for RetailFramePump {
    fn default() -> Self {
        Self::new()
    }
}

/// Integer clock matching `DAT_004FED60` and its 64-bit microsecond source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetailTickClock {
    accumulated_micros: u64,
}

impl RetailTickClock {
    /// Construct a reset clock at tick zero.
    pub const fn new() -> Self {
        Self {
            accumulated_micros: 0,
        }
    }

    /// Reset both the published tick and its carried sub-tick remainder.
    pub fn reset(&mut self) {
        self.accumulated_micros = 0;
    }

    /// Advance by one frame delta and return the resulting retail tick.
    ///
    /// Deltas above 125 ms are charged as exactly 125 ms. The accumulator is
    /// deliberately wrapping, matching the original unsigned 64-bit add; the
    /// published value is the low 32 bits of the integer quotient.
    pub fn advance(&mut self, frame_delta_micros: u64) -> u32 {
        let charged_micros = frame_delta_micros.min(u64::from(RETAIL_FRAME_DELTA_MAX_US));
        self.accumulated_micros = self.accumulated_micros.wrapping_add(charged_micros);
        self.current()
    }

    /// Return the current low-32-bit 50 Hz world tick without advancing it.
    pub const fn current(&self) -> u32 {
        (self.accumulated_micros / RETAIL_TICK_MICROS) as u32
    }

    /// Return the carried microseconds below the next 50 Hz tick boundary.
    pub const fn remainder_micros(&self) -> u64 {
        self.accumulated_micros % RETAIL_TICK_MICROS
    }
}

#[cfg(test)]
mod tests {
    use super::{
        retail_frame_delta_micros, RetailTickClock, RETAIL_FRAME_DELTA_MAX_US,
        RETAIL_MIN_FRAME_PERIOD_MICROS, RETAIL_TICK_MICROS,
    };

    #[test]
    fn the_retail_pump_runs_whole_millisecond_frames_of_at_least_8_ms() {
        // 494115..49411F: a delta below task +0x20 runs no frame.
        assert_eq!(retail_frame_delta_micros(100, 107), None);
        assert_eq!(
            retail_frame_delta_micros(100, 108),
            Some(RETAIL_MIN_FRAME_PERIOD_MICROS)
        );
        assert_eq!(retail_frame_delta_micros(100, 117), Some(17_000));
        // A clock behind the last frame never runs a negative frame.
        assert_eq!(retail_frame_delta_micros(100, 90), None);
    }

    #[test]
    fn tick_boundary_carries_19_999_plus_one_microsecond() {
        let mut clock = RetailTickClock::new();

        assert_eq!(clock.advance(19_999), 0);
        assert_eq!(clock.remainder_micros(), 19_999);
        assert_eq!(clock.advance(1), 1);
        assert_eq!(clock.remainder_micros(), 0);
    }

    #[test]
    fn sixteen_millisecond_frames_carry_their_remainder() {
        let mut clock = RetailTickClock::new();
        let ticks = (0..5).map(|_| clock.advance(16_000)).collect::<Vec<_>>();

        assert_eq!(ticks, [0, 1, 2, 3, 4]);
        assert_eq!(clock.remainder_micros(), 0);
    }

    #[test]
    fn one_500_millisecond_frame_is_clamped_to_125_milliseconds() {
        let mut clock = RetailTickClock::new();

        assert_eq!(clock.advance(500_000), 6);
        assert_eq!(clock.remainder_micros(), 5_000);
        assert_eq!(
            u64::from(RETAIL_FRAME_DELTA_MAX_US),
            6 * RETAIL_TICK_MICROS + 5_000
        );
    }

    #[test]
    fn reset_clears_ticks_and_sub_tick_remainder() {
        let mut clock = RetailTickClock::new();
        clock.advance(47_000);

        clock.reset();

        assert_eq!(clock.current(), 0);
        assert_eq!(clock.remainder_micros(), 0);
    }

    #[test]
    fn published_tick_wraps_at_the_u32_boundary() {
        let mut clock = RetailTickClock {
            accumulated_micros: u32::MAX as u64 * RETAIL_TICK_MICROS + (RETAIL_TICK_MICROS - 1),
        };

        assert_eq!(clock.current(), u32::MAX);
        assert_eq!(clock.advance(1), 0);
        assert_eq!(clock.remainder_micros(), 0);
    }
}
