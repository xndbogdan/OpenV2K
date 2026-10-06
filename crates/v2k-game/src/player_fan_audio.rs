//! Persistent two-layer player-fan presentation.
//!
//! `FUN_00420920` owns two looping positional voices for the lifetime of the
//! controlled craft. The high layer has a deliberately louder logical-record
//! creation call, then both records are retuned every controller callback.
//! Their physical mixer voices exist only while the craft is audible: retail
//! destroys those buffers outside the positional radius and restarts them from
//! the beginning when they become audible again.

use v2k_render::{
    original_positional_mix, LoopingVoiceParams, SoundListener, SoundManager, VoiceHandle,
};

use crate::player::{PlayerFanSoundFrame, PLAYER_FAN_LOWER_SOUND_ID, PLAYER_FAN_UPPER_SOUND_ID};

#[derive(Debug, Default)]
pub struct PlayerFanAudio {
    upper: FanLoopState,
    lower: FanLoopState,
}

#[derive(Debug, Default)]
struct FanLoopState {
    /// Physical SDL voice, equivalent to the retail logical record's +0x1C
    /// DirectSound buffer. It is absent while the emitter is out of range.
    physical: Option<VoiceHandle>,
    /// The retail logical sound record survives positional culling. This flag
    /// preserves its creation/update distinction independently of the buffer.
    logical_created: bool,
}

impl PlayerFanAudio {
    /// `44F3E0(8) -> 44CE70`: stop physical fan voices on modal entry while
    /// preserving the two C830 records and their creation/update distinction.
    /// The next active world callback re-admits each voice at its current gain.
    pub fn suspend_physical_voices(&mut self, mut sound_manager: Option<&mut SoundManager>) {
        for state in [&mut self.upper, &mut self.lower] {
            if let Some(handle) = state.physical.take() {
                if let Some(manager) = sound_manager.as_deref_mut() {
                    manager.stop(handle);
                }
            }
        }
    }

    /// Submit the current callback's fan state at the craft's callback-entry
    /// position. Out-of-range logical records remain alive, but their physical
    /// voices are stopped exactly as `FUN_0044C970/FUN_0044C810` require.
    pub fn update(
        &mut self,
        sound_manager: &mut SoundManager,
        frame: PlayerFanSoundFrame,
        emitter_position: [f32; 3],
        listener: SoundListener,
    ) {
        let Some(mix) = original_positional_mix(listener, emitter_position) else {
            suspend_physical_loop(sound_manager, &mut self.upper);
            suspend_physical_loop(sound_manager, &mut self.lower);
            // FUN_00420920 has still created/updated both logical records even
            // when the later positional walker declines to create a buffer.
            self.upper.logical_created = true;
            self.lower.logical_created = true;
            return;
        };
        let (positional_gain, pan) = (mix.gain, mix.pan);
        let controls = fan_loop_controls(frame, positional_gain, pan);

        let upper_start = upper_physical_start_controls(self.upper.logical_created, controls);
        maintain_loop(
            sound_manager,
            &mut self.upper.physical,
            PLAYER_FAN_UPPER_SOUND_ID,
            upper_start,
            controls.upper_update,
        );
        maintain_loop(
            sound_manager,
            &mut self.lower.physical,
            PLAYER_FAN_LOWER_SOUND_ID,
            controls.lower,
            controls.lower,
        );
        self.upper.logical_created = true;
        self.lower.logical_created = true;
    }

    /// Forget handles after an explicit world/audio teardown. The sound
    /// manager's generational handles also reject stale slots, but clearing the
    /// owner records the intended player-spawn lifetime directly. Callers must
    /// stop the sound manager first; all current teardown paths do so.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn suspend_physical_loop(sound_manager: &mut SoundManager, state: &mut FanLoopState) {
    if let Some(handle) = state.physical.take() {
        sound_manager.stop(handle);
    }
}

fn upper_physical_start_controls(
    logical_created: bool,
    controls: FanLoopControls,
) -> LoopingVoiceParams {
    if logical_created {
        controls.upper_update
    } else {
        controls.upper_start
    }
}

