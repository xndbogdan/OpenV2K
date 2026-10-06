//! Shared class7 flyer mover/tasks for authored Intro2 Wasps/Deathwas and
//! allocation-authenticated native Type15 Hive children.
//!
//! Section-12 collision records in 1X3XX.OVL establish that both type 15 (wasp)
//! and type 87 (deathwas) author:
//! - Sub-B lateral descriptor: threshold 1_000_000, correction 1_000_000
//! - Sub-D steering descriptor: divisor 64, couple_yaw_into_roll 1, flags 0
//! - Sub-E projectile emitter (wasp: method 30, sound 70; deathwas: method 20, sound 92)
//! - Sub-G flight table (104 bytes, forces 20/30 or 20/40, speed caps 500/750)
//! - Single behavior choice: Class 7 (`Search And Attack Target`, choice 0)
//! - No Sub-A (propulsion), Sub-C (lift), Sub-H (external frame), or Sub-I (animation)
//!
//! In retail Intro2, `FUN_00401430` evaluates their movement via Sub-D steering
//! (yaw rate integration, zero obstacle avoidance) and Sub-G flight dynamics
//! (terrain clearance, pitch tracking and body-up force). Sub-G suppresses the
//! C/A/B tail despite the authored Sub-B descriptor. Class-7 C690/ADE0 retain the live
//! acquiring/pursuing graph; the outer world owner supplies callback timing,
//! body-basis refresh, environment and one final position integration.

