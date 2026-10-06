//! Canonical worker records selecting the shared class6/54/14 ABDI graph.
//!
//! Section12 keeps the allocation identity, model, Sub-I cues and world effects
//! distinct even where the class styles and 20450/06070 constructors coincide.
//! Type7 is deliberately absent: its D probes, four-choice root and effects
//! require an independently complete owner.

use super::Intro2Type8Block;
use crate::{
    common_mover::{
        actor_abdi::ActorAbdiTopology,
        sub_d::{ORDINARY_TYPE90_SUB_D, ORDINARY_TYPE9_SUB_D},
    },
    entity_collision_state::{
        CommonWorldEffectProfile, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
};
use v2k_formats::collision::{ActorAnimationDescriptor, SubDSteeringDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeWorkerProfile {
    Type8,
    Type79,
    Type90,
    Type91,
    Type116,
}

impl NativeWorkerProfile {
    pub(crate) const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            8 => Some(Self::Type8),
            79 => Some(Self::Type79),
            90 => Some(Self::Type90),
            91 => Some(Self::Type91),
            116 => Some(Self::Type116),
            _ => None,
        }
    }

    pub(crate) const fn entity_type(self) -> u32 {
        match self {
            Self::Type8 => 8,
            Self::Type79 => 79,
            Self::Type90 => 90,
            Self::Type91 => 91,
            Self::Type116 => 116,
        }
    }

    pub(crate) const fn model_id(self) -> usize {
        match self {
            Self::Type8 => 559,
            Self::Type79 => 979,
            Self::Type90 => 890,
            Self::Type91 => 662,
            Self::Type116 => 1136,
        }
    }

    pub(super) const fn health(self) -> i32 {
        match self {
            Self::Type8 | Self::Type116 => 1500,
            Self::Type79 | Self::Type90 | Self::Type91 => 2000,
        }
    }

    pub(super) const fn animation_descriptor(self) -> ActorAnimationDescriptor {
        let (capability_bit_3_sound_id, attention_stop_sound_id) = match self {
            Self::Type8 | Self::Type91 => (0, 0),
            Self::Type79 => (72, 72),
            Self::Type90 => (85, 0),
            Self::Type116 => (0, 72),
        };
        ActorAnimationDescriptor {
            capability_bit_3_sound_id,
            capability_mask_0x201_sound_id: 0,
            attention_stop_sound_id,
            variable_binding: 1,
            frames_per_direction: 4,
        }
    }

    pub(super) const fn axis(self) -> (i32, u32) {
        match self {
            Self::Type8 | Self::Type116 => (0xf00, 0x84),
            Self::Type79 | Self::Type90 => (0xa00, 0),
            Self::Type91 => (0xc00, 0),
        }
    }

    pub(super) const fn choices(self) -> &'static [(u32, u32, u32)] {
        match self {
            Self::Type8 | Self::Type116 => &[(13, 100, 54), (1, 1, 6)],
            Self::Type79 | Self::Type90 | Self::Type91 => &[(1, 1, 6), (13, 200, 54)],
        }
    }

    pub(super) const fn sub_d(self) -> SubDSteeringDescriptor {
        match self {
            Self::Type8 | Self::Type116 => ORDINARY_TYPE9_SUB_D,
            Self::Type79 | Self::Type90 | Self::Type91 => ORDINARY_TYPE90_SUB_D,
        }
    }

    const fn hit_sound(self) -> Option<u16> {
        match self {
            Self::Type90 => Some(74),
            _ => None,
        }
    }

    const fn death_sound(self) -> Option<u16> {
        match self {
            Self::Type90 => Some(35),
            _ => None,
        }
    }

    const fn world_effects(self) -> CommonWorldEffectProfile {
        let (surface_selectors, surface_lifetime_ms) = match self {
            Self::Type79 => ([0, 0], 0),
            _ => ([1, 0], 5000),
        };
        CommonWorldEffectProfile {
            surface_selectors,
            surface_lifetime_ms,
            low_health_effect_words: [0; 3],
        }
    }
}

