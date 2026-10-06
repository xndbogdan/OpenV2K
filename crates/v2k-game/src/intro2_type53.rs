//! Shared Type53 construction through 104B0/D4A0/381F0/425680.
//!
//! Selection sees only earlier successful Section-13 allocations. The
//! Sub-A constructor consumes its word before selection. The selected
//! B6C0 or B740 initializer publishes its real acquiring tasks and both 06070
//! suffixes before the next actor is admitted. Later live acquisition/movement
//! remains separate; an inactive actor can still receive the ordinary death
//! callback using this retained allocation and its actual current style.

mod aim;
#[cfg(test)]
mod behavior;
mod capture_pursuit;
mod construction;
pub mod contact;
pub(crate) mod impact;
mod live;
mod mover;
mod profile;
mod search;
pub(crate) use construction::{publish_authored_type53, Type53AuthoredConstructionRequest};
pub use live::{
    tick_intro2_type53, Intro2Type53Block, Intro2Type53Frame, Intro2Type53Outcome,
    Intro2Type53Owner, Intro2Type53Tick,
};

use std::num::NonZeroU32;

use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_acquiring_runtime_task, prepare_shared_acquiring_runtime_task,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::{
        intro2_type53_first_query_owner_for_birth, Type9SubDFrameOwner, Type9SubDRuntime,
    },
    entity::{Entity, EntityManager},
    entity_behavior::{
        initial_behavior_state_policy, select_initial_behavior, BehaviorContextRuntime,
        BehaviorSelection, BehaviorWeightRule,
    },
    entity_collision_state::{
        CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
    },
    follow_beacons::apply_follow_beacons_acquiring_task_setup,
    guard_location_owner::acquisition::{
        select_guard_location_candidate, GuardLocationCandidateFilter,
        GuardLocationCandidateRequest, GuardLocationCandidateSelection, GuardLocationEntityRef,
        GuardLocationSearchContext,
    },
    main_base_abort::MainBaseAbortActorLease,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
    sub_h_external_frame::SubHRuntimeState,
    wrapped_axis_range::WrappedAxisRange,
};
use v2k_formats::{
    collision::{
        BehaviorChoice, CommonAxisDescriptor, SubAPropulsionDescriptor, SubDSteeringDescriptor,
    },
    terrain::TerrainGrid,
};