use crate::actor_task_dispatcher::{prepare_target_acquisition_runtime_task, ActorTaskRuntime};
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit, PreparedActorTask};
use crate::chase_target::{
    chase_target_transition_after_unwind, evaluate_chase_target_callback,
    ChaseTargetCallbackPrefix, ChaseTargetCallbackRequest, ChaseTargetCallbackResult,
    ChaseTargetCommonMoverReturn, ChaseTargetProximityControl, ChaseTargetTargetRuntimeState,
};
use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
use crate::common_mover::sub_d::{Type9SubDFrameOwner, Type9SubDRuntime, FLYER_SUB_D};
use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity::{Entity, EntityManager};
use crate::entity_behavior::{
    behavior_program, initial_behavior_state_policy, select_initial_behavior, ActiveBehaviorStyle,
    BehaviorChoiceListSource, BehaviorContextRuntime, BehaviorDescriptorIdentity,
    BehaviorSelection, BehaviorWeightRule,
};
use crate::entity_collision_state::{
    CommonMoverComponentTopology, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::resource_cache::ResourceCache;
use crate::search_attack::{
    search_attack_variant_setup, SearchAttackCandidateFilter, SearchAttackRadius,
    SearchAttackTaskLifetime, SearchAttackTaskRole, SearchAttackVariant,
    SEARCH_ATTACK_BEHAVIOR_CLASS_ID,
};
use crate::search_attack_live::{
    apply_search_attack_acquisition_live, SearchAttackLiveAcquisitionOutcome,
    SearchAttackLiveHandoffRequirement, SearchAttackLiveSamePassAim,
};
use crate::search_attack_owner::{apply_search_attack_task_setup, SearchAttackTaskSetupRequest};
use crate::shared_retarget_mover::{
    shared_retarget_after_unwind, SharedRetargetCallbackPrefix, SharedRetargetPostUnwind,
    SharedRetargetTaskState,
};
use crate::sub_g_runtime::SubG06070RuntimeState;
use crate::wander_near_location::{WanderNearCommonMoverReturn, WanderNearPrivateState};
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::WrappedAxisRange;
use v2k_formats::collision::{CommonAxisDescriptor, SubBLateralDescriptor};
use v2k_formats::terrain::TerrainGrid;

pub const INTRO2_TYPE15_SPAWN_INDEX: usize = 44;
pub const INTRO2_TYPE87_SPAWN_INDEX: usize = 46;
pub const INTRO2_FLYER_SPAWN_INDICES: [usize; 2] =
    [INTRO2_TYPE15_SPAWN_INDEX, INTRO2_TYPE87_SPAWN_INDEX];

pub const INTRO2_TYPE15_ENTITY_TYPE: u32 = 15;
pub const INTRO2_TYPE87_ENTITY_TYPE: u32 = 87;

pub const INTRO2_TYPE15_MODEL_ID: u16 = 276;
pub const INTRO2_TYPE87_MODEL_ID: u16 = 270;

pub const INTRO2_TYPE15_POSITION_RAW: [i16; 3] = [-17152, 0, 32256];
pub const INTRO2_TYPE87_POSITION_RAW: [i16; 3] = [-18176, 1024, 4864];

pub const FLYER_COMMON_MOVER_TOPOLOGY: CommonMoverComponentTopology =
    CommonMoverComponentTopology {
        sub_a: false,
        sub_b: true,
        sub_c: false,
        sub_d: true,
        sub_e: true,
        sub_f: false,
        sub_g: true,
        sub_h: false,
        sub_i: false,
        sub_j: false,
        sub_k: false,
        sub_l: false,
        sub_m: false,
        sub_n: false,
        sub_o: false,
    };

pub const FLYER_SUB_B: SubBLateralDescriptor = SubBLateralDescriptor {
    projection_threshold_rate_raw: 1_000_000,
    correction_rate_raw: 1_000_000,
};

pub const FLYER_COMMON_AXIS: CommonAxisDescriptor = CommonAxisDescriptor {
    strict_axis_limit_raw: 7680,
    raw_word_at_0x04: 1,
};

pub const TYPE15_SUB_G_DESCRIPTOR: [u8; 104] = [
    20, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 244, 1, 0, 0, 188, 2, 0, 0, 88, 2, 0, 0, 0,
    64, 0, 0, 244, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 128, 0, 96, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub const TYPE87_SUB_G_DESCRIPTOR: [u8; 104] = [
    20, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 244, 1, 0, 0, 188, 2, 0, 0, 88, 2, 0, 0, 0,
    64, 0, 0, 238, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 128, 0, 96, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const FLYER_WANDER_LIFETIME_MS: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2FlyerFrameOwner {
    pub birth_provenance: FlyerBirthProvenance,
    pub entity_type: u32,
    pub sub_d_runtime: Type9SubDRuntime,
    pub sub_d_frame_owner: Type9SubDFrameOwner,
    pub wing_var1: i32,
}

/// The authored Intro2 subset and a native zero-record Wasp keep distinct
/// constructor authority while sharing the same class7 task and mover policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlyerBirthProvenance {
    AuthoredIntro2 { spawn_index: usize },
    NativeType15(NativeType15Birth),
}

/// Minted only after the complete native SubG/weighted/task constructor phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeType15Birth {
    allocation: crate::main_base_abort::MainBaseAbortActorLease,
}

impl FlyerBirthProvenance {
    pub const fn authored_spawn_index(self) -> Option<usize> {
        match self {
            Self::AuthoredIntro2 { spawn_index } => Some(spawn_index),
            Self::NativeType15(_) => None,
        }
    }

    pub const fn allocation(self) -> Option<crate::main_base_abort::MainBaseAbortActorLease> {
        match self {
            Self::AuthoredIntro2 { .. } => None,
            Self::NativeType15(birth) => Some(birth.allocation),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2FlyerAdmission {
    pub spawn_index: usize,
    pub entity_type: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2FlyerPublicationError {
    EntityInactive,
    MissingEntity { spawn_index: usize },
    MissingMetadata,
    MetadataMismatch,
    EntityIdentityMismatch,
    UnexpectedModel,
    UnexpectedEntityType,
    InitialBehaviorAlreadyResolved,
    CurrentBehaviorAlreadyResolved,
    TaskTableNotEmpty,
    SubGRuntimeUnavailable,
    BehaviorProgramUnavailable,
    BehaviorContextUnavailable,
    TaskSetupMismatch,
}

/// Authenticate Intro2 spawn 44 (type 15) or spawn 46 (type 87) against
/// cumulative Section-12 collision metadata.
pub fn authenticate_intro2_flyer(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<Intro2FlyerAdmission, Intro2FlyerPublicationError> {
    if !entity.active {
        return Err(Intro2FlyerPublicationError::EntityInactive);
    }
    let Some(spawn_index) = entity.authored_spawn_index else {
        return Err(Intro2FlyerPublicationError::EntityIdentityMismatch);
    };
    if !INTRO2_FLYER_SPAWN_INDICES.contains(&spawn_index) {
        return Err(Intro2FlyerPublicationError::EntityIdentityMismatch);
    }
    let (expected_type, expected_model, expected_pos) = match spawn_index {
        INTRO2_TYPE15_SPAWN_INDEX => (
            INTRO2_TYPE15_ENTITY_TYPE,
            INTRO2_TYPE15_MODEL_ID as usize,
            INTRO2_TYPE15_POSITION_RAW,
        ),
        INTRO2_TYPE87_SPAWN_INDEX => (
            INTRO2_TYPE87_ENTITY_TYPE,
            INTRO2_TYPE87_MODEL_ID as usize,
            INTRO2_TYPE87_POSITION_RAW,
        ),
        _ => unreachable!(),
    };
    if entity.entity_type != expected_type {
        return Err(Intro2FlyerPublicationError::EntityIdentityMismatch);
    }
    let position_raw = entity.position_raw();
    if position_raw[0] != expected_pos[0] || position_raw[2] != expected_pos[2] {
        return Err(Intro2FlyerPublicationError::EntityIdentityMismatch);
    }
    if entity.model_slots != [Some(expected_model); 4] || entity.model_index != Some(expected_model)
    {
        return Err(Intro2FlyerPublicationError::UnexpectedModel);
    }
    if let RetailRuntimeValue::Known(Some(existing)) = entity.initial_behavior {
        if existing.program.class_id != SEARCH_ATTACK_BEHAVIOR_CLASS_ID
            || existing.choice_index != 0
        {
            return Err(Intro2FlyerPublicationError::InitialBehaviorAlreadyResolved);
        }
    }
    if entity.current_behavior_context != RetailRuntimeValue::Unresolved {
        return Err(Intro2FlyerPublicationError::CurrentBehaviorAlreadyResolved);
    }
    if ActorTaskSlot::IN_RETAIL_TICK_ORDER
        .into_iter()
        .any(|slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2FlyerPublicationError::TaskTableNotEmpty);
    }

    if metadata.common_mover_topology != RetailRuntimeValue::Known(FLYER_COMMON_MOVER_TOPOLOGY)
        || metadata.sub_b_lateral_descriptor != RetailRuntimeValue::Known(Some(FLYER_SUB_B))
        || metadata.sub_d_steering_descriptor != RetailRuntimeValue::Known(Some(FLYER_SUB_D))
    {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    };
    if initializer.common_axis_descriptor != FLYER_COMMON_AXIS {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }
    let Some(choice) = initializer.behavior_choices.first() else {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    };
    if choice.behavior_class_id != u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID) {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }

    Ok(Intro2FlyerAdmission {
        spawn_index,
        entity_type: expected_type,
    })
}

/// Publish authenticated Intro2 flyer entity (spawn 44 or 46) into live
/// Class-7 acquiring state.
pub fn publish_intro2_flyer(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    _terrain: Option<&TerrainGrid>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2FlyerAdmission, Intro2FlyerPublicationError> {
    let admission = authenticate_intro2_flyer(entity, metadata)?;
    publish_flyer_constructor_phase(
        entity,
        metadata,
        FlyerBirthProvenance::AuthoredIntro2 {
            spawn_index: admission.spawn_index,
        },
        Type9SubDRuntime::default(),
        Type9SubDFrameOwner::flyer(),
        next_random,
    )?;
    Ok(admission)
}

fn publish_flyer_constructor_phase(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    birth_provenance: FlyerBirthProvenance,
    sub_d_runtime: Type9SubDRuntime,
    sub_d_frame_owner: Type9SubDFrameOwner,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerPublicationError> {
    let descriptor = authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    authenticate_flyer_choices(metadata)?;
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(
        SubG06070RuntimeState::from_1b8c0_constructor(descriptor, next_random() as u16),
    ));
    publish_flyer_behavior_phase(
        entity,
        metadata,
        birth_provenance,
        sub_d_runtime,
        sub_d_frame_owner,
        next_random,
    )
}

/// D4A0/25680 and the selected class7 initializer run after component birth.
fn publish_flyer_behavior_phase(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    birth_provenance: FlyerBirthProvenance,
    sub_d_runtime: Type9SubDRuntime,
    sub_d_frame_owner: Type9SubDFrameOwner,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerPublicationError> {
    // 425680 consumes a selector word even for a sole Always choice.
    let selection = select_flyer_behavior(metadata, next_random)?;
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2FlyerPublicationError::BehaviorContextUnavailable)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    publish_flyer_acquiring(entity, metadata, context, next_random)?;
    entity.intro2_flyer_frame_owner = Some(Intro2FlyerFrameOwner {
        birth_provenance,
        entity_type: entity.entity_type,
        sub_d_runtime,
        sub_d_frame_owner,
        // Constructor-resolved variable storage is zero; AA60 is the first
        // per-frame writer of the descriptor's selector-one animation word.
        wing_var1: 0,
    });
    // Native birth policy for retail's unwritten heap word +B2:
    // initialize the transient callback-mass offset deterministically once.
    // Later animation/contact writers and scheduler resets retain ownership.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    //10090 sets every Section12 type+7C to4C8A30;438080's special patches
    // admit67/46/51, not15/87. Original4C8A30+30 is null. This generic
    //14E90 hook is distinct from Search's primary/infected style hooks.
    entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
    crate::intro2_flyer_aim::ensure_intro2_flyer_aim_runtime(entity, Some(metadata))
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    let [heading, pitch, roll] = entity.rotation_heading_pitch_roll_raw();
    entity.physical_body_basis_q31 =
        RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(heading, pitch, roll));
    Ok(())
}

fn authenticate_flyer_choices(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2FlyerPublicationError> {
    let Some(initializer) = &metadata.initializer else {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    };
    if initializer.common_axis_descriptor != FLYER_COMMON_AXIS
        || initializer.behavior_choices.len() != 1
        || initializer.behavior_choices[0].weight_rule_id != 1
        || initializer.behavior_choices[0].weight_multiplier == 0
        || initializer.behavior_choices[0].behavior_class_id
            != u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)
    {
        return Err(Intro2FlyerPublicationError::MetadataMismatch);
    }
    Ok(())
}

fn select_flyer_behavior(
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<BehaviorSelection, Intro2FlyerPublicationError> {
    authenticate_flyer_choices(metadata)?;
    select_initial_behavior(
        &metadata.initializer.as_ref().unwrap().behavior_choices,
        |rule| {
            debug_assert_eq!(rule, BehaviorWeightRule::Always);
            1
        },
        next_random,
    )
    .map_err(|_| Intro2FlyerPublicationError::BehaviorProgramUnavailable)?
    .ok_or(Intro2FlyerPublicationError::BehaviorProgramUnavailable)
}

fn publish_flyer_acquiring(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    context: BehaviorContextRuntime,
    next_random: &mut impl FnMut() -> u32,
) -> Result<(), Intro2FlyerPublicationError> {
    let descriptor = authenticate_flyer_mover_metadata(entity.entity_type, metadata)
        .map_err(|_| Intro2FlyerPublicationError::MetadataMismatch)?;
    authenticate_flyer_choices(metadata)?;
    let RetailRuntimeValue::Known(Some(mut sub_g)) = entity.sub_g_06070_runtime else {
        return Err(Intro2FlyerPublicationError::SubGRuntimeUnavailable);
    };
    let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID))
        .ok_or(Intro2FlyerPublicationError::BehaviorProgramUnavailable)?;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(context));
    let policy = initial_behavior_state_policy(program);
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(policy.set_bits | policy.clear_bits, policy.set_bits);
    entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(FLYER_COMMON_AXIS);
    let owner_pos_raw = entity.position_raw();
    let radius = SearchAttackRadius::from_raw(FLYER_COMMON_AXIS.strict_axis_limit_raw);
    let filter = SearchAttackCandidateFilter::from_raw(FLYER_COMMON_AXIS.raw_word_at_0x04);
    let setup = search_attack_variant_setup(SearchAttackVariant::Acquiring);
    let source = u32::from_le_bytes(descriptor[0..4].try_into().unwrap());
    let clearance = i32::from(i16::from_le_bytes(descriptor[12..14].try_into().unwrap()));
    let setup_result = apply_search_attack_task_setup(
        &mut entity.actor_tasks,
        SearchAttackTaskSetupRequest::Acquiring,
        |preparation| {
            let prepared = match preparation.task.role {
                SearchAttackTaskRole::AcquireTarget => {
                    prepare_target_acquisition_runtime_task(preparation, radius, filter, 0)
                        .map_err(|_| Intro2FlyerPublicationError::TaskSetupMismatch)?
                }
                SearchAttackTaskRole::Wander => {
                    if preparation.request != SearchAttackTaskSetupRequest::Acquiring
                        || preparation.phase_index != 1
                        || preparation.task != setup.ordered_phases[1].install
                        || preparation.task.lifetime
                            != SearchAttackTaskLifetime::FixedMilliseconds(FLYER_WANDER_LIFETIME_MS)
                    {
                        return Err(Intro2FlyerPublicationError::TaskSetupMismatch);
                    }
                    PreparedActorTask::new(ActorTaskRuntime::SharedRetarget(
                        SharedRetargetTaskState::new(owner_pos_raw, FLYER_WANDER_LIFETIME_MS),
                    ))
                }
                _ => return Err(Intro2FlyerPublicationError::TaskSetupMismatch),
            };
            // Both task constructors invoke 06070. Each B940 call owns a
            // distinct RNG word; neither reuses the allocation's 1B8C0 draw.
            sub_g.apply_shared_06070_sub_g_branch(
                clearance.wrapping_add(i32::from((next_random() as u16) >> 8)),
                source,
            );
            Ok(prepared)
        },
    );
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(sub_g));
    setup_result.map_err(|_| Intro2FlyerPublicationError::TaskSetupMismatch)
}
/// Publish both captured Intro2 flyer spawns (44 and 46) together.
pub fn publish_captured_intro2_flyers(
    manager: &mut EntityManager,
    type_metadata: &[EntityTypeRuntimeMetadata],
    terrain: Option<&TerrainGrid>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<[Intro2FlyerAdmission; 2], Intro2FlyerPublicationError> {
    let metadata_15 = type_metadata
        .get(INTRO2_TYPE15_ENTITY_TYPE as usize)
        .ok_or(Intro2FlyerPublicationError::MissingMetadata)?;
    let metadata_87 = type_metadata
        .get(INTRO2_TYPE87_ENTITY_TYPE as usize)
        .ok_or(Intro2FlyerPublicationError::MissingMetadata)?;

    let entity_15_id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE15_SPAWN_INDEX))
        .map(|e| e.id)
        .ok_or(Intro2FlyerPublicationError::MissingEntity {
            spawn_index: INTRO2_TYPE15_SPAWN_INDEX,
        })?;
    let entity_15 = manager.intro2_flyer_entity_mut(entity_15_id).ok_or(
        Intro2FlyerPublicationError::MissingEntity {
            spawn_index: INTRO2_TYPE15_SPAWN_INDEX,
        },
    )?;
    let admission_15 = publish_intro2_flyer(entity_15, metadata_15, terrain, next_random)?;

    let entity_87_id = manager
        .iter_all()
        .find(|e| e.authored_spawn_index == Some(INTRO2_TYPE87_SPAWN_INDEX))
        .map(|e| e.id)
        .ok_or(Intro2FlyerPublicationError::MissingEntity {
            spawn_index: INTRO2_TYPE87_SPAWN_INDEX,
        })?;
    let entity_87 = manager.intro2_flyer_entity_mut(entity_87_id).ok_or(
        Intro2FlyerPublicationError::MissingEntity {
            spawn_index: INTRO2_TYPE87_SPAWN_INDEX,
        },
    )?;
    let admission_87 = publish_intro2_flyer(entity_87, metadata_87, terrain, next_random)?;

    Ok([admission_15, admission_87])
}

