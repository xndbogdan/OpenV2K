//! Bounded fresh-Level-1 Type-47 Main Base-abort adapter.
//!
//! The complete generic-death/class-12 transaction remains owned by
//! `ordinary_type47_death_live`. This module only authenticates the current
//! Main Base actor lease and route, translates the publisher result, and
//! carries its linear Common-Dying owner back to the caller for scheduler
//! adoption. Successor traversal is deliberately sampled by `EntityManager`
//! only after the callback returns.

use crate::{
    main_base_abort::{
        MainBaseAbortActorLease, MainBaseAbortActorObservation, MainBaseAbortActorRoute,
    },
    ordinary_type47_death_live::{Type47CommonDyingPublication, Type47CommonDyingPublicationError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType47DeathOutcome {
    RemoteOwnedNoOp { entity_id: u32, entity_type: u32 },
    AlreadyDyingNoOp { entity_id: u32, entity_type: u32 },
    CommonDyingPublished(Type47CommonDyingPublication),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseType47DeathAdvance {
    Advanced {
        outcome: MainBaseType47DeathOutcome,
        next_actor: Option<MainBaseAbortActorObservation>,
    },
    SuccessorUnavailableAfterCommit {
        outcome: MainBaseType47DeathOutcome,
    },
}

/// Missing custody or publisher evidence before any Type-47 mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainBaseType47DeathBlock {
    EntityMissing,
    ActorLeaseMismatch {
        expected: MainBaseAbortActorLease,
        actual: MainBaseAbortActorLease,
    },
    NotOrdinaryRoute(MainBaseAbortActorRoute),
    UnsupportedEntityType {
        actual: u32,
    },
    TypeMetadataUnavailable,
    Publication(Type47CommonDyingPublicationError),
}
