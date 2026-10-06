//! Shared Type-9 class-45 production custody for birth and live root selection.
//!
//! Retail schedules the initial class-45 graph in physical Primary/Secondary/
//! Tertiary order. Candidate selection may synchronously retire both itself and
//! its sibling Cue while publishing Target Route, so this owner keeps the
//! initializer-minted leases linear and never reconstructs them from the task
//! table. A published Target Route resumes only on the following production
//! frame, then remains under the same scheduler, mover, root-transition, and
//! outer-tail custody as the initial graph.

use crate::gameplay_notifications::OrdinaryType9LiveNotificationContext;
use crate::ordinary_type9_current_task::{
    OrdinaryType9CurrentTaskAuthority, OrdinaryType9CurrentTaskPublication,
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
    entity_behavior::{audited_behavior_style, behavior_program, BehaviorContextRuntime},
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
    go_to_job::GoToJobTaskSpec,
    guard_location_owner::acquisition::GuardLocationEntityRef,
    job_nearby::JobCapacityState,
    main_base_abort::MainBaseAbortActorLease,
    ordinary_type9_attract_attention_cue::{
        OrdinaryType9AttractAttentionCuePreflightError, OrdinaryType9AttractAttentionCueTransition,
    },
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
    ordinary_type9_go_to_job_initializer::{
        OrdinaryType9GoToJobAllocationDecision, OrdinaryType9GoToJobCandidateEvidence,
        OrdinaryType9GoToJobConstructorEvidence, OrdinaryType9GoToJobInitializerFailure,
    },
    ordinary_type9_initial_production::{
        FreshLevel1Type9AttractAttentionProvenance, FreshLevel1Type9InitialProductionOwner,
    },
    ordinary_type9_initial_selection::{
        LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID, LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID,
        LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID,
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
        apply_ordinary_type9_root_attract_attention_parts,
        OrdinaryType9RootAttractAttentionApplicationOutcome,
        OrdinaryType9RootAttractAttentionMutableParts,
        OrdinaryType9RootAttractAttentionPreflightError,
        OrdinaryType9RootAttractAttentionPublicationAuth,
    },
    ordinary_type9_root_go_to_job_application::{
        apply_ordinary_type9_root_go_to_job_from_attract_attention,
        apply_ordinary_type9_root_go_to_job_from_attract_attention_parts,
        OrdinaryType9RootGoToJobApplicationOutcome, OrdinaryType9RootGoToJobMutableParts,
        OrdinaryType9RootGoToJobPreflightError, OrdinaryType9RootGoToJobPublicationOwner,
    },
    ordinary_type9_root_reselection::{
        plan_ordinary_type9_root_reselection, OrdinaryType9RootEntityRef,
        OrdinaryType9RootReselectionError, OrdinaryType9RootReselectionPlan,
        OrdinaryType9RootReselectionRequest, OrdinaryType9RootSelection,
    },
    ordinary_type9_root_run_away_application::{
        apply_ordinary_type9_root_run_away_from_attract_attention,
        apply_ordinary_type9_root_run_away_from_attract_attention_parts,
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
    shared_retarget_mover::SharedRetargetTransitionRequest,
    shared_target_route::{
        SharedTargetRouteCommonMoverReturn, SharedTargetRoutePredicate,
        SharedTargetRouteTargetRuntimeState,
    },
    wander_near_location::{WanderNearCommonMoverReturn, WanderNearPrivateState},
    world_fx::{TerrainCollisionContext, WorldFx},
    wrapped_axis_range::{within_wrapped_axis_range, WrappedAxisRange},
};

const BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG: u32 = 0x0000_4000;
const LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID: u16 = 85;
const LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW: u32 = 0;

/// Which linear class-45 leases remain authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionProductionState {
    InitialGraph,
    TargetRouteActive,
    TargetRouteParked {
        post_task_frame: OrdinaryType9AttractAttentionPostTaskFrame,
    },
    PostBasisTailPending {
        post_task_frame: OrdinaryType9AttractAttentionPostTaskFrame,
    },
    RootTransitionPending {
        post_task_frame: OrdinaryType9AttractAttentionPostTaskFrame,
    },
    CallbackFailurePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionPostTaskFrame {
    callback_elapsed_micros: u32,
    origin_retail_tick: u32,
    latched_effective_flags: u32,
    post_task_angles_raw: [i16; 3],
    pre_basis: RetailRuntimeValue<Type9BodyBasis>,
}

impl OrdinaryType9AttractAttentionPostTaskFrame {
    pub const fn callback_elapsed_micros(self) -> u32 {
        self.callback_elapsed_micros
    }

    fn published_body_basis(self) -> Type9BodyBasis {
        Type9BodyBasis::from_angle_words(
            self.post_task_angles_raw[0],
            self.post_task_angles_raw[1],
            self.post_task_angles_raw[2],
        )
    }
}

/// Manager-retained owner for one selected class-45 actor.
#[derive(Debug, PartialEq, Eq)]
pub struct OrdinaryType9AttractAttentionProductionOwner {
    provenance: OrdinaryType9AttractAttentionProductionProvenance,
    initial_owners: Option<OrdinaryType9AttractAttentionInitialOwners>,
    actor_lease: MainBaseAbortActorLease,
    state: OrdinaryType9AttractAttentionProductionState,
    target_route: Option<OrdinaryType9AttractAttentionTargetRouteOwner>,
    next_transaction_id: u64,
    pending_root_plan: Option<OrdinaryType9RootReselectionPlan>,
    pending_root_task_visits: Option<[Option<ActorTaskVisit>; 3]>,
    pending_root_authority: Option<OrdinaryType9AttractAttentionRootAuthority>,
    pending_root_continuation: Option<OrdinaryType9AttractAttentionDispatcherContinuation>,
    root_publication: Option<OrdinaryType9AttractAttentionRootPublication>,
    outer_tail: Option<OrdinaryType9AttractAttentionOuterTailCustody>,
    pending_outer_outcome: Option<OrdinaryType9AttractAttentionProductionOutcome>,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdinaryType9AttractAttentionProductionProvenance {
    Initial {
        original_manager_sidecar_index: usize,
        construction: FreshLevel1Type9AttractAttentionProvenance,
    },
    Current(OrdinaryType9CurrentTaskAuthority),
}

impl OrdinaryType9AttractAttentionProductionProvenance {
    fn fork_for_main_base_abort_transaction(&self) -> Self {
        match self {
            Self::Initial {
                original_manager_sidecar_index,
                construction,
            } => Self::Initial {
                original_manager_sidecar_index: *original_manager_sidecar_index,
                construction: construction.fork_for_main_base_abort_transaction(),
            },
            Self::Current(authority) => {
                Self::Current(authority.fork_for_main_base_abort_transaction())
            }
        }
    }
}

type OrdinaryType9AttractAttentionOuterTailCustody = OrdinaryType9OuterTailCustody;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrdinaryType9AttractAttentionRootAuthority {
    entity_id: u32,
    context: BehaviorContextRuntime,
    selected: OrdinaryType9SelectedComponentRuntime,
    initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
    actor_common_axis: CommonAxisDescriptor,
    sub_a_propulsion_runtime: RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    actor_animation_runtime: ActorAnimationController,
    tasks: [Option<(ActorTaskVisit, ActorTaskRuntime)>; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrdinaryType9AttractAttentionDispatcherContinuation {
    next_slot: Option<ActorTaskSlot>,
    root_facts: OrdinaryType9RootWanderEntityFacts,
    snapshots: Vec<EntitySnapshot>,
    candidate_refs: Vec<GuardLocationEntityRef>,
    callback_elapsed_micros: u32,
    global_elapsed_micros: u32,
    scheduler_mode: i32,
    pre_basis: RetailRuntimeValue<Type9BodyBasis>,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdinaryType9AttractAttentionRootPublication {
    Wander(OrdinaryType9RootWanderPublicationOwner),
    GoToJob(OrdinaryType9RootGoToJobPublicationOwner),
    RunAway(OrdinaryType9RootRunAwayPublicationOwner),
    AttractAttention(OrdinaryType9RootAttractAttentionPublicationAuth),
    InitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
    GoToJobInitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        actor_common_axis: CommonAxisDescriptor,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
    RunAwayInitializerFallback {
        entity_id: u32,
        context: BehaviorContextRuntime,
        selected: OrdinaryType9SelectedComponentRuntime,
        initial_behavior: RetailRuntimeValue<Option<crate::entity_behavior::BehaviorSelection>>,
        actor_common_axis: CommonAxisDescriptor,
        sub_a: SubAPropulsionRuntime,
        state_policy_mask: u32,
        state_policy_bits: u32,
    },
}

enum OrdinaryType9AttractAttentionCandidateCompletion {
    TargetRoute {
        target_id: u32,
        constructor: AttractAttentionTargetExecutionOutcome,
    },
    InitializerFallback {
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
}

impl OrdinaryType9AttractAttentionProductionOwner {
    pub(crate) fn adopt(
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
        actor_lease: MainBaseAbortActorLease,
    ) -> Result<Self, FreshLevel1Type9InitialProductionOwner> {
        if initial_owner.entity_id() != actor_lease.entity_id {
            return Err(initial_owner);
        }
        let (provenance, initial_owners) =
            initial_owner.into_successful_attract_attention_parts()?;
        Ok(Self {
            provenance: OrdinaryType9AttractAttentionProductionProvenance::Initial {
                original_manager_sidecar_index,
                construction: provenance,
            },
            initial_owners: Some(initial_owners),
            actor_lease,
            state: OrdinaryType9AttractAttentionProductionState::InitialGraph,
            target_route: None,
            next_transaction_id: 1,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_authority: None,
            pending_root_continuation: None,
            root_publication: None,
            outer_tail: None,
            pending_outer_outcome: None,
        })
    }

    /// Move exact class45 callback owners across a completed external writer.
    /// A completed initial graph retains its cue/candidate leases without a
    /// stale constructor pose; Target Route keeps its exact task owner.
    pub(crate) fn from_current_attract(
        mut authority: OrdinaryType9CurrentTaskAuthority,
        actor_lease: MainBaseAbortActorLease,
        next_transaction_id: u64,
    ) -> Self {
        debug_assert_eq!(authority.entity_id(), actor_lease.entity_id);
        let target_route = authority.take_target_route_continuation();
        let (initial_owners, root_publication) = match authority.take_attract_continuation() {
            Some(
                crate::ordinary_type9_current_task::OrdinaryType9AttractContinuation::Completed(
                    owners,
                ),
            ) => (Some(owners), None),
            Some(
                crate::ordinary_type9_current_task::OrdinaryType9AttractContinuation::Publication {
                    publication,
                    owners,
                },
            ) => (
                Some(owners),
                Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)),
            ),
            None => (None, None),
        };
        let state = if target_route.is_some() {
            assert!(initial_owners.is_none());
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        } else {
            assert!(
                initial_owners.is_some(),
                "current initial class45 retains its linear callbacks"
            );
            OrdinaryType9AttractAttentionProductionState::InitialGraph
        };
        Self {
            provenance: OrdinaryType9AttractAttentionProductionProvenance::Current(authority),
            initial_owners,
            actor_lease,
            state,
            target_route,
            next_transaction_id,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_authority: None,
            pending_root_continuation: None,
            root_publication,
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
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
                | OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
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
        let publication = self.root_publication.as_ref()?;
        if !matches!(
            publication,
            OrdinaryType9AttractAttentionRootPublication::Wander(..)
                | OrdinaryType9AttractAttentionRootPublication::GoToJob(..)
                | OrdinaryType9AttractAttentionRootPublication::RunAway(..)
        ) || !publication.authenticates(entity)
        {
            return None;
        }
        let publication = match self
            .root_publication
            .take()
            .expect("verified current publication")
        {
            OrdinaryType9AttractAttentionRootPublication::Wander(receipt) => {
                OrdinaryType9CurrentTaskPublication::Wander(receipt)
            }
            OrdinaryType9AttractAttentionRootPublication::GoToJob(receipt) => {
                OrdinaryType9CurrentTaskPublication::GoToJob(receipt)
            }
            OrdinaryType9AttractAttentionRootPublication::RunAway(receipt) => {
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

    /// Transfer a complete class45 visit with its actual Candidate/Cue or
    /// Target Route owner. A hit consumes no task time or constructor effects.
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
        let continuation = if let Some(route) = self.target_route.take() {
            NativeType9TaskContinuation::TargetRoute(route)
        } else if let Some(owners) = self.initial_owners.take() {
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
                        self.initial_owners = Some(owners)
                    }
                    NativeType9TaskContinuation::TargetRoute(route) => {
                        self.target_route = Some(route)
                    }
                }
                return Err(self);
            }
        };
        Ok((authority, self.next_transaction_id))
    }

    /// Presentation and relation callbacks enter only between completed actor visits.
    /// A newly adopted root graph is also idle before its first prefix.
    pub(crate) fn completed_visit_lease(
        &self,
        manager: &EntityManager,
    ) -> Option<MainBaseAbortActorLease> {
        let complete = match (&self.state, &self.outer_tail) {
            (
                OrdinaryType9AttractAttentionProductionState::InitialGraph
                | OrdinaryType9AttractAttentionProductionState::TargetRouteActive,
                None,
            ) => true,
            (
                OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
                | OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. },
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
        (post_task_owner_authority_authenticates(self, entity, manager.type_runtime_metadata(9)?)
            && manager.ordinary_type9_selected_actor_lease(self.entity_id())
                == Some(self.actor_lease))
        .then_some(self.actor_lease)
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
            Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)) => {
                publication.acknowledge_presented_view_detail(entity, previous_state)
            }
            _ => true,
        }
    }

    pub const fn actor_lease(&self) -> MainBaseAbortActorLease {
        self.actor_lease
    }

    pub const fn state(&self) -> OrdinaryType9AttractAttentionProductionState {
        self.state
    }

    pub(crate) fn fork_for_main_base_abort_transaction(&self) -> Self {
        Self {
            provenance: self.provenance.fork_for_main_base_abort_transaction(),
            initial_owners: self.initial_owners.as_ref().map(fork_initial_owners),
            actor_lease: self.actor_lease,
            state: self.state,
            target_route: self.target_route.as_ref().map(
                OrdinaryType9AttractAttentionTargetRouteOwner::fork_for_main_base_abort_transaction,
            ),
            next_transaction_id: self.next_transaction_id,
            pending_root_plan: self
                .pending_root_plan
                .as_ref()
                .map(OrdinaryType9RootReselectionPlan::fork_for_main_base_abort_transaction),
            pending_root_task_visits: self.pending_root_task_visits,
            pending_root_authority: self.pending_root_authority,
            pending_root_continuation: self.pending_root_continuation.clone(),
            root_publication: self.root_publication.as_ref().map(
                OrdinaryType9AttractAttentionRootPublication::fork_for_main_base_abort_transaction,
            ),
            outer_tail: self.outer_tail.as_ref().map(
                OrdinaryType9AttractAttentionOuterTailCustody::fork_for_main_base_abort_transaction,
            ),
            pending_outer_outcome: self.pending_outer_outcome.clone(),
        }
    }

    pub(crate) const fn main_base_abort_compatible(&self) -> bool {
        matches!(
            self.state,
            OrdinaryType9AttractAttentionProductionState::InitialGraph
        ) && matches!(
            self.provenance,
            OrdinaryType9AttractAttentionProductionProvenance::Initial { .. }
        ) && self.initial_owners.is_some()
            && self.target_route.is_none()
            && self.pending_root_plan.is_none()
            && self.pending_root_task_visits.is_none()
            && self.pending_root_authority.is_none()
            && self.pending_root_continuation.is_none()
            && self.root_publication.is_none()
            && self.outer_tail.is_none()
            && self.pending_outer_outcome.is_none()
    }

    pub(crate) fn decompose_for_main_base_abort(
        self,
    ) -> (
        usize,
        FreshLevel1Type9InitialProductionOwner,
        OrdinaryType9AttractAttentionProductionResume,
    ) {
        let initial_owners = self
            .initial_owners
            .expect("only InitialGraph may enter the Main Base transaction");
        let OrdinaryType9AttractAttentionProductionProvenance::Initial {
            original_manager_sidecar_index,
            construction,
        } = self.provenance
        else {
            unreachable!("only an initial graph may enter the Main Base transaction")
        };
        let initial_owner =
            FreshLevel1Type9InitialProductionOwner::from_successful_attract_attention_parts(
                construction,
                initial_owners,
            );
        (
            original_manager_sidecar_index,
            initial_owner,
            OrdinaryType9AttractAttentionProductionResume {
                actor_lease: self.actor_lease,
                next_transaction_id: self.next_transaction_id,
            },
        )
    }

    pub(crate) fn resume_after_main_base_abort_noop(
        original_manager_sidecar_index: usize,
        initial_owner: FreshLevel1Type9InitialProductionOwner,
        resume: OrdinaryType9AttractAttentionProductionResume,
    ) -> Result<Self, FreshLevel1Type9InitialProductionOwner> {
        if initial_owner.entity_id() != resume.actor_lease.entity_id {
            return Err(initial_owner);
        }
        let (provenance, initial_owners) =
            initial_owner.into_successful_attract_attention_parts()?;
        Ok(Self {
            provenance: OrdinaryType9AttractAttentionProductionProvenance::Initial {
                original_manager_sidecar_index,
                construction: provenance,
            },
            initial_owners: Some(initial_owners),
            actor_lease: resume.actor_lease,
            state: OrdinaryType9AttractAttentionProductionState::InitialGraph,
            target_route: None,
            next_transaction_id: resume.next_transaction_id,
            pending_root_plan: None,
            pending_root_task_visits: None,
            pending_root_authority: None,
            pending_root_continuation: None,
            root_publication: None,
            outer_tail: None,
            pending_outer_outcome: None,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9AttractAttentionProductionResume {
    actor_lease: MainBaseAbortActorLease,
    next_transaction_id: u64,
}

fn fork_initial_owners(
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

impl OrdinaryType9AttractAttentionRootPublication {
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
                selected,
                initial_behavior,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => Self::InitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected: *selected,
                initial_behavior: *initial_behavior,
                sub_a: *sub_a,
                state_policy_mask: *state_policy_mask,
                state_policy_bits: *state_policy_bits,
            },
            Self::GoToJobInitializerFallback {
                entity_id,
                context,
                selected,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => Self::GoToJobInitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected: *selected,
                initial_behavior: *initial_behavior,
                actor_common_axis: *actor_common_axis,
                sub_a: *sub_a,
                state_policy_mask: *state_policy_mask,
                state_policy_bits: *state_policy_bits,
            },
            Self::RunAwayInitializerFallback {
                entity_id,
                context,
                selected,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => Self::RunAwayInitializerFallback {
                entity_id: *entity_id,
                context: *context,
                selected: *selected,
                initial_behavior: *initial_behavior,
                actor_common_axis: *actor_common_axis,
                sub_a: *sub_a,
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
                selected,
                initial_behavior,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => {
                entity.id == *entity_id
                    && entity.active
                    && entity.ordinary_type9_pending_initial_selection.is_none()
                    && entity.main_base_type9_death_component_runtime.is_none()
                    && entity.initial_behavior == *initial_behavior
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(*context))
                    && entity.ordinary_type9_selected_component_runtime == Some(*selected)
                    && selected.kind()
                        == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(*sub_a))
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(*state_policy_mask)
                        == RetailRuntimeValue::Known(*state_policy_bits)
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none())
            }
            Self::GoToJobInitializerFallback {
                entity_id,
                context,
                selected,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            }
            | Self::RunAwayInitializerFallback {
                entity_id,
                context,
                selected,
                initial_behavior,
                actor_common_axis,
                sub_a,
                state_policy_mask,
                state_policy_bits,
            } => {
                entity.id == *entity_id
                    && entity.active
                    && entity.ordinary_type9_pending_initial_selection.is_none()
                    && entity.main_base_type9_death_component_runtime.is_none()
                    && entity.initial_behavior == *initial_behavior
                    && entity.current_behavior_context == RetailRuntimeValue::Known(Some(*context))
                    && entity.ordinary_type9_selected_component_runtime == Some(*selected)
                    && selected.kind()
                        == OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
                    && entity.actor_common_axis_descriptor
                        == RetailRuntimeValue::Known(*actor_common_axis)
                    && entity.sub_a_propulsion_runtime == RetailRuntimeValue::Known(Some(*sub_a))
                    && entity
                        .collision
                        .state_flags_at_0x08
                        .masked(*state_policy_mask)
                        == RetailRuntimeValue::Known(*state_policy_bits)
                    && ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .all(|slot| entity.actor_task_state(slot).is_none())
            }
        }
    }
}

pub struct OrdinaryType9AttractAttentionProductionFrame<'a> {
    pub dispatch_resource_text:
        &'a dyn Fn(crate::attract_attention::AttractAttentionResourceTextRequest, u32),
    pub resources: &'a ResourceCache,
    pub retail_tick: u32,
    pub elapsed_micros: u32,
    pub global_elapsed_micros: u32,
    pub candidates_in_intrusive_order: &'a [GuardLocationEntityRef],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionProductionDrop {
    EntityUnavailable,
    ActorLeaseChanged,
    InitialOwnerMismatch,
    RootPublicationMismatch,
    PostTaskFrameMismatch,
    PostBasisPublicationMismatch,
    OuterTailStateMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionProductionBlock {
    PostBasisTailPending,
    RootTransitionPending,
    CallbackFailurePending,
    RemoteSchedulerOwnerUnsupported,
    SchedulerCallbackDisabled,
    NormalSchedulerOwnerStateUnresolved,
    CallbackMassUnavailable,
    SchedulerModeUnresolved,
    SchedulerStateUnresolved,
    RuntimeMetadataUnavailable,
    RuntimeTopologyUnavailable,
    CurrentTerrainUnavailable,
    SelectedComponentRuntimeUnavailable,
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationUnavailable,
    PhysicalBodyBasisUnavailable,
    InitialGraphUnavailable,
    InitialOwnerMismatch,
    Cue(OrdinaryType9AttractAttentionCuePreflightError),
    RootReselection(OrdinaryType9RootReselectionError),
    RootWanderPreflight(OrdinaryType9RootWanderPreflightError),
    RootGoToJobPreflight(OrdinaryType9RootGoToJobPreflightError),
    RootRunAwayPreflight(OrdinaryType9RootRunAwayPreflightError),
    RootAttractAttentionPreflight(OrdinaryType9RootAttractAttentionPreflightError),
    AuthoredRunAwayAudioUnresolved,
    AuthoredRunAwayAudioMismatch {
        actual_sound_id: Option<u16>,
        actual_period_raw: u32,
    },
    UnsupportedRootSelection {
        class_id: u32,
    },
    RootTransitionRequestChanged,
    RootTaskVisitsUnavailable,
    ActiveModelUnavailable,
    OuterOwnerStateUnavailable,
    OuterOwner(crate::common_mover::type9_owner::OrdinaryType9OwnerBlock),
    OuterLifecyclePending,
    TargetRoute(
        OrdinaryType9AttractAttentionTargetRouteTickError<
            OrdinaryType9AttractAttentionAdapterError,
            OrdinaryType9AttractAttentionAdapterError,
            OrdinaryType9AttractAttentionAdapterError,
        >,
    ),
    TargetRouteAdapter(OrdinaryType9AttractAttentionAdapterError),
    TargetRouteWrapperRetired,
    Dispatcher(ActorTaskDispatcherError<OrdinaryType9AttractAttentionAdapterError>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionProductionOutcome {
    SchedulerWaiting {
        entity_id: u32,
    },
    OuterTailComplete {
        entity_id: u32,
        callback_elapsed_micros: u32,
    },
    TargetRoutePublished {
        entity_id: u32,
        target_id: u32,
        constructor: AttractAttentionTargetExecutionOutcome,
    },
    RootWanderPublished {
        entity_id: u32,
        constructor: OrdinaryType9WanderConstructorEvidence,
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
    RootInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9WanderInitializerFailure,
    },
    TargetInitializerFallbackPublished {
        entity_id: u32,
        failure: OrdinaryType9AttractAttentionTargetInitializerFailure,
    },
    SurfaceLifecycleClass14Published {
        entity_id: u32,
        task_lease: Option<crate::main_base_type9_abort::MainBaseType9ExplodingTaskLease>,
    },
    Blocked {
        entity_id: u32,
        reason: OrdinaryType9AttractAttentionProductionBlock,
    },
    Dropped {
        entity_id: u32,
        reason: OrdinaryType9AttractAttentionProductionDrop,
    },
}

impl OrdinaryType9AttractAttentionProductionOutcome {
    pub const fn entity_id(&self) -> u32 {
        match self {
            Self::SchedulerWaiting { entity_id }
            | Self::OuterTailComplete { entity_id, .. }
            | Self::TargetRoutePublished { entity_id, .. }
            | Self::RootWanderPublished { entity_id, .. }
            | Self::RootAttractAttentionPublished { entity_id, .. }
            | Self::RootAttractAttentionInitializerFallbackPublished { entity_id, .. }
            | Self::RootGoToJobPublished { entity_id, .. }
            | Self::RootGoToJobInitializerFallbackPublished { entity_id, .. }
            | Self::RootRunAwayPublished { entity_id, .. }
            | Self::RootRunAwayInitializerFallbackPublished { entity_id, .. }
            | Self::RootInitializerFallbackPublished { entity_id, .. }
            | Self::TargetInitializerFallbackPublished { entity_id, .. }
            | Self::SurfaceLifecycleClass14Published { entity_id, .. }
            | Self::Blocked { entity_id, .. }
            | Self::Dropped { entity_id, .. } => *entity_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OrdinaryType9AttractAttentionOwnerTick {
    pub outcome: OrdinaryType9AttractAttentionProductionOutcome,
    pub retained_owner: Option<OrdinaryType9AttractAttentionProductionOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdinaryType9AttractAttentionAdapterError {
    InitialOwnerMismatch,
    Candidate(OrdinaryType9AttractAttentionCandidateTickError),
    TargetAcquisition(TargetAcquisitionCallbackError),
    CandidateLeaseChanged,
    CueLeaseChanged,
    TransitionGateUnresolved,
    TargetStateUnresolved {
        target_id: u32,
    },
    TargetPositionUnavailable {
        target_id: u32,
    },
    ActorCommonAxisDescriptorUnavailable,
    ActorCommonAxisDescriptorMismatch {
        expected: CommonAxisDescriptor,
        actual: CommonAxisDescriptor,
    },
    CommonMoverUnavailable,
    RuntimeMetadataUnavailable,
    RuntimeTopologyUnavailable,
    CurrentTerrainUnavailable,
    SelectedComponentRuntimeUnavailable,
    SubARuntimeUnavailable,
    SubATargetSpeedUnresolved,
    ActorAnimationUnavailable,
    PhysicalBodyBasisUnavailable,
    CommonMoverFrameBlocked(crate::common_mover::type9::OrdinaryType9FrameBlock),
    CommonMoverCommitBlocked(crate::common_mover::type9_transaction::OrdinaryType9ExternalBlock),
}

fn preflight_normal_scheduler_owner(
    collision: &EntityCollisionRuntimeState,
) -> Result<(), OrdinaryType9AttractAttentionProductionBlock> {
    match collision
        .state_flags_at_0x08
        .masked(REMOTE_OWNED_STATE_BIT | COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) if bits & REMOTE_OWNED_STATE_BIT != 0 => {
            Err(OrdinaryType9AttractAttentionProductionBlock::RemoteSchedulerOwnerUnsupported)
        }
        RetailRuntimeValue::Known(bits)
            if bits & COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT == 0 =>
        {
            Err(OrdinaryType9AttractAttentionProductionBlock::SchedulerCallbackDisabled)
        }
        RetailRuntimeValue::Known(_) => Ok(()),
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9AttractAttentionProductionBlock::NormalSchedulerOwnerStateUnresolved)
        }
    }
}

fn authored_run_away_audio(
    metadata: &EntityTypeRuntimeMetadata,
) -> Result<RunAwayAuthoredAudio, OrdinaryType9AttractAttentionProductionBlock> {
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
            OrdinaryType9AttractAttentionProductionBlock::AuthoredRunAwayAudioMismatch {
                actual_sound_id,
                actual_period_raw,
            },
        ),
        _ => Err(OrdinaryType9AttractAttentionProductionBlock::AuthoredRunAwayAudioUnresolved),
    }
}

fn read_transition_suppressed(
    collision: &EntityCollisionRuntimeState,
) -> Result<bool, OrdinaryType9AttractAttentionAdapterError> {
    match collision
        .state_flags_at_0x08
        .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => Ok(false),
        RetailRuntimeValue::Known(_) => Ok(true),
        RetailRuntimeValue::Unresolved => {
            Err(OrdinaryType9AttractAttentionAdapterError::TransitionGateUnresolved)
        }
    }
}

enum AttractAnimationCustody<'a> {
    Live(&'a mut ActorAnimationController),
    Skipped { scratch: ActorAnimationController },
}

impl AttractAnimationCustody<'_> {
    fn runtime_mut(&mut self) -> &mut ActorAnimationController {
        match self {
            Self::Live(runtime) => runtime,
            Self::Skipped { scratch } => scratch,
        }
    }
}

fn bind_animation(
    runtime: &mut RetailRuntimeValue<Option<ActorAnimationController>>,
    scheduler_mode: i32,
) -> Result<AttractAnimationCustody<'_>, OrdinaryType9AttractAttentionAdapterError> {
    if scheduler_mode != 0 {
        return Ok(AttractAnimationCustody::Skipped {
            scratch: ActorAnimationController::from_descriptor(
                crate::main_base_type9_abort::LEVEL_ONE_TYPE9_SUB_I_DESCRIPTOR,
            )
            .expect("the exact Level-1 Type-9 descriptor has a valid animation binding"),
        });
    }
    match runtime {
        RetailRuntimeValue::Known(Some(runtime)) => Ok(AttractAnimationCustody::Live(runtime)),
        _ => Err(OrdinaryType9AttractAttentionAdapterError::ActorAnimationUnavailable),
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

const fn slot_after(slot: ActorTaskSlot) -> Option<ActorTaskSlot> {
    match slot {
        ActorTaskSlot::Primary => Some(ActorTaskSlot::Secondary),
        ActorTaskSlot::Secondary => Some(ActorTaskSlot::Tertiary),
        ActorTaskSlot::Tertiary => None,
    }
}

fn live_task_visits(owner: &ActorTaskOwner<ActorTaskRuntime>) -> [Option<ActorTaskVisit>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        owner
            .task_in_slot(slot)
            .map(|task_id| ActorTaskVisit { slot, task_id })
    })
}

fn live_task_snapshot(
    owner: &ActorTaskOwner<ActorTaskRuntime>,
) -> [Option<(ActorTaskVisit, ActorTaskRuntime)>; 3] {
    ActorTaskSlot::IN_RETAIL_TICK_ORDER.map(|slot| {
        owner.task_in_slot(slot).and_then(|task_id| {
            owner
                .task_state(task_id)
                .copied()
                .map(|state| (ActorTaskVisit { slot, task_id }, state))
        })
    })
}

impl OrdinaryType9AttractAttentionRootAuthority {
    fn authenticates(&self, entity: &Entity) -> bool {
        entity.id == self.entity_id
            && entity.active
            && entity.ordinary_type9_pending_initial_selection.is_none()
            && entity.main_base_type9_death_component_runtime.is_none()
            && entity.current_behavior_context == RetailRuntimeValue::Known(Some(self.context))
            && entity.ordinary_type9_selected_component_runtime == Some(self.selected)
            && entity.initial_behavior == self.initial_behavior
            && entity.actor_common_axis_descriptor
                == RetailRuntimeValue::Known(self.actor_common_axis)
            && entity.sub_a_propulsion_runtime == self.sub_a_propulsion_runtime
            && entity.actor_animation_runtime
                == RetailRuntimeValue::Known(Some(self.actor_animation_runtime))
            && live_task_snapshot(&entity.actor_tasks) == self.tasks
            && self.tasks.into_iter().flatten().all(|(visit, _)| {
                entity.actor_tasks.wrapper_flags(visit.task_id)
                    == Some(ActorTaskWrapperFlags {
                        alive: true,
                        in_callback: false,
                    })
            })
    }
}

fn initial_graph_authenticates(
    owner: &OrdinaryType9AttractAttentionProductionOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> bool {
    let Some(owners) = owner.initial_owners.as_ref() else {
        return false;
    };
    let graph_authority = match owner.root_publication.as_ref() {
        Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)) => {
            publication.authenticates_publication(entity)
        }
        Some(_) => false,
        None => match &owner.provenance {
            OrdinaryType9AttractAttentionProductionProvenance::Initial { construction, .. } => {
                construction.authenticates_live_initial_graph(entity, metadata, owners)
            }
            OrdinaryType9AttractAttentionProductionProvenance::Current(authority) => {
                authority.authenticates_identity(entity)
            }
        },
    };
    graph_authority && initial_owner_leases_authenticate(owners, entity, metadata)
}

