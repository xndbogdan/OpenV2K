//! Native ordinary Type18 construction through 104B0/09A80/D4A0/AC60.
//!
//! Nineteen authored rows (worlds 16, 21, 22, 23, 30, 33, 34 and 36) own
//! A/B/C/D/E/H/J. The weighted root is Defecate Virus 4, Capture People 9,
//! Search And Attack 7, Follow Beacons 33 and Trash Furniture 26, each on its
//! existing shared task program. Method20, `+C0` 0x431 (no drag) and Sub-D
//! classifier 0x10 are this row's own policies; the shared ground host runs
//! the mover, tasks, contacts and the class12 death.

pub mod aim;
mod construction;
pub mod contact;
pub(crate) mod impact;
mod live;
pub(crate) mod profile;
#[cfg(test)]
mod tests;
pub(crate) use construction::{publish_native_type18, NativeType18ConstructionRequest};
pub use live::{tick_type18, Type18Block, Type18Frame, Type18Outcome, Type18Owner, Type18Tick};

use crate::{
    actor_task_owner::ActorTaskSlot,
    common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime, NATIVE_TYPE18_SUB_D},
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
        BehaviorChoice, CommonAxisDescriptor, ProjectileEmitterDescriptor,
        SubAPropulsionDescriptor, SubBLateralDescriptor, SubCLiftDescriptor,
    },
    terrain::TerrainGrid,
};

pub(crate) const ENTITY_TYPE: u32 = 18;
pub(crate) const MODEL: usize = 35;
pub(crate) const HEALTH: i32 = 8000;
/// `+C0` 0x431: E640, gravity, D4A0 ground and the generic crush 0x400, but
/// no drag bit 8 (E100 skips 44EC60) and no terrain/water lane.
pub(crate) const DEFAULT_FLAGS: u32 = 0x431;
pub(crate) const EMITTER: ProjectileEmitterDescriptor = ProjectileEmitterDescriptor {
    projectile_method: 20,
    random_interval_us: 500_000,
    spread_raw: 100,
    aim_threshold_raw: 30_000,
    speed_override_raw: 0,
    target_axis_tolerance_raw: 3072,
    sound_id: 69,
    raw_word_at_0x12: 156,
    alternate_emitter_raw: 0,
    stochastic_gate_mode: 0,
    auxiliary_command: 0,
    variable_bindings: [0; 4],
};
/// Candidate mask 0x25: the player, Main Base 0x20 and 0x04 buildings.
pub(crate) const AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 4352,
    raw_word_at_0x04: 0x25,
};
pub(crate) const CHOICES: [BehaviorChoice; 5] = [
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 1,
        behavior_class_id: 4,
    },
    BehaviorChoice {
        weight_rule_id: 10,
        weight_multiplier: 8,
        behavior_class_id: 9,
    },
    BehaviorChoice {
        weight_rule_id: 6,
        weight_multiplier: 20,
        behavior_class_id: 7,
    },
    BehaviorChoice {
        weight_rule_id: 1,
        weight_multiplier: 3,
        behavior_class_id: 33,
    },
    BehaviorChoice {
        weight_rule_id: 8,
        weight_multiplier: 1,
        behavior_class_id: 26,
    },
];
pub(crate) const TOPOLOGY: CommonMoverComponentTopology = crate::native_type122::TOPOLOGY;
const SUB_H_RECORDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type18Runtime {
    entity_id: u32,
    pub(crate) spawn_index: usize,
    anchor_raw: [i16; 3],
    allocation: MainBaseAbortActorLease,
    pub(crate) sub_d_runtime: Type9SubDRuntime,
    pub(crate) sub_d_owner: Type9SubDFrameOwner,
    pub(crate) sub_e_runtime: crate::generic_projectile_emitter::GenericEmitterRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type18Publication {
    pub selection: BehaviorSelection,
    pub selector_word: u32,
    pub people_nearby: bool,
    pub player_nearby: bool,
    pub furniture_nearby: bool,
    pub initializer_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type18Error {
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
    entity.native_type18_runtime.is_some_and(|runtime| {
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
                observation.lease == entity.native_type18_runtime.unwrap().allocation
            })
}

pub(crate) fn authenticate_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Type18Error> {
    use crate::damage::DamageProfile;
    use crate::entity_collision_state::{CommonMoverGklPayloads, CommonWorldEffectProfile};
    use RetailRuntimeValue::Known;
    let Some(initializer) = &metadata.initializer else {
        return Err(Type18Error::Metadata);
    };
    let Known(Some(h)) = &metadata.sub_h_external_frame_descriptor else {
        return Err(Type18Error::Metadata);
    };
    let Known(Some(j)) = &metadata.sub_j_attachment_descriptor else {
        return Err(Type18Error::Metadata);
    };
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
        || metadata.initial_health_raw != Some(HEALTH)
        || metadata.damage_profile
            != Some(DamageProfile {
                thresholds_raw: [0, 2000, 2500, 500, 200, 0, 0],
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
        || metadata.projectile_emitter_descriptor != Known(Some(EMITTER))
        || metadata.actor_animation_descriptor != Known(None)
        || metadata.status_component_descriptor != Known(None)
        || metadata.model_variable_count_raw != Known(0)
        || metadata.sub_n_payload.is_some()
        || metadata.constructor_sound_attachment_id != Known(None)
        || metadata.accepted_hit_presentation_sound_id != Known(Some(87))
        || metadata.infected_model_presentation_sound_id != Known(None)
        || metadata.generic_hit_sound_id != Known(None)
        || metadata.death_sound_id != Known(Some(75))
        || metadata.target_warning_sound_id != Known(None)
        || metadata.search_attack_optional_prelude_sound_id != Known(None)
        // Aim's sound82 has period0, below 0400: it never draws or plays.
        || metadata.search_attack_aim_sound_id != Known(Some(82))
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
                acceleration_raw: 1300,
                overspeed_correction_raw: -2000,
                target_speed_base_raw: 300,
            }))
        || metadata.sub_b_lateral_descriptor
            != Known(Some(SubBLateralDescriptor {
                projection_threshold_rate_raw: 1_000_000,
                correction_rate_raw: 1_000_000,
            }))
        || metadata.sub_c_lift_descriptor
            != Known(Some(SubCLiftDescriptor {
                base_clearance_raw: 25,
                lift_range_raw: 75,
                strength_raw: 0x40_0000,
                near_boost_range_raw: 100,
                damping_range_raw: 200,
                surface_mode_raw: 0,
                offset_sample_raw: 0,
                reserved_at_0x0e: [0; 2],
            }))
        || metadata.sub_d_steering_descriptor != Known(Some(NATIVE_TYPE18_SUB_D))
        || initializer.initializer_state_flags_raw != DEFAULT_FLAGS
        || initializer.common_axis_descriptor != AXIS
        || initializer.behavior_choices.as_ref() != CHOICES
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || h.completion_sound_id.is_some()
        || h.records.len() != SUB_H_RECORDS
        || h.records.iter().enumerate().any(|(index, record)| {
            let base = 60 + index as u16;
            record.resolver_flags_raw != 0x2000_0000
                || record.phase_rate_raw != 0x1000_0000
                || record.vertex_refs != [base, base + 8, base + 16]
                || record.axis_mode_raw != 0
                || record.dependencies != dependencies[index]
        })
        || j.reserved_at_0x01 != 0
        || j.slots.len() != 1
        || j.slots[0].policy_word_raw != 1
        || j.slots[0].local_offset_raw != [0, 20, 350]
    {
        return Err(Type18Error::Metadata);
    }
    Ok(())
}

/// 24E30 clears the emitter then copies method and sound; bindings are absent.
fn emitter_constructor() -> crate::generic_projectile_emitter::GenericEmitterRuntime {
    crate::generic_projectile_emitter::GenericEmitterRuntime {
        joint_bindings: [None; 2],
        projectile_method: 20,
        emitter_selector: 0,
        sound_id: 69,
        direct_mode: 0,
        remaining_time_raw: 0,
        manual_step_raw: 0,
        remaining_bursts_raw: 0,
        cadence_raw: 0,
        basis_adjustment_identity: None,
    }
}

/// The authored terrain, as resident when 104B0 runs.
pub(crate) struct Type18BirthWorld<'a> {
    pub terrain: &'a TerrainGrid,
    pub objects: Option<&'a TerrainObjectTable>,
}

