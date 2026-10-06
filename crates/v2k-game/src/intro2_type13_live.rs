//! Native Intro2 type-13 birth publication and live visit.
//!
//! TTD `V200001.run` and the accepted Intro2 actor-task transcript construct
//! one type-13 identity (spawn 0, handle `04fc0001`, position
//! `b300,0200,2800`) and publish class-7 variant-0 `FUN_0040B6C0`: slot-1
//! `FUN_00402080` plus duration-500 slot-0 `FUN_00402BA0`. This owner
//! authenticates that identity, applies `FUN_0041B8C0`, then the native
//! weighted class-5 ACD0 or class-7 B6C0 graph, and ticks the published graph
//! in retail slot order. Slot-0 now commits Type-13's classifier-free common
//! mover through D/G and mode-specific K/L callbacks, including persistent
//! Sub-G oscillator, terrain-attitude, force, sound, and Entity selector state.
//! The callback is exact for its supplied pre-call basis. Completed owner
//! traversal now publishes the DCA0/E870 post-task basis, after same-pass Aim
//! reads the retained current matrix. The production world owner supplies
//! 12DA0 timing, dormant clearing/activation, E100 environment, E370 timer,
//! and master motion for the admitted normal Intro2 profile.
//! A continuing class-7 slot-0 then runs
//! slot-1's recovered C7D0/ADE0 live acquisition. A successful ADE0 handoff
//! retains the pursuing Chase/Aim graph; later visits commit Chase through
//! the same Type-13 `FUN_00401430` without revisiting the newborn Primary on
//! the ADE0 pass. ADE0 same-pass Aim and later Aim both run Type-13
//! `FUN_00424650` (method 10, sound 75) once per owner visit; a later
//! live-list drain materializes class 38. Chase tag `0x9C01`, Aim tag
//! `0x9C00`, and strict 5,000-ms expiry enter the same C690/AC60 root from
//! variant-1 `+0x00/+0x04`. Tertiary Aim C690 has no later slot, so it does
//! not visit newborn Secondary. An entered mover failure retains the consumed
//! Primary lifetime/retarget prefix and parks its exact task/context receipt;
//! later owner visits only observe that failure and cannot replay RNG, mover,
//! or later slots. A tagged or strict post-500-ms SharedRetarget result
//! now enters the exact class-7 `FUN_0040C690` -> `FUN_0040AC60` root: state
//! bit `0x1000` suppresses it, otherwise one TypeDefault selector word
//! publishes class 5 or class 7 while preserving the allocated behavior
//! context. The owner now retains either exact SharedRetarget graph: class 5
//! visits only its 5,000-ms Primary, while suppression and a new class-7 graph
//! freshly visit Secondary in the same pass. Primary is not revisited. Dying
//! Type-13 standard death enters the shared Class1 blast/radial terminal from
//! incoming and contact owners with their complete synchronous frame. Direct
//! `--level 50` and Level-1 type 13 births never enter.

mod callback_failure;
pub mod impact;
mod post_task_basis;
mod world;

pub use world::{
    tick_intro2_type13_world_owner_with_random, Intro2Type13WorldBlock, Intro2Type13WorldDrop,
    Intro2Type13WorldOutcome, Intro2Type13WorldOwner, Intro2Type13WorldTick,
};

use crate::actor_task_dispatcher::ActorTaskRuntime;
use crate::actor_task_owner::{ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags};
use crate::aim_and_fire::{
    AimAndFireFrameOutcome, AimAndFireInvalidTargetReason, AimAndFireLifetimeStatus,
    AimAndFireTaggedSingleton, AimAndFireTransitionReason,
};
use crate::chase_target::{
    evaluate_chase_target_callback, ChaseTargetCallbackPrefix, ChaseTargetCallbackRequest,
    ChaseTargetCallbackResult, ChaseTargetCommonMoverReturn, ChaseTargetLifetimeStatus,
    ChaseTargetProximityControl, ChaseTargetTaggedSingleton, ChaseTargetTargetRuntimeState,
};
use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
use crate::common_mover::target_prelude::CommonMoverTrackedTargetSnapshot;
use crate::entity::Entity;
use crate::entity_behavior::{behavior_program, BehaviorContextRuntime, BehaviorSelection};
use crate::entity_collision_state::{
    EntityTypeRuntimeMetadata, RetailRuntimeValue, DYING_STATE_BIT,
};
use crate::intro2_type13_aim::{tick_intro2_type13_aim, Type13AimError, Type13AimTickOutcome};
use crate::search_attack::SEARCH_ATTACK_BEHAVIOR_CLASS_ID;
use crate::search_attack_acquisition::TargetAcquisitionCallbackResult;
use crate::search_attack_live::{
    apply_search_attack_acquisition_live, SearchAttackLiveAcquisitionOutcome,
    SearchAttackLiveHandoffRequirement, SearchAttackLiveSamePassAim, TYPE13_SEARCH_ATTACK_SUB_G,
};
use crate::shared_retarget_mover::{
    shared_retarget_after_unwind, SharedRetarget, SharedRetargetCallbackPrefix,
    SharedRetargetPostUnwind, SharedRetargetTransitionRequest,
};
use crate::sub_g_runtime::{SubG06070RuntimeState, Type13SubGSound};
use crate::type13_c690::{
    plan_type13_variant0_c690, Type13C690Predecessor, Type13C690Reselection,
    Type13C690ReselectionBlock, Type13Variant0C690Request, TYPE13_C690_DYING_STATE_BIT,
    TYPE13_C690_SUPPRESS_STATE_BIT,
};
use crate::type13_common_mover::{
    evaluate_type13_common_mover, GklCommonMoverBlock, GklCommonMoverRequest, GklCommonMoverRuntime,
};
use crate::type13_initial_behavior::{
    apply_type13_published_class7_acquisition_live,
    authenticate_published_type13_class5_aimless_graph,
    authenticate_published_type13_class5_aimless_task_graph,
    authenticate_published_type13_class7_b6c0_graph,
    authenticate_published_type13_class7_b6c0_task_graph,
    authenticate_published_type13_class7_pursuing_graph, plan_type13_initial_behavior,
    publish_type13_initial_behavior, publish_type13_reselected_initial_behavior,
    Type13Class7AcquiringError, Type13Class7LiveAcquisitionError, Type13InitialBehaviorError,
    Type13InitialBehaviorPublication, Type13WeightedSelection,
};
use crate::wander_near_location::WanderNearCommonMoverReturn;
use crate::world_fx::WorldFx;
use crate::wrapped_axis_range::WrappedAxisRange;
use callback_failure::{park_type13_callback_failure, PendingType13CallbackFailure};
pub use post_task_basis::Intro2Type13PostTaskBasisBlock;
use post_task_basis::{task_traversal_completed, Type13PostTaskBasisFrame};
use v2k_formats::terrain::TerrainGrid;

pub use crate::type13_initial_behavior::TYPE13_ENTITY_TYPE;

pub const INTRO2_TYPE13_SPAWN_INDEX: usize = 0;
pub const INTRO2_TYPE13_MODEL_ID: usize = 291;
pub const INTRO2_TYPE13_POSITION_RAW: [i16; 3] = [0xB300u16 as i16, 0x0200, 0x2800];
const INTRO2_TYPE13_CAPTURED_CLASS7_SAMPLE_LOW16: u16 = 0x4000;
const INTRO2_TYPE13_CAPTURED_CHOICE_INDEX: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type13Admission {
    pub spawn_index: usize,
}

