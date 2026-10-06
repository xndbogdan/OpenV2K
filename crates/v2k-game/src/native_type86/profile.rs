//! Constructor identities sharing the class45/10/54/6/14 task graph.
//!
//! Type7 is a factory worker, not a Main Base person. Its rule13, capability,
//! query-free steering, damage and audio remain its own Section12 policy.

use v2k_formats::collision::SubDSteeringDescriptor;

use super::{NativePersonProfile, Type86Block};
use crate::{
    common_mover::sub_d::{ORDINARY_TYPE7_SUB_D, ORDINARY_TYPE90_SUB_D},
    damage::DamageProfile,
    entity_collision_state::EntityTypeRuntimeMetadata,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeFourChoiceProfile {
    Person(NativePersonProfile),
    DiverWorker,
}

impl NativeFourChoiceProfile {
    pub(crate) const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            7 => Some(Self::DiverWorker),
            _ => match NativePersonProfile::from_entity_type(entity_type) {
                Some(profile) => Some(Self::Person(profile)),
                None => None,
            },
        }
    }

    pub(crate) const fn entity_type(self) -> u32 {
        match self {
            Self::Person(profile) => profile.entity_type(),
            Self::DiverWorker => 7,
        }
    }

    pub(crate) const fn model_id(self) -> usize {
        match self {
            Self::Person(profile) => profile.model_id(),
            Self::DiverWorker => 1249,
        }
    }

    pub(crate) const fn capability(self) -> u32 {
        match self {
            Self::Person(_) => super::CAPABILITY,
            Self::DiverWorker => 0x1404,
        }
    }

    pub(super) const fn axis(self) -> (i32, u32) {
        match self {
            Self::Person(profile) => profile.axis(),
            Self::DiverWorker => (3072, 0xa1),
        }
    }

    pub(super) const fn choices(self) -> [(u32, u32, u32); 4] {
        match self {
            Self::Person(profile) => profile.choices(),
            Self::DiverWorker => [(1, 1, 6), (13, 200, 54), (6, 20, 45), (7, 10, 10)],
        }
    }

    pub(super) const fn sub_d(self) -> SubDSteeringDescriptor {
        match self {
            Self::Person(_) => ORDINARY_TYPE90_SUB_D,
            Self::DiverWorker => ORDINARY_TYPE7_SUB_D,
        }
    }

    pub(super) const fn animation_cues(self) -> (u16, u16, u16, u8, u8) {
        match self {
            Self::Person(_) => (85, 0, super::ATTENTION_STOP_SOUND, 1, 4),
            Self::DiverWorker => (0, 0, 106, 1, 4),
        }
    }

    pub(super) const fn death_sound(self) -> u16 {
        match self {
            Self::Person(profile) => profile.death_sound(),
            Self::DiverWorker => 35,
        }
    }

    pub(super) const fn accepted_hit_sound(self) -> Option<u16> {
        match self {
            Self::Person(_) => Some(super::ACCEPTED_HIT_SOUND),
            Self::DiverWorker => None,
        }
    }

    pub(super) const fn surface_selectors(self) -> [u8; 2] {
        match self {
            Self::Person(_) => [1, 0],
            Self::DiverWorker => [0, 0],
        }
    }

    pub(super) const fn surface_lifetime_ms(self) -> u32 {
        match self {
            Self::Person(profile) => profile.surface_lifetime_ms(),
            Self::DiverWorker => 0,
        }
    }

    pub(super) const fn effect_words(self) -> [u16; 3] {
        match self {
            Self::Person(_) => [0; 3],
            Self::DiverWorker => [40, 0, 0],
        }
    }

    pub(super) const fn damage_profile(self) -> DamageProfile {
        match self {
            Self::Person(_) => DamageProfile {
                thresholds_raw: [0, 2000, 400, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 512],
            },
            Self::DiverWorker => DamageProfile {
                thresholds_raw: [0, 4000, 200, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 256],
            },
        }
    }
}

pub(crate) fn validate_four_choice_metadata(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type86Block> {
    let profile = NativeFourChoiceProfile::from_entity_type(entity_type)
        .ok_or(Type86Block::Runtime("four-choice profile"))?;
    super::birth::validate_metadata(profile, metadata)
}
