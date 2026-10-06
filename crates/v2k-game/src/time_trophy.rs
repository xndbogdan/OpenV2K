//! Per-world time-trophy controller recovered from `FUN_0042D9B0` and
//! `FUN_0042E570` (demo `FUN_0042D400` / `FUN_0042DFC0`).
//!
//! This is the authoritative owner of player-controller state `+0x1F0` and
//! signed remaining milliseconds `+0x1EC`. Ordinary hive/world-complete now
//! calls the campaign-completion transaction and retains its state 3 or 4.
//! Hive death does not itself enter the later progress map. Main Base abort
//! enters state 5 without marking the world saved.

use crate::power_up_contact::{
    CampaignControlSlotError, PlayerCampaignProgress, TrophyAcquisition,
    RETAIL_CONTROL_HIDDEN_TROPHY_BIT, RETAIL_CONTROL_TIME_TROPHY_BIT,
    RETAIL_CONTROL_WORLD_SAVED_BIT,
};

pub const TIME_TROPHY_WARNING_SOUND_ID: u16 = 51;
pub const TIME_TROPHY_EXPIRED_SOUND_ID: u16 = 2;
pub const TIME_TROPHY_NORMAL_RATE_Q16: u32 = 0x1_0000;
pub const TIME_TROPHY_URGENT_RATE_Q16: u32 = 0x1_8000;
pub const TIME_TROPHY_HIGH_RATE_Q16: u32 = 0x1_4000;
pub const TIME_TROPHY_MEDIUM_RATE_Q16: u32 = 0x1_2000;

/// Exact values stored at player-controller `+0x1F0`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum TimeTrophyState {
    #[default]
    Dormant = 0,
    Active = 1,
    Expired = 2,
    WorldSavedOutsideActiveWindow = 3,
    Secured = 4,
    ResultsActive = 5,
}

/// One direct, centered global-sound request emitted by the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTrophySound {
    pub sound_id: u16,
    pub rate_q16: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTrophyLoadOutcome {
    pub state: TimeTrophyState,
    /// True only when a zero-deadline initialization newly set campaign bit
    /// `0x08`.
    pub auto_claimed_now: bool,
    /// Exact operation-0x13F reward, present only on a new claim. The caller
    /// submits selector-0x3F's sound and optional extra-life notification.
    pub reward: Option<TrophyAcquisition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTrophyWorldCompletionOutcome {
    pub state: TimeTrophyState,
    pub world_saved_now: bool,
    pub time_trophy_claimed_now: bool,
    pub reward: Option<TrophyAcquisition>,
}

/// Exact per-load time-trophy state retained by the world controller.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimeTrophyRuntime {
    state: TimeTrophyState,
    remaining_millis_raw: i32,
    /// User-requested pickup policy, distinct from retail's +1F0/bit8 award.
    /// Static 445A90/56820(0)/42ED20 only sets hidden bit2; it does not stop
    /// the clock. Keep this explicit until a matched pickup observation
    /// establishes any additional native controller transition.
    hidden_pickup_stops_countdown: bool,
}

