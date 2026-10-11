//! Native Intro2 dragons: their own Type10 allocation and DEGKL component state.
//!
//! Ordinary Type5 rows share this owner through [`Type10Profile`]: the same
//! components, choice, K/L bindings, mass, health and sounds, with their own
//! model, damage thresholds, Sub-D divisor, two Sub-G words, emitter and axis.

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
mod carrier_tests;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod ordinary_tests;

pub use death::{
    tick_intro2_type10_tumble, Intro2Type10TumbleFrame, Intro2Type10TumbleOutcome,
    Intro2Type10TumbleOwner,
};
pub use live::{
    tick_intro2_type10, Intro2Type10Block, Intro2Type10Frame, Intro2Type10Outcome,
    Intro2Type10Owner, Intro2Type10Tick,
};
pub(crate) use native::{
    authenticate_metadata, publish_authored_type10_family, publish_intro2_type10,
    Type10AuthoredConstruction,
};

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::Entity,
    entity_behavior::BehaviorSelection,
    entity_collision_state::{CommonMoverComponentTopology, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterRuntime,
    main_base_abort::MainBaseAbortActorLease,
};
use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor, SubDSteeringDescriptor,
};

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

/// Ordinary Type5 (worlds 42/46/47): Section-12 row values that differ.
pub const TYPE5_MODEL: usize = 1122;
pub const TYPE5_AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1900,
    raw_word_at_0x04: 7,
};
/// Type10's G payload with +0E = 69 and +20 = 1500.
pub const TYPE5_SUB_G: [u8; 104] = [
    0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 188, 2, 69, 0, 50, 0, 0, 0, 130, 0, 0, 0, 44, 1, 0, 0, 0,
    64, 0, 0, 220, 5, 0, 0, 1, 2, 3, 4, 5, 8, 9, 0, 0, 0, 0, 32, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 0, 17, 230, 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 16, 210, 0, 0, 0, 1, 0, 0, 0,
];
pub const TYPE5_EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    spread_raw: 512,
    aim_threshold_raw: 40_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 5120,
    raw_word_at_0x12: 48,
    ..EMITTER
};

/// Power-up carriers Type80 (world 39) and Type126 (world 25): Type10's row
/// apart from G +20 = 1500, a wider emitter and alternate class63.
pub const CARRIER_SUB_G: [u8; 104] = [
    0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 188, 2, 63, 0, 50, 0, 0, 0, 130, 0, 0, 0, 44, 1, 0, 0, 0,
    64, 0, 0, 220, 5, 0, 0, 1, 2, 3, 4, 5, 8, 9, 0, 0, 0, 0, 32, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 0, 17, 230, 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 16, 210, 0, 0, 0, 1, 0, 0, 0,
];
pub const TYPE80_EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    aim_threshold_raw: 24_000,
    target_axis_tolerance_raw: 5120,
    ..EMITTER
};
/// Type126 also fires with sound 82.
pub const TYPE126_EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    sound_id: 82,
    ..TYPE80_EMITTER
};
pub const TYPE126_AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 0x1e00,
    raw_word_at_0x04: 1,
};

/// Section-12 rows sharing this D/E/G/K/L Search And Attack owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type10Profile {
    /// Intro2 dragons, spawns 55/56.
    Type10,
    /// Ordinary-world Type5 births.
    Type5,
    /// Ordinary-world Type80 power-up carriers.
    Type80,
    /// Ordinary-world Type126 power-up carriers.
    Type126,
}

