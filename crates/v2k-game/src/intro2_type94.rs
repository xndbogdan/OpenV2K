//! Native Intro2 Type94 construction, current task graph, and component custody.

pub mod aim;
mod behavior;

pub mod impact;
mod live;
#[cfg(test)]
mod live_tests;
mod mover;
mod native;
#[cfg(test)]
mod ordinary_tests;
mod search;
mod world;

pub use live::{
    tick_intro2_type94, Intro2Type94Block, Intro2Type94Frame, Intro2Type94Outcome,
    Intro2Type94Owner, Intro2Type94Tick,
};
pub(crate) use native::authenticate_metadata;
pub(crate) use native::publish_intro2_type94;
pub(crate) use native::{publish_authored_type94, Type94AuthoredConstruction};

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

pub const INTRO2_TYPE94_SPAWN_INDICES: [usize; 1] = [43];
pub const MODEL: usize = 272;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1200,
    raw_word_at_0x04: 37,
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
    sub_j: false,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
pub const INITIAL_CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 10,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 10,
        weight_multiplier: 8,
        behavior_class_id: 9,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type94Runtime {
    entity_id: u32,
    spawn_index: usize,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    sub_d_owner: Type9SubDFrameOwner,
    sub_e_runtime: crate::generic_projectile_emitter::GenericEmitterRuntime,
    /// Ordinary-world 104B0 receipt; Intro2 spawn43 carries none and owns its
    /// recorded first-query Sub-D seed instead.
    ordinary_allocation: Option<crate::main_base_abort::MainBaseAbortActorLease>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type94Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type94Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    NearbyEvidence,
    Selection,
}

pub(crate) fn intro2_type94_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type94_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 94
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && match runtime.ordinary_allocation {
                Some(lease) => lease.entity_id == entity.id,
                None => INTRO2_TYPE94_SPAWN_INDICES.contains(&runtime.spawn_index),
            }
            && entity.model_slots == [Some(MODEL); 4]
    })
}

/// An ordinary receipt must also match its issuing manager generation.
pub(crate) fn type94_manager_allocation_authenticates(
    manager: &crate::entity::EntityManager,
    id: u32,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            intro2_type94_allocation_authenticates(entity)
                && entity
                    .intro2_type94_runtime
                    .and_then(|runtime| runtime.ordinary_allocation)
                    .is_none_or(|lease| {
                        manager
                            .main_base_abort_actor_observation(id)
                            .is_some_and(|observation| observation.lease == lease)
                    })
        })
}