fn commit_intro2_flyer_mover_pose(entity: &mut Entity, outcome: Intro2FlyerCommonMoverOutcome) {
    entity.intro2_flyer_frame_owner = Some(outcome.frame_owner);
    entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(outcome.sub_g_runtime));
    entity.set_rotation_heading_pitch_roll_raw([
        outcome.heading_raw as i16,
        outcome.pitch_raw,
        outcome.roll_raw,
    ]);
    entity.set_motion_raw(outcome.position_raw, outcome.velocity_raw);
}

#[derive(Clone, Copy)]
pub struct Intro2FlyerFrame<'a> {
    pub resources: &'a ResourceCache,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2FlyerSchedulerOwner {
    entity_id: u32,
    birth_provenance: FlyerBirthProvenance,
    parked: Option<Intro2FlyerRuntimeBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerSchedulerAdoptionError {
    EntityUnavailable,
    GraphUnavailable { entity_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerSchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerRuntimeBlock {
    FrameUnavailable,
    MetadataMismatch,
    StateUnavailable,
    AlternateBehaviorUnsupported,
    TaskUnavailable,
    TargetStateUnavailable,
    CommonMover(Intro2FlyerCommonMoverBlock),
    Publication,
    Acquisition(crate::search_attack_live::SearchAttackLiveAcquisitionBlock),
    Aim(crate::intro2_flyer_aim::Intro2FlyerAimError),
    World(Intro2FlyerWorldBlock),
    /// A late 11AD0 response committed state/effects before its suffix blocked.
    ContactPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2FlyerPrimaryVisitResult {
    Continue,
    TaggedZeroMover,
    CommonMoverBlocked(Intro2FlyerCommonMoverBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2FlyerPrimaryVisit {
    pub prefix: SharedRetargetCallbackPrefix,
    pub result: Intro2FlyerPrimaryVisitResult,
    pub post_unwind: SharedRetargetPostUnwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2FlyerChaseVisit {
    pub prefix: ChaseTargetCallbackPrefix,
    pub result: ChaseTargetCallbackResult,
    pub transition_due: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2FlyerSchedulerProductionOutcome {
    SchedulerWaiting {
        entity_id: u32,
    },
    CallbackDisabled {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    B6c0Visit {
        entity_id: u32,
        primary: Intro2FlyerPrimaryVisit,
        reselected: bool,
        acquisition: SearchAttackLiveAcquisitionOutcome,
        aim: Option<crate::intro2_flyer_aim::Intro2FlyerAimTickOutcome>,
    },
    PursuingVisit {
        entity_id: u32,
        chase: Intro2FlyerChaseVisit,
        reselected: bool,
        acquisition: SearchAttackLiveAcquisitionOutcome,
        aim: Option<crate::intro2_flyer_aim::Intro2FlyerAimTickOutcome>,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2FlyerRuntimeBlock,
    },
    Dropped {
        entity_id: u32,
        reason: Intro2FlyerSchedulerProductionDrop,
    },
}

impl Intro2FlyerSchedulerProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::SchedulerWaiting { entity_id }
            | Self::CallbackDisabled { entity_id, .. }
            | Self::B6c0Visit { entity_id, .. }
            | Self::PursuingVisit { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

pub struct Intro2FlyerSchedulerOwnerTick {
    pub outcome: Intro2FlyerSchedulerProductionOutcome,
    pub retained_owner: Option<Intro2FlyerSchedulerOwner>,
}

impl Intro2FlyerSchedulerOwner {
    pub(crate) const fn has_pending_prefix(self) -> bool {
        self.parked.is_some()
    }

    pub(crate) fn park_contact_prefix(&mut self) {
        self.parked = Some(Intro2FlyerRuntimeBlock::ContactPrefix);
    }

    pub(crate) fn matches_completed_entity(self, entity: &Entity) -> bool {
        !self.has_pending_prefix()
            && Self::adopt_published(entity) == Ok(self)
            && ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().all(|slot| {
                entity.actor_tasks.task_in_slot(slot).is_none_or(|task| {
                    entity
                        .actor_tasks
                        .wrapper_flags(task)
                        .is_some_and(|flags| flags.alive && !flags.in_callback)
                })
            })
    }

    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }
    pub const fn birth_provenance(self) -> FlyerBirthProvenance {
        self.birth_provenance
    }

    pub(crate) fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt_published(entity: &Entity) -> Result<Self, Intro2FlyerSchedulerAdoptionError> {
        if !intro2_flyer_acquiring_graph_authenticates(entity)
            && !intro2_flyer_pursuing_graph_authenticates(entity)
        {
            return Err(Intro2FlyerSchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            });
        }
        Ok(Self {
            entity_id: entity.id,
            birth_provenance: entity.intro2_flyer_frame_owner.unwrap().birth_provenance,
            parked: None,
        })
    }
}

pub(crate) fn flyer_identity_authenticates(entity: &Entity) -> bool {
    let Some(owner) = entity.intro2_flyer_frame_owner else {
        return false;
    };
    let expected = match owner.birth_provenance {
        FlyerBirthProvenance::AuthoredIntro2 { spawn_index }
            if entity.authored_spawn_index == Some(spawn_index) =>
        {
            match spawn_index {
                INTRO2_TYPE15_SPAWN_INDEX => (INTRO2_TYPE15_ENTITY_TYPE, INTRO2_TYPE15_MODEL_ID),
                INTRO2_TYPE87_SPAWN_INDEX => (INTRO2_TYPE87_ENTITY_TYPE, INTRO2_TYPE87_MODEL_ID),
                _ => return false,
            }
        }
        FlyerBirthProvenance::NativeType15(birth)
            if entity.authored_spawn_index.is_none() && entity.id == birth.allocation.entity_id =>
        {
            (INTRO2_TYPE15_ENTITY_TYPE, INTRO2_TYPE15_MODEL_ID)
        }
        _ => return false,
    };
    entity.active
        && entity.entity_type == expected.0
        && entity.model_slots == [Some(usize::from(expected.1)); 4]
        && entity.model_index == Some(usize::from(expected.1))
        && owner.entity_type == entity.entity_type
        && matches!(
            entity.sub_g_06070_runtime,
            RetailRuntimeValue::Known(Some(_))
        )
}

/// An allocation receipt cannot authorize an actor id reused in another world.
pub(crate) fn flyer_manager_identity_authenticates(manager: &EntityManager, id: u32) -> bool {
    let Some(entity) = manager.iter_all().find(|entity| entity.id == id) else {
        return false;
    };
    flyer_identity_authenticates(entity)
        && entity.intro2_flyer_frame_owner.is_some_and(|owner| {
            owner
                .birth_provenance
                .allocation()
                .is_none_or(|allocation| {
                    manager
                        .main_base_abort_actor_observation(id)
                        .is_some_and(|observation| observation.lease == allocation)
                })
        })
}

fn flyer_context_authenticates(entity: &Entity, variant: SearchAttackVariant) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    let Some(program) = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID)) else {
        return false;
    };
    let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        return false;
    };
    context.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && context.choice_list_source()
            == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.style_table_index_raw_at_0x10()
            == match variant {
                SearchAttackVariant::Acquiring => 0,
                SearchAttackVariant::Pursuing => 1,
                SearchAttackVariant::ExternalEvent => 2,
            }
        && style.class_id == SEARCH_ATTACK_BEHAVIOR_CLASS_ID
        && style.frame_address == variant.style_address()
}

fn flyer_wrappers_live(entity: &Entity) -> bool {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.into_iter().all(|slot| {
        entity.actor_tasks.task_in_slot(slot).is_none_or(|task_id| {
            entity
                .actor_tasks
                .wrapper_flags(task_id)
                .is_some_and(|flags| flags.alive)
        })
    })
}

fn intro2_flyer_acquiring_graph_authenticates(entity: &Entity) -> bool {
    flyer_identity_authenticates(entity)
        && flyer_context_authenticates(entity, SearchAttackVariant::Acquiring)
        && flyer_wrappers_live(entity)
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        )
        && matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::TargetAcquisition(_))
        )
        && entity.actor_task_state(ActorTaskSlot::Tertiary).is_none()
}