fn initial_owner_leases_authenticate(
    owners: &OrdinaryType9AttractAttentionInitialOwners,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> bool {
    match owners {
        OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue } => {
            cue.validate_for_dispatch(entity, metadata).is_ok()
        }
        OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue } => {
            candidate.validate_for_dispatch(entity, metadata).is_ok()
                && cue.validate_for_dispatch(entity, metadata).is_ok()
        }
    }
}

fn post_task_owner_authority_authenticates(
    owner: &OrdinaryType9AttractAttentionProductionOwner,
    entity: &Entity,
    metadata: &EntityTypeRuntimeMetadata,
) -> bool {
    if let Some(route) = owner.target_route.as_ref() {
        let current_graph_matches = match &owner.provenance {
            OrdinaryType9AttractAttentionProductionProvenance::Initial { .. } => true,
            OrdinaryType9AttractAttentionProductionProvenance::Current(authority) => {
                // Subsequent native class-45 constructors can replace this
                // route. Its linear owner validates the current graph; retain
                // the transferred publication's unchanged birth identity.
                authority.authenticates_identity(entity)
            }
        };
        return current_graph_matches && route.validate(entity).is_ok();
    }
    if let Some(publication) = owner.root_publication.as_ref() {
        return publication.authenticates(entity);
    }
    let Some(owners) = owner.initial_owners.as_ref() else {
        return false;
    };
    let graph_authority = match &owner.provenance {
        OrdinaryType9AttractAttentionProductionProvenance::Initial { construction, .. } => {
            construction.authenticates_post_task_initial_graph(entity, metadata, owners)
        }
        OrdinaryType9AttractAttentionProductionProvenance::Current(authority) => {
            authority.authenticates_identity(entity)
        }
    };
    graph_authority && initial_owner_leases_authenticate(owners, entity, metadata)
}

fn post_task_basis_input_authenticates(
    frame: OrdinaryType9AttractAttentionPostTaskFrame,
    entity: &Entity,
) -> bool {
    entity.rotation_heading_pitch_roll_raw() == frame.post_task_angles_raw
        && entity.physical_body_basis_q31 == frame.pre_basis
}

fn attract_outer_tail_observation_authenticates(
    manager: &EntityManager,
    owner: &OrdinaryType9AttractAttentionProductionOwner,
) -> bool {
    owner.outer_tail.as_ref().is_some_and(|custody| {
        outer_tail_observation_authenticates(manager, owner.actor_lease, custody)
    })
}

fn map_outer_tail_block(
    reason: OrdinaryType9OuterTailBlock,
) -> OrdinaryType9AttractAttentionProductionBlock {
    match reason {
        OrdinaryType9OuterTailBlock::CurrentTerrainUnavailable => {
            OrdinaryType9AttractAttentionProductionBlock::CurrentTerrainUnavailable
        }
        OrdinaryType9OuterTailBlock::ActiveModelUnavailable => {
            OrdinaryType9AttractAttentionProductionBlock::ActiveModelUnavailable
        }
        OrdinaryType9OuterTailBlock::OuterOwnerStateUnavailable => {
            OrdinaryType9AttractAttentionProductionBlock::OuterOwnerStateUnavailable
        }
        OrdinaryType9OuterTailBlock::OuterOwner(block) => {
            OrdinaryType9AttractAttentionProductionBlock::OuterOwner(block)
        }
        OrdinaryType9OuterTailBlock::OuterLifecyclePending => {
            OrdinaryType9AttractAttentionProductionBlock::OuterLifecyclePending
        }
    }
}

fn blocked(
    owner: OrdinaryType9AttractAttentionProductionOwner,
    reason: OrdinaryType9AttractAttentionProductionBlock,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let entity_id = owner.entity_id();
    OrdinaryType9AttractAttentionOwnerTick {
        outcome: OrdinaryType9AttractAttentionProductionOutcome::Blocked { entity_id, reason },
        retained_owner: Some(owner),
    }
}

fn dropped(
    entity_id: u32,
    reason: OrdinaryType9AttractAttentionProductionDrop,
) -> OrdinaryType9AttractAttentionOwnerTick {
    OrdinaryType9AttractAttentionOwnerTick {
        outcome: OrdinaryType9AttractAttentionProductionOutcome::Dropped { entity_id, reason },
        retained_owner: None,
    }
}

pub(crate) fn commit_attract_attention_root_sounds(
    world_fx: &mut WorldFx,
    effects: Vec<OrdinaryType9AttractAttentionCommittedEffects>,
) {
    for committed in effects {
        // BA40 text was already submitted at its dispatch callback, before
        // cue construction. Only the positional sound awaits WorldFx custody.
        if let Some(sound) = committed.positional_sound {
            debug_assert_eq!(sound.gain_16_16, 0x1_0000);
            debug_assert_eq!(sound.rate_16_16, 0x1_0000);
            world_fx.queue_fixed_positional_sound_raw(sound.global_sound_id, sound.position_raw);
        }
    }
}

fn retain_outer_tail_drive(
    manager: &EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    drive: OrdinaryType9OuterTailDrive,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let entity_id = owner.entity_id();
    if !matches!(&drive, OrdinaryType9OuterTailDrive::StateMismatch) {
        let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
            return dropped(
                entity_id,
                OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
            );
        };
        if outer_tail_entity_published_class14(manager, owner.actor_lease) {
            return OrdinaryType9AttractAttentionOwnerTick {
                outcome: OrdinaryType9AttractAttentionProductionOutcome::SurfaceLifecycleClass14Published {
                    entity_id,
                    task_lease: outer_tail_class14_exploding_task_lease(
                        manager,
                        owner.actor_lease,
                    ),
                },
                retained_owner: None,
            };
        }
        if !refresh_retained_attract_attention_publication(&mut owner, entity) {
            return dropped(
                entity_id,
                OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
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
                    OrdinaryType9AttractAttentionProductionDrop::OuterTailStateMismatch,
                );
            }
            owner.outer_tail = Some(OrdinaryType9AttractAttentionOuterTailCustody::Complete {
                expected_state,
                expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(0),
            });
            let outcome = owner
                .pending_outer_outcome
                .take()
                .expect("F70 retains the exact branch success until outer completion");
            OrdinaryType9AttractAttentionOwnerTick {
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
            OrdinaryType9AttractAttentionProductionDrop::OuterTailStateMismatch,
        ),
    }
}

fn start_outer_tail_after_published_basis(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    production_frame: &OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let entity_id = owner.entity_id();
    let post_task_frame = match owner.state {
        OrdinaryType9AttractAttentionProductionState::TargetRouteParked { post_task_frame }
        | OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { post_task_frame } => {
            post_task_frame
        }
        _ => unreachable!("F70 publishes a post-task tail state"),
    };
    let transaction_id = take_outer_transaction_id(&mut owner.next_transaction_id);
    let (transaction, initial_animation_offset_at_0xb2) = match start_outer_tail_transaction(
        manager,
        owner.actor_lease,
        transaction_id,
        production_frame.resources,
        post_task_frame.callback_elapsed_micros,
    ) {
        Ok(started) => started,
        Err(OrdinaryType9OuterTailStartError::PostBasisPublicationMismatch) => {
            return dropped(
                entity_id,
                OrdinaryType9AttractAttentionProductionDrop::PostBasisPublicationMismatch,
            )
        }
        Err(error) => return blocked(owner, map_outer_tail_block(error.into())),
    };
    let terrain_collision =
        TerrainCollisionContext::from_current_level_cache(production_frame.resources);
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

fn publish_post_task_basis(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    metadata: &EntityTypeRuntimeMetadata,
    production_frame: &OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    frame: OrdinaryType9AttractAttentionPostTaskFrame,
    outcome: OrdinaryType9AttractAttentionProductionOutcome,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let entity_id = owner.entity_id();
    let Some(entity) = manager.ordinary_type9_selected_entity_mut(entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        );
    };
    if !post_task_owner_authority_authenticates(&owner, entity, metadata) {
        return dropped(
            entity_id,
            if owner.root_publication.is_some() || owner.target_route.is_some() {
                OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch
            } else {
                OrdinaryType9AttractAttentionProductionDrop::InitialOwnerMismatch
            },
        );
    }
    if entity.rotation_heading_pitch_roll_raw() != frame.post_task_angles_raw
        || entity.physical_body_basis_q31 != frame.pre_basis
    {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::PostTaskFrameMismatch,
        );
    }
    debug_assert_eq!(
        frame.latched_effective_flags,
        ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS
    );
    debug_assert_eq!(
        frame.latched_effective_flags
            & (TERRAIN_ATTITUDE_EFFECTIVE_FLAG | BODY_BASIS_REBUILD_SUPPRESSED_EFFECTIVE_FLAG),
        0
    );
    entity.physical_body_basis_q31 = RetailRuntimeValue::Known(frame.published_body_basis());
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
    if !refresh_retained_attract_attention_publication(&mut owner, entity) {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
        );
    }
    owner.state = if owner.target_route.is_some() {
        OrdinaryType9AttractAttentionProductionState::TargetRouteParked {
            post_task_frame: frame,
        }
    } else {
        OrdinaryType9AttractAttentionProductionState::PostBasisTailPending {
            post_task_frame: frame,
        }
    };
    owner.pending_outer_outcome = Some(outcome);
    start_outer_tail_after_published_basis(
        manager,
        owner,
        production_frame,
        world_fx,
        next_shared_random,
    )
}

pub(crate) fn tick_ordinary_type9_attract_attention_owner_with_random(
    manager: &mut EntityManager,
    owner: OrdinaryType9AttractAttentionProductionOwner,
    frame: OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9AttractAttentionOwnerTick {
    tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
        manager,
        owner,
        frame,
        world_fx,
        next_shared_random,
        |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
        |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
        |_| OrdinaryType9WanderAllocationDecision::Prepared,
    )
}

enum OrdinaryType9AttractAttentionOuterTailEntry {
    Continue(OrdinaryType9AttractAttentionProductionOwner),
    Return(OrdinaryType9AttractAttentionOwnerTick),
}

fn enter_published_outer_tail(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    metadata: Option<&EntityTypeRuntimeMetadata>,
    production_frame: &OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
) -> OrdinaryType9AttractAttentionOuterTailEntry {
    let entity_id = owner.entity_id();
    let Some(metadata) = metadata else {
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RuntimeMetadataUnavailable,
        ));
    };
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        ));
    };
    if !post_task_owner_authority_authenticates(&owner, entity, metadata) {
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(dropped(
            entity_id,
            if owner.target_route.is_some() || owner.root_publication.is_some() {
                OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch
            } else {
                OrdinaryType9AttractAttentionProductionDrop::PostBasisPublicationMismatch
            },
        ));
    }

    if owner.outer_tail.is_none() {
        let post_task_frame = match owner.state {
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { post_task_frame }
            | OrdinaryType9AttractAttentionProductionState::PostBasisTailPending {
                post_task_frame,
            } => post_task_frame,
            _ => unreachable!("only a published F70 state enters the outer tail"),
        };
        if entity.rotation_heading_pitch_roll_raw() != post_task_frame.post_task_angles_raw
            || entity.physical_body_basis_q31
                != RetailRuntimeValue::Known(post_task_frame.published_body_basis())
            || entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT)
                != RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
            || owner.pending_outer_outcome.is_none()
        {
            return OrdinaryType9AttractAttentionOuterTailEntry::Return(dropped(
                entity_id,
                OrdinaryType9AttractAttentionProductionDrop::PostBasisPublicationMismatch,
            ));
        }
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(
            start_outer_tail_after_published_basis(
                manager,
                owner,
                production_frame,
                world_fx,
                next_shared_random,
            ),
        );
    }

    if !attract_outer_tail_observation_authenticates(manager, &owner) {
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::OuterTailStateMismatch,
        ));
    }
    if matches!(
        owner.outer_tail,
        Some(OrdinaryType9AttractAttentionOuterTailCustody::BubblePending { .. })
    ) {
        let custody = owner.outer_tail.take().expect("matched above");
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(
            match resume_outer_tail_bubble(
                manager,
                owner.actor_lease,
                custody,
                production_frame.resources,
                world_fx,
                next_shared_random,
            ) {
                Ok(drive) => retain_outer_tail_drive(manager, owner, drive),
                Err(custody) => {
                    owner.outer_tail = Some(custody);
                    blocked(
                        owner,
                        OrdinaryType9AttractAttentionProductionBlock::CurrentTerrainUnavailable,
                    )
                }
            },
        );
    }
    let parked_reason = match owner.outer_tail.as_ref().expect("checked above") {
        OrdinaryType9AttractAttentionOuterTailCustody::Transaction { block, .. } => Some(
            OrdinaryType9AttractAttentionProductionBlock::OuterOwner(*block),
        ),
        OrdinaryType9AttractAttentionOuterTailCustody::LifecyclePending { .. } => {
            Some(OrdinaryType9AttractAttentionProductionBlock::OuterLifecyclePending)
        }
        OrdinaryType9AttractAttentionOuterTailCustody::BubblePending { .. } => unreachable!(),
        OrdinaryType9AttractAttentionOuterTailCustody::Complete { .. } => None,
    };
    if let Some(reason) = parked_reason {
        return OrdinaryType9AttractAttentionOuterTailEntry::Return(blocked(owner, reason));
    }
    debug_assert!(owner.pending_outer_outcome.is_none());
    if owner.target_route.is_some() {
        owner.outer_tail = None;
        owner.state = OrdinaryType9AttractAttentionProductionState::TargetRouteActive;
        return OrdinaryType9AttractAttentionOuterTailEntry::Continue(owner);
    }
    if let Some(publication) = owner.root_publication.as_ref() {
        if !matches!(
            publication,
            OrdinaryType9AttractAttentionRootPublication::AttractAttention(_)
        ) {
            return OrdinaryType9AttractAttentionOuterTailEntry::Return(blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::PostBasisTailPending,
            ));
        }
    }
    owner.outer_tail = None;
    owner.state = OrdinaryType9AttractAttentionProductionState::InitialGraph;
    OrdinaryType9AttractAttentionOuterTailEntry::Continue(owner)
}

