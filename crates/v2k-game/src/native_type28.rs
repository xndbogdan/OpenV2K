//! Native ordinary Type28 construction through 104B0/09A80/D4A0/AC60.
//!
//! Eleven authored rows (worlds 21, 22, 23, 30 and 36) own A/B/C/D/H/J and no
//! emitter. The weighted root is Capture People 9, Run Away 10, Trash
//! Furniture 26 and Follow Beacons 33, each on its existing shared task
//! program, with Type122's capture and class12 death. Sub-D classifier 0x10
//! is this row's own steering policy.

mod construction;
pub mod contact;
pub(crate) mod impact;
mod live;
pub(crate) mod profile;
#[cfg(test)]
mod tests;
pub(crate) use construction::{publish_native_type28, NativeType28ConstructionRequest};
pub use live::{tick_type28, Type28Block, Type28Frame, Type28Outcome, Type28Owner, Type28Tick};

use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime, NATIVE_TYPE28_SUB_D},
    entity::{Entity, EntityManager},
    entity_behavior::{
        select_initial_behavior, BehaviorContextRuntime, BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    main_base_abort::MainBaseAbortActorLease,
    native_ground_actor as shared,
    sub_h_external_frame::SubHRuntimeState,
};
use v2k_formats::{
    anim_frames::TerrainObjectTable,
    collision::{
        BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor, SubBLateralDescriptor,
        SubCLiftDescriptor,
    },
    terrain::TerrainGrid,
};

pub(crate) const ENTITY_TYPE: u32 = 28;
pub(crate) const MODEL: usize = 31;
pub(crate) const HEALTH: i32 = 5000;
/// `+C0` 0x439: Type122's ground policy, with drag and the generic crush.
pub(crate) const DEFAULT_FLAGS: u32 = 0x439;
/// Candidate mask 5: the player and 0x04 buildings.
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3840,
    raw_word_at_0x04: 5,
};
pub(crate) const CHOICES: [BehaviorChoice; 4] = [
    BehaviorChoice {
        weight_rule_id: 10,
        weight_multiplier: 9,
        behavior_class_id: 9,
    },
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 20,
        behavior_class_id: 10,
    },
    BehaviorChoice {
        weight_rule_id: 8,
        weight_multiplier: 5,
        behavior_class_id: 26,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 3,
        behavior_class_id: 33,
    },
];
/// A/B/C/D/H/J: Type122's topology without the emitter.
pub(crate) const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_e: false,
    ..crate::native_type122::TOPOLOGY
};
const SUB_H_RECORDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type28Runtime {
    entity_id: u32,
    pub(crate) spawn_index: usize,
    anchor_raw: [i16; 3],
    allocation: MainBaseAbortActorLease,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type28Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub under_attack: bool,
    pub furniture_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type28Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    NearbyEvidence,
    Selection,
    Runtime(&'static str),
}

pub(crate) fn allocation_authenticates(entity: &Entity) -> bool {
    entity.native_type28_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == ENTITY_TYPE
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == 8
            && runtime.allocation.entity_id == entity.id
    })
}

