//! Native ordinary Type43: an emitter-only class7 Search And Attack shooter.
//!
//! The 12 authored rows (worlds 42/46/47) carry Section-12 Sub-E alone. Their
//! B6C0 constructors therefore have no 06070 component work, and 01430 runs
//! only its target prelude before returning one. Authored `+C0` 0x2003B grounds
//! the body in D4A0, keeps DF70 grounding every callback and drops the
//! terrain/water lane (state 0x10000). Its fixed-body 0x20000 is reversed by
//! the Search style's +38, so the living body still integrates hits and wind
//! and enters 11AD0's static scan and active pairs. Lethal damage publishes
//! alternate class1 (BAC0) through the shared class49 terminal.

pub mod aim;
pub mod impact;
mod live;
mod mover;
mod native;
mod search;
mod world;

#[cfg(test)]
mod tests;

pub use live::{tick_type43, Type43Block, Type43Frame, Type43Outcome, Type43Owner, Type43Tick};
pub use mover::Type43MoverBlock;
pub(crate) use native::{
    authenticate_metadata, publish_authored_type43, Type43AuthoredConstruction,
};

use crate::{
    damage::DamageProfile,
    entity::{Entity, EntityManager},
    entity_behavior::BehaviorSelection,
    entity_collision_state::CommonMoverComponentTopology,
    generic_projectile_emitter::GenericEmitterRuntime,
    main_base_abort::MainBaseAbortActorLease,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor};

pub const ENTITY_TYPE: u32 = 43;
pub const MODEL: usize = 1185;
pub const MASS: u16 = 1000;
pub const HEALTH: i32 = 5000;
pub const CAPABILITY: u32 = 8;
/// Section-12 `+C0`: D4A0 ground (0x20), E640 (0x10), drag (0x08), DF70
/// (0x02), no terrain/water lane (0x01) and a fixed body (0x20000) that
/// class7's Search style reverses.
pub const DEFAULT_FLAGS: u32 = 0x0002_003b;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 2560,
    raw_word_at_0x04: 1,
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
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
pub const INITIAL_CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 7,
}];
pub const BEHAVIOR_RULE_REF: u32 = 3;
pub const ALTERNATE_BEHAVIOR_CLASS: u32 = 1;
pub const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 10,
    random_interval_us: 300_000,
    spread_raw: 1024,
    aim_threshold_raw: 65_535,
    speed_override_raw: 1500,
    target_axis_tolerance_raw: 7680,
    sound_id: 68,
    raw_word_at_0x12: 0,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};
pub const DAMAGE: DamageProfile = DamageProfile {
    thresholds_raw: [0, 2000, 200, 0, 200, 0, 0],
    multipliers_q8: [0, 256, 256, 512, 128, 0, 0],
};

/// The ordinary 104B0 receipt and the E allocation outlive task replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type43Runtime {
    pub(crate) entity_id: u32,
    pub(crate) spawn_index: usize,
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type43Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type43Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Selection,
    AlternateBehavior,
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type43_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && runtime.allocation.entity_id == entity.id
            && entity.entity_type == ENTITY_TYPE
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == [Some(MODEL); 4]
    })
}

/// The receipt must also match its issuing manager generation.
pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager.iter_all().any(|entity| {
        entity.id == id
            && allocation_authenticates(entity)
            && manager
                .main_base_abort_actor_observation(id)
                .zip(entity.native_type43_runtime)
                .is_some_and(|(observation, runtime)| observation.lease == runtime.allocation)
    })
}
