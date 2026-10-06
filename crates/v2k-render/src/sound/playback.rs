//! The integer alias-resolution phase of FUN_00495480, before audio-device
//! allocation. Resolving a request consumes its RNG even without a mixer.

use v2k_formats::anim_sound::{SoundPool, SoundResolutionError};

/// Native one-shot wrapper versus a retained 0044C830 logical loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPlaybackMode {
    OneShot,
    Looping,
}

/// One native sound-wrapper request before Section-11 alias modulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundPlaybackRequest {
    pub global_sound_id: usize,
    pub volume_q16: u32,
    pub frequency_q16: u32,
    /// DirectSound pan in hundredths of a decibel, -10000 through 10000.
    pub pan: i32,
    pub mode: SoundPlaybackMode,
}

impl SoundPlaybackRequest {
    /// FUN_0042A860's full-volume, native-rate, centered one-shot parameters.
    pub const fn centered(global_sound_id: usize) -> Self {
        Self {
            global_sound_id,
            volume_q16: 0x10000,
            frequency_q16: 0x10000,
            pan: 0,
            mode: SoundPlaybackMode::OneShot,
        }
    }
}

/// Completed alias modulation. Submitting this value never samples RNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSoundPlayback {
    pub pcm_global_id: usize,
    pub volume_q16: u32,
    pub frequency_q16: u32,
    pub pan: i32,
    pub mode: SoundPlaybackMode,
}

impl ResolvedSoundPlayback {
    /// FUN_004957C0 multiplies in 32 bits before shifting; 004AD0A0 then
    /// clamps the resulting unsigned Hz value for DirectSound.
    pub fn frequency_hz(self) -> u32 {
        (self.frequency_q16.wrapping_mul(22_050) >> 16).clamp(100, 100_000)
    }
}

/// Resolve a validated global alias chain in requested-to-target order.
///
/// Every alias consumes one word, including zero variance or volume. This
/// phase precedes PCM creation and voice allocation in 00495480; callers must
/// also run it when sound is muted or no audio device is available. Invalid
/// asset chains fail before consuming RNG instead of entering native invalid
/// pointer/recursive-cycle behavior.
pub fn resolve_sound_playback(
    pool: &SoundPool<'_>,
    request: SoundPlaybackRequest,
    next_random: &mut impl FnMut() -> u16,
) -> Result<ResolvedSoundPlayback, SoundResolutionError> {
    let resolved = pool.resolve(request.global_sound_id)?;
    let mut frequency = request.frequency_q16;
    let mut volume = request.volume_q16;
    for hop in resolved.alias_hops {
        frequency = multiply_frequency_q16(frequency, hop.frequency_multiplier_16_16);
        let amplitude = multiply_frequency_q16(frequency, hop.frequency_variance_16_16);
        // 4954EC..495508: the product wraps before an arithmetic shift.
        // The computed perturbation's parity chooses its sign, not the RNG
        // word's parity and not an independent symmetric random float.
        let delta = (u32::from(next_random()).wrapping_mul(amplitude) as i32) >> 16;
        frequency = if delta & 1 == 0 {
            frequency.wrapping_sub(delta as u32)
        } else {
            frequency.wrapping_add(delta as u32)
        };
        if hop.volume_multiplier_16_16 != 0x10000 {
            volume = multiply_volume_q16(volume, hop.volume_multiplier_16_16);
        }
    }
    Ok(ResolvedSoundPlayback {
        pcm_global_id: resolved.pcm_global_id,
        frequency_q16: frequency,
        volume_q16: volume,
        pan: request.pan,
        mode: request.mode,
    })
}

/// FUN_00495670's unsigned split multiply, retaining the low output word.
fn multiply_frequency_q16(value: u32, multiplier: u32) -> u32 {
    ((u64::from(value) * u64::from(multiplier)) >> 16) as u32
}

