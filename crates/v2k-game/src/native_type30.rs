//! Native authored icecurly Type30: 104B0/09A80/D4A0/D720 and classes5/7/26.
//! The fireecto Type33 (worlds 42/43/47) shares the owner as a second row:
//! the same A/B/C/D/E/H/K/L shape and class12 death, with Always Wander Near
//! Location 6 and Guard Location 32 and a Sub-C that rides the waved sea.
//!
//! The allocation retains its own H/D/K/L/A/E components across weighted
//! selections and Class12. Component shape alone never grants actor custody.

pub mod aim;
pub(crate) mod construction;
pub mod contact;
pub mod impact;
pub(crate) mod profile;
mod tasks;
pub(crate) use construction::{publish_authored_type30, Type30AuthoredConstructionRequest};
#[cfg(test)]
mod live_tests;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod type33_tests;

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::{Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterRuntime,
    intro2_common_mover::{Intro2KlComponents, Intro2KlConstructionError},
    main_base_abort::MainBaseAbortActorLease,
    native_ground_actor as shared,
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor};

pub(crate) const MODEL: usize = 1042;
pub(crate) const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCDEHKL_TOPOLOGY;
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3584,
    raw_word_at_0x04: 0xA5,
};
pub(crate) const CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 5,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 9,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 4,
        behavior_class_id: 26,
    },
];
pub(crate) const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 20,
    random_interval_us: 500_000,
    spread_raw: 512,
    aim_threshold_raw: 25_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 3840,
    sound_id: 75,
    raw_word_at_0x12: 50,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

pub(crate) const TYPE33_MODEL: usize = 1171;
pub(crate) const TYPE33_AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3072,
    raw_word_at_0x04: 5,
};
pub(crate) const TYPE33_CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 6,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 5,
        behavior_class_id: 32,
    },
];
pub(crate) const TYPE33_EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 24,
    random_interval_us: 400_000,
    spread_raw: 256,
    aim_threshold_raw: 12_000,
    speed_override_raw: 1500,
    target_axis_tolerance_raw: 3072,
    sound_id: 105,
    raw_word_at_0x12: 48,
    ..EMITTER
};

/// Section-12 rows sharing this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type30Row {
    /// Icecurlies: Aimless, Search and Trash Furniture.
    Type30,
    /// Fireectos: Wander Near Location and Guard Location.
    Type33,
}

impl Type30Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            30 => Some(Self::Type30),
            33 => Some(Self::Type33),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type30 => 30,
            Self::Type33 => 33,
        }
    }
    pub const fn model(self) -> usize {
        match self {
            Self::Type30 => MODEL,
            Self::Type33 => TYPE33_MODEL,
        }
    }
    pub const fn axis(self) -> CommonAxisDescriptor {
        match self {
            Self::Type30 => AXIS,
            Self::Type33 => TYPE33_AXIS,
        }
    }
    pub const fn emitter(self) -> ProjectileEmitterDescriptor {
        match self {
            Self::Type30 => EMITTER,
            Self::Type33 => TYPE33_EMITTER,
        }
    }
    pub const fn health_raw(self) -> i32 {
        match self {
            Self::Type30 => 12_000,
            Self::Type33 => 6000,
        }
    }
    pub const fn sub_h_records(self) -> usize {
        match self {
            Self::Type30 => 10,
            Self::Type33 => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type30Runtime {
    pub(crate) row: Type30Row,
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) spawn_index: usize,
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) kl_components: Intro2KlComponents,
}

impl Type30Runtime {
    pub const fn row(&self) -> Type30Row {
        self.row
    }
    pub const fn allocation(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn spawn_index(&self) -> usize {
        self.spawn_index
    }
    pub const fn anchor_raw(&self) -> [i16; 3] {
        self.anchor_raw
    }
    pub fn publish_model_variables(&self, vars: &mut v2k_formats::models::AnimVars) {
        self.kl_components.publish_model_variables(vars);
    }
}

pub type Type30Owner = shared::NativeGroundActorOwner<profile::Type30Profile>;
pub type Type33Owner = shared::NativeGroundActorOwner<profile::Type33Profile>;
pub type Type30Outcome = shared::NativeGroundActorOutcome;

/// One scheduler entry for both rows; each keeps its own framework profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type30FamilyOwner {
    Type30(Type30Owner),
    Type33(Type33Owner),
}