#[allow(clippy::too_many_arguments)]
fn tick_active_target_route(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    metadata: Option<EntityTypeRuntimeMetadata>,
    frame: &OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    allocate_target: &mut impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    allocate_attract_root: &mut impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_root: &mut impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job: &mut impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away: &mut impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: frame.dispatch_resource_text,
        retail_tick: frame.retail_tick,
    };
    let entity_id = owner.entity_id();
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
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        );
    };
    if owner
        .target_route
        .as_ref()
        .is_none_or(|route| route.validate(entity).is_err())
        || matches!(&owner.provenance,
            OrdinaryType9AttractAttentionProductionProvenance::Current(authority)
                if !authority.authenticates_identity(entity))
    {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
        );
    }
    if let Err(reason) = preflight_normal_scheduler_owner(&entity.collision) {
        return blocked(owner, reason);
    }
    apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut entity.collision);
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
                OrdinaryType9AttractAttentionProductionBlock::SchedulerModeUnresolved,
            )
        }
    };
    let callback_mass_before_prefix = if scheduler_mode == 0 {
        match common_scheduler_callback_mass(
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => Some(value),
            RetailRuntimeValue::Unresolved => {
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::CallbackMassUnavailable,
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
            OrdinaryType9AttractAttentionProductionBlock::SchedulerStateUnresolved,
        );
    };
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let callback_elapsed_micros = match prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            return OrdinaryType9AttractAttentionOwnerTick {
                outcome: OrdinaryType9AttractAttentionProductionOutcome::SchedulerWaiting {
                    entity_id,
                },
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
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::CallbackMassUnavailable,
                );
            }
        },
    };
    entity.mass_raw = callback_mass;
    let Some(metadata) = metadata else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    let topology = match OrdinaryType9Topology::from_metadata(9, &metadata) {
        Ok(value) => value,
        Err(_) => {
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::RuntimeTopologyUnavailable,
            )
        }
    };
    let expected_actor_common_axis_descriptor = metadata
        .initializer
        .as_ref()
        .map(|initializer| initializer.common_axis_descriptor);
    let actor_common_axis_descriptor = entity.actor_common_axis_descriptor;

    // FUN_00403780 receives external controller/movement stores. Keep those
    // writes staged while the exact wrapper owns its task-private stage, then
    // publish both only after the callback has resolved.
    let pre_basis = entity.physical_body_basis_q31;
    let position_raw = entity.position_raw();
    let root_facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
    let mut staged_selected = entity.ordinary_type9_selected_component_runtime;
    let mut staged_actor_common_axis = entity.actor_common_axis_descriptor;
    let mut staged_sub_a = entity.sub_a_propulsion_runtime;
    let mut staged_animation = entity.actor_animation_runtime.clone();
    let RetailRuntimeValue::Known(Some(mut staged_context)) = entity.current_behavior_context
    else {
        owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::InitialOwnerMismatch,
        );
    };
    let mut staged_collision = entity.collision.clone();
    let mut staged_heading = entity.heading;
    let mut staged_velocity = entity.velocity;
    let route_owner = owner
        .target_route
        .take()
        .expect("active Target Route retains its exact linear owner");
    let mut movement_state = ();
    let mut controller_context = ();
    let mut replacement_target_route = None;
    let adapter = RefCell::new(SelectedAttractAttentionAdapter {
        notifications,
        entity_id,
        entity_type: root_facts.entity_type,
        capability_flags: root_facts.capability_flags,
        position_raw,
        root_facts,
        metadata: &metadata,
        topology,
        terrain: frame.resources.level_terrain(),
        snapshots,
        candidate_refs: frame.candidates_in_intrusive_order,
        initial_owners: &mut owner.initial_owners,
        target_route: &mut replacement_target_route,
        selected: &mut staged_selected,
        actor_common_axis: &mut staged_actor_common_axis,
        sub_a: &mut staged_sub_a,
        animation: &mut staged_animation,
        context: &mut staged_context,
        collision: &mut staged_collision,
        heading: &mut staged_heading,
        velocity: &mut staged_velocity,
        pre_basis,
        elapsed_micros: callback_elapsed_micros,
        global_elapsed_micros: frame.global_elapsed_micros,
        scheduler_mode,
        next_transaction_id: &mut owner.next_transaction_id,
        random: Rc::new(RefCell::new(&mut random)),
        // A root replacement can publish an odd Attract graph. Its new
        // Secondary runs in this same suffix and may acquire another route.
        allocate_target,
        allocate_attract_root,
        allocate_root,
        allocate_root_go_to_job,
        allocate_root_run_away,
        root_run_away_audio: None,
        candidate_completion: None,
        root_completion: None,
        committed_attract_attention_effects: Vec::new(),
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
    let mut adapter = adapter.into_inner();
    let mut deferred = None;
    let mut retained_route = None;
    let mut route_state = None;
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
            match adapter
                .apply_root_transition(&mut entity.actor_tasks, Some(ActorTaskSlot::Secondary))
            {
                Ok(result) => {
                    deferred = result;
                    if deferred.is_none()
                        && matches!(
                            adapter.root_completion,
                            Some(
                                OrdinaryType9AttractAttentionRootCompletion::AttractAttention { .. }
                                    | OrdinaryType9AttractAttentionRootCompletion::RunAway { .. }
                            )
                        )
                    {
                        match tick_actor_task_dispatcher_from_slot(
                            &mut entity.actor_tasks,
                            ActorTaskDispatcherFrame {
                                elapsed_micros: callback_elapsed_micros,
                                scheduler_mode: scheduler_mode as u32,
                            },
                            ActorTaskSlot::Secondary,
                            &mut adapter,
                        ) {
                            Ok(result) => {
                                deferred = result;
                                None
                            }
                            Err(error) => {
                                route_state = Some(
                                    OrdinaryType9AttractAttentionProductionState::CallbackFailurePending,
                                );
                                Some(OrdinaryType9AttractAttentionProductionBlock::Dispatcher(
                                    error,
                                ))
                            }
                        }
                    } else {
                        None
                    }
                }
                Err(error) => {
                    route_state =
                        Some(OrdinaryType9AttractAttentionProductionState::CallbackFailurePending);
                    Some(OrdinaryType9AttractAttentionProductionBlock::TargetRouteAdapter(error))
                }
            }
        }
        Ok(OrdinaryType9AttractAttentionTargetRouteTickOutcome::WrapperRetiredDuringCallback {
            ..
        }) => {
            route_state =
                Some(OrdinaryType9AttractAttentionProductionState::CallbackFailurePending);
            Some(OrdinaryType9AttractAttentionProductionBlock::TargetRouteWrapperRetired)
        }
        Err(failure) => {
            let error = failure.error.clone();
            if let Some(route) = failure.into_owner() {
                retained_route = Some(route);
                route_state = Some(OrdinaryType9AttractAttentionProductionState::TargetRouteActive);
            } else {
                route_state =
                    Some(OrdinaryType9AttractAttentionProductionState::CallbackFailurePending);
            }
            Some(OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
                error,
            ))
        }
    };
    let candidate_completion = adapter.candidate_completion.take();
    let mut root_completion = adapter.root_completion.take();
    let committed_attract_attention_effects =
        std::mem::take(&mut adapter.committed_attract_attention_effects);
    drop(adapter);
    owner.target_route = replacement_target_route.or(retained_route);
    if let Some(state) = route_state {
        owner.state = state;
    }

    entity.ordinary_type9_selected_component_runtime = staged_selected;
    entity.actor_common_axis_descriptor = staged_actor_common_axis;
    entity.sub_a_propulsion_runtime = staged_sub_a;
    entity.actor_animation_runtime = staged_animation;
    entity.current_behavior_context = RetailRuntimeValue::Known(Some(staged_context));
    entity.collision = staged_collision;
    entity.heading = staged_heading;
    entity.velocity = staged_velocity;
    let post_task_frame = OrdinaryType9AttractAttentionPostTaskFrame {
        callback_elapsed_micros,
        origin_retail_tick: frame.retail_tick,
        latched_effective_flags: ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        post_task_angles_raw: entity.rotation_heading_pitch_roll_raw(),
        pre_basis,
    };
    let post_basis_matches = entity.physical_body_basis_q31 == pre_basis;
    let publication_refreshed = if candidate_completion.is_none() {
        match root_completion.as_mut() {
            Some(completion) => completion.refresh_after_owned_dispatch(entity),
            None => refresh_retained_attract_attention_publication(&mut owner, entity),
        }
    } else {
        true
    };
    let (completion_outcome, publication_update) = if let Some(completion) = candidate_completion {
        match completion {
            OrdinaryType9AttractAttentionCandidateCompletion::TargetRoute {
                target_id,
                constructor,
            } => (
                Some(
                    OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished {
                        entity_id,
                        target_id,
                        constructor,
                    },
                ),
                OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(None),
            ),
            OrdinaryType9AttractAttentionCandidateCompletion::InitializerFallback {
                failure,
                publication,
            } => (
                Some(
                    OrdinaryType9AttractAttentionProductionOutcome::TargetInitializerFallbackPublished {
                        entity_id,
                        failure,
                    },
                ),
                OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(Some(publication)),
            ),
        }
    } else if let Some(completion) = root_completion {
        let (publication, outcome) = completion.into_parts(entity_id);
        (
            Some(outcome),
            OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(Some(publication)),
        )
    } else {
        (
            None,
            OrdinaryType9AttractAttentionRootPublicationUpdate::Preserve,
        )
    };
    if let OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(publication) =
        publication_update
    {
        owner.root_publication = publication;
    }
    commit_attract_attention_root_sounds(world_fx, committed_attract_attention_effects);
    if !publication_refreshed {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
        );
    }
    if let Some(reason) = route_error {
        return blocked(owner, reason);
    }
    if !post_basis_matches {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::PostTaskFrameMismatch,
        );
    }
    if let Some(deferred) = deferred {
        owner.pending_root_plan = deferred.plan;
        owner.pending_root_task_visits = Some(deferred.predecessor_task_visits);
        owner.pending_root_authority = Some(deferred.authority);
        owner.pending_root_continuation = Some(deferred.continuation);
        owner.state = if deferred.retryable {
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { post_task_frame }
        } else {
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        };
        return blocked(owner, deferred.reason);
    }
    let outcome = completion_outcome.unwrap_or(
        OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
            entity_id,
            callback_elapsed_micros,
        },
    );
    publish_post_task_basis(
        manager,
        owner,
        &metadata,
        frame,
        world_fx,
        next_shared_random,
        post_task_frame,
        outcome,
    )
}

fn tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
    manager: &mut EntityManager,
    owner: OrdinaryType9AttractAttentionProductionOwner,
    frame: OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    next_shared_random: impl FnMut(&mut WorldFx) -> u32,
    allocate_target: impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    allocate_attract_root: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_root: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
) -> OrdinaryType9AttractAttentionOwnerTick {
    tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
        manager,
        owner,
        frame,
        world_fx,
        next_shared_random,
        allocate_target,
        allocate_attract_root,
        allocate_root,
        |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
        |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
    )
}

#[allow(clippy::too_many_arguments)]
fn tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    frame: OrdinaryType9AttractAttentionProductionFrame<'_>,
    world_fx: &mut WorldFx,
    mut next_shared_random: impl FnMut(&mut WorldFx) -> u32,
    mut allocate_target: impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    mut allocate_attract_root: impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    mut allocate_root: impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    mut allocate_root_go_to_job: impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    mut allocate_root_run_away: impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: frame.dispatch_resource_text,
        retail_tick: frame.retail_tick,
    };
    let entity_id = owner.entity_id();
    let Some(current_lease) = manager.ordinary_type9_selected_actor_lease(entity_id) else {
        return OrdinaryType9AttractAttentionOwnerTick {
            outcome: OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
            },
            retained_owner: None,
        };
    };
    if current_lease != owner.actor_lease {
        return OrdinaryType9AttractAttentionOwnerTick {
            outcome: OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionDrop::ActorLeaseChanged,
            },
            retained_owner: None,
        };
    }
    let metadata = manager.type_runtime_metadata(9).cloned();
    match owner.state {
        OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        | OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. } => {
            match enter_published_outer_tail(
                manager,
                owner,
                metadata.as_ref(),
                &frame,
                world_fx,
                &mut next_shared_random,
            ) {
                OrdinaryType9AttractAttentionOuterTailEntry::Continue(retained) => {
                    owner = retained;
                }
                OrdinaryType9AttractAttentionOuterTailEntry::Return(tick) => return tick,
            }
        }
        OrdinaryType9AttractAttentionProductionState::RootTransitionPending { post_task_frame } => {
            return continue_root_transition(
                manager,
                owner,
                metadata,
                &frame,
                post_task_frame,
                world_fx,
                &mut next_shared_random,
                &mut allocate_target,
                &mut allocate_attract_root,
                &mut allocate_root,
                &mut allocate_root_go_to_job,
                &mut allocate_root_run_away,
            );
        }
        OrdinaryType9AttractAttentionProductionState::CallbackFailurePending => {
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::CallbackFailurePending,
            );
        }
        OrdinaryType9AttractAttentionProductionState::InitialGraph
        | OrdinaryType9AttractAttentionProductionState::TargetRouteActive => {}
    }
    if owner.state == OrdinaryType9AttractAttentionProductionState::TargetRouteActive {
        return tick_active_target_route(
            manager,
            owner,
            metadata,
            &frame,
            world_fx,
            &mut next_shared_random,
            &mut allocate_target,
            &mut allocate_attract_root,
            &mut allocate_root,
            &mut allocate_root_go_to_job,
            &mut allocate_root_run_away,
        );
    }

    let terrain = frame.resources.level_terrain();
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
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        );
    };
    let Some(metadata) = metadata else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    // The initializer-minted Candidate/Cue leases must authenticate before
    // the scheduler can draw, advance wait state, or publish mass.
    if !initial_graph_authenticates(&owner, entity, &metadata) {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::InitialOwnerMismatch,
        );
    }
    if let Err(reason) = preflight_normal_scheduler_owner(&entity.collision) {
        return blocked(owner, reason);
    }
    apply_fresh_level1_ordinary_type9_first_scheduler_state(&mut entity.collision);
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
                OrdinaryType9AttractAttentionProductionBlock::SchedulerModeUnresolved,
            )
        }
    };
    let callback_mass_before_prefix = if scheduler_mode == 0 {
        match common_scheduler_callback_mass(
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => Some(value),
            RetailRuntimeValue::Unresolved => {
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::CallbackMassUnavailable,
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
            OrdinaryType9AttractAttentionProductionBlock::SchedulerStateUnresolved,
        );
    };
    commit_common_scheduler_prefix(&mut entity.collision, prefix);
    let callback_elapsed_micros = match prefix.flow {
        CommonSchedulerPrefixFlow::WaitingAtCallbackGate => {
            return OrdinaryType9AttractAttentionOwnerTick {
                outcome: OrdinaryType9AttractAttentionProductionOutcome::SchedulerWaiting {
                    entity_id,
                },
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
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW,
            entity.collision.animation_offset_at_0xb2,
        ) {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::CallbackMassUnavailable,
                );
            }
        },
    };
    entity.mass_raw = callback_mass;
    let topology = match OrdinaryType9Topology::from_metadata(9, &metadata) {
        Ok(value) => value,
        Err(_) => {
            owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::RuntimeTopologyUnavailable,
            );
        }
    };
    let pre_basis = entity.physical_body_basis_q31;
    let position_raw = entity.position_raw();
    let root_facts = crate::ordinary_type9_root_wander_application::OrdinaryType9RootWanderEntityFacts::from_entity(entity);
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
        owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::InitialOwnerMismatch,
        );
    };
    let mut adapter = SelectedAttractAttentionAdapter {
        notifications,
        entity_id,
        entity_type: root_facts.entity_type,
        capability_flags: root_facts.capability_flags,
        position_raw,
        root_facts,
        metadata: &metadata,
        topology,
        terrain,
        snapshots,
        candidate_refs: frame.candidates_in_intrusive_order,
        initial_owners: &mut owner.initial_owners,
        target_route: &mut owner.target_route,
        selected: ordinary_type9_selected_component_runtime,
        actor_common_axis: actor_common_axis_descriptor,
        sub_a: sub_a_propulsion_runtime,
        animation: actor_animation_runtime,
        context,
        collision,
        heading,
        velocity,
        pre_basis,
        elapsed_micros: callback_elapsed_micros,
        global_elapsed_micros: frame.global_elapsed_micros,
        scheduler_mode,
        next_transaction_id: &mut owner.next_transaction_id,
        random: Rc::new(RefCell::new(&mut random)),
        allocate_target: &mut allocate_target,
        allocate_attract_root: &mut allocate_attract_root,
        allocate_root: &mut allocate_root,
        allocate_root_go_to_job: &mut allocate_root_go_to_job,
        allocate_root_run_away: &mut allocate_root_run_away,
        root_run_away_audio: None,
        candidate_completion: None,
        root_completion: None,
        committed_attract_attention_effects: Vec::new(),
    };
    let dispatcher = tick_actor_task_dispatcher(
        actor_tasks,
        ActorTaskDispatcherFrame {
            elapsed_micros: callback_elapsed_micros,
            scheduler_mode: scheduler_mode as u32,
        },
        &mut adapter,
    );
    let candidate_completion = adapter.candidate_completion.take();
    let mut root_completion = adapter.root_completion.take();
    let committed_attract_attention_effects =
        std::mem::take(&mut adapter.committed_attract_attention_effects);
    drop(adapter);
    commit_attract_attention_root_sounds(world_fx, committed_attract_attention_effects);
    let Some(post_entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        );
    };
    let post_task_frame = OrdinaryType9AttractAttentionPostTaskFrame {
        callback_elapsed_micros,
        origin_retail_tick: frame.retail_tick,
        latched_effective_flags: ORDINARY_TYPE9_EFFECTIVE_ENVIRONMENT_FLAGS,
        post_task_angles_raw: post_entity.rotation_heading_pitch_roll_raw(),
        pre_basis,
    };
    if post_entity.physical_body_basis_q31 != pre_basis {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::PostTaskFrameMismatch,
        );
    }
    if candidate_completion.is_none() {
        let publication_refreshed = match root_completion.as_mut() {
            Some(completion) => completion.refresh_after_owned_dispatch(post_entity),
            None => refresh_retained_attract_attention_publication(&mut owner, post_entity),
        };
        if !publication_refreshed {
            return dropped(
                entity_id,
                OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
            );
        }
    }
    let (completion_outcome, publication_update) = if let Some(completion) = candidate_completion {
        match completion {
            OrdinaryType9AttractAttentionCandidateCompletion::TargetRoute {
                target_id,
                constructor,
            } => (
                Some(
                    OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished {
                        entity_id,
                        target_id,
                        constructor,
                    },
                ),
                OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(None),
            ),
            OrdinaryType9AttractAttentionCandidateCompletion::InitializerFallback {
                failure,
                publication,
            } => (
                Some(
                    OrdinaryType9AttractAttentionProductionOutcome::TargetInitializerFallbackPublished {
                        entity_id,
                        failure,
                    },
                ),
                OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(Some(publication)),
            ),
        }
    } else if let Some(completion) = root_completion {
        let (publication, outcome) = completion.into_parts(entity_id);
        (
            Some(outcome),
            OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(Some(publication)),
        )
    } else {
        (
            None,
            OrdinaryType9AttractAttentionRootPublicationUpdate::Preserve,
        )
    };
    if let OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(publication) =
        publication_update
    {
        owner.root_publication = publication;
    }
    let output = match dispatcher {
        Ok(value) => value,
        Err(error) => {
            owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::Dispatcher(error),
            );
        }
    };
    if let Some(deferred) = output {
        owner.pending_root_plan = deferred.plan;
        owner.pending_root_task_visits = Some(deferred.predecessor_task_visits);
        owner.pending_root_authority = Some(deferred.authority);
        owner.pending_root_continuation = Some(deferred.continuation);
        owner.state = if deferred.retryable {
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { post_task_frame }
        } else {
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        };
        return blocked(owner, deferred.reason);
    }
    let outcome = completion_outcome.unwrap_or(
        OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
            entity_id,
            callback_elapsed_micros,
        },
    );
    publish_post_task_basis(
        manager,
        owner,
        &metadata,
        &frame,
        world_fx,
        &mut next_shared_random,
        post_task_frame,
        outcome,
    )
}

fn continue_root_transition(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    metadata: Option<EntityTypeRuntimeMetadata>,
    production_frame: &OrdinaryType9AttractAttentionProductionFrame<'_>,
    post_task_frame: OrdinaryType9AttractAttentionPostTaskFrame,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    allocate_target: &mut impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    allocate_attract_root: &mut impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_root: &mut impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job: &mut impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away: &mut impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let entity_id = owner.entity_id();
    let Some(metadata) = metadata else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RuntimeMetadataUnavailable,
        );
    };
    let Some(authority) = owner.pending_root_authority else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
        );
    };
    let Some(continuation) = owner.pending_root_continuation.clone() else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
        );
    };
    let Some(entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        );
    };
    if !authority.authenticates(entity) {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
        );
    }
    let Some(visits) = owner.pending_root_task_visits else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RootTaskVisitsUnavailable,
        );
    };
    if !post_task_basis_input_authenticates(post_task_frame, entity) {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::PostTaskFrameMismatch,
        );
    }
    if owner.pending_root_plan.is_none() {
        let Some(active_model_id) = entity.model_index else {
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::ActiveModelUnavailable,
            );
        };
        let mut candidates = continuation
            .snapshots
            .iter()
            .map(root_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        let live_owner = root_entity_ref(entity);
        if let Some(controlled) = candidates
            .iter_mut()
            .find(|candidate| candidate.id == entity_id)
        {
            *controlled = live_owner;
        }
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id,
                metadata: &metadata,
                owner: live_owner,
                current_context: entity.current_behavior_context,
                candidates_in_intrusive_order: &candidates,
            },
            || next_shared_random(world_fx),
        );
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::RootReselection(error),
                )
            }
        };
        owner.pending_root_plan = Some(plan);
    }
    let plan = owner.pending_root_plan.take().unwrap();
    let class_id = match plan.selection() {
        OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
        OrdinaryType9RootSelection::Weighted { selection, .. } => {
            u32::from(selection.program.class_id)
        }
    };
    if class_id == LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID
        || class_id == u32::from(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID)
    {
        return continue_retained_root_attract_attention(
            manager,
            owner,
            &metadata,
            production_frame,
            post_task_frame,
            world_fx,
            next_shared_random,
            allocate_target,
            allocate_attract_root,
            allocate_root,
            allocate_root_go_to_job,
            allocate_root_run_away,
            plan,
            visits,
            authority,
            continuation,
        );
    }
    if class_id == u32::from(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID) {
        let RetailRuntimeValue::Known(Some(expected_sub_a)) = authority.sub_a_propulsion_runtime
        else {
            owner.pending_root_plan = Some(plan);
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
            );
        };
        let candidate_evidence = go_to_job_candidate_evidence(&continuation.snapshots);
        let entity = manager
            .ordinary_type9_selected_entity_mut(entity_id)
            .expect("authenticated actor remains live");
        let application = apply_ordinary_type9_root_go_to_job_from_attract_attention(
            entity,
            &metadata,
            plan,
            visits,
            authority.actor_common_axis,
            expected_sub_a,
            &candidate_evidence,
            allocate_root_go_to_job,
            || next_shared_random(world_fx),
        );
        let application = match application {
            Ok(value) => value,
            Err(failure) => {
                let reason = failure.error.clone();
                owner.pending_root_plan = Some(failure.into_plan());
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::RootGoToJobPreflight(reason),
                );
            }
        };
        owner.initial_owners.take();
        owner.target_route.take();
        owner.pending_root_authority = None;
        owner.pending_root_task_visits = None;
        owner.pending_root_continuation = None;
        let (publication, outcome) =
            root_go_to_job_application_publication(entity_id, entity, application);
        owner.root_publication = Some(publication);
        return publish_post_task_basis(
            manager,
            owner,
            &metadata,
            production_frame,
            world_fx,
            next_shared_random,
            post_task_frame,
            outcome,
        );
    }
    if class_id
        != u32::from(crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID)
    {
        owner.pending_root_plan = Some(plan);
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::UnsupportedRootSelection { class_id },
        );
    }
    let entity = manager
        .ordinary_type9_selected_entity_mut(entity_id)
        .expect("authenticated actor remains live");
    let application =
        apply_ordinary_type9_root_wander(entity, &metadata, plan, visits, allocate_root, || {
            next_shared_random(world_fx)
        });
    let application = match application {
        Ok(value) => value,
        Err(failure) => {
            let reason = failure.error.clone();
            owner.pending_root_plan = Some(failure.into_plan());
            return blocked(
                owner,
                OrdinaryType9AttractAttentionProductionBlock::RootWanderPreflight(reason),
            );
        }
    };
    owner.initial_owners.take();
    owner.pending_root_authority = None;
    owner.pending_root_task_visits = None;
    owner.pending_root_continuation = None;
    let (publication, outcome) = root_application_publication(entity_id, entity, application);
    owner.root_publication = Some(publication);
    publish_post_task_basis(
        manager,
        owner,
        &metadata,
        production_frame,
        world_fx,
        next_shared_random,
        post_task_frame,
        outcome,
    )
}