pub const INTRO2_TYPE53_SPAWN_INDICES: [usize; 4] = [20, 26, 38, 41];
const MODEL: usize = 302;
const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 3840,
    raw_word_at_0x04: 3077,
};
const CHOICES: [BehaviorChoice; 4] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 2,
        weight_multiplier: 20,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 10,
        weight_multiplier: 4,
        behavior_class_id: 9,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 5,
        behavior_class_id: 7,
    },
];
const TOPOLOGY: CommonMoverComponentTopology = CommonMoverComponentTopology {
    sub_a: true,
    sub_b: true,
    sub_c: true,
    sub_d: true,
    sub_e: false,
    sub_f: false,
    sub_g: false,
    sub_h: true,
    sub_i: false,
    sub_j: true,
    sub_k: false,
    sub_l: false,
    sub_m: false,
    sub_n: false,
    sub_o: false,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type53Runtime {
    entity_id: u32,
    spawn_index: usize,
    model_slots: [Option<usize>; 4],
    anchor_raw: [i16; 3],
    origin: Type53ConstructionOrigin,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type53ConstructionOrigin {
    Native(MainBaseAbortActorLease),
    CapturedIntro2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type53Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub player_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type53Error {
    Identity,
    Metadata,
    AlreadyPublished,
    ComponentStorage,
    Prefix,
    NearbyEvidence,
    Selection,
    Runtime(&'static str),
}

pub(crate) fn intro2_type53_allocation_authenticates(entity: &Entity) -> bool {
    entity.intro2_type53_runtime.is_some_and(|runtime| {
        entity.active
            && entity.id == runtime.entity_id
            && entity.entity_type == 53
            && entity.authored_spawn_index == Some(runtime.spawn_index)
            && entity.model_slots == runtime.model_slots
            && entity.model_slots == [Some(MODEL); 4]
            && entity.capability_flags == 8
            && match runtime.origin {
                Type53ConstructionOrigin::Native(lease) => lease.entity_id == entity.id,
                Type53ConstructionOrigin::CapturedIntro2 => {
                    INTRO2_TYPE53_SPAWN_INDICES.contains(&runtime.spawn_index)
                }
            }
    })
}

/// A native receipt belongs to its issuing manager generation. Current pose,
/// callback-private values and the mutable Sub-D cache are not identity.
pub(crate) fn type53_manager_allocation_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    if !intro2_type53_allocation_authenticates(entity) {
        return false;
    }
    match entity.intro2_type53_runtime.unwrap().origin {
        Type53ConstructionOrigin::Native(lease) => manager
            .main_base_abort_actor_observation(id)
            .is_some_and(|observation| observation.lease == lease),
        Type53ConstructionOrigin::CapturedIntro2 => true,
    }
}

fn candidate(entity: &Entity) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        // 22C10 does not read candidate relations. Owner relation is zero at birth.
        attached_entity_handle: RetailRuntimeValue::Known(None),
    }
}

fn nearby(
    owner: GuardLocationEntityRef,
    prefix: &[GuardLocationEntityRef],
    mask: u32,
) -> Result<bool, Intro2Type53Error> {
    select_guard_location_candidate(GuardLocationCandidateRequest {
        owner,
        candidates_in_intrusive_order: prefix,
        search_context: GuardLocationSearchContext::new(
            WrappedAxisRange::from_raw(AXIS.strict_axis_limit_raw),
            GuardLocationCandidateFilter::CapabilityMask(NonZeroU32::new(mask).unwrap()),
        ),
    })
    .map(|selection| matches!(selection, GuardLocationCandidateSelection::Selected(_)))
    .map_err(|_| Intro2Type53Error::NearbyEvidence)
}

fn authenticate_metadata(metadata: &EntityTypeRuntimeMetadata) -> Result<(), Intro2Type53Error> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2Type53Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Intro2Type53Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_j)) = &metadata.sub_j_attachment_descriptor else {
        return Err(Intro2Type53Error::Metadata);
    };
    let refs = [
        [86, 88, 94],
        [87, 89, 95],
        [148, 90, 96],
        [149, 91, 97],
        [2, 92, 98],
        [3, 93, 99],
    ];
    let dependencies = [
        [1, 2, 3, 4],
        [0, 2, 3, 5],
        [0, 1, 4, 5],
        [0, 1, 4, 5],
        [0, 2, 3, 5],
        [1, 2, 3, 4],
    ];
    if metadata.model_slots != [MODEL as u16; 4]
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(3000)
        || metadata.common_mover_topology != RetailRuntimeValue::Known(TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 400,
            }))
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
        || initializer.alternate_behavior_class_ref != 12
        || sub_h.completion_sound_id.is_some()
        || sub_h.records.len() != 6
        || sub_h.records.iter().enumerate().any(|(i, record)| {
            record.resolver_flags_raw != 0x20000000
                || record.phase_rate_raw != 0x30000000
                || record.vertex_refs != refs[i]
                || record.axis_mode_raw != 0
                || record.dependencies != dependencies[i]
        })
        || sub_j.slots.len() != 1
        || sub_j.slots[0].policy_word_raw != 1
        || sub_j.slots[0].local_offset_raw != [0, 0, 120]
    {
        return Err(Intro2Type53Error::Metadata);
    }
    Ok(())
}

/// Explicit replay entry; its fixed coordinates and first-query receipts must
/// never replace an ordinary/native process Sub-D allocation.
pub(crate) fn publish_intro2_type53(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type53Publication, Intro2Type53Error> {
    let Some(spawn) = entity.authored_spawn_index else {
        return Err(Intro2Type53Error::Identity);
    };
    let (xz, sub_d_seed) = match spawn {
        20 => ([-28160, -28928], 0x13),
        26 => ([-15360, 5632], 0x16),
        38 => ([-28672, -28928], 0x18),
        41 => ([-26880, 27136], 0x1a),
        _ => return Err(Intro2Type53Error::Identity),
    };
    if !entity.active
        || entity.entity_type != 53
        || entity.model_slots != [Some(MODEL); 4]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
    {
        return Err(Intro2Type53Error::Identity);
    }
    authenticate_metadata(metadata)?;
    if entity.intro2_type53_runtime.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type53Error::AlreadyPublished);
    }
    if entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || !matches!(entity.sub_a_propulsion_runtime, RetailRuntimeValue::Known(Some(sub_a)) if sub_a.drive_scale_percent() == 100)
        || !matches!(&entity.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(sub_j)) if sub_j.is_empty() && sub_j.authored_slot_count() == 1)
    {
        return Err(Intro2Type53Error::ComponentStorage);
    }
    if preceding.iter().any(|candidate| {
        candidate
            .authored_spawn_index
            .is_none_or(|index| index >= spawn)
    }) {
        return Err(Intro2Type53Error::Prefix);
    }
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        entity.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type53Error::Metadata);
    };
    let receipt = Intro2Type53Runtime {
        entity_id: entity.id,
        spawn_index: spawn,
        model_slots: entity.model_slots,
        anchor_raw,
        origin: Type53ConstructionOrigin::CapturedIntro2,
        sub_d_runtime: Type9SubDRuntime::from_constructor(),
        sub_d_owner: intro2_type53_first_query_owner_for_birth(spawn, sub_d_seed)
            .unwrap_or_else(|| Type9SubDFrameOwner::pending_constructor_origin(sub_d_seed)),
    };
    publish_birth(entity, metadata, preceding, receipt, 0, next_random)
}

