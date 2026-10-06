//! `FUN_004292B0`'s active-countdown trophy and clock, before the cargo row.

use super::{retail_orientation, GameplayHudCargoAnimation, GameplayHudPoint};
use crate::time_trophy::TimeTrophyRuntime;
use v2k_formats::models::AnimVars;

pub const TIME_TROPHY_HUD_MODEL_ID: usize = 138;

/// Absolute Section-1 points 40/41 (common-overlay local points 27/28).
/// Unlike the status-orb offsets these do not add the orb's base position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayHudTimeTrophyLayout {
    pub model: GameplayHudPoint,
    pub text: GameplayHudPoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayHudTimeTrophyFrame {
    pub global_model_id: usize,
    pub model_position: GameplayHudPoint,
    pub text_position: GameplayHudPoint,
    pub text: String,
    pub spin_raw: u16,
    pub animation: GameplayHudCargoAnimation,
}

impl GameplayHudTimeTrophyFrame {
    /// Retail 429509 tests state 1 exactly, so expiry, completion and abort
    /// all remove the HUD trophy and clock. This is not a world-entity death.
    pub fn from_runtime(
        runtime: TimeTrophyRuntime,
        layout: GameplayHudTimeTrophyLayout,
        cargo_spin_raw: u16,
        retail_tick: u32,
    ) -> Option<Self> {
        if !runtime.countdown_active() {
            return None;
        }
        let seconds = runtime.remaining_millis_raw() / 1_000;
        Some(Self {
            global_model_id: TIME_TROPHY_HUD_MODEL_ID,
            model_position: layout.model,
            text_position: layout.text,
            // Executable 4295FA..42961F: signed /60 and %60, "%1d:%02d".
            text: format!("{}:{:02}", seconds / 60, seconds % 60),
            spin_raw: cargo_spin_raw.wrapping_mul(2),
            animation: GameplayHudCargoAnimation::from_retail_tick(retail_tick),
        })
    }

    pub const fn depth_raw(&self) -> i32 {
        2_000
    }

    pub fn orientation(&self) -> [[f32; 3]; 3] {
        retail_orientation(self.spin_raw, 0, 0)
    }

    pub fn animation_vars(&self) -> AnimVars {
        self.animation.anim_vars()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power_up_contact::PlayerCampaignProgress;

    const LAYOUT: GameplayHudTimeTrophyLayout = GameplayHudTimeTrophyLayout {
        model: GameplayHudPoint { x: 60, y: 30 },
        text: GameplayHudPoint { x: 90, y: 25 },
    };

    fn active(seconds: u32) -> (TimeTrophyRuntime, PlayerCampaignProgress) {
        let mut progress = PlayerCampaignProgress::new();
        progress.set_current_control_slot(Some(2));
        let (runtime, _) =
            TimeTrophyRuntime::from_loaded_world(seconds, false, &mut progress).unwrap();
        (runtime, progress)
    }

    #[test]
    fn active_clock_keeps_authored_anchors_double_spin_and_linked_model_callback() {
        let (runtime, _) = active(270);
        let frame =
            GameplayHudTimeTrophyFrame::from_runtime(runtime, LAYOUT, 0xc000, 0x12345).unwrap();
        assert_eq!(frame.text, "4:30");
        assert_eq!(frame.model_position, LAYOUT.model);
        assert_eq!(frame.text_position, LAYOUT.text);
        assert_eq!(frame.global_model_id, 138);
        assert_eq!(frame.depth_raw(), 2_000);
        assert_eq!(frame.spin_raw, 0x8000);
        assert_eq!(frame.animation_vars().dynamic[0], 0x2345);
    }

    #[test]
    fn quotient_zero_remains_visible_until_expiry_and_completion_hides_immediately() {
        let (mut runtime, mut progress) = active(1);
        runtime.advance(1_000_000);
        assert_eq!(
            GameplayHudTimeTrophyFrame::from_runtime(runtime, LAYOUT, 0, 0)
                .unwrap()
                .text,
            "0:00"
        );
        let mut expired = runtime;
        expired.advance(500_000);
        assert!(GameplayHudTimeTrophyFrame::from_runtime(expired, LAYOUT, 0, 0).is_none());
        runtime.complete_current_world(&mut progress).unwrap();
        assert!(GameplayHudTimeTrophyFrame::from_runtime(runtime, LAYOUT, 0, 0).is_none());
    }

    #[test]
    fn untimed_and_aborted_worlds_do_not_display_an_active_challenge() {
        let (mut runtime, _) = active(0);
        assert!(GameplayHudTimeTrophyFrame::from_runtime(runtime, LAYOUT, 0, 0).is_none());
        runtime.enter_results_active();
        assert!(GameplayHudTimeTrophyFrame::from_runtime(runtime, LAYOUT, 0, 0).is_none());
        assert!(GameplayHudTimeTrophyFrame::from_runtime(
            TimeTrophyRuntime::default(),
            LAYOUT,
            0,
            0
        )
        .is_none());
    }

    #[test]
    fn requested_hidden_pickup_policy_hides_trophy_and_clock_together() {
        let (mut runtime, _) = active(270);
        runtime.stop_countdown_for_hidden_pickup();
        assert!(GameplayHudTimeTrophyFrame::from_runtime(runtime, LAYOUT, 0, 0).is_none());
        assert_eq!(runtime.advance(1_000_000), None);
        assert_eq!(runtime.remaining_millis_raw(), 270_500);
    }
}
