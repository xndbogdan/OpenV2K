//! 104B0/09A80 component construction, D4A0 placement, and AC60 initial choice.

use super::*;
use crate::{
    actor_task_dispatcher::{
        prepare_follow_beacons_acquiring_runtime_task, prepare_shared_acquiring_runtime_task,
        SharedGenericConstructorEffect,
    },
    actor_task_owner::ActorTaskSlot,
    common_mover::{
        sub_d::{type17_first_query_owner_for_seed, Type9SubDFrameOwner, Type9SubDRuntime},
        type9_attitude::Type9BodyBasis,
        SubAPropulsionRuntime,
    },
    entity_behavior::{initial_behavior_state_policy, BehaviorContextRuntime},
    entity_collision_state::{
        EntityTypeRuntimeMetadata, RetailRuntimeValue, BODY_BASIS_REBUILT_STATE_BIT,
    },
    follow_beacons::apply_follow_beacons_acquiring_task_setup,
    run_away::{apply_run_away_task_setup, RunAwayTaskSetupRequest},
    sub_h_external_frame::SubHRuntimeState,
    type17_impact_live::TYPE17_MODEL256_COMPONENT_TOPOLOGY,
    type17_impact_reselection::{plan_type17_weighted_selection, Type17WeightedSelectionRequest},
    type17_initial_behavior_live::fresh_type17_candidate_ref,
};
use v2k_formats::{
    collision::{
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor, SubDSteeringDescriptor,
    },
    terrain::TerrainGrid,
};

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type17Error> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2Type17Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Intro2Type17Error::Metadata);
    };
    let RetailRuntimeValue::Known(Some(sub_j)) = &metadata.sub_j_attachment_descriptor else {
        return Err(Intro2Type17Error::Metadata);
    };
    let refs = [
        [0, 56, 64],
        [1, 57, 65],
        [52, 58, 66],
        [53, 59, 67],
        [54, 60, 68],
        [55, 61, 69],
        [150, 62, 70],
        [151, 63, 71],
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
        || metadata.mass_raw != 100
        || metadata.capability_flags != 8
        || metadata.initial_health_raw != Some(5000)
        || metadata.common_mover_topology
            != RetailRuntimeValue::Known(TYPE17_MODEL256_COMPONENT_TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(SubAPropulsionDescriptor {
                acceleration_raw: 1500,
                overspeed_correction_raw: -3000,
                target_speed_base_raw: 250,
            }))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 10000,
                correction_rate_raw: 1000,
            }))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 75,
                lift_range_raw: 75,
                strength_raw: 0x300000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0, 0],
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
        || initializer.behavior_choices.as_ref() != INITIAL_CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || sub_h.completion_sound_id.is_some()
        || sub_h.records.len() != refs.len()
        || sub_h.records.iter().enumerate().any(|(i, record)| {
            record.resolver_flags_raw != 0x40000000
                || record.phase_rate_raw != 0x0c000000
                || record.vertex_refs != refs[i]
                || record.axis_mode_raw != 0
                || record.dependencies != dependencies[i]
        })
        || sub_j.slots.len() != 1
        || sub_j.slots[0].policy_word_raw != 1
        || sub_j.slots[0].local_offset_raw != [0, 10, 110]
    {
        return Err(Intro2Type17Error::Metadata);
    }
    Ok(())
}

pub(crate) fn publish_intro2_type17(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    terrain: &TerrainGrid,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type17Publication, Intro2Type17Error> {
    let Some(spawn) = entity.authored_spawn_index else {
        return Err(Intro2Type17Error::Identity);
    };
    let (xz, sub_d_seed) = match spawn {
        4 => ([-29696, -28672], 0x04),
        30 => ([-16128, -32768], 0x17),
        _ => return Err(Intro2Type17Error::Identity),
    };
    if !entity.active
        || entity.entity_type != 17
        || entity.model_slots != [Some(MODEL); 4]
        || [entity.position_raw()[0], entity.position_raw()[2]] != xz
    {
        return Err(Intro2Type17Error::Identity);
    }
    authenticate_metadata(metadata)?;
    validate_unpublished(entity)?;
    validate_component_storage(entity)?;
    if preceding.iter().any(|candidate| {
        candidate
            .authored_spawn_index
            .is_none_or(|index| index >= spawn)
    }) {
        return Err(Intro2Type17Error::Prefix);
    }
    let RetailRuntimeValue::Known(anchor_raw) = crate::entity_initializer::constructor_position_raw(
        metadata,
        entity.position_raw(),
        Some(terrain),
        None,
    ) else {
        return Err(Intro2Type17Error::Metadata);
    };
    let receipt = Intro2Type17Runtime {
        entity_id: entity.id,
        spawn_index: spawn,
        model_slots: entity.model_slots,
        anchor_raw,
        origin: Type17ConstructionOrigin::CapturedIntro2,
    };
    publish_birth(
        Type17PreparedBirth {
            entity,
            metadata,
            preceding,
            receipt,
            sub_d_runtime: Type9SubDRuntime::from_constructor(),
            // These are the own constructor/register/first-query joins in
            // V200002, distinct from native process allocation policy.
            sub_d_frame_owner: type17_first_query_owner_for_seed(sub_d_seed)
                .expect("audited Intro2 seed"),
            retail_tick: 0,
        },
        next_random,
    )
}

pub(super) fn validate_unpublished(entity: &Entity) -> Result<(), Intro2Type17Error> {
    if entity.intro2_type17_runtime.is_some()
        || entity.type17_sub_d_runtime.is_some()
        || entity.type17_sub_d_frame_owner.is_some()
        || entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type17Error::AlreadyPublished);
    }
    Ok(())
}

