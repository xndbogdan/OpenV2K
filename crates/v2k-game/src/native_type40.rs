//! Authored Alpine ice-louse104B0/D4A0: its own ABCDEH, classes7/9 and class18.
//!
//! A true absent Sub-J is retained. Class9 pursuit is shared; C910's full/null-J
//! person-kill branch belongs to the native pair host, never a transport alias.

pub mod aim;
mod behavior;
mod construction;
pub mod contact;
pub mod death;
pub(crate) mod impact;
pub(crate) mod profile;
#[cfg(test)]
mod tests;
pub(crate) use construction::{publish_native_type40, NativeType40ConstructionRequest};

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
    world_fx::WorldFx,
};
use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor, SubAPropulsionDescriptor,
    SubBLateralDescriptor, SubCLiftDescriptor, SubDSteeringDescriptor,
};

pub(crate) const MODEL: usize = 1040;
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 5120,
    raw_word_at_0x04: 3,
};
pub(crate) const CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 1,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 9,
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
    projectile_method: 1,
    random_interval_us: 600_000,
    spread_raw: 256,
    aim_threshold_raw: 16_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 5120,
    sound_id: 0,
    raw_word_at_0x12: 0,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type40Runtime {
    pub(crate) entity_id: u32,
    pub(crate) spawn_index: usize,
    pub(crate) model_slots: [Option<usize>; 4],
    pub(crate) anchor_raw: [i16; 3],
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: GenericEmitterRuntime,
    pub(crate) split_terminal: Option<death::Type40SplitTerminalState>,
}
impl Type40Runtime {
    pub const fn allocation(self) -> MainBaseAbortActorLease {
        self.allocation
    }
    pub const fn spawn_index(self) -> usize {
        self.spawn_index
    }
    pub const fn anchor_raw(self) -> [i16; 3] {
        self.anchor_raw
    }
    pub const fn split_terminal(self) -> Option<death::Type40SplitTerminalState> {
        self.split_terminal
    }
}
pub type Type40Owner = shared::NativeGroundActorOwner<profile::Type40Profile>;
pub type Type40Block = shared::NativeGroundActorBlock;
pub type Type40Outcome = shared::NativeGroundActorOutcome;
pub type Type40Tick = shared::NativeGroundActorTick<profile::Type40Profile>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type40Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub initializer_fallback: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type40Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    Selection,
    Runtime(&'static str),
}

pub struct Type40Frame<'a> {
    pub resources: &'a ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
    pub tasks: &'a mut dyn shared::NativeGroundTaskCustody,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
}
pub fn tick_type40(
    manager: &mut EntityManager,
    owner: Type40Owner,
    frame: Type40Frame<'_>,
) -> Type40Tick {
    shared::tick_native_ground_actor(
        manager,
        owner,
        shared::NativeGroundActorFrame {
            resources: shared::NativeGroundResources::ReadOnly(frame.resources),
            world_fx: frame.world_fx,
            elapsed_micros: frame.elapsed_micros,
            retail_tick: frame.retail_tick,
            capture: shared::NativeCaptureDispatch::AbsentJ {
                tasks: frame.tasks,
                notifications: frame.notifications,
            },
        },
    )
}
pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type40_runtime.is_some_and(|r| {
        entity.active
            && entity.id == r.entity_id
            && entity.entity_type == 40
            && entity.authored_spawn_index == Some(r.spawn_index)
            && r.allocation.entity_id == entity.id
            && entity.model_slots == r.model_slots
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == 8
    })
}
pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|e| e.id == id) else {
        return false;
    };
    allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|o| o.lease == entity.native_type40_runtime.unwrap().allocation)
}

/// Canonical normal-tier section12 Type40 record: exact source-used components.
pub(crate) fn authenticate_metadata(m: &EntityTypeRuntimeMetadata) -> Result<(), Type40Error> {
    let Some(initializer) = &m.initializer else {
        return Err(Type40Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(h)) = &m.sub_h_external_frame_descriptor else {
        return Err(Type40Error::Metadata);
    };
    if m.common_mover_gkl_payloads
        != RetailRuntimeValue::Known(crate::entity_collision_state::CommonMoverGklPayloads {
            sub_g: None,
            sub_k: None,
            sub_l: None,
        })
    {
        return Err(Type40Error::Metadata);
    }
    if m.model_slots != [MODEL as u16; 4]
        || m.mass_raw != 100
        || m.capability_flags != 8
        || m.initial_health_raw != Some(8000)
        || m.damage_profile.is_none()
        || m.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || m.projectile_emitter_descriptor != RetailRuntimeValue::Known(Some(EMITTER))
        || m.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 200,
            }))
        || m.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 10_000,
            }))
        || m.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 50,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || m.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(SubDSteeringDescriptor {
                steering_divisor_raw: 128,
                couple_yaw_into_roll_raw: 0,
                enable_pitch_steering_raw: 0,
                forward_probe_raw: 512,
                lateral_probe_raw: 256,
                classifier_flags: 19,
                reserved_at_0x0b: 0,
            }))
        || m.sub_j_attachment_descriptor != RetailRuntimeValue::Known(None)
        || m.sub_f_swimming_descriptor != RetailRuntimeValue::Known(None)
        || m.actor_animation_descriptor != RetailRuntimeValue::Known(None)
        || m.status_component_descriptor != RetailRuntimeValue::Known(None)
        || m.model_variable_count_raw != RetailRuntimeValue::Known(0)
        || m.sub_n_payload.is_some()
        || m.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
        || m.death_sound_id != RetailRuntimeValue::Known(None)
        || initializer.common_axis_descriptor != AXIS
        || initializer.initializer_state_flags_raw != 0x39
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 18
        || h.records.len() != 14
    {
        return Err(Type40Error::Metadata);
    }
    for (i, row) in h.records.iter().enumerate() {
        let expected_flags = if i == 8 { 0x0400_0000 } else { 0x4000_0000 };
        if row.phase_rate_raw != 0x3000_0000 || row.resolver_flags_raw != expected_flags {
            return Err(Type40Error::Metadata);
        }
    }
    Ok(())
}
pub(crate) fn emitter_constructor() -> GenericEmitterRuntime {
    GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: 1,
        emitter_selector: 0,
        sound_id: 0,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    }
}
