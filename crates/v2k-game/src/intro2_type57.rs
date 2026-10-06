//! Native Intro2 Type57 bat: own DEGKL allocation, class7 Search-and-Attack,
//! and method24 / class68 fire. Do not borrow Type10's spawn identity.

pub mod aim;
pub mod contact;
pub mod death;
pub mod impact;
mod live;
mod mover;
mod native;
mod search;
mod world;

pub use contact::{
    resolve_intro2_type57_tumble_contact, Intro2Type57ContactBlock, Intro2Type57ContactOutcome,
    Intro2Type57ContactReport,
};
pub use death::{
    publish_intro2_type57_standard_death, tick_intro2_type57_tumble, Intro2Type57DeathBlock,
    Intro2Type57TerminalReceipt, Intro2Type57TumbleContact, Intro2Type57TumbleFrame,
    Intro2Type57TumbleOutcome, Intro2Type57TumbleOwner, Intro2Type57TumbleTask,
    Intro2Type57TumbleTick,
};
pub use impact::{
    apply_intro2_type57_particle_hit, Intro2Type57ImpactBlock, Intro2Type57ImpactOutcome,
};
pub use live::{
    tick_intro2_type57, Intro2Type57Block, Intro2Type57Frame, Intro2Type57Outcome,
    Intro2Type57Owner, Intro2Type57Tick,
};
pub(crate) use native::{authenticate_metadata, publish_intro2_type57};

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::Entity,
    entity_behavior::BehaviorSelection,
    entity_collision_state::{CommonMoverComponentTopology, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterRuntime,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor};

pub const INTRO2_TYPE57_SPAWN_INDICES: [usize; 1] = [1];
pub const MODEL: usize = 122;
pub const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1e00,
    raw_word_at_0x04: 5,
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
    weight_multiplier: 2,
    behavior_class_id: 7,
}];
pub use crate::common_mover::sub_d::FLYER_SUB_D as SUB_D;

const fn type57_sub_g() -> [u8; 104] {
    let mut bytes = crate::intro2_type10::SUB_G;
    bytes[0x0E] = 70;
    let velocity = 550u16.to_le_bytes();
    bytes[0x20] = velocity[0];
    bytes[0x21] = velocity[1];
    bytes
}

pub const SUB_G: [u8; 104] = type57_sub_g();
pub const SUB_G_ANIMATION_BINDINGS: [usize; 7] = [1, 2, 3, 4, 5, 8, 9];
pub const SUB_K: [u8; 2] = [11, 10];
pub const SUB_L: [u8; 6] = [6, 7, 0x40, 0x1f, 0x40, 0x1f];
pub const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 24,
    random_interval_us: 600_000,
    spread_raw: 256,
    aim_threshold_raw: 16_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 7680,
    sound_id: 93,
    raw_word_at_0x12: 42,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type57Runtime {
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
pub struct Intro2Type57Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type57Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Selection,
    AlternateBehavior,
}

pub fn intro2_type57_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type57_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 57
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && INTRO2_TYPE57_SPAWN_INDICES.contains(&runtime.spawn_index)
            && entity.model_slots == [Some(MODEL); 4]
            && matches!(
                entity.sub_g_06070_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
    })
}
