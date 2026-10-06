//! Production custody for selected fresh-Level-1 ordinary Type-9 Go To Job.
//!
//! This owner binds class 54 to the real common scheduler and heterogeneous
//! task dispatcher. It preserves the exact target/predicate/common-mover
//! callback, canonical root reselection, selected class-6/class-10/class-54 application, and
//! the outer `FUN_00413F70` basis publication. A published F70 frame continues
//! through the shared E100/DF70/E370 tail, the unconditional `+0xB2` clear,
//! and master motion. Fresh first `FUN_00412DA0` visits apply live flags
//! `0x00468805` and wait-clear `+0xB2` to `Known(0)` before callback-mass. E370
//! expiry runs the generic Type-9 standard-death publisher, consumes this
//! owner after class 14, and exposes the exploding Primary lease for
//! scheduler adoption.

use crate::gameplay_notifications::OrdinaryType9LiveNotificationContext;
use crate::ordinary_type9_current_task::{
    OrdinaryType9CurrentTaskAuthority, OrdinaryType9CurrentTaskPublication,
    OrdinaryType9ProductionAuthority,
};

use std::{cell::RefCell, num::NonZeroU64, rc::Rc};

use v2k_formats::collision::CommonAxisDescriptor;

use crate::{
    actor_animation::ActorAnimationController,
    actor_task_dispatcher::{
        prepare_run_away_runtime_task, tick_actor_task_dispatcher,
        tick_actor_task_dispatcher_from_slot, ActorTaskDispatcherAdapter, ActorTaskDispatcherError,
        ActorTaskDispatcherFrame, ActorTaskRuntime, AttractAttentionCueTransitionOutcome,
        FollowBeaconsFollowingAdapterError, FollowBeaconsFollowingStyleResultOutcome,
        RunAwayRuntimeConstructorEffect, SharedGenericConstructorEffect,
    },
    actor_task_owner::{ActorTaskOwner, ActorTaskSlot, ActorTaskVisit, ActorTaskWrapperFlags},
    attract_attention::{
        AttractAttentionInitialTaskPreparation, AttractAttentionTargetExecutionOutcome,
        AttractAttentionTargetTaskPreparation,
    },
    common_mover::{
        actor_abdi::ActorAbdiAnimationPolicy,
        target_prelude::CommonMoverTrackedTargetSnapshot,
        type9::OrdinaryType9Topology,
        type9_attitude::{Type9BodyBasis, TERRAIN_ATTITUDE_EFFECTIVE_FLAG},
        type9_tail::ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        type9_transaction::{OrdinaryType9FrameLease, OrdinaryType9TransactionId},
        SubAPropulsionRuntime,
    },
    entity::{
        radians_to_binary_angle, raw_position_world, world_position_raw, Entity, EntityManager,
    },
    entity_behavior::{
        audited_behavior_style, behavior_program, ActiveBehaviorStyle, BehaviorChoiceListSource,
        BehaviorContextRuntime, BehaviorDescriptorIdentity,
    },
    entity_collision_state::{
        EntityCollisionRuntimeState, EntityTypeRuntimeMetadata, RetailRuntimeValue,
        ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT, DYING_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    entity_scheduler::{
        commit_common_scheduler_prefix, common_scheduler_callback_mass,
        plan_common_scheduler_prefix, CommonSchedulerPrefixFlow,
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
    },
    go_to_job::{GoToJobTaskSpec, GO_TO_JOB_TASK_LIFETIME_MS},
    go_to_job_owner::{
        evaluate_go_to_job_callback, GoToJobCallbackError, GoToJobCallbackRequest,
        GoToJobCallbackResult, GoToJobCallbackStage, GoToJobCommonMoverReturn,
        GoToJobLifetimeStatus, GoToJobRoutePredicate, GoToJobTargetRuntimeState, GoToJobTaskState,
        GoToJobTransitionReason, GoToJobTransitionRequest,
    },
    guard_location_owner::acquisition::GuardLocationEntityRef,
    job_nearby::JobCapacityState,
    main_base_abort::MainBaseAbortActorLease,
    main_base_type9_abort::{LEVEL_ONE_TYPE9_MASS_RAW, LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR},
    ordinary_type9_attract_attention_handoff::{
        publish_candidate_dispatch_filter, OrdinaryType9AttractAttentionCandidateDispatch,
        OrdinaryType9AttractAttentionCandidateDispatchParts,
        OrdinaryType9AttractAttentionCandidateTickError,
        OrdinaryType9AttractAttentionTargetAllocationDecision,
        OrdinaryType9AttractAttentionTargetInitializerFailure,
        OrdinaryType9AttractAttentionTargetRouteFrame,
        OrdinaryType9AttractAttentionTargetRouteOwner,
        OrdinaryType9AttractAttentionTargetRouteTickError,
        OrdinaryType9AttractAttentionTargetRouteTickOutcome,
    },
    ordinary_type9_attract_attention_initializer::{
        OrdinaryType9AttractAttentionAllocationDecision,
        OrdinaryType9AttractAttentionCommittedEffects, OrdinaryType9AttractAttentionInitialOwners,
        OrdinaryType9AttractAttentionInitializerFailure,
    },
    ordinary_type9_attract_attention_production::commit_attract_attention_root_sounds,
    ordinary_type9_go_to_job_initializer::{
        OrdinaryType9GoToJobAllocationDecision, OrdinaryType9GoToJobCandidateEvidence,
        OrdinaryType9GoToJobConstructorEvidence, OrdinaryType9GoToJobInitializerFailure,
    },
    ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner,
    ordinary_type9_initial_selection::{
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID, LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
        LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID, LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
    },
    ordinary_type9_live::{
        apply_fresh_level1_ordinary_type9_first_scheduler_state,
        run_selected_ordinary_type9_common_mover_with_animation_policy,
        OrdinaryType9SelectedCommonMoverComponentCustody, OrdinaryType9SelectedCommonMoverError,
        OrdinaryType9SelectedCommonMoverRequest, OrdinaryType9SelectedComponentRuntime,
        OrdinaryType9SelectedRuntimeKind,
    },
    ordinary_type9_outer_tail::{
        drive_outer_tail_transaction, outer_tail_class14_exploding_task_lease,
        outer_tail_entity_published_class14, outer_tail_observation_authenticates,
        resume_outer_tail_bubble, snapshot_outer_tail_animation_offset, snapshot_outer_tail_state,
        start_outer_tail_transaction, take_outer_transaction_id, OrdinaryType9OuterTailBlock,
        OrdinaryType9OuterTailCustody, OrdinaryType9OuterTailDrive,
        OrdinaryType9OuterTailStartError,
    },
    ordinary_type9_root_attract_attention_application::{
        apply_ordinary_type9_root_attract_attention,
        apply_ordinary_type9_root_attract_attention_from_go_to_job,
        apply_ordinary_type9_root_attract_attention_from_go_to_job_parts,
        apply_ordinary_type9_root_attract_attention_parts,
        OrdinaryType9RootAttractAttentionApplicationOutcome,
        OrdinaryType9RootAttractAttentionMutableParts,
        OrdinaryType9RootAttractAttentionPreflightError,
        OrdinaryType9RootAttractAttentionPublicationAuth,
    },
    ordinary_type9_root_go_to_job_application::{
        apply_ordinary_type9_root_go_to_job,
        apply_ordinary_type9_root_go_to_job_from_attract_attention,
        apply_ordinary_type9_root_go_to_job_from_attract_attention_parts,
        apply_ordinary_type9_root_go_to_job_parts, OrdinaryType9RootGoToJobApplicationOutcome,
        OrdinaryType9RootGoToJobMutableParts, OrdinaryType9RootGoToJobPreflightError,
        OrdinaryType9RootGoToJobPublicationOwner,
    },
    ordinary_type9_root_reselection::{
        plan_ordinary_type9_root_reselection, OrdinaryType9RootEntityRef,
        OrdinaryType9RootReselectionError, OrdinaryType9RootReselectionPlan,
        OrdinaryType9RootReselectionRequest, OrdinaryType9RootSelection,
    },
    ordinary_type9_root_run_away_application::{
        apply_ordinary_type9_root_run_away_from_attract_attention,
        apply_ordinary_type9_root_run_away_from_attract_attention_parts,
        apply_ordinary_type9_root_run_away_from_go_to_job,
        apply_ordinary_type9_root_run_away_from_go_to_job_parts,
        OrdinaryType9RootRunAwayApplicationOutcome, OrdinaryType9RootRunAwayMutableParts,
        OrdinaryType9RootRunAwayPreflightError, OrdinaryType9RootRunAwayPublicationOwner,
    },
    ordinary_type9_root_wander_application::{
        apply_ordinary_type9_root_wander, apply_ordinary_type9_root_wander_parts,
        OrdinaryType9RootWanderApplicationOutcome, OrdinaryType9RootWanderEntityFacts,
        OrdinaryType9RootWanderMutableParts, OrdinaryType9RootWanderPreflightError,
        OrdinaryType9RootWanderPublicationOwner,
    },
    ordinary_type9_run_away_initializer::{
        OrdinaryType9RunAwayAllocationDecision, OrdinaryType9RunAwayConstructorEvidence,
        OrdinaryType9RunAwayInitializerFailure,
    },
    ordinary_type9_wander_initializer::{
        OrdinaryType9WanderAllocationDecision, OrdinaryType9WanderConstructorEvidence,
        OrdinaryType9WanderInitializerFailure,
    },
    ordinary_type9_wander_owner::OrdinaryType9WanderTaskSpec,
    resource_cache::ResourceCache,
    run_away::{
        apply_run_away_task_setup, RunAwayAuthoredAudio, RunAwayTaskPreparation,
        RunAwayTaskSetupRequest, RUN_AWAY_BEHAVIOR_CLASS_ID,
    },
    search_attack::{SearchAttackEntityRef, SearchAttackTargetHandoff},
    search_attack_acquisition::{
        evaluate_target_acquisition_callback, TargetAcquisitionCallbackError,
        TargetAcquisitionCallbackPrefix, TargetAcquisitionCallbackResult,
    },
    shared_target_route::{
        SharedTargetRouteCommonMoverReturn, SharedTargetRoutePredicate,
        SharedTargetRouteTargetRuntimeState,
    },
    wander_near_location::{WanderNearCommonMoverReturn, WanderNearPrivateState},
    world_fx::{TerrainCollisionContext, WorldFx},
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};

fn fork_attract_attention_initial_owners(
    owners: &OrdinaryType9AttractAttentionInitialOwners,
) -> OrdinaryType9AttractAttentionInitialOwners {
    match owners {
        OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue } => {
            OrdinaryType9AttractAttentionInitialOwners::CueOnly {
                cue: cue.fork_for_main_base_abort_transaction(),
            }
        }
        OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue } => {
            OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                candidate: candidate.fork_for_main_base_abort_transaction(),
                cue: cue.fork_for_main_base_abort_transaction(),
            }
        }
    }
}

const BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG: u32 = 0x0000_4000;
const LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID: u16 = 85;
const LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobPostTaskFrame {
    callback_elapsed_micros: u32,
    origin_retail_tick: u32,
    latched_effective_flags: u32,
    post_task_angles_raw: [i16; 3],
    pre_basis: RetailRuntimeValue<Type9BodyBasis>,
}

impl OrdinaryType9GoToJobPostTaskFrame {
    pub const fn callback_elapsed_micros(self) -> u32 {
        self.callback_elapsed_micros
    }

    pub const fn latched_effective_flags(self) -> u32 {
        self.latched_effective_flags
    }

    pub const fn post_task_angles_raw(self) -> [i16; 3] {
        self.post_task_angles_raw
    }

    pub fn published_body_basis(self) -> Type9BodyBasis {
        Type9BodyBasis::from_angle_words(
            self.post_task_angles_raw[0],
            self.post_task_angles_raw[1],
            self.post_task_angles_raw[2],
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobRootTransition {
    entity_id: u32,
    request: GoToJobTransitionRequest,
    task_state: GoToJobTaskState,
    selected_runtime: OrdinaryType9SelectedComponentRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobProductionState {
    Active,
    PostBasisTailPending {
        post_task_frame: OrdinaryType9GoToJobPostTaskFrame,
    },
    RootTransitionPending {
        transition: Option<OrdinaryType9GoToJobRootTransition>,
        post_task_frame: OrdinaryType9GoToJobPostTaskFrame,
    },
    CallbackFailurePending,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9GoToJobProductionOwner {
    original_manager_sidecar_index: Option<usize>,
    task_authority: OrdinaryType9ProductionAuthority,
    actor_lease: MainBaseAbortActorLease,
    state: OrdinaryType9GoToJobProductionState,
    next_transaction_id: u64,
    pending_root_plan: Option<OrdinaryType9RootReselectionPlan>,
    pending_root_task_visits: Option<[Option<ActorTaskVisit>; 3]>,
    pending_root_actor_common_axis: Option<CommonAxisDescriptor>,
    pending_root_sub_a: Option<SubAPropulsionRuntime>,
    pending_root_actor_animation: Option<ActorAnimationController>,
    pending_root_dispatcher_continuation: Option<OrdinaryType9GoToJobDispatcherContinuation>,
    root_publication: Option<OrdinaryType9GoToJobRootPublication>,
    root_attract_attention_initial_owners: Option<OrdinaryType9AttractAttentionInitialOwners>,
    root_attract_attention_target_route: Option<OrdinaryType9AttractAttentionTargetRouteOwner>,
    outer_tail: Option<OrdinaryType9OuterTailCustody>,
    pending_outer_outcome: Option<OrdinaryType9GoToJobProductionOutcome>,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdinaryType9GoToJobRootPublication {
    Wander(OrdinaryType9RootWanderPublicationOwner),
    GoToJob(OrdinaryType9RootGoToJobPublicationOwner),
    RunAway(OrdinaryType9RootRunAwayPublicationOwner),
    AttractAttention(OrdinaryType9RootAttractAttentionPublicationAuth),
    InitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected_runtime: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
    GoToJobInitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected_runtime: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        actor_common_axis: CommonAxisDescriptor,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
    RunAwayInitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected_runtime: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        actor_common_axis: CommonAxisDescriptor,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
    AttractAttentionInitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected_runtime: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        actor_common_axis: CommonAxisDescriptor,
        sub_a: SubAPropulsionRuntime,
        actor_animation: ActorAnimationController,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
}

impl OrdinaryType9GoToJobRootPublication {
    fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::Wander(owner) => Self::Wander(owner.fork_for_main_base_abort_transaction()),
            Self::GoToJob(owner) => Self::GoToJob(owner.fork_for_main_base_abort_transaction()),
            Self::RunAway(owner) => Self::RunAway(owner.fork_for_main_base_abort_transaction()),
            Self::AttractAttention(publication) => {
                Self::AttractAttention(publication.fork_for_main_base_abort_transaction())
            }
            Self::InitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => Self::InitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected_runtime: *selected_runtime,
                initial_behavior: *initial_behavior,
                sub_a: *sub_a,
                state_policy_mask: *state_policy_mask,
                state_policy_bits: *state_policy_bits,
            },
            Self::GoToJobInitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => Self::GoToJobInitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected_runtime: *selected_runtime,
                initial_behavior: *initial_behavior,
                actor_common_axis: *actor_common_axis,
                sub_a: *sub_a,
                state_policy_mask: *state_policy_mask,
                state_policy_bits: *state_policy_bits,
            },
            Self::RunAwayInitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => Self::RunAwayInitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected_runtime: *selected_runtime,
                initial_behavior: *initial_behavior,
                actor_common_axis: *actor_common_axis,
                sub_a: *sub_a,
                state_policy_mask: *state_policy_mask,
                state_policy_bits: *state_policy_bits,
            },
            Self::AttractAttentionInitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                actor_common_axis,
                sub_a,
                actor_animation,
                state_policy_mask,
                state_policy_bits,
            } => Self::AttractAttentionInitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected_runtime: *selected_runtime,
                initial_behavior: *initial_behavior,
                actor_common_axis: *actor_common_axis,
                sub_a: *sub_a,
                actor_animation: *actor_animation,
                state_policy_mask: *state_policy_mask,
                state_policy_bits: *state_policy_bits,
            },
        }
    }

    fn authenticates(&self, entity: &Entity) -> bool {
        match self {
            Self::Wander(owner) => owner.authenticates_publication(entity),
            Self::GoToJob(owner) => owner.authenticates_publication(entity),
            Self::RunAway(owner) => owner.authenticates_publication(entity),
            Self::AttractAttention(publication) => publication.authenticates_publication(entity),
            Self::InitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => {
                entity.id == *entity_id
                    && entity.active
                    && entity.initial_behavior == *initial_behavior
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(*context))
                    && entity.ordinary_type9_selected_component_runtime == Some(*selected_runtime)
                    && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(*sub_a))
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(*state_policy_mask)
                        == RetailRuntimeValue::Known(*state_policy_bits)
                    && entity.ordinary_type9_pending_initial_selection.is_none()
                    && entity.main_base_type9_death_component_runtime.is_none()
                    && selected_runtime.kind()
                        == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none())
            }
            Self::GoToJobInitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => {
                entity.id == *entity_id
                    && entity.active
                    && entity.initial_behavior == *initial_behavior
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(*context))
                    && entity.ordinary_type9_selected_component_runtime == Some(*selected_runtime)
                    && entity.actor_common_axis_descriptor
                        == RetailRuntimeValue::Known(*actor_common_axis)
                    && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(*sub_a))
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(*state_policy_mask)
                        == RetailRuntimeValue::Known(*state_policy_bits)
                    && entity.ordinary_type9_pending_initial_selection.is_none()
                    && entity.main_base_type9_death_component_runtime.is_none()
                    && selected_runtime.kind()
                        == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none())
            }
            Self::RunAwayInitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => {
                entity.id == *entity_id
                    && entity.active
                    && entity.initial_behavior == *initial_behavior
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(*context))
                    && entity.ordinary_type9_selected_component_runtime == Some(*selected_runtime)
                    && entity.actor_common_axis_descriptor
                        == RetailRuntimeValue::Known(*actor_common_axis)
                    && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(*sub_a))
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(*state_policy_mask)
                        == RetailRuntimeValue::Known(*state_policy_bits)
                    && entity.ordinary_type9_pending_initial_selection.is_none()
                    && entity.main_base_type9_death_component_runtime.is_none()
                    && selected_runtime.kind()
                        == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none())
            }
            Self::AttractAttentionInitializerFallback {
                entity_id,
                context,
                selected_runtime,
                initial_behavior,
                actor_common_axis,
                sub_a,
                actor_animation,
                state_policy_mask,
                state_policy_bits,
            } => {
                entity.id == *entity_id
                    && entity.active
                    && entity.initial_behavior == *initial_behavior
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(*context))
                    && entity.ordinary_type9_selected_component_runtime == Some(*selected_runtime)
                    && entity.actor_common_axis_descriptor
                        == RetailRuntimeValue::Known(*actor_common_axis)
                    && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(*sub_a))
                    && entity.actor_animation_runtime
                        == RetailRuntimeValue::Known(Some(*actor_animation))
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(*state_policy_mask)
                        == RetailRuntimeValue::Known(*state_policy_bits)
                    && entity.ordinary_type9_pending_initial_selection.is_none()
                    && entity.main_base_type9_death_component_runtime.is_none()
                    && selected_runtime.kind()
                        == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none())
            }
        }
    }
}

impl OrdinaryType9GoToJobProductionOwner {
    pub(crate) fn adopt(
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
        actor_lease: MainBaseAbortActorLease,
    ) -> Result<Self, FreshLevel1Type9InitialProductionOwner> {
        if initial_owner.successful_go_to_job_selection().is_none()
            || initial_owner.entity_id() != actor_lease.entity_id
        {
            return Err(initial_owner);
        }
        Ok(Self {
            original_manager_sidecar_index: Some(original_manager_sidecar_index),
            task_authority: OrdinaryType9ProductionAuthority::Initial(initial_owner),
            actor_lease,
            state: OrdinaryType9GoToJobProductionState::Active,
            next_transaction_id: 1,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_actor_common_axis: None,
            pending_root_sub_a: None,
            pending_root_actor_animation: None,
            pending_root_dispatcher_continuation: None,
            root_publication: None,
            root_attract_attention_initial_owners: None,
            root_attract_attention_target_route: None,
            outer_tail: None,
            pending_outer_outcome: None,
        })
    }

    pub(crate) fn from_current_task(
        authority: OrdinaryType9CurrentTaskAuthority,
        actor_lease: MainBaseAbortActorLease,
        next_transaction_id: u64,
    ) -> Self {
        debug_assert_eq!(authority.entity_id(), actor_lease.entity_id);
        Self {
            original_manager_sidecar_index: None,
            task_authority: OrdinaryType9ProductionAuthority::Current(authority),
            actor_lease,
            state: OrdinaryType9GoToJobProductionState::Active,
            next_transaction_id,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_actor_common_axis: None,
            pending_root_sub_a: None,
            pending_root_actor_animation: None,
            pending_root_dispatcher_continuation: None,
            root_publication: None,
            root_attract_attention_initial_owners: None,
            root_attract_attention_target_route: None,
            outer_tail: None,
            pending_outer_outcome: None,
        }
    }

    /// Move a completed root publication to the callback family that it
    /// actually installed. No prefix, constructor or shared RNG is replayed.
    pub(crate) fn take_completed_current_task(
        &mut self,
        manager: &EntityManager,
    ) -> Option<(OrdinaryType9CurrentTaskAuthority, u64)> {
        let tail = self.outer_tail.as_ref()?;
        if !matches!(
            self.state,
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ) || !matches!(tail, OrdinaryType9OuterTailCustody::Complete { .. })
            || self.pending_root_plan.is_some()
            || self.pending_outer_outcome.is_some()
            || !outer_tail_observation_authenticates(manager, self.actor_lease, tail)
        {
            return None;
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == self.entity_id())?;
        if let Some(route) = self.root_attract_attention_target_route.as_ref() {
            if route.validate(entity).is_err() {
                return None;
            }
            let route = self
                .root_attract_attention_target_route
                .take()
                .expect("the completed target route was authenticated");
            return Some((
                OrdinaryType9CurrentTaskAuthority::from_publication(
                    entity,
                    OrdinaryType9CurrentTaskPublication::AttractAttentionTargetRoute(route),
                )
                .expect("the target route was authenticated before custody transfer"),
                self.next_transaction_id,
            ));
        }
        let publication = self.root_publication.as_ref()?;
        if !matches!(
            publication,
            OrdinaryType9GoToJobRootPublication::Wander(..)
                | OrdinaryType9GoToJobRootPublication::GoToJob(..)
                | OrdinaryType9GoToJobRootPublication::RunAway(..)
        ) || !publication.authenticates(entity)
        {
            return None;
        }
        let publication = match self
            .root_publication
            .take()
            .expect("verified current publication")
        {
            OrdinaryType9GoToJobRootPublication::Wander(receipt) => {
                OrdinaryType9CurrentTaskPublication::Wander(receipt)
            }
            OrdinaryType9GoToJobRootPublication::GoToJob(receipt) => {
                OrdinaryType9CurrentTaskPublication::GoToJob(receipt)
            }
            OrdinaryType9GoToJobRootPublication::RunAway(receipt) => {
                OrdinaryType9CurrentTaskPublication::RunAway(receipt)
            }
            _ => unreachable!("the supported publication family was checked before taking custody"),
        };
        Some((
            OrdinaryType9CurrentTaskAuthority::from_publication(entity, publication)
                .expect("the exact root publication was authenticated before custody transfer"),
            self.next_transaction_id,
        ))
    }

    /// A fully preflighted radial pass owns exactly this impending velocity
    /// write. Keep task clocks, current family and every other tail field exact.
    pub(crate) fn acknowledge_planned_radial_velocity(
        &mut self,
        manager: &EntityManager,
        velocity_raw: [i16; 3],
    ) -> bool {
        if self.completed_visit_lease(manager) != Some(self.actor_lease) {
            return false;
        }
        if let Some(OrdinaryType9OuterTailCustody::Complete { expected_state, .. }) =
            &mut self.outer_tail
        {
            expected_state.velocity_raw = velocity_raw;
        }
        true
    }

    /// Presentation and relation callbacks enter only between completed actor visits.
    /// A newly adopted root graph is also idle before its first prefix.
    pub(crate) fn completed_visit_lease(
        &self,
        manager: &EntityManager,
    ) -> Option<MainBaseAbortActorLease> {
        let complete = match (&self.state, &self.outer_tail) {
            (OrdinaryType9GoToJobProductionState::Active, None) => true,
            (
                OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. },
                Some(tail @ OrdinaryType9OuterTailCustody::Complete { .. }),
            ) => outer_tail_observation_authenticates(manager, self.actor_lease, tail),
            _ => false,
        };
        if !complete || self.pending_root_plan.is_some() || self.pending_outer_outcome.is_some() {
            return None;
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == self.entity_id())?;
        (post_task_owner_authority_authenticates(self, entity)
            && manager.ordinary_type9_selected_actor_lease(self.entity_id())
                == Some(self.actor_lease))
        .then_some(self.actor_lease)
    }

    /// Consume an authenticated completed visit before a native Type9 hit.
    /// Task clocks and the next transaction identity survive this transfer.
    pub(crate) fn into_completed_native_hit(
        mut self,
        manager: &EntityManager,
    ) -> Result<(OrdinaryType9CurrentTaskAuthority, u64), Self> {
        if self.completed_visit_lease(manager) != Some(self.actor_lease) {
            return Err(self);
        }
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == self.entity_id())
            .unwrap();
        use crate::ordinary_type9_current_task::NativeType9TaskContinuation;
        let continuation = if let Some(route) = self.root_attract_attention_target_route.take() {
            NativeType9TaskContinuation::TargetRoute(route)
        } else if let Some(owners) = self.root_attract_attention_initial_owners.take() {
            NativeType9TaskContinuation::AttractAttention(owners)
        } else {
            NativeType9TaskContinuation::Ordinary
        };
        let authority = match OrdinaryType9CurrentTaskAuthority::from_completed_native_visit(
            entity,
            continuation,
        ) {
            Ok(authority) => authority,
            Err(continuation) => {
                match continuation {
                    NativeType9TaskContinuation::Ordinary => {}
                    NativeType9TaskContinuation::AttractAttention(owners) => {
                        self.root_attract_attention_initial_owners = Some(owners)
                    }
                    NativeType9TaskContinuation::TargetRoute(route) => {
                        self.root_attract_attention_target_route = Some(route)
                    }
                }
                return Err(self);
            }
        };
        Ok((authority, self.next_transaction_id))
    }
    pub const fn entity_id(&self) -> u32 {
        self.actor_lease.entity_id
    }

    /// Acknowledge the classifier write immediately after the presenter has
    /// authenticated `completed_visit_lease`. This does not admit a new task
    /// phase or relax a pending simulation transaction.
    pub(crate) fn acknowledge_presented_view_detail(
        &mut self,
        entity: &Entity,
        previous_state: crate::entity_collision_state::RetailStateWord,
    ) -> bool {
        match self.root_publication.as_mut() {
            Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) => {
                publication.acknowledge_presented_view_detail(entity, previous_state)
            }
            _ => true,
        }
    }

    pub const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.actor_lease
    }

    pub const fn state(&self) -> OrdinaryType9GoToJobProductionState {
        self.state
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            original_manager_sidecar_index: self.original_manager_sidecar_index,
            task_authority: self.task_authority.fork_for_main_base_abort_transaction(),
            actor_lease: self.actor_lease,
            state: self.state,
            next_transaction_id: self.next_transaction_id,
            pending_root_plan: self
                .pending_root_plan
                .as_ref()
                .map(OrdinaryType9RootReselectionPlan::fork_for_main_base_abort_transaction),
            pending_root_task_visits: self.pending_root_task_visits,
            pending_root_actor_common_axis: self.pending_root_actor_common_axis,
            pending_root_sub_a: self.pending_root_sub_a,
            pending_root_actor_animation: self.pending_root_actor_animation,
            pending_root_dispatcher_continuation: self.pending_root_dispatcher_continuation.clone(),
            root_publication: self
                .root_publication
                .as_ref()
                .map(OrdinaryType9GoToJobRootPublication::fork_for_main_base_abort_transaction),
            root_attract_attention_initial_owners: self
                .root_attract_attention_initial_owners
                .as_ref()
                .map(fork_attract_attention_initial_owners),
            root_attract_attention_target_route: self
                .root_attract_attention_target_route
                .as_ref()
                .map(
                OrdinaryType9AttractAttentionTargetRouteOwner::fork_for_main_base_abort_transaction,
            ),
            outer_tail: self
                .outer_tail
                .as_ref()
                .map(OrdinaryType9OuterTailCustody::fork_for_main_base_abort_transaction),
            pending_outer_outcome: self.pending_outer_outcome.clone(),
        }
    }

    pub(crate) const fn main_base_abort_compatible(&self) -> bool {
        self.task_authority.is_initial()
            && self.pending_root_plan.is_none()
            && self.pending_root_task_visits.is_none()
            && self.pending_root_actor_common_axis.is_none()
            && self.pending_root_sub_a.is_none()
            && self.pending_root_actor_animation.is_none()
            && self.pending_root_dispatcher_continuation.is_none()
            && self.root_publication.is_none()
            && self.root_attract_attention_initial_owners.is_none()
            && self.root_attract_attention_target_route.is_none()
            && self.outer_tail.is_none()
            && self.pending_outer_outcome.is_none()
            && !matches!(
                self.state,
                OrdinaryType9GoToJobProductionState::RootTransitionPending { .. }
                    | OrdinaryType9GoToJobProductionState::CallbackFailurePending
                    | OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
            )
    }

    pub(crate) fn decompose_for_main_base_abort(
        self,
    ) -> (
        usize,
        FreshLevel1Type9InitialProductionOwner,
        OrdinaryType9GoToJobProductionResume,
    ) {
        (
            self.original_manager_sidecar_index
                .expect("initial manager custody retains its sidecar index"),
            self.task_authority.into_initial(),
            OrdinaryType9GoToJobProductionResume {
                actor_lease: self.actor_lease,
                state: self.state,
                next_transaction_id: self.next_transaction_id,
            },
        )
    }

    pub(crate) fn resume_after_main_base_abort_noop(
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
        resume: OrdinaryType9GoToJobProductionResume,
    ) -> Result<Self, FreshLevel1Type9InitialProductionOwner> {
        if initial_owner.entity_id() != resume.actor_lease.entity_id
            || initial_owner.successful_go_to_job_selection().is_none()
        {
            return Err(initial_owner);
        }
        Ok(Self {
            original_manager_sidecar_index: Some(original_manager_sidecar_index),
            task_authority: OrdinaryType9ProductionAuthority::Initial(initial_owner),
            actor_lease: resume.actor_lease,
            state: resume.state,
            next_transaction_id: resume.next_transaction_id,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_actor_common_axis: None,
            pending_root_sub_a: None,
            pending_root_actor_animation: None,
            pending_root_dispatcher_continuation: None,
            root_publication: None,
            root_attract_attention_initial_owners: None,
            root_attract_attention_target_route: None,
            outer_tail: None,
            pending_outer_outcome: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrdinaryType9GoToJobProductionResume {
    actor_lease: MainBaseAbortActorLease,
    state: OrdinaryType9GoToJobProductionState,
    next_transaction_id: u64,
}

pub struct OrdinaryType9GoToJobProductionFrame<'a> {
    pub dispatch_resource_text:
        &'a dyn Fn(crate::attract_attention::AttractAttentionResourceTextRequest, u32),
    pub resources: &'a ResourceCache,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobProductionDrop {
    EntityUnavailable,
    ActorLeaseChanged,
    InitialOwnerMismatch,
    RootPublicationMismatch,
    PostTaskFrameMismatch,
    PostBasisPublicationMismatch,
    OuterTailStateMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobProductionBlock {
    PostBasisTailPending,
    RootTransitionPending,
    CallbackFailurePending,
    RootTransitionRequestChanged,
    RootTaskVisitsUnavailable,
    RootDispatcherContinuationUnavailable,
    RootDispatcherContinuationMismatch,
    RootDispatcherContinuationProducedTransition,
    RootPredecessorActorCommonAxisUnavailable,
    RootPredecessorActorCommonAxisChanged {
        expected: CommonAxisDescriptor,
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    },
    RootPredecessorSubAUnavailable,
    RootPredecessorSubAChanged {
        expected: SubAPropulsionRuntime,
        actual: SubAPropulsionRuntime,
    },
    ActiveModelUnavailable,
    RootReselection(OrdinaryType9RootReselectionError),
    UnsupportedRootSelection {
        class_id: u32,
    },
    RootWanderPreflight(OrdinaryType9RootWanderPreflightError),
    RootGoToJobPreflight(OrdinaryType9RootGoToJobPreflightError),
    RootRunAwayPreflight(OrdinaryType9RootRunAwayPreflightError),
    RootAttractAttentionPreflight(OrdinaryType9RootAttractAttentionPreflightError),
    RootPredecessorActorAnimationUnavailable,
    RootPredecessorActorAnimationChanged {
        expected: ActorAnimationController,
        actual: RetailRuntimeValue<Option<ActorAnimationController>>,
    },
    AuthoredRunAwayAudioUnresolved,
    AuthoredRunAwayAudioMismatch {
        actual_sound_id: Option<u16>,
        actual_period_raw: u32,
    },
    CurrentTerrainUnavailable,
    RuntimeMetadataUnavailable,
    RuntimeTopologyUnavailable,
    ActorCommonAxisDescriptorUnavailable,
    ActorCommonAxisDescriptorMismatch {
        expected: CommonAxisDescriptor,
        actual: CommonAxisDescriptor,
    },
    SelectedComponentRuntimeUnavailable,
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationUnavailable,
    PhysicalBodyBasisUnavailable,
    BehaviorContextUnavailable,
    RemoteSchedulerOwnerUnsupported,
    SchedulerCallbackDisabled,
    NormalSchedulerOwnerStateUnresolved,
    CallbackMassUnavailable,
    SchedulerModeUnresolved,
    SchedulerStateUnresolved,
    OuterOwnerStateUnavailable,
    OuterOwner(crate::common_mover::type9_owner::OrdinaryType9OwnerBlock),
    OuterLifecyclePending,
    Dispatcher(ActorTaskDispatcherError<OrdinaryType9GoToJobAdapterError>),
    TargetRouteTransitionPending,
    TargetRouteWrapperRetired,
    TargetRoute(
        OrdinaryType9AttractAttentionTargetRouteTickError<
            OrdinaryType9GoToJobAdapterError,
            OrdinaryType9GoToJobAdapterError,
            OrdinaryType9GoToJobAdapterError,
        >,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobProductionOutcome {
    SchedulerWaiting {
        entity_id: u32,
    },
    PostBasisTailPending {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    RootWanderPublished {
        entity_id: u32,
        constructor: OrdinaryType9WanderConstructorEvidence,
    },
    RootInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9WanderInitializerFailure,
    },
    RootGoToJobPublished {
        entity_id: u32,
        target_id: u32,
        constructor: OrdinaryType9GoToJobConstructorEvidence,
    },
    RootGoToJobInitializerFallbackPublished {
        entity_id: u32,
        target_id: u32,
        failure: OrdinaryType9GoToJobInitializerFailure,
    },
    RootRunAwayPublished {
        entity_id: u32,
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
    },
    RootRunAwayInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
    },
    RootAttractAttentionPublished {
        entity_id: u32,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
    },
    RootAttractAttentionInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9AttractAttentionInitializerFailure,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
    },
    RootAttractAttentionTargetRoutePublished {
        entity_id: u32,
        target_id: u32,
        constructor: AttractAttentionTargetExecutionOutcome,
    },
    RootAttractAttentionTargetInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
    },
    SurfaceLifecycleClass14Published {
        entity_id: u32,
        task_lease: Option<crate::main_base_type9_abort::MainBaseType9ExplodingTaskLease>,
    },
    Blocked {
        entity_id: u32,
        reason: OrdinaryType9GoToJobProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: OrdinaryType9GoToJobProductionDrop,
    },
}

impl OrdinaryType9GoToJobProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::SchedulerWaiting { entity_id }
            | Self::PostBasisTailPending { entity_id, .. }
            | Self::RootWanderPublished { entity_id, .. }
            | Self::RootInitializerFallbackPublished { entity_id, .. }
            | Self::RootGoToJobPublished { entity_id, .. }
            | Self::RootGoToJobInitializerFallbackPublished { entity_id, .. }
            | Self::RootRunAwayPublished { entity_id, .. }
            | Self::RootRunAwayInitializerFallbackPublished { entity_id, .. }
            | Self::RootAttractAttentionPublished { entity_id, .. }
            | Self::RootAttractAttentionInitializerFallbackPublished { entity_id, .. }
            | Self::RootAttractAttentionTargetRoutePublished { entity_id, .. }
            | Self::RootAttractAttentionTargetInitializerFallbackPublished { entity_id, .. }
            | Self::SurfaceLifecycleClass14Published { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9GoToJobOwnerTick {
    pub outcome: OrdinaryType9GoToJobProductionOutcome,
    pub retained_owner: Option<OrdinaryType9GoToJobProductionOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobTargetError {
    TargetStateUnresolved { target_id: u32 },
    TargetPositionUnavailable { target_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9GoToJobAdapterError {
    Target(OrdinaryType9GoToJobTargetError),
    TargetAcquisition(TargetAcquisitionCallbackError),
    AttractAttentionCandidate(OrdinaryType9AttractAttentionCandidateTickError),
    AttractAttentionCandidateLeaseChanged,
    AttractAttentionCueLeaseChanged,
    RuntimeMetadataUnavailable,
    RuntimeTopologyUnavailable,
    CurrentTerrainUnavailable,
    ActorCommonAxisDescriptorUnavailable,
    ActorCommonAxisDescriptorMismatch {
        expected: CommonAxisDescriptor,
        actual: CommonAxisDescriptor,
    },
    SelectedComponentRuntimeUnavailable,
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationUnavailable,
    PhysicalBodyBasisUnavailable,
    TransitionGateUnresolved,
    CommonMoverFrameBlocked(crate::common_mover::type9::OrdinaryType9FrameBlock),
    CommonMoverCommitBlocked(crate::common_mover::type9_transaction::OrdinaryType9ExternalBlock),
    TargetStateUnresolved {
        target_id: u32,
    },
    TargetPositionUnavailable {
        target_id: u32,
    },
}

fn preflight_normal_scheduler_owner(
    collision: &EntityCollisionRuntimeState,
) -> Result<(), OrdinaryType9GoToJobProductionBlock> {
    match collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) if bits & REMOTE_OWNED_STATE_BIT != 0 => {
            Err(OrdinaryType9GoToJobProductionBlock::RemoteSchedulerOwnerUnsupported)
        }
        RetailRuntimeValue::Known(bits)
            if bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 =>
        {
            Err(OrdinaryType9GoToJobProductionBlock::SchedulerCallbackDisabled)
        }
        RetailRuntimeValue::Known(_) => Ok(()),
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9GoToJobProductionBlock::NormalSchedulerOwnerStateUnresolved)
        }
    }
}

fn authored_run_away_audio(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<RunAwayAuthoredAudio, OrdinaryType9GoToJobProductionBlock> {
    match (
        metadata.run_away_optional_sound_id,
        metadata.run_away_sound_period_raw,
    ) {
        (
            RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID)),
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW),
        ) => Ok(RunAwayAuthoredAudio {
            sound_id: LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID,
            period_raw: LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW,
        }),
        (
            RetailRuntimeValue::Known(actual_sound_id),
            RetailRuntimeValue::Known(actual_period_raw),
        ) => Err(
            OrdinaryType9GoToJobProductionBlock::AuthoredRunAwayAudioMismatch {
                actual_sound_id,
                actual_period_raw,
            },
        ),
        _ => Err(OrdinaryType9GoToJobProductionBlock::AuthoredRunAwayAudioUnresolved),
    }
}

