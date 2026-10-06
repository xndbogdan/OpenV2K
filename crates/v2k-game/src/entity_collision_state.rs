//! Evidence-backed runtime state required by active-entity collision.
//!
//! Retail's pair pass (`FUN_00411AD0`) does not operate on entity type alone.
//! It reads the live state word at `+0x08`, the active model slot encoded by
//! bits `0x2000/0x4000`, the recent-relation fields at `+0x60/+0x68`, the
//! scheduler fields at `+0x6c/+0x70/+0xb6`, health at `+0x30`, the damage
//! buffer at `+0x50`, and the intrusive entity order. This module retains the
//! constructor-backed values we can prove; [`crate::entity_initializer`] owns
//! the masked initial state/behavior policy and [`crate::entity_scheduler`]
//! owns the exact common scheduler transition. None of these modules pretends
//! the remaining weighted evaluator or later type callbacks have run. The stable
//! `20260718-024518-pair-collision-boundary-*` capture proves the initial
//! type-46/6/66/68 values and their null component-contact callbacks, but it
//! does not turn those per-instance observations into a general callback or
//! mutable damage policy; the full first-world chain contains additional
//! pair-eligible types.
//!
//! Nothing here dispatches pair collision. In particular, an
//! [`Unresolved`](RetailRuntimeValue::Unresolved) value must never be replaced
//! with a type whitelist or a later snapshot. Masked state preserves only
//! constructor/initializer domains that are invariant across the remaining
//! evaluator and setup alternatives.

use crate::damage::DamageProfile;
use v2k_formats::collision::{
    ActorAnimationDescriptor, BehaviorChoice, CollisionEntry, CommonAxisDescriptor,
    ProjectileEmitterDescriptor, StatusComponentDescriptor, SubAPropulsionDescriptor,
    SubBLateralDescriptor, SubCLiftDescriptor, SubDSteeringDescriptor, SubFSwimmingDescriptor,
    SubHExternalFrameDescriptor, SubJAttachmentDescriptor,
};

/// The entity participates in the basic subject/candidate eligibility test.
pub const PAIR_COLLISION_ENABLED_STATE_BIT: u32 = 0x0000_8000;
/// Alternate live eligibility bit accepted by `FUN_00414AE0` even when the
/// ordinary pair-ineligible bit is set.
pub const RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT: u32 = 0x0000_0800;
/// `FUN_00413F70` has rebuilt the entity's complete signed-Q31 body basis.
pub const BODY_BASIS_REBUILT_STATE_BIT: u32 = 0x0000_0004;
/// `FUN_00415040` uses the same live bit as its checked-damage eligibility
/// gate. Keep the damage role explicit instead of coupling that path to pair
/// scan terminology.
pub const CHECKED_DAMAGE_ENABLED_STATE_BIT: u32 = PAIR_COLLISION_ENABLED_STATE_BIT;
/// This live state excludes an entity from both sides of the retail pair scan.
pub const PAIR_COLLISION_INELIGIBLE_STATE_BIT: u32 = 0x0000_1000;
/// `FUN_00416410` reads this same bit before an actor-task tagged or timeout
/// result may enter the behavior owner's transition callback.
pub const ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT: u32 = PAIR_COLLISION_INELIGIBLE_STATE_BIT;
/// Enables the oriented model-versus-terrain pass and the following whole-body
/// water classifier in `FUN_00411AD0`.
pub const TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT: u32 = 0x0001_0000;
/// Solid terrain response/grounded latch set by `FUN_004141D0`. It remains set
/// throughout the accepted type-46 survey; the common update can clear it when
/// the separate `0x0008_0000` drag-state bit is also active.
pub const TERRAIN_CONTACT_RESPONSE_STATE_BIT: u32 = 0x0080_0000;
/// Low bit of the four-way model-slot selector (`slot += 1`).
pub const ACTIVE_MODEL_SLOT_LOW_STATE_BIT: u32 = 0x0000_4000;
/// `FUN_00414E90` independently reads this same bit as its dying-state gate.
/// The semantic alias preserves that recovered role at damage call sites.
pub const DYING_STATE_BIT: u32 = ACTIVE_MODEL_SLOT_LOW_STATE_BIT;
/// High bit of the four-way model-slot selector (`slot += 2`).
pub const ACTIVE_MODEL_SLOT_HIGH_STATE_BIT: u32 = 0x0000_2000;
/// Exact state-word mask written by deferred-destroy helper `FUN_00410B70`.
///
/// The helper clears `0x0006_0000` and sets `0x0010_0000` before incrementing
/// the process-global deferred-removal count.
pub const DEFERRED_DESTROY_STATE_WRITE_MASK: u32 = 0x0016_0000;
pub const DEFERRED_DESTROY_PENDING_STATE_BIT: u32 = 0x0010_0000;
/// Entity's collision sphere is wholly below the current terrain/water surface.
pub const FULLY_BELOW_SURFACE_STATE_BIT: u32 = 0x0020_0000;
/// Entity's collision sphere is wholly above the current terrain/water surface.
pub const FULLY_ABOVE_SURFACE_STATE_BIT: u32 = 0x0040_0000;
/// Mutually exclusive surface-classification bits maintained by `FUN_004129B0`.
pub const SURFACE_STATE_MASK: u32 = FULLY_BELOW_SURFACE_STATE_BIT | FULLY_ABOVE_SURFACE_STATE_BIT;
/// `FUN_00412760` treats this entity as fixed during pair response.
pub const PAIR_COLLISION_FIXED_STATE_BIT: u32 = 0x0800_0000;
/// `FUN_00414E90` forwards damage for entities owned by another runtime
/// domain instead of applying the local buffer/health transaction.
pub const REMOTE_OWNED_STATE_BIT: u32 = 0x8000_0000;
/// Strict age window used by `FUN_00411A20` for recent-relation suppression.
pub const RECENT_RELATION_SUPPRESSION_WINDOW_US: u32 = 750_000;

/// A retail runtime value whose current value is either retained or explicitly
/// unavailable because an unported callback/lifecycle owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetailRuntimeValue<T> {
    Known(T),
    Unresolved,
}

impl<T> Default for RetailRuntimeValue<T> {
    fn default() -> Self {
        Self::Unresolved
    }
}

impl<T> RetailRuntimeValue<T> {
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> RetailRuntimeValue<U> {
        match self {
            Self::Known(value) => RetailRuntimeValue::Known(map(value)),
            Self::Unresolved => RetailRuntimeValue::Unresolved,
        }
    }
}

/// A retail bitfield whose independent bits can be known without pretending
/// the complete word is available.
///
/// Unknown bits are normalized to zero in `value`; callers must use
/// [`Self::masked`] before making a decision from any subset.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetailStateWord {
    value: u32,
    known_mask: u32,
}

impl RetailStateWord {
    pub const fn unknown() -> Self {
        Self {
            value: 0,
            known_mask: 0,
        }
    }

    pub const fn exact(value: u32) -> Self {
        Self {
            value,
            known_mask: u32::MAX,
        }
    }

    pub const fn from_known_bits(value: u32, known_mask: u32) -> Self {
        Self {
            value: value & known_mask,
            known_mask,
        }
    }

    /// Return the normalized value of known bits only.
    ///
    /// This is intended for diagnostics and whole-fixture assertions. Runtime
    /// decisions must use [`Self::masked`] so unknown zeroes are never treated
    /// as evidence.
    pub const fn known_value_bits(self) -> u32 {
        self.value
    }

    pub const fn known_mask(self) -> u32 {
        self.known_mask
    }

    /// Read one subset only when every requested bit is known.
    pub const fn masked(self, mask: u32) -> RetailRuntimeValue<u32> {
        if self.known_mask & mask == mask {
            RetailRuntimeValue::Known(self.value & mask)
        } else {
            RetailRuntimeValue::Unresolved
        }
    }

