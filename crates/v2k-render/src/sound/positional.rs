//! FUN_0044C970's logical sound admission, before warble and alias RNG.

use super::{resolve_sound_playback, ResolvedSoundPlayback, SoundPlaybackRequest};
use v2k_formats::{
    anim_sound::{SoundPool, SoundResolutionError},
    fixed_math::retail_integer_sqrt,
};

/// Original listener origin and the three Q31 world-to-view rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionalSoundListener {
    pub position_raw: [i16; 3],
    pub world_to_view_q31: [[i32; 3]; 3],
}

/// One logical positional sound before listener-dependent admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionalSoundRequest {
    pub playback: SoundPlaybackRequest,
    pub position_raw: [i16; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionalSoundMix {
    pub volume_q16: u32,
    pub pan: i32,
    pub distance_raw: u32,
}

fn q31_multiply(value: i32, coefficient: i32) -> i32 {
    ((i64::from(value) * i64::from(coefficient)) >> 31) as i32
}

/// Native signed-word wrap, Q31 view transform, distance, volume and pan.
///
/// A zero result skips both the logical warble callback and alias resolution.
/// The 0x2000 per-axis precheck occurs before transforming the delta, and the
/// transformed integer length must be strictly below that same radius.
pub fn positional_sound_mix(
    listener: PositionalSoundListener,
    position_raw: [i16; 3],
    volume_q16: u32,
) -> Option<PositionalSoundMix> {
    let delta: [i32; 3] = std::array::from_fn(|axis| {
        i32::from(position_raw[axis].wrapping_sub(listener.position_raw[axis]))
    });
    if volume_q16 == 0 || delta.iter().any(|value| value.abs() >= 0x2000) {
        return None;
    }
    let view: [i32; 3] = listener.world_to_view_q31.map(|row| {
        row.into_iter()
            .zip(delta)
            .fold(0i32, |sum, (coefficient, value)| {
                sum.wrapping_add(q31_multiply(value, coefficient))
            })
    });
    let distance_raw = retail_integer_sqrt(view.into_iter().fold(0i32, |sum, value| {
        sum.wrapping_add(value.wrapping_mul(value))
    }));
    if distance_raw >= 0x2000 {
        return None;
    }
    let pan = if view[0].abs() > 0x1000 {
        if view[0] < 0 {
            -10_000
        } else {
            10_000
        }
    } else {
        // 457680's nonnegative ratio has denominator 0x2000 here. Equality
        // saturates at MAX, so the positive endpoint maps to 9999, not 10000.
        let numerator = view[0] + 0x1000;
        let ratio = if numerator >= 0x2000 {
            i32::MAX
        } else {
            numerator << 18
        };
        q31_multiply(20_000, ratio) - 10_000
    };
    let volume_q16 = if distance_raw < 0x101 {
        volume_q16
    } else {
        let fade_q31 = ((0x2000 - distance_raw) << 18) as i32;
        q31_multiply(volume_q16 as i32, fade_q31) as u32
    };
    (volume_q16 != 0).then_some(PositionalSoundMix {
        volume_q16,
        pan,
        distance_raw,
    })
}

/// Resolve audible one-shot or loop creation at the logical 0044C970 phase.
/// Inaudible requests consume no random words, even if their aliases vary.
pub fn resolve_positional_sound(
    pool: &SoundPool<'_>,
    listener: PositionalSoundListener,
    request: PositionalSoundRequest,
    next_random: &mut impl FnMut() -> u16,
) -> Result<Option<ResolvedSoundPlayback>, SoundResolutionError> {
    let Some(mix) =
        positional_sound_mix(listener, request.position_raw, request.playback.volume_q16)
    else {
        return Ok(None);
    };
    resolve_sound_playback(
        pool,
        SoundPlaybackRequest {
            volume_q16: mix.volume_q16,
            pan: mix.pan,
            ..request.playback
        },
        next_random,
    )
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::anim_sound::{AnimSoundEntry, EntryType};

    fn listener() -> PositionalSoundListener {
        PositionalSoundListener {
            position_raw: [0; 3],
            world_to_view_q31: [[i32::MIN, 0, 0], [0, i32::MIN, 0], [0, 0, i32::MIN]],
        }
    }

    #[test]
    fn integer_admission_preserves_full_volume_threshold_and_both_radius_checks() {
        assert_eq!(
            positional_sound_mix(listener(), [256, 0, 0], 0x10000)
                .unwrap()
                .volume_q16,
            0x10000
        );
        assert_eq!(
            positional_sound_mix(listener(), [257, 0, 0], 0x10000)
                .unwrap()
                .volume_q16,
            63_480
        );
        assert_eq!(
            positional_sound_mix(listener(), [8191, 0, 0], 0x10000)
                .unwrap()
                .volume_q16,
            8
        );
        assert_eq!(
            positional_sound_mix(listener(), [8192, 0, 0], 0x10000),
            None
        );
        assert_eq!(
            positional_sound_mix(listener(), [-8192, 0, 0], 0x10000),
            None
        );
        assert_eq!(
            positional_sound_mix(listener(), [6000, 6000, 0], 0x10000),
            None
        );
        assert_eq!(positional_sound_mix(listener(), [0; 3], 0), None);
        assert_eq!(positional_sound_mix(listener(), [8191, 0, 0], 1), None);
    }

    #[test]
    fn q31_narrowing_and_pan_endpoint_asymmetry_remain_observable() {
        assert_eq!(
            positional_sound_mix(listener(), [-4096, 0, 0], 0x10000)
                .unwrap()
                .pan,
            9999
        );
        assert_eq!(
            positional_sound_mix(listener(), [4096, 0, 0], 0x10000)
                .unwrap()
                .pan,
            -10000
        );
        assert_eq!(
            positional_sound_mix(listener(), [-4097, 0, 0], 0x10000)
                .unwrap()
                .pan,
            10000
        );
        let positive = PositionalSoundListener {
            world_to_view_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
            ..listener()
        };
        let mix = positional_sound_mix(positive, [256, 0, 0], 0x10000).unwrap();
        assert_eq!((mix.distance_raw, mix.pan), (255, 622));
    }

    #[test]
    fn signed_word_seam_and_all_three_view_rows_control_the_gate() {
        let listener = PositionalSoundListener {
            position_raw: [32760, 0, 0],
            ..listener()
        };
        assert_eq!(
            positional_sound_mix(listener, [-32760, 0, 0], 0x10000)
                .unwrap()
                .distance_raw,
            16
        );
        let rotated = PositionalSoundListener {
            position_raw: [0; 3],
            world_to_view_q31: [[0, 0, i32::MIN], [0, i32::MIN, 0], [i32::MIN, 0, 0]],
        };
        let mix = positional_sound_mix(rotated, [0, 0, -4096], 0x10000).unwrap();
        assert_eq!((mix.distance_raw, mix.pan), (4096, 9999));
    }

    #[test]
    fn only_audible_logical_requests_resolve_aliases_even_without_a_device() {
        let mut table = v2k_formats::anim_sound::parse_anim_sound(&[0; 4]).unwrap();
        table.entries = vec![
            AnimSoundEntry {
                index: 0,
                entry_type: EntryType::DataBlob,
                raw_type: 1,
                offset_or_index: 0,
                size_or_scale: 0,
                param1: 0,
                param2: 0,
            },
            AnimSoundEntry {
                index: 1,
                entry_type: EntryType::Alias,
                raw_type: 5,
                offset_or_index: 0,
                size_or_scale: 0,
                param1: 0x8000,
                param2: 0x8000,
            },
        ];
        let pool = SoundPool::from_tables(&[&table]);
        let request = PositionalSoundRequest {
            playback: SoundPlaybackRequest {
                frequency_q16: 0x18000,
                ..SoundPlaybackRequest::centered(1)
            },
            position_raw: [4096, 0, 0],
        };
        let mut draws = 0;
        let playback = resolve_positional_sound(&pool, listener(), request, &mut || {
            draws += 1;
            0xFFFF
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            draws, 1,
            "an audible zero-variance alias still draws with empty PCM"
        );
        assert_eq!(playback.pcm_global_id, 0);
        assert_eq!(playback.frequency_q16, 0xC000);
        assert_eq!(
            playback.volume_q16, 0x4000,
            "distance gain precedes alias volume"
        );
        assert_eq!(playback.pan, -10_000);
        for position_raw in [[8192, 0, 0], [6000, 6000, 0]] {
            assert_eq!(
                resolve_positional_sound(
                    &pool,
                    listener(),
                    PositionalSoundRequest {
                        position_raw,
                        ..request
                    },
                    &mut || panic!("inaudible one-shots never resolve aliases")
                )
                .unwrap(),
                None
            );
        }
        assert_eq!(
            resolve_positional_sound(
                &pool,
                listener(),
                PositionalSoundRequest {
                    playback: SoundPlaybackRequest {
                        volume_q16: 0,
                        ..request.playback
                    },
                    ..request
                },
                &mut || panic!("zero-volume requests never resolve aliases")
            )
            .unwrap(),
            None
        );
    }
}
