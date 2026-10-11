//! Native ordinary Type38/Type129 Aimless and Search shooters.
//!
//! Both Section-12 rows carry A/B/C/D/E/H/K/L, an Always class5 Move About
//! Aimlessly choice and a PlayerNearby class7 Search And Attack choice, with
//! method10 Aim. They run the shared ground host's mover, tasks and contacts.
//! Type129 is Type38's row apart from a silent emitter, no target-warning
//! sound, axis +04 and alternate class63; Type38's alternate is class1. Both
//! deaths are the shared class49 terminal (BAF0, then BAC0 or BC90).

pub mod aim;
pub(crate) mod construction;
pub mod contact;
pub mod impact;
pub(crate) mod profile;
mod tasks;
pub(crate) use construction::{publish_authored_type38, Type38AuthoredConstructionRequest};
#[cfg(test)]
mod tests;

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::{Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
    generic_projectile_emitter::GenericEmitterRuntime,
    intro2_common_mover::{Intro2KlComponents, Intro2KlConstructionError},
    main_base_abort::MainBaseAbortActorLease,
    native_ground_actor as shared,
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};
use v2k_formats::collision::{BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor};

pub(crate) const MODEL: usize = 1131;
pub(crate) const HEALTH: i32 = 15_000;
pub(crate) const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCDEHKL_TOPOLOGY;
/// `+C0` 0x39: E640, drag, D4A0 ground and no terrain/water lane. No 0x400:
/// D920 never runs the generic crush.
pub(crate) const DEFAULT_FLAGS: u32 = 0x39;
pub(crate) const CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 5,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 5,
        behavior_class_id: 7,
    },
];
pub(crate) const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 10,
    random_interval_us: 600_000,
    spread_raw: 256,
    aim_threshold_raw: 4000,
    speed_override_raw: 1500,
    target_axis_tolerance_raw: 5120,
    sound_id: 82,
    raw_word_at_0x12: 122,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

/// Section-12 rows sharing this owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type38Row {
    /// Worlds 42/43, alternate class1.
    Type38,
    /// World 41 power-up carriers, alternate class63.
    Type129,
}

impl Type38Row {
    pub const fn from_entity_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            38 => Some(Self::Type38),
            129 => Some(Self::Type129),
            _ => None,
        }
    }
    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type38 => 38,
            Self::Type129 => 129,
        }
    }
    /// 24E30 copies this sound into Sub-E; Type129's emitter is silent.
    pub const fn emitter(self) -> ProjectileEmitterDescriptor {
        match self {
            Self::Type38 => EMITTER,
            Self::Type129 => ProjectileEmitterDescriptor {
                sound_id: 0,
                ..EMITTER
            },
        }
    }
    pub const fn target_warning_sound(self) -> Option<u16> {
        match self {
            Self::Type38 => Some(87),
            Self::Type129 => None,
        }
    }
    pub const fn axis(self) -> CommonAxisDescriptor {
        CommonAxisDescriptor {
            strict_axis_limit_raw: 3840,
            raw_word_at_0x04: match self {
                Self::Type38 => 3109,
                Self::Type129 => 3073,
            },
        }
    }
    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type38 => 1,
            Self::Type129 => 63,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type38Runtime {
    pub(crate) row: Type38Row,
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) spawn_index: usize,
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) kl_components: Intro2KlComponents,
}

impl Type38Runtime {
    pub const fn row(&self) -> Type38Row {
        self.row
    }
    pub const fn allocation(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub fn publish_model_variables(&self, vars: &mut v2k_formats::models::AnimVars) {
        self.kl_components.publish_model_variables(vars);
    }
}

pub type Type38Owner = shared::NativeGroundActorOwner<profile::Type38Profile>;
pub type Type129Owner = shared::NativeGroundActorOwner<profile::Type129Profile>;
pub type Type38Outcome = shared::NativeGroundActorOutcome;

/// One scheduler entry for both rows; each keeps its own framework profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type38FamilyOwner {
    Type38(Type38Owner),
    Type129(Type129Owner),
}

impl Type38FamilyOwner {
    pub fn adopt(manager: &EntityManager, id: u32) -> Result<Self, shared::NativeGroundActorBlock> {
        match manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type38_row)
        {
            Some(Type38Row::Type38) => Type38Owner::adopt(manager, id).map(Self::Type38),
            Some(Type38Row::Type129) => Type129Owner::adopt(manager, id).map(Self::Type129),
            None => Err(shared::NativeGroundActorBlock::Allocation),
        }
    }
    pub const fn entity_id(self) -> u32 {
        match self {
            Self::Type38(owner) => owner.entity_id(),
            Self::Type129(owner) => owner.entity_id(),
        }
    }
    pub(crate) const fn actor_lease(self) -> MainBaseAbortActorLease {
        match self {
            Self::Type38(owner) => owner.actor_lease(),
            Self::Type129(owner) => owner.actor_lease(),
        }
    }
    pub(crate) const fn has_pending_prefix(self) -> bool {
        match self {
            Self::Type38(owner) => owner.has_pending_prefix(),
            Self::Type129(owner) => owner.has_pending_prefix(),
        }
    }
    pub(crate) fn park_external_prefix(&mut self) {
        match self {
            Self::Type38(owner) => owner.park_external_prefix(),
            Self::Type129(owner) => owner.park_external_prefix(),
        }
    }
    pub(crate) const fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }
    pub(crate) fn completed_mutation_boundary(self, manager: &EntityManager) -> bool {
        match self {
            Self::Type38(owner) => owner.completed_mutation_boundary(manager),
            Self::Type129(owner) => owner.completed_mutation_boundary(manager),
        }
    }
}