enum GoToJobAnimationCustody<'a> {
    Live(&'a mut ActorAnimationController),
    Skipped { scratch: ActorAnimationController },
}

impl GoToJobAnimationCustody<'_> {
    fn runtime_mut(&mut self) -> &mut ActorAnimationController {
        match self {
            Self::Live(runtime) => runtime,
            Self::Skipped { scratch } => scratch,
        }
    }
}

fn bind_go_to_job_animation(
    runtime: &mut RetailRuntimeValue<Option<ActorAnimationController>>,
    scheduler_mode: i32,
) -> Result<GoToJobAnimationCustody<'_>, OrdinaryType9GoToJobAdapterError> {
    if scheduler_mode != 0 {
        return Ok(GoToJobAnimationCustody::Skipped {
            scratch: ActorAnimationController::from_descriptor(LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR)
                .expect("the exact Level-1 Type-9 descriptor has a valid animation binding"),
        });
    }
    match runtime {
        RetailRuntimeValue::Known(Some(runtime)) => Ok(GoToJobAnimationCustody::Live(runtime)),
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9GoToJobAdapterError::ActorAnimationUnavailable)
        }
    }
}

fn read_transition_suppressed(
    collision: &EntityCollisionRuntimeState,
) -> Result<bool, OrdinaryType9GoToJobAdapterError> {
    match collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => Ok(false),
        RetailRuntimeValue::Known(_) => Ok(true),
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9GoToJobAdapterError::TransitionGateUnresolved)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EntitySnapshot {
    id: u32,
    entity_type: u32,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    capability_flags: u32,
    active: bool,
    collision: EntityCollisionRuntimeState,
    job_capacity: RetailRuntimeValue<Option<JobCapacityState>>,
}

/// Exact unvisited suffix of one already-entered A800 traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OrdinaryType9GoToJobDispatcherContinuation {
    entity_id: u32,
    snapshots: Vec<EntitySnapshot>,
    root_facts: OrdinaryType9RootWanderEntityFacts,
    root_owner_snapshot: OrdinaryType9RootEntityRef,
    actor_common_axis_at_transition: RetailRuntimeValue<CommonAxisDescriptor>,
    next_slot: Option<ActorTaskSlot>,
    callback_elapsed_micros: u32,
    global_elapsed_micros: u32,
    scheduler_mode: i32,
}

impl OrdinaryType9GoToJobDispatcherContinuation {
    fn authenticates(
        &self,
        entity_id: u32,
        post_task_frame: OrdinaryType9GoToJobPostTaskFrame,
    ) -> bool {
        self.authenticates_cursor(entity_id, post_task_frame)
            && self.next_slot == Some(ActorTaskSlot::Secondary)
    }

    fn authenticates_attract_origin(
        &self,
        entity_id: u32,
        post_task_frame: OrdinaryType9GoToJobPostTaskFrame,
    ) -> bool {
        self.authenticates_cursor(entity_id, post_task_frame)
            && matches!(self.next_slot, None | Some(ActorTaskSlot::Secondary))
    }

    fn authenticates_cursor(
        &self,
        entity_id: u32,
        post_task_frame: OrdinaryType9GoToJobPostTaskFrame,
    ) -> bool {
        self.entity_id == entity_id
            && self.root_facts.entity_id == entity_id
            && self.root_owner_snapshot.id == entity_id
            && self.callback_elapsed_micros == post_task_frame.callback_elapsed_micros()
            && matches!(self.scheduler_mode, 0 | 1)
            && self
                .snapshots
                .iter()
                .filter(|snapshot| snapshot.id == entity_id)
                .count()
                == 1
            && self
                .snapshots
                .iter()
                .find(|snapshot| snapshot.id == entity_id)
                .is_some_and(|snapshot| {
                    root_entity_ref_from_snapshot(snapshot) == self.root_owner_snapshot
                })
    }
}

fn job_capacity_from_entity(entity: &Entity) -> RetailRuntimeValue<Option<JobCapacityState>> {
    match entity.base_factory_runtime {
        RetailRuntimeValue::Known(Some(state)) => {
            RetailRuntimeValue::Known(Some(JobCapacityState {
                current_jobs_raw: i32::from(state.current_scientists),
                capacity_raw: i32::from(state.required_scientists),
            }))
        }
        RetailRuntimeValue::Known(None) => RetailRuntimeValue::Known(None),
        RetailRuntimeValue::Unresolved => RetailRuntimeValue::Unresolved,
    }
}

fn go_to_job_candidate_evidence(
    snapshots: &[EntitySnapshot],
) -> Vec<OrdinaryType9GoToJobCandidateEvidence> {
    snapshots
        .iter()
        .map(|snapshot| OrdinaryType9GoToJobCandidateEvidence {
            candidate_id: snapshot.id,
            state_flags: snapshot.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(snapshot.capability_flags),
            capacity: snapshot.job_capacity,
        })
        .collect()
}

fn refresh_controlled_snapshot(
    snapshots: &mut [EntitySnapshot],
    entity_id: u32,
    position_raw: [i16; 3],
    velocity_raw: [i16; 3],
    capability_flags: u32,
    active: bool,
    collision: &EntityCollisionRuntimeState,
) {
    if let Some(snapshot) = snapshots
        .iter_mut()
        .find(|snapshot| snapshot.id == entity_id)
    {
        snapshot.position_raw = position_raw;
        snapshot.velocity_raw = velocity_raw;
        snapshot.capability_flags = capability_flags;
        snapshot.active = active;
        snapshot.collision = collision.clone();
    }
}

fn root_entity_ref(entity: &Entity) -> OrdinaryType9RootEntityRef {
    OrdinaryType9RootEntityRef {
        id: entity.id,
        entity_type: entity.entity_type,
        position_raw: entity.position_raw(),
        state_flags_raw: entity.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
        attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
    }
}

fn attract_attention_committed_effects(
    completion: &OrdinaryType9GoToJobRootCompletion,
) -> Option<OrdinaryType9AttractAttentionCommittedEffects> {
    match completion {
        OrdinaryType9GoToJobRootCompletion::AttractAttention { committed, .. }
        | OrdinaryType9GoToJobRootCompletion::AttractAttentionInitializerFallback {
            committed,
            ..
        } => Some(*committed),
        _ => None,
    }
}

fn fold_attract_attention_candidate_completion(
    entity_id: u32,
    completion: OrdinaryType9GoToJobAttractAttentionCandidateCompletion,
) -> (
    Option<OrdinaryType9GoToJobRootPublication>,
    OrdinaryType9GoToJobProductionOutcome,
) {
    match completion {
        OrdinaryType9GoToJobAttractAttentionCandidateCompletion::TargetRoute {
            target_id,
            constructor,
        } => (
            None,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished {
                entity_id,
                target_id,
                constructor,
            },
        ),
        OrdinaryType9GoToJobAttractAttentionCandidateCompletion::InitializerFallback {
            failure,
            publication,
        } => (
            Some(publication),
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetInitializerFallbackPublished {
                entity_id,
                failure,
            },
        ),
    }
}

fn guard_location_entity_ref_from_snapshot(snapshot: &EntitySnapshot) -> GuardLocationEntityRef {
    GuardLocationEntityRef {
        id: snapshot.id,
        entity_type: snapshot.entity_type,
        position_raw: snapshot.position_raw,
        state_flags_raw: snapshot.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(snapshot.capability_flags),
        attached_entity_handle: snapshot.collision.recent_relation_id_at_0x60,
    }
}

fn root_entity_ref_from_snapshot(snapshot: &EntitySnapshot) -> OrdinaryType9RootEntityRef {
    OrdinaryType9RootEntityRef {
        id: snapshot.id,
        entity_type: snapshot.entity_type,
        position_raw: snapshot.position_raw,
        state_flags_raw: snapshot.collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(snapshot.capability_flags),
        attached_entity_handle: snapshot.collision.recent_relation_id_at_0x60,
    }
}

fn root_entity_ref_from_parts(
    facts: OrdinaryType9RootWanderEntityFacts,
    collision: &EntityCollisionRuntimeState,
) -> OrdinaryType9RootEntityRef {
    OrdinaryType9RootEntityRef {
        id: facts.entity_id,
        entity_type: facts.entity_type,
        position_raw: facts.position_raw,
        state_flags_raw: collision.state_flags_at_0x08,
        capability_flags: RetailRuntimeValue::Known(facts.capability_flags),
        attached_entity_handle: collision.recent_relation_id_at_0x60,
    }
}

fn live_task_visits(owner: &ActorTaskOwner<ActorTaskRuntime>) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        owner
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}

fn authenticates_root_transition_parts(
    entity_id: u32,
    pending_initial_selection_present: bool,
    death_component_custody_present: bool,
    owner: &ActorTaskOwner<ActorTaskRuntime>,
    selected_runtime: Option<OrdinaryType9SelectedComponentRuntime>,
    context: RetailRuntimeValue<Option<BehaviorContextRuntime>>,
    transition: OrdinaryType9GoToJobRootTransition,
) -> bool {
    let Some(program) = behavior_program(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID) else {
        return false;
    };
    let RetailRuntimeValue::Known(Some(context)) = context else {
        return false;
    };
    let visit = ActorTaskVisit {
        slot: transition.request.slot,
        task_id: transition.request.task_id,
    };
    let expected_lifetime_status =
        if transition.task_state.elapsed_ms() > GO_TO_JOB_TASK_LIFETIME_MS {
            GoToJobLifetimeStatus::OwnerTransitionDue
        } else {
            GoToJobLifetimeStatus::WithinLifetime
        };
    entity_id == transition.entity_id
        && context.descriptor() == BehaviorDescriptorIdentity::Named(program)
        && context.choice_list_source()
            == RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        && context.active_style() == ActiveBehaviorStyle::Audited(program.initial_style)
        && context.style_table_index_raw_at_0x10() == program.initial_style_table_index_raw
        // Root reselection preserves the existing target and auxiliary words.
        // Their exact identity belongs to the retained scheduler authority.
        && !pending_initial_selection_present
        && !death_component_custody_present
        && transition.request.slot == ActorTaskSlot::Primary
        && owner.task_in_slot(ActorTaskSlot::Primary) == Some(transition.request.task_id)
        && owner.wrapper_flags(transition.request.task_id)
            == Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: false,
            })
        && matches!(
            owner.task_state(visit.task_id),
            Some(ActorTaskRuntime::GoToJob(live)) if *live == transition.task_state
        )
        && transition.request.committed_prefix.lifetime_status == expected_lifetime_status
        && match transition.request.reason {
            GoToJobTransitionReason::TaggedCallbackResult(_) => true,
            GoToJobTransitionReason::LifetimeExpired => {
                expected_lifetime_status == GoToJobLifetimeStatus::OwnerTransitionDue
            }
        }
        && selected_runtime == Some(transition.selected_runtime)
        && transition.selected_runtime.kind() == OrdinaryType9SelectedRuntimeKind::GoToJobPublished
        && owner.state_in_slot(ActorTaskSlot::Secondary).is_none()
        && owner.state_in_slot(ActorTaskSlot::Tertiary).is_none()
}

fn authenticates_root_transition(
    entity: &Entity,
    transition: OrdinaryType9GoToJobRootTransition,
) -> bool {
    authenticates_root_transition_parts(
        entity.id,
        entity.ordinary_type9_pending_initial_selection.is_some(),
        entity.main_base_type9_death_component_runtime.is_some(),
        &entity.actor_tasks,
        entity.ordinary_type9_selected_component_runtime,
        entity.current_behavior_context,
        transition,
    )
}

fn post_task_owner_authority_authenticates(
    owner: &OrdinaryType9GoToJobProductionOwner,
    entity: &Entity,
) -> bool {
    if let Some(target_route) = owner.root_attract_attention_target_route.as_ref() {
        return target_route.validate(entity).is_ok();
    }
    owner.root_publication.as_ref().map_or_else(
        || owner.task_authority.authenticates_retained_entity(entity),
        |publication| publication.authenticates(entity),
    )
}

fn post_task_basis_input_authenticates(
    frame: OrdinaryType9GoToJobPostTaskFrame,
    entity: &Entity,
) -> bool {
    entity.rotation_heading_pitch_roll_raw() == frame.post_task_angles_raw
        && entity.physical_body_basis_q31 == frame.pre_basis
}

fn post_basis_publication_authenticates(
    owner: &OrdinaryType9GoToJobProductionOwner,
    frame: OrdinaryType9GoToJobPostTaskFrame,
    entity: &Entity,
) -> bool {
    post_task_owner_authority_authenticates(owner, entity)
        && entity.rotation_heading_pitch_roll_raw() == frame.post_task_angles_raw
        && entity.physical_body_basis_q31 == RetailRuntimeValue::Known(frame.published_body_basis())
        && entity
            .collision
            .state_flags_at_0x08
            .masked(BODY_BASIS_REBUILT_STATE_BIT)
            == RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
}

fn post_task_authority_mismatch(
    owner: &OrdinaryType9GoToJobProductionOwner,
) -> OrdinaryType9GoToJobProductionDrop {
    if !owner.task_authority.is_initial()
        || owner.root_publication.is_some()
        || owner.root_attract_attention_target_route.is_some()
    {
        OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch
    } else {
        OrdinaryType9GoToJobProductionDrop::InitialOwnerMismatch
    }
}

fn map_outer_tail_block(
    reason: OrdinaryType9OuterTailBlock,
) -> OrdinaryType9GoToJobProductionBlock {
    match reason {
        OrdinaryType9OuterTailBlock::CurrentTerrainUnavailable => {
            OrdinaryType9GoToJobProductionBlock::CurrentTerrainUnavailable
        }
        OrdinaryType9OuterTailBlock::ActiveModelUnavailable => {
            OrdinaryType9GoToJobProductionBlock::ActiveModelUnavailable
        }
        OrdinaryType9OuterTailBlock::OuterOwnerStateUnavailable => {
            OrdinaryType9GoToJobProductionBlock::OuterOwnerStateUnavailable
        }
        OrdinaryType9OuterTailBlock::OuterOwner(block) => {
            OrdinaryType9GoToJobProductionBlock::OuterOwner(block)
        }
        OrdinaryType9OuterTailBlock::OuterLifecyclePending => {
            OrdinaryType9GoToJobProductionBlock::OuterLifecyclePending
        }
    }
}

fn refresh_retained_go_to_job_publication(
    owner: &mut OrdinaryType9GoToJobProductionOwner,
    entity: &Entity,
) -> bool {
    match owner.root_publication.as_mut() {
        Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) => {
            publication.refresh_after_owned_dispatch(entity)
        }
        Some(publication) => publication.authenticates(entity),
        None => true,
    }
}

fn retain_outer_tail_drive(
    manager: &EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    drive: OrdinaryType9OuterTailDrive,
) -> OrdinaryType9GoToJobOwnerTick {
    let entity_id = owner.entity_id();
    if !matches!(&drive, OrdinaryType9OuterTailDrive::StateMismatch) {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
            );
        };
        if outer_tail_entity_published_class14(manager, owner.actor_lease) {
            return OrdinaryType9GoToJobOwnerTick {
                outcome: OrdinaryType9GoToJobProductionOutcome::SurfaceLifecycleClass14Published {
                    entity_id,
                    task_lease: outer_tail_class14_exploding_task_lease(manager, owner.actor_lease),
                },
                retained_owner: None,
            };
        }
        if !refresh_retained_go_to_job_publication(&mut owner, entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
    }
    match drive {
        OrdinaryType9OuterTailDrive::Complete(expected_state) => {
            if snapshot_outer_tail_state(manager, owner.actor_lease) != Some(expected_state)
                || snapshot_outer_tail_animation_offset(manager, owner.actor_lease)
                    != Some(RetailRuntimeValue::Known(0))
            {
                return dropped(
                    entity_id,
                    OrdinaryType9GoToJobProductionDrop::OuterTailStateMismatch,
                );
            }
            owner.outer_tail = Some(OrdinaryType9OuterTailCustody::Complete {
                expected_state,
                expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
            });
            let outcome = owner
                .pending_outer_outcome
                .take()
                .expect("F70 retains the exact branch success until outer completion");
            OrdinaryType9GoToJobOwnerTick {
                outcome,
                retained_owner: Some(owner),
            }
        }
        OrdinaryType9OuterTailDrive::Blocked { custody, reason } => {
            owner.outer_tail = Some(custody);
            blocked(owner, map_outer_tail_block(reason))
        }
        OrdinaryType9OuterTailDrive::StateMismatch => dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::OuterTailStateMismatch,
        ),
    }
}

fn start_outer_tail_after_published_basis(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9GoToJobOwnerTick {
    let entity_id = owner.entity_id();
    let post_task_frame = match owner.state {
        OrdinaryType9GoToJobProductionState::PostBasisTailPending { post_task_frame } => {
            post_task_frame
        }
        _ => unreachable!("F70 publishes a post-task tail state"),
    };
    let transaction_id = take_outer_transaction_id(&mut owner.next_transaction_id);
    let (transaction, initial_animation_offset_at_0xb2) = match start_outer_tail_transaction(
        manager,
        owner.actor_lease,
        transaction_id,
        resources,
        post_task_frame.callback_elapsed_micros,
    ) {
        Ok(started) => started,
        Err(OrdinaryType9OuterTailStartError::PostBasisPublicationMismatch) => {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::PostBasisPublicationMismatch,
            )
        }
        Err(error) => return blocked(owner, map_outer_tail_block(error.into())),
    };
    let terrain_collision = TerrainCollisionContext::from_current_level_cache(resources);
    let drive = drive_outer_tail_transaction(
        manager,
        owner.actor_lease,
        transaction,
        post_task_frame.origin_retail_tick,
        initial_animation_offset_at_0xb2,
        terrain_collision,
        world_fx,
        next_shared_random,
    );
    retain_outer_tail_drive(manager, owner, drive)
}

enum OrdinaryType9GoToJobOuterTailEntry {
    Continue(OrdinaryType9GoToJobProductionOwner),
    Return(OrdinaryType9GoToJobOwnerTick),
}

fn enter_published_outer_tail(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9GoToJobOuterTailEntry {
    let entity_id = owner.entity_id();
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return OrdinaryType9GoToJobOuterTailEntry::Return(dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        ));
    };
    if !post_task_owner_authority_authenticates(&owner, entity) {
        return OrdinaryType9GoToJobOuterTailEntry::Return(dropped(
            entity_id,
            post_task_authority_mismatch(&owner),
        ));
    }

    if owner.outer_tail.is_none() {
        let post_task_frame = match owner.state {
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { post_task_frame } => {
                post_task_frame
            }
            _ => unreachable!("only a published F70 state enters the outer tail"),
        };
        if !post_basis_publication_authenticates(&owner, post_task_frame, entity)
            || owner.pending_outer_outcome.is_none()
        {
            return OrdinaryType9GoToJobOuterTailEntry::Return(dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::PostBasisPublicationMismatch,
            ));
        }
        return OrdinaryType9GoToJobOuterTailEntry::Return(start_outer_tail_after_published_basis(
            manager,
            owner,
            resources,
            world_fx,
            next_shared_random,
        ));
    }

    if !owner.outer_tail.as_ref().is_some_and(|custody| {
        outer_tail_observation_authenticates(manager, owner.actor_lease, custody)
    }) {
        return OrdinaryType9GoToJobOuterTailEntry::Return(dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::OuterTailStateMismatch,
        ));
    }
    if matches!(
        owner.outer_tail,
        Some(OrdinaryType9OuterTailCustody::BubblePending { .. })
    ) {
        let custody = owner.outer_tail.take().expect("matched above");
        return OrdinaryType9GoToJobOuterTailEntry::Return(
            match resume_outer_tail_bubble(
                manager,
                owner.actor_lease,
                custody,
                resources,
                world_fx,
                next_shared_random,
            ) {
                Ok(drive) => retain_outer_tail_drive(manager, owner, drive),
                Err(custody) => {
                    owner.outer_tail = Some(custody);
                    blocked(
                        owner,
                        OrdinaryType9GoToJobProductionBlock::CurrentTerrainUnavailable,
                    )
                }
            },
        );
    }
    let parked_reason = match owner.outer_tail.as_ref().expect("checked above") {
        OrdinaryType9OuterTailCustody::Transaction { block, .. } => {
            Some(OrdinaryType9GoToJobProductionBlock::OuterOwner(*block))
        }
        OrdinaryType9OuterTailCustody::LifecyclePending { .. } => {
            Some(OrdinaryType9GoToJobProductionBlock::OuterLifecyclePending)
        }
        OrdinaryType9OuterTailCustody::BubblePending { .. } => unreachable!(),
        OrdinaryType9OuterTailCustody::Complete { .. } => None,
    };
    if let Some(reason) = parked_reason {
        return OrdinaryType9GoToJobOuterTailEntry::Return(blocked(owner, reason));
    }
    debug_assert!(owner.pending_outer_outcome.is_none());
    owner.outer_tail = None;
    owner.state = OrdinaryType9GoToJobProductionState::Active;
    OrdinaryType9GoToJobOuterTailEntry::Continue(owner)
}

fn publish_post_task_basis(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    resources: &ResourceCache,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    frame: OrdinaryType9GoToJobPostTaskFrame,
    outcome: OrdinaryType9GoToJobProductionOutcome,
) -> OrdinaryType9GoToJobOwnerTick {
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::ActorLeaseChanged,
        );
    }
    let mismatch = post_task_authority_mismatch(&owner);
    let Some(entity) = manager.ordinary_type9_selected_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if !post_task_owner_authority_authenticates(&owner, entity) {
        return dropped(entity_id, mismatch);
    }
    if !post_task_basis_input_authenticates(frame, entity) {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::PostTaskFrameMismatch,
        );
    }

    debug_assert_eq!(
        frame.latched_effective_flags,
        ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS
    );
    debug_assert_eq!(
        frame.latched_effective_flags & TERRAIN_ATTITUDE_EFFECTIVE_FLAG,
        0
    );
    debug_assert_eq!(
        frame.latched_effective_flags & BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG,
        0
    );
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(frame.published_body_basis());
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    debug_assert!(post_basis_publication_authenticates(&owner, frame, entity));

    owner.state = OrdinaryType9GoToJobProductionState::PostBasisTailPending {
        post_task_frame: frame,
    };
    owner.pending_outer_outcome = Some(outcome);
    start_outer_tail_after_published_basis(manager, owner, resources, world_fx, next_shared_random)
}

fn dropped(
    entity_id: u32,
    reason: OrdinaryType9GoToJobProductionDrop,
) -> OrdinaryType9GoToJobOwnerTick {
    OrdinaryType9GoToJobOwnerTick {
        outcome: OrdinaryType9GoToJobProductionOutcome::Dropped { entity_id, reason },
        retained_owner: None,
    }
}

fn blocked(
    owner: OrdinaryType9GoToJobProductionOwner,
    reason: OrdinaryType9GoToJobProductionBlock,
) -> OrdinaryType9GoToJobOwnerTick {
    let entity_id = owner.entity_id();
    OrdinaryType9GoToJobOwnerTick {
        outcome: OrdinaryType9GoToJobProductionOutcome::Blocked { entity_id, reason },
        retained_owner: Some(owner),
    }
}

const fn slot_after(slot: ActorTaskSlot) -> Option<ActorTaskSlot> {
    match slot {
        ActorTaskSlot::Primary => Some(ActorTaskSlot::Secondary),
        ActorTaskSlot::Secondary => Some(ActorTaskSlot::Tertiary),
        ActorTaskSlot::Tertiary => None,
    }
}