impl TimeTrophyRuntime {
    /// Rebuild the controller for one freshly loaded world.
    ///
    /// The campaign-state precedence and `deadline * 1000 + 500` bias are the
    /// instruction-identical retail/demo `FUN_0042E570` contract.
    pub fn from_loaded_world(
        deadline_seconds: u32,
        results_active: bool,
        progress: &mut PlayerCampaignProgress,
    ) -> Result<(Self, TimeTrophyLoadOutcome), CampaignControlSlotError> {
        let control_state = progress.current_control_state_bits()?;
        let world_saved = control_state & RETAIL_CONTROL_WORLD_SAVED_BIT != 0;
        let time_trophy_claimed = control_state & RETAIL_CONTROL_TIME_TROPHY_BIT != 0;

        let (runtime, auto_claimed_now) = if world_saved {
            let state = if time_trophy_claimed {
                TimeTrophyState::Secured
            } else {
                TimeTrophyState::WorldSavedOutsideActiveWindow
            };
            (
                Self {
                    state,
                    remaining_millis_raw: 0,
                    hidden_pickup_stops_countdown: false,
                },
                false,
            )
        } else if results_active {
            (
                Self {
                    state: TimeTrophyState::ResultsActive,
                    remaining_millis_raw: 0,
                    hidden_pickup_stops_countdown: false,
                },
                false,
            )
        } else if deadline_seconds == 0 {
            let auto_claimed_now = progress.claim_current_time_trophy()?;
            (
                Self {
                    state: TimeTrophyState::Secured,
                    remaining_millis_raw: 0,
                    hidden_pickup_stops_countdown: false,
                },
                auto_claimed_now,
            )
        } else {
            (
                Self {
                    state: TimeTrophyState::Active,
                    remaining_millis_raw: deadline_seconds.wrapping_mul(1_000).wrapping_add(500)
                        as i32,
                    hidden_pickup_stops_countdown: control_state & RETAIL_CONTROL_HIDDEN_TROPHY_BIT
                        != 0,
                },
                false,
            )
        };

        Ok((
            runtime,
            TimeTrophyLoadOutcome {
                state: runtime.state,
                auto_claimed_now,
                reward: auto_claimed_now.then(|| progress.acquire_time_trophy_award()),
            },
        ))
    }

    pub const fn state(self) -> TimeTrophyState {
        self.state
    }

    pub const fn remaining_millis_raw(self) -> i32 {
        self.remaining_millis_raw
    }

    /// The retail active-state gate plus the explicitly requested hidden-
    /// pickup clock policy. This owns ticking and HUD visibility together.
    pub const fn countdown_active(self) -> bool {
        matches!(self.state, TimeTrophyState::Active) && !self.hidden_pickup_stops_countdown
    }

    /// Stop and hide the clock after a successful amount-zero trophy pickup.
    /// Do not manufacture the independent timed award or replay acquisition.
    pub fn stop_countdown_for_hidden_pickup(&mut self) {
        self.hidden_pickup_stops_countdown = true;
    }

    /// Advance one active-gameplay frame and return at most one sound request.
    ///
    /// Retail discards each frame's sub-millisecond remainder and tests only
    /// the final whole-second quotient. A long frame therefore does not catch
    /// up warning points it skipped.
    pub fn advance(&mut self, elapsed_micros: u32) -> Option<TimeTrophySound> {
        if !self.countdown_active() {
            return None;
        }

        let elapsed_millis = (elapsed_micros / 1_000) as i32;
        if self.remaining_millis_raw <= elapsed_millis {
            self.remaining_millis_raw = 0;
            self.state = TimeTrophyState::Expired;
            return Some(TimeTrophySound {
                sound_id: TIME_TROPHY_EXPIRED_SOUND_ID,
                rate_q16: TIME_TROPHY_NORMAL_RATE_Q16,
            });
        }

        let old_seconds = self.remaining_millis_raw / 1_000;
        self.remaining_millis_raw -= elapsed_millis;
        let new_seconds = self.remaining_millis_raw / 1_000;
        if old_seconds == new_seconds {
            return None;
        }

        let (interval_seconds, rate_q16) = if new_seconds < 10 {
            (1, TIME_TROPHY_URGENT_RATE_Q16)
        } else if new_seconds < 20 {
            (2, TIME_TROPHY_HIGH_RATE_Q16)
        } else if new_seconds < 60 {
            (10, TIME_TROPHY_MEDIUM_RATE_Q16)
        } else {
            (30, TIME_TROPHY_NORMAL_RATE_Q16)
        };
        (new_seconds % interval_seconds == 0).then_some(TimeTrophySound {
            sound_id: TIME_TROPHY_WARNING_SOUND_ID,
            rate_q16,
        })
    }