/// A native receipt belongs to its issuing manager generation.
pub(crate) fn manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    allocation_authenticates(entity)
        && manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| {
                observation.lease == entity.native_type28_runtime.unwrap().allocation
            })
}

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type28Error> {
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type28Error::Metadata);
    };
    let Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type28Error::Metadata);
    };
    let Known(Some(j)) = &metadata.sub_j_attachment_descriptor else {
        return Err(Type28Error::Metadata);
    };
    // Each walking leg depends on a cross-body partner, not its own group.
    let dependencies = [
        [2, 2, 0, 0],
        [3, 7, 1, 1],
        [4, 4, 2, 2],
        [5, 1, 3, 3],
        [6, 6, 4, 4],
        [7, 3, 5, 5],
        [0, 0, 6, 6],
        [1, 5, 7, 7],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 2000, 2000, 500, 200, 0, 0],
                multipliers_q8: [0, 256, 256, 128, 128, 0, 0],
            })
        || metadata.common_mover_topology != Known(TOPOLOGY)
        || metadata.common_mover_gkl_payloads
            != Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: None,
                sub_l: None,
            })
        || metadata.sub_f_swimming_descriptor != Known(None)
        || metadata.projectile_emitter_descriptor != Known(None)
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.model_variable_count_raw != Known(0)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.accepted_hit_presentation_sound_id != Known(Some(6))
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.death_sound_id != Known(Some(62))
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(None)
        || metadata.search_attack_aim_sound_id != Known(None)
        || metadata.search_attack_aim_sound_period_raw != Known(0)
        || metadata.run_away_optional_sound_id != Known(None)
        || metadata.run_away_sound_period_raw != Known(0)
        || metadata.terrain_contact_task_lifetime_ms != Known(0)
        || metadata.common_world_effects
            != Known(CommonWorldEffectProfile {
                surface_selectors: [0, 0],
                surface_lifetime_ms: 0,
                low_health_effect_words: [0; 3],
            })
        || metadata.sub_a_propulsion_descriptor
            != Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1800,
                overspeed_correction_raw: -2200,
                target_speed_base_raw: 550,
            }))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10_000,
                correction_rate_raw: 10_000,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 50,
                lift_range_raw: 75,
                strength_raw: 0x30_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor != Known(Some(NATIVE_TYPE28_SUB_D))
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || h.completion_sound_id.is_some()
        || h.records.len() != SUB_H_RECORDS
        || h.records.iter().enumerate().any(|(index, record)| {
            let base = 24 + index as u16;
            record.resolver_flags_raw != 0x2000_0000
                || record.phase_rate_raw != 0x1000_0000
                || record.vertex_refs != [base, base + 8, base + 16]
                || record.axis_mode_raw != 0
                || record.dependencies != dependencies[index]
        })
        || j.reserved_at_0x01 != 0
        || j.slots.len() != 1
        || j.slots[0].policy_word_raw != 1
        || j.slots[0].local_offset_raw != [0, 30, 100]
    {
        return Err(Type28Error::Metadata);
    }
    Ok(())
}

/// The authored terrain, as resident when 104B0 runs.
pub(crate) struct Type28BirthWorld<'a> {
    pub terrain: &'a TerrainGrid,
    pub objects: Option<&'a TerrainObjectTable>,
    pub retail_tick: u32,
}

/// Execute the source-ordered component, selector and task suffixes.
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    receipt: Type28Runtime,
    world: Type28BirthWorld<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type28Publication, Type28Error> {
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(shared::candidate)
        .collect();
    let sub_h = SubHRuntimeState::new(SUB_H_RECORDS).map_err(|_| Type28Error::ComponentStorage)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("authenticated Type28 Sub-A");
    };
    // 09A80 constructs H and D before A. 20450 consumes its own word even
    // though each later 06070 suffix overwrites the randomized target speed.
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.native_type28_runtime = Some(receipt);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(
            sub_a_descriptor,
            next_random() as u16,
        ),
    ));
    // D4A0's terrain snap and immutable +90 copy precede every 425680 rule.
    entity.set_position_raw(receipt.anchor_raw);
    let mut owner = shared::candidate(entity);
    owner.attached_entity_handle = entity.collision.recent_relation_id_at_0x60;
    let people_nearby = shared::nearby::<profile::Type28Profile>(owner, &prefix, 0x0c00)
        .map_err(|_| Type28Error::NearbyEvidence)?;
    // Fresh +34=0 makes 16490 false for every current tick: tick<250 and
    // signed tick>249 cannot both hold. Keep the actual clock at the reader.
    let under_attack = world.retail_tick < 250 && (world.retail_tick as i32) > 249;
    // Rule8 scans objects within a quarter of the authored strict axis.
    let furniture_nearby = crate::trash_furniture::find_furniture(
        world.terrain,
        world.objects,
        entity.position_raw(),
        AXIS.strict_axis_limit_raw / 4,
        -1,
    )
    .is_some();
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::PeopleNearby => i32::from(people_nearby),
            BehaviorWeightRule::UnderAttack => i32::from(under_attack),
            BehaviorWeightRule::FurnitureNearby => i32::from(furniture_nearby),
            _ => unreachable!("authenticated Type28 choices"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type28Error::Selection)?
    .ok_or(Type28Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type28Error::Selection)?;

    // Explicit native policy for the allocator's unwritten transient +B2.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_acquiring(entity, metadata, selection, context, next_random);
    // The successful 104B0 wrapper constructs the body matrix only after the
    // initializer, preserving every authored Euler word.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(heading, pitch, roll),
    );
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
    );
    Ok(Type28Publication {
        selection,
        selector_word,
        people_nearby,
        under_attack,
        furniture_nearby,
        initializer_fallback: !succeeded,
    })
}

/// AC60's selected initializer, on the ground host's shared programs.
pub(crate) fn publish_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    matches!(selection.program.class_id, 9 | 10 | 26 | 33)
        && shared::publication::publish_shared_root_class(
            entity,
            metadata,
            selection,
            context,
            AXIS,
            next_random,
        )
}
