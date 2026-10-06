//! Authored Type6 `104B0/18A90 -> AC60 -> 25730/25CD0` and Primary25D60.
//!
//! The singleton selector is shared process RNG. 25730 clears T then S and
//! publishes an actual01020 task; it has no06070 component-reset suffix.

mod death;
pub mod impact;
mod live;
mod native;

pub use death::{publish_main_base_standard_death, MainBaseDeathBlock, MainBaseDeathOutcome};

pub use live::{tick_main_base_owner, MainBaseFrame, MainBaseOutcome, MainBaseOwner, MainBaseTick};
pub(crate) use native::publish_main_base;

use crate::{
    entity::{Entity, EntityManager},
    entity_behavior::BehaviorSelection,
    entity_collision_state::RetailRuntimeValue,
    factory_production::FactorySection13Config,
    main_base_abort::MainBaseAbortActorLease,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseRuntime {
    allocation: MainBaseAbortActorLease,
    spawn_index: usize,
    model_slots: [Option<usize>; 4],
    config: FactorySection13Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBasePublication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainBaseError {
    Identity,
    Metadata,
    Config,
    ComponentStorage,
    AlreadyPublished,
    Selection,
    Runtime(&'static str),
}

/// 25CD0's01020 descriptor owns zero lifetime and a +28 notification latch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainBaseTaskState {
    allocation_identity: u64,
    elapsed_ms: u32,
    under_attack_latched: bool,
}

impl MainBaseTaskState {
    const fn new(allocation_identity: u64) -> Self {
        Self {
            allocation_identity,
            elapsed_ms: 0,
            under_attack_latched: false,
        }
    }

    pub const fn elapsed_ms(self) -> u32 {
        self.elapsed_ms
    }
    pub const fn under_attack_latched(self) -> bool {
        self.under_attack_latched
    }
}

pub(crate) fn main_base_allocation_authenticates(entity: &Entity) -> bool {
    entity.main_base_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == 6
            && entity.id == runtime.allocation.entity_id
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == runtime.model_slots
            && matches!(
                entity.base_factory_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
    })
}

pub(crate) fn main_base_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            main_base_allocation_authenticates(entity)
                && manager
                    .main_base_abort_actor_observation(id)
                    .is_some_and(|observed| {
                        entity
                            .main_base_runtime
                            .is_some_and(|runtime| runtime.allocation == observed.lease)
                    })
        })
}

#[cfg(test)]
mod tests;