    /// Apply retail's campaign "save world" transaction.
    ///
    /// This is campaign completion bit `0x01`, not save-to-disk. Only an
    /// active timer secures bit `0x08`; every other prior state becomes `3`.
    pub fn complete_current_world(
        &mut self,
        progress: &mut PlayerCampaignProgress,
    ) -> Result<TimeTrophyWorldCompletionOutcome, CampaignControlSlotError> {
        let world_saved_now = progress.mark_current_world_saved()?;
        let time_trophy_claimed_now = if self.state == TimeTrophyState::Active {
            self.state = TimeTrophyState::Secured;
            progress.claim_current_time_trophy()?
        } else {
            self.state = TimeTrophyState::WorldSavedOutsideActiveWindow;
            false
        };

        Ok(TimeTrophyWorldCompletionOutcome {
            state: self.state,
            world_saved_now,
            time_trophy_claimed_now,
            reward: time_trophy_claimed_now.then(|| progress.acquire_time_trophy_award()),
        })
    }

    /// Commit the existing Main Base/results transition at controller
    /// `+0x1F0`. The remaining timer is deliberately retained.
    pub(crate) fn enter_results_active(&mut self) {
        self.state = TimeTrophyState::ResultsActive;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress_for(slot: usize) -> PlayerCampaignProgress {
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(slot));
        progress
    }

    fn active_with_remaining(remaining_millis_raw: i32) -> TimeTrophyRuntime {
        TimeTrophyRuntime {
            state: TimeTrophyState::Active,
            remaining_millis_raw,
            hidden_pickup_stops_countdown: false,
        }
    }

    #[test]
    fn active_load_uses_deadline_bias_without_claiming_any_campaign_bit() {
        let mut progress = progress_for(1);
        let (runtime, outcome) =
            TimeTrophyRuntime::from_loaded_world(270, false, &mut progress).unwrap();

        assert_eq!(runtime.state(), TimeTrophyState::Active);
        assert_eq!(runtime.remaining_millis_raw(), 270_500);
        assert_eq!(outcome.state, TimeTrophyState::Active);
        assert!(!outcome.auto_claimed_now);
        assert_eq!(progress.control_slot_world_saved(1), Some(false));
        assert_eq!(progress.control_slot_claimed(1), Some(false));
        assert_eq!(progress.control_slot_time_trophy_claimed(1), Some(false));
    }

    #[test]
    fn requested_pickup_policy_freezes_without_replaying_timed_award_and_restores_from_hidden_bit()
    {
        let mut progress = progress_for(1);
        let (mut runtime, _) =
            TimeTrophyRuntime::from_loaded_world(270, false, &mut progress).unwrap();
        runtime.advance(1_000_000);
        runtime.stop_countdown_for_hidden_pickup();
        assert!(!runtime.countdown_active());
        assert_eq!(runtime.state(), TimeTrophyState::Active);
        assert_eq!(runtime.advance(300_000_000), None);
        assert_eq!(runtime.remaining_millis_raw(), 269_500);
        assert_eq!(progress.control_slot_time_trophy_claimed(1), Some(false));
        assert_eq!(progress.trophy_count(), 0);

        let mut bits = [0; crate::power_up_contact::RETAIL_CONTROL_SLOT_COUNT];
        bits[1] = RETAIL_CONTROL_HIDDEN_TROPHY_BIT;
        let mut restored = PlayerCampaignProgress::from_native_snapshot(1, bits, 0, 1);
        let (mut runtime, outcome) =
            TimeTrophyRuntime::from_loaded_world(270, false, &mut restored).unwrap();
        assert!(!runtime.countdown_active());
        assert_eq!(runtime.advance(1_000_000), None);
        assert!(outcome.reward.is_none());
        assert_eq!(restored.control_slot_time_trophy_claimed(1), Some(false));
        assert_eq!(restored.trophy_count(), 1);
    }

    #[test]
    fn saved_world_precedes_results_and_selects_state_from_independent_bit_eight() {
        let mut missed = progress_for(2);
        missed.mark_current_world_saved().unwrap();
        let (runtime, _) = TimeTrophyRuntime::from_loaded_world(600, true, &mut missed).unwrap();
        assert_eq!(
            runtime.state(),
            TimeTrophyState::WorldSavedOutsideActiveWindow
        );

        let mut secured = progress_for(2);
        secured.mark_current_world_saved().unwrap();
        secured.claim_current_time_trophy().unwrap();
        let (runtime, _) = TimeTrophyRuntime::from_loaded_world(600, true, &mut secured).unwrap();
        assert_eq!(runtime.state(), TimeTrophyState::Secured);
    }