#[allow(clippy::too_many_arguments)]
fn continue_retained_root_attract_attention(
    manager: &mut EntityManager,
    mut owner: OrdinaryType9AttractAttentionProductionOwner,
    metadata: &EntityTypeRuntimeMetadata,
    production_frame: &OrdinaryType9AttractAttentionProductionFrame<'_>,
    mut post_task_frame: OrdinaryType9AttractAttentionPostTaskFrame,
    world_fx: &mut WorldFx,
    next_shared_random: &mut impl FnMut(&mut WorldFx) -> u32,
    allocate_target: &mut impl FnMut(
        AttractAttentionTargetTaskPreparation,
    ) -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    allocate_attract_root: &mut impl FnMut(
        AttractAttentionInitialTaskPreparation,
    ) -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_root: &mut impl FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job: &mut impl FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away: &mut impl FnMut(
        RunAwayTaskPreparation,
    ) -> OrdinaryType9RunAwayAllocationDecision,
    plan: OrdinaryType9RootReselectionPlan,
    visits: [Option<ActorTaskVisit>; 3],
    authority: OrdinaryType9AttractAttentionRootAuthority,
    continuation: OrdinaryType9AttractAttentionDispatcherContinuation,
) -> OrdinaryType9AttractAttentionOwnerTick {
    let notifications = OrdinaryType9LiveNotificationContext {
        dispatch_resource_text: production_frame.dispatch_resource_text,
        retail_tick: production_frame.retail_tick,
    };
    let entity_id = owner.entity_id();
    let RetailRuntimeValue::Known(Some(expected_sub_a)) = authority.sub_a_propulsion_runtime else {
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
        );
    };
    let class_id = match plan.selection() {
        OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
        OrdinaryType9RootSelection::Weighted { selection, .. } => {
            u32::from(selection.program.class_id)
        }
    };
    let root_run_away_audio = if class_id == u32::from(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID) {
        match authored_run_away_audio(metadata) {
            Ok(audio) => Some(audio),
            Err(reason) => {
                owner.pending_root_plan = Some(plan);
                return blocked(owner, reason);
            }
        }
    } else {
        None
    };
    let entity = manager
        .ordinary_type9_selected_entity_mut(entity_id)
        .expect("authenticated actor remains live");
    let mut committed_effects = Vec::new();
    let mut root_completion = if root_run_away_audio.is_some() {
        let application = apply_ordinary_type9_root_run_away_from_attract_attention(
            entity,
            metadata,
            plan,
            visits,
            authority.actor_common_axis,
            expected_sub_a,
            &mut *allocate_root_run_away,
            || next_shared_random(world_fx),
        );
        let application = match application {
            Ok(value) => value,
            Err(failure) => {
                let reason = failure.error.clone();
                owner.pending_root_plan = Some(failure.into_plan());
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::RootRunAwayPreflight(reason),
                );
            }
        };
        owner.initial_owners.take();
        owner.target_route.take();
        match application {
            OrdinaryType9RootRunAwayApplicationOutcome::Published {
                constructors_by_phase,
                owner: publication,
            } => OrdinaryType9AttractAttentionRootCompletion::RunAway {
                constructors_by_phase,
                publication: OrdinaryType9AttractAttentionRootPublication::RunAway(publication),
            },
            OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                failure,
                constructors_by_phase,
            } => {
                let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                else {
                    unreachable!("class-10 fallback publishes context")
                };
                let selected = entity
                    .ordinary_type9_selected_component_runtime
                    .expect("class-10 fallback retains selected runtime");
                let RetailRuntimeValue::Known(actor_common_axis) =
                    entity.actor_common_axis_descriptor
                else {
                    unreachable!("class-10 fallback retains actor-axis custody")
                };
                let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                    unreachable!("class-10 fallback retains Sub-A")
                };
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                OrdinaryType9AttractAttentionRootCompletion::RunAwayInitializerFallback {
                    failure,
                    constructors_by_phase,
                    publication:
                        OrdinaryType9AttractAttentionRootPublication::RunAwayInitializerFallback {
                            entity_id,
                            context,
                            selected,
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
        let application = apply_ordinary_type9_root_attract_attention(
            entity,
            metadata,
            plan,
            visits,
            authority.actor_common_axis,
            expected_sub_a,
            authority.actor_animation_runtime,
            &mut *allocate_attract_root,
            || next_shared_random(world_fx),
            |request| notifications.dispatch(request),
            |_| {},
        );
        let application = match application {
            Ok(value) => value,
            Err(failure) => {
                let reason = failure.error.clone();
                owner.pending_root_plan = Some(failure.into_plan());
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::RootAttractAttentionPreflight(
                        reason,
                    ),
                );
            }
        };
        match application {
            OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                committed,
                owners,
                publication,
            } => {
                committed_effects.push(committed);
                owner.initial_owners = Some(owners);
                OrdinaryType9AttractAttentionRootCompletion::AttractAttention {
                    committed,
                    publication: OrdinaryType9AttractAttentionRootPublication::AttractAttention(
                        publication,
                    ),
                }
            }
            OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                failure,
                committed,
            } => {
                committed_effects.push(committed);
                owner.initial_owners = None;
                let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context
                else {
                    unreachable!("class-45 fallback publishes context")
                };
                let selected = entity
                    .ordinary_type9_selected_component_runtime
                    .expect("class-45 fallback retains selected runtime");
                let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                    unreachable!("class-45 fallback retains Sub-A")
                };
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                OrdinaryType9AttractAttentionRootCompletion::AttractAttentionInitializerFallback {
                    failure,
                    committed,
                    publication:
                        OrdinaryType9AttractAttentionRootPublication::InitializerFallback {
                            entity_id,
                            context,
                            selected,
                            initial_behavior: entity.initial_behavior,
                            sub_a,
                            state_policy_mask: policy.set_bits | policy.clear_bits,
                            state_policy_bits: policy.set_bits,
                        },
                }
            }
        }
    };
    owner.pending_root_plan = None;
    owner.pending_root_task_visits = None;
    owner.pending_root_authority = None;
    owner.pending_root_continuation = None;

    let mut candidate_completion = None;
    let mut deferred = None;
    let mut dispatcher_error = None;
    if matches!(
        root_completion,
        OrdinaryType9AttractAttentionRootCompletion::AttractAttention { .. }
            | OrdinaryType9AttractAttentionRootCompletion::RunAway { .. }
    ) {
        if let Some(next_slot) = continuation.next_slot {
            let topology = match OrdinaryType9Topology::from_metadata(9, metadata) {
                Ok(value) => value,
                Err(_) => {
                    owner.state =
                        OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
                    commit_attract_attention_root_sounds(world_fx, committed_effects);
                    return blocked(
                        owner,
                        OrdinaryType9AttractAttentionProductionBlock::RuntimeTopologyUnavailable,
                    );
                }
            };
            let terrain = production_frame.resources.level_terrain();
            let entity = manager
                .ordinary_type9_selected_entity_mut(entity_id)
                .expect("root-applied actor remains live");
            let position_raw = entity.position_raw();
            let root_facts = OrdinaryType9RootWanderEntityFacts::from_entity(entity);
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
                owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
                commit_attract_attention_root_sounds(world_fx, committed_effects);
                return blocked(
                    owner,
                    OrdinaryType9AttractAttentionProductionBlock::InitialOwnerMismatch,
                );
            };
            let mut random = || next_shared_random(world_fx);
            let mut adapter = SelectedAttractAttentionAdapter {
                notifications,
                entity_id,
                entity_type: root_facts.entity_type,
                capability_flags: root_facts.capability_flags,
                position_raw,
                root_facts,
                metadata,
                topology,
                terrain,
                snapshots: continuation.snapshots.clone(),
                candidate_refs: &continuation.candidate_refs,
                initial_owners: &mut owner.initial_owners,
                target_route: &mut owner.target_route,
                selected: ordinary_type9_selected_component_runtime,
                actor_common_axis: actor_common_axis_descriptor,
                sub_a: sub_a_propulsion_runtime,
                animation: actor_animation_runtime,
                context,
                collision,
                heading,
                velocity,
                pre_basis: continuation.pre_basis,
                elapsed_micros: continuation.callback_elapsed_micros,
                global_elapsed_micros: continuation.global_elapsed_micros,
                scheduler_mode: continuation.scheduler_mode,
                next_transaction_id: &mut owner.next_transaction_id,
                random: Rc::new(RefCell::new(&mut random)),
                allocate_target,
                allocate_attract_root,
                allocate_root,
                allocate_root_go_to_job,
                allocate_root_run_away,
                root_run_away_audio,
                candidate_completion: None,
                root_completion: Some(root_completion),
                committed_attract_attention_effects: committed_effects,
            };
            let dispatcher = tick_actor_task_dispatcher_from_slot(
                actor_tasks,
                ActorTaskDispatcherFrame {
                    elapsed_micros: continuation.callback_elapsed_micros,
                    scheduler_mode: continuation.scheduler_mode as u32,
                },
                next_slot,
                &mut adapter,
            );
            candidate_completion = adapter.candidate_completion.take();
            root_completion = adapter
                .root_completion
                .take()
                .expect("root suffix retains a terminal root publication");
            committed_effects = std::mem::take(&mut adapter.committed_attract_attention_effects);
            drop(adapter);
            match dispatcher {
                Ok(result) => deferred = result,
                Err(error) => dispatcher_error = Some(error),
            }
        }
    }

    let Some(post_entity) = manager.iter_all().find(|entity| entity.id == entity_id) else {
        commit_attract_attention_root_sounds(world_fx, committed_effects);
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::EntityUnavailable,
        );
    };
    post_task_frame.post_task_angles_raw = post_entity.rotation_heading_pitch_roll_raw();
    if post_entity.physical_body_basis_q31 != continuation.pre_basis {
        commit_attract_attention_root_sounds(world_fx, committed_effects);
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::PostTaskFrameMismatch,
        );
    }
    let publication_refreshed = if candidate_completion.is_none() {
        root_completion.refresh_after_owned_dispatch(post_entity)
    } else {
        true
    };
    let (outcome, publication_update) = if let Some(completion) = candidate_completion {
        match completion {
            OrdinaryType9AttractAttentionCandidateCompletion::TargetRoute {
                target_id,
                constructor,
            } => (
                OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished {
                    entity_id,
                    target_id,
                    constructor,
                },
                OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(None),
            ),
            OrdinaryType9AttractAttentionCandidateCompletion::InitializerFallback {
                failure,
                publication,
            } => (
                OrdinaryType9AttractAttentionProductionOutcome::TargetInitializerFallbackPublished {
                    entity_id,
                    failure,
                },
                OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(Some(publication)),
            ),
        }
    } else {
        let (publication, outcome) = root_completion.into_parts(entity_id);
        (
            outcome,
            OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(Some(publication)),
        )
    };
    if let OrdinaryType9AttractAttentionRootPublicationUpdate::Replace(publication) =
        publication_update
    {
        owner.root_publication = publication;
    }
    commit_attract_attention_root_sounds(world_fx, committed_effects);
    if !publication_refreshed {
        return dropped(
            entity_id,
            OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
        );
    }
    if let Some(error) = dispatcher_error {
        owner.state = OrdinaryType9AttractAttentionProductionState::CallbackFailurePending;
        return blocked(
            owner,
            OrdinaryType9AttractAttentionProductionBlock::Dispatcher(error),
        );
    }
    if let Some(deferred) = deferred {
        owner.pending_root_plan = deferred.plan;
        owner.pending_root_task_visits = Some(deferred.predecessor_task_visits);
        owner.pending_root_authority = Some(deferred.authority);
        owner.pending_root_continuation = Some(deferred.continuation);
        owner.state = if deferred.retryable {
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { post_task_frame }
        } else {
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        };
        return blocked(owner, deferred.reason);
    }
    publish_post_task_basis(
        manager,
        owner,
        metadata,
        production_frame,
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
    OrdinaryType9AttractAttentionRootPublication,
    OrdinaryType9AttractAttentionProductionOutcome,
) {
    match application {
        OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } => (
            OrdinaryType9AttractAttentionRootPublication::Wander(owner),
            OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished {
                entity_id,
                constructor,
            },
        ),
        OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                unreachable!("fallback publishes context")
            };
            let selected = entity
                .ordinary_type9_selected_component_runtime
                .expect("fallback retains selected runtime");
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                unreachable!("fallback retains Sub-A")
            };
            let policy = crate::entity_behavior::translate_state_policy(
                crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
            );
            (
                OrdinaryType9AttractAttentionRootPublication::InitializerFallback {
                    entity_id,
                    context,
                    selected,
                    initial_behavior: entity.initial_behavior,
                    sub_a,
                    state_policy_mask: policy.set_bits | policy.clear_bits,
                    state_policy_bits: policy.set_bits,
                },
                OrdinaryType9AttractAttentionProductionOutcome::RootInitializerFallbackPublished {
                    entity_id,
                    failure,
                },
            )
        }
    }
}

fn root_go_to_job_application_publication(
    entity_id: u32,
    entity: &Entity,
    application: OrdinaryType9RootGoToJobApplicationOutcome,
) -> (
    OrdinaryType9AttractAttentionRootPublication,
    OrdinaryType9AttractAttentionProductionOutcome,
) {
    match application {
        OrdinaryType9RootGoToJobApplicationOutcome::Published {
            target_id,
            constructor,
            owner,
        } => (
            OrdinaryType9AttractAttentionRootPublication::GoToJob(owner),
            OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobPublished {
                entity_id,
                target_id,
                constructor,
            },
        ),
        OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
            target_id,
            failure,
        } => {
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                unreachable!("Go-To-Job fallback publishes context")
            };
            let selected = entity
                .ordinary_type9_selected_component_runtime
                .expect("Go-To-Job fallback retains selected runtime");
            let RetailRuntimeValue::Known(actor_common_axis) = entity.actor_common_axis_descriptor
            else {
                unreachable!("Go-To-Job fallback retains actor-axis custody")
            };
            let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
                unreachable!("Go-To-Job fallback retains Sub-A")
            };
            let policy = crate::entity_behavior::translate_state_policy(
                crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
            );
            (
                OrdinaryType9AttractAttentionRootPublication::GoToJobInitializerFallback {
                    entity_id,
                    context,
                    selected,
                    initial_behavior: entity.initial_behavior,
                    actor_common_axis,
                    sub_a,
                    state_policy_mask: policy.set_bits | policy.clear_bits,
                    state_policy_bits: policy.set_bits,
                },
                OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobInitializerFallbackPublished {
                    entity_id,
                    target_id,
                    failure,
                },
            )
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct OrdinaryType9AttractAttentionDeferredRootTransition {
    plan: Option<OrdinaryType9RootReselectionPlan>,
    predecessor_task_visits: [Option<ActorTaskVisit>; 3],
    authority: OrdinaryType9AttractAttentionRootAuthority,
    continuation: OrdinaryType9AttractAttentionDispatcherContinuation,
    reason: OrdinaryType9AttractAttentionProductionBlock,
    retryable: bool,
}

