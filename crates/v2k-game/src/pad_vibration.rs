//! Pad vibration: the craft's feedback strengths and the console pad's motor.
//!
//! Five Player Control callbacks hand a strength to the output bindings
//! (`FUN_00471460`): the terrain strike (`FUN_00448280`), the dying bounce
//! (`FUN_00447AF0`), static contact (`FUN_00447C90`), pair contact
//! (`FUN_00447D70`) and the damage modifier (`FUN_004484A0`). On the PC every
//! device registers without an output hook, so `FUN_00494C90` installs the
//! no-op `FUN_00438050` and nothing vibrates. The PlayStation release stores
//! the strength as the pad's level and runs the pad's small motor from an
//! 8 ms poll; [`PadVibration`] is that driver, for a pad playing a console
//! layout. See `docs/re/CONTROLS.md`, "Console vibration".

use std::cell::Cell;

use crate::terrain_contact::DEFAULT_PLAYER_TERRAIN_HAPTIC_SCALE_RAW as VIBRATION;

/// `0x800EE848`: the small motor runs on a poll whose level reaches the entry
/// for the current video frame (mod 7).
const PATTERN: [u8; 7] = [0x20, 0x60, 0xC0, 0x40, 0xA0, 0x80, 0xE0];
/// The pad poll `0x800CFCD8` runs every 8 ms.
const POLL_US: u64 = 8_000;
/// The PAL console's vertical blank, which counts the video frames.
const VIDEO_FRAME_US: u64 = 20_000;
/// `FUN_004484A0` feeds back only hits above this filtered amount.
const DAMAGE_FEEDBACK_THRESHOLD_RAW: i32 = 2000;

thread_local! {
    /// The last strength the game submitted since the driver's last frame.
    static SUBMITTED: Cell<Option<u8>> = const { Cell::new(None) };
}

/// `FUN_00471460`: hand a strength to the bound pad. The pad's output routine
/// (`0x800D090C`) stores its low byte as the pad's level, so the last
/// submission wins.
pub fn submit(strength_raw: i32) {
    SUBMITTED.set(Some(strength_raw as u8));
}

/// `FUN_004484A0`'s first step: a filtered hit on the controlled craft above
/// 2000 raw submits `min(0xFF, vibration × (damage − 2000) / 15)`, dying or not.
pub fn submit_player_damage(filtered_damage_raw: i32) {
    if let Some(strength_raw) = damage_strength(filtered_damage_raw) {
        submit(strength_raw);
    }
}

/// The strength [`submit_player_damage`] submits, if any.
pub fn damage_strength(filtered_damage_raw: i32) -> Option<i32> {
    (filtered_damage_raw > DAMAGE_FEEDBACK_THRESHOLD_RAW).then(|| {
        let scaled = VIBRATION.wrapping_mul(filtered_damage_raw - DAMAGE_FEEDBACK_THRESHOLD_RAW);
        (scaled / 15).min(0xFF)
    })
}

/// `FUN_00447D70`'s feedback when the craft touches a body that is not a
/// pickup: `min(0xFF, vibration × (speed − 0x200) / 45)`, from the retail
/// integer root of the squared difference of the two velocities (`+9C..A0`).
pub fn pair_contact_strength(craft_raw: [i16; 3], body_raw: [i16; 3]) -> Option<i32> {
    let squared = (0..3).fold(0_i32, |sum, axis| {
        let delta = i32::from(craft_raw[axis].wrapping_sub(body_raw[axis]));
        sum.wrapping_add(delta.wrapping_mul(delta))
    });
    // The root's low word is compared and scaled as a signed short.
    let distance = v2k_formats::fixed_math::retail_integer_sqrt(squared) as u16 as i16;
    (distance > 0x200).then(|| (VIBRATION * i32::from(distance - 0x200) / 45).min(0xFF))
}

/// The console pad driver: the level the output routine stores and the poll
/// that runs the small motor from it, then fades it by 15/16.
#[derive(Debug, Default)]
pub struct PadVibration {
    level: u8,
    clock_us: u64,
    motor: bool,
}

