//! Native dynamic louse Type56:104B0/09A80/D4A0/D720, classes7/4/10 and quiet death.
//!
//! A split child is a new allocation. It has no authored spawn index, retains
//! its own process Sub-D history, fourteen-row H and method30 Sub-E, and is
//! linked before its constructor returns to the parent's radial callback.

pub mod aim;
mod behavior;
pub(crate) mod construction;
pub mod contact;
pub mod death;
pub mod impact;
pub(crate) mod profile;
#[cfg(test)]
mod tests;

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime},
    entity::{Entity, EntityManager},
    entity_behavior::{BehaviorContextRuntime, BehaviorSelection},
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    generic_projectile_emitter::GenericEmitterRuntime,
    main_base_abort::MainBaseAbortActorLease,
    native_ground_actor as shared,
    resource_cache::ResourceCache,
    split_and_explode::SplitChildRequest,
    world_fx::WorldFx,
};
use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor, SubAPropulsionDescriptor,
    SubDSteeringDescriptor,
};

pub(crate) const MODEL: usize = 1041;
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3072,
    raw_word_at_0x04: 1,
};
pub(crate) const CHOICES: [BehaviorChoice; 3] = [
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 1,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 4,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 1,
        behavior_class_id: 10,
    },
];
pub(crate) const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
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
pub(crate) const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 30,
    random_interval_us: 300_000,
    spread_raw: 64,
    aim_threshold_raw: 32_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 2560,
    sound_id: 70,
    raw_word_at_0x12: 96,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type56Runtime {
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) birth: SplitChildRequest,
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) quiet_death_context: Option<BehaviorContextRuntime>,
}

impl Type56Runtime {
    pub const fn birth(self) -> SplitChildRequest {
        self.birth
    }
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn anchor_raw(self) -> [i16; 3] {
        self.anchor_raw
    }
}

pub type Type56Owner = shared::NativeGroundActorOwner<profile::Type56Profile>;
pub type Type56Outcome = shared::NativeGroundActorOutcome;
pub type Type56Tick = shared::NativeGroundActorTick<profile::Type56Profile>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type56Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type56BirthPublication {
    pub owner: Type56Owner,
    pub publication: Type56Publication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type56Block {
    Identity,
    Metadata,
    Allocation,
    Terrain,
    AlreadyPublished,
    Runtime(&'static str),
    Ground(shared::NativeGroundActorBlock),
}

pub struct Type56Frame<'a> {
    pub resources: &'a mut ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn tick_type56(
    manager: &mut EntityManager,
    owner: Type56Owner,
    frame: Type56Frame<'_>,
) -> Type56Tick {
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
    entity.native_type56_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == 56
            && entity.authored_spawn_index.is_none()
            && entity.id == runtime.allocation.entity_id
            && runtime.birth.entity_type == 56
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
                observation.lease == entity.native_type56_runtime.unwrap().allocation
            })
}

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type56Block> {
    let initializer = metadata.initializer.as_ref().ok_or(Type56Block::Metadata)?;
    let RetailRuntimeValue::Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type56Block::Metadata);
    };
    let RetailRuntimeValue::Known(Some(c)) = metadata.sub_c_lift_descriptor else {
        return Err(Type56Block::Metadata);
    };
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(6000)
        || metadata.damage_profile.is_none()
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != RetailRuntimeValue::Known(crate::entity_collision_state::CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: None,
            })
        || metadata.sub_f_swimming_descriptor != RetailRuntimeValue::Known(None)
        || metadata.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || metadata.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(EMITTER))
        || metadata.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || metadata.status_component_descriptor != RetailRuntimeValue::Known(None)
        || metadata.model_variable_count_raw != RetailRuntimeValue::Known(0)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 600,
            }))
        || !matches!(
            metadata.sub_b_lateral_descriptor,
            RetailRuntimeValue::Known(Some(_))
        )
        || c.surface_mode_raw != 0
        || c.offset_sample_raw != 0
        || c.reserved_at_0x0e != [0; 2]
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(SubDSteeringDescriptor {
                steering_divisor_raw: 64,
                couple_yaw_into_roll_raw: 0,
                enable_pitch_steering_raw: 0,
                forward_probe_raw: 512,
                lateral_probe_raw: 256,
                classifier_flags: 19,
                reserved_at_0x0b: 0,
            }))
        || initializer.initializer_state_flags_raw != 0x39
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 2
        || h.records.len() != 14
        || h.records.iter().enumerate().any(|(index, record)| {
            record.phase_rate_raw != 0x3000_0000
                || record.resolver_flags_raw != if index == 8 { 0x0400_0000 } else { 0x4000_0000 }
        })
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || !matches!(metadata.common_world_effects, RetailRuntimeValue::Known(_))
        || !matches!(metadata.death_sound_id, RetailRuntimeValue::Known(_))
        || !matches!(
            metadata.run_away_optional_sound_id,
            RetailRuntimeValue::Known(_)
        )
        || !matches!(
            metadata.run_away_sound_period_raw,
            RetailRuntimeValue::Known(_)
        )
    {
        return Err(Type56Block::Metadata);
    }
    Ok(())
}

pub(crate) fn emitter_constructor() -> GenericEmitterRuntime {
    GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: 30,
        emitter_selector: 0,
        sound_id: 70,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    }
}