impl Type30FamilyOwner {
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, shared::NativeGroundActorBlock> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type30_row)
        {
            Some(Type30Row::Type30) => Type30Owner::adopt(manager, id).map(Self::Type30),
            Some(Type30Row::Type33) => Type33Owner::adopt(manager, id).map(Self::Type33),
            None => Err(shared::NativeGroundActorBlock::Allocation),
        }
    }
    pub(crate) fn adopt_blocked_prefix(
        manager: &EntityManager,
        id: u32,
    ) -> Result<Self, shared::NativeGroundActorBlock> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type30_row)
        {
            Some(Type30Row::Type30) => {
                Type30Owner::adopt_blocked_prefix(manager, id).map(Self::Type30)
            }
            Some(Type30Row::Type33) => {
                Type33Owner::adopt_blocked_prefix(manager, id).map(Self::Type33)
            }
            None => Err(shared::NativeGroundActorBlock::Allocation),
        }
    }
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Type30(owner) => owner.entity_id(),
            Self::Type33(owner) => owner.entity_id(),
        }
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        match self {
            Self::Type30(owner) => owner.has_pending_prefix(),
            Self::Type33(owner) => owner.has_pending_prefix(),
        }
    }
    pub(crate) fn park_external_prefix(&mut self) {
        match self {
            Self::Type30(owner) => owner.park_external_prefix(),
            Self::Type33(owner) => owner.park_external_prefix(),
        }
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        match self {
            Self::Type30(owner) => owner.completed_mutation_boundary(manager),
            Self::Type33(owner) => owner.completed_mutation_boundary(manager),
        }
    }
}

pub struct Type30FamilyTick {
    pub outcome: Type30Outcome,
    pub retained_owner: Option<Type30FamilyOwner>,
    pub replacement_terminal: Option<shared::NativeGroundTerminalPublication>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type30Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type30Block {
    Identity,
    Metadata,
    ComponentStorage,
    Kl(Intro2KlConstructionError),
    Prefix,
    AlreadyPublished,
    Runtime(&'static str),
    Ground(shared::NativeGroundActorBlock),
}

pub struct Type30Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn tick_type30(
    manager: &mut EntityManager,
    owner: Type30FamilyOwner,
    frame: Type30Frame<'_>,
) -> Type30FamilyTick {
    let shared_frame = shared::NativeGroundActorFrame {
        resources: shared::NativeGroundResources::Mutable(frame.resources),
        world_fx: frame.world_fx,
        elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
        capture: shared::NativeCaptureDispatch::NoCapture,
    };
    match owner {
        Type30FamilyOwner::Type30(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            Type30FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type30FamilyOwner::Type30),
                replacement_terminal: tick.replacement_terminal,
            }
        }
        Type30FamilyOwner::Type33(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            Type30FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type30FamilyOwner::Type33),
                replacement_terminal: tick.replacement_terminal,
            }
        }
    }
}

/// Either row's allocation; [`type30_row`] names which.
pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity
        .native_type30_runtime
        .as_ref()
        .is_some_and(|runtime| {
            entity.active
                && entity.entity_type == runtime.row.entity_type()
                && entity.id == runtime.allocation.entity_id
                && entity.authored_spawn_index == Some(runtime.spawn_index)
                && entity.model_slots == [Some(runtime.row.model()); 4]
                && entity.capability_flags == 8
        })
}

/// The authenticated row of a Type30-family allocation.
pub(crate) fn type30_row(entity: &Entity) -> Option<Type30Row> {
    allocation_authenticates(entity)
        .then(|| {
            entity
                .native_type30_runtime
                .as_ref()
                .map(|runtime| runtime.row)
        })
        .flatten()
}