    /// Apply a known masked write and preserve every unrelated known bit.
    pub fn overwrite(&mut self, mask: u32, bits: u32) {
        self.value = (self.value & !mask) | (bits & mask);
        self.known_mask |= mask;
        self.value &= self.known_mask;
    }

    /// Apply retail `FUN_00410B70`'s deferred-destroy word.
    ///
    /// The helper clears `0x0006_0000` (callback-enable and master-motion) and
    /// sets `0x0010_0000`. Main Base conversion must preflight this exact write
    /// on the type-9 source; setting only the pending bit leaves the first-
    /// scheduler word `0x00468805` stale against the later queue.
    pub fn apply_deferred_destroy_pending_write(&mut self) {
        self.overwrite(
            DEFERRED_DESTROY_STATE_WRITE_MASK,
            DEFERRED_DESTROY_PENDING_STATE_BIT,
        );
    }

    /// Forget a subset without disturbing independent evidence.
    pub fn invalidate(&mut self, mask: u32) {
        self.known_mask &= !mask;
        self.value &= self.known_mask;
    }

    /// Retain only bits known with the same value in every possible outcome.
    pub fn merge_alternatives(alternatives: &[Self]) -> Self {
        let Some(first) = alternatives.first().copied() else {
            return Self::unknown();
        };
        let mut known_mask = first.known_mask;
        for alternative in &alternatives[1..] {
            known_mask &= alternative.known_mask;
            known_mask &= !(first.value ^ alternative.value);
        }
        Self::from_known_bits(first.value, known_mask)
    }
}

