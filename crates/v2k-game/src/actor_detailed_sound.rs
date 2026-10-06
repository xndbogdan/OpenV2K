//! FUN_0040DCA0's post-task sound gate, before E640/F70/E100.

use v2k_formats::collision::CollisionEntry;

pub struct ActorDetailedSoundFrame<'a> {
    pub type_record: &'a CollisionEntry,
    pub health_raw: i32,
    pub visible: bool,
    pub callback_elapsed_micros: u32,
}

/// The sound fields read by DCA0, retained independently from optional actor
/// components so the Hive's shared component pass can use the same policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorDetailedSoundPolicy {
    pub full_health_raw: i32,
    pub healthy_sounds: [u16; 2],
    pub low_health_sound: u16,
    /// Signed Section-12 +A4 (healthy) and +B0 (low-health) periods.
    pub periods_raw: [i32; 2],
}

impl ActorDetailedSoundPolicy {
    pub fn from_section12(record: &CollisionEntry) -> Self {
        let header = &record.raw_header;
        let word = |offset| u16::from_le_bytes([header[offset], header[offset + 1]]);
        let dword = |offset| i32::from_le_bytes(header[offset..offset + 4].try_into().unwrap());
        Self {
            full_health_raw: dword(0x14),
            healthy_sounds: [word(0x94), word(0x96)],
            low_health_sound: word(0xa0),
            periods_raw: [dword(0xa4), dword(0xb0)],
        }
    }

    pub fn plan(
        self,
        health_raw: i32,
        visible: bool,
        callback_elapsed_micros: u32,
        next_random: &mut impl FnMut() -> u32,
    ) -> Option<u16> {
        let (sound, alternate, period) = if health_raw < self.full_health_raw / 2 {
            (self.low_health_sound, 0, self.periods_raw[1])
        } else if visible {
            (
                self.healthy_sounds[0],
                self.healthy_sounds[1],
                self.periods_raw[0],
            )
        } else {
            return None;
        };
        if sound == 0 {
            return None;
        }
        // x86 signed cmp of period & FFFFFC00, then arithmetic shift; the
        // unsigned dt<<6 division and inclusive chance comparison are separate.
        let divisor = if period & !0x3ff < 0x400 {
            1
        } else {
            (period >> 10) as u32
        };
        let threshold = callback_elapsed_micros.wrapping_shl(6) / divisor;
        if threshold < u32::from(next_random() as u16) {
            return None;
        }
        if alternate != 0 && next_random() as u16 & 1 != 0 {
            Some(alternate)
        } else {
            Some(sound)
        }
    }
}

/// Return the selected fixed-volume, fixed-frequency 44F480 request. Sound
/// alias resolution and mixer admission are later phases and own their RNG.
pub fn plan_actor_detailed_sound(
    frame: ActorDetailedSoundFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Option<u16> {
    ActorDetailedSoundPolicy::from_section12(frame.type_record).plan(
        frame.health_raw,
        frame.visible,
        frame.callback_elapsed_micros,
        next_random,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detailed_gate_is_signed_inclusive_and_owns_only_selected_draws() {
        let mut record = CollisionEntry {
            index: 0,
            offset: 0,
            size: 0x128,
            type_tag: 0,
            scale: 0,
            id_field: 0,
            model_ids: [0; 4],
            bounds: [0; 2],
            dims: [0; 6],
            initializer_state_flags_raw: 0,
            flags: 0,
            faces: vec![],
            behavior_choices: vec![],
            active_subs: vec![],
            raw_header: [0; 0x128],
            subsections: vec![],
            behavior_rule_ref: 0,
            alternate_behavior_class_ref: 0,
            sub_a_pos: None,
        };
        record.raw_header[0x14..0x18].copy_from_slice(&100i32.to_le_bytes());
        record.raw_header[0x94..0x96].copy_from_slice(&7u16.to_le_bytes());
        record.raw_header[0x96..0x98].copy_from_slice(&8u16.to_le_bytes());
        record.raw_header[0xa4..0xa8].copy_from_slice(&1024i32.to_le_bytes());
        let mut draws = [64, 1].into_iter();
        assert_eq!(
            plan_actor_detailed_sound(
                ActorDetailedSoundFrame {
                    type_record: &record,
                    health_raw: 50,
                    visible: true,
                    callback_elapsed_micros: 1
                },
                &mut || draws.next().unwrap()
            ),
            Some(8)
        );
        assert_eq!(draws.next(), None);
        record.raw_header[0x96..0x98].copy_from_slice(&0u16.to_le_bytes());
        record.raw_header[0xa4..0xa8].copy_from_slice(&(-1024i32).to_le_bytes());
        let mut signed_draws = [1].into_iter();
        assert_eq!(
            plan_actor_detailed_sound(
                ActorDetailedSoundFrame {
                    type_record: &record,
                    health_raw: 50,
                    visible: true,
                    callback_elapsed_micros: 1,
                },
                &mut || signed_draws.next().unwrap(),
            ),
            Some(7),
            "a negative signed period takes divisor one before unsigned chance math"
        );
        assert_eq!(signed_draws.next(), None);
        record.raw_header[0xa4..0xa8].copy_from_slice(&2048i32.to_le_bytes());
        let mut rejected_draws = [33].into_iter();
        assert_eq!(
            plan_actor_detailed_sound(
                ActorDetailedSoundFrame {
                    type_record: &record,
                    health_raw: 50,
                    visible: true,
                    callback_elapsed_micros: 1,
                },
                &mut || rejected_draws.next().unwrap(),
            ),
            None,
            "divisor two gives chance 32, so word 33 rejects without a pitch draw"
        );
        assert_eq!(rejected_draws.next(), None);
        assert_eq!(
            plan_actor_detailed_sound(
                ActorDetailedSoundFrame {
                    type_record: &record,
                    health_raw: -1,
                    visible: true,
                    callback_elapsed_micros: 1
                },
                &mut || panic!("zero low-health sound must consume nothing")
            ),
            None
        );
        assert_eq!(
            plan_actor_detailed_sound(
                ActorDetailedSoundFrame {
                    type_record: &record,
                    health_raw: 100,
                    visible: false,
                    callback_elapsed_micros: 1
                },
                &mut || panic!("invisible healthy actor must consume nothing")
            ),
            None
        );
    }
}