pub(crate) fn validate_worker_metadata(
    entity_type: u32,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type8Block> {
    let profile = NativeWorkerProfile::from_entity_type(entity_type)
        .ok_or(Intro2Type8Block::Runtime("worker entity type"))?;
    let Some(init) = metadata.initializer.as_ref() else {
        return Err(Intro2Type8Block::Runtime("initializer"));
    };
    let axis = profile.axis();
    if metadata.model_slots != [profile.model_id() as u16; 4]
        || metadata.actor_animation_descriptor
            != RetailRuntimeValue::Known(Some(profile.animation_descriptor()))
        || metadata.capability_flags != 0x1404
        || metadata.mass_raw != 10
        || metadata.initial_health_raw != Some(profile.health())
        || init.initializer_state_flags_raw != 0x2f
        || init.behavior_rule_ref != 1
        || init.alternate_behavior_class_ref != 14
        || init.common_axis_descriptor.strict_axis_limit_raw != axis.0
        || init.common_axis_descriptor.raw_word_at_0x04 != axis.1
        || !init
            .behavior_choices
            .iter()
            .map(|c| (c.weight_rule_id, c.weight_multiplier, c.behavior_class_id))
            .eq(profile.choices().iter().copied())
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(profile.death_sound())
        || metadata.accepted_hit_presentation_sound_id
            != RetailRuntimeValue::Known(profile.hit_sound())
        || metadata.common_world_effects != RetailRuntimeValue::Known(profile.world_effects())
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(profile.sub_d()))
        || ActorAbdiTopology::from_metadata(entity_type as u16, metadata).is_err()
    {
        return Err(Intro2Type8Block::Runtime("canonical worker metadata"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        damage::DamageProfile,
        intro2_type8::{
            authored_tests, intro2_type8_allocation_authenticates,
            intro2_type8_manager_allocation_authenticates, Intro2Type8Owner,
        },
        session::GameSession,
        world_fx::WorldFx,
    };

    fn canonical_session() -> Option<GameSession> {
        let data = v2k_test_support::retail_dir();
        let mut session = GameSession::init(&data).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        Some(session)
    }

    #[v2k_test_support::retail_test]
    fn worker_profiles_preserve_all_five_canonical_records_and_exclude_type7() {
        let Some(session) = canonical_session() else {
            return;
        };
        for (entity_type, model, health, axis, cues, surface) in [
            (8, 559, 1500, (3840, 0x84), (0, 0, 0), ([1, 0], 5000)),
            (79, 979, 2000, (2560, 0), (72, 0, 72), ([0, 0], 0)),
            (90, 890, 2000, (2560, 0), (85, 0, 0), ([1, 0], 5000)),
            (91, 662, 2000, (3072, 0), (0, 0, 0), ([1, 0], 5000)),
            (116, 1136, 1500, (3840, 0x84), (0, 0, 72), ([1, 0], 5000)),
        ] {
            let metadata = EntityTypeRuntimeMetadata::from_section12(
                session
                    .cache
                    .global_entity_type(entity_type as usize)
                    .unwrap(),
            );
            validate_worker_metadata(entity_type, &metadata).unwrap();
            assert_eq!(metadata.model_slots, [model; 4]);
            assert_eq!(metadata.initial_health_raw, Some(health));
            let init = metadata.initializer.as_ref().unwrap();
            assert_eq!(
                (
                    init.common_axis_descriptor.strict_axis_limit_raw,
                    init.common_axis_descriptor.raw_word_at_0x04
                ),
                axis
            );
            let RetailRuntimeValue::Known(Some(animation)) = metadata.actor_animation_descriptor
            else {
                panic!("canonical Sub-I");
            };
            assert_eq!(
                (
                    animation.capability_bit_3_sound_id,
                    animation.capability_mask_0x201_sound_id,
                    animation.attention_stop_sound_id
                ),
                cues
            );
            assert_eq!(
                metadata.common_world_effects,
                RetailRuntimeValue::Known(CommonWorldEffectProfile {
                    surface_selectors: surface.0,
                    surface_lifetime_ms: surface.1,
                    low_health_effect_words: [0; 3],
                })
            );
            assert_eq!(
                metadata.damage_profile,
                Some(DamageProfile {
                    thresholds_raw: [0, 2000, 200, 0, 200, 0, 0],
                    multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
                })
            );
        }
        let type7 =
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(7).unwrap());
        assert_eq!(NativeWorkerProfile::from_entity_type(7), None);
        assert!(validate_worker_metadata(7, &type7).is_err());
    }

    #[v2k_test_support::retail_test]
    fn worker79_and91_reject_another_profiles_cues_axis_surface_and_sub_d() {
        let Some(session) = canonical_session() else {
            return;
        };
        let metadata = |id| {
            EntityTypeRuntimeMetadata::from_section12(session.cache.global_entity_type(id).unwrap())
        };
        for entity_type in [79, 91] {
            let actual = metadata(entity_type as usize);
            for wrong in [8, 79, 90, 91, 116]
                .into_iter()
                .filter(|id| *id != entity_type)
            {
                assert!(validate_worker_metadata(wrong, &actual).is_err());
            }
            let mut changed = actual.clone();
            changed.actor_animation_descriptor = metadata(90).actor_animation_descriptor;
            assert!(validate_worker_metadata(entity_type, &changed).is_err());
            changed = actual.clone();
            changed
                .initializer
                .as_mut()
                .unwrap()
                .common_axis_descriptor
                .strict_axis_limit_raw += 1;
            assert!(validate_worker_metadata(entity_type, &changed).is_err());
            changed = actual.clone();
            changed.common_world_effects =
                metadata(if entity_type == 79 { 91 } else { 79 }).common_world_effects;
            assert!(validate_worker_metadata(entity_type, &changed).is_err());
            changed = actual.clone();
            changed.sub_d_steering_descriptor = metadata(8).sub_d_steering_descriptor;
            assert!(validate_worker_metadata(entity_type, &changed).is_err());
        }
    }

    #[v2k_test_support::retail_test]
    fn worker79_and91_allocation_receipts_reject_profile_and_model_retagging() {
        for (world, entity_type, replacement) in [
            (19, 79, NativeWorkerProfile::Type91),
            (40, 91, NativeWorkerProfile::Type79),
        ] {
            let Some((_session, mut manager)) = authored_tests::load(world, &mut WorldFx::new())
            else {
                return;
            };
            let id = manager
                .iter_all()
                .find(|entity| entity.entity_type == entity_type)
                .unwrap()
                .id;
            assert!(intro2_type8_manager_allocation_authenticates(&manager, id));
            let entity = manager.entity_mut(id).unwrap();
            let original = entity.intro2_type8_runtime.unwrap();
            let mut forged = original;
            forged.profile = replacement;
            assert!(!original.same_allocation(forged));
            entity.intro2_type8_runtime = Some(forged);
            assert!(!intro2_type8_allocation_authenticates(entity));
            assert!(Intro2Type8Owner::adopt_published(entity).is_none());
            entity.intro2_type8_runtime = Some(original);
            entity.entity_type = replacement.entity_type();
            entity.model_slots = [Some(replacement.model_id()); 4];
            entity.model_index = Some(replacement.model_id());
            assert!(!intro2_type8_allocation_authenticates(entity));
            assert!(Intro2Type8Owner::adopt_published(entity).is_none());
        }
    }
}
