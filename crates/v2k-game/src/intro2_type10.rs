//! Native Intro2 dragons: their own Type10 allocation and DEGKL component state.

pub mod aim;
pub mod contact;
pub mod death;
pub mod impact;
mod live;
mod mover;
mod native;
mod search;
mod world;

#[cfg(test)]
mod live_tests;

pub use death::{
    tick_intro2_type10_tumble, Intro2Type10TumbleFrame, Intro2Type10TumbleOutcome,
    Intro2Type10TumbleOwner,
};
pub use live::{
    tick_intro2_type10, Intro2Type10Block, Intro2Type10Frame, Intro2Type10Outcome,
    Intro2Type10Owner, Intro2Type10Tick,
};
pub(crate) use native::{authenticate_metadata, publish_intro2_type10};

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::Entity,
    entity_behavior::BehaviorSelection,
    entity_collision_state::{CommonMoverComponentTopology, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterRuntime,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor};

pub const INTRO2_TYPE10_SPAWN_INDICES: [usize; 2] = [55, 56];
pub const MODEL: usize = 351;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1e00,
    raw_word_at_0x04: 0x0c85,
};
pub const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: false,
    sub_b: false,
    sub_c: false,
    sub_d: true,
    sub_e: true,
    sub_f: false,
    sub_g: true,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: true,
    sub_l: true,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
pub const INITIAL_CHOICES: [BehaviorChoice; 1] = [BehaviorChoice {
    weight_rule_id: 1,
    weight_multiplier: 1,
    behavior_class_id: 7,
}];
pub use crate::common_mover::sub_d::INTRO2_TYPE10_SUB_D as SUB_D;
pub const SUB_G: [u8; 104] = [
    0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 188, 2, 63, 0, 50, 0, 0, 0, 130, 0, 0, 0, 44, 1, 0, 0, 0,
    64, 0, 0, 238, 2, 0, 0, 1, 2, 3, 4, 5, 8, 9, 0, 0, 0, 0, 32, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 0, 17, 230, 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 16, 210, 0, 0, 0, 1, 0, 0, 0,
];
pub const SUB_G_ANIMATION_BINDINGS: [usize; 7] = [1, 2, 3, 4, 5, 8, 9];
pub const SUB_K: [u8; 2] = [11, 10];
pub const SUB_L: [u8; 6] = [6, 7, 0x40, 0x1f, 0x40, 0x1f];
pub const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 10,
    random_interval_us: 300_000,
    spread_raw: 128,
    aim_threshold_raw: 12_000,
    speed_override_raw: 1500,
    target_axis_tolerance_raw: 3840,
    sound_id: 81,
    raw_word_at_0x12: 44,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

/// Mutable component allocations outlive task/behavior replacements. The G
/// allocation is retained in Entity::sub_g_06070_runtime; E owns cadence here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10Runtime {
    pub(crate) entity_id: u32,
    pub(crate) spawn_index: usize,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_frame_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) sub_k_smoothed_raw: i32,
    pub(crate) sub_k_output_raw: [i16; 2],
    pub(crate) sub_l_target_raw: [i16; 3],
    pub(crate) sub_l_exact_raw: i32,
    pub(crate) sub_l_output_raw: [i16; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type10Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Selection,
    AlternateBehavior,
}

pub(crate) fn intro2_type10_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type10_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 10
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && INTRO2_TYPE10_SPAWN_INDICES.contains(&runtime.spawn_index)
            && entity.model_slots == [Some(MODEL); 4]
            && matches!(
                entity.sub_g_06070_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
    })
}
