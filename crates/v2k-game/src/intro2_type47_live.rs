//! Shared native and explicit Intro2-replay Type47 Guard/Wander task visits.
//!
//! TTD `V200001.run` constructs three Intro2 type-47 identities (spawns
//! 6/7/8, seeds `0x06/0x07/0x08`) and first-queries them from Guard-anchor
//! wander `FUN_00402EB0`. This owner authenticates those identities, publishes
//! the authored class-32 Guard or class-6 Wander graph after the Sub-A
//! constructor and weighted selector, retains the
//! pending first-query Sub-D owner, and ticks slot-0 `FUN_00402EB0`
//! through the detached Intro2 Type-47 `FUN_00401430` bind and commits the
//! task-local pose. The sibling [`world`] owner supplies the outer
//! `FUN_00412DA0` cadence, distinct E870/DCA0 gates, environment and
//! master-motion suffixes. Model presentation owns `FUN_0041D360` Sub-H
//! writeback after command and face selection. The
//! task callback itself
//! freshly reads slot 1, runs the one-draw first-eligible Guard acquisition,
//! reuses the shared class-32 C7D0/ADE0 handoff, and visits a newly published
//! slot-2 Aim task in the same pass. Accepted passive targets are Type-9
//! allocations. Later visits run Primary Chase before Tertiary Aim through
//! the shared emitter core, apply the normal class-32 C690 tag/timeout
//! transitions, and retain the Intro2-owned Sub-D pair across Guard/Wander
//! replacement. Native ordinary and Intro2 allocations enter through their
//! manager-bound construction receipts; captured first-query publication
//! remains a separate replay adapter.
//! The same retained allocation also admits the generic class-12 transition;
//! a published death receipt replaces this Guard owner at the outer pass.

use v2k_formats::terrain::TerrainGrid;

pub mod world;

/// A800 uses the scheduler-adjusted step while Sub-D yaw separately consumes
/// the current global frame delta (DAT_004D04E4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type47CallbackFrame {
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
}

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit};
use crate::aim_and_fire::{
    AimAndFireFrameOutcome, AimAndFireLifetimeStatus, AimAndFireTransitionReason,
};
use crate::aim_and_fire_transaction::AimAndFireEmitterTransactionId;
use crate::chase_target::{
    evaluate_chase_target_callback, ChaseTargetCallbackPrefix, ChaseTargetCallbackRequest,
    ChaseTargetCallbackResult, ChaseTargetCommonMoverReturn, ChaseTargetLifetimeStatus,
    ChaseTargetProximityControl, ChaseTargetTaggedSingleton,
};
use crate::common_mover::sub_d::{intro2_type47_first_query_owner_for_seed, Type9SubDRuntime};
use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity::{type47_d4a0_immutable_anchor_raw, Entity};
use crate::entity_behavior::{
    behavior_program, select_initial_behavior, BehaviorContextRuntime, BehaviorSelection,
    BehaviorWeightRule,
};
use crate::entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue};
use crate::generic_projectile_emitter::GenericEmitterTransactionId;
use crate::guard_location_owner::acquisition::{
    evaluate_guard_location_acquisition_callback, GuardLocationAcquisitionCallbackError,
    GuardLocationAcquisitionCallbackPrefix, GuardLocationAcquisitionCallbackResult,
    GuardLocationCandidateFilter, GuardLocationEntityRef, GuardLocationSearchContext,
};
use crate::intro2_type47_common_mover::{
    evaluate_intro2_type47_common_mover, run_native_type47_common_mover,
    Intro2Type47CommonMoverBlock, Intro2Type47CommonMoverOutcome, Intro2Type47CommonMoverRequest,
    NativeType47MoverFrame, INTRO2_TYPE47_TTD_POSITIONS_RAW,
};
use crate::ordinary_type47_death_live::{
    TYPE47_COMMON_DYING_BEHAVIOR_CHOICES, TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR,
    TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR,
    TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR, TYPE47_COMMON_DYING_SUB_H_RECORDS,
};
use crate::ordinary_type47_live::{
    bind_intro2_type47_aim_and_fire, tick_intro2_type47_aim_and_fire,
    OrdinaryType47AimAndFireFrameRequest, OrdinaryType47AimAndFireRuntime,
    OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError,
    FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY, FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID,
    FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D, FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT,
};
use crate::sub_h_external_frame::SubHRuntimeState;
use crate::type47_c690::{
    Type47C690Reselection, Type47C690ReselectionBlock, Type47C690Slot, Type47ImpactC690Outcome,
};
use crate::type47_initial_behavior_live::{
    publish_type47_guard_location, publish_type47_reselected_initial_behavior,
    publish_type47_wander_near, FreshType47InitialBehaviorError, FreshType47InitializerPublication,
    TYPE47_GUARD_BEHAVIOR_CLASS_ID, TYPE47_WANDER_BEHAVIOR_CLASS_ID,
};
use crate::type47_scheduler_production::{
    apply_type47_guard_pursuing_handoff, plan_type47_scheduler_c690_live,
    type47_chase_target_runtime_state, type47_guard_entity_ref,
    type47_guard_pursuing_graph_contract_authenticates, Type47SchedulerC690Transition,
};
use crate::wander_near_location::{
    WanderNearLifetimeStatus, WanderNearPrivateState, WanderNearRetarget,
};
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::WrappedAxisRange;

pub const INTRO2_TYPE47_ENTITY_TYPE: u32 = 47;
pub const INTRO2_TYPE47_SPAWN_INDICES: [usize; 3] = [6, 7, 8];
pub const INTRO2_TYPE47_SUB_D_SEEDS: [u8; 3] = [0x06, 0x07, 0x08];
pub const INTRO2_TYPE47_MODEL_ID: usize = FRESH_LEVEL1_ORDINARY_TYPE47_MODEL_ID;

