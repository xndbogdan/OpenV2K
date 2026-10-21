//! Native ordinary Type43 and Type35: emitter-only class7 Search And Attack.
//!
//! The 12 authored Type43 rows (worlds 42/46/47) carry Section-12 Sub-E alone.
//! Their B6C0 constructors therefore have no 06070 component work, and 01430
//! runs only its target prelude before returning one. Authored `+C0` 0x2003B
//! grounds the body in D4A0, keeps DF70 grounding every callback and drops the
//! terrain/water lane (state 0x10000). Its fixed-body 0x20000 is reversed by
//! the Search style's +38, so the living body still integrates hits and wind
//! and enters 11AD0's static scan and active pairs. Lethal damage publishes
//! alternate class1 (BAC0) through the shared class49 terminal.
//!
//! The 12 Type35 rows (worlds 41-47) are the same shape without a model: the
//! meteor launchers. Method 11 births a Type34 meteor from each shot. `+C0`
//! 0x201A7 adds the gravity disable 0x04 and drops E640 and drag. Every damage
//! multiplier is zero, so only 10C10 kills one; its alternate is class7 again.

pub mod aim;
pub mod impact;
mod live;
mod mover;
mod native;
mod search;
mod world;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod type35_tests;

pub use live::{tick_type43, Type43Block, Type43Frame, Type43Outcome, Type43Owner, Type43Tick};
pub use mover::Type43MoverBlock;
pub(crate) use native::{
    authenticate_metadata, publish_authored_type43, publish_type35_standard_death,
    Type43AuthoredConstruction,
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

pub const TYPE35_ENTITY_TYPE: u32 = 35;
/// D4A0 ground (0x20), DF70 (0x02), no terrain/water lane (0x01), no
/// gravity (0x04), 0x80 and 0x100 (read by none of DCA0, E100 or DF70) and
/// the fixed body 0x20000.
pub const TYPE35_DEFAULT_FLAGS: u32 = 0x0002_01a7;
pub const TYPE35_EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 11,
    random_interval_us: 600_000,
    spread_raw: 65_535,
    aim_threshold_raw: 65_535,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 7680,
    sound_id: 0,
    raw_word_at_0x12: 0,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

/// The two Section-12 rows sharing this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type43Row {
    Type43,
    Type35,
}

impl Type43Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            ENTITY_TYPE => Some(Self::Type43),
            TYPE35_ENTITY_TYPE => Some(Self::Type35),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type43 => ENTITY_TYPE,
            Self::Type35 => TYPE35_ENTITY_TYPE,
        }
    }
    /// Type35's Section-12 model words are zero: it has no model.
    pub const fn model(self) -> Option<usize> {
        match self {
            Self::Type43 => Some(MODEL),
            Self::Type35 => None,
        }
    }
    pub const fn mass(self) -> u16 {
        match self {
            Self::Type43 => MASS,
            Self::Type35 => 1,
        }
    }
    pub const fn capability(self) -> u32 {
        match self {
            Self::Type43 => CAPABILITY,
            Self::Type35 => 2,
        }
    }
    pub const fn health(self) -> i32 {
        match self {
            Self::Type43 => HEALTH,
            Self::Type35 => 100_000,
        }
    }
    pub const fn damage(self) -> DamageProfile {
        match self {
            Self::Type43 => DAMAGE,
            Self::Type35 => DamageProfile {
                thresholds_raw: [0, 10_000, 200, 0, 200, 0, 0],
                multipliers_q8: [0; 7],
            },
        }
    }
    pub const fn default_flags(self) -> u32 {
        match self {
            Self::Type43 => DEFAULT_FLAGS,
            Self::Type35 => TYPE35_DEFAULT_FLAGS,
        }
    }
    pub const fn axis(self) -> CommonAxisDescriptor {
        match self {
            Self::Type43 => AXIS,
            Self::Type35 => CommonAxisDescriptor {
                strict_axis_limit_raw: 7680,
                raw_word_at_0x04: 1,
            },
        }
    }
    pub const fn behavior_rule_ref(self) -> u32 {
        match self {
            Self::Type43 => BEHAVIOR_RULE_REF,
            Self::Type35 => 1,
        }
    }
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type43 => ALTERNATE_BEHAVIOR_CLASS,
            Self::Type35 => 7,
        }
    }
    pub const fn emitter(self) -> ProjectileEmitterDescriptor {
        match self {
            Self::Type43 => EMITTER,
            Self::Type35 => TYPE35_EMITTER,
        }
    }
    const fn model_slots(self) -> [Option<usize>; 4] {
        [self.model(); 4]
    }
}

/// The ordinary 104B0 receipt and the E allocation outlive task replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type43Runtime {
    pub(crate) entity_id: u32,
    pub(crate) spawn_index: usize,
    pub(crate) row: Type43Row,
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
    type43_row(entity).is_some()
}

/// The row of an authenticated allocation.
pub(crate) fn type43_row(entity: &Entity) -> Option<Type43Row> {
    entity
        .native_type43_runtime
        .filter(|runtime| {
            entity.active
                && entity.id == runtime.entity_id
                && runtime.allocation.entity_id == entity.id
                && entity.entity_type == runtime.row.entity_type()
                && entity.authored_spawn_index == Some(runtime.spawn_index)
                && entity.model_slots == runtime.row.model_slots()
        })
        .map(|runtime| runtime.row)
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
