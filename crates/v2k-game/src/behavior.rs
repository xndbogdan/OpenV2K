//! Recovered world/entity behavior primitives.
//!
//! These are kept integer-exact where the original mutates authored resource
//! state. Higher-level behavior dispatch remains separate so a recovered
//! primitive cannot accidentally become a level-specific scripted guess.

use v2k_formats::terrain::TerrainGrid;

/// Remaining raw Section-10 sea-level delta for `FUN_004013A0` (the named
/// `Change Sea Level` behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeaLevelTransition {
    remaining: i32,
}

impl SeaLevelTransition {
    pub fn new(raw_delta: i32) -> Self {
        Self {
            remaining: raw_delta,
        }
    }

    pub fn remaining(&self) -> i32 {
        self.remaining
    }

    pub fn is_complete(&self) -> bool {
        self.remaining == 0
    }

    /// Advance using the engine's integer timestep domain.
    ///
    /// `FUN_004013A0` computes `step = ((dt << 10) * 0x6400) >> 31`.
    /// Falling water consumes a full step; rising water consumes half a step.
    /// If the absolute remainder is strictly smaller than a full step, the
    /// entire remainder is applied and the behavior completes.
    pub fn tick(&mut self, terrain: &mut TerrainGrid, delta_time: i32) -> bool {
        if self.remaining == 0 {
            return true;
        }

        let shifted_dt = delta_time.wrapping_shl(10);
        let step = ((shifted_dt as i64 * 0x6400_i64) >> 31) as i32;
        if step <= 0 {
            return false;
        }

        let sign = self.remaining >> 31;
        let wrapping_abs = (self.remaining ^ sign).wrapping_sub(sign);
        let applied = if wrapping_abs < step {
            self.remaining
        } else if self.remaining < 0 {
            -step
        } else {
            step / 2
        };

        terrain.header[0] = terrain.header[0].wrapping_add(applied);
        self.remaining = self.remaining.wrapping_sub(applied);
        self.is_complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain_with_sea(raw: i32) -> TerrainGrid {
        TerrainGrid {
            header: [raw, 0, 0, 0, 0],
            cells: Vec::new(),
        }
    }

    #[test]
    fn rising_water_uses_half_step() {
        let mut terrain = terrain_with_sea(10_000);
        let mut transition = SeaLevelTransition::new(1_000);

        // dt=0x10000 gives a full step of 800, hence a +400 rise.
        assert!(!transition.tick(&mut terrain, 0x10000));
        assert_eq!(terrain.header[0], 10_400);
        assert_eq!(transition.remaining(), 600);
    }

    #[test]
    fn falling_water_uses_full_step() {
        let mut terrain = terrain_with_sea(10_000);
        let mut transition = SeaLevelTransition::new(-1_000);

        assert!(!transition.tick(&mut terrain, 0x10000));
        assert_eq!(terrain.header[0], 9_200);
        assert_eq!(transition.remaining(), -200);
    }

    #[test]
    fn sub_step_remainder_is_applied_exactly() {
        let mut terrain = terrain_with_sea(10_000);
        let mut transition = SeaLevelTransition::new(123);

        assert!(transition.tick(&mut terrain, 0x10000));
        assert_eq!(terrain.header[0], 10_123);
        assert_eq!(transition.remaining(), 0);
    }

    #[test]
    fn zero_timestep_does_not_consume_the_behavior() {
        let mut terrain = terrain_with_sea(10_000);
        let mut transition = SeaLevelTransition::new(123);

        assert!(!transition.tick(&mut terrain, 0));
        assert_eq!(terrain.header[0], 10_000);
        assert_eq!(transition.remaining(), 123);
    }

    #[test]
    fn minimum_remainder_uses_retail_wrapping_abs_terminal_branch() {
        let mut terrain = terrain_with_sea(10_000);
        let mut transition = SeaLevelTransition::new(i32::MIN);

        assert!(transition.tick(&mut terrain, 0x10000));
        assert_eq!(terrain.header[0], 10_000_i32.wrapping_add(i32::MIN));
        assert_eq!(transition.remaining(), 0);
    }
}
