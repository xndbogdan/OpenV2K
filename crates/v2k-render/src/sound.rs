//! Polyphonic audio mixer for sound effects, driven by the global
//! Section-11 sound pool.
//!
//! Uses SDL2's callback-based audio device (`AudioDevice<SoundMixer>`) with
//! software mixing. The callback struct holds pre-decoded sounds and a
//! 24-voice pool (the original engine caps at 24 DirectSound buffers),
//! mixing all active voices into the output buffer each callback.
//!
//! Pool semantics mirror the runtime array at DAT_004FE64C: tables are
//! concatenated in level order (system level 2 = global ids 0-6, level 3 =
//! ids 7-109) and type=5 aliases reference targets by GLOBAL id.
//! Native requests run [`resolve_sound_playback`] before [`SoundManager::play_resolved`]:
//! FUN_00495480's integer modulation consumes one shared random word per alias
//! hop, including zero variance, before any audio-device/voice allocation.
//! Older world wrappers below still retain their separate floating-point pitch
//! policy until their callers publish native resolved requests.
//!
//! Source format: 22050 Hz mono 16-bit signed PCM (matching Section 11 blobs).
//! The output device is stereo so the original positional pan can be
//! reproduced for world sounds.

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::Sdl;
use v2k_formats::anim_sound::{AnimSoundTable, SoundPool};

mod playback;
mod positional;
pub use playback::{
    resolve_sound_playback, ResolvedSoundPlayback, SoundPlaybackMode, SoundPlaybackRequest,
};
pub use positional::{
    positional_sound_mix, resolve_positional_sound, PositionalSoundListener, PositionalSoundMix,
    PositionalSoundRequest,
};

/// Number of simultaneous voices (original: max 24 DirectSound buffers).
const MAX_VOICES: usize = 24;

/// The original world coordinate period: signed 8.8 positions wrap after
/// 65536 raw units, or 256 world units.
const WORLD_PERIOD: f32 = 256.0;
/// `DAT_004D0118 = 0x2000` raw 8.8 units in the retail executable.
const POSITIONAL_MAX_DISTANCE: f32 = 32.0;
/// `DAT_004D011C = 0x1000` raw 8.8 units in the retail executable.
const POSITIONAL_PAN_DISTANCE: f32 = 16.0;
/// `FUN_0044C970` preserves full volume through raw distance `0x100`.
const POSITIONAL_FULL_VOLUME_DISTANCE: f32 = 1.0;

/// Listener state needed by the retail positional-sound transform.
///
/// `right` is the listener/camera's normalized world-space right vector. The
/// original transforms the wrapped emitter delta through the full camera
/// basis, but only its local X component affects pan; Euclidean distance is
/// invariant under that orthonormal transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundListener {
    pub position: [f32; 3],
    pub right: [f32; 3],
}

impl SoundListener {
    pub fn new(position: [f32; 3], right: [f32; 3]) -> Self {
        Self { position, right }
    }
}

/// Gain and normalized pan derived from the original spatial-audio path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionalMix {
    /// Relative linear gain before the Section-11 alias volume multiplier.
    pub gain: f32,
    /// DirectSound pan normalized from its original `[-10000, +10000]` range.
    /// Negative is left and positive is right.
    pub pan: f32,
    /// Shortest wrapped emitter-to-listener distance in world units.
    pub distance: f32,
}

/// Reproduce `FUN_0044C970`'s positional gain and pan calculation.
///
/// Retail subtracts signed 8.8 position words, so every component follows the
/// shortest displacement on a 256-unit coordinate period. Sounds at or beyond
/// 32 units are inaudible. Gain is full through one unit and then follows the
/// original `(32 - distance) / 32` fade. Pan reaches the DirectSound limit at
/// 16 units along the camera-right axis.
pub fn original_positional_mix(
    listener: SoundListener,
    emitter_position: [f32; 3],
) -> Option<PositionalMix> {
    let delta: [f32; 3] = std::array::from_fn(|axis| {
        wrapped_world_delta(emitter_position[axis] - listener.position[axis])
    });
    let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if !distance.is_finite() || distance >= POSITIONAL_MAX_DISTANCE {
        return None;
    }

    let right_len = (listener.right[0] * listener.right[0]
        + listener.right[1] * listener.right[1]
        + listener.right[2] * listener.right[2])
        .sqrt();
    let local_x = if right_len > f32::EPSILON {
        (delta[0] * listener.right[0] + delta[1] * listener.right[1] + delta[2] * listener.right[2])
            / right_len
    } else {
        0.0
    };
    let pan = (local_x / POSITIONAL_PAN_DISTANCE).clamp(-1.0, 1.0);
    let gain = if distance <= POSITIONAL_FULL_VOLUME_DISTANCE {
        1.0
    } else {
        (POSITIONAL_MAX_DISTANCE - distance) / POSITIONAL_MAX_DISTANCE
    };

    Some(PositionalMix {
        gain,
        pan,
        distance,
    })
}