    #[test]
    fn results_precedes_deadline_and_zero_deadline_claims_only_bit_eight() {
        let mut results = progress_for(3);
        let (runtime, _) = TimeTrophyRuntime::from_loaded_world(600, true, &mut results).unwrap();
        assert_eq!(runtime.state(), TimeTrophyState::ResultsActive);
        assert_eq!(results.control_slot_time_trophy_claimed(3), Some(false));

        let mut no_deadline = progress_for(3);
        let (runtime, outcome) =
            TimeTrophyRuntime::from_loaded_world(0, false, &mut no_deadline).unwrap();
        assert_eq!(runtime.state(), TimeTrophyState::Secured);
        assert!(outcome.auto_claimed_now);
        assert_eq!(no_deadline.control_slot_world_saved(3), Some(false));
        assert_eq!(no_deadline.control_slot_claimed(3), Some(false));
        assert_eq!(no_deadline.control_slot_time_trophy_claimed(3), Some(true));
    }

    #[test]
    fn sub_millisecond_input_is_discarded_per_frame() {
        let mut runtime = active_with_remaining(10_500);
        assert_eq!(runtime.advance(999), None);
        assert_eq!(runtime.remaining_millis_raw(), 10_500);
    }

    #[test]
    fn warning_bands_use_the_final_whole_second_quotient() {
        for (before, expected_rate) in [
            (61_000, TIME_TROPHY_NORMAL_RATE_Q16),
            (21_000, TIME_TROPHY_MEDIUM_RATE_Q16),
            (13_000, TIME_TROPHY_HIGH_RATE_Q16),
            (10_000, TIME_TROPHY_URGENT_RATE_Q16),
        ] {
            let mut runtime = active_with_remaining(before);
            assert_eq!(
                runtime.advance(1_000_000),
                Some(TimeTrophySound {
                    sound_id: TIME_TROPHY_WARNING_SOUND_ID,
                    rate_q16: expected_rate,
                })
            );
        }
    }

    #[test]
    fn endpoint_only_cadence_skips_crossed_points_and_same_second_updates() {
        let mut skipped = active_with_remaining(61_000);
        assert_eq!(skipped.advance(12_000_000), None);
        assert_eq!(skipped.remaining_millis_raw(), 49_000);

        let mut same_second = active_with_remaining(60_900);
        assert_eq!(same_second.advance(500_000), None);
        assert_eq!(same_second.remaining_millis_raw(), 60_400);
    }

    #[test]
    fn quotient_zero_warns_before_the_later_expiry() {
        let mut runtime = active_with_remaining(1_500);
        assert_eq!(
            runtime.advance(1_000_000),
            Some(TimeTrophySound {
                sound_id: TIME_TROPHY_WARNING_SOUND_ID,
                rate_q16: TIME_TROPHY_URGENT_RATE_Q16,
            })
        );
        assert_eq!(runtime.state(), TimeTrophyState::Active);
        assert_eq!(runtime.remaining_millis_raw(), 500);
        assert_eq!(
            runtime.advance(500_000),
            Some(TimeTrophySound {
                sound_id: TIME_TROPHY_EXPIRED_SOUND_ID,
                rate_q16: TIME_TROPHY_NORMAL_RATE_Q16,
            })
        );
        assert_eq!(runtime.state(), TimeTrophyState::Expired);
        assert_eq!(runtime.remaining_millis_raw(), 0);
    }

    #[test]
    fn exact_expiry_equality_uses_sound_two_and_nonactive_states_freeze() {
        let mut runtime = active_with_remaining(500);
        assert_eq!(
            runtime.advance(500_000),
            Some(TimeTrophySound {
                sound_id: TIME_TROPHY_EXPIRED_SOUND_ID,
                rate_q16: TIME_TROPHY_NORMAL_RATE_Q16,
            })
        );
        assert_eq!(runtime.advance(1_000_000), None);
        assert_eq!(runtime.remaining_millis_raw(), 0);
    }