/// Execute the source-ordered component, selector and task suffixes.
fn publish_birth(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    preceding: &[Entity],
    receipt: Type18Runtime,
    world: Type18BirthWorld<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Type18Publication, Type18Error> {
    let prefix: Vec<_> = preceding
        .iter()
        .filter(|entity| entity.active)
        .map(shared::candidate)
        .collect();
    let sub_h = SubHRuntimeState::new(SUB_H_RECORDS).map_err(|_| Type18Error::ComponentStorage)?;
    let RetailRuntimeValue::Known(Some(sub_a_descriptor)) = metadata.sub_a_propulsion_descriptor
    else {
        unreachable!("authenticated Type18 Sub-A");
    };
    // 09A80 constructs H and D before A. 20450 consumes its own word even
    // though each later 06070 suffix overwrites the randomized target speed.
    entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(sub_h));
    entity.native_type18_runtime = Some(receipt);
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
    let people_nearby = shared::nearby::<profile::Type18Profile>(owner, &prefix, 0x0c00)
        .map_err(|_| Type18Error::NearbyEvidence)?;
    let player_nearby = shared::nearby::<profile::Type18Profile>(owner, &prefix, 1)
        .map_err(|_| Type18Error::NearbyEvidence)?;
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
            BehaviorWeightRule::PlayerNearby => i32::from(player_nearby),
            BehaviorWeightRule::FurnitureNearby => i32::from(furniture_nearby),
            _ => unreachable!("authenticated Type18 choices"),
        },
        || {
            selector_word = next_random();
            selector_word
        },
    )
    .map_err(|_| Type18Error::Selection)?
    .ok_or(Type18Error::Selection)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Type18Error::Selection)?;

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
    Ok(Type18Publication {
        selection,
        selector_word,
        people_nearby,
        player_nearby,
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
    matches!(selection.program.class_id, 4 | 7 | 9 | 26 | 33)
        && shared::publication::publish_shared_root_class(
            entity,
            metadata,
            selection,
            context,
            AXIS,
            next_random,
        )
}