fn wrapped_world_delta(delta: f32) -> f32 {
    (delta + WORLD_PERIOD * 0.5).rem_euclid(WORLD_PERIOD) - WORLD_PERIOD * 0.5
}

/// Pre-decoded PCM sound sample.
#[derive(Clone)]
struct SoundSample {
    /// 16-bit signed PCM samples (mono 22050 Hz).
    samples: Vec<i16>,
}

/// A single voice slot in the mixer.
#[derive(Clone)]
struct Voice {
    /// Monotonic slot generation used to reject stale public handles.
    generation: u32,
    /// Whether this voice is currently playing.
    active: bool,
    /// Index into the sounds array.
    sound_id: usize,
    /// Current playback position (fractional sample index).
    pos: f32,
    /// Playback step per output sample (1.0 = native 22050 Hz).
    step: f32,
    /// Whether this voice loops.
    looping: bool,
    /// Per-voice volume (0.0 to 1.0).
    volume: f32,
    /// DirectSound-style normalized pan (-1.0 left, +1.0 right).
    pan: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            generation: 0,
            active: false,
            sound_id: 0,
            pos: 0.0,
            step: 1.0,
            looping: false,
            volume: 1.0,
            pan: 0.0,
        }
    }
}

/// Audio callback struct that mixes active voices into the output buffer.
pub struct SoundMixer {
    sounds: Vec<SoundSample>,
    voices: [Voice; MAX_VOICES],
    master_volume: f32,
    channels: usize,
}

impl SoundMixer {
    fn new(sounds: Vec<SoundSample>, channels: u8, master_volume: f32) -> Self {
        Self {
            sounds,
            voices: std::array::from_fn(|_| Voice::default()),
            master_volume: master_volume.clamp(0.0, 1.0),
            channels: usize::from(channels.max(1)),
        }
    }

    /// Find a free voice slot. If all busy, steal first non-looping voice.
    fn allocate_voice(&mut self) -> Option<usize> {
        // First: find an inactive slot
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return Some(i);
        }
        // Second: steal first non-looping voice
        if let Some(i) = self.voices.iter().position(|v| !v.looping) {
            return Some(i);
        }
        None
    }

    /// Play at native rate / full gain (kept for tests and simple callers).
    #[allow(dead_code)]
    fn play(&mut self, sound_id: usize, looping: bool) -> Option<VoiceHandle> {
        self.play_with(sound_id, looping, 1.0, 1.0)
    }

    /// Start a voice with an explicit playback rate and gain.
    fn play_with(
        &mut self,
        sound_id: usize,
        looping: bool,
        rate: f32,
        gain: f32,
    ) -> Option<VoiceHandle> {
        self.play_with_pan(sound_id, looping, rate, gain, 0.0)
    }

    /// Start a voice with an explicit playback rate, gain, and pan.
    fn play_with_pan(
        &mut self,
        sound_id: usize,
        looping: bool,
        rate: f32,
        gain: f32,
        pan: f32,
    ) -> Option<VoiceHandle> {
        if sound_id >= self.sounds.len() || self.sounds[sound_id].samples.is_empty() {
            return None;
        }
        let slot = self.allocate_voice()?;
        let generation = self.voices[slot].generation.wrapping_add(1);
        self.voices[slot] = Voice {
            generation,
            active: true,
            sound_id,
            pos: 0.0,
            // SetFrequency clamps to 100..100000 Hz in the original.
            step: rate.clamp(100.0 / 22050.0, 100000.0 / 22050.0),
            looping,
            volume: gain.clamp(0.0, 1.0),
            pan: pan.clamp(-1.0, 1.0),
        };
        Some(VoiceHandle { slot, generation })
    }

    fn stop_voice(&mut self, handle: VoiceHandle) {
        if let Some(voice) = self.voices.get_mut(handle.slot) {
            if voice.generation == handle.generation {
                voice.active = false;
            }
        }
    }

    /// Mutate one persistent voice without resetting its playback cursor.
    /// The resolved PCM id and looping bit are part of the handle contract, so
    /// a stale slot cannot retune an unrelated one-shot after `stop_all`.
    fn update_looping_voice(
        &mut self,
        handle: VoiceHandle,
        sound_id: usize,
        rate: f32,
        gain: f32,
        pan: f32,
    ) -> bool {
        let Some(voice) = self.voices.get_mut(handle.slot) else {
            return false;
        };
        if !voice.active
            || voice.generation != handle.generation
            || !voice.looping
            || voice.sound_id != sound_id
        {
            return false;
        }
        voice.step = rate.clamp(100.0 / 22050.0, 100000.0 / 22050.0);
        voice.volume = gain.clamp(0.0, 1.0);
        voice.pan = pan.clamp(-1.0, 1.0);
        true
    }

    fn stop_all(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
        }
    }
}

