//! Native Intro2 Type16 construction and task ownership.

pub mod aim;
mod behavior;
mod defecate;
pub mod impact;
mod live;
mod mover;
mod native;
mod search;
mod world;

#[cfg(test)]
mod carrier_tests;
#[cfg(test)]
mod contact_tests;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod ordinary_tests;

pub use live::{
    tick_intro2_type16, Intro2Type16Block, Intro2Type16Frame, Intro2Type16Outcome,
    Intro2Type16Owner, Intro2Type16Tick,
};
pub(crate) use native::authenticate_metadata;
pub(crate) use native::publish_intro2_type16;
pub(crate) use native::{publish_authored_type16, Type16AuthoredConstruction};

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::Entity,
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
};
use v2k_formats::{
    collision::{BehaviorChoice, CommonAxisDescriptor},
    terrain::TerrainGrid,
};

pub const INTRO2_TYPE16_SPAWN_INDICES: [usize; 2] = [5, 42];
pub const MODEL: usize = 257;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3584,
    raw_word_at_0x04: 3109,
};
pub const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: true,
    sub_b: true,
    sub_c: true,
    sub_d: true,
    sub_e: true,
    sub_f: false,
    sub_g: false,
    sub_h: true,
    sub_i: false,
    sub_j: true,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
pub const INITIAL_CHOICES: [BehaviorChoice; 5] = [
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 50,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 10,
        weight_multiplier: 10,
        behavior_class_id: 9,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 10,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 5,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 4,
    },
];

/// Section-12 rows on this owner. Type128 (worlds 25/28) is Type16's row
/// with a silent emitter and the class63 power-up carrier alternate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type16Row {
    Type16,
    Type128,
}

impl Type16Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            16 => Some(Self::Type16),
            128 => Some(Self::Type128),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type16 => 16,
            Self::Type128 => 128,
        }
    }
    /// Emitter descriptor `+10`, which 24E30 copies into Sub-E.
    pub const fn emitter_sound(self) -> u16 {
        match self {
            Self::Type16 => 81,
            Self::Type128 => 0,
        }
    }
    /// AC60's direct alternate: Flip Over And Die, or Auto Pilot.
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type16 => 12,
            Self::Type128 => 63,
        }
    }
}

/// Allocation identity and its own mutable steering state survive root changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type16Runtime {
    entity_id: u32,
    spawn_index: usize,
    pub(crate) row: Type16Row,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    sub_d_owner: Type9SubDFrameOwner,
    sub_e_runtime: crate::generic_projectile_emitter::GenericEmitterRuntime,
    /// Ordinary-world 104B0 receipt; Intro2 spawns5/42 carry none and own
    /// their recorded first-query Sub-D seeds instead.
    ordinary_allocation: Option<crate::main_base_abort::MainBaseAbortActorLease>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type16Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type16Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    NearbyEvidence,
    Selection,
}

pub(crate) fn intro2_type16_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type16_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == runtime.row.entity_type()
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && match runtime.ordinary_allocation {
                Some(lease) => lease.entity_id == entity.id,
                None => {
                    runtime.row == Type16Row::Type16
                        && INTRO2_TYPE16_SPAWN_INDICES.contains(&runtime.spawn_index)
                }
            }
            && entity.model_slots == [Some(MODEL); 4]
    })
}

/// The authenticated allocation's own Section-12 row.
pub(crate) fn type16_row(entity: &Entity) -> Option<Type16Row> {
    intro2_type16_allocation_authenticates(entity)
        .then(|| entity.intro2_type16_runtime.map(|runtime| runtime.row))
        .flatten()
}

/// An ordinary Type16-family row whose death is class63's BAF0/BC90 terminal.
pub(crate) fn type16_auto_pilot_row(entity: &Entity) -> Option<Type16Row> {
    type16_row(entity).filter(|row| {
        row.alternate_behavior_class() == 63
            && entity
                .intro2_type16_runtime
                .is_some_and(|runtime| runtime.ordinary_allocation.is_some())
    })
}

/// An ordinary receipt must also match its issuing manager generation.
pub(crate) fn type16_manager_allocation_authenticates(
    manager: &crate::entity::EntityManager,
    id: u32,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            intro2_type16_allocation_authenticates(entity)
                && entity
                    .intro2_type16_runtime
                    .and_then(|runtime| runtime.ordinary_allocation)
                    .is_none_or(|lease| {
                        manager
                            .main_base_abort_actor_observation(id)
                            .is_some_and(|observation| observation.lease == lease)
                    })
        })
}