fn continue_root_transition(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    resources: &ResourceCache,
    notifications: OrdinaryType9LiveNotificationContext<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    mut allocate_root_wander: impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
    mut allocate_root_go_to_job: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    mut allocate_root_run_away: impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
    mut allocate_root_attract_attention: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    )
        -> OrdinaryType9AttractAttentionAllocationDecision,
    mut allocate_attract_attention_target: impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
) -> OrdinaryType9GoToJobOwnerTick {
    let entity_id = owner.entity_id();
    let (transition, post_task_frame) = match owner.state {
        OrdinaryType9GoToJobProductionState::RootTransitionPending {
            transition,
            post_task_frame,
        } => (transition, post_task_frame),
        _ => {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootTransitionPending,
            )
        }
    };
    let attract_origin = transition.is_none();
    let Some(current_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::ActorLeaseChanged,
        );
    }
    let Some(metadata) = manager.type_runtime_metadata(9).cloned() else {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if let Some(transition) = transition {
        if !authenticates_root_transition(entity, transition)
            || !owner.task_authority.authenticates_retained_entity(entity)
        {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootTransitionRequestChanged,
            );
        }
    } else if !post_task_owner_authority_authenticates(&owner, entity) {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
        );
    }
    let Some(predecessor_visits) = owner.pending_root_task_visits else {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RootTaskVisitsUnavailable,
        );
    };
    if !post_task_basis_input_authenticates(post_task_frame, entity) {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::PostTaskFrameMismatch,
        );
    }
    let Some(dispatcher_continuation) = owner.pending_root_dispatcher_continuation.clone() else {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RootDispatcherContinuationUnavailable,
        );
    };
    let continuation_ok = if attract_origin {
        dispatcher_continuation.authenticates_attract_origin(entity_id, post_task_frame)
    } else {
        dispatcher_continuation.authenticates(entity_id, post_task_frame)
    };
    if !continuation_ok {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RootDispatcherContinuationMismatch,
        );
    }
    let live_predecessor_sub_a = match entity.sub_a_propulsion_runtime {
        RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
        RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAUnavailable,
            )
        }
    };
    if let Some(expected) = owner.pending_root_sub_a {
        if live_predecessor_sub_a != expected {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAChanged {
                    expected,
                    actual: live_predecessor_sub_a,
                },
            );
        }
    } else if owner.pending_root_plan.is_some() {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAUnavailable,
        );
    } else {
        // Bind the first resolved word before any later snapshot/model/planner
        // preflight can block, so a planless retry cannot accept Sub-A drift.
        owner.pending_root_sub_a = Some(live_predecessor_sub_a);
    }
    if let Some(expected) = owner.pending_root_actor_common_axis {
        let actual = entity.actor_common_axis_descriptor;
        if actual != RetailRuntimeValue::Known(expected) {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisChanged {
                    expected,
                    actual,
                },
            );
        }
    }
    if let Some(expected) = owner.pending_root_actor_animation {
        let actual = entity.actor_animation_runtime;
        if actual != RetailRuntimeValue::Known(Some(expected)) {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootPredecessorActorAnimationChanged {
                    expected,
                    actual,
                },
            );
        }
    }

    if owner.pending_root_plan.is_none() {
        if root_entity_ref(entity) != dispatcher_continuation.root_owner_snapshot {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootTransitionRequestChanged,
            );
        }
        let Some(active_model_id) = entity.model_index else {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::ActiveModelUnavailable,
            );
        };
        let owner_snapshot = dispatcher_continuation.root_owner_snapshot;
        let mut candidates = dispatcher_continuation
            .snapshots
            .iter()
            .map(root_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        let Some(controlled) = candidates
            .iter_mut()
            .find(|candidate| candidate.id == entity_id)
        else {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootTransitionRequestChanged,
            );
        };
        *controlled = owner_snapshot;
        let mut selector_draws = 0_u32;
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id,
                metadata: &metadata,
                owner: owner_snapshot,
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &candidates,
            },
            || {
                selector_draws = selector_draws.wrapping_add(1);
                next_shared_random(world_fx)
            },
        );
        match plan {
            Ok(plan) => owner.pending_root_plan = Some(plan),
            Err(error) => {
                if selector_draws != 0 {
                    owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
                    owner.pending_root_task_visits = None;
                    owner.pending_root_actor_common_axis = None;
                    owner.pending_root_sub_a = None;
                    owner.pending_root_dispatcher_continuation = None;
                }
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::RootReselection(error),
                );
            }
        }
    }

    let expected_predecessor_sub_a = owner
        .pending_root_sub_a
        .expect("root planning binds the first resolved predecessor Sub-A");

    let selection = owner.pending_root_plan.as_ref().unwrap().selection();
    let selected_class_id = match selection {
        OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
        OrdinaryType9RootSelection::Weighted { selection, .. } => {
            u32::from(selection.program.class_id)
        }
    };
    if selected_class_id != u32::from(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
        && selected_class_id != LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
        && selected_class_id != LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID
        && selected_class_id != LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
    {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::UnsupportedRootSelection {
                class_id: selected_class_id,
            },
        );
    }

    if selected_class_id == LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
        || selected_class_id == LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID
        || selected_class_id == LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
    {
        let expected_predecessor_actor_common_axis =
            if let Some(expected) = owner.pending_root_actor_common_axis {
                expected
            } else {
                match entity.actor_common_axis_descriptor {
                RetailRuntimeValue::Known(actual) => actual,
                RetailRuntimeValue::Unresolved => return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                ),
            }
            };
        if owner.pending_root_actor_common_axis.is_none() {
            // A transition-time unresolved word may become available before a
            // retry. Bind the first resolved value before any later preflight
            // can block so subsequent retries cannot silently accept drift.
            owner.pending_root_actor_common_axis = Some(expected_predecessor_actor_common_axis);
        }
        let root_run_away_audio = if selected_class_id == LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID {
            match authored_run_away_audio(&metadata) {
                Ok(audio) => Some(audio),
                Err(reason) => return blocked(owner, reason),
            }
        } else {
            None
        };
        let plan = owner
            .pending_root_plan
            .take()
            .expect("canonical root application consumes the retained plan");
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("authenticated selected actor remains live");
        let initial_completion = if selected_class_id == LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
        {
            let expected_predecessor_animation = if let Some(expected) =
                owner.pending_root_actor_animation
            {
                expected
            } else {
                match entity.actor_animation_runtime {
                    RetailRuntimeValue::Known(Some(actual)) => actual,
                    _ => {
                        owner.pending_root_plan = Some(plan);
                        return blocked(
                            owner,
                            OrdinaryType9GoToJobProductionBlock::RootPredecessorActorAnimationUnavailable,
                        );
                    }
                }
            };
            if owner.pending_root_actor_animation.is_none() {
                owner.pending_root_actor_animation = Some(expected_predecessor_animation);
            }
            let application = if attract_origin {
                apply_ordinary_type9_root_attract_attention(
                    entity,
                    &metadata,
                    plan,
                    predecessor_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    expected_predecessor_animation,
                    &mut allocate_root_attract_attention,
                    || next_shared_random(world_fx),
                    |request| notifications.dispatch(request),
                    |_| {},
                )
            } else {
                apply_ordinary_type9_root_attract_attention_from_go_to_job(
                    entity,
                    &metadata,
                    plan,
                    predecessor_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    expected_predecessor_animation,
                    &mut allocate_root_attract_attention,
                    || next_shared_random(world_fx),
                    |request| notifications.dispatch(request),
                    |_| {},
                )
            };
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    owner.pending_root_plan = Some(failure.into_plan());
                    return blocked(
                        owner,
                        OrdinaryType9GoToJobProductionBlock::RootAttractAttentionPreflight(reason),
                    );
                }
            };
            match application {
                OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                    committed,
                    owners,
                    publication,
                } => {
                    owner.root_attract_attention_initial_owners = Some(owners);
                    owner.root_attract_attention_target_route = None;
                    OrdinaryType9GoToJobRootCompletion::AttractAttention {
                        committed,
                        publication: OrdinaryType9GoToJobRootPublication::AttractAttention(
                            publication,
                        ),
                    }
                }
                OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    committed,
                } => {
                    owner.root_attract_attention_initial_owners = None;
                    owner.root_attract_attention_target_route = None;
                    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                    else {
                        unreachable!("class-45 fallback publishes a resolved context")
                    };
                    let selected_runtime = entity
                        .ordinary_type9_selected_component_runtime
                        .expect("class-45 fallback retains selected-component custody");
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        entity.actor_common_axis_descriptor
                    else {
                        unreachable!("class-45 fallback retains actor-axis custody")
                    };
                    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime
                    else {
                        unreachable!("class-45 fallback retains Sub-A custody")
                    };
                    let RetailRuntimeValue::Known(Some(actor_animation)) =
                        entity.actor_animation_runtime
                    else {
                        unreachable!("class-45 fallback retains actor-animation custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::AttractAttentionInitializerFallback {
                        failure,
                        committed,
                        publication:
                            OrdinaryType9GoToJobRootPublication::AttractAttentionInitializerFallback {
                                entity_id,
                                context,
                                selected_runtime,
                                initial_behavior: entity.initial_behavior,
                                actor_common_axis,
                                sub_a,
                                actor_animation,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            }
        } else if selected_class_id == LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID {
            let application = if attract_origin {
                apply_ordinary_type9_root_run_away_from_attract_attention(
                    entity,
                    &metadata,
                    plan,
                    predecessor_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    &mut allocate_root_run_away,
                    || next_shared_random(world_fx),
                )
            } else {
                apply_ordinary_type9_root_run_away_from_go_to_job(
                    entity,
                    &metadata,
                    plan,
                    predecessor_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    &mut allocate_root_run_away,
                    || next_shared_random(world_fx),
                )
            };
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    owner.pending_root_plan = Some(failure.into_plan());
                    return blocked(
                        owner,
                        OrdinaryType9GoToJobProductionBlock::RootRunAwayPreflight(reason),
                    );
                }
            };
            match application {
                OrdinaryType9RootRunAwayApplicationOutcome::Published {
                    constructors_by_phase,
                    owner: published,
                } => OrdinaryType9GoToJobRootCompletion::RunAway {
                    constructors_by_phase,
                    publication: OrdinaryType9GoToJobRootPublication::RunAway(published),
                },
                OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    constructors_by_phase,
                } => {
                    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                    else {
                        unreachable!("initializer fallback publishes a resolved context")
                    };
                    let selected_runtime = entity
                        .ordinary_type9_selected_component_runtime
                        .expect("initializer fallback retains selected-component custody");
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        entity.actor_common_axis_descriptor
                    else {
                        unreachable!("initializer fallback retains actor-axis custody")
                    };
                    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime
                    else {
                        unreachable!("initializer fallback retains Sub-A custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::RunAwayInitializerFallback {
                        failure,
                        constructors_by_phase,
                        publication:
                            OrdinaryType9GoToJobRootPublication::RunAwayInitializerFallback {
                                entity_id,
                                context,
                                selected_runtime,
                                initial_behavior: entity.initial_behavior,
                                actor_common_axis,
                                sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            }
        } else {
            let candidate_evidence =
                go_to_job_candidate_evidence(&dispatcher_continuation.snapshots);
            let application = if attract_origin {
                apply_ordinary_type9_root_go_to_job_from_attract_attention(
                    entity,
                    &metadata,
                    plan,
                    predecessor_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    &candidate_evidence,
                    &mut allocate_root_go_to_job,
                    || next_shared_random(world_fx),
                )
            } else {
                apply_ordinary_type9_root_go_to_job(
                    entity,
                    &metadata,
                    plan,
                    predecessor_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    &candidate_evidence,
                    &mut allocate_root_go_to_job,
                    || next_shared_random(world_fx),
                )
            };
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    owner.pending_root_plan = Some(failure.into_plan());
                    return blocked(
                        owner,
                        OrdinaryType9GoToJobProductionBlock::RootGoToJobPreflight(reason),
                    );
                }
            };
            match application {
                OrdinaryType9RootGoToJobApplicationOutcome::Published {
                    target_id,
                    constructor,
                    owner: published,
                } => OrdinaryType9GoToJobRootCompletion::GoToJob {
                    target_id,
                    constructor,
                    publication: OrdinaryType9GoToJobRootPublication::GoToJob(published),
                },
                OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
                    target_id,
                    failure,
                } => {
                    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                    else {
                        unreachable!("initializer fallback publishes a resolved context")
                    };
                    let selected_runtime = entity
                        .ordinary_type9_selected_component_runtime
                        .expect("initializer fallback retains selected-component custody");
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        entity.actor_common_axis_descriptor
                    else {
                        unreachable!("initializer fallback retains actor-axis custody")
                    };
                    let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime
                    else {
                        unreachable!("initializer fallback retains Sub-A custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::GoToJobInitializerFallback {
                        target_id,
                        failure,
                        publication:
                            OrdinaryType9GoToJobRootPublication::GoToJobInitializerFallback {
                                entity_id,
                                context,
                                selected_runtime,
                                initial_behavior: entity.initial_behavior,
                                actor_common_axis,
                                sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            }
        };

        let OrdinaryType9GoToJobDispatcherContinuation {
            snapshots,
            next_slot,
            callback_elapsed_micros,
            global_elapsed_micros,
            scheduler_mode,
            root_facts,
            ..
        } = dispatcher_continuation;
        let mut random = || next_shared_random(world_fx);
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("the continuation lease remains live");
        let position_raw = root_facts.position_raw;
        let expected_root_task_visits = live_task_visits(&entity.actor_tasks);
        let Entity {
            actor_tasks,
            ordinary_type9_selected_component_runtime,
            actor_common_axis_descriptor,
            sub_a_propulsion_runtime,
            actor_animation_runtime,
            current_behavior_context,
            collision,
            heading,
            velocity,
            ..
        } = entity;
        let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
            unreachable!("root publication retains behavior-context custody")
        };
        let expected_actor_common_axis_descriptor = metadata
            .initializer
            .as_ref()
            .map(|initializer| initializer.common_axis_descriptor);
        let mut adapter = SelectedGoToJobAdapter {
            notifications,
            entity_id,
            position_raw,
            root_facts,
            expected_root_task_visits,
            metadata: Some(metadata.clone()),
            terrain: resources.level_terrain(),
            expected_actor_common_axis_descriptor,
            actor_common_axis_descriptor,
            snapshots,
            selected: ordinary_type9_selected_component_runtime,
            sub_a: sub_a_propulsion_runtime,
            animation: actor_animation_runtime,
            context,
            heading,
            velocity,
            pre_mover_basis: post_task_frame.pre_basis,
            elapsed_micros: callback_elapsed_micros,
            global_elapsed_micros,
            scheduler_mode,
            collision,
            next_transaction_id: &mut owner.next_transaction_id,
            random: Rc::new(RefCell::new(&mut random)),
            allocate_root_wander: &mut allocate_root_wander,
            allocate_root_go_to_job: &mut allocate_root_go_to_job,
            allocate_root_run_away: &mut allocate_root_run_away,
            allocate_root_attract_attention: &mut allocate_root_attract_attention,
            allocate_attract_attention_target: &mut allocate_attract_attention_target,
            root_run_away_audio,
            root_attract_attention_initial_owners: &mut owner.root_attract_attention_initial_owners,
            root_attract_attention_target_route: &mut owner.root_attract_attention_target_route,
            attract_attention_candidate_completion: None,
            root_completion: Some(initial_completion),
        };
        let dispatcher = match next_slot {
            Some(slot) => tick_actor_task_dispatcher_from_slot(
                actor_tasks,
                ActorTaskDispatcherFrame {
                    elapsed_micros: callback_elapsed_micros,
                    scheduler_mode: scheduler_mode as u32,
                },
                slot,
                &mut adapter,
            ),
            None => Ok(None),
        };
        let candidate_completion = adapter.attract_attention_candidate_completion.take();
        let completion = adapter.root_completion.take();
        drop(adapter);
        if let Some(committed) = completion
            .as_ref()
            .and_then(attract_attention_committed_effects)
        {
            commit_attract_attention_root_sounds(world_fx, vec![committed]);
        }
        match dispatcher {
            Err(error) => {
                let Some(completion) = completion else {
                    return dropped(
                        entity_id,
                        OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
                    );
                };
                let publication = completion.into_publication();
                if !manager
                    .iter_all()
                    .find(|entity| entity.id == entity_id)
                    .is_some_and(|entity| publication.authenticates(entity))
                {
                    return dropped(
                        entity_id,
                        OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
                    );
                }
                owner.root_publication = Some(publication);
                owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
                owner.pending_root_plan = None;
                owner.pending_root_task_visits = None;
                owner.pending_root_actor_common_axis = None;
                owner.pending_root_sub_a = None;
                owner.pending_root_dispatcher_continuation = None;
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::Dispatcher(error),
                );
            }
            Ok(Some(_)) => {
                let Some(completion) = completion else {
                    return dropped(
                        entity_id,
                        OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
                    );
                };
                let publication = completion.into_publication();
                if !manager
                    .iter_all()
                    .find(|entity| entity.id == entity_id)
                    .is_some_and(|entity| publication.authenticates(entity))
                {
                    return dropped(
                        entity_id,
                        OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
                    );
                }
                owner.root_publication = Some(publication);
                owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
                owner.pending_root_plan = None;
                owner.pending_root_task_visits = None;
                owner.pending_root_actor_common_axis = None;
                owner.pending_root_sub_a = None;
                owner.pending_root_dispatcher_continuation = None;
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::RootDispatcherContinuationProducedTransition,
                );
            }
            Ok(None) => {}
        }
        if let Some(candidate_completion) = candidate_completion {
            let (publication, outcome) =
                fold_attract_attention_candidate_completion(entity_id, candidate_completion);
            owner.pending_root_task_visits = None;
            owner.pending_root_actor_common_axis = None;
            owner.pending_root_sub_a = None;
            owner.pending_root_actor_animation = None;
            owner.pending_root_dispatcher_continuation = None;
            owner.root_publication = publication;
            return publish_post_task_basis(
                manager,
                owner,
                resources,
                world_fx,
                next_shared_random,
                post_task_frame,
                outcome,
            );
        }
        let Some(mut completion) = completion else {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        };
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
            );
        };
        if !completion.refresh_after_owned_dispatch(entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        let (publication, outcome) = completion.into_parts(entity_id);
        if !publication.authenticates(entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        owner.pending_root_task_visits = None;
        owner.pending_root_actor_common_axis = None;
        owner.pending_root_sub_a = None;
        owner.pending_root_dispatcher_continuation = None;
        owner.root_publication = Some(publication);
        return publish_post_task_basis(
            manager,
            owner,
            resources,
            world_fx,
            next_shared_random,
            post_task_frame,
            outcome,
        );
    }

    let plan = owner.pending_root_plan.take().unwrap();
    let entity = manager
        .ordinary_type9_selected_entity_mut(entity_id)
        .expect("authenticated selected actor remains live");
    let application = apply_ordinary_type9_root_wander(
        entity,
        &metadata,
        plan,
        predecessor_visits,
        &mut allocate_root_wander,
        || next_shared_random(world_fx),
    );
    let application = match application {
        Ok(application) => application,
        Err(failure) => {
            let reason = failure.error.clone();
            owner.pending_root_plan = Some(failure.into_plan());
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootWanderPreflight(reason),
            );
        }
    };
    owner.pending_root_task_visits = None;
    owner.pending_root_actor_common_axis = None;
    owner.pending_root_sub_a = None;
    owner.pending_root_dispatcher_continuation = None;
    let (publication, outcome) = root_application_publication(entity_id, entity, application);
    owner.root_publication = Some(publication);
    publish_post_task_basis(
        manager,
        owner,
        resources,
        world_fx,
        next_shared_random,
        post_task_frame,
        outcome,
    )
}

fn root_application_publication(
    entity_id: u32,
    entity: &Entity,
    application: OrdinaryType9RootWanderApplicationOutcome,
) -> (
    OrdinaryType9GoToJobRootPublication,
    OrdinaryType9GoToJobProductionOutcome,
) {
    match application {
        OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } => (
            OrdinaryType9GoToJobRootPublication::Wander(owner),
            OrdinaryType9GoToJobProductionOutcome::RootWanderPublished {
                entity_id,
                constructor,
            },
        ),
        OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                unreachable!("initializer fallback publishes a resolved context")
            };
            let selected_runtime = entity
                .ordinary_type9_selected_component_runtime
                .expect("initializer fallback retains selected component custody");
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                unreachable!("initializer fallback retains Sub-A custody")
            };
            let policy = crate::entity_behavior::translate_state_policy(
                crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
            );
            (
                OrdinaryType9GoToJobRootPublication::InitializerFallback {
                    entity_id,
                    context,
                    selected_runtime,
                    initial_behavior: entity.initial_behavior,
                    sub_a,
                    state_policy_mask: policy.set_bits | policy.clear_bits,
                    state_policy_bits: policy.set_bits,
                },
                OrdinaryType9GoToJobProductionOutcome::RootInitializerFallbackPublished {
                    entity_id,
                    failure,
                },
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_go_to_job_published_attract_followup(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    frame: OrdinaryType9GoToJobProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    allocate_root_wander: &mut impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job: &mut impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away: &mut impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
    allocate_root_attract_attention: &mut impl FnMut(
        AttractAttentionInitialTaskPreparation,
    )
        -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_attract_attention_target: &mut impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
) -> OrdinaryType9GoToJobOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: frame.dispatch_resource_text,
        retail_tick: frame.retail_tick,
    };
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::ActorLeaseChanged,
        );
    }
    let Some(metadata) = manager.type_runtime_metadata(9).cloned() else {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    let mut snapshots = manager
        .iter_all()
        .map(|entity| EntitySnapshot {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            capability_flags: entity.capability_flags,
            active: entity.active,
            collision: entity.collision.clone(),
            job_capacity: job_capacity_from_entity(entity),
        })
        .collect::<Vec<_>>();
    let Some(entity) = manager.ordinary_type9_selected_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if !post_task_owner_authority_authenticates(&owner, entity) {
        return dropped(entity_id, post_task_authority_mismatch(&owner));
    }
    if let Err(reason) = preflight_normal_scheduler_owner(&entity.collision) {
        return blocked(owner, reason);
    }
    apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut entity.collision);
    let expected_actor_common_axis_descriptor = metadata
        .initializer
        .as_ref()
        .map(|initializer| initializer.common_axis_descriptor);
    let scheduler_mode = match entity
        .collision
        .state_flags_at_0x08
        .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => 1,
        RetailRuntimeValue::Known(_) => 0,
        RetailRuntimeValue::Unresolved => {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::SchedulerModeUnresolved,
            )
        }
    };
    let callback_mass_before_prefix = if scheduler_mode == 0 {
        match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => Some(value),
            RetailRuntimeValue::Unresolved => {
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::CallbackMassUnavailable,
                )
            }
        }
    } else {
        None
    };
    let mut random = || next_shared_random(world_fx);
    let scheduler_prefix = {
        let RetailRuntimeValue::Known(prefix) =
            plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut random)
        else {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::SchedulerStateUnresolved,
            );
        };
        prefix
    };
    commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
    let callback_elapsed_micros = match scheduler_prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            return OrdinaryType9GoToJobOwnerTick {
                outcome: OrdinaryType9GoToJobProductionOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
            }
        }
        CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us,
        } => callback_elapsed_us,
    };
    let callback_mass_raw = match callback_mass_before_prefix {
        Some(value) => value,
        None => match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::CallbackMassUnavailable,
                );
            }
        },
    };
    entity.mass_raw = callback_mass_raw;
    refresh_controlled_snapshot(
        &mut snapshots,
        entity_id,
        entity.position_raw(),
        entity.velocity_raw(),
        entity.capability_flags,
        entity.active,
        &entity.collision,
    );
    let pre_mover_basis = entity.physical_body_basis_q31;
    let position_raw = entity.position_raw();
    let root_facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
    let expected_root_task_visits = live_task_visits(&entity.actor_tasks);
    let Entity {
        actor_tasks,
        ordinary_type9_selected_component_runtime,
        actor_common_axis_descriptor,
        sub_a_propulsion_runtime,
        actor_animation_runtime,
        current_behavior_context,
        collision,
        heading,
        velocity,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
        );
    };
    let mut adapter = SelectedGoToJobAdapter {
        notifications,
        entity_id,
        position_raw,
        root_facts,
        expected_root_task_visits,
        metadata: Some(metadata),
        terrain: frame.resources.level_terrain(),
        expected_actor_common_axis_descriptor,
        actor_common_axis_descriptor,
        snapshots,
        selected: ordinary_type9_selected_component_runtime,
        sub_a: sub_a_propulsion_runtime,
        animation: actor_animation_runtime,
        context,
        heading,
        velocity,
        pre_mover_basis,
        elapsed_micros: callback_elapsed_micros,
        global_elapsed_micros: frame.global_elapsed_micros,
        scheduler_mode,
        collision,
        next_transaction_id: &mut owner.next_transaction_id,
        random: Rc::new(RefCell::new(&mut random)),
        allocate_root_wander,
        allocate_root_go_to_job,
        allocate_root_run_away,
        allocate_root_attract_attention,
        allocate_attract_attention_target,
        root_run_away_audio: None,
        root_attract_attention_initial_owners: &mut owner.root_attract_attention_initial_owners,
        root_attract_attention_target_route: &mut owner.root_attract_attention_target_route,
        attract_attention_candidate_completion: None,
        root_completion: None,
    };
    let dispatcher = tick_actor_task_dispatcher(
        actor_tasks,
        ActorTaskDispatcherFrame {
            elapsed_micros: callback_elapsed_micros,
            scheduler_mode: scheduler_mode as u32,
        },
        &mut adapter,
    );
    let mut root_completion = adapter.root_completion.take();
    let candidate_completion = adapter.attract_attention_candidate_completion.take();
    drop(adapter);
    if let Some(committed) = root_completion
        .as_ref()
        .and_then(attract_attention_committed_effects)
    {
        commit_attract_attention_root_sounds(world_fx, vec![committed]);
    }
    let (output, dispatcher_error) = match dispatcher {
        Ok(output) => (output, None),
        Err(error) => (None, Some(error)),
    };
    let Some(post_task_entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if post_task_entity.physical_body_basis_q31 != pre_mover_basis {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::PostTaskFrameMismatch,
        );
    }
    let post_task_frame = OrdinaryType9GoToJobPostTaskFrame {
        callback_elapsed_micros,
        origin_retail_tick: frame.retail_tick,
        latched_effective_flags: ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        post_task_angles_raw: post_task_entity.rotation_heading_pitch_roll_raw(),
        pre_basis: pre_mover_basis,
    };
    if let Some(error) = dispatcher_error {
        owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::Dispatcher(error),
        );
    }
    if let Some(deferred) = output {
        if !refresh_retained_go_to_job_publication(&mut owner, post_task_entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        if deferred.retryable {
            owner.state = OrdinaryType9GoToJobProductionState::RootTransitionPending {
                transition: deferred.transition,
                post_task_frame,
            };
            owner.pending_root_plan = deferred.plan;
            owner.pending_root_task_visits = Some(deferred.predecessor_task_visits);
            owner.pending_root_actor_common_axis = deferred.expected_predecessor_actor_common_axis;
            owner.pending_root_sub_a = deferred.expected_predecessor_sub_a;
            owner.pending_root_dispatcher_continuation = Some(deferred.dispatcher_continuation);
        } else {
            owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
        }
        return blocked(owner, deferred.reason);
    }
    if let Some(completion) = candidate_completion {
        let (publication, outcome) =
            fold_attract_attention_candidate_completion(entity_id, completion);
        owner.root_publication = publication;
        if !post_task_owner_authority_authenticates(&owner, post_task_entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        return publish_post_task_basis(
            manager,
            owner,
            frame.resources,
            world_fx,
            next_shared_random,
            post_task_frame,
            outcome,
        );
    }
    if let Some(mut completion) = root_completion.take() {
        if !completion.refresh_after_owned_dispatch(post_task_entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        let (publication, outcome) = completion.into_parts(entity_id);
        owner.root_publication = Some(publication);
        return publish_post_task_basis(
            manager,
            owner,
            frame.resources,
            world_fx,
            next_shared_random,
            post_task_frame,
            outcome,
        );
    }
    if !refresh_retained_go_to_job_publication(&mut owner, post_task_entity) {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
        );
    }
    publish_post_task_basis(
        manager,
        owner,
        frame.resources,
        world_fx,
        next_shared_random,
        post_task_frame,
        OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
            entity_id,
            callback_elapsed_micros,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn tick_go_to_job_published_target_route(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    frame: OrdinaryType9GoToJobProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    allocate_root_wander: &mut impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job: &mut impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away: &mut impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
    allocate_root_attract_attention: &mut impl FnMut(
        AttractAttentionInitialTaskPreparation,
    )
        -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_attract_attention_target: &mut impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
) -> OrdinaryType9GoToJobOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: frame.dispatch_resource_text,
        retail_tick: frame.retail_tick,
    };
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::ActorLeaseChanged,
        );
    }
    let Some(metadata) = manager.type_runtime_metadata(9).cloned() else {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    let snapshots = manager
        .iter_all()
        .map(|entity| EntitySnapshot {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            capability_flags: entity.capability_flags,
            active: entity.active,
            collision: entity.collision.clone(),
            job_capacity: job_capacity_from_entity(entity),
        })
        .collect::<Vec<_>>();
    let Some(entity) = manager.ordinary_type9_selected_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if owner
        .root_attract_attention_target_route
        .as_ref()
        .is_none_or(|route| route.validate(entity).is_err())
    {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
        );
    }
    if let Err(reason) = preflight_normal_scheduler_owner(&entity.collision) {
        return blocked(owner, reason);
    }
    apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut entity.collision);
    let expected_actor_common_axis_descriptor = metadata
        .initializer
        .as_ref()
        .map(|initializer| initializer.common_axis_descriptor);
    let actor_common_axis_descriptor = entity.actor_common_axis_descriptor;
    let scheduler_mode = match entity
        .collision
        .state_flags_at_0x08
        .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => 1,
        RetailRuntimeValue::Known(_) => 0,
        RetailRuntimeValue::Unresolved => {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::SchedulerModeUnresolved,
            )
        }
    };
    let callback_mass_before_prefix = if scheduler_mode == 0 {
        match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => Some(value),
            RetailRuntimeValue::Unresolved => {
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::CallbackMassUnavailable,
                )
            }
        }
    } else {
        None
    };
    let mut random = || next_shared_random(world_fx);
    let RetailRuntimeValue::Known(prefix) =
        plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut random)
    else {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::SchedulerStateUnresolved,
        );
    };
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let callback_elapsed_micros = match prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            return OrdinaryType9GoToJobOwnerTick {
                outcome: OrdinaryType9GoToJobProductionOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
            }
        }
        CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us,
        } => callback_elapsed_us,
    };
    let callback_mass = match callback_mass_before_prefix {
        Some(value) => value,
        None => match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::CallbackMassUnavailable,
                );
            }
        },
    };
    entity.mass_raw = callback_mass;
    let pre_mover_basis = entity.physical_body_basis_q31;
    let position_raw = entity.position_raw();
    let root_facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
    let expected_root_task_visits = live_task_visits(&entity.actor_tasks);
    let mut staged_selected = entity.ordinary_type9_selected_component_runtime;
    let mut staged_actor_common_axis = entity.actor_common_axis_descriptor;
    let mut staged_sub_a = entity.sub_a_propulsion_runtime;
    let mut staged_animation = entity.actor_animation_runtime;
    let RetailRuntimeValue::Known(Some(mut staged_context)) = entity.current_behavior_context
    else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
        );
    };
    let mut staged_collision = entity.collision.clone();
    let mut staged_heading = entity.heading;
    let mut staged_velocity = entity.velocity;
    let route_owner = owner
        .root_attract_attention_target_route
        .take()
        .expect("active Target Route retains its exact linear owner");
    let mut movement_state = ();
    let mut controller_context = ();
    let mut replacement_target_route = None;
    let adapter = RefCell::new(SelectedGoToJobAdapter {
        notifications,
        entity_id,
        position_raw,
        root_facts,
        expected_root_task_visits,
        metadata: Some(metadata.clone()),
        terrain: frame.resources.level_terrain(),
        expected_actor_common_axis_descriptor,
        actor_common_axis_descriptor: &mut staged_actor_common_axis,
        snapshots,
        selected: &mut staged_selected,
        sub_a: &mut staged_sub_a,
        animation: &mut staged_animation,
        context: &mut staged_context,
        heading: &mut staged_heading,
        velocity: &mut staged_velocity,
        pre_mover_basis,
        elapsed_micros: callback_elapsed_micros,
        global_elapsed_micros: frame.global_elapsed_micros,
        scheduler_mode,
        collision: &mut staged_collision,
        next_transaction_id: &mut owner.next_transaction_id,
        random: Rc::new(RefCell::new(&mut random)),
        allocate_root_wander,
        allocate_root_go_to_job,
        allocate_root_run_away,
        allocate_root_attract_attention,
        allocate_attract_attention_target,
        root_run_away_audio: None,
        root_attract_attention_initial_owners: &mut owner.root_attract_attention_initial_owners,
        root_attract_attention_target_route: &mut replacement_target_route,
        attract_attention_candidate_completion: None,
        root_completion: None,
    });
    let route_tick = route_owner.tick(
        entity,
        &metadata,
        OrdinaryType9AttractAttentionTargetRouteFrame {
            movement_state: &mut movement_state,
            controller_context: &mut controller_context,
            elapsed_micros: callback_elapsed_micros,
            scheduler_mode: scheduler_mode as u32,
        },
        |target_id| adapter.borrow().target_route_target_state(target_id),
        |request| {
            adapter.borrow().target_route_predicate(
                request.target_id,
                expected_actor_common_axis_descriptor,
                actor_common_axis_descriptor,
            )
        },
        |request| {
            adapter
                .borrow_mut()
                .run_common_mover(request.visit, request.target_state)
                .map(|result| match result {
                    WanderNearCommonMoverReturn::Zero => SharedTargetRouteCommonMoverReturn::Zero,
                    WanderNearCommonMoverReturn::NonZero => {
                        SharedTargetRouteCommonMoverReturn::NonZero
                    }
                    WanderNearCommonMoverReturn::Unresolved => unreachable!(
                        "selected common-mover binding never fabricates unresolved return"
                    ),
                })
        },
    );
    drop(adapter);
    entity.ordinary_type9_selected_component_runtime = staged_selected;
    entity.actor_common_axis_descriptor = staged_actor_common_axis;
    entity.sub_a_propulsion_runtime = staged_sub_a;
    entity.actor_animation_runtime = staged_animation;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(staged_context));
    entity.collision = staged_collision;
    entity.heading = staged_heading;
    entity.velocity = staged_velocity;
    let mut retained_route = None;
    let route_error = match route_tick {
        Ok(OrdinaryType9AttractAttentionTargetRouteTickOutcome::Continue {
            owner: route, ..
        })
        | Ok(OrdinaryType9AttractAttentionTargetRouteTickOutcome::TransitionSuppressed {
            owner: route,
            ..
        }) => {
            retained_route = Some(route);
            None
        }
        Ok(OrdinaryType9AttractAttentionTargetRouteTickOutcome::TransitionPending { .. }) => {
            owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
            Some(OrdinaryType9GoToJobProductionBlock::TargetRouteTransitionPending)
        }
        Ok(OrdinaryType9AttractAttentionTargetRouteTickOutcome::WrapperRetiredDuringCallback {
            ..
        }) => {
            owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
            Some(OrdinaryType9GoToJobProductionBlock::TargetRouteWrapperRetired)
        }
        Err(failure) => {
            let error = failure.error.clone();
            if let Some(route) = failure.into_owner() {
                retained_route = Some(route);
            } else {
                owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
            }
            Some(OrdinaryType9GoToJobProductionBlock::TargetRoute(error))
        }
    };
    owner.root_attract_attention_target_route = replacement_target_route.or(retained_route);
    if entity.physical_body_basis_q31 != pre_mover_basis {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::PostTaskFrameMismatch,
        );
    }
    let post_task_frame = OrdinaryType9GoToJobPostTaskFrame {
        callback_elapsed_micros,
        origin_retail_tick: frame.retail_tick,
        latched_effective_flags: ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        post_task_angles_raw: entity.rotation_heading_pitch_roll_raw(),
        pre_basis: pre_mover_basis,
    };
    if let Some(reason) = route_error {
        return blocked(owner, reason);
    }
    if !post_task_owner_authority_authenticates(&owner, entity) {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
        );
    }
    publish_post_task_basis(
        manager,
        owner,
        frame.resources,
        world_fx,
        next_shared_random,
        post_task_frame,
        OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
            entity_id,
            callback_elapsed_micros,
        },
    )
}

