//! Shared Type17 allocation and current behavior ownership.
//!
//! Native authored actors retain their actual manager allocation and process
//! Sub-D constructor. The explicit Intro2 replay adapter keeps the two first-
//! query receipts from V200002; neither policy substitutes a captured choice
//! for the selector over the already-linked allocation prefix.

pub(crate) mod behavior;
pub mod capture;
pub(crate) mod carry_tasks;
mod construction;
pub mod contact;
pub mod impact;
pub(crate) mod live;
pub(crate) mod mover;
mod native;
pub mod pair;
mod run_away;
mod world;
pub use live::{
    tick_intro2_type17, Intro2Type17Block, Intro2Type17Frame, Intro2Type17Outcome,
    Intro2Type17Owner, Intro2Type17Tick,
};

pub(crate) use construction::{publish_authored_type17, Type17AuthoredConstructionRequest};
pub(crate) use native::publish_intro2_type17;

use crate::{
    entity::{Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    main_base_abort::MainBaseAbortActorLease,
    type17_impact_live::TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR,
    type17_impact_reselection::{Type17ImpactReselectionError, TYPE17_IMPACT_BEHAVIOR_CHOICES},
};
pub(crate) use native::authenticate_metadata;
use v2k_formats::{
    collision::{BehaviorChoice, CommonAxisDescriptor},
    terrain::TerrainGrid,
};

pub const INTRO2_TYPE17_SPAWN_INDICES: [usize; 2] = [4, 30];
const MODEL: usize = 256;
const AXIS: CommonAxisDescriptor = TYPE17_MODEL256_COMMON_AXIS_DESCRIPTOR;
const INITIAL_CHOICES: [BehaviorChoice; 4] = TYPE17_IMPACT_BEHAVIOR_CHOICES;

/// Allocation identity survives movement, task replacement, and death. The
/// existing Entity Type17 fields retain the mutable Sub-D frame and runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type17Runtime {
    entity_id: u32,
    spawn_index: usize,
    model_slots: [Option<usize>; 4],
    anchor_raw: [i16; 3],
    origin: Type17ConstructionOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type17ConstructionOrigin {
    Native(MainBaseAbortActorLease),
    CapturedIntro2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type17Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type17Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    Runtime(&'static str),
    Selection(Type17ImpactReselectionError),
}

pub(crate) fn intro2_type17_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type17_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 17
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == runtime.model_slots
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == 8
            && match runtime.origin {
                Type17ConstructionOrigin::Native(lease) => lease.entity_id == entity.id,
                Type17ConstructionOrigin::CapturedIntro2 => {
                    INTRO2_TYPE17_SPAWN_INDICES.contains(&runtime.spawn_index)
                }
            }
    })
}

/// Mutable position, task graphs and animation words do not identify an
/// allocation. Native entry points must also retain the manager generation.
pub(crate) fn type17_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    if !intro2_type17_allocation_authenticates(entity) {
        return false;
    }
    match entity
        .intro2_type17_runtime
        .expect("authenticated receipt")
        .origin
    {
        Type17ConstructionOrigin::Native(lease) => manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| observation.lease == lease),
        Type17ConstructionOrigin::CapturedIntro2 => true,
    }
}

#[cfg(test)]
mod construction_tests;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
pub(crate) mod tests;