impl PadVibration {
    /// Take the submission made since the last call, then run the polls that
    /// fell in the `elapsed_us` since it. Returns whether the small motor runs.
    pub fn advance(&mut self, elapsed_us: u64) -> bool {
        if let Some(level) = SUBMITTED.take() {
            self.level = level;
        }
        let end = self.clock_us + elapsed_us;
        let mut poll = (self.clock_us / POLL_US + 1) * POLL_US;
        while poll <= end {
            if self.level == 0 {
                self.motor = false;
                break;
            }
            self.motor = self.level >= PATTERN[(poll / VIDEO_FRAME_US % 7) as usize];
            self.level = (u32::from(self.level) * 15 >> 4) as u8;
            poll += POLL_US;
        }
        self.clock_us = end;
        self.motor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each poll's motor state over `polls` 8 ms frames.
    fn polls(vibration: &mut PadVibration, polls: usize) -> Vec<bool> {
        (0..polls).map(|_| vibration.advance(POLL_US)).collect()
    }

    #[test]
    fn a_full_strength_hit_thins_out_as_it_fades() {
        let mut vibration = PadVibration::default();
        submit(0xFF);
        let motor = polls(&mut vibration, 40);
        // Levels 255, 239, 224, 210, 196, 183, 171, 160, 150, 140, ... meet
        // the pattern of video frames 0, 0, 1, 1, 2, 2, 2, 3, 3, 4, ...: on
        // for polls 1-5, 8-9 and 18-19. 33 on poll 30 is the last level at or
        // above 0x20.
        let on: Vec<usize> = (0..40)
            .filter(|&poll| motor[poll])
            .map(|poll| poll + 1)
            .collect();
        assert_eq!(on, [1, 2, 3, 4, 5, 8, 9, 18, 19]);
        assert_eq!(vibration.level, 13);
    }

    #[test]
    fn the_pattern_entry_follows_the_video_frame() {
        // Video frame 1 (20..40 ms) needs 0x60; frame 2 (40..60 ms) 0xC0.
        let mut vibration = PadVibration {
            clock_us: 16_000,
            ..PadVibration::default()
        };
        submit(0x66);
        assert!(
            vibration.advance(8_000),
            "poll at 24 ms, frame 1: 0x66 >= 0x60"
        );
        // 0x66 fades to 0x5F, under frame 1's 0x60 at 32 ms.
        assert!(!vibration.advance(8_000));
        submit(0xBF);
        assert!(
            !vibration.advance(8_000),
            "poll at 40 ms, frame 2: 0xBF < 0xC0"
        );
    }

    #[test]
    fn the_last_submission_replaces_the_level() {
        let mut vibration = PadVibration::default();
        submit(0xFF);
        submit(0x10);
        assert!(!vibration.advance(POLL_US));
        assert_eq!(vibration.level, 0x0F);
    }

    #[test]
    fn a_long_frame_runs_every_poll_it_spans() {
        let mut stepped = PadVibration::default();
        let mut whole = PadVibration::default();
        submit(0xC8);
        let last = *polls(&mut stepped, 7).last().unwrap();
        submit(0xC8);
        assert_eq!(whole.advance(7 * POLL_US), last);
        assert_eq!(whole.level, stepped.level);
        assert_eq!(whole.advance(10_000_000_000), false);
    }

    #[test]
    fn damage_feeds_back_above_2000_raw() {
        let mut vibration = PadVibration::default();
        submit_player_damage(2000);
        vibration.advance(0);
        assert_eq!(vibration.level, 0);
        submit_player_damage(2001);
        vibration.advance(0);
        assert_eq!(vibration.level, 1);
        submit_player_damage(30_000);
        vibration.advance(0);
        assert_eq!(vibration.level, 0xFF);
    }

    #[test]
    fn pair_contact_scales_the_root_beyond_0x200_by_a_third() {
        let at = |x: i16, z: i16| pair_contact_strength([0; 3], [x, 0, z]);
        assert_eq!(at(0x200, 0), None);
        // A root of 0x200 still fails the strict test, short of 513 squared.
        assert_eq!(at(512, 31), None);
        assert_eq!(at(515, 0), Some(1));
        assert_eq!(at(0, -0x400), Some(0xAA));
        assert_eq!(at(2000, 0), Some(0xFF));
        // Word deltas wrap across the world's seam.
        assert_eq!(
            pair_contact_strength([i16::MAX, 0, 0], [i16::MIN + 514, 0, 0]),
            Some(1)
        );
        // Squares summing to 0x4000_0000 or more root to 0xFFFF, a negative
        // short; a sum that wraps negative roots to zero.
        assert_eq!(at(0x6000, 0x6000), None);
        assert_eq!(at(-0x8000, -0x8000), None);
    }
}