pub(crate) fn intro2_flyer_pursuing_graph_authenticates(entity: &Entity) -> bool {
    if !flyer_identity_authenticates(entity)
        || !flyer_context_authenticates(entity, SearchAttackVariant::Pursuing)
        || !flyer_wrappers_live(entity)
        || entity.actor_task_state(ActorTaskSlot::Secondary).is_some()
    {
        return false;
    }
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(target_id)) = context.target_handle_at_0x08() else {
        return false;
    };
    matches!(entity.actor_task_state(ActorTaskSlot::Primary), Some(ActorTaskRuntime::ChaseTarget(task)) if task.target_id() == target_id)
        && matches!(entity.actor_task_state(ActorTaskSlot::Tertiary), Some(ActorTaskRuntime::AimAndFire(task)) if task.private_state().target_entity_id() == target_id)
}

fn park_flyer(
    mut owner: Intro2FlyerSchedulerOwner,
    reason: Intro2FlyerRuntimeBlock,
) -> Intro2FlyerSchedulerOwnerTick {
    // A callback may already have consumed time/RNG or appended a shot. Keep
    // that failed visit explicit; repairing evidence must never replay Primary.
    owner.parked = Some(reason);
    Intro2FlyerSchedulerOwnerTick {
        outcome: Intro2FlyerSchedulerProductionOutcome::Blocked {
            entity_id: owner.entity_id,
            reason,
        },
        retained_owner: Some(owner),
    }
}

