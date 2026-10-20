//! Native ordinary Type12/Type127 building-trashers on the shared ground host.
//!
//! Both Section-12 rows carry A/B/C/D/E/H/K/L with model684's eight legs and
//! an 82-word variable bank. The weighted root is Follow Beacons 33 (Always),
//! Search And Attack 7 (PlayerNearby), Capture People 9 (PeopleNearby) and
//! Trash Buildings 15 (BuildingsNearby). Neither row has a Sub-J, so Capture
//! takes the true absent-J branch Type40 owns: `C910` kills the touched
//! person. Class15's `ADB0` installs only `403230`'s unlimited task, whose
//! `403250` finds no Sub-I and never transitions; the style's `C890` static
//! hook trashes whatever the body touches. Type12 (worlds 31/35) dies through
//! class1; the Type127 power-up carriers (world 27) through class63, dropping
//! the Type61 authored in their `+88`.

pub mod aim;
mod construction;
pub mod contact;
pub(crate) mod impact;
pub(crate) mod profile;
#[cfg(test)]
mod tests;
pub(crate) use construction::{publish_native_type12, NativeType12ConstructionRequest};

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

pub(crate) const MODEL: usize = 684;
pub(crate) const HEALTH: i32 = 10_000;
pub(crate) const MASS: u16 = 200;
pub(crate) const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCDEHKL_TOPOLOGY;
/// `+C0` 0x39: E640, drag, D4A0 ground, no terrain/water lane and no crush.
pub(crate) const DEFAULT_FLAGS: u32 = 0x39;
pub(crate) const MODEL_VARIABLES: u32 = 82;
pub(crate) const SUB_H_RECORDS: usize = 8;
pub(crate) const SUB_H_DEPENDENCIES: [[u8; 4]; SUB_H_RECORDS] = [
    [1, 2, 3, 4],
    [0, 2, 3, 5],
    [0, 3, 4, 5],
    [1, 2, 4, 5],
    [2, 5, 6, 7],
    [3, 4, 6, 7],
    [2, 4, 5, 7],
    [3, 4, 5, 6],
];
pub(crate) const CHOICES: [BehaviorChoice; 4] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 2,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 8,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 10,
        weight_multiplier: 3,
        behavior_class_id: 9,
    },
    BehaviorChoice {
        weight_rule_id: 9,
        weight_multiplier: 1,
        behavior_class_id: 15,
    },
];
pub(crate) const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 24,
    random_interval_us: 200_000,
    spread_raw: 256,
    aim_threshold_raw: 4000,
    speed_override_raw: 1500,
    target_axis_tolerance_raw: 3072,
    sound_id: 82,
    raw_word_at_0x12: 56,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

/// Section-12 rows sharing this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type12Row {
    /// Worlds 31/35, alternate class1.
    Type12,
    /// World 27's power-up carriers, alternate class63.
    Type127,
}

impl Type12Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            12 => Some(Self::Type12),
            127 => Some(Self::Type127),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type12 => 12,
            Self::Type127 => 127,
        }
    }
    pub const fn axis(self) -> CommonAxisDescriptor {
        match self {
            Self::Type12 => CommonAxisDescriptor {
                strict_axis_limit_raw: 3584,
                raw_word_at_0x04: 5,
            },
            Self::Type127 => CommonAxisDescriptor {
                strict_axis_limit_raw: 4096,
                raw_word_at_0x04: 1,
            },
        }
    }
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type12 => 1,
            Self::Type127 => 63,
        }
    }
    /// Defecate's terrain task lifetime word; neither row selects class4.
    const fn terrain_contact_task_lifetime_ms(self) -> u16 {
        match self {
            Self::Type12 => 67,
            Self::Type127 => 0,
        }
    }
    const fn generic_hit_sound(self) -> Option<u16> {
        match self {
            Self::Type12 => Some(87),
            Self::Type127 => None,
        }
    }
    const fn search_prelude_sound(self) -> Option<u16> {
        match self {
            Self::Type12 => Some(75),
            Self::Type127 => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type12Runtime {
    pub(crate) row: Type12Row,
    pub(crate) allocation: MainBaseAbortActorLease,
    /// None for a zeroed (Hive) birth record.
    pub(crate) spawn_index: Option<usize>,
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) kl_components: Intro2KlComponents,
}

impl Type12Runtime {
    pub const fn row(&self) -> Type12Row {
        self.row
    }
    pub const fn allocation(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub fn publish_model_variables(&self, vars: &mut v2k_formats::models::AnimVars) {
        self.kl_components.publish_model_variables(vars);
    }
}

pub type Type12Owner = shared::NativeGroundActorOwner<profile::Type12Profile>;
pub type Type127Owner = shared::NativeGroundActorOwner<profile::Type127Profile>;
pub type Type12Outcome = shared::NativeGroundActorOutcome;

/// One scheduler entry for both rows; each keeps its own framework profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type12FamilyOwner {
    Type12(Type12Owner),
    Type127(Type127Owner),
}

impl Type12FamilyOwner {
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, shared::NativeGroundActorBlock> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type12_row)
        {
            Some(Type12Row::Type12) => Type12Owner::adopt(manager, id).map(Self::Type12),
            Some(Type12Row::Type127) => Type127Owner::adopt(manager, id).map(Self::Type127),
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
            .and_then(type12_row)
        {
            Some(Type12Row::Type12) => {
                Type12Owner::adopt_blocked_prefix(manager, id).map(Self::Type12)
            }
            Some(Type12Row::Type127) => {
                Type127Owner::adopt_blocked_prefix(manager, id).map(Self::Type127)
            }
            None => Err(shared::NativeGroundActorBlock::Allocation),
        }
    }
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Type12(owner) => owner.entity_id(),
            Self::Type127(owner) => owner.entity_id(),
        }
    }
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        match self {
            Self::Type12(owner) => owner.actor_lease(),
            Self::Type127(owner) => owner.actor_lease(),
        }
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        match self {
            Self::Type12(owner) => owner.has_pending_prefix(),
            Self::Type127(owner) => owner.has_pending_prefix(),
        }
    }
    pub(crate) fn park_external_prefix(&mut self) {
        match self {
            Self::Type12(owner) => owner.park_external_prefix(),
            Self::Type127(owner) => owner.park_external_prefix(),
        }
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        match self {
            Self::Type12(owner) => owner.completed_mutation_boundary(manager),
            Self::Type127(owner) => owner.completed_mutation_boundary(manager),
        }
    }
}