fn maintain_loop(
    sound_manager: &mut SoundManager,
    handle: &mut Option<VoiceHandle>,
    sound_id: usize,
    start: LoopingVoiceParams,
    update: LoopingVoiceParams,
) {
    if handle.is_some_and(|voice| sound_manager.update_looping(voice, sound_id, update)) {
        return;
    }
    *handle = sound_manager.play_looping(sound_id, start);
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FanLoopControls {
    upper_start: LoopingVoiceParams,
    upper_update: LoopingVoiceParams,
    lower: LoopingVoiceParams,
}

fn fan_loop_controls(
    frame: PlayerFanSoundFrame,
    positional_gain: f32,
    pan: f32,
) -> FanLoopControls {
    let upper_rate = q16(frame.upper_rate_q16);
    let upper_start = LoopingVoiceParams {
        rate: upper_rate,
        gain: q16(frame.upper_start_gain_q16) * positional_gain,
        pan,
    };
    let upper_update = LoopingVoiceParams {
        rate: upper_rate,
        gain: q16(frame.upper_update_gain_q16) * positional_gain,
        pan,
    };
    let lower = LoopingVoiceParams {
        rate: q16(frame.lower_rate_q16),
        gain: q16(frame.lower_gain_q16) * positional_gain,
        pan,
    };
    FanLoopControls {
        upper_start,
        upper_update,
        lower,
    }
}

fn q16(value: i32) -> f32 {
    value as f32 / 65_536.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retail_fan_controls_preserve_creation_asymmetry_and_shared_position() {
        let controls = fan_loop_controls(
            PlayerFanSoundFrame {
                upper_rate_q16: 0x2_4000,
                upper_start_gain_q16: 0x1_0000,
                upper_update_gain_q16: 0x1_0000 / 6,
                lower_rate_q16: 0x8000,
                lower_gain_q16: 0xA000,
            },
            0.5,
            -0.25,
        );

        assert_eq!(controls.upper_start.rate, 2.25);
        assert_eq!(controls.upper_start.gain, 0.5);
        assert!((controls.upper_update.gain - (1.0 / 12.0)).abs() < 1.0e-5);
        assert_eq!(controls.lower.rate, 0.5);
        assert_eq!(controls.lower.gain, 0.3125);
        assert_eq!(controls.upper_start.pan, -0.25);
        assert_eq!(controls.upper_update.pan, -0.25);
        assert_eq!(controls.lower.pan, -0.25);
    }

    #[test]
    fn zero_positional_gain_still_produces_valid_controls_inside_the_radius() {
        let controls = fan_loop_controls(
            PlayerFanSoundFrame {
                upper_rate_q16: 0x1_8000,
                upper_start_gain_q16: 0x2000,
                upper_update_gain_q16: 0x2000 / 6,
                lower_rate_q16: 0x8000,
                lower_gain_q16: 0x3000,
            },
            0.0,
            0.0,
        );
        assert_eq!(controls.upper_start.gain, 0.0);
        assert_eq!(controls.upper_update.gain, 0.0);
        assert_eq!(controls.lower.gain, 0.0);
    }

    #[test]
    fn reentering_range_after_logical_creation_uses_steady_upper_gain() {
        let controls = fan_loop_controls(
            PlayerFanSoundFrame {
                upper_rate_q16: 0x2_4000,
                upper_start_gain_q16: 0x1_0000,
                upper_update_gain_q16: 0x1_0000 / 6,
                lower_rate_q16: 0x8000,
                lower_gain_q16: 0xA000,
            },
            1.0,
            0.0,
        );
        let first_audible_start = upper_physical_start_controls(false, controls);
        let reentry_start = upper_physical_start_controls(true, controls);

        assert_eq!(first_audible_start.gain, 1.0);
        assert_eq!(reentry_start.gain, q16(0x1_0000 / 6));
    }

    #[test]
    fn modal_suspension_preserves_fan_creation_gain_across_repeated_entry() {
        let controls = fan_loop_controls(
            PlayerFanSoundFrame {
                upper_rate_q16: 0x2_4000,
                upper_start_gain_q16: 0x1_0000,
                upper_update_gain_q16: 0x1_0000 / 6,
                lower_rate_q16: 0x8000,
                lower_gain_q16: 0xA000,
            },
            1.0,
            0.0,
        );
        for logical_created in [false, true] {
            let mut audio = PlayerFanAudio::default();
            audio.upper.logical_created = logical_created;
            audio.lower.logical_created = logical_created;
            let before = upper_physical_start_controls(logical_created, controls);
            audio.suspend_physical_voices(None);
            audio.suspend_physical_voices(None);
            assert_eq!(audio.upper.logical_created, logical_created);
            assert_eq!(audio.lower.logical_created, logical_created);
            assert_eq!(
                upper_physical_start_controls(audio.upper.logical_created, controls),
                before,
                "resuming a retained fan must not replay the louder creation gain"
            );
        }
    }
}