pub(crate) fn reselect_flyer(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2FlyerRuntimeBlock> {
    let RetailRuntimeValue::Known(flags) =
        entity.collision.state_flags_at_0x08.masked(0x1000 | 0x4000)
    else {
        return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
    };
    if flags & 0x1000 != 0 {
        return Ok(false);
    }
    if flags & 0x4000 != 0 {
        return Err(Intro2FlyerRuntimeBlock::AlternateBehaviorUnsupported);
    }
    if !flyer_context_authenticates(entity, SearchAttackVariant::Acquiring)
        && !flyer_context_authenticates(entity, SearchAttackVariant::Pursuing)
    {
        return Err(Intro2FlyerRuntimeBlock::MetadataMismatch);
    }
    authenticate_flyer_choices(metadata).map_err(|_| Intro2FlyerRuntimeBlock::MetadataMismatch)?;
    let RetailRuntimeValue::Known(Some(previous)) = entity.current_behavior_context else {
        return Err(Intro2FlyerRuntimeBlock::StateUnavailable);
    };
    let selection = select_flyer_behavior(metadata, next_random)
        .map_err(|_| Intro2FlyerRuntimeBlock::Publication)?;
    let context = previous
        .reselect_named_type_default(
            selection.program,
            selection.program.initial_style_table_index_raw,
            selection.program.initial_style,
        )
        .ok_or(Intro2FlyerRuntimeBlock::Publication)?;
    publish_flyer_acquiring(entity, metadata, context, next_random)
        .map_err(|_| Intro2FlyerRuntimeBlock::Publication)?;
    Ok(true)
}

#[derive(Clone, Copy)]
struct FlyerTaskFrame<'a> {
    metadata: &'a EntityTypeRuntimeMetadata,
    terrain: &'a TerrainGrid,
    active_model_extent_raw: u16,
    attached_cargo_mass: u32,
    elapsed_micros: u32,
    global_elapsed_micros: u32,
    retail_tick: u32,
    dispatch_mode: CommonMoverDispatchMode,
}

