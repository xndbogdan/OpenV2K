//! Native E/L gun turrets, with distinct authored and campaign origins.

pub mod aim;
pub mod impact;
mod live;
pub(crate) mod native;
mod profile;
pub mod task;

pub use live::{
    tick_intro2_gun_turret, Intro2GunTurretBlock, Intro2GunTurretFrame, Intro2GunTurretOutcome,
    Intro2GunTurretOwner, Intro2GunTurretTick,
};
pub(crate) use native::{
    authenticate_metadata, publish_authored_gun_turret, publish_intro2_gun_turret,
    publish_ordinary_class29_turret_at_position, reselect_intro2_gun_turret,
    GunTurretAuthoredConstruction,
};
use profile::profile_for_entity;
pub use profile::Intro2GunTurretProfile;

use crate::{
    entity::{Entity, EntityManager},
    entity_behavior::BehaviorSelection,
    entity_collision_state::CommonMoverComponentTopology,
    generic_projectile_emitter::GenericEmitterRuntime,
    main_base_abort::MainBaseAbortActorLease,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor};

pub const INTRO2_GUN_TURRET_SPAWN_INDICES: [usize; 4] = [52, 53, 54, 61];
pub const MODEL: usize = 162;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 5120,
    raw_word_at_0x04: 11,
};
pub const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: false,
    sub_b: false,
    sub_c: false,
    sub_d: false,
    sub_e: true,
    sub_f: false,
    sub_g: false,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: false,
    sub_l: true,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
pub const SUB_L: [u8; 6] = [1, 2, 0x40, 0x1f, 0x40, 0x1f];
pub const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 14,
    random_interval_us: 300_000,
    spread_raw: 512,
    aim_threshold_raw: 4000,
    speed_override_raw: 5000,
    target_axis_tolerance_raw: 6400,
    sound_id: 78,
    raw_word_at_0x12: 46,
    alternate_emitter_raw: 48,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0, 3, 0, 0],
};
pub const INITIAL_CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 29,
}];
pub const FLOWER_CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 5,
        weight_multiplier: 1,
        behavior_class_id: 29,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 0,
    },
];

/// The allocation receipt and component words survive task replacements.
/// L owns animation selectors1/2; Type102 alone binds E's joint to selector3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2GunTurretRuntime {
    pub(crate) entity_id: u32,
    pub(crate) spawn_index: usize,
    pub(crate) profile: Intro2GunTurretProfile,
    origin: GunTurretConstructionOrigin,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) sub_e_joint_word_raw: u16,
    pub(crate) sub_l_target_raw: [i16; 3],
    pub(crate) sub_l_exact_raw: i32,
    pub(crate) sub_l_output_raw: [i16; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GunTurretConstructionOrigin {
    Intro2Authored,
    NativeOrdinary(MainBaseAbortActorLease),
    /// E/L reconstruction and cargo callbacks do not authorize a living owner.
    CampaignReconstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2GunTurretPublication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2GunTurretError {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Selection,
    AlternateBehavior,
}

pub(crate) fn intro2_gun_turret_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_gun_turret_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == runtime.profile.entity_type()
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots
                == runtime
                    .profile
                    .model_slots()
                    .map(|model| Some(usize::from(model)))
            && match runtime.origin {
                GunTurretConstructionOrigin::Intro2Authored => {
                    runtime.profile.authored_xz(runtime.spawn_index).is_some()
                }
                GunTurretConstructionOrigin::NativeOrdinary(lease) => {
                    Intro2GunTurretProfile::for_native_authored(entity.entity_type)
                        == Some(runtime.profile)
                        && lease.entity_id == entity.id
                }
                GunTurretConstructionOrigin::CampaignReconstruction => false,
            }
    })
}

pub(crate) fn intro2_gun_turret_manager_allocation_authenticates(
    manager: &EntityManager,
    id: u32,
) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    if !intro2_gun_turret_allocation_authenticates(entity) {
        return false;
    }
    match entity.intro2_gun_turret_runtime.unwrap().origin {
        GunTurretConstructionOrigin::NativeOrdinary(lease) => manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| observation.lease == lease),
        GunTurretConstructionOrigin::Intro2Authored => true,
        GunTurretConstructionOrigin::CampaignReconstruction => false,
    }
}

#[cfg(test)]
mod type92_tests;

#[cfg(test)]
mod flower_tests;

#[cfg(test)]
pub(crate) mod authored_tests;

#[cfg(test)]
mod castle_tests;