pub const fn intro2_type47_seed_for_spawn(spawn_index: usize) -> Option<u8> {
    let mut index = 0;
    while index < INTRO2_TYPE47_SPAWN_INDICES.len() {
        if INTRO2_TYPE47_SPAWN_INDICES[index] == spawn_index {
            return Some(INTRO2_TYPE47_SUB_D_SEEDS[index]);
        }
        index += 1;
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47PublicationError {
    EntityIdentityMismatch,
    UnexpectedModel,
    MetadataMismatch,
    BehaviorContextUnavailable,
    SubARuntimeUnavailable,
    SubHRuntimeUnavailable,
    Setup,
    AlreadyPublished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47BirthSelection {
    /// 20450 Sub-A word, weighted selector, then both Guard suffixes or
    /// Wander's single suffix. No constructor draw is discarded as padding.
    Weighted,
    /// Explicit first-consumer fixture: retain the observed Guard graph,
    /// replaying only its two task suffixes, as in the accepted transcript.
    CapturedGuard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type47Admission {
    pub spawn_index: usize,
    pub seed: u8,
}

pub fn authenticate_intro2_type47(
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<Intro2Type47Admission, Intro2Type47PublicationError> {
    let Some(spawn_index) = entity.authored_spawn_index else {
        return Err(Intro2Type47PublicationError::EntityIdentityMismatch);
    };
    let Some(seed) = intro2_type47_seed_for_spawn(spawn_index) else {
        return Err(Intro2Type47PublicationError::EntityIdentityMismatch);
    };
    if !entity.active || entity.entity_type != INTRO2_TYPE47_ENTITY_TYPE {
        return Err(Intro2Type47PublicationError::EntityIdentityMismatch);
    }
    let expected_position = INTRO2_TYPE47_TTD_POSITIONS_RAW[INTRO2_TYPE47_SPAWN_INDICES
        .iter()
        .position(|&index| index == spawn_index)
        .expect("seed lookup already matched the spawn")];
    let position_raw = entity.position_raw();
    if position_raw[0] != expected_position[0] || position_raw[2] != expected_position[2] {
        return Err(Intro2Type47PublicationError::EntityIdentityMismatch);
    }
    if entity.model_slots != [Some(INTRO2_TYPE47_MODEL_ID); 4]
        || entity.model_index != Some(INTRO2_TYPE47_MODEL_ID)
    {
        return Err(Intro2Type47PublicationError::UnexpectedModel);
    }
    authenticate_type47_metadata(metadata)?;
    Ok(Intro2Type47Admission { spawn_index, seed })
}

/// Authored component and initializer shape shared by native and replay entry.
pub(crate) fn authenticate_type47_metadata(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<(), Intro2Type47PublicationError> {
    let Some(initializer) = metadata.initializer.as_ref() else {
        return Err(Intro2Type47PublicationError::MetadataMismatch);
    };
    if metadata.common_mover_topology
        != RetailRuntimeValue::Known(FRESH_LEVEL1_ORDINARY_TYPE47_COMPONENT_TOPOLOGY)
        || metadata.sub_a_propulsion_descriptor
            != RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR))
        || metadata.sub_b_lateral_descriptor
            != RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_B_DESCRIPTOR))
        || metadata.sub_c_lift_descriptor
            != RetailRuntimeValue::Known(Some(TYPE47_COMMON_DYING_SUB_C_DESCRIPTOR))
        || metadata.sub_d_steering_descriptor
            != RetailRuntimeValue::Known(Some(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_D))
        || initializer.behavior_choices.as_ref() != TYPE47_COMMON_DYING_BEHAVIOR_CHOICES
        || initializer.common_axis_descriptor != TYPE47_COMMON_DYING_COMMON_AXIS_DESCRIPTOR
        || initializer.behavior_rule_ref != 1
        || initializer.alternate_behavior_class_ref != 12
        || initializer.initializer_state_flags_raw != 0x2039
        || metadata.constructor_sound_attachment_id != RetailRuntimeValue::Known(None)
    {
        return Err(Intro2Type47PublicationError::MetadataMismatch);
    }
    match &metadata.sub_h_external_frame_descriptor {
        RetailRuntimeValue::Known(Some(descriptor))
            if descriptor.records.as_slice() == TYPE47_COMMON_DYING_SUB_H_RECORDS => {}
        _ => return Err(Intro2Type47PublicationError::MetadataMismatch),
    }
    Ok(())
}

pub fn publish_intro2_type47(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    terrain: Option<&TerrainGrid>,
    birth_selection: Intro2Type47BirthSelection,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type47Admission, Intro2Type47PublicationError> {
    let admission = authenticate_intro2_type47(entity, metadata)?;
    if entity.initial_behavior != RetailRuntimeValue::Unresolved
        || entity.current_behavior_context != RetailRuntimeValue::Unresolved
        || ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .iter()
            .any(|&slot| entity.actor_task_state(slot).is_some())
    {
        return Err(Intro2Type47PublicationError::AlreadyPublished);
    }
    let aim_and_fire_runtime = OrdinaryType47AimAndFireRuntime::from_authenticated_intro2_metadata(
        metadata,
        admission.seed,
    )
    .ok_or(Intro2Type47PublicationError::MetadataMismatch)?;
    if !matches!(
        entity.sub_a_propulsion_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        return Err(Intro2Type47PublicationError::SubARuntimeUnavailable);
    }
    if !matches!(
        entity.sub_h_external_frame_runtime,
        RetailRuntimeValue::Known(Some(_))
    ) {
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(Some(
            SubHRuntimeState::new(FRESH_LEVEL1_ORDINARY_TYPE47_SUB_H_RECORD_COUNT)
                .map_err(|_| Intro2Type47PublicationError::SubHRuntimeUnavailable)?,
        ));
    }

    let selection = match birth_selection {
        Intro2Type47BirthSelection::Weighted => {
            let RetailRuntimeValue::Known(anchor) =
                type47_d4a0_immutable_anchor_raw(Some(metadata), entity.position_raw(), terrain)
            else {
                return Err(Intro2Type47PublicationError::Setup);
            };
            // 09A80 constructs Sub-A with 20450 before D4A0/381F0 selects
            // behavior. This real state write is later replaced by 06070.
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(
                crate::common_mover::SubAPropulsionRuntime::from_20450_constructor(
                    TYPE47_COMMON_DYING_SUB_A_DESCRIPTOR,
                    next_random() as u16,
                ),
            ));
            // D4A0's type bit0x20 snaps current Y before copying +90 and
            // entering the weighted behavior constructor.
            entity.set_position_raw(anchor);
            select_initial_behavior(
                &TYPE47_COMMON_DYING_BEHAVIOR_CHOICES,
                |rule| i32::from(rule == BehaviorWeightRule::Always),
                &mut *next_random,
            )
            .map_err(|_| Intro2Type47PublicationError::Setup)?
            .ok_or(Intro2Type47PublicationError::BehaviorContextUnavailable)?
        }
        Intro2Type47BirthSelection::CapturedGuard => BehaviorSelection {
            choice_index: 0,
            program: behavior_program(u32::from(TYPE47_GUARD_BEHAVIOR_CLASS_ID))
                .ok_or(Intro2Type47PublicationError::BehaviorContextUnavailable)?,
        },
    };
    let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection)
        .ok_or(Intro2Type47PublicationError::BehaviorContextUnavailable)?;
    let publication = match selection.program.class_id {
        TYPE47_GUARD_BEHAVIOR_CLASS_ID => {
            publish_type47_guard_location(entity, context, metadata, next_random)
        }
        TYPE47_WANDER_BEHAVIOR_CLASS_ID => {
            publish_type47_wander_near(entity, context, metadata, next_random)
        }
        _ => return Err(Intro2Type47PublicationError::Setup),
    };
    if matches!(
        publication,
        FreshType47InitializerPublication::InitializerFallback { .. }
    ) {
        return Err(Intro2Type47PublicationError::Setup);
    }
    entity.initial_behavior = RetailRuntimeValue::Known(Some(selection));
    entity.intro2_type47_sub_d_frame_owner =
        intro2_type47_first_query_owner_for_seed(admission.seed);
    entity.intro2_type47_sub_d_runtime = Some(Type9SubDRuntime::from_constructor());
    entity.ordinary_type47_aim_and_fire_runtime = Some(aim_and_fire_runtime);
    // FUN_004104B0 installs a null instance +0x44 for Type 47. This birth
    // write is independent of the later mutable damage callback custody.
    entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
    // The allocator leaves retail +B2 as heap residue. Native births use the
    // same explicit deterministic empty accumulator policy as native Type 9;
    // generic constructors and all later contributions remain untouched.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    entity.type47_immutable_anchor_raw_at_0x90 = type47_d4a0_immutable_anchor_raw(
        Some(metadata),
        INTRO2_TYPE47_TTD_POSITIONS_RAW[INTRO2_TYPE47_SPAWN_INDICES
            .iter()
            .position(|&index| index == admission.spawn_index)
            .expect("admission already matched the spawn")],
        terrain,
    );
    Ok(admission)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47PrimaryVisitBlock {
    EntityIdentityMismatch,
    WanderVisitUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47PrimaryVisitResult {
    Continue,
    TaggedZeroMover,
    CommonMoverBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type47PrimaryVisit {
    pub lifetime: WanderNearLifetimeStatus,
    pub retarget: WanderNearRetarget,
    pub result: Intro2Type47PrimaryVisitResult,
}

fn commit_intro2_type47_mover_pose(entity: &mut Entity, outcome: Intro2Type47CommonMoverOutcome) {
    entity.set_heading_raw(outcome.heading_raw);
    entity.set_motion_raw(outcome.position_raw, outcome.velocity_raw);
    if let RetailRuntimeValue::Known(Some(sub_a)) = outcome.sub_a_runtime {
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(sub_a));
    }
}

fn evaluate_intro2_type47_entity_common_mover(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    frame: Intro2Type47CallbackFrame,
    target_private: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type47CommonMoverOutcome, Intro2Type47CommonMoverBlock> {
    if entity.native_type47_construction.is_some() {
        return run_native_type47_common_mover(
            entity,
            NativeType47MoverFrame {
                metadata: metadata.ok_or(Intro2Type47CommonMoverBlock::TypeMetadataUnavailable)?,
                terrain: terrain.ok_or(Intro2Type47CommonMoverBlock::NativeRuntime("terrain"))?,
                elapsed_micros: frame.elapsed_micros,
                global_elapsed_micros: frame.global_elapsed_micros,
                target: target_private,
                tracked_target,
            },
            next_random,
        );
    }
    let elapsed_micros = frame.elapsed_micros;
    let Some(seed) = live_type47_sub_d_seed(entity) else {
        return Err(Intro2Type47CommonMoverBlock::SubDFirstQueryUnavailable);
    };
    let position_raw = entity.position_raw();
    let velocity_raw = entity.velocity_raw();
    let heading_raw = entity.heading_raw();
    let roll_raw = entity.rotation_heading_pitch_roll_raw()[2] as u16;
    let sub_a_runtime = entity.sub_a_propulsion_runtime;
    let (body_right_q31, body_forward_q31, body_up_q31) = match entity.physical_body_basis_q31 {
        RetailRuntimeValue::Known(Type9BodyBasis {
            lateral,
            forward,
            up,
        }) => (
            RetailRuntimeValue::Known(lateral),
            RetailRuntimeValue::Known(forward),
            RetailRuntimeValue::Known(up),
        ),
        RetailRuntimeValue::Unresolved => (
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
        ),
    };
    let mut sub_h_runtime = match &mut entity.sub_h_external_frame_runtime {
        RetailRuntimeValue::Known(Some(runtime)) => Some(runtime),
        _ => None,
    };
    evaluate_intro2_type47_common_mover(
        Intro2Type47CommonMoverRequest {
            entity_id: entity.id,
            entity_type: entity.entity_type,
            metadata,
            position_raw,
            velocity_raw,
            heading_raw,
            roll_raw,
            sub_a_runtime,
            sub_d_stagger_seed: Some(seed),
            sub_d_frame_owner: entity.intro2_type47_sub_d_frame_owner.as_mut(),
            sub_d_runtime: entity.intro2_type47_sub_d_runtime.as_mut(),
            body_right_q31,
            body_forward_q31,
            body_up_q31,
            sub_h_runtime: sub_h_runtime.as_deref_mut(),
            terrain,
            global_elapsed_micros: frame.global_elapsed_micros,
            target_private,
            tracked_target,
            elapsed_micros,
        },
        next_random,
    )
}

pub fn tick_intro2_type47_primary(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    terrain: Option<&TerrainGrid>,
    frame: Intro2Type47CallbackFrame,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type47PrimaryVisit, Intro2Type47PrimaryVisitBlock> {
    let elapsed_micros = frame.elapsed_micros;
    let Some(seed) = live_type47_sub_d_seed(entity) else {
        return Err(Intro2Type47PrimaryVisitBlock::EntityIdentityMismatch);
    };
    if !entity.active || entity.entity_type != INTRO2_TYPE47_ENTITY_TYPE {
        return Err(Intro2Type47PrimaryVisitBlock::EntityIdentityMismatch);
    }
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(Intro2Type47PrimaryVisitBlock::WanderVisitUnavailable);
    };
    if !matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::OrdinaryType9Wander(_))
    ) {
        return Err(Intro2Type47PrimaryVisitBlock::WanderVisitUnavailable);
    }
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(lifetime) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::OrdinaryType9Wander(state) = runtime else {
            unreachable!("Primary identity already authenticated as OrdinaryType9Wander")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Err(Intro2Type47PrimaryVisitBlock::WanderVisitUnavailable);
    };

    let mut stage = match entity.type47_immutable_anchor_raw_at_0x90 {
        RetailRuntimeValue::Known(anchor) => {
            let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
                entity.actor_tasks.task_state_mut(visit.task_id)
            else {
                let _ = entity.actor_tasks.finish_exact_visit(visit);
                return Err(Intro2Type47PrimaryVisitBlock::WanderVisitUnavailable);
            };
            state.stage_callback(anchor, &mut *next_random)
        }
        RetailRuntimeValue::Unresolved => {
            let Some(ActorTaskRuntime::OrdinaryType9Wander(state)) =
                entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
            else {
                let _ = entity.actor_tasks.finish_exact_visit(visit);
                return Err(Intro2Type47PrimaryVisitBlock::WanderVisitUnavailable);
            };
            crate::ordinary_type9_wander_owner::OrdinaryType9WanderCallbackStage::retained(
                state.private_state(),
            )
        }
    };

    debug_assert_eq!(live_type47_sub_d_seed(entity), Some(seed));
    let mover = evaluate_intro2_type47_entity_common_mover(
        entity,
        metadata,
        terrain,
        frame,
        stage.private_state_mut(),
        RetailRuntimeValue::Known(None),
        next_random,
    );
    let result = match mover {
        Ok(outcome) => {
            commit_intro2_type47_mover_pose(entity, outcome);
            match outcome.result {
                ChaseTargetCommonMoverReturn::NonZero => Intro2Type47PrimaryVisitResult::Continue,
                ChaseTargetCommonMoverReturn::Zero => {
                    Intro2Type47PrimaryVisitResult::TaggedZeroMover
                }
            }
        }
        Err(_) => Intro2Type47PrimaryVisitResult::CommonMoverBlocked,
    };
    // Native mover phases commit target-private writes before a later block,
    // just as they retain body/cache writes. Captured atomic replay retains
    // its original success-only commit contract.
    if entity.native_type47_construction.is_some()
        || matches!(
            result,
            Intro2Type47PrimaryVisitResult::Continue
                | Intro2Type47PrimaryVisitResult::TaggedZeroMover
        )
    {
        if let Some(ActorTaskRuntime::OrdinaryType9Wander(surviving)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        {
            stage.commit(surviving);
        }
    }
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    Ok(Intro2Type47PrimaryVisit {
        lifetime,
        retarget: stage.retarget(),
        result,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type47SchedulerOwner {
    entity_id: u32,
    next_transaction_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47SchedulerAdoptionError {
    EntityUnavailable,
    GraphUnavailable { entity_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type47SchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47SchedulerProductionBlock {
    CommonMoverUnavailable,
    SearchContextUnresolved,
    AcquisitionVisitUnavailable,
    Acquisition(GuardLocationAcquisitionCallbackError),
    ChaseSearchContextUnresolved,
    ChaseTargetStateUnresolved,
    ChaseVisitUnavailable,
    C690MetadataUnavailable,
    C690(Type47C690ReselectionBlock),
    C690Publication(FreshType47InitialBehaviorError),
    C690AlternateClass12(crate::ordinary_type47_death_live::Type47CommonDyingPublicationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47ChaseVisitResult {
    Tagged(ChaseTargetTaggedSingleton),
    Continue,
    CommonMoverBlocked(Intro2Type47CommonMoverBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2Type47ChaseVisit {
    pub prefix: ChaseTargetCallbackPrefix,
    pub result: Intro2Type47ChaseVisitResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2Type47PostChaseGuardPass {
    pub acquisition_prefix: GuardLocationAcquisitionCallbackPrefix,
    pub acquisition: GuardLocationAcquisitionCallbackResult,
    pub same_pass_aim: Option<Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type47SchedulerProductionOutcome {
    Primary {
        entity_id: u32,
        visit: Intro2Type47PrimaryVisit,
        acquisition_prefix: GuardLocationAcquisitionCallbackPrefix,
        acquisition: GuardLocationAcquisitionCallbackResult,
        same_pass_aim: Option<Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError>>,
    },
    WanderNear {
        entity_id: u32,
        visit: Intro2Type47PrimaryVisit,
    },
    /// Primary returned9C01 or exceeded5000ms; C690 runs after unwind.
    /// A replacement Primary waits, while new later physical slots run now.
    RootTransition {
        entity_id: u32,
        visit: Intro2Type47PrimaryVisit,
        transition: Type47SchedulerC690Transition,
        post_primary_guard: Option<Intro2Type47PostChaseGuardPass>,
    },
    Blocked {
        entity_id: u32,
        reason: Intro2Type47SchedulerProductionBlock,
    },
    Pursuing {
        entity_id: u32,
        chase: Intro2Type47ChaseVisit,
        post_chase_plus00_c690: Option<Type47SchedulerC690Transition>,
        post_chase_guard: Option<Intro2Type47PostChaseGuardPass>,
        aim: Option<Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError>>,
        post_aim_plus04_c690: Option<Type47SchedulerC690Transition>,
        post_aim_plus00_c690: Option<Type47SchedulerC690Transition>,
    },
    Dropped {
        entity_id: u32,
        reason: Intro2Type47SchedulerProductionDrop,
    },
}

impl Intro2Type47SchedulerProductionOutcome {
    pub(crate) fn replacement_common_dying_owner(
        &self,
    ) -> Option<crate::ordinary_type47_death_live::FreshLevelOneType47CommonDyingOwner> {
        if let Self::RootTransition { transition, .. } = self {
            return match transition {
                Type47SchedulerC690Transition::Class12Published { publication, .. } => {
                    Some(publication.owner)
                }
                _ => None,
            };
        }
        let Self::Pursuing {
            post_chase_plus00_c690,
            post_aim_plus04_c690,
            post_aim_plus00_c690,
            ..
        } = self
        else {
            return None;
        };
        [
            post_chase_plus00_c690,
            post_aim_plus04_c690,
            post_aim_plus00_c690,
        ]
        .into_iter()
        .flatten()
        .find_map(|transition| match transition {
            Type47SchedulerC690Transition::Class12Published { publication, .. } => {
                Some(publication.owner)
            }
            _ => None,
        })
    }

    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::Primary { entity_id, .. }
            | Self::WanderNear { entity_id, .. }
            | Self::RootTransition { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Pursuing { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

pub struct Intro2Type47SchedulerOwnerTick {
    pub outcome: Intro2Type47SchedulerProductionOutcome,
    pub retained_owner: Option<Intro2Type47SchedulerOwner>,
}

impl Intro2Type47SchedulerOwner {
    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    fn take_aim_transaction_pair(
        &mut self,
    ) -> (AimAndFireEmitterTransactionId, GenericEmitterTransactionId) {
        let parent = self.next_transaction_id.max(1);
        let child = parent.wrapping_add(1).max(1);
        self.next_transaction_id = child.wrapping_add(1).max(1);
        (
            AimAndFireEmitterTransactionId::new(parent).expect("parent id is nonzero"),
            GenericEmitterTransactionId::new(child).expect("child id is nonzero"),
        )
    }

    pub fn adopt_published(entity: &Entity) -> Result<Self, Intro2Type47SchedulerAdoptionError> {
        if !intro2_type47_primary_graph_authenticates(entity)
            && !intro2_type47_wander_graph_authenticates(entity)
            && !intro2_type47_pursuing_graph_authenticates(entity)
        {
            return Err(Intro2Type47SchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            });
        }
        Ok(Self {
            entity_id: entity.id,
            next_transaction_id: 1,
        })
    }
}

fn intro2_type47_primary_graph_authenticates(entity: &Entity) -> bool {
    intro2_type47_cohort_runtime_authenticates(entity)
        && crate::type47_impact_live::type47_live_graph_is_initial_guard(entity)
}

fn intro2_type47_wander_graph_authenticates(entity: &Entity) -> bool {
    intro2_type47_cohort_runtime_authenticates(entity)
        && crate::type47_impact_live::type47_live_graph_is_initial_wander(entity)
}

pub(crate) fn intro2_type47_cohort_runtime_authenticates(entity: &Entity) -> bool {
    let Some(seed) = live_type47_sub_d_seed(entity) else {
        return false;
    };
    entity.active
        && entity.entity_type == INTRO2_TYPE47_ENTITY_TYPE
        && entity.model_slots == [Some(INTRO2_TYPE47_MODEL_ID); 4]
        && entity.model_index == Some(INTRO2_TYPE47_MODEL_ID)
        && entity.intro2_type47_sub_d_frame_owner.is_some()
        && entity.intro2_type47_sub_d_runtime.is_some()
        && entity
            .ordinary_type47_aim_and_fire_runtime
            .as_ref()
            .is_some_and(|runtime| {
                if entity.native_type47_construction.is_some() {
                    runtime.authenticates_native_cohort(seed)
                } else {
                    runtime.authenticates_intro2_cohort(seed)
                }
            })
}

/// Scene-independent native identity precedes the explicit replay seed map.
/// This entity-only proof is used inside an already manager-authenticated
/// visit; outer owners additionally prove the issuing allocation generation.
fn live_type47_sub_d_seed(entity: &Entity) -> Option<u8> {
    if let Some(receipt) = entity.native_type47_construction {
        return receipt
            .entity_authenticates(entity)
            .then_some(receipt.sub_d_seed());
    }
    entity
        .authored_spawn_index
        .and_then(intro2_type47_seed_for_spawn)
}

fn intro2_type47_pursuing_graph_authenticates(entity: &Entity) -> bool {
    intro2_type47_cohort_runtime_authenticates(entity)
        && type47_guard_pursuing_graph_contract_authenticates(entity)
}

pub fn tick_intro2_type47_scheduler_owner(
    manager: &mut crate::entity::EntityManager,
    mut owner: Intro2Type47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    frame: Intro2Type47CallbackFrame,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type47SchedulerOwnerTick {
    if !crate::shared_type47::type47_manager_allocation_authenticates(manager, owner.entity_id()) {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Intro2Type47SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    if manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .is_some_and(intro2_type47_pursuing_graph_authenticates)
    {
        return tick_intro2_type47_pursuing(
            manager,
            owner,
            world_fx,
            terrain,
            frame,
            next_shared_random,
        );
    }
    if manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .is_some_and(intro2_type47_wander_graph_authenticates)
    {
        return tick_intro2_type47_lone_wander(
            manager,
            owner,
            world_fx,
            terrain,
            frame,
            next_shared_random,
        );
    }
    let metadata = manager
        .type_runtime_metadata(INTRO2_TYPE47_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.intro2_type47_entity_mut(owner.entity_id()) else {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Intro2Type47SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    if !intro2_type47_primary_graph_authenticates(entity) {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Intro2Type47SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let visit =
        match tick_intro2_type47_primary(entity, metadata.as_ref(), terrain, frame, &mut || {
            next_shared_random(world_fx)
        }) {
            Ok(visit) => visit,
            Err(_) => {
                return Intro2Type47SchedulerOwnerTick {
                    outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Intro2Type47SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                };
            }
        };
    if visit.result == Intro2Type47PrimaryVisitResult::CommonMoverBlocked {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Blocked {
                entity_id: owner.entity_id(),
                reason: Intro2Type47SchedulerProductionBlock::CommonMoverUnavailable,
            },
            retained_owner: Some(owner),
        };
    }
    if type47_primary_requests_plus00(&visit) {
        return tick_type47_initial_primary_transition(
            manager,
            owner,
            world_fx,
            visit,
            frame,
            next_shared_random,
        );
    }
    let pass = match tick_intro2_guard_acquisition_after_primary(
        manager,
        &mut owner,
        world_fx,
        frame,
        next_shared_random,
    ) {
        Ok(pass) => pass,
        Err(Intro2GuardAcquisitionPassFailure::Blocked(reason)) => {
            return Intro2Type47SchedulerOwnerTick {
                outcome: Intro2Type47SchedulerProductionOutcome::Blocked {
                    entity_id: owner.entity_id(),
                    reason,
                },
                retained_owner: Some(owner),
            };
        }
        Err(Intro2GuardAcquisitionPassFailure::Dropped(reason)) => {
            return Intro2Type47SchedulerOwnerTick {
                outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason,
                },
                retained_owner: None,
            };
        }
    };
    let retained_owner =
        intro2_type47_live_graph_authenticates(manager, owner.entity_id()).then_some(owner);
    Intro2Type47SchedulerOwnerTick {
        outcome: Intro2Type47SchedulerProductionOutcome::Primary {
            entity_id: owner.entity_id(),
            visit,
            acquisition_prefix: pass.acquisition_prefix,
            acquisition: pass.acquisition,
            same_pass_aim: pass.same_pass_aim,
        },
        retained_owner,
    }
}

enum Intro2GuardAcquisitionPassFailure {
    Blocked(Intro2Type47SchedulerProductionBlock),
    Dropped(Intro2Type47SchedulerProductionDrop),
}

fn tick_intro2_guard_acquisition_after_primary(
    manager: &mut crate::entity::EntityManager,
    owner: &mut Intro2Type47SchedulerOwner,
    world_fx: &mut WorldFx,
    frame: Intro2Type47CallbackFrame,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Intro2Type47PostChaseGuardPass, Intro2GuardAcquisitionPassFailure> {
    let candidates = manager
        .retail_live_order_ids()
        .filter_map(|id| {
            manager
                .iter_all()
                .find(|entity| entity.id == id)
                .map(type47_guard_entity_ref)
        })
        .collect::<Vec<GuardLocationEntityRef>>();
    let metadata = manager
        .type_runtime_metadata(INTRO2_TYPE47_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.intro2_type47_entity_mut(owner.entity_id()) else {
        return Err(Intro2GuardAcquisitionPassFailure::Dropped(
            Intro2Type47SchedulerProductionDrop::EntityUnavailable,
        ));
    };
    let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
        return Err(Intro2GuardAcquisitionPassFailure::Blocked(
            Intro2Type47SchedulerProductionBlock::SearchContextUnresolved,
        ));
    };
    let mut search_context = GuardLocationSearchContext::new(
        WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
        GuardLocationCandidateFilter::from_raw(axis.raw_word_at_0x04),
    );
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary) else {
        return Err(Intro2GuardAcquisitionPassFailure::Dropped(
            Intro2Type47SchedulerProductionDrop::GraphMismatch,
        ));
    };
    let acquisition_visit = ActorTaskVisit {
        slot: ActorTaskSlot::Secondary,
        task_id,
    };
    let Some(acquisition_prefix) =
        entity
            .actor_tasks
            .begin_exact_visit_with(acquisition_visit, |runtime| {
                let ActorTaskRuntime::GuardLocationAcquisition(state) = runtime else {
                    unreachable!("authenticated Intro2 Guard Secondary changed family")
                };
                let random = next_shared_random(world_fx);
                state.before_callback(&mut search_context, random)
            })
    else {
        return Err(Intro2GuardAcquisitionPassFailure::Blocked(
            Intro2Type47SchedulerProductionBlock::AcquisitionVisitUnavailable,
        ));
    };
    let owner_ref = type47_guard_entity_ref(entity);
    let acquisition = evaluate_guard_location_acquisition_callback(
        acquisition_prefix,
        owner_ref,
        &candidates,
        Some(
            |handoff: crate::guard_location_owner::acquisition::GuardLocationCandidateHandoff| {
                let Some(metadata) = metadata.as_ref() else {
                    return 1;
                };
                match apply_type47_guard_pursuing_handoff(entity, metadata, handoff.candidate.id) {
                    Ok(()) => 0,
                    Err(_) => 1,
                }
            },
        ),
    );
    if entity
        .actor_tasks
        .wrapper_flags(acquisition_visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(acquisition_visit);
    }
    let acquisition = acquisition.map_err(|error| {
        Intro2GuardAcquisitionPassFailure::Blocked(
            Intro2Type47SchedulerProductionBlock::Acquisition(error),
        )
    })?;
    let same_pass_aim = matches!(
        acquisition,
        GuardLocationAcquisitionCallbackResult::TaggedCandidateAccepted { .. }
    )
    .then(|| tick_intro2_bound_aim(manager, owner, world_fx, frame));
    Ok(Intro2Type47PostChaseGuardPass {
        acquisition_prefix,
        acquisition,
        same_pass_aim,
    })
}

fn intro2_type47_live_graph_authenticates(
    manager: &crate::entity::EntityManager,
    entity_id: u32,
) -> bool {
    manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(|entity| {
            intro2_type47_primary_graph_authenticates(entity)
                || intro2_type47_wander_graph_authenticates(entity)
                || intro2_type47_pursuing_graph_authenticates(entity)
        })
}

fn tick_intro2_type47_lone_wander(
    manager: &mut crate::entity::EntityManager,
    owner: Intro2Type47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    frame: Intro2Type47CallbackFrame,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    let metadata = manager
        .type_runtime_metadata(INTRO2_TYPE47_ENTITY_TYPE)
        .cloned();
    let Some(entity) = manager.intro2_type47_entity_mut(entity_id) else {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type47SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    if !intro2_type47_wander_graph_authenticates(entity) {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type47SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let visit =
        match tick_intro2_type47_primary(entity, metadata.as_ref(), terrain, frame, &mut || {
            next_shared_random(world_fx)
        }) {
            Ok(visit) => visit,
            Err(_) => {
                return Intro2Type47SchedulerOwnerTick {
                    outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type47SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                };
            }
        };
    if type47_primary_requests_plus00(&visit) {
        return tick_type47_initial_primary_transition(
            manager,
            owner,
            world_fx,
            visit,
            frame,
            next_shared_random,
        );
    }
    Intro2Type47SchedulerOwnerTick {
        outcome: Intro2Type47SchedulerProductionOutcome::WanderNear { entity_id, visit },
        retained_owner: intro2_type47_live_graph_authenticates(manager, entity_id).then_some(owner),
    }
}

fn type47_primary_requests_plus00(visit: &Intro2Type47PrimaryVisit) -> bool {
    visit.result == Intro2Type47PrimaryVisitResult::TaggedZeroMover
        || (visit.result == Intro2Type47PrimaryVisitResult::Continue
            && visit.lifetime == WanderNearLifetimeStatus::OwnerTransitionDue)
}

/// 01120 reads each later slot freshly after the completed Primary callback.
/// C690 may install Guard's Secondary and Aim may then be installed in slot2;
/// the replacement slot0 is never visited a second time in this pass.
fn tick_type47_initial_primary_transition(
    manager: &mut crate::entity::EntityManager,
    mut owner: Intro2Type47SchedulerOwner,
    world_fx: &mut WorldFx,
    visit: Intro2Type47PrimaryVisit,
    frame: Intro2Type47CallbackFrame,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    let transition = match apply_intro2_type47_pursuing_c690(
        manager,
        entity_id,
        world_fx,
        Type47C690Slot::Plus00,
        next_shared_random,
    ) {
        Ok(transition) => transition,
        Err(reason) => {
            return Intro2Type47SchedulerOwnerTick {
                outcome: Intro2Type47SchedulerProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
            }
        }
    };
    let mut post_primary_guard = None;
    if manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(intro2_type47_primary_graph_authenticates)
    {
        post_primary_guard = match tick_intro2_guard_acquisition_after_primary(
            manager,
            &mut owner,
            world_fx,
            frame,
            next_shared_random,
        ) {
            Ok(pass) => Some(pass),
            Err(Intro2GuardAcquisitionPassFailure::Blocked(reason)) => {
                return Intro2Type47SchedulerOwnerTick {
                    outcome: Intro2Type47SchedulerProductionOutcome::Blocked { entity_id, reason },
                    retained_owner: Some(owner),
                }
            }
            Err(Intro2GuardAcquisitionPassFailure::Dropped(reason)) => {
                return Intro2Type47SchedulerOwnerTick {
                    outcome: Intro2Type47SchedulerProductionOutcome::Dropped { entity_id, reason },
                    retained_owner: None,
                }
            }
        };
    }
    Intro2Type47SchedulerOwnerTick {
        outcome: Intro2Type47SchedulerProductionOutcome::RootTransition {
            entity_id,
            visit,
            transition,
            post_primary_guard,
        },
        retained_owner: intro2_type47_live_graph_authenticates(manager, entity_id).then_some(owner),
    }
}

fn tick_intro2_bound_aim(
    manager: &mut crate::entity::EntityManager,
    owner: &mut Intro2Type47SchedulerOwner,
    world_fx: &mut WorldFx,
    frame: Intro2Type47CallbackFrame,
) -> Result<OrdinaryType47AimAndFireTickOutcome, OrdinaryType47LiveError> {
    let elapsed_micros = frame.elapsed_micros;
    let Some(entity) = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
    else {
        return Err(OrdinaryType47LiveError::SourceEntityMissing);
    };
    let aim_owner = bind_intro2_type47_aim_and_fire(entity)?;
    let (parent_transaction_id, child_transaction_id) = owner.take_aim_transaction_pair();
    tick_intro2_type47_aim_and_fire(
        manager,
        world_fx,
        aim_owner,
        OrdinaryType47AimAndFireFrameRequest {
            parent_transaction_id,
            child_transaction_id,
            elapsed_micros,
        },
    )
}

fn tick_intro2_type47_pursuing(
    manager: &mut crate::entity::EntityManager,
    mut owner: Intro2Type47SchedulerOwner,
    world_fx: &mut WorldFx,
    terrain: Option<&TerrainGrid>,
    frame: Intro2Type47CallbackFrame,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type47SchedulerOwnerTick {
    let entity_id = owner.entity_id();
    let chase = match tick_intro2_type47_pursuing_chase(
        manager,
        world_fx,
        entity_id,
        terrain,
        frame,
        next_shared_random,
    ) {
        Ok(chase) => chase,
        Err(reason) => {
            return Intro2Type47SchedulerOwnerTick {
                outcome: Intro2Type47SchedulerProductionOutcome::Blocked { entity_id, reason },
                retained_owner: Some(owner),
            };
        }
    };
    let mut post_chase_plus00_c690 = None;
    let mut post_chase_guard = None;
    if matches!(
        chase.result,
        Intro2Type47ChaseVisitResult::CommonMoverBlocked(_)
    ) {
        return Intro2Type47SchedulerOwnerTick {
            outcome: Intro2Type47SchedulerProductionOutcome::Pursuing {
                entity_id,
                chase,
                post_chase_plus00_c690: None,
                post_chase_guard: None,
                aim: None,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690: None,
            },
            retained_owner: Some(owner),
        };
    }
    if intro2_type47_chase_requests_plus00(&chase) {
        let transition = match apply_intro2_type47_pursuing_c690(
            manager,
            entity_id,
            world_fx,
            Type47C690Slot::Plus00,
            next_shared_random,
        ) {
            Ok(transition) => transition,
            Err(reason) => {
                return Intro2Type47SchedulerOwnerTick {
                    outcome: Intro2Type47SchedulerProductionOutcome::Blocked { entity_id, reason },
                    retained_owner: Some(owner),
                };
            }
        };
        let completed = transition.completed();
        let published_guard = matches!(
            &transition,
            Type47SchedulerC690Transition::GuardPublished { .. }
        );
        post_chase_plus00_c690 = Some(transition);
        if published_guard {
            post_chase_guard = match tick_intro2_guard_acquisition_after_primary(
                manager,
                &mut owner,
                world_fx,
                frame,
                next_shared_random,
            ) {
                Ok(pass) => Some(pass),
                Err(Intro2GuardAcquisitionPassFailure::Blocked(reason)) => {
                    return Intro2Type47SchedulerOwnerTick {
                        outcome: Intro2Type47SchedulerProductionOutcome::Blocked {
                            entity_id,
                            reason,
                        },
                        retained_owner: Some(owner),
                    };
                }
                Err(Intro2GuardAcquisitionPassFailure::Dropped(reason)) => {
                    return Intro2Type47SchedulerOwnerTick {
                        outcome: Intro2Type47SchedulerProductionOutcome::Dropped {
                            entity_id,
                            reason,
                        },
                        retained_owner: None,
                    };
                }
            };
            return Intro2Type47SchedulerOwnerTick {
                outcome: Intro2Type47SchedulerProductionOutcome::Pursuing {
                    entity_id,
                    chase,
                    post_chase_plus00_c690,
                    post_chase_guard,
                    aim: None,
                    post_aim_plus04_c690: None,
                    post_aim_plus00_c690: None,
                },
                retained_owner: intro2_type47_live_graph_authenticates(manager, entity_id)
                    .then_some(owner),
            };
        }
        if completed {
            return Intro2Type47SchedulerOwnerTick {
                outcome: Intro2Type47SchedulerProductionOutcome::Pursuing {
                    entity_id,
                    chase,
                    post_chase_plus00_c690,
                    post_chase_guard,
                    aim: None,
                    post_aim_plus04_c690: None,
                    post_aim_plus00_c690: None,
                },
                retained_owner: intro2_type47_live_graph_authenticates(manager, entity_id)
                    .then_some(owner),
            };
        }
    }

    let aim = Some(tick_intro2_bound_aim(manager, &mut owner, world_fx, frame));
    let mut post_aim_plus04_c690 = None;
    let mut post_aim_plus00_c690 = None;
    if let Some(Ok(outcome)) = aim.as_ref() {
        match outcome.resolution.outcome {
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::TaggedInvalidTarget { .. },
            } => {
                let transition = match apply_intro2_type47_pursuing_c690(
                    manager,
                    entity_id,
                    world_fx,
                    Type47C690Slot::Plus04,
                    next_shared_random,
                ) {
                    Ok(transition) => transition,
                    Err(reason) => {
                        return Intro2Type47SchedulerOwnerTick {
                            outcome: Intro2Type47SchedulerProductionOutcome::Blocked {
                                entity_id,
                                reason,
                            },
                            retained_owner: Some(owner),
                        };
                    }
                };
                let suppressed = !transition.completed();
                post_aim_plus04_c690 = Some(transition);
                if suppressed
                    && outcome.resolution.prefix.lifetime_status
                        == AimAndFireLifetimeStatus::OwnerTransitionDue
                {
                    post_aim_plus00_c690 = Some(
                        match apply_intro2_type47_pursuing_c690(
                            manager,
                            entity_id,
                            world_fx,
                            Type47C690Slot::Plus00,
                            next_shared_random,
                        ) {
                            Ok(transition) => transition,
                            Err(reason) => {
                                return Intro2Type47SchedulerOwnerTick {
                                    outcome: Intro2Type47SchedulerProductionOutcome::Blocked {
                                        entity_id,
                                        reason,
                                    },
                                    retained_owner: Some(owner),
                                };
                            }
                        },
                    );
                }
            }
            AimAndFireFrameOutcome::RequestOwnerTransition {
                reason: AimAndFireTransitionReason::LifetimeExpired,
            } => {
                post_aim_plus00_c690 = Some(
                    match apply_intro2_type47_pursuing_c690(
                        manager,
                        entity_id,
                        world_fx,
                        Type47C690Slot::Plus00,
                        next_shared_random,
                    ) {
                        Ok(transition) => transition,
                        Err(reason) => {
                            return Intro2Type47SchedulerOwnerTick {
                                outcome: Intro2Type47SchedulerProductionOutcome::Blocked {
                                    entity_id,
                                    reason,
                                },
                                retained_owner: Some(owner),
                            };
                        }
                    },
                );
            }
            AimAndFireFrameOutcome::Continue
            | AimAndFireFrameOutcome::ReturnGenericEmitterResult(_) => {}
        }
    }
    Intro2Type47SchedulerOwnerTick {
        outcome: Intro2Type47SchedulerProductionOutcome::Pursuing {
            entity_id,
            chase,
            post_chase_plus00_c690,
            post_chase_guard,
            aim,
            post_aim_plus04_c690,
            post_aim_plus00_c690,
        },
        retained_owner: intro2_type47_live_graph_authenticates(manager, entity_id).then_some(owner),
    }
}

fn intro2_type47_chase_requests_plus00(chase: &Intro2Type47ChaseVisit) -> bool {
    matches!(chase.result, Intro2Type47ChaseVisitResult::Tagged(_))
        || (matches!(chase.result, Intro2Type47ChaseVisitResult::Continue)
            && chase.prefix.lifetime_status == ChaseTargetLifetimeStatus::OwnerTransitionDue)
}

fn apply_intro2_type47_pursuing_c690(
    manager: &mut crate::entity::EntityManager,
    entity_id: u32,
    world_fx: &mut WorldFx,
    slot: Type47C690Slot,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Type47SchedulerC690Transition, Intro2Type47SchedulerProductionBlock> {
    let metadata = manager
        .type_runtime_metadata(INTRO2_TYPE47_ENTITY_TYPE)
        .cloned()
        .ok_or(Intro2Type47SchedulerProductionBlock::C690MetadataUnavailable)?;
    let entity = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .ok_or(Intro2Type47SchedulerProductionBlock::C690MetadataUnavailable)?;
    let reselection =
        plan_type47_scheduler_c690_live(entity, &metadata, slot, || next_shared_random(world_fx))
            .map_err(Intro2Type47SchedulerProductionBlock::C690)?;
    match reselection {
        Type47C690Reselection::SuppressedByEntityState => {
            Ok(Type47SchedulerC690Transition::SuppressedByEntityState { slot })
        }
        Type47C690Reselection::Applied(
            planned @ Type47ImpactC690Outcome::AlternateCommonDying { .. },
        ) => {
            let entity = manager
                .intro2_type47_entity_mut(entity_id)
                .ok_or(Intro2Type47SchedulerProductionBlock::C690MetadataUnavailable)?;
            let publication =
                crate::ordinary_type47_death_live::publish_type47_c690_alternate_class12(
                    entity, &metadata, world_fx,
                )
                .map_err(Intro2Type47SchedulerProductionBlock::C690AlternateClass12)?;
            Ok(Type47SchedulerC690Transition::Class12Published {
                slot,
                planned,
                publication,
            })
        }
        Type47C690Reselection::Applied(
            planned @ Type47ImpactC690Outcome::Weighted { selection, .. },
        ) => {
            let entity = manager
                .intro2_type47_entity_mut(entity_id)
                .ok_or(Intro2Type47SchedulerProductionBlock::C690MetadataUnavailable)?;
            let cohort = crate::type47_initial_behavior_live::live_type47_cohort(entity)
                .ok_or(Intro2Type47SchedulerProductionBlock::C690MetadataUnavailable)?;
            let initializer = publish_type47_reselected_initial_behavior(
                entity, &metadata, cohort, selection, world_fx,
            )
            .map_err(Intro2Type47SchedulerProductionBlock::C690Publication)?;
            Ok(match selection.program.class_id {
                32 => Type47SchedulerC690Transition::GuardPublished {
                    slot,
                    planned,
                    initializer,
                },
                6 => Type47SchedulerC690Transition::WanderPublished {
                    slot,
                    planned,
                    initializer,
                },
                _ => unreachable!("C690 planner admits only Type-47 Guard/Wander choices"),
            })
        }
    }
}

fn tick_intro2_type47_pursuing_chase(
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    terrain: Option<&TerrainGrid>,
    frame: Intro2Type47CallbackFrame,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Result<Intro2Type47ChaseVisit, Intro2Type47SchedulerProductionBlock> {
    let elapsed_micros = frame.elapsed_micros;
    let metadata = manager
        .type_runtime_metadata(INTRO2_TYPE47_ENTITY_TYPE)
        .cloned();
    let (target_id, axis, owner_position_raw) = {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
        };
        let RetailRuntimeValue::Known(Some(target_id)) = context.target_handle_at_0x08() else {
            return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
        };
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Err(Intro2Type47SchedulerProductionBlock::ChaseSearchContextUnresolved);
        };
        (target_id, axis, entity.position_raw())
    };
    let (target_state, tracked_target) = type47_chase_target_runtime_state(manager, target_id)
        .map_err(|_| Intro2Type47SchedulerProductionBlock::ChaseTargetStateUnresolved)?;
    let Some(entity) = manager.intro2_type47_entity_mut(entity_id) else {
        return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::ChaseTarget(state) = runtime else {
            unreachable!("authenticated Intro2 pursuing Primary changed family")
        };
        state.before_callback(elapsed_micros)
    }) else {
        return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
    };
    let Some(ActorTaskRuntime::ChaseTarget(state)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
        return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
    };
    let mut stage = state.stage_callback();
    let mut movement = ();
    let mut controller = ();
    let mut applied_mover = None;
    let evaluated = evaluate_chase_target_callback(
        &mut stage,
        ChaseTargetCallbackRequest {
            visit,
            entity_id,
            owner_position_raw,
            route_range: WrappedAxisRange::from_raw(axis.strict_axis_limit_raw),
            proximity_control: ChaseTargetProximityControl::Disabled,
            movement_state: &mut movement,
            controller_context: &mut controller,
            elapsed_micros,
            scheduler_mode: 0,
        },
        |_| Ok::<_, ()>(target_state),
        |request| {
            let outcome = evaluate_intro2_type47_entity_common_mover(
                entity,
                metadata.as_ref(),
                terrain,
                frame,
                request.target_state,
                tracked_target,
                &mut || next_shared_random(world_fx),
            )?;
            let result = outcome.result;
            applied_mover = Some(outcome);
            Ok::<_, Intro2Type47CommonMoverBlock>(result)
        },
        |_, _| unreachable!("Intro2 Type-47 Chase disables proximity-controller writes"),
    );
    let result = match &evaluated {
        Ok(ChaseTargetCallbackResult::Tagged(singleton)) => {
            Some(Intro2Type47ChaseVisitResult::Tagged(*singleton))
        }
        Ok(ChaseTargetCallbackResult::Continue { .. }) => {
            Some(Intro2Type47ChaseVisitResult::Continue)
        }
        Err(crate::chase_target::ChaseTargetCallbackError::CommonMover { error, .. }) => Some(
            Intro2Type47ChaseVisitResult::CommonMoverBlocked(error.clone()),
        ),
        Err(_) => None,
    };
    let survived = if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        entity.actor_tasks.finish_exact_visit(visit)
    } else {
        false
    };
    if (evaluated.is_ok() || entity.native_type47_construction.is_some()) && survived {
        if let Some(ActorTaskRuntime::ChaseTarget(surviving)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        {
            stage.commit(surviving);
        }
    }
    let Some(result) = result else {
        return Err(Intro2Type47SchedulerProductionBlock::ChaseVisitUnavailable);
    };
    if let Some(outcome) = applied_mover {
        commit_intro2_type47_mover_pose(entity, outcome);
    }
    Ok(Intro2Type47ChaseVisit { prefix, result })
}