/// The Section-12 values copied or referenced by entity construction.
///
/// `None` for the health/profile fields means the caller did not supply an
/// exact type record. It does not mean retail authored a zero health or a null
/// damage profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityInitializerSpec {
    /// Authored type policy copied from Section 12 `+0xC0` to entity `+0xC8`.
    pub initializer_state_flags_raw: u32,
    /// Unconditional common descriptor copied from Section 12 `+0xC8/+0xCC`
    /// into the entity component. Rule 13 consumes its first dword.
    pub common_axis_descriptor: CommonAxisDescriptor,
    /// Terminated weighted behavior candidates at Section 12 `+0x118`.
    pub behavior_choices: Box<[BehaviorChoice]>,
    /// Alternate evaluator-rule reference at Section 12 `+0x11C`.
    pub behavior_rule_ref: u32,
    /// Alternate behavior class used by the live-state `0x4000` path.
    pub alternate_behavior_class_ref: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityTypeRuntimeMetadata {
    pub model_slots: [u16; 4],
    pub mass_raw: u16,
    pub capability_flags: u32,
    pub initial_health_raw: Option<i32>,
    pub damage_profile: Option<DamageProfile>,
    /// Authored E370 surface and DCA0 low-health effect controls. Absence of
    /// the exact Section-12 record must not silently become an all-zero policy.
    pub common_world_effects: RetailRuntimeValue<CommonWorldEffectProfile>,
    /// DCA0's visible/low-health positional cues and signed chance periods.
    pub detailed_sound_policy:
        RetailRuntimeValue<crate::actor_detailed_sound::ActorDetailedSoundPolicy>,
    /// Section-12 `+0xA2`. Class-4 uses this exact unsigned word as the
    /// slot-2 terrain-contact task's millisecond lifetime.
    pub terrain_contact_task_lifetime_ms: RetailRuntimeValue<u16>,
    /// Section-12 `+0x80`, used only by the accepted-hit presentation wrapper.
    pub accepted_hit_presentation_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x82`, `FUN_00411250` positional cue after bit `0x2000`.
    pub infected_model_presentation_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x84`, `FUN_00411320` cue after clearing bit `0x2000`.
    /// Unlike the infected cue, dying state does not suppress this sound.
    pub cured_model_presentation_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x90`, requested by the generic death transition.
    pub death_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x92`, unsigned target-warning cue used by `FUN_00416360`
    /// after stamping the target's `+0x34`. Zero means no authored cue.
    pub target_warning_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0xB4`, conditionally retained at entity `+0x8C` by the
    /// constructor and released after the independent death cue.
    pub constructor_sound_attachment_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x98`, requested on every live generic hit path.
    pub generic_hit_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x9A`, optional class-7 `FUN_0040ADE0` prelude cue
    /// (`FUN_0044F480` at entity `+0x96` when nonzero).
    pub search_attack_optional_prelude_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0x9C`, copied into Aim-and-Fire by `FUN_0040ADE0`.
    pub search_attack_aim_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0xA8`, signed Aim-and-Fire period copied by `FUN_0040ADE0`.
    pub search_attack_aim_sound_period_raw: RetailRuntimeValue<i32>,
    /// Section-12 `+0x9E`, copied into class-10 Run Away's fleeing task.
    pub run_away_optional_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Section-12 `+0xAC`, copied verbatim into class-10 Run Away's fleeing
    /// task and interpreted there through a signed `>= 0x400` gate.
    pub run_away_sound_period_raw: RetailRuntimeValue<u32>,
    /// Exact Section-12 Sub-A propulsion descriptor. Authored absence remains
    /// distinct from compatibility metadata which never supplied Section 12.
    pub sub_a_propulsion_descriptor: RetailRuntimeValue<Option<SubAPropulsionDescriptor>>,
    /// Exact Section-12 Sub-B lateral-velocity descriptor. This remains
    /// independent from the player Hover configuration because the generic
    /// common mover applies Sub-A before Sub-B.
    pub sub_b_lateral_descriptor: RetailRuntimeValue<Option<SubBLateralDescriptor>>,
    /// Exact Section-12 Sub-C lift/surface descriptor.  Its signed byte at
    /// `+0x0C` is also consumed independently by Common-Dying's terrain-
    /// attitude call, so this cannot be represented by topology alone.
    pub sub_c_lift_descriptor: RetailRuntimeValue<Option<SubCLiftDescriptor>>,
    /// Exact Section-12 Sub-D steering/avoidance descriptor. The live runtime
    /// cache retains allocator-owned unknown bytes separately; the authored
    /// descriptor itself is fully decoded here.
    pub sub_d_steering_descriptor: RetailRuntimeValue<Option<SubDSteeringDescriptor>>,
    /// Exact Sub-F swimming descriptor. A malformed present block remains unresolved.
    pub sub_f_swimming_descriptor: RetailRuntimeValue<Option<SubFSwimmingDescriptor>>,
    /// Type+110's allocation size in u16 model-variable words (09A80).
    pub model_variable_count_raw: RetailRuntimeValue<u32>,
    /// Exact Section-12 Sub-E projectile-emitter descriptor. `Known(Some(_))`
    /// proves authored emitter data, while `Known(None)` proves its absence.
    /// Compatibility metadata remains unresolved so a model/type coincidence
    /// cannot silently acquire a live firing component.
    pub projectile_emitter_descriptor: RetailRuntimeValue<Option<ProjectileEmitterDescriptor>>,
    /// Exact fixed-size Section-12 Sub-N payload. The loader allocates its
    /// runtime independently from behavior selection; a Section-13 animation
    /// block optionally enriches that zero-filled allocation, using words
    /// 2..4 as signed attachment coordinates.
    pub sub_n_payload: Option<[u8; 10]>,
    /// Exact Section-12 Sub-M descriptor. `Known(Some(_))` proves that the
    /// loader allocates the shared 0xB8 status component used here by the Main
    /// Base / Working Factory lifecycle; `Known(None)` proves its absence. Compatibility metadata
    /// remains unresolved rather than treating a missing table as authored
    /// absence.
    pub status_component_descriptor: RetailRuntimeValue<Option<StatusComponentDescriptor>>,
    /// Exact Section-12 Sub-I actor-animation descriptor. `Known(None)` proves
    /// authored absence; compatibility model/mass tables remain unresolved so
    /// they cannot silently acquire the generic controller.
    pub actor_animation_descriptor: RetailRuntimeValue<Option<ActorAnimationDescriptor>>,
    /// Exact Section-12 Sub-H external-frame descriptor. `Known(Some(_))`
    /// proves both the authored component and the live-record count used by
    /// `FUN_0041D2A0`; malformed or compatibility-only metadata remains
    /// unresolved instead of manufacturing an empty runtime.
    pub sub_h_external_frame_descriptor: RetailRuntimeValue<Option<SubHExternalFrameDescriptor>>,
    /// Exact Section-12 Sub-J attachment descriptor. The authored count is
    /// the backing-array ceiling, while each entity owns a separate ordered
    /// live length and mutable capacity. Malformed present data remains
    /// unresolved instead of degrading to a scalar cargo width.
    pub sub_j_attachment_descriptor: RetailRuntimeValue<Option<SubJAttachmentDescriptor>>,
    /// Exact optional-component presence retained from Section 12.
    ///
    /// Descriptor fields alone cannot prove the branch selected by the common
    /// mover dispatcher: F/H/G/K/L can pre-empt or supplement Sub-I even when
    /// A/B/D/I are all present. Compatibility metadata therefore leaves the
    /// complete topology unresolved instead of treating omitted descriptors as
    /// authored absence.
    pub common_mover_topology: RetailRuntimeValue<CommonMoverComponentTopology>,
    /// Exact initializer payload, absent only when the caller supplied a
    /// compatibility model/mass table instead of a Section-12 type record.
    pub initializer: Option<EntityInitializerSpec>,
    /// Exact Section-12 Sub-G/K/L payloads used by the common-mover G then
    /// independent K/L dispatch. `Known` preserves authored presence or
    /// absence; compatibility metadata stays unresolved so ADE0 Chase cannot
    /// invent those components.
    pub common_mover_gkl_payloads: RetailRuntimeValue<CommonMoverGklPayloads>,
}

/// Raw fields read by the common world callbacks, independent of Sub-C/I/J.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommonWorldEffectProfile {
    /// Section-12 bytes +0x72/+0x73, used by FUN_0040E370.
    pub surface_selectors: [u8; 2],
    /// Section-12 dword +0x74, used by FUN_0040E370.
    pub surface_lifetime_ms: u32,
    /// DCA0 sound selectors: visible normal-health +0x94/+0x96, low-health +0xA0.
    /// Nonzero normal-health cues do not imply a low-health cue is present.
    pub low_health_effect_words: [u16; 3],
}

/// Exact optional Sub-G/K/L bytes retained from Section 12.
///
/// Field names stay payload-only: the common mover consumes these blocks
/// through typed adapters; Type 13 now owns both detailed and coarse routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonMoverGklPayloads {
    pub sub_g: Option<[u8; 104]>,
    pub sub_k: Option<[u8; 2]>,
    pub sub_l: Option<[u8; 6]>,
}

/// Presence of every authored optional Section-12 component that can affect
/// the common mover or its adjacent component-owned phases.
///
/// Keeping named fields is intentional: a numeric mask would make the loader's
/// field order look like runtime policy and would make later gates easy to
/// weaken accidentally.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommonMoverComponentTopology {
    pub sub_a: bool,
    pub sub_b: bool,
    pub sub_c: bool,
    pub sub_d: bool,
    pub sub_e: bool,
    pub sub_f: bool,
    pub sub_g: bool,
    pub sub_h: bool,
    pub sub_i: bool,
    pub sub_j: bool,
    pub sub_k: bool,
    pub sub_l: bool,
    pub sub_m: bool,
    pub sub_n: bool,
    pub sub_o: bool,
}

impl CommonMoverComponentTopology {
    pub fn from_section12(record: &CollisionEntry) -> Self {
        let present = |name| record.subsection(name).is_some();
        Self {
            sub_a: present("A"),
            sub_b: present("B"),
            sub_c: present("C"),
            sub_d: present("D"),
            sub_e: present("E"),
            sub_f: present("F"),
            sub_g: present("G"),
            sub_h: present("H"),
            sub_i: present("I"),
            sub_j: present("J"),
            sub_k: present("K"),
            sub_l: present("L"),
            sub_m: present("M"),
            sub_n: present("N"),
            sub_o: present("O"),
        }
    }
}

impl EntityTypeRuntimeMetadata {
    /// Decode every currently retained constructor/collision value from one
    /// exact cumulative Section-12 record.
    pub fn from_section12(record: &CollisionEntry) -> Self {
        let common_mover_topology = CommonMoverComponentTopology::from_section12(record);
        let sub_c_lift_descriptor = match record.sub_c_lift_descriptor() {
            Some(descriptor) => RetailRuntimeValue::Known(Some(descriptor)),
            None if common_mover_topology.sub_c => RetailRuntimeValue::Unresolved,
            None => RetailRuntimeValue::Known(None),
        };
        let sub_h_external_frame_descriptor = match record.sub_h_external_frame_descriptor() {
            Some(descriptor) => RetailRuntimeValue::Known(Some(descriptor)),
            None if common_mover_topology.sub_h => RetailRuntimeValue::Unresolved,
            None => RetailRuntimeValue::Known(None),
        };
        let sub_j_attachment_descriptor = match record.sub_j_attachment_descriptor() {
            Some(descriptor) => RetailRuntimeValue::Known(Some(descriptor)),
            None if common_mover_topology.sub_j => RetailRuntimeValue::Unresolved,
            None => RetailRuntimeValue::Known(None),
        };
        Self {
            model_slots: record.model_ids,
            mass_raw: record.scale as u16,
            capability_flags: record.id_field,
            initial_health_raw: Some(record.dims[0] as i32),
            damage_profile: DamageProfile::from_section12_header(&record.raw_header),
            common_world_effects: RetailRuntimeValue::Known(CommonWorldEffectProfile {
                surface_selectors: [record.raw_header[0x72], record.raw_header[0x73]],
                surface_lifetime_ms: u32::from_le_bytes(
                    record.raw_header[0x74..0x78].try_into().unwrap(),
                ),
                low_health_effect_words: [0x94, 0x96, 0xA0].map(|offset| {
                    u16::from_le_bytes([record.raw_header[offset], record.raw_header[offset + 1]])
                }),
            }),
            detailed_sound_policy: RetailRuntimeValue::Known(
                crate::actor_detailed_sound::ActorDetailedSoundPolicy::from_section12(record),
            ),
            terrain_contact_task_lifetime_ms: RetailRuntimeValue::Known(
                record.terrain_contact_task_lifetime_ms(),
            ),
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(
                record.accepted_hit_presentation_sound_id(),
            ),
            infected_model_presentation_sound_id: RetailRuntimeValue::Known(
                record.infected_model_presentation_sound_id(),
            ),
            cured_model_presentation_sound_id: RetailRuntimeValue::Known({
                let sound = u16::from_le_bytes([record.raw_header[0x84], record.raw_header[0x85]]);
                (sound != 0).then_some(sound)
            }),
            death_sound_id: RetailRuntimeValue::Known(record.death_sound_id()),
            target_warning_sound_id: RetailRuntimeValue::Known({
                let sound = u16::from_le_bytes([record.raw_header[0x92], record.raw_header[0x93]]);
                (sound != 0).then_some(sound)
            }),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(
                record.constructor_sound_attachment_id(),
            ),
            generic_hit_sound_id: RetailRuntimeValue::Known(record.generic_hit_sound_id()),
            search_attack_optional_prelude_sound_id: RetailRuntimeValue::Known(
                record.search_attack_optional_prelude_sound_id(),
            ),
            search_attack_aim_sound_id: RetailRuntimeValue::Known(
                record.search_attack_aim_sound_id(),
            ),
            search_attack_aim_sound_period_raw: RetailRuntimeValue::Known(
                record.search_attack_aim_sound_period_raw(),
            ),
            run_away_optional_sound_id: RetailRuntimeValue::Known(
                record.run_away_optional_sound_id(),
            ),
            run_away_sound_period_raw: RetailRuntimeValue::Known(
                record.run_away_sound_period_raw(),
            ),
            sub_a_propulsion_descriptor: RetailRuntimeValue::Known(
                record.sub_a_propulsion_descriptor(),
            ),
            sub_b_lateral_descriptor: RetailRuntimeValue::Known(record.sub_b_lateral_descriptor()),
            sub_c_lift_descriptor,
            sub_d_steering_descriptor: RetailRuntimeValue::Known(
                record.sub_d_steering_descriptor(),
            ),
            sub_f_swimming_descriptor: match record.sub_f_swimming_descriptor() {
                Some(descriptor) => RetailRuntimeValue::Known(Some(descriptor)),
                None if common_mover_topology.sub_f => RetailRuntimeValue::Unresolved,
                None => RetailRuntimeValue::Known(None),
            },
            model_variable_count_raw: RetailRuntimeValue::Known(u32::from_le_bytes(
                record.raw_header[0x110..0x114].try_into().unwrap(),
            )),
            projectile_emitter_descriptor: RetailRuntimeValue::Known(
                record.projectile_emitter_descriptor(),
            ),
            sub_n_payload: record
                .subsection("N")
                .and_then(|payload| payload.try_into().ok()),
            status_component_descriptor: RetailRuntimeValue::Known(
                record.status_component_descriptor(),
            ),
            actor_animation_descriptor: RetailRuntimeValue::Known(
                record.actor_animation_descriptor(),
            ),
            sub_h_external_frame_descriptor,
            sub_j_attachment_descriptor,
            common_mover_topology: RetailRuntimeValue::Known(common_mover_topology),
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: record.initializer_state_flags_raw,
                common_axis_descriptor: record.common_axis_descriptor(),
                behavior_choices: record.behavior_choices.clone().into_boxed_slice(),
                behavior_rule_ref: record.behavior_rule_ref,
                alternate_behavior_class_ref: record.alternate_behavior_class_ref,
            }),
            common_mover_gkl_payloads: RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: record.sub_g_payload(),
                sub_k: record.sub_k_payload(),
                sub_l: record.sub_l_payload(),
            }),
        }
    }
}

/// Component-state callback identity reached after both behavior callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairComponentContactPolicy {
    None,
    DescriptorContact,
    UnknownAddress(u32),
}

/// Proven way to reconstruct the active collision model's world basis from
/// retained entity state. Arbitrary actors may pitch/roll or use callback-
/// owned bases and therefore remain unresolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairOrientationPolicy {
    /// The live heading remains the authoritative first Euler word. The two
    /// retained Section-13 words complete the exact authored body basis for
    /// an allocation whose captured lifecycle does not replace them.
    LiveHeadingWithAuthoredPitchRoll { pitch_raw: u16, roll_raw: u16 },
}

impl PairComponentContactPolicy {
    pub const fn from_address(address: Option<u32>) -> Self {
        match address {
            None => Self::None,
            Some(0x0040_2DA0) => Self::DescriptorContact,
            Some(address) => Self::UnknownAddress(address),
        }
    }
}

/// Callback fields consulted by `FUN_00411AD0` and its two local damage
/// deliveries. Addresses are retained as live data so an unported lifecycle
/// can invalidate or replace one field without changing collision dispatch
/// policy globally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityPairCallbackRuntimeState {
    pub component_contact: RetailRuntimeValue<[PairComponentContactPolicy; 3]>,
    pub damage_modifier_address: RetailRuntimeValue<Option<u32>>,
    /// Whether every linked input consumed by a known non-null modifier is
    /// currently absent, making that callback an audited identity operation.
    /// This is live state, not an address-level promise.
    pub damage_modifier_identity_context_empty: RetailRuntimeValue<bool>,
    pub type_hit_callback_address: RetailRuntimeValue<Option<u32>>,
    pub orientation_policy: RetailRuntimeValue<PairOrientationPolicy>,
}

impl EntityPairCallbackRuntimeState {
    pub const fn unresolved() -> Self {
        Self {
            component_contact: RetailRuntimeValue::Unresolved,
            damage_modifier_address: RetailRuntimeValue::Unresolved,
            damage_modifier_identity_context_empty: RetailRuntimeValue::Unresolved,
            type_hit_callback_address: RetailRuntimeValue::Unresolved,
            orientation_policy: RetailRuntimeValue::Unresolved,
        }
    }

    pub(crate) const fn audited_local(
        component_slot_zero: Option<u32>,
        orientation_policy: RetailRuntimeValue<PairOrientationPolicy>,
    ) -> Self {
        Self {
            component_contact: RetailRuntimeValue::Known([
                PairComponentContactPolicy::from_address(component_slot_zero),
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
            ]),
            damage_modifier_address: RetailRuntimeValue::Known(None),
            damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
            type_hit_callback_address: RetailRuntimeValue::Known(None),
            orientation_policy,
        }
    }

    /// Constructor `FUN_004104B0` default: instance `+0x44` is null.
    ///
    /// Component, type-hit, and orientation remain unresolved until a census
    /// or type-6/66 constructor pair identity fills them.
    pub const fn unresolved_with_constructor_null_modifier() -> Self {
        Self {
            component_contact: RetailRuntimeValue::Unresolved,
            damage_modifier_address: RetailRuntimeValue::Known(None),
            damage_modifier_identity_context_empty: RetailRuntimeValue::Known(true),
            type_hit_callback_address: RetailRuntimeValue::Unresolved,
            orientation_policy: RetailRuntimeValue::Unresolved,
        }
    }

    pub(crate) const fn audited_player() -> Self {
        Self {
            component_contact: RetailRuntimeValue::Known([
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
                PairComponentContactPolicy::None,
            ]),
            // FUN_004484A0 is an identity operation only while the captured
            // linked type-63/controller damage inputs remain absent.
            damage_modifier_address: RetailRuntimeValue::Known(Some(0x0044_84A0)),
            // This depends on controller +0x220's live attachment list and is
            // synchronized by the player-pair adapter for every pass.
            damage_modifier_identity_context_empty: RetailRuntimeValue::Unresolved,
            type_hit_callback_address: RetailRuntimeValue::Known(None),
            // The player basis is owned by PlayerCraft, not reconstructed
            // from the generic entity heading.
            orientation_policy: RetailRuntimeValue::Unresolved,
        }
    }
}

impl Default for EntityPairCallbackRuntimeState {
    fn default() -> Self {
        Self::unresolved()
    }
}

/// Collision-relevant state retained alongside one live port entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityCollisionRuntimeState {
    /// Retail entity `+0x30`.
    pub health_raw: RetailRuntimeValue<i32>,
    /// Retail entity `+0x50`, consumed before health by `FUN_00414E90`.
    pub pre_health_damage_buffer_raw: RetailRuntimeValue<i32>,
    /// Seven-channel Section-12 filter used by collision damage.
    pub damage_profile: RetailRuntimeValue<DamageProfile>,
    /// Retail entity `+0x34`. `FUN_00410D30` updates it after its strict
    /// cadence gate, while class-1 `FUN_00410EB0` stamps it unconditionally
    /// before checked delivery. Other indirect hit handlers also share this
    /// field, so it is not a wrapper-private clock.
    pub last_hit_presentation_tick_at_0x34: RetailRuntimeValue<u32>,
    /// Type-record `+0x80`; distinct from generic hit and death audio.
    pub accepted_hit_presentation_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Live logical-sound attachment stored at entity `+0x8C`. The constructor
    /// seeds this from type-record `+0xB4`, but generic death releases and
    /// clears only this per-allocation custody; type metadata remains fixed.
    pub constructor_sound_attachment_id_at_0x8c: RetailRuntimeValue<Option<u16>>,
    /// `FUN_0044C920` copy of entity `+0x96/+0x98/+0x9A` into the `+0x8C`
    /// record's `+0x24/+0x28` emitter words. Sound-11 owners must mix from
    /// this pre-integration snapshot rather than a later render pose.
    pub constructor_sound_follow_position_raw: RetailRuntimeValue<[i16; 3]>,
    /// Set when this frame's `FUN_00412DA0` visit already published the
    /// follow position. The leftover live-list pass must not overwrite it
    /// with a post-integration body.
    pub(crate) constructor_sound_follow_published: bool,
    /// Type-record `+0x90` generic death cue.
    pub death_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Type-record `+0x98` generic live-hit cue.
    pub generic_hit_sound_id: RetailRuntimeValue<Option<u16>>,
    /// Effective entity state at `+0x08`. Creation-time water classification
    /// can remain unknown without hiding independently proven scheduler,
    /// pair-eligibility, fixedness, and model-selector bits.
    pub state_flags_at_0x08: RetailStateWord,
    /// Type-default state policy copied from Section 12 `+0xC0` to entity
    /// `+0xC8`. Relation release refreshes this copy from the live type record;
    /// compatibility entities keep it unresolved rather than inventing zero.
    pub default_state_flags_at_0xc8: RetailRuntimeValue<u32>,
    /// Entity handle at `+0x60` used by `FUN_00411A20`'s symmetric relation
    /// suppression. This is not a generic ownership/attachment field.
    pub recent_relation_id_at_0x60: RetailRuntimeValue<Option<u32>>,
    /// Elapsed scheduler time at `+0x68` for the recent-relation gate.
    pub recent_relation_elapsed_us_at_0x68: RetailRuntimeValue<u32>,
    /// Secondary scheduler accumulator at `+0x6c`.
    pub callback_scheduler_accumulator_us_at_0x6c: RetailRuntimeValue<u32>,
    /// Randomized scheduler value at `+0x70`; subjects scan only while zero.
    pub subject_scan_gate_at_0x70: RetailRuntimeValue<u32>,
    /// Byte at `+0xb6` that reduces a continuing scheduler delta to one.
    pub scheduler_unit_delta_flag_at_0xb6: RetailRuntimeValue<u8>,
    /// Transient callback-mass contribution at `+0xb2`, cleared by the
    /// callback-gate wait and the outer callback tail.
    pub animation_offset_at_0xb2: RetailRuntimeValue<u16>,
    /// Native fresh-Type-9 startup phase, separate from the mutable retail
    /// `+0xB2` value. Only authenticated fresh-New-Game construction arms it.
    pub(crate) fresh_level1_type9_first_scheduler_pending: bool,
    /// Live callback identities used by active-pair contact and damage.
    pub pair_callbacks: EntityPairCallbackRuntimeState,
}

impl EntityCollisionRuntimeState {
    /// Retain the constructor values whose source is exact. The caller supplies
    /// the independently resolved state domains; the relationship sentinel and
    /// scheduler fields are universal because every reachable initializer
    /// leaves the constructor's values unchanged. `+0xB2` alone is
    /// allocator-indeterminate until the scheduler clears it.
    pub fn from_constructor(
        metadata: Option<&EntityTypeRuntimeMetadata>,
        initial_damage_buffer_raw: i32,
        state_flags_at_0x08: RetailStateWord,
    ) -> Self {
        Self {
            health_raw: metadata
                .and_then(|metadata| metadata.initial_health_raw)
                .map_or(RetailRuntimeValue::Unresolved, RetailRuntimeValue::Known),
            pre_health_damage_buffer_raw: RetailRuntimeValue::Known(initial_damage_buffer_raw),
            damage_profile: metadata
                .and_then(|metadata| metadata.damage_profile)
                .map_or(RetailRuntimeValue::Unresolved, RetailRuntimeValue::Known),
            last_hit_presentation_tick_at_0x34: RetailRuntimeValue::Known(0),
            accepted_hit_presentation_sound_id: metadata
                .map_or(RetailRuntimeValue::Unresolved, |metadata| {
                    metadata.accepted_hit_presentation_sound_id
                }),
            constructor_sound_attachment_id_at_0x8c: metadata
                .map_or(RetailRuntimeValue::Unresolved, |metadata| {
                    metadata.constructor_sound_attachment_id
                }),
            constructor_sound_follow_position_raw: RetailRuntimeValue::Unresolved,
            constructor_sound_follow_published: false,
            death_sound_id: metadata.map_or(RetailRuntimeValue::Unresolved, |metadata| {
                metadata.death_sound_id
            }),
            generic_hit_sound_id: metadata.map_or(RetailRuntimeValue::Unresolved, |metadata| {
                metadata.generic_hit_sound_id
            }),
            state_flags_at_0x08,
            default_state_flags_at_0xc8: metadata
                .and_then(|metadata| metadata.initializer.as_ref())
                .map_or(RetailRuntimeValue::Unresolved, |initializer| {
                    RetailRuntimeValue::Known(initializer.initializer_state_flags_raw)
                }),
            recent_relation_id_at_0x60: RetailRuntimeValue::Known(None),
            recent_relation_elapsed_us_at_0x68: RetailRuntimeValue::Known(0),
            callback_scheduler_accumulator_us_at_0x6c: RetailRuntimeValue::Known(0),
            subject_scan_gate_at_0x70: RetailRuntimeValue::Known(0),
            scheduler_unit_delta_flag_at_0xb6: RetailRuntimeValue::Known(0),
            animation_offset_at_0xb2: RetailRuntimeValue::Unresolved,
            fresh_level1_type9_first_scheduler_pending: false,
            // Callback/component allocations are initializer-owned live
            // state. Exact per-instance evidence is attached by the entity
            // constructor after it has validated the authored spawn identity;
            // a Section-12 type record alone cannot prove these fields.
            pair_callbacks: EntityPairCallbackRuntimeState::unresolved(),
        }
    }

    /// State for a port-created presentation helper that did not pass through
    /// retail's entity constructor.
    pub fn unresolved_port_entity(active_model_slot: u8) -> Self {
        let selector = ((active_model_slot as u32) & 1) * ACTIVE_MODEL_SLOT_LOW_STATE_BIT
            | (((active_model_slot as u32) >> 1) & 1) * ACTIVE_MODEL_SLOT_HIGH_STATE_BIT;
        Self {
            health_raw: RetailRuntimeValue::Unresolved,
            pre_health_damage_buffer_raw: RetailRuntimeValue::Unresolved,
            damage_profile: RetailRuntimeValue::Unresolved,
            last_hit_presentation_tick_at_0x34: RetailRuntimeValue::Unresolved,
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Unresolved,
            constructor_sound_attachment_id_at_0x8c: RetailRuntimeValue::Unresolved,
            constructor_sound_follow_position_raw: RetailRuntimeValue::Unresolved,
            constructor_sound_follow_published: false,
            death_sound_id: RetailRuntimeValue::Unresolved,
            generic_hit_sound_id: RetailRuntimeValue::Unresolved,
            state_flags_at_0x08: RetailStateWord::from_known_bits(
                selector,
                ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            ),
            default_state_flags_at_0xc8: RetailRuntimeValue::Unresolved,
            recent_relation_id_at_0x60: RetailRuntimeValue::Unresolved,
            recent_relation_elapsed_us_at_0x68: RetailRuntimeValue::Unresolved,
            callback_scheduler_accumulator_us_at_0x6c: RetailRuntimeValue::Unresolved,
            subject_scan_gate_at_0x70: RetailRuntimeValue::Unresolved,
            scheduler_unit_delta_flag_at_0xb6: RetailRuntimeValue::Unresolved,
            animation_offset_at_0xb2: RetailRuntimeValue::Unresolved,
            fresh_level1_type9_first_scheduler_pending: false,
            pair_callbacks: EntityPairCallbackRuntimeState::unresolved(),
        }
    }

    /// Decode the authoritative model slot only when both selector bits are
    /// known. No parallel slot cache is allowed to override this state.
    pub const fn active_model_slot(&self) -> RetailRuntimeValue<usize> {
        match self
            .state_flags_at_0x08
            .masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
        {
            RetailRuntimeValue::Known(flags) => {
                RetailRuntimeValue::Known(active_model_slot_from_state_flags(flags))
            }
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        }
    }
}

/// Exact symmetric recent-relation suppression from `FUN_00411A20`.
///
/// A known non-matching relation does not need its age. A known suppression on
/// either side also dominates unresolved state on the other side; otherwise
/// missing relation/age state remains explicitly unresolved.
pub fn recent_relation_suppresses_pair(
    first_id: u32,
    first: &EntityCollisionRuntimeState,
    second_id: u32,
    second: &EntityCollisionRuntimeState,
) -> RetailRuntimeValue<bool> {
    fn one_side(
        relation: &RetailRuntimeValue<Option<u32>>,
        age: &RetailRuntimeValue<u32>,
        other_id: u32,
    ) -> RetailRuntimeValue<bool> {
        match relation {
            RetailRuntimeValue::Known(Some(id)) if *id == other_id => match age {
                RetailRuntimeValue::Known(age) => {
                    RetailRuntimeValue::Known(*age < RECENT_RELATION_SUPPRESSION_WINDOW_US)
                }
                RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
            },
            RetailRuntimeValue::Known(_) => RetailRuntimeValue::Known(false),
            RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
        }
    }

    match (
        one_side(
            &first.recent_relation_id_at_0x60,
            &first.recent_relation_elapsed_us_at_0x68,
            second_id,
        ),
        one_side(
            &second.recent_relation_id_at_0x60,
            &second.recent_relation_elapsed_us_at_0x68,
            first_id,
        ),
    ) {
        (RetailRuntimeValue::Known(true), _) | (_, RetailRuntimeValue::Known(true)) => {
            RetailRuntimeValue::Known(true)
        }
        (RetailRuntimeValue::Known(false), RetailRuntimeValue::Known(false)) => {
            RetailRuntimeValue::Known(false)
        }
        _ => RetailRuntimeValue::Unresolved,
    }
}

/// Decode `FUN_00411AD0`'s four-way model selector from the live state word.
pub const fn active_model_slot_from_state_flags(state_flags: u32) -> usize {
    ((state_flags & ACTIVE_MODEL_SLOT_LOW_STATE_BIT) != 0) as usize
        | ((((state_flags & ACTIVE_MODEL_SLOT_HIGH_STATE_BIT) != 0) as usize) << 1)
}

/// Basic eligibility shared by subjects and candidates in `FUN_00411AD0`.
pub const fn pair_collision_base_eligible(state_flags: u32) -> bool {
    state_flags & PAIR_COLLISION_ENABLED_STATE_BIT != 0
        && state_flags & PAIR_COLLISION_INELIGIBLE_STATE_BIT == 0
}

/// Subject eligibility adds the exact `entity+0x70 == 0` scheduler gate.
pub const fn pair_collision_subject_eligible(state_flags: u32, subject_scan_gate: u32) -> bool {
    pair_collision_base_eligible(state_flags) && subject_scan_gate == 0
}

/// F980's active-model candidate gate, before model lookup or sweep refinement.
///
/// EXE 43FA3D..43FA51 requires 8000 and accepts 800 even when 1000 is set.
/// This differs from the active-pair gate. Unrelated unknown surface/detail
/// bits do not block the query, and a known disabled 8000 needs no other read.
pub const fn particle_model_collision_eligible(
    collision: &EntityCollisionRuntimeState,
) -> RetailRuntimeValue<bool> {
    match collision.state_flags_at_0x08.masked(0x8000) {
        RetailRuntimeValue::Known(0) => return RetailRuntimeValue::Known(false),
        RetailRuntimeValue::Known(_) => {}
        RetailRuntimeValue::Unresolved => return RetailRuntimeValue::Unresolved,
    }
    let alternate = collision.state_flags_at_0x08.masked(0x800);
    let ineligible = collision.state_flags_at_0x08.masked(0x1000);
    match (alternate, ineligible) {
        (RetailRuntimeValue::Known(0x800), _) | (_, RetailRuntimeValue::Known(0)) => {
            RetailRuntimeValue::Known(true)
        }
        (RetailRuntimeValue::Known(0), RetailRuntimeValue::Known(0x1000)) => {
            RetailRuntimeValue::Known(false)
        }
        _ => RetailRuntimeValue::Unresolved,
    }
}

/// Exact subject-side gates for retail's oriented model-versus-terrain pass.
///
/// A nonzero active-model collision radius is required before retail enters
/// the model program. The live state and scheduler fields deliberately remain
/// [`Unresolved`](RetailRuntimeValue::Unresolved) unless every bit/value read by
/// this decision is known; callers must enable terrain contact only for
/// [`Known(true)`](RetailRuntimeValue::Known).
pub const fn terrain_collision_subject_eligible(
    collision: &EntityCollisionRuntimeState,
    active_model_collision_radius_raw: u16,
) -> RetailRuntimeValue<bool> {
    if active_model_collision_radius_raw == 0 {
        return RetailRuntimeValue::Known(false);
    }

    const REQUIRED_STATE_MASK: u32 = PAIR_COLLISION_ENABLED_STATE_BIT
        | PAIR_COLLISION_INELIGIBLE_STATE_BIT
        | TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT
        | PAIR_COLLISION_FIXED_STATE_BIT
        | REMOTE_OWNED_STATE_BIT;

    match (
        collision.state_flags_at_0x08.masked(REQUIRED_STATE_MASK),
        collision.subject_scan_gate_at_0x70,
    ) {
        (RetailRuntimeValue::Known(state_flags), RetailRuntimeValue::Known(scan_gate)) => {
            RetailRuntimeValue::Known(
                pair_collision_subject_eligible(state_flags, scan_gate)
                    && state_flags & TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT != 0
                    && state_flags & (PAIR_COLLISION_FIXED_STATE_BIT | REMOTE_OWNED_STATE_BIT) == 0,
            )
        }
        _ => RetailRuntimeValue::Unresolved,
    }
}

/// Whether `FUN_00412760` selects this entity's fixed-body response branch.
pub const fn pair_collision_response_is_fixed(state_flags: u32) -> bool {
    state_flags & PAIR_COLLISION_FIXED_STATE_BIT != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_model_gate_preserves_f980_alternate_bit_and_masked_unknowns() {
        let mut collision = EntityCollisionRuntimeState::unresolved_port_entity(0);
        for (state, eligible) in [
            (0, false),
            (0x800, false),
            (0x1000, false),
            (0x1800, false),
            (0x8000, true),
            (0x8800, true),
            (0x9000, false),
            (0x9800, true),
        ] {
            // Only these three bits are evidence; remote/fixed/terrain/detail
            // flags are irrelevant to this particle candidate decision.
            collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(state, 0x9800);
            assert_eq!(
                particle_model_collision_eligible(&collision),
                RetailRuntimeValue::Known(eligible),
                "state {state:x}"
            );
        }
        for (value, known, expected) in [
            (0, 0x8000, RetailRuntimeValue::Known(false)),
            (0x8800, 0x8800, RetailRuntimeValue::Known(true)),
            (0x8000, 0x9000, RetailRuntimeValue::Known(true)),
            (0x8000, 0x8800, RetailRuntimeValue::Unresolved),
            (0, 0, RetailRuntimeValue::Unresolved),
        ] {
            collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(value, known);
            assert_eq!(particle_model_collision_eligible(&collision), expected);
        }
    }

    fn section12_with_subsections(
        subsections: Vec<v2k_formats::collision::CollisionSubsection>,
    ) -> CollisionEntry {
        CollisionEntry {
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
            faces: Vec::new(),
            behavior_choices: Vec::new(),
            active_subs: subsections
                .iter()
                .map(|subsection| subsection.name.to_owned())
                .collect(),
            raw_header: [0; 0x128],
            subsections,
            behavior_rule_ref: 0,
            alternate_behavior_class_ref: 0,
            sub_a_pos: None,
        }
    }

    #[test]
    fn cure_sound_retains_independent_unsigned_type_plus_84_and_authored_absence() {
        let mut record = section12_with_subsections(Vec::new());
        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&record).cured_model_presentation_sound_id,
            RetailRuntimeValue::Known(None)
        );
        record.raw_header[0x82..0x84].copy_from_slice(&92_u16.to_le_bytes());
        record.raw_header[0x84..0x86].copy_from_slice(&0x8012_u16.to_le_bytes());
        let metadata = EntityTypeRuntimeMetadata::from_section12(&record);
        assert_eq!(
            metadata.infected_model_presentation_sound_id,
            RetailRuntimeValue::Known(Some(92))
        );
        assert_eq!(
            metadata.cured_model_presentation_sound_id,
            RetailRuntimeValue::Known(Some(0x8012))
        );
        assert_eq!(
            EntityTypeRuntimeMetadata::default().cured_model_presentation_sound_id,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn target_warning_sound_is_its_own_unsigned_word_and_preserves_absence() {
        let mut record = section12_with_subsections(Vec::new());
        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&record).target_warning_sound_id,
            RetailRuntimeValue::Known(None)
        );
        record.raw_header[0x90..0x92].copy_from_slice(&17_u16.to_le_bytes());
        record.raw_header[0x92..0x94].copy_from_slice(&0x8012_u16.to_le_bytes());
        let metadata = EntityTypeRuntimeMetadata::from_section12(&record);
        assert_eq!(
            metadata.target_warning_sound_id,
            RetailRuntimeValue::Known(Some(0x8012))
        );
        assert_eq!(metadata.death_sound_id, RetailRuntimeValue::Known(Some(17)));
        assert_eq!(
            metadata.accepted_hit_presentation_sound_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            EntityTypeRuntimeMetadata::default().target_warning_sound_id,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn common_world_effects_preserve_field_widths_and_missing_record_evidence() {
        let mut record = section12_with_subsections(Vec::new());
        record.raw_header[0x72..0x78].copy_from_slice(&[0x81, 0xFE, 0, 1, 2, 0x80]);
        record.raw_header[0x94..0x98].copy_from_slice(&[0x34, 0x12, 0xCD, 0xAB]);
        record.raw_header[0xA0..0xA2].copy_from_slice(&[0xFF, 0x80]);
        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&record).common_world_effects,
            RetailRuntimeValue::Known(CommonWorldEffectProfile {
                surface_selectors: [0x81, 0xFE],
                surface_lifetime_ms: 0x8002_0100,
                low_health_effect_words: [0x1234, 0xABCD, 0x80FF],
            }),
        );
        assert_eq!(
            EntityTypeRuntimeMetadata::default().common_world_effects,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn authored_malformed_sub_h_is_unresolved_not_authored_absence() {
        let malformed =
            section12_with_subsections(vec![v2k_formats::collision::CollisionSubsection {
                name: "H",
                field_index: 0x3b,
                // Declares one 0x14-byte record but omits it.
                data: vec![1, 0, 0, 0],
            }]);
        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&malformed).sub_h_external_frame_descriptor,
            RetailRuntimeValue::Unresolved
        );

        let absent = section12_with_subsections(Vec::new());
        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&absent).sub_h_external_frame_descriptor,
            RetailRuntimeValue::Known(None)
        );
    }

    #[test]
    fn authored_sub_j_keeps_exact_absent_and_malformed_states_distinct() {
        let exact = section12_with_subsections(vec![v2k_formats::collision::CollisionSubsection {
            name: "J",
            field_index: 0x3d,
            data: vec![1, 0, 1, 0, 0, 0, 10, 0, 110, 0],
        }]);
        let metadata = EntityTypeRuntimeMetadata::from_section12(&exact);
        let RetailRuntimeValue::Known(Some(descriptor)) = metadata.sub_j_attachment_descriptor
        else {
            panic!("exact authored Sub-J must decode")
        };
        assert_eq!(descriptor.reserved_at_0x01, 0);
        assert_eq!(descriptor.slots.len(), 1);
        assert_eq!(descriptor.slots[0].policy_word_raw, 1);
        assert_eq!(descriptor.slots[0].local_offset_raw, [0, 10, 110]);

        let malformed =
            section12_with_subsections(vec![v2k_formats::collision::CollisionSubsection {
                name: "J",
                field_index: 0x3d,
                data: vec![1, 0],
            }]);
        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&malformed).sub_j_attachment_descriptor,
            RetailRuntimeValue::Unresolved
        );

        assert_eq!(
            EntityTypeRuntimeMetadata::from_section12(&section12_with_subsections(Vec::new()))
                .sub_j_attachment_descriptor,
            RetailRuntimeValue::Known(None)
        );
    }

    #[test]
    fn deferred_destroy_write_clears_first_scheduler_mask_bits() {
        // Fresh Type-9 first `FUN_00412DA0` word `0x00468805` has both
        // `0x00020000` and `0x00040000` set. Setting only `0x00100000` leaves
        // those bits live and makes a later Main Base suffix snapshot stale.
        let mut flags = RetailStateWord::exact(0x0046_8805);
        flags.apply_deferred_destroy_pending_write();
        assert_eq!(
            flags.masked(DEFERRED_DESTROY_STATE_WRITE_MASK),
            RetailRuntimeValue::Known(DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
        assert_eq!(flags.masked(0x0006_0000), RetailRuntimeValue::Known(0));

        let mut pending_bit_only = RetailStateWord::exact(0x0046_8805);
        pending_bit_only.overwrite(
            DEFERRED_DESTROY_PENDING_STATE_BIT,
            DEFERRED_DESTROY_PENDING_STATE_BIT,
        );
        assert_ne!(flags, pending_bit_only);
    }

    fn known_runtime_state() -> EntityCollisionRuntimeState {
        let mut state = EntityCollisionRuntimeState::unresolved_port_entity(2);
        state.health_raw = RetailRuntimeValue::Known(400);
        state.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(12);
        state.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(0);
        state.accepted_hit_presentation_sound_id = RetailRuntimeValue::Known(Some(7));
        state.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(Some(10));
        state.death_sound_id = RetailRuntimeValue::Known(Some(8));
        state.generic_hit_sound_id = RetailRuntimeValue::Known(Some(9));
        state.state_flags_at_0x08 = RetailStateWord::exact(0);
        state.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        state.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        state.callback_scheduler_accumulator_us_at_0x6c = RetailRuntimeValue::Known(0);
        state.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
        state.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
        state.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        state
    }

    #[test]
    fn state_bits_decode_all_four_retail_model_slots() {
        assert_eq!(active_model_slot_from_state_flags(0), 0);
        assert_eq!(
            active_model_slot_from_state_flags(ACTIVE_MODEL_SLOT_LOW_STATE_BIT),
            1
        );
        assert_eq!(
            active_model_slot_from_state_flags(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            2
        );
        assert_eq!(
            active_model_slot_from_state_flags(
                ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            ),
            3
        );
    }

    #[test]
    fn constructor_distinguishes_authored_zero_sounds_from_missing_metadata() {
        let metadata = EntityTypeRuntimeMetadata {
            accepted_hit_presentation_sound_id: RetailRuntimeValue::Known(None),
            constructor_sound_attachment_id: RetailRuntimeValue::Known(None),
            death_sound_id: RetailRuntimeValue::Known(None),
            generic_hit_sound_id: RetailRuntimeValue::Known(None),
            ..EntityTypeRuntimeMetadata::default()
        };
        let authored_zero = EntityCollisionRuntimeState::from_constructor(
            Some(&metadata),
            0,
            RetailStateWord::exact(0),
        );
        assert_eq!(
            authored_zero.accepted_hit_presentation_sound_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            authored_zero.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            authored_zero.death_sound_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            authored_zero.generic_hit_sound_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            authored_zero.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(0)
        );

        let missing =
            EntityCollisionRuntimeState::from_constructor(None, 0, RetailStateWord::exact(0));
        assert_eq!(
            missing.accepted_hit_presentation_sound_id,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            missing.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(missing.death_sound_id, RetailRuntimeValue::Unresolved);
        assert_eq!(missing.generic_hit_sound_id, RetailRuntimeValue::Unresolved);

        let helper = EntityCollisionRuntimeState::unresolved_port_entity(0);
        assert_eq!(
            helper.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Unresolved,
            "port-only helpers did not pass through the constructor"
        );
    }

    #[test]
    fn constructor_copies_sound_attachment_into_per_entity_custody() {
        let metadata = EntityTypeRuntimeMetadata {
            constructor_sound_attachment_id: RetailRuntimeValue::Known(Some(44)),
            ..EntityTypeRuntimeMetadata::default()
        };
        let mut runtime = EntityCollisionRuntimeState::from_constructor(
            Some(&metadata),
            0,
            RetailStateWord::exact(0),
        );
        assert_eq!(
            runtime.constructor_sound_attachment_id_at_0x8c,
            RetailRuntimeValue::Known(Some(44))
        );

        runtime.constructor_sound_attachment_id_at_0x8c = RetailRuntimeValue::Known(None);
        assert_eq!(
            metadata.constructor_sound_attachment_id,
            RetailRuntimeValue::Known(Some(44)),
            "per-entity release must not mutate the type-record source"
        );
    }

    #[test]
    fn constructor_retains_type_default_state_copy_only_from_exact_initializer() {
        let metadata = EntityTypeRuntimeMetadata {
            initializer: Some(EntityInitializerSpec {
                initializer_state_flags_raw: 0x39,
                common_axis_descriptor: CommonAxisDescriptor::default(),
                behavior_choices: Box::new([]),
                behavior_rule_ref: 0,
                alternate_behavior_class_ref: 0,
            }),
            ..EntityTypeRuntimeMetadata::default()
        };
        let authored = EntityCollisionRuntimeState::from_constructor(
            Some(&metadata),
            0,
            RetailStateWord::exact(0),
        );
        assert_eq!(
            authored.default_state_flags_at_0xc8,
            RetailRuntimeValue::Known(0x39)
        );

        let compatibility = EntityCollisionRuntimeState::from_constructor(
            Some(&EntityTypeRuntimeMetadata::default()),
            0,
            RetailStateWord::exact(0),
        );
        assert_eq!(
            compatibility.default_state_flags_at_0xc8,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(
            EntityCollisionRuntimeState::unresolved_port_entity(0).default_state_flags_at_0xc8,
            RetailRuntimeValue::Unresolved
        );
    }

    #[test]
    fn partial_state_reads_and_masked_writes_preserve_independent_domains() {
        let surface_mask = 0x0060_0000;
        let mut state = RetailStateWord::exact(0x0e42_8805);
        state.invalidate(surface_mask);
        assert_eq!(state.known_mask(), !surface_mask);
        assert_eq!(state.masked(surface_mask), RetailRuntimeValue::Unresolved);
        assert_eq!(
            state.masked(0x0800_f000),
            RetailRuntimeValue::Known(0x0800_8000)
        );

        state.overwrite(
            ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        assert_eq!(
            state.masked(ACTIVE_MODEL_SLOT_LOW_STATE_BIT | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            RetailRuntimeValue::Known(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT)
        );
        assert_eq!(state.known_mask(), !surface_mask);
    }

    #[test]
    fn alternative_merge_keeps_only_equal_known_bits() {
        let first = RetailStateWord::from_known_bits(0b1010, 0b1111);
        let second = RetailStateWord::from_known_bits(0b1110, 0b1111);
        let third = RetailStateWord::from_known_bits(0b1010, 0b1011);
        assert_eq!(
            RetailStateWord::merge_alternatives(&[first, second, third]),
            RetailStateWord::from_known_bits(0b1010, 0b1011)
        );
    }

    #[test]
    fn subject_eligibility_and_fixedness_remain_separate_decisions() {
        let movable = PAIR_COLLISION_ENABLED_STATE_BIT;
        assert!(pair_collision_base_eligible(movable));
        assert!(pair_collision_subject_eligible(movable, 0));
        assert!(!pair_collision_subject_eligible(movable, 1));
        assert!(!pair_collision_base_eligible(
            movable | PAIR_COLLISION_INELIGIBLE_STATE_BIT
        ));
        assert!(!pair_collision_response_is_fixed(movable));
        assert!(pair_collision_response_is_fixed(
            movable | PAIR_COLLISION_FIXED_STATE_BIT
        ));
    }

    #[test]
    fn terrain_subject_eligibility_requires_every_retail_gate() {
        let mut collision = known_runtime_state();
        let eligible_state =
            PAIR_COLLISION_ENABLED_STATE_BIT | TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT;
        collision.state_flags_at_0x08 = RetailStateWord::exact(eligible_state);
        assert_eq!(
            terrain_collision_subject_eligible(&collision, 1),
            RetailRuntimeValue::Known(true)
        );

        for excluded_bit in [
            PAIR_COLLISION_INELIGIBLE_STATE_BIT,
            PAIR_COLLISION_FIXED_STATE_BIT,
            REMOTE_OWNED_STATE_BIT,
        ] {
            collision.state_flags_at_0x08 = RetailStateWord::exact(eligible_state | excluded_bit);
            assert_eq!(
                terrain_collision_subject_eligible(&collision, 1),
                RetailRuntimeValue::Known(false),
                "state bit {excluded_bit:#010x} must exclude terrain contact"
            );
        }

        collision.state_flags_at_0x08 =
            RetailStateWord::exact(TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT);
        assert_eq!(
            terrain_collision_subject_eligible(&collision, 1),
            RetailRuntimeValue::Known(false)
        );
        collision.state_flags_at_0x08 = RetailStateWord::exact(PAIR_COLLISION_ENABLED_STATE_BIT);
        assert_eq!(
            terrain_collision_subject_eligible(&collision, 1),
            RetailRuntimeValue::Known(false)
        );

        collision.state_flags_at_0x08 = RetailStateWord::exact(eligible_state);
        collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(1);
        assert_eq!(
            terrain_collision_subject_eligible(&collision, 1),
            RetailRuntimeValue::Known(false)
        );
    }

    #[test]
    fn terrain_subject_eligibility_fails_closed_on_unresolved_runtime_state() {
        let mut collision = known_runtime_state();
        let eligible_state =
            PAIR_COLLISION_ENABLED_STATE_BIT | TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT;
        collision.state_flags_at_0x08 = RetailStateWord::from_known_bits(
            eligible_state,
            PAIR_COLLISION_ENABLED_STATE_BIT | TERRAIN_WATER_COLLISION_ENABLED_STATE_BIT,
        );
        assert_eq!(
            terrain_collision_subject_eligible(&collision, 1),
            RetailRuntimeValue::Unresolved
        );

        collision.state_flags_at_0x08 = RetailStateWord::exact(eligible_state);
        collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            terrain_collision_subject_eligible(&collision, 1),
            RetailRuntimeValue::Unresolved
        );

        assert_eq!(
            terrain_collision_subject_eligible(&collision, 0),
            RetailRuntimeValue::Known(false),
            "a zero collision radius independently excludes the model program"
        );
    }

    #[test]
    fn recent_relation_suppression_is_symmetric_and_strictly_age_bounded() {
        let mut first = known_runtime_state();
        let mut second = known_runtime_state();
        first.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(20));
        first.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(749_999);
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Known(true)
        );

        first.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(750_000);
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Known(false)
        );

        first.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        second.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(10));
        second.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(3);
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Known(true)
        );

        second.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(99));
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Known(false)
        );
    }

    #[test]
    fn recent_relation_suppression_preserves_only_relevant_unknowns() {
        let mut first = known_runtime_state();
        let mut second = known_runtime_state();

        first.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(99));
        first.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Known(false)
        );

        first.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(Some(20));
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Unresolved
        );

        first.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(1);
        second.recent_relation_id_at_0x60 = RetailRuntimeValue::Unresolved;
        assert_eq!(
            recent_relation_suppresses_pair(10, &first, 20, &second),
            RetailRuntimeValue::Known(true)
        );
    }
}
