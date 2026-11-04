//! Each input value record's on-time within a frame (`FUN_004715F0`,
//! `FUN_00471400`).
//!
//! The frame pump's drain restamps every record that is on with the previous
//! drain's time and zeroes its on-time, then applies the queued key events:
//! a record turning off adds the time since its stamp, one turning on is
//! stamped with the event's time. The reader then adds the time from the
//! stamp to this drain for a record still on. A record held through the
//! frame therefore reads the whole frame, one pressed or released within it
//! the part it was on. Event times are `timeGetTime` milliseconds in retail
//! and SDL's millisecond event timestamps here.

use std::collections::HashSet;

use sdl2::keyboard::Keycode;

/// The key events of one frame, in order, with where in the frame each one
/// happened.
#[derive(Debug, Clone, Default)]
pub struct FrameKeyEvents {
    /// The frame's duration in microseconds (the reader's `dt`).
    pub dt_us: u32,
    /// The keys down at the frame's end.
    pub keys_after: HashSet<Keycode>,
    /// `(key, pressed, microseconds into the frame)`.
    pub events: Vec<(Keycode, bool, u32)>,
}

impl FrameKeyEvents {
    /// A frame with no key events: every key down was held throughout.
    pub fn held_throughout(keys: &HashSet<Keycode>, dt_us: u32) -> Self {
        Self {
            dt_us,
            keys_after: keys.clone(),
            events: Vec::new(),
        }
    }

    /// Where an event at SDL time `event_ms` falls in the frame polled at
    /// `poll_ms` after one polled at `previous_poll_ms`. Scripted events
    /// carry no time and count from the frame's start, as a key down at
    /// the frame's end was held for all of it before.
    pub fn offset_us(
        event_ms: Option<u32>,
        previous_poll_ms: u32,
        poll_ms: u32,
        dt_us: u32,
    ) -> u32 {
        match event_ms {
            None => 0,
            Some(ms) if ms >= poll_ms => dt_us,
            Some(ms) => {
                (u64::from(ms.saturating_sub(previous_poll_ms)) * 1000).min(u64::from(dt_us)) as u32
            }
        }
    }

    /// The microseconds within the frame that `record` was on, where
    /// `record` says whether the binding matches a set of keys down. Keys
    /// held through the frame read `dt`.
    pub fn on_time_us(&self, record: impl Fn(&HashSet<Keycode>) -> bool) -> u32 {
        // The keys down at the frame's start: undo this frame's events.
        let mut keys = self.keys_after.clone();
        for &(key, pressed, _) in self.events.iter().rev() {
            if pressed {
                keys.remove(&key);
            } else {
                keys.insert(key);
            }
        }
        let mut on = record(&keys);
        let mut stamp = 0u32;
        let mut held = 0u32;
        for &(key, pressed, at) in &self.events {
            if pressed {
                keys.insert(key);
            } else {
                keys.remove(&key);
            }
            let now_on = record(&keys);
            if on && !now_on {
                held += at.saturating_sub(stamp);
            } else if !on && now_on {
                stamp = at;
            }
            on = now_on;
        }
        if on {
            held += self.dt_us.saturating_sub(stamp);
        }
        held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(keys: &HashSet<Keycode>) -> bool {
        keys.contains(&Keycode::Up)
    }

    #[test]
    fn a_key_held_through_reads_the_frame_and_one_pressed_within_it_the_rest() {
        let held = FrameKeyEvents::held_throughout(&HashSet::from([Keycode::Up]), 20_000);
        assert_eq!(held.on_time_us(up), 20_000);
        let pressed = FrameKeyEvents {
            dt_us: 20_000,
            keys_after: HashSet::from([Keycode::Up]),
            events: vec![(Keycode::Up, true, 5_000)],
        };
        assert_eq!(pressed.on_time_us(up), 15_000);
        let released = FrameKeyEvents {
            dt_us: 20_000,
            keys_after: HashSet::new(),
            events: vec![(Keycode::Up, false, 5_000)],
        };
        assert_eq!(released.on_time_us(up), 5_000);
        // A tap inside the frame is on for its length.
        let tapped = FrameKeyEvents {
            dt_us: 20_000,
            keys_after: HashSet::new(),
            events: vec![(Keycode::Up, true, 4_000), (Keycode::Up, false, 7_000)],
        };
        assert_eq!(tapped.on_time_us(up), 3_000);
    }

    #[test]
    fn a_record_follows_its_exclusions() {
        // A fires only while / is up: holding / over A switches it off.
        let fire_a =
            |keys: &HashSet<Keycode>| keys.contains(&Keycode::A) && !keys.contains(&Keycode::Slash);
        let frame = FrameKeyEvents {
            dt_us: 20_000,
            keys_after: HashSet::from([Keycode::A, Keycode::Slash]),
            events: vec![(Keycode::Slash, true, 8_000)],
        };
        assert_eq!(frame.on_time_us(fire_a), 8_000);
    }

    #[test]
    fn event_times_map_into_the_frame() {
        assert_eq!(FrameKeyEvents::offset_us(None, 100, 120, 20_000), 0);
        assert_eq!(
            FrameKeyEvents::offset_us(Some(105), 100, 120, 20_000),
            5_000
        );
        assert_eq!(FrameKeyEvents::offset_us(Some(99), 100, 120, 20_000), 0);
        assert_eq!(
            FrameKeyEvents::offset_us(Some(120), 100, 120, 20_000),
            20_000
        );
        // A frame shorter than its milliseconds clamps to its duration.
        assert_eq!(
            FrameKeyEvents::offset_us(Some(119), 100, 120, 16_667),
            16_667
        );
    }
}