pub(super) fn validate_component_storage(entity: &Entity) -> Result<(), Intro2Type17Error> {
    if entity.collision.last_hit_presentation_tick_at_0x34 != RetailRuntimeValue::Known(0)
        || entity.collision.recent_relation_id_at_0x60 != RetailRuntimeValue::Known(None)
        || !matches!(entity.sub_a_propulsion_runtime, RetailRuntimeValue::Known(Some(sub_a)) if sub_a.drive_scale_percent() == 100)
        || !matches!(&entity.sub_j_attachment_runtime, RetailRuntimeValue::Known(Some(sub_j)) if sub_j.is_empty() && sub_j.authored_slot_count() == 1)
        || entity.actor_common_axis_descriptor != RetailRuntimeValue::Known(AXIS)
    {
        return Err(Intro2Type17Error::ComponentStorage);
    }
    Ok(())
}

/// Both origin policies enter the same component/selector/task constructor.
/// Admission remains outside this phase so malformed input consumes no RNG;
/// once 20450 runs, a later selector failure preserves that committed prefix.
pub(super) struct Type17PreparedBirth<'a> {
    pub entity: &'a mut Entity,
    pub metadata: &'a EntityTypeRuntimeMetadata,
    pub preceding: &'a [Entity],
    pub receipt: Intro2Type17Runtime,
    pub sub_d_runtime: Type9SubDRuntime,
    pub sub_d_frame_owner: Type9SubDFrameOwner,
    pub retail_tick: u32,
}

pub(super) fn publish_birth(
    request: Type17PreparedBirth<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type17Publication, Intro2Type17Error> {
    let Type17PreparedBirth {
        entity,
        metadata,
        preceding,
        receipt,
        sub_d_runtime,
        sub_d_frame_owner,
        retail_tick,
    } = request;
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(fresh_type17_candidate_ref)
        .collect();
    let sub_h = SubHRuntimeState::new(8).map_err(|_| Intro2Type17Error::ComponentStorage)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("authenticated Type17 Sub-A");
    };
    entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
        SubAPropulsionRuntime::from_20450_constructor(sub_a_descriptor, next_random() as u16),
    ));
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.type17_sub_d_runtime = Some(sub_d_runtime);
    entity.type17_sub_d_frame_owner = Some(sub_d_frame_owner);
    // D4A0 and its +90 copy precede 381F0/AC60's nearby predicates.
    entity.set_position_raw(receipt.anchor_raw);
    let owner = fresh_type17_candidate_ref(entity);
    let weighted = plan_type17_weighted_selection(
        Type17WeightedSelectionRequest {
            active_model_id: MODEL as u16,
            current_tick: retail_tick,
            last_hit_tick: 0,
            metadata,
            owner,
            candidates_in_intrusive_order: &prefix,
        },
        &mut *next_random,
    )
    .map_err(Intro2Type17Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(weighted.selection)
        .expect("authenticated Type17 choices have named initial styles");

    entity.intro2_type17_runtime = Some(receipt);
    // Explicit native policy for unwritten allocator residue at +B2. This
    // birth-only write never resets later animation/mass contributions.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.initial_behavior = RetailRuntimeValue::Known(Some(weighted.selection));
    let succeeded = publish_acquiring(entity, metadata, weighted.selection, context, next_random);
    // 104B0's successful wrapper builds the physical basis after the selected
    // initializer, from the actual authored Euler words.
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    Ok(Intro2Type17Publication {
        selection: weighted.selection,
        selector_word: weighted.random_word,
        people_nearby: weighted.evaluator_evidence.people_nearby.weight() != 0,
        player_nearby: weighted.evaluator_evidence.player_nearby.weight() != 0,
        initializer_fallback: !succeeded,
    })
}

/// ABB0/C6B0 initializer phase shared by birth and authenticated C690 reentry.
/// The caller retains allocation/component custody and the selected context;
/// this phase does not recreate Sub-D/H/A or reset the transient +B2 word.
pub(super) fn publish_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    selection: BehaviorSelection,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> bool {
    debug_assert!(matches!(selection.program.class_id, 9 | 10 | 33));
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(selection.program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    if selection.program.class_id != 33 {
        // B6C0 resets only actor-local +04. B740 does not reset either word.
        let RetailRuntimeValue::Known(mut axis) = entity.actor_common_axis_descriptor else {
            unreachable!("caller retains authenticated actor-local axis");
        };
        axis.raw_word_at_0x04 = AXIS.raw_word_at_0x04;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(axis);
    }
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