/// Both birth policies execute the same source-ordered component, selector and
/// task suffixes. A later predicate failure retains the actual H/D/A prefix.
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    receipt: Intro2Type53Runtime,
    retail_tick: u32,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type53Publication, Intro2Type53Error> {
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(candidate)
        .collect();
    let sub_h = SubHRuntimeState::new(6).map_err(|_| Intro2Type53Error::ComponentStorage)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("authenticated Type53 Sub-A");
    };
    // 09A80 constructs H and D before A. 20450 consumes its own word even
    // though each later 06070 suffix overwrites the randomized target speed.
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.intro2_type53_runtime = Some(receipt);
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(
            sub_a_descriptor,
            next_random() as u16,
        ),
    ));
    // D4A0's terrain snap and immutable +90 copy precede all 381F0 predicates.
    entity.set_position_raw(receipt.anchor_raw);
    let owner = candidate(entity);
    let people_nearby = nearby(owner, &prefix, 0x0c00)?;
    let player_nearby = nearby(owner, &prefix, 1)?;
    // Fresh +34=0 makes 16490 false for every current tick: tick<250 and
    // signed tick>249 cannot both hold. Keep the actual clock at the reader.
    let under_attack = retail_tick < 250 && (retail_tick as i32) > 249;
    let mut selector_word = 0;
    let selection = select_initial_behavior(
        &CHOICES,
        |rule| match rule {
            BehaviorWeightRule::Always => 1,
            BehaviorWeightRule::UnderAttack => i32::from(under_attack),
            BehaviorWeightRule::PeopleNearby => i32::from(people_nearby),
            BehaviorWeightRule::PlayerNearby => i32::from(player_nearby),
            _ => unreachable!("authenticated Type53 choices"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Intro2Type53Error::Selection)?
    .ok_or(Intro2Type53Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type53Error::Selection)?;

    // Explicit native policy for the allocator's unwritten transient +B2.
    // This single birth write does not alter any later scheduler contribution.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    let succeeded = publish_acquiring(entity, metadata, selection, context, next_random);
    // The successful common104B0 wrapper constructs the body matrix only after
    // the initializer, preserving every authored Euler word.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(
        crate::common_mover::type9_attitude::Type9BodyBasis::from_angle_words(heading, pitch, roll),
    );
    entity.collision.state_flags_at_0x08.overwrite(
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
        crate::entity_collision_state::BODY_BASIS_REBUILT_STATE_BIT,
    );
    Ok(Intro2Type53Publication {
        selection,
        selector_word,
        people_nearby,
        player_nearby,
        initializer_fallback: !succeeded,
    })
}

fn publish_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(AXIS);
    let position_raw = entity.position_raw();

    let Entity {
        actor_tasks,
        sub_a_propulsion_runtime,
        sub_h_external_frame_runtime,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(sub_a)) = sub_a_propulsion_runtime else {
        unreachable!()
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = sub_h_external_frame_runtime else {
        unreachable!()
    };
    let mut apply = |effect| match effect {
        SharedGenericConstructorEffect::WriteSubHState08 { value } => sub_h.set_enabled(value != 0),
        SharedGenericConstructorEffect::WriteSubADirection {
            direction_multiplier,
        } => sub_a.set_direction_multiplier(direction_multiplier),
        SharedGenericConstructorEffect::WriteSubATargetSpeed {
            target_speed_raw, ..
        } => sub_a.apply_shared_initializer_target_speed_write(target_speed_raw),
    };
    let succeeded = if selection.program.class_id == 33 {
        apply_follow_beacons_acquiring_task_setup(actor_tasks, |preparation| {
            prepare_follow_beacons_acquiring_runtime_task(preparation, position_raw, metadata)
                .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
        })
        .is_ok()
    } else {
        // Classes7/9 share B6C0's exact two constructors; only style+44 differs.
        let RetailRuntimeValue::Known(filter) = selection.program.initializer_argument_raw else {
            unreachable!()
        };
        apply_run_away_task_setup(
            actor_tasks,
            RunAwayTaskSetupRequest::Acquiring,
            |preparation| {
                prepare_shared_acquiring_runtime_task(preparation, position_raw, metadata, filter)
                    .map(|prepared| prepared.apply_suffix(&mut *next_random, &mut apply))
            },
        )
        .is_ok()
    };
    if !succeeded {
        entity.publish_behavior_initializer_failure_fallback(context);
    }
    succeeded
}

#[cfg(test)]
pub(crate) mod authored_tests;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod tests;
