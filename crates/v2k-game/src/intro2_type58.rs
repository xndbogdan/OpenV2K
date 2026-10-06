//! Shared Type58 construction, current task graph, and component custody.

pub mod aim;
mod behavior;
mod construction;
pub mod contact;
pub mod impact;
mod live;
mod mover;
mod native;
mod search;
mod world;

pub(crate) use construction::{publish_authored_type58, Type58AuthoredConstructionRequest};
pub use live::{
    tick_intro2_type58, Intro2Type58Block, Intro2Type58Frame, Intro2Type58Outcome,
    Intro2Type58Owner, Intro2Type58Tick,
};
pub(crate) use native::authenticate_metadata;
pub(crate) use native::publish_intro2_type58;

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::{Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    main_base_abort::MainBaseAbortActorLease,
};
use v2k_formats::{
    collision::{BehaviorChoice, CommonAxisDescriptor},
    terrain::TerrainGrid,
};

pub const INTRO2_TYPE58_SPAWN_INDICES: [usize; 1] = [40];
pub const MODEL: usize = 273;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1000,
    raw_word_at_0x04: 5,
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
        weight_rule_id: 8,
        weight_multiplier: 5,
        behavior_class_id: 26,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 10,
        behavior_class_id: 7,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type58Runtime {
    entity_id: u32,
    spawn_index: usize,
    model_slots: [Option<usize>; 4],
    anchor_raw: [i16; 3],
    origin: Type58ConstructionOrigin,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    sub_e_runtime: crate::generic_projectile_emitter::GenericEmitterRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type58ConstructionOrigin {
    Native(MainBaseAbortActorLease),
    CapturedIntro2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type58Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub furniture_nearby: bool,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type58Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    NearbyEvidence,
    Selection,
    Runtime(&'static str),
}

pub(crate) fn intro2_type58_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type58_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 58
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == runtime.model_slots
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == 8
            && match runtime.origin {
                Type58ConstructionOrigin::Native(lease) => lease.entity_id == entity.id,
                Type58ConstructionOrigin::CapturedIntro2 => {
                    INTRO2_TYPE58_SPAWN_INDICES.contains(&runtime.spawn_index)
                }
            }
    })
}

/// Authenticate the issuing manager, independently of mutable pose/task state.
pub(crate) fn type58_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    if !intro2_type58_allocation_authenticates(entity) {
        return false;
    }
    match entity.intro2_type58_runtime.unwrap().origin {
        Type58ConstructionOrigin::Native(lease) => manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| observation.lease == lease),
        Type58ConstructionOrigin::CapturedIntro2 => true,
    }
}

#[cfg(test)]
mod live_tests;

#[cfg(test)]
mod authored_tests;