enum OrdinaryType9AttractAttentionRootCompletion {
    Wander {
        constructor: OrdinaryType9WanderConstructorEvidence,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    InitializerFallback {
        failure: OrdinaryType9WanderInitializerFailure,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    GoToJob {
        target_id: u32,
        constructor: OrdinaryType9GoToJobConstructorEvidence,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    GoToJobInitializerFallback {
        target_id: u32,
        failure: OrdinaryType9GoToJobInitializerFailure,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    RunAway {
        constructors_by_phase: [OrdinaryType9RunAwayConstructorEvidence; 2],
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    RunAwayInitializerFallback {
        failure: OrdinaryType9RunAwayInitializerFailure,
        constructors_by_phase: [Option<OrdinaryType9RunAwayConstructorEvidence>; 2],
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    AttractAttention {
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
    AttractAttentionInitializerFallback {
        failure: OrdinaryType9AttractAttentionInitializerFailure,
        committed: OrdinaryType9AttractAttentionCommittedEffects,
        publication: OrdinaryType9AttractAttentionRootPublication,
    },
}

enum OrdinaryType9AttractAttentionRootPublicationUpdate {
    Preserve,
    Replace(Option<OrdinaryType9AttractAttentionRootPublication>),
}

impl OrdinaryType9AttractAttentionRootCompletion {
    fn refresh_after_owned_dispatch(&mut self, entity: &Entity) -> bool {
        let publication = match self {
            Self::AttractAttention { publication, .. } => publication,
            Self::Wander { .. }
            | Self::InitializerFallback { .. }
            | Self::GoToJob { .. }
            | Self::GoToJobInitializerFallback { .. }
            | Self::RunAway { .. }
            | Self::RunAwayInitializerFallback { .. }
            | Self::AttractAttentionInitializerFallback { .. } => return true,
        };
        let OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication) =
            publication
        else {
            return false;
        };
        publication.refresh_after_owned_dispatch(entity)
    }

    fn into_parts(
        self,
        entity_id: u32,
    ) -> (
        OrdinaryType9AttractAttentionRootPublication,
        OrdinaryType9AttractAttentionProductionOutcome,
    ) {
        match self {
            Self::Wander {
                constructor,
                publication,
            } => (
                publication,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished {
                    entity_id,
                    constructor,
                },
            ),
            Self::InitializerFallback {
                failure,
                publication,
            } => (
                publication,
                OrdinaryType9AttractAttentionProductionOutcome::RootInitializerFallbackPublished {
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
                OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobPublished {
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
                OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobInitializerFallbackPublished {
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
                OrdinaryType9AttractAttentionProductionOutcome::RootRunAwayPublished {
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
                OrdinaryType9AttractAttentionProductionOutcome::RootRunAwayInitializerFallbackPublished {
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
                OrdinaryType9AttractAttentionProductionOutcome::RootAttractAttentionPublished {
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
                OrdinaryType9AttractAttentionProductionOutcome::RootAttractAttentionInitializerFallbackPublished {
                    entity_id,
                    failure,
                    committed,
                },
            ),
        }
    }
}

fn refresh_retained_attract_attention_publication(
    owner: &mut OrdinaryType9AttractAttentionProductionOwner,
    entity: &Entity,
) -> bool {
    match owner.root_publication.as_mut() {
        Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)) => {
            publication.refresh_after_owned_dispatch(entity)
        }
        _ => true,
    }
}

struct SelectedAttractAttentionAdapter<'a, Random> {
    notifications: OrdinaryType9LiveNotificationContext<'a>,
    entity_id: u32,
    entity_type: u32,
    capability_flags: u32,
    position_raw: [i16; 3],
    root_facts: OrdinaryType9RootWanderEntityFacts,
    metadata: &'a EntityTypeRuntimeMetadata,
    topology: OrdinaryType9Topology,
    terrain: Option<&'a v2k_formats::terrain::TerrainGrid>,
    snapshots: Vec<EntitySnapshot>,
    candidate_refs: &'a [GuardLocationEntityRef],
    initial_owners: &'a mut Option<OrdinaryType9AttractAttentionInitialOwners>,
    target_route: &'a mut Option<OrdinaryType9AttractAttentionTargetRouteOwner>,
    selected: &'a mut Option<OrdinaryType9SelectedComponentRuntime>,
    actor_common_axis: &'a mut RetailRuntimeValue<CommonAxisDescriptor>,
    sub_a: &'a mut RetailRuntimeValue<Option<SubAPropulsionRuntime>>,
    animation: &'a mut RetailRuntimeValue<Option<ActorAnimationController>>,
    context: &'a mut BehaviorContextRuntime,
    collision: &'a mut EntityCollisionRuntimeState,
    heading: &'a mut f32,
    velocity: &'a mut [f32; 3],
    pre_basis: RetailRuntimeValue<Type9BodyBasis>,
    elapsed_micros: u32,
    global_elapsed_micros: u32,
    scheduler_mode: i32,
    next_transaction_id: &'a mut u64,
    random: Rc<RefCell<&'a mut Random>>,
    allocate_target: &'a mut dyn FnMut(
        AttractAttentionTargetTaskPreparation,
    )
        -> OrdinaryType9AttractAttentionTargetAllocationDecision,
    allocate_attract_root: &'a mut dyn FnMut(
        AttractAttentionInitialTaskPreparation,
    )
        -> OrdinaryType9AttractAttentionAllocationDecision,
    allocate_root:
        &'a mut dyn FnMut(OrdinaryType9WanderTaskSpec) -> OrdinaryType9WanderAllocationDecision,
    allocate_root_go_to_job:
        &'a mut dyn FnMut(GoToJobTaskSpec) -> OrdinaryType9GoToJobAllocationDecision,
    allocate_root_run_away:
        &'a mut dyn FnMut(RunAwayTaskPreparation) -> OrdinaryType9RunAwayAllocationDecision,
    root_run_away_audio: Option<RunAwayAuthoredAudio>,
    candidate_completion: Option<OrdinaryType9AttractAttentionCandidateCompletion>,
    root_completion: Option<OrdinaryType9AttractAttentionRootCompletion>,
    committed_attract_attention_effects: Vec<OrdinaryType9AttractAttentionCommittedEffects>,
}

impl<Random: FnMut() -> u32> SelectedAttractAttentionAdapter<'_, Random> {
    fn target_route_target_state(
        &self,
        target_id: u32,
    ) -> Result<SharedTargetRouteTargetRuntimeState, OrdinaryType9AttractAttentionAdapterError>
    {
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
                Err(OrdinaryType9AttractAttentionAdapterError::TargetStateUnresolved { target_id })
            };
        }
        match state.masked(DYING_STATE_BIT) {
            RetailRuntimeValue::Known(0) => Ok(SharedTargetRouteTargetRuntimeState::Live),
            RetailRuntimeValue::Known(_) => Ok(SharedTargetRouteTargetRuntimeState::Dying),
            RetailRuntimeValue::Unresolved => {
                Err(OrdinaryType9AttractAttentionAdapterError::TargetStateUnresolved { target_id })
            }
        }
    }

    fn target_route_predicate(
        &self,
        target_id: u32,
        expected: Option<CommonAxisDescriptor>,
        actual: RetailRuntimeValue<CommonAxisDescriptor>,
    ) -> Result<SharedTargetRoutePredicate, OrdinaryType9AttractAttentionAdapterError> {
        let expected = expected
            .ok_or(OrdinaryType9AttractAttentionAdapterError::RuntimeMetadataUnavailable)?;
        let controller = match actual {
            RetailRuntimeValue::Unresolved => {
                return Err(
                    OrdinaryType9AttractAttentionAdapterError::ActorCommonAxisDescriptorUnavailable,
                )
            }
            RetailRuntimeValue::Known(actual)
                if actual.strict_axis_limit_raw != expected.strict_axis_limit_raw =>
            {
                return Err(
                    OrdinaryType9AttractAttentionAdapterError::ActorCommonAxisDescriptorMismatch {
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
            .ok_or(
                OrdinaryType9AttractAttentionAdapterError::TargetPositionUnavailable { target_id },
            )?;
        let range = WrappedAxisRange::from_raw(controller.strict_axis_limit_raw);
        Ok(
            if within_wrapped_axis_range(range, self.position_raw, target_position_raw) {
                SharedTargetRoutePredicate::NonZero
            } else {
                SharedTargetRoutePredicate::Zero
            },
        )
    }

    fn tracked_target(
        &self,
        tracked_entity_handle: u32,
    ) -> RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>> {
        if tracked_entity_handle == 0 {
            return RetailRuntimeValue::Unresolved;
        }
        let Some(target) = self
            .snapshots
            .iter()
            .find(|target| target.id == tracked_entity_handle)
        else {
            return RetailRuntimeValue::Known(None);
        };
        if !target.active {
            return RetailRuntimeValue::Unresolved;
        }
        RetailRuntimeValue::Known(Some(CommonMoverTrackedTargetSnapshot {
            state_flags: target.collision.state_flags_at_0x08,
            position_raw: target.position_raw,
            velocity_raw: target.velocity_raw,
        }))
    }

    fn run_common_mover(
        &mut self,
        visit: ActorTaskVisit,
        private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, OrdinaryType9AttractAttentionAdapterError> {
        let tracked_target = self.tracked_target(private_state.tracked_entity_handle);
        let mut heading_raw = radians_to_binary_angle(*self.heading);
        let mut velocity_raw = world_position_raw(*self.velocity);
        let terrain = self
            .terrain
            .ok_or(OrdinaryType9AttractAttentionAdapterError::CurrentTerrainUnavailable)?;
        let selected = self.selected.as_mut().ok_or(
            OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
        )?;
        let sub_a = match self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a))
                if sub_a.target_speed_raw() != RetailRuntimeValue::Unresolved =>
            {
                sub_a
            }
            RetailRuntimeValue::Known(Some(_)) => {
                return Err(OrdinaryType9AttractAttentionAdapterError::SubATargetSpeedUnresolved)
            }
            _ => return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable),
        };
        let mut animation = bind_animation(self.animation, self.scheduler_mode)?;
        let pre_mover_basis = match self.pre_basis {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                return Err(OrdinaryType9AttractAttentionAdapterError::PhysicalBodyBasisUnavailable)
            }
        };
        let transaction_id = NonZeroU64::new(*self.next_transaction_id)
            .expect("production transaction ids never use zero");
        *self.next_transaction_id = self.next_transaction_id.wrapping_add(1).max(1);
        let random = Rc::clone(&self.random);
        let animation_policy =
            if selected.kind() == OrdinaryType9SelectedRuntimeKind::AttractAttentionPublished {
                ActorAbdiAnimationPolicy::AttractAttentionForcedStop
            } else {
                ActorAbdiAnimationPolicy::Neutral
            };
        let result = run_selected_ordinary_type9_common_mover_with_animation_policy(
            OrdinaryType9SelectedCommonMoverRequest {
                transaction_id: OrdinaryType9TransactionId::from(transaction_id),
                lease: OrdinaryType9FrameLease {
                    controlled_entity_id: self.entity_id,
                    task_visit: visit,
                },
                topology: self.topology,
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
                position_raw: self.position_raw,
                pre_mover_basis,
                elapsed_micros: self.elapsed_micros,
                global_elapsed_micros: self.global_elapsed_micros,
                scheduler_mode: self.scheduler_mode,
            },
            animation_policy,
            || (&mut **random.borrow_mut())(),
        )
        .map_err(|error| match error {
            OrdinaryType9SelectedCommonMoverError::FrameBlocked(reason) => {
                OrdinaryType9AttractAttentionAdapterError::CommonMoverFrameBlocked(reason)
            }
            OrdinaryType9SelectedCommonMoverError::CommitBlocked(reason) => {
                OrdinaryType9AttractAttentionAdapterError::CommonMoverCommitBlocked(reason)
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
        let RetailRuntimeValue::Known(actor_common_axis) = *self.actor_common_axis else {
            unreachable!("class-10 root publication retains actor-axis custody")
        };
        let RetailRuntimeValue::Known(Some(sub_a)) = self.sub_a else {
            unreachable!("class-10 root publication retains Sub-A custody")
        };
        if let Some(OrdinaryType9AttractAttentionRootCompletion::RunAway {
            publication: OrdinaryType9AttractAttentionRootPublication::RunAway(publication),
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
                    self.metadata,
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
        if let Some(OrdinaryType9AttractAttentionRootCompletion::RunAway {
            publication: OrdinaryType9AttractAttentionRootPublication::RunAway(publication),
            ..
        }) = self.root_completion.as_mut()
        {
            publication.finish_fleeing_handoff_parts(owner, *selected, *sub_a, *self.context);
        }
        0
    }

    fn root_continuation(
        &self,
        next_slot: Option<ActorTaskSlot>,
    ) -> OrdinaryType9AttractAttentionDispatcherContinuation {
        OrdinaryType9AttractAttentionDispatcherContinuation {
            next_slot,
            root_facts: self.root_facts,
            snapshots: self.snapshots.clone(),
            candidate_refs: self.candidate_refs.to_vec(),
            callback_elapsed_micros: self.elapsed_micros,
            global_elapsed_micros: self.global_elapsed_micros,
            scheduler_mode: self.scheduler_mode,
            pre_basis: self.pre_basis,
        }
    }

    fn apply_root_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        next_slot: Option<ActorTaskSlot>,
    ) -> Result<
        Option<OrdinaryType9AttractAttentionDeferredRootTransition>,
        OrdinaryType9AttractAttentionAdapterError,
    > {
        let selected = self.selected.ok_or(
            OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
        )?;
        let actor_common_axis =
            match *self.actor_common_axis {
                RetailRuntimeValue::Known(axis) => axis,
                RetailRuntimeValue::Unresolved => return Err(
                    OrdinaryType9AttractAttentionAdapterError::ActorCommonAxisDescriptorUnavailable,
                ),
            };
        let sub_a_runtime = match *self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
            _ => return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable),
        };
        let actor_animation_runtime = match *self.animation {
            RetailRuntimeValue::Known(Some(animation)) => animation,
            _ => return Err(OrdinaryType9AttractAttentionAdapterError::ActorAnimationUnavailable),
        };
        let authority = OrdinaryType9AttractAttentionRootAuthority {
            entity_id: self.entity_id,
            context: *self.context,
            selected,
            initial_behavior: self.root_facts.initial_behavior,
            actor_common_axis,
            sub_a_propulsion_runtime: RetailRuntimeValue::Known(Some(sub_a_runtime)),
            actor_animation_runtime,
            tasks: live_task_snapshot(owner),
        };
        let continuation = self.root_continuation(next_slot);
        let predecessor_task_visits = live_task_visits(owner);
        let Some(active_model_id) = self.root_facts.active_model else {
            return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                plan: None,
                predecessor_task_visits,
                authority,
                continuation,
                reason: OrdinaryType9AttractAttentionProductionBlock::ActiveModelUnavailable,
                retryable: true,
            }));
        };
        let owner_snapshot = OrdinaryType9RootEntityRef {
            id: self.entity_id,
            entity_type: self.entity_type,
            position_raw: self.position_raw,
            state_flags_raw: self.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(self.capability_flags),
            attached_entity_handle: self.collision.recent_relation_id_at_0x60,
        };
        let mut candidates = self
            .snapshots
            .iter()
            .map(root_entity_ref_from_snapshot)
            .collect::<Vec<_>>();
        if let Some(controlled) = candidates
            .iter_mut()
            .find(|candidate| candidate.id == self.entity_id)
        {
            *controlled = owner_snapshot;
        }
        let random = Rc::clone(&self.random);
        let mut selector_draws = 0_u32;
        let plan = plan_ordinary_type9_root_reselection(
            OrdinaryType9RootReselectionRequest {
                active_model_id,
                metadata: self.metadata,
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
                return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                    plan: None,
                    predecessor_task_visits,
                    authority,
                    continuation,
                    reason: OrdinaryType9AttractAttentionProductionBlock::RootReselection(error),
                    retryable: selector_draws == 0,
                }))
            }
        };
        let class_id = match plan.selection() {
            OrdinaryType9RootSelection::Alternate { program } => u32::from(program.class_id),
            OrdinaryType9RootSelection::Weighted { selection, .. } => {
                u32::from(selection.program.class_id)
            }
        };
        if class_id == LEVEL_ONE_TYPE9_ATTRACT_ATTENTION_CLASS_ID {
            let selected_runtime = self.selected.as_mut().ok_or(
                OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
            )?;
            let sub_a = match self.sub_a {
                RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
                _ => return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable),
            };
            let actor_animation = match self.animation {
                RetailRuntimeValue::Known(Some(animation)) => animation,
                _ => {
                    return Err(
                        OrdinaryType9AttractAttentionAdapterError::ActorAnimationUnavailable,
                    )
                }
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_attract_attention_parts(
                self.root_facts,
                self.metadata,
                plan,
                predecessor_task_visits,
                actor_common_axis,
                sub_a_runtime,
                actor_animation_runtime,
                OrdinaryType9RootAttractAttentionMutableParts {
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime,
                    actor_common_axis: self.actor_common_axis,
                    sub_a,
                    actor_animation,
                    current_context: self.context,
                },
                &mut *self.allocate_attract_root,
                || (&mut **random.borrow_mut())(),
                |request| self.notifications.dispatch(request),
                |_| {},
            );
            let application = match application {
                Ok(value) => value,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        authority,
                        continuation,
                        reason: OrdinaryType9AttractAttentionProductionBlock::RootAttractAttentionPreflight(reason),
                        retryable: true,
                    }));
                }
            };
            match application {
                OrdinaryType9RootAttractAttentionApplicationOutcome::Published {
                    committed,
                    owners,
                    publication,
                } => {
                    self.committed_attract_attention_effects.push(committed);
                    *self.initial_owners = Some(owners);
                    self.root_completion = Some(
                        OrdinaryType9AttractAttentionRootCompletion::AttractAttention {
                            committed,
                            publication:
                                OrdinaryType9AttractAttentionRootPublication::AttractAttention(
                                    publication,
                                ),
                        },
                    );
                }
                OrdinaryType9RootAttractAttentionApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    committed,
                } => {
                    self.committed_attract_attention_effects.push(committed);
                    self.initial_owners.take();
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    self.root_completion = Some(
                        OrdinaryType9AttractAttentionRootCompletion::AttractAttentionInitializerFallback {
                            failure,
                            committed,
                            publication:
                                OrdinaryType9AttractAttentionRootPublication::InitializerFallback {
                                    entity_id: self.entity_id,
                                    context: *self.context,
                                    selected: *selected_runtime,
                                    initial_behavior: self.root_facts.initial_behavior,
                                    sub_a: *sub_a,
                                    state_policy_mask: policy.set_bits | policy.clear_bits,
                                    state_policy_bits: policy.set_bits,
                                },
                        },
                    );
                }
            }
            return Ok(None);
        }
        if class_id == u32::from(LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID) {
            let audio = match authored_run_away_audio(self.metadata) {
                Ok(audio) => audio,
                Err(reason) => {
                    return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                        plan: Some(plan),
                        predecessor_task_visits,
                        authority,
                        continuation,
                        reason,
                        retryable: true,
                    }));
                }
            };
            let selected = self.selected.as_mut().ok_or(
                OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
            )?;
            let sub_a = match self.sub_a {
                RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
                _ => return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable),
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_run_away_from_attract_attention_parts(
                self.root_facts,
                self.metadata,
                plan,
                predecessor_task_visits,
                actor_common_axis,
                sub_a_runtime,
                OrdinaryType9RootRunAwayMutableParts {
                    actor_animation: self.animation,
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis,
                    sub_a,
                    current_context: self.context,
                },
                &mut *self.allocate_root_run_away,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(value) => value,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        authority,
                        continuation,
                        reason: OrdinaryType9AttractAttentionProductionBlock::RootRunAwayPreflight(
                            reason,
                        ),
                        retryable: true,
                    }));
                }
            };
            self.initial_owners.take();
            self.target_route.take();
            self.root_run_away_audio = Some(audio);
            self.root_completion = Some(match application {
                OrdinaryType9RootRunAwayApplicationOutcome::Published {
                    constructors_by_phase,
                    owner,
                } => OrdinaryType9AttractAttentionRootCompletion::RunAway {
                    constructors_by_phase,
                    publication: OrdinaryType9AttractAttentionRootPublication::RunAway(owner),
                },
                OrdinaryType9RootRunAwayApplicationOutcome::InitializerFallbackPublished {
                    failure,
                    constructors_by_phase,
                } => {
                    let RetailRuntimeValue::Known(published_actor_common_axis) =
                        *self.actor_common_axis
                    else {
                        unreachable!("class-10 fallback retains actor-axis custody")
                    };
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9AttractAttentionRootCompletion::RunAwayInitializerFallback {
                        failure,
                        constructors_by_phase,
                        publication:
                            OrdinaryType9AttractAttentionRootPublication::RunAwayInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                actor_common_axis: published_actor_common_axis,
                                sub_a: *sub_a,
                                state_policy_mask: policy.set_bits | policy.clear_bits,
                                state_policy_bits: policy.set_bits,
                            },
                    }
                }
            });
            return Ok(None);
        }
        if class_id == u32::from(LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID) {
            let candidate_evidence = go_to_job_candidate_evidence(&self.snapshots);
            let selected = self.selected.as_mut().ok_or(
                OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
            )?;
            let sub_a = match self.sub_a {
                RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
                _ => return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable),
            };
            let random = Rc::clone(&self.random);
            let application = apply_ordinary_type9_root_go_to_job_from_attract_attention_parts(
                self.root_facts,
                self.metadata,
                plan,
                predecessor_task_visits,
                actor_common_axis,
                sub_a_runtime,
                &candidate_evidence,
                OrdinaryType9RootGoToJobMutableParts {
                    actor_animation: self.animation,
                    collision: self.collision,
                    actor_tasks: owner,
                    selected_runtime: selected,
                    actor_common_axis: self.actor_common_axis,
                    sub_a,
                    current_context: self.context,
                },
                &mut *self.allocate_root_go_to_job,
                || (&mut **random.borrow_mut())(),
            );
            let application = match application {
                Ok(value) => value,
                Err(failure) => {
                    let reason = failure.error.clone();
                    return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                        plan: Some(failure.into_plan()),
                        predecessor_task_visits,
                        authority,
                        continuation,
                        reason: OrdinaryType9AttractAttentionProductionBlock::RootGoToJobPreflight(
                            reason,
                        ),
                        retryable: true,
                    }));
                }
            };
            self.initial_owners.take();
            self.target_route.take();
            self.root_completion = Some(match application {
                OrdinaryType9RootGoToJobApplicationOutcome::Published {
                    target_id,
                    constructor,
                    owner,
                } => OrdinaryType9AttractAttentionRootCompletion::GoToJob {
                    target_id,
                    constructor,
                    publication: OrdinaryType9AttractAttentionRootPublication::GoToJob(owner),
                },
                OrdinaryType9RootGoToJobApplicationOutcome::InitializerFallbackPublished {
                    target_id,
                    failure,
                } => {
                    let policy = crate::entity_behavior::translate_state_policy(
                        crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                    );
                    OrdinaryType9AttractAttentionRootCompletion::GoToJobInitializerFallback {
                        target_id,
                        failure,
                        publication:
                            OrdinaryType9AttractAttentionRootPublication::GoToJobInitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected: *selected,
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
        if class_id
            != u32::from(
                crate::ordinary_type9_initial_selection::LEVEL_ONE_TYPE9_WANDER_NEAR_CLASS_ID,
            )
        {
            return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                plan: Some(plan),
                predecessor_task_visits,
                authority,
                continuation,
                reason: OrdinaryType9AttractAttentionProductionBlock::UnsupportedRootSelection {
                    class_id,
                },
                retryable: true,
            }));
        }
        let selected = self.selected.as_mut().ok_or(
            OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
        )?;
        let sub_a = match self.sub_a {
            RetailRuntimeValue::Known(Some(sub_a)) => sub_a,
            _ => return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable),
        };
        let random = Rc::clone(&self.random);
        let application = apply_ordinary_type9_root_wander_parts(
            self.root_facts,
            self.metadata,
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
            &mut *self.allocate_root,
            || (&mut **random.borrow_mut())(),
        );
        let application = match application {
            Ok(value) => value,
            Err(failure) => {
                let reason = failure.error.clone();
                return Ok(Some(OrdinaryType9AttractAttentionDeferredRootTransition {
                    plan: Some(failure.into_plan()),
                    predecessor_task_visits,
                    authority,
                    continuation,
                    reason: OrdinaryType9AttractAttentionProductionBlock::RootWanderPreflight(
                        reason,
                    ),
                    retryable: true,
                }));
            }
        };
        self.initial_owners.take();
        self.root_completion = Some(match application {
            OrdinaryType9RootWanderApplicationOutcome::Published { constructor, owner } => {
                OrdinaryType9AttractAttentionRootCompletion::Wander {
                    constructor,
                    publication: OrdinaryType9AttractAttentionRootPublication::Wander(owner),
                }
            }
            OrdinaryType9RootWanderApplicationOutcome::InitializerFallbackPublished { failure } => {
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                OrdinaryType9AttractAttentionRootCompletion::InitializerFallback {
                    failure,
                    publication:
                        OrdinaryType9AttractAttentionRootPublication::InitializerFallback {
                            entity_id: self.entity_id,
                            context: *self.context,
                            selected: *selected,
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

impl<Random: FnMut() -> u32> ActorTaskDispatcherAdapter
    for SelectedAttractAttentionAdapter<'_, Random>
{
    type Output = OrdinaryType9AttractAttentionDeferredRootTransition;
    type Error = OrdinaryType9AttractAttentionAdapterError;

    fn ordinary_wander_anchor_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        unreachable!("initial Attract graph rejects ordinary Wander")
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
            self.initial_owners.as_mut()
        else {
            return Err(OrdinaryType9AttractAttentionAdapterError::CandidateLeaseChanged);
        };
        let prefix = candidate
            .commit_prevalidated_dispatch_prefix(visit, task_state, random)
            .map_err(OrdinaryType9AttractAttentionAdapterError::Candidate)?;
        if let crate::guard_location_owner::acquisition::GuardLocationAcquisitionCallbackPrefix::Acquire {
            filter_write: Some(filter),
            ..
        } = prefix
        {
            publish_candidate_dispatch_filter(self.actor_common_axis, filter);
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
            return Err(OrdinaryType9AttractAttentionAdapterError::SubARuntimeUnavailable);
        };
        if self.selected.is_none() {
            return Err(
                OrdinaryType9AttractAttentionAdapterError::SelectedComponentRuntimeUnavailable,
            );
        }
        let RetailRuntimeValue::Known(Some(actor_animation)) = self.animation else {
            return Err(OrdinaryType9AttractAttentionAdapterError::ActorAnimationUnavailable);
        };
        let Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue }) =
            self.initial_owners.take()
        else {
            return Err(OrdinaryType9AttractAttentionAdapterError::CandidateLeaseChanged);
        };
        *self.initial_owners = Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue });
        let selected = self
            .selected
            .as_mut()
            .expect("selected storage checked before linear owners moved");
        let random = Rc::clone(&self.random);
        let result = candidate.dispatch_callback_parts(
            OrdinaryType9AttractAttentionCandidateDispatchParts {
                actor_animation,
                actor_common_axis: self.actor_common_axis,
                entity_id: self.entity_id,
                entity_type: self.entity_type,
                position_raw: self.position_raw,
                capability_flags: self.capability_flags,
                collision: self.collision,
                actor_tasks: owner,
                selected,
                sub_a,
                context: self.context,
            },
            visit,
            prefix,
            self.candidate_refs,
            &mut *self.allocate_target,
            || (&mut **random.borrow_mut())(),
        );
        let dispatch = match result {
            Ok(value) => value,
            Err(failure) => {
                let error = failure.error.clone();
                let owner = failure.into_owner();
                let cue = match self.initial_owners.take() {
                    Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue })
                    | Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                        cue,
                        ..
                    }) => cue,
                    None => {
                        return Err(
                            OrdinaryType9AttractAttentionAdapterError::CandidateLeaseChanged,
                        )
                    }
                };
                *self.initial_owners = Some(
                    OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue {
                        candidate: owner,
                        cue,
                    },
                );
                return Err(OrdinaryType9AttractAttentionAdapterError::Candidate(error));
            }
        };
        match dispatch {
            OrdinaryType9AttractAttentionCandidateDispatch::Pending { owner, result } => {
                // The sibling Cue was never moved out of the graph: recover it
                // from the prevalidated initial owners before continuing.
                let Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { cue }) =
                    self.initial_owners.take()
                else {
                    return Err(OrdinaryType9AttractAttentionAdapterError::CueLeaseChanged);
                };
                *self.initial_owners = Some(
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
                let _retired_cue = self.initial_owners.take();
                *self.target_route = Some(owner);
                self.candidate_completion = Some(
                    OrdinaryType9AttractAttentionCandidateCompletion::TargetRoute {
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
                let _retired_cue = self.initial_owners.take();
                let policy = crate::entity_behavior::translate_state_policy(
                    crate::entity_behavior::INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY,
                );
                self.candidate_completion = Some(
                    OrdinaryType9AttractAttentionCandidateCompletion::InitializerFallback {
                        failure,
                        publication:
                            OrdinaryType9AttractAttentionRootPublication::InitializerFallback {
                                entity_id: self.entity_id,
                                context: *self.context,
                                selected: *selected,
                                initial_behavior: self.root_facts.initial_behavior,
                                sub_a: *sub_a,
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
        unreachable!("class-45 candidate callback returns only zero or accepted tag")
    }

    fn attract_attention_cue_transition_owner_id(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        visit: ActorTaskVisit,
        _prefix: crate::ordinary_type9_attract_attention_cue::OrdinaryType9AttractAttentionCueCallbackPrefix,
    ) -> Result<u32, Self::Error> {
        let Some(owners) = self.initial_owners.as_ref() else {
            return Err(OrdinaryType9AttractAttentionAdapterError::CueLeaseChanged);
        };
        if owners.cue().visit() != visit {
            return Err(OrdinaryType9AttractAttentionAdapterError::CueLeaseChanged);
        }
        Ok(self.entity_id)
    }

    fn attract_attention_cue_transition(
        &mut self,
        owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        request: OrdinaryType9AttractAttentionCueTransition,
    ) -> Result<AttractAttentionCueTransitionOutcome<Self::Output>, Self::Error> {
        if read_transition_suppressed(self.collision)? {
            return Ok(AttractAttentionCueTransitionOutcome::SuppressedByEntityState);
        }
        let deferred = self.apply_root_transition(owner, slot_after(request.expired_visit.slot))?;
        Ok(AttractAttentionCueTransitionOutcome::Completed(deferred))
    }

    fn ordinary_wander_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        unreachable!("initial Attract graph rejects ordinary Wander")
    }

    fn ordinary_wander_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionRequest,
    ) -> Result<
        crate::ordinary_type9_wander_owner::OrdinaryType9WanderTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("initial Attract graph rejects ordinary Wander")
    }

    fn go_to_job_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _stage: &mut crate::go_to_job_owner::GoToJobCallbackStage,
    ) -> Result<crate::go_to_job_owner::GoToJobCallbackResult, Self::Error> {
        unreachable!("initial Attract graph rejects Go To Job")
    }

    fn go_to_job_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::go_to_job_owner::GoToJobTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("initial Attract graph rejects Go To Job")
    }

    fn chase_target_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _stage: &mut crate::chase_target::ChaseTargetCallbackStage,
    ) -> Result<crate::chase_target::ChaseTargetCallbackResult, Self::Error> {
        unreachable!("initial Attract graph rejects Chase Target")
    }

    fn chase_target_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::chase_target::ChaseTargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("initial Attract graph rejects Chase Target")
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
        .map_err(OrdinaryType9AttractAttentionAdapterError::TargetAcquisition)
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
        unreachable!("initial Attract graph rejects Follow Beacons")
    }

    fn follow_beacon_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("initial Attract graph rejects Follow Beacons")
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
        unreachable!("initial Attract graph rejects Follow Beacons")
    }

    fn follow_beacons_following_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::follow_beacons::FollowBeaconsFollowingTransitionRequest,
    ) -> Result<
        crate::actor_task_dispatcher::FollowBeaconsFollowingTransitionOutcome<Self::Output>,
        Self::Error,
    > {
        unreachable!("initial Attract graph rejects Follow Beacons")
    }

    fn follow_beacons_following_style_result(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::FollowBeaconsFollowingStyleResultRequest,
    ) -> Result<FollowBeaconsFollowingStyleResultOutcome<Self::Output>, Self::Error> {
        unreachable!("initial Attract graph rejects Follow Beacons")
    }

    fn aim_and_fire_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _state: crate::aim_and_fire::AimAndFirePrivateState,
    ) -> Result<crate::aim_and_fire::AimAndFireCallbackResult, Self::Error> {
        unreachable!("initial Attract graph rejects Aim and Fire")
    }

    fn aim_and_fire_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::AimAndFireTransitionRequest,
    ) -> Result<crate::actor_task_dispatcher::AimAndFireTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("initial Attract graph rejects Aim and Fire")
    }

    fn aim_and_fire_propagated_result(&mut self, _result: std::num::NonZeroU32) -> Self::Output {
        unreachable!("initial Attract graph rejects Aim and Fire")
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
        unreachable!("initial Attract graph rejects Guard Location")
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
        unreachable!("initial Attract graph rejects Guard Location")
    }

    fn guard_location_acquisition_propagated_result(
        &mut self,
        _result: std::num::NonZeroU32,
    ) -> Self::Output {
        unreachable!("initial Attract graph rejects Guard Location")
    }

    fn run_away_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
        _stage: &mut crate::run_away::RunAwayCallbackStage,
    ) -> Result<crate::run_away::RunAwayCallbackResult, Self::Error> {
        unreachable!("initial Attract graph rejects Run Away")
    }

    fn run_away_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::run_away::RunAwayTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("initial Attract graph rejects Run Away")
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
        request: SharedRetargetTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        if read_transition_suppressed(self.collision)? {
            return Ok(None);
        }
        self.apply_root_transition(owner, slot_after(request.slot))
    }

    fn defecate_virus_terrain_request(
        &mut self,
        _visit: ActorTaskVisit,
        _elapsed_micros: u32,
    ) -> Result<crate::defecate_virus::DefecateVirusCallbackRequest, Self::Error> {
        unreachable!("initial Attract graph rejects Defecate Virus")
    }

    fn apply_defecate_virus_terrain_plan(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _plan: crate::defecate_virus::DefecateVirusCallbackPlan,
    ) -> Result<(), Self::Error> {
        unreachable!("initial Attract graph rejects Defecate Virus")
    }

    fn defecate_virus_terrain_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::defecate_virus::DefecateVirusTerrainTransitionRequest,
    ) -> Result<crate::defecate_virus::DefecateVirusTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("initial Attract graph rejects Defecate Virus")
    }

    fn defecate_virus_wander_actor_position_raw(&mut self, _visit: ActorTaskVisit) -> [i16; 3] {
        unreachable!("initial Attract graph rejects Defecate Virus")
    }

    fn defecate_virus_wander_common_mover(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _private_state: &mut WanderNearPrivateState,
    ) -> Result<WanderNearCommonMoverReturn, Self::Error> {
        unreachable!("initial Attract graph rejects Defecate Virus")
    }

    fn defecate_virus_wander_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::defecate_virus::DefecateVirusWanderTransitionRequest,
    ) -> Result<crate::defecate_virus::DefecateVirusTransitionOutcome<Self::Output>, Self::Error>
    {
        unreachable!("initial Attract graph rejects Defecate Virus")
    }

    fn common_dying_callback(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _visit: ActorTaskVisit,
        _frame: ActorTaskDispatcherFrame,
    ) -> Result<crate::common_dying::CommonDyingCallbackResult, Self::Error> {
        unreachable!("initial Attract graph rejects Common Dying")
    }

    fn common_dying_transition(
        &mut self,
        _owner: &mut ActorTaskOwner<ActorTaskRuntime>,
        _request: crate::actor_task_dispatcher::CommonDyingTransitionRequest,
    ) -> Result<Option<Self::Output>, Self::Error> {
        unreachable!("initial Attract graph rejects Common Dying")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{
        base_factory_progression::ProgressiveDeathState,
        entity::{
            exact_level_one_type9_attract_attention_manager, BaseFactoryRuntimeState, EntityKind,
        },
        entity_collision_state::{
            RetailStateWord, ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            BODY_BASIS_REBUILT_STATE_BIT, DEFERRED_DESTROY_PENDING_STATE_BIT, DYING_STATE_BIT,
        },
        entity_scheduler::SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
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

    const ATTRACT_ROOT_SELECTOR_WORD: u32 = 3_063;
    const GO_TO_JOB_ROOT_SELECTOR_WORD: u32 = 3_982;
    const RUN_AWAY_ROOT_SELECTOR_WORD: u32 = 0;
    const ROOT_BASE_ID: u32 = 0x05ac_0054;
    const LATE_ROOT_BASE_ID: u32 = 0x05ac_0055;
    const RUN_AWAY_TARGET_ID: u32 = 0x04ae_1001;

    fn append_attract_root_witnesses(manager: &mut EntityManager, position_raw: [i16; 3]) {
        for (id, capability_flags) in [
            (0x05ac_00b1, LEVEL_ONE_TYPE9_BADDIE_NEARBY_CAPABILITY_MASK),
            (0x05ac_00b2, LEVEL_ONE_TYPE9_PLAYER_NEARBY_CAPABILITY_MASK),
            (0x05ac_00b3, LEVEL_ONE_TYPE9_BASE_NEARBY_CAPABILITY_MASK),
        ] {
            let mut witness = Entity::unresolved_port_entity(id, EntityKind::Unknown(99), 99);
            witness.set_motion_raw(position_raw, [0; 3]);
            witness.capability_flags = capability_flags;
            witness.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
            manager.append_entity_for_test(witness);
        }
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

    fn resolve_run_away_audio(manager: &mut EntityManager) {
        let metadata = manager
            .type_runtime_metadata_mut_for_test(9)
            .expect("exact fixture retains Type-9 metadata");
        metadata.run_away_optional_sound_id =
            RetailRuntimeValue::Known(Some(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_ID));
        metadata.run_away_sound_period_raw =
            RetailRuntimeValue::Known(LEVEL_ONE_TYPE9_RUN_AWAY_SOUND_PERIOD_RAW);
    }

    fn terrain_resources() -> ResourceCache {
        terrain_resources_with_height(0)
    }

    fn terrain_resources_with_height(height: i8) -> ResourceCache {
        let terrain = TerrainGrid {
            header: [0; 5],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        };
        let mut resources = ResourceCache::new(Vec::new());
        resources.load_level(crate::level::LevelState {
            source_path: "attract-attention-production-test.ovl".into(),
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

    fn production_owner_fixture(
        odd_parity: bool,
    ) -> (
        EntityManager,
        OrdinaryType9AttractAttentionProductionOwner,
        u32,
    ) {
        let mut manager = exact_level_one_type9_attract_attention_manager(odd_parity);
        let entity_id = manager
            .fresh_level1_type9_initial_productions()
            .first()
            .map(FreshLevel1Type9InitialProductionOwner::entity_id)
            .expect("exact fixture publishes Attract Attention");
        // This fixture exercises the production tail, whose retained E370
        // input is an explicit precondition rather than a constructor guess.
        manager
            .entity_mut_for_test(entity_id)
            .expect("fixture actor remains live")
            .surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(0);
        manager
            .entity_mut_for_test(entity_id)
            .expect("fixture actor remains live")
            .collision
            .state_flags_at_0x08
            .overwrite(
                crate::entity_collision_state::SURFACE_STATE_MASK,
                crate::entity_collision_state::FULLY_ABOVE_SURFACE_STATE_BIT,
            );
        let actor_lease = manager
            .ordinary_type9_selected_actor_lease(entity_id)
            .expect("exact fixture has a live actor lease");
        let (original_index, _, initial_owner) = manager
            .take_fresh_level1_type9_scheduler_productions()
            .expect("exact fixture sidecar authenticates")
            .into_iter()
            .next()
            .expect("exact fixture extraction returns Attract Attention");
        let owner = OrdinaryType9AttractAttentionProductionOwner::adopt(
            original_index,
            initial_owner,
            actor_lease,
        )
        .expect("exact fixture adopts Attract Attention");
        (manager, owner, entity_id)
    }

    fn frame(
        resources: &ResourceCache,
        elapsed_micros: u32,
    ) -> OrdinaryType9AttractAttentionProductionFrame<'_> {
        OrdinaryType9AttractAttentionProductionFrame {
            dispatch_resource_text: &|_, _| {},
            resources,
            retail_tick: elapsed_micros,
            elapsed_micros,
            global_elapsed_micros: elapsed_micros,
            candidates_in_intrusive_order: &[],
        }
    }

    fn retained_root_plan_fixture() -> (
        EntityManager,
        OrdinaryType9AttractAttentionProductionOwner,
        u32,
        ResourceCache,
        WorldFx,
    ) {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::SharedRetarget(state)) =
            entity.actor_tasks.task_state_mut(primary)
        else {
            panic!("exact Attract Primary is SharedRetarget")
        };
        state.before_callback(900_000);
        let exact_model_slots = entity.model_slots;
        entity.model_slots = [None; 4];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = [0x1111_u32, 0x2222, 0xffff].into_iter();
        let first = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 501_000),
            &mut world_fx,
            |_| words.next().expect("retarget X/Z and root selector words"),
        );
        assert!(words.next().is_none());
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootWanderPreflight(
                    OrdinaryType9RootWanderPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("retryable root preflight retains production custody");
        assert!(retained.pending_root_plan.is_some());
        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
        (manager, retained, entity_id, resources, world_fx)
    }

    fn assert_initial_graph_tamper_drops_before_scheduler(
        mut manager: EntityManager,
        owner: OrdinaryType9AttractAttentionProductionOwner,
        entity_id: u32,
    ) {
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        let collision_before = entity.collision.clone();
        let tasks_before = live_task_snapshot(&entity.actor_tasks);
        let context_before = entity.current_behavior_context;
        let selected_before = entity.ordinary_type9_selected_component_runtime;
        let sub_a_before = entity.sub_a_propulsion_runtime;
        let animation_before = entity.actor_animation_runtime;
        let mass_before = entity.mass_raw;
        let angles_before = entity.rotation_heading_pitch_roll_raw();
        let basis_before = entity.physical_body_basis_q31;
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;
        let mut target_allocations = 0;
        let mut root_allocations = 0;

        let tick = tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
            &mut manager,
            owner,
            frame(&resources, 50_000),
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
            |_| {
                target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
            },
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| {
                root_allocations += 1;
                OrdinaryType9WanderAllocationDecision::Prepared
            },
        );

        assert_eq!(
            (random_calls, target_allocations, root_allocations),
            (0, 0, 0)
        );
        assert!(matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                reason: OrdinaryType9AttractAttentionProductionDrop::InitialOwnerMismatch,
                ..
            }
        ));
        assert!(tick.retained_owner.is_none());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(entity.collision, collision_before);
        assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_before);
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.sub_a_propulsion_runtime, sub_a_before);
        assert_eq!(entity.actor_animation_runtime, animation_before);
        assert_eq!(entity.mass_raw, mass_before);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), angles_before);
        assert_eq!(entity.physical_body_basis_q31, basis_before);
    }

    #[test]
    fn fresh_native_startup_wait_preserves_callback_and_rng_order() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        entity.collision.fresh_level1_type9_first_scheduler_pending = true;
        let tasks_before = live_task_snapshot(&entity.actor_tasks);
        let context_before = entity.current_behavior_context;
        let selected_before = entity.ordinary_type9_selected_component_runtime;
        let mass_before = entity.mass_raw;
        let angles_before = entity.rotation_heading_pitch_roll_raw();
        let basis_before = entity.physical_body_basis_q31;
        let resources = ResourceCache::new(Vec::new());
        let mut world_fx = WorldFx::new();
        let mut words = [u32::MAX, u32::MAX].into_iter();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 50_000),
            &mut world_fx,
            |_| {
                words
                    .next()
                    .expect("first visit wait prefix consumes exactly two words")
            },
        );

        assert!(matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::SchedulerWaiting { .. }
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
        assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_before);
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.ordinary_type9_selected_component_runtime,
            selected_before
        );
        assert_eq!(entity.mass_raw, mass_before);
        assert_eq!(entity.rotation_heading_pitch_roll_raw(), angles_before);
        assert_eq!(entity.physical_body_basis_q31, basis_before);
        assert_eq!(
            tick.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::InitialGraph
        );
    }

    #[test]
    fn initial_graph_rejects_sub_a_direction_tamper_before_scheduler_rng_or_mutation() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("exact Attract fixture has Sub-A propulsion")
        };
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                sub_a.target_speed_raw(),
                -sub_a.direction_multiplier(),
                sub_a.drive_scale_percent(),
            )));

        assert_initial_graph_tamper_drops_before_scheduler(manager, owner, entity_id);
    }

    #[test]
    fn initial_graph_rejects_sub_a_drive_scale_tamper_before_scheduler_rng_or_mutation() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("exact Attract fixture has Sub-A propulsion")
        };
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                sub_a.target_speed_raw(),
                sub_a.direction_multiplier(),
                sub_a.drive_scale_percent() + 1,
            )));

        assert_initial_graph_tamper_drops_before_scheduler(manager, owner, entity_id);
    }

    #[test]
    fn initial_graph_rejects_private_reversal_seed_before_scheduler_rng_or_mutation() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("exact Attract graph has SharedRetarget Primary");
        let Some(ActorTaskRuntime::SharedRetarget(state)) =
            entity.actor_tasks.task_state_mut(primary)
        else {
            panic!("exact Attract Primary is SharedRetarget")
        };
        let mut private_state = state.private_state();
        private_state.reversal_timer_ms = 1;
        state.set_private_state_for_test(private_state);

        assert_initial_graph_tamper_drops_before_scheduler(manager, owner, entity_id);
    }

    #[test]
    fn initial_graph_rejects_animation_descriptor_tamper_before_scheduler_rng_or_mutation() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!("exact Attract fixture has an animation controller")
        };
        let mut descriptor = animation.descriptor();
        descriptor.attention_stop_sound_id ^= 1;
        let mut foreign = ActorAnimationController::from_descriptor(descriptor)
            .expect("tampered descriptor retains a valid variable binding");
        foreign.apply_attract_attention_forced_stop();
        entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(foreign));

        assert_initial_graph_tamper_drops_before_scheduler(manager, owner, entity_id);
    }

    #[test]
    fn even_primary_and_tertiary_share_callback_delta_before_f70() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let RetailRuntimeValue::Known(Some(animation_before)) = entity.actor_animation_runtime
        else {
            panic!("exact Attract fixture has an actor-animation runtime")
        };
        assert!(animation_before.forced_stop());
        assert!(!animation_before.special_mode());
        assert_eq!(animation_before.linked_handle(), None);
        let mut expected_animation = animation_before;
        let forced_stop_selection = expected_animation.advance(1_999, 0, false);
        assert!(forced_stop_selection.zero_velocity);
        let primary_visit = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("even graph has SharedRetarget Primary");
        let tertiary_visit = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .expect("even graph has Cue Tertiary");
        assert!(entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .is_none());
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 1_999),
            &mut world_fx,
            |_| 1,
        );

        assert!(
            matches!(
                &tick.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 1_999,
                    ..
                }
            ),
            "unexpected even production outcome: {:?}",
            tick.outcome
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(primary_visit)
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
            Some(tertiary_visit)
        );
        let Some(ActorTaskRuntime::SharedRetarget(primary)) =
            entity.actor_task_state(ActorTaskSlot::Primary)
        else {
            panic!("even graph retains SharedRetarget Primary")
        };
        let Some(ActorTaskRuntime::AttractAttentionCue(tertiary)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("even graph retains Cue Tertiary")
        };
        assert_eq!(primary.elapsed_ms(), 1);
        assert_eq!(tertiary.elapsed_ms(), 1);
        assert_eq!(
            entity.actor_animation_runtime,
            RetailRuntimeValue::Known(Some(expected_animation)),
            "exact 00420520 forced-stop Sub-I state survives after selecting its linked-group frame"
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                entity.rotation_heading_pitch_roll_raw()[0],
                entity.rotation_heading_pitch_roll_raw()[1],
                entity.rotation_heading_pitch_roll_raw()[2],
            ))
        );
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "same-visit outer completion clears +0xB2 before returning"
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        let retained = tick.retained_owner.expect("F70 retains linear custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending {
                post_task_frame
            } if post_task_frame.callback_elapsed_micros() == 1_999
        ));
    }

    #[test]
    fn odd_candidate_gate_miss_restores_candidate_and_visits_cue() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let context_before = entity.current_behavior_context;
        let candidate_visit = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .expect("odd graph has Candidate Secondary");
        let cue_visit = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .expect("odd graph has Cue Tertiary");
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 2_999),
            &mut world_fx,
            |_| 1,
        );

        assert!(
            matches!(
                &tick.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 2_999,
                    ..
                }
            ),
            "unexpected odd production outcome: {:?}",
            tick.outcome
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert_eq!(entity.current_behavior_context, context_before);
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Secondary),
            Some(candidate_visit)
        );
        assert_eq!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Tertiary),
            Some(cue_visit)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::AttractAttentionCandidate(_))
        ));
        let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        else {
            panic!("odd graph retains Cue Tertiary")
        };
        assert_eq!(cue.elapsed_ms(), 2);
        let retained = tick.retained_owner.expect("pending graph retains custody");
        let Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue }) =
            retained.initial_owners.as_ref()
        else {
            panic!("gate miss restores both initializer-minted owners")
        };
        assert_eq!(candidate.visit().task_id, candidate_visit);
        assert_eq!(cue.visit().task_id, cue_visit);
    }

    #[test]
    fn missing_bubble_environment_retains_origin_tick_and_issued_effect_without_replay() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(3_800);
        let resources = terrain_resources_with_height(-100);
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;

        let first = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 77,
                elapsed_micros: 1_000,
                global_elapsed_micros: 1_000,
                candidates_in_intrusive_order: &[],
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::CurrentTerrainUnavailable,
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("the unexecuted bubble receipt remains linear");
        assert!(matches!(
            retained.outer_tail,
            Some(
                OrdinaryType9AttractAttentionOuterTailCustody::BubblePending {
                    origin_retail_tick: 77,
                    expected_animation_offset_at_0xb2: RetailRuntimeValue::Known(7),
                    ..
                }
            )
        ));
        let calls_after_issue = random_calls;
        let expected_fork = retained.fork_for_main_base_abort_transaction();
        assert_eq!(expected_fork, retained);
        assert!(!retained.main_base_abort_compatible());

        let retry = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 999,
                elapsed_micros: 50_000,
                global_elapsed_micros: 50_000,
                candidates_in_intrusive_order: &[],
            },
            &mut world_fx,
            |_| {
                random_calls += 1;
                0
            },
        );
        assert_eq!(random_calls, calls_after_issue);
        assert!(matches!(
            retry.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::CurrentTerrainUnavailable,
                ..
            }
        ));
        assert_eq!(retry.retained_owner.unwrap(), expected_fork);
    }

    #[test]
    fn attract_fixture_surface_expiry_preflight_is_admitted() {
        let (mut manager, _owner, entity_id) = production_owner_fixture(false);
        let mut world_fx = WorldFx::new();
        let result = manager.publish_ordinary_type9_standard_death(
            entity_id,
            crate::ordinary_type9_standard_death::OrdinaryType9StandardDeathEntry::SurfaceLifetimeExpiry,
            crate::main_base_type9_abort::MainBaseType9ResultScreenState::NotShown,
            &mut world_fx,
            0,
            None,
        );
        assert!(
            result.is_ok(),
            "selected Attract E370 expiry must be admitted: {result:?}"
        );
    }

    #[test]
    fn exact_expiry_publishes_class14_and_consumes_selected_owner() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        entity.surface_lifetime_timer_ms_at_0x48 = RetailRuntimeValue::Known(4_999);
        let resources = terrain_resources_with_height(-100);
        let mut world_fx = WorldFx::new();
        let mut random_calls = 0;

        let first = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 1_000),
            &mut world_fx,
            |_| {
                random_calls += 1;
                1
            },
        );
        assert!(
            matches!(
                first.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::SurfaceLifecycleClass14Published {
                    entity_id: published_id,
                    task_lease: Some(task_lease),
                } if published_id == entity_id
                    && task_lease.actor().entity_id == entity_id
            ),
            "E370 expiry must run FUN_00416750 then class 14: {:?}",
            first.outcome
        );
        assert!(
            first.retained_owner.is_none(),
            "class-14 publication consumes the selected Attract owner"
        );
        let OrdinaryType9AttractAttentionProductionOutcome::SurfaceLifecycleClass14Published {
            task_lease: Some(task_lease),
            ..
        } = first.outcome
        else {
            panic!("class-14 consume must expose the exploding Primary lease")
        };
        let mut scheduler =
            crate::specialized_actor_task_production::SpecializedActorTaskScheduler::new();
        scheduler
            .register_ordinary_type9_class14(task_lease)
            .expect("selected-owner class-14 consume adopts the exploding task");
        assert_eq!(
            scheduler.family_for(entity_id),
            Some(
                crate::specialized_actor_task_production::SpecializedActorTaskFamily::OrdinaryType9Class14
            )
        );
        let actor_lease = task_lease.actor();
        let suffix_id = crate::common_mover::type9_owner::OrdinaryType9OwnerTransactionId::new(
            std::num::NonZeroU64::MIN,
        );
        let (transaction, expected_b2) =
            crate::ordinary_type9_outer_tail::start_class14_suffix_transaction(
                &manager,
                actor_lease,
                suffix_id,
                &resources,
                20_000,
            )
            .expect("class-14 E870 suffix admits the post-expiry allocation");
        let mut suffix_random = |_world_fx: &mut WorldFx| 1;
        let suffix = crate::ordinary_type9_outer_tail::drive_outer_tail_transaction(
            &mut manager,
            actor_lease,
            transaction,
            1_000,
            expected_b2,
            None,
            &mut world_fx,
            &mut suffix_random,
        );
        assert!(
            matches!(
                suffix,
                crate::ordinary_type9_outer_tail::OrdinaryType9OuterTailDrive::Complete(_)
            ),
            "already-dying E370 must continue the suffix, not park"
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(task))
                if task.lifetime_ms()
                    == crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS
        ));
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0),
            "the same E370 visit still clears +0xB2 after class 14"
        );
        assert!(random_calls > 0);

        let Some(ActorTaskRuntime::SharedRetarget(task)) =
            entity.actor_tasks.task_state_mut(task_lease.task_id())
        else {
            panic!("class-14 Primary remains SharedRetarget before terminal")
        };
        assert_eq!(
            task.before_callback(
                (crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS + 1) * 1_000
            )
            .elapsed_ms,
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_EXPLODING_LIFETIME_MS + 1
        );
        let terrain = resources
            .level_terrain()
            .expect("attract fixture retains Section 10");
        let mut terminal_random = || 1;
        let terminal = crate::main_base_type9_production::tick_main_base_type9_exploding_owner(
            &mut manager,
            crate::main_base_type9_production::MainBaseType9ExplodingProductionOwner::adopt(
                task_lease,
            ),
            crate::main_base_type9_production::MainBaseType9ExplodingProductionFrame {
                scheduler_mode: 0,
                terrain,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
            },
            &mut terminal_random,
        );
        assert!(
            matches!(
                terminal.outcome,
                crate::main_base_type9_production::MainBaseType9ExplodingProductionOutcome::Terminal(
                    _
                )
            ),
            "1,000-ms class-14 task must reach FUN_0040C470"
        );
        assert!(terminal.retained_owner.is_none());
        let (terminal_transaction, terminal_b2) =
            crate::ordinary_type9_outer_tail::start_class14_suffix_transaction(
                &manager,
                actor_lease,
                suffix_id,
                &resources,
                20_000,
            )
            .expect("FUN_0040C470 still admits the ordinary E870 suffix");
        let terminal_suffix = crate::ordinary_type9_outer_tail::drive_outer_tail_transaction(
            &mut manager,
            actor_lease,
            terminal_transaction,
            1_000,
            terminal_b2,
            None,
            &mut world_fx,
            &mut suffix_random,
        );
        match terminal_suffix {
            crate::ordinary_type9_outer_tail::OrdinaryType9OuterTailDrive::Complete(_) => {}
            crate::ordinary_type9_outer_tail::OrdinaryType9OuterTailDrive::Blocked {
                reason,
                ..
            } => panic!("terminal E870 suffix blocked: {reason:?}"),
            crate::ordinary_type9_outer_tail::OrdinaryType9OuterTailDrive::StateMismatch => {
                panic!("terminal E870 suffix state mismatch")
            }
        }
        assert_eq!(manager.pending_actor_deferred_destroy_ids(), [entity_id]);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(entity.actor_task_state(ActorTaskSlot::Primary).is_none());
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(DEFERRED_DESTROY_PENDING_STATE_BIT),
            RetailRuntimeValue::Known(DEFERRED_DESTROY_PENDING_STATE_BIT)
        );
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
        assert_eq!(
            manager.cleanup_pending_actor_deferred_destroys(),
            [entity_id]
        );
        assert!(manager.iter_all().all(|entity| entity.id != entity_id));
    }

    #[test]
    fn class14_scheduler_prefix_skips_wait_when_disable_bit_is_set() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let actor_lease = owner.actor_lease();
        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.state_flags_at_0x08.overwrite(
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
                SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
            );
            entity.collision.recent_relation_elapsed_us_at_0x68 = RetailRuntimeValue::Known(0);
            entity.collision.callback_scheduler_accumulator_us_at_0x6c =
                RetailRuntimeValue::Known(0);
            entity.collision.subject_scan_gate_at_0x70 = RetailRuntimeValue::Known(0);
            entity.collision.scheduler_unit_delta_flag_at_0xb6 = RetailRuntimeValue::Known(0);
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(0);
        }
        let mut world_fx = WorldFx::new();
        let mut draws = 0;
        let prefix = crate::ordinary_type9_outer_tail::apply_class14_scheduler_prefix(
            &mut manager,
            actor_lease,
            20_000,
            &mut world_fx,
            &mut |_world_fx: &mut WorldFx| {
                draws += 1;
                1
            },
        );
        assert_eq!(
            prefix,
            Ok(
                crate::ordinary_type9_outer_tail::OrdinaryType9Class14SchedulerPrefix::Continue {
                    callback_elapsed_us: 20_000,
                }
            )
        );
        assert_eq!(draws, 0, "INSTALL 0x06C64825 wait-disable consumes no RNG");
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(
            entity.mass_raw,
            crate::main_base_type9_abort::LEVEL_ONE_TYPE9_MASS_RAW
        );
        assert_eq!(
            entity.collision.callback_scheduler_accumulator_us_at_0x6c,
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn completed_tail_authenticates_full_state_and_zero_b2_before_next_scheduler_visit() {
        for tamper_b2 in [false, true] {
            let (mut manager, owner, entity_id) = production_owner_fixture(false);
            manager
                .entity_mut_for_test(entity_id)
                .unwrap()
                .collision
                .animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let resources = terrain_resources();
            let mut world_fx = WorldFx::new();
            let first = tick_ordinary_type9_attract_attention_owner_with_random(
                &mut manager,
                owner,
                frame(&resources, 1_999),
                &mut world_fx,
                |_| 1,
            );
            let retained = first.retained_owner.expect("tail completes in one visit");
            assert!(matches!(
                retained.outer_tail,
                Some(OrdinaryType9AttractAttentionOuterTailCustody::Complete {
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
            let retry = tick_ordinary_type9_attract_attention_owner_with_random(
                &mut manager,
                retained,
                frame(&resources, 50_000),
                &mut world_fx,
                |_| {
                    replay_random_calls += 1;
                    0
                },
            );
            assert_eq!(replay_random_calls, 0);
            assert!(matches!(
                retry.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                    reason: OrdinaryType9AttractAttentionProductionDrop::OuterTailStateMismatch,
                    ..
                }
            ));
            assert!(retry.retained_owner.is_none());
        }
    }

    #[test]
    fn restricted_mode_rejects_special_or_linked_animation_before_scheduler_mutation() {
        for tamper_linked_handle in [false, true] {
            let (mut manager, owner, entity_id) = production_owner_fixture(false);
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity
                .collision
                .state_flags_at_0x08
                .overwrite(SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT, 0);
            let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime
            else {
                panic!("exact Attract fixture has an actor-animation runtime")
            };
            if tamper_linked_handle {
                animation.set_linked_handle_for_test(Some(0x04ac_0001));
            } else {
                animation.apply_exploding_person_reset();
            }
            let collision_before = entity.collision.clone();
            let tasks_before = live_task_snapshot(&entity.actor_tasks);
            let task_flags_before = live_task_visits(&entity.actor_tasks).map(|visit| {
                visit.and_then(|visit| entity.actor_tasks.wrapper_flags(visit.task_id))
            });
            let context_before = entity.current_behavior_context;
            let animation_before = entity.actor_animation_runtime;
            let resources = ResourceCache::new(Vec::new());
            let mut world_fx = WorldFx::new();
            let mut random_calls = 0;

            let tick = tick_ordinary_type9_attract_attention_owner_with_random(
                &mut manager,
                owner,
                frame(&resources, 50_000),
                &mut world_fx,
                |_| {
                    random_calls += 1;
                    0
                },
            );

            assert_eq!(random_calls, 0);
            assert!(matches!(
                tick.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                    reason: OrdinaryType9AttractAttentionProductionDrop::InitialOwnerMismatch,
                    ..
                }
            ));
            assert!(tick.retained_owner.is_none());
            let entity = manager
                .iter_all()
                .find(|entity| entity.id == entity_id)
                .unwrap();
            assert_eq!(entity.collision, collision_before);
            assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_before);
            assert_eq!(
                live_task_visits(&entity.actor_tasks).map(|visit| {
                    visit.and_then(|visit| entity.actor_tasks.wrapper_flags(visit.task_id))
                }),
                task_flags_before
            );
            assert_eq!(entity.current_behavior_context, context_before);
            assert_eq!(entity.actor_animation_runtime, animation_before);
        }
    }

    fn eligible_attract_target(id: u32, position_raw: [i16; 3]) -> GuardLocationEntityRef {
        GuardLocationEntityRef {
            id,
            entity_type: 99,
            position_raw,
            state_flags_raw: crate::entity_collision_state::RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Known(0x0000_0201),
            attached_entity_handle: RetailRuntimeValue::Known(None),
        }
    }

    #[test]
    fn accepted_candidate_publishes_f70_then_adopts_missing_target_route_next_frame() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime else {
            panic!()
        };
        animation.advance(1_000, 0, false);
        animation.advance(81_000, 0, false);
        let target_id = 0x04ac_2233;
        let candidates = [eligible_attract_target(target_id, entity.position_raw())];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("retarget X/Z, candidate gate, then constructor");
                consumed.push(word);
                word
            },
        );

        let OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished {
            target_id: published_target,
            constructor,
            ..
        } = tick.outcome
        else {
            panic!("accepted Candidate did not publish Target Route")
        };
        assert_eq!(published_target, target_id);
        assert_eq!(constructor.constructor_suffix.random_word, 0x1234);
        assert_eq!(consumed, [0x1111, 0x2222, 0, 0x1234]);
        assert!(words.is_empty());
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.target_id() == Some(target_id) && route.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert_eq!(animation.phase(), 0);
        assert!(
            !animation.forced_stop(),
            "AF50 retires cue during the accepted Candidate callback"
        );
        assert_eq!(
            animation.output(),
            34,
            "402AC0 preserves the last published word until another Sub-I visit"
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                entity.rotation_heading_pitch_roll_raw()[0],
                entity.rotation_heading_pitch_roll_raw()[1],
                entity.rotation_heading_pitch_roll_raw()[2],
            ))
        );
        let retained = tick
            .retained_owner
            .expect("parked Target Route retains custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));

        let mut replay_words = VecDeque::from([0xffff_u32, 0xd2f6]);
        let mut replay_consumed = Vec::new();
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                let word = replay_words
                    .pop_front()
                    .expect("missing target skips mover and draws root selector then constructor");
                replay_consumed.push(word);
                word
            },
        );
        assert_eq!(replay_consumed, [0xffff, 0xd2f6]);
        assert!(replay_words.is_empty());
        assert!(
            matches!(
                &replay.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
            ),
            "unexpected route transition outcome: {:?}",
            replay.outcome
        );
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(route))
                if route.elapsed_ms() == 0
        ));
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn missing_accepted_target_route_adopts_attract_then_resumes_at_candidate_and_cue() {
        let live_text = RefCell::new(Vec::new());
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_7788;
        let candidates = [eligible_attract_target(target_id, position_raw)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let mut publish_consumed = Vec::new();

        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| {
                let word = publish_words
                    .pop_front()
                    .expect("old Primary mover, Candidate gate, and Target Route suffix");
                publish_consumed.push(word);
                word
            },
        );
        assert_eq!(publish_consumed, [0x1111, 0x2222, 0, 0x1234]);
        assert!(publish_words.is_empty());
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished {
                target_id: published_target,
                ..
            } if published_target == target_id
        ));
        let retained = published
            .retained_owner
            .expect("accepted absent target retains parked Target Route custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
        assert!(manager.iter_all().all(|entity| entity.id != target_id));

        append_attract_root_witnesses(&mut manager, position_raw);
        let receipts_before = live_text.borrow().len();
        let mut root_words =
            VecDeque::from([ATTRACT_ROOT_SELECTOR_WORD, 1_u32, 0x3333, 0x4444, 0x5555, 0]);
        let mut root_consumed = Vec::new();
        let rooted = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                ..frame(&resources, 20_000)
            },
            &mut world_fx,
            |_| {
                let word = root_words
                    .pop_front()
                    .expect("root selector, odd class-45 transaction, then Candidate gate word");
                root_consumed.push(word);
                word
            },
        );

        assert_eq!(
            root_consumed,
            [ATTRACT_ROOT_SELECTOR_WORD, 1, 0x3333, 0x4444, 0x5555, 0,],
            "Target Route missing-target root selection must resume at Secondary"
        );
        assert!(root_words.is_empty());
        let committed = match rooted.outcome {
            OrdinaryType9AttractAttentionProductionOutcome::RootAttractAttentionPublished {
                committed,
                ..
            } => committed,
            other => panic!("unexpected missing-route class-45 outcome: {other:?}"),
        };
        assert_eq!(committed.parity_random_sample_low16, 1);
        assert_eq!(live_text.borrow().len(), receipts_before + 1);
        world_fx.process_pending();
        assert_eq!(world_fx.take_positional_sounds().len(), 1);

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Primary),
                Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
            ),
            "newborn Primary must not be revisited after a Primary-origin root transition"
        );
        let secondary_visit = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Secondary)
            .expect("odd rooted Attract retains Candidate Secondary");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::AttractAttentionCandidate(_))
        ));
        let tertiary_visit = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Tertiary)
            .expect("odd rooted Attract retains Cue Tertiary");
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 20
        ));
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("rooted class-45 graph retains a live context")
        };
        assert_eq!(
            context.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(target_id)),
            "root context preserves the absent Target Route handle"
        );
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0),
            "candidate search does not overwrite the behavior auxiliary word"
        );
        assert!(matches!(entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(axis)
                if axis.raw_word_at_0x04 == 0x201 && axis.strict_axis_limit_raw == 0xF00
        ));

        let retained = rooted
            .retained_owner
            .expect("same-pass Candidate/Cue suffix retains rooted custody");
        let Some(OrdinaryType9AttractAttentionInitialOwners::CandidateAndCue { candidate, cue }) =
            retained.initial_owners.as_ref()
        else {
            panic!("odd rooted Attract retains both linear sibling owners")
        };
        assert_eq!(candidate.visit().task_id, secondary_visit);
        assert_eq!(cue.visit().task_id, tertiary_visit);
        let Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)) =
            retained.root_publication.as_ref()
        else {
            panic!("rooted class-45 publication must survive the suffix dispatch")
        };
        assert!(publication.authenticates_publication(entity));
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
    }

    #[test]
    fn published_live_target_route_runs_selected_mover_on_the_next_frame() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_4455;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        let retained = published.retained_owner.unwrap();
        let animation_before = manager
            .entity_mut_for_test(entity_id)
            .unwrap()
            .actor_animation_runtime;
        let next = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );

        assert!(matches!(
            next.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                callback_elapsed_micros: 20_000,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the live route committed the neutral selected-mover animation stage"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.elapsed_ms() == 20
        ));
        assert!(matches!(
            next.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
    }

    #[test]
    fn published_target_route_axis_mismatch_consumes_prefix_without_mover_or_replay() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5566;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!("axis mismatch must block after callback entry")
        };
        let OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionAdapterError::ActorCommonAxisDescriptorMismatch {
                expected,
                actual,
            }
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        );
        assert!(
            retained.target_route.is_none(),
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
        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
    }

    #[test]
    fn published_target_route_unresolved_axis_consumes_prefix_without_mover_or_replay() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5567;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved axis must block after callback entry: {:?}",
                failed.outcome
            )
        };
        let OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionAdapterError::ActorCommonAxisDescriptorUnavailable
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        );
        assert!(
            retained.target_route.is_none(),
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
        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
    }

    #[test]
    fn published_target_route_unresolved_sub_a_fails_preflight_without_mover_or_route_age() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_556e;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved Sub-A must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );
        assert!(
            retained.target_route.as_ref().is_some_and(|route| {
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

        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each TargetRouteActive retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(known_sub_a));
        }

        let recovered = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "restored Sub-A must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the recovered route committed the neutral selected-mover animation stage"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.elapsed_ms() == 20
        ));
        assert!(matches!(
            recovered.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
    }

    #[test]
    fn published_target_route_unresolved_sub_a_target_speed_fails_preflight_without_mover_or_route_age(
    ) {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_556f;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved Sub-A target speed must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );
        assert!(
            retained.target_route.as_ref().is_some_and(|route| {
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

        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each TargetRouteActive retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
            entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(known_sub_a));
        }

        let recovered = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "restored Sub-A target speed must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the recovered route committed the neutral selected-mover animation stage"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.elapsed_ms() == 20
        ));
        assert!(matches!(
            recovered.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
    }

    #[test]
    fn published_target_route_unresolved_animation_fails_preflight_without_mover_or_route_age() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5570;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved animation must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );
        assert!(
            retained.target_route.as_ref().is_some_and(|route| {
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

        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each TargetRouteActive retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
            entity.actor_animation_runtime = RetailRuntimeValue::Known(Some(known_animation));
        }

        let recovered = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
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
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.elapsed_ms() == 20
        ));
        assert!(matches!(
            recovered.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
    }

    #[test]
    fn published_target_route_unresolved_physical_body_basis_fails_preflight_without_mover_or_route_age(
    ) {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5571;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

        // The published F70 tail observes a Known body basis. Consume that
        // Complete custody with the already-closed animation preflight so the
        // TargetRouteActive visit can unset the basis without
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
        let armed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = armed.outcome
        else {
            panic!(
                "animation setup must reach TargetRouteActive preflight: {:?}",
                armed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
                OrdinaryType9AttractAttentionTargetRouteTickError::Preflight(
                    OrdinaryType9AttractAttentionTargetRoutePreflightError::ActorAnimationRuntimeUnavailable
                )
            )
        );
        let retained = armed
            .retained_owner
            .expect("animation preflight retains TargetRouteActive custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved body basis must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );
        assert!(
            retained.target_route.as_ref().is_some_and(|route| {
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

        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each TargetRouteActive retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
            entity.physical_body_basis_q31 = RetailRuntimeValue::Known(known_basis);
        }

        let recovered = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
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
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.elapsed_ms() == 20
        ));
        assert!(matches!(
            recovered.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
    }

    #[test]
    fn published_target_route_unresolved_immutable_anchor_fails_preflight_without_mover_or_route_age(
    ) {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5572;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved immutable anchor must block at Target Route preflight: {:?}",
                failed.outcome
            )
        };
        assert_eq!(
            reason,
            OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );
        assert!(
            retained.target_route.as_ref().is_some_and(|route| {
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

        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert_eq!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionProductionState::TargetRouteActive
        );

        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            assert_eq!(
                entity.collision.recent_relation_elapsed_us_at_0x68,
                RetailRuntimeValue::Known(scheduler_elapsed_before.wrapping_add(40_000)),
                "each TargetRouteActive retry still commits the outer scheduler prefix"
            );
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("retryable preflight leaves the published wrapper installed")
            };
            assert_eq!(route.elapsed_ms(), 0);
            assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
            entity
                .ordinary_type9_selected_component_runtime
                .as_mut()
                .expect("retryable preflight retains selected component custody")
                .set_immutable_anchor_for_test(RetailRuntimeValue::Known(known_anchor));
        }

        let recovered = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        assert!(
            matches!(
                recovered.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "restored immutable anchor must enter the selected mover: {:?}",
            recovered.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the recovered route committed the neutral selected-mover animation stage"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.elapsed_ms() == 20
        ));
        assert!(matches!(
            recovered.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
    }

    #[test]
    fn published_target_route_in_callback_wrapper_drops_publication_before_preflight() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5573;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

        let (
            scheduler_elapsed_before,
            selected_before,
            sub_a_before,
            animation_before,
            heading_before,
            velocity_before,
            collision_before,
            tasks_before,
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
                live_task_snapshot(&entity.actor_tasks),
                route.private_state().target_position_raw,
            )
        };
        let mut callback_random_calls = 0;
        let dropped = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        assert_eq!(
            dropped.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Dropped {
                entity_id,
                reason: OrdinaryType9AttractAttentionProductionDrop::RootPublicationMismatch,
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
        assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_before);
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
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5574;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        let retained = published.retained_owner.unwrap();

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

        let next = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| 1,
        );
        assert!(
            !matches!(
                next.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                    reason: OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
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
    fn published_target_route_dying_target_skips_mover_and_applies_wander_root() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5568;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        let retained = published.retained_owner.unwrap();
        manager
            .entity_mut_for_test(target_id)
            .expect("published Target Route retains its live target")
            .collision
            .state_flags_at_0x08 = RetailStateWord::exact(1 | DYING_STATE_BIT);

        let mut root_words = VecDeque::from([0xffff_u32, 0xd2f6]);
        let mut consumed = Vec::new();
        let rooted = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                let word = root_words
                    .pop_front()
                    .expect("dying target skips mover and draws root selector then constructor");
                consumed.push(word);
                word
            },
        );
        assert_eq!(consumed, [0xffff, 0xd2f6]);
        assert!(root_words.is_empty());
        assert!(
            matches!(
                rooted.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
            ),
            "dying target must apply the existing no-redraw class-6 root: {:?}",
            rooted.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) if wander.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            manager
                .entity_mut_for_test(target_id)
                .expect("dying target remains in the world")
                .collision
                .state_flags_at_0x08
                .masked(DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        let retained = rooted
            .retained_owner
            .expect("rooted Wander retains scheduler custody");
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::Wander(_))
        ));
    }

    #[test]
    fn published_target_route_inactive_target_skips_mover_and_applies_wander_root() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_5569;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        let retained = published.retained_owner.unwrap();
        manager
            .entity_mut_for_test(target_id)
            .expect("published Target Route retains its live target")
            .active = false;

        let mut root_words = VecDeque::from([0xffff_u32, 0xd2f6]);
        let mut consumed = Vec::new();
        let rooted = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                let word = root_words
                    .pop_front()
                    .expect("inactive target skips mover and draws root selector then constructor");
                consumed.push(word);
                word
            },
        );
        assert_eq!(consumed, [0xffff, 0xd2f6]);
        assert!(root_words.is_empty());
        assert!(
            matches!(
                rooted.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
            ),
            "inactive target must apply the existing no-redraw class-6 root: {:?}",
            rooted.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) if wander.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        assert!(
            !manager
                .entity_mut_for_test(target_id)
                .expect("inactive target remains in the world")
                .active
        );
        let retained = rooted
            .retained_owner
            .expect("rooted Wander retains scheduler custody");
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::Wander(_))
        ));
    }

    #[test]
    fn published_target_route_unresolved_target_state_consumes_prefix_without_mover_or_replay() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_556a;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

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
                .entity_mut_for_test(target_id)
                .expect("published Target Route retains its live target")
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
        let failed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        let OrdinaryType9AttractAttentionProductionOutcome::Blocked { reason, .. } = failed.outcome
        else {
            panic!(
                "unresolved target state must block after callback entry: {:?}",
                failed.outcome
            )
        };
        let OrdinaryType9AttractAttentionProductionBlock::TargetRoute(
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
            OrdinaryType9AttractAttentionAdapterError::TargetStateUnresolved { target_id }
        );
        let retained = failed
            .retained_owner
            .expect("production retains terminal failure custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        );
        assert!(
            retained.target_route.is_none(),
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
        let tasks_after_failure = live_task_snapshot(&entity.actor_tasks);
        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_eq!(entity.collision, collision_after_failure);
        assert_eq!(live_task_snapshot(&entity.actor_tasks), tasks_after_failure);
    }

    #[test]
    fn published_target_route_expiry_applies_wander_root() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_556b;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

        {
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
        }

        let mut root_words = VecDeque::from([0xffff_u32, 0xd2f6]);
        let mut consumed = Vec::new();
        let rooted = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                let word = root_words.pop_front().expect(
                    "expired route runs the mover then draws root selector then constructor",
                );
                consumed.push(word);
                word
            },
        );
        assert_eq!(consumed, [0xffff, 0xd2f6]);
        assert!(root_words.is_empty());
        assert!(
            matches!(
                rooted.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
            ),
            "elapsed > 5000 must apply the existing no-redraw class-6 root: {:?}",
            rooted.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) if wander.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let retained = rooted
            .retained_owner
            .expect("rooted Wander retains scheduler custody");
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::Wander(_))
        ));
    }

    #[test]
    fn published_target_route_zero_predicate_applies_wander_root() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let target_id = 0x04ac_556d;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

        {
            let actor_position = manager
                .entity_mut_for_test(entity_id)
                .expect("published actor remains live")
                .position_raw();
            let live_target = manager
                .entity_mut_for_test(target_id)
                .expect("published Target Route retains its live target");
            let mut live_position = live_target.position_raw();
            live_position[0] = actor_position[0].wrapping_add(0x0F00);
            live_target.set_motion_raw(live_position, [0; 3]);
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let Some(ActorTaskRuntime::AttractAttentionTargetRoute(route)) =
                entity.actor_task_state(ActorTaskSlot::Primary)
            else {
                panic!("published Primary must remain Target Route")
            };
            assert_eq!(route.elapsed_ms(), 0);
        }

        let mut root_words = VecDeque::from([0xffff_u32, 0xd2f6]);
        let mut consumed = Vec::new();
        let rooted = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                let word = root_words.pop_front().expect(
                    "zero-predicate runs the mover then draws root selector then constructor",
                );
                consumed.push(word);
                word
            },
        );
        assert_eq!(consumed, [0xffff, 0xd2f6]);
        assert!(root_words.is_empty());
        assert!(
            matches!(
                rooted.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
            ),
            "out-of-range live target must apply the existing no-redraw class-6 root: {:?}",
            rooted.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::OrdinaryType9Wander(wander)) if wander.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity.collision.animation_offset_at_0xb2,
            RetailRuntimeValue::Known(0)
        );
        let retained = rooted
            .retained_owner
            .expect("rooted Wander retains scheduler custody");
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::Wander(_))
        ));
    }

    #[test]
    fn published_target_route_suppressed_expiry_retains_the_route_owner() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let actor_position = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.collision.state_flags_at_0x08.overwrite(
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
                ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT,
            );
            entity.position_raw()
        };
        let target_id = 0x04ac_556c;
        let target_position = [
            actor_position[0].wrapping_add(0x100),
            actor_position[1],
            actor_position[2],
        ];
        let mut target = Entity::unresolved_port_entity(target_id, EntityKind::Unknown(99), 99);
        target.set_motion_raw(target_position, [0; 3]);
        target.collision.state_flags_at_0x08 = RetailStateWord::exact(1);
        manager.append_entity_for_test(target);
        let candidates = [eligible_attract_target(target_id, target_position)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| publish_words.pop_front().unwrap(),
        );
        assert!(matches!(
            published.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetRoutePublished { .. }
        ));
        assert!(publish_words.is_empty());
        let retained = published.retained_owner.unwrap();

        let animation_before = {
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
            entity.actor_animation_runtime
        };
        let mut callback_random_calls = 0;
        let suppressed = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                callback_random_calls += 1;
                1
            },
        );
        assert_eq!(callback_random_calls, 0);
        assert!(
            matches!(
                suppressed.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::OuterTailComplete {
                    callback_elapsed_micros: 20_000,
                    ..
                }
            ),
            "0x1000 must suppress LifetimeExpired and keep the live route: {:?}",
            suppressed.outcome
        );
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_animation_runtime, animation_before,
            "the suppressed expiry frame still commits the neutral selected-mover animation stage"
        );
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::AttractAttentionTargetRoute(route))
                if route.target_id() == Some(target_id) && route.elapsed_ms() == 5_020
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT),
            RetailRuntimeValue::Known(ACTOR_OWNER_TRANSITION_SUPPRESSED_STATE_BIT)
        );
        let retained = suppressed
            .retained_owner
            .expect("suppressed expiry retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::TargetRouteParked { .. }
        ));
        assert!(retained
            .target_route
            .as_ref()
            .is_some_and(|route| route.validate(entity).is_ok()));
    }

    #[test]
    fn target_allocation_failure_uses_gate_only_and_publishes_exact_fallback() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let target_id = 0x04ac_3344;
        let candidates = [eligible_attract_target(target_id, entity.position_raw())];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0]);
        let mut consumed = Vec::new();
        let mut allocation_calls = 0;

        let tick = tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_: &mut WorldFx| {
                let word = words
                    .pop_front()
                    .expect("retarget X/Z and Candidate gate only");
                consumed.push(word);
                word
            },
            |_| {
                allocation_calls += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Failed
            },
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
        );

        assert_eq!(consumed, [0x1111, 0x2222, 0]);
        assert!(words.is_empty(), "failed preparation skips constructor RNG");
        assert_eq!(allocation_calls, 1);
        assert!(matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::TargetInitializerFallbackPublished {
                failure: OrdinaryType9AttractAttentionTargetInitializerFailure {
                    slot: ActorTaskSlot::Primary,
                    role: crate::attract_attention::AttractAttentionTaskRole::RouteToTarget,
                },
                ..
            }
        ));
        let entity = manager
            .iter_all()
            .find(|entity| entity.id == entity_id)
            .unwrap();
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .expect("fallback retains selected components")
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
        assert_eq!(
            entity.physical_body_basis_q31,
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(
                entity.rotation_heading_pitch_roll_raw()[0],
                entity.rotation_heading_pitch_roll_raw()[1],
                entity.rotation_heading_pitch_roll_raw()[2],
            ))
        );
        assert!(matches!(
            tick.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
    }

    #[test]
    fn expired_shared_retarget_publishes_root_wander_before_returning() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::SharedRetarget(state)) =
            entity.actor_tasks.task_state_mut(primary)
        else {
            panic!("exact Attract Primary is SharedRetarget")
        };
        state.before_callback(900_000);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, 0xffff, 0xd2f6]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 501_000),
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("retarget X/Z, root selector, then constructor");
                consumed.push(word);
                word
            },
        );

        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
            ),
            "unexpected root-expiry outcome: {:?}",
            tick.outcome
        );
        assert_eq!(consumed, [0x1111, 0x2222, 0xffff, 0xd2f6]);
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
            tick.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
    }

    #[test]
    fn expired_shared_retarget_publishes_root_go_to_job_without_visiting_newborn_primary() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let (position_raw, predecessor_task_id, target_before, auxiliary_before, initial_before) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let predecessor_task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(predecessor_task_id)
            else {
                panic!("exact Attract Primary is SharedRetarget")
            };
            state.before_callback(900_000);
            let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
                panic!("exact Attract graph has a live context")
            };
            (
                entity.position_raw(),
                predecessor_task_id,
                context.target_handle_at_0x08(),
                context.auxiliary_word_at_0x0c(),
                entity.initial_behavior,
            )
        };
        append_open_base(&mut manager, ROOT_BASE_ID, position_raw);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            0x1111_u32,
            0x2222,
            GO_TO_JOB_ROOT_SELECTOR_WORD,
            0x1234_D2F6,
        ]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 501_000),
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("retarget X/Z, class-54 selector, then constructor");
                consumed.push(word);
                word
            },
        );

        let constructor = match tick.outcome {
            OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobPublished {
                entity_id: seen_entity,
                target_id: ROOT_BASE_ID,
                constructor,
            } if seen_entity == entity_id => constructor,
            outcome => panic!("unexpected class-54 root outcome: {outcome:?}"),
        };
        assert_eq!(constructor.random_sample_low16, 0xD2F6);
        assert_eq!(constructor.sub_a_target_speed_raw, 333);
        assert_eq!(
            consumed,
            [0x1111, 0x2222, GO_TO_JOB_ROOT_SELECTOR_WORD, 0x1234_D2F6]
        );
        assert!(words.is_empty());
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let new_task_id = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .expect("class-54 publishes Primary");
        assert_ne!(new_task_id, predecessor_task_id);
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(task))
                if task.elapsed_ms() == 0 && task.target_id() == Some(ROOT_BASE_ID)
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("class-54 publication retains a live context")
        };
        assert_eq!(context.target_handle_at_0x08(), target_before);
        assert_eq!(context.auxiliary_word_at_0x0c(), auxiliary_before);
        assert_eq!(entity.initial_behavior, initial_before);
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("class-54 retains resolved Sub-A")
        };
        assert_eq!(sub_a.target_speed_raw(), RetailRuntimeValue::Known(333));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(BODY_BASIS_REBUILT_STATE_BIT),
            RetailRuntimeValue::Known(BODY_BASIS_REBUILT_STATE_BIT)
        );
        let retained = tick
            .retained_owner
            .expect("class-54 retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(!retained
            .fork_for_main_base_abort_transaction()
            .main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn expired_cue_class54_allocation_failure_is_terminal_without_constructor_rng() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let tertiary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap();
            let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
                entity.actor_tasks.task_state_mut(tertiary)
            else {
                panic!("exact Attract Tertiary is Cue")
            };
            cue.advance_elapsed(cue.lifetime_ms().saturating_mul(1_000));
            entity.position_raw()
        };
        append_open_base(&mut manager, ROOT_BASE_ID, position_raw);
        let exact_model_slots = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let exact = entity.model_slots;
            entity.model_slots = [None; 4];
            exact
        };
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, GO_TO_JOB_ROOT_SELECTOR_WORD]);
        let mut allocations = 0;

        let first = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            owner,
            frame(&resources, 1_000),
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("old Primary mover and one class-54 selector word")
            },
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| {
                allocations += 1;
                OrdinaryType9GoToJobAllocationDecision::Failed
            },
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
        );

        assert_eq!(allocations, 0, "preflight precedes class-54 allocation");
        assert!(
            words.is_empty(),
            "preflight failure consumes only mover and selector RNG"
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootGoToJobPreflight(
                    OrdinaryType9RootGoToJobPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("Cue-origin retry retains frozen root plan");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { .. }
        ));
        assert_eq!(
            retained
                .pending_root_continuation
                .as_ref()
                .expect("Cue-origin retry retains dispatcher cursor")
                .next_slot,
            None
        );

        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
        let tick = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| panic!("Cue-origin retry cannot redraw selector or constructor RNG"),
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| {
                allocations += 1;
                OrdinaryType9GoToJobAllocationDecision::Failed
            },
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
        );

        assert_eq!(allocations, 1);
        assert!(matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobInitializerFallbackPublished {
                entity_id: seen_entity,
                target_id: ROOT_BASE_ID,
                ..
            } if seen_entity == entity_id
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert_eq!(animation.phase(), 0);
        assert!(
            !animation.forced_stop(),
            "expired cue retirement precedes the failed root allocation"
        );
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
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
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn missing_target_route_retries_frozen_class54_plan_from_secondary_cursor() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let missing_target_id = 0x04ac_8854;
        let candidates = [eligible_attract_target(missing_target_id, position_raw)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| {
                publish_words
                    .pop_front()
                    .expect("Target Route publication word")
            },
        );
        assert!(publish_words.is_empty());
        let retained = published
            .retained_owner
            .expect("parked route retains custody");
        append_open_base(&mut manager, ROOT_BASE_ID, position_raw);
        let exact_model_slots = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let exact = entity.model_slots;
            entity.model_slots = [None; 4];
            exact
        };
        let mut selector_words = VecDeque::from([GO_TO_JOB_ROOT_SELECTOR_WORD]);
        let mut first_allocations = 0;
        let first = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                selector_words
                    .pop_front()
                    .expect("one class-54 selector word")
            },
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| {
                first_allocations += 1;
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
        );
        assert!(selector_words.is_empty());
        assert_eq!(
            first_allocations, 0,
            "preflight precedes class-54 allocation"
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootGoToJobPreflight(
                    OrdinaryType9RootGoToJobPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("retry retains frozen root plan");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { .. }
        ));
        assert_eq!(
            retained
                .pending_root_continuation
                .as_ref()
                .expect("retry retains dispatcher cursor")
                .next_slot,
            Some(ActorTaskSlot::Secondary)
        );
        assert!(matches!(
            retained
                .pending_root_plan
                .as_ref()
                .expect("retry retains already-drawn selection")
                .selection(),
            OrdinaryType9RootSelection::Weighted { selection, .. }
                if u32::from(selection.program.class_id) == LEVEL_ONE_TYPE9_GO_TO_JOB_CLASS_ID
        ));

        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
        let base = manager.entity_mut_for_test(ROOT_BASE_ID).unwrap();
        let RetailRuntimeValue::Known(Some(base)) = &mut base.base_factory_runtime else {
            panic!("frozen target has exact capacity")
        };
        base.current_scientists = base.required_scientists;
        append_open_base(&mut manager, LATE_ROOT_BASE_ID, position_raw);
        let mut constructor_words = VecDeque::from([0xCAFE_D2F6_u32]);
        let mut retry_allocations = 0;
        let retry = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| {
                constructor_words
                    .pop_front()
                    .expect("retry skips selector and draws only constructor RNG")
            },
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| {
                retry_allocations += 1;
                OrdinaryType9GoToJobAllocationDecision::Prepared
            },
            |_| OrdinaryType9RunAwayAllocationDecision::Prepared,
        );
        assert!(constructor_words.is_empty());
        assert_eq!(retry_allocations, 1);
        assert!(matches!(
            retry.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::RootGoToJobPublished {
                entity_id: seen_entity,
                target_id: ROOT_BASE_ID,
                ..
            } if seen_entity == entity_id
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::GoToJob(task))
                if task.target_id() == Some(ROOT_BASE_ID) && task.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
        let retained = retry
            .retained_owner
            .expect("class-54 retry retains custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn expired_primary_adopts_attract_root_and_visits_only_newborn_suffix() {
        let live_text = RefCell::new(Vec::new());
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("exact Attract Primary is SharedRetarget")
            };
            state.before_callback(900_000);
            entity.position_raw()
        };
        append_attract_root_witnesses(&mut manager, position_raw);
        let receipts_before = live_text.borrow().len();
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            0x1111_u32,
            0x2222,
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
        ]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                ..frame(&resources, 501_000)
            },
            &mut world_fx,
            |_| {
                let word = words.pop_front().expect(
                    "old Primary mover, selector, odd constructor, and Candidate gate words",
                );
                consumed.push(word);
                word
            },
        );

        assert_eq!(
            consumed,
            [
                0x1111,
                0x2222,
                ATTRACT_ROOT_SELECTOR_WORD,
                1,
                0x3333,
                0x4444,
                0x5555,
                0,
            ]
        );
        assert!(words.is_empty());
        let committed = match &tick.outcome {
            OrdinaryType9AttractAttentionProductionOutcome::RootAttractAttentionPublished {
                committed,
                ..
            } => *committed,
            other => panic!("unexpected rooted Attract outcome: {other:?}"),
        };
        assert_eq!(committed.parity_random_sample_low16, 1);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Secondary),
            Some(ActorTaskRuntime::AttractAttentionCandidate(_))
        ));
        assert!(
            matches!(
                entity.actor_task_state(ActorTaskSlot::Tertiary),
                Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 125
            ),
            "newborn Cue did not receive the original callback delta: {:?}",
            entity.actor_task_state(ActorTaskSlot::Tertiary)
        );
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            panic!("root Attract retains a live context")
        };
        assert_eq!(
            context.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert!(matches!(entity.actor_common_axis_descriptor,
            RetailRuntimeValue::Known(axis)
                if axis.raw_word_at_0x04 == 0x201 && axis.strict_axis_limit_raw == 0xF00
        ));
        assert_eq!(live_text.borrow().len(), receipts_before + 1);
        world_fx.process_pending();
        assert_eq!(
            world_fx.take_positional_sounds().len(),
            usize::from(committed.positional_sound.is_some())
        );
        let retained = tick
            .retained_owner
            .expect("root Attract retains scheduler custody");
        assert!(!retained.main_base_abort_compatible());
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(_))
        ));
        let Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)) =
            retained.root_publication.as_ref()
        else {
            unreachable!()
        };
        assert!(
            publication.authenticates_publication(manager.entity_mut_for_test(entity_id).unwrap()),
            "completed outer tail must retain an exact rooted Attract publication"
        );

        let next = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                ..frame(&resources, 1_000)
            },
            &mut world_fx,
            |_| 0,
        );
        assert!(
            !matches!(
                next.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::Dropped { .. }
            ),
            "rooted Attract failed to re-enter after its completed tail: {:?}",
            next.outcome
        );
        let retained = next
            .retained_owner
            .expect("following rooted Attract frame retains custody");
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(_))
        ));
        assert!(!retained.main_base_abort_compatible());
    }

    #[test]
    fn expired_cue_adopts_attract_root_without_visiting_newborn_graph() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let RetailRuntimeValue::Known(Some(animation)) = &mut entity.actor_animation_runtime
            else {
                panic!()
            };
            animation.advance(1_000, 0, false);
            animation.advance(81_000, 0, false);
            assert_eq!(animation.phase(), 2);
            let tertiary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap();
            let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
                entity.actor_tasks.task_state_mut(tertiary)
            else {
                panic!("exact Attract Tertiary is Cue")
            };
            cue.advance_elapsed(cue.lifetime_ms().saturating_mul(1_000));
            entity.position_raw()
        };
        append_attract_root_witnesses(&mut manager, position_raw);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            0x1111_u32,
            0x2222,
            ATTRACT_ROOT_SELECTOR_WORD,
            0,
            0x3333,
            0x4444,
        ]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 1_000),
            &mut world_fx,
            |_| {
                let word = words
                    .pop_front()
                    .expect("old Primary mover, selector, and even constructor transaction words");
                consumed.push(word);
                word
            },
        );

        assert_eq!(
            consumed,
            [
                0x1111,
                0x2222,
                ATTRACT_ROOT_SELECTOR_WORD,
                0,
                0x3333,
                0x4444,
            ]
        );
        assert!(matches!(
            tick.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::RootAttractAttentionPublished { .. }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 0
        ));
        let RetailRuntimeValue::Known(Some(animation)) = entity.actor_animation_runtime else {
            panic!()
        };
        assert_eq!(
            animation.phase(),
            0,
            "old cue destructor runs after unwind before replacing it"
        );
        assert!(
            animation.forced_stop(),
            "new cue applies 420870 after the old cue reset"
        );
    }

    #[test]
    fn rooted_attract_effects_survive_later_candidate_dispatch_failure_once() {
        let live_text = RefCell::new(Vec::new());
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("exact Attract Primary is SharedRetarget")
            };
            state.before_callback(900_000);
            entity.position_raw()
        };
        append_attract_root_witnesses(&mut manager, position_raw);
        let receipts_before = live_text.borrow().len();
        let unresolved_candidate = GuardLocationEntityRef {
            id: 0x05ac_00ff,
            entity_type: 99,
            position_raw,
            state_flags_raw: RetailStateWord::exact(1),
            capability_flags: RetailRuntimeValue::Unresolved,
            attached_entity_handle: RetailRuntimeValue::Known(None),
        };
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            0x1111_u32,
            0x2222,
            ATTRACT_ROOT_SELECTOR_WORD,
            1,
            0x3333,
            0x4444,
            0x5555,
            0,
        ]);

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                resources: &resources,
                retail_tick: 501_000,
                elapsed_micros: 501_000,
                global_elapsed_micros: 501_000,
                candidates_in_intrusive_order: &[unresolved_candidate],
            },
            &mut world_fx,
            |_| words.pop_front().expect("exact class-45 failure-path word"),
        );

        assert!(words.is_empty());
        assert!(
            matches!(
                tick.outcome,
                OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                    reason: OrdinaryType9AttractAttentionProductionBlock::Dispatcher(_),
                    ..
                }
            ),
            "unexpected post-root Candidate failure outcome: {:?}",
            tick.outcome
        );
        assert_eq!(live_text.borrow().len(), receipts_before + 1);
        world_fx.process_pending();
        assert_eq!(world_fx.take_positional_sounds().len(), 1);
        let retained = tick
            .retained_owner
            .expect("post-root callback failure retains terminal custody");
        assert_eq!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::CallbackFailurePending
        );
        assert!(matches!(
            retained.root_publication,
            Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(_))
        ));
        assert!(!retained.main_base_abort_compatible());

        let mut replay_random_calls = 0;
        let replay = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                ..frame(&resources, 1)
            },
            &mut world_fx,
            |_| {
                replay_random_calls += 1;
                0
            },
        );
        assert_eq!(replay_random_calls, 0);
        assert!(matches!(
            replay.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::CallbackFailurePending,
                ..
            }
        ));
        assert_eq!(live_text.borrow().len(), receipts_before + 1);
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
    }

    #[test]
    fn root_preflight_retry_reuses_retained_plan_without_selector_redraw() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
        let primary = entity
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary)
            .unwrap();
        let Some(ActorTaskRuntime::SharedRetarget(state)) =
            entity.actor_tasks.task_state_mut(primary)
        else {
            panic!("exact Attract Primary is SharedRetarget")
        };
        state.before_callback(900_000);
        let exact_model_slots = entity.model_slots;
        entity.model_slots = [None; 4];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words = VecDeque::from([0x1111_u32, 0x2222, 0xffff]);
        let mut first_consumed = Vec::new();

        let first = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 501_000),
            &mut world_fx,
            |_| {
                let word = first_words
                    .pop_front()
                    .expect("retarget X/Z and one root selector draw");
                first_consumed.push(word);
                word
            },
        );

        assert_eq!(first_consumed, [0x1111, 0x2222, 0xffff]);
        assert!(first_words.is_empty());
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootWanderPreflight(
                    OrdinaryType9RootWanderPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("retryable root preflight retains production custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { .. }
        ));
        assert!(retained.pending_root_plan.is_some());

        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
        let mut retry_words = VecDeque::from([0xd2f6_u32]);
        let mut retry_consumed = Vec::new();
        let retry = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| {
                let word = retry_words
                    .pop_front()
                    .expect("retained plan skips selector and draws only the constructor word");
                retry_consumed.push(word);
                word
            },
        );

        assert_eq!(retry_consumed, [0xd2f6]);
        assert!(retry_words.is_empty());
        assert!(matches!(
            retry.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::RootWanderPublished { .. }
        ));
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
            retry.retained_owner.unwrap().state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
    }

    #[test]
    fn rooted_attract_preflight_retry_defers_initializer_rng_and_effects_without_selector_redraw() {
        let live_text = RefCell::new(Vec::new());
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let (position_raw, exact_model_slots) = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let primary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(primary)
            else {
                panic!("exact Attract Primary is SharedRetarget")
            };
            state.before_callback(900_000);
            let exact_model_slots = entity.model_slots;
            entity.model_slots = [None; 4];
            (entity.position_raw(), exact_model_slots)
        };
        append_attract_root_witnesses(&mut manager, position_raw);
        let receipts_before = live_text.borrow().len();
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut first_words = VecDeque::from([0x1111_u32, 0x2222, ATTRACT_ROOT_SELECTOR_WORD]);
        let mut first_consumed = Vec::new();
        let mut first_target_allocations = 0;
        let mut first_attract_allocations = 0;
        let mut first_wander_allocations = 0;

        let first = tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                retail_tick: 77,
                ..frame(&resources, 501_000)
            },
            &mut world_fx,
            |_: &mut WorldFx| {
                let word = first_words
                    .pop_front()
                    .expect("old Primary mover and one class-45 selector word");
                first_consumed.push(word);
                word
            },
            |_| {
                first_target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
            },
            |_| {
                first_attract_allocations += 1;
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| {
                first_wander_allocations += 1;
                OrdinaryType9WanderAllocationDecision::Prepared
            },
        );

        assert_eq!(first_consumed, [0x1111, 0x2222, ATTRACT_ROOT_SELECTOR_WORD]);
        assert!(first_words.is_empty());
        assert_eq!(
            (
                first_target_allocations,
                first_attract_allocations,
                first_wander_allocations,
            ),
            (0, 0, 0),
            "class-45 preflight must precede every allocation seam"
        );
        assert_eq!(live_text.borrow().len(), receipts_before);
        world_fx.process_pending();
        assert!(world_fx.take_positional_sounds().is_empty());
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootAttractAttentionPreflight(
                    OrdinaryType9RootAttractAttentionPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("retryable class-45 preflight retains production custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { .. }
        ));
        assert!(matches!(
            retained
                .pending_root_plan
                .as_ref()
                .expect("failed preflight retains the already-drawn plan")
                .selection(),
            OrdinaryType9RootSelection::Weighted {
                selection,
                initializer_identity:
                    crate::ordinary_type9_initial_selection::FreshLevel1Type9InitializerIdentity::AttractAttentionInitial,
                ..
            } if selection.choice_index == 1
        ));

        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
        let mut retry_words = VecDeque::from([0_u32, 0x3333, 0x4444]);
        let mut retry_consumed = Vec::new();
        let mut retry_target_allocations = 0;
        let mut retry_attract_phases = Vec::new();
        let mut retry_wander_allocations = 0;
        let retry = tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
            &mut manager,
            retained,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|request, tick| {
                    live_text.borrow_mut().push((request, tick))
                },
                retail_tick: 79,
                ..frame(&resources, 1)
            },
            &mut world_fx,
            |_: &mut WorldFx| {
                let word = retry_words
                    .pop_front()
                    .expect("retained plan draws only parity and Cue/Wander suffix words");
                retry_consumed.push(word);
                word
            },
            |_| {
                retry_target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
            },
            |preparation| {
                assert_eq!(
                    live_text.borrow().last().map(|(_, tick)| *tick),
                    Some(79),
                    "retained root dispatch uses the retry visit, before Cue construction"
                );
                retry_attract_phases.push(preparation.phase_index);
                OrdinaryType9AttractAttentionAllocationDecision::Prepared
            },
            |_| {
                retry_wander_allocations += 1;
                OrdinaryType9WanderAllocationDecision::Prepared
            },
        );

        assert_eq!(retry_consumed, [0, 0x3333, 0x4444]);
        assert!(retry_words.is_empty(), "retry cannot redraw the selector");
        assert_eq!(retry_target_allocations, 0);
        assert_eq!(retry_attract_phases, [1, 2]);
        assert_eq!(retry_wander_allocations, 0);
        let committed = match retry.outcome {
            OrdinaryType9AttractAttentionProductionOutcome::RootAttractAttentionPublished {
                committed,
                ..
            } => committed,
            other => panic!("unexpected retained class-45 outcome: {other:?}"),
        };
        assert_eq!(committed.parity_random_sample_low16, 0);
        assert!(committed.resource_text.is_some());
        assert!(committed.positional_sound.is_some());
        assert_eq!(live_text.borrow().len(), receipts_before + 1);
        world_fx.process_pending();
        assert_eq!(world_fx.take_positional_sounds().len(), 1);

        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::SharedRetarget(primary)) if primary.elapsed_ms() == 0
        ));
        assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Tertiary),
            Some(ActorTaskRuntime::AttractAttentionCue(cue)) if cue.elapsed_ms() == 125
        ));
        let retained = retry
            .retained_owner
            .expect("successful class-45 retry retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(matches!(
            retained.initial_owners,
            Some(OrdinaryType9AttractAttentionInitialOwners::CueOnly { .. })
        ));
        let Some(OrdinaryType9AttractAttentionRootPublication::AttractAttention(publication)) =
            retained.root_publication.as_ref()
        else {
            panic!("successful retry must retain rooted class-45 publication")
        };
        assert!(publication.authenticates_publication(entity));
        assert!(!retained.main_base_abort_compatible());
    }

    #[test]
    fn retained_root_plan_rejects_initial_behavior_tamper_before_rng_or_allocation() {
        let (mut manager, retained, entity_id, resources, mut world_fx) =
            retained_root_plan_fixture();
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.initial_behavior,
            RetailRuntimeValue::Known(Some(_))
        ));
        entity.initial_behavior = RetailRuntimeValue::Known(None);
        let mut rng_calls = 0;
        let mut target_allocations = 0;
        let mut root_allocations = 0;

        let retry = tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| {
                rng_calls += 1;
                0
            },
            |_| {
                target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
            },
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| {
                root_allocations += 1;
                OrdinaryType9WanderAllocationDecision::Prepared
            },
        );

        assert!(matches!(
            retry.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
                ..
            }
        ));
        assert_eq!((rng_calls, target_allocations, root_allocations), (0, 0, 0));
    }

    #[test]
    fn retained_root_plan_rejects_exact_sub_a_tamper_before_rng_or_allocation() {
        let (mut manager, retained, entity_id, resources, mut world_fx) =
            retained_root_plan_fixture();
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        let RetailRuntimeValue::Known(Some(sub_a)) = entity.sub_a_propulsion_runtime else {
            panic!("exact Attract fixture has Sub-A propulsion")
        };
        entity.sub_a_propulsion_runtime =
            RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_retail_words(
                sub_a.target_speed_raw(),
                sub_a.direction_multiplier(),
                sub_a.drive_scale_percent() + 1,
            )));
        let mut rng_calls = 0;
        let mut target_allocations = 0;
        let mut root_allocations = 0;

        let retry = tick_ordinary_type9_attract_attention_owner_with_random_and_allocators(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| {
                rng_calls += 1;
                0
            },
            |_| {
                target_allocations += 1;
                OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared
            },
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| {
                root_allocations += 1;
                OrdinaryType9WanderAllocationDecision::Prepared
            },
        );

        assert!(matches!(
            retry.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootTransitionRequestChanged,
                ..
            }
        ));
        assert_eq!((rng_calls, target_allocations, root_allocations), (0, 0, 0));
    }

    #[test]
    fn expired_shared_retarget_publishes_root_run_away_and_fleeing_primary_same_tick() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        let predecessor_task_id = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let predecessor_task_id = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Primary)
                .unwrap();
            let Some(ActorTaskRuntime::SharedRetarget(state)) =
                entity.actor_tasks.task_state_mut(predecessor_task_id)
            else {
                panic!("exact Attract Primary is SharedRetarget")
            };
            state.before_callback(900_000);
            predecessor_task_id
        };
        resolve_run_away_audio(&mut manager);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, true);
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([
            0x1111_u32,
            0x2222,
            RUN_AWAY_ROOT_SELECTOR_WORD,
            0x3333,
            0x4444,
            0x5555,
        ]);
        let mut consumed = Vec::new();

        let tick = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            frame(&resources, 501_000),
            &mut world_fx,
            |_| {
                let word = words.pop_front().expect(
                    "retarget X/Z, class-10 selector, both acquiring constructors, then fleeing",
                );
                consumed.push(word);
                word
            },
        );

        let constructors = match tick.outcome {
            OrdinaryType9AttractAttentionProductionOutcome::RootRunAwayPublished {
                entity_id: seen_entity,
                constructors_by_phase,
            } if seen_entity == entity_id => constructors_by_phase,
            outcome => panic!("unexpected class-10 root outcome: {outcome:?}"),
        };
        assert_eq!(constructors[0].random_sample_low16, 0x3333);
        assert_eq!(constructors[1].random_sample_low16, 0x4444);
        assert_eq!(
            consumed,
            [
                0x1111,
                0x2222,
                RUN_AWAY_ROOT_SELECTOR_WORD,
                0x3333,
                0x4444,
                0x5555
            ]
        );
        assert!(words.is_empty());
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            Some(predecessor_task_id)
        );
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
        let retained = tick
            .retained_owner
            .expect("class-10 retains scheduler custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn expired_cue_class10_allocation_failure_is_terminal_without_constructor_rng() {
        let (mut manager, owner, entity_id) = production_owner_fixture(false);
        {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            let tertiary = entity
                .actor_tasks
                .task_in_slot(ActorTaskSlot::Tertiary)
                .unwrap();
            let Some(ActorTaskRuntime::AttractAttentionCue(cue)) =
                entity.actor_tasks.task_state_mut(tertiary)
            else {
                panic!("exact Attract Tertiary is Cue")
            };
            cue.advance_elapsed(cue.lifetime_ms().saturating_mul(1_000));
        }
        resolve_run_away_audio(&mut manager);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, false);
        let exact_model_slots = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let exact = entity.model_slots;
            entity.model_slots = [None; 4];
            exact
        };
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut words = VecDeque::from([0x1111_u32, 0x2222, RUN_AWAY_ROOT_SELECTOR_WORD]);
        let mut allocations = Vec::new();

        let first = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            owner,
            frame(&resources, 1_000),
            &mut world_fx,
            |_| {
                words
                    .pop_front()
                    .expect("old Primary mover and one class-10 selector word")
            },
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |preparation| {
                allocations.push(preparation.phase_index);
                OrdinaryType9RunAwayAllocationDecision::Failed
            },
        );

        assert!(
            allocations.is_empty(),
            "preflight precedes class-10 allocation"
        );
        assert!(
            words.is_empty(),
            "preflight failure consumes only mover and selector RNG"
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootRunAwayPreflight(
                    OrdinaryType9RootRunAwayPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("Cue-origin retry retains frozen root plan");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { .. }
        ));
        assert_eq!(
            retained
                .pending_root_continuation
                .as_ref()
                .expect("Cue-origin retry retains dispatcher cursor")
                .next_slot,
            None
        );

        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
        let tick = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| panic!("Cue-origin retry cannot redraw selector or constructor RNG"),
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |preparation| {
                allocations.push(preparation.phase_index);
                OrdinaryType9RunAwayAllocationDecision::Failed
            },
        );

        assert_eq!(allocations, [0]);
        match tick.outcome {
            OrdinaryType9AttractAttentionProductionOutcome::RootRunAwayInitializerFallbackPublished {
                entity_id: seen_entity,
                failure,
                constructors_by_phase,
            } if seen_entity == entity_id => {
                assert_eq!(failure.phase_index, 0);
                assert_eq!(constructors_by_phase, [None, None]);
            }
            outcome => panic!("unexpected Cue-origin class-10 fallback: {outcome:?}"),
        }
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_task_state(slot).is_none()));
        assert_eq!(
            entity
                .ordinary_type9_selected_component_runtime
                .unwrap()
                .kind(),
            OrdinaryType9SelectedRuntimeKind::InitializerFailureFallbackPublished
        );
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
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }

    #[test]
    fn missing_target_route_retries_frozen_class10_plan_from_secondary_cursor() {
        let (mut manager, owner, entity_id) = production_owner_fixture(true);
        let position_raw = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            entity.collision.animation_offset_at_0xb2 = RetailRuntimeValue::Known(7);
            entity.position_raw()
        };
        let missing_target_id = 0x04ac_8854;
        let candidates = [eligible_attract_target(missing_target_id, position_raw)];
        let resources = terrain_resources();
        let mut world_fx = WorldFx::new();
        let mut publish_words = VecDeque::from([0x1111_u32, 0x2222, 0, 0x1234]);
        let published = tick_ordinary_type9_attract_attention_owner_with_random(
            &mut manager,
            owner,
            OrdinaryType9AttractAttentionProductionFrame {
                dispatch_resource_text: &|_, _| {},
                resources: &resources,
                retail_tick: 2_999,
                elapsed_micros: 2_999,
                global_elapsed_micros: 2_999,
                candidates_in_intrusive_order: &candidates,
            },
            &mut world_fx,
            |_| {
                publish_words
                    .pop_front()
                    .expect("Target Route publication word")
            },
        );
        assert!(publish_words.is_empty());
        let retained = published
            .retained_owner
            .expect("parked route retains custody");
        resolve_run_away_audio(&mut manager);
        append_run_away_candidate(&mut manager, entity_id, RUN_AWAY_TARGET_ID, true);
        let exact_model_slots = {
            let entity = manager.entity_mut_for_test(entity_id).unwrap();
            let exact = entity.model_slots;
            entity.model_slots = [None; 4];
            exact
        };
        let mut selector_words = VecDeque::from([RUN_AWAY_ROOT_SELECTOR_WORD]);
        let mut first_allocations = Vec::new();
        let first = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            retained,
            frame(&resources, 20_000),
            &mut world_fx,
            |_| {
                selector_words
                    .pop_front()
                    .expect("one class-10 selector word")
            },
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |preparation| {
                first_allocations.push(preparation.phase_index);
                OrdinaryType9RunAwayAllocationDecision::Prepared
            },
        );
        assert!(selector_words.is_empty());
        assert!(
            first_allocations.is_empty(),
            "preflight precedes class-10 allocation"
        );
        assert!(matches!(
            first.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::Blocked {
                reason: OrdinaryType9AttractAttentionProductionBlock::RootRunAwayPreflight(
                    OrdinaryType9RootRunAwayPreflightError::ModelSlotsMismatch {
                        actual: [None, None, None, None]
                    }
                ),
                ..
            }
        ));
        let retained = first
            .retained_owner
            .expect("retry retains frozen root plan");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::RootTransitionPending { .. }
        ));
        assert_eq!(
            retained
                .pending_root_continuation
                .as_ref()
                .expect("retry retains dispatcher cursor")
                .next_slot,
            Some(ActorTaskSlot::Secondary)
        );
        assert!(matches!(
            retained
                .pending_root_plan
                .as_ref()
                .expect("retry retains already-drawn selection")
                .selection(),
            OrdinaryType9RootSelection::Weighted { selection, .. }
                if u32::from(selection.program.class_id) == LEVEL_ONE_TYPE9_RUN_AWAY_CLASS_ID
        ));

        manager.entity_mut_for_test(entity_id).unwrap().model_slots = exact_model_slots;
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
        let mut retry_allocations = Vec::new();
        let retry = tick_ordinary_type9_attract_attention_owner_with_random_and_all_allocators(
            &mut manager,
            retained,
            frame(&resources, 1),
            &mut world_fx,
            |_| {
                constructor_words
                    .pop_front()
                    .expect("two acquiring and one fleeing constructor; no selector redraw")
            },
            |_| OrdinaryType9AttractAttentionTargetAllocationDecision::Prepared,
            |_| OrdinaryType9AttractAttentionAllocationDecision::Prepared,
            |_| OrdinaryType9WanderAllocationDecision::Prepared,
            |_| OrdinaryType9GoToJobAllocationDecision::Prepared,
            |preparation| {
                retry_allocations.push(preparation.phase_index);
                OrdinaryType9RunAwayAllocationDecision::Prepared
            },
        );
        assert!(constructor_words.is_empty());
        assert_eq!(retry_allocations, [0, 1]);
        assert!(matches!(
            retry.outcome,
            OrdinaryType9AttractAttentionProductionOutcome::RootRunAwayPublished { .. }
        ));
        let entity = manager.entity_mut_for_test(entity_id).unwrap();
        assert!(matches!(
            entity.actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::RunAway(task))
                if task.target_id() == RUN_AWAY_TARGET_ID && task.elapsed_ms() == 0
        ));
        let retained = retry
            .retained_owner
            .expect("class-10 retry retains custody");
        assert!(matches!(
            retained.state(),
            OrdinaryType9AttractAttentionProductionState::PostBasisTailPending { .. }
        ));
        assert!(!retained.main_base_abort_compatible());
        assert!(retained
            .root_publication
            .as_ref()
            .is_some_and(|publication| publication.authenticates(entity)));
    }
}
