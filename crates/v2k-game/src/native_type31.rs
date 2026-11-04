//! Native Type31 virus slugs, the Type105 slugs their class18 splits, and
//! the Type73/Type109 splitting bugs (worlds 18 and 26).
//!
//! Type31 (worlds 16/31) is an authored A/B/C/D/E/H ground row; its weighted
//! root is Run Away 10 (UnderAttack), Defecate Virus 4 and Move About
//! Aimlessly 5 (Always). Lethal damage publishes alternate class18, whose
//! `FUN_0040C080` births eight Type105 children through `438080`. Type105 is
//! the same body without H: Run Away 10 or Defecate Virus 4, dying through
//! quiet class2. Neither row selects class7, so the 09A80 method30 Sub-E is
//! allocated but never fires.
//!
//! Type73 (A/B/C/D) and Type109 (A/B/C/D/E) run Wander Near Location 6,
//! Run Away 10 and Defecate Virus 4 and die through quiet class2. Their
//! instance damage modifier `FUN_00416820` splits a lethal hit into two
//! newborns of the same type (see [`split`]).

pub(crate) mod construction;
pub mod contact;
pub mod death;
pub mod impact;
pub(crate) mod profile;
pub mod split;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod type73_tests;
pub(crate) use construction::{publish_native_type31, NativeType31ConstructionRequest};

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::{Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    generic_projectile_emitter::GenericEmitterRuntime,
    main_base_abort::MainBaseAbortActorLease,
    native_ground_actor as shared,
    resource_cache::ResourceCache,
    split_and_explode::SplitChildRequest,
    world_fx::WorldFx,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor};

/// Type31's fourteen Sub-H legs: `[a, b, c]` model references per record.
pub(crate) const TYPE31_SUB_H_REFS: [[u16; 3]; 14] = [
    [52, 80, 66],
    [53, 81, 67],
    [54, 82, 68],
    [55, 83, 69],
    [56, 84, 70],
    [57, 85, 71],
    [58, 86, 72],
    [59, 87, 73],
    [60, 88, 74],
    [61, 89, 75],
    [62, 90, 74],
    [63, 91, 75],
    [64, 92, 78],
    [65, 93, 79],
];
pub(crate) const TYPE31_SUB_H_DEPENDENCIES: [[u8; 4]; 14] = [
    [1, 2, 3, 3],
    [0, 2, 3, 3],
    [0, 1, 3, 3],
    [0, 1, 2, 2],
    [5, 6, 7, 7],
    [4, 6, 7, 7],
    [4, 5, 7, 7],
    [4, 5, 6, 6],
    [9, 10, 11, 11],
    [8, 10, 11, 11],
    [11, 12, 13, 13],
    [10, 12, 13, 13],
    [10, 11, 13, 13],
    [10, 11, 12, 12],
];
const TYPE31_CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 8,
        behavior_class_id: 10,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 4,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 2,
        behavior_class_id: 5,
    },
];
const TYPE105_CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 8,
        behavior_class_id: 10,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 4,
    },
];
const TYPE73_CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 20,
        behavior_class_id: 6,
    },
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 100,
        behavior_class_id: 10,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 20,
        behavior_class_id: 4,
    },
];
const TYPE109_CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 10,
        behavior_class_id: 6,
    },
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 100,
        behavior_class_id: 10,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 20,
        behavior_class_id: 4,
    },
];
pub(crate) const TYPE31_TOPOLOGY: CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCDEH_TOPOLOGY;
pub(crate) const TYPE105_TOPOLOGY: CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCDE_TOPOLOGY;
/// Type73 owns no E: its 09A80 allocates A/B/C/D alone.
pub(crate) const TYPE73_TOPOLOGY: CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCD_TOPOLOGY;

/// Section-12 rows sharing this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type31Row {
    /// Authored in worlds 16 and 31; alternate class18.
    Type31,
    /// Born only by Type31's split; alternate class2.
    Type105,
    /// Authored in worlds 18 and 26, or born by a lethal `416820` split.
    Type73,
    /// Authored in world 26, or born by a lethal `416820` split.
    Type109,
}