pub(crate) fn tick_ordinary_type9_go_to_job_owner_with_random(
    manager: &mut EntityManager,
    owner: OrdinaryType9GoToJobProductionOwner,
    frame: OrdinaryType9GoToJobProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9GoToJobOwnerTick {
    tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
        manager,
        owner,
        frame,
        world_fx,
        next_shared_random,
        |_| OrdinaryType9WanderAllocationDecision::Prepared,
        |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
        |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
        |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
        |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
    )
}

fn tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9GoToJobProductionOwner,
    frame: OrdinaryType9GoToJobProductionFrame<'_>,
    world_fx: &mut WorldFx,
    mut next_shared_random: impl FnMut(&mut WorldFx) -> u32,
    mut allocate_root_wander: impl FnMut(
        OrdinaryType9WanderTaskSpec,
    ) -> OrdinaryType9WanderAllocationDecision,
    mut allocate_root_go_to_job: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    mut allocate_root_run_away: impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
    mut allocate_root_attract_attention: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    )
        -> OrdinaryType9AttractAttentionAllocationDecision,
    mut allocate_attract_attention_target: impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
) -> OrdinaryType9GoToJobOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: frame.dispatch_resource_text,
        retail_tick: frame.retail_tick,
    };
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if current_lease != owner.actor_lease {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::ActorLeaseChanged,
        );
    }
    if matches!(
        owner.state,
        OrdinaryType9GoToJobProductionState::RootTransitionPending { .. }
    ) {
        return continue_root_transition(
            manager,
            owner,
            frame.resources,
            notifications,
            world_fx,
            &mut next_shared_random,
            &mut allocate_root_wander,
            &mut allocate_root_go_to_job,
            &mut allocate_root_run_away,
            &mut allocate_root_attract_attention,
            &mut allocate_attract_attention_target,
        );
    }
    if matches!(
        owner.state,
        OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
    ) {
        match enter_published_outer_tail(
            manager,
            owner,
            frame.resources,
            world_fx,
            &mut next_shared_random,
        ) {
            OrdinaryType9GoToJobOuterTailEntry::Return(tick) => return tick,
            OrdinaryType9GoToJobOuterTailEntry::Continue(continued) => owner = continued,
        }
    }
    if owner.root_attract_attention_initial_owners.is_some()
        && matches!(
            owner.root_publication,
            Some(OrdinaryType9GoToJobRootPublication::AttractAttention(_))
        )
        && owner.state == OrdinaryType9GoToJobProductionState::Active
    {
        return tick_go_to_job_published_attract_followup(
            manager,
            owner,
            frame,
            world_fx,
            &mut next_shared_random,
            &mut allocate_root_wander,
            &mut allocate_root_go_to_job,
            &mut allocate_root_run_away,
            &mut allocate_root_attract_attention,
            &mut allocate_attract_attention_target,
        );
    }
    if owner.root_attract_attention_target_route.is_some()
        && owner.state == OrdinaryType9GoToJobProductionState::Active
    {
        return tick_go_to_job_published_target_route(
            manager,
            owner,
            frame,
            world_fx,
            &mut next_shared_random,
            &mut allocate_root_wander,
            &mut allocate_root_go_to_job,
            &mut allocate_root_run_away,
            &mut allocate_root_attract_attention,
            &mut allocate_attract_attention_target,
        );
    }
    if owner.state == OrdinaryType9GoToJobProductionState::CallbackFailurePending {
        return blocked(
            owner,
            OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
        );
    }

    // These callback-local authorities are captured without requiring them.
    // Retail does not consume them until a live target reaches its predicate
    // and common mover, so scheduler wait and invalid-target paths must not be
    // blocked by their absence.
    let metadata = manager.type_runtime_metadata(9).cloned();
    let terrain = frame.resources.level_terrain();
    let mut snapshots = manager
        .iter_all()
        .map(|entity| EntitySnapshot {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            velocity_raw: entity.velocity_raw(),
            capability_flags: entity.capability_flags,
            active: entity.active,
            collision: entity.collision.clone(),
            job_capacity: job_capacity_from_entity(entity),
        })
        .collect::<Vec<_>>();
    let Some(entity) = manager.ordinary_type9_selected_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if !owner.task_authority.authenticates_retained_entity(entity) {
        return dropped(entity_id, post_task_authority_mismatch(&owner));
    }
    if let Err(reason) = preflight_normal_scheduler_owner(&entity.collision) {
        return blocked(owner, reason);
    }
    apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut entity.collision);
    let expected_actor_common_axis_descriptor = metadata
        .as_ref()
        .and_then(|metadata| metadata.initializer.as_ref())
        .map(|initializer| initializer.common_axis_descriptor);
    let scheduler_mode = match entity
        .collision
        .state_flags_at_0x08
        .masked(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => 1,
        RetailRuntimeValue::Known(_) => 0,
        RetailRuntimeValue::Unresolved => {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::SchedulerModeUnresolved,
            )
        }
    };
    let callback_mass_before_prefix = if scheduler_mode == 0 {
        match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => Some(value),
            RetailRuntimeValue::Unresolved => {
                // This detailed visit cannot wait or draw. An unresolved
                // contribution must block before prefix mutation; native
                // fresh construction supplies its own defined initial value.
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::CallbackMassUnavailable,
                );
            }
        }
    } else {
        None
    };

    let mut random = || next_shared_random(world_fx);
    let scheduler_prefix = {
        let RetailRuntimeValue::Known(prefix) =
            plan_common_scheduler_prefix(&entity.collision, frame.elapsed_micros, &mut random)
        else {
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::SchedulerStateUnresolved,
            );
        };
        prefix
    };
    commit_common_scheduler_prefix(&mut entity.collision, scheduler_prefix);
    let callback_elapsed_micros = match scheduler_prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            return OrdinaryType9GoToJobOwnerTick {
                outcome: OrdinaryType9GoToJobProductionOutcome::SchedulerWaiting { entity_id },
                retained_owner: Some(owner),
            }
        }
        CommonSchedulerPrefixFlow::Continue {
            callback_elapsed_us,
        } => callback_elapsed_us,
    };
    let callback_mass_raw = match callback_mass_before_prefix {
        Some(value) => value,
        None => match common_scheduler_callback_mass(
            LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
                return blocked(
                    owner,
                    OrdinaryType9GoToJobProductionBlock::CallbackMassUnavailable,
                );
            }
        },
    };
    entity.mass_raw = callback_mass_raw;
    refresh_controlled_snapshot(
        &mut snapshots,
        entity_id,
        entity.position_raw(),
        entity.velocity_raw(),
        entity.capability_flags,
        entity.active,
        &entity.collision,
    );

    let pre_mover_basis = entity.physical_body_basis_q31;
    let position_raw = entity.position_raw();
    let root_facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
    let expected_root_task_visits = owner.task_authority.task_visits();
    debug_assert_eq!(
        live_task_visits(&entity.actor_tasks),
        expected_root_task_visits
    );
    let Entity {
        actor_tasks,
        ordinary_type9_selected_component_runtime,
        actor_common_axis_descriptor,
        sub_a_propulsion_runtime,
        actor_animation_runtime,
        current_behavior_context,
        collision,
        heading,
        velocity,
        ..
    } = entity;
    let RetailRuntimeValue::Known(Some(context)) = current_behavior_context else {
        unreachable!("the authenticated initial owner retains its exact behavior context")
    };
    let mut adapter = SelectedGoToJobAdapter {
        notifications,
        entity_id,
        position_raw,
        root_facts,
        expected_root_task_visits,
        metadata,
        terrain,
        expected_actor_common_axis_descriptor,
        actor_common_axis_descriptor,
        snapshots,
        selected: ordinary_type9_selected_component_runtime,
        sub_a: sub_a_propulsion_runtime,
        animation: actor_animation_runtime,
        context,
        heading,
        velocity,
        pre_mover_basis,
        elapsed_micros: callback_elapsed_micros,
        global_elapsed_micros: frame.global_elapsed_micros,
        scheduler_mode,
        collision,
        next_transaction_id: &mut owner.next_transaction_id,
        random: Rc::new(RefCell::new(&mut random)),
        allocate_root_wander: &mut allocate_root_wander,
        allocate_root_go_to_job: &mut allocate_root_go_to_job,
        allocate_root_run_away: &mut allocate_root_run_away,
        allocate_root_attract_attention: &mut allocate_root_attract_attention,
        allocate_attract_attention_target: &mut allocate_attract_attention_target,
        root_run_away_audio: None,
        root_attract_attention_initial_owners: &mut owner.root_attract_attention_initial_owners,
        root_attract_attention_target_route: &mut owner.root_attract_attention_target_route,
        attract_attention_candidate_completion: None,
        root_completion: None,
    };
    let dispatcher = tick_actor_task_dispatcher(
        actor_tasks,
        ActorTaskDispatcherFrame {
            elapsed_micros: callback_elapsed_micros,
            scheduler_mode: scheduler_mode as u32,
        },
        &mut adapter,
    );
    let candidate_completion = adapter.attract_attention_candidate_completion.take();
    let root_completion = adapter.root_completion.take();
    drop(adapter);
    if let Some(committed) = root_completion
        .as_ref()
        .and_then(attract_attention_committed_effects)
    {
        commit_attract_attention_root_sounds(world_fx, vec![committed]);
    }
    let output = match dispatcher {
        Ok(output) => output,
        Err(error) => {
            if let Some(completion) = root_completion {
                let publication = completion.into_publication();
                if !manager
                    .iter_all()
                    .find(|entity| entity.id == entity_id)
                    .is_some_and(|entity| publication.authenticates(entity))
                {
                    return dropped(
                        entity_id,
                        OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
                    );
                }
                owner.root_publication = Some(publication);
            }
            owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
            owner.pending_root_plan = None;
            owner.pending_root_task_visits = None;
            owner.pending_root_actor_common_axis = None;
            owner.pending_root_sub_a = None;
            owner.pending_root_dispatcher_continuation = None;
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::Dispatcher(error),
            );
        }
    };

    let Some(post_task_entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::EntityUnavailable,
        );
    };
    if post_task_entity.physical_body_basis_q31 != pre_mover_basis {
        return dropped(
            entity_id,
            OrdinaryType9GoToJobProductionDrop::PostTaskFrameMismatch,
        );
    }
    let post_task_frame = OrdinaryType9GoToJobPostTaskFrame {
        callback_elapsed_micros,
        origin_retail_tick: frame.retail_tick,
        latched_effective_flags: ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        post_task_angles_raw: post_task_entity.rotation_heading_pitch_roll_raw(),
        pre_basis: pre_mover_basis,
    };
    if let Some(deferred) = output {
        if let Some(completion) = root_completion {
            let publication = completion.into_publication();
            if !manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .is_some_and(|entity| publication.authenticates(entity))
            {
                return dropped(
                    entity_id,
                    OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
                );
            }
            owner.root_publication = Some(publication);
            owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
            owner.pending_root_plan = None;
            owner.pending_root_task_visits = None;
            owner.pending_root_actor_common_axis = None;
            owner.pending_root_sub_a = None;
            owner.pending_root_dispatcher_continuation = None;
            return blocked(
                owner,
                OrdinaryType9GoToJobProductionBlock::RootDispatcherContinuationProducedTransition,
            );
        }
        if deferred.retryable {
            owner.state = OrdinaryType9GoToJobProductionState::RootTransitionPending {
                transition: deferred.transition,
                post_task_frame,
            };
            owner.pending_root_plan = deferred.plan;
            owner.pending_root_task_visits = Some(deferred.predecessor_task_visits);
            owner.pending_root_actor_common_axis = deferred.expected_predecessor_actor_common_axis;
            owner.pending_root_sub_a = deferred.expected_predecessor_sub_a;
            owner.pending_root_dispatcher_continuation = Some(deferred.dispatcher_continuation);
        } else {
            owner.state = OrdinaryType9GoToJobProductionState::CallbackFailurePending;
            owner.pending_root_plan = None;
            owner.pending_root_task_visits = None;
            owner.pending_root_actor_common_axis = None;
            owner.pending_root_sub_a = None;
            owner.pending_root_dispatcher_continuation = None;
        }
        return blocked(owner, deferred.reason);
    }
    if let Some(candidate_completion) = candidate_completion {
        let (publication, outcome) =
            fold_attract_attention_candidate_completion(entity_id, candidate_completion);
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
        owner.pending_root_actor_common_axis = None;
        owner.pending_root_sub_a = None;
        owner.pending_root_actor_animation = None;
        owner.pending_root_dispatcher_continuation = None;
        owner.root_publication = publication;
        return publish_post_task_basis(
            manager,
            owner,
            frame.resources,
            world_fx,
            &mut next_shared_random,
            post_task_frame,
            outcome,
        );
    }
    if let Some(mut completion) = root_completion {
        owner.pending_root_plan = None;
        owner.pending_root_task_visits = None;
        owner.pending_root_actor_common_axis = None;
        owner.pending_root_sub_a = None;
        owner.pending_root_dispatcher_continuation = None;
        if !completion.refresh_after_owned_dispatch(post_task_entity) {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        let (publication, outcome) = completion.into_parts(entity_id);
        if !manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .is_some_and(|entity| publication.authenticates(entity))
        {
            return dropped(
                entity_id,
                OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            );
        }
        owner.root_publication = Some(publication);
        return publish_post_task_basis(
            manager,
            owner,
            frame.resources,
            world_fx,
            &mut next_shared_random,
            post_task_frame,
            outcome,
        );
    }
    publish_post_task_basis(
        manager,
        owner,
        frame.resources,
        world_fx,
        &mut next_shared_random,
        post_task_frame,
        OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
            entity_id,
            callback_elapsed_micros,
        },
    )
}

#[derive(Debug, PartialEq, Eq)]
struct OrdinaryType9GoToJobDeferredRootTransition {
    transition: Option<OrdinaryType9GoToJobRootTransition>,
    plan: Option<OrdinaryType9RootReselectionPlan>,
    predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    expected_predecessor_actor_common_axis: Option<CommonAxisDescriptor>,
    expected_predecessor_sub_a: Option<SubAPropulsionRuntime>,
    dispatcher_continuation: OrdinaryType9GoToJobDispatcherContinuation,
    reason: OrdinaryType9GoToJobProductionBlock,
    retryable: bool,
}

enum OrdinaryType9GoToJobAttractAttentionCandidateCompletion {
    TargetRoute {
        target_id: u32,
        constructor: AttractAttentionTargetExecutionOutcome,
    },
    InitializerFallback {
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
        publication: OrdinaryType9GoToJobRootPublication,
    },
}

enum OrdinaryType9GoToJobRootCompletion {
    Wander {
        constructor: OrdinaryType9WanderConstructorEvidence,
        publication: OrdinaryType9GoToJobRootPublication,
    },
    InitializerFallback {
        failure: OrdinaryType9WanderInitializerFailure,
        publication: OrdinaryType9GoToJobRootPublication,
    },
    GoToJob {
        target_id: u32,
        constructor: OrdinaryType9GoToJobConstructorEvidence,
        publication: OrdinaryType9GoToJobRootPublication,
    },
    GoToJobInitializerFallback {
        target_id: u32,
        failure: OrdinaryType9GoToJobInitializerFailure,
        publication: OrdinaryType9GoToJobRootPublication,
    },
    RunAway {
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
        publication: OrdinaryType9GoToJobRootPublication,
    },
    RunAwayInitializerFallback {
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
        publication: OrdinaryType9GoToJobRootPublication,
    },
    AttractAttention {
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        publication: OrdinaryType9GoToJobRootPublication,
    },
    AttractAttentionInitializerFallback {
        failure: OrdinaryType9AttractAttentionInitializerFailure,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        publication: OrdinaryType9GoToJobRootPublication,
    },
}

impl OrdinaryType9GoToJobRootCompletion {
    fn refresh_after_owned_dispatch(&mut self, entity: &Entity) -> bool {
        match self {
            Self::AttractAttention {
                publication: OrdinaryType9GoToJobRootPublication::AttractAttention(publication),
                ..
            } => publication.refresh_after_owned_dispatch(entity),
            _ => true,
        }
    }

    fn into_publication(self) -> OrdinaryType9GoToJobRootPublication {
        match self {
            Self::Wander { publication, .. }
            | Self::InitializerFallback { publication, .. }
            | Self::GoToJob { publication, .. }
            | Self::GoToJobInitializerFallback { publication, .. }
            | Self::RunAway { publication, .. }
            | Self::RunAwayInitializerFallback { publication, .. }
            | Self::AttractAttention { publication, .. }
            | Self::AttractAttentionInitializerFallback { publication, .. } => publication,
        }
    }

    fn into_parts(
        self,
        entity_id: u32,
    ) -> (
        OrdinaryType9GoToJobRootPublication,
        OrdinaryType9GoToJobProductionOutcome,
    ) {
        match self {
            Self::Wander {
                constructor,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootWanderPublished {
                    entity_id,
                    constructor,
                },
            ),
            Self::InitializerFallback {
                failure,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootInitializerFallbackPublished {
                    entity_id,
                    failure,
                },
            ),
            Self::GoToJob {
                target_id,
                constructor,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootGoToJobPublished {
                    entity_id,
                    target_id,
                    constructor,
                },
            ),
            Self::GoToJobInitializerFallback {
                target_id,
                failure,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootGoToJobInitializerFallbackPublished {
                    entity_id,
                    target_id,
                    failure,
                },
            ),
            Self::RunAway {
                constructors_by_phase,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootRunAwayPublished {
                    entity_id,
                    constructors_by_phase,
                },
            ),
            Self::RunAwayInitializerFallback {
                failure,
                constructors_by_phase,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootRunAwayInitializerFallbackPublished {
                    entity_id,
                    failure,
                    constructors_by_phase,
                },
            ),
            Self::AttractAttention {
                committed,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                    entity_id,
                    committed,
                },
            ),
            Self::AttractAttentionInitializerFallback {
                failure,
                committed,
                publication,
            } => (
                publication,
                OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionInitializerFallbackPublished {
                    entity_id,
                    failure,
                    committed,
                },
            ),
        }
    }
}

struct SelectedGoToJobAdapter<'a, Random> {
    notifications: OrdinaryType9LiveNotificationContext<'a>,
    entity_id: u32,
    position_raw: [i16; 3],
    root_facts: OrdinaryType9RootWanderEntityFacts,
    expected_root_task_visits: [Option<ActorTaskVisit>; 3],
    metadata: Option<EntityTypeRuntimeMetadata>,
    terrain: Option<&'a v2k_formats::terrain::TerrainGrid>,
    expected_actor_common_axis_descriptor: Option<CommonAxisDescriptor>,
    actor_common_axis_descriptor: &'a mut RetailRuntimeValue<CommonAxisDescriptor>,
    snapshots: Vec<EntitySnapshot>,
    selected: &'a mut Option<OrdinaryType9SelectedComponentRuntime>,
    sub_a: &'a mut RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    animation: &'a mut RetailRuntimeValue<Option<ActorAnimationController>>,
    context: &'a mut BehaviorContextRuntime,
    heading: &'a mut f32,
    velocity: &'a mut [f32; 3],
    pre_mover_basis: RetailRuntimeValue<Type9BodyBasis>,
    elapsed_micros: u32,
    global_elapsed_micros: u32,
    scheduler_mode: i32,
    collision: &'a mut EntityCollisionRuntimeState,
    next_transaction_id: &'a mut u64,
    random: Rc<RefCell<&'a mut Random>>,
    allocate_root_wander:
        &'a mut dyn FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job:
        &'a mut dyn FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away:
        &'a mut dyn FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    allocate_root_attract_attention:
        &'a mut dyn FnMut(
            AttractAttentionInitialTaskPreparation,
        ) -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_attract_attention_target:
        &'a mut dyn FnMut(
            AttractAttentionTargetTaskPreparation,
        ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    root_run_away_audio: Option<RunAwayAuthoredAudio>,
    root_attract_attention_initial_owners:
        &'a mut Option<OrdinaryType9AttractAttentionInitialOwners>,
    root_attract_attention_target_route:
        &'a mut Option<OrdinaryType9AttractAttentionTargetRouteOwner>,
    attract_attention_candidate_completion:
        Option<OrdinaryType9GoToJobAttractAttentionCandidateCompletion>,
    root_completion: Option<OrdinaryType9GoToJobRootCompletion>,
}

