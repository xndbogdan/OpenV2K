//! Shared timer data for class 0's `C490 -> 02800 -> 05F80` Primary task.
//!
//! The task has zero private bytes and a null callback. Its wrapper retains
//! age and performs a strict timeout test; the actor's specialized owner must
//! still supply admission, wrapper unwind and behavior reselection. This is
//! separate from `03230`, whose callback advances a linked Sub-I controller.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Class0TimerTaskState {
    elapsed_ms: u32,
}

impl Class0TimerTaskState {
    pub const fn new() -> Self {
        Self { elapsed_ms: 0 }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }

    pub fn advance_prefix(&mut self, elapsed_micros: u32) -> bool {
        self.elapsed_ms = self.elapsed_ms.wrapping_add(elapsed_micros / 1000);
        self.elapsed_ms > 9000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_uses_callback_milliseconds_strictly() {
        let mut task = Class0TimerTaskState::new();
        assert!(!task.advance_prefix(9_000_999));
        assert_eq!(task.elapsed_ms(), 9000);
        assert!(!task.advance_prefix(999));
        assert!(task.advance_prefix(1000));
        task.elapsed_ms = u32::MAX;
        assert!(!task.advance_prefix(1000));
        assert_eq!(task.elapsed_ms(), 0);
    }
}