impl Type10Profile {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            10 => Some(Self::Type10),
            5 => Some(Self::Type5),
            80 => Some(Self::Type80),
            126 => Some(Self::Type126),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type10 => 10,
            Self::Type5 => 5,
            Self::Type80 => 80,
            Self::Type126 => 126,
        }
    }
    pub const fn model(self) -> usize {
        match self {
            Self::Type10 | Self::Type80 | Self::Type126 => MODEL,
            Self::Type5 => TYPE5_MODEL,
        }
    }
    pub const fn axis(self) -> CommonAxisDescriptor {
        match self {
            Self::Type10 | Self::Type80 => AXIS,
            Self::Type5 => TYPE5_AXIS,
            Self::Type126 => TYPE126_AXIS,
        }
    }
    pub const fn sub_d(self) -> SubDSteeringDescriptor {
        match self {
            Self::Type10 | Self::Type80 | Self::Type126 => SUB_D,
            Self::Type5 => crate::common_mover::sub_d::FLYER_SUB_D,
        }
    }
    pub const fn sub_g(self) -> &'static [u8; 104] {
        match self {
            Self::Type10 => &SUB_G,
            Self::Type5 => &TYPE5_SUB_G,
            Self::Type80 | Self::Type126 => &CARRIER_SUB_G,
        }
    }
    pub const fn emitter(self) -> ProjectileEmitterDescriptor {
        match self {
            Self::Type10 => EMITTER,
            Self::Type5 => TYPE5_EMITTER,
            Self::Type80 => TYPE80_EMITTER,
            Self::Type126 => TYPE126_EMITTER,
        }
    }
    pub const fn damage_thresholds_raw(self) -> [i32; 7] {
        match self {
            Self::Type10 | Self::Type80 | Self::Type126 => [0, 9000, 2200, 0, 200, 0, 0],
            Self::Type5 => [0, 2000, 1800, 0, 200, 0, 0],
        }
    }
    /// AC60's direct alternate: Tumble for 10/5, Auto Pilot for the carriers.
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type10 | Self::Type5 => 11,
            Self::Type80 | Self::Type126 => 63,
        }
    }
    /// G payload +0C: B6C0's 06070 randomized target base (700 for both).
    pub const fn sub_g_randomized_target_base(self) -> i32 {
        let g = self.sub_g();
        u16::from_le_bytes([g[12], g[13]]) as i32
    }
}

/// Mutable component allocations outlive task/behavior replacements. The G
/// allocation is retained in Entity::sub_g_06070_runtime; E owns cadence here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type10Runtime {
    pub(crate) entity_id: u32,
    pub(crate) spawn_index: usize,
    pub(crate) profile: Type10Profile,
    /// Ordinary 104B0 receipt; Intro2 spawns 55/56 carry none.
    pub(crate) ordinary_allocation: Option<MainBaseAbortActorLease>,
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
            && entity.entity_type == runtime.profile.entity_type()
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && match runtime.ordinary_allocation {
                Some(lease) => lease.entity_id == entity.id,
                None => {
                    runtime.profile == Type10Profile::Type10
                        && INTRO2_TYPE10_SPAWN_INDICES.contains(&runtime.spawn_index)
                }
            }
            && entity.model_slots == [Some(runtime.profile.model()); 4]
            && matches!(
                entity.sub_g_06070_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
    })
}

/// An ordinary receipt must also match its issuing manager generation.
pub(crate) fn type10_manager_allocation_authenticates(
    manager: &crate::entity::EntityManager,
    id: u32,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| {
            intro2_type10_allocation_authenticates(entity)
                && entity
                    .intro2_type10_runtime
                    .and_then(|runtime| runtime.ordinary_allocation)
                    .is_none_or(|lease| {
                        manager
                            .main_base_abort_actor_observation(id)
                            .is_some_and(|observation| observation.lease == lease)
                    })
        })
}

/// The authenticated row profile of a Type10-family allocation.
pub(crate) fn type10_profile(entity: &Entity) -> Option<Type10Profile> {
    intro2_type10_allocation_authenticates(entity)
        .then(|| entity.intro2_type10_runtime.map(|runtime| runtime.profile))
        .flatten()
}

/// An ordinary Type10-family row whose death is class63's BAF0/BC90 terminal.
pub(crate) fn type10_auto_pilot_profile(entity: &Entity) -> Option<Type10Profile> {
    type10_profile(entity).filter(|profile| {
        profile.alternate_behavior_class() == 63
            && entity
                .intro2_type10_runtime
                .is_some_and(|runtime| runtime.ordinary_allocation.is_some())
    })
}