impl<Random: FnMut() -> u32> SelectedGoToJobAdapter<'_, Random> {
    fn dispatcher_continuation(&self) -> OrdinaryType9GoToJobDispatcherContinuation {
        self.dispatcher_continuation_from(Some(ActorTaskSlot::Secondary))
    }

    fn dispatcher_continuation_from(
        &self,
        next_slot: Option<ActorTaskSlot>,
    ) -> OrdinaryType9GoToJobDispatcherContinuation {
        OrdinaryType9GoToJobDispatcherContinuation {
            entity_id: self.entity_id,
            snapshots: self.snapshots.clone(),
            root_facts: self.root_facts,
            root_owner_snapshot: root_entity_ref_from_parts(self.root_facts, self.collision),
            actor_common_axis_at_transition: *self.actor_common_axis_descriptor,
            next_slot,
            callback_elapsed_micros: self.elapsed_micros,
            global_elapsed_micros: self.global_elapsed_micros,
            scheduler_mode: self.scheduler_mode,
        }
    }

    fn target_route_target_state(
        &self,
        target_id: u32,
    ) -> Result<SharedTargetRouteTargetRuntimeState, OrdinaryType9GoToJobAdapterError> {
        let Some(target) = self.snapshots.iter().find(|target| target.id == target_id) else {
            return Ok(SharedTargetRouteTargetRuntimeState::Missing);
        };
        if !target.active {
            return Ok(SharedTargetRouteTargetRuntimeState::Inactive);
        }
        let state = target.collision.state_flags_at_0x08;
        if state.known_value_bits() == 0 {
            return if state.known_mask() == u32::MAX {
                Ok(SharedTargetRouteTargetRuntimeState::Inactive)
            } else {
                Err(OrdinaryType9GoToJobAdapterError::TargetStateUnresolved { target_id })
            };
        }
        match state.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(0) => Ok(SharedTargetRouteTargetRuntimeState::Live),
            RetailRuntimeValue::Known(_) => Ok(SharedTargetRouteTargetRuntimeState::Dying),
            RetailRuntimeValue::Unresolved => {
                Err(OrdinaryType9GoToJobAdapterError::TargetStateUnresolved { target_id })
            }
        }
    }

    fn target_route_predicate(
        &self,
        target_id: u32,
        expected: Option<CommonAxisDescriptor>,
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    ) -> Result<SharedTargetRoutePredicate, OrdinaryType9GoToJobAdapterError> {
        let expected =
            expected.ok_or(OrdinaryType9GoToJobAdapterError::RuntimeMetadataUnavailable)?;
        let controller = match actual {
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorUnavailable)
            }
            RetailRuntimeValue::Known(actual)
                if actual.strict_axis_limit_raw != expected.strict_axis_limit_raw =>
            {
                return Err(
                    OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorMismatch {
                        expected,
                        actual,
                    },
                )
            }
            RetailRuntimeValue::Known(actual) => actual,
        };
        let target_position_raw = self
            .snapshots
            .iter()
            .find(|target| target.id == target_id)
            .map(|target| target.position_raw)
            .ok_or(OrdinaryType9GoToJobAdapterError::TargetPositionUnavailable { target_id })?;
        let range = WrappedAxisRange::from_raw(controller.strict_axis_limit_raw);
        Ok(
            if within_wrapped_axis_range(range, self.position_raw, target_position_raw) {
                SharedTargetRoutePredicate::NonZero
            } else {
                SharedTargetRoutePredicate::Zero
            },
        )
    }

    fn route_controller_descriptor(
        expected: Option<CommonAxisDescriptor>,
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    ) -> Result<CommonAxisDescriptor, OrdinaryType9GoToJobAdapterError> {
        let expected =
            expected.ok_or(OrdinaryType9GoToJobAdapterError::RuntimeMetadataUnavailable)?;
        match actual {
            RetailRuntimeValue::Unresolved => {
                Err(OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorUnavailable)
            }
            RetailRuntimeValue::Known(actual)
                if actual.strict_axis_limit_raw != expected.strict_axis_limit_raw =>
            {
                Err(
                    OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorMismatch {
                        expected,
                        actual,
                    },
                )
            }
            RetailRuntimeValue::Known(actual) => Ok(actual),
        }
    }

    fn target_state(
        snapshots: &[EntitySnapshot],
        target_id: u32,
    ) -> Result<GoToJobTargetRuntimeState, OrdinaryType9GoToJobTargetError> {
        let Some(target) = snapshots.iter().find(|target| target.id == target_id) else {
            return Ok(GoToJobTargetRuntimeState::Missing);
        };
        if !target.active {
            return Ok(GoToJobTargetRuntimeState::Inactive);
        }
        let state = target.collision.state_flags_at_0x08;
        if state.known_value_bits() == 0 {
            return if state.known_mask() == u32::MAX {
                Ok(GoToJobTargetRuntimeState::Inactive)
            } else {
                Err(OrdinaryType9GoToJobTargetError::TargetStateUnresolved { target_id })
            };
        }
        match state.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(0) => Ok(GoToJobTargetRuntimeState::Live),
            RetailRuntimeValue::Known(_) => Ok(GoToJobTargetRuntimeState::Dying),
            RetailRuntimeValue::Unresolved => {
                Err(OrdinaryType9GoToJobTargetError::TargetStateUnresolved { target_id })
            }
        }
    }

    fn target_position(
        snapshots: &[EntitySnapshot],
        target_id: u32,
    ) -> Result<[i16; 3], OrdinaryType9GoToJobTargetError> {
        snapshots
            .iter()
            .find(|target| target.id == target_id)
            .map(|target| target.position_raw)
            .ok_or(OrdinaryType9GoToJobTargetError::TargetPositionUnavailable { target_id })
    }

    fn tracked_target(
        snapshots: &[EntitySnapshot],
        target_id: u32,
    ) -> Result<
        RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
        OrdinaryType9GoToJobTargetError,
    > {
        let Some(target) = snapshots.iter().find(|target| target.id == target_id) else {
            return Ok(RetailRuntimeValue::Known(None));
        };
        let state = target.collision.state_flags_at_0x08;
        Ok(RetailRuntimeValue::Known(Some(
            CommonMoverTrackedTargetSnapshot {
                state_flags: state,
                position_raw: target.position_raw,
                velocity_raw: target.velocity_raw,
            },
        )))
    }

    fn run_common_mover(
        &mut self,
        visit: ActorTaskVisit,
        private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, OrdinaryType9GoToJobAdapterError> {
        let tracked_target =
            Self::tracked_target(&self.snapshots, private_state.tracked_entity_handle)
                .map_err(OrdinaryType9GoToJobAdapterError::Target)?;
        let mut heading_raw = radians_to_binary_angle(*self.heading);
        let mut velocity_raw = world_position_raw(*self.velocity);
        let metadata = self
            .metadata
            .as_ref()
            .ok_or(OrdinaryType9GoToJobAdapterError::RuntimeMetadataUnavailable)?;
        let topology = OrdinaryType9Topology::from_metadata(9, metadata)
            .map_err(|_| OrdinaryType9GoToJobAdapterError::RuntimeTopologyUnavailable)?;
        let terrain = self
            .terrain
            .ok_or(OrdinaryType9GoToJobAdapterError::CurrentTerrainUnavailable)?;
        let selected = self
            .selected
            .as_mut()
            .ok_or(OrdinaryType9GoToJobAdapterError::SelectedComponentRuntimeUnavailable)?;
        let sub_a = match self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a))
                if sub_a.target_speed_raw() != RetailRuntimeValue::Unresolved =>
            {
                sub_a
            }
            RetailRuntimeValue::Known(Some(_)) => {
                return Err(OrdinaryType9GoToJobAdapterError::SubATargetSpeedUnresolved)
            }
            _ => return Err(OrdinaryType9GoToJobAdapterError::SubARuntimeUnavailable),
        };
        let mut animation = bind_go_to_job_animation(self.animation, self.scheduler_mode)?;
        let pre_mover_basis = match self.pre_mover_basis {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9GoToJobAdapterError::PhysicalBodyBasisUnavailable)
            }
        };
        let transaction_id = NonZeroU64::new(*self.next_transaction_id)
            .expect("production transaction ids never use zero");
        *self.next_transaction_id = self.next_transaction_id.wrapping_add(1).max(1);
        let entity_id = self.entity_id;
        let position_raw = self.position_raw;
        let elapsed_micros = self.elapsed_micros;
        let global_elapsed_micros = self.global_elapsed_micros;
        let scheduler_mode = self.scheduler_mode;
        let animation_policy = match selected.kind() {
            OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished => {
                ActorAbdiAnimationPolicy::AttractAttentionForcedStop
            }
            _ => ActorAbdiAnimationPolicy::Neutral,
        };
        let random = Rc::clone(&self.random);
        let result = run_selected_ordinary_type9_common_mover_with_animation_policy(
            OrdinaryType9SelectedCommonMoverRequest {
                transaction_id: OrdinaryType9TransactionId::from(transaction_id),
                lease: OrdinaryType9FrameLease {
                    controlled_entity_id: entity_id,
                    task_visit: visit,
                },
                topology,
                component_custody: OrdinaryType9SelectedCommonMoverComponentCustody::from_selected(
                    selected,
                ),
                staged_wander: private_state,
                sub_a_runtime: sub_a,
                actor_animation: animation.runtime_mut(),
                heading_raw: &mut heading_raw,
                velocity_raw: &mut velocity_raw,
                tracked_target,
                terrain,
                position_raw,
                pre_mover_basis,
                elapsed_micros,
                global_elapsed_micros,
                scheduler_mode,
            },
            animation_policy,
            || (&mut **random.borrow_mut())(),
        )
        .map_err(|error| match error {
            OrdinaryType9SelectedCommonMoverError::FrameBlocked(reason) => {
                OrdinaryType9GoToJobAdapterError::CommonMoverFrameBlocked(reason)
            }
            OrdinaryType9SelectedCommonMoverError::CommitBlocked(reason) => {
                OrdinaryType9GoToJobAdapterError::CommonMoverCommitBlocked(reason)
            }
        })?;
        if result == WanderNearCommonMoverReturn::NonZero {
            *self.heading = crate::entity::binary_angle_to_radians(heading_raw);
            *self.velocity = raw_position_world(velocity_raw);
        }
        Ok(result)
    }

    fn apply_fleeing_handoff(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        handoff: SearchAttackTargetHandoff,
    ) -> u32 {
        let selected = self
            .selected
            .as_mut()
            .expect("class-10 root publication retains selected-component custody");
        let RetailRuntimeValue::Known(actor_common_axis) = *self.actor_common_axis_descriptor
        else {
            unreachable!("class-10 root publication retains actor-axis custody")
        };
        let RetailRuntimeValue::Known(Some(sub_a)) = self.sub_a else {
            unreachable!("class-10 root publication retains Sub-A custody")
        };
        if let Some(OrdinaryType9GoToJobRootCompletion::RunAway {
            publication: OrdinaryType9GoToJobRootPublication::RunAway(publication),
            ..
        }) = self.root_completion.as_ref()
        {
            assert!(publication.authenticates_acquiring_handoff_parts(
                self.root_facts,
                self.collision,
                owner,
                *selected,
                RetailRuntimeValue::Known(actor_common_axis),
                *sub_a,
                *self.context,
            ));
        }

        let program = behavior_program(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID))
            .expect("Run Away behavior descriptor is statically audited");
        let style = audited_behavior_style(u32::from(RUN_AWAY_BEHAVIOR_CLASS_ID), 1)
            .expect("Run Away fleeing style is statically audited");
        *self.context = BehaviorContextRuntime::named_audited(
            program,
            1,
            self.context.choice_list_source(),
            RetailRuntimeValue::Known(Some(handoff.target_id)),
            self.context.auxiliary_word_at_0x0c(),
            *style,
        )
        .expect("the audited Run Away descriptor owns fleeing table index one");

        let metadata = self
            .metadata
            .as_ref()
            .expect("class-10 root preflight authenticated runtime metadata");
        let audio = self
            .root_run_away_audio
            .expect("class-10 root preflight authenticated authored audio");
        let random = &self.random;
        apply_run_away_task_setup(
            owner,
            RunAwayTaskSetupRequest::Fleeing {
                target_id: handoff.target_id,
                audio,
            },
            |preparation| {
                let prepared = prepare_run_away_runtime_task(
                    preparation,
                    self.entity_id,
                    self.position_raw,
                    metadata,
                )?;
                Ok::<_, crate::actor_task_dispatcher::RunAwayRuntimePreparationError>(
                    prepared.apply_suffix(
                        || (&mut **random.borrow_mut())(),
                        |effect| match effect {
                            RunAwayRuntimeConstructorEffect::Generic(
                                SharedGenericConstructorEffect::WriteSubHState08 { .. },
                            ) => unreachable!("ordinary Type-9 has no Sub-H constructor branch"),
                            RunAwayRuntimeConstructorEffect::Generic(
                                SharedGenericConstructorEffect::WriteSubADirection {
                                    direction_multiplier,
                                },
                            ) => sub_a.set_direction_multiplier(direction_multiplier),
                            RunAwayRuntimeConstructorEffect::Generic(
                                SharedGenericConstructorEffect::WriteSubATargetSpeed {
                                    target_speed_raw,
                                    ..
                                },
                            ) => {
                                sub_a.apply_shared_initializer_target_speed_write(target_speed_raw)
                            }
                            RunAwayRuntimeConstructorEffect::ApplyFixedSubATargetSpeed {
                                owner_id,
                                suffix,
                            } => {
                                assert_eq!(owner_id, self.entity_id);
                                sub_a.apply_shared_initializer_target_speed_write(
                                    suffix
                                        .sub_a_target_speed_raw()
                                        .expect("exact Type-9 topology retains Sub-A"),
                                );
                            }
                        },
                    ),
                )
            },
        )
        .expect("production preflight authenticates infallible fleeing publication");
        selected.set_kind(OrdinaryType9SelectedRuntimeKind::RunAwayFleeingPublished);
        if let Some(OrdinaryType9GoToJobRootCompletion::RunAway {
            publication: OrdinaryType9GoToJobRootPublication::RunAway(publication),
            ..
        }) = self.root_completion.as_mut()
        {
            publication.finish_fleeing_handoff_parts(owner, *selected, *sub_a, *self.context);
        }
        0
    }

    fn apply_root_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        transition: OrdinaryType9GoToJobRootTransition,
    ) -> Result<Option<OrdinaryType9GoToJobDeferredRootTransition>, OrdinaryType9GoToJobAdapterError>
    {
        if read_transition_suppressed(self.collision)? {
            return Ok(None);
        }
        let dispatcher_continuation = self.dispatcher_continuation();
        let expected_predecessor_actor_common_axis =
            match dispatcher_continuation.actor_common_axis_at_transition {
                RetailRuntimeValue::Known(axis) => Some(axis),
                RetailRuntimeValue::Unresolved => None,
            };
        let expected_predecessor_sub_a = match *self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a)) => Some(sub_a),
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => None,
        };
        let predecessor_task_visits = live_task_visits(owner);
        let visits_authenticate = predecessor_task_visits == self.expected_root_task_visits
            && predecessor_task_visits.into_iter().flatten().all(|visit| {
                owner.wrapper_flags(visit.task_id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            });
        if !visits_authenticate
            || !authenticates_root_transition_parts(
                self.entity_id,
                self.root_facts.pending_initial_selection_present,
                self.root_facts.death_component_custody_present,
                owner,
                *self.selected,
                RetailRuntimeValue::Known(Some(*self.context)),
                transition,
            )
        {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition: Some(transition),
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::RootTransitionRequestChanged,
                retryable: true,
            }));
        }
        let transition = Some(transition);
        let Some(active_model_id) = self.root_facts.active_model else {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::ActiveModelUnavailable,
                retryable: true,
            }));
        };
        let Some(metadata) = self.metadata.as_ref() else {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::RuntimeMetadataUnavailable,
                retryable: true,
            }));
        };
        let selected = self
            .selected
            .as_mut()
            .ok_or(OrdinaryType9GoToJobAdapterError::SelectedComponentRuntimeUnavailable)?;
        let Some(expected_sub_a) = expected_predecessor_sub_a else {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAUnavailable,
                retryable: true,
            }));
        };
        let sub_a = match self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                unreachable!("resolved predecessor Sub-A was captured from this same runtime")
            }
        };
        let owner_snapshot = root_entity_ref_from_parts(self.root_facts, self.collision);
        let mut candidates = self
            .snapshots
            .iter()
            .map(root_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        if let Some(controlled) = candidates.iter_mut().find(|item| item.id == self.entity_id) {
            *controlled = owner_snapshot;
        }
        let random = Rc::clone(&self.random);
        let mut selector_draws = 0_u32;
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id,
                metadata,
                owner: owner_snapshot,
                current_context: RetailRuntimeValue::Known(Some(*self.context)),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                selector_draws = selector_draws.wrapping_add(1);
                (&mut **random.borrow_mut())()
            },
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition,
                    plan: None,
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason: OrdinaryType9GoToJobProductionBlock::RootReselection(error),
                    retryable: selector_draws == 0,
                }));
            }
        };
        let selected_class_id = match plan.selection() {
            OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
            OrdinaryType9RootSelection::Weighted { selection, .. } => {
                u32::from(selection.program.class_id)
            }
        };
        if selected_class_id != u32::from(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
            && selected_class_id != LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
            && selected_class_id != LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID
            && selected_class_id != LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
        {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition,
                plan: Some(plan),
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::UnsupportedRootSelection {
                    class_id: selected_class_id,
                },
                retryable: true,
            }));
        }
        if selected_class_id == LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID {
            let Some(expected_actor_common_axis) = expected_predecessor_actor_common_axis else {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition,
                    plan: Some(plan),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason:
                        OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                    retryable: true,
                }));
            };
            let animation = match self.animation {
                RetailRuntimeValue::Known(Some(animation)) => animation,
                RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => {
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition,
                        plan: Some(plan),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootPredecessorActorAnimationUnavailable,
                        retryable: true,
                    }));
                }
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_attract_attention_from_go_to_job_parts(
                self.root_facts,
                metadata,
                plan,
                predecessor_task_visits,
                expected_actor_common_axis,
                expected_sub_a,
                *animation,
                OrdinaryType9RootAttractAttentionMutableParts {
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis_descriptor,
                    sub_a,
                    actor_animation: animation,
                    current_context: self.context,
                },
                &mut *self.allocate_root_attract_attention,
                || (&mut **random.borrow_mut())(),
                |request| self.notifications.dispatch(request),
                |_| {},
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootAttractAttentionPreflight(
                            reason,
                        ),
                        retryable: true,
                    }));
                }
            };
            self.root_completion = Some(match application {
                OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                    committed,
                    owners,
                    publication,
                } => {
                    *self.root_attract_attention_initial_owners = Some(owners);
                    OrdinaryType9GoToJobRootCompletion::AttractAttention {
                        committed,
                        publication: OrdinaryType9GoToJobRootPublication::AttractAttention(
                            publication,
                        ),
                    }
                }
                OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    committed,
                } => {
                    self.root_attract_attention_initial_owners.take();
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        *self.actor_common_axis_descriptor
                    else {
                        unreachable!("class-45 fallback retains actor-axis custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::AttractAttentionInitializerFallback {
                        failure,
                        committed,
                        publication:
                            OrdinaryType9GoToJobRootPublication::AttractAttentionInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *sub_a,
                                actor_animation: *animation,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        if selected_class_id == LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID {
            let Some(expected_actor_common_axis) = expected_predecessor_actor_common_axis else {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition,
                    plan: Some(plan),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason:
                        OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                    retryable: true,
                }));
            };
            let audio = match authored_run_away_audio(metadata) {
                Ok(audio) => audio,
                Err(reason) => {
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition,
                        plan: Some(plan),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason,
                        retryable: true,
                    }));
                }
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_run_away_from_go_to_job_parts(
                self.root_facts,
                metadata,
                plan,
                predecessor_task_visits,
                expected_actor_common_axis,
                expected_sub_a,
                OrdinaryType9RootRunAwayMutableParts {
                    actor_animation: self.animation,
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis_descriptor,
                    sub_a,
                    current_context: self.context,
                },
                &mut *self.allocate_root_run_away,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootRunAwayPreflight(reason),
                        retryable: true,
                    }));
                }
            };
            debug_assert!(self.root_completion.is_none());
            self.root_run_away_audio = Some(audio);
            self.root_completion = Some(match application {
                OrdinaryType9RootRunAwayApplicationOutcome::Published {
                    constructors_by_phase,
                    owner,
                } => OrdinaryType9GoToJobRootCompletion::RunAway {
                    constructors_by_phase,
                    publication: OrdinaryType9GoToJobRootPublication::RunAway(owner),
                },
                OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    constructors_by_phase,
                } => {
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        *self.actor_common_axis_descriptor
                    else {
                        unreachable!("initializer fallback retains actor-axis custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::RunAwayInitializerFallback {
                        failure,
                        constructors_by_phase,
                        publication:
                            OrdinaryType9GoToJobRootPublication::RunAwayInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        if selected_class_id == LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID {
            let Some(expected_actor_common_axis) = expected_predecessor_actor_common_axis else {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition,
                    plan: Some(plan),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason:
                        OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                    retryable: true,
                }));
            };
            let candidate_evidence = go_to_job_candidate_evidence(&self.snapshots);
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_go_to_job_parts(
                self.root_facts,
                metadata,
                plan,
                predecessor_task_visits,
                expected_actor_common_axis,
                expected_sub_a,
                &candidate_evidence,
                OrdinaryType9RootGoToJobMutableParts {
                    actor_animation: self.animation,
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis_descriptor,
                    sub_a,
                    current_context: self.context,
                },
                &mut *self.allocate_root_go_to_job,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootGoToJobPreflight(reason),
                        retryable: true,
                    }));
                }
            };
            debug_assert!(self.root_completion.is_none());
            self.root_completion = Some(match application {
                OrdinaryType9RootGoToJobApplicationOutcome::Published {
                    target_id,
                    constructor,
                    owner,
                } => OrdinaryType9GoToJobRootCompletion::GoToJob {
                    target_id,
                    constructor,
                    publication: OrdinaryType9GoToJobRootPublication::GoToJob(owner),
                },
                OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
                    target_id,
                    failure,
                } => {
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::GoToJobInitializerFallback {
                        target_id,
                        failure,
                        publication:
                            OrdinaryType9GoToJobRootPublication::GoToJobInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis: expected_actor_common_axis,
                                sub_a: *sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        let random = Rc::clone(&self.random);
        let application = apply_ordinary_type9_root_wander_parts(
            self.root_facts,
            metadata,
            plan,
            predecessor_task_visits,
            OrdinaryType9RootWanderMutableParts {
                actor_animation: self.animation,
                collision: self.collision,
                actor_tasks: owner,
                selected_runtime: selected,
                sub_a,
                current_context: self.context,
            },
            &mut *self.allocate_root_wander,
            || (&mut **random.borrow_mut())(),
        );
        let application = match application {
            Ok(application) => application,
            Err(failure) => {
                let reason = failure.error.clone();
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition,
                    plan: Some(failure.into_plan()),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason: OrdinaryType9GoToJobProductionBlock::RootWanderPreflight(reason),
                    retryable: true,
                }));
            }
        };
        self.root_completion = Some(match application {
            OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } => {
                OrdinaryType9GoToJobRootCompletion::Wander {
                    constructor,
                    publication: OrdinaryType9GoToJobRootPublication::Wander(owner),
                }
            }
            OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                OrdinaryType9GoToJobRootCompletion::InitializerFallback {
                    failure,
                    publication: OrdinaryType9GoToJobRootPublication::InitializerFallback {
                        entity_id: self.entity_id,
                        context: *self.context,
                        selected_runtime: *selected,
                        initial_behavior: self.root_facts.initial_behavior,
                        sub_a: *sub_a,
                        state_policy_mask: policy.set_bits | policy.clear_bits,
                        state_policy_bits: policy.set_bits,
                    },
                }
            }
        });
        Ok(None)
    }

    fn apply_attract_origin_root_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        next_slot: Option<ActorTaskSlot>,
    ) -> Result<Option<OrdinaryType9GoToJobDeferredRootTransition>, OrdinaryType9GoToJobAdapterError>
    {
        let predecessor_task_visits = live_task_visits(owner);
        let dispatcher_continuation = self.dispatcher_continuation_from(next_slot);
        let expected_predecessor_actor_common_axis =
            match dispatcher_continuation.actor_common_axis_at_transition {
                RetailRuntimeValue::Known(axis) => Some(axis),
                RetailRuntimeValue::Unresolved => None,
            };
        let expected_predecessor_sub_a = match *self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a)) => Some(sub_a),
            RetailRuntimeValue::Known(None) | RetailRuntimeValue::Unresolved => None,
        };
        let Some(active_model_id) = self.root_facts.active_model else {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition: None,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::ActiveModelUnavailable,
                retryable: true,
            }));
        };
        let Some(metadata) = self.metadata.as_ref() else {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition: None,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::RuntimeMetadataUnavailable,
                retryable: true,
            }));
        };
        let selected = self
            .selected
            .as_mut()
            .ok_or(OrdinaryType9GoToJobAdapterError::SelectedComponentRuntimeUnavailable)?;
        let Some(expected_sub_a) = expected_predecessor_sub_a else {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition: None,
                plan: None,
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAUnavailable,
                retryable: true,
            }));
        };
        let sub_a = match self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
            _ => unreachable!("resolved predecessor Sub-A was captured from this same runtime"),
        };
        let owner_snapshot = root_entity_ref_from_parts(self.root_facts, self.collision);
        let mut candidates = self
            .snapshots
            .iter()
            .map(root_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        if let Some(controlled) = candidates.iter_mut().find(|item| item.id == self.entity_id) {
            *controlled = owner_snapshot;
        }
        let random = Rc::clone(&self.random);
        let mut selector_draws = 0_u32;
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id,
                metadata,
                owner: owner_snapshot,
                current_context: RetailRuntimeValue::Known(Some(*self.context)),
                candidates_in_intrusive_order: &candidates,
            },
            || {
                selector_draws = selector_draws.wrapping_add(1);
                (&mut **random.borrow_mut())()
            },
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition: None,
                    plan: None,
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason: OrdinaryType9GoToJobProductionBlock::RootReselection(error),
                    retryable: selector_draws == 0,
                }));
            }
        };
        let selected_class_id = match plan.selection() {
            OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
            OrdinaryType9RootSelection::Weighted { selection, .. } => {
                u32::from(selection.program.class_id)
            }
        };
        if selected_class_id == LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID {
            let Some(expected_actor_common_axis) = expected_predecessor_actor_common_axis else {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition: None,
                    plan: Some(plan),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason:
                        OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                    retryable: true,
                }));
            };
            let animation = match self.animation {
                RetailRuntimeValue::Known(Some(animation)) => animation,
                _ => {
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition: None,
                        plan: Some(plan),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootPredecessorActorAnimationUnavailable,
                        retryable: true,
                    }));
                }
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_attract_attention_parts(
                self.root_facts,
                metadata,
                plan,
                predecessor_task_visits,
                expected_actor_common_axis,
                expected_sub_a,
                *animation,
                OrdinaryType9RootAttractAttentionMutableParts {
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis_descriptor,
                    sub_a,
                    actor_animation: animation,
                    current_context: self.context,
                },
                &mut *self.allocate_root_attract_attention,
                || (&mut **random.borrow_mut())(),
                |request| self.notifications.dispatch(request),
                |_| {},
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition: None,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootAttractAttentionPreflight(
                            reason,
                        ),
                        retryable: true,
                    }));
                }
            };
            self.root_completion = Some(match application {
                OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                    committed,
                    owners,
                    publication,
                } => {
                    *self.root_attract_attention_initial_owners = Some(owners);
                    OrdinaryType9GoToJobRootCompletion::AttractAttention {
                        committed,
                        publication: OrdinaryType9GoToJobRootPublication::AttractAttention(
                            publication,
                        ),
                    }
                }
                OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    committed,
                } => {
                    self.root_attract_attention_initial_owners.take();
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        *self.actor_common_axis_descriptor
                    else {
                        unreachable!("class-45 fallback retains actor-axis custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9GoToJobRootCompletion::AttractAttentionInitializerFallback {
                        failure,
                        committed,
                        publication:
                            OrdinaryType9GoToJobRootPublication::AttractAttentionInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *sub_a,
                                actor_animation: *animation,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        if selected_class_id == LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID {
            let Some(expected_actor_common_axis) = expected_predecessor_actor_common_axis else {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition: None,
                    plan: Some(plan),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason:
                        OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                    retryable: true,
                }));
            };
            let candidate_evidence = go_to_job_candidate_evidence(&self.snapshots);
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_go_to_job_from_attract_attention_parts(
                self.root_facts,
                metadata,
                plan,
                predecessor_task_visits,
                expected_actor_common_axis,
                expected_sub_a,
                &candidate_evidence,
                OrdinaryType9RootGoToJobMutableParts {
                    actor_animation: self.animation,
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis_descriptor,
                    sub_a,
                    current_context: self.context,
                },
                &mut *self.allocate_root_go_to_job,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition: None,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootGoToJobPreflight(reason),
                        retryable: true,
                    }));
                }
            };
            self.root_completion = Some(match application {
                OrdinaryType9RootGoToJobApplicationOutcome::Published {
                    target_id,
                    constructor,
                    owner,
                } => OrdinaryType9GoToJobRootCompletion::GoToJob {
                    target_id,
                    constructor,
                    publication: OrdinaryType9GoToJobRootPublication::GoToJob(owner),
                },
                OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
                    target_id,
                    failure,
                } => {
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        *self.actor_common_axis_descriptor
                    else {
                        unreachable!("class-54 preflight retained actor-axis custody")
                    };
                    OrdinaryType9GoToJobRootCompletion::GoToJobInitializerFallback {
                        target_id,
                        failure,
                        publication:
                            OrdinaryType9GoToJobRootPublication::GoToJobInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        if selected_class_id == LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID {
            let Some(expected_actor_common_axis) = expected_predecessor_actor_common_axis else {
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition: None,
                    plan: Some(plan),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason:
                        OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                    retryable: true,
                }));
            };
            let audio = match authored_run_away_audio(metadata) {
                Ok(audio) => audio,
                Err(reason) => {
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition: None,
                        plan: Some(plan),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason,
                        retryable: true,
                    }));
                }
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_run_away_from_attract_attention_parts(
                self.root_facts,
                metadata,
                plan,
                predecessor_task_visits,
                expected_actor_common_axis,
                expected_sub_a,
                OrdinaryType9RootRunAwayMutableParts {
                    actor_animation: self.animation,
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis_descriptor,
                    sub_a,
                    current_context: self.context,
                },
                &mut *self.allocate_root_run_away,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(application) => application,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                        transition: None,
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        expected_predecessor_actor_common_axis: Some(expected_actor_common_axis),
                        expected_predecessor_sub_a: Some(expected_sub_a),
                        dispatcher_continuation,
                        reason: OrdinaryType9GoToJobProductionBlock::RootRunAwayPreflight(reason),
                        retryable: true,
                    }));
                }
            };
            debug_assert!(self.root_completion.is_none());
            self.root_run_away_audio = Some(audio);
            self.root_completion = Some(match application {
                OrdinaryType9RootRunAwayApplicationOutcome::Published {
                    constructors_by_phase,
                    owner,
                } => OrdinaryType9GoToJobRootCompletion::RunAway {
                    constructors_by_phase,
                    publication: OrdinaryType9GoToJobRootPublication::RunAway(owner),
                },
                OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    constructors_by_phase,
                } => {
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    let RetailRuntimeValue::Known(actor_common_axis) =
                        *self.actor_common_axis_descriptor
                    else {
                        unreachable!("class-10 preflight retained actor-axis custody")
                    };
                    OrdinaryType9GoToJobRootCompletion::RunAwayInitializerFallback {
                        failure,
                        constructors_by_phase,
                        publication:
                            OrdinaryType9GoToJobRootPublication::RunAwayInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        if selected_class_id != u32::from(LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID) {
            return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                transition: None,
                plan: Some(plan),
                predecessor_task_visits,
                expected_predecessor_actor_common_axis,
                expected_predecessor_sub_a,
                dispatcher_continuation,
                reason: OrdinaryType9GoToJobProductionBlock::UnsupportedRootSelection {
                    class_id: selected_class_id,
                },
                retryable: true,
            }));
        }
        let random = Rc::clone(&self.random);
        let application = apply_ordinary_type9_root_wander_parts(
            self.root_facts,
            metadata,
            plan,
            predecessor_task_visits,
            OrdinaryType9RootWanderMutableParts {
                actor_animation: self.animation,
                collision: self.collision,
                actor_tasks: owner,
                selected_runtime: selected,
                sub_a,
                current_context: self.context,
            },
            &mut *self.allocate_root_wander,
            || (&mut **random.borrow_mut())(),
        );
        let application = match application {
            Ok(application) => application,
            Err(failure) => {
                let reason = failure.error.clone();
                return Ok(Some(OrdinaryType9GoToJobDeferredRootTransition {
                    transition: None,
                    plan: Some(failure.into_plan()),
                    predecessor_task_visits,
                    expected_predecessor_actor_common_axis,
                    expected_predecessor_sub_a,
                    dispatcher_continuation,
                    reason: OrdinaryType9GoToJobProductionBlock::RootWanderPreflight(reason),
                    retryable: true,
                }));
            }
        };
        self.root_completion = Some(match application {
            OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } => {
                OrdinaryType9GoToJobRootCompletion::Wander {
                    constructor,
                    publication: OrdinaryType9GoToJobRootPublication::Wander(owner),
                }
            }
            OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                OrdinaryType9GoToJobRootCompletion::InitializerFallback {
                    failure,
                    publication: OrdinaryType9GoToJobRootPublication::InitializerFallback {
                        entity_id: self.entity_id,
                        context: *self.context,
                        selected_runtime: *selected,
                        initial_behavior: self.root_facts.initial_behavior,
                        sub_a: *sub_a,
                        state_policy_mask: policy.set_bits | policy.clear_bits,
                        state_policy_bits: policy.set_bits,
                    },
                }
            }
        });
        Ok(None)
    }
}