pub struct Type38FamilyTick {
    pub outcome: Type38Outcome,
    pub retained_owner: Option<Type38FamilyOwner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type38Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type38Block {
    Identity,
    Metadata,
    ComponentStorage,
    Kl(Intro2KlConstructionError),
    Prefix,
    AlreadyPublished,
    Runtime(&'static str),
    Ground(shared::NativeGroundActorBlock),
}

pub struct Type38Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn tick_type38_family(
    manager: &mut EntityManager,
    owner: Type38FamilyOwner,
    frame: Type38Frame<'_>,
) -> Type38FamilyTick {
    let shared_frame = shared::NativeGroundActorFrame {
        resources: shared::NativeGroundResources::Mutable(frame.resources),
        world_fx: frame.world_fx,
        elapsed_micros: frame.elapsed_micros,
        retail_tick: frame.retail_tick,
        capture: shared::NativeCaptureDispatch::NoCapture,
    };
    // Neither row has a class12 publication: a terminal death leaves no
    // replacement owner (E370's lifecycle death is a held boundary).
    match owner {
        Type38FamilyOwner::Type38(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            debug_assert!(tick.replacement_terminal.is_none());
            Type38FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type38FamilyOwner::Type38),
            }
        }
        Type38FamilyOwner::Type129(owner) => {
            let tick = shared::tick_native_ground_actor(manager, owner, shared_frame);
            debug_assert!(tick.replacement_terminal.is_none());
            Type38FamilyTick {
                outcome: tick.outcome,
                retained_owner: tick.retained_owner.map(Type38FamilyOwner::Type129),
            }
        }
    }
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity
        .native_type38_runtime
        .as_ref()
        .is_some_and(|runtime| {
            entity.active
                && entity.entity_type == runtime.row.entity_type()
                && entity.id == runtime.allocation.entity_id
                && entity.authored_spawn_index == Some(runtime.spawn_index)
                && entity.model_slots == [Some(MODEL); 4]
                && entity.capability_flags == 8
        })
}

pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                observation.lease == entity.native_type38_runtime.as_ref().unwrap().allocation
            })
}

/// The authenticated row of a Type38-family allocation.
pub(crate) fn type38_row(entity: &Entity) -> Option<Type38Row> {
    allocation_authenticates(entity)
        .then(|| {
            entity
                .native_type38_runtime
                .as_ref()
                .map(|runtime| runtime.row)
        })
        .flatten()
}

/// Authenticate a canonical Section-12 row and its model1131 H/K/L bindings.
pub(crate) fn authenticate_metadata(
    row: Type38Row,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type38Block> {
    use crate::common_mover::sub_d::NATIVE_TYPE38_SUB_D;
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    };
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type38Block::Metadata);
    };
    let Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type38Block::Metadata);
    };
    let refs = [
        [4, 34, 42],
        [5, 35, 43],
        [10, 36, 44],
        [11, 37, 45],
        [16, 38, 46],
        [17, 39, 47],
        [22, 40, 48],
        [23, 41, 49],
    ];
    let dependencies = [
        [1, 2, 3, 3],
        [0, 2, 3, 3],
        [0, 1, 3, 3],
        [0, 1, 2, 2],
        [5, 6, 7, 7],
        [4, 6, 7, 7],
        [4, 5, 7, 7],
        [4, 5, 6, 6],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.model_variable_count_raw != Known(4)
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 10_000, 5000, 10_000, 200, 500, 0],
                multipliers_q8: [0, 128, 128, 128, 128, 512, 0],
            })
        || metadata.common_world_effects
            != Known(CommonWorldEffectProfile {
                surface_selectors: [1, 0],
                surface_lifetime_ms: 500,
                low_health_effect_words: [0; 3],
            })
        || metadata.common_mover_topology != Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some([4, 3]),
                sub_l: Some([1, 2, 64, 31, 160, 15]),
            })
        || Intro2KlComponents::from_native_constructor(metadata).is_err()
        || metadata.sub_a_propulsion_descriptor
            != Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 300,
            }))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 100_000,
                correction_rate_raw: 100_000,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 150,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor != Known(Some(NATIVE_TYPE38_SUB_D))
        || metadata.projectile_emitter_descriptor != Known(Some(row.emitter()))
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.sub_j_attachment_descriptor != Known(None)
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.terrain_contact_task_lifetime_ms != Known(0)
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.death_sound_id != Known(Some(75))
        || metadata.accepted_hit_presentation_sound_id != Known(Some(82))
        || metadata.target_warning_sound_id != Known(row.target_warning_sound())
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != row.axis()
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != row.alternate_behavior_class()
        || h.completion_sound_id.is_some()
        || h.records.len() != refs.len()
        || h.records.iter().enumerate().any(|(index, record)| {
            record.phase_rate_raw != 0x2000_0000
                || record.resolver_flags_raw != 0x2000_0000
                || record.axis_mode_raw != 0
                || record.vertex_refs != refs[index]
                || record.dependencies != dependencies[index]
        })
    {
        return Err(Type38Block::Metadata);
    }
    Ok(())
}