/// Native selection and a captured transcript are distinct RNG contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13BirthSelection {
    /// 1B8C0, one 425680 word, then one ACD0 or two B6C0 suffix words.
    Weighted,
    /// Accepted spawn-zero transcript: retain class7 without replaying its
    /// already-observed selector. Used only by explicit evidence fixtures.
    CapturedClass7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13PublicationError {
    EntityIdentityMismatch,
    UnexpectedModel,
    BehaviorContextUnavailable,
    SubGRuntimeUnavailable,
    Selection(Type13InitialBehaviorError),
    Constructor(Type13Class7AcquiringError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13PrimaryVisitBlock {
    EntityIdentityMismatch,
    GraphMismatch,
    PrimaryVisitUnavailable,
    SubGRuntimeUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13PrimaryVisitResult {
    Continue,
    TaggedZeroMover,
    CommonMoverBlocked(GklCommonMoverBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type13PrimaryVisit {
    pub prefix: SharedRetargetCallbackPrefix,
    pub result: Intro2Type13PrimaryVisitResult,
    pub post_unwind: SharedRetargetPostUnwind,
    pub sound: Option<Type13SubGSound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13ChaseBlock {
    GraphMismatch,
    VisitUnavailable,
    CommonAxisUnresolved,
    TargetStateUnresolved,
    SubGRuntimeUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13ChaseVisitResult {
    Continue,
    Tagged(ChaseTargetTaggedSingleton),
    CommonMoverBlocked(GklCommonMoverBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type13ChaseVisit {
    pub prefix: ChaseTargetCallbackPrefix,
    pub result: Intro2Type13ChaseVisitResult,
    pub sound: Option<Type13SubGSound>,
}

/// Resource values owned outside the slot-0 task wrapper but read by its
/// Type-13 common-mover call. Callers resolve these before mutable entity
/// custody so an unavailable model, terrain, or attachment list consumes no
/// task/component state.
#[derive(Debug, Clone, Copy)]
pub struct Intro2Type13PrimaryFrame<'a> {
    pub dispatch_mode: CommonMoverDispatchMode,
    pub terrain: &'a TerrainGrid,
    pub active_model_extent_raw: u16,
    pub attached_cargo_mass: u32,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

pub fn authenticate_intro2_type13(
    entity: &Entity,
) -> Result<Intro2Type13Admission, Intro2Type13PublicationError> {
    if !entity.active
        || entity.entity_type != TYPE13_ENTITY_TYPE
        || entity.authored_spawn_index != Some(INTRO2_TYPE13_SPAWN_INDEX)
    {
        return Err(Intro2Type13PublicationError::EntityIdentityMismatch);
    }
    if entity.model_slots != [Some(INTRO2_TYPE13_MODEL_ID); 4]
        || entity.model_index != Some(INTRO2_TYPE13_MODEL_ID)
    {
        return Err(Intro2Type13PublicationError::UnexpectedModel);
    }
    Ok(Intro2Type13Admission {
        spawn_index: INTRO2_TYPE13_SPAWN_INDEX,
    })
}

fn captured_intro2_type13_class7_plan(
) -> Result<Type13WeightedSelection, Intro2Type13PublicationError> {
    let program = behavior_program(u32::from(SEARCH_ATTACK_BEHAVIOR_CLASS_ID))
        .ok_or(Intro2Type13PublicationError::BehaviorContextUnavailable)?;
    Ok(Type13WeightedSelection {
        selection: BehaviorSelection {
            choice_index: INTRO2_TYPE13_CAPTURED_CHOICE_INDEX,
            program,
        },
        random_sample_low16: INTRO2_TYPE13_CAPTURED_CLASS7_SAMPLE_LOW16,
    })
}

/// FUN_00411400 publishes the detail bits consumed on the next 12DA0 pass.
/// This uses the live actor position even while Intro2 retains its presentation
/// proxy. Only the captured, published spawn-0 allocation is admitted.
pub fn publish_intro2_type13_view_detail(
    manager: &mut crate::entity::EntityManager,
    context: crate::entity_view_detail::RetailViewDetailContext,
) -> Result<
    RetailRuntimeValue<Option<crate::entity_view_detail::RetailViewDetail>>,
    Intro2Type13PublicationError,
> {
    let entity_id = manager
        .iter_all()
        .find(|entity| {
            authenticate_intro2_type13(entity).is_ok()
                && entity.intro2_type13_common_mover_runtime.is_some()
        })
        .map(|entity| entity.id)
        .ok_or(Intro2Type13PublicationError::EntityIdentityMismatch)?;
    let entity = manager
        .intro2_type13_entity_mut(entity_id)
        .ok_or(Intro2Type13PublicationError::EntityIdentityMismatch)?;
    Ok(context.publish(
        entity.position_raw(),
        &mut entity.collision.state_flags_at_0x08,
    ))
}

fn apply_intro2_type13_1b8c0(
    entity: &mut Entity,
    next_random: &mut impl FnMut() -> u32,
) -> Result<u16, Intro2Type13PublicationError> {
    match entity.sub_g_06070_runtime {
        RetailRuntimeValue::Known(Some(_)) => {
            let sample = next_random() as u16;
            entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(
                SubG06070RuntimeState::from_1b8c0_constructor(&TYPE13_SEARCH_ATTACK_SUB_G, sample),
            ));
            Ok(sample)
        }
        _ => Err(Intro2Type13PublicationError::SubGRuntimeUnavailable),
    }
}

pub fn publish_intro2_type13(
    entity: &mut Entity,
    metadata: &EntityTypeRuntimeMetadata,
    birth_selection: Intro2Type13BirthSelection,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type13Admission, Intro2Type13PublicationError> {
    let admission = authenticate_intro2_type13(entity)?;
    if entity.position_raw() != INTRO2_TYPE13_POSITION_RAW {
        return Err(Intro2Type13PublicationError::EntityIdentityMismatch);
    }
    apply_intro2_type13_1b8c0(entity, next_random)?;
    let planned = match birth_selection {
        Intro2Type13BirthSelection::Weighted => {
            plan_type13_initial_behavior(metadata, &mut *next_random)
                .map_err(Intro2Type13PublicationError::Selection)?
        }
        Intro2Type13BirthSelection::CapturedClass7 => captured_intro2_type13_class7_plan()?,
    };
    publish_type13_initial_behavior(entity, metadata, planned, &mut *next_random)
        .map_err(Intro2Type13PublicationError::Constructor)?;
    entity.initial_behavior = RetailRuntimeValue::Known(Some(planned.selection));
    entity.intro2_type13_common_mover_runtime =
        Some(GklCommonMoverRuntime::from_intro2_constructor_capture());
    // Native birth policy for retail's unwritten allocator word +B2. Choose
    // deterministic zero once, as for the other native Intro2 actors; neither
    // task reselection nor world admission may replace a later contribution.
    entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
    Ok(admission)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intro2Type13Graph {
    Class5Aimless,
    Class7Acquiring,
    Class7Pursuing,
}

impl Intro2Type13Graph {
    fn c690_predecessor(self) -> Type13C690Predecessor {
        match self {
            Self::Class5Aimless => Type13C690Predecessor::Class5Aimless,
            Self::Class7Acquiring => Type13C690Predecessor::Class7Acquiring,
            Self::Class7Pursuing => Type13C690Predecessor::Class7Pursuing,
        }
    }
}

pub fn tick_intro2_type13_primary(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    frame: Intro2Type13PrimaryFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type13PrimaryVisit, Intro2Type13PrimaryVisitBlock> {
    tick_intro2_type13_shared_retarget_primary(
        entity,
        Intro2Type13Graph::Class7Acquiring,
        metadata,
        frame,
        next_random,
    )
}

pub fn tick_intro2_type13_class5_primary(
    entity: &mut Entity,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    frame: Intro2Type13PrimaryFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type13PrimaryVisit, Intro2Type13PrimaryVisitBlock> {
    tick_intro2_type13_shared_retarget_primary(
        entity,
        Intro2Type13Graph::Class5Aimless,
        metadata,
        frame,
        next_random,
    )
}

fn tick_intro2_type13_shared_retarget_primary(
    entity: &mut Entity,
    graph: Intro2Type13Graph,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    frame: Intro2Type13PrimaryFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type13PrimaryVisit, Intro2Type13PrimaryVisitBlock> {
    authenticate_intro2_type13(entity)
        .map_err(|_| Intro2Type13PrimaryVisitBlock::EntityIdentityMismatch)?;
    match graph {
        Intro2Type13Graph::Class5Aimless => {
            authenticate_published_type13_class5_aimless_graph(entity)
                .map_err(|_| Intro2Type13PrimaryVisitBlock::GraphMismatch)?;
        }
        Intro2Type13Graph::Class7Acquiring => {
            authenticate_published_type13_class7_b6c0_graph(entity)
                .map_err(|_| Intro2Type13PrimaryVisitBlock::GraphMismatch)?;
        }
        Intro2Type13Graph::Class7Pursuing => {
            return Err(Intro2Type13PrimaryVisitBlock::GraphMismatch);
        }
    }
    let RetailRuntimeValue::Known(Some(sub_g_runtime)) = entity.sub_g_06070_runtime else {
        return Err(Intro2Type13PrimaryVisitBlock::SubGRuntimeUnavailable);
    };
    let Some(common_mover_runtime) = entity.intro2_type13_common_mover_runtime else {
        return Err(Intro2Type13PrimaryVisitBlock::SubGRuntimeUnavailable);
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(Intro2Type13PrimaryVisitBlock::PrimaryVisitUnavailable);
    };
    let Some(ActorTaskRuntime::SharedRetarget(mut staged_task)) =
        entity.actor_task_state(ActorTaskSlot::Primary).copied()
    else {
        return Err(Intro2Type13PrimaryVisitBlock::PrimaryVisitUnavailable);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    if !entity
        .actor_tasks
        .wrapper_flags(task_id)
        .is_some_and(|flags| flags.alive && !flags.in_callback)
    {
        return Err(Intro2Type13PrimaryVisitBlock::PrimaryVisitUnavailable);
    }
    let lifetime = staged_task.before_callback(frame.elapsed_micros);
    let owner_position_raw = entity.position_raw();
    let mut stage = staged_task.stage_callback(owner_position_raw, || next_random() as u16);
    let retarget = stage.retarget();
    // A800 ages the task before 02BA0, whose retarget writes precede 01430.
    // A later evidence block must retain that consumed RNG-owned prefix.
    let Some(()) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        *runtime = ActorTaskRuntime::SharedRetarget(staged_task);
    }) else {
        return Err(Intro2Type13PrimaryVisitBlock::PrimaryVisitUnavailable);
    };
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
    let mover = evaluate_type13_common_mover(
        GklCommonMoverRequest {
            dispatch_mode: frame.dispatch_mode,
            entity_id: entity.id,
            entity_type: entity.entity_type,
            metadata,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            heading_raw: heading_raw as u16,
            pitch_raw,
            roll_raw: roll_raw as u16,
            body_basis: entity.physical_body_basis_q31(),
            runtime: common_mover_runtime,
            sub_g_runtime,
            target_private: stage.private_state(),
            tracked_target: RetailRuntimeValue::Known(None),
            terrain: frame.terrain,
            active_model_extent_raw: frame.active_model_extent_raw,
            self_mass_raw: entity.mass_raw,
            attached_cargo_mass: frame.attached_cargo_mass,
            capability_flags: entity.capability_flags,
            retail_tick: frame.retail_tick,
            elapsed_micros: frame.elapsed_micros,
            global_elapsed_micros: frame.global_elapsed_micros,
        },
        || next_random(),
    );
    let (result, sound, mover_return) = match mover {
        Ok(outcome) => {
            *stage.private_state_mut() = outcome.target_private;
            staged_task.commit_callback_stage(stage);
            *entity
                .actor_tasks
                .task_state_mut(visit.task_id)
                .expect("detached mover retains the entered Primary wrapper") =
                ActorTaskRuntime::SharedRetarget(staged_task);
            entity.intro2_type13_common_mover_runtime = Some(outcome.runtime);
            entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(outcome.sub_g_runtime));
            entity.set_rotation_heading_pitch_roll_raw([
                outcome.heading_raw as i16,
                outcome.pitch_raw,
                outcome.roll_raw as i16,
            ]);
            entity.set_velocity_raw(outcome.velocity_raw);
            let (result, mover_return) = match outcome.result {
                ChaseTargetCommonMoverReturn::NonZero => (
                    Intro2Type13PrimaryVisitResult::Continue,
                    WanderNearCommonMoverReturn::NonZero,
                ),
                ChaseTargetCommonMoverReturn::Zero => (
                    Intro2Type13PrimaryVisitResult::TaggedZeroMover,
                    WanderNearCommonMoverReturn::Zero,
                ),
            };
            (result, outcome.sound, Some(mover_return))
        }
        Err(block) => (
            Intro2Type13PrimaryVisitResult::CommonMoverBlocked(block),
            None,
            None,
        ),
    };
    if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
    }
    let prefix = SharedRetargetCallbackPrefix::from_parts(lifetime, retarget);
    let post_unwind = match mover_return {
        Some(mover_return) => shared_retarget_after_unwind(visit, prefix, mover_return),
        None => SharedRetargetPostUnwind::UnresolvedCommonMover,
    };
    Ok(Intro2Type13PrimaryVisit {
        prefix,
        result,
        post_unwind,
        sound,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type13SchedulerOwner {
    entity_id: u32,
    stage: Intro2Type13SchedulerStage,
    post_task_basis: Option<Type13PostTaskBasisFrame>,
    dispatch_mode: CommonMoverDispatchMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intro2Type13SchedulerStage {
    Running(Intro2Type13Graph),
    CallbackFailurePending(PendingType13CallbackFailure),
    PendingC690Plan {
        predecessor: Intro2Type13Graph,
        request: SharedRetargetTransitionRequest,
        continuation_elapsed_micros: u32,
    },
    PendingC690Publication {
        predecessor: Intro2Type13Graph,
        request: SharedRetargetTransitionRequest,
        planned: Type13WeightedSelection,
        predecessor_context: BehaviorContextRuntime,
        continuation_elapsed_micros: u32,
    },
    PendingPursuingC690Plan {
        receipt: PendingPursuingC690Receipt,
        reason: Intro2Type13PursuingC690Reason,
        continuation_elapsed_micros: u32,
        post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
    },
    PendingPursuingC690Publication {
        receipt: PendingPursuingC690Receipt,
        reason: Intro2Type13PursuingC690Reason,
        planned: Type13WeightedSelection,
        predecessor_context: BehaviorContextRuntime,
        continuation_elapsed_micros: u32,
        post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13SchedulerAdoptionError {
    EntityUnavailable,
    GraphUnavailable { entity_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13SchedulerProductionDrop {
    EntityUnavailable,
    GraphMismatch,
    FrameUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13C690Transition {
    SuppressedByEntityState,
    Published(Type13InitialBehaviorPublication),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13C690Slot {
    Plus00,
    Plus04,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13PursuingC690Reason {
    ChaseTagged(ChaseTargetTaggedSingleton),
    ChaseLifetimeExpired,
    AimTaggedInvalidTarget {
        singleton: AimAndFireTaggedSingleton,
        reason: AimAndFireInvalidTargetReason,
        lifetime_due: bool,
    },
    AimLifetimeExpired,
}

impl Intro2Type13PursuingC690Reason {
    pub const fn slot(self) -> Intro2Type13C690Slot {
        match self {
            Self::ChaseTagged(_) | Self::ChaseLifetimeExpired | Self::AimLifetimeExpired => {
                Intro2Type13C690Slot::Plus00
            }
            Self::AimTaggedInvalidTarget { .. } => Intro2Type13C690Slot::Plus04,
        }
    }

    const fn falls_through_to_plus00_when_suppressed(self) -> bool {
        matches!(
            self,
            Self::AimTaggedInvalidTarget {
                lifetime_due: true,
                ..
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type13PursuingC690Request {
    pub source: ActorTaskVisit,
    pub reason: Intro2Type13PursuingC690Reason,
}

impl Intro2Type13PursuingC690Request {
    pub const fn slot(self) -> Intro2Type13C690Slot {
        self.reason.slot()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intro2Type13PursuingC690Attempt {
    pub request: Intro2Type13PursuingC690Request,
    pub transition: Intro2Type13C690Transition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2Type13C690Block {
    MetadataUnavailable,
    Plan(Type13C690ReselectionBlock),
    Publication(Type13Class7AcquiringError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intro2Type13SchedulerProductionOutcome {
    B6c0Visit {
        entity_id: u32,
        primary: Intro2Type13PrimaryVisit,
        acquisition: SearchAttackLiveAcquisitionOutcome,
        aim: Option<Result<Type13AimTickOutcome, Type13AimError>>,
    },
    Class5AimlessVisit {
        entity_id: u32,
        primary: Intro2Type13PrimaryVisit,
    },
    PursuingVisit {
        entity_id: u32,
        chase: Option<Intro2Type13ChaseVisit>,
        post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
        acquisition: Option<SearchAttackLiveAcquisitionOutcome>,
        aim: Option<Result<Type13AimTickOutcome, Type13AimError>>,
        post_aim_plus04_c690: Option<Intro2Type13PursuingC690Attempt>,
        post_aim_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
    },
    PursuingChaseBlocked {
        entity_id: u32,
        reason: Intro2Type13ChaseBlock,
    },
    PursuingC690Blocked {
        entity_id: u32,
        chase: Option<Intro2Type13ChaseVisit>,
        /// Present only when ADE0 or Tertiary Aim produced this request in the
        /// current scheduler call. A resumed continuation does not replay it.
        aim: Option<Result<Type13AimTickOutcome, Type13AimError>>,
        /// Present only when the current call published and visited a new
        /// class-7 acquisition before Tertiary Aim requested this transition.
        acquisition: Option<SearchAttackLiveAcquisitionOutcome>,
        request: Intro2Type13PursuingC690Request,
        post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
        reason: Intro2Type13C690Block,
    },
    PrimaryBlocked {
        entity_id: u32,
        primary: Intro2Type13PrimaryVisit,
    },
    /// Observation of an already-entered failed callback. No task, mover,
    /// random, or later-slot phase executes again, even if evidence is repaired.
    CallbackFailurePending {
        entity_id: u32,
        source: ActorTaskVisit,
        reason: GklCommonMoverBlock,
    },
    /// The outer basis policy could not be admitted before any task or RNG.
    PostTaskBasisBlocked {
        entity_id: u32,
        reason: Intro2Type13PostTaskBasisBlock,
    },
    C690Transition {
        entity_id: u32,
        request: SharedRetargetTransitionRequest,
        transition: Intro2Type13C690Transition,
        /// Present only when the Primary callback produced this request in the
        /// current scheduler call. A resumed linear continuation does not
        /// replay the callback.
        primary: Option<Intro2Type13PrimaryVisit>,
        /// Suppression and a new class-7 root freshly read Secondary at the
        /// original outer-dispatch cursor. Class 5 has no Secondary.
        acquisition: Option<SearchAttackLiveAcquisitionOutcome>,
        /// ADE0 same-pass Type-13 `FUN_00424650` when Secondary published Aim.
        aim: Option<Result<Type13AimTickOutcome, Type13AimError>>,
    },
    C690Blocked {
        entity_id: u32,
        request: SharedRetargetTransitionRequest,
        reason: Intro2Type13C690Block,
        /// Present only on the frame which first produced the transition.
        primary: Option<Intro2Type13PrimaryVisit>,
    },
    Dropped {
        entity_id: u32,
        reason: Intro2Type13SchedulerProductionDrop,
    },
}

impl Intro2Type13SchedulerProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::B6c0Visit { entity_id, .. }
            | Self::Class5AimlessVisit { entity_id, .. }
            | Self::PursuingVisit { entity_id, .. }
            | Self::PursuingChaseBlocked { entity_id, .. }
            | Self::PursuingC690Blocked { entity_id, .. }
            | Self::PrimaryBlocked { entity_id, .. }
            | Self::CallbackFailurePending { entity_id, .. }
            | Self::PostTaskBasisBlocked { entity_id, .. }
            | Self::C690Transition { entity_id, .. }
            | Self::C690Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

pub struct Intro2Type13SchedulerOwnerTick {
    pub outcome: Intro2Type13SchedulerProductionOutcome,
    pub retained_owner: Option<Intro2Type13SchedulerOwner>,
}

impl Intro2Type13SchedulerOwner {
    pub const fn entity_id(self) -> u32 {
        self.entity_id
    }

    pub const fn has_pending_prefix(self) -> bool {
        !matches!(self.stage, Intro2Type13SchedulerStage::Running(_))
    }

    pub(crate) fn fork_for_main_base_abort_transaction(self) -> Self {
        self
    }

    pub fn adopt(
        manager: &crate::entity::EntityManager,
    ) -> Result<Self, Intro2Type13SchedulerAdoptionError> {
        let Some(entity) = manager.iter_all().find(|entity| {
            entity.active
                && entity.entity_type == TYPE13_ENTITY_TYPE
                && entity.authored_spawn_index == Some(INTRO2_TYPE13_SPAWN_INDEX)
        }) else {
            return Err(Intro2Type13SchedulerAdoptionError::EntityUnavailable);
        };
        Self::adopt_published(entity)
    }

    pub fn adopt_published(entity: &Entity) -> Result<Self, Intro2Type13SchedulerAdoptionError> {
        authenticate_intro2_type13(entity).map_err(|_| {
            Intro2Type13SchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            }
        })?;
        let graph = if authenticate_published_type13_class7_b6c0_graph(entity).is_ok() {
            Intro2Type13Graph::Class7Acquiring
        } else if authenticate_published_type13_class7_pursuing_graph(entity).is_ok() {
            Intro2Type13Graph::Class7Pursuing
        } else if authenticate_published_type13_class5_aimless_graph(entity).is_ok() {
            Intro2Type13Graph::Class5Aimless
        } else {
            return Err(Intro2Type13SchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            });
        };
        if graph == Intro2Type13Graph::Class7Pursuing
            && [ActorTaskSlot::Primary, ActorTaskSlot::Tertiary]
                .into_iter()
                .any(|slot| {
                    entity
                        .actor_tasks
                        .task_in_slot(slot)
                        .and_then(|task_id| entity.actor_tasks.wrapper_flags(task_id))
                        != Some(ActorTaskWrapperFlags {
                            alive: true,
                            in_callback: false,
                        })
                })
        {
            return Err(Intro2Type13SchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            });
        }
        if entity.intro2_type13_common_mover_runtime.is_none()
            || !matches!(
                entity.sub_g_06070_runtime,
                RetailRuntimeValue::Known(Some(_))
            )
        {
            return Err(Intro2Type13SchedulerAdoptionError::GraphUnavailable {
                entity_id: entity.id,
            });
        }
        let dispatch_mode = CommonMoverDispatchMode::Normal;
        Ok(Self {
            entity_id: entity.id,
            stage: Intro2Type13SchedulerStage::Running(graph),
            post_task_basis: None,
            dispatch_mode,
        })
    }
}

pub fn tick_intro2_type13_scheduler_owner(
    manager: &mut crate::entity::EntityManager,
    owner: Intro2Type13SchedulerOwner,
    world_fx: &mut WorldFx,
    frame: Option<Intro2Type13PrimaryFrame<'_>>,
) -> Intro2Type13SchedulerOwnerTick {
    let mut next_shared_random =
        |world_fx: &mut WorldFx| u32::from(world_fx.next_shared_retail_random_u16());
    tick_intro2_type13_scheduler_owner_with_random(
        manager,
        owner,
        world_fx,
        frame,
        &mut next_shared_random,
    )
}

/// Tick the Intro2 Type-13 owner against an explicit process-wide RNG feed.
///
/// The production scheduler uses [`WorldFx::next_shared_retail_random_u16`].
/// This seam keeps selector and constructor-suffix ordering observable without
/// inventing a second random owner.
pub fn tick_intro2_type13_scheduler_owner_with_random(
    manager: &mut crate::entity::EntityManager,
    owner: Intro2Type13SchedulerOwner,
    world_fx: &mut WorldFx,
    frame: Option<Intro2Type13PrimaryFrame<'_>>,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type13SchedulerOwnerTick {
    let mut owner = owner;
    if matches!(owner.stage, Intro2Type13SchedulerStage::Running(_)) {
        if let Some(frame) = frame {
            owner.dispatch_mode = frame.dispatch_mode;
        }
    }
    let dispatch_mode = owner.dispatch_mode;
    let basis_frame = match owner.post_task_basis {
        Some(latched) => Some(latched),
        None if matches!(owner.stage, Intro2Type13SchedulerStage::Running(_))
            && frame.is_some() =>
        {
            let Some(entity) = manager
                .iter_all()
                .find(|entity| entity.id == owner.entity_id())
            else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id: owner.entity_id(),
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                };
            };
            match Type13PostTaskBasisFrame::capture(entity) {
                Ok(latched) => Some(latched),
                Err(reason) => {
                    return Intro2Type13SchedulerOwnerTick {
                        outcome: Intro2Type13SchedulerProductionOutcome::PostTaskBasisBlocked {
                            entity_id: owner.entity_id(),
                            reason,
                        },
                        retained_owner: Some(owner),
                    };
                }
            }
        }
        None => None,
    };
    let elapsed_micros = frame.as_ref().map_or(0, |frame| frame.elapsed_micros);
    let mut tick = tick_intro2_type13_scheduler_owner_with_elapsed_and_random(
        manager,
        owner,
        world_fx,
        frame,
        elapsed_micros,
        next_shared_random,
    );
    if let Some(latched) = basis_frame {
        if task_traversal_completed(&tick) {
            let entity = manager
                .intro2_type13_entity_mut(owner.entity_id())
                .expect("a completed Type-13 traversal retains its live owner");
            latched.publish(entity);
        }
        if let Some(retained) = tick.retained_owner.as_mut() {
            // A retained C690 or entered callback failure is the same outer
            // pass even when its later observation has no new frame. Running
            // owners start a fresh pass on their next visit.
            retained.post_task_basis =
                (!matches!(retained.stage, Intro2Type13SchedulerStage::Running(_)))
                    .then_some(latched);
        }
    }
    if let Some(retained) = tick.retained_owner.as_mut() {
        retained.dispatch_mode = dispatch_mode;
    }
    tick
}

fn tick_intro2_type13_scheduler_owner_with_elapsed_and_random(
    manager: &mut crate::entity::EntityManager,
    owner: Intro2Type13SchedulerOwner,
    world_fx: &mut WorldFx,
    frame: Option<Intro2Type13PrimaryFrame<'_>>,
    elapsed_micros: u32,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type13SchedulerOwnerTick {
    let dispatch_mode = owner.dispatch_mode;
    let (graph, pending, pending_pursuing) = match owner.stage {
        Intro2Type13SchedulerStage::Running(graph) => (Some(graph), None, None),
        Intro2Type13SchedulerStage::CallbackFailurePending(pending) => {
            return pending.observe(manager, owner);
        }
        Intro2Type13SchedulerStage::PendingC690Plan {
            predecessor,
            request,
            continuation_elapsed_micros,
        } => (
            None,
            Some(PendingType13C690::Plan {
                predecessor,
                request,
                continuation_elapsed_micros,
            }),
            None,
        ),
        Intro2Type13SchedulerStage::PendingC690Publication {
            predecessor,
            request,
            planned,
            predecessor_context,
            continuation_elapsed_micros,
        } => (
            None,
            Some(PendingType13C690::Publication {
                predecessor,
                request,
                planned,
                predecessor_context,
                continuation_elapsed_micros,
            }),
            None,
        ),
        Intro2Type13SchedulerStage::PendingPursuingC690Plan {
            receipt,
            reason,
            continuation_elapsed_micros,
            post_chase_plus00_c690,
        } => (
            None,
            None,
            Some(PendingPursuingC690::Plan {
                receipt,
                reason,
                continuation_elapsed_micros,
                post_chase_plus00_c690,
            }),
        ),
        Intro2Type13SchedulerStage::PendingPursuingC690Publication {
            receipt,
            reason,
            planned,
            predecessor_context,
            continuation_elapsed_micros,
            post_chase_plus00_c690,
        } => (
            None,
            None,
            Some(PendingPursuingC690::Publication {
                receipt,
                reason,
                planned,
                predecessor_context,
                continuation_elapsed_micros,
                post_chase_plus00_c690,
            }),
        ),
    };
    if let Some(pending) = pending_pursuing {
        return apply_intro2_type13_pursuing_c690(
            dispatch_mode,
            manager,
            owner.entity_id(),
            None,
            None,
            pending,
            None,
            None,
            world_fx,
            next_shared_random,
        );
    }
    if let Some(pending) = pending {
        let pending_graph_survives = manager
            .iter_all()
            .find(|entity| entity.id == owner.entity_id())
            .is_some_and(|entity| pending_type13_transition_graph_survives(entity, pending));
        if !pending_graph_survives {
            return Intro2Type13SchedulerOwnerTick {
                outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            };
        }
        return finish_intro2_type13_c690(
            dispatch_mode,
            manager,
            owner.entity_id(),
            pending,
            None,
            world_fx,
            next_shared_random,
        );
    }
    let graph = graph.expect("a non-pending Type-13 owner retains its authenticated graph");
    let Some(frame) = frame else {
        return Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Intro2Type13SchedulerProductionDrop::FrameUnavailable,
            },
            retained_owner: None,
        };
    };
    if graph == Intro2Type13Graph::Class7Pursuing {
        return tick_intro2_type13_pursuing_owner(
            manager,
            owner,
            world_fx,
            frame,
            next_shared_random,
        );
    }
    let metadata = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned();
    let Some(entity) = manager.intro2_type13_entity_mut(owner.entity_id()) else {
        return Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                entity_id: owner.entity_id(),
                reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    let primary = match tick_intro2_type13_shared_retarget_primary(
        entity,
        graph,
        metadata.as_ref(),
        frame,
        &mut || next_shared_random(world_fx),
    ) {
        Ok(visit) => visit,
        Err(block) => {
            let reason = match block {
                Intro2Type13PrimaryVisitBlock::SubGRuntimeUnavailable => {
                    Intro2Type13SchedulerProductionDrop::FrameUnavailable
                }
                _ => Intro2Type13SchedulerProductionDrop::GraphMismatch,
            };
            return Intro2Type13SchedulerOwnerTick {
                outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason,
                },
                retained_owner: None,
            };
        }
    };
    if let Some(sound) = primary.sound {
        world_fx.queue_fixed_positional_sound_raw_at_rate(
            sound.sound_id,
            sound.position_raw,
            sound.rate_q16,
        );
    }
    match primary.post_unwind {
        SharedRetargetPostUnwind::Continue => {}
        SharedRetargetPostUnwind::UnresolvedCommonMover => {
            let Intro2Type13PrimaryVisitResult::CommonMoverBlocked(reason) = primary.result else {
                unreachable!("unresolved Primary return retains its mover block");
            };
            return park_type13_callback_failure(
                manager,
                owner,
                reason,
                Intro2Type13SchedulerProductionOutcome::PrimaryBlocked {
                    entity_id: owner.entity_id(),
                    primary,
                },
            );
        }
        SharedRetargetPostUnwind::Transition(request) => {
            return finish_intro2_type13_c690(
                dispatch_mode,
                manager,
                owner.entity_id(),
                PendingType13C690::Plan {
                    predecessor: graph,
                    request,
                    continuation_elapsed_micros: elapsed_micros,
                },
                Some(primary),
                world_fx,
                next_shared_random,
            );
        }
    }
    if graph == Intro2Type13Graph::Class5Aimless {
        return Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::Class5AimlessVisit {
                entity_id: owner.entity_id(),
                primary,
            },
            retained_owner: Some(owner),
        };
    }
    let acquisition = match apply_type13_published_class7_acquisition_live(
        manager,
        owner.entity_id(),
        frame.elapsed_micros,
        world_fx,
    ) {
        Ok(outcome) => outcome,
        Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph) => {
            return Intro2Type13SchedulerOwnerTick {
                outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                },
                retained_owner: None,
            };
        }
        Err(_) => {
            return Intro2Type13SchedulerOwnerTick {
                outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                    entity_id: owner.entity_id(),
                    reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                },
                retained_owner: None,
            };
        }
    };
    let aim = tick_intro2_type13_ade0_same_pass_aim(
        dispatch_mode,
        manager,
        world_fx,
        owner.entity_id(),
        frame.elapsed_micros,
        Some(&acquisition),
    );
    if let Some(reason) = type13_aim_c690_reason(aim.as_ref()) {
        return apply_intro2_type13_pursuing_c690(
            dispatch_mode,
            manager,
            owner.entity_id(),
            None,
            aim,
            PendingPursuingC690::Fresh {
                source_slot: ActorTaskSlot::Tertiary,
                reason,
                continuation_elapsed_micros: frame.elapsed_micros,
            },
            None,
            Some(acquisition),
            world_fx,
            next_shared_random,
        );
    }
    let retained_graph = manager
        .iter_all()
        .find(|entity| entity.id == owner.entity_id())
        .and_then(|entity| {
            if authenticate_published_type13_class7_b6c0_graph(entity).is_ok() {
                Some(Intro2Type13Graph::Class7Acquiring)
            } else if authenticate_published_type13_class7_pursuing_graph(entity).is_ok() {
                Some(Intro2Type13Graph::Class7Pursuing)
            } else {
                None
            }
        });
    Intro2Type13SchedulerOwnerTick {
        outcome: Intro2Type13SchedulerProductionOutcome::B6c0Visit {
            entity_id: owner.entity_id(),
            primary,
            acquisition,
            aim,
        },
        retained_owner: retained_graph.map(|graph| Intro2Type13SchedulerOwner {
            entity_id: owner.entity_id(),
            stage: Intro2Type13SchedulerStage::Running(graph),
            post_task_basis: None,
            dispatch_mode,
        }),
    }
}

fn tick_intro2_type13_pursuing_owner(
    manager: &mut crate::entity::EntityManager,
    owner: Intro2Type13SchedulerOwner,
    world_fx: &mut WorldFx,
    frame: Intro2Type13PrimaryFrame<'_>,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type13SchedulerOwnerTick {
    let dispatch_mode = owner.dispatch_mode;
    let entity_id = owner.entity_id();
    let metadata = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned();
    let chase = match tick_intro2_type13_pursuing_chase(
        manager,
        entity_id,
        metadata.as_ref(),
        frame,
        &mut || next_shared_random(world_fx),
    ) {
        Ok(visit) => visit,
        Err(reason) => {
            let drop = matches!(
                reason,
                Intro2Type13ChaseBlock::GraphMismatch | Intro2Type13ChaseBlock::VisitUnavailable
            );
            return Intro2Type13SchedulerOwnerTick {
                outcome: if drop {
                    Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                    }
                } else {
                    Intro2Type13SchedulerProductionOutcome::PursuingChaseBlocked {
                        entity_id,
                        reason,
                    }
                },
                retained_owner: (!drop).then_some(owner),
            };
        }
    };
    if let Some(sound) = chase.sound {
        world_fx.queue_fixed_positional_sound_raw_at_rate(
            sound.sound_id,
            sound.position_raw,
            sound.rate_q16,
        );
    }
    if let Intro2Type13ChaseVisitResult::CommonMoverBlocked(reason) = chase.result {
        return park_type13_callback_failure(
            manager,
            owner,
            reason,
            Intro2Type13SchedulerProductionOutcome::PursuingVisit {
                entity_id,
                chase: Some(chase),
                post_chase_plus00_c690: None,
                acquisition: None,
                aim: None,
                post_aim_plus04_c690: None,
                post_aim_plus00_c690: None,
            },
        );
    }
    if let Some(reason) = type13_chase_c690_reason(&chase) {
        return apply_intro2_type13_pursuing_c690(
            dispatch_mode,
            manager,
            entity_id,
            Some(chase),
            None,
            PendingPursuingC690::Fresh {
                source_slot: ActorTaskSlot::Primary,
                reason,
                continuation_elapsed_micros: frame.elapsed_micros,
            },
            None,
            None,
            world_fx,
            next_shared_random,
        );
    }
    let metadata = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned();
    let aim = tick_intro2_type13_aim(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        frame.elapsed_micros,
        metadata.as_ref(),
    );
    if let Some(reason) = type13_aim_c690_reason(Some(&aim)) {
        return apply_intro2_type13_pursuing_c690(
            dispatch_mode,
            manager,
            entity_id,
            Some(chase),
            Some(aim),
            PendingPursuingC690::Fresh {
                source_slot: ActorTaskSlot::Tertiary,
                reason,
                continuation_elapsed_micros: frame.elapsed_micros,
            },
            None,
            None,
            world_fx,
            next_shared_random,
        );
    }
    let retain = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(|entity| authenticate_published_type13_class7_pursuing_graph(entity).is_ok());
    Intro2Type13SchedulerOwnerTick {
        outcome: Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            entity_id,
            chase: Some(chase),
            post_chase_plus00_c690: None,
            acquisition: None,
            aim: Some(aim),
            post_aim_plus04_c690: None,
            post_aim_plus00_c690: None,
        },
        retained_owner: retain.then_some(owner),
    }
}

fn type13_chase_c690_reason(
    chase: &Intro2Type13ChaseVisit,
) -> Option<Intro2Type13PursuingC690Reason> {
    match chase.result {
        Intro2Type13ChaseVisitResult::Tagged(singleton) => {
            Some(Intro2Type13PursuingC690Reason::ChaseTagged(singleton))
        }
        Intro2Type13ChaseVisitResult::Continue
            if chase.prefix.lifetime_status == ChaseTargetLifetimeStatus::OwnerTransitionDue =>
        {
            Some(Intro2Type13PursuingC690Reason::ChaseLifetimeExpired)
        }
        Intro2Type13ChaseVisitResult::Continue
        | Intro2Type13ChaseVisitResult::CommonMoverBlocked(_) => None,
    }
}

fn type13_aim_c690_reason(
    aim: Option<&Result<Type13AimTickOutcome, Type13AimError>>,
) -> Option<Intro2Type13PursuingC690Reason> {
    let outcome = aim?.as_ref().ok()?;
    match outcome.resolution.outcome {
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::TaggedInvalidTarget { singleton, reason },
        } => Some(Intro2Type13PursuingC690Reason::AimTaggedInvalidTarget {
            singleton,
            reason,
            lifetime_due: outcome.resolution.prefix.lifetime_status
                == AimAndFireLifetimeStatus::OwnerTransitionDue,
        }),
        AimAndFireFrameOutcome::RequestOwnerTransition {
            reason: AimAndFireTransitionReason::LifetimeExpired,
        } => Some(Intro2Type13PursuingC690Reason::AimLifetimeExpired),
        AimAndFireFrameOutcome::Continue
        | AimAndFireFrameOutcome::ReturnGenericEmitterResult(_) => None,
    }
}

fn type13_ade0_published_without_aim_visit(
    acquisition: &SearchAttackLiveAcquisitionOutcome,
) -> bool {
    matches!(
        acquisition,
        SearchAttackLiveAcquisitionOutcome::Applied {
            result: TargetAcquisitionCallbackResult::TaggedTargetAccepted { .. },
            same_pass_aim: None,
            ..
        }
    )
}

fn tick_intro2_type13_ade0_same_pass_aim(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut crate::entity::EntityManager,
    world_fx: &mut WorldFx,
    entity_id: u32,
    elapsed_micros: u32,
    acquisition: Option<&SearchAttackLiveAcquisitionOutcome>,
) -> Option<Result<Type13AimTickOutcome, Type13AimError>> {
    let Some(acquisition) = acquisition else {
        return None;
    };
    if !type13_ade0_published_without_aim_visit(acquisition) {
        return None;
    }
    let metadata = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned();
    Some(tick_intro2_type13_aim(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        elapsed_micros,
        metadata.as_ref(),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommittedType13TaskReceipt {
    visit: ActorTaskVisit,
    committed_task: ActorTaskRuntime,
}

impl CommittedType13TaskReceipt {
    fn capture(entity: &Entity, slot: ActorTaskSlot) -> Option<Self> {
        let task_id = entity.actor_tasks.task_in_slot(slot)?;
        let committed_task = *entity.actor_tasks.task_state(task_id)?;
        if entity.actor_tasks.wrapper_flags(task_id)
            != Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        {
            return None;
        }
        Some(Self {
            visit: ActorTaskVisit { slot, task_id },
            committed_task,
        })
    }

    fn survives(self, entity: &Entity) -> bool {
        entity.actor_tasks.task_in_slot(self.visit.slot) == Some(self.visit.task_id)
            && entity.actor_tasks.task_state(self.visit.task_id) == Some(&self.committed_task)
            && entity.actor_tasks.wrapper_flags(self.visit.task_id)
                == Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingPursuingC690Receipt {
    source: ActorTaskVisit,
    primary: CommittedType13TaskReceipt,
    tertiary: Option<CommittedType13TaskReceipt>,
    predecessor_context: BehaviorContextRuntime,
}

impl PendingPursuingC690Receipt {
    fn capture(entity: &Entity, source_slot: ActorTaskSlot) -> Option<Self> {
        authenticate_intro2_type13(entity).ok()?;
        let RetailRuntimeValue::Known(Some(predecessor_context)) = entity.current_behavior_context
        else {
            return None;
        };
        let primary = CommittedType13TaskReceipt::capture(entity, ActorTaskSlot::Primary)?;
        if !matches!(primary.committed_task, ActorTaskRuntime::ChaseTarget(_)) {
            return None;
        }
        let tertiary = match source_slot {
            ActorTaskSlot::Primary => None,
            ActorTaskSlot::Tertiary => {
                let receipt = CommittedType13TaskReceipt::capture(entity, source_slot)?;
                if !matches!(receipt.committed_task, ActorTaskRuntime::AimAndFire(_)) {
                    return None;
                }
                Some(receipt)
            }
            ActorTaskSlot::Secondary => return None,
        };
        let source = tertiary.map_or(primary.visit, |receipt| receipt.visit);
        Some(Self {
            source,
            primary,
            tertiary,
            predecessor_context,
        })
    }

    fn survives(self, entity: &Entity) -> bool {
        authenticate_intro2_type13(entity).is_ok()
            && self.primary.survives(entity)
            && self.tertiary.is_none_or(|receipt| receipt.survives(entity))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingPursuingC690 {
    Fresh {
        source_slot: ActorTaskSlot,
        reason: Intro2Type13PursuingC690Reason,
        continuation_elapsed_micros: u32,
    },
    Plan {
        receipt: PendingPursuingC690Receipt,
        reason: Intro2Type13PursuingC690Reason,
        continuation_elapsed_micros: u32,
        post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
    },
    Publication {
        receipt: PendingPursuingC690Receipt,
        reason: Intro2Type13PursuingC690Reason,
        planned: Type13WeightedSelection,
        predecessor_context: BehaviorContextRuntime,
        continuation_elapsed_micros: u32,
        post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
    },
}

impl PendingPursuingC690 {
    const fn receipt(self) -> PendingPursuingC690Receipt {
        match self {
            Self::Plan { receipt, .. } | Self::Publication { receipt, .. } => receipt,
            Self::Fresh { .. } => panic!("a fresh C690 continuation has no task receipt"),
        }
    }

    const fn reason(self) -> Intro2Type13PursuingC690Reason {
        match self {
            Self::Fresh { reason, .. }
            | Self::Plan { reason, .. }
            | Self::Publication { reason, .. } => reason,
        }
    }

    const fn continuation_elapsed_micros(self) -> u32 {
        match self {
            Self::Fresh {
                continuation_elapsed_micros,
                ..
            }
            | Self::Plan {
                continuation_elapsed_micros,
                ..
            }
            | Self::Publication {
                continuation_elapsed_micros,
                ..
            } => continuation_elapsed_micros,
        }
    }

    const fn post_chase_plus00_c690(self) -> Option<Intro2Type13PursuingC690Attempt> {
        match self {
            Self::Fresh { .. } => None,
            Self::Plan {
                post_chase_plus00_c690,
                ..
            }
            | Self::Publication {
                post_chase_plus00_c690,
                ..
            } => post_chase_plus00_c690,
        }
    }

    const fn predecessor_context(self) -> Option<BehaviorContextRuntime> {
        match self {
            Self::Fresh { .. } | Self::Plan { .. } => None,
            Self::Publication {
                predecessor_context,
                ..
            } => Some(predecessor_context),
        }
    }
}

fn apply_intro2_type13_pursuing_c690(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut crate::entity::EntityManager,
    entity_id: u32,
    chase: Option<Intro2Type13ChaseVisit>,
    prior_aim: Option<Result<Type13AimTickOutcome, Type13AimError>>,
    pending: PendingPursuingC690,
    prior_post_chase_plus00_c690: Option<Intro2Type13PursuingC690Attempt>,
    prior_acquisition: Option<SearchAttackLiveAcquisitionOutcome>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type13SchedulerOwnerTick {
    let pending = match pending {
        PendingPursuingC690::Fresh {
            source_slot,
            reason,
            continuation_elapsed_micros,
        } => {
            let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                };
            };
            let Some(receipt) = PendingPursuingC690Receipt::capture(entity, source_slot) else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                };
            };
            PendingPursuingC690::Plan {
                receipt,
                reason,
                continuation_elapsed_micros,
                post_chase_plus00_c690: prior_post_chase_plus00_c690,
            }
        }
        pending => pending,
    };
    let receipt = pending.receipt();
    let receipt_survives = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .is_some_and(|entity| receipt.survives(entity));
    if !receipt_survives {
        return Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    if pending.predecessor_context().is_some_and(|expected| {
        manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .is_none_or(|entity| {
                entity.current_behavior_context != RetailRuntimeValue::Known(Some(expected))
            })
    }) {
        return Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let reason = pending.reason();
    let request = Intro2Type13PursuingC690Request {
        source: receipt.source,
        reason,
    };
    let continuation_elapsed_micros = pending.continuation_elapsed_micros();
    let prior_post_chase_plus00_c690 = pending.post_chase_plus00_c690();
    let visit_later_slots = receipt.source.slot == ActorTaskSlot::Primary;
    let retry_stage = match pending {
        PendingPursuingC690::Plan {
            receipt,
            reason,
            continuation_elapsed_micros,
            post_chase_plus00_c690,
        } => Intro2Type13SchedulerStage::PendingPursuingC690Plan {
            receipt,
            reason,
            continuation_elapsed_micros,
            post_chase_plus00_c690,
        },
        PendingPursuingC690::Publication {
            receipt,
            reason,
            planned,
            predecessor_context,
            continuation_elapsed_micros,
            post_chase_plus00_c690,
        } => Intro2Type13SchedulerStage::PendingPursuingC690Publication {
            receipt,
            reason,
            planned,
            predecessor_context,
            continuation_elapsed_micros,
            post_chase_plus00_c690,
        },
        PendingPursuingC690::Fresh { .. } => unreachable!("fresh continuation was bound above"),
    };
    let blocked = |reason: Intro2Type13C690Block, stage: Intro2Type13SchedulerStage| {
        Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::PursuingC690Blocked {
                entity_id,
                chase,
                aim: prior_aim,
                acquisition: prior_acquisition.clone(),
                request,
                post_chase_plus00_c690: prior_post_chase_plus00_c690,
                reason,
            },
            retained_owner: Some(Intro2Type13SchedulerOwner {
                entity_id,
                stage,
                post_task_basis: None,
                dispatch_mode,
            }),
        }
    };

    let state_flags_raw = match pending {
        PendingPursuingC690::Publication { .. } => None,
        PendingPursuingC690::Plan { .. } => {
            let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                };
            };
            let suppression = entity
                .collision
                .state_flags_at_0x08
                .masked(TYPE13_C690_SUPPRESS_STATE_BIT);
            if matches!(suppression, RetailRuntimeValue::Unresolved) {
                return blocked(
                    Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
                    retry_stage,
                );
            }
            if matches!(
                suppression,
                RetailRuntimeValue::Known(flags) if flags & TYPE13_C690_SUPPRESS_STATE_BIT != 0
            ) {
                Some(RetailRuntimeValue::Known(TYPE13_C690_SUPPRESS_STATE_BIT))
            } else {
                Some(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(TYPE13_C690_SUPPRESS_STATE_BIT | TYPE13_C690_DYING_STATE_BIT),
                )
            }
        }
        PendingPursuingC690::Fresh { .. } => unreachable!("fresh continuation was bound above"),
    };
    let suppressed = matches!(
        state_flags_raw,
        Some(RetailRuntimeValue::Known(flags)) if flags & TYPE13_C690_SUPPRESS_STATE_BIT != 0
    );
    if !suppressed
        && matches!(pending, PendingPursuingC690::Plan { .. })
        && manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .is_none_or(|entity| {
                entity.current_behavior_context
                    != RetailRuntimeValue::Known(Some(receipt.predecessor_context))
            })
    {
        return Intro2Type13SchedulerOwnerTick {
            outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                entity_id,
                reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
            },
            retained_owner: None,
        };
    }
    let metadata = if suppressed {
        None
    } else {
        match manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned() {
            Some(metadata) => Some(metadata),
            None => return blocked(Intro2Type13C690Block::MetadataUnavailable, retry_stage),
        }
    };

    let (planned, predecessor_context) = if suppressed {
        (None, None)
    } else {
        match pending {
            PendingPursuingC690::Publication {
                planned,
                predecessor_context,
                ..
            } => (Some(planned), Some(predecessor_context)),
            PendingPursuingC690::Plan { .. } => {
                let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
                    return Intro2Type13SchedulerOwnerTick {
                        outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                            entity_id,
                            reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                        },
                        retained_owner: None,
                    };
                };
                match plan_type13_variant0_c690(
                    Type13Variant0C690Request {
                        entity_type: entity.entity_type,
                        predecessor: Type13C690Predecessor::Class7Pursuing,
                        current_behavior_context: entity.current_behavior_context,
                        state_flags_raw: state_flags_raw
                            .expect("the plan preflight retained resolved state flags"),
                        metadata: metadata
                            .as_ref()
                            .expect("an unsuppressed plan retained Type-13 metadata"),
                    },
                    || next_shared_random(world_fx),
                ) {
                    Ok(Type13C690Reselection::SuppressedByEntityState) => {
                        unreachable!("the outer state gate already admitted C690")
                    }
                    Ok(Type13C690Reselection::Applied(planned)) => {
                        (Some(planned), Some(receipt.predecessor_context))
                    }
                    Err(reason) => {
                        return blocked(Intro2Type13C690Block::Plan(reason), retry_stage)
                    }
                }
            }
            PendingPursuingC690::Fresh { .. } => unreachable!("fresh continuation was bound above"),
        }
    };

    let transition = match planned {
        None => Intro2Type13C690Transition::SuppressedByEntityState,
        Some(planned) => {
            let Some(entity) = manager.intro2_type13_entity_mut(entity_id) else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                };
            };
            match publish_type13_reselected_initial_behavior(
                entity,
                metadata
                    .as_ref()
                    .expect("a planned publication retained Type-13 metadata"),
                planned,
                || next_shared_random(world_fx),
            ) {
                Ok(publication) => Intro2Type13C690Transition::Published(publication),
                Err(publication_error) => {
                    return blocked(
                        Intro2Type13C690Block::Publication(publication_error),
                        Intro2Type13SchedulerStage::PendingPursuingC690Publication {
                            receipt,
                            reason,
                            planned,
                            predecessor_context: predecessor_context
                                .expect("a planned pursuing publication retained its context"),
                            continuation_elapsed_micros,
                            post_chase_plus00_c690: prior_post_chase_plus00_c690,
                        },
                    )
                }
            }
        }
    };

    let resulting_graph = match transition {
        Intro2Type13C690Transition::SuppressedByEntityState => Intro2Type13Graph::Class7Pursuing,
        Intro2Type13C690Transition::Published(
            Type13InitialBehaviorPublication::MoveAboutAimlessly(_),
        ) => Intro2Type13Graph::Class5Aimless,
        Intro2Type13C690Transition::Published(
            Type13InitialBehaviorPublication::SearchAndAttack(_),
        ) => Intro2Type13Graph::Class7Acquiring,
    };
    let mut post_chase_plus00_c690 = prior_post_chase_plus00_c690;
    let mut post_aim_plus04_c690 = None;
    let mut post_aim_plus00_c690 = None;
    let attempt = Intro2Type13PursuingC690Attempt {
        request,
        transition,
    };
    match (receipt.source.slot, request.slot()) {
        (ActorTaskSlot::Primary, Intro2Type13C690Slot::Plus00) => {
            post_chase_plus00_c690 = Some(attempt);
        }
        (ActorTaskSlot::Tertiary, Intro2Type13C690Slot::Plus04) => {
            post_aim_plus04_c690 = Some(attempt);
            if reason.falls_through_to_plus00_when_suppressed()
                && transition == Intro2Type13C690Transition::SuppressedByEntityState
            {
                post_aim_plus00_c690 = Some(Intro2Type13PursuingC690Attempt {
                    request: Intro2Type13PursuingC690Request {
                        source: receipt.source,
                        reason: Intro2Type13PursuingC690Reason::AimLifetimeExpired,
                    },
                    transition: Intro2Type13C690Transition::SuppressedByEntityState,
                });
            }
        }
        (ActorTaskSlot::Tertiary, Intro2Type13C690Slot::Plus00) => {
            post_aim_plus00_c690 = Some(attempt);
        }
        _ => unreachable!("pursuing C690 route must match its triggering task slot"),
    }
    let acquisition = if visit_later_slots && resulting_graph == Intro2Type13Graph::Class7Acquiring
    {
        match apply_type13_published_class7_acquisition_live(
            manager,
            entity_id,
            continuation_elapsed_micros,
            world_fx,
        ) {
            Ok(outcome) => Some(outcome),
            Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph) => {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                }
            }
            Err(_) => {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                }
            }
        }
    } else {
        prior_acquisition
    };
    let same_pass_aim = if visit_later_slots {
        tick_intro2_type13_ade0_same_pass_aim(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            continuation_elapsed_micros,
            acquisition.as_ref(),
        )
    } else {
        None
    };
    let aim = if same_pass_aim.is_some() {
        same_pass_aim
    } else if prior_aim.is_some() {
        prior_aim
    } else if visit_later_slots
        && matches!(
            transition,
            Intro2Type13C690Transition::SuppressedByEntityState
        )
    {
        let metadata = manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned();
        Some(tick_intro2_type13_aim(
            dispatch_mode,
            manager,
            world_fx,
            entity_id,
            continuation_elapsed_micros,
            metadata.as_ref(),
        ))
    } else {
        None
    };
    if visit_later_slots {
        if let Some(reason) = type13_aim_c690_reason(aim.as_ref()) {
            return apply_intro2_type13_pursuing_c690(
                dispatch_mode,
                manager,
                entity_id,
                chase,
                aim,
                PendingPursuingC690::Fresh {
                    source_slot: ActorTaskSlot::Tertiary,
                    reason,
                    continuation_elapsed_micros,
                },
                post_chase_plus00_c690,
                acquisition,
                world_fx,
                next_shared_random,
            );
        }
    }
    let retained_graph = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| match resulting_graph {
            Intro2Type13Graph::Class5Aimless => {
                authenticate_published_type13_class5_aimless_graph(entity)
                    .ok()
                    .map(|()| Intro2Type13Graph::Class5Aimless)
            }
            Intro2Type13Graph::Class7Acquiring => {
                if authenticate_published_type13_class7_b6c0_graph(entity).is_ok() {
                    Some(Intro2Type13Graph::Class7Acquiring)
                } else if authenticate_published_type13_class7_pursuing_graph(entity).is_ok() {
                    Some(Intro2Type13Graph::Class7Pursuing)
                } else {
                    None
                }
            }
            Intro2Type13Graph::Class7Pursuing => {
                authenticate_published_type13_class7_pursuing_graph(entity)
                    .ok()
                    .map(|()| Intro2Type13Graph::Class7Pursuing)
            }
        });
    Intro2Type13SchedulerOwnerTick {
        outcome: Intro2Type13SchedulerProductionOutcome::PursuingVisit {
            entity_id,
            chase,
            post_chase_plus00_c690,
            acquisition,
            aim,
            post_aim_plus04_c690,
            post_aim_plus00_c690,
        },
        retained_owner: retained_graph.map(|graph| Intro2Type13SchedulerOwner {
            entity_id,
            stage: Intro2Type13SchedulerStage::Running(graph),
            post_task_basis: None,
            dispatch_mode,
        }),
    }
}

fn tick_intro2_type13_pursuing_chase(
    manager: &mut crate::entity::EntityManager,
    entity_id: u32,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    frame: Intro2Type13PrimaryFrame<'_>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<Intro2Type13ChaseVisit, Intro2Type13ChaseBlock> {
    let (axis, owner_position_raw, target_id) = {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return Err(Intro2Type13ChaseBlock::VisitUnavailable);
        };
        authenticate_intro2_type13(entity).map_err(|_| Intro2Type13ChaseBlock::GraphMismatch)?;
        authenticate_published_type13_class7_pursuing_graph(entity)
            .map_err(|_| Intro2Type13ChaseBlock::GraphMismatch)?;
        let RetailRuntimeValue::Known(axis) = entity.actor_common_axis_descriptor else {
            return Err(Intro2Type13ChaseBlock::CommonAxisUnresolved);
        };
        let Some(ActorTaskRuntime::ChaseTarget(state)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            return Err(Intro2Type13ChaseBlock::VisitUnavailable);
        };
        (axis, entity.position_raw(), state.target_id())
    };
    let (target_state, tracked_target) = type13_chase_target_runtime_state(manager, target_id)
        .map_err(|()| Intro2Type13ChaseBlock::TargetStateUnresolved)?;
    let Some(entity) = manager.intro2_type13_entity_mut(entity_id) else {
        return Err(Intro2Type13ChaseBlock::VisitUnavailable);
    };
    let RetailRuntimeValue::Known(Some(sub_g_runtime)) = entity.sub_g_06070_runtime else {
        return Err(Intro2Type13ChaseBlock::SubGRuntimeUnavailable);
    };
    let Some(common_mover_runtime) = entity.intro2_type13_common_mover_runtime else {
        return Err(Intro2Type13ChaseBlock::SubGRuntimeUnavailable);
    };
    let Some(task_id) = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary) else {
        return Err(Intro2Type13ChaseBlock::VisitUnavailable);
    };
    let visit = ActorTaskVisit {
        slot: ActorTaskSlot::Primary,
        task_id,
    };
    let Some(prefix) = entity.actor_tasks.begin_exact_visit_with(visit, |runtime| {
        let ActorTaskRuntime::ChaseTarget(state) = runtime else {
            unreachable!("authenticated Type-13 pursuing Primary is ChaseTarget");
        };
        state.before_callback(frame.elapsed_micros)
    }) else {
        return Err(Intro2Type13ChaseBlock::VisitUnavailable);
    };
    let Some(ActorTaskRuntime::ChaseTarget(state)) =
        entity.actor_tasks.state_in_slot(ActorTaskSlot::Primary)
    else {
        let _ = entity.actor_tasks.finish_exact_visit(visit);
        return Err(Intro2Type13ChaseBlock::VisitUnavailable);
    };
    let mut stage = state.stage_callback();
    let mut movement = ();
    let mut controller = ();
    let mut applied_mover = None;
    let [heading_raw, pitch_raw, roll_raw] = entity.rotation_heading_pitch_roll_raw();
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
            elapsed_micros: frame.elapsed_micros,
            scheduler_mode: match frame.dispatch_mode {
                CommonMoverDispatchMode::Normal => 0,
                CommonMoverDispatchMode::Restricted => 1,
            },
        },
        |_| Ok::<_, ()>(target_state),
        |request| {
            let outcome = evaluate_type13_common_mover(
                GklCommonMoverRequest {
                    dispatch_mode: frame.dispatch_mode,
                    entity_id,
                    entity_type: entity.entity_type,
                    metadata,
                    position_raw: entity.position_raw(),
                    velocity_raw: entity.velocity_raw(),
                    heading_raw: heading_raw as u16,
                    pitch_raw,
                    roll_raw: roll_raw as u16,
                    body_basis: entity.physical_body_basis_q31(),
                    runtime: common_mover_runtime,
                    sub_g_runtime,
                    target_private: *request.target_state,
                    tracked_target,
                    terrain: frame.terrain,
                    active_model_extent_raw: frame.active_model_extent_raw,
                    self_mass_raw: entity.mass_raw,
                    attached_cargo_mass: frame.attached_cargo_mass,
                    capability_flags: entity.capability_flags,
                    retail_tick: frame.retail_tick,
                    elapsed_micros: frame.elapsed_micros,
                    global_elapsed_micros: frame.global_elapsed_micros,
                },
                || next_random(),
            )?;
            *request.target_state = outcome.target_private;
            let result = outcome.result;
            applied_mover = Some(outcome);
            Ok::<_, GklCommonMoverBlock>(result)
        },
        |_, _| unreachable!("Type-13 Chase disables proximity-controller writes"),
    );
    let survived = if entity
        .actor_tasks
        .wrapper_flags(visit.task_id)
        .is_some_and(|flags| flags.in_callback)
    {
        entity.actor_tasks.finish_exact_visit(visit)
    } else {
        false
    };
    if evaluated.is_ok() && survived {
        if let Some(ActorTaskRuntime::ChaseTarget(surviving)) =
            entity.actor_tasks.task_state_mut(visit.task_id)
        {
            stage.commit(surviving);
        }
    }
    let result = match evaluated {
        Ok(ChaseTargetCallbackResult::Tagged(singleton)) => {
            Intro2Type13ChaseVisitResult::Tagged(singleton)
        }
        Ok(ChaseTargetCallbackResult::Continue { .. }) => Intro2Type13ChaseVisitResult::Continue,
        Err(crate::chase_target::ChaseTargetCallbackError::CommonMover { error, .. }) => {
            Intro2Type13ChaseVisitResult::CommonMoverBlocked(error)
        }
        Err(_) => return Err(Intro2Type13ChaseBlock::VisitUnavailable),
    };
    let mut sound = None;
    if let Some(outcome) = applied_mover {
        entity.intro2_type13_common_mover_runtime = Some(outcome.runtime);
        entity.sub_g_06070_runtime = RetailRuntimeValue::Known(Some(outcome.sub_g_runtime));
        entity.set_rotation_heading_pitch_roll_raw([
            outcome.heading_raw as i16,
            outcome.pitch_raw,
            outcome.roll_raw as i16,
        ]);
        entity.set_velocity_raw(outcome.velocity_raw);
        sound = outcome.sound;
    }
    Ok(Intro2Type13ChaseVisit {
        prefix,
        result,
        sound,
    })
}

fn type13_chase_target_runtime_state(
    manager: &crate::entity::EntityManager,
    target_id: u32,
) -> Result<
    (
        ChaseTargetTargetRuntimeState,
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    ),
    (),
> {
    let Some(target) = manager.iter_all().find(|entity| entity.id == target_id) else {
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
    let state = target.collision.state_flags_at_0x08;
    let snapshot = RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
        state_flags: state,
        position_raw: target.position_raw(),
        velocity_raw: target.velocity_raw(),
    }));
    match state.masked(DYING_STATE_BIT) {
        RetailRuntimeValue::Unresolved => return Err(()),
        RetailRuntimeValue::Known(bits) if bits != 0 => {
            return Ok((ChaseTargetTargetRuntimeState::Dying, snapshot));
        }
        RetailRuntimeValue::Known(_) => {}
    }
    if state.known_value_bits() == 0 {
        if state.known_mask() != u32::MAX {
            return Err(());
        }
        return Ok((ChaseTargetTargetRuntimeState::Inactive, snapshot));
    }
    Ok((
        ChaseTargetTargetRuntimeState::Live {
            position_raw: target.position_raw(),
        },
        snapshot,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingType13C690 {
    Plan {
        predecessor: Intro2Type13Graph,
        request: SharedRetargetTransitionRequest,
        continuation_elapsed_micros: u32,
    },
    Publication {
        predecessor: Intro2Type13Graph,
        request: SharedRetargetTransitionRequest,
        planned: Type13WeightedSelection,
        predecessor_context: BehaviorContextRuntime,
        continuation_elapsed_micros: u32,
    },
}

impl PendingType13C690 {
    const fn predecessor(self) -> Intro2Type13Graph {
        match self {
            Self::Plan { predecessor, .. } | Self::Publication { predecessor, .. } => predecessor,
        }
    }

    const fn request(self) -> SharedRetargetTransitionRequest {
        match self {
            Self::Plan { request, .. } | Self::Publication { request, .. } => request,
        }
    }

    const fn continuation_elapsed_micros(self) -> u32 {
        match self {
            Self::Plan {
                continuation_elapsed_micros,
                ..
            }
            | Self::Publication {
                continuation_elapsed_micros,
                ..
            } => continuation_elapsed_micros,
        }
    }

    const fn predecessor_context(self) -> Option<BehaviorContextRuntime> {
        match self {
            Self::Plan { .. } => None,
            Self::Publication {
                predecessor_context,
                ..
            } => Some(predecessor_context),
        }
    }
}

fn finish_intro2_type13_c690(
    dispatch_mode: CommonMoverDispatchMode,
    manager: &mut crate::entity::EntityManager,
    entity_id: u32,
    pending: PendingType13C690,
    primary: Option<Intro2Type13PrimaryVisit>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> Intro2Type13SchedulerOwnerTick {
    let request = pending.request();
    let predecessor = pending.predecessor();
    let continuation_elapsed_micros = pending.continuation_elapsed_micros();
    let retry_stage = match pending {
        PendingType13C690::Plan {
            predecessor,
            request,
            continuation_elapsed_micros,
        } => Intro2Type13SchedulerStage::PendingC690Plan {
            predecessor,
            request,
            continuation_elapsed_micros,
        },
        PendingType13C690::Publication {
            predecessor,
            request,
            planned,
            predecessor_context,
            continuation_elapsed_micros,
        } => Intro2Type13SchedulerStage::PendingC690Publication {
            predecessor,
            request,
            planned,
            predecessor_context,
            continuation_elapsed_micros,
        },
    };

    // FUN_00401120 owns the state gate before the C690 callback. In
    // particular, suppression neither needs Type-13 metadata/context nor
    // consumes the root selector.
    let state_flags_raw = match pending {
        PendingType13C690::Publication { .. } => None,
        PendingType13C690::Plan { .. } => {
            let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                };
            };
            let suppression = entity
                .collision
                .state_flags_at_0x08
                .masked(TYPE13_C690_SUPPRESS_STATE_BIT);
            if matches!(suppression, RetailRuntimeValue::Unresolved) {
                return blocked_intro2_type13_c690(
                    dispatch_mode,
                    entity_id,
                    request,
                    Intro2Type13C690Block::Plan(Type13C690ReselectionBlock::StateFlagsUnresolved),
                    primary,
                    retry_stage,
                );
            }
            if matches!(
                suppression,
                RetailRuntimeValue::Known(flags) if flags & TYPE13_C690_SUPPRESS_STATE_BIT != 0
            ) {
                Some(RetailRuntimeValue::Known(TYPE13_C690_SUPPRESS_STATE_BIT))
            } else {
                Some(
                    entity
                        .collision
                        .state_flags_at_0x08
                        .masked(TYPE13_C690_SUPPRESS_STATE_BIT | TYPE13_C690_DYING_STATE_BIT),
                )
            }
        }
    };
    let suppressed = matches!(
        state_flags_raw,
        Some(RetailRuntimeValue::Known(flags)) if flags & TYPE13_C690_SUPPRESS_STATE_BIT != 0
    );

    let metadata = if suppressed {
        None
    } else {
        match manager.type_runtime_metadata(TYPE13_ENTITY_TYPE).cloned() {
            Some(metadata) => Some(metadata),
            None => {
                return blocked_intro2_type13_c690(
                    dispatch_mode,
                    entity_id,
                    request,
                    Intro2Type13C690Block::MetadataUnavailable,
                    primary,
                    retry_stage,
                )
            }
        }
    };

    let (planned, predecessor_context) = if suppressed {
        (None, None)
    } else {
        match pending {
            PendingType13C690::Publication {
                planned,
                predecessor_context,
                ..
            } => (Some(planned), Some(predecessor_context)),
            PendingType13C690::Plan { .. } => {
                let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
                    return Intro2Type13SchedulerOwnerTick {
                        outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                            entity_id,
                            reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                        },
                        retained_owner: None,
                    };
                };
                let predecessor_context = match entity.current_behavior_context {
                    RetailRuntimeValue::Known(Some(context)) => Some(context),
                    _ => None,
                };
                match plan_type13_variant0_c690(
                    Type13Variant0C690Request {
                        entity_type: entity.entity_type,
                        predecessor: predecessor.c690_predecessor(),
                        current_behavior_context: entity.current_behavior_context,
                        state_flags_raw: state_flags_raw
                            .expect("the plan preflight retained resolved state flags"),
                        metadata: metadata
                            .as_ref()
                            .expect("an unsuppressed plan retained Type-13 metadata"),
                    },
                    || next_shared_random(world_fx),
                ) {
                    Ok(Type13C690Reselection::SuppressedByEntityState) => {
                        unreachable!("the outer state gate already admitted C690")
                    }
                    Ok(Type13C690Reselection::Applied(planned)) => {
                        (
                            Some(planned),
                            Some(predecessor_context.expect(
                                "an applied C690 plan authenticated the predecessor context",
                            )),
                        )
                    }
                    Err(reason) => {
                        return blocked_intro2_type13_c690(
                            dispatch_mode,
                            entity_id,
                            request,
                            Intro2Type13C690Block::Plan(reason),
                            primary,
                            retry_stage,
                        )
                    }
                }
            }
        }
    };

    let transition = match planned {
        None => Intro2Type13C690Transition::SuppressedByEntityState,
        Some(planned) => {
            let Some(entity) = manager.intro2_type13_entity_mut(entity_id) else {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                };
            };
            match publish_type13_reselected_initial_behavior(
                entity,
                metadata
                    .as_ref()
                    .expect("a planned publication retained Type-13 metadata"),
                planned,
                || next_shared_random(world_fx),
            ) {
                Ok(publication) => Intro2Type13C690Transition::Published(publication),
                Err(reason) => {
                    return blocked_intro2_type13_c690(
                        dispatch_mode,
                        entity_id,
                        request,
                        Intro2Type13C690Block::Publication(reason),
                        primary,
                        Intro2Type13SchedulerStage::PendingC690Publication {
                            predecessor,
                            request,
                            planned,
                            predecessor_context: predecessor_context
                                .expect("a planned publication retained its predecessor context"),
                            continuation_elapsed_micros,
                        },
                    )
                }
            }
        }
    };

    let resulting_graph = match transition {
        Intro2Type13C690Transition::SuppressedByEntityState => predecessor,
        Intro2Type13C690Transition::Published(
            Type13InitialBehaviorPublication::MoveAboutAimlessly(_),
        ) => Intro2Type13Graph::Class5Aimless,
        Intro2Type13C690Transition::Published(
            Type13InitialBehaviorPublication::SearchAndAttack(_),
        ) => Intro2Type13Graph::Class7Acquiring,
    };
    let visit_secondary = resulting_graph == Intro2Type13Graph::Class7Acquiring;
    let acquisition = if visit_secondary {
        let acquisition = if matches!(
            transition,
            Intro2Type13C690Transition::SuppressedByEntityState
        ) {
            let task_graph_survives = manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .is_some_and(|entity| {
                    authenticate_published_type13_class7_b6c0_task_graph(entity).is_ok()
                });
            if !task_graph_survives {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                };
            }
            Ok(apply_search_attack_acquisition_live(
                manager,
                entity_id,
                continuation_elapsed_micros,
                world_fx,
                SearchAttackLiveHandoffRequirement::RequiredClass7Variant0,
                SearchAttackLiveSamePassAim::Skip,
            ))
        } else {
            apply_type13_published_class7_acquisition_live(
                manager,
                entity_id,
                continuation_elapsed_micros,
                world_fx,
            )
        };
        match acquisition {
            Ok(outcome) => Some(outcome),
            Err(Type13Class7LiveAcquisitionError::UnpublishedB6c0Graph) => {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::GraphMismatch,
                    },
                    retained_owner: None,
                }
            }
            Err(_) => {
                return Intro2Type13SchedulerOwnerTick {
                    outcome: Intro2Type13SchedulerProductionOutcome::Dropped {
                        entity_id,
                        reason: Intro2Type13SchedulerProductionDrop::EntityUnavailable,
                    },
                    retained_owner: None,
                }
            }
        }
    } else {
        None
    };
    let aim = tick_intro2_type13_ade0_same_pass_aim(
        dispatch_mode,
        manager,
        world_fx,
        entity_id,
        continuation_elapsed_micros,
        acquisition.as_ref(),
    );
    if let Some(reason) = type13_aim_c690_reason(aim.as_ref()) {
        return apply_intro2_type13_pursuing_c690(
            dispatch_mode,
            manager,
            entity_id,
            None,
            aim,
            PendingPursuingC690::Fresh {
                source_slot: ActorTaskSlot::Tertiary,
                reason,
                continuation_elapsed_micros,
            },
            None,
            acquisition,
            world_fx,
            next_shared_random,
        );
    }
    let retained_graph = manager
        .iter_all()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| match resulting_graph {
            Intro2Type13Graph::Class5Aimless => {
                authenticate_published_type13_class5_aimless_graph(entity)
                    .ok()
                    .map(|()| Intro2Type13Graph::Class5Aimless)
            }
            Intro2Type13Graph::Class7Acquiring => {
                if authenticate_published_type13_class7_b6c0_graph(entity).is_ok() {
                    Some(Intro2Type13Graph::Class7Acquiring)
                } else if authenticate_published_type13_class7_pursuing_graph(entity).is_ok() {
                    Some(Intro2Type13Graph::Class7Pursuing)
                } else {
                    None
                }
            }
            Intro2Type13Graph::Class7Pursuing => {
                authenticate_published_type13_class7_pursuing_graph(entity)
                    .ok()
                    .map(|()| Intro2Type13Graph::Class7Pursuing)
            }
        });
    Intro2Type13SchedulerOwnerTick {
        outcome: Intro2Type13SchedulerProductionOutcome::C690Transition {
            entity_id,
            request,
            transition,
            primary,
            acquisition,
            aim,
        },
        retained_owner: retained_graph.map(|graph| Intro2Type13SchedulerOwner {
            entity_id,
            stage: Intro2Type13SchedulerStage::Running(graph),
            post_task_basis: None,
            dispatch_mode,
        }),
    }
}

