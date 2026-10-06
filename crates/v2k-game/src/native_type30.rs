//! Native authored icecurly Type30: 104B0/09A80/D4A0/D720 and classes5/7/26.
//!
//! The allocation retains its own H/D/K/L/A/E components across weighted
//! selections and Class12. Component shape alone never grants actor custody.

pub mod aim;
pub(crate) mod construction;
pub mod contact;
pub mod impact;
pub(crate) mod profile;
mod tasks;
pub(crate) use construction::{publish_authored_type30, Type30AuthoredConstructionRequest};
#[cfg(test)]
mod live_tests;
#[cfg(test)]
pub(crate) mod tests;

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

pub(crate) const MODEL: usize = 1042;
pub(crate) const TOPOLOGY: crate::entity_collision_state::CommonMoverComponentTopology =
    crate::intro2_common_mover::ABCDEHKL_TOPOLOGY;
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3584,
    raw_word_at_0x04: 0xA5,
};
pub(crate) const CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 5,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 9,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 4,
        behavior_class_id: 26,
    },
];
pub(crate) const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 20,
    random_interval_us: 500_000,
    spread_raw: 512,
    aim_threshold_raw: 25_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 3840,
    sound_id: 75,
    raw_word_at_0x12: 50,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type30Runtime {
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) spawn_index: usize,
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) kl_components: Intro2KlComponents,
}

impl Type30Runtime {
    pub const fn allocation(&self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn spawn_index(&self) -> usize {
        self.spawn_index
    }
    pub const fn anchor_raw(&self) -> [i16; 3] {
        self.anchor_raw
    }
    pub fn publish_model_variables(&self, vars: &mut v2k_formats::models::AnimVars) {
        self.kl_components.publish_model_variables(vars);
    }
}

pub type Type30Owner = shared::NativeGroundActorOwner<profile::Type30Profile>;
pub type Type30Tick = shared::NativeGroundActorTick<profile::Type30Profile>;
pub type Type30Outcome = shared::NativeGroundActorOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type30Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type30Block {
    Identity,
    Metadata,
    ComponentStorage,
    Kl(Intro2KlConstructionError),
    Prefix,
    AlreadyPublished,
    Runtime(&'static str),
    Ground(shared::NativeGroundActorBlock),
}

pub struct Type30Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn tick_type30(
    manager: &mut EntityManager,
    owner: Type30Owner,
    frame: Type30Frame<'_>,
) -> Type30Tick {
    shared::tick_native_ground_actor(
        manager,
        owner,
        shared::NativeGroundActorFrame {
            resources: shared::NativeGroundResources::Mutable(frame.resources),
            world_fx: frame.world_fx,
            elapsed_micros: frame.elapsed_micros,
            retail_tick: frame.retail_tick,
            capture: shared::NativeCaptureDispatch::NoCapture,
        },
    )
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity
        .native_type30_runtime
        .as_ref()
        .is_some_and(|runtime| {
            entity.active
                && entity.entity_type == 30
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
                observation.lease == entity.native_type30_runtime.as_ref().unwrap().allocation
            })
}

/// Authenticate canonical Section12 and actual model1042 H/K/L bindings.
pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type30Block> {
    use crate::common_mover::sub_d::NATIVE_TYPE30_SUB_D;
    use v2k_formats::collision::{
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    };
    let Some(initializer) = &metadata.initializer else {
        return Err(Type30Block::Metadata);
    };
    let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type30Block::Metadata);
    };
    let refs = [
        [32, 34, 36],
        [33, 35, 37],
        [32, 34, 36],
        [33, 35, 37],
        [30, 32, 34],
        [31, 33, 35],
        [32, 34, 36],
        [33, 35, 37],
        [32, 34, 36],
        [33, 35, 37],
    ];
    let dependencies = [
        [8, 1, 2, 2],
        [9, 0, 3, 3],
        [0, 3, 4, 4],
        [1, 2, 5, 5],
        [2, 5, 6, 6],
        [3, 4, 7, 7],
        [4, 7, 8, 8],
        [5, 6, 9, 9],
        [6, 9, 0, 0],
        [7, 8, 1, 1],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(12_000)
        || metadata.damage_profile.is_none()
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || Intro2KlComponents::from_native_constructor(metadata).is_err()
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 200,
            }))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 10_000,
            }))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 100,
                lift_range_raw: 50,
                strength_raw: 0x300000,
                near_boost_range_raw: 150,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(NATIVE_TYPE30_SUB_D))
        || metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(EMITTER))
        || metadata.sub_f_swimming_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.status_component_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_n_payload.is_some()
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.death_sound_id != RetailRuntimeValue::Known(Some(75))
        || metadata.accepted_hit_presentation_sound_id != RetailRuntimeValue::Known(Some(93))
        || initializer.initializer_state_flags_raw != 0x439
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 3
        || initializer.alternate_behavior_class_ref != 12
        || h.completion_sound_id.is_some()
        || h.records.len() != refs.len()
        || h.records.iter().enumerate().any(|(index, record)| {
            record.phase_rate_raw != 0x30000000
                || record.resolver_flags_raw != 0x40000000
                || record.axis_mode_raw != 0
                || record.vertex_refs != refs[index]
                || record.dependencies != dependencies[index]
        })
    {
        return Err(Type30Block::Metadata);
    }
    Ok(())
}