pub struct Type12FamilyTick {
    pub outcome: Type12Outcome,
    pub retained_owner: Option<Type12FamilyOwner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type12Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub player_nearby: bool,
    pub buildings_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type12Block {
    Identity,
    Metadata,
    ComponentStorage,
    Kl(Intro2KlConstructionError),
    Prefix,
    AlreadyPublished,
    Runtime(&'static str),
    Ground(shared::NativeGroundActorBlock),
}

pub struct Type12Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub tasks: &'a mut dyn shared::NativeGroundTaskCustody,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
}

pub fn tick_type12_family(
    manager: &mut EntityManager,
    owner: Type12FamilyOwner,
    frame: Type12Frame<'_>,
) -> Type12FamilyTick {
    let shared_frame = shared::NativeGroundActorFrame {
        resources: shared::NativeGroundResources::ReadOnly(frame.resources),
        world_fx: frame.world_fx,
        elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
        capture: shared::NativeCaptureDispatch::AbsentJ {
            tasks: frame.tasks,
            notifications: frame.notifications,
        },
    };
    // Neither row has a class12 publication: a terminal death leaves no
    // replacement owner (E370's lifecycle death is a held boundary).
    match owner {
        Type12FamilyOwner::Type12(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            debug_assert!(tick.replacement_terminal.is_none());
            Type12FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type12FamilyOwner::Type12),
            }
        }
        Type12FamilyOwner::Type127(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            debug_assert!(tick.replacement_terminal.is_none());
            Type12FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type12FamilyOwner::Type127),
            }
        }
    }
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity
        .native_type12_runtime
        .as_ref()
        .is_some_and(|runtime| {
            entity.active
                && entity.entity_type == runtime.row.entity_type()
                && entity.id == runtime.allocation.entity_id
                && entity.authored_spawn_index == runtime.spawn_index
                && entity.model_slots == [Some(MODEL); 4]
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
                observation.lease == entity.native_type12_runtime.as_ref().unwrap().allocation
            })
}

/// The authenticated row of a Type12-family allocation.
pub(crate) fn type12_row(entity: &Entity) -> Option<Type12Row> {
    allocation_authenticates(entity)
        .then(|| {
            entity
                .native_type12_runtime
                .as_ref()
                .map(|runtime| runtime.row)
        })
        .flatten()
}

/// Authenticate a canonical Section-12 row and its model684 H/K/L bindings.
pub(crate) fn authenticate_metadata(
    row: Type12Row,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type12Block> {
    use crate::common_mover::sub_d::TYPE47_SUB_D;
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    };
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type12Block::Metadata);
    };
    let Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type12Block::Metadata);
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.model_variable_count_raw != Known(MODEL_VARIABLES)
        || metadata.mass_raw != MASS
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 2000, 2000, 2000, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 0, 128, 0, 0],
            })
        || metadata.common_world_effects != Known(CommonWorldEffectProfile::default())
        || metadata.common_mover_topology != Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some([81, 82]),
                sub_l: Some([2, 1, 244, 1, 120, 5]),
            })
        || Intro2KlComponents::from_native_constructor(metadata).is_err()
        || metadata.sub_a_propulsion_descriptor
            != Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 3000,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 400,
            }))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 1_000_000,
                correction_rate_raw: 100_000,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 75,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor != Known(Some(TYPE47_SUB_D))
        || metadata.projectile_emitter_descriptor != Known(Some(EMITTER))
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.terrain_contact_task_lifetime_ms
            != Known(row.terrain_contact_task_lifetime_ms())
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.death_sound_id != Known(Some(75))
        || metadata.accepted_hit_presentation_sound_id != Known(Some(6))
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.cured_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(row.generic_hit_sound())
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(row.search_prelude_sound())
        || metadata.search_attack_aim_sound_id != Known(None)
        || metadata.search_attack_aim_sound_period_raw != Known(0)
        || metadata.run_away_optional_sound_id != Known(None)
        || metadata.run_away_sound_period_raw != Known(0)
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != row.axis()
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != row.alternate_behavior_class()
        || h.completion_sound_id.is_some()
        || h.records.len() != SUB_H_RECORDS
        || h.records.iter().enumerate().any(|(index, record)| {
            let base = 110 + index as u16;
            record.phase_rate_raw != 0x3000_0000
                || record.resolver_flags_raw != 0x2000_0000
                || record.axis_mode_raw != 0
                || record.vertex_refs != [base, base + 8, base + 16]
                || record.dependencies != SUB_H_DEPENDENCIES[index]
        })
    {
        return Err(Type12Block::Metadata);
    }
    Ok(())
}

/// 24E30 zeroes 0x3c and copies method/sound. E+12 is not a model binding.
pub(crate) fn emitter_constructor() -> GenericEmitterRuntime {
    GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: EMITTER.projectile_method,
        emitter_selector: 0,
        sound_id: u32::from(EMITTER.sound_id),
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    }
}