impl AudioCallback for SoundMixer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        // Zero the output buffer.
        for sample in out.iter_mut() {
            *sample = 0;
        }

        let master = self.master_volume;
        let channels = self.channels;

        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }

            let sound = &self.sounds[voice.sound_id];
            let sound_len = sound.samples.len();
            if sound_len == 0 {
                voice.active = false;
                continue;
            }

            let (left_gain, right_gain) = direct_sound_pan_gains(voice.pan);
            for frame in out.chunks_exact_mut(channels) {
                if voice.pos >= sound_len as f32 {
                    if voice.looping {
                        // Wrap by modulo, not a single subtraction: a high
                        // playback rate on a very short sample can overshoot
                        // the length by more than one period per output step.
                        voice.pos = voice.pos.rem_euclid(sound_len as f32);
                    } else {
                        voice.active = false;
                        break;
                    }
                }

                // Linear interpolation between adjacent source samples.
                let i = (voice.pos as usize).min(sound_len - 1);
                let frac = voice.pos - i as f32;
                let a = sound.samples[i] as f32;
                let b = if i + 1 < sound_len {
                    sound.samples[i + 1] as f32
                } else if voice.looping {
                    sound.samples[0] as f32
                } else {
                    a
                };
                let s = (a + (b - a) * frac) * voice.volume * master;
                if channels == 1 {
                    mix_sample(&mut frame[0], s);
                } else {
                    // Section-11 samples are mono. DirectSound's pan leaves
                    // the favored channel unchanged and logarithmically
                    // attenuates the opposite channel by up to 100 dB.
                    mix_sample(&mut frame[0], s * left_gain);
                    mix_sample(&mut frame[1], s * right_gain);
                    // The real device is requested as stereo. Keep any extra
                    // channels deterministic if SDL supplies a wider format.
                    for sample in &mut frame[2..] {
                        mix_sample(sample, s);
                    }
                }

                voice.pos += voice.step;
            }
        }
    }
}

fn mix_sample(output: &mut i16, sample: f32) {
    let mixed = *output as i32 + sample as i32;
    *output = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
}

/// DirectSound pan is hundredths of a decibel, from -10000 to +10000. One
/// channel stays at unity while the other is attenuated by `abs(pan)` dB.
fn direct_sound_pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    let quiet_gain = 10.0_f32.powf(-5.0 * pan.abs());
    if pan < 0.0 {
        (1.0, quiet_gain)
    } else {
        (quiet_gain, 1.0)
    }
}

/// Resolved playback parameters for one global pool id: the alias chain
/// flattened to a target blob + accumulated multipliers (FUN_00495480).
#[derive(Debug, Clone)]
struct PlaySpec {
    /// Global id of the type=1 blob at the end of the alias chain.
    blob: usize,
    /// Product of the chain's 16.16 frequency multipliers (1.0 = native).
    freq_mul: f32,
    /// Product of the chain's 16.16 volume multipliers.
    vol_mul: f32,
    /// Frequency variances collected along the chain; each randomizes the
    /// running rate per trigger: `rate *= 1 ± rand·variance`.
    variances: Vec<f32>,
}

/// Opaque handle to a playing voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceHandle {
    slot: usize,
    generation: u32,
}

/// Mutable parameters for a persistent looping voice.
///
/// `rate` and `gain` multiply the global Section-11 slot's authored alias
/// parameters. `pan` uses the normalized DirectSound range `[-1, +1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopingVoiceParams {
    pub rate: f32,
    pub gain: f32,
    pub pan: f32,
}

impl LoopingVoiceParams {
    fn normalized(self) -> Option<Self> {
        if !self.rate.is_finite()
            || self.rate <= 0.0
            || !self.gain.is_finite()
            || !self.pan.is_finite()
        {
            return None;
        }
        Some(Self {
            rate: self.rate,
            gain: self.gain.clamp(0.0, 1.0),
            pan: self.pan.clamp(-1.0, 1.0),
        })
    }
}

/// Public API for game sound effects.
///
/// Owns the SDL2 audio device. Access the mixer via `device.lock()`.
pub struct SoundManager {
    device: AudioDevice<SoundMixer>,
    specs: Vec<Option<PlaySpec>>,
    sound_count: usize,
    /// Legacy world-wrapper pitch state. Native resolved submissions do not
    /// access it; their alias words already came from the process RNG.
    rng: u32,
}

impl SoundManager {
    /// Submit an already-resolved voice without consuming random words.
    /// Resolve first even when the audio device is absent or muted, so native
    /// alias calls remain ordered with the caller's other process RNG uses.
    pub fn play_resolved(&mut self, playback: ResolvedSoundPlayback) -> Option<VoiceHandle> {
        if !(-10_000..=10_000).contains(&playback.pan) {
            return None;
        }
        self.device.lock().play_with_pan(
            playback.pcm_global_id,
            playback.mode == SoundPlaybackMode::Looping,
            playback.frequency_hz() as f32 / 22_050.0,
            playback.volume_q16 as i32 as f32 / 65_536.0,
            playback.pan as f32 / 10_000.0,
        )
    }

