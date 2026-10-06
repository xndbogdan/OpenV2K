//! Descriptor-backed Type66 allocation and Working Factory task custody.
//!
//! Intro2's post-load health override is a separate scene policy. Authored
//! ordinary factories use the same constructor and retain their own template.

pub mod death;
pub mod impact;
mod live;
mod native;
mod production;
mod terminal_effects;

pub use live::{
    tick_intro2_type66_owner, Intro2Type66Block, Intro2Type66Frame, Intro2Type66Outcome,
    Intro2Type66Owner, Intro2Type66Tick,
};
pub use native::apply_intro2_type66_post_load_health;
pub(crate) use native::{authenticate_metadata, publish_working_factory, reselect_working_factory};

use crate::{
    entity::Entity,
    entity_behavior::BehaviorSelection,
    entity_collision_state::{CommonMoverComponentTopology, RetailRuntimeValue},
};
use v2k_formats::collision::{BehaviorChoice, StatusComponentDescriptor};

pub const INTRO2_TYPE66_SPAWN_INDICES: [usize; 2] = [36, 51];
pub const INITIAL_HEALTH_RAW: i32 = 99_999;
pub const INITIALIZER_STATE_RAW: u32 = 0x0002_5027;
pub const STATUS_DESCRIPTOR: StatusComponentDescriptor = StatusComponentDescriptor {
    raw_word_at_0x00: 8,
    variable_bindings: [4, 3, 0, 2, 1, 0],
    raw_tail: [0; 10],
};
pub const INITIAL_CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 39,
}];
pub const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: false,
    sub_b: false,
    sub_c: false,
    sub_d: false,
    sub_e: false,
    sub_f: false,
    sub_g: false,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: false,
    sub_l: false,
    sub_m: true,
    sub_n: false,
    sub_o: false,
};

/// Birth identity survives repair, task reselection and the destroyed model.
/// Mutable production and presentation fields stay with the Sub-M allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type66Runtime {
    pub(super) entity_id: u32,
    pub(super) spawn_index: usize,
    pub(super) allocation: crate::main_base_abort::MainBaseAbortActorLease,
    pub(super) model_slots: [Option<usize>; 4],
    pub(super) config: crate::factory_production::FactorySection13Config,
    pub(super) factory_allocation_identity: u64,
    //18A90/C830 copies position before D4A0 grounds the body.18F60 passes
    // null to C8E0, so production retunes never move this emitter.
    pub(super) voice_position_raw: [i16; 3],
}

impl Intro2Type66Runtime {
    pub(crate) const fn config(self) -> crate::factory_production::FactorySection13Config {
        self.config
    }

    pub(crate) const fn allocation(self) -> crate::main_base_abort::MainBaseAbortActorLease {
        self.allocation
    }

    pub(crate) const fn voice_position_raw(self) -> [i16; 3] {
        self.voice_position_raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type66Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type66Error {
    Identity,
    Metadata,
    Config,
    ComponentStorage,
    AlreadyPublished,
    Selection,
}

pub(crate) fn intro2_type66_allocation_authenticates(entity: &Entity) -> bool {
    let Some(runtime) = entity.intro2_type66_runtime else {
        return false;
    };
    entity.active
        && entity.id == runtime.entity_id
        && runtime.allocation.entity_id == entity.id
        && entity.entity_type == 66
        && entity.authored_spawn_index == Some(runtime.spawn_index)
        && entity.model_slots == runtime.model_slots
        && matches!(entity.base_factory_runtime, RetailRuntimeValue::Known(Some(factory))
            if factory.live_owner.is_some_and(|owner|
                owner.allocation_identity == runtime.factory_allocation_identity))
}

pub(crate) fn type66_manager_allocation_authenticates(
    manager: &crate::entity::EntityManager,
    id: u32,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            intro2_type66_allocation_authenticates(entity)
                && manager
                    .main_base_abort_actor_observation(id)
                    .is_some_and(|observed| {
                        entity
                            .intro2_type66_runtime
                            .is_some_and(|runtime| runtime.allocation == observed.lease)
                    })
        })
}
