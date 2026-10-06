//! Shared Main Base / Working Factory destruction progression.
//!
//! `FUN_00419750` turns the first lethal transition into a staged callback
//! instead of allowing the generic entity death path to remove the structure.
//! `FUN_00419B50` then advances one authored model-effect stage per 100 ms and
//! re-enters ordinary death at stage 32.  This module owns only that pure clock;
//! entity health/state and presentation remain transactional at the caller.

pub const PROGRESSION_REVIVE_HEALTH_RAW: i32 = 10_000_000;
pub const PROGRESSION_STAGE_MICROS: i32 = 100_000;
pub const PROGRESSION_FINAL_STAGE: i32 = 32;

/// The subset of the shared 0xB8 component required by the recovered death
/// callbacks. Other factory production fields remain on the entity runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressiveDeathState {
    /// Runtime component `+0x94`. Zero is idle, positive is the elapsed death
    /// clock, and `-1` selects the callback's terminal re-entry branch.
    pub elapsed_micros_raw: i32,
    /// Low byte of runtime component `+0x18`. Bit `0x20` skips directly to the
    /// final stage; it comes from the exact behavior-instance template.
    pub config_flags_at_0x18: u8,
}

impl ProgressiveDeathState {
    pub const fn idle(config_flags_at_0x18: u8) -> Self {
        Self {
            elapsed_micros_raw: 0,
            config_flags_at_0x18,
        }
    }

    /// Apply the non-terminal branch of `FUN_00419750`.
    pub const fn begin(self) -> ProgressiveDeathBegin {
        if self.elapsed_micros_raw == -1 {
            return ProgressiveDeathBegin::TerminalReentryRequired;
        }
        let elapsed_micros_raw = if self.elapsed_micros_raw == 0 {
            1
        } else {
            self.elapsed_micros_raw
        };
        ProgressiveDeathBegin::Revived {
            state: Self {
                elapsed_micros_raw,
                ..self
            },
        }
    }

    /// Advance `FUN_00419B50` with C's signed wrapping and truncating division.
    pub fn advance(self, elapsed_micros: u32) -> ProgressiveDeathAdvance {
        if self.elapsed_micros_raw <= 0 {
            return ProgressiveDeathAdvance {
                state: self,
                effects: Vec::new(),
                reached_terminal_death: false,
            };
        }

        let old_stage = self.elapsed_micros_raw / PROGRESSION_STAGE_MICROS;
        let elapsed_micros_raw = self.elapsed_micros_raw.wrapping_add(elapsed_micros as i32);
        let new_stage = elapsed_micros_raw / PROGRESSION_STAGE_MICROS;
        let mut stage = old_stage;
        let mut effects = Vec::new();
        let mut reached_terminal_death = false;

        while stage < new_stage {
            stage += 1;
            if stage == 1 {
                effects.push(ProgressiveModelEffect { threshold: 0x19 });
                continue;
            }
            if self.config_flags_at_0x18 & 0x20 != 0 {
                stage = PROGRESSION_FINAL_STAGE;
            }
            if stage < 31 {
                effects.push(ProgressiveModelEffect {
                    threshold: stage / 2,
                });
            } else if stage != PROGRESSION_FINAL_STAGE {
                effects.push(ProgressiveModelEffect {
                    threshold: 0x1_0000,
                });
            } else {
                reached_terminal_death = true;
                break;
            }
        }

        ProgressiveDeathAdvance {
            state: Self {
                elapsed_micros_raw: if reached_terminal_death {
                    -1
                } else {
                    elapsed_micros_raw
                },
                ..self
            },
            effects,
            reached_terminal_death,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressiveDeathBegin {
    Revived { state: ProgressiveDeathState },
    TerminalReentryRequired,
}

/// One probability threshold passed to `FUN_00419C90`'s authored model walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressiveModelEffect {
    pub threshold: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressiveDeathAdvance {
    pub state: ProgressiveDeathState,
    pub effects: Vec<ProgressiveModelEffect>,
    pub reached_terminal_death: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_death_starts_at_one_but_positive_progress_is_preserved() {
        assert_eq!(
            ProgressiveDeathState::idle(0).begin(),
            ProgressiveDeathBegin::Revived {
                state: ProgressiveDeathState {
                    elapsed_micros_raw: 1,
                    config_flags_at_0x18: 0,
                }
            }
        );
        let in_progress = ProgressiveDeathState {
            elapsed_micros_raw: 123_456,
            config_flags_at_0x18: 0,
        };
        assert_eq!(
            in_progress.begin(),
            ProgressiveDeathBegin::Revived { state: in_progress }
        );
        assert_eq!(
            ProgressiveDeathState {
                elapsed_micros_raw: -1,
                config_flags_at_0x18: 0,
            }
            .begin(),
            ProgressiveDeathBegin::TerminalReentryRequired
        );
    }

    #[test]
    fn retail_stage_thresholds_and_terminal_time_are_exact() {
        let state = ProgressiveDeathState::idle(0).begin();
        let ProgressiveDeathBegin::Revived { mut state } = state else {
            unreachable!()
        };
        let first = state.advance(100_000);
        assert_eq!(first.effects, [ProgressiveModelEffect { threshold: 0x19 }]);
        assert!(!first.reached_terminal_death);
        state = first.state;

        let through_stage_31 = state.advance(3_000_000);
        assert_eq!(through_stage_31.effects.last().unwrap().threshold, 0x1_0000);
        assert!(!through_stage_31.reached_terminal_death);

        let terminal = through_stage_31.state.advance(100_000);
        assert!(terminal.effects.is_empty());
        assert!(terminal.reached_terminal_death);
        assert_eq!(terminal.state.elapsed_micros_raw, -1);
    }

    #[test]
    fn flag_20_shortcuts_to_terminal_on_the_next_crossed_stage() {
        let state = ProgressiveDeathState {
            elapsed_micros_raw: 100_001,
            config_flags_at_0x18: 0x20,
        };
        let tick = state.advance(100_000);
        assert!(tick.effects.is_empty());
        assert!(tick.reached_terminal_death);
        assert_eq!(tick.state.elapsed_micros_raw, -1);
    }

    #[test]
    fn idle_and_terminal_states_do_not_advance() {
        for elapsed_micros_raw in [0, -1] {
            let state = ProgressiveDeathState {
                elapsed_micros_raw,
                config_flags_at_0x18: 0,
            };
            let tick = state.advance(u32::MAX);
            assert_eq!(tick.state, state);
            assert!(tick.effects.is_empty());
            assert!(!tick.reached_terminal_death);
        }
    }
}