impl<Random: FnMut() -> u32> ActorTaskDispatcherAdapter for SelectedGoToJobAdapter<'_, Random> {
    type Output = OrdinaryType9GoToJobDeferredRootTransition;
    type Error = OrdinaryType9GoToJobAdapterError;

    fn ordinary_wander_anchor_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        unreachable!("selected Go To Job preflight rejects ordinary Wander")
    }

    fn next_random(&mut self) -> u32 {
        (&mut **self.random.borrow_mut())()
    }

    fn attract_attention_candidate_prefix(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        task_state: crate::attract_attention::AttractAttentionCandidateTaskState,
        random: u32,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
        Self::Error,
    > {
        let Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, .. }) =
            self.root_attract_attention_initial_owners.as_mut()
        else {
            return Err(OrdinaryType9GoToJobAdapterError::AttractAttentionCandidateLeaseChanged);
        };
        let prefix = candidate
            .commit_prevalidated_dispatch_prefix(visit, task_state, random)
            .map_err(OrdinaryType9GoToJobAdapterError::AttractAttentionCandidate)?;
        if let crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix::Acquire {
            filter_write: Some(filter),
            ..
        } = prefix
        {
            publish_candidate_dispatch_filter(self.actor_common_axis_descriptor, filter);
        }
        Ok(prefix)
    }

    fn attract_attention_candidate_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        prefix: crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackResult,
        Self::Error,
    > {
        let RetailRuntimeValue::Known(Some(sub_a)) = self.sub_a else {
            return Err(OrdinaryType9GoToJobAdapterError::SubARuntimeUnavailable);
        };
        if self.selected.is_none() {
            return Err(OrdinaryType9GoToJobAdapterError::SelectedComponentRuntimeUnavailable);
        }
        let RetailRuntimeValue::Known(Some(actor_animation)) = self.animation else {
            return Err(OrdinaryType9GoToJobAdapterError::ActorAnimationUnavailable);
        };
        let Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue }) =
            self.root_attract_attention_initial_owners.take()
        else {
            return Err(OrdinaryType9GoToJobAdapterError::AttractAttentionCandidateLeaseChanged);
        };
        *self.root_attract_attention_initial_owners =
            Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue });
        let selected = self
            .selected
            .as_mut()
            .expect("selected storage checked before linear owners moved");
        let candidate_refs = self
            .snapshots
            .iter()
            .map(guard_location_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        let random = Rc::clone(&self.random);
        let result = candidate.dispatch_callback_parts(
            OrdinaryType9AttractAttentionCandidateDispatchParts {
                actor_animation,
                actor_common_axis: self.actor_common_axis_descriptor,
                entity_id: self.entity_id,
                entity_type: self.root_facts.entity_type,
                position_raw: self.position_raw,
                capability_flags: self.root_facts.capability_flags,
                collision: self.collision,
                actor_tasks: owner,
                selected,
                sub_a,
                context: self.context,
            },
            visit,
            prefix,
            &candidate_refs,
            &mut *self.allocate_attract_attention_target,
            || (&mut **random.borrow_mut())(),
        );
        let dispatch = match result {
            Ok(dispatch) => dispatch,
            Err(failure) => {
                let error = failure.error.clone();
                let candidate = failure.into_owner();
                let cue =
                    match self.root_attract_attention_initial_owners.take() {
                        Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue })
                        | Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                            cue,
                            ..
                        }) => cue,
                        None => return Err(
                            OrdinaryType9GoToJobAdapterError::AttractAttentionCandidateLeaseChanged,
                        ),
                    };
                *self.root_attract_attention_initial_owners = Some(
                    OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue },
                );
                return Err(OrdinaryType9GoToJobAdapterError::AttractAttentionCandidate(
                    error,
                ));
            }
        };
        match dispatch {
            OrdinaryType9AttractAttentionCandidateDispatch::Pending { owner, result } => {
                let Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue }) =
                    self.root_attract_attention_initial_owners.take()
                else {
                    return Err(OrdinaryType9GoToJobAdapterError::AttractAttentionCueLeaseChanged);
                };
                *self.root_attract_attention_initial_owners = Some(
                    OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                        candidate: owner,
                        cue,
                    },
                );
                Ok(result)
            }
            OrdinaryType9AttractAttentionCandidateDispatch::TargetRoutePublished {
                target,
                owner,
                constructor,
                result,
            } => {
                let _retired_cue = self.root_attract_attention_initial_owners.take();
                *self.root_attract_attention_target_route = Some(owner);
                self.attract_attention_candidate_completion = Some(
                    OrdinaryType9GoToJobAttractAttentionCandidateCompletion::TargetRoute {
                        target_id: target.id,
                        constructor,
                    },
                );
                Ok(result)
            }
            OrdinaryType9AttractAttentionCandidateDispatch::InitializerFallbackPublished {
                failure,
                result,
                ..
            } => {
                let _retired_cue = self.root_attract_attention_initial_owners.take();
                self.root_attract_attention_target_route.take();
                let RetailRuntimeValue::Known(actor_common_axis) =
                    *self.actor_common_axis_descriptor
                else {
                    return Err(
                        OrdinaryType9GoToJobAdapterError::AttractAttentionCandidateLeaseChanged,
                    );
                };
                let RetailRuntimeValue::Known(Some(actor_animation)) = self.animation else {
                    return Err(OrdinaryType9GoToJobAdapterError::ActorAnimationUnavailable);
                };
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                self.attract_attention_candidate_completion = Some(
                    OrdinaryType9GoToJobAttractAttentionCandidateCompletion::InitializerFallback {
                        failure,
                        publication:
                            OrdinaryType9GoToJobRootPublication::AttractAttentionInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected_runtime: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis,
                                sub_a: *sub_a,
                                actor_animation: *actor_animation,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    },
                );
                Ok(result)
            }
        }
    }

    fn attract_attention_candidate_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("Go-To-Job Candidate returns zero after synchronous publication")
    }

    fn attract_attention_cue_transition_owner_id(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        _prefix: crate::ordinary_type9_attract_attention_cue::OrdinaryType9AttractAttentionCueCallbackPrefix,
    ) -> Result<u32, Self::Error> {
        let Some(owners) = self.root_attract_attention_initial_owners.as_ref() else {
            return Err(OrdinaryType9GoToJobAdapterError::AttractAttentionCueLeaseChanged);
        };
        if owners.cue().visit() != visit {
            return Err(OrdinaryType9GoToJobAdapterError::AttractAttentionCueLeaseChanged);
        }
        Ok(self.entity_id)
    }

    fn attract_attention_cue_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: crate::ordinary_type9_attract_attention_cue::OrdinaryType9AttractAttentionCueTransition,
    ) -> Result<AttractAttentionCueTransitionOutcome<Self::Output>, Self::Error> {
        if read_transition_suppressed(self.collision)? {
            return Ok(AttractAttentionCueTransitionOutcome::SuppressedByEntityState);
        }
        debug_assert_eq!(slot_after(request.expired_visit.slot), None);
        let deferred = self.apply_attract_origin_root_transition(owner, None)?;
        Ok(AttractAttentionCueTransitionOutcome::Completed(deferred))
    }

    fn ordinary_wander_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        unreachable!("selected Go To Job preflight rejects ordinary Wander")
    }

    fn ordinary_wander_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionRequest,
    ) -> Result<
        crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("selected Go To Job preflight rejects ordinary Wander")
    }

    fn go_to_job_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        stage: &mut GoToJobCallbackStage,
    ) -> Result<GoToJobCallbackResult, Self::Error> {
        let snapshots = self.snapshots.clone();
        let owner_position_raw = self.position_raw;
        let expected_route_controller = self.expected_actor_common_axis_descriptor;
        let actor_route_controller = *self.actor_common_axis_descriptor;
        let mut movement = ();
        let mut controller_context = ();
        evaluate_go_to_job_callback(
            stage,
            GoToJobCallbackRequest {
                visit,
                entity_id: self.entity_id,
                movement_state: &mut movement,
                controller_context: &mut controller_context,
                elapsed_micros: self.elapsed_micros,
                scheduler_mode: self.scheduler_mode as u32,
            },
            |target_id| Self::target_state(&snapshots, target_id),
            |request| {
                let controller = Self::route_controller_descriptor(
                    expected_route_controller,
                    actor_route_controller,
                )?;
                let target_position_raw = Self::target_position(&snapshots, request.target_id)
                    .map_err(OrdinaryType9GoToJobAdapterError::Target)?;
                let route_range = WrappedAxisRange::from_raw(controller.strict_axis_limit_raw);
                Ok(
                    if within_wrapped_axis_range(
                        route_range,
                        owner_position_raw,
                        target_position_raw,
                    ) {
                        GoToJobRoutePredicate::NonZero
                    } else {
                        GoToJobRoutePredicate::Zero
                    },
                )
            },
            |request| {
                self.run_common_mover(request.visit, request.target_state)
                    .map(|result| match result {
                        WanderNearCommonMoverReturn::Zero => GoToJobCommonMoverReturn::Zero,
                        WanderNearCommonMoverReturn::NonZero => GoToJobCommonMoverReturn::NonZero,
                        WanderNearCommonMoverReturn::Unresolved => unreachable!(
                            "selected common-mover binding never fabricates unresolved return"
                        ),
                    })
            },
        )
        .map_err(|error| match error {
            GoToJobCallbackError::TargetValidation { error, .. } => {
                OrdinaryType9GoToJobAdapterError::Target(error)
            }
            GoToJobCallbackError::RoutePredicate { error, .. } => error,
            GoToJobCallbackError::CommonMover { error, .. } => error,
        })
    }

    fn go_to_job_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: GoToJobTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        let Some(ActorTaskRuntime::GoToJob(task_state)) = owner.task_state(request.task_id) else {
            unreachable!("dispatcher authenticates the surviving Go To Job wrapper")
        };
        self.apply_root_transition(
            owner,
            OrdinaryType9GoToJobRootTransition {
                entity_id: self.entity_id,
                request,
                task_state: *task_state,
                selected_runtime: self
                    .selected
                    .expect("the surviving Go-To-Job task retains selected custody"),
            },
        )
    }

    fn chase_target_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _stage: &mut crate::chase_target::ChaseTargetCallbackStage,
    ) -> Result<crate::chase_target::ChaseTargetCallbackResult, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Chase Target")
    }

    fn chase_target_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::chase_target::ChaseTargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Chase Target")
    }

    fn target_acquisition_callback(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        prefix: TargetAcquisitionCallbackPrefix,
    ) -> Result<TargetAcquisitionCallbackResult, Self::Error> {
        let snapshots = self.snapshots.clone();
        let refs = snapshots
            .iter()
            .map(|entity| SearchAttackEntityRef {
                id: entity.id,
                entity_type: entity.entity_type,
                position_raw: entity.position_raw,
                capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                collision: &entity.collision,
            })
            .collect::<Vec<_>>();
        let owner_ref = refs
            .iter()
            .copied()
            .find(|candidate| candidate.id == self.entity_id)
            .expect("the controlled actor is present in its intrusive-list snapshot");
        evaluate_target_acquisition_callback(
            prefix,
            owner_ref,
            &refs,
            Some(|handoff| self.apply_fleeing_handoff(owner, handoff)),
        )
        .map_err(OrdinaryType9GoToJobAdapterError::TargetAcquisition)
    }

    fn target_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("Run Away C7D0/C6B0 returns zero after synchronous publication")
    }

    fn follow_beacon_acquisition_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _prefix: crate::follow_beacons::FollowBeaconAcquisitionCallbackPrefix,
    ) -> Result<crate::follow_beacons::FollowBeaconAcquisitionCallbackResult, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Follow Beacons")
    }

    fn follow_beacon_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("selected Go To Job preflight rejects Follow Beacons")
    }

    fn follow_beacons_following_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _stage: &mut crate::follow_beacons::FollowBeaconsFollowingCallbackStage,
    ) -> Result<
        crate::follow_beacons::FollowBeaconsFollowingCallbackResult,
        FollowBeaconsFollowingAdapterError<Self::Error>,
    > {
        unreachable!("selected Go To Job preflight rejects Follow Beacons")
    }

    fn follow_beacons_following_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::follow_beacons::FollowBeaconsFollowingTransitionRequest,
    ) -> Result<
        crate::actor_task_dispatcher::FollowBeaconsFollowingTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("selected Go To Job preflight rejects Follow Beacons")
    }

    fn follow_beacons_following_style_result(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::FollowBeaconsFollowingStyleResultRequest,
    ) -> Result<FollowBeaconsFollowingStyleResultOutcome<Self::Output>, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Follow Beacons")
    }

    fn aim_and_fire_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _state: crate::aim_and_fire::AimAndFirePrivateState,
    ) -> Result<crate::aim_and_fire::AimAndFireCallbackResult, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Aim and Fire")
    }

    fn aim_and_fire_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::AimAndFireTransitionRequest,
    ) -> Result<crate::actor_task_dispatcher::AimAndFireTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("selected Go To Job preflight rejects Aim and Fire")
    }

    fn aim_and_fire_propagated_result(&mut self, _result: std::num::NonZeroU32) -> Self::Output {
        unreachable!("selected Go To Job preflight rejects Aim and Fire")
    }

    fn guard_location_acquisition_prefix(
        &mut self,
        _visit: ActorTaskVisit,
        _state: crate::guard_location_owner::acquisition::GuardLocationAcquisitionTaskState,
        _random: u32,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
        Self::Error,
    > {
        unreachable!("selected Go To Job preflight rejects Guard Location")
    }

    fn guard_location_acquisition_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _prefix: crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix,
    ) -> Result<
        crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackResult,
        Self::Error,
    > {
        unreachable!("selected Go To Job preflight rejects Guard Location")
    }

    fn guard_location_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("selected Go To Job preflight rejects Guard Location")
    }

    fn run_away_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _stage: &mut crate::run_away::RunAwayCallbackStage,
    ) -> Result<crate::run_away::RunAwayCallbackResult, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Run Away")
    }

    fn run_away_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::run_away::RunAwayTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Run Away")
    }

    fn shared_retarget_actor_position_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        self.position_raw
    }

    fn shared_retarget_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        self.run_common_mover(visit, private_state)
    }

    fn shared_retarget_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: crate::shared_retarget_mover::SharedRetargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        if read_transition_suppressed(self.collision)? {
            return Ok(None);
        }
        debug_assert_eq!(slot_after(request.slot), Some(ActorTaskSlot::Secondary));
        self.apply_attract_origin_root_transition(owner, Some(ActorTaskSlot::Secondary))
    }

    fn defecate_virus_terrain_request(
        &mut self,
        _visit: ActorTaskVisit,
        _elapsed_micros: u32,
    ) -> Result<crate::defecate_virus::DefecateVirusCallbackRequest, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Defecate Virus")
    }

    fn apply_defecate_virus_terrain_plan(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _plan: crate::defecate_virus::DefecateVirusCallbackPlan,
    ) -> Result<(), Self::Error> {
        unreachable!("selected Go To Job preflight rejects Defecate Virus")
    }

    fn defecate_virus_terrain_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::defecate_virus::DefecateVirusTerrainTransitionRequest,
    ) -> Result<crate::defecate_virus::DefecateVirusTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("selected Go To Job preflight rejects Defecate Virus")
    }

    fn defecate_virus_wander_actor_position_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        unreachable!("selected Go To Job preflight rejects Defecate Virus")
    }

    fn defecate_virus_wander_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Defecate Virus")
    }

    fn defecate_virus_wander_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::defecate_virus::DefecateVirusWanderTransitionRequest,
    ) -> Result<crate::defecate_virus::DefecateVirusTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("selected Go To Job preflight rejects Defecate Virus")
    }

    fn common_dying_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
    ) -> Result<crate::common_dying::CommonDyingCallbackResult, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Common Dying")
    }

    fn common_dying_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::CommonDyingTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("selected Go To Job preflight rejects Common Dying")
    }
}

#[cfg(test)]
mod tests {
    mod candidate_followup;

    use std::collections::VecDeque;

    use super::*;
    use crate::{
        base_factory_progression::ProgressiveDeathState,
        entity::{exact_level_one_type9_go_to_job_manager, BaseFactoryRuntimeState, EntityKind},
        entity_collision_state::{
            RetailStateWord, FULLY_ABOVE_SURFACE_STATE_BIT, SURFACE_STATE_MASK,
        },
        level::LevelState,
        main_base_type9_abort::{exact_level_one_type9_metadata, LEVEL_ONE_TYPE9_ENTITY_TYPE},
        ordinary_type9_attract_attention_handoff::OrdinaryType9AttractAttentionTargetRoutePreflightError,
        ordinary_type9_initial_production::FreshLevel1Type9InitialProductionOwner,
        ordinary_type9_initial_selection::{
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK,
            LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK,
            LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK,
        },
    };
    use v2k_formats::{
        collision::StatusComponentDescriptor,
        terrain::{TerrainCell, TerrainGrid, GRID_SIZE},
    };

    const RESELECTED_BASE_ID: u32 = 0x04AD_0002;
    const CLOSER_RESELECTED_BASE_ID: u32 = 0x04AD_0003;
    const RUN_AWAY_TARGET_ID: u32 = 0x04AE_0001;
    const ROOT_PLAYER_ID: u32 = 0x04AE_0002;
    const ATTRACT_ROOT_SELECTOR_WORD: u32 = 3_063;

    fn terrain_resources() -> ResourceCache {
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: 0,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let mut resources = ResourceCache::new(Vec::new());
        resources.load_level(LevelState {
            source_path: "go-to-job-controller-test.ovl".into(),
            system_level: None,
            fixup_data: None,
            fixup_code: None,
            strings: Vec::new(),
            sprites: None,
            params: None,
            display_modes: None,
            fog_gradient: None,
            color_palettes: None,
            models: None,
            anim_frames: None,
            terrain: Some(terrain),
            anim_sound: None,
            collision: None,
            level: None,
            linkage: None,
        });
        resources
    }

    fn production_owner_fixture() -> (
        EntityManager,
        OrdinaryType9GoToJobProductionOwner,
        u32,
        CommonAxisDescriptor,
    ) {
        let mut manager = exact_level_one_type9_go_to_job_manager();
        let entity_id = manager
            .fresh_level1_type9_initial_productions()
            .iter()
            .find(|owner| owner.successful_go_to_job_selection().is_some())
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("exact fixture publishes Go-To-Job");
        // The production tail's retained E370 input is an explicit
        // precondition rather than a constructor guess.
        manager
            .entity_mut_for_test(entity_id)
            .expect("fixture actor remains live")
            .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        manager
            .entity_mut_for_test(entity_id)
            .expect("fixture actor remains live")
            .collision
            .state_flags_at_0x08
            .overwrite(SURFACE_STATE_MASK, FULLY_ABOVE_SURFACE_STATE_BIT);
        let actor_lease = manager
            .ordinary_type9_selected_actor_lease(entity_id)
            .expect("exact fixture has a live actor lease");
        let expected = manager
            .type_runtime_metadata(9)
            .and_then(|metadata| metadata.initializer.as_ref())
            .map(|initializer| initializer.common_axis_descriptor)
            .expect("exact fixture has Type-9 initializer metadata");
        let (original_index, _, initial_owner) = manager
            .take_fresh_level1_type9_scheduler_productions()
            .expect("exact fixture sidecar authenticates")
            .into_iter()
            .find(|(_, _, owner)| owner.successful_go_to_job_selection().is_some())
            .expect("exact fixture extraction returns Go-To-Job");
        let owner =
            OrdinaryType9GoToJobProductionOwner::adopt(original_index, initial_owner, actor_lease)
                .expect("exact fixture adopts Go-To-Job");
        (manager, owner, entity_id, expected)
    }

    fn append_open_base(manager: &mut EntityManager, id: u32, position_raw: [i16; 3]) {
        let mut target = Entity::unresolved_port_entity(id, EntityKind::Unknown(66), 66);
        target.active = true;
        target.capability_flags = LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK;
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.base_factory_runtime = RetailRuntimeValue::Known(Some(BaseFactoryRuntimeState {
            status_descriptor: StatusComponentDescriptor {
                raw_word_at_0x00: 0,
                variable_bindings: [0; 6],
                raw_tail: [0; 10],
            },
            control_value_raw: 0,
            required_scientists: 1,
            current_scientists: 0,
            lifter_progress_raw: 0,
            production_progress_raw: 0,
            recovery_progress_raw: 0,
            production: None,
            live_owner: None,
            progressive_death: ProgressiveDeathState::idle(0),
        }));
        target.set_motion_raw(position_raw, [0; 3]);
        manager.append_entity_for_test(target);
    }

    fn append_run_away_candidate(
        manager: &mut EntityManager,
        owner_id: u32,
        id: u32,
        same_entity_type: bool,
    ) {
        let (mut position_raw, owner_type) = manager
            .iter_all()
            .find(|entity| entity.id == owner_id)
            .map(|entity| (entity.position_raw(), entity.entity_type))
            .expect("selected actor remains live");
        position_raw[0] = position_raw[0].wrapping_add(10);
        let entity_type = if same_entity_type { owner_type } else { 99 };
        let mut target =
            Entity::unresolved_port_entity(id, EntityKind::Unknown(entity_type), entity_type);
        target.active = true;
        target.capability_flags = if same_entity_type {
            u32::MAX
        } else {
            LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK
        };
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
        target.set_motion_raw(position_raw, [0; 3]);
        manager.append_entity_for_test(target);
    }

    fn append_root_player(manager: &mut EntityManager, owner_id: u32) {
        let mut position_raw = manager
            .iter_all()
            .find(|entity| entity.id == owner_id)
            .map(Entity::position_raw)
            .expect("selected actor remains live");
        position_raw[0] = position_raw[0].wrapping_add(20);
        let mut target =
            Entity::unresolved_port_entity(ROOT_PLAYER_ID, EntityKind::Unknown(99), 99);
        target.active = true;
        target.capability_flags = LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK;
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        target.collision.recent_relation_id_at_0x60 = RetailRuntimeValue::Known(None);
        target.set_motion_raw(position_raw, [0; 3]);
        manager.append_entity_for_test(target);
    }