    /// Retune an admitted native loop through 004957C0 without resolving its
    /// aliases again. The logical owner supplies the current wrapper controls.
    pub fn update_resolved(
        &mut self,
        handle: VoiceHandle,
        playback: ResolvedSoundPlayback,
    ) -> bool {
        if playback.mode != SoundPlaybackMode::Looping
            || !(-10_000..=10_000).contains(&playback.pan)
        {
            return false;
        }
        self.device.lock().update_looping_voice(
            handle,
            playback.pcm_global_id,
            playback.frequency_hz() as f32 / 22_050.0,
            playback.volume_q16 as i32 as f32 / 65_536.0,
            playback.pan as f32 / 10_000.0,
        )
    }

    /// Build the sound manager from a single Section 11 table (global ids =
    /// local indices). Prefer [`Self::from_tables`] with the full system
    /// pool (levels 2 + 3). `master_volume` is installed before the SDL
    /// device starts, so the manager never exposes an unrelated default gain.
    pub fn new(sdl: &Sdl, table: &AnimSoundTable, master_volume: f32) -> Result<Self, String> {
        Self::from_tables(sdl, &[table], master_volume)
    }

    /// Build the GLOBAL sound pool from Section 11 tables concatenated in
    /// level order, mirroring DAT_004FE64C (pass system level 2 then 3).
    /// Type=5 alias targets are GLOBAL ids into the concatenated pool. The
    /// supplied master gain is clamped to the mixer's linear 0.0..=1.0 range.
    pub fn from_tables(
        sdl: &Sdl,
        tables: &[&AnimSoundTable],
        master_volume: f32,
    ) -> Result<Self, String> {
        let pool = SoundPool::from_tables(tables);
        let sounds = pool
            .slots()
            .iter()
            .map(|slot| {
                let samples = slot
                    .table
                    .blob_data(slot.entry)
                    .map(|bytes| {
                        bytes
                            .chunks_exact(2)
                            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
                            .collect()
                    })
                    .unwrap_or_default();
                SoundSample { samples }
            })
            .collect::<Vec<_>>();
        let specs = pool
            .slots()
            .iter()
            .map(|slot| {
                pool.resolve(slot.global_id).ok().map(|resolved| PlaySpec {
                    blob: resolved.pcm_global_id,
                    freq_mul: resolved.frequency_multiplier as f32,
                    vol_mul: resolved.volume_multiplier as f32,
                    variances: resolved
                        .alias_hops
                        .iter()
                        .filter_map(|hop| {
                            (hop.frequency_variance_16_16 != 0)
                                .then_some(hop.frequency_variance_16_16 as f32 / 65_536.0)
                        })
                        .collect(),
                })
            })
            .collect::<Vec<_>>();

        let sound_count = sounds.len();

        let audio = sdl.audio()?;
        let desired = AudioSpecDesired {
            freq: Some(22050),
            channels: Some(2),
            samples: Some(1024),
        };

        let device = audio.open_playback(None, &desired, |spec| {
            SoundMixer::new(sounds, spec.channels, master_volume)
        })?;
        device.resume();

        Ok(Self {
            device,
            specs,
            sound_count,
            rng: 0x2F6E2B1,
        })
    }