impl<'a> FlyerTaskFrame<'a> {
    fn request(
        self,
        entity: &Entity,
        target_private: WanderNearPrivateState,
        tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ) -> Result<Intro2FlyerCommonMoverRequest<'a>, Intro2FlyerRuntimeBlock> {
        let frame_owner = entity
            .intro2_flyer_frame_owner
            .ok_or(Intro2FlyerRuntimeBlock::MetadataMismatch)?;
        let RetailRuntimeValue::Known(Some(sub_g_runtime)) = entity.sub_g_06070_runtime else {
            return Err(Intro2FlyerRuntimeBlock::CommonMover(
                Intro2FlyerCommonMoverBlock::SubGRuntimeUnavailable,
            ));
        };
        let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
        Ok(Intro2FlyerCommonMoverRequest {
            dispatch_mode: self.dispatch_mode,
            entity_id: entity.id,
            metadata: self.metadata,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            heading_raw: heading_raw as u16,
            pitch_raw,
            roll_raw,
            body_basis: entity.physical_body_basis_q31(),
            frame_owner,
            sub_g_runtime,
            target_private,
            tracked_target,
            terrain: self.terrain,
            active_model_extent_raw: self.active_model_extent_raw,
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: self.attached_cargo_mass,
            capability_flags: entity.capability_flags,
            retail_tick: self.retail_tick,
            elapsed_micros: self.elapsed_micros,
            global_elapsed_micros: self.global_elapsed_micros,
        })
    }
}