impl Type31Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            31 => Some(Self::Type31),
            105 => Some(Self::Type105),
            73 => Some(Self::Type73),
            109 => Some(Self::Type109),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type31 => 31,
            Self::Type105 => 105,
            Self::Type73 => 73,
            Self::Type109 => 109,
        }
    }
    pub const fn model(self) -> usize {
        match self {
            Self::Type31 => 279,
            Self::Type105 => 280,
            Self::Type73 => 303,
            Self::Type109 => 304,
        }
    }
    pub const fn health(self) -> i32 {
        match self {
            Self::Type31 => 8000,
            Self::Type105 => 3000,
            Self::Type73 | Self::Type109 => 1000,
        }
    }
    pub const fn axis(self) -> CommonAxisDescriptor {
        match self {
            Self::Type31 => CommonAxisDescriptor {
                strict_axis_limit_raw: 2560,
                raw_word_at_0x04: 5,
            },
            Self::Type105 => CommonAxisDescriptor {
                strict_axis_limit_raw: 2048,
                raw_word_at_0x04: 5,
            },
            Self::Type73 | Self::Type109 => CommonAxisDescriptor {
                strict_axis_limit_raw: 3584,
                raw_word_at_0x04: 3,
            },
        }
    }
    /// `+C0`: 0x439 adds 0x400 to 0x39.
    pub const fn default_flags(self) -> u32 {
        match self {
            Self::Type31 | Self::Type109 => 0x439,
            Self::Type105 | Self::Type73 => 0x39,
        }
    }
    pub const fn choices(self) -> &'static [BehaviorChoice] {
        match self {
            Self::Type31 => &TYPE31_CHOICES,
            Self::Type105 => &TYPE105_CHOICES,
            Self::Type73 => &TYPE73_CHOICES,
            Self::Type109 => &TYPE109_CHOICES,
        }
    }
    pub const fn topology(self) -> CommonMoverComponentTopology {
        match self {
            Self::Type31 => TYPE31_TOPOLOGY,
            Self::Type105 | Self::Type109 => TYPE105_TOPOLOGY,
            Self::Type73 => TYPE73_TOPOLOGY,
        }
    }
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type31 => 18,
            Self::Type105 | Self::Type73 | Self::Type109 => 2,
        }
    }
    /// The rows whose `104B0` installs `FUN_00416820` at `+44`.
    pub const fn splits_on_lethal_hit(self) -> bool {
        matches!(self, Self::Type73 | Self::Type109)
    }
    const fn sub_a(self) -> v2k_formats::collision::SubAPropulsionDescriptor {
        let (acceleration_raw, overspeed_correction_raw, target_speed_base_raw) = match self {
            Self::Type31 => (2500, -3000, 200),
            Self::Type105 => (2600, -3000, 300),
            Self::Type73 => (1000, -1000, 400),
            Self::Type109 => (1000, -1000, 800),
        };
        v2k_formats::collision::SubAPropulsionDescriptor {
            acceleration_raw,
            overspeed_correction_raw,
            target_speed_base_raw,
        }
    }
    /// Classifier 6 is material and water; Type105's 4 is material alone;
    /// Type73's 0x11 is steepness and object, Type109's 0x15 adds material.
    pub(crate) const fn sub_d(self) -> v2k_formats::collision::SubDSteeringDescriptor {
        match self {
            Self::Type31 => crate::common_mover::sub_d::NATIVE_TYPE31_SUB_D,
            Self::Type105 => crate::common_mover::sub_d::NATIVE_TYPE105_SUB_D,
            Self::Type73 => crate::common_mover::sub_d::NATIVE_TYPE73_SUB_D,
            Self::Type109 => crate::common_mover::sub_d::NATIVE_TYPE109_SUB_D,
        }
    }
    const fn damage(self) -> crate::damage::DamageProfile {
        match self {
            Self::Type31 => crate::damage::DamageProfile {
                thresholds_raw: [0, 4000, 200, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            },
            Self::Type105 => crate::damage::DamageProfile {
                thresholds_raw: [0, 3000, 1500, 500, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
            },
            Self::Type73 | Self::Type109 => crate::damage::DamageProfile {
                thresholds_raw: [0, 3000, 1000, 1000, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 256, 256, 0, 0],
            },
        }
    }
    /// Healthy sound and its period: the slugs' 54 every four seconds, the
    /// bugs' own cue twice a second.
    const fn healthy_sound(self) -> (u16, i32) {
        match self {
            Self::Type31 | Self::Type105 => (54, 4_000_000),
            Self::Type73 => (92, 500_000),
            Self::Type109 => (93, 500_000),
        }
    }
    /// Sub-B rates and Sub-C `+0C`: the bugs also stand on the waved sea.
    const fn lateral_rate_and_surface_mode(self) -> (i32, i8) {
        match self {
            Self::Type31 | Self::Type105 => (1_000_000, 0),
            Self::Type73 | Self::Type109 => (1000, 1),
        }
    }
    const fn emitter(self) -> Option<v2k_formats::collision::ProjectileEmitterDescriptor> {
        use v2k_formats::collision::ProjectileEmitterDescriptor as E;
        let (method, interval, spread, threshold, speed, tolerance, sound) = match self {
            Self::Type31 => (30, 1_000_000, 256, 4000, 1500, 2560, 105),
            Self::Type105 => (30, 600_000, 256, 4000, 1500, 2560, 105),
            Self::Type73 => return None,
            Self::Type109 => (16, 1_500_000, 100, 20_000, 0, 3072, 64),
        };
        Some(E {
            projectile_method: method,
            random_interval_us: interval,
            spread_raw: spread,
            aim_threshold_raw: threshold,
            speed_override_raw: speed,
            target_axis_tolerance_raw: tolerance,
            sound_id: sound,
            raw_word_at_0x12: 0,
            alternate_emitter_raw: 0,
            stochastic_gate_mode: 0,
            auxiliary_command: 0,
            variable_bindings: [0; 4],
        })
    }
    /// Section-12 `+A4`: Defecate Virus's terrain task lifetime.
    pub(crate) const fn terrain_contact_lifetime_ms(self) -> u16 {
        self.bug_cues().0
    }
    /// Terrain-contact task lifetime, accepted-hit cue and Run Away audio.
    const fn bug_cues(self) -> (u16, Option<u16>, Option<u16>, u32) {
        match self {
            Self::Type31 | Self::Type105 => (0, None, None, 0),
            Self::Type73 | Self::Type109 => (64, Some(93), Some(86), 1_000_000),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type31Runtime {
    pub(crate) row: Type31Row,
    pub(crate) allocation: MainBaseAbortActorLease,
    /// The authored index; a dynamic birth has none.
    pub(crate) spawn_index: Option<usize>,
    /// A dynamic birth's record: Type31's class18 or a lethal 416820 split.
    pub(crate) birth: Option<SplitChildRequest>,
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    /// The 09A80 E record; Type73 owns none.
    pub(crate) sub_e_runtime: Option<GenericEmitterRuntime>,
    pub(crate) split_terminal: Option<death::Type31SplitTerminalState>,
    pub(crate) quiet_death_context: Option<BehaviorContextRuntime>,
}

impl Type31Runtime {
    pub const fn row(self) -> Type31Row {
        self.row
    }
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn birth(self) -> Option<SplitChildRequest> {
        self.birth
    }
    pub const fn split_terminal(self) -> Option<death::Type31SplitTerminalState> {
        self.split_terminal
    }
}

pub type Type31Owner = shared::NativeGroundActorOwner<profile::Type31Profile>;
pub type Type105Owner = shared::NativeGroundActorOwner<profile::Type105Profile>;
pub type Type73Owner = shared::NativeGroundActorOwner<profile::Type73Profile>;
pub type Type109Owner = shared::NativeGroundActorOwner<profile::Type109Profile>;
pub type Type31Outcome = shared::NativeGroundActorOutcome;

/// One scheduler entry for every row; each keeps its own framework profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type31FamilyOwner {
    Type31(Type31Owner),
    Type105(Type105Owner),
    Type73(Type73Owner),
    Type109(Type109Owner),
}

macro_rules! each_owner {
    ($self:expr, $owner:ident => $body:expr) => {
        match $self {
            Type31FamilyOwner::Type31($owner) => $body,
            Type31FamilyOwner::Type105($owner) => $body,
            Type31FamilyOwner::Type73($owner) => $body,
            Type31FamilyOwner::Type109($owner) => $body,
        }
    };
}

impl Type31FamilyOwner {
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, shared::NativeGroundActorBlock> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type31_row)
        {
            Some(Type31Row::Type31) => Type31Owner::adopt(manager, id).map(Self::Type31),
            Some(Type31Row::Type105) => Type105Owner::adopt(manager, id).map(Self::Type105),
            Some(Type31Row::Type73) => Type73Owner::adopt(manager, id).map(Self::Type73),
            Some(Type31Row::Type109) => Type109Owner::adopt(manager, id).map(Self::Type109),
            None => Err(shared::NativeGroundActorBlock::Allocation),
        }
    }
    pub const fn entity_id(self) -> u32 {
        each_owner!(self, owner => owner.entity_id())
    }
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        each_owner!(self, owner => owner.actor_lease())
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        each_owner!(self, owner => owner.has_pending_prefix())
    }
    pub(crate) fn park_external_prefix(&mut self) {
        each_owner!(self, owner => owner.park_external_prefix())
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        each_owner!(self, owner => owner.completed_mutation_boundary(manager))
    }
}