    fn resolve_run_away_audio(manager: &mut EntityManager) {
        let metadata = manager
            .type_runtime_metadata_mut_for_test(9)
            .expect("exact fixture retains Type-9 metadata");
        metadata.run_away_optional_sound_id =
            RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID));
        metadata.run_away_sound_period_raw =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW);
    }

    fn kill_tracked_target_and_append_open_base(
        manager: &mut EntityManager,
        entity_id: u32,
    ) -> u32 {
        let (tracked_target_id, mut position_raw) = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("Go-To-Job owns Primary");
            let Some(ActorTaskRuntime::GoToJob(task)) = entity.actor_tasks.task_state(task_id)
            else {
                panic!("Go-To-Job owns the Primary wrapper")
            };
            (
                task.target_id()
                    .expect("fixture selected a live job target"),
                entity.position_raw(),
            )
        };
        manager
            .entity_mut_for_test(tracked_target_id)
            .expect("selected target remains live")
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(1 | DYING_STATE_BIT);
        position_raw[0] = position_raw[0].wrapping_add(10);
        append_open_base(manager, RESELECTED_BASE_ID, position_raw);
        RESELECTED_BASE_ID
    }

    fn tick_live_target(
        actor_descriptor: RetailRuntimeValue<CommonAxisDescriptor>,
    ) -> (OrdinaryType9GoToJobProductionOutcome, usize) {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let entity = manager
            .entity_mut_for_test(entity_id)
            .expect("exact fixture entity remains live");
        entity.actor_common_axis_descriptor = actor_descriptor;
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;
        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );
        (tick.outcome, random_calls)
    }

    #[test]
    fn unresolved_actor_controller_blocks_at_live_target_predicate() {
        let (outcome, random_calls) = tick_live_target(RetailRuntimeValue::Unresolved);
        assert_eq!(random_calls, 0);
        assert!(matches!(
            outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::Dispatcher(
                    ActorTaskDispatcherError::GoToJobCallback {
                        error:
                            OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorUnavailable,
                        ..
                    }
                ),
                ..
            }
        ));
    }

    #[test]
    fn mismatched_actor_range_blocks_at_live_target_predicate() {
        let (_, _, _, expected) = production_owner_fixture();
        let actual = CommonAxisDescriptor {
            strict_axis_limit_raw: expected.strict_axis_limit_raw + 1,
            ..expected
        };
        let (outcome, random_calls) = tick_live_target(RetailRuntimeValue::Known(actual));
        assert_eq!(random_calls, 0);
        assert!(matches!(
            outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::Dispatcher(
                    ActorTaskDispatcherError::GoToJobCallback {
                        error: OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorMismatch {
                            expected: seen_expected,
                            actual: seen_actual,
                        },
                        ..
                    }
                ),
                ..
            } if seen_expected == expected && seen_actual == actual
        ));
    }

    #[test]
    fn live_target_predicate_preserves_candidate_filter_mutation() {
        let (_, _, _, expected) = production_owner_fixture();
        let actual = CommonAxisDescriptor {
            raw_word_at_0x04: 0x201,
            ..expected
        };
        // FUN_00423030 consumes the range; FUN_00401FB0 independently owns
        // this persistent filter write before a later Go-To-Job root.
        assert_eq!(
            tick_live_target(RetailRuntimeValue::Known(actual)),
            tick_live_target(RetailRuntimeValue::Known(expected))
        );
    }

    #[test]
    fn outer_scheduler_gate_precedes_callback_controller_evidence() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let entity = manager
            .entity_mut_for_test(entity_id)
            .expect("exact fixture entity remains live");
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(REMOTE_OWNED_STATE_BIT, REMOTE_OWNED_STATE_BIT);

        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;
        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );

        assert_eq!(random_calls, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::RemoteSchedulerOwnerUnsupported,
                ..
            }
        ));
    }

    #[test]
    fn scheduler_wait_commits_prefix_and_skips_every_callback_dependency() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let entity = manager
            .entity_mut_for_test(entity_id)
            .expect("exact fixture entity remains live");
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Unresolved;
        entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
        entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
        manager
            .type_runtime_metadata_mut_for_test(9)
            .expect("fixture has Type-9 metadata")
            .common_mover_topology = RetailRuntimeValue::Unresolved;

        let resources = ResourceCache::new(Vec::new());
        let mut world_fx = WorldFx::new();
        let mut words = [0xffff_u32, 0xffff_u32].into_iter();
        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .next()
                    .expect("waiting prefix consumes two exact words")
            },
        );

        assert!(matches!(
            tick.outcome,
            OrdinaryType9GoToJobProductionOutcome::SchedulerWaiting { .. }
        ));
        assert_eq!(words.next(), None);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(1)
        );
    }

    #[test]
    fn dying_target_skips_route_and_mover_dependencies_then_publishes_f70() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let target_id = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            );
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
            entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
            entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("Go-To-Job owns Primary");
            let Some(ActorTaskRuntime::GoToJob(task)) = entity.actor_tasks.task_state(task_id)
            else {
                panic!("Go-To-Job owns the Primary wrapper")
            };
            task.target_id()
                .expect("fixture selected a live job target")
        };
        manager
            .entity_mut_for_test(target_id)
            .expect("selected target remains live")
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(1 | DYING_STATE_BIT);
        manager
            .type_runtime_metadata_mut_for_test(9)
            .expect("fixture has Type-9 metadata")
            .common_mover_topology = RetailRuntimeValue::Unresolved;

        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;
        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );

        assert_eq!(
            random_calls, 0,
            "invalid target never reaches mover/root RNG"
        );
        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending { .. }
            ),
            "unexpected outcome: {:?}",
            tick.outcome
        );
        let retained = tick.retained_owner.expect("F70 retains linear custody");
        assert!(matches!(
            retained.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete {
                expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
                ..
            })
        ));
        assert!(!retained.main_base_abort_compatible());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn completed_tail_authenticates_full_state_and_zero_b2_before_next_scheduler_visit() {
        for tamper_b2 in [false, true] {
            let (mut manager, owner, entity_id, _) = production_owner_fixture();
            manager
                .entity_mut_for_test(entity_id)
                .unwrap()
                .collision
                .animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let resources = terrain_resources();
            let mut world_fx = WorldFx::new();
            let first = tick_ordinary_type9_go_to_job_owner_with_random(
                &mut manager,
                owner,
                OrdinaryType9GoToJobProductionFrame {
                    dispatch_resource_text: &|_, _| {},
                    resources: &resources,
                    elapsed_micros: 1,
                    global_elapsed_micros: 1,
                    retail_tick: 0,
                },
                &mut world_fx,
                |_| 0,
            );
            let retained = first.retained_owner.expect("tail completes in one visit");
            assert!(matches!(
                retained.outer_tail,
                Some(OrdinaryType9OuterTailCustody::Complete {
                    expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
                    ..
                })
            ));
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.animation_offset_at_0xb2,
                RetailRuntimeValue::Known(0)
            );
            if tamper_b2 {
                entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(1);
            } else {
                let mut position = entity.position_raw();
                position[0] = position[0].wrapping_add(1);
                entity.set_motion_raw(position, entity.velocity_raw());
            }
            let mut replay_random_calls = 0;
            let retry = tick_ordinary_type9_go_to_job_owner_with_random(
                &mut manager,
                retained,
                OrdinaryType9GoToJobProductionFrame {
                    dispatch_resource_text: &|_, _| {},
                    resources: &resources,
                    elapsed_micros: 1,
                    global_elapsed_micros: 1,
                    retail_tick: 0,
                },
                &mut world_fx,
                |_| {
                    replay_random_calls += 1;
                    0
                },
            );
            assert_eq!(replay_random_calls, 0);
            assert!(matches!(
                retry.outcome,
                OrdinaryType9GoToJobProductionOutcome::Dropped {
                    reason: OrdinaryType9GoToJobProductionDrop::OuterTailStateMismatch,
                    ..
                }
            ));
            assert!(retry.retained_owner.is_none());
        }
    }

    #[test]
    fn dying_target_transition_publishes_root_wander_and_f70_in_the_same_tick() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let target_id = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("Go-To-Job owns Primary");
            let Some(ActorTaskRuntime::GoToJob(task)) = entity.actor_tasks.task_state(task_id)
            else {
                panic!("Go-To-Job owns the Primary wrapper")
            };
            task.target_id()
                .expect("fixture selected a live job target")
        };
        manager
            .entity_mut_for_test(target_id)
            .expect("selected target remains live")
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(1 | DYING_STATE_BIT);

        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0xffff_u32, 0xd2f6]);
        let mut consumed = Vec::new();
        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("root selector then Wander constructor");
                consumed.push(word);
                word
            },
        );

        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9GoToJobProductionOutcome::RootWanderPublished { .. }
            ),
            "unexpected root-transition outcome: {:?}",
            tick.outcome
        );
        assert_eq!(consumed, [0xffff, 0xd2f6]);
        assert!(words.is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) if wander.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert!(matches!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        assert!(matches!(
            tick.retained_owner.unwrap().state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
    }

    #[test]
    fn root_wander_allocation_failure_publishes_fallback_without_constructor_rng() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let (
            target_id,
            initial_behavior,
            predecessor_target,
            predecessor_auxiliary,
            predecessor_sub_a,
        ) = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("Go-To-Job owns Primary");
            let Some(ActorTaskRuntime::GoToJob(task)) = entity.actor_tasks.task_state(task_id)
            else {
                panic!("Go-To-Job owns the Primary wrapper")
            };
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("Go-To-Job retains its behavior context")
            };
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!("Go-To-Job retains Sub-A")
            };
            (
                task.target_id()
                    .expect("fixture selected a live job target"),
                entity.initial_behavior,
                context.target_handle_at_0x08(),
                context.auxiliary_word_at_0x0c(),
                sub_a,
            )
        };
        manager
            .entity_mut_for_test(target_id)
            .expect("selected target remains live")
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(1 | DYING_STATE_BIT);

        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0xffff_u32]);
        let mut allocations = 0;
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("only the root selector is consumed")
            },
            |_| {
                allocations += 1;
                OrdinaryType9WanderAllocationDecision::Failed
            },
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );

        assert_eq!(allocations, 1);
        assert!(words.is_empty());
        assert!(matches!(
            tick.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootInitializerFallbackPublished {
                entity_id: seen_entity,
                failure: OrdinaryType9WanderInitializerFailure {
                    action_index: 2,
                    slot: ActorTaskSlot::Primary,
                },
            } if seen_entity == entity_id
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("fallback actor remains live");
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("fallback publishes a resolved behavior context")
        };
        assert_eq!(
            context.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(context.target_handle_at_0x08(), predecessor_target);
        assert_eq!(context.auxiliary_word_at_0x0c(), predecessor_auxiliary);
        assert_eq!(entity.initial_behavior, initial_behavior);
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(predecessor_sub_a))
        );
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .expect("fallback retains selected-component custody")
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert!(matches!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        let retained = tick
            .retained_owner
            .expect("fallback retains publication custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn dying_target_transition_publishes_root_run_away_and_hands_off_secondary_same_tick() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        resolve_run_away_audio(&mut manager);
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, true);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0_u32, 0x3333, 0x4444, 0x5555]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("selector, both acquiring constructors, then fleeing constructor");
                consumed.push(word);
                word
            },
        );

        let constructors = match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootRunAwayPublished {
                entity_id: seen_entity,
                constructors_by_phase,
            } if seen_entity == entity_id => constructors_by_phase,
            outcome => panic!("unexpected root-transition outcome: {outcome:?}"),
        };
        assert_eq!(constructors[0].random_sample_low16, 0x3333);
        assert_eq!(constructors[1].random_sample_low16, 0x4444);
        assert_eq!(consumed, [0, 0x3333, 0x4444, 0x5555]);
        assert!(words.is_empty());
        let retained = tick
            .retained_owner
            .expect("root publication retains custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("root-published actor remains live");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::RunAway(task))
                if task.target_id() == RUN_AWAY_TARGET_ID && task.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("fleeing publication retains Sub-A")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(416));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("fleeing publication retains behavior context")
        };
        assert_eq!(context.style_table_index_raw_at_0x10(), 1);
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(RUN_AWAY_TARGET_ID))
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn root_run_away_phase_one_allocation_failure_keeps_only_phase_zero_rng_prefix() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        resolve_run_away_audio(&mut manager);
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0_u32, 0x3333]);
        let mut phases = Vec::new();

        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector then successful phase-zero suffix")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |preparation| {
                phases.push(preparation.phase_index);
                if preparation.phase_index == 1 {
                    OrdinaryType9RunAwayAllocationDecision::Failed
                } else {
                    OrdinaryType9RunAwayAllocationDecision::Prepared
                }
            },
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );

        match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootRunAwayInitializerFallbackPublished {
                entity_id: seen_entity,
                failure,
                constructors_by_phase,
            } if seen_entity == entity_id => {
                assert_eq!(failure.phase_index, 1);
                assert_eq!(
                    constructors_by_phase[0].unwrap().random_sample_low16,
                    0x3333
                );
                assert_eq!(constructors_by_phase[1], None);
            }
            outcome => panic!("unexpected root fallback outcome: {outcome:?}"),
        }
        assert_eq!(phases, [0, 1]);
        assert!(words.is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("fallback actor remains live");
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        let retained = tick
            .retained_owner
            .expect("fallback retains publication custody");
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn dying_target_transition_publishes_root_go_to_job_and_f70_in_the_same_tick() {
        let (mut manager, owner, entity_id, expected_axis) = production_owner_fixture();
        let target_id = kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([3_982_u32, 0x1234_D2F6]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("root selector then Go-To-Job constructor");
                consumed.push(word);
                word
            },
        );

        let constructor = match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootGoToJobPublished {
                entity_id: seen_entity,
                target_id: seen_target,
                constructor,
            } if seen_entity == entity_id && seen_target == target_id => constructor,
            outcome => panic!("unexpected root-transition outcome: {outcome:?}"),
        };
        assert_eq!(
            constructor,
            OrdinaryType9GoToJobConstructorEvidence {
                random_sample_low16: 0xD2F6,
                sub_a_target_speed_raw: 333,
            }
        );
        assert_eq!(consumed, [3_982, 0x1234_D2F6]);
        assert!(words.is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(task))
                if task.elapsed_ms() == 0 && task.target_id() == Some(target_id)
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(expected_axis)
        );
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("class-54 publication retains resolved Sub-A")
        };
        assert_eq!(sub_a.direction_multiplier(), 1);
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(333));
        assert!(matches!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(_)
        ));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        let retained = tick.retained_owner.unwrap();
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(!retained
            .fork_for_main_base_abort_transaction()
            .main_base_abort_compatible());
    }

    #[test]
    fn root_go_to_job_allocation_failure_publishes_fallback_without_constructor_rng() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let target_id = kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([3_982_u32]);
        let mut allocations = 0;

        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("only the root selector is consumed")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| {
                allocations += 1;
                OrdinaryType9GoToJobAllocationDecision::Failed
            },
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );

        assert_eq!(allocations, 1);
        assert!(words.is_empty());
        assert!(matches!(
            tick.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootGoToJobInitializerFallbackPublished {
                entity_id: seen_entity,
                target_id: seen_target,
                ..
            } if seen_entity == entity_id && seen_target == target_id
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert!(matches!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(_)
        ));
        let retained = tick.retained_owner.unwrap();
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
    }

    #[test]
    fn missing_predecessor_sub_a_resolves_once_then_rejects_drift_without_redraw() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let target_id = kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        let (original_sub_a, exact_model) = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                panic!("exact fixture retains Sub-A")
            };
            let model = entity
                .model_index
                .expect("exact fixture has an active model");
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
            (sub_a, model)
        };
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;

        let tick = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );

        assert_eq!(random_calls, 0);
        assert!(matches!(
            tick.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAUnavailable,
                ..
            }
        ));
        let retained = tick
            .retained_owner
            .expect("missing Sub-A retains the exact dispatcher continuation");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::RootTransitionPending { .. }
        ));
        assert!(retained.pending_root_plan.is_none());
        assert!(retained.pending_root_dispatcher_continuation.is_some());
        assert!(!retained.main_base_abort_compatible());

        {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("selected actor remains live");
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(original_sub_a));
            entity.model_index = None;
        }
        let second = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| panic!("active-model preflight precedes the selector"),
            |_| panic!("active-model preflight precedes Wander allocation"),
            |_| panic!("active-model preflight precedes Go-To-Job allocation"),
            |_| panic!("active-model preflight precedes Run Away allocation"),
            |_| panic!("active-model preflight precedes Attract allocation"),
            |_| panic!("active-model preflight precedes Attract target allocation"),
        );
        assert!(matches!(
            second.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::ActiveModelUnavailable,
                ..
            }
        ));
        let retained = second
            .retained_owner
            .expect("first resolved Sub-A remains retained before root planning");
        let mut changed_sub_a = original_sub_a;
        changed_sub_a.set_direction_multiplier(original_sub_a.direction_multiplier() + 1);
        let (context_before_drift, visits_before_drift) = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("selected actor remains live");
            entity.model_index = Some(exact_model);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(changed_sub_a));
            (
                entity.current_behavior_context,
                live_task_visits(&entity.actor_tasks),
            )
        };
        let third = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| panic!("Sub-A drift precedes all RNG"),
            |_| panic!("Sub-A drift precedes Wander allocation"),
            |_| panic!("Sub-A drift precedes Go-To-Job allocation"),
            |_| panic!("Sub-A drift precedes Run Away allocation"),
            |_| panic!("Sub-A drift precedes Attract allocation"),
            |_| panic!("Sub-A drift precedes Attract target allocation"),
        );
        assert!(matches!(
            third.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::RootPredecessorSubAChanged {
                    expected,
                    actual,
                },
                ..
            } if expected == original_sub_a && actual == changed_sub_a
        ));
        let retained = third.retained_owner.expect("Sub-A drift retains custody");
        {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("selected actor remains live");
            assert_eq!(entity.current_behavior_context, context_before_drift);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_before_drift);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(original_sub_a));
        }
        let mut root_words = VecDeque::from([3_982_u32, 0xCAFE_D2F6]);
        let completed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                root_words
                    .pop_front()
                    .expect("restored retry consumes selector then constructor")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );
        assert!(root_words.is_empty());
        assert!(matches!(
            completed.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootGoToJobPublished {
                target_id: seen_target,
                ..
            } if seen_target == target_id
        ));
    }

    #[test]
    fn planless_retry_rejects_live_root_drift_before_rng_then_uses_frozen_snapshot() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let target_id = kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        let (active_model, original_position) = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            let active_model = entity.model_index.take().expect("fixture has active model");
            (active_model, entity.position_raw())
        };
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;

        let first = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );
        assert_eq!(random_calls, 0);
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::ActiveModelUnavailable,
                ..
            }
        ));
        let first_retained = first
            .retained_owner
            .expect("active-model dependency retains planless continuation");
        assert!(first_retained.pending_root_plan.is_none());

        {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            entity.model_index = Some(active_model);
            let mut drifted_position = original_position;
            drifted_position[0] = drifted_position[0].wrapping_add(1);
            entity.set_position_raw(drifted_position);
        }
        let second = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            first_retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );
        assert_eq!(random_calls, 0, "live root drift precedes selector RNG");
        assert!(matches!(
            second.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::RootTransitionRequestChanged,
                ..
            }
        ));
        let second_retained = second
            .retained_owner
            .expect("root drift preserves the planless continuation");
        assert!(second_retained.pending_root_plan.is_none());

        manager
            .entity_mut_for_test(entity_id)
            .expect("exact fixture entity remains live")
            .set_position_raw(original_position);
        let mut words = VecDeque::from([3_982_u32, 0xABCD_D2F6]);
        let third = tick_ordinary_type9_go_to_job_owner_with_random(
            &mut manager,
            second_retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("restored retry consumes selector then constructor")
            },
        );

        assert!(words.is_empty());
        assert!(matches!(
            third.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootGoToJobPublished {
                entity_id: seen_entity,
                target_id: seen_target,
                ..
            } if seen_entity == entity_id && seen_target == target_id
        ));
    }

    #[test]
    fn deferred_root_go_to_job_retries_frozen_plan_without_selector_or_manager_redraw() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        let frozen_target_id = kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        let exact_model_slots = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            let exact = entity.model_slots;
            entity.model_slots = [None; 4];
            exact
        };
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut selector_words = VecDeque::from([3_982_u32]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                selector_words
                    .pop_front()
                    .expect("one retained selector draw")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| panic!("root preflight precedes Go-To-Job allocation"),
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );
        assert!(selector_words.is_empty());
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::RootGoToJobPreflight(
                    OrdinaryType9RootGoToJobPreflightError::ModelSlotsMismatch { .. }
                ),
                ..
            }
        ));
        let retained = first.retained_owner.expect("retry retains linear custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::RootTransitionPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());

        let actor_position = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("exact fixture entity remains live");
            entity.model_slots = exact_model_slots;
            entity.position_raw()
        };
        append_open_base(&mut manager, CLOSER_RESELECTED_BASE_ID, actor_position);
        let mut constructor_words = VecDeque::from([0xCAFE_D2F6_u32]);
        let second = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                constructor_words
                    .pop_front()
                    .expect("retry consumes only the retained constructor suffix")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );

        assert!(constructor_words.is_empty());
        assert!(matches!(
            second.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootGoToJobPublished {
                entity_id: seen_entity,
                target_id,
                ..
            } if seen_entity == entity_id && target_id == frozen_target_id
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(task))
                if task.target_id() == Some(frozen_target_id)
        ));
        let retained = second.retained_owner.unwrap();
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
    }

    #[test]
    fn retained_root_run_away_latches_resolved_axis_rejects_drift_and_uses_frozen_target() {
        let (mut manager, owner, entity_id, expected_axis) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, true);
        manager
            .entity_mut_for_test(entity_id)
            .expect("selected actor remains live")
            .actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut selector_words = VecDeque::from([0_u32]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| selector_words.pop_front().expect("one root selector draw"),
            |_| panic!("axis preflight precedes Wander allocation"),
            |_| panic!("axis preflight precedes Go-To-Job allocation"),
            |_| panic!("axis preflight precedes Run Away allocation"),
            |_| panic!("axis preflight precedes Attract allocation"),
            |_| panic!("axis preflight precedes Attract target allocation"),
        );
        assert!(selector_words.is_empty());
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason:
                    OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisUnavailable,
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("selector plan remains retained");

        manager
            .entity_mut_for_test(entity_id)
            .expect("selected actor remains live")
            .actor_common_axis_descriptor = RetailRuntimeValue::Known(expected_axis);
        let second = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| panic!("retained plan cannot redraw the selector"),
            |_| panic!("audio preflight precedes Wander allocation"),
            |_| panic!("audio preflight precedes Go-To-Job allocation"),
            |_| panic!("audio preflight precedes Run Away allocation"),
            |_| panic!("audio preflight precedes Attract allocation"),
            |_| panic!("audio preflight precedes Attract target allocation"),
        );
        assert!(matches!(
            second.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::AuthoredRunAwayAudioUnresolved,
                ..
            }
        ));
        let retained = second
            .retained_owner
            .expect("first resolved axis is retained before the audio block");
        let changed_axis = CommonAxisDescriptor {
            strict_axis_limit_raw: expected_axis.strict_axis_limit_raw.wrapping_add(1),
            ..expected_axis
        };
        let (context_before_drift, visits_before_drift, sub_a_before_drift) = {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("selected actor remains live");
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(changed_axis);
            (
                entity.current_behavior_context,
                live_task_visits(&entity.actor_tasks),
                entity.sub_a_propulsion_runtime,
            )
        };
        resolve_run_away_audio(&mut manager);
        let third = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| panic!("axis drift precedes all RNG"),
            |_| panic!("axis drift precedes Wander allocation"),
            |_| panic!("axis drift precedes Go-To-Job allocation"),
            |_| panic!("axis drift precedes Run Away allocation"),
            |_| panic!("axis drift precedes Attract allocation"),
            |_| panic!("axis drift precedes Attract target allocation"),
        );
        assert!(matches!(
            third.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason:
                    OrdinaryType9GoToJobProductionBlock::RootPredecessorActorCommonAxisChanged {
                        expected,
                        actual: RetailRuntimeValue::Known(actual),
                    },
                ..
            } if expected == expected_axis && actual == changed_axis
        ));
        let retained = third
            .retained_owner
            .expect("axis drift keeps linear custody");
        {
            let entity = manager
                .entity_mut_for_test(entity_id)
                .expect("selected actor remains live");
            assert_eq!(entity.current_behavior_context, context_before_drift);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_before_drift);
            assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before_drift);
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(expected_axis);
        }
        manager
            .entity_mut_for_test(RUN_AWAY_TARGET_ID)
            .expect("frozen target remains in the live manager")
            .active = false;
        append_run_away_candidate(
            &mut manager,
            entity_id,
            RUN_AWAY_TARGET_ID.wrapping_add(1),
            true,
        );
        let mut constructor_words = VecDeque::from([0x3333_u32, 0x4444, 0x5555]);
        let completed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                constructor_words
                    .pop_front()
                    .expect("two acquiring and one fleeing constructor; no selector redraw")
            },
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("no Candidate target handoff"),
        );
        assert!(constructor_words.is_empty());
        assert!(matches!(
            completed.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootRunAwayPublished { .. }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("root-published actor remains live");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::RunAway(task))
                if task.target_id() == RUN_AWAY_TARGET_ID && task.elapsed_ms() == 0
        ));
        let retained = completed
            .retained_owner
            .expect("completed root publication retains custody");
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn root_transition_authenticates_lifetime_prefix_and_timeout_reason() {
        let (manager, _, entity_id, _) = production_owner_fixture();
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("exact fixture entity remains live");
        let task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("Go-To-Job owns Primary");
        let Some(ActorTaskRuntime::GoToJob(task_state)) = entity.actor_tasks.task_state(task_id)
        else {
            panic!("Go-To-Job owns the Primary wrapper")
        };
        let selected_runtime = entity
            .ordinary_type9_selected_component_runtime
            .expect("selected component custody is live");
        let transition = OrdinaryType9GoToJobRootTransition {
            entity_id,
            request: GoToJobTransitionRequest {
                slot: ActorTaskSlot::Primary,
                task_id,
                reason: GoToJobTransitionReason::TaggedCallbackResult(
                    crate::go_to_job_owner::GoToJobTaggedSingleton::ZeroPredicate,
                ),
                committed_prefix: crate::go_to_job_owner::GoToJobCallbackPrefix {
                    lifetime_status: GoToJobLifetimeStatus::WithinLifetime,
                },
            },
            task_state: *task_state,
            selected_runtime,
        };

        assert!(authenticates_root_transition(entity, transition));

        let mut mismatched_prefix = transition;
        mismatched_prefix.request.committed_prefix.lifetime_status =
            GoToJobLifetimeStatus::OwnerTransitionDue;
        assert!(!authenticates_root_transition(entity, mismatched_prefix));

        let mut premature_timeout = transition;
        premature_timeout.request.reason = GoToJobTransitionReason::LifetimeExpired;
        assert!(!authenticates_root_transition(entity, premature_timeout));
    }

    #[test]
    fn missing_target_publishes_even_attract_graph_without_visiting_newborn_primary() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let predecessor_task_id = manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);
        let mut phases = Vec::new();

        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |preparation| {
                phases.push(preparation.phase_index);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        let committed = match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                entity_id: seen_entity,
                committed,
            } if seen_entity == entity_id => committed,
            outcome => panic!("unexpected class-45 root outcome: {outcome:?}"),
        };
        assert!(words.is_empty());
        assert_eq!(phases, [1, 2]);
        assert_eq!(committed.parity_random_sample_low16, 0x1000);
        assert_eq!(committed.constructors_by_phase[0], None);
        assert!(committed.constructors_by_phase[1].is_some());
        assert!(committed.constructors_by_phase[2].is_some());
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(predecessor_task_id)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(_))
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AttractAttentionCue(_))
        ));
        let retained = tick
            .retained_owner
            .expect("class-45 retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn expired_attract_cue_after_outer_tail_applies_attract_root() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                first_words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                entity_id: seen_entity,
                ..
            } if seen_entity == entity_id
        ));
        let mut owner = first
            .retained_owner
            .expect("class-45 retains scheduler custody");
        assert!(matches!(
            owner.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete { .. })
        ));

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let tertiary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap();
            let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
                entity.actor_tasks.task_state_mut(tertiary)
            else {
                panic!("published Attract Tertiary is Cue")
            };
            cue.advance_elapsed(1_001_000);
            let Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) =
                owner.root_publication.as_mut()
            else {
                panic!("first visit published Attract")
            };
            assert!(publication.refresh_after_owned_dispatch(entity));
        }

        let mut words = VecDeque::from([0x1111_u32, 0x2222, 3_063, 0, 0x5555, 0x6666]);
        let mut consumed = Vec::new();
        let mut phases = Vec::new();
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_001,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("SharedRetarget mover, class-45 selector, and even constructor words");
                consumed.push(word);
                word
            },
            |_| panic!("class-45 reselection cannot allocate Wander"),
            |_| panic!("class-45 reselection cannot allocate Go-To-Job"),
            |_| panic!("class-45 reselection cannot allocate Run Away"),
            |preparation| {
                phases.push(preparation.phase_index);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                    entity_id: seen_entity,
                    ..
                } if seen_entity == entity_id
            ),
            "expired Cue after completed tail must apply a root: {:?}",
            tick.outcome
        );
        assert_eq!(phases, [1, 2]);
        assert_eq!(consumed, [0x1111, 0x2222, 3_063, 0, 0x5555, 0x6666]);
        assert!(words.is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("re-rooted actor remains live");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 0
        ));
    }

    #[test]
    fn expired_cue_attract_origin_retries_without_go_to_job_request() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                first_words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );
        let mut owner = first
            .retained_owner
            .expect("publication retains Go-To-Job production custody");

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let tertiary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap();
            let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
                entity.actor_tasks.task_state_mut(tertiary)
            else {
                panic!("published Attract Tertiary is Cue")
            };
            cue.advance_elapsed(876_000);
            let Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) =
                owner.root_publication.as_mut()
            else {
                panic!("first visit published Attract")
            };
            assert!(publication.refresh_after_owned_dispatch(entity));
        }
        let exact_hit_sound = {
            let metadata = manager
                .type_runtime_metadata_mut_for_test(9)
                .expect("exact fixture retains type-9 metadata");
            let exact = metadata.accepted_hit_presentation_sound_id;
            metadata.accepted_hit_presentation_sound_id = RetailRuntimeValue::Known(Some(1));
            exact
        };

        let mut words = VecDeque::from([0x1111_u32, 0x2222, 3_063]);
        let mut consumed = Vec::new();
        let mut allocations = 0;
        let blocked = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_001,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("SharedRetarget mover and one class-45 selector word");
                consumed.push(word);
                word
            },
            |_| panic!("Attract preflight cannot allocate Wander"),
            |_| panic!("Attract preflight cannot allocate Go-To-Job"),
            |_| panic!("Attract preflight cannot allocate Run Away"),
            |_| {
                allocations += 1;
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        assert_eq!(allocations, 0, "preflight precedes class-45 allocation");
        assert!(
            matches!(
                blocked.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::RootReselection(
                        OrdinaryType9RootReselectionError::MetadataNotExact
                    ),
                    ..
                }
            ),
            "Cue-origin reselection must park without a Go-To-Job request: {:?} consumed {consumed:?}",
            blocked.outcome
        );
        assert_eq!(consumed, [0x1111, 0x2222]);
        assert_eq!(words, [3_063]);
        let retained = blocked
            .retained_owner
            .expect("Cue-origin retry retains Attract-origin custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::RootTransitionPending {
                transition: None,
                ..
            }
        ));
        assert!(retained.pending_root_plan.is_none());
        assert_eq!(
            retained
                .pending_root_dispatcher_continuation
                .as_ref()
                .expect("Cue-origin retry retains dispatcher cursor")
                .next_slot,
            None
        );

        manager
            .type_runtime_metadata_mut_for_test(9)
            .expect("exact fixture retains type-9 metadata")
            .accepted_hit_presentation_sound_id = exact_hit_sound;
        let mut retry_words = VecDeque::from([3_063_u32, 0, 0x5555, 0x6666]);
        let mut retry_consumed = Vec::new();
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1_001_002,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                let word = retry_words
                    .pop_front()
                    .expect("Cue-origin retry consumes only the frozen even constructor words");
                retry_consumed.push(word);
                word
            },
            |_| panic!("class-45 retry cannot allocate Wander"),
            |_| panic!("class-45 retry cannot allocate Go-To-Job"),
            |_| panic!("class-45 retry cannot allocate Run Away"),
            |_| {
                allocations += 1;
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        assert_eq!(
            allocations, 2,
            "Cue-origin retry must consume the even Attract allocations: {:?} consumed {retry_consumed:?}",
            tick.outcome
        );
        assert_eq!(retry_consumed, [3_063, 0, 0x5555, 0x6666]);
        assert!(retry_words.is_empty());
        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                    entity_id: seen_entity,
                    ..
                } if seen_entity == entity_id
            ),
            "Cue-origin retry must apply the frozen Attract plan: {:?}",
            tick.outcome
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("re-rooted actor remains live");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 0
            ),
            "Cue-origin retry is terminal and must not visit the newborn Cue: {:?}",
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        );
    }

    #[test]
    fn expired_shared_retarget_attract_origin_retries_without_go_to_job_request() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                first_words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );
        let mut owner = first
            .retained_owner
            .expect("publication retains Go-To-Job production custody");

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("published Attract Primary is SharedRetarget")
            };
            assert_eq!(state.elapsed_ms(), 0);
            state.before_callback(900_000);
            assert_eq!(state.elapsed_ms(), 900);
            let Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) =
                owner.root_publication.as_mut()
            else {
                panic!("first visit published Attract")
            };
            assert!(publication.refresh_after_owned_dispatch(entity));
        }
        let exact_hit_sound = {
            let metadata = manager
                .type_runtime_metadata_mut_for_test(9)
                .expect("exact fixture retains type-9 metadata");
            let exact = metadata.accepted_hit_presentation_sound_id;
            metadata.accepted_hit_presentation_sound_id = RetailRuntimeValue::Known(Some(1));
            exact
        };

        let mut words = VecDeque::from([0x1111_u32, 0x2222, 3_063]);
        let mut consumed = Vec::new();
        let mut allocations = 0;
        let blocked = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_001,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("SharedRetarget mover and one class-45 selector word");
                consumed.push(word);
                word
            },
            |_| panic!("Attract preflight cannot allocate Wander"),
            |_| panic!("Attract preflight cannot allocate Go-To-Job"),
            |_| panic!("Attract preflight cannot allocate Run Away"),
            |_| {
                allocations += 1;
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        assert_eq!(allocations, 0, "preflight precedes class-45 allocation");
        assert!(
            matches!(
                blocked.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::RootReselection(
                        OrdinaryType9RootReselectionError::MetadataNotExact
                    ),
                    ..
                }
            ),
            "SharedRetarget-origin reselection must park without a Go-To-Job request: {:?} consumed {consumed:?}",
            blocked.outcome
        );
        assert_eq!(consumed, [0x1111, 0x2222]);
        assert_eq!(words, [3_063]);
        let retained = blocked
            .retained_owner
            .expect("SharedRetarget-origin retry retains Attract-origin custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::RootTransitionPending {
                transition: None,
                ..
            }
        ));
        assert!(retained.pending_root_plan.is_none());
        assert_eq!(
            retained
                .pending_root_dispatcher_continuation
                .as_ref()
                .expect("SharedRetarget-origin retry retains dispatcher cursor")
                .next_slot,
            Some(ActorTaskSlot::Secondary)
        );

        manager
            .type_runtime_metadata_mut_for_test(9)
            .expect("exact fixture retains type-9 metadata")
            .accepted_hit_presentation_sound_id = exact_hit_sound;
        let mut retry_words = VecDeque::from([3_063_u32, 0, 0x5555, 0x6666]);
        let mut retry_consumed = Vec::new();
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_002,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                let word = retry_words.pop_front().expect(
                    "SharedRetarget-origin retry consumes even constructor words then suffix",
                );
                retry_consumed.push(word);
                word
            },
            |_| panic!("class-45 retry cannot allocate Wander"),
            |_| panic!("class-45 retry cannot allocate Go-To-Job"),
            |_| panic!("class-45 retry cannot allocate Run Away"),
            |_| {
                allocations += 1;
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        assert_eq!(
            allocations, 2,
            "SharedRetarget-origin retry must consume the even Attract allocations: {:?} consumed {retry_consumed:?}",
            tick.outcome
        );
        assert_eq!(retry_consumed, [3_063, 0, 0x5555, 0x6666]);
        assert!(retry_words.is_empty());
        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                    entity_id: seen_entity,
                    ..
                } if seen_entity == entity_id
            ),
            "SharedRetarget-origin retry must apply the frozen Attract plan: {:?}",
            tick.outcome
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("re-rooted actor remains live");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 125
            ),
            "SharedRetarget-origin retry must resume at Secondary and age the newborn Cue: {:?}",
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        );
    }

    #[test]
    fn expired_cue_attract_origin_retries_frozen_run_away_plan_without_selector_redraw() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, true);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                first_words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );
        let mut owner = first
            .retained_owner
            .expect("publication retains Go-To-Job production custody");

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let tertiary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap();
            let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
                entity.actor_tasks.task_state_mut(tertiary)
            else {
                panic!("published Attract Tertiary is Cue")
            };
            cue.advance_elapsed(876_000);
            let Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) =
                owner.root_publication.as_mut()
            else {
                panic!("first visit published Attract")
            };
            assert!(publication.refresh_after_owned_dispatch(entity));
        }

        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0]);
        let mut consumed = Vec::new();
        let blocked = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_001,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("SharedRetarget mover and one class-10 selector word");
                consumed.push(word);
                word
            },
            |_| panic!("audio preflight cannot allocate Wander"),
            |_| panic!("audio preflight cannot allocate Go-To-Job"),
            |_| panic!("audio preflight cannot allocate Run Away"),
            |_| panic!("audio preflight cannot allocate Attract"),
            |_| panic!("even class-10 graph has no Candidate target handoff"),
        );

        assert!(
            matches!(
                blocked.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::AuthoredRunAwayAudioUnresolved,
                    ..
                }
            ),
            "Cue-origin class-10 reselection must park the frozen plan: {:?} consumed {consumed:?}",
            blocked.outcome
        );
        assert_eq!(consumed, [0x1111, 0x2222, 0]);
        assert!(words.is_empty());
        let retained = blocked
            .retained_owner
            .expect("Cue-origin after-plan retry retains Attract-origin custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::RootTransitionPending {
                transition: None,
                ..
            }
        ));
        assert!(retained.pending_root_plan.is_some());
        assert_eq!(
            retained
                .pending_root_dispatcher_continuation
                .as_ref()
                .expect("Cue-origin retry retains dispatcher cursor")
                .next_slot,
            None
        );

        resolve_run_away_audio(&mut manager);
        let mut retry_words = VecDeque::from([0x3333_u32, 0x4444]);
        let mut retry_consumed = Vec::new();
        let mut allocations = 0;
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1_001_002,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                let word = retry_words
                    .pop_front()
                    .expect("Cue-origin retry consumes only the frozen class-10 constructor words");
                retry_consumed.push(word);
                word
            },
            |_| panic!("class-10 retry cannot allocate Wander"),
            |_| panic!("class-10 retry cannot allocate Go-To-Job"),
            |_| {
                allocations += 1;
                OrdinaryType9RunAwayAllocationDecision::Prepared
            },
            |_| panic!("class-10 retry cannot allocate Attract"),
            |_| panic!("even class-10 graph has no Candidate target handoff"),
        );

        assert_eq!(
            allocations, 2,
            "Cue-origin retry must consume both acquiring allocations: {:?} consumed {retry_consumed:?}",
            tick.outcome
        );
        assert_eq!(retry_consumed, [0x3333, 0x4444]);
        assert!(retry_words.is_empty());
        let constructors = match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootRunAwayPublished {
                entity_id: seen_entity,
                constructors_by_phase,
            } if seen_entity == entity_id => constructors_by_phase,
            outcome => panic!("Cue-origin retry must apply the frozen class-10 plan: {outcome:?}"),
        };
        assert_eq!(constructors[0].random_sample_low16, 0x3333);
        assert_eq!(constructors[1].random_sample_low16, 0x4444);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("re-rooted actor remains live");
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
            ),
            "Cue-origin retry is terminal and must not run the Fleeing suffix: {:?}",
            entity.actor_task_state(ActorTaskSlot::Primary)
        );
    }

    #[test]
    fn expired_shared_retarget_attract_origin_retries_frozen_run_away_plan_and_flees() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, true);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                first_words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );
        let mut owner = first
            .retained_owner
            .expect("publication retains Go-To-Job production custody");

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("published Attract Primary is SharedRetarget")
            };
            assert_eq!(state.elapsed_ms(), 0);
            state.before_callback(900_000);
            assert_eq!(state.elapsed_ms(), 900);
            let Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) =
                owner.root_publication.as_mut()
            else {
                panic!("first visit published Attract")
            };
            assert!(publication.refresh_after_owned_dispatch(entity));
        }

        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0]);
        let mut consumed = Vec::new();
        let blocked = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_001,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("SharedRetarget mover and one class-10 selector word");
                consumed.push(word);
                word
            },
            |_| panic!("audio preflight cannot allocate Wander"),
            |_| panic!("audio preflight cannot allocate Go-To-Job"),
            |_| panic!("audio preflight cannot allocate Run Away"),
            |_| panic!("audio preflight cannot allocate Attract"),
            |_| panic!("even class-10 graph has no Candidate target handoff"),
        );

        assert!(
            matches!(
                blocked.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::AuthoredRunAwayAudioUnresolved,
                    ..
                }
            ),
            "SharedRetarget-origin class-10 reselection must park the frozen plan: {:?} consumed {consumed:?}",
            blocked.outcome
        );
        assert_eq!(consumed, [0x1111, 0x2222, 0]);
        assert!(words.is_empty());
        let retained = blocked
            .retained_owner
            .expect("SharedRetarget-origin after-plan retry retains Attract-origin custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::RootTransitionPending {
                transition: None,
                ..
            }
        ));
        assert!(retained.pending_root_plan.is_some());
        assert_eq!(
            retained
                .pending_root_dispatcher_continuation
                .as_ref()
                .expect("SharedRetarget-origin retry retains dispatcher cursor")
                .next_slot,
            Some(ActorTaskSlot::Secondary)
        );

        resolve_run_away_audio(&mut manager);
        let mut retry_words = VecDeque::from([0x3333_u32, 0x4444, 0x5555]);
        let mut retry_consumed = Vec::new();
        let mut allocations = 0;
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1_001_002,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                let word = retry_words.pop_front().expect(
                    "SharedRetarget-origin retry consumes acquiring constructors then Fleeing",
                );
                retry_consumed.push(word);
                word
            },
            |_| panic!("class-10 retry cannot allocate Wander"),
            |_| panic!("class-10 retry cannot allocate Go-To-Job"),
            |_| {
                allocations += 1;
                OrdinaryType9RunAwayAllocationDecision::Prepared
            },
            |_| panic!("class-10 retry cannot allocate Attract"),
            |_| panic!("even class-10 graph has no Candidate target handoff"),
        );

        assert_eq!(
            allocations, 2,
            "SharedRetarget-origin retry must consume both acquiring allocations: {:?} consumed {retry_consumed:?}",
            tick.outcome
        );
        assert_eq!(retry_consumed, [0x3333, 0x4444, 0x5555]);
        assert!(retry_words.is_empty());
        let constructors = match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootRunAwayPublished {
                entity_id: seen_entity,
                constructors_by_phase,
            } if seen_entity == entity_id => constructors_by_phase,
            outcome => panic!(
                "SharedRetarget-origin retry must apply the frozen class-10 plan: {outcome:?}"
            ),
        };
        assert_eq!(constructors[0].random_sample_low16, 0x3333);
        assert_eq!(constructors[1].random_sample_low16, 0x4444);
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("re-rooted actor remains live");
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::RunAway(task))
                    if task.target_id() == RUN_AWAY_TARGET_ID && task.elapsed_ms() == 0
            ),
            "SharedRetarget-origin retry must resume at Secondary and publish Fleeing: {:?}",
            entity.actor_task_state(ActorTaskSlot::Primary)
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    }

    #[test]
    fn expired_shared_retarget_after_outer_tail_applies_attract_and_visits_newborn_cue() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 0x1000, 0xCAFE_D2F6, 0xBEEF_1234]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                first_words
                    .pop_front()
                    .expect("selector, parity, Cue, local-Wander")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                entity_id: seen_entity,
                ..
            } if seen_entity == entity_id
        ));
        let mut owner = first
            .retained_owner
            .expect("class-45 retains scheduler custody");
        assert!(matches!(
            owner.outer_tail,
            Some(OrdinaryType9OuterTailCustody::Complete { .. })
        ));

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("published Attract Primary is SharedRetarget")
            };
            assert_eq!(state.elapsed_ms(), 0);
            state.before_callback(900_000);
            assert_eq!(state.elapsed_ms(), 900);
            let Some(OrdinaryType9GoToJobRootPublication::AttractAttention(publication)) =
                owner.root_publication.as_mut()
            else {
                panic!("first visit published Attract")
            };
            assert!(publication.refresh_after_owned_dispatch(entity));
        }

        let mut words = VecDeque::from([0x1111_u32, 0x2222, 3_063, 0, 0x5555, 0x6666]);
        let mut consumed = Vec::new();
        let mut phases = Vec::new();
        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1_001_000,
                global_elapsed_micros: 1_001_001,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                let word = words.pop_front().expect(
                    "SharedRetarget mover, class-45 selector, even constructor, then suffix",
                );
                consumed.push(word);
                word
            },
            |_| panic!("class-45 reselection cannot allocate Wander"),
            |_| panic!("class-45 reselection cannot allocate Go-To-Job"),
            |_| panic!("class-45 reselection cannot allocate Run Away"),
            |preparation| {
                phases.push(preparation.phase_index);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| panic!("even class-45 graph has no Candidate target handoff"),
        );

        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionPublished {
                    entity_id: seen_entity,
                    ..
                } if seen_entity == entity_id
            ),
            "expired SharedRetarget after completed tail must apply a root: {:?}",
            tick.outcome
        );
        assert_eq!(phases, [1, 2]);
        assert_eq!(consumed, [0x1111, 0x2222, 3_063, 0, 0x5555, 0x6666]);
        assert!(words.is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("re-rooted actor remains live");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 125
            ),
            "Primary expiry must resume at Secondary and age the newborn Cue: {:?}",
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        );
        let retained = tick
            .retained_owner
            .expect("SharedRetarget-origin root retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn odd_attract_candidate_accepts_target_and_publishes_route_in_the_same_pass() {
        let live_text = RefCell::new(Vec::new());
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let predecessor_task_id = manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);
        let mut phases = Vec::new();
        let mut target_allocations = 0;

        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |preparation| {
                phases.push(preparation.phase_index);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| {
                target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
            },
        );

        let constructor = match tick.outcome {
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished {
                entity_id: seen_entity,
                target_id: ROOT_PLAYER_ID,
                constructor,
            } if seen_entity == entity_id => constructor,
            outcome => panic!("unexpected odd class-45 outcome: {outcome:?}"),
        };
        assert!(words.is_empty());
        assert_eq!(phases, [0, 1, 2]);
        assert_eq!(target_allocations, 1);
        assert_eq!(constructor.constructor_suffix.random_word, 0x6666);
        assert_eq!(live_text.borrow().len(), 1);
        world_fx.process_pending();
        let sounds = world_fx.take_positional_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].sound_id, 72);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("target route actor remains live");
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(predecessor_task_id)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.target_id() == Some(ROOT_PLAYER_ID) && route.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let retained = tick
            .retained_owner
            .expect("target route publication retains custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(retained.root_publication.is_none());
        assert!(retained.root_attract_attention_initial_owners.is_none());
        assert!(retained
            .root_attract_attention_target_route
            .as_ref()
            .is_some_and(|route| route.validate(entity).is_ok()));
    }

    #[test]
    fn published_target_route_runs_selected_mover_on_the_next_frame() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");
        let animation_before = manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .actor_animation_runtime;

        let next = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("live Target Route cannot allocate Wander"),
            |_| panic!("live Target Route cannot allocate Go-To-Job"),
            |_| panic!("live Target Route cannot allocate Run Away"),
            |_| panic!("live Target Route cannot allocate Attract"),
            |_| panic!("live Target Route cannot allocate another target"),
        );

        assert!(
            matches!(
                next.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "published Target Route must complete one selected-mover visit: {:?}",
            next.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the live route committed the neutral selected-mover animation stage"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.target_id() == Some(ROOT_PLAYER_ID) && route.elapsed_ms() == 20
        ));
        let retained = next
            .retained_owner
            .expect("live Target Route retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(retained
            .root_attract_attention_target_route
            .as_ref()
            .is_some_and(|route| route.validate(entity).is_ok()));
    }

    #[test]
    fn published_target_route_expiry_parks_transition_without_applying_a_root() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (scheduler_elapsed_before, animation_before) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("published Primary must remain Target Route");
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_tasks.task_state_mut(task_id)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(
                route.advance_frame(5_000_000),
                crate::wander_near_location::WanderNearLifetimeStatus::Active
            );
            assert_eq!(route.elapsed_ms(), 5_000);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.actor_animation_runtime,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let expired = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("expired Target Route cannot allocate Wander"),
            |_| panic!("expired Target Route cannot allocate Go-To-Job"),
            |_| panic!("expired Target Route cannot allocate Run Away"),
            |_| panic!("expired Target Route cannot allocate Attract"),
            |_| panic!("expired Target Route cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        assert!(
            matches!(
                expired.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::TargetRouteTransitionPending,
                    ..
                }
            ),
            "elapsed > 5000 must run the mover then park the unsuppressed lifetime request: {:?}",
            expired.outcome
        );
        let retained = expired
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "unsuppressed LifetimeExpired consumes the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before the selected-mover expiry frame"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("expiry leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 5_020);
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the expiry frame still commits the neutral selected-mover animation stage"
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());

        let collision_after_expiry = entity.collision.clone();
        let visits_after_expiry = live_task_visits(&entity.actor_tasks);
        let animation_after_expiry = entity.actor_animation_runtime;
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked expiry cannot allocate Wander"),
            |_| panic!("parked expiry cannot allocate Go-To-Job"),
            |_| panic!("parked expiry cannot allocate Run Away"),
            |_| panic!("parked expiry cannot allocate Attract"),
            |_| panic!("parked expiry cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_expiry);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_expiry);
        assert_eq!(entity.actor_animation_runtime, animation_after_expiry);
    }

    #[test]
    fn published_target_route_zero_predicate_parks_transition_without_applying_a_root() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (scheduler_elapsed_before, animation_before) = {
            let actor_position = manager
                .entity_mut_for_test(entity_id)
                .expect("published actor remains live")
                .position_raw();
            let player = manager
                .entity_mut_for_test(ROOT_PLAYER_ID)
                .expect("published Target Route retains its player target");
            let mut player_position = player.position_raw();
            player_position[0] = actor_position[0].wrapping_add(0x0F00);
            player.set_motion_raw(player_position, [0; 3]);
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.actor_animation_runtime,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let tagged = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("zero-predicate cannot allocate Wander"),
            |_| panic!("zero-predicate cannot allocate Go-To-Job"),
            |_| panic!("zero-predicate cannot allocate Run Away"),
            |_| panic!("zero-predicate cannot allocate Attract"),
            |_| panic!("zero-predicate cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        assert!(
            matches!(
                tagged.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::TargetRouteTransitionPending,
                    ..
                }
            ),
            "out-of-range live target must run the mover then park ZeroPredicate: {:?}",
            tagged.outcome
        );
        let retained = tagged
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "unsuppressed tagged ZeroPredicate consumes the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local range validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "ZeroPredicate still commits the neutral selected-mover animation stage"
        );
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());

        let collision_after_tag = entity.collision.clone();
        let visits_after_tag = live_task_visits(&entity.actor_tasks);
        let animation_after_tag = entity.actor_animation_runtime;
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 40_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked zero-predicate cannot allocate Wander"),
            |_| panic!("parked zero-predicate cannot allocate Go-To-Job"),
            |_| panic!("parked zero-predicate cannot allocate Run Away"),
            |_| panic!("parked zero-predicate cannot allocate Attract"),
            |_| panic!("parked zero-predicate cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_tag);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_tag);
        assert_eq!(entity.actor_animation_runtime, animation_after_tag);
    }

    #[test]
    fn published_target_route_axis_mismatch_consumes_prefix_without_mover_or_replay() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let expected = crate::main_base_type9_abort::LEVEL_ONE_TYPE9_COMMON_AXIS_DESCRIPTOR;
        let actual = CommonAxisDescriptor {
            strict_axis_limit_raw: expected.strict_axis_limit_raw.wrapping_add(1),
            ..expected
        };
        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Known(actual);
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("axis mismatch cannot allocate Wander"),
            |_| panic!("axis mismatch cannot allocate Go-To-Job"),
            |_| panic!("axis mismatch cannot allocate Run Away"),
            |_| panic!("axis mismatch cannot allocate Attract"),
            |_| panic!("axis mismatch cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "axis mismatch must block after callback entry: {:?}",
                failed.outcome
            )
        };
        let OrdinaryType9GoToJobProductionBlock::TargetRoute(
            OrdinaryType9AttractAttentionTargetRouteTickError::Callback(
                crate::shared_target_route::SharedTargetRouteCallbackError::RoutePredicate {
                    error,
                    ..
                },
            ),
        ) = reason
        else {
            panic!("axis mismatch must be a consumed route-predicate failure: {reason:?}")
        };
        assert_eq!(
            error,
            OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorMismatch {
                expected,
                actual
            }
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "post-entry adapter failure consumes the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local axis validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "predicate failure must not commit the mover's staged route state"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let collision_after_failure = entity.collision.clone();
        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked failure cannot allocate Wander"),
            |_| panic!("parked failure cannot allocate Go-To-Job"),
            |_| panic!("parked failure cannot allocate Run Away"),
            |_| panic!("parked failure cannot allocate Attract"),
            |_| panic!("parked failure cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
    }

    #[test]
    fn published_target_route_unresolved_axis_consumes_prefix_without_mover_or_replay() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.actor_common_axis_descriptor = RetailRuntimeValue::Unresolved;
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved axis cannot allocate Wander"),
            |_| panic!("unresolved axis cannot allocate Go-To-Job"),
            |_| panic!("unresolved axis cannot allocate Run Away"),
            |_| panic!("unresolved axis cannot allocate Attract"),
            |_| panic!("unresolved axis cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved axis must block after callback entry: {:?}",
                failed.outcome
            )
        };
        let OrdinaryType9GoToJobProductionBlock::TargetRoute(
            OrdinaryType9AttractAttentionTargetRouteTickError::Callback(
                crate::shared_target_route::SharedTargetRouteCallbackError::RoutePredicate {
                    error,
                    ..
                },
            ),
        ) = reason
        else {
            panic!("unresolved axis must be a consumed route-predicate failure: {reason:?}")
        };
        assert_eq!(
            error,
            OrdinaryType9GoToJobAdapterError::ActorCommonAxisDescriptorUnavailable
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "post-entry adapter failure consumes the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local axis validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "predicate failure must not commit the mover's staged route state"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let collision_after_failure = entity.collision.clone();
        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked failure cannot allocate Wander"),
            |_| panic!("parked failure cannot allocate Go-To-Job"),
            |_| panic!("parked failure cannot allocate Run Away"),
            |_| panic!("parked failure cannot allocate Attract"),
            |_| panic!("parked failure cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
    }

    #[test]
    fn published_target_route_unresolved_sub_a_fails_preflight_without_mover_or_route_age() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            known_sub_a,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let RetailRuntimeValue::Known(Some(known_sub_a)) = entity.sub_a_propulsion_runtime
            else {
                panic!("published Target Route must retain its exact Sub-A runtime")
            };
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Unresolved;
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                known_sub_a,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved Sub-A cannot allocate Wander"),
            |_| panic!("unresolved Sub-A cannot allocate Go-To-Job"),
            |_| panic!("unresolved Sub-A cannot allocate Run Away"),
            |_| panic!("unresolved Sub-A cannot allocate Attract"),
            |_| panic!("unresolved Sub-A cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved Sub-A must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9GoToJobProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::SubARuntimeUnavailable
                )
            )
        );
        let retained = failed
            .retained_owner
            .expect("production retains mutation-free preflight custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );
        assert!(
            retained
                .root_attract_attention_target_route
                .as_ref()
                .is_some_and(|route| {
                    manager
                        .entity_mut_for_test(entity_id)
                        .is_some_and(|entity| route.validate(entity).is_ok())
                }),
            "Target Route preflight retains the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "production commits the outer scheduler prefix before Target Route preflight"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("preflight failure leaves the published wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "preflight must not age or stage the route"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("retryable preflight cannot allocate Wander"),
            |_| panic!("retryable preflight cannot allocate Go-To-Job"),
            |_| panic!("retryable preflight cannot allocate Run Away"),
            |_| panic!("retryable preflight cannot allocate Attract"),
            |_| panic!("retryable preflight cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9GoToJobProductionBlock::TargetRoute(
                    OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                        OrdinaryType9AttractAttentionTargetRoutePreflightError::SubARuntimeUnavailable
                    )
                ),
            }
        );
        let retained = replay
            .retained_owner
            .expect("retryable preflight still retains custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each Active retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(known_sub_a));
        }

        let recovered = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 3,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("restored Sub-A cannot allocate Wander"),
            |_| panic!("restored Sub-A cannot allocate Go-To-Job"),
            |_| panic!("restored Sub-A cannot allocate Run Away"),
            |_| panic!("restored Sub-A cannot allocate Attract"),
            |_| panic!("restored Sub-A cannot allocate another target"),
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "restored Sub-A must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the recovered route committed the neutral selected-mover animation stage"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("recovered Target Route must remain installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
    }

    #[test]
    fn published_target_route_unresolved_sub_a_target_speed_fails_preflight_without_mover_or_route_age(
    ) {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            known_sub_a,
            unresolved_speed_sub_a,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let RetailRuntimeValue::Known(Some(known_sub_a)) = entity.sub_a_propulsion_runtime
            else {
                panic!("published Target Route must retain its exact Sub-A runtime")
            };
            let unresolved_speed_sub_a = SubAPropulsionRuntime::from_retail_words(
                RetailRuntimeValue::Unresolved,
                known_sub_a.direction_multiplier(),
                known_sub_a.drive_scale_percent(),
            );
            entity.sub_a_propulsion_runtime =
                RetailRuntimeValue::Known(Some(unresolved_speed_sub_a));
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                known_sub_a,
                unresolved_speed_sub_a,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved Sub-A target speed cannot allocate Wander"),
            |_| panic!("unresolved Sub-A target speed cannot allocate Go-To-Job"),
            |_| panic!("unresolved Sub-A target speed cannot allocate Run Away"),
            |_| panic!("unresolved Sub-A target speed cannot allocate Attract"),
            |_| panic!("unresolved Sub-A target speed cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved Sub-A target speed must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9GoToJobProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::SubATargetSpeedUnresolved
                )
            )
        );
        let retained = failed
            .retained_owner
            .expect("production retains mutation-free preflight custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );
        assert!(
            retained
                .root_attract_attention_target_route
                .as_ref()
                .is_some_and(|route| {
                    manager
                        .entity_mut_for_test(entity_id)
                        .is_some_and(|entity| route.validate(entity).is_ok())
                }),
            "Target Route preflight retains the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "production commits the outer scheduler prefix before Target Route preflight"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("preflight failure leaves the published wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "preflight must not age or stage the route"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(
            entity.sub_a_propulsion_runtime,
            RetailRuntimeValue::Known(Some(unresolved_speed_sub_a))
        );
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("retryable preflight cannot allocate Wander"),
            |_| panic!("retryable preflight cannot allocate Go-To-Job"),
            |_| panic!("retryable preflight cannot allocate Run Away"),
            |_| panic!("retryable preflight cannot allocate Attract"),
            |_| panic!("retryable preflight cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9GoToJobProductionBlock::TargetRoute(
                    OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                        OrdinaryType9AttractAttentionTargetRoutePreflightError::SubATargetSpeedUnresolved
                    )
                ),
            }
        );
        let retained = replay
            .retained_owner
            .expect("retryable preflight still retains custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each Active retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(known_sub_a));
        }

        let recovered = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 3,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("restored Sub-A target speed cannot allocate Wander"),
            |_| panic!("restored Sub-A target speed cannot allocate Go-To-Job"),
            |_| panic!("restored Sub-A target speed cannot allocate Run Away"),
            |_| panic!("restored Sub-A target speed cannot allocate Attract"),
            |_| panic!("restored Sub-A target speed cannot allocate another target"),
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "restored Sub-A target speed must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the recovered route committed the neutral selected-mover animation stage"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("recovered Target Route must remain installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
    }

    #[test]
    fn published_target_route_unresolved_animation_fails_preflight_without_mover_or_route_age() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            known_animation,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let RetailRuntimeValue::Known(Some(known_animation)) = entity.actor_animation_runtime
            else {
                panic!("published Target Route must retain its exact animation runtime")
            };
            entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                known_animation,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved animation cannot allocate Wander"),
            |_| panic!("unresolved animation cannot allocate Go-To-Job"),
            |_| panic!("unresolved animation cannot allocate Run Away"),
            |_| panic!("unresolved animation cannot allocate Attract"),
            |_| panic!("unresolved animation cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved animation must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9GoToJobProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::ActorAnimationRuntimeUnavailable
                )
            )
        );
        let retained = failed
            .retained_owner
            .expect("production retains mutation-free preflight custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );
        assert!(
            retained
                .root_attract_attention_target_route
                .as_ref()
                .is_some_and(|route| {
                    manager
                        .entity_mut_for_test(entity_id)
                        .is_some_and(|entity| route.validate(entity).is_ok())
                }),
            "Target Route preflight retains the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "production commits the outer scheduler prefix before Target Route preflight"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("preflight failure leaves the published wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "preflight must not age or stage the route"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("retryable preflight cannot allocate Wander"),
            |_| panic!("retryable preflight cannot allocate Go-To-Job"),
            |_| panic!("retryable preflight cannot allocate Run Away"),
            |_| panic!("retryable preflight cannot allocate Attract"),
            |_| panic!("retryable preflight cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9GoToJobProductionBlock::TargetRoute(
                    OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                        OrdinaryType9AttractAttentionTargetRoutePreflightError::ActorAnimationRuntimeUnavailable
                    )
                ),
            }
        );
        let retained = replay
            .retained_owner
            .expect("retryable preflight still retains custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each Active retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
            entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(known_animation));
        }

        let recovered = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 3,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("restored animation cannot allocate Wander"),
            |_| panic!("restored animation cannot allocate Go-To-Job"),
            |_| panic!("restored animation cannot allocate Run Away"),
            |_| panic!("restored animation cannot allocate Attract"),
            |_| panic!("restored animation cannot allocate another target"),
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "restored animation must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(known_animation)),
            "the recovered route committed the neutral selected-mover animation stage"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("recovered Target Route must remain installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
    }

    #[test]
    fn published_target_route_unresolved_physical_body_basis_fails_preflight_without_mover_or_route_age(
    ) {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        // The published F70 tail observes a Known body basis. Consume that
        // Complete custody with the already-closed animation preflight so the
        // Active Target Route visit can unset the basis without
        // OuterTailStateMismatch.
        let known_animation = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let RetailRuntimeValue::Known(Some(known_animation)) = entity.actor_animation_runtime
            else {
                panic!("published Target Route must retain its exact animation runtime")
            };
            entity.actor_animation_runtime = RetailRuntimeValue::Unresolved;
            known_animation
        };
        let armed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("animation preflight cannot allocate Wander"),
            |_| panic!("animation preflight cannot allocate Go-To-Job"),
            |_| panic!("animation preflight cannot allocate Run Away"),
            |_| panic!("animation preflight cannot allocate Attract"),
            |_| panic!("animation preflight cannot allocate another target"),
        );
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = armed.outcome else {
            panic!(
                "animation setup must reach Active Target Route preflight: {:?}",
                armed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9GoToJobProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::ActorAnimationRuntimeUnavailable
                )
            )
        );
        let retained = armed
            .retained_owner
            .expect("animation preflight retains Active custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            known_basis,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let RetailRuntimeValue::Known(known_basis) = entity.physical_body_basis_q31 else {
                panic!("published Target Route must retain its exact body basis")
            };
            entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(known_animation));
            entity.physical_body_basis_q31 = RetailRuntimeValue::Unresolved;
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                known_basis,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved body basis cannot allocate Wander"),
            |_| panic!("unresolved body basis cannot allocate Go-To-Job"),
            |_| panic!("unresolved body basis cannot allocate Run Away"),
            |_| panic!("unresolved body basis cannot allocate Attract"),
            |_| panic!("unresolved body basis cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved body basis must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9GoToJobProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::PhysicalBodyBasisUnavailable
                )
            )
        );
        let retained = failed
            .retained_owner
            .expect("production retains mutation-free preflight custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );
        assert!(
            retained
                .root_attract_attention_target_route
                .as_ref()
                .is_some_and(|route| {
                    manager
                        .entity_mut_for_test(entity_id)
                        .is_some_and(|entity| route.validate(entity).is_ok())
                }),
            "Target Route preflight retains the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "production commits the outer scheduler prefix before Target Route preflight"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("preflight failure leaves the published wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "preflight must not age or stage the route"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(known_animation))
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Unresolved
        );
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 3,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("retryable preflight cannot allocate Wander"),
            |_| panic!("retryable preflight cannot allocate Go-To-Job"),
            |_| panic!("retryable preflight cannot allocate Run Away"),
            |_| panic!("retryable preflight cannot allocate Attract"),
            |_| panic!("retryable preflight cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9GoToJobProductionBlock::TargetRoute(
                    OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                        OrdinaryType9AttractAttentionTargetRoutePreflightError::PhysicalBodyBasisUnavailable
                    )
                ),
            }
        );
        let retained = replay
            .retained_owner
            .expect("retryable preflight still retains custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each Active retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(known_basis);
        }

        let recovered = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 4,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("restored body basis cannot allocate Wander"),
            |_| panic!("restored body basis cannot allocate Go-To-Job"),
            |_| panic!("restored body basis cannot allocate Run Away"),
            |_| panic!("restored body basis cannot allocate Attract"),
            |_| panic!("restored body basis cannot allocate another target"),
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "restored body basis must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(known_animation)),
            "the recovered route committed the neutral selected-mover animation stage"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("recovered Target Route must remain installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
    }

    #[test]
    fn published_target_route_unresolved_immutable_anchor_fails_preflight_without_mover_or_route_age(
    ) {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            known_anchor,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let known_anchor = {
                let selected = entity
                    .ordinary_type9_selected_component_runtime
                    .as_mut()
                    .expect("published Target Route retains selected component custody");
                let RetailRuntimeValue::Known(known_anchor) =
                    selected.components().immutable_anchor_raw_at_0x90()
                else {
                    panic!("published Target Route must retain its exact immutable anchor")
                };
                selected.set_immutable_anchor_for_test(RetailRuntimeValue::Unresolved);
                known_anchor
            };
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                known_anchor,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved immutable anchor cannot allocate Wander"),
            |_| panic!("unresolved immutable anchor cannot allocate Go-To-Job"),
            |_| panic!("unresolved immutable anchor cannot allocate Run Away"),
            |_| panic!("unresolved immutable anchor cannot allocate Attract"),
            |_| panic!("unresolved immutable anchor cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved immutable anchor must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9GoToJobProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::ImmutableAnchorUnavailable
                )
            )
        );
        let retained = failed
            .retained_owner
            .expect("production retains mutation-free preflight custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );
        assert!(
            retained
                .root_attract_attention_target_route
                .as_ref()
                .is_some_and(|route| {
                    manager
                        .entity_mut_for_test(entity_id)
                        .is_some_and(|entity| route.validate(entity).is_ok())
                }),
            "Target Route preflight retains the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "production commits the outer scheduler prefix before Target Route preflight"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("preflight failure leaves the published wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "preflight must not age or stage the route"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("retryable preflight cannot allocate Wander"),
            |_| panic!("retryable preflight cannot allocate Go-To-Job"),
            |_| panic!("retryable preflight cannot allocate Run Away"),
            |_| panic!("retryable preflight cannot allocate Attract"),
            |_| panic!("retryable preflight cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9GoToJobProductionBlock::TargetRoute(
                    OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                        OrdinaryType9AttractAttentionTargetRoutePreflightError::ImmutableAnchorUnavailable
                    )
                ),
            }
        );
        let retained = replay
            .retained_owner
            .expect("retryable preflight still retains custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::Active
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each Active retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
            entity
                .ordinary_type9_selected_component_runtime
                .as_mut()
                .expect("retryable preflight retains selected component custody")
                .set_immutable_anchor_for_test(RetailRuntimeValue::Known(known_anchor));
        }

        let recovered = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 3,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("restored immutable anchor cannot allocate Wander"),
            |_| panic!("restored immutable anchor cannot allocate Go-To-Job"),
            |_| panic!("restored immutable anchor cannot allocate Run Away"),
            |_| panic!("restored immutable anchor cannot allocate Attract"),
            |_| panic!("restored immutable anchor cannot allocate another target"),
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "restored immutable anchor must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the recovered route committed the neutral selected-mover animation stage"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("recovered Target Route must remain installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
    }

    #[test]
    fn published_target_route_in_callback_wrapper_drops_publication_before_preflight() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            collision_before,
            visits_before,
            private_position_before,
        ) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .expect("published Target Route retains its Primary wrapper");
            assert_eq!(
                entity.actor_tasks.wrapper_flags(task_id),
                Some(ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: false,
                })
            );
            entity.actor_tasks.set_wrapper_flags_for_test(
                task_id,
                ActorTaskWrapperFlags {
                    alive: true,
                    in_callback: true,
                },
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                entity.collision.clone(),
                live_task_visits(&entity.actor_tasks),
                route.private_state().target_position_raw,
            )
        };
        let mut callback_random_calls = 0;
        let dropped = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("in-callback wrapper cannot allocate Wander"),
            |_| panic!("in-callback wrapper cannot allocate Go-To-Job"),
            |_| panic!("in-callback wrapper cannot allocate Run Away"),
            |_| panic!("in-callback wrapper cannot allocate Attract"),
            |_| panic!("in-callback wrapper cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        assert_eq!(
            dropped.outcome,
            OrdinaryType9GoToJobProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType9GoToJobProductionDrop::RootPublicationMismatch,
            }
        );
        assert!(
            dropped.retained_owner.is_none(),
            "publication failure drops the Target Route owner before preflight"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision, collision_before,
            "RootPublicationMismatch must not commit the outer scheduler prefix"
        );
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            scheduler_elapsed_before
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("publication drop leaves the published wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 0);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_before);
        assert_eq!(
            entity.actor_tasks.wrapper_flags(
                entity
                    .actor_tasks
                    .task_in_slot(ActorTaskSlot::Primary)
                    .expect("published wrapper remains")
            ),
            Some(ActorTaskWrapperFlags {
                alive: true,
                in_callback: true,
            })
        );
    }

    #[test]
    fn published_target_route_metadata_contract_mismatch_is_unreachable_without_forging_entity_type(
    ) {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(entity.entity_type, LEVEL_ONE_TYPE9_ENTITY_TYPE);
            let metadata = manager
                .type_runtime_metadata(9)
                .expect("published Target Route retains the type-9 metadata row");
            assert!(
                exact_level_one_type9_metadata(metadata),
                "production supplies the exact Level-1 type-9 row, not a forged contract"
            );
        }

        let next = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| 1,
            |_| panic!("intact type-9 contract cannot allocate Wander"),
            |_| panic!("intact type-9 contract cannot allocate Go-To-Job"),
            |_| panic!("intact type-9 contract cannot allocate Run Away"),
            |_| panic!("intact type-9 contract cannot allocate Attract"),
            |_| panic!("intact type-9 contract cannot allocate another target"),
        );
        assert!(
            !matches!(
                next.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::TargetRoute(
                        OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                            OrdinaryType9AttractAttentionTargetRoutePreflightError::MetadataContractMismatch
                        )
                    ),
                    ..
                }
            ),
            "MetadataContractMismatch requires forging entity_type or the type-9 row: {:?}",
            next.outcome
        );
        assert!(
            matches!(
                next.outcome,
                OrdinaryType9GoToJobProductionOutcome::PostBasisTailPending {
                    entity_id: seen_entity,
                    callback_elapsed_micros: 20_000,
                } if seen_entity == entity_id
            ),
            "intact type-9 contract must enter the selected mover: {:?}",
            next.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.entity_type, LEVEL_ONE_TYPE9_ENTITY_TYPE);
        assert!(exact_level_one_type9_metadata(
            manager
                .type_runtime_metadata(9)
                .expect("production still reads type_runtime_metadata(9)")
        ));
    }

    #[test]
    fn published_target_route_dying_target_skips_mover_and_parks_transition() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            manager
                .entity_mut_for_test(ROOT_PLAYER_ID)
                .expect("published Target Route retains its player target")
                .collision
                .state_flags_at_0x08 = RetailStateWord::exact(1 | DYING_STATE_BIT);
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("dying target cannot allocate Wander"),
            |_| panic!("dying target cannot allocate Go-To-Job"),
            |_| panic!("dying target cannot allocate Run Away"),
            |_| panic!("dying target cannot allocate Attract"),
            |_| panic!("dying target cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        assert!(
            matches!(
                failed.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::TargetRouteTransitionPending,
                    ..
                }
            ),
            "dying target must skip the mover and park the unsuppressed tagged result: {:?}",
            failed.outcome
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "unsuppressed tagged InvalidTarget consumes the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local target validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "invalid-target skip must not commit the mover's staged route state"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let collision_after_failure = entity.collision.clone();
        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked failure cannot allocate Wander"),
            |_| panic!("parked failure cannot allocate Go-To-Job"),
            |_| panic!("parked failure cannot allocate Run Away"),
            |_| panic!("parked failure cannot allocate Attract"),
            |_| panic!("parked failure cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
    }

    #[test]
    fn published_target_route_inactive_target_skips_mover_and_parks_transition() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            manager
                .entity_mut_for_test(ROOT_PLAYER_ID)
                .expect("published Target Route retains its player target")
                .active = false;
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("inactive target cannot allocate Wander"),
            |_| panic!("inactive target cannot allocate Go-To-Job"),
            |_| panic!("inactive target cannot allocate Run Away"),
            |_| panic!("inactive target cannot allocate Attract"),
            |_| panic!("inactive target cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        assert!(
            matches!(
                failed.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::TargetRouteTransitionPending,
                    ..
                }
            ),
            "inactive target must skip the mover and park the unsuppressed tagged result: {:?}",
            failed.outcome
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "unsuppressed tagged InvalidTarget consumes the exact route owner"
        );
        assert!(
            !manager
                .entity_mut_for_test(ROOT_PLAYER_ID)
                .expect("inactive target remains in the world")
                .active
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local target validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "invalid-target skip must not commit the mover's staged route state"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let collision_after_failure = entity.collision.clone();
        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked failure cannot allocate Wander"),
            |_| panic!("parked failure cannot allocate Go-To-Job"),
            |_| panic!("parked failure cannot allocate Run Away"),
            |_| panic!("parked failure cannot allocate Attract"),
            |_| panic!("parked failure cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
    }

    #[test]
    fn published_target_route_missing_target_skips_mover_and_parks_transition() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            manager.remove_entity_for_test(ROOT_PLAYER_ID);
            assert!(
                manager.entity_mut_for_test(ROOT_PLAYER_ID).is_none(),
                "missing-target fixture must drop the published player from the live list"
            );
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("missing target cannot allocate Wander"),
            |_| panic!("missing target cannot allocate Go-To-Job"),
            |_| panic!("missing target cannot allocate Run Away"),
            |_| panic!("missing target cannot allocate Attract"),
            |_| panic!("missing target cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        assert!(
            matches!(
                failed.outcome,
                OrdinaryType9GoToJobProductionOutcome::Blocked {
                    reason: OrdinaryType9GoToJobProductionBlock::TargetRouteTransitionPending,
                    ..
                }
            ),
            "missing target must skip the mover and park the unsuppressed tagged result: {:?}",
            failed.outcome
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "unsuppressed tagged InvalidTarget consumes the exact route owner"
        );
        assert!(manager.entity_mut_for_test(ROOT_PLAYER_ID).is_none());

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local target validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "invalid-target skip must not commit the mover's staged route state"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let collision_after_failure = entity.collision.clone();
        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked failure cannot allocate Wander"),
            |_| panic!("parked failure cannot allocate Go-To-Job"),
            |_| panic!("parked failure cannot allocate Run Away"),
            |_| panic!("parked failure cannot allocate Attract"),
            |_| panic!("parked failure cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
    }

    #[test]
    fn published_target_route_unresolved_target_state_consumes_prefix_without_mover_or_replay() {
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
            0x6666,
        ]);

        let first = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("selector, odd graph, Candidate gate, and target suffix")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetRoutePublished { .. }
        ));
        let retained = first
            .retained_owner
            .expect("target route publication retains custody");

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            private_position_before,
        ) = {
            manager
                .entity_mut_for_test(ROOT_PLAYER_ID)
                .expect("published Target Route retains its player target")
                .collision
                .state_flags_at_0x08 = RetailStateWord::unknown();
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
            (
                entity.collision.recent_relation_elapsed_us_at_0x68,
                entity.ordinary_type9_selected_component_runtime,
                entity.sub_a_propulsion_runtime,
                entity.actor_animation_runtime,
                entity.heading,
                entity.velocity,
                route.private_state().target_position_raw,
            )
        };
        let RetailRuntimeValue::Known(scheduler_elapsed_before) = scheduler_elapsed_before else {
            panic!("published scheduler state must be exact")
        };
        let mut callback_random_calls = 0;
        let failed = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 1,
            },
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
            |_| panic!("unresolved target state cannot allocate Wander"),
            |_| panic!("unresolved target state cannot allocate Go-To-Job"),
            |_| panic!("unresolved target state cannot allocate Run Away"),
            |_| panic!("unresolved target state cannot allocate Attract"),
            |_| panic!("unresolved target state cannot allocate another target"),
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9GoToJobProductionOutcome::Blocked { reason, .. } = failed.outcome else {
            panic!(
                "unresolved target state must block after callback entry: {:?}",
                failed.outcome
            )
        };
        let OrdinaryType9GoToJobProductionBlock::TargetRoute(
            OrdinaryType9AttractAttentionTargetRouteTickError::Callback(
                crate::shared_target_route::SharedTargetRouteCallbackError::TargetValidation {
                    error,
                    ..
                },
            ),
        ) = reason
        else {
            panic!(
                "unresolved target state must be a consumed target-validation failure: {reason:?}"
            )
        };
        assert_eq!(
            error,
            OrdinaryType9GoToJobAdapterError::TargetStateUnresolved {
                target_id: ROOT_PLAYER_ID,
            }
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::CallbackFailurePending
        );
        assert!(
            retained.root_attract_attention_target_route.is_none(),
            "post-entry adapter failure consumes the exact route owner"
        );

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.recent_relation_elapsed_us_at_0x68,
            RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(20_000)),
            "scheduler prefix commits before callback-local target validation"
        );
        let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("consumed failure leaves retail's surviving wrapper installed")
        };
        assert_eq!(route.elapsed_ms(), 20);
        assert_eq!(
            route.private_state().target_position_raw,
            private_position_before,
            "target-validation failure must not commit the mover's staged route state"
        );
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.heading, heading_before);
        assert_eq!(entity.velocity, velocity_before);

        let collision_after_failure = entity.collision.clone();
        let visits_after_failure = live_task_visits(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
                retail_tick: 2,
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
            |_| panic!("parked failure cannot allocate Wander"),
            |_| panic!("parked failure cannot allocate Go-To-Job"),
            |_| panic!("parked failure cannot allocate Run Away"),
            |_| panic!("parked failure cannot allocate Attract"),
            |_| panic!("parked failure cannot allocate another target"),
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9GoToJobProductionOutcome::Blocked {
                reason: OrdinaryType9GoToJobProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_visits(&entity.actor_tasks), visits_after_failure);
    }

    #[test]
    fn odd_attract_target_allocation_failure_publishes_exact_fallback_once() {
        let live_text = RefCell::new(Vec::new());
        let (mut manager, owner, entity_id, _) = production_owner_fixture();
        kill_tracked_target_and_append_open_base(&mut manager, entity_id);
        append_root_player(&mut manager, entity_id);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 1, 0x3333, 0x4444, 0x5555, 0]);
        let mut phases = Vec::new();
        let mut target_allocations = 0;

        let tick = tick_ordinary_type9_go_to_job_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9GoToJobProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                resources: &resources,
                elapsed_micros: 1,
                global_elapsed_micros: 1,
                retail_tick: 0,
            },
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("failed target allocation consumes through Candidate gate only")
            },
            |_| panic!("class-45 selection cannot allocate root Wander"),
            |_| panic!("class-45 selection cannot allocate Go-To-Job"),
            |_| panic!("class-45 selection cannot allocate Run Away"),
            |preparation| {
                phases.push(preparation.phase_index);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| {
                target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Failed
            },
        );

        assert!(words.is_empty());
        assert_eq!(phases, [0, 1, 2]);
        assert_eq!(target_allocations, 1);
        assert!(matches!(
            tick.outcome,
            OrdinaryType9GoToJobProductionOutcome::RootAttractAttentionTargetInitializerFallbackPublished {
                entity_id: seen_entity,
                failure: OrdinaryType9AttractAttentionTargetInitializerFailure {
                    slot: ActorTaskSlot::Primary,
                    role: crate::attract_attention::AttractAttentionTaskRole::RouteToTarget,
                },
            } if seen_entity == entity_id
        ));
        assert_eq!(live_text.borrow().len(), 1);
        world_fx.process_pending();
        assert_eq!(world_fx.take_positional_sounds().len(), 1);

        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .expect("fallback actor remains live");
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .expect("fallback retains selected runtime")
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        let retained = tick
            .retained_owner
            .expect("target fallback retains exact scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9GoToJobProductionState::PostBasisTailPending { .. }
        ));
        assert!(retained.root_attract_attention_initial_owners.is_none());
        assert!(retained.root_attract_attention_target_route.is_none());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }
}
