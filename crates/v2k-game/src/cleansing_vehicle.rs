//! Type49 rover: source-owned cleansing, stationary and carrying graphs.

pub(crate) mod cargo;
pub mod impact;
mod live;
pub(crate) mod mover;
mod native;
#[cfg(test)]
mod radial_tests;
pub(crate) mod tasks;
#[cfg(test)]
mod tests;
pub use live::{
    tick_cleansing_vehicle, CleansingVehicleBlock, CleansingVehicleFrame, CleansingVehicleOutcome,
    CleansingVehicleOwner, CleansingVehicleTick,
};
pub(crate) use native::{publish_cleansing_vehicle, CleansingVehicleConstruction};

use crate::{
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime, CLEANSING_VEHICLE_SUB_D},
    entity::{Entity, EntityManager},
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    main_base_abort::MainBaseAbortActorLease,
};
use v2k_formats::collision::{
    BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
    SubCLiftDescriptor,
};

pub(crate) const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: true,
    sub_b: true,
    sub_c: true,
    sub_d: true,
    sub_e: false,
    sub_f: false,
    sub_g: false,
    sub_h: false,
    sub_i: false,
    sub_j: false,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3072,
    raw_word_at_0x04: 0,
};
pub(crate) const CHOICES: [BehaviorChoice; 2] = [
    BehaviorChoice {
        weight_rule_id: 12,
        weight_multiplier: 32000,
        behavior_class_id: 68,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 42,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CleansingVehicleRuntime {
    pub(crate) allocation: MainBaseAbortActorLease,
    pub(crate) spawn_index: Option<usize>,
    pub(crate) immutable_anchor_raw: [i16; 3],
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleansingVehicleError {
    Allocation,
    Metadata,
    AlreadyPublished,
    Graph,
    Runtime(&'static str),
    NearbyEvidence,
    UnsupportedDeath,
}

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), CleansingVehicleError> {
    let Some(init) = &metadata.initializer else {
        return Err(CleansingVehicleError::Metadata);
    };
    if metadata.model_slots != [266; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 0x1204
        || metadata.initial_health_raw != Some(2000)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 500,
            }))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 100000,
                correction_rate_raw: 100000,
            }))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 100,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 1,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(CLEANSING_VEHICLE_SUB_D))
        || metadata.terrain_contact_task_lifetime_ms != RetailRuntimeValue::Known(0)
        || init.initializer_state_flags_raw != 0x8039
        || init.common_axis_descriptor != AXIS
        || init.behavior_choices.as_ref() != CHOICES
        || init.behavior_rule_ref != 1
        || init.alternate_behavior_class_ref != 49
    {
        return Err(CleansingVehicleError::Metadata);
    }
    Ok(())
}

pub(crate) fn entity_authenticates(entity: &Entity) -> bool {
    entity.cleansing_vehicle_runtime.is_some_and(|runtime| {
        entity.active
            && entity.entity_type == 49
            && entity.id == runtime.allocation.entity_id
            && entity.authored_spawn_index == runtime.spawn_index
            && entity.model_slots == [Some(266); 4]
    })
}

pub(crate) fn allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    entity_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                Some(observation.lease) == entity.cleansing_vehicle_runtime.map(|r| r.allocation)
            })
}
