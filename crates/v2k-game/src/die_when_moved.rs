//! Class36 Die When Moved: the Primary `FUN_0040B980 -> FUN_00404A20`
//! publishes on Type50.
//!
//! The task has no timeout. `FUN_00401350/FUN_004012E0` copies the body's
//! position into its private record at publication. Each detailed visit of
//! `FUN_00404AA0` (`FUN_0040DCA0` passes 0; the coarse `FUN_0040E870` passes 1
//! and skips it) compares the current position with that record, stores the
//! current one, and calls generic death `FUN_00410C10` once the squared
//! distance exceeds 0x100: a move of more than 16 units.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DieWhenMovedTaskState {
    elapsed_ms: u32,
    stored_position_raw: [i16; 3],
}

impl DieWhenMovedTaskState {
    pub const fn new(position_raw: [i16; 3]) -> Self {
        Self {
            elapsed_ms: 0,
            stored_position_raw: position_raw,
        }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub const fn stored_position_raw(self) -> [i16; 3] {
        self.stored_position_raw
    }

    /// `FUN_00401120`'s age prefix; a zero lifetime never times out.
    pub fn advance_prefix(&mut self, elapsed_micros: u32) {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1000);
    }

    /// One detailed `FUN_00404AA0` visit: store the position and report
    /// whether the body moved past 16 units. Retail sums the squares in
    /// wrapping 32-bit arithmetic.
    pub fn record_detailed_visit(&mut self, position_raw: [i16; 3]) -> bool {
        let delta =
            |axis: usize| i32::from(self.stored_position_raw[axis]) - i32::from(position_raw[axis]);
        let (dx, dy, dz) = (delta(0), delta(1), delta(2));
        self.stored_position_raw = position_raw;
        dy.wrapping_mul(dy)
            .wrapping_add(dz.wrapping_mul(dz))
            .wrapping_add(dx.wrapping_mul(dx))
            > 0x100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_past_sixteen_units_kills_and_every_visit_stores() {
        let mut task = DieWhenMovedTaskState::new([100, 200, 300]);
        // Exactly 16 units is not a move: 0x100 is not over 0x100.
        assert!(!task.record_detailed_visit([116, 200, 300]));
        assert_eq!(task.stored_position_raw(), [116, 200, 300]);
        // Each visit compares with the previous visit, not the birth.
        assert!(!task.record_detailed_visit([126, 210, 300]));
        assert!(task.record_detailed_visit([126, 210, 317]));
        // A jump across the whole signed range overflows the 32-bit sum
        // negative, which retail does not count as a move.
        let mut task = DieWhenMovedTaskState::new([i16::MAX, 0, 0]);
        assert!(!task.record_detailed_visit([i16::MIN, 0, 0]));
        task.advance_prefix(20_999);
        assert_eq!(task.elapsed_ms(), 20);
    }
}