    #[test]
    fn completion_claims_time_trophy_only_while_active() {
        let mut active_progress = progress_for(4);
        let mut active = active_with_remaining(12_345);
        assert_eq!(
            active.complete_current_world(&mut active_progress).unwrap(),
            TimeTrophyWorldCompletionOutcome {
                state: TimeTrophyState::Secured,
                world_saved_now: true,
                time_trophy_claimed_now: true,
                reward: Some(TrophyAcquisition {
                    restored_hull: false,
                    first_claim: false,
                    counted_trophy: true,
                    awarded_extra_life: false,
                    trophy_count: 1,
                    extra_lives: 0,
                }),
            }
        );
        assert_eq!(active_progress.control_slot_world_saved(4), Some(true));
        assert_eq!(
            active_progress.control_slot_time_trophy_claimed(4),
            Some(true)
        );
        assert_eq!(active.remaining_millis_raw(), 12_345);

        let mut expired_progress = progress_for(5);
        let mut expired = TimeTrophyRuntime {
            state: TimeTrophyState::Expired,
            remaining_millis_raw: 0,
            hidden_pickup_stops_countdown: false,
        };
        assert_eq!(
            expired
                .complete_current_world(&mut expired_progress)
                .unwrap(),
            TimeTrophyWorldCompletionOutcome {
                state: TimeTrophyState::WorldSavedOutsideActiveWindow,
                world_saved_now: true,
                time_trophy_claimed_now: false,
                reward: None,
            }
        );
        assert_eq!(
            expired_progress.control_slot_time_trophy_claimed(5),
            Some(false)
        );
    }

    #[test]
    fn results_transition_preserves_remaining_time() {
        let mut runtime = active_with_remaining(54_321);
        runtime.enter_results_active();
        assert_eq!(runtime.state(), TimeTrophyState::ResultsActive);
        assert_eq!(runtime.remaining_millis_raw(), 54_321);
        assert_eq!(runtime.advance(1_000_000), None);
    }

    #[test]
    fn zero_deadline_counts_once_without_claiming_the_hidden_pickup() {
        let mut progress = progress_for(1);
        let (_, first) = TimeTrophyRuntime::from_loaded_world(0, false, &mut progress).unwrap();
        let reward = first.reward.unwrap();
        assert_eq!(reward.trophy_count, 1);
        assert!(!reward.restored_hull);
        assert!(!reward.first_claim);
        assert_eq!(progress.control_slot_claimed(1), Some(false));
        assert_eq!(progress.control_slot_world_saved(1), Some(false));

        let (mut runtime, repeat) =
            TimeTrophyRuntime::from_loaded_world(0, false, &mut progress).unwrap();
        assert!(repeat.reward.is_none());
        assert!(!repeat.auto_claimed_now);
        assert!(runtime
            .complete_current_world(&mut progress)
            .unwrap()
            .reward
            .is_none());
        assert_eq!(progress.trophy_count(), 1);
    }

    #[test]
    fn timed_and_untimed_awards_share_the_every_fifth_trophy_extra_life_counter() {
        let mut progress = progress_for(0);
        for slot in 0..5 {
            progress.set_current_control_slot(Some(slot));
            let reward = if slot % 2 == 0 {
                let (_, outcome) =
                    TimeTrophyRuntime::from_loaded_world(0, false, &mut progress).unwrap();
                outcome.reward.unwrap()
            } else {
                let (mut runtime, outcome) =
                    TimeTrophyRuntime::from_loaded_world(300, false, &mut progress).unwrap();
                assert!(outcome.reward.is_none());
                runtime
                    .complete_current_world(&mut progress)
                    .unwrap()
                    .reward
                    .unwrap()
            };
            assert_eq!(reward.trophy_count, slot as u8 + 1);
            assert_eq!(reward.awarded_extra_life, slot == 4);
            assert_eq!(progress.control_slot_claimed(slot), Some(false));
        }
        assert_eq!(progress.extra_lives(), 1);
    }
}