fn tick_flyer_shared_retarget(
    entity: &mut Entity,
    frame: FlyerTaskFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2FlyerPrimaryVisit, Intro2FlyerRuntimeBlock> {
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let lifetime = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::SharedRetarget(task) = runtime else {
                unreachable!()
            };
            task.before_callback(frame.elapsed_micros)
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let position = entity.position_raw();
    let Some(ActorTaskRuntime::SharedRetarget(task)) =
        entity.actor_tasks.exact_callback_state_mut(visit)
    else {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    };
    let mut stage = task.stage_callback(position, || next_random() as u16);
    let prefix = SharedRetargetCallbackPrefix::from_parts(lifetime, stage.retarget());
    let request = frame.request(
        entity,
        stage.private_state(),
        RetailRuntimeValue::Known(None),
    )?;
    let evaluated = evaluate_intro2_flyer_common_mover(request, next_random);
    let survived = entity.actor_tasks.finish_exact_visit(visit);
    if !survived {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    }
    let outcome = evaluated.map_err(Intro2FlyerRuntimeBlock::CommonMover)?;
    *stage.private_state_mut() = outcome.target_private;
    if let Some(ActorTaskRuntime::SharedRetarget(task)) = entity.actor_tasks.task_state_mut(task_id)
    {
        task.commit_callback_stage(stage);
    }
    commit_intro2_flyer_mover_pose(entity, outcome);
    let (result, mover_return) = match outcome.result {
        ChaseTargetCommonMoverReturn::NonZero => (
            Intro2FlyerPrimaryVisitResult::Continue,
            WanderNearCommonMoverReturn::NonZero,
        ),
        ChaseTargetCommonMoverReturn::Zero => (
            Intro2FlyerPrimaryVisitResult::TaggedZeroMover,
            WanderNearCommonMoverReturn::Zero,
        ),
    };
    Ok(Intro2FlyerPrimaryVisit {
        prefix,
        result,
        post_unwind: shared_retarget_after_unwind(visit, prefix, mover_return),
    })
}

fn flyer_target_snapshot(
    manager: &EntityManager,
    target_id: u32,
) -> Result<
    (
        ChaseTargetTargetRuntimeState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ),
    Intro2FlyerRuntimeBlock,
> {
    let Some(target) = manager.iter_all().find(|target| target.id == target_id) else {
        return Ok((
            ChaseTargetTargetRuntimeState::Missing,
            RetailRuntimeValue::Known(None),
        ));
    };
    if !target.active {
        return Ok((
            ChaseTargetTargetRuntimeState::Inactive,
            RetailRuntimeValue::Known(None),
        ));
    }
    let flags = target.collision.state_flags_at_0x08;
    let snapshot = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
        state_flags: flags,
        position_raw: target.position_raw(),
        velocity_raw: target.velocity_raw(),
    }));
    match flags.masked(0x4000) {
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return Ok((ChaseTargetTargetRuntimeState::Dying, snapshot))
        }
        RetailRuntimeValue::Unresolved => {
            return Err(Intro2FlyerRuntimeBlock::TargetStateUnavailable)
        }
        _ => {}
    }
    if flags.known_value_bits() != 0 {
        return Ok((
            ChaseTargetTargetRuntimeState::Live {
                position_raw: target.position_raw(),
            },
            snapshot,
        ));
    }
    if flags.known_mask() == u32::MAX {
        return Ok((ChaseTargetTargetRuntimeState::Inactive, snapshot));
    }
    Err(Intro2FlyerRuntimeBlock::TargetStateUnavailable)
}

fn tick_flyer_chase(
    manager: &mut EntityManager,
    entity_id: u32,
    frame: FlyerTaskFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2FlyerChaseVisit, Intro2FlyerRuntimeBlock> {
    let target_id = match manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| entity.actor_task_state(ActorTaskSlot::Primary))
    {
        Some(ActorTaskRuntime::ChaseTarget(task)) => task.target_id(),
        _ => return Err(Intro2FlyerRuntimeBlock::TaskUnavailable),
    };
    let (target_state, tracked_target) = flyer_target_snapshot(manager, target_id)?;
    let entity = manager
        .intro2_flyer_entity_mut(entity_id)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let task_id = entity
        .actor_tasks
        .task_in_slot(ActorTaskSlot::Primary)
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let prefix = entity
        .actor_tasks
        .begin_exact_visit_with(visit, |runtime| {
            let ActorTaskRuntime::ChaseTarget(task) = runtime else {
                unreachable!()
            };
            task.before_callback(frame.elapsed_micros)
        })
        .ok_or(Intro2FlyerRuntimeBlock::TaskUnavailable)?;
    let Some(ActorTaskRuntime::ChaseTarget(task)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    };
    let mut stage = task.stage_callback();
    let mut applied = None;
    let result = evaluate_chase_target_callback(
        &mut stage,
        ChaseTargetCallbackRequest {
            visit,
            entity_id,
            owner_position_raw: entity.position_raw(),
            route_range: WrappedAxisRange::from_raw(FLYER_COMMON_AXIS.strict_axis_limit_raw),
            proximity_control: ChaseTargetProximityControl::Disabled,
            movement_state: &mut (),
            controller_context: &mut (),
            elapsed_micros: frame.elapsed_micros,
            scheduler_mode: match frame.dispatch_mode {
                CommonMoverDispatchMode::Normal => 0,
                CommonMoverDispatchMode::Restricted => 1,
            },
        },
        |_| Ok::<_, Intro2FlyerRuntimeBlock>(target_state),
        |request| {
            let outcome = evaluate_intro2_flyer_common_mover(
                frame.request(entity, *request.target_state, tracked_target)?,
                &mut *next_random,
            )
            .map_err(Intro2FlyerRuntimeBlock::CommonMover)?;
            *request.target_state = outcome.target_private;
            applied = Some(outcome);
            Ok::<_, Intro2FlyerRuntimeBlock>(outcome.result)
        },
        |_, _| unreachable!("B/D/E/G has no proximity-controller component"),
    );
    if !entity.actor_tasks.finish_exact_visit(visit) {
        return Err(Intro2FlyerRuntimeBlock::TaskUnavailable);
    }
    let result = result.map_err(|error| match error {
        crate::chase_target::ChaseTargetCallbackError::CommonMover { error, .. } => error,
        _ => Intro2FlyerRuntimeBlock::TaskUnavailable,
    })?;
    if let Some(ActorTaskRuntime::ChaseTarget(task)) = entity.actor_tasks.task_state_mut(task_id) {
        stage.commit(task);
    }
    if let Some(outcome) = applied {
        commit_intro2_flyer_mover_pose(entity, outcome);
    }
    Ok(Intro2FlyerChaseVisit {
        prefix,
        result,
        transition_due: chase_target_transition_after_unwind(visit, prefix, result).is_some(),
    })
}