    /// Uniform random in [-1.0, 1.0) (xorshift32).
    fn rand_unit(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Play a one-shot sound effect by GLOBAL pool id, honoring the alias
    /// chain's rate/volume and randomizing the rate by each variance
    /// (per-trigger, like the original).
    pub fn play_sound(&mut self, sound_id: usize) -> Option<VoiceHandle> {
        self.play_one_shot(sound_id, 1.0, 0.0)
    }

    /// Play a centered one-shot with an explicit gain multiplier while still
    /// honoring the global slot's authored alias rate, volume, and variance.
    ///
    /// This is the non-positional `FUN_004958C0` path used by callers such as
    /// gameplay text, whose wrapper parameters request half gain without
    /// changing the sample's native playback rate.
    pub fn play_centered_sound_with_gain(
        &mut self,
        sound_id: usize,
        gain: f32,
    ) -> Option<VoiceHandle> {
        self.play_one_shot(sound_id, normalized_gain(gain)?, 0.0)
    }

    /// Play a centered one-shot with an explicit unsigned Q16.16 wrapper
    /// playback-rate multiplier. An equal deterministic range preserves the
    /// authored alias chain while avoiding an extra wrapper RNG draw.
    pub fn play_centered_sound_with_rate_q16(
        &mut self,
        sound_id: usize,
        rate_q16: u32,
    ) -> Option<VoiceHandle> {
        let rate = rate_q16 as f32 / 65_536.0;
        self.play_one_shot_with_rate_range(sound_id, 1.0, 0.0, rate, rate)
    }

    /// Play a world-positioned one-shot using the retail attenuation, pan,
    /// and 256-unit wrapped-coordinate rules from `FUN_0044C970`.
    ///
    /// Returns `None` both for invalid sound ids and emitters outside the
    /// original 32-unit audible radius.
    pub fn play_positional_sound(
        &mut self,
        sound_id: usize,
        emitter_position: [f32; 3],
        listener: SoundListener,
    ) -> Option<VoiceHandle> {
        self.play_positional_sound_with_rate_range(sound_id, emitter_position, listener, 1.0, 1.0)
    }

    /// Play a world-positioned one-shot with an additional uniformly random
    /// playback-rate multiplier. The wrapper multiplier is selected before
    /// the alias chain applies its own per-hop pitch variances.
    ///
    /// Bounds may be supplied in either order, but both must be finite and
    /// positive. An equal range is deterministic and does not advance the
    /// pitch RNG.
    pub fn play_positional_sound_with_rate_range(
        &mut self,
        sound_id: usize,
        emitter_position: [f32; 3],
        listener: SoundListener,
        min_rate: f32,
        max_rate: f32,
    ) -> Option<VoiceHandle> {
        let mix = original_positional_mix(listener, emitter_position)?;
        self.play_one_shot_with_rate_range(sound_id, mix.gain, mix.pan, min_rate, max_rate)
    }

    fn play_one_shot(&mut self, sound_id: usize, gain: f32, pan: f32) -> Option<VoiceHandle> {
        self.play_one_shot_with_rate_range(sound_id, gain, pan, 1.0, 1.0)
    }

    fn play_one_shot_with_rate_range(
        &mut self,
        sound_id: usize,
        gain: f32,
        pan: f32,
        min_rate: f32,
        max_rate: f32,
    ) -> Option<VoiceHandle> {
        let spec = self.specs.get(sound_id)?.clone()?;
        let (min_rate, max_rate) = valid_rate_range(min_rate, max_rate)?;
        let wrapper_rate = if min_rate == max_rate {
            min_rate
        } else {
            uniform_rate_multiplier(self.rand_unit(), min_rate, max_rate)
        };
        let mut rate = spec.freq_mul * wrapper_rate;
        for &v in &spec.variances {
            rate *= 1.0 + self.rand_unit() * v;
        }
        let mut mixer = self.device.lock();
        mixer.play_with_pan(spec.blob, false, rate, spec.vol_mul * gain, pan)
    }

    /// Start one persistent looping sound by global id without rerolling alias
    /// variance. Later updates retain its playback cursor.
    pub fn play_looping(
        &mut self,
        sound_id: usize,
        params: LoopingVoiceParams,
    ) -> Option<VoiceHandle> {
        let spec = self.specs.get(sound_id)?.clone()?;
        let params = params.normalized()?;
        let mut mixer = self.device.lock();
        mixer.play_with_pan(
            spec.blob,
            true,
            spec.freq_mul * params.rate,
            spec.vol_mul * params.gain,
            params.pan,
        )
    }

    /// Update one persistent loop in place. Returns false when the handle no
    /// longer names the same active resolved PCM voice, allowing the owner to
    /// recreate it after a global stop or level transition.
    pub fn update_looping(
        &mut self,
        handle: VoiceHandle,
        sound_id: usize,
        params: LoopingVoiceParams,
    ) -> bool {
        let Some(spec) = self.specs.get(sound_id).cloned().flatten() else {
            return false;
        };
        let Some(params) = params.normalized() else {
            return false;
        };
        let mut mixer = self.device.lock();
        mixer.update_looping_voice(
            handle,
            spec.blob,
            spec.freq_mul * params.rate,
            spec.vol_mul * params.gain,
            params.pan,
        )
    }

    /// Stop a specific voice.
    pub fn stop(&mut self, handle: VoiceHandle) {
        let mut mixer = self.device.lock();
        mixer.stop_voice(handle);
    }

    /// Stop all playing voices.
    pub fn stop_all(&mut self) {
        let mut mixer = self.device.lock();
        mixer.stop_all();
    }

    /// Number of global pool slots.
    pub fn sound_count(&self) -> usize {
        self.sound_count
    }

    /// Set the master volume (0.0–1.0).
    ///
    /// The original's Sound slider (0-15) maps to linear buffer gain
    /// s·17/255 then to DirectSound dB via 10000·ln(lin)/ln(65536), which
    /// works out to amplitude ≈ lin^1.038 — effectively linear, so a plain
    /// s/15 master here matches the original curve.
    pub fn set_master_volume(&mut self, vol: f32) {
        let mut mixer = self.device.lock();
        mixer.master_volume = vol.clamp(0.0, 1.0);
    }

    /// Get the current master volume.
    pub fn master_volume(&mut self) -> f32 {
        let mixer = self.device.lock();
        mixer.master_volume
    }
}

fn valid_rate_range(min_rate: f32, max_rate: f32) -> Option<(f32, f32)> {
    if !min_rate.is_finite() || !max_rate.is_finite() || min_rate <= 0.0 || max_rate <= 0.0 {
        return None;
    }
    Some((min_rate.min(max_rate), min_rate.max(max_rate)))
}

fn normalized_gain(gain: f32) -> Option<f32> {
    gain.is_finite().then(|| gain.clamp(0.0, 1.0))
}

fn uniform_rate_multiplier(signed_unit: f32, min_rate: f32, max_rate: f32) -> f32 {
    let unit = ((signed_unit + 1.0) * 0.5).clamp(0.0, 1.0);
    min_rate + (max_rate - min_rate) * unit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_one_shot_gain_rejects_nonfinite_and_clamps_to_unit_range() {
        assert_eq!(normalized_gain(0.5), Some(0.5));
        assert_eq!(normalized_gain(-1.0), Some(0.0));
        assert_eq!(normalized_gain(2.0), Some(1.0));
        assert_eq!(normalized_gain(f32::NAN), None);
    }

    /// Create a SoundMixer with test samples (no SDL2 device needed).
    fn test_mixer(sounds: Vec<Vec<i16>>) -> SoundMixer {
        test_mixer_with_master_volume(sounds, 0.8)
    }

    fn test_mixer_with_master_volume(sounds: Vec<Vec<i16>>, master_volume: f32) -> SoundMixer {
        let samples: Vec<SoundSample> = sounds
            .into_iter()
            .map(|s| SoundSample { samples: s })
            .collect();
        SoundMixer::new(samples, 1, master_volume)
    }

    /// Create the same mixer using an interleaved stereo output buffer.
    fn test_stereo_mixer(sounds: Vec<Vec<i16>>) -> SoundMixer {
        let samples: Vec<SoundSample> = sounds
            .into_iter()
            .map(|s| SoundSample { samples: s })
            .collect();
        SoundMixer::new(samples, 2, 0.8)
    }

    #[test]
    fn empty_callback_produces_silence() {
        let mut mixer = test_mixer(vec![vec![100, 200, 300]]);
        // No voices active — output should be all zeros
        let mut out = vec![0i16; 8];
        mixer.callback(&mut out);
        assert!(out.iter().all(|&s| s == 0));
    }

    #[test]
    fn configured_master_volume_applies_to_the_first_callback() {
        let mut mixer = test_mixer_with_master_volume(vec![vec![10_000]], 4.0 / 15.0);
        mixer.play(0, false).unwrap();

        let mut out = vec![0i16; 1];
        mixer.callback(&mut out);

        assert_eq!(out[0], (10_000.0 * (4.0 / 15.0)) as i16);
    }

    #[test]
    fn oneshot_plays_then_stops() {
        let mut mixer = test_mixer(vec![vec![1000, 2000, 3000]]);
        let slot = mixer.play(0, false).unwrap();

        // First callback: 4 samples, but sound is only 3 long
        let mut out = vec![0i16; 4];
        mixer.callback(&mut out);

        // First 3 samples should have the sound data (scaled by volume 0.8)
        let expected_0 = (1000.0 * 0.8) as i16;
        let expected_1 = (2000.0 * 0.8) as i16;
        let expected_2 = (3000.0 * 0.8) as i16;
        assert_eq!(out[0], expected_0);
        assert_eq!(out[1], expected_1);
        assert_eq!(out[2], expected_2);
        // 4th sample: voice ended, should be 0
        assert_eq!(out[3], 0);

        // Voice should now be inactive
        assert!(!mixer.voices[slot.slot].active);
    }

    #[test]
    fn looping_wraps_around() {
        let mut mixer = test_mixer(vec![vec![1000, 2000]]);
        mixer.play(0, true).unwrap();

        // Request 5 samples from a 2-sample looping sound
        let mut out = vec![0i16; 5];
        mixer.callback(&mut out);

        let s0 = (1000.0 * 0.8) as i16;
        let s1 = (2000.0 * 0.8) as i16;
        // Pattern: s0, s1, s0, s1, s0
        assert_eq!(out[0], s0);
        assert_eq!(out[1], s1);
        assert_eq!(out[2], s0);
        assert_eq!(out[3], s1);
        assert_eq!(out[4], s0);

        // Voice should still be active
        assert!(mixer.voices[0].active);
    }

    #[test]
    fn half_rate_doubles_duration() {
        let mut mixer = test_mixer(vec![vec![1000, 2000, 3000, 4000]]);
        mixer.play_with(0, false, 0.5, 1.0).unwrap();

        let mut out = vec![0i16; 9];
        mixer.callback(&mut out);

        // At rate 0.5 with linear interpolation the voice spans 8 output
        // samples (positions 0.0..3.5), then deactivates on the 9th fetch.
        let g = 0.8;
        assert_eq!(out[0], (1000.0 * g) as i16);
        assert_eq!(out[1], (1500.0 * g) as i16);
        assert_eq!(out[2], (2000.0 * g) as i16);
        assert_eq!(out[6], (4000.0 * g) as i16);
        assert_eq!(out[8], 0);
        assert!(!mixer.voices[0].active);
    }

    #[test]
    fn per_voice_gain_applies() {
        let mut mixer = test_mixer(vec![vec![10000]]);
        mixer.play_with(0, false, 1.0, 0.5).unwrap();
        let mut out = vec![0i16; 1];
        mixer.callback(&mut out);
        assert_eq!(out[0], (10000.0 * 0.5 * 0.8) as i16);
    }

    #[test]
    fn looping_update_changes_controls_without_resetting_cursor() {
        let mut mixer = test_mixer(vec![vec![1000, 2000, 3000]]);
        let handle = mixer.play_with_pan(0, true, 1.0, 0.25, 0.0).unwrap();
        mixer.voices[handle.slot].pos = 1.25;

        assert!(mixer.update_looping_voice(handle, 0, 2.25, 0.5, -0.75));
        let voice = &mixer.voices[handle.slot];
        assert_eq!(voice.pos, 1.25);
        assert_eq!(voice.step, 2.25);
        assert_eq!(voice.volume, 0.5);
        assert_eq!(voice.pan, -0.75);
    }

    #[test]
    fn stale_looping_handle_cannot_retune_a_reused_slot() {
        let mut mixer = test_mixer(vec![vec![1000], vec![2000]]);
        let stale = mixer.play(0, true).unwrap();
        mixer.stop_voice(stale);
        let replacement = mixer.play(1, false).unwrap();
        assert_eq!(replacement.slot, stale.slot);
        assert_ne!(replacement.generation, stale.generation);

        assert!(!mixer.update_looping_voice(stale, 0, 2.0, 0.1, 1.0));
        let voice = &mixer.voices[replacement.slot];
        assert_eq!(voice.sound_id, 1);
        assert_eq!(voice.step, 1.0);
        assert_eq!(voice.volume, 1.0);
        assert_eq!(voice.pan, 0.0);
    }

    #[test]
    fn looping_controls_reject_nonfinite_rate_gain_or_pan() {
        let valid = LoopingVoiceParams {
            rate: 1.5,
            gain: 2.0,
            pan: -2.0,
        }
        .normalized()
        .unwrap();
        assert_eq!(valid.gain, 1.0);
        assert_eq!(valid.pan, -1.0);
        for invalid in [
            LoopingVoiceParams {
                rate: 0.0,
                gain: 1.0,
                pan: 0.0,
            },
            LoopingVoiceParams {
                rate: f32::NAN,
                gain: 1.0,
                pan: 0.0,
            },
            LoopingVoiceParams {
                rate: 1.0,
                gain: f32::INFINITY,
                pan: 0.0,
            },
            LoopingVoiceParams {
                rate: 1.0,
                gain: 1.0,
                pan: f32::NAN,
            },
        ] {
            assert!(invalid.normalized().is_none());
        }
    }

    #[test]
    fn stereo_center_pan_duplicates_mono_source() {
        let mut mixer = test_stereo_mixer(vec![vec![10000]]);
        mixer.play_with_pan(0, false, 1.0, 1.0, 0.0).unwrap();
        let mut out = vec![0i16; 2];
        mixer.callback(&mut out);
        assert_eq!(out, vec![8000, 8000]);
    }

    #[test]
    fn stereo_pan_uses_direct_sound_logarithmic_attenuation() {
        let mut mixer = test_stereo_mixer(vec![vec![10000], vec![10000]]);
        mixer.play_with_pan(0, false, 1.0, 1.0, 1.0).unwrap();
        mixer.play_with_pan(1, false, 1.0, 1.0, -1.0).unwrap();
        let mut out = vec![0i16; 2];
        mixer.callback(&mut out);

        // Each full-pan voice contributes 8000 to its favored side and less
        // than one integer sample after the opposite side's 100 dB cut.
        assert_eq!(out, vec![8000, 8000]);
    }

    #[test]
    fn positional_mix_matches_retail_distance_and_pan_constants() {
        let listener = SoundListener::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

        let full = original_positional_mix(listener, [1.0, 0.0, 0.0]).unwrap();
        assert_eq!(full.gain, 1.0);
        assert!((full.pan - 1.0 / 16.0).abs() < 1e-6);

        let faded = original_positional_mix(listener, [16.0, 0.0, 0.0]).unwrap();
        assert!((faded.gain - 0.5).abs() < 1e-6);
        assert_eq!(faded.pan, 1.0);

        let edge = original_positional_mix(listener, [31.0, 0.0, 0.0]).unwrap();
        assert!((edge.gain - 1.0 / 32.0).abs() < 1e-6);
        assert!(original_positional_mix(listener, [32.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn positional_mix_uses_camera_right_axis() {
        let listener = SoundListener::new([4.0, 2.0, 8.0], [0.0, 0.0, -1.0]);
        let mix = original_positional_mix(listener, [4.0, 2.0, 0.0]).unwrap();
        assert_eq!(mix.distance, 8.0);
        assert_eq!(mix.pan, 0.5);
        assert!((mix.gain - 0.75).abs() < 1e-6);
    }

    #[test]
    fn positional_mix_wraps_the_original_signed_position_words() {
        let listener = SoundListener::new([255.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let mix = original_positional_mix(listener, [1.0, 0.0, 0.0]).unwrap();
        assert_eq!(mix.distance, 2.0);
        assert_eq!(mix.pan, 0.125);
        assert!((mix.gain - 30.0 / 32.0).abs() < 1e-6);
    }

    #[test]
    fn positional_rate_range_is_order_independent_and_uniform() {
        let retail_max = 0x11fff as f32 / 65536.0;
        assert_eq!(valid_rate_range(retail_max, 1.0), Some((1.0, retail_max)));
        assert_eq!(uniform_rate_multiplier(-1.0, 1.0, retail_max), 1.0);
        assert_eq!(
            uniform_rate_multiplier(0.0, 1.0, retail_max),
            1.0 + (retail_max - 1.0) * 0.5
        );
        assert_eq!(uniform_rate_multiplier(1.0, 1.0, retail_max), retail_max);
        assert!(valid_rate_range(0.0, 1.0).is_none());
        assert!(valid_rate_range(f32::NAN, 1.0).is_none());
    }

    #[test]
    fn multi_voice_mixing_clamps() {
        // Two sounds at max positive value — mixing should clamp to i16::MAX
        let mut mixer = test_mixer(vec![vec![i16::MAX, i16::MAX], vec![i16::MAX, i16::MAX]]);
        mixer.play(0, false).unwrap();
        mixer.play(1, false).unwrap();

        let mut out = vec![0i16; 2];
        mixer.callback(&mut out);

        // Both voices contribute ~0.8 * 32767 each ≈ 26213 + 26213 = 52426 > 32767
        // Should clamp to i16::MAX
        assert_eq!(out[0], i16::MAX);
        assert_eq!(out[1], i16::MAX);
    }

    #[test]
    fn short_loop_high_rate_does_not_panic() {
        // A 3-sample looping sound played faster than one sample per output
        // step must wrap by modulo, not a single subtraction (else pos stays
        // past the end and the sample index panics out of bounds).
        let mut mixer = test_mixer(vec![vec![1000, 2000, 3000]]);
        mixer.play_with(0, true, 4.5, 1.0).unwrap();
        let mut out = vec![0i16; 64];
        mixer.callback(&mut out); // must not panic
        assert!(mixer.voices[0].active);
    }

    #[test]
    fn voice_stealing_when_all_full() {
        // Only 1 sound, fill all voice slots
        let mut mixer = test_mixer(vec![vec![100; 1000]]);

        for _ in 0..MAX_VOICES {
            mixer.play(0, false).unwrap();
        }
        // All slots full with non-looping voices
        assert!(mixer.voices.iter().all(|v| v.active));

        // Should steal slot 0 (first non-looping)
        let slot = mixer.play(0, false).unwrap();
        assert_eq!(slot.slot, 0);
        assert_eq!(mixer.voices[0].pos, 0.0); // freshly allocated
    }

    #[test]
    fn voice_stealing_skips_looping() {
        let mut mixer = test_mixer(vec![vec![100; 1000]]);

        // Fill all slots with looping voices
        for _ in 0..MAX_VOICES {
            mixer.play(0, true).unwrap();
        }

        // All looping — stealing should fail (returns None)
        assert!(mixer.play(0, false).is_none());
    }

    #[test]
    fn stop_all_clears_everything() {
        let mut mixer = test_mixer(vec![vec![100; 100]]);
        mixer.play(0, false).unwrap();
        mixer.play(0, true).unwrap();
        mixer.play(0, false).unwrap();

        assert_eq!(mixer.voices.iter().filter(|v| v.active).count(), 3);

        mixer.stop_all();
        assert!(mixer.voices.iter().all(|v| !v.active));
    }

    #[test]
    fn invalid_sound_id_returns_none() {
        let mut mixer = test_mixer(vec![vec![100]]);
        assert!(mixer.play(1, false).is_none()); // out of range
        assert!(mixer.play(999, false).is_none());
    }
}