pub struct Type31FamilyTick {
    pub outcome: Type31Outcome,
    pub retained_owner: Option<Type31FamilyOwner>,
    /// A surface death's deferred class18/class2 publication.
    pub replacement_terminal: Option<shared::NativeGroundTerminalPublication>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type31Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub initializer_fallback: bool,
}

/// A dynamic birth: Type31's class18 Type105, or a lethal split's newborn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type105BirthPublication {
    pub owner: Type31FamilyOwner,
    pub publication: Type31Publication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type31Block {
    Identity,
    Metadata,
    Allocation,
    Terrain,
    ComponentStorage,
    Prefix,
    AlreadyPublished,
    Runtime(&'static str),
    Ground(shared::NativeGroundActorBlock),
}

pub struct Type31Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub tasks: &'a mut dyn shared::NativeGroundTaskCustody,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
}

pub fn tick_type31_family(
    manager: &mut EntityManager,
    owner: Type31FamilyOwner,
    frame: Type31Frame<'_>,
) -> Type31FamilyTick {
    // Class4's 02850 writes the terrain the later actor phases read. Class18
    // births its children inline, so the surface death needs task custody.
    let shared_frame = shared::NativeGroundActorFrame {
        terminal: None,
        resources: shared::NativeGroundResources::Mutable(frame.resources),
        world_fx: frame.world_fx,
        elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
        capture: shared::NativeCaptureDispatch::AbsentJ {
            tasks: frame.tasks,
            notifications: frame.notifications,
        },
    };
    macro_rules! tick {
        ($owner:expr, $variant:ident) => {{
            let tick = shared::tick_native_ground_actor(manager, $owner, shared_frame);
            Type31FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type31FamilyOwner::$variant),
                replacement_terminal: tick.replacement_terminal,
            }
        }};
    }
    match owner {
        Type31FamilyOwner::Type31(owner) => tick!(owner, Type31),
        Type31FamilyOwner::Type105(owner) => tick!(owner, Type105),
        Type31FamilyOwner::Type73(owner) => tick!(owner, Type73),
        Type31FamilyOwner::Type109(owner) => tick!(owner, Type109),
    }
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type31_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == runtime.row.entity_type()
            && entity.id == runtime.allocation.entity_id
            && entity.authored_spawn_index == runtime.spawn_index
            && match runtime.row {
                Type31Row::Type31 => runtime.spawn_index.is_some() && runtime.birth.is_none(),
                Type31Row::Type105 => runtime.spawn_index.is_none() && runtime.birth.is_some(),
                // Authored, or born by a lethal 416820 split.
                Type31Row::Type73 | Type31Row::Type109 => {
                    runtime.spawn_index.is_some() != runtime.birth.is_some()
                }
            }
            && entity.model_slots == [Some(runtime.row.model()); 4]
            && entity.capability_flags == 8
    })
}

pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                observation.lease == entity.native_type31_runtime.unwrap().allocation
            })
}

/// The authenticated row of a Type31-family allocation.
pub(crate) fn type31_row(entity: &Entity) -> Option<Type31Row> {
    allocation_authenticates(entity)
        .then(|| entity.native_type31_runtime.map(|runtime| runtime.row))
        .flatten()
}

/// Authenticate one canonical Section-12 row, including Type31's H legs.
pub(crate) fn authenticate_metadata(
    row: Type31Row,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type31Block> {
    use crate::actor_detailed_sound::ActorDetailedSoundPolicy;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use v2k_formats::collision::{SubBLateralDescriptor, SubCLiftDescriptor};
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type31Block::Metadata);
    };
    let h_matches = match (row, &metadata.sub_h_external_frame_descriptor) {
        (Type31Row::Type31, Known(Some(h))) => {
            h.completion_sound_id.is_none()
                && h.records.len() == TYPE31_SUB_H_REFS.len()
                && h.records.iter().enumerate().all(|(index, record)| {
                    record.phase_rate_raw == 0x3000_0000
                        && record.resolver_flags_raw
                            == if index == 8 { 0x0400_0000 } else { 0x4000_0000 }
                        && record.axis_mode_raw == 0
                        && record.vertex_refs == TYPE31_SUB_H_REFS[index]
                        && record.dependencies == TYPE31_SUB_H_DEPENDENCIES[index]
                })
        }
        (Type31Row::Type105 | Type31Row::Type73 | Type31Row::Type109, Known(None)) => true,
        _ => false,
    };
    let (healthy_sound, healthy_period) = row.healthy_sound();
    let (lateral_rate, surface_mode) = row.lateral_rate_and_surface_mode();
    let (contact_lifetime, accepted_hit_sound, run_away_sound, run_away_period) = row.bug_cues();
    if !h_matches
        || metadata.model_slots != [row.model() as u16; 4]
        || metadata.model_variable_count_raw != Known(0)
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(row.health())
        || metadata.damage_profile != Some(row.damage())
        || metadata.common_world_effects
            != Known(CommonWorldEffectProfile {
                surface_selectors: [0, 0],
                surface_lifetime_ms: 0,
                low_health_effect_words: [healthy_sound, 0, 0],
            })
        || metadata.detailed_sound_policy
            != Known(ActorDetailedSoundPolicy {
                full_health_raw: row.health(),
                healthy_sounds: [healthy_sound, 0],
                low_health_sound: 0,
                periods_raw: [healthy_period, 0],
            })
        || metadata.common_mover_topology != Known(row.topology())
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: None,
            })
        || metadata.sub_a_propulsion_descriptor != Known(Some(row.sub_a()))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: lateral_rate,
                correction_rate_raw: lateral_rate,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 50,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: surface_mode,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor != Known(Some(row.sub_d()))
        || metadata.projectile_emitter_descriptor != Known(row.emitter())
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.terrain_contact_task_lifetime_ms != Known(contact_lifetime)
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.death_sound_id != Known(None)
        || metadata.accepted_hit_presentation_sound_id != Known(accepted_hit_sound)
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.cured_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(None)
        || metadata.search_attack_aim_sound_id != Known(None)
        || metadata.search_attack_aim_sound_period_raw != Known(0)
        || metadata.run_away_optional_sound_id != Known(run_away_sound)
        || metadata.run_away_sound_period_raw != Known(run_away_period)
        || initializer.initializer_state_flags_raw != row.default_flags()
        || initializer.common_axis_descriptor != row.axis()
        || initializer.behavior_choices.as_ref() != row.choices()
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != row.alternate_behavior_class()
    {
        return Err(Type31Block::Metadata);
    }
    Ok(())
}

/// 24E30 zeroes the record and copies the row's method and sound.
pub(crate) fn emitter_constructor(row: Type31Row) -> Option<GenericEmitterRuntime> {
    row.emitter().map(|emitter| GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: emitter.projectile_method,
        emitter_selector: 0,
        sound_id: u32::from(emitter.sound_id),
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    })
}