fn blocked_intro2_type13_c690(
    dispatch_mode: CommonMoverDispatchMode,
    entity_id: u32,
    request: SharedRetargetTransitionRequest,
    reason: Intro2Type13C690Block,
    primary: Option<Intro2Type13PrimaryVisit>,
    stage: Intro2Type13SchedulerStage,
) -> Intro2Type13SchedulerOwnerTick {
    Intro2Type13SchedulerOwnerTick {
        outcome: Intro2Type13SchedulerProductionOutcome::C690Blocked {
            entity_id,
            request,
            reason,
            primary,
        },
        retained_owner: Some(Intro2Type13SchedulerOwner {
            entity_id,
            stage,
            post_task_basis: None,
            dispatch_mode,
        }),
    }
}

fn pending_type13_transition_graph_survives(entity: &Entity, pending: PendingType13C690) -> bool {
    let request = pending.request();
    let task_graph_survives = match pending.predecessor() {
        Intro2Type13Graph::Class5Aimless => {
            authenticate_published_type13_class5_aimless_task_graph(entity).is_ok()
        }
        Intro2Type13Graph::Class7Acquiring => {
            authenticate_published_type13_class7_b6c0_task_graph(entity).is_ok()
        }
        Intro2Type13Graph::Class7Pursuing => false,
    };
    if authenticate_intro2_type13(entity).is_err()
        || !task_graph_survives
        || request.slot != ActorTaskSlot::Primary
        || entity.actor_tasks.task_in_slot(request.slot) != Some(request.task_id)
        || !entity
            .actor_tasks
            .wrapper_flags(request.task_id)
            .is_some_and(|flags| flags.alive && !flags.in_callback)
    {
        return false;
    }
    if pending.predecessor_context().is_some_and(|expected| {
        entity.current_behavior_context != RetailRuntimeValue::Known(Some(expected))
    }) {
        return false;
    }
    let Some(ActorTaskRuntime::SharedRetarget(state)) = entity.actor_task_state(request.slot)
    else {
        return false;
    };
    if state.elapsed_ms() != request.committed_prefix.elapsed_ms
        || state.lifetime_ms() != request.committed_prefix.lifetime_ms
    {
        return false;
    }
    match request.committed_prefix.retarget {
        SharedRetarget::Retained => true,
        SharedRetarget::Replaced {
            target_position_raw,
            ..
        } => state.private_state().target_position_raw == target_position_raw,
    }
}