fn tick_flyer_tasks(
    manager: &mut EntityManager,
    owner: Intro2FlyerSchedulerOwner,
    frame: FlyerTaskFrame<'_>,
    world_fx: &mut WorldFx,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2FlyerSchedulerOwnerTick {
    if let Some(reason) = owner.parked {
        return park_flyer(owner, reason);
    }
    let entity_id = owner.entity_id;
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2FlyerSchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    let acquiring = intro2_flyer_acquiring_graph_authenticates(entity);
    if !acquiring && !intro2_flyer_pursuing_graph_authenticates(entity) {
        return Intro2FlyerSchedulerOwnerTick {
            outcome: Intro2FlyerSchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2FlyerSchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let mut primary = None;
    let mut chase = None;
    let transition_due = if acquiring {
        match tick_flyer_shared_retarget(
            manager.intro2_flyer_entity_mut(entity_id).unwrap(),
            frame,
            &mut || next_random(world_fx),
        ) {
            Ok(visit) => {
                primary = Some(visit);
                matches!(visit.post_unwind, SharedRetargetPostUnwind::Transition(_))
            }
            Err(reason) => return park_flyer(owner, reason),
        }
    } else {
        match tick_flyer_chase(manager, entity_id, frame, &mut || next_random(world_fx)) {
            Ok(visit) => {
                chase = Some(visit);
                visit.transition_due
            }
            Err(reason) => return park_flyer(owner, reason),
        }
    };
    let mut reselected = false;
    if transition_due {
        match reselect_flyer(
            manager.intro2_flyer_entity_mut(entity_id).unwrap(),
            frame.metadata,
            &mut || next_random(world_fx),
        ) {
            Ok(applied) => reselected |= applied,
            Err(reason) => return park_flyer(owner, reason),
        }
    }
    // The generic owner freshly reads Secondary after Primary/C690. ADE0 may
    // replace the graph here, but the new Primary never runs twice this pass.
    let acquisition = apply_search_attack_acquisition_live(
        manager,
        entity_id,
        frame.elapsed_micros,
        world_fx,
        SearchAttackLiveHandoffRequirement::RequiredClass7Variant0,
        SearchAttackLiveSamePassAim::Skip,
    );
    if let SearchAttackLiveAcquisitionOutcome::Blocked { reason, .. } = acquisition {
        return park_flyer(owner, Intro2FlyerRuntimeBlock::Acquisition(reason));
    }
    let mut aim = None;
    if manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(intro2_flyer_pursuing_graph_authenticates)
    {
        let outcome = match crate::intro2_flyer_aim::tick_intro2_flyer_aim(
            frame.dispatch_mode,
            manager,
            world_fx,
            entity_id,
            frame.elapsed_micros,
            Some(frame.metadata),
        ) {
            Ok(outcome) => outcome,
            Err(error) => return park_flyer(owner, Intro2FlyerRuntimeBlock::Aim(error)),
        };
        if matches!(
            outcome.resolution.outcome,
            crate::aim_and_fire::AimAndFireFrameOutcome::RequestOwnerTransition { .. }
        ) {
            match reselect_flyer(
                manager.intro2_flyer_entity_mut(entity_id).unwrap(),
                frame.metadata,
                &mut || next_random(world_fx),
            ) {
                Ok(applied) => reselected |= applied,
                Err(reason) => return park_flyer(owner, reason),
            }
        }
        aim = Some(outcome);
    }
    let outcome = if let Some(primary) = primary {
        Intro2FlyerSchedulerProductionOutcome::B6c0Visit {
            entity_id,
            primary,
            reselected,
            acquisition,
            aim,
        }
    } else {
        Intro2FlyerSchedulerProductionOutcome::PursuingVisit {
            entity_id,
            chase: chase.unwrap(),
            reselected,
            acquisition,
            aim,
        }
    };
    Intro2FlyerSchedulerOwnerTick {
        outcome,
        retained_owner: Some(owner),
    }
}

/// Direct normal-world entry. The world adapter owns the scheduler prefix and
/// one post-task environment/master-motion suffix.
pub fn tick_intro2_flyer_scheduler_owner(
    manager: &mut EntityManager,
    owner: Intro2FlyerSchedulerOwner,
    frame: Intro2FlyerFrame<'_>,
    world_fx: &mut WorldFx,
) -> Intro2FlyerSchedulerOwnerTick {
    tick_intro2_flyer_scheduler_owner_with_random(
        manager,
        owner,
        frame,
        world_fx,
        &mut |world_fx| u32::from(world_fx.next_shared_retail_random_u16()),
    )
}

pub(crate) fn tick_intro2_flyer_scheduler_owner_with_random(
    manager: &mut EntityManager,
    owner: Intro2FlyerSchedulerOwner,
    frame: Intro2FlyerFrame<'_>,
    world_fx: &mut WorldFx,
    next_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2FlyerSchedulerOwnerTick {
    world::tick_world(manager, owner, frame, world_fx, next_random)
}

mod birth;
mod mover;
mod world;
pub(crate) use birth::{
    authenticate_native_type15_body, authenticate_native_type15_metadata, publish_native_type15,
    NativeType15ConstructionRequest,
};
pub(crate) use mover::authenticate_flyer_mover_metadata;
pub use mover::{
    evaluate_intro2_flyer_common_mover, Intro2FlyerCommonMoverBlock, Intro2FlyerCommonMoverOutcome,
    Intro2FlyerCommonMoverRequest,
};
pub use world::Intro2FlyerWorldBlock;