pub(crate) fn authenticate_row_metadata(
    row: Type30Row,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type30Block> {
    match row {
        Type30Row::Type30 => authenticate_metadata(metadata),
        Type30Row::Type33 => authenticate_type33_metadata(metadata),
    }
}

/// Authenticate canonical Section12 and actual model1171 H/K/L bindings.
pub(crate) fn authenticate_type33_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type30Block> {
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    };
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type30Block::Metadata);
    };
    let Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type30Block::Metadata);
    };
    let refs = [[44, 46, 48], [45, 47, 49], [34, 36, 38], [35, 37, 39]];
    let dependencies = [[1; 4], [0; 4], [3; 4], [2; 4]];
    if metadata.model_slots != [TYPE33_MODEL as u16; 4]
        || metadata.model_variable_count_raw != Known(4)
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(6000)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 8000, 3000, 0, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 64, 128, 128, 0],
            })
        || metadata.common_world_effects != Known(CommonWorldEffectProfile::default())
        || metadata.common_mover_topology != Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some([4, 3]),
                sub_l: Some([1, 2, 64, 31, 64, 31]),
            })
        || Intro2KlComponents::from_native_constructor(metadata).is_err()
        || metadata.sub_a_propulsion_descriptor
            != Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 300,
            }))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 1000,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 220,
                lift_range_raw: 55,
                strength_raw: 0x60000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 1,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor
            != Known(Some(crate::common_mover::sub_d::NATIVE_TYPE33_SUB_D))
        || metadata.projectile_emitter_descriptor != Known(Some(TYPE33_EMITTER))
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.terrain_contact_task_lifetime_ms != Known(0)
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.death_sound_id != Known(Some(75))
        || metadata.accepted_hit_presentation_sound_id != Known(Some(109))
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.cured_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(None)
        || metadata.search_attack_aim_sound_id != Known(None)
        || metadata.search_attack_aim_sound_period_raw != Known(0)
        || initializer.initializer_state_flags_raw != 0x31
        || initializer.common_axis_descriptor != TYPE33_AXIS
        || initializer.behavior_choices.as_ref() != TYPE33_CHOICES
        || initializer.behavior_rule_ref != 3
        || initializer.alternate_behavior_class_ref != 12
        || h.completion_sound_id.is_some()
        || h.records.len() != refs.len()
        || h.records.iter().enumerate().any(|(index, record)| {
            record.phase_rate_raw != 0x1000_0000
                || record.resolver_flags_raw != 0x2000_0000
                || record.axis_mode_raw != 0
                || record.vertex_refs != refs[index]
                || record.dependencies != dependencies[index]
        })
    {
        return Err(Type30Block::Metadata);
    }
    Ok(())
}

/// Either row's allocation under its issuing manager receipt.
pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                observation.lease == entity.native_type30_runtime.as_ref().unwrap().allocation
            })
}

/// Authenticate canonical Section12 and actual model1042 H/K/L bindings.
pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type30Block> {
    use crate::common_mover::sub_d::NATIVE_TYPE30_SUB_D;
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    };
    let Some(initializer) = &metadata.initializer else {
        return Err(Type30Block::Metadata);
    };
    let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type30Block::Metadata);
    };
    let refs = [
        [32, 34, 36],
        [33, 35, 37],
        [32, 34, 36],
        [33, 35, 37],
        [30, 32, 34],
        [31, 33, 35],
        [32, 34, 36],
        [33, 35, 37],
        [32, 34, 36],
        [33, 35, 37],
    ];
    let dependencies = [
        [8, 1, 2, 2],
        [9, 0, 3, 3],
        [0, 3, 4, 4],
        [1, 2, 5, 5],
        [2, 5, 6, 6],
        [3, 4, 7, 7],
        [4, 7, 8, 8],
        [5, 6, 9, 9],
        [6, 9, 0, 0],
        [7, 8, 1, 1],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(12_000)
        || metadata.damage_profile.is_none()
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || Intro2KlComponents::from_native_constructor(metadata).is_err()
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 200,
            }))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 10_000,
            }))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 100,
                lift_range_raw: 50,
                strength_raw: 0x300000,
                near_boost_range_raw: 150,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(NATIVE_TYPE30_SUB_D))
        || metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(EMITTER))
        || metadata.sub_f_swimming_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.status_component_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(Some(75))
        || metadata.accepted_hit_presentation_sound_id != RetailRuntimeValue::Known(Some(93))
        || initializer.initializer_state_flags_raw != 0x439
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 3
        || initializer.alternate_behavior_class_ref != 12
        || h.completion_sound_id.is_some()
        || h.records.len() != refs.len()
        || h.records.iter().enumerate().any(|(index, record)| {
            record.phase_rate_raw != 0x30000000
                || record.resolver_flags_raw != 0x40000000
                || record.axis_mode_raw != 0
                || record.vertex_refs != refs[index]
                || record.dependencies != dependencies[index]
        })
    {
        return Err(Type30Block::Metadata);
    }
    Ok(())
}