/// FUN_004956B0 is distinct from frequency multiplication. Executable
/// 4956DA really masks the low *byte* when value >= 0x10000.
fn multiply_volume_q16(value: u32, multiplier: u32) -> u32 {
    let fraction = multiplier & 0xFFFF;
    let integer = (multiplier >> 16).wrapping_mul(value);
    if value < 0x10000 {
        integer.wrapping_add(fraction.wrapping_mul(value) >> 16)
    } else {
        integer
            .wrapping_add((value >> 16).wrapping_mul(fraction))
            .wrapping_add((value & 0xFF).wrapping_mul(fraction) >> 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_sound::{AnimSoundEntry, AnimSoundTable, EntryType};

    fn sound_table(aliases: &[(u32, u32, u32, u32)]) -> AnimSoundTable {
        // The terminal PCM has no bytes: resolution still owns all alias
        // draws even when the eventual mixer cannot create a voice.
        let mut table = v2k_formats::anim_sound::parse_anim_sound(&[0; 4]).unwrap();
        table.entries.push(AnimSoundEntry {
            index: 0,
            entry_type: EntryType::DataBlob,
            raw_type: 1,
            offset_or_index: 0,
            size_or_scale: 0,
            param1: 0,
            param2: 0,
        });
        for &(target, frequency, variance, volume) in aliases {
            table.entries.push(AnimSoundEntry {
                index: table.entries.len(),
                entry_type: EntryType::Alias,
                raw_type: 5,
                offset_or_index: target,
                size_or_scale: variance,
                param1: frequency,
                param2: volume,
            });
        }
        table
    }

    #[test]
    fn zero_variance_alias_still_draws_while_direct_pcm_does_not() {
        let table = sound_table(&[(0, 0x10000, 0, 0x10000)]);
        let pool = SoundPool::from_tables(&[&table]);
        let mut words = [0xFFFF].into_iter();
        let alias = resolve_sound_playback(&pool, SoundPlaybackRequest::centered(1), &mut || {
            words.next().expect("one alias word")
        })
        .unwrap();
        assert_eq!(words.next(), None);
        let direct = resolve_sound_playback(&pool, SoundPlaybackRequest::centered(0), &mut || {
            panic!("PCM resolution does not draw")
        })
        .unwrap();
        assert_eq!(alias, direct);
        assert_eq!(direct.frequency_hz(), 22_050);
    }

    #[test]
    fn hops_apply_modulation_before_the_next_hops_multiplier() {
        let table = sound_table(&[(0, 0x18000, 0x10000, 0x8000), (1, 0x8000, 0x8000, 0xC000)]);
        let pool = SoundPool::from_tables(&[&table]);
        let mut words = [0x8000, 3].into_iter();
        let resolved =
            resolve_sound_playback(&pool, SoundPlaybackRequest::centered(2), &mut || {
                words.next().expect("two aliases")
            })
            .unwrap();
        assert_eq!(words.next(), None);
        assert_eq!(resolved.frequency_q16, 36_865);
        assert_eq!(resolved.volume_q16, 24_576);
        assert_eq!(resolved.pcm_global_id, 0);
    }

    #[test]
    fn pitch_sign_uses_perturbation_parity_and_wrapped_signed_product() {
        let table = sound_table(&[(0, 0x10000, 0x8000, 0x10000)]);
        let pool = SoundPool::from_tables(&[&table]);
        for (word, frequency) in [(2, 65_537), (4, 65_534), (0xFFFF, 98_303)] {
            let resolved =
                resolve_sound_playback(&pool, SoundPlaybackRequest::centered(1), &mut || word)
                    .unwrap();
            assert_eq!(resolved.frequency_q16, frequency);
        }
        let table = sound_table(&[(0, 0x10000, 0x10000, 0x10000)]);
        let pool = SoundPool::from_tables(&[&table]);
        let resolved =
            resolve_sound_playback(&pool, SoundPlaybackRequest::centered(1), &mut || 0xFFFF)
                .unwrap();
        assert_eq!(
            resolved.frequency_q16, 65_535,
            "signed delta is -1, not +65535"
        );
    }

    #[test]
    fn volume_uses_the_executable_low_byte_branch_and_unity_bypass() {
        assert_eq!(multiply_volume_q16(0xFFFF, 0x8000), 0x7FFF);
        assert_eq!(multiply_volume_q16(0x10000, 0x8000), 0x8000);
        assert_eq!(multiply_volume_q16(0x18080, 0x8000), 0x8040);
        let table = sound_table(&[(0, 0x10000, 0, 0x10000)]);
        let pool = SoundPool::from_tables(&[&table]);
        let request = SoundPlaybackRequest {
            volume_q16: 0x18080,
            pan: -1234,
            ..SoundPlaybackRequest::centered(1)
        };
        let resolved = resolve_sound_playback(&pool, request, &mut || 0).unwrap();
        assert_eq!(resolved.volume_q16, 0x18080);
        assert_eq!(resolved.pan, -1234);
    }

    #[test]
    fn final_frequency_keeps_wrapping_multiply_before_unsigned_shift_and_clamp() {
        let playback = ResolvedSoundPlayback {
            pcm_global_id: 0,
            volume_q16: 0x10000,
            frequency_q16: 0,
            pan: 0,
            mode: SoundPlaybackMode::OneShot,
        };
        assert_eq!(playback.frequency_hz(), 100);
        assert_eq!(
            ResolvedSoundPlayback {
                frequency_q16: 0x40000,
                ..playback
            }
            .frequency_hz(),
            22_664
        );
        assert_eq!(
            ResolvedSoundPlayback {
                frequency_q16: u32::MAX,
                ..playback
            }
            .frequency_hz(),
            65_535
        );
    }

    #[test]
    fn malformed_asset_chain_fails_before_rng() {
        let table = sound_table(&[(1, 0x10000, 0, 0x10000)]);
        let pool = SoundPool::from_tables(&[&table]);
        assert!(matches!(
            resolve_sound_playback(&pool, SoundPlaybackRequest::centered(1), &mut || panic!(
                "invalid chain"
            )),
            Err(SoundResolutionError::AliasCycle { .. })
        ));
    }
}
